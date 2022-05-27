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
pub mod archiveinfo;
mod constant;

use std::path::{Path, PathBuf};

use crate::{
    archive::Archiver,
    extract::Extractor,
    archiveinfo::ArchiveInfo,
    filedata::FileData,
    config::{Config, Mode},
    sort::{Sort, sort_files},
    error::{
        ConfigError, 
        ArchiveError, 
        ExtractError
    },
    formatting::fmt_root_output,
};


/// Prisirv API. Allows for creating or extracting a Prisirv archive
/// using method chaining syntax or by supplying an existing Config.
#[derive(Clone, Default)]
pub struct Prisirv {
    cfg: Config,
}
impl Prisirv {
    /// Create a Prisirv archiver or extractor with an existing Config.
    pub fn new(cfg: Config) -> Prisirv {
        Prisirv {
            cfg 
        }
    }

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

    pub fn inputs(mut self, paths: &[&str]) -> Self {
        self.cfg.inputs = paths.iter()
            .map(PathBuf::from)
            .map(FileData::new)
            .collect::<Vec<FileData>>();
        self
    }

    pub fn ex_arch(mut self, path: &str) -> Self {
        self.cfg.ex_arch = FileData::new(PathBuf::from(path));
        self
    }

    /// Create an archive from inputs specified in Config.
    pub fn create_archive(mut self) -> Result<(), ArchiveError> {
        self.cfg.mode = Mode::CreateArchive;
        self.cfg.ex_arch = self.cfg.inputs[0].clone();
        self.cfg.out = fmt_root_output(&self.cfg);
        println!("{}", self.cfg);

        self.cfg.inputs = sort_inputs(&self.cfg);

        Archiver::new(self.cfg).create_archive()?;
        Ok(())
    }

    /// Extract inputs specified in Config.
    pub fn extract_archive(mut self) -> Result<(), ExtractError> {
        self.cfg.mode = Mode::ExtractArchive;
        self.cfg.out = fmt_root_output(&self.cfg);
        println!("{}", self.cfg);
        Extractor::new(self.cfg).extract_archive()?;
        Ok(())
    }

    pub fn append_files(mut self) -> Result<(), ArchiveError> {
        self.cfg.mode = Mode::AppendFiles;
        println!("{}", self.cfg);

        self.cfg.inputs = sort_inputs(&self.cfg);

        Archiver::new(self.cfg).append_files()?;
        Ok(())
    }

    pub fn extract_files(mut self) -> Result<(), ExtractError> {
        self.cfg.mode = Mode::ExtractFiles;
        self.cfg.out = fmt_root_output(&self.cfg);
        println!("{}", self.cfg);
        Extractor::new(self.cfg).extract_files()?;
        Ok(())
    }

    pub fn info(mut self) -> Result<ArchiveInfo, ExtractError> {
        self.cfg.mode = Mode::ListArchive;
        println!("{}", self.cfg);
        ArchiveInfo::new(&self.cfg.ex_arch)
    }

    pub fn fv(mut self) -> Result<(), ExtractError> {
        self.cfg.mode = Mode::Fv;
        println!("{}", self.cfg);
        self.cfg.out = fmt_root_output(&self.cfg);
        self.cfg.inputs = sort_inputs(&self.cfg);
        fv::new(&self.cfg)?;
        Ok(())
    }
}

fn sort_inputs(cfg: &Config) -> Vec<FileData> {
    let mut files = Vec::new();
    collect_files(&cfg.inputs, &mut files);
    files.sort_by(|f1, f2|
        sort_files(&f1.path, &f2.path, cfg.sort).unwrap()
    );
    files
}

/// Recursively collect all files into a vector for sorting before compression.
pub fn collect_files(inputs: &[FileData], files: &mut Vec<FileData>) {
    // Group files and directories 
    let (fi, dirs): (Vec<FileData>, Vec<FileData>) =
        inputs.iter().cloned()
        .partition(|f| f.path.is_file());

    // Walk through directories and collect all files
    for file in fi.into_iter() {
        files.push(file);
    }
    for dir in dirs.iter() {
        collect(&dir.path, files);
    }
}
fn collect(dir_in: &Path, files: &mut Vec<FileData>) {
    let (fi, dirs): (Vec<FileData>, Vec<FileData>) =
        dir_in.read_dir().unwrap()
        .map(|d| FileData::new(d.unwrap().path()))
        .partition(|f| f.path.is_file());

    for file in fi.into_iter() {
        files.push(file);
    }
    for dir in dirs.iter() {
        collect(&dir.path, files);
    }
}
