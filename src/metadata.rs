use std::{
    path::Path,
    cmp::min,
    ffi::OsStr,
};


/// Metadata Structure ==================================================================
///
/// A prisirv non-solid archive contains a 48 byte header followed by compressed data.
/// A prisirv solid archive contains a 32 byte header followed by compressed data,
/// followed by a footer containing information about each compressed file.
///
/// Non-solid: (nB = n byte value)
///
///      8B[Memory Option   ] 8B[Magic Number] 8B[File Extension]
///      8B[Final Block Size] 8B[Block Size  ] 8B[Block Count   ]
///      [Compressed Data --------------------------------------]
///      [------------------------------------------------------]
///      [------------------------------------------------------]
///
/// Solid: (nB = n byte value)
///
///      8B[Memory Option] 8B[Magic Number] 8B[Block Size] 8B[Files Pointer]
///      [Compressed Data -------------------------------------------------]
///      [-----------------------------------------------------------------]
///      [-----------------------------------------------------------------]
///      nB S[Files]*
///      
///      *Files Structure ---------------------------------------------
///          
///      8B[Number of files] 
///      nB[1B[File Path Length] nB[File Path       ]
///         8B[Block Count     ] 8B[Final Block Size]]
///       
///      Files consists of an 8 byte value denoting the number of 
///      compressed files, followed by a list of file paths, block 
///      counts, and final block sizes.
///      
///      The 8 byte 'number of files' value and the 1 byte 'file path
///      length' values are included for ease of parsing.
///
///
/// Memory Option = 1 << 20..1 << 29,
/// Magic Number  = 'prisirv ' for non-solid archives,
///                 'prisirvS' for solid archives,
/// Block Size    = 1 << 20,
///
/// =====================================================================================

#[derive(Debug)]
pub struct Metadata {
    pub mgc:      usize, // Magic Number
    pub mgcs:     usize, // Magic Number (Solid)
    pub ext:      usize, // Extension
    pub fblk_sz:  usize, // Final block size
    pub blk_sz:   usize, // Block size
    pub blk_c:    usize, // Block count
    pub enc_blk_szs: Vec<usize>, // Commpressed block sizes
    
    // Solid archives only ---------------
    pub f_ptr: usize, // Pointer to 'files'
    // Path, block_count, final_block_size
    pub files: Vec<(String, usize, usize)>,      
}
impl Metadata {
    pub fn new() -> Metadata {
        Metadata {
            mgc:      0x7673_7270,
            mgcs:     0x5653_5250,
            ext:      0,
            fblk_sz:  0,
            blk_sz:   1 << 20,
            blk_c:    0,
            f_ptr:    0,
            files:    Vec::new(),
            enc_blk_szs: Vec::new(),
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
        // Ignoring 00 bytes, convert ext to string
        String::from_utf8(
            self.ext.to_le_bytes().iter().cloned()
            .filter(|&i| i != 0).collect()
        ).unwrap()
    }
}