use std::{
    fs::File,
    io::{BufWriter, BufReader, Write},
    path::PathBuf,
};
use crate::{
    metadata::FileData,
    buffered_io::{BufferedWrite, BufferedRead},
};


#[derive(Clone)]
pub struct Block {
    pub id:     u32,           // Block id
    pub chksum: u32,           // Input block checksum
    pub sizeo:  u64,           // Output data size
    pub sizei:  u64,           // Input data size
    pub crtd:   u64,           // Creation time
    pub files:  Vec<FileData>, // Files in this block
    pub data:   Vec<u8>,       // Block data 
}
impl Block {
    pub fn new(blk_sz: usize) -> Block {
        Block {
            id:     0,
            chksum: 0,
            sizeo:  0,
            sizei:  0,
            crtd:   0,  
            files:  Vec::new(),
            data:   Vec::with_capacity(blk_sz),
        }
    }
    pub fn next(&mut self) {
        self.data.clear();
        self.files.clear();
        self.id += 1;
    }
    pub fn write_to(&mut self, archive: &mut BufWriter<File>) {
        archive.write_u32(self.id);
        archive.write_u32(self.chksum);
        archive.write_u64(self.sizeo);
        archive.write_u64(self.sizei);
        archive.write_u32(self.files.len() as u32);

        for file in self.files.iter() {
            let path = file.path.to_str().unwrap().as_bytes();
            archive.write_all(path).unwrap();
            archive.write_byte(0);
            archive.write_u64(file.len);
        }

        for byte in self.data.iter() {
            archive.write_byte(*byte);
        }
    }
    pub fn read_from(&mut self, archive: &mut BufReader<File>) {
        self.id       = archive.read_u32();
        self.chksum   = archive.read_u32();
        self.sizeo    = archive.read_u64();
        self.sizei    = archive.read_u64();
        let num_files = archive.read_u32();

        let mut path: Vec<u8> = Vec::with_capacity(64);

        // Read null terminated path strings and lengths
        for _ in 0..num_files {
            loop {
                match archive.read_byte() {
                    0 => {
                        let path_string = path.iter()
                            .map(|b| *b as char)
                            .collect::<String>();
                        let file_len = archive.read_u64();
    
                        self.files.push(
                            FileData {
                                path: PathBuf::from(&path_string),
                                len:  file_len,
                            }
                        );
                        path.clear();
                        break;
                    }
                    byte => path.push(byte),
                }
            }
        }

        // Read compressed data
        for _ in 0..self.sizeo {
            self.data.push(archive.read_byte());
        }
    }
    pub fn size(&self) -> u64 {
        let mut total: u64 = 0;
        total += self.data.len() as u64;
        for file in self.files.iter() {
            total += file.size() + 1; // Add 1 for null byte
        }
        total + 28
    }
    pub fn print(&self) {
        println!();
        println!("Block {}:", self.id);
        println!("==========================================");
        println!("Compressed Size:   {}", self.sizeo);
        println!("Uncompressed Size: {}", self.sizei);
        println!("Checksum:          {:x}", self.chksum);
        println!("Creation time:     {}", self.crtd);
        println!();
        println!("Files:");
        for file in self.files.iter() {
            println!("  {}", file.path.display());
            println!("  {}", file.len)
        }
        println!("==========================================");
    }
}

/// Stores compressed or decompressed blocks. Blocks need to be written in
/// the same order that they were read, but no guarantee can be made about
/// which blocks will be compressed/decompressed first, so each block is 
/// added to a BlockQueue, which handles outputting in the correct order.
pub struct BlockQueue {
    pub blocks:  Vec<Block>, // Blocks to be output
    next_out:    u32,        // Next block to be output
}
impl BlockQueue {
    /// Create a new BlockQueue.
    pub fn new() -> BlockQueue {
        BlockQueue {
            blocks:    Vec::new(),
            next_out:  0,
        }
    }

    /// Try getting the next block to be output. If this block hasn't been 
    /// added to the queue yet, do nothing.
    pub fn try_get_block(&mut self) -> Option<Block> {
        let mut i: usize = 0;
        while i < self.blocks.len() {
            if self.blocks[i].id == self.next_out {
                let blk = self.blocks[i].clone();
                self.blocks.swap_remove(i);
                self.next_out += 1;
                return Some(blk);
            } 
            i += 1;
        }
        None
    }
}
