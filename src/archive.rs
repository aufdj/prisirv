use std::{
    path::{Path, PathBuf},
    io::{Seek, SeekFrom},
    fs::File,
    io::{BufReader, BufWriter},
    process::exit,
};

use crate::{
    Mode,
    progress::Progress,
    metadata::Metadata,
    parse_args::Config,
    threads::ThreadPool,
    buffered_io::{
        BufferedRead, BufferedWrite, BufferState, file_len, 
        new_input_file, new_output_file, new_dir_checked,
    },
    formatting::{
        fmt_file_out_ns_archive,
        fmt_file_out_ns_extract,
        fmt_nested_dir_ns_archive,
        fmt_nested_dir_ns_extract,
    },
};


/// An archiver creates non-solid archives. A non-solid archive is an archive containing
/// independently compressed files. Non-solid archiving typically results in worse 
/// compression ratios than solid archiving, but allows for extracting individual files.
pub struct Archiver {
    cfg:    Config,
    prg:    Progress,
    files:  Vec<PathBuf>, // Keep list of files already compressed to prevent accidental clobbering
}
impl Archiver {
    /// Create a new Archiver.
    pub fn new(cfg: Config) -> Archiver {
        let prg = Progress::new(&cfg, Mode::Compress);
        Archiver {
            cfg, prg, files: Vec::with_capacity(32),
        }
    }
    pub fn create_archive(&mut self) {
        new_dir_checked(&self.cfg.dir_out, self.cfg.clbr);

        let (files, dirs): (Vec<PathBuf>, Vec<PathBuf>) = 
            self.cfg.inputs.clone().into_iter().partition(|f| f.is_file());

        let mut dir_out = self.cfg.dir_out.clone();

        for file_in in files.iter() {
            if !self.cfg.quiet { println!("Compressing {}", file_in.display()); }
            self.compress_file(file_in, &dir_out);
        }
        for dir_in in dirs.iter() {
            self.compress_dir(dir_in, &mut dir_out);      
        }
    }
    /// Compresses a single file. The main thread parses the file into 
    /// blocks and each block is compressed by a seperate encoder.
    pub fn compress_file(&mut self, file_in_path: &Path, dir_out: &str) {
        self.prg.get_input_size_enc(file_in_path);

        let mut mta: Metadata = Metadata::new();
        mta.blk_sz = self.cfg.blk_sz;
        mta.mem = self.cfg.mem;

        let file_out_path = fmt_file_out_ns_archive(dir_out, file_in_path, self.cfg.clbr, &self.files);
        if self.cfg.clbr { self.files.push(file_out_path.clone()); }
        
        // Create input file with buffer = block size
        let mut file_in  = new_input_file(mta.blk_sz, file_in_path);

        // Create output file and write metadata placeholder
        let mut file_out = new_output_file(4096, &file_out_path);
        for _ in 0..7 { file_out.write_u64(0); }

        // Set metadata extension field
        mta.set_ext(file_in_path);
        
        let mut index: u64 = 0;
        let mut blks_wrtn: u64 = 0;
        let mut tp = ThreadPool::new(self.cfg.threads, self.cfg.mem, self.prg);

        while file_in.fill_buffer() == BufferState::NotEmpty {
            mta.fblk_sz = file_in.buffer().len();
            tp.compress_block(file_in.buffer().to_vec(), index, self.cfg.blk_sz);
            index += 1;
            mta.blk_c += 1;
        }
        while blks_wrtn != mta.blk_c {
            blks_wrtn += tp.bq.lock().unwrap().try_write_block_enc(&mut mta, &mut file_out);
        }   

        self.write_footer(&mut file_out, &mut mta);
        self.write_header(&mut file_out, &mta);

        self.prg.print_file_stats(file_len(&file_out_path));
    }
    
    /// Compresses every file in a directory.
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
            if !self.cfg.quiet { println!("Compressing {}", file_in.display()); } 
            self.compress_file(file_in, &dir_out);
        }
        for dir_in in dirs.iter() {
            self.compress_dir(dir_in, &mut dir_out);
        }
    } 

    /// Rewind to beginning of file and write header.
    fn write_header(&mut self, file_out: &mut BufWriter<File>, mta: &Metadata) {
        file_out.rewind().unwrap();
        file_out.write_u64(mta.mem);
        file_out.write_u64(mta.mgc);
        file_out.write_u64(mta.ext);
        file_out.write_u64(mta.fblk_sz as u64);
        file_out.write_u64(mta.blk_sz as u64);
        file_out.write_u64(mta.blk_c);
        file_out.write_u64(mta.f_ptr);
    }

    /// Write compressed block sizes to archive so that compressed 
    /// blocks can be parsed ahead of time during decompression.
    fn write_footer(&mut self, file_out: &mut BufWriter<File>, mta: &mut Metadata) {
        // Get index to end of file metadata
        mta.f_ptr = file_out.stream_position().unwrap();

        for blk_sz in mta.enc_blk_szs.iter() {
            file_out.write_u64(*blk_sz);
        }
    } 
}

