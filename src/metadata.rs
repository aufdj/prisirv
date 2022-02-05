use std::{
    path::Path,
    cmp::min,
    ffi::OsStr,
};


// Metadata Structure 
//
// A prisirv non-solid archive contains a 56 byte header followed by compressed data.
// A prisirv solid archive contains a 56 byte header followed by compressed data,
// followed by a footer containing information about each compressed file.
//
// Key -------------------------------- 
//     nB - n byte value,      
//     N  - Used in non-solid archives,
//     S  - Used in solid archives,
// ------------------------------------
//
// 
// 8BNS[Memory Option   ] 8BNS[Magic Number] 8BN [File Extension]
// 8BN [Final Block Size] 8BNS[Block Size  ] 8BN [Block Count   ]
// 8B S[Files Pointer   ]
// [Compressed Data --------------------------------------------]
// [------------------------------------------------------------]
// [------------------------------------------------------------]
// nB S[Files]*
//
// Memory Option = 1 << 20..1 << 29,
// Magic Number  = 'prisirv ' for non-solid archives,
//                 'prisirvS' for solid archives,
// Block Size    = 1 << 20,
//
//
// *Files is a list of every compressed file's full pathway, 
// block count, and final block size.
//
// Files Structure ---------------------------------------------
//     
// 8B [Number of files] 
// nB*[1B[File Path Length] nB[File Path       ]
//     8B[Block Count     ] 8B[Final Block Size]]
//  
// *Repeat n times, where n is the number of files compressed.
// 
// The 8 byte 'number of files' value and the 1 byte 'file path
// length' values are included for ease of parsing.


#[derive(Debug)]
pub struct Metadata {
    pub mgc:      usize, // Magic Number
    pub ext:      usize, // Extension
    pub f_bl_sz:  usize, // Final block size
    pub bl_sz:    usize, // Block size
    pub bl_c:     usize, // Block count
    pub f_ptr:    usize, // Pointer to 'files'
    
    // Solid archives only ---------------
    // Path, block_count, final_block_size
    pub files: Vec<(String, usize, usize)>,      
}
impl Metadata {
    pub fn new() -> Metadata {
        Metadata {
            mgc:      0x76_7269_7369_7270,
            ext:      0,
            f_bl_sz:  0,
            bl_sz:    1 << 20,
            bl_c:     0,
            f_ptr:    0,
            files:    Vec::new(),  
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