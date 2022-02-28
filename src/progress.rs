use std::{
    path::{Path, PathBuf},
    time::Instant
};

use crate::{
    Mode,
    parse_args::Config,
    buffered_io::file_len,
};

const CLEAR: &str = "\x1B[2J\x1B[1;1H";

/// Tracks compression or decompression progress.
#[derive(Copy, Clone, Debug)]
pub struct Progress {
    in_size: u64,
    blks: u64,
    total_blks: u64,
    blk_sz: u64,
    time: Instant,
    quiet: bool,
    mode: Mode,
}
#[allow(dead_code)]
impl Progress {
    /// Initialize values needed for tracking progress, including starting a timer.
    pub fn new(cfg: &Config, mode: Mode) -> Progress {
        Progress {
            in_size: 0,
            blks: 0,
            total_blks: 0,
            blk_sz: cfg.blk_sz as u64,
            quiet: cfg.quiet,
            time: Instant::now(),
            mode,
        }
    }
    
    // Non-Solid Archives ==========================

    /// Get input file size and calculate total block count by dividing input size by block size.
    pub fn get_input_size_enc(&mut self, input: &Path) {
        self.in_size = file_len(input);
        self.total_blks = (self.in_size as f64/self.blk_sz as f64).ceil() as u64;
    }

    /// Get the input size and total block count of a file. 
    /// Since compressed blocks are variable size, the count 
    /// can't be calculated and is instead obtained directly 
    /// from metadata.
    pub fn get_input_size_dec(&mut self, input: &Path, blk_c: usize) {
        self.in_size = file_len(input);
        self.total_blks = blk_c as u64;
    }

    /// Print final compressed file size and time elapsed.
    pub fn print_file_stats(&self, out_size: u64) {
        if !self.quiet {
            println!("{} bytes -> {} bytes in {:.2?}\n", 
                self.in_size, out_size, self.time.elapsed());
        }
    }



    // Solid Archives ==============================

    /// Get input archive size and calculate total block count by dividing input size by block size.
    pub fn get_input_size_solid_enc(&mut self, files: &[(String, u64)]) {
        for file in files.iter().map(|f| f.0.clone()).map(PathBuf::from) {
            self.in_size += file_len(&file);
        }
        self.total_blks = (self.in_size as f64/self.blk_sz as f64).ceil() as u64;
    }
    /// Get the input size and total block count of an archive. Since compressed 
    /// blocks are variable size, the count can't be calculated and is instead 
    /// obtained directly from metadata.
    pub fn get_input_size_solid_dec(&mut self, files: &[PathBuf], blk_c: usize) {
        for file in files.iter() {
            self.in_size += file_len(file);
        }
        self.total_blks = blk_c as u64;
    }
    /// Print final compressed archive size and time elapsed.
    pub fn print_archive_stats(&self, out_size: u64) {
        if !self.quiet {
            println!("{} bytes -> {} bytes in {:.2?}\n", 
                self.in_size, out_size, self.time.elapsed());
        }
    }



    /// Increase current block count by 1 and print current stats.
    pub fn update(&mut self) {
        self.blks += 1;
        self.print_block_stats();
    }

    /// Print the current number of blocks compressed and the current time elapsed.
    fn print_block_stats(&self) {
        if !self.quiet {
            match self.mode {
                Mode::Compress => {
                    println!("{}Compressed block {} of {} ({:.2}%) (Time elapsed: {:.2?})", 
                    CLEAR, self.blks, self.total_blks, 
                    (self.blks as f64/self.total_blks as f64)*100.0,
                    self.time.elapsed());
                }
                Mode::Decompress =>  {
                    println!("{}Decompressed block {} of {} ({:.2}%) (Time elapsed: {:.2?})", 
                    CLEAR, self.blks, self.total_blks, 
                    (self.blks as f64/self.total_blks as f64)*100.0,
                    self.time.elapsed());
                }
            } 
        }
    }
}
