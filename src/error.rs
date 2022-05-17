use std::{
    path::PathBuf,
    fmt,
};

/// Possible errors encountered while parsing Config arguments.
pub enum ConfigError {
    InvalidSortCriteria(String),
    InvalidLvl(String),
    OutOfRangeMemory(u64),
    InvalidMemory(String),
    InvalidBlockSize(String),
    InvalidBlockMagnitude(String),
    OutOfRangeThreadCount(usize),
    InvalidThreadCount(String),
    InvalidInput(PathBuf),
    InvalidSortMethod(SortError),
    InvalidInsertId(String),
    InputsEmpty,
}

/// Possible errors encountered while sorting files.
#[derive(Debug)]
pub enum SortError {
    MetadataNotSupported,
    CreationTimeNotSupported,
    AccessTimeNotSupported,
    ModifiedTimeNotSupported,
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::InvalidSortCriteria(m) => {
                write!(f,  "
                    \r{m} is not a valid sort criteria.\n
                    \rSorting Methods:\n
                    \r    -sort ext      Sort by extension
                    \r    -sort name     Sort by name
                    \r    -sort len      Sort by length
                    \r    -sort prt n    Sort by nth parent directory
                    \r    -sort crtd     Sort by creation time
                    \r    -sort accd     Sort by last access time
                    \r    -sort mod      Sort by last modification time\n"
                )
            }
            ConfigError::InvalidLvl(lvl) => {
                write!(f,  "
                    \r{lvl} is not a valid directory level.\n
                    \rTo sort by nth parent directory, use option
                    \r'-sort prt n'.\n"
                )
            }
            ConfigError::OutOfRangeMemory(mem) => {
                write!(f, "
                    \r{mem} is outside the valid range of memory options (0..9).\n
                    \rMemory Options:\n
                    \r-mem 0  6 MB   -mem 5  99 MB
                    \r-mem 1  9 MB   -mem 6  195 MB
                    \r-mem 2  15 MB  -mem 7  387 MB
                    \r-mem 3  27 MB  -mem 8  771 MB
                    \r-mem 4  51 MB  -mem 9  1539 MB\n"
                )
            }
            ConfigError::InvalidMemory(mem) => {
                write!(f, "
                    \r{mem} is not a valid memory option.\n
                    \rMemory Options:\n
                    \r-mem 0  6 MB   -mem 5  99 MB
                    \r-mem 1  9 MB   -mem 6  195 MB
                    \r-mem 2  15 MB  -mem 7  387 MB
                    \r-mem 3  27 MB  -mem 8  771 MB
                    \r-mem 4  51 MB  -mem 9  1539 MB\n"
                )
            }
            ConfigError::InvalidBlockSize(size) => {
                write!(f, "
                    \r{size} is not a valid block size.\n"
                )
            }
            ConfigError::InvalidBlockMagnitude(mag) => {
                write!(f, "
                    \r{mag} is not a valid magnitude.\n
                    \rValid magnitudes are:
                    \r'B' (Bytes), 
                    \r'K' (Kibibytes), 
                    \r'M' (Mebibytes), 
                    \r'G' (Gibibytes)\n"
                )
            }
            ConfigError::OutOfRangeThreadCount(count) => {
                write!(f, "
                    \r{count} is outside the accepted thread count range (1..128).\n"
                )
            }
            ConfigError::InvalidThreadCount(count) => {
                write!(f, "
                    \r{count} is not a valid thread count.
                    \rThread count must be a number 1..128.\n"
                )
            }
            ConfigError::InvalidInput(path) => {
                write!(f, "
                    \r{} is not a valid path.\n", 
                    path.display()
                )
            }
            ConfigError::InputsEmpty => {
                write!(f, "
                    No inputs found.\n"
                )
            }
            ConfigError::InvalidSortMethod(method) => {
                match method {
                    SortError::MetadataNotSupported => {
                        write!(f, "
                            \rMetadata not supported.\n"
                        )
                    }
                    SortError::CreationTimeNotSupported => {
                        write!(f, "
                            \rCreation time metadata not supported.\n"
                        )
                    }
                    SortError::AccessTimeNotSupported => {
                        write!(f, "
                            \rAccess time metadata not supported.\n"
                        )
                    }
                    SortError::ModifiedTimeNotSupported => {
                        write!(f, "
                            \rModified time metadata not supported.\n"
                        )
                    }
                }
            }
            ConfigError::InvalidInsertId(id) => {
                write!(f, "
                    \r{id} is not a valid insert id.\n"
                )
            }
        }
    }
}

#[derive(Debug)]
pub enum ExtractError {
    MalformedBlockHeader(u32),
    FileNotFound(PathBuf),
}
impl fmt::Display for ExtractError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExtractError::MalformedBlockHeader(id) => {
                write!(f, "
                    \rDid not find valid magic number in block {id} header.\n"
                )
            }
            ExtractError::FileNotFound(file) => {
                write!(f, "
                    \r{} not found.\n", 
                    file.display()
                )
            }
        }
    }
}