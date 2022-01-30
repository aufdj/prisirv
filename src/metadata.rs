use std::{
    path::Path,
    cmp::min,
    ffi::OsStr,
};

#[derive(Debug)]
pub struct Metadata {
    pub ext:      usize, // Extension
    pub f_bl_sz:  usize, // Final block size
    pub bl_sz:    usize, // Block size
    pub bl_c:     usize, // Block count

    // Solid archives only ---------------
    // Path, block_count, final_block_size
    pub files:  Vec<(String, usize, usize)>,     
    pub f_ptr:  usize, // Pointer to 'files'
}
impl Metadata {
    pub fn new() -> Metadata {
        Metadata {
            ext:      0,
            f_bl_sz:  0,
            bl_sz:    1 << 20,
            bl_c:     0,
            files:    Vec::new(),
            f_ptr:    0,
        }
    }
    // Set metadata extension field
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
            self.ext = (self.ext << 8) | *byte as usize;
        }
    }
    // Get metadata extension field
    pub fn get_ext(&self) -> String {
        String::from_utf8(
            self.ext.to_le_bytes().iter().cloned()
            .filter(|&i| i != 0).collect()
        ).unwrap()
    }
}