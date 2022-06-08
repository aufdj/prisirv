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
pub mod error;
mod fv;
mod block;
mod cm;
mod lzw;
pub mod archiveinfo;
mod constant;

use std::{
    fmt,
    path::PathBuf,
};

use crate::{
    archive::Archiver,
    extract::Extractor,
    archiveinfo::ArchiveInfo,
    filedata::FileData,
    config::{Config, Mode, Method},
    sort::{Sort, sort_files},
    error::{
        ConfigError, 
        ArchiveError, 
    },
    formatting::fmt_root_output,
    constant::Version,
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
    pub fn threads(mut self, count: usize) -> Result<Self, ConfigError> {
        if count > 0 || count < 128 {
            self.cfg.threads = count;
        }
        else {
            return Err(ConfigError::OutOfRangeThreadCount(count));
        }
        Ok(self)
    }

    /// Suppress output other than errors.
    pub fn quiet(mut self) -> Self {
        self.cfg.quiet = true;
        self
    }

    /// Allow file clobbering.
    pub fn clobber(mut self) -> Self {
        self.cfg.clobber = true;
        self
    }

    pub fn lzw(mut self) -> Self {
        self.cfg.method = Method::Lzw;
        self
    }

    pub fn store(mut self) -> Self {
        self.cfg.method = Method::Store;
        self
    }
    

    /// Choose block size in bytes.
    pub fn block_size(mut self, size: usize) -> Self {
        self.cfg.blk_sz = size;
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

    /// Choose inputs.
    pub fn inputs(mut self, paths: &[&str]) -> Self {
        self.cfg.inputs = paths.iter()
            .map(PathBuf::from)
            .map(FileData::new)
            .collect::<Vec<FileData>>();
        self
    }

    /// Choose existing archive.
    pub fn ex_arch(mut self, path: &str) -> Self {
        self.cfg.ex_arch = FileData::new(PathBuf::from(path));
        self
    }

    /// Create an archive from inputs.
    pub fn create_archive(mut self) -> Result<(), ArchiveError> {
        self.cfg.mode = Mode::CreateArchive;
        self.cfg.ex_arch = self.cfg.inputs[0].clone();
        self.cfg.out = fmt_root_output(&self.cfg);
        println!("{}", self.cfg);
        sort_inputs(&mut self.cfg.inputs, self.cfg.sort);
        Archiver::new(self.cfg)?.create_archive()?;
        Ok(())
    }

    /// Extract an archive.
    pub fn extract_archive(mut self) -> Result<(), ArchiveError> {
        self.cfg.mode = Mode::ExtractArchive;
        self.cfg.out = fmt_root_output(&self.cfg);
        println!("{}", self.cfg);
        Extractor::new(self.cfg)?.extract_archive()?;
        Ok(())
    }

    /// Append inputs to archive.
    pub fn append_files(mut self) -> Result<(), ArchiveError> {
        self.cfg.mode = Mode::AppendFiles;
        println!("{}", self.cfg);
        sort_inputs(&mut self.cfg.inputs, self.cfg.sort);
        Archiver::new(self.cfg)?.append_files()?;
        Ok(())
    }

    /// Extract inputs from archive.
    pub fn extract_files(mut self) -> Result<(), ArchiveError> {
        self.cfg.mode = Mode::ExtractFiles;
        self.cfg.out = fmt_root_output(&self.cfg);
        println!("{}", self.cfg);
        Extractor::new(self.cfg)?.extract_files()?;
        Ok(())
    }

    /// Append inputs to archive.
    pub fn merge_archives(mut self) -> Result<(), ArchiveError> {
        self.cfg.mode = Mode::MergeArchives;
        println!("{}", self.cfg);
        Archiver::new(self.cfg)?.merge_archives()?;
        Ok(())
    }

    /// Get information about archive.
    pub fn info(mut self) -> Result<ArchiveInfo, ArchiveError> {
        self.cfg.mode = Mode::ListArchive;
        println!("{}", self.cfg);
        ArchiveInfo::new(&self.cfg.ex_arch)
    }

    /// Visualize file.
    pub fn fv(mut self) -> Result<(), ArchiveError> {
        self.cfg.mode = Mode::Fv;
        println!("{}", self.cfg);
        self.cfg.out = fmt_root_output(&self.cfg);
        sort_inputs(&mut self.cfg.inputs, self.cfg.sort);
        fv::new(&self.cfg)?;
        Ok(())
    }
}

impl fmt::Display for Prisirv {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let version = Version::current();
        write!(f, "
         ______   ______     ________  ______    ________  ______    __   __     
        /_____/\\ /_____/\\   /_______/\\/_____/\\  /_______/\\/_____/\\  /_/\\ /_/\\    
        \\:::_ \\ \\\\:::_ \\ \\  \\__.::._\\/\\::::_\\/_ \\__.::._\\/\\:::_ \\ \\ \\:\\ \\\\ \\ \\   
         \\:(_) \\ \\\\:(_) ) )_   \\::\\ \\  \\:\\/___/\\   \\::\\ \\  \\:(_) ) )_\\:\\ \\\\ \\ \\  
          \\: ___\\/ \\: __ `\\ \\  _\\::\\ \\__\\_::._\\:\\  _\\::\\ \\__\\: __ `\\ \\\\:\\_/.:\\ \\ 
           \\ \\ \\    \\ \\ `\\ \\ \\/__\\::\\__/\\ /____\\:\\/__\\::\\__/\\\\ \\ `\\ \\ \\\\ ..::/ / 
            \\_\\/     \\_\\/ \\_\\/\\________\\/ \\_____\\/\\________\\/ \\_\\/ \\_\\/ \\___/_(
                
        Prisirv {version}
        Copyright (C) 2022 aufdj
        
        This program is free software: you can redistribute it and/or modify
        it under the terms of the GNU General Public License as published by
        the Free Software Foundation, either version 3 of the License, or
        (at your option) any later version.
        
        This program is distributed in the hope that it will be useful,
        but WITHOUT ANY WARRANTY; without even the implied warranty of
        MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
        GNU General Public License for more details.
        
        You should have received a copy of the GNU General Public License
        along with this program.  If not, see <https://www.gnu.org/licenses/>.
        
        Source code available at https://github.com/aufdj/prisirv
        

        USAGE: PROG_NAME [REQUIRED] [OPTIONS|FLAGS]
    
        REQUIRED:
           c,  create           Create archive
           x,  extract          Extract archive
           a,  append           Append files to archive
           p,  pick             Extract select files from archive
           m,  merge            Merge archives together
           ls                   List info about archive
           fv                   Visualize file
                
        One of the above commands must be used, and all are mutually exclusive.
                
        OPTIONS:
          -i,    -inputs        Specify list of input files/dirs
          -out,  -output-path   Specify output path
          -mem,  -memory        Specify memory usage     (Default - 2 (15 MiB))
          -blk,  -block-size    Specify block size       (Default - 10 MiB)
          -threads              Specify thread count     (Default - 4)
          -sort                 Sort files               (Default - none)
                
        Options '-memory', '-block-size', and '-sort' have no effect on extraction.
                
        FLAGS:
          -q,  -quiet           Suppresses output other than errors
          -clobber              Allow file clobbering
          -file-align           Truncate blocks to align with file boundaries
          -lzw                  Use LZW compression method
                
        Flags '-file-align' and '-lzw' have no effect on extraction.
                
        Sorting Methods:
          -sort ext      Sort by extension
          -sort name     Sort by name
          -sort len      Sort by length
          -sort prt n    Sort by nth parent directory
          -sort crtd     Sort by creation time
          -sort accd     Sort by last access time
          -sort mod      Sort by last modification time
                
        Memory Options:
          -mem 0  6 MB   -mem 5  99 MB
          -mem 1  9 MB   -mem 6  195 MB
          -mem 2  15 MB  -mem 7  387 MB
          -mem 3  27 MB  -mem 8  771 MB
          -mem 4  51 MB  -mem 9  1539 MB


        EXAMPLES:
                
        Compress file [/foo/bar.txt] and directory [/baz] into archive [/foo/qux.prsv], 
        sorting files by creation time:
               
            prisirv create -inputs /foo/bar.txt /baz -sort crtd -output-path qux
               
        Extract archive [/foo/qux.prsv]:
               
            prisirv extract /foo/qux.prsv
               
        Append file [foo.txt] to archive [/foo/qux.prsv]:
               
            prisirv append-files /foo/qux.prsv -inputs foo.txt
               
        Extract file [foo.txt] from archive [/foo/qux.prsv]:
               
            prisirv extract-files /foo/qux.prsv -inputs foo.txt
               
        List information about archive [/foo/qux.prsv]:
               
            prisirv ls /foo/qux.prsv
               
        Visualize file [foo.bin]:
               
            prisirv fv foo.bin
        
        "
        )
    }
}

fn sort_inputs(inputs: &mut Vec<FileData>, sort: Sort) {
    while expand(inputs).is_some() {}
    inputs.sort_by(|f1, f2|
        sort_files(&f1.path, &f2.path, sort).unwrap()
    );
}

fn expand(inputs: &mut Vec<FileData>) -> Option<usize> {
    let mut index: Option<usize> = None;

    for (i, input) in inputs.iter_mut().enumerate() {
        if let Ok(dir) = input.path.read_dir() {
            for data in dir.map(FileData::from) {
                inputs.push(data);
            }
            index = Some(i);
            break;
        }
    }
    
    if let Some(i) = index {
        inputs.swap_remove(i);
    }
    index
}