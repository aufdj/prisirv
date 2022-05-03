use std::{
    path::PathBuf,
    fmt,
};


#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FileData {
    pub path:     PathBuf,
    pub len:      u64,
    pub seg_beg:  u64,
    pub seg_end:  u64,
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
        }
    }
    pub fn path_str(&self) -> &str {
        self.path.to_str().unwrap()
    }
    // Total size of FileData
    pub fn size(&self) -> u64 {
        (self.path_str().as_bytes().len() + 24) as u64
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