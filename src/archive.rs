use std::{
    path::{Path, PathBuf},
    time::Instant,
    io::{Seek, SeekFrom},
    sync::mpsc,
};

use crate::{
    Arch,
    file_len, 
    metadata::Metadata,
    encoder::Encoder,
    decoder::Decoder,
    parse_args::Config,
    buffered_io::{
        BufferedRead, BufferedWrite, BufferState,
        new_input_file, new_output_file, new_dir_checked,
    },
    formatting::{
        fmt_file_out_ns_archive,
        fmt_file_out_ns_extract,
        fmt_nested_dir_ns_archive,
        fmt_nested_dir_ns_extract,
    },
    threads::{
        self, ThreadPool, BlockQueue, 
    }
};

/// Check for a valid magic number.
/// Non-solid archives - 'prsv'
/// Solid archives     - 'PRSV'
fn verify_magic_number(mgc: usize, arch: Arch) {
    match (arch, mgc) {
        (Arch::Solid, 0x5653_5250) => {},
        (Arch::Solid, 0x7673_7270) => {
            println!();
            println!("Expected solid archive, found non-solid archive.");
            std::process::exit(0);
        },
        (Arch::NonSolid, 0x7673_7270) => {},
        (Arch::NonSolid, 0x5653_5250) => {
            println!();
            println!("Expected non-solid archive, found solid archive.");
            std::process::exit(0);
        }
        (_, _) => {
            println!("Not a prisirv archive.");
            std::process::exit(0);
        }
    }
}

/*
use std::thread;
use std::sync::mpsc;
use std::sync::mpsc::TryRecvError;
use std::sync::mpsc::{Sender, Receiver};
use std::thread::JoinHandle;
use std::sync::Arc;
use std::sync::Mutex;

enum Message {
    NewJob(Job),
    Terminate,
}

type Job = Box<dyn FnOnce() + Send + 'static>;

pub struct ThreadPool {
    workers: Vec<Worker>,
    sndr: Sender<Message>,
}

impl ThreadPool {
    pub fn new(size: usize) -> ThreadPool {
        assert!(size > 0);

        let (sndr, rcvr) = mpsc::channel();

        let rcvr = Arc::new(Mutex::new(rcvr));

        let mut workers = Vec::with_capacity(size);

        for id in 0..size {
            workers.push(Worker::new(id, Arc::clone(&rcvr)));
        }

        ThreadPool { workers, sndr }
    }
    pub fn execute<F>(&self, f: F)
    where F: FnOnce() + Send + 'static {
        let message = Message::NewJob(Box::new(f));

        self.sndr.send(message).unwrap();
    }
}
impl Drop for ThreadPool {
    fn drop(&mut self) {
        println!("Sending terminate message to all workers.");

        for _ in &self.workers {
            self.sndr.send(Message::Terminate).unwrap();
        }

        println!("Shutting down all workers.");

        for worker in &mut self.workers {
            println!("Shutting down worker {}", worker.id);

            if let Some(thread) = worker.thread.take() {
                thread.join().unwrap();
            }
        }
    }
}
struct Worker {
    thread: Option<JoinHandle<()>>,
    id: usize,
}
impl Worker {
    fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Message>>>) -> Worker {
        let thread = thread::spawn(move || loop {
            let message = receiver.lock().unwrap().recv().unwrap();

            match message {
                Message::NewJob(job) => {
                    println!("Worker {} got a job; executing.", id);

                    job();
                }
                Message::Terminate => {
                    println!("Worker {} was told to terminate.", id);

                    break;
                }
            }
        });

        Worker { thread: Some(thread), id }
    }
}

struct BlockQueue {
    blocks: Vec<(Vec<u8>, usize)>, // Blocks to be output
    next_out: usize, // Next block to be output
}
impl BlockQueue {
    fn new() -> BlockQueue {
        BlockQueue {
            blocks: Vec::new(),
            next_out: 0,
        }
    }
    fn try_write_block_enc(&mut self, mta: &mut Metadata, enc: &mut Encoder) {
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
        self.next_out += len - self.blocks.len();
    }
    fn try_write_block_dec(&mut self, file_out: &mut BufWriter<File>) {
        let len = self.blocks.len();
        let next_out = self.next_out;

        self.blocks.retain(|block|
            if block.1 == next_out {
                println!("writing block");
                for byte in block.0.iter() {
                    file_out.write_byte(*byte);
                }
                false
            }
            else { true } 
        );
        self.next_out += len - self.blocks.len();
    }
    fn try_get_block(&mut self, block: Result<(Vec<u8>, usize), TryRecvError>) {
        match block {
            Ok(block) => self.blocks.push(block),
            Err(_) => {},
        }
    }
}

fn compress_block(sndr: Sender<(Vec<u8>, usize)>, input: &[u8], index: usize, mem: usize, blk_sz: usize) {
    let mut enc = SubEncoder::new(mem, blk_sz);
    enc.compress_block(&input);
    sndr.send((enc.out, index)).unwrap();
}
fn decompress_block(sndr: Sender<(Vec<u8>, usize)>, block_in: Vec<u8>, mem: usize, index: usize, blk_sz: usize) {
    let mut dec = SubDecoder::new(block_in, mem);
    dec.init_x();
    sndr.send((dec.decompress_block(blk_sz), index)).unwrap();
}
*/

