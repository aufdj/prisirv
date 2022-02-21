use std::{
    path::{Path, PathBuf},
    time::Instant,
    io::{Seek, SeekFrom},
    sync::mpsc,
    mem::drop,
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
    },
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

/// An archiver creates non-solid archives. A non-solid archive is an archive containing
/// independently compressed files. Non-solid archiving typically results in worse 
/// compression ratios than solid archiving, but allows for extracting individual files.
pub struct Archiver {
    cfg:    Config,
    files:  Vec<PathBuf>, // Keep list of files already compressed to prevent accidental clobbering
}
impl Archiver {
    /// Create a new Archiver.
    pub fn new(cfg: Config) -> Archiver {
        Archiver {
            cfg, 
            files: Vec::with_capacity(32),
        }
    }

    /// Compresses a single file. If single threaded, a single encoder is used to 
    /// compress. If multithreaded, the main thread parses the file into blocks and 
    /// each block is compressed by a seperate encoder, so files compressed 
    /// with a single thread can't be decompressed with multiple threads, or vice versa.
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
                println!("Compressed block {}", mta.blk_c);
                mta.blk_c += 1;
            }
        }
        else {
            let quiet = self.cfg.quiet;
            let mut index = 0;
            let mut blocks_written = 0;
            let mut tp = ThreadPool::new(self.cfg.threads, self.cfg.mem);

            loop {
                if file_in.fill_buffer() == BufferState::Empty { break; }
                mta.fblk_sz = file_in.buffer().len();

                tp.compress_block(file_in.buffer().to_vec(), index, self.cfg.blk_sz);
                index += 1;
                mta.blk_c += 1;
                //std::thread::sleep(std::time::Duration::from_millis(2000));
                //tp.bq.lock().unwrap().try_write_block_enc(&mut mta, &mut enc);
            }
            while blocks_written != mta.blk_c {
                blocks_written += tp.bq.lock().unwrap().try_write_block_enc(&mut mta, &mut enc);
            }   
            drop(tp);
        }

        enc.flush();
        if self.cfg.threads == 1 {}
        else { self.write_footer(&mut enc, &mut mta); }
        enc.write_header(&mta, Arch::NonSolid);
        file_len(&file_out_path)
    }
    
    /// Compress any nested directories.
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

    /// If compression is multithreaded, write compressed block sizes to archive
    /// so that compressed blocks can be parsed ahead of time during decompression.
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
            for b in 0..(mta.blk_c - 1) {
                let block = dec.decompress_block(mta.blk_sz);
                println!("Decompressed block {}", b);
                for byte in block.iter() {
                    file_out.write_byte(*byte);
                }
            }
            // Decompress final variable size block
            let block = dec.decompress_block(mta.fblk_sz);
            println!("Decompressed block {}", mta.blk_c-1);
            for byte in block.iter() {
                file_out.write_byte(*byte);
            }
        }
        else {
            let quiet = self.cfg.quiet;
            let mut index = 0;
            let mut blocks_written = 0;
            let blk_c = mta.enc_blk_szs.len();
            let mut tp = ThreadPool::new(self.cfg.threads, dec.mem);
            
            for _ in 0..(mta.blk_c-1) {
                // Read and decompress compressed blocks
                let mut block_in = Vec::with_capacity(mta.blk_sz);
                for _ in 0..mta.enc_blk_szs[index] {
                    block_in.push(dec.file_in.read_byte());
                }
                tp.decompress_block(block_in, index, mta.blk_sz);
                index += 1;
            }

            // Read and decompress final compressed block
            let mut block_in = Vec::with_capacity(mta.blk_sz);
            for _ in 0..mta.enc_blk_szs[index] {
                block_in.push(dec.file_in.read_byte());
            }
            tp.decompress_block(block_in, index, mta.fblk_sz);

            while blocks_written != blk_c {
                blocks_written += tp.bq.lock().unwrap().try_write_block_dec(&mut file_out);
            }

            drop(tp);
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
