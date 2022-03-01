mod encoder;       mod predictor;   mod logistic;
mod decoder;       mod mixer;       mod metadata;
mod archive;       mod statemap;    mod tables;
mod solid_archive; mod apm;         mod sort;
mod buffered_io;   mod hash_table;  mod parse_args;
mod formatting;    mod match_model; mod threads;
mod progress;      

use std::{
    path::{Path, PathBuf},
    env,  
};
use crate::{
    metadata::Metadata,
    archive::{Archiver, Extractor},
    solid_archive::{SolidArchiver, SolidExtractor},
    sort::{Sort, sort_files},
    parse_args::Config,
    buffered_io::{new_dir_checked, file_len},
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

// Recursively collect all files into a vector for sorting before compression.
fn collect_files(dir_in: &Path, mta: &mut Metadata) {
    let (files, dirs): (Vec<PathBuf>, Vec<PathBuf>) =
        dir_in.read_dir().unwrap()
        .map(|d| d.unwrap().path())
        .partition(|f| f.is_file());

    for file in files.iter() {
        mta.files.push(
            (file.display().to_string(), file_len(file))
        );
    }
    for dir in dirs.iter() {
        collect_files(dir, mta);
    }
}

fn main() {
    let mut cfg = Config::new(&env::args().skip(1).collect::<Vec<String>>());

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
                    (file.display().to_string(), file_len(file))
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

            let mut sld_arch = SolidArchiver::new(mta, cfg);
            sld_arch.create_archive();
        }
        (Arch::Solid, Mode::Decompress) => {
            let mta: Metadata = Metadata::new();

            if !cfg.inputs[0].is_file() {
                println!("Input {} is not a solid archive.", cfg.inputs[0].display());
                println!("To extract a non-solid archive, omit option '-sld'.");
                std::process::exit(0);
            }

            let mut sld_extr = SolidExtractor::new(mta, cfg);
            sld_extr.extract_archive();
        }
        (Arch::NonSolid, Mode::Compress) => {
            new_dir_checked(&cfg.dir_out, cfg.clbr);
            
            let (files, dirs): (Vec<PathBuf>, Vec<PathBuf>) = 
                cfg.inputs.clone().into_iter().partition(|f| f.is_file());

            let mut arch = Archiver::new(cfg.clone());
            for file_in in files.iter() {
                if !cfg.quiet { println!("Compressing {}", file_in.display()); }
                arch.compress_file(file_in, &cfg.dir_out);
            }
            for dir_in in dirs.iter() {
                arch.compress_dir(dir_in, &mut cfg.dir_out);      
            }
        }
        (Arch::NonSolid, Mode::Decompress) => {
            new_dir_checked(&cfg.dir_out, cfg.clbr);
            
            let (files, dirs): (Vec<PathBuf>, Vec<PathBuf>) = 
                cfg.inputs.clone().into_iter().partition(|f| f.is_file());

            let mut extr = Extractor::new(cfg.clone());
            for file_in in files.iter() {
                if !cfg.quiet { println!("Decompressing {}", file_in.display()); } 
                extr.decompress_file(file_in, &cfg.dir_out);
            }
            for dir_in in dirs.iter() {
                extr.decompress_dir(dir_in, &mut cfg.dir_out, true);      
            }
        }
    }  
}
