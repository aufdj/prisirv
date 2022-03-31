use std::{
    thread::{self, JoinHandle},
    sync::{
        mpsc::{self, Sender, Receiver},
        Arc, Mutex,
    },
    fs::File,
    io::BufWriter,
};
use crate::{
    encoder::Encoder,
    decoder::Decoder,
    metadata::Metadata,
    buffered_io::BufferedWrite,
    progress::Progress,
    crc32::Crc32,
};

pub enum Message {
    NewJob(Job),
    Terminate,
}


type Job = Box<dyn FnOnce() -> Block + Send + 'static>;
type SharedBlockQueue = Arc<Mutex<BlockQueue>>;
type SharedReceiver   = Arc<Mutex<Receiver<Message>>>;
type SharedProgress   = Arc<Mutex<Progress>>;

/// A threadpool spawns a set number of threads and handles sending new 
/// jobs to idle threads, where a job is a function that returns a
/// compressed or decompressed block.
pub struct ThreadPool {
    threads:  Vec<Thread>,
    sndr:     Sender<Message>,
    mem:      u64,
    pub bq:   SharedBlockQueue,
}
impl ThreadPool {
    /// Create a new ThreadPool.
    pub fn new(size: usize, mem: u64, prg: Progress) -> ThreadPool {
        let (sndr, rcvr) = mpsc::channel();
        let mut threads = Vec::with_capacity(size);

        let rcvr = Arc::new(Mutex::new(rcvr));
        let bq   = Arc::new(Mutex::new(BlockQueue::new()));
        let prg  = Arc::new(Mutex::new(prg));

        for _ in 0..size {
            threads.push(
                Thread::new(
                    Arc::clone(&rcvr), Arc::clone(&bq), Arc::clone(&prg)
                )
            );
        }
        ThreadPool { threads, sndr, mem, bq }
    }
    
    /// Create a new message containing a job consisting of compressing an
    /// input block and returning the compressed block and its index.
    pub fn compress_block(&mut self, block: Vec<u8>, index: u64) {
        let mem = self.mem as usize;
        self.sndr.send(
            Message::NewJob(
                Box::new(move || {
                    let mut enc = Encoder::new(mem, block.len());
                    enc.compress_block(&block);
                    Block {
                        chksum: (&block).crc32(),
                        data:   enc.blk_out,
                        id:     index,
                    }
                })
            )
        ).unwrap();   
    }

    /// Create a new message containing a job consisting of decompressing
    /// an input block and returning the compressed block and its index.
    //  The decompressed block will be larger than the compressed block, 
    //  so pass in blk_sz instead of blk.len() to avoid growing output vector.
    pub fn decompress_block(&mut self, blk: Vec<u8>, index: u64, blk_sz: usize) {
        let mem = self.mem as usize;
        self.sndr.send(
            Message::NewJob(
                Box::new(move || {
                    let mut dec = Decoder::new(blk, mem);
                    let blk_out = dec.decompress_block(blk_sz);
                    Block {
                        chksum: (&blk_out).crc32(),
                        data:   blk_out,
                        id:     index,
                    }
                })
            )
        ).unwrap();   
    }
}

/// Send a terminate message to every spawned thread and join all handles.
impl Drop for ThreadPool {
    fn drop(&mut self) {
        for _ in &self.threads {
            self.sndr.send(Message::Terminate).unwrap();
        }

        for thread in &mut self.threads {
            if let Some(handle) = thread.handle.take() {
                handle.join().unwrap();
            }
        }
    }
}


/// A thread and associated handle. A thread recieves a block from the main
/// thread and compresses or decompresses it, then pushes the new block to 
/// a block queue.
struct Thread {
    handle: Option<JoinHandle<()>>,
}
impl Thread {
    /// Spawn a thread and enter a loop, waiting to recieve a message 
    /// containing a new block to compress or decompress, or a message
    /// to terminate the thread.
    fn new(rcvr: SharedReceiver, bq: SharedBlockQueue, prg: SharedProgress) -> Thread {
        let handle = thread::spawn(move || loop {
            let message = rcvr.lock().unwrap().recv().unwrap();

            match message {
                Message::NewJob(job) => { 
                    let block = job();
                    {
                        let prg_guard = prg.lock().unwrap();
                        let mut prg = prg_guard;
                        prg.update();
                    }
                    let queue_guard = bq.lock().unwrap();
                    let mut queue = queue_guard;
                    queue.blocks.push(block);
                }
                Message::Terminate => { break; }
            }
        });
        Thread { handle: Some(handle) }
    }
}


pub struct Block {
    data:   Vec<u8>,
    id:     u64,
    chksum: u32,
}

/// Stores compressed or decompressed blocks. Blocks need to be written in
/// the same order that they were read, but no guarantee can be made about
/// which blocks will be compressed/decompressed first, so each block is 
/// added to a BlockQueue, which handles outputting in the correct order.
pub struct BlockQueue {
    pub blocks: Vec<Block>, // Blocks to be output
    next_out:   u64, // Next block to be output
}
impl BlockQueue {
    /// Create a new BlockQueue.
    pub fn new() -> BlockQueue {
        BlockQueue {
            blocks: Vec::new(),
            next_out: 0,
        }
    }

    /// Try writing the next compressed block to be output. If this block 
    /// hasn't been added to the queue yet, do nothing.
    pub fn try_write_block_enc(&mut self, mta: &mut Metadata, file_out: &mut BufWriter<File>) -> u64 {
        let len = self.blocks.len();
        let mut next_out = self.next_out;

        self.blocks.retain(|block|
            if block.id == next_out {
                mta.enc_blk_szs.push(block.data.len() as u64);
                for byte in block.data.iter() {
                    file_out.write_byte(*byte);
                }
                next_out += 1;
                false
            }
            else { 
                true 
            }
        );
        let blocks_written = (len - self.blocks.len()) as u64;
        self.next_out += blocks_written;
        blocks_written
    }

    /// Try writing the next decompressed block to be output. If this block 
    /// hasn't been added to the queue yet, do nothing.
    pub fn try_write_block_dec(&mut self, file_out: &mut BufWriter<File>) -> u64 {
        let len = self.blocks.len();
        let mut next_out = self.next_out;

        self.blocks.retain(|block|
            if block.id == next_out {
                for byte in block.data.iter() {
                    file_out.write_byte(*byte);
                }
                next_out += 1;
                false
            }
            else { 
                true 
            } 
        );
        let blocks_written = (len - self.blocks.len()) as u64;
        self.next_out += blocks_written;
        blocks_written
    }

    /// Try getting the next block to be output. If this block hasn't been 
    /// added to the queue yet, do nothing.
    pub fn try_get_block(&mut self, blk_out: &mut Vec<u8>) {
        let mut index = None;

        // Try to find next block to be output
        for (blk_i, blk) in self.blocks.iter_mut().enumerate() {
            if blk.id == self.next_out {
                blk_out.append(&mut blk.data);
                index = Some(blk_i);
            }
        }

        // If next block was found, remove from list
        if let Some(i) = index {
            self.blocks.swap_remove(i);
            self.next_out += 1;
        }
    }
}
