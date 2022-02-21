mod encoder;       mod predictor;   mod logistic;
mod decoder;       mod mixer;       mod metadata;
mod archive;       mod statemap;    mod tables;
mod solid_archive; mod apm;         mod sort;
mod buffered_io;   mod hash_table;  mod parse_args;
mod formatting;    mod match_model; mod threads;
mod progress;

use std::{
    path::{Path, PathBuf},
    time::Instant,
    env,  
};
use crate::{
    encoder::Encoder,
    decoder::Decoder,
    metadata::Metadata,
    archive::{Archiver, Extractor},
    solid_archive::{SolidArchiver, SolidExtractor},
    sort::{Sort, sort_files},
    formatting::fmt_root_output_dir,
    parse_args::Config,
    buffered_io::{
        new_input_file, new_dir_checked, 
        new_output_file_checked,
    },
};

#[derive(PartialEq, Copy, Clone, Debug)]
pub enum Mode {
    Compress,
    Decompress,
}
#[derive(PartialEq, Copy, Clone)]
pub enum Arch {
    Solid,
    NonSolid,
}


pub fn file_path_ext(path: &Path) -> String {
    path.to_str().unwrap().to_string()
}
pub fn file_len(path: &Path) -> u64 {
    path.metadata().unwrap().len()
}

// Recursively collect all files into a vector for sorting before compression.
fn collect_files(dir_in: &Path, mta: &mut Metadata) {
    let (files, dirs): (Vec<PathBuf>, Vec<PathBuf>) =
        dir_in.read_dir().unwrap()
        .map(|d| d.unwrap().path())
        .partition(|f| f.is_file());

    for file in files.iter() {
        mta.files.push(
            (file_path_ext(file), 0, 0)
        );
    }
    for dir in dirs.iter() {
        collect_files(dir, mta);
    }
}

fn print_config(cfg: &Config, dir_out: &str) {
    if !cfg.quiet {
        println!();
        println!("=======================================================================");
        println!(" {} {} Archive of Inputs:", 
            if cfg.mode == Mode::Compress { "Creating" } else { "Extracting" },
            if cfg.arch == Arch::Solid { "Solid" } else { "Non-Solid" });
        for input in cfg.inputs.iter() {
            println!("    {} ({})", 
                input.display(),
                if input.is_file() { "File" }
                else if input.is_dir() { "Directory" }
                else { "" }
            );
        }
        println!();
        println!(" Output Directory: {}", dir_out);
        if cfg.mode == Mode::Compress {
            println!(" Sorting by: {}", 
            match cfg.sort {
                Sort::None     => "None",
                Sort::Ext      => "Extension",
                Sort::Name     => "Name",
                Sort::Len      => "Length",
                Sort::Created  => "Creation time",
                Sort::Accessed => "Last accessed time",
                Sort::Modified => "Last modified time",
                Sort::PrtDir(_) => "Parent Directory",
            });
            println!(" {}", format!("Memory Usage: {} MB", 3 + (cfg.mem >> 20) * 3));
            println!(" Block Size: {} MB", cfg.blk_sz/1024/1024);
            println!(" Threads: Up to {}", cfg.threads);
        }
        println!("=======================================================================");
        println!();
    }
}


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

fn main() {
    let cfg = Config::new(&env::args().skip(1).collect::<Vec<String>>());

    let mut dir_out = fmt_root_output_dir(&cfg);

    print_config(&cfg, &dir_out);

    match (cfg.arch, cfg.mode) {
        (Arch::Solid, Mode::Compress) => {
            let mut mta: Metadata = Metadata::new();
            mta.blk_sz = cfg.blk_sz;

            // Group files and directories 
            let (files, dirs): (Vec<PathBuf>, Vec<PathBuf>) =
                cfg.inputs.clone().into_iter().partition(|f| f.is_file());

            // Walk through directories and collect all files
            for file in files.iter() {
                mta.files.push(
                    (file_path_ext(file), 0, 0)
                );
            }
            for dir in dirs.iter() {
                collect_files(dir, &mut mta);
            }

            // Sort files to potentially improve compression of solid archives
            match cfg.sort {
                Sort::None => {},
                _ => mta.files.sort_by(|f1, f2| sort_files(&f1.0, &f2.0, &cfg.sort)),
            }

            let file_out = new_output_file_checked(&dir_out, cfg.clbr);

            let enc = Encoder::new(file_out, &cfg);
            let mut sld_arch = SolidArchiver::new(enc, mta, cfg);

            sld_arch.create_archive();
            sld_arch.write_metadata();
        }
        (Arch::Solid, Mode::Decompress) => {
            let mta: Metadata = Metadata::new();

            if !cfg.inputs[0].is_file() {
                println!("Input {} is not a solid archive.", cfg.inputs[0].display());
                println!("To extract a non-solid archive, omit option '-sld'.");
                std::process::exit(0);
            }

            let dec = Decoder::new(new_input_file(4096, &cfg.inputs[0]));
            let mut sld_extr = SolidExtractor::new(dec, mta, cfg);

            sld_extr.read_metadata();
            sld_extr.extract_archive(&dir_out);
        }
        (Arch::NonSolid, Mode::Compress) => {
            new_dir_checked(&dir_out, cfg.clbr);
            let quiet = cfg.quiet;
            
            let (files, dirs): (Vec<PathBuf>, Vec<PathBuf>) = 
                cfg.inputs.clone().into_iter().partition(|f| f.is_file());

            let mut arch = Archiver::new(cfg);
            for file_in in files.iter() {
                if !quiet { println!("Compressing {}", file_in.display()); }
                arch.compress_file(file_in, &dir_out);
            }
            for dir_in in dirs.iter() {
                arch.compress_dir(dir_in, &mut dir_out);      
            }
        }
        (Arch::NonSolid, Mode::Decompress) => {
            new_dir_checked(&dir_out, cfg.clbr);
            let quiet = cfg.quiet;
            
            let (files, dirs): (Vec<PathBuf>, Vec<PathBuf>) = 
                cfg.inputs.clone().into_iter().partition(|f| f.is_file());

            let mut extr = Extractor::new(cfg);
            for file_in in files.iter() {
                //let time = Instant::now();
                if !quiet { println!("Decompressing {}", file_in.display()); }
                //let file_in_size  = file_len(file_in); 
                extr.decompress_file(file_in, &dir_out);
                //if !quiet { println!("{} bytes -> {} bytes in {:.2?}\n", 
                //    file_in_size, file_out_size, time.elapsed()); } 
            }
            for dir_in in dirs.iter() {
                extr.decompress_dir(dir_in, &mut dir_out, true);      
            }
        }
    }  
}