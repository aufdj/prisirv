use std::{
    path::Path,
    io::{Seek, BufWriter},
    fs::File,
};

use crate::{
    sort::sort_files,
    filedata::FileData,
    threads::ThreadPool,
    progress::Progress,
    config::{Config, Align},
    buffered_io::{
        BufferedRead, BufferedWrite,
        new_input_file, new_output_file_checked,
    },
    block::Block,
};

/// An archiver by default creates solid archives, or an archive containing
/// files compressed as one stream. Solid archives take advantage of redundancy 
/// across files and therefore achieve better compression ratios than non-
/// solid archives, but don't allow for extracting individual files like
/// non-solid archives.
pub struct Archiver {
    pub archive:  BufWriter<File>,
    cfg:          Config,
    files:        Vec<FileData>,
    tp:           ThreadPool,
}
impl Archiver {
    /// Create a new Archiver.
    pub fn new(cfg: Config) -> Archiver {
        let mut files = Vec::new();
        
        // Collect and sort files.
        collect_files(&cfg.inputs, &mut files);
        files.sort_by(|f1, f2| 
            sort_files(&f1.path, &f2.path, cfg.sort)
        );

        let prg = Progress::new(&cfg, &files);
        let tp = ThreadPool::new(cfg.threads, prg);
        let archive = new_output_file_checked(&cfg.out, cfg.clbr);

        Archiver { 
            archive, cfg, files, tp, 
        }
    }

    /// Parse files into blocks and compress blocks.
    pub fn create_archive(&mut self) {
        let mut blk = Block::new(&self.cfg);

        // Read files into blocks and compress
        for file in self.files.iter_mut() {
            let mut file_in = new_input_file(self.cfg.blk_sz, &file.path);
            file.blk_pos = blk.data.len() as u64;

            for _ in 0..file.len {
                blk.data.push(file_in.read_byte());
                if blk.data.len() >= self.cfg.blk_sz {
                    file.seg_end = file_in.stream_position().unwrap();
                    blk.files.push(file.clone());
                    self.tp.compress_block(blk.clone());
                    blk.next();
                    file.blk_pos = 0;
                    file.seg_beg = file_in.stream_position().unwrap();
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

/// Recursively collect all files into a vector for sorting before compression.
pub fn collect_files(inputs: &[FileData], files: &mut Vec<FileData>) {
    // Group files and directories 
    let (fi, dirs): (Vec<FileData>, Vec<FileData>) =
        inputs.iter().cloned()
        .partition(|f| f.path.is_file());

    // Walk through directories and collect all files
    for file in fi.into_iter() {
        files.push(file);
    }
    for dir in dirs.iter() {
        collect(&dir.path, files);
    }
}
fn collect(dir_in: &Path, files: &mut Vec<FileData>) {
    let (fi, dirs): (Vec<FileData>, Vec<FileData>) =
        dir_in.read_dir().unwrap()
        .map(|d| FileData::new(d.unwrap().path()))
        .partition(|f| f.path.is_file());

    for file in fi.into_iter() {
        files.push(file);
    }
    for dir in dirs.iter() {
        collect(&dir.path, files);
    }
}
