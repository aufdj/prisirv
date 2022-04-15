use std::{
    path::PathBuf,
    fmt,
};

/// Input file data
#[derive(Debug, Clone)]
pub struct FileData {
    pub path:  PathBuf,
    pub len:   u64,
}
impl FileData {
    pub fn new(path: PathBuf) -> FileData {
        let len = match path.metadata() {
            Ok(file) => file.len(),
            Err(_)   => 0,
        };
        FileData { path, len }
    }
    pub fn path_str(&self) -> &str {
        self.path.to_str().unwrap()
    }
    // Total size of FileData (length of path + 8 byte file length)
    pub fn size(&self) -> u64 {
        (self.path_str().as_bytes().len() + 8) as u64
    }
}
impl Default for FileData {
    fn default() -> FileData {
        FileData { 
            path: PathBuf::from(""), 
            len: 0 
        }
    }
}
impl fmt::Display for FileData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.path.display())
    }
}