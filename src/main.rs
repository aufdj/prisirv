mod buffered_io; mod logistic; mod statemap;
mod apm;         mod mixer;    mod match_model;
mod hash_table;  mod metadata; mod predictor;
mod encoder;     mod decoder;  mod archive;
mod tables;      mod sort;     mod formatting;
mod parse_args;

use std::{
    path::{Path, PathBuf},
    time::Instant,
    env,  
};
use crate::{
    encoder::Encoder,
    decoder::Decoder,
    metadata::Metadata,
    archive::{
        Archiver, Extractor,
        SolidArchiver, SolidExtractor
    },
    buffered_io::{
        new_input_file, new_dir_checked, 
        new_output_file_checked,
    },
    sort::{Sort, sort_files},
    formatting::fmt_root_output_dir,
    parse_args::Config,
};

#[derive(PartialEq, Copy, Clone)]
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
        println!("//////////////////////////////////////////////////////////////");
        println!(
            "{} {} archive {} of inputs:\n{:#?},\nsorting by {}{}.",
            if cfg.mode == Mode::Compress { "Creating" } else { "Extracting" },
            if cfg.arch == Arch::Solid { "solid" } else { "non-solid" },
            dir_out, 
            cfg.inputs,
            match cfg.sort {
                Sort::None     => "none",
                Sort::Ext      => "extension",
                Sort::Name     => "name",
                Sort::Len      => "length",
                Sort::Created  => "creation time",
                Sort::Accessed => "last accessed time",
                Sort::Modified => "last modified time",
                Sort::PrtDir(_) => "parent",
            },
            // During extraction, memory usage isn't known until decoder initializtion.
            if cfg.mode == Mode::Compress {
                format!(", using {} MB of memory", 3 + (cfg.mem >> 20) * 3)
            } else { String::from("") }
        );
        println!("//////////////////////////////////////////////////////////////");
        println!();
    }
}

fn main() {
    let cfg = Config::new(&env::args().skip(1).collect::<Vec<String>>());

    let mut dir_out = fmt_root_output_dir(&cfg);

    print_config(&cfg, &dir_out);

    match (cfg.arch, cfg.mode) {
        (Arch::Solid, Mode::Compress) => {
            let mut mta: Metadata = Metadata::new();

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
                let time = Instant::now();
                if !quiet { println!("Compressing {}", file_in.display()); }
                let file_in_size  = file_len(file_in); 
                let file_out_size = arch.compress_file(file_in, &dir_out);
                if !quiet { println!("{} bytes -> {} bytes in {:.2?}\n", 
                    file_in_size, file_out_size, time.elapsed()); }
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

            let extr = Extractor::new(cfg);
            for file_in in files.iter() {
                let time = Instant::now();
                if !quiet { println!("Decompressing {}", file_in.display()); }
                let file_in_size  = file_len(file_in); 
                let file_out_size = extr.decompress_file(file_in, &dir_out);
                if !quiet { println!("{} bytes -> {} bytes in {:.2?}\n", 
                    file_in_size, file_out_size, time.elapsed()); } 
            }
            for dir_in in dirs.iter() {
                extr.decompress_dir(dir_in, &mut dir_out, true);      
            }
        }
    }  
}