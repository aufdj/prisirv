use std::{
    time::Instant,
    io::Write,
};

use crate::{
    config::{Config, Mode},
    block::Block,
};


/// Track compression or decompression progress.
#[derive(Clone, Debug)]
pub struct Progress {
    sizei:    u64,     // Input size
    sizeo:    u64,     // Output size
    current:  u64,     // Portion of input data (de)compressed
    quiet:    bool,    // Suppress output
    mode:     Mode,    // (De)compress or add
    time:     Instant, // Timer
}
impl Progress {
    /// Initialize values needed for tracking progress, including starting a timer.
    pub fn new(cfg: &Config) -> Progress {
        let sizei = 
        if cfg.mode == Mode::ExtractArchive
        || cfg.mode == Mode::ExtractFiles {
            cfg.arch.len
        }
        else {
            cfg.inputs.iter()
            .map(|f| f.len)
            .sum()
        };
        
        let sizeo =
        if cfg.mode == Mode::ExtractFiles
        || cfg.mode == Mode::ExtractArchive {
            0
        }
        else { 
            cfg.arch.len
        };
        Progress {
            sizei,
            sizeo,
            current:  0,
            quiet:    cfg.quiet,
            mode:     cfg.mode,
            time:     Instant::now(),
        }
    }

    /// Update and print stats. A compressed block includes extra metadata
    /// whereas a decompressed block does not, so if mode is compress, track 
    /// the total size of the block including metadata, rather than just the 
    /// compressed data.
    pub fn update(&mut self, blk: &Block) {
        self.current += blk.sizei;
        if self.mode == Mode::CreateArchive
        || self.mode == Mode::AppendFiles
        || self.mode == Mode::MergeArchives {
            self.sizeo += blk.size();
        }
        else { 
            self.sizeo += blk.sizeo; 
        }
        self.print_stats();
    }

    // Print percentage and elapsed time.
    fn print_stats(&self) {
        if !self.quiet {
            let percent = (self.current as f64 / self.sizei as f64) * 100.0;
            print!("\r{} ({:.2}%) (Time elapsed: {:.2?})  ",
                bar(percent),
                percent,
                self.time.elapsed()
            );
            std::io::stdout().flush().unwrap();
        }
    }
}
impl Drop for Progress {
    fn drop(&mut self) {
        if !self.quiet && self.sizeo > 0 {
            match self.mode {
                Mode::ExtractFiles |
                Mode::ExtractArchive => {
                    println!("\rExtracted {} bytes in {:.2?}                                                   ", 
                        self.sizeo, self.time.elapsed()
                    );
                }
                _ => {
                    println!("\r                                                                                   
                        \rArchive size: {} bytes
                        \rTime Elapsed: {:.2?}", 
                        self.sizeo, self.time.elapsed()
                    );
                }
            }
        }
        
    }
}
fn bar(percent: f64) -> &'static str {
    match percent as u64 {
         0..=4   => "[=>                   ]",
         5..=9   => "[==>                  ]",
        10..=14  => "[===>                 ]",
        15..=19  => "[====>                ]",
        20..=24  => "[=====>               ]",
        25..=29  => "[======>              ]",
        30..=34  => "[=======>             ]",
        35..=39  => "[========>            ]",
        40..=44  => "[=========>           ]",
        45..=49  => "[==========>          ]",
        50..=54  => "[===========>         ]",
        55..=59  => "[============>        ]",
        60..=64  => "[=============>       ]",
        65..=69  => "[==============>      ]",
        70..=74  => "[===============>     ]",
        75..=79  => "[================>    ]",
        80..=84  => "[=================>   ]",
        85..=89  => "[==================>  ]",
        90..=94  => "[===================> ]",
        95..=99  => "[====================>]",
        _        => ""
    }
}
                                               

