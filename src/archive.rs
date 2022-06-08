use std::{
    io::{Seek, SeekFrom, BufWriter},
    fs::File,
};

use crate::{
    threads::ThreadPool,
    progress::Progress,
    config::{Config, Align, Mode},
    buffered_io::{
        BufferedRead, BufferedWrite,
        new_input_file, new_output_file,
    },
    error::ArchiveError,
    block::Block,
    archiveinfo::ArchiveInfo,
};

/// An archive consists of blocks, with each block containing a
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
    pub fn new(cfg: Config) -> Result<Archiver, ArchiveError> {
        let prg = Progress::new(&cfg);
        let tp = ThreadPool::new(&cfg, prg);

        let path = 
        if cfg.mode == Mode::CreateArchive {
            &cfg.out
        }
        else {
            &cfg.ex_arch
        };

        let archive = new_output_file(path, cfg.clobber)?;
        
        Ok(
            Archiver {
                archive, cfg, tp
            }
        )
    }

    /// Parse files into blocks and compress blocks.
    pub fn create_archive(&mut self) -> Result<(), ArchiveError> {
        let mut blk = Block::new(&self.cfg);

        // Read files into blocks and compress
        for file in self.cfg.inputs.iter_mut() {
            let mut file_in = new_input_file(&file.path)?;
            file.blk_pos = blk.data.len() as u64;

            for _ in 0..file.len {
                blk.data.push(file_in.read_byte());
                if blk.data.len() >= self.cfg.blk_sz {
                    let pos = file_in.stream_position()?;
                    file.seg_end = pos;
                    blk.files.push(file.clone());
                    self.tp.compress_block(blk.clone());
                    blk.next();
                    file.blk_pos = 0;
                    file.seg_beg = pos;
                }
            }
            file.seg_end = file_in.stream_position()?;
            
            // Truncate final block to align with end of file
            if self.cfg.align == Align::File && !blk.data.is_empty() {
                blk.files.push(file.clone());
                self.tp.compress_block(blk.clone());
                blk.next();
                file.seg_beg = file_in.stream_position()?;
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
        Ok(())
    }
    /// Add files to existing archive.
    pub fn append_files(&mut self) -> Result<(), ArchiveError> {
        self.archive.seek(SeekFrom::Start(self.cfg.ex_info.end_of_data()))?;
        let mut blk = Block::new(&self.cfg);
        blk.id = self.cfg.ex_info.block_count();
    
        // Read files into blocks and compress
        for file in self.cfg.inputs.iter_mut() {
            let mut file_in = new_input_file(&file.path)?;
            file.blk_pos = blk.data.len() as u64;

            for _ in 0..file.len {
                blk.data.push(file_in.read_byte());
                if blk.data.len() >= self.cfg.blk_sz {
                    let pos = file_in.stream_position()?;
                    file.seg_end = pos;
                    blk.files.push(file.clone());
                    self.tp.compress_block(blk.clone());
                    blk.next();
                    file.blk_pos = 0;
                    file.seg_beg = pos;
                }
            }
            file.seg_end = file_in.stream_position()?;

            // Truncate block to align with end of file
            if self.cfg.align == Align::File && !blk.data.is_empty() {
                blk.files.push(file.clone());
                self.tp.compress_block(blk.clone());
                blk.next();
                file.seg_beg = file_in.stream_position()?;
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
        Ok(())
    }

    pub fn merge_archives(&mut self) -> Result<(), ArchiveError> {
        let mut id = self.cfg.ex_info.block_count();

        self.archive.seek(SeekFrom::Start(self.cfg.ex_info.end_of_data()))?;
        let mut blk = Block::default();

        let mut info = Vec::new();
        for input in self.cfg.inputs.iter() {
            info.push(ArchiveInfo::new(input)?);
        }

        if !info.iter().all(|i| i.version == self.cfg.ex_info.version) {
            return Err(ArchiveError::IncompatibleVersions);
        }

        for file in self.cfg.inputs.iter() {
            let mut file_in = new_input_file(&file.path)?;
            loop {
                blk.read_from(&mut file_in)?;
                if blk.data.is_empty() {
                    break;
                }
                blk.id = id;
                id += 1;
                blk.write_to(&mut self.archive);
                blk.next();
            }
        }
        blk.id = id;
        blk.write_to(&mut self.archive);
        Ok(())
    }
}
