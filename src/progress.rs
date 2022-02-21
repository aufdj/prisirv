use std::time::Instant;
use std::path::Path;

use crate::Mode;
use crate::parse_args::Config;
use crate::buffered_io::file_len;


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
    pub fn get_input_size_enc(&mut self, input: &Path) {
        self.in_size = file_len(&input);
        self.total_blks = (self.in_size as f64/self.blk_sz as f64).ceil() as u64;
    }
    pub fn get_input_size_dec(&mut self, input: &Path, blk_c: usize) {
        self.in_size = file_len(&input);
        self.total_blks = blk_c as u64;
    }
    pub fn update(&mut self) {
        self.blks += 1;
        self.print_block_stats();
    }
    pub fn print_block_stats(&self) {
        if !self.quiet {
            match self.mode {
                Mode::Compress => {
                    println!("Compressed block {} of {} ({:.2}%) (Time elapsed: {:.2?})", 
                    self.blks, self.total_blks, 
                    (self.blks as f64/self.total_blks as f64)*100.0,
                    self.time.elapsed());
                }
                Mode::Decompress =>  {
                    println!("Decompressed block {} of {} ({:.2}%) (Time elapsed: {:.2?})", 
                    self.blks, self.total_blks, 
                    (self.blks as f64/self.total_blks as f64)*100.0,
                    self.time.elapsed());
                }
            }
            
        }
    }
    pub fn print_file_stats(&self, out_size: u64) {
        if !self.quiet {
            println!("{} bytes -> {} bytes in {:.2?}\n", 
                self.in_size, out_size, self.time.elapsed());
        }
    }
}
