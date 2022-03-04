mod encoder;       mod predictor;   mod logistic;
mod decoder;       mod mixer;       mod metadata;
mod archive;       mod statemap;    mod tables;
mod solid_archive; mod apm;         mod sort;
mod buffered_io;   mod hash_table;  mod parse_args;
mod formatting;    mod match_model; mod threads;
mod progress;

use crate::{
    archive::{Archiver, Extractor},
    solid_archive::{SolidArchiver, SolidExtractor},
    parse_args::Config,
};

#[derive(PartialEq, Copy, Clone, Debug)]
pub enum Mode {
    Compress,
    Decompress,
}
#[derive(PartialEq, Copy, Clone, Debug)]
pub enum Arch {
    Solid,
    NonSolid,
}

fn main() {
    let cfg = Config::new(&std::env::args().skip(1).collect::<Vec<String>>());

    match (cfg.arch, cfg.mode) {
        (Arch::Solid,    Mode::Compress)   => { SolidArchiver::new(cfg).create_archive();   }
        (Arch::Solid,    Mode::Decompress) => { SolidExtractor::new(cfg).extract_archive(); }
        (Arch::NonSolid, Mode::Compress)   => { Archiver::new(cfg).create_archive();        }
        (Arch::NonSolid, Mode::Decompress) => { Extractor::new(cfg).extract_archive();      }
    }  
}