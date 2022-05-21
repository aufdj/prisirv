use std::{
    fmt,
    path::{Path, PathBuf},
};

use crate::{
    sort::{Sort, sort_files}, 
    formatting::fmt_root_output,
    error::ConfigError,
    filedata::FileData,
    archiveinfo::ArchiveInfo,
};


/// Parsing states.
enum Parse {
    None,
    Compress,
    Decompress,
    DirOut,
    Sort,
    Inputs,
    Quiet,
    Clobber,
    Mem,
    Lvl,
    BlkSz,
    Threads,
    List,
    Fv,
    ColScale,
    Width,
    Align,
    Lzw,
    Store,
    AppendFiles,
    ExtractFiles,
}

#[derive(PartialEq, Copy, Clone, Debug)]
pub enum Mode {
    CreateArchive,
    ExtractArchive,
    AppendFiles,
    ExtractFiles,
    ListArchive,
    Fv,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Align {
    File,
    Fixed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    Cm,
    Lzw,
    Store,
}
impl Default for Method {
    fn default() -> Method {
        Method::Cm 
    }
}
impl From<u8> for Method {
    fn from(num: u8) -> Method {
        match num {
            0 => Method::Cm,
            1 => Method::Lzw,
            _ => Method::Store,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Fv {
    pub col_scale:  f64,
    pub width:      i32,
}
impl Default for Fv {
    fn default() -> Fv {
        Fv {
            col_scale:  10.0,
            width:      512,
        }
    }
}


/// User defined configuration settings.
#[derive(Clone, Debug)]
pub struct Config {
    pub sort:       Sort,          // Sorting method
    pub user_out:   String,        // User specified output directory (optional)
    pub out:        FileData,      // Output
    pub inputs:     Vec<FileData>, // Inputs to be archived or extracted
    pub quiet:      bool,          // Suppresses output other than errors
    pub mode:       Mode,          // Create archive, extract files, add files, extract files
    pub mem:        u64,           // Memory usage 
    pub clobber:    bool,          // Allow clobbering files
    pub blk_sz:     usize,         // Block size
    pub threads:    usize,         // Maximum number of threads
    pub align:      Align,         // Block size exactly as specified or truncated to file boundary
    pub method:     Method,        // Compression method, 0 = Context Mixing, 1 = LZW, 2 = No compression
    pub ex_arch:    FileData,      // An existing Prisirv archive
    pub insert_id:  u32,           // Starting id of new blocks appended to archive
    pub insert_pos: u64,           // Where to insert new blocks
    pub fv:         Fv,
}
impl Config {
    /// Create a new Config with the specified command line arguments.
    pub fn new(args: &[String]) -> Result<Config, ConfigError> {
        if args.is_empty() { 
            print_program_info(); 
        }

        let mut parser = Parse::None;
        let mut cfg    = Config::default();
        
        for arg in args.iter() {
            match arg.as_str() {
                "-sort" => {
                    parser = Parse::Sort;
                    continue;
                }, 
                "-out" | "-output-path" => {
                    parser = Parse::DirOut;
                    continue;
                },     
                "-i" | "-inputs" => { 
                    parser = Parse::Inputs;
                    continue;
                },
                "-mem" | "-memory" => {
                    parser = Parse::Mem;
                    continue;
                }
                "-blk" | "-block-size" => {
                    parser = Parse::BlkSz;
                    continue;
                } 
                "-threads" => {
                    parser = Parse::Threads;
                    continue;
                }
                "fv" => {
                    parser = Parse::Fv;
                }
                "-col-scale" => {
                    parser = Parse::ColScale;
                    continue;
                }
                "-width" => {
                    parser = Parse::Width;
                    continue;
                }
                "-lzw" => {
                    parser = Parse::Lzw;
                }
                "append" => {
                    parser = Parse::AppendFiles;
                    continue;
                }
                "-store" => {
                    parser = Parse::Store;
                }
                "create" => {
                    parser = Parse::Compress;
                }
                "extract" => {
                    parser = Parse::Decompress;
                }
                "-q" | "-quiet" => {
                    parser = Parse::Quiet;
                }
                "-clobber" => {
                    parser = Parse::Clobber;
                }
                "-file-align" => {
                    parser = Parse::Align;
                }
                "ls" | "list" => {
                    parser = Parse::List;
                    continue;
                }
                "extract-files" => {
                    parser = Parse::ExtractFiles;
                    continue;
                }
                "help" => {
                    print_program_info();
                }
                _ => {},
            }
            match parser {
                Parse::Sort => {
                    match arg.as_str() {
                        "ext"  => cfg.sort = Sort::Ext,
                        "name" => cfg.sort = Sort::Name,
                        "len"  => cfg.sort = Sort::Len,
                        "crtd" => cfg.sort = Sort::Created,
                        "accd" => cfg.sort = Sort::Accessed,
                        "mod"  => cfg.sort = Sort::Modified,
                        "prt"  => {
                            parser = Parse::Lvl;
                            cfg.sort = Sort::PrtDir(1);
                        },
                        m => { 
                            return Err(ConfigError::InvalidSortCriteria(m.to_string()));
                        }
                    }
                }
                Parse::Lvl => {
                    if let Ok(lvl) = arg.parse::<usize>() {
                        cfg.sort = Sort::PrtDir(lvl);
                    }
                    else {
                        return Err(ConfigError::InvalidLvl(arg.to_string()));
                    }
                }
                Parse::Mem => {
                    if let Ok(mem) = arg.parse::<u64>() {
                        if mem <= 9 {
                            cfg.mem = 1 << (20 + mem);
                        }
                        else {
                            return Err(ConfigError::OutOfRangeMemory(mem));
                        }
                    }
                    else {
                        return Err(ConfigError::InvalidMemory(arg.to_string()));
                    }
                } 
                Parse::BlkSz => {
                    let size  = arg.chars().filter(|c|  c.is_numeric()).collect::<String>();
                    let scale = arg.chars().filter(|c| !c.is_numeric()).collect::<String>();

                    let scale = match scale.as_str() {
                        "B" => { 1 },
                        "K" => { 1024 },
                        "M" => { 1024*1024 },
                        "G" => { 1024*1024*1024 },
                        _ => return Err(ConfigError::InvalidBlockMagnitude(scale)),
                    };

                    if let Ok(size) = size.parse::<usize>() {
                        cfg.blk_sz = size * scale;
                    }
                    else {
                        return Err(ConfigError::InvalidBlockSize(arg.to_string()));
                    }
                }
                Parse::Threads => {
                    if let Ok(count) = arg.parse::<usize>() {
                        if count > 0 || count < 128 {
                            cfg.threads = count;
                        }
                        else {
                            return Err(ConfigError::OutOfRangeThreadCount(count));
                        }
                    }
                    else {
                        return Err(ConfigError::InvalidThreadCount(arg.to_string()));
                    }
                }
                Parse::List => {
                    cfg.mode = Mode::ListArchive;
                    cfg.ex_arch = FileData::new(PathBuf::from(arg));
                }
                Parse::Fv => {
                    cfg.mode = Mode::Fv;
                    parser = Parse::Inputs;
                }
                Parse::ColScale => {
                    if let Ok(c) = arg.parse::<f64>() {
                        cfg.fv.col_scale = c;
                    }
                    else {
                        return Err(ConfigError::InvalidColorScale(arg.to_string()));
                    }
                }
                Parse::Width => {
                    if let Ok(w) = arg.parse::<i32>() {
                        cfg.fv.width = w;
                    }
                    else {
                        return Err(ConfigError::InvalidImageWidth(arg.to_string()));
                    }
                }
                Parse::AppendFiles => {
                    cfg.mode = Mode::AppendFiles; 
                    cfg.ex_arch = FileData::new(PathBuf::from(arg));
                    let info = ArchiveInfo::new(&cfg.ex_arch)?;
                    cfg.insert_id = info.block_count();
                    cfg.insert_pos = info.end_of_data();
                }
                Parse::ExtractFiles => {
                    cfg.mode = Mode::ExtractFiles;
                    cfg.ex_arch = FileData::new(PathBuf::from(arg));
                }
                Parse::Lzw => {
                    cfg.method = Method::Lzw;
                }
                Parse::Store => {
                    cfg.method = Method::Store;
                }
                Parse::Compress => {
                    cfg.mode = Mode::CreateArchive;
                }
                Parse::Decompress => {
                    cfg.mode = Mode::ExtractArchive;
                }
                Parse::DirOut => {
                    cfg.user_out = arg.to_string();
                }
                Parse::Inputs => {
                    cfg.inputs.push(FileData::new(PathBuf::from(arg)));
                }
                Parse::Quiet => {
                    cfg.quiet = true;
                }
                Parse::Clobber => {
                    cfg.clobber = true;
                }
                Parse::Align => {
                    cfg.align = Align::File;
                }
                Parse::None => {},
            }
        }

        if cfg.mode != Mode::ListArchive {
            if cfg.inputs.is_empty() {
                return Err(ConfigError::InputsEmpty);
            }
    
            for input in cfg.inputs.iter() {
                if !(input.path.is_file() || input.path.is_dir()) {
                    return Err(ConfigError::InvalidInput(input.path.clone()));
                }
            }
    
            cfg.out = fmt_root_output(&cfg);
        }
        
        println!("{cfg}");

        if cfg.mode == Mode::CreateArchive || cfg.mode == Mode::AppendFiles {
            let mut files = Vec::new();
            collect_files(&cfg.inputs, &mut files);

            files.sort_by(|f1, f2| 
                sort_files(&f1.path, &f2.path, cfg.sort).unwrap()
            );

            cfg.inputs = files;
        }
        
        Ok(cfg)
    }
}
impl fmt::Display for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if !self.quiet {
            match self.mode {
                Mode::CreateArchive => {
                    write!(f, "
                        \r=============================================================
                        \r Creating Archive of Inputs:"
                    )?;
                    for input in self.inputs.iter() {
                        write!(f, "
                            \r    {} ({})", 
                            input.path.display(),
                            if input.path.is_file() { 
                                "File" 
                            }
                            else if input.path.is_dir() { 
                                "Directory" 
                            }
                            else { 
                                "" 
                            }
                        )?;
                    }
                    let (size, suffix) = format(self.blk_sz);
                    write!(f, "\n
                        \r Output Path:     {}
                        \r Method:          {}
                        \r Sorting by:      {}
                        \r Memory Usage:    {} MiB
                        \r Block Size:      {} {}
                        \r Block Alignment: {}
                        \r Threads:         {}
                        \r=============================================================\n",
                        self.out.path.display(),
                        match self.method {
                            Method::Cm  => "Context Mixing",
                            Method::Lzw => "LZW",
                            _           => "No Compression",
                        },
                        match self.sort {
                            Sort::None      => "None",
                            Sort::Ext       => "Extension",
                            Sort::Name      => "Name",
                            Sort::Len       => "Length",
                            Sort::Created   => "Creation time",
                            Sort::Accessed  => "Last accessed time",
                            Sort::Modified  => "Last modified time",
                            Sort::PrtDir(_) => "Parent Directory",
                        },
                        3 + (self.mem >> 20) * 3,
                        size, suffix,
                        match self.align {
                            Align::File  => "File",
                            Align::Fixed => "Fixed",
                        },
                        self.threads
                    )
                }
                Mode::ExtractArchive => { 
                    write!(f, "
                        \r=============================================================
                        \r Extracting Archive of Inputs:"
                    )?;
                    for input in self.inputs.iter() {
                        write!(f, "
                            \r    {} ({})", 
                            input.path.display(),
                            if input.path.is_file() { 
                                "File" 
                            }
                            else if input.path.is_dir() { 
                                "Directory" 
                            }
                            else { 
                                "" 
                            }
                        )?;
                    }
                    write!(f, "\n
                        \r Output Path: {}
                        \r Threads:     {}
                        \r=============================================================\n",
                        self.out.path.display(),
                        self.threads
                    )
                },
                Mode::AppendFiles => { 
                    write!(f, "
                        \r=============================================================
                        \r Adding to archive {}\n
                        \r Inputs:", 
                        self.ex_arch.path.display()
                    )?;
                    for input in self.inputs.iter() {
                        write!(f, "
                            \r    {} ({})", 
                            input.path.display(),
                            if input.path.is_file() { 
                                "File" 
                            }
                            else if input.path.is_dir() { 
                                "Directory" 
                            }
                            else { 
                                "" 
                            }
                        )?;
                    }
                    let (size, suffix) = format(self.blk_sz);
                    write!(f, "\n
                        \r Method:          {}
                        \r Sorting by:      {}
                        \r Memory Usage:    {} MiB
                        \r Block Size:      {} {}
                        \r Block Alignment: {}
                        \r Threads:         {}
                        \r=============================================================\n",
                        match self.method {
                            Method::Cm  => "Context Mixing",
                            Method::Lzw => "LZW",
                            _           => "No Compression",
                        },
                        match self.sort {
                            Sort::None      => "None",
                            Sort::Ext       => "Extension",
                            Sort::Name      => "Name",
                            Sort::Len       => "Length",
                            Sort::Created   => "Creation time",
                            Sort::Accessed  => "Last accessed time",
                            Sort::Modified  => "Last modified time",
                            Sort::PrtDir(_) => "Parent Directory",
                        },
                        3 + (self.mem >> 20) * 3,
                        size, suffix,
                        match self.align {
                            Align::File  => "File",
                            Align::Fixed => "Fixed",
                        },
                        self.threads
                    )
                },
                Mode::ExtractFiles => {
                    write!(f, "
                        \r=============================================================
                        \r Extracting file {} from archive {}", 
                        self.inputs[0].path.display(), self.ex_arch.path.display()
                    )?;
                    for input in self.inputs.iter() {
                        write!(f, "
                            \r    {} ({})", 
                            input.path.display(),
                            if input.path.is_file() { 
                                "File" 
                            }
                            else if input.path.is_dir() { 
                                "Directory" 
                            }
                            else { 
                                "" 
                            }
                        )?;
                    }
                    write!(f, "\n
                        \r Output Path: {}
                        \r Threads:     {}
                        \r=============================================================\n",
                        self.out.path.display(),
                        self.threads
                    )
                },
                Mode::ListArchive => {
                    write!(f, "")
                }
                Mode::Fv => {
                    write!(f, "")
                }
            }
        }
        else {
            Ok(())
        }
    }
}
impl Default for Config {
    fn default() -> Config {
        Config {
            sort:       Sort::None,
            user_out:   String::new(),
            blk_sz:     10 << 20,
            mem:        1 << 22,
            mode:       Mode::CreateArchive,
            quiet:      false,
            clobber:    false,
            threads:    4,
            inputs:     Vec::new(),
            out:        FileData::default(),
            align:      Align::Fixed,
            method:     Method::Cm,
            ex_arch:    FileData::default(),
            insert_id:  0,
            insert_pos: 0,
            fv:         Fv::default(),
        }
    }
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


/// Print information about Prisirv.
fn print_program_info() {
    println!();
    println!("     ______   ______     ________  ______    ________  ______    __   __     
    /_____/\\ /_____/\\   /_______/\\/_____/\\  /_______/\\/_____/\\  /_/\\ /_/\\    
    \\:::_ \\ \\\\:::_ \\ \\  \\__.::._\\/\\::::_\\/_ \\__.::._\\/\\:::_ \\ \\ \\:\\ \\\\ \\ \\   
     \\:(_) \\ \\\\:(_) ) )_   \\::\\ \\  \\:\\/___/\\   \\::\\ \\  \\:(_) ) )_\\:\\ \\\\ \\ \\  
      \\: ___\\/ \\: __ `\\ \\  _\\::\\ \\__\\_::._\\:\\  _\\::\\ \\__\\: __ `\\ \\\\:\\_/.:\\ \\ 
       \\ \\ \\    \\ \\ `\\ \\ \\/__\\::\\__/\\ /____\\:\\/__\\::\\__/\\\\ \\ `\\ \\ \\\\ ..::/ / 
        \\_\\/     \\_\\/ \\_\\/\\________\\/ \\_____\\/\\________\\/ \\_\\/ \\_\\/ \\___/_(  
                                                                             ");
    println!("  
      Prisirv File Archiver
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
      along with this program.  If not, see <https://www.gnu.org/licenses/>.");
    println!("
      Source code available at https://github.com/aufdj/prisirv");
    println!();
    println!();
    println!("  USAGE: PROG_NAME [create|extract] [-inputs [..]] [OPTIONS|FLAGS]");
    println!();
    println!("  REQUIRED:");
    println!("     create                Create archive");
    println!("     extract               Extract archive");
    println!("    -i,     -inputs        Specify list of input files/dirs");
    println!();
    println!("  OPTIONS:");
    println!("    -out,   -output-path   Specify output path");
    println!("    -mem,   -memory        Specify memory usage     (Default - 2 (15 MiB))");
    println!("    -blk,   -block-size    Specify block size       (Default - 10 MiB)");
    println!("    -threads               Specify thread count     (Default - 4)");
    println!("    -sort                  Sort files               (Default - none)");
    println!("     append a              Append files to existing archive 'a'");
    println!("     extract-files a       Extract files from existing archive 'a'");
    println!();
    println!("  FLAGS:");
    println!("    -q,     -quiet         Suppresses output other than errors");
    println!("    -clobber               Allow file clobbering");
    println!("    -file-align            Truncate blocks to align with file boundaries");
    println!("    -lzw                   Use LZW compression method");
    println!();
    println!("  Sorting Methods:");
    println!("    -sort ext      Sort by extension");
    println!("    -sort name     Sort by name");
    println!("    -sort len      Sort by length");
    println!("    -sort prt n    Sort by nth parent directory");
    println!("    -sort crtd     Sort by creation time");
    println!("    -sort accd     Sort by last access time");
    println!("    -sort mod      Sort by last modification time");
    println!();
    println!("  Any sorting option specified for extraction will be ignored.");
    println!();
    println!("  Memory Options:");
    println!("    -mem 0  6 MB   -mem 5  99 MB");
    println!("    -mem 1  9 MB   -mem 6  195 MB");
    println!("    -mem 2  15 MB  -mem 7  387 MB");
    println!("    -mem 3  27 MB  -mem 8  771 MB");
    println!("    -mem 4  51 MB  -mem 9  1539 MB");
    println!();
    println!("  Extraction requires same memory option used for archiving.");
    println!("  Any memory option specified for extraction will be ignored."); 
    println!();
    println!("  EXAMPLE:");
    println!();
    println!("  Compress file [\\foo\\bar.txt] and directory [\\baz] into archive [\\foo\\arch], \n  sorting files by creation time:");
    println!();
    println!("      prisirv create -inputs \\foo\\bar.txt \\baz -output-path arch -sort crtd ");
    println!();
    println!("  Extract the archive:");
    println!();
    println!("      prisirv extract -inputs \\foo\\arch.prsv ");
    std::process::exit(0);
}

fn format(size: usize) -> (usize, String) {
    if size >= 1024*1024*1024 {
        (size/1024/1024/1024, String::from("GiB"))
    }
    else if size >= 1024*1024 {
        (size/1024/1024, String::from("MiB"))
    }
    else if size >= 1024 {
        (size/1024, String::from("KiB"))
    }
    else if size >= 1 {
        (size, String::from("B"))
    }
    else { 
        (0, String::from("")) 
    }
}

