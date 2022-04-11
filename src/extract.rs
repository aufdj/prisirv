use std::{
    io::{BufWriter, BufReader, Seek, SeekFrom},
    fs::File,
};

use crate::{
    metadata::{Metadata, FileData},
    threads::ThreadPool,
    progress::Progress,
    block::Block,
    formatting::fmt_file_out_extract,
    config::Config,
    buffered_io::{
        BufferedRead, BufferedWrite,
        new_input_file, new_output_file, 
        new_dir, new_output_file_no_trunc,
    },
    error,
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
    fn current(&self) -> FileData {
        self.file_in.clone()
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
    pub mta:      Metadata,
    prg:          Progress,
}
impl Extractor {
    /// Create a new Extractor.
    pub fn new(cfg: Config) -> Extractor {
        let mut archive = new_input_file(4096, &cfg.inputs[0].path);
        let mta = read_metadata(&mut archive);
        let prg = Progress::new(&cfg);
        
        let mut extr = Extractor { 
            archive, mta, cfg, prg, 
        };
        extr.prg.get_archive_size(&extr.cfg.inputs);
        extr
    }

    /// Decompress blocks and parse blocks into files. A block can span 
    /// multiple files.
    pub fn extract_archive(&mut self) {
        new_dir(&self.cfg.out, self.cfg.clbr);
        let mut tp = ThreadPool::new(self.cfg.threads, self.mta.mem, self.prg);
        let mut blk = Block::new(self.mta.blk_sz);

        for _ in 0..self.mta.blk_c {
            blk.read_from(&mut self.archive);
            tp.decompress_block(blk.clone());
            blk.next();
        }

        let mut blks_wrtn: u64 = 0;
        let mut pos = 0;
        let mut file_data = FileData::default(); 

        // Write blocks to output 
        while blks_wrtn != self.mta.blk_c {
            if let Some(mut blk) = tp.bq.lock().unwrap().try_get_block() {

                // Add last file of previous block to new block, assuming that
                // the file crossed a block boundary. If the file did end on a
                // block boundary, it will be immediately skipped.
                if file_data.path.exists() { 
                    blk.files.insert(0, file_data); 
                }
                let mut fw = FileWriter::new(blk.files.clone(), &self.cfg.out.path_str(), pos);

                for byte in blk.data.iter() {
                    fw.write_byte(*byte);
                }

                fw.file_out.flush_buffer();
                file_data = fw.current();
                blks_wrtn += 1;
                
                // To handle files that cross block boundaries, 
                // save the current file position.
                pos = fw.file_out_pos; 
            }  
        }
        let mut lens: Vec<u64> = Vec::new();
        get_file_out_lens(&self.cfg.out, &mut lens);
        self.prg.print_archive_stats(lens.iter().sum());
    } 
}

pub fn read_metadata(archive: &mut BufReader<File>) -> Metadata {
    let mut mta: Metadata = Metadata::new();
    mta.mem     = archive.read_u64();
    mta.mgc     = archive.read_u32();
    mta.blk_sz  = archive.read_u64() as usize;
    mta.blk_c   = archive.read_u64();
    verify_magic_number(mta.mgc); 
    mta
}

/// Check for a valid magic number.
fn verify_magic_number(mgc: u32) {
    if mgc != 0x5653_5250 {
        error::no_prisirv_archive();
    }
}

// Get total size of decompressed archive.
fn get_file_out_lens(dir_in: &FileData, lens: &mut Vec<u64>) {
    let (files, dirs): (Vec<FileData>, Vec<FileData>) =
        dir_in.path.read_dir().unwrap()
        .map(|d| FileData::new(d.unwrap().path()))
        .partition(|f| f.path.is_file());

    for file in files.iter() {
        lens.push(file.len);
    }
    for dir in dirs.iter() {
        get_file_out_lens(&dir, lens);
    }
}