/// Archiver ============================================================================
///
/// An archiver creates non-solid archives. A non-solid archive is an archive containing
/// independently compressed files. Non-solid archiving typically results in worse 
/// compression ratios than solid archiving, but allows for extracting individual files.
///
/// =====================================================================================
pub struct Archiver {
    cfg:    Config,
    files:  Vec<PathBuf>, // Keep list of files already compressed to prevent accidental clobbering
}
impl Archiver {
    pub fn new(cfg: Config) -> Archiver {
        Archiver {
            cfg,
            files: Vec::with_capacity(32),
        }
    }
    pub fn compress_file(&mut self, file_in_path: &Path, dir_out: &str) -> u64 {
        let mut mta: Metadata = Metadata::new();
        mta.blk_sz = self.cfg.blk_sz;

        let file_out_path = fmt_file_out_ns_archive(dir_out, file_in_path, self.cfg.clbr, &self.files);
        if self.cfg.clbr { self.files.push(file_out_path.clone()); }
        
        // Create input file with buffer = block size
        let mut file_in = new_input_file(mta.blk_sz, file_in_path);
        let mut enc = Encoder::new(new_output_file(4096, &file_out_path), &self.cfg);

        // Set metadata extension field
        mta.set_ext(file_in_path);

        if self.cfg.threads == 1 {
            loop {
                if file_in.fill_buffer() == BufferState::Empty { break; }
                mta.fblk_sz = file_in.buffer().len();
                enc.compress_block(file_in.buffer());
                mta.blk_c += 1;
            }
        }
        else {
            let (sndr, rcvr) = mpsc::channel();
            let mem = self.cfg.mem;
            let blk_sz = self.cfg.blk_sz;
            let mut index = 0;
            let mut bq = BlockQueue::new();
            let tp = ThreadPool::new(self.cfg.threads);

            loop {
                if file_in.fill_buffer() == BufferState::Empty {
                    break; 
                }
                mta.fblk_sz = file_in.buffer().len();
                let input = file_in.buffer().to_vec();
                let sndrn = sndr.clone();

                tp.execute(move || {
                    threads::compress_block(sndrn, &input, index, mem, blk_sz);
                });
                index += 1;
                mta.blk_c += 1;

                bq.try_get_block(rcvr.try_recv());
                bq.try_write_block_enc(&mut mta, &mut enc);
            }
            std::mem::drop(tp);
            std::mem::drop(sndr);
            for rcv in rcvr.try_iter() {
                bq.try_get_block(Ok(rcv));
            }

            while !bq.blocks.is_empty() {
                bq.try_write_block_enc(&mut mta, &mut enc);
            }
        }

        enc.flush();
        if self.cfg.threads == 1 {}
        else { self.write_footer(&mut enc, &mut mta); }
        enc.write_header(&mta, Arch::NonSolid);
        file_len(&file_out_path)
    }
    
    pub fn compress_dir(&mut self, dir_in: &Path, dir_out: &mut String) {
        let mut dir_out = fmt_nested_dir_ns_archive(dir_out, dir_in);
        new_dir_checked(&dir_out, self.cfg.clbr);

        // Sort files and directories
        let (files, dirs): (Vec<PathBuf>, Vec<PathBuf>) = 
            dir_in.read_dir().unwrap()
            .map(|d| d.unwrap().path())
            .partition(|f| f.is_file());

        // Compress files first, then directories
        for file_in in files.iter() {
            let time = Instant::now();
            if !self.cfg.quiet { println!("Compressing {}", file_in.display()); }
            let file_in_size  = file_len(file_in); 
            let file_out_size = self.compress_file(file_in, &dir_out);
            if !self.cfg.quiet { println!("{} bytes -> {} bytes in {:.2?}\n", 
                file_in_size, file_out_size, time.elapsed()); }
        }
        for dir_in in dirs.iter() {
            self.compress_dir(dir_in, &mut dir_out);
        }
    } 
    // Write compressed block sizes in footer
    fn write_footer(&mut self, enc: &mut Encoder, mta: &mut Metadata) {
        // Get index to end of file metadata
        mta.f_ptr =
            enc.file_out.stream_position()
            .unwrap() as usize;

        for blk_sz in mta.enc_blk_szs.iter() {
            enc.file_out.write_usize(*blk_sz);
        }
    } 
}

