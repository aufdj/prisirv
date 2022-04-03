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
    total:    u64,
    current:  u64,
    blks:     u64,
    quiet:    bool,
    mode:     Mode,
    time:     Instant,
}
#[allow(dead_code)]
impl Progress {
    /// Initialize values needed for tracking progress, including starting a timer.
    pub fn new(cfg: &Config) -> Progress {
        Progress {
            total:    0,
            current:  0,
            blks:     0,
            quiet:    cfg.quiet,
            mode:     cfg.mode,
            time:     Instant::now(),
        }
    }
    
    // Non-Solid Archives ==========================

    /// Get input file size.
    pub fn get_file_size(&mut self, input: &Path) {
        self.total = file_len(input);
    }

    /// Print final compressed file size and time elapsed.
    pub fn print_file_stats(&self, out_size: u64) {
        if !self.quiet {
            println!("\n{} bytes -> {} bytes in {:.2?}\n", 
                self.total, out_size, self.time.elapsed());
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

    /// Get size of files to be archived.
    pub fn get_archive_size_enc(&mut self, files: &[FileData]) {
        for file in files.iter() {
            self.total += file.len;
        }
    }

    /// Get size of archive.
    pub fn get_archive_size_dec(&mut self, files: &[PathBuf]) {
        for file in files.iter() {
            self.total += file_len(file);
        }
    }

    /// Print final compressed archive size and time elapsed.
    pub fn print_archive_stats(&self, out_size: u64) {
        if !self.quiet {
            println!("\n{} bytes -> {} bytes in {:.2?}\n", 
                self.total, out_size, self.time.elapsed());
        }
    }



    /// Update current output size and print stats.
    pub fn update(&mut self, size: u64) {
        self.current += size;
        if self.blks > 0 {
            self.print_stats();
        }
        self.blks += 1;
    }

    // Print percentage and elapsed time.
    fn print_stats(&self) {
        if !self.quiet {
            let percent = (self.current as f64 / self.total as f64) * 100.0;
            match self.mode {
                Mode::Compress => {
                    print!("\r{} ({:.2}%) (Time elapsed: {:.2?})  ", 
                        bar(percent),
                        percent,
                        self.time.elapsed()
                    );
                    std::io::stdout().flush().unwrap();
                }
                Mode::Decompress =>  {
                    print!("\r{} ({:.2}%) (Time elapsed: {:.2?})  ", 
                        bar(percent),
                        percent,
                        self.time.elapsed()
                    );
                    std::io::stdout().flush().unwrap();
                }
            } 
        }
    }
}
fn bar(percent: f64) -> &'static str {
    match percent as u64 {
         0..=9   => "[=>         ]",
        10..=19  => "[==>        ]",
        20..=29  => "[===>       ]",
        30..=39  => "[====>      ]",
        40..=49  => "[=====>     ]",
        50..=59  => "[======>    ]",
        60..=69  => "[=======>   ]",
        70..=79  => "[========>  ]",
        80..=89  => "[=========> ]",
        90..=99  => "[==========>]",
        _        => "[===========]"
    }
}
                                               

