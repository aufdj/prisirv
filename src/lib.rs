mod filedata;
mod archive;       
mod extract;
mod sort;
mod buffered_io;  
mod formatting;    
mod threads;
mod progress; 
pub mod config;     
pub mod crc32;
mod error;
mod fv;
mod block;
mod cm;
mod lzw;
mod archivemod;
mod archivescan;

use std::path::PathBuf;

use crate::{
    archive::Archiver,
    extract::Extractor,
    archivemod::ArchiveModifier,
    filedata::FileData,
    config::{Config, Mode},
    sort::Sort,
    error::{ConfigError, ExtractError},
    formatting::fmt_root_output,
};



/// Prisirv API. Allows for creating or extracting a Prisirv archive
/// using method chaining syntax or by supplying an existing Config.
#[derive(Clone, Default)]
pub struct Prisirv {
    cfg: Config,
}
impl Prisirv {
    /// Choose number of threads to use.
    pub fn threads(mut self, count: usize) -> Self {
        self.cfg.threads = count;
        self
    }

    /// Supress output other than errors.
    pub fn quiet(mut self) -> Self {
        self.cfg.quiet = true;
        self
    }

    /// Allow clobbering of files.
    pub fn clobber(mut self) -> Self {
        self.cfg.clobber = true;
        self
    }

    /// Choose block size in MiB.
    pub fn block_size(mut self, size: usize) -> Self {
        self.cfg.blk_sz = size*1024*1024;
        self
    }

    /// Choose memory option (0..9)
    pub fn memory(mut self, mem: u64) -> Result<Self, ConfigError> {
        if mem <= 9 {
            self.cfg.mem = 1 << (20 + mem);
        }
        else { 
            return Err(ConfigError::InvalidMemory(mem.to_string()));
        } 
        Ok(self)
    }

    /// Sort files before solid archiving.
    pub fn sort(mut self, method: Sort) -> Self {
        self.cfg.sort = method;
        self
    }

    /// Choose an output path.
    pub fn output(mut self, path: &str) -> Self {
        self.cfg.user_out = path.to_string();
        self
    }

    /// Create archive of supplied paths.
    pub fn create_archive_of(mut self, paths: &[&str]) {
        self.cfg.mode = Mode::CreateArchive;
        let paths = paths.iter()
            .map(PathBuf::from)
            .map(FileData::new)
            .collect::<Vec<FileData>>();

        self.cfg.inputs.extend_from_slice(&paths);

        self.cfg.out = fmt_root_output(&self.cfg);

        self.cfg.print();

        Archiver::new(self.cfg.clone()).create_archive();
    }

    /// Extract supplied paths.
    pub fn extract_archive_of(mut self, paths: &[&str]) -> Result<(), ExtractError> {
        self.cfg.mode = Mode::ExtractArchive;
        let paths = paths.iter()
            .map(PathBuf::from)
            .map(FileData::new)
            .collect::<Vec<FileData>>();

        self.cfg.inputs.extend_from_slice(&paths);

        self.cfg.out = fmt_root_output(&self.cfg);

        self.cfg.print();

        Extractor::new(self.cfg.clone()).extract_archive()?; 
        Ok(())
    }


    /// Create a Prisirv archiver or extractor with an existing Config.
    pub fn new(cfg: Config) -> Prisirv {
        Prisirv { 
            cfg 
        }
    }

    /// Create an archive from inputs specified in Config.
    pub fn create_archive(self) {
        Archiver::new(self.cfg).create_archive();  
    }

    /// Extract inputs specified in Config.
    pub fn extract_archive(self) -> Result<(), ExtractError> {
        Extractor::new(self.cfg).extract_archive()?; 
        Ok(())
    }

    pub fn add_files(self) {
        ArchiveModifier::new(self.cfg).add_files();
    }

    pub fn extract_files(self) -> Result<(), ExtractError>  {
        Extractor::new(self.cfg).extract_files()?; 
        Ok(())
    }
}
