use std::{
    fmt,
    path::PathBuf,
};

use crate::{
    sort::Sort,
    error::ConfigError,
    filedata::FileData,
    constant::Version,
};


/// Parsing states.
enum Parse {
    None,
    CreateArchive,
    AppendFiles,
    MergeArchives,
    ExtractArchive,
    ExtractFiles,
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
    Verbose,
    Fv,
    ColScale,
    Width,
    Align,
    Cm,
    Lzwc,
    Store,
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
    Lzwc,
    Lzws,
    Store,
}
impl Default for Method {
    fn default() -> Method {
        Method::Lzws
    }
}
impl From<u8> for Method {
    fn from(num: u8) -> Method {
        match num {
            0 => Method::Cm,
            1 => Method::Lzwc,
            2 => Method::Lzws,
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
    pub arch:       FileData,      // A Prisirv archive
    pub fv:         Fv,            // File visualization options
    pub verbose:    bool,          // Print verbose archive contents with 'ls'
}
impl Config {
    /// Create a new Config with the specified command line arguments.
    pub fn new(args: Vec<String>) -> Result<Config, ConfigError> {
        let mut parser = Parse::None;
        let mut cfg    = Config::default();
        
        for arg in args.into_iter() {
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
                "-cm" => {
                    parser = Parse::Cm;
                }
                "-lzwc" => {
                    parser = Parse::Lzwc;
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
                    cfg.arch = FileData::from(&arg);
                }
                Parse::AppendFiles => {
                    cfg.mode = Mode::AppendFiles;
                    cfg.arch = FileData::from(&arg);
                }
                Parse::ExtractFiles => {
                    cfg.mode = Mode::ExtractFiles;
                    cfg.arch = FileData::from(&arg);
                }
                Parse::MergeArchives => {
                    cfg.mode = Mode::MergeArchives; 
                    cfg.arch = FileData::from(&arg);
                }
                Parse::List => {
                    cfg.mode = Mode::ListArchive;
                    cfg.arch = FileData::from(&arg);
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
                        return Err(ConfigError::InvalidColorScale(arg));
                    }
                }
                Parse::Width => {
                    if let Ok(w) = arg.parse::<i32>() {
                        cfg.fv.width = w;
                    }
                    else {
                        return Err(ConfigError::InvalidImageWidth(arg));
                    }
                }
                Parse::Inputs => {
                    let path = PathBuf::from(&arg);
                    if path.exists() {
                        cfg.inputs.push(FileData::new(path));
                    }
                    else {
                        return Err(ConfigError::InvalidInput(path));
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
                        return Err(ConfigError::InvalidMemory(arg));
                    }
                } 
                Parse::BlkSz => {
                    let size  = arg.chars().filter(|c|  c.is_numeric()).collect::<String>();
                    let scale = arg.chars().filter(|c| !c.is_numeric()).collect::<String>();

                    let scale = match scale.as_str() {
                        "B" => 1,
                        "K" => 1024,
                        "M" => 1024*1024,
                        "G" => 1024*1024*1024,
                        _ => return Err(ConfigError::InvalidBlockMagnitude(scale)),
                    };

                    if let Ok(size) = size.parse::<usize>() {
                        cfg.blk_sz = size * scale;
                    }
                    else {
                        return Err(ConfigError::InvalidBlockSize(arg));
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
                        return Err(ConfigError::InvalidThreadCount(arg));
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
                        return Err(ConfigError::InvalidLvl(arg));
                    }
                }
                Parse::DirOut => {
                    cfg.user_out = arg;
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
                Parse::Cm => {
                    cfg.method = Method::Cm;
                }
                Parse::Lzwc => {
                    cfg.method = Method::Lzwc;
                }
                Parse::Store => {
                    cfg.method = Method::Store;
                }
                Parse::None => {},
            }
        } 
        Ok(cfg)
    }
    pub fn input_total(&self) -> u64 {
        self.inputs.iter().map(|f| f.len).sum()
    }
}
impl fmt::Display for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if !self.quiet {
            let version = Version::current();
            match self.mode {
                Mode::CreateArchive => {
                    write!(f, "
                        \rPrisirv {version}
                        \r=============================================================
                        \r Creating Archive of Inputs:"
                    )?;
                    for input in self.inputs.iter().take(5) {
                        write!(f, "
                            \r    {} ({})", 
                            input.path.display(),
                            input.len,
                        )?;
                    }
                    if self.inputs.len() > 5 {
                        write!(f, "
                            \r    ...")?;
                    }
                    
                    let (size, suffix) = format(self.blk_sz);
                    write!(f, "\n
                        \r Input Size:      {} Bytes
                        \r Output Path:     {}
                        \r Method:          {}
                        \r Sorting by:      {}
                        \r Memory Usage:    {} MiB
                        \r Block Size:      {} {}
                        \r Block Alignment: {}
                        \r Threads:         {}
                        \r=============================================================\n",
                        self.input_total(),
                        self.arch.path.display(),
                        match self.method {
                            Method::Cm    => "Context Mixing",
                            Method::Lzwc  => "LZWC",
                            Method::Lzws  => "LZWS",
                            Method::Store => "No Compression",
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
                        \rPrisirv {version}
                        \r=============================================================
                        \r Extracting Archive {}:",
                        self.arch.path.display()
                    )?;
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
                        \rPrisirv {version}
                        \r=============================================================
                        \r Adding to archive {}\n
                        \r Inputs:", 
                        self.arch.path.display()
                    )?;
                    for input in self.inputs.iter().take(5) {
                        write!(f, "
                            \r    {} ({})", 
                            input.path.display(),
                            input.len,
                        )?;
                    }
                    if self.inputs.len() > 5 {
                        write!(f, "
                            \r    ...")?;
                    }
                    let (size, suffix) = format(self.blk_sz);
                    write!(f, "\n
                        \r Input Size:      {} Bytes
                        \r Method:          {}
                        \r Sorting by:      {}
                        \r Memory Usage:    {} MiB
                        \r Block Size:      {} {}
                        \r Block Alignment: {}
                        \r Threads:         {}
                        \r=============================================================\n",
                        self.input_total(),
                        match self.method {
                            Method::Cm    => "Context Mixing",
                            Method::Lzwc  => "LZWC",
                            Method::Lzws  => "LZWS",
                            Method::Store => "No Compression",
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
                        \rPrisirv {version}
                        \r=============================================================
                        \r Merging into archive {}:",
                        self.arch.path.display()
                    )?;
                    for input in self.inputs.iter().take(5) {
                        write!(f, "
                            \r    {} ({})", 
                            input.path.display(),
                            input.len,
                        )?;
                    }
                    if self.inputs.len() > 5 {
                        write!(f, "
                            \r    ...")?;
                    }
                    write!(f, "\n
                        \r Input Size:      {} Bytes",
                        self.input_total(),
                    )?;
                    writeln!(f, "\r=============================================================")
                }
                Mode::ExtractFiles => {
                    write!(f, "
                        \rPrisirv {version}
                        \r=============================================================
                        \r Extracting files from archive {}:", 
                        self.arch.path.display()
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
            sort:      Sort::None,
            user_out:  String::new(),
            blk_sz:    10 << 20,
            mem:       1 << 22,
            mode:      Mode::None,
            quiet:     false,
            clobber:   false,
            threads:   4,
            inputs:    Vec::new(),
            out:       FileData::default(),
            align:     Align::Fixed,
            method:    Method::default(),
            arch:      FileData::default(),
            fv:        Fv::default(),
            verbose:   false,
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

