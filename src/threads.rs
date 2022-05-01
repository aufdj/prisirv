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
    config::Method,
    lzw,
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
    pub bq:   SharedBlockQueue,
}
impl ThreadPool {
    /// Create a new ThreadPool.
    pub fn new(size: usize, prg: Progress) -> ThreadPool {
        let (sndr, rcvr) = mpsc::channel();
        let mut threads = Vec::with_capacity(size);

        let rcvr = Arc::new(Mutex::new(rcvr));
        let bq   = Arc::new(Mutex::new(BlockQueue::new()));
        let prg  = Arc::new(Mutex::new(prg));

        for _ in 0..size {
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

    pub fn store_block(&mut self, blk_in: Block) {
        self.sndr.send(
            Message::NewJob(
                Box::new(move || {
                    let crtd = SystemTime::now()
                        .duration_since(SystemTime::UNIX_EPOCH)
                        .unwrap().as_secs() as u64;

                    Block {
                        method: blk_in.method,
                        mem:    blk_in.mem,
                        blk_sz: blk_in.blk_sz,
                        chksum: blk_in.chksum,
                        sizeo:  blk_in.sizeo,
                        sizei:  blk_in.sizei,
                        files:  blk_in.files,
                        data:   blk_in.data,
                        id:     blk_in.id,
                        crtd,
                    }
                })
            )
        ).unwrap();
    }
    
    /// Create a new message containing a job consisting of compressing an
    /// input block and returning the compressed block.
    pub fn compress_block(&mut self, blk_in: Block) {
        self.sndr.send(
            Message::NewJob(
                Box::new(move || {
                    let chksum = (&blk_in.data).crc32();
                    let sizei = blk_in.data.len() as u64;

                    let blk_out = 
                    if blk_in.method == Method::Cm {
                        let mut enc = Encoder::new(blk_in.mem as usize, blk_in.data.len());
                        enc.compress_block(&blk_in.data);
                        enc.blk_out
                    }
                    else if blk_in.method == Method::Lzw {
                        lzw::encoder::compress(&blk_in.data, blk_in.mem as usize)
                    }
                    else { blk_in.data };
                    
                    let crtd = SystemTime::now()
                        .duration_since(SystemTime::UNIX_EPOCH)
                        .unwrap().as_secs() as u64;

                    Block {
                        method: blk_in.method,
                        mem:    blk_in.mem,
                        blk_sz: blk_in.blk_sz,
                        chksum,
                        sizeo:  blk_out.len() as u64,
                        sizei, 
                        files:  blk_in.files,
                        data:   blk_out,
                        id:     blk_in.id,
                        crtd,
                    }
                })
            )
        ).unwrap();
    }

    /// Create a new message containing a job consisting of decompressing
    /// an input block and returning the decompressed block.
    pub fn decompress_block(&mut self, blk_in: Block) {
        let len = blk_in.data.len();
        self.sndr.send(
            Message::NewJob(
                Box::new(move || {
                    let blk_out = 
                    if blk_in.method == Method::Cm {
                        let mut dec = Decoder::new(blk_in.data, blk_in.mem as usize);
                        dec.decompress_block(blk_in.sizei as usize)
                    }
                    else if blk_in.method == Method::Lzw {
                        lzw::decoder::decompress(&blk_in.data, blk_in.mem as usize)
                    }
                    else { blk_in.data };
                    
                    let chksum = (&blk_out).crc32();
                    if chksum != blk_in.chksum {
                        println!("Incorrect Checksum: Block {}", blk_in.id);
                    }
                    
                    Block {
                        method: blk_in.method,
                        mem:    blk_in.mem,
                        blk_sz: blk_in.blk_sz,
                        chksum,
                        sizeo:  blk_out.len() as u64,
                        sizei:  len as u64,
                        files:  blk_in.files,
                        data:   blk_out,
                        id:     blk_in.id,
                        crtd:   0,
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
                    let blk = job();
                    { prg.lock().unwrap().update(&blk); }
                    bq.lock().unwrap().blocks.push(blk);
                }
                Message::Terminate => { break; }
            }
        });
        Thread { handle: Some(handle) }
    }
}

