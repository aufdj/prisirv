use std::{
    io::{Seek, BufWriter, BufReader},
    fs::File,
};

use crate::{
    sort::sort_files,
    archive::collect_files,
    filedata::FileData,
    threads::ThreadPool,
    progress::Progress,
    config::{Config, Align},
    buffered_io::{
        BufferedRead, BufferedWrite,
        new_input_file, new_output_file,
    },
    block::Block,
};


pub struct ArchiveModifier {
    old:    BufReader<File>,
    new:    BufWriter<File>,
    files:  Vec<FileData>,
    cfg:    Config,
    tp:     ThreadPool,
}
impl ArchiveModifier {
    pub fn new(cfg: Config) -> ArchiveModifier {
        let mut files = Vec::new();
        // Collect and sort files.
        collect_files(&cfg.inputs, &mut files);
        files.sort_by(|f1, f2| 
            sort_files(&f1.path, &f2.path, cfg.sort)
        );
        let old = new_input_file(4096, &cfg.ex_arch.path);
        let new = new_output_file(4096, &cfg.out.path);
        let prg = Progress::new(&cfg, &files);
        let tp  = ThreadPool::new(cfg.threads, prg);

        ArchiveModifier {
            old, new, files, cfg, tp,
        }
    }
    pub fn add(&mut self) {
        let mut blks_added = 0;
        let mut blk = Block::new(&self.cfg);
        for _ in 0..self.cfg.insert_id {
            blk.read_from(&mut self.old);
            self.tp.store_block(blk.clone());
            blk.next();
        }
        // Read files into blocks and compress
        for file in self.files.iter_mut() {
            let mut file_in = new_input_file(self.cfg.blk_sz, &file.path);

            for _ in 0..file.len {
                blk.data.push(file_in.read_byte());
                if blk.data.len() >= self.cfg.blk_sz {
                    file.seg_end = file_in.stream_position().unwrap();
                    blk.files.push(file.clone());
                    self.tp.compress_block(blk.clone());
                    blks_added += 1;
                    blk.next();
                    file.seg_beg = file_in.stream_position().unwrap();
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
            if blk.data.is_empty() { break; }
            blk.next();
        }

        // Output blocks
        loop {
            if let Some(mut blk) = self.tp.bq.lock().unwrap().try_get_block() {
                blk.write_to(&mut self.new);
                if blk.data.is_empty() { break; }
            }
        }
        self.new.flush_buffer();
    }
}