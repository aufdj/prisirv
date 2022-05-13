use std::{
    path::{Path, PathBuf},
    process::exit,
    fmt,
};

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
    //NotPrisirvArchive(),
    //MetadataNotSupported(),
    //CreationTimeNotSupported(),
    //AccessTimeNotSupported(),
    //ModifiedTimeNotSupported(),
    InputsEmpty,
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
{mem} is outside the valid range of memory options (0..9).\n
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
        }
    }
}

pub fn no_prisirv_archive() -> ! {
    println!("Not a prisirv archive.");
    exit(0);
}

pub fn metadata_not_supported() -> ! {
    println!("Couldn't get metadata.");
    exit(0);
}

pub fn creation_time_not_supported() -> ! {
    println!("Creation time metadata not supported on this platform.");
    exit(0);
}

pub fn access_time_not_supported() -> ! {
    println!("Last accessed time metadata not supported on this platform.");
    exit(0);
}

pub fn modified_time_not_supported() -> ! {
    println!("Last modified time metadata not supported on this platform.");
    exit(0);
}



pub fn file_general(path: &Path) -> ! {
    println!("Couldn't open file {}", path.display());
    exit(0);
}

pub fn dir_general(path: &Path) -> ! {
    println!("Couldn't create directory {}", path.display());
    exit(0);
}

pub fn file_already_exists(path: &Path) -> ! {
    println!("A file at location {} already exists.", path.display());
    println!("To overwrite existing files, enable file clobbering via '-clb' or '-clobber'.");
    exit(0);
}

pub fn file_not_found(path: & Path) -> ! {
    println!("Couldn't open file {}: Not Found", path.display());
    exit(0);
}

pub fn permission_denied(path: &Path) -> ! {
    println!("Couldn't open file {}: Permission Denied", path.display());
    exit(0);
}

pub fn dir_already_exists(path: &Path) -> ! {
    println!("A directory at location {} already exists.", path.display());
    println!("To overwrite existing directories, enable file clobbering via '-clb' or '-clobber'.");
    exit(0);
}