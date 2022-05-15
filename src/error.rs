use std::{
    path::PathBuf,
    process::exit,
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
{m} is not a valid sort criteria.\n
Sorting Methods:\n
    -sort ext      Sort by extension
    -sort name     Sort by name
    -sort len      Sort by length
    -sort prt n    Sort by nth parent directory
    -sort crtd     Sort by creation time
    -sort accd     Sort by last access time
    -sort mod      Sort by last modification time")
            }

            ConfigError::InvalidLvl(lvl) => {
                write!(f,  "
{lvl} is not a valid directory level.\n
To sort by nth parent directory, use option
'-sort prt n'.")
            }

            ConfigError::OutOfRangeMemory(mem) => {
                write!(f, "
{mem} is outside the valid range of memory options (0..9).\n
Memory Options:\n
-mem 0  6 MB   -mem 5  99 MB
-mem 1  9 MB   -mem 6  195 MB
-mem 2  15 MB  -mem 7  387 MB
-mem 3  27 MB  -mem 8  771 MB
-mem 4  51 MB  -mem 9  1539 MB")
            }

            ConfigError::InvalidMemory(mem) => {
                write!(f, "
{mem} is not a valid memory option.\n
Memory Options:\n
-mem 0  6 MB   -mem 5  99 MB
-mem 1  9 MB   -mem 6  195 MB
-mem 2  15 MB  -mem 7  387 MB
-mem 3  27 MB  -mem 8  771 MB
-mem 4  51 MB  -mem 9  1539 MB")
            }

            ConfigError::InvalidBlockSize(size) => {
                write!(f, "{size} is not a valid block size.")
            }

            ConfigError::InvalidBlockMagnitude(mag) => {
                write!(f, "
{mag} is not a valid magnitude.\n
Valid magnitudes are 'B' (Bytes), 'K' (Kibibytes), 'M' (Mebibytes), or 'G' (Gibibytes)")
            }

            ConfigError::OutOfRangeThreadCount(count) => {
                write!(f, "{count} is outside the accepted thread count range (1..128).")
            }

            ConfigError::InvalidThreadCount(count) => {
                write!(f, "
{count} is not a valid thread count.
Thread count must be a number 1..128.")
            }

            ConfigError::InvalidInput(path) => {
                write!(f, "{} is not a valid path.", path.display())
            }

            ConfigError::InputsEmpty => {
                write!(f, "No inputs found.")
            }

            ConfigError::InvalidSortMethod(method) => {
                match method {
                    SortError::MetadataNotSupported => {
                        write!(f, "Metadata not supported.")
                    }
                    SortError::CreationTimeNotSupported => {
                        write!(f, "Creation time metadata not supported.")
                    }
                    SortError::AccessTimeNotSupported => {
                        write!(f, "Access time metadata not supported.")
                    }
                    SortError::ModifiedTimeNotSupported => {
                        write!(f, "Modified time metadata not supported.")
                    }
                }
            }
        }
    }
}

pub fn no_prisirv_archive() -> ! {
    println!("Not a prisirv archive.");
    exit(0);
}