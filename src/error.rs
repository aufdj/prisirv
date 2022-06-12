use std::{
    time::SystemTimeError,
    path::PathBuf,
    fmt,
    io,
};

use crate::constant::Version;


/// Possible errors encountered while sorting files.
#[derive(Debug)]
pub enum SortError {
    MetadataNotSupported,
    CreationTimeNotSupported,
    AccessTimeNotSupported,
    ModifiedTimeNotSupported,
}
impl fmt::Display for SortError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
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
}

/// Possible errors encountered while parsing Config arguments.
#[derive(Debug)]
pub enum ConfigError {
    InvalidSortCriteria(String),
    InvalidLvl(String),
    OutOfRangeMemory(u64),
    InvalidMemory(String),
    InvalidBlockSize(String),
    InvalidBlockMagnitude(String),
    OutOfRangeThreadCount(usize),
    InvalidThreadCount(String),
    InvalidInput(String),
    InvalidSortMethod(SortError),
    InvalidInsertId(String),
    IoError(io::Error),
    ArchiveError(ArchiveError),
    InvalidColorScale(String),
    InvalidImageWidth(String),
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
                    \r{path} is not a valid path.\n"
                )
            }
            ConfigError::InvalidSortMethod(err) => {
                write!(f, "
                    \r{err}\n"
                )
            }
            ConfigError::InvalidInsertId(id) => {
                write!(f, "
                    \r{id} is not a valid insert id.\n"
                )
            }
            ConfigError::IoError(err) => {
                write!(f, "
                    \r{err}\n"
                )
            }
            ConfigError::ArchiveError(err) => {
                write!(f, "
                    \r{err}\n"
                )
            }
            ConfigError::InvalidColorScale(n) => {
                write!(f, "
                    \r{n} is not a valid color scale.\n"
                )
            }
            ConfigError::InvalidImageWidth(w) => {
                write!(f, "
                    \r{w} is not a valid image width.\n"
                )
            }
        }
    }
}
impl From<io::Error> for ConfigError {
    fn from(err: io::Error) -> ConfigError {
        ConfigError::IoError(err)
    }
}
impl From<ArchiveError> for ConfigError {
    fn from(err: ArchiveError) -> ConfigError {
        ConfigError::ArchiveError(err)
    }
}
impl From<SortError> for ConfigError {
    fn from(err: SortError) -> ConfigError {
        ConfigError::InvalidSortMethod(err)
    }
}


/// Possible errors encountered during extraction, either reading or 
/// decompressing.
#[derive(Debug)]
pub enum ArchiveError {
    InvalidMagicNumber(u32),
    InvalidVersion(Version),
    IncompatibleVersions,
    FileNotFound(PathBuf),
    IncorrectChecksum(u32),
    IoError(io::Error),
    CreationTimeError(SystemTimeError),
}
impl From<io::Error> for ArchiveError {
    fn from(err: io::Error) -> ArchiveError {
        ArchiveError::IoError(err)
    }
}
impl From<SystemTimeError> for ArchiveError {
    fn from(err: SystemTimeError) -> ArchiveError {
        ArchiveError::CreationTimeError(err)
    }
}
impl fmt::Display for ArchiveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ArchiveError::InvalidMagicNumber(id) => {
                write!(f, "
                    \rDid not find valid magic number in block {id} header.\n"
                )
            }
            ArchiveError::InvalidVersion(version) => {
                let current = Version::current();
                write!(f, "
                    \rThis archive was created with Prisirv version {version},
                    \rand cannot be extracted with version {current}.\n",
                )
            }
            ArchiveError::IncompatibleVersions => {
                write!(f, "
                    \rCannot merge archives with incompatible versions.\n",
                )
            }
            ArchiveError::FileNotFound(file) => {
                write!(f, "
                    \r{} not found.\n", 
                    file.display()
                )
            }
            ArchiveError::IncorrectChecksum(id) => {
                write!(f, "
                    \rBlock {id} checksum is invalid.\n"
                )
            }
            ArchiveError::IoError(err) => {
                write!(f, "
                    \r{err}.\n"
                )
            }
            ArchiveError::CreationTimeError(err) => {
                write!(f, "
                    \r{err}.\n"
                )
            }
        }
    }
}

#[derive(Debug)]
pub enum PrisirvError {
    ConfigError(ConfigError),
    ArchiveError(ArchiveError),
}
impl fmt::Display for PrisirvError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PrisirvError::ConfigError(err) => {
                write!(f, "{err}")
            }
            PrisirvError::ArchiveError(err) => {
                write!(f, "{err}")
            }
        }
    }
}
impl From<ConfigError> for PrisirvError {
    fn from(err: ConfigError) -> PrisirvError {
        PrisirvError::ConfigError(err)
    }
}
impl From<ArchiveError> for PrisirvError {
    fn from(err: ArchiveError) -> PrisirvError {
        PrisirvError::ArchiveError(err)
    }
}