use std::{
    time::SystemTime,
    cmp::Ordering,
    collections::BinaryHeap,
    thread::{self, JoinHandle},
    sync::{
        mpsc::{self, Sender, Receiver},
        Arc, Mutex,
    },
};
use crate::{
    progress::Progress,
    crc32::Crc32,
    block::Block,
    config::{Config, Method},
    error::ArchiveError,
    constant::Version,
    lzw, cm
};

pub enum Task {
    Compress(B),
    Decompress(B),
    Terminate,
}

type B = Box<dyn FnOnce() -> Result<Block, ArchiveError> + Send + 'static>;

type SharedBlockQueue = Arc<Mutex<BlockQueue>>;
type SharedReceiver   = Arc<Mutex<Receiver<Task>>>;
type SharedProgress   = Arc<Mutex<Progress>>;

/// A threadpool spawns a set number of threads and handles sending new
/// tasks to idle threads, where a task is a function that returns a
/// compressed or decompressed block.
pub struct ThreadPool {
    threads:  Vec<Thread>,
    sndr:     Sender<Task>,
    pub bq:   SharedBlockQueue,
}
impl ThreadPool {
    /// Create a new ThreadPool.
    pub fn new(offset: u32, cfg: &Config) -> ThreadPool {
        let (sndr, rcvr) = mpsc::channel();
        let mut threads = Vec::with_capacity(cfg.threads);

        let rcvr = Arc::new(Mutex::new(rcvr));
        let prg  = Arc::new(Mutex::new(Progress::new(cfg)));
        let bq   = Arc::new(Mutex::new(BlockQueue::new(offset)));

        for _ in 0..cfg.threads {
            threads.push(
                Thread::new(
                    Arc::clone(&rcvr), 
                    Arc::clone(&bq), 
                    Arc::clone(&prg)
                )
            );
        }
        ThreadPool { 
            threads, sndr, bq 
        }
    }

    /// Create a new task consisting of compressing an
    /// input block and returning the compressed block.
    pub fn compress_block(&mut self, blk_in: Block) {
        let len = blk_in.data.len();
        let mem = blk_in.mem as usize;
        
        self.sndr.send(
            Task::Compress(
                Box::new(move || {
                    let chksum = (&blk_in.data).crc32();
                    let sizei = blk_in.data.len() as u64;

                    let blk_out = match blk_in.method {
                        Method::Cm => {
                            let mut enc = cm::encoder::Encoder::new(mem, len);
                            enc.compress_block(&blk_in.data);
                            enc.blk_out
                        }
                        Method::Lzw => {
                            lzw::encoder::compress(blk_in.data, mem)

                            // let block = lzw::encoder::compress(blk_in.data, mem);
                            // let mut enc = lzw::ari_enc::Encoder::new(len);
                            // enc.compress_block(&block);
                            // enc.blk_out
                        }
                        Method::Store => {
                            blk_in.data
                        }
                    };
                    
                    let crtd = SystemTime::now()
                        .duration_since(SystemTime::UNIX_EPOCH)?
                        .as_secs() as u64;

                    Ok(
                        Block {
                            sizeo:  blk_out.len() as u64,
                            data:   blk_out,
                            chksum,
                            sizei, 
                            crtd,
                            ..blk_in
                        }
                    ) 
                })
            )
        ).unwrap();
    }

    /// Create a new task containing a job consisting of decompressing
    /// an input block and returning the decompressed block.
    pub fn decompress_block(&mut self, blk_in: Block) -> Result<(), ArchiveError> {
        if blk_in.ver != Version::current() {
            return Err(ArchiveError::InvalidVersion(blk_in.ver));
        }
        let len = blk_in.data.len();
        let mem = blk_in.mem as usize;    
        self.sndr.send(
            Task::Decompress(
                Box::new(move || {
                    let blk_out = match blk_in.method {
                        Method::Cm => {
                            cm::decoder::Decoder::new(blk_in.data, mem)
                            .decompress_block(blk_in.sizei as usize)
                        }
                        Method::Lzw => {
                            lzw::decoder::decompress(blk_in.data, mem)
                            // let mut dec = lzw::ari_dec::Decoder::new(blk_in.data);
                            // let block = dec.decompress_block(blk_in.sizei as usize);
                            // lzw::decoder::decompress(block, mem)
                        }
                        Method::Store => {
                            blk_in.data 
                        }
                    };
                    
                    let chksum = (&blk_out).crc32();
                    if chksum != blk_in.chksum {
                        return Err(ArchiveError::IncorrectChecksum(blk_in.id));
                    }
                    
                    Ok(
                        Block {
                            sizeo:  blk_out.len() as u64,
                            sizei:  len as u64,
                            data:   blk_out,
                            crtd:   0,
                            chksum,
                            ..blk_in
                        }
                    )
                })
            )
        ).unwrap();   
        Ok(())
    }
}

/// Send a terminate task to every spawned thread and join all handles.
impl Drop for ThreadPool {
    fn drop(&mut self) {
        for _ in &self.threads {
            self.sndr.send(Task::Terminate).unwrap();
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
            let task = rcvr.lock().unwrap().recv().unwrap();

            match task {
                Task::Compress(job) => {
                    if let Ok(blk) = job() {
                        prg.lock().unwrap().update(&blk);
                        bq.lock().unwrap().blocks.push(blk);
                    }
                    else {
                        break;
                    }
                }
                Task::Decompress(job) => {
                    if let Ok(blk) = job() {
                        prg.lock().unwrap().update(&blk);
                        bq.lock().unwrap().blocks.push(blk);
                    }
                    else {
                        break;
                    }
                }
                Task::Terminate => {
                    break;
                }
            }
        });
        
        Thread {
            handle: Some(handle)
        }
    }
}

/// Stores compressed or decompressed blocks. Blocks need to be written in
/// the same order that they were read, but no guarantee can be made about
/// which blocks will be compressed/decompressed first, so each block is
/// added to a BlockQueue, which handles outputting in the correct order.
pub struct BlockQueue {
    pub blocks:   BinaryHeap<Block>, // Priority Queue based on block id
    pub offset:   u32, // Starting id when appending to archive
    pub next_out: u32, // Next block to be output
}
impl BlockQueue {
    /// Create a new BlockQueue.
    pub fn new(start: u32) -> BlockQueue {
        BlockQueue {
            blocks:    BinaryHeap::new(),
            offset:    start,
            next_out:  0,
        }
    }

    /// Get block with highest priority (lowest id).
    /// Return block if its id equals next out.
    pub fn try_get_block(&mut self) -> Option<Block> {
        if let Some(blk) = self.blocks.peek() {
            if blk.id == self.next_out {
                self.next_out += 1;
                let mut block = self.blocks.pop().unwrap(); 
                block.id += self.offset;
                return Some(block);
            }
        }
        None
    }
}


impl Ord for Block {
    fn cmp(&self, other: &Self) -> Ordering {
        other.id.cmp(&self.id)
    }
}
impl PartialOrd for Block {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Eq for Block {}
impl PartialEq for Block {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
