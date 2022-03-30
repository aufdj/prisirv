use std::{
    path::{Path, PathBuf},
    time::Instant,
    io::Write,
};

use crate::{
    Mode,
    config::Config,
    buffered_io::file_len,
    metadata::FileData,
};


/// Track compression or decompression progress.
#[derive(Copy, Clone, Debug)]
pub struct Progress {
    in_size:     u64,
    blks:        u64,
    total_blks:  u64,
    blk_sz:      u64,
    time:        Instant,
    quiet:       bool,
    mode:        Mode,
}
#[allow(dead_code)]
impl Progress {
    /// Initialize values needed for tracking progress, including starting a timer.
    pub fn new(cfg: &Config) -> Progress {
        Progress {
            in_size:     0,
            blks:        0,
            total_blks:  0,
            blk_sz:      cfg.blk_sz as u64,
            quiet:       cfg.quiet,
            time:        Instant::now(),
            mode:        cfg.mode,
        }
    }
    
    // Non-Solid Archives ==========================

    /// Get input file size and calculate total block count by dividing 
    /// input size by block size.
    pub fn get_file_size_enc(&mut self, input: &Path) {
        self.in_size = file_len(input);
        self.total_blks = (self.in_size as f64/self.blk_sz as f64).ceil() as u64;
    }

    /// Get the input size and block count of a file. Because compressed 
    /// blocks are variable size, the count can't be calculated and is 
    /// instead obtained directly from metadata.
    pub fn get_file_size_dec(&mut self, input: &Path, blk_c: usize) {
        self.in_size = file_len(input);
        self.total_blks = blk_c as u64;
    }

    /// Print final compressed file size and time elapsed.
    pub fn print_file_stats(&self, out_size: u64) {
        if !self.quiet {
            println!("\n{} bytes -> {} bytes in {:.2?}\n", 
                self.in_size, out_size, self.time.elapsed());
        }
    }

    pub fn print_file_name(&self, file: &Path) {
        if !self.quiet { 
            match self.mode {
                Mode::Compress => {
                    println!("Compressing {}", file.display());
                }
                Mode::Decompress => {
                    println!("Decompressing {}", file.display());
                }   
            } 
        } 
    }


    // Solid Archives ==============================

    /// Get input archive size and calculate total block count by dividing 
    /// input size by block size.
    pub fn get_archive_size_enc(&mut self, files: &[FileData]) {
        for file in files.iter() {
            self.in_size += file.len;
        }
        self.total_blks = (self.in_size as f64/self.blk_sz as f64).ceil() as u64;
    }

    /// Get the input size and block count of an archive. Since compressed 
    /// blocks are variable size, the count can't be calculated and is 
    /// instead obtained directly from metadata.
    pub fn get_archive_size_dec(&mut self, files: &[PathBuf], blk_c: u64) {
        for file in files.iter() {
            self.in_size += file_len(file);
        }
        self.total_blks = blk_c;
    }

    /// Print final compressed archive size and time elapsed.
    pub fn print_archive_stats(&self, out_size: u64) {
        if !self.quiet {
            println!("\n{} bytes -> {} bytes in {:.2?}\n", 
                self.in_size, out_size, self.time.elapsed());
        }
    }



    /// Increase current block count by 1 and print current stats.
    pub fn update(&mut self) {
        self.blks += 1;
        self.print_block_stats();
    }

    /// Print the current number of blocks compressed and the current 
    /// time elapsed.
    fn print_block_stats(&self) {
        if !self.quiet {
            match self.mode {
                Mode::Compress => {
                    print!("\rCompressed block {} of {} ({:.2}%) (Time elapsed: {:.2?})  ", 
                         self.blks, 
                         self.total_blks, 
                        (self.blks as f64/self.total_blks as f64)*100.0,
                         self.time.elapsed()
                    );
                    std::io::stdout().flush().unwrap();
                }
                Mode::Decompress =>  {
                    print!("\rDecompressed block {} of {} ({:.2}%) (Time elapsed: {:.2?})  ", 
                         self.blks, 
                         self.total_blks, 
                        (self.blks as f64/self.total_blks as f64)*100.0,
                         self.time.elapsed()
                    );
                    std::io::stdout().flush().unwrap();
                }
            } 
        }
    }
}
                                               

