use std::{
    path::{Path, PathBuf},
    io::{Seek, SeekFrom, BufWriter, BufReader},
    fs::File,
};

use crate::{
    Mode,
    metadata::Metadata,
    threads::ThreadPool,
    progress::Progress,
    formatting::fmt_file_out_s_extract,
    config::Config,
    buffered_io::{
        BufferedRead, BufferedWrite, file_len,
        new_input_file, new_output_file, 
        new_dir_checked, 
    },
    error,
};

/// A decompressed block can span multiple files, so a FileWriter is used 
/// to handle swapping files when needed while writing a block.
struct FileWriter {
    file_in_paths:  Box<dyn Iterator<Item = PathBuf>>,
    file_in_lens:   Box<dyn Iterator<Item = u64>>,
    file_out:       BufWriter<File>,
    file_out_pos:   u64,
    file_in_len:    u64,
    dir_out:        String,
}
impl FileWriter {
    fn new(files: Vec<(PathBuf, u64)>, dir_out: &str) -> FileWriter {
        let (paths, lens): (Vec<PathBuf>, Vec<u64>) = files.into_iter().unzip();
        let mut paths = paths.into_iter();
        let mut lens  = lens.into_iter();

        let file_out = next_file(&paths.next().unwrap(), dir_out);
        let file_in_len = lens.next().unwrap();

        FileWriter {
            file_in_paths: Box::new(paths), 
            file_in_lens:  Box::new(lens), 
            file_out, 
            file_in_len,
            file_out_pos: 0,
            dir_out: dir_out.to_string(),
        }
    }
    fn update(&mut self) {
        self.file_out = next_file(&self.file_in_paths.next().unwrap(), &self.dir_out);
        self.file_in_len = self.file_in_lens.next().unwrap();
        self.file_out_pos = 0;
    }
    fn write_byte(&mut self, byte: &u8) {
        if self.file_out_pos == self.file_in_len {
            self.update();
        }
        self.file_out.write_byte(*byte);
        self.file_out_pos += 1;
    }
}

/// A SolidExtractor extracts solid archives.
pub struct SolidExtractor {
    archive:  BufReader<File>,
    pub cfg:  Config,
    prg:      Progress,
}
impl SolidExtractor {
    /// Create a new SolidExtractor.
    pub fn new(cfg: Config) -> SolidExtractor {
        if !cfg.inputs[0].is_file() {
            error::not_solid_archive(&cfg.inputs[0]);
        }

        let archive = new_input_file(4096, &cfg.inputs[0]);
        let prg = Progress::new(&cfg, Mode::Decompress);
        
        SolidExtractor { 
            archive, cfg, prg 
        }
    }

    /// Decompress blocks and parse blocks into files. A block can span 
    /// multiple files.
    pub fn extract_archive(&mut self) {
        let mta: Metadata = self.read_metadata();

        self.prg.get_archive_size_dec(&self.cfg.inputs, mta.blk_c);
        new_dir_checked(&self.cfg.dir_out, self.cfg.clbr);

        let mut tp = ThreadPool::new(self.cfg.threads, mta.mem, self.prg);
        let mut index: u64 = 0;
        
        let mut blk_in = Vec::with_capacity(mta.blk_sz);
        
        // Full blocks
        for _ in 0..mta.blk_c-1 {
            for _ in 0..mta.enc_blk_szs[index as usize] {
                blk_in.push(self.archive.read_byte());
            }
            tp.decompress_block(blk_in.clone(), index, mta.blk_sz);
            blk_in.clear();
            index += 1;
        }

        // Final block
        for _ in 0..mta.enc_blk_szs[index as usize] {
            blk_in.push(self.archive.read_byte());
        }
        tp.decompress_block(blk_in, index, mta.fblk_sz);

        let mut fw = FileWriter::new(mta.files, &self.cfg.dir_out);

        let mut blks_wrtn: u64 = 0;
        let mut blk_out = Vec::new();
        
        // Write blocks to output 
        while blks_wrtn != mta.blk_c {
            tp.bq.lock().unwrap().try_get_block(&mut blk_out);
            if !blk_out.is_empty() {
                for byte in blk_out.iter() {
                    fw.write_byte(byte);
                }
                blks_wrtn += 1;
                blk_out.clear();
            }  
        }

        fw.file_out.flush_buffer();

        let mut lens: Vec<u64> = Vec::new();
        get_file_out_lens(Path::new(&self.cfg.dir_out), &mut lens);
        self.prg.print_archive_stats(lens.iter().sum());
    }

    pub fn read_metadata(&mut self) -> Metadata {
        let mut mta: Metadata = Metadata::new();
        mta.mem     = self.archive.read_u64();
        mta.mgcs    = self.archive.read_u64();
        mta.blk_sz  = self.archive.read_u64() as usize;
        mta.fblk_sz = self.archive.read_u64() as usize;
        mta.blk_c   = self.archive.read_u64();
        mta.f_ptr   = self.archive.read_u64();

        self.verify_magic_number(mta.mgcs);

        // Seek to end of file metadata
        self.archive.seek(SeekFrom::Start(mta.f_ptr)).unwrap();
        let mut path: Vec<u8> = Vec::with_capacity(64);

        let num_files = self.archive.read_u64();

        // Read null terminated file path strings and lengths.
        for _ in 0..num_files {
            loop {
                match self.archive.read_byte() {
                    0 => {
                        let path_string: String = path.iter().map(|b| *b as char).collect();
                        let file_len = self.archive.read_u64();
                        mta.files.push((PathBuf::from(&path_string), file_len));
                        path.clear();
                        break;
                    }
                    byte => path.push(byte),
                }
            }
        }

        // Get compressed block sizes
        for _ in 0..mta.blk_c {
            mta.enc_blk_szs.push(self.archive.read_u64());
        }

        // Seek back to beginning of compressed data
        self.archive.seek(SeekFrom::Start(48)).unwrap();
        mta
    }

    /// Check for a valid magic number.
    /// * Non-solid archives - 'prsv'
    /// * Solid archives     - 'PRSV'
    fn verify_magic_number(&self, mgc: u64) {
        match mgc {
            0x5653_5250 => {},
            0x7673_7270 => error::found_non_solid_archive(),
            _ => error::no_prisirv_archive(),
        }
    }
}

/// Get the next output file and the input file length, the input file being
/// the original file that was compressed.
///
/// The input file length is needed to know when the output file is the 
/// correct size.
fn next_file(file_in_path: &Path, dir_out: &str) -> BufWriter<File> {
    let file_out_path = fmt_file_out_s_extract(dir_out, file_in_path);
    let file_out      = new_output_file(4096, &file_out_path);
    file_out
}

/// Get total size of decompressed archive.
fn get_file_out_lens(dir_in: &Path, lens: &mut Vec<u64>) {
    let (files, dirs): (Vec<PathBuf>, Vec<PathBuf>) =
        dir_in.read_dir().unwrap()
        .map(|d| d.unwrap().path())
        .partition(|f| f.is_file());

    for file in files.iter() {
        lens.push(file_len(file));
    }
    for dir in dirs.iter() {
        get_file_out_lens(dir, lens);
    }
}

