mod encoder;       mod predictor;   mod logistic;
mod decoder;       mod mixer;       mod metadata;
mod archive;       mod statemap;    mod tables;
mod solid_archive; mod apm;         mod sort;
mod buffered_io;   mod hash_table;  pub mod config;
mod formatting;    mod match_model; mod threads;
mod progress;      pub mod crc32;


use std::path::PathBuf;

use crate::{
    archive::{Archiver, Extractor},
    solid_archive::{SolidArchiver, SolidExtractor},
    config::Config,
    sort::Sort,
    formatting::fmt_root_output_dir,
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

#[derive(Clone)]
pub struct Prisirv {
    cfg: Config,
}
impl Prisirv {
    pub fn new() -> Prisirv {
        let cfg = Config::new_empty();
        Prisirv { cfg }
    }
    pub fn solid(&mut self) -> Self {
        self.cfg.arch = Arch::Solid;
        self.clone()
    }
    pub fn threads(&mut self, count: usize) -> Self {
        self.cfg.threads = count;
        self.clone()
    }
    pub fn quiet(&mut self) -> Self {
        self.cfg.quiet = true;
        self.clone()
    }
    pub fn clobber(&mut self) -> Self {
        self.cfg.clbr = true;
        self.clone()
    }
    pub fn block_size(&mut self, size: usize) -> Self {
        self.cfg.blk_sz = size;
        self.clone()
    }
    pub fn memory(&mut self, mem: u64) -> Self {
        self.cfg.mem = mem;
        self.clone()
    }
    pub fn sort(&mut self, method: Sort) -> Self {
        self.cfg.sort = method;
        self.clone()
    }
    pub fn output(&mut self, path: &str) -> Self {
        self.cfg.user_out = path.to_string();
        self.clone()
    }
    pub fn create_archive_of(&mut self, paths: &[&str]) {
        self.cfg.mode = Mode::Compress;
        let paths = paths.iter().map(PathBuf::from).collect::<Vec<PathBuf>>();
        self.cfg.inputs.extend_from_slice(&paths);

        self.cfg.dir_out = fmt_root_output_dir(self.cfg.arch, self.cfg.mode, &self.cfg.user_out, &self.cfg.inputs[0]);

        self.cfg.print();

        match self.cfg.arch {
            Arch::Solid    => { SolidArchiver::new(self.cfg.clone()).create_archive(); }
            Arch::NonSolid => { Archiver::new(self.cfg.clone()).create_archive();      }
        }  
    }
    pub fn extract_archive_of(&mut self, paths: &[&str]) {
        self.cfg.mode = Mode::Decompress;
        let paths = paths.iter().map(PathBuf::from).collect::<Vec<PathBuf>>();
        self.cfg.inputs.extend_from_slice(&paths);

        self.cfg.dir_out = fmt_root_output_dir(self.cfg.arch, self.cfg.mode, &self.cfg.user_out, &self.cfg.inputs[0]);
        println!("{}", self.cfg.dir_out);

        self.cfg.print();
        match self.cfg.arch {
            Arch::Solid    => { SolidExtractor::new(self.cfg.clone()).extract_archive(); }
            Arch::NonSolid => { Extractor::new(self.cfg.clone()).extract_archive();      }
        }  
    }

    // Used in binary
    pub fn new_with_cfg(cfg: Config) -> Prisirv {
        Prisirv { cfg }
    }
    pub fn create_archive(self) {
        match self.cfg.arch {
            Arch::Solid    => { SolidArchiver::new(self.cfg).create_archive(); }
            Arch::NonSolid => { Archiver::new(self.cfg).create_archive();      }
        }  
    }
    pub fn extract_archive(self) {
        match self.cfg.arch {
            Arch::Solid    => { SolidExtractor::new(self.cfg).extract_archive(); }
            Arch::NonSolid => { Extractor::new(self.cfg).extract_archive();      }
        }  
    }
}