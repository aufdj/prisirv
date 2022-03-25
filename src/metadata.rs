use std::{
    path::{Path, PathBuf},
    cmp::min,
    ffi::OsStr,
};

use crate::config::Config;

/// Input file data
#[derive(Debug, Clone)]
pub struct FileData {
    pub path:  PathBuf,
    pub len:   u64,
}


/// # Metadata Structure 
///
/// * A prisirv non-solid archive contains a 56 byte header followed by compressed 
/// data, followed by a footer containing the size of each compressed block.
/// * A prisirv solid archive contains a 48 byte header followed by compressed data,
/// followed by a footer containing information about each compressed file.
///
/// ## Non-Solid Archive
///
/// * Memory Option     
/// * Magic Number
/// * File Extension
/// * Final Block Size
/// * Block Size
/// * Block Count
/// * Footer Pointer
/// * Compressed Data
/// * Footer
///    
/// The footer for a non-solid archive contains a list of 
/// compressed sizes for each block, each value being 8 bytes.
///
/// ## Solid Archive
///
/// * Memory Option
/// * Magic Number
/// * Block Size
/// * Footer Pointer
/// * Compressed Data
/// * Footer
///     
/// A solid archive footer consists of an 8 byte value denoting  
/// the number of compressed files, followed by a list of file 
/// paths and lengths.
/// 
///
/// Memory Option: 1<<20..1<<29,
/// Magic Number:  'prsv' for non-solid archives, 'PRSV' for solid archives,
/// Block Size:    10<<20,

#[derive(Debug)]
pub struct Metadata {
    pub mem:      u64,   // Memory Usage
    pub mgc:      u64,   // Magic Number
    pub mgcs:     u64,   // Magic Number (Solid)
    pub ext:      u64,   // Extension
    pub fblk_sz:  usize, // Final block size
    pub blk_sz:   usize, // Block size
    pub blk_c:    u64,   // Block count
    pub f_ptr:    u64,   // Pointer to footer
    pub enc_blk_szs: Vec<u64>, // Compressed block sizes
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
            fblk_sz:  0,
            blk_sz:   10 << 20,
            blk_c:    0,
            f_ptr:    0,
            files:    Vec::new(),
            enc_blk_szs: Vec::new(),
        }
    }
    pub fn new_with_cfg(cfg: &Config) -> Metadata {
        Metadata {
            mem:      cfg.mem,
            mgc:      0x7673_7270,
            mgcs:     0x5653_5250,
            ext:      0,
            fblk_sz:  0,
            blk_sz:   cfg.blk_sz,
            blk_c:    0,
            f_ptr:    0,
            files:    Vec::new(),
            enc_blk_szs: Vec::new(),
        }
    }

    /// Set metadata extension field to the input file's extension.
    pub fn set_ext(&mut self, path: &Path) {
        // Handle no extension
        let ext = match path.extension() {
            Some(ext) => { ext },
            None => OsStr::new(""),
        };
        // Get extension as byte slice, truncated to 8 bytes
        let mut ext = ext.to_str().unwrap().as_bytes();
        ext = &ext[..min(ext.len(), 8)];

        for byte in ext.iter().rev() {
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