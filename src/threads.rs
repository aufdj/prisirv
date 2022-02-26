use std::{
    thread::{self, JoinHandle},
    sync::{
        mpsc::{self, Sender, Receiver},
        Arc, Mutex,
    },
    fs::File,
    io::{Seek, BufWriter},
    path::{Path, PathBuf},
};
use crate::{
    encoder::{Encoder, SubEncoder},
    decoder::SubDecoder,
    metadata::Metadata,
    buffered_io::{BufferedWrite, new_output_file, file_len},
    progress::Progress,
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
    pub fn new(size: usize, mem: usize, prg: Progress) -> ThreadPool {
        let (sndr, rcvr) = mpsc::channel();
        let mut workers = Vec::with_capacity(size);

        let rcvr = Arc::new(Mutex::new(rcvr));
        let bq   = Arc::new(Mutex::new(BlockQueue::new()));
        let prg  = Arc::new(Mutex::new(prg));

        for _ in 0..size {
            workers.push(Worker::new(Arc::clone(&rcvr), Arc::clone(&bq), Arc::clone(&prg)));
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
                    (enc.out, index)
                })
            )
        ).unwrap();   
    }
    pub fn decompress_block(&mut self, block: Vec<u8>, index: usize, blk_sz: usize) {
        let mem = self.mem;
        self.sndr.send(
            Message::NewJob(
                Box::new(move || {
                    let mut dec = SubDecoder::new(block, mem);
                    dec.init_x();
                    let block_out = dec.decompress_block(blk_sz);
                    (block_out, index)
                })
            )
        ).unwrap();   
    }
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

type SharedBlockQueue = Arc<Mutex<BlockQueue>>;
type SharedReceiver   = Arc<Mutex<Receiver<Message>>>;
type SharedProgress   = Arc<Mutex<Progress>>;

struct Worker {
    thread: Option<JoinHandle<()>>,
}
impl Worker {
    fn new(receiver: SharedReceiver, bq: SharedBlockQueue, prg: SharedProgress) -> Worker {
        let thread = thread::spawn(move || loop {
            let message = receiver.lock().unwrap().recv().unwrap();

            match message {
                Message::NewJob(job) => { 
                    let (block, index) = job();
                    {
                        let prg_guard = prg.lock().unwrap();
                        match prg_guard {
                            mut prg => prg.update(),
                        }
                    }
                    let queue_guard = bq.lock().unwrap();
                    match queue_guard {
                        mut queue => {
                            println!("pushing block");
                            queue.blocks.push((block, index));
                            println!("finished pushing block");
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
    next_out:   usize, // Next block to be output
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
    pub fn try_get_block(&mut self) -> Option<Vec<u8>> {
        let mut index = None;
        let mut blk_out = Vec::new();

        for (blk_i, mut blk) in self.blocks.clone().into_iter().enumerate() {
            if blk.1 == self.next_out {
                self.next_out += 1;
                blk_out.append(&mut blk.0);
                index = Some(blk_i);
            }
        }
        match index {
            Some(i) =>  {
                self.blocks.swap_remove(i);
            }
            None => {},
        }
        if blk_out.is_empty() {
            return None;
        }
        else {
            return Some(blk_out);
        }
    }
}