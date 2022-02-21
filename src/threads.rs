use std::{
    thread::{self, JoinHandle},
    sync::{
        mpsc::{self, TryRecvError, Sender, Receiver},
        Arc, Mutex,
    },
    fs::File,
    io::BufWriter,
};
use crate::{
    encoder::{Encoder, SubEncoder},
    decoder::SubDecoder,
    metadata::Metadata,
    buffered_io::BufferedWrite,
};

pub enum Message {
    NewJob(Job),
    Terminate,
}

type Job = Box<dyn FnOnce() -> (Vec<u8>, usize) + Send + 'static>;

pub struct ThreadPool {
    workers: Vec<Worker>,
    sndr: Sender<Message>,
    mem: usize,
    pub bq: Arc<Mutex<BlockQueue>>,
}

impl ThreadPool {
    pub fn new(size: usize, mem: usize) -> ThreadPool {
        let (sndr, rcvr) = mpsc::channel();
        let rcvr = Arc::new(Mutex::new(rcvr));
        let mut workers = Vec::with_capacity(size);
        let bq = Arc::new(Mutex::new(BlockQueue::new()));

        for _ in 0..size {
            workers.push(Worker::new(Arc::clone(&rcvr), Arc::clone(&bq)));
        }
        ThreadPool { workers, sndr, mem, bq }
    }
    pub fn compress_block(&mut self, block: Vec<u8>, index: usize, blk_sz: usize) {
        let mem = self.mem;
        self.sndr.send(
            Message::NewJob(
                Box::new(move || {
                    let mut enc = SubEncoder::new(mem, blk_sz);
                    enc.compress_block(&block);
                    //if !quiet { println!("Compressed block {}", index); }
                    (enc.out, index)
                })
            )
        );   
    }
    pub fn decompress_block(&mut self, block: Vec<u8>, index: usize, blk_sz: usize) {
        let mem = self.mem;
        self.sndr.send(
            Message::NewJob(
                Box::new(move || {
                    let mut dec = SubDecoder::new(block, mem);
                    dec.init_x();
                    let block_out = dec.decompress_block(blk_sz);
                    //if !quiet { println!("Compressed block {}", index); }
                    (block_out, index)
                })
            )
        );   
    }
    /*
    pub fn execute<F>(&self, f: F)
    where F: FnOnce() + Send + 'static {
        let message = Message::NewJob(Box::new(f));
        self.sndr.send(message).unwrap();
    }
    */
}
impl Drop for ThreadPool {
    fn drop(&mut self) {
        for _ in &self.workers {
            self.sndr.send(Message::Terminate).unwrap();
        }

        for worker in &mut self.workers {
            if let Some(thread) = worker.thread.take() {
                thread.join().unwrap();
            }
        }
    }
}

struct Worker {
    thread: Option<JoinHandle<()>>,
}
impl Worker {
    fn new(receiver: Arc<Mutex<Receiver<Message>>>, bq: Arc<Mutex<BlockQueue>>) -> Worker {
        let thread = thread::spawn(move || loop {
            let message = receiver.lock().unwrap().recv().unwrap();

            match message {
                Message::NewJob(job) => { 
                    let (block, index) = job();
                    let mutex_guard = bq.lock().unwrap();
                    match mutex_guard {
                        mut queue => {
                            queue.blocks.push((block, index));
                        }
                    }; 
                }
                Message::Terminate => { break; }
            }
        });
        Worker { thread: Some(thread) }
    }
}

pub struct BlockQueue {
    pub blocks: Vec<(Vec<u8>, usize)>, // Blocks to be output
    next_out: usize, // Next block to be output
}
impl BlockQueue {
    pub fn new() -> BlockQueue {
        BlockQueue {
            blocks: Vec::new(),
            next_out: 0,
        }
    }
    pub fn try_write_block_enc(&mut self, mta: &mut Metadata, enc: &mut Encoder) -> usize {
        let len = self.blocks.len();
        let next_out = self.next_out;
        //println!("blocks: {:#?}", self.blocks);

        self.blocks.retain(|block|
            if block.1 == next_out {
                mta.enc_blk_szs.push(block.0.len());
                for byte in block.0.iter() {
                    enc.file_out.write_byte(*byte);
                }
                false
            }
            else { true }
        );
        let blocks_written: usize = (len - self.blocks.len()) as usize;
        self.next_out += blocks_written;
        blocks_written
        
    }
    pub fn try_write_block_dec(&mut self, file_out: &mut BufWriter<File>) -> usize {
        let len = self.blocks.len();
        let next_out = self.next_out;

        self.blocks.retain(|block|
            if block.1 == next_out {
                for byte in block.0.iter() {
                    file_out.write_byte(*byte);
                }
                false
            }
            else { true } 
        );
        let blocks_written: usize = (len - self.blocks.len()) as usize;
        self.next_out += blocks_written;
        blocks_written
    }
}

/*
pub fn compress_block(sndr: Sender<(Vec<u8>, usize)>, input: &[u8], index: usize, mem: usize, blk_sz: usize, quiet: bool) {
    let mut enc = SubEncoder::new(mem, blk_sz);
    enc.compress_block(&input);
    if !quiet { println!("Compressed block {}", index); }
    sndr.send((enc.out, index)).unwrap();
}
pub fn decompress_block(sndr: Sender<(Vec<u8>, usize)>, block_in: Vec<u8>, mem: usize, index: usize, blk_sz: usize, quiet: bool) {
    let mut dec = SubDecoder::new(block_in, mem);
    dec.init_x();
    let block = dec.decompress_block(blk_sz);
    if !quiet { println!("Decompressed block {}", index); }
    sndr.send((block, index)).unwrap();
}
*/