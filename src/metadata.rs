use std::{
    path::PathBuf,
    fmt,
};

use crate::config::Config;

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

        FileData { len, path }
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


/// # Metadata Structure 
///
/// * A prisirv archive contains a 28 byte 
///   header followed by compressed data.
///
/// * Memory Option
/// * Magic Number
/// * Block Size
/// * Block Count
/// * Compressed Data

#[derive(Debug)]
pub struct Metadata {
    pub mem:      u64,   // Memory Usage
    pub mgc:      u32,   // Magic Number
    pub ext:      u64,   // Extension
    pub blk_sz:   usize, // Block size
    pub blk_c:    u64,   // Block count
    pub files:    Vec<FileData>,
}
impl Metadata {
    /// Initialize new metadata.
    pub fn new() -> Metadata {
        Metadata {
            mem:      0,
            mgc:      0x5653_5250,
            ext:      0,
            blk_sz:   10 << 20,
            blk_c:    0,
            files:    Vec::new(),
        }
    }
    pub fn new_with_cfg(cfg: &Config) -> Metadata {
        Metadata {
            mem:      cfg.mem,
            mgc:      0x5653_5250,
            ext:      0,
            blk_sz:   cfg.blk_sz,
            blk_c:    0,
            files:    Vec::new(),
        }
    }
}