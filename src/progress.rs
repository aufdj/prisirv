use std::{
    time::Instant,
    io::Write,
};

use crate::{
    Mode,
    config::Config,
    block::Block,
    filedata::FileData,
};


/// Track compression or decompression progress.
#[derive(Clone, Debug)]
pub struct Progress {
    sizei:      u64,     // Input size
    pub sizeo:  u64,     // Output size
    current:    u64,     // Portion of input data (de)compressed
    quiet:      bool,    // Suppress output
    mode:       Mode,
    time:       Instant, // Timer
}
impl Progress {
    /// Initialize values needed for tracking progress, including starting a timer.
    pub fn new(cfg: &Config, files: &[FileData]) -> Progress {
        Progress {
            sizei:    files.iter().map(|f| f.len).sum(),
            sizeo:    0,
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
        if self.mode == Mode::Compress {
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
        // If mode is compress, add the 28 byte header to the total.
        if self.mode == Mode::Compress { self.sizeo += 28; }
        if !self.quiet {
            println!("\r{} bytes -> {} bytes in {:.2?}                                                   ", 
                self.sizei, self.sizeo, self.time.elapsed());
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
                                               

