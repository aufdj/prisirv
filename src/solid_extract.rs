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
    file_in_paths:  Box<dyn Iterator<Item = PathBuf>>, // All input file paths
    file_in_lens:   Box<dyn Iterator<Item = u64>>,     // All input file lengths
    file_out:       BufWriter<File>,                   // Current output file
    file_out_pos:   u64,                               // Current output file position
    file_in_len:    u64,                               // Uncompressed file length
    dir_out:        String,                            // Output directory
}
impl FileWriter {
    fn new(files: &[(PathBuf, u64)], dir_out: &str) -> FileWriter {
        let (paths, lens): (Vec<PathBuf>, Vec<u64>) = 
            files.to_owned().into_iter().unzip();

        let mut paths = paths.into_iter();
        let mut lens  = lens.into_iter();

        let file_out = next_file(&paths.next().unwrap(), dir_out);
        let file_in_len = lens.next().unwrap();

        FileWriter {
            file_out_pos:   0,
            dir_out:        dir_out.to_string(),
            file_in_paths:  Box::new(paths), 
            file_in_lens:   Box::new(lens), 
            file_out, 
            file_in_len,
        }
    }
    fn update(&mut self) {
        self.file_out = next_file(&self.file_in_paths.next().unwrap(), &self.dir_out);
        self.file_in_len = self.file_in_lens.next().unwrap();
        self.file_out_pos = 0;
    }
    fn write_byte(&mut self, byte: u8) {
        if self.file_out_pos == self.file_in_len {
            self.update();
        }
        self.file_out.write_byte(byte);
        self.file_out_pos += 1;
    }
}

/// Get the next output file and the input file length, the input file being
/// the original file that was compressed.
///
/// The input file length is needed to know when the output file is the 
/// correct size.
fn next_file(file_in_path: &Path, dir_out: &str) -> BufWriter<File> {
    let file_out_path = fmt_file_out_s_extract(dir_out, file_in_path);
    new_output_file(4096, &file_out_path)
}

/// A SolidExtractor extracts solid archives.
pub struct SolidExtractor {
    archive:  BufReader<File>,
    pub cfg:  Config,
    prg:      Progress,
    mta:      Metadata,
}
impl SolidExtractor {
    /// Create a new SolidExtractor.
    pub fn new(cfg: Config) -> SolidExtractor {
        new_dir_checked(&cfg.dir_out, cfg.clbr);
        let first_input = &cfg.inputs[0];
        let prg = Progress::new(&cfg, Mode::Decompress);
        
        let mut extr = SolidExtractor { 
            archive:  new_input_file(4096, first_input), 
            mta:      Metadata::new(),
            cfg, 
            prg, 
        };
        extr.mta = extr.read_metadata();
        extr.prg.get_archive_size_dec(&extr.cfg.inputs, extr.mta.blk_c);
        extr
    }

    /// Decompress blocks and parse blocks into files. A block can span 
    /// multiple files.
    pub fn extract_archive(&mut self) {
        let mut tp = ThreadPool::new(self.cfg.threads, self.mta.mem, self.prg);
        let mut blk = Vec::with_capacity(self.mta.blk_sz);
        let mut index: u64 = 0;

        // Full blocks
        for _ in 0..self.mta.blk_c-1 {
            for _ in 0..self.mta.enc_blk_szs[index as usize] {
                blk.push(self.archive.read_byte());
            }
            tp.decompress_block(blk.clone(), index, self.mta.blk_sz);
            blk.clear();
            index += 1;
        }

        // Final block
        for _ in 0..self.mta.enc_blk_szs[index as usize] {
            blk.push(self.archive.read_byte());
        }
        tp.decompress_block(blk.clone(), index, self.mta.fblk_sz);
        blk.clear();

        let mut fw = FileWriter::new(&self.mta.files, &self.cfg.dir_out);
        let mut blks_wrtn: u64 = 0;
        
        // Write blocks to output 
        while blks_wrtn != self.mta.blk_c {
            tp.bq.lock().unwrap().try_get_block(&mut blk);
            if !blk.is_empty() {
                for byte in blk.iter() {
                    fw.write_byte(*byte);
                }
                blks_wrtn += 1;
                blk.clear();
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
                        let path_string = path.iter()
                            .map(|b| *b as char)
                            .collect::<String>();
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

