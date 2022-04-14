use std::{
    path::Path,
    io::{Write, Seek, BufWriter},
    fs::File,
};

use crate::{
    sort::sort_files,
    metadata::{Metadata, FileData},
    threads::ThreadPool,
    progress::Progress,
    config::{Config, Align},
    buffered_io::{
        BufferedRead, BufferedWrite,
        new_input_file, new_output_file_checked,
    },
    block::Block,
};

/// Size of header in bytes
const PLACEHOLDER: [u8; 20] = [0; 20];

/// An archiver by default creates solid archives, or an archive containing 
/// files compressed as one stream. Solid archives take advantage of redundancy 
/// across files and therefore achieve better compression ratios than non-
/// solid archives, but don't allow for extracting individual files like
/// non-solid archives.
pub struct Archiver {
    pub archive:  BufWriter<File>,
    cfg:          Config,
    mta:          Metadata,
    tp:           ThreadPool,
}
impl Archiver {
    /// Create a new Archiver.
    pub fn new(cfg: Config) -> Archiver {
        let mut mta = Metadata::new_with_cfg(&cfg);
        
        // Collect and sort files.
        collect_files(&cfg.inputs, &mut mta);
        mta.files.sort_by(|f1, f2| 
            sort_files(&f1.path, &f2.path, cfg.sort)
        );

        let prg = Progress::new(&cfg, &mta.files);
        let tp = ThreadPool::new(cfg.threads, cfg.mem, prg);

        let mut archive = new_output_file_checked(&cfg.out, cfg.clbr);
        archive.write_all(&PLACEHOLDER).unwrap();

        Archiver { 
            archive, cfg, mta, tp, 
        }
    }

    /// Parse files into blocks and compress blocks.
    pub fn create_archive(&mut self) {
        let mut blk = Block::new(self.cfg.blk_sz);

        // Read files into blocks and compress
        for file in self.mta.files.iter() {
            blk.files.push(file.clone());
            let mut file_in = new_input_file(self.cfg.blk_sz, &file.path);

            for _ in 0..file.len {
                blk.data.push(file_in.read_byte());
                if blk.data.len() >= self.cfg.blk_sz {
                    self.tp.compress_block(blk.clone());
                    blk.next();
                }
            }
            // Truncate final block to align with end of file
            if self.cfg.align == Align::File && !blk.data.is_empty() {
                self.tp.compress_block(blk.clone());
                blk.next();
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
                if blk.data.is_empty() { break; }
            }
        }
        self.archive.flush_buffer();

        self.write_metadata();
    }

    /// Write header
    fn write_metadata(&mut self) {
        self.archive.rewind().unwrap();
        self.archive.write_u64(self.mta.mem);     
        self.archive.write_u32(self.mta.mgc);
        self.archive.write_u64(self.mta.blk_sz as u64);
    }
}

/// Recursively collect all files into a vector for sorting before compression.
fn collect_files(inputs: &[FileData], mta: &mut Metadata) {
    // Group files and directories 
    let (files, dirs): (Vec<FileData>, Vec<FileData>) =
        inputs.iter().cloned()
        .partition(|f| f.path.is_file());

    // Walk through directories and collect all files
    for file in files.into_iter() {
        mta.files.push(file);
    }
    for dir in dirs.iter() {
        collect(&dir.path, mta);
    }
}
fn collect(dir_in: &Path, mta: &mut Metadata) {
    let (files, dirs): (Vec<FileData>, Vec<FileData>) =
        dir_in.read_dir().unwrap()
        .map(|d| FileData::new(d.unwrap().path()))
        .partition(|f| f.path.is_file());

    for file in files.into_iter() {
        mta.files.push(file);
    }
    for dir in dirs.iter() {
        collect(&dir.path, mta);
    }
}
