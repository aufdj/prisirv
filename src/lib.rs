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

/// Mode (Compress | Decompress)
#[derive(PartialEq, Copy, Clone, Debug)]
pub enum Mode {
    Compress,
    Decompress,
}

/// Archive type (Solid | Non-Solid)
#[derive(PartialEq, Copy, Clone, Debug)]
pub enum Arch {
    Solid,
    NonSolid,
}

/// Main Prisirv API. Allows for creating or extracting a Prisirv archive
/// using method chaining syntax or by supplying an existing Config.
#[derive(Clone)]
pub struct Prisirv {
    cfg: Config,
}
impl Prisirv {
    /// Create a new Prisirv archiver or extractor with an empty Config.
    pub fn new() -> Prisirv {
        let cfg = Config::new_empty();
        Prisirv { cfg }
    }

    /// Create a solid archive instead of non-solid.
    pub fn solid(&mut self) -> Self {
        self.cfg.arch = Arch::Solid;
        self.clone()
    }

    /// Choose number of threads to use.
    pub fn threads(&mut self, count: usize) -> Self {
        self.cfg.threads = count;
        self.clone()
    }

    /// Supress output other than errors.
    pub fn quiet(&mut self) -> Self {
        self.cfg.quiet = true;
        self.clone()
    }

    /// Allow clobbering of files.
    pub fn clobber(&mut self) -> Self {
        self.cfg.clbr = true;
        self.clone()
    }

    /// Choose block size in MiB.
    pub fn block_size(&mut self, size: usize) -> Self {
        self.cfg.blk_sz = size;
        self.clone()
    }

    /// Choose memory option (0..9)
    pub fn memory(&mut self, mem: u64) -> Self {
        self.cfg.mem = mem;
        self.clone()
    }

    /// Sort files before solid archiving.
    pub fn sort(&mut self, method: Sort) -> Self {
        self.cfg.sort = method;
        self.clone()
    }

    /// Choose an output path.
    pub fn output(&mut self, path: &str) -> Self {
        self.cfg.user_out = path.to_string();
        self.clone()
    }

    /// Create archive of supplied paths.
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

    /// Extract supplied paths.
    pub fn extract_archive_of(&mut self, paths: &[&str]) {
        self.cfg.mode = Mode::Decompress;
        let paths = paths.iter().map(PathBuf::from).collect::<Vec<PathBuf>>();
        self.cfg.inputs.extend_from_slice(&paths);

        self.cfg.dir_out = fmt_root_output_dir(self.cfg.arch, self.cfg.mode, &self.cfg.user_out, &self.cfg.inputs[0]);

        self.cfg.print();
        match self.cfg.arch {
            Arch::Solid    => { SolidExtractor::new(self.cfg.clone()).extract_archive(); }
            Arch::NonSolid => { Extractor::new(self.cfg.clone()).extract_archive();      }
        }  
    }


    /// Create a Prisirv archiver or extractor with an existing Config.
    pub fn new_with_cfg(cfg: Config) -> Prisirv {
        Prisirv { cfg }
    }

    /// Create an archive from inputs specified in Config.
    pub fn create_archive(self) {
        match self.cfg.arch {
            Arch::Solid    => { SolidArchiver::new(self.cfg).create_archive(); }
            Arch::NonSolid => { Archiver::new(self.cfg).create_archive();      }
        }  
    }

    /// Extract inputs specified in Config.
    pub fn extract_archive(self) {
        match self.cfg.arch {
            Arch::Solid    => { SolidExtractor::new(self.cfg).extract_archive(); }
            Arch::NonSolid => { Extractor::new(self.cfg).extract_archive();      }
        }  
    }
}