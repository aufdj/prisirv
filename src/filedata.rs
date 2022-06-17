use std::{
    fs::DirEntry,
    path::PathBuf,
    ffi::OsStr,
    fmt,
    io,
};

/// Files in an archive are represented as file segments. If a file
/// doesn't cross a block boundary, then 'seg_beg' will be 0 and 'seg_end'
/// will be equal to len. If a file does cross a block boundary, the segment
/// boundaries descibe which segment of the file is in that block.
//
//  EXAMPLE:
//
//  File does not cross block boundary:
//
//  Block 0:
//      FileData {
//          len: 100,
//          seg_beg: 0,
//          seg_end: 100,
//      }
//  
//  File crosses block boundary at position 50:
//
//  Block 0:
//      FileData {
//          len: 100,
//          seg_beg: 0,
//          seg_end: 50,
//      }
//  
//  Block 1:
//      FileData {
//          len: 100,
//          seg_beg: 50,
//          seg_end: 100,
//      }
//  

/// File type. 
#[derive(Clone, PartialEq, Eq)]
pub enum Type {
    Unknown,
    Compressed,
    Text,
    Binary, 
    Executable,
}
impl From<&OsStr> for Type {
    fn from(s: &OsStr) -> Type {
        match s.to_str() {
            None => Type::Unknown,
            Some(s) => {
                match s {
                    "zip" => Type::Compressed,
                    "7z"  => Type::Compressed,
                    "xz"  => Type::Compressed,
                    "bz2" => Type::Compressed,
                    "txt" => Type::Text,
                    "exe" => Type::Executable,
                    _     => Type::Unknown,
                }
            }
        }
    }
}
impl Default for Type {
    fn default() -> Type {
        Type::Unknown
    }
}
#[derive(Clone, PartialEq, Eq, Default)]
pub struct FileData {
    pub path:     PathBuf, // File path
    pub len:      u64,     // File length
    pub seg_beg:  u64,     // Beginning segment position
    pub seg_end:  u64,     // End segment position
    pub blk_pos:  u64,     // Starting block position
    pub kind:     Type,    // File type
}
impl FileData {
    pub fn new(path: PathBuf) -> FileData {
        let len = match path.metadata() {
            Ok(file) => file.len(),
            Err(_)   => 0,
        }; 
        let kind = match path.extension() {
            Some(ext) => Type::from(ext),
            None      => Type::Unknown,
        };
        FileData { 
            path, 
            len,
            seg_beg: 0,
            seg_end: len,
            blk_pos: 0,
            kind,  
        }
    }
    // Total size of FileData
    pub fn size(&self) -> u64 {
        32 + self.path.as_os_str().len() as u64
    }
}
impl From<io::Result<DirEntry>> for FileData {
    fn from(entry: io::Result<DirEntry>) -> FileData {
        FileData::new(entry.unwrap().path())
    }
}
impl From<&String> for FileData {
    fn from(s: &String) -> FileData {
        FileData::new(PathBuf::from(s))
    }
}
impl From<&str> for FileData {
    fn from(s: &str) -> FileData {
        FileData::new(PathBuf::from(s))
    }
}
impl fmt::Display for FileData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.path.display())
    }
}
impl fmt::Debug for FileData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "
            Path:   {}\n
            Length: {}\n
            Segment Begin: {}\n
            Segment End:   {}\n", 
            self.path.display(),
            self.len,
            self.seg_beg,
            self.seg_end
        )
    }
}
