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
const PLACEHOLDER: [u8; 28] = [0; 28];

/// A solid archiver creates solid archives, or an archive containing files 
/// compressed as one stream. Solid archives take advantage of redundancy 
/// across files and therefore achieve better compression ratios than non-
/// solid archives, but don't allow for extracting individual files.
pub struct SolidArchiver {
    pub archive:  BufWriter<File>,
    cfg:          Config,
    mta:          Metadata,
    prg:          Progress,
}
impl SolidArchiver {
    /// Create a new SolidArchiver.
    pub fn new(cfg: Config) -> SolidArchiver {
        let mut mta = Metadata::new_with_cfg(&cfg);
        
        // Collect and sort files.
        collect_files(&cfg.inputs, &mut mta);
        mta.files.sort_by(|f1, f2| 
            sort_files(&f1.path, &f2.path, cfg.sort)
        );

        let mut prg = Progress::new(&cfg);
        prg.get_archive_size(&mta.files);

        let mut archive = new_output_file_checked(&cfg.dir_out, cfg.clbr);
        archive.write_all(&PLACEHOLDER).unwrap();

        SolidArchiver { 
            archive, cfg, prg, mta
        }
    }

    /// Parse files into blocks and compress blocks.
    pub fn create_archive(&mut self) {
        let mut tp = ThreadPool::new(self.cfg.threads, self.cfg.mem, self.prg);
        let mut blk = Block::new(self.cfg.blk_sz);

        // Read files into blocks and compress
        for file in self.mta.files.iter() {
            blk.files.push(file.clone());
            let mut file_in = new_input_file(blk.data.capacity(), &file.path);

            if self.cfg.align == Align::File {
                for _ in 0..file.len {
                    blk.data.push(file_in.read_byte());
                }
                if blk.data.len() >= blk.data.capacity() {
                    tp.compress_block(blk.clone());
                    self.mta.blk_c += 1;
                    blk.next();
                }
            }
            else {
                for _ in 0..file.len {
                    blk.data.push(file_in.read_byte());
                    if blk.data.len() >= blk.data.capacity() {
                        tp.compress_block(blk.clone());
                        self.mta.blk_c += 1;
                        blk.next();
                    }
                }
            }
        }

        // Compress final block
        if !blk.data.is_empty() {
            tp.compress_block(blk.clone());
            self.mta.blk_c += 1;
        }
        
        // Output blocks
        let mut blks_wrtn: u64 = 0;
        while blks_wrtn != self.mta.blk_c {
            if let Some(mut blk) = tp.bq.lock().unwrap().try_get_block() {
                blk.write_to(&mut self.archive);
                blks_wrtn += 1;
            }
        }
        self.archive.flush_buffer();

        self.write_metadata();
    }

    /// Write footer containing file paths and lengths.
    fn write_metadata(&mut self) {
        // Return final archive size including footer
        self.prg.print_archive_stats(self.archive.stream_position().unwrap());

        self.archive.rewind().unwrap();
        self.archive.write_u64(self.mta.mem);     
        self.archive.write_u32(self.mta.mgcs);
        self.archive.write_u64(self.mta.blk_sz as u64);
        self.archive.write_u64(self.mta.blk_c);
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
