use std::{
    thread::{self, JoinHandle},
    sync::{
        mpsc::{self, Sender, Receiver},
        Arc, Mutex,
    },
};
use crate::{
    encoder::Encoder,
    decoder::Decoder,
    progress::Progress,
    crc32::Crc32,
    block::{Block, BlockQueue},
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
    /// input block and returning the compressed block.
    pub fn compress_block(&mut self, blk: Block) {
        let mem = self.mem as usize;
        self.sndr.send(
            Message::NewJob(
                Box::new(move || {
                    let mut enc = Encoder::new(mem, blk.data.len());
                    enc.compress_block(&blk.data);
                    Block {
                        chksum: (&blk.data).crc32(),
                        files:  blk.files,
                        size:   enc.blk_out.len() as u64,
                        data:   enc.blk_out,
                        id:     blk.id,
                        unsize: blk.data.len() as u64,
                    }
                })
            )
        ).unwrap();   
    }

    /// Create a new message containing a job consisting of decompressing
    /// an input block and returning the decompressed block.
    pub fn decompress_block(&mut self, blk: Block) {
        let mem = self.mem as usize;
        self.sndr.send(
            Message::NewJob(
                Box::new(move || {
                    let mut dec = Decoder::new(blk.data, mem);
                    let blk_out = dec.decompress_block(blk.unsize as usize);
                    // Verify checksum
                    Block {
                        chksum: (&blk_out).crc32(),
                        files:  blk.files,
                        data:   blk_out,
                        id:     blk.id,
                        size:   0,
                        unsize: 0,
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