/// An Extractor extracts non-solid archives.
pub struct Extractor {
    cfg: Config,
    prg: Progress,
}
impl Extractor {
    /// Create a new extractor.
    pub fn new(cfg: Config) -> Extractor {
        let prg = Progress::new(&cfg, Mode::Decompress);
        Extractor {
            cfg, prg,
        }
    }
    pub fn extract_archive(&mut self) {
        new_dir_checked(&self.cfg.dir_out, self.cfg.clbr);
            
        let (files, dirs): (Vec<PathBuf>, Vec<PathBuf>) = 
            self.cfg.inputs.clone().into_iter().partition(|f| f.is_file());

        let mut dir_out = self.cfg.dir_out.clone();

        for file_in in files.iter() {
            if !self.cfg.quiet { println!("Decompressing {}", file_in.display()); } 
            self.decompress_file(file_in, &dir_out);
        }
        for dir_in in dirs.iter() {
            self.decompress_dir(dir_in, &mut dir_out, true);      
        }
    }

    pub fn decompress_file(&mut self, file_in_path: &Path, dir_out: &str) {
        let mut file_in = new_input_file(4096, file_in_path);
        let mut mta: Metadata = self.read_header(&mut file_in);

        self.read_footer(&mut file_in, &mut mta);
        self.prg.get_input_size_dec(file_in_path, mta.enc_blk_szs.len());

        self.verify_magic_number(mta.mgc);

        let file_out_path = fmt_file_out_ns_extract(&mta.get_ext(), dir_out, file_in_path);
        let mut file_out = new_output_file(4096, &file_out_path);
        
        let mut index: u64 = 0;
        let mut blks_wrtn: u64 = 0;
        let mut tp = ThreadPool::new(self.cfg.threads, mta.mem, self.prg);
        
        for _ in 0..(mta.blk_c-1) {
            // Read and decompress compressed blocks
            let mut block_in = Vec::with_capacity(mta.blk_sz);
            for _ in 0..mta.enc_blk_szs[index as usize] {
                block_in.push(file_in.read_byte());
            }
            tp.decompress_block(block_in, index, mta.blk_sz);
            index += 1;
        }

        // Read and decompress final compressed block
        let mut block_in = Vec::with_capacity(mta.blk_sz);
        for _ in 0..mta.enc_blk_szs[index as usize] {
            block_in.push(file_in.read_byte());
        }
        tp.decompress_block(block_in, index, mta.fblk_sz);

        while blks_wrtn != mta.blk_c {
            blks_wrtn += tp.bq.lock().unwrap().try_write_block_dec(&mut file_out);
        }
    
        file_out.flush_buffer();
        self.prg.print_file_stats(file_len(&file_out_path));
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
            if !self.cfg.quiet { println!("Decompressing {}", file_in.display()); }
            self.decompress_file(file_in, &dir_out);
        }
        for dir_in in dirs.iter() {
            self.decompress_dir(dir_in, &mut dir_out, false); 
        }
    }

    fn read_header(&mut self, file_in: &mut BufReader<File>) -> Metadata {
        let mut mta: Metadata = Metadata::new();
        mta.mem =     file_in.read_u64();
        mta.mgc =     file_in.read_u64();
        mta.ext =     file_in.read_u64();
        mta.fblk_sz = file_in.read_u64() as usize;
        mta.blk_sz =  file_in.read_u64() as usize;
        mta.blk_c =   file_in.read_u64();
        mta.f_ptr =   file_in.read_u64();
        mta
    }

    fn read_footer(&mut self, file_in: &mut BufReader<File>, mta: &mut Metadata) {
        // Seek to end of file metadata
        file_in.seek(SeekFrom::Start(mta.f_ptr)).unwrap();

        for _ in 0..mta.blk_c {
            mta.enc_blk_szs.push(file_in.read_u64());
        }

        // Seek back to beginning of compressed data
        file_in.seek(SeekFrom::Start(56)).unwrap();
    }

    /// Check for a valid magic number.
    /// * Non-solid archives - 'prsv'
    /// * Solid archives     - 'PRSV'
    fn verify_magic_number(&self, mgc: u64) {
        match mgc {
            0x7673_7270 => {},
            0x5653_5250 => {
                println!();
                println!("Expected non-solid archive, found solid archive.");
                exit(0);
            }
            _ => {
                println!("Not a prisirv archive.");
                exit(0);
            }
        }
    }
}

