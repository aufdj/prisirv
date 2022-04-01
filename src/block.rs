use std::{
    fs::File,
    io::{BufWriter, BufReader, Write},
    path::PathBuf,
};
use crate::{
    metadata::{Metadata, FileData},
    buffered_io::{BufferedWrite, BufferedRead},
};

#[derive(Clone)]
pub struct Block {
    pub data:   Vec<u8>,       // Compressed data
    pub files:  Vec<FileData>, // Files in this block
    pub id:     u64,           // Block id
    pub chksum: u32,           // Uncompressed block checksum
    pub size:   u64,           // Compressed data size
    pub unsize: u64,           // Uncompressed data size
}
impl Block {
    pub fn new(blk_sz: usize) -> Block {
        Block {
            data:   Vec::with_capacity(blk_sz),
            files:  Vec::new(),
            id:     0,
            chksum: 0,
            size:   0,
            unsize: 0,
        }
    }
    pub fn next(&mut self) {
        self.data.clear();
        self.files.clear();
        self.id += 1;
    }
    fn get_size(&mut self) {
        self.size = self.data.len() as u64;
    }
    pub fn write(&mut self, archive: &mut BufWriter<File>) {
        self.get_size();

        archive.write_u64(self.size);
        archive.write_u64(self.unsize);
        archive.write_u32(self.chksum);
        archive.write_u64(self.files.len() as u64);

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
    pub fn read(&mut self, archive: &mut BufReader<File>) {
        let mut path: Vec<u8> = Vec::with_capacity(64);

        self.size = archive.read_u64();
        self.unsize = archive.read_u64();
        self.chksum = archive.read_u32();
        let num_files = archive.read_u64();

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
        for _ in 0..self.size {
            self.data.push(archive.read_byte());
        }
    }
    //pub fn print(&self) {
    //    println!();
    //    println!("Size: {}", self.size);
    //    println!("Files:");
    //    for file in self.files.iter() {
    //        println!("  {}", file.path.display());
    //        println!("  {}", file.len)
    //    }
    //    println!("Id: {}", self.id);
    //    println!("Checksum: {:x}", self.chksum);
    //}
}

/// Stores compressed or decompressed blocks. Blocks need to be written in
/// the same order that they were read, but no guarantee can be made about
/// which blocks will be compressed/decompressed first, so each block is 
/// added to a BlockQueue, which handles outputting in the correct order.
pub struct BlockQueue {
    pub blocks: Vec<Block>, // Blocks to be output
    next_out:   u64, // Next block to be output
}
impl BlockQueue {
    /// Create a new BlockQueue.
    pub fn new() -> BlockQueue {
        BlockQueue {
            blocks: Vec::new(),
            next_out: 0,
        }
    }

    /// Try writing the next compressed block to be output. If this block 
    /// hasn't been added to the queue yet, do nothing.
    pub fn try_write_block_enc(&mut self, mta: &mut Metadata, file_out: &mut BufWriter<File>) -> u64 {
        let len = self.blocks.len();
        let mut index = None;

        for (i, blk) in self.blocks.iter_mut().enumerate() {
            if blk.id == self.next_out {
                mta.enc_blk_szs.push(blk.data.len() as u64);
                blk.write(file_out);
                index = Some(i);
            }
        }

        // If next block was found, remove from list
        if let Some(i) = index {
            self.blocks.swap_remove(i as usize);
            self.next_out += 1;
        }

        (len - self.blocks.len()) as u64
    }

    /// Try writing the next decompressed block to be output. If this block 
    /// hasn't been added to the queue yet, do nothing.
    pub fn try_write_block_dec(&mut self, file_out: &mut BufWriter<File>) -> u64 {
        let len = self.blocks.len();
        let mut next_out = self.next_out;

        self.blocks.retain(|block|
            if block.id == next_out {
                for byte in block.data.iter() {
                    file_out.write_byte(*byte);
                }
                next_out += 1;
                false
            }
            else { 
                true 
            } 
        );
        let blks_wrtn = (len - self.blocks.len()) as u64;
        self.next_out += blks_wrtn;
        blks_wrtn
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
