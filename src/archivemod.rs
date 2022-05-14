use std::{
    io::{Seek, BufWriter, BufReader},
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

/// An archive modifier is used for modifying archives. Currently, the only
/// modification supported is adding files to an existing archive, but in 
/// the future removing files or changing metadata may be supported.
pub struct ArchiveModifier {
    old:    BufReader<File>,
    new:    BufWriter<File>,
    cfg:    Config,
    tp:     ThreadPool,
}
impl ArchiveModifier {
    pub fn new(cfg: Config) -> ArchiveModifier {
        let old = new_input_file(&cfg.ex_arch.path);
        let new = new_output_file(&cfg.out, cfg.clobber);
        let prg = Progress::new(&cfg);
        let tp  = ThreadPool::new(cfg.threads, prg);

        ArchiveModifier {
            old, new, cfg, tp,
        }
    }
    /// Add files to existing archive.
    pub fn add_files(&mut self) {
        let mut blks_added = 0;
        let mut blk = Block::new(&self.cfg);
        for _ in 0..self.cfg.insert_id {
            blk.read_from(&mut self.old);
            self.tp.store_block(blk.clone());
            blk.next();
        }
        // Read files into blocks and compress
        for file in self.cfg.inputs.iter_mut() {
            let mut file_in = new_input_file(&file.path);

            for _ in 0..file.len {
                blk.data.push(file_in.read_byte());
                if blk.data.len() >= self.cfg.blk_sz {
                    let pos = file_in.stream_position().unwrap();
                    file.seg_end = pos;
                    blk.files.push(file.clone());
                    self.tp.compress_block(blk.clone());
                    blks_added += 1;
                    blk.next();
                    file.seg_beg = pos;
                }    
            }
            file.seg_end = file_in.stream_position().unwrap();

            // Truncate final block to align with end of file
            if self.cfg.align == Align::File && !blk.data.is_empty() {
                blk.files.push(file.clone());
                self.tp.compress_block(blk.clone());
                blks_added += 1;
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
            blks_added += 1;
            blk.next();
        }

        loop {
            blk.read_from(&mut self.old);
            blk.id += blks_added;
            self.tp.store_block(blk.clone());
            if blk.data.is_empty() { 
                break; 
            }
            blk.next();
        }

        // Output blocks
        loop {
            if let Some(mut blk) = self.tp.bq.lock().unwrap().try_get_block() {
                blk.write_to(&mut self.new);
                if blk.data.is_empty() { 
                    break; 
                }
            }
        }
        self.new.flush_buffer();
    }
}