use std::{
    path::{Path, PathBuf},
    io::{Seek, SeekFrom, BufWriter, BufReader},
    fs::File,
};

use crate::{
    metadata::{Metadata, FileData},
    threads::ThreadPool,
    progress::Progress,
    block::Block,
    formatting::fmt_file_out_s_extract,
    config::Config,
    buffered_io::{
        BufferedRead, BufferedWrite, file_len,
        new_input_file, new_output_file, 
        new_dir_checked, 
    },
    error,
};

/// A decompressed block can span multiple files, so a FileWriter is used 
/// to handle swapping files when needed while writing a block.
struct FileWriter {
    files:         Box<dyn Iterator<Item = FileData>>, // Input file data
    file_out:      BufWriter<File>,                    // Current output file
    file_out_pos:  u64,                                // Current output file position
    file_in_len:   u64,                                // Uncompressed file length
    dir_out:       String,                             // Output directory
}
impl FileWriter {
    fn new(files: Vec<FileData>, dir_out: &str) -> FileWriter {
        let mut files = files.into_iter();

        let file = files.next().unwrap();

        FileWriter {
            dir_out:      dir_out.to_string(),
            files:        Box::new(files),
            file_out_pos: 0,
            file_out:     next_file(&file.path, dir_out),
            file_in_len:  file.len,
        }
    }
    fn update(&mut self) {
        let file = self.files.next().unwrap();
        self.file_out = next_file(&file.path, &self.dir_out);
        self.file_in_len = file.len;
        self.file_out_pos = 0;
    }
    fn write_byte(&mut self, byte: u8) {
        if self.file_out_pos == self.file_in_len {
            self.update();
        }
        self.file_out.write_byte(byte);
        self.file_out_pos += 1;
    }
}

/// Get the next output file and the input file length, the input file being
/// the original file that was compressed.
///
/// The input file length is needed to know when the output file is the 
/// correct size.
fn next_file(file_in_path: &Path, dir_out: &str) -> BufWriter<File> {
    let file_out_path = fmt_file_out_s_extract(dir_out, file_in_path);
    new_output_file(4096, &file_out_path)
}

/// A SolidExtractor extracts solid archives.
pub struct SolidExtractor {
    archive:  BufReader<File>,
    pub cfg:  Config,
    prg:      Progress,
    pub mta:  Metadata,
}
impl SolidExtractor {
    /// Create a new SolidExtractor.
    pub fn new(cfg: Config) -> SolidExtractor {
        new_dir_checked(&cfg.dir_out, cfg.clbr);
        let mut archive = new_input_file(4096, &cfg.inputs[0]);
        let mta = read_metadata(&mut archive);
        let prg = Progress::new(&cfg);
        
        let mut extr = SolidExtractor { 
            archive, mta, cfg, prg, 
        };
        extr.prg.get_archive_size_dec(&extr.cfg.inputs, extr.mta.blk_c);
        extr
    }

    /// Decompress blocks and parse blocks into files. A block can span 
    /// multiple files.
    pub fn extract_archive(&mut self) {
        let mut tp = ThreadPool::new(self.cfg.threads, self.mta.mem, self.prg);
        let mut blk = Block::new(self.mta.blk_sz);

        for _ in 0..self.mta.blk_c {
            blk.read(&mut self.archive);
            tp.decompress_block(blk.clone());
            blk.next();
        }

        let mut blks_wrtn: u64 = 0;
    
        // Write blocks to output 
        while blks_wrtn != self.mta.blk_c {
            if let Some(blk) = tp.bq.lock().unwrap().try_get_block() {
                let mut fw = FileWriter::new(blk.files, &self.cfg.dir_out);
                for byte in blk.data.iter() {
                    fw.write_byte(*byte);
                }
                blks_wrtn += 1;
                //fw.file_out.flush_buffer();
            }  
        }


        /*
        let mut tp = ThreadPool::new(self.cfg.threads, self.mta.mem, self.prg);
        let mut blk = Block::new(self.mta.blk_sz);

        // Full blocks
        for _ in 0..self.mta.blk_c-1 {
            for _ in 0..self.mta.enc_blk_szs[index as usize] {
                blk.data.push(self.archive.read_byte());
            }
            tp.decompress_block(blk.clone(), self.mta.blk_sz);
            blk.next();
        }

        // Final block
        for _ in 0..self.mta.enc_blk_szs[index as usize] {
            blk.push(self.archive.read_byte());
        }
        tp.decompress_block(blk.clone(), self.mta.fblk_sz);

        let mut fw = FileWriter::new(self.mta.files.clone(), &self.cfg.dir_out);
        let mut blks_wrtn: u64 = 0;
        
        // Write blocks to output 
        let mut blk = Vec::with_capacity(self.mta.blk_sz);
        while blks_wrtn != self.mta.blk_c {
            tp.bq.lock().unwrap().try_get_block(&mut blk);
            if !blk.is_empty() {
                for byte in blk.iter() {
                    fw.write_byte(*byte);
                }
                blks_wrtn += 1;
                blk.clear();
            }  
        }

        fw.file_out.flush_buffer();

        let mut lens: Vec<u64> = Vec::new();
        get_file_out_lens(Path::new(&self.cfg.dir_out), &mut lens);
        self.prg.print_archive_stats(lens.iter().sum());
        */
    } 
}

pub fn read_metadata(archive: &mut BufReader<File>) -> Metadata {
    let mut mta: Metadata = Metadata::new();
    mta.mem     = archive.read_u64();
    mta.mgcs    = archive.read_u64();
    mta.blk_sz  = archive.read_u64() as usize;
    mta.fblk_sz = archive.read_u64() as usize;
    mta.blk_c   = archive.read_u64();
    mta.f_ptr   = archive.read_u64();
    verify_magic_number(mta.mgcs);

    /*
    // Seek to end of file metadata
    archive.seek(SeekFrom::Start(mta.f_ptr)).unwrap();

    let mut path: Vec<u8> = Vec::with_capacity(64);

    let num_files = archive.read_u64();

    // Read null terminated file path strings and lengths.
    for _ in 0..num_files {
        loop {
            match archive.read_byte() {
                0 => {
                    let path_string = path.iter()
                        .map(|b| *b as char)
                        .collect::<String>();
                    let file_len = archive.read_u64();

                    mta.files.push(
                        FileData {
                            path: PathBuf::from(&path_string),
                            len:  file_len,
                        }
                    );
                    path.clear();
                    break;
                }
                byte => path.push(byte),
            }
        }
    }

    // Get compressed block sizes
    for _ in 0..mta.blk_c {
        mta.enc_blk_szs.push(archive.read_u64());
    }
    */
    // Seek back to beginning of compressed data
    archive.seek(SeekFrom::Start(48)).unwrap();
    
    mta
}

/// Check for a valid magic number.
/// * Non-solid archives - 'prsv'
/// * Solid archives     - 'PRSV'
fn verify_magic_number(mgc: u64) {
    match mgc {
        0x5653_5250 => {},
        0x7673_7270 => error::found_non_solid_archive(),
        _ => error::no_prisirv_archive(),
    }
}

/// Get total size of decompressed archive.
fn get_file_out_lens(dir_in: &Path, lens: &mut Vec<u64>) {
    let (files, dirs): (Vec<PathBuf>, Vec<PathBuf>) =
        dir_in.read_dir().unwrap()
        .map(|d| d.unwrap().path())
        .partition(|f| f.is_file());

    for file in files.iter() {
        lens.push(file_len(file));
    }
    for dir in dirs.iter() {
        get_file_out_lens(dir, lens);
    }
}

