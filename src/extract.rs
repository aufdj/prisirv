use std::{
    io::{BufWriter, BufReader, Seek, SeekFrom},
    fs::File,
};

use crate::{
    filedata::FileData,
    threads::ThreadPool,
    progress::Progress,
    block::Block,
    formatting::fmt_file_out_extract,
    config::Config,
    buffered_io::{
        BufferedWrite, new_input_file, new_output_file, 
        new_dir, new_output_file_no_trunc,
    },
};



/// A decompressed block can span multiple files, so a FileWriter is used 
/// to handle swapping files when needed while writing a block.
struct FileWriter {
    files:         Box<dyn Iterator<Item = FileData>>, // Input file data
    file_out:      BufWriter<File>,                    // Current output file
    file_in:       FileData,                           // Current input file data
    file_out_pos:  u64,                                // Current output file position
    dir_out:       String,                             // Output directory
}
impl FileWriter {
    fn new(files: Vec<FileData>, dir_out: &str, pos: u64) -> FileWriter {
        let mut files = files.into_iter();

        let file_in = files.next().unwrap();                  
        let mut file_out = next_file(&file_in, dir_out);  
        file_out.seek(SeekFrom::Start(pos)).unwrap();         

        FileWriter {
            dir_out:      dir_out.to_string(),
            files:        Box::new(files),
            file_out_pos: pos,
            file_out,
            file_in,
        }
    }
    /// Switch to new file when current is correct size.
    fn update(&mut self) {
        let file_in = self.files.next().unwrap();
        self.file_out = next_file(&file_in, &self.dir_out);
        self.file_in = file_in;
        self.file_out_pos = 0;
    }
    fn write_byte(&mut self, byte: u8) {
        if self.file_out_pos == self.file_in.len {
            self.file_out.flush_buffer();
            self.update();
        }
        self.file_out.write_byte(byte);
        self.file_out_pos += 1;
    }
    fn current(&self) -> (u64, FileData) {
        (self.file_out_pos, self.file_in.clone())
    }
}

/// Get the next output file and the input file length, the input file being
/// the original file that was compressed.
///
/// The input file length is needed to know when the output file is the 
/// correct size.
fn next_file(file_in: &FileData, dir_out: &str) -> BufWriter<File> {
    let file_out_path = fmt_file_out_extract(dir_out, &file_in.path);
    if file_out_path.exists() {
        return new_output_file_no_trunc(4096, &file_out_path)
    }
    new_output_file(4096, &file_out_path)
}

/// An Extractor extracts archives.
pub struct Extractor {
    pub archive:  BufReader<File>,
    pub cfg:      Config,
    tp:           ThreadPool,
}
impl Extractor {
    /// Create a new Extractor.
    pub fn new(cfg: Config) -> Extractor {
        let archive = new_input_file(4096, &cfg.inputs[0].path);
        let prg = Progress::new(&cfg, &cfg.inputs);
        let tp = ThreadPool::new(cfg.threads, prg);
        
        Extractor { 
            archive, cfg, tp, 
        }
    }

    /// Decompress blocks and parse blocks into files. A block can span 
    /// multiple files.
    pub fn extract_archive(&mut self) {
        new_dir(&self.cfg.out, self.cfg.clbr);
        
        let mut blk = Block::new(0, 0);

        // Read and decompress blocks
        loop {
            blk.read_from(&mut self.archive);
            self.tp.decompress_block(blk.clone());
            if blk.data.is_empty() { break; }
            blk.next();
        }

        let mut pos = 0;
        let mut file_data = FileData::default();

        // Write blocks to output 
        loop {
            if let Some(mut blk) = self.tp.bq.lock().unwrap().try_get_block() {
                // Check for sentinel block
                if blk.data.is_empty() { break; }

                // Add last file of previous block to new block, assuming that
                // the file crossed a block boundary. If the file did end on a
                // block boundary, it will be immediately skipped.
                if file_data.path.exists() { 
                    blk.files.insert(0, file_data);
                }
                let mut fw = FileWriter::new(blk.files.clone(), self.cfg.out.path_str(), pos);

                for byte in blk.data.iter() {
                    fw.write_byte(*byte);
                }
                fw.file_out.flush_buffer();

                // To handle files that cross block boundaries, 
                // save the current file data and position.
                (pos, file_data) = fw.current();
            } 
        }
    } 
}

