use std::{
    path::{Path, PathBuf},
    ffi::OsStr,
};

use crate::config::Config;

/// Return the length of a file.
pub fn file_len(path: &Path) -> u64 {
    path.metadata().unwrap().len()
}

/// Input file data
#[derive(Debug, Clone)]
pub struct FileData {
    pub path:  PathBuf,
    pub len:   u64,
}
impl FileData {
    pub fn new(path: PathBuf) -> FileData {
        FileData {
            len: file_len(&path),
            path,
        }
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


/// # Metadata Structure 
///
/// * A prisirv non-solid archive contains a 36 byte header followed by compressed data.
/// * A prisirv solid archive contains a 28 byte header followed by compressed data.
///
/// ## Non-Solid Archive
///
/// * Memory Option     
/// * Magic Number
/// * File Extension
/// * Block Size
/// * Block Count
/// * Compressed Data
///    
///
/// ## Solid Archive
///
/// * Memory Option
/// * Magic Number
/// * Block Size
/// * Block Count
/// * Compressed Data
///     
/// 
/// Memory Option: 1<<20..1<<29,
/// Magic Number:  'prsv' for non-solid archives, 'PRSV' for solid archives,
/// Block Size:    10<<20,

#[derive(Debug)]
pub struct Metadata {
    pub mem:      u64,   // Memory Usage
    pub mgc:      u32,   // Magic Number
    pub mgcs:     u32,   // Magic Number (Solid)
    pub ext:      u64,   // Extension
    pub blk_sz:   usize, // Block size
    pub blk_c:    u64,   // Block count
    pub files: Vec<FileData>,
}
impl Metadata {
    /// Initialize new metadata.
    pub fn new() -> Metadata {
        Metadata {
            mem:      0,
            mgc:      0x7673_7270,
            mgcs:     0x5653_5250,
            ext:      0,
            blk_sz:   10 << 20,
            blk_c:    0,
            files:    Vec::new(),
        }
    }
    pub fn new_with_cfg(cfg: &Config) -> Metadata {
        Metadata {
            mem:      cfg.mem,
            mgc:      0x7673_7270,
            mgcs:     0x5653_5250,
            ext:      0,
            blk_sz:   cfg.blk_sz,
            blk_c:    0,
            files:    Vec::new(),
        }
    }

    /// Set metadata extension field to the input file's extension.
    pub fn set_ext(&mut self, path: &Path) {
        // Handle no extension
        let ext = match path.extension() {
            Some(ext) => { ext },
            None => OsStr::new(""),
        };
        let bytes = ext.to_str().unwrap().as_bytes();

        for byte in bytes.iter().take(8).rev() {
            self.ext = (self.ext << 8) | *byte as u64;
        }
    }

    /// Get metadata extension field.
    pub fn get_ext(&self) -> String {
        // Ignoring 00 bytes, convert ext to string
        String::from_utf8(
            self.ext.to_le_bytes().iter().cloned()
            .filter(|&i| i != 0).collect()
        ).unwrap()
    }
}