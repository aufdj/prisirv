use std::{
    thread::{self, JoinHandle},
    sync::{
        mpsc::{self, Sender, Receiver},
        Arc, Mutex,
    },
    time::SystemTime,
};
use crate::{
    cm::encoder::Encoder,
    cm::decoder::Decoder,
    progress::Progress,
    crc32::Crc32,
    block::{Block, BlockQueue},
    config::{Config, Method},
    error::ExtractError,
    lzw,
};

pub enum Task {
    Compress(A),
    Decompress(B),
    Terminate,
}

type A = Box<dyn FnOnce() -> Block + Send + 'static>;
type B = Box<dyn FnOnce() -> Result<Block, ExtractError> + Send + 'static>;
type SharedBlockQueue = Arc<Mutex<BlockQueue>>;
type SharedReceiver   = Arc<Mutex<Receiver<Task>>>;
type SharedProgress   = Arc<Mutex<Progress>>;

/// A threadpool spawns a set number of threads and handles sending new 
/// jobs to idle threads, where a job is a function that returns a
/// compressed or decompressed block.
pub struct ThreadPool {
    threads:  Vec<Thread>,
    sndr:     Sender<Task>,
    pub bq:   SharedBlockQueue,
}
impl ThreadPool {
    /// Create a new ThreadPool.
    pub fn new(cfg: &Config, prg: Progress) -> ThreadPool {
        let (sndr, rcvr) = mpsc::channel();
        let mut threads = Vec::with_capacity(cfg.threads);

        let rcvr = Arc::new(Mutex::new(rcvr));
        let bq   = Arc::new(Mutex::new(BlockQueue::new(cfg.insert_id)));
        let prg  = Arc::new(Mutex::new(prg));

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

    // pub fn store_block(&mut self, blk_in: Block) {
    //     self.sndr.send(
    //         Task::Compress(
    //             Box::new(move || {
    //                 let crtd = SystemTime::now()
    //                     .duration_since(SystemTime::UNIX_EPOCH)
    //                     .unwrap().as_secs() as u64;

    //                 Block {
    //                     crtd,
    //                     ..blk_in
    //                 }
    //             })
    //         )
    //     ).unwrap();
    // }
    
    /// Create a new task containing a job consisting of compressing an
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
                            let mut enc = Encoder::new(mem, len);
                            enc.compress_block(&blk_in.data);
                            enc.blk_out
                        }
                        Method::Lzw => {
                            lzw::encoder::compress(&blk_in.data, mem)
                        }
                        Method::Store => {
                            blk_in.data
                        }
                    };
                    
                    let crtd = SystemTime::now()
                        .duration_since(SystemTime::UNIX_EPOCH)
                        .unwrap().as_secs() as u64;

                    Block {
                        sizeo:  blk_out.len() as u64,
                        data:   blk_out,
                        chksum,
                        sizei, 
                        crtd,
                        ..blk_in
                    }
                    
                })
            )
        ).unwrap();
    }

    /// Create a new task containing a job consisting of decompressing
    /// an input block and returning the decompressed block.
    pub fn decompress_block(&mut self, blk_in: Block) {
        let len = blk_in.data.len();
        let mem = blk_in.mem as usize;
        self.sndr.send(
            Task::Decompress(
                Box::new(move || {
                    let blk_out = match blk_in.method {
                        Method::Cm => {
                            Decoder::new(blk_in.data, mem)
                            .decompress_block(blk_in.sizei as usize)
                        }
                        Method::Lzw => {
                            lzw::decoder::decompress(&blk_in.data, mem)
                        }
                        Method::Store => {
                            blk_in.data 
                        }
                    };
                    
                    let chksum = (&blk_out).crc32();
                    if chksum != blk_in.chksum {
                        return Err(ExtractError::IncorrectChecksum(blk_in.id));
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
                    let blk = job();
                    { prg.lock().unwrap().update(&blk); }
                    bq.lock().unwrap().blocks.push(blk);
                }
                Task::Decompress(job) => {
                    match job() {
                        Ok(blk) => {
                            { prg.lock().unwrap().update(&blk); }
                            bq.lock().unwrap().blocks.push(blk);
                        }
                        Err(_) => {
                            break;
                        }
                    };
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

