use std::{
    fmt,
    path::PathBuf,
};

use crate::{
    sort::Sort,
    error::ConfigError,
    filedata::FileData,
    constant::Version,
    archiveinfo::ArchiveInfo,
};


/// Parsing states.
enum Parse {
    None,
    CreateArchive,
    ExtractArchive,
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
    MergeArchives,
    Verbose,
}

#[derive(PartialEq, Copy, Clone, Debug)]
pub enum Mode {
    CreateArchive,
    ExtractArchive,
    AppendFiles,
    ExtractFiles,
    MergeArchives,
    ListArchive,
    Fv,
    None,
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
    pub ex_info:    ArchiveInfo,   // Info about existing archive
    pub fv:         Fv,
    pub verbose:    bool,
}
impl Config {
    /// Create a new Config with the specified command line arguments.
    pub fn new(args: &[String]) -> Result<Config, ConfigError> {
        let mut parser = Parse::None;
        let mut cfg    = Config::default();
        
        for arg in args.iter() {
            match arg.as_str() {
                "c" | "create" => {
                    parser = Parse::CreateArchive;
                }
                "x" | "extract" => {
                    parser = Parse::ExtractArchive;
                    continue;
                }
                "a" | "append" => {
                    parser = Parse::AppendFiles;
                    continue;
                }
                "p" | "pick" => {
                    parser = Parse::ExtractFiles;
                    continue;
                }
                "m" | "merge" => {
                    parser = Parse::MergeArchives;
                    continue;
                }
                "ls" | "list" => {
                    parser = Parse::List;
                    continue;
                }
                "-verbose" => {
                    parser = Parse::Verbose;
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
                "-sort" => {
                    parser = Parse::Sort;
                    continue;
                }, 
                "-out" | "-output-path" => {
                    parser = Parse::DirOut;
                    continue;
                },
                "-q" | "-quiet" => {
                    parser = Parse::Quiet;
                }
                "-clobber" => {
                    parser = Parse::Clobber;
                }
                "-file-align" => {
                    parser = Parse::Align;
                }
                "-lzw" => {
                    parser = Parse::Lzw;
                }
                "-store" => {
                    parser = Parse::Store;
                }
                _ => {},
            }
            match parser {
                Parse::CreateArchive => {
                    cfg.mode = Mode::CreateArchive;
                }
                Parse::ExtractArchive => {
                    cfg.mode = Mode::ExtractArchive;
                    cfg.ex_arch = FileData::new(PathBuf::from(arg));
                }
                Parse::AppendFiles => {
                    cfg.mode = Mode::AppendFiles;
                    cfg.ex_arch = FileData::new(PathBuf::from(arg));
                    cfg.ex_arch.seg_beg = !0; // Don't truncate archive
                    cfg.clobber = true;
                }
                Parse::ExtractFiles => {
                    cfg.mode = Mode::ExtractFiles;
                    cfg.ex_arch = FileData::new(PathBuf::from(arg));
                }
                Parse::MergeArchives => {
                    cfg.mode = Mode::MergeArchives; 
                    cfg.ex_arch = FileData::new(PathBuf::from(arg));
                    cfg.ex_arch.seg_beg = !0; // Don't truncate archive
                    cfg.clobber = true;
                }
                Parse::List => {
                    cfg.mode = Mode::ListArchive;
                    cfg.ex_arch = FileData::new(PathBuf::from(arg));
                }
                Parse::Verbose => {
                    cfg.verbose = true;
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
                Parse::Inputs => {
                    cfg.inputs.push(FileData::new(PathBuf::from(arg)));
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
                Parse::DirOut => {
                    cfg.user_out = arg.to_string();
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
                Parse::Lzw => {
                    cfg.method = Method::Lzw;
                }
                Parse::Store => {
                    cfg.method = Method::Store;
                }
                Parse::None => {},
            }
        }

        if cfg.ex_arch.path.is_file() && !(cfg.mode == Mode::ExtractArchive || cfg.mode == Mode::ExtractFiles) {
            cfg.ex_info = ArchiveInfo::new(&cfg.ex_arch)?;
        }

        match cfg.mode {
            Mode::ListArchive | Mode::ExtractArchive | Mode::None => {},
            _ => {
                if cfg.inputs.is_empty() {
                    return Err(ConfigError::InputsEmpty);
                }
        
                for input in cfg.inputs.iter() {
                    if !(input.path.is_file() || input.path.is_dir()) {
                        return Err(ConfigError::InvalidInput(input.path.clone()));
                    }
                } 
            }
        }
        
        Ok(cfg)
    }
}
impl fmt::Display for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if !self.quiet {
            match self.mode {
                Mode::CreateArchive => {
                    let version = Version::current();
                    write!(f, "
                        \rPrisirv {version}
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
                Mode::MergeArchives => {
                    write!(f, "
                        \r Merging into archive {}:",
                        self.ex_arch.path.display()
                    )?;
                    for input in self.inputs.iter() {
                        write!(f, "
                            \r    {}", 
                            input.path.display(),
                        )?;
                    }
                    Ok(())
                }
                Mode::ExtractFiles => {
                    write!(f, "
                        \r=============================================================
                        \r Extracting files from archive {}:", 
                        self.ex_arch.path.display()
                    )?;
                    for input in self.inputs.iter() {
                        write!(f, "
                            \r    {}", 
                            input.path.display(),
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
                    Ok(())
                }
                Mode::Fv => {
                    Ok(())
                }
                Mode::None => {
                    Ok(())
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
            mode:       Mode::None,
            quiet:      false,
            clobber:    false,
            threads:    4,
            inputs:     Vec::new(),
            out:        FileData::default(),
            align:      Align::Fixed,
            method:     Method::Cm,
            ex_arch:    FileData::default(),
            ex_info:    ArchiveInfo::default(),
            fv:         Fv::default(),
            verbose:    false,
        }
    }
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