/// An Extractor extracts non-solid archives.
pub struct Extractor {
    cfg: Config,
}
impl Extractor {
    pub fn new(cfg: Config) -> Extractor {
        Extractor {
            cfg,
        }
    }
    pub fn decompress_file(&mut self, file_in_path: &Path, dir_out: &str) -> u64 {
        let mut dec = Decoder::new(new_input_file(4096, file_in_path));
        let mut mta: Metadata = dec.read_header(Arch::NonSolid);
        if self.cfg.threads == 1 {}
        else { self.read_footer(&mut dec, &mut mta); }

        verify_magic_number(mta.mgc, Arch::NonSolid);

        let file_out_path = fmt_file_out_ns_extract(&mta.get_ext(), dir_out, file_in_path);
        let mut file_out = new_output_file(4096, &file_out_path);
        
        if self.cfg.threads == 1 {
            // Call after reading header
            dec.init_x();

            // Decompress full blocks
            for _ in 0..(mta.blk_c - 1) {
                let block = dec.decompress_block(mta.blk_sz);
                for byte in block.iter() {
                    file_out.write_byte(*byte);
                }
            }
            // Decompress final variable size block
            let block = dec.decompress_block(mta.fblk_sz);
            for byte in block.iter() {
                file_out.write_byte(*byte);
            }
        }
        else {
            let blk_sz = mta.blk_sz;
            let fblk_sz = mta.fblk_sz;
            let mem = dec.mem;
            let (sndr, rcvr) = mpsc::channel();
            let mut index = 0;
            let mut bq = BlockQueue::new();
            let tp = ThreadPool::new(self.cfg.threads);
            
            for _ in 0..(mta.blk_c-1) {
                // Read and decompress compressed blocks
                let mut block_in = Vec::with_capacity(blk_sz);
                for _ in 0..mta.enc_blk_szs[index] {
                    block_in.push(dec.file_in.read_byte());
                }
                let sndrn = sndr.clone();

                tp.execute(move || {
                    threads::decompress_block(sndrn, block_in, mem, index, blk_sz);
                });
                index += 1;
                bq.try_get_block(rcvr.try_recv());
                bq.try_write_block_dec(&mut file_out);
            }

            // Read and decompress final compressed block
            let mut block_in = Vec::with_capacity(blk_sz);
            for _ in 0..mta.enc_blk_szs[index] {
                block_in.push(dec.file_in.read_byte());
            }
            let sndrn = sndr.clone();
            tp.execute(move || {
                threads::decompress_block(sndrn, block_in, mem, index, fblk_sz);
            });
            std::mem::drop(tp);
            std::mem::drop(sndr);

            for rcv in rcvr.try_iter() {
                bq.try_get_block(Ok(rcv));
            }
            while !bq.blocks.is_empty() {
                bq.try_write_block_dec(&mut file_out);
            }
        } 
        file_out.flush_buffer();
        file_len(&file_out_path)
    }
    pub fn decompress_dir(&mut self, dir_in: &Path, dir_out: &mut String, root: bool) {
        let mut dir_out = fmt_nested_dir_ns_extract(dir_out, dir_in, root);
        new_dir_checked(&dir_out, self.cfg.clbr);

        // Sort files and directories
        let (files, dirs): (Vec<PathBuf>, Vec<PathBuf>) =
            dir_in.read_dir().unwrap()
            .map(|d| d.unwrap().path())
            .partition(|f| f.is_file());

        // Decompress files first, then directories
        for file_in in files.iter() {
            let time = Instant::now();
            if !self.cfg.quiet { println!("Decompressing {}", file_in.display()); }
            let file_in_size  = file_len(file_in);
            let file_out_size = self.decompress_file(file_in, &dir_out);
            if !self.cfg.quiet { println!("{} bytes -> {} bytes in {:.2?}\n",
                file_in_size, file_out_size, time.elapsed()); }
        }
        for dir_in in dirs.iter() {
            self.decompress_dir(dir_in, &mut dir_out, false); 
        }
    }
    fn read_footer(&mut self, dec: &mut Decoder, mta: &mut Metadata) {
        // Seek to end of file metadata
        dec.file_in.seek(SeekFrom::Start(mta.f_ptr as u64)).unwrap();

        for _ in 0..mta.blk_c {
            mta.enc_blk_szs.push(dec.file_in.read_usize());
        }

        // Seek back to beginning of compressed data
        #[cfg(target_pointer_width = "64")]
        dec.file_in.seek(SeekFrom::Start(56)).unwrap();

        #[cfg(target_pointer_width = "32")]
        dec.file_in.seek(SeekFrom::Start(28)).unwrap();
    }
}
