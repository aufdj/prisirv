use std::{
    path::{Path, PathBuf},
    fs::File,
    io::{Write, BufReader},
};

use crate::{
    progress::Progress,
    metadata::{Metadata, FileData},
    config::Config,
    threads::ThreadPool,
    block::Block,
    buffered_io::{
        BufferedRead, BufferedWrite, file_len, 
        new_input_file, new_output_file, new_dir_checked,
    },
    formatting::{
        fmt_file_out_ns_extract,
        fmt_nested_dir_ns_extract,
    },
    error,
};

/// An Extractor extracts non-solid archives.
pub struct Extractor {
    cfg: Config,
    prg: Progress,
}
impl Extractor {
    /// Create a new extractor.
    pub fn new(cfg: Config) -> Extractor {
        let prg = Progress::new(&cfg);
        
        Extractor { 
            cfg, prg 
        }
    }

    /// Extract all files in an archive.
    pub fn extract_archive(&mut self) {
        new_dir_checked(&self.cfg.dir_out, self.cfg.clbr);
            
        let (files, dirs): (Vec<FileData>, Vec<FileData>) = 
            self.cfg.inputs.clone().into_iter()
            .partition(|f| f.path.is_file());

        let mut dir_out = self.cfg.dir_out.clone();

        for file in files.iter() {
            self.prg.print_file_name(&file.path);
            self.decompress_file(&file.path, &dir_out);
        }
        for dir in dirs.iter() {
            self.decompress_dir(&dir.path, &mut dir_out, true);      
        }
    }

    /// Decompress a single file.
    pub fn decompress_file(&mut self, file_in_path: &Path, dir_out: &str) {
        let mut file_in = new_input_file(4096, file_in_path);
        let mta: Metadata = self.read_metadata(&mut file_in);

        self.prg.get_file_size(file_in_path);

        let file_out_path = fmt_file_out_ns_extract(&mta.get_ext(), dir_out, file_in_path);
        let mut file_out = new_output_file(4096, &file_out_path);
        
        let mut blks_wrtn: u64 = 0;
        let mut tp = ThreadPool::new(self.cfg.threads, mta.mem, self.prg);
        let mut blk = Block::new(mta.blk_sz);

        for _ in 0..mta.blk_c {
            blk.read_from(&mut file_in);
            tp.decompress_block(blk.clone());
            blk.next();
        }

        while blks_wrtn != mta.blk_c {
            if let Some(blk) = tp.bq.lock().unwrap().try_get_block() {
                file_out.write_all(&blk.data).unwrap();
                blks_wrtn += 1;
            }
        }
    
        file_out.flush_buffer();
        self.prg.print_file_stats(file_len(&file_out_path));
    }

    /// Decompress all files in a directory.
    pub fn decompress_dir(&mut self, dir_in: &Path, dir_out: &mut String, root: bool) {
        let mut dir_out = fmt_nested_dir_ns_extract(dir_out, dir_in, root);
        new_dir_checked(&dir_out, self.cfg.clbr);

        // Sort files and directories
        let (files, dirs): (Vec<PathBuf>, Vec<PathBuf>) =
            dir_in.read_dir().unwrap()
            .map(|d| d.unwrap().path())
            .partition(|f| f.is_file());

        // Decompress files first, then directories
        for file in files.iter() {
            self.prg.print_file_name(file);
            self.decompress_file(file, &dir_out);
        }
        for dir in dirs.iter() {
            self.decompress_dir(dir, &mut dir_out, false); 
        }
    }

    /// Read 56 byte header.
    fn read_metadata(&mut self, file_in: &mut BufReader<File>) -> Metadata {
        let mut mta = Metadata::new();
        mta.mem     = file_in.read_u64();
        mta.mgc     = file_in.read_u32();
        mta.ext     = file_in.read_u64();
        mta.blk_sz  = file_in.read_u64() as usize;
        mta.blk_c   = file_in.read_u64();
        self.verify_magic_number(mta.mgc);
        mta
    }

    /// Check for a valid magic number.
    /// * Non-solid archives - 'prsv'
    /// * Solid archives     - 'PRSV'
    fn verify_magic_number(&self, mgc: u32) {
        match mgc {
            0x7673_7270 => {},
            0x5653_5250 => error::found_solid_archive(),
            _ => error::no_prisirv_archive(),
        }
    }
}
