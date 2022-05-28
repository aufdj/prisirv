use std::{
    path::PathBuf,
    fmt,
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
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FileData {
    pub path:     PathBuf, // File path
    pub len:      u64,     // File length
    pub seg_beg:  u64,     // Beginning segment position
    pub seg_end:  u64,     // End segment position
    pub blk_pos:  u64,     // Starting block position
}
impl FileData {
    pub fn new(path: PathBuf) -> FileData {
        let len = match path.metadata() {
            Ok(file) => file.len(),
            Err(_)   => 0,
        };
        FileData { 
            path, 
            len,
            seg_beg: 0,
            seg_end: len,
            blk_pos: 0,
        }
    }
    pub fn path_str(&self) -> &str {
        self.path.to_str().unwrap()
    }
    // Total size of FileData
    pub fn size(&self) -> u64 {
        (self.path_str().as_bytes().len() + 32) as u64
    }
}
impl fmt::Display for FileData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, 
            "
            Path:   {}\n
            Length: {}\n
            Segment Begin: {}\n
            Segment End:   {}\n
            ", 
            self.path.display(),
            self.len,
            self.seg_beg,
            self.seg_end)
    }
}