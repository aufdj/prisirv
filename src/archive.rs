use std::{
    io::{Seek, BufWriter},
    fs::File,
};

use crate::{
    threads::ThreadPool,
    progress::Progress,
    config::{Config, Align},
    buffered_io::{
        BufferedRead, BufferedWrite,
        new_input_file, new_output_file,
    },
    block::Block,
};

/// A Prisirv archive consists of blocks, with each block containing a 
/// header followed by compressed data. Blocks can either be fixed size,
/// or truncated to align with the end of the current file. The end of an 
/// archive is marked by an empty block.
pub struct Archiver {
    archive:  BufWriter<File>,
    cfg:      Config,
    tp:       ThreadPool,
}
impl Archiver {
    /// Create a new Archiver.
    pub fn new(cfg: Config) -> Archiver {
        let prg = Progress::new(&cfg);
        let tp = ThreadPool::new(cfg.threads, prg);
        let archive = new_output_file(&cfg.out, cfg.clobber).unwrap();

        Archiver { 
            archive, cfg, tp, 
        }
    }

    /// Parse files into blocks and compress blocks.
    pub fn create_archive(&mut self) {
        let mut blk = Block::new(&self.cfg);

        // Read files into blocks and compress
        for file in self.cfg.inputs.iter_mut() {
            let mut file_in = new_input_file(&file.path).unwrap();
            file.blk_pos = blk.data.len() as u64;

            for _ in 0..file.len {
                blk.data.push(file_in.read_byte());
                if blk.data.len() >= self.cfg.blk_sz {
                    let pos = file_in.stream_position().unwrap();
                    file.seg_end = pos;
                    blk.files.push(file.clone());
                    self.tp.compress_block(blk.clone());
                    blk.next();
                    file.blk_pos = 0;
                    file.seg_beg = pos;
                }    
            }
            file.seg_end = file_in.stream_position().unwrap();
            
            // Truncate final block to align with end of file
            if self.cfg.align == Align::File && !blk.data.is_empty() {
                blk.files.push(file.clone());
                self.tp.compress_block(blk.clone());
                blk.next();
                file.seg_beg = file_in.stream_position().unwrap();
            }
            if !blk.files.contains(file) {
                blk.files.push(file.clone());
            }
        }

        // Compress final block
        if !blk.data.is_empty() {
            self.tp.compress_block(blk.clone());
            blk.next();
        }

        // Empty sentinel block
        self.tp.compress_block(blk.clone());
        
        // Output blocks
        loop {
            if let Some(mut blk) = self.tp.bq.lock().unwrap().try_get_block() {
                blk.write_to(&mut self.archive);
                if blk.data.is_empty() { 
                    break; 
                }
            }
        }
        self.archive.flush_buffer();
    }
}
