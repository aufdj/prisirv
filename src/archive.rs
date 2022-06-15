use std::{
    io::{Seek, SeekFrom, BufWriter},
    fs::File,
};

use crate::{
    threads::ThreadPool,
    progress::Progress,
    config::{Config, Align},
    buffered_io::{
        BufferedRead, 
        new_input_file, new_output_file,
    },
    error::ArchiveError,
    block::Block,
    archiveinfo::ArchiveInfo,
};


/// An existing archive and associated information.
struct ExArchive {
    file: BufWriter<File>,
    info: ArchiveInfo,
}
impl ExArchive {
    fn new(cfg: &Config) -> Result<ExArchive, ArchiveError> {
        let info = ArchiveInfo::new(&cfg.ex_arch)?;
        let mut file = new_output_file(&cfg.ex_arch, cfg.clobber)?;
        file.seek(SeekFrom::Start(info.end_of_data()))?;

        Ok(
            ExArchive {
                info, file
            }
        )
    }
}

/// An archive consists of blocks, with each block containing a
/// header followed by compressed data. Blocks can either be fixed size,
/// or truncated to align with the end of the current file. The end of an
/// archive is marked by an empty block.
pub struct Archiver {
    cfg:      Config,
}
impl Archiver {
    /// Create a new Archiver.
    pub fn new(cfg: Config) -> Archiver {
        Archiver {
            cfg
        }
    }

    /// Parse files into blocks and compress blocks.
    pub fn create_archive(&mut self) -> Result<(), ArchiveError> {
        let mut archive = new_output_file(&self.cfg.out, self.cfg.clobber)?;
        let mut tp = ThreadPool::new(0, &self.cfg);

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
                    tp.compress_block(blk.clone());
                    blk.next();
                    file.blk_pos = 0;
                    file.seg_beg = pos;
                }
            }
            file.seg_end = file_in.stream_position()?;
            
            // Truncate final block to align with end of file
            if self.cfg.align == Align::File && !blk.data.is_empty() {
                blk.files.push(file.clone());
                tp.compress_block(blk.clone());
                blk.next();
                file.seg_beg = file_in.stream_position()?;
            }
            if !blk.files.contains(file) {
                blk.files.push(file.clone());
            }
        }

        // Compress final block
        if !blk.data.is_empty() {
            tp.compress_block(blk.clone());
            blk.next();
        }

        // Empty sentinel block
        tp.compress_block(blk.clone());
        
        // Output blocks
        loop {
            if let Some(blk) = tp.bq.lock().unwrap().try_get_block() {
                blk.write_to(&mut archive);
                if blk.data.is_empty() {
                    break;
                }
            }
        }
        Ok(())
    }
    /// Add files to existing archive.
    pub fn append_files(&mut self) -> Result<(), ArchiveError> {
        let mut archive = ExArchive::new(&self.cfg)?;
        let mut tp = ThreadPool::new(archive.info.next_id(), &self.cfg);

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
                    tp.compress_block(blk.clone());
                    blk.next();
                    file.blk_pos = 0;
                    file.seg_beg = pos;
                }
            }
            file.seg_end = file_in.stream_position()?;

            // Truncate block to align with end of file
            if self.cfg.align == Align::File && !blk.data.is_empty() {
                blk.files.push(file.clone());
                tp.compress_block(blk.clone());
                blk.next();
                file.seg_beg = file_in.stream_position()?;
            }
            if !blk.files.contains(file) {
                blk.files.push(file.clone());
            }
        }

        // Compress final block
        if !blk.data.is_empty() {
            tp.compress_block(blk.clone());
            blk.next();
        }

        // Empty sentinel block
        tp.compress_block(blk.clone());

        // Output blocks
        loop {
            if let Some(blk) = tp.bq.lock().unwrap().try_get_block() {
                blk.write_to(&mut archive.file);
                if blk.data.is_empty() { 
                    break; 
                }
            }
        }
        Ok(())
    }

    pub fn merge_archives(&mut self) -> Result<(), ArchiveError> {
        let mut archive = ExArchive::new(&self.cfg)?;
        let mut prg = Progress::new(&self.cfg);

        let mut blk = Block::default();

        for input in self.cfg.inputs.iter() {
            if ArchiveInfo::new(input)?.ver != archive.info.ver {
                return Err(ArchiveError::IncompatibleVersions);
            }
        }

        for file in self.cfg.inputs.iter() {
            let mut file_in = new_input_file(&file.path)?;
            loop {
                blk.read_from(&mut file_in)?;
                if blk.data.is_empty() {
                    break;
                }
                blk.id = archive.info.next_id();
                blk.write_to(&mut archive.file);
                prg.update(&blk);
                blk.next();
            }
        }
        blk.id = archive.info.next_id();
        blk.write_to(&mut archive.file);
        prg.update(&blk);
        Ok(())
    }
}
