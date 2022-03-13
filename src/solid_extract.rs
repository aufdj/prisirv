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
        
        SolidExtractor { archive, cfg, prg }
    }

    /// Decompress blocks and parse blocks into files. A block can span 
    /// multiple files.
    pub fn extract_archive(&mut self) {
        let mta: Metadata = self.read_metadata();

        self.prg.get_archive_size_dec(&self.cfg.inputs, mta.blk_c);
        new_dir_checked(&self.cfg.dir_out, self.cfg.clbr);

        let mut tp = ThreadPool::new(self.cfg.threads, mta.mem, self.prg);
        let mut index: u64 = 0;
        
        // Decompress blocks ----------------------------------------
        // Full blocks
        for _ in 0..mta.blk_c-1 {
            let mut blk_in = Vec::with_capacity(mta.blk_sz);
            for _ in 0..mta.enc_blk_szs[index as usize] {
                blk_in.push(self.archive.read_byte());
            }
            tp.decompress_block(blk_in, index, mta.blk_sz);
            index += 1;
        }
        // Final block
        let mut blk_in = Vec::with_capacity(mta.blk_sz);
        for _ in 0..mta.enc_blk_szs[index as usize] {
            blk_in.push(self.archive.read_byte());
        }
        tp.decompress_block(blk_in, index, mta.fblk_sz);
        // ----------------------------------------------------------


        let mut file_in_paths = 
            mta.files.iter()
            .map(|f| PathBuf::from(&f.0))
            .collect::<Vec<PathBuf>>()
            .into_iter();

        let file_in_path = file_in_paths.next().unwrap();

        let (mut file_in_len, mut file_out) = next_file(&file_in_path, &self.cfg.dir_out);

        let mut file_out_pos = 0;
        let mut blks_wrtn: u64 = 0;
        let mut blk_out = Vec::new();
        
        // Write blocks to output -----------------------------------
        while blks_wrtn != mta.blk_c {
            tp.bq.lock().unwrap().try_get_block(&mut blk_out);
            if !blk_out.is_empty() {
                for byte in blk_out.iter() {
                    // When current output file reaches the 
                    // correct size, move to next file.
                    if file_out_pos == file_in_len {
                        let file_in_path = match file_in_paths.next() {
                            Some(path) => { path }
                            None => break,
                        };
                        (file_in_len, file_out) = next_file(&file_in_path, &self.cfg.dir_out);
                        file_out_pos = 0;
                    }
                    file_out.write_byte(*byte);
                    file_out_pos += 1;
                }
                blks_wrtn += 1;  
                blk_out.clear();
            }  
        }
        // ----------------------------------------------------------

        file_out.flush_buffer();

        let mut lens: Vec<u64> = Vec::new();
        get_archive_size(Path::new(&self.cfg.dir_out), &mut lens);
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

        // Get number of files
        let num_files = self.archive.read_u64();

        for _ in 0..num_files {
            // Get length of next path
            let len = self.archive.read_byte();

            // Get file path and length
            for _ in 0..len {
                path.push(self.archive.read_byte());
            }

            let path_string = path.iter().map(|b| *b as char).collect();
            let file_len = self.archive.read_u64();
            mta.files.push((path_string, file_len));
            
            path.clear();
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
/// i.e. output file is foo_d\bar.txt, input file is foo\bar.txt
///
/// The input file length is needed to know when the output file is the 
/// correct size.
///
/// The output file paths are tracked so the final extracted archive size 
/// can be computed at the end of extraction.
fn next_file(file_in_path: &Path, dir_out: &str) -> (u64, BufWriter<File>) {
    let file_in_len   = file_len(file_in_path);
    let file_out_path = fmt_file_out_s_extract(dir_out, file_in_path);
    let file_out      = new_output_file(4096, &file_out_path);
    (file_in_len, file_out)
}

fn get_archive_size(dir_in: &Path, lens: &mut Vec<u64>) {
    let (files, dirs): (Vec<PathBuf>, Vec<PathBuf>) =
        dir_in.read_dir().unwrap()
        .map(|d| d.unwrap().path())
        .partition(|f| f.is_file());

    for file in files.iter() {
        lens.push(file_len(file));
    }
    for dir in dirs.iter() {
        get_archive_size(dir, lens);
    }
}

