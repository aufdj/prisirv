use std::{
    io::{
        BufWriter, BufReader, 
        Seek, SeekFrom
    },
    fs::File,
    path::PathBuf,
};

use crate::{
    filedata::FileData,
    threads::ThreadPool,
    block::Block,
    formatting::fmt_file_out_extract,
    config::Config,
    buffered_io::{
        BufferedWrite, new_input_file, new_output_file, 
        new_dir,
    },
    error::ArchiveError,
};

/// Format and return new output file.
fn next_file(file_in: &FileData, dir_out: &FileData, clobber: bool) -> Result<BufWriter<File>, ArchiveError> {
    let file_out = fmt_file_out_extract(dir_out, file_in);
    new_output_file(&file_out, clobber)
}

/// An Extractor extracts archives.
pub struct Extractor {
    pub archive:  BufReader<File>,
    pub cfg:      Config,
    tp:           ThreadPool,
}
impl Extractor {
    /// Create a new Extractor.
    pub fn new(cfg: Config) -> Result<Extractor, ArchiveError> {
        let tp = ThreadPool::new(0, &cfg);
        let archive = new_input_file(&cfg.arch.path)?;
        
        Ok(
            Extractor { 
                archive, cfg, tp,
            }
        )
    }

    /// Decompress blocks and parse blocks into files. A block can span
    /// multiple files.
    pub fn extract_archive(&mut self) -> Result<(), ArchiveError> {
        new_dir(&self.cfg.out)?;
        
        let mut blk = Block::default();

        // Read and decompress blocks
        loop {
            blk.read_from(&mut self.archive)?;
            self.tp.decompress_block(blk.clone())?;

            if blk.data.is_empty() {
                break;
            }
            
            blk.next();
        }

        // Write blocks to output 
        loop {
            if let Some(blk) = self.tp.bq.lock().unwrap().try_get_block() {
                // Check for sentinel block
                if blk.data.is_empty() {
                    break; 
                }

                for file in blk.files.iter() {
                    let mut file_out = next_file(file, &self.cfg.out, self.cfg.clobber)?;
                    file_out.seek(SeekFrom::Start(file.seg_beg))?;

                    // Get segment of block containing target file's data.
                    let beg = file.blk_pos as usize;
                    let end = (file.blk_pos + (file.seg_end - file.seg_beg)) as usize;
                
                    for byte in blk.data[beg..end].iter() {
                        file_out.write_byte(*byte);
                    }
                    file_out.flush_buffer();
                }
            }
        }
        Ok(())
    } 

    pub fn extract_files(&mut self) -> Result<(), ArchiveError> {
        new_dir(&self.cfg.out)?;
        
        let mut blk = Block::default();
        let mut id = 0;

        let paths = self.cfg.inputs.iter()
            .map(|f| f.path.clone())
            .collect::<Vec<PathBuf>>();

        // Read and decompress blocks
        loop {
            blk.read_from(&mut self.archive)?;

            if blk.files.iter().any(|f| paths.contains(&f.path)) {
                blk.id = id;
                id += 1;
                self.tp.decompress_block(blk.clone())?;
            }

            if blk.data.is_empty() {
                blk.id = id;
                self.tp.decompress_block(blk.clone())?;
                break;
            }

            blk.next();
        }

        // Write blocks to output 
        loop {
            if let Some(mut blk) = self.tp.bq.lock().unwrap().try_get_block() {
                // Check for sentinel block
                if blk.data.is_empty() {
                    break; 
                }

                blk.files.retain(|file| paths.contains(&file.path));
                
                for file in blk.files.iter() {
                    let mut file_out = next_file(file, &self.cfg.out, self.cfg.clobber)?;
                    file_out.seek(SeekFrom::Start(file.seg_beg))?;

                    // Get segment of block containing target file's data.
                    let beg = file.blk_pos as usize;
                    let end = (file.blk_pos + (file.seg_end - file.seg_beg)) as usize;
                
                    for byte in blk.data[beg..end].iter() {
                        file_out.write_byte(*byte);
                    }
                    file_out.flush_buffer();
                }
            }
        }
        Ok(())
    }
}

