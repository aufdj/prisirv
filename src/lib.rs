mod encoder; 
mod decoder;      
mod predictor; 
mod match_model;  
mod statemap;
mod apm;   
mod mixer;  
mod hash_table; 
mod logistic;      
mod metadata;
mod archive;       
mod solid_archive; 
mod extract;
mod solid_extract;
mod tables;  
mod sort;
mod buffered_io;   
mod formatting;    
mod threads;
mod progress; 
pub mod config;     
pub mod crc32;
mod error;

use std::path::PathBuf;

use crate::{
    archive::Archiver,
    extract::Extractor,
    solid_archive::SolidArchiver,
    solid_extract::SolidExtractor,
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

/// Prisirv API. Allows for creating or extracting a Prisirv archive
/// using method chaining syntax or by supplying an existing Config.
#[derive(Clone)]
pub struct Prisirv {
    cfg: Config,
}
impl Prisirv {
    /// Create a new Prisirv archiver or extractor with an empty Config.
    pub fn new() -> Prisirv {
        Prisirv { cfg: Config::new_empty() }
    }

    /// Create a solid archive instead of non-solid.
    pub fn solid(&mut self) -> &mut Self {
        self.cfg.arch = Arch::Solid;
        &mut *self
    }

    /// Choose number of threads to use.
    pub fn threads(&mut self, count: usize) -> &mut Self {
        self.cfg.threads = count;
        &mut *self
    }

    /// Supress output other than errors.
    pub fn quiet(&mut self) -> &mut Self {
        self.cfg.quiet = true;
        &mut *self
    }

    /// Allow clobbering of files.
    pub fn clobber(&mut self) -> &mut Self {
        self.cfg.clbr = true;
        &mut *self
    }

    /// Choose block size in MiB.
    pub fn block_size(&mut self, size: usize) -> &mut Self {
        self.cfg.blk_sz = size*1024*1024;
        &mut *self
    }

    /// Choose memory option (0..9)
    pub fn memory(&mut self, mem: u64) -> &mut Self {
        if mem <= 9 {
            self.cfg.mem = 1 << (20 + mem);
        }
        else { error::invalid_memory_option(); } 
        &mut *self
    }

    /// Sort files before solid archiving.
    pub fn sort(&mut self, method: Sort) -> &mut Self {
        self.cfg.sort = method;
        &mut *self
    }

    /// Choose an output path.
    pub fn output(&mut self, path: &str) -> &mut Self {
        self.cfg.user_out = path.to_string();
        &mut *self
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
impl Default for Prisirv {
    fn default() -> Self {
        Self::new()
    }
}
