use std::{
    path::{Path, PathBuf},
    io::{Seek, SeekFrom, BufWriter, BufReader},
    fs::File,
    process::exit,
};

use crate::{
    Mode,
    metadata::Metadata,
    threads::ThreadPool,
    progress::Progress,
    formatting::fmt_file_out_s_extract,
    config::Config,
    buffered_io::{
        BufferedRead, BufferedWrite, file_len,
        new_input_file, new_output_file, 
        new_dir_checked, 
    },
};

/// A SolidExtractor extracts solid archives.
pub struct SolidExtractor {
    file_in:  BufReader<File>,
    pub mta:  Metadata,
    pub cfg:  Config,
    prg:      Progress,
}
impl SolidExtractor {
    /// Create a new SolidExtractor.
    pub fn new(cfg: Config) -> SolidExtractor {
        if !cfg.inputs[0].is_file() {
            println!("Input {} is not a solid archive.", cfg.inputs[0].display());
            println!("To extract a non-solid archive, omit option '-sld'.");
            std::process::exit(0);
        }

        let mta: Metadata = Metadata::new();
        let prg = Progress::new(&cfg, Mode::Decompress);
        let file_in = new_input_file(4096, &cfg.inputs[0]);

        SolidExtractor {
            file_in, mta, cfg, prg, 
        }
    }

    /// Decompress blocks and parse blocks into files. A block can span 
    /// multiple files.
    pub fn extract_archive(&mut self) {
        self.read_metadata();
        self.prg.get_input_size_solid_dec(&self.cfg.inputs, self.mta.blk_c);
        new_dir_checked(&self.cfg.dir_out, self.cfg.clbr);

        let mut tp = ThreadPool::new(self.cfg.threads, self.mta.mem, self.prg);
        let mut index: u64 = 0;
        
        // Decompress blocks ----------------------------------------
        for _ in 0..self.mta.blk_c-1 {
            let mut blk_in = Vec::with_capacity(self.mta.blk_sz);
            for _ in 0..self.mta.enc_blk_szs[index as usize] {
                blk_in.push(self.file_in.read_byte());
            }
            tp.decompress_block(blk_in, index, self.mta.blk_sz);
            index += 1;
        }
        let mut blk_in = Vec::with_capacity(self.mta.blk_sz);
        for _ in 0..self.mta.enc_blk_szs[index as usize] {
            blk_in.push(self.file_in.read_byte());
        }
        tp.decompress_block(blk_in, index, self.mta.fblk_sz);
        // ----------------------------------------------------------


        let mut file_in_paths = 
            self.mta.files.iter()
            .map(|f| PathBuf::from(f.0.clone()))
            .collect::<Vec<PathBuf>>().into_iter();

        let mut file_out_paths = Vec::new();

        let file_in_path = match file_in_paths.next() {
            Some(path) => {
                if !path.is_file() { exit(0); }
                path
            }
            None => exit(0),
        };

        let (mut file_in_len, mut file_out) = 
            next_file(&file_in_path, &self.cfg.dir_out, &mut file_out_paths);

        let mut file_out_pos = 0;
        let mut blks_wrtn: u64 = 0;
        // Write blocks to output -----------------------------------
        while blks_wrtn != self.mta.blk_c {
            if let Some(block) = tp.bq.lock().unwrap().try_get_block() { 
                for byte in block.iter() {
                    // When current output file reaches the 
                    // correct size, move to next file.
                    if file_out_pos == file_in_len {
                        let file_in_path = match file_in_paths.next() {
                            Some(path) => {
                                if !path.is_file() { break; }
                                path
                            }
                            None => break,
                        };
                        (file_in_len, file_out) =  
                            next_file(&file_in_path, &self.cfg.dir_out, &mut file_out_paths);
                        file_out_pos = 0;
                    }
                    file_out.write_byte(*byte);
                    file_out_pos += 1;
                }
                blks_wrtn += 1;   
            }
        }
        // ----------------------------------------------------------

        file_out.flush_buffer();
        self.prg.print_archive_stats(file_out_paths.iter().map(|f| file_len(f)).sum());
    }

    /// Read footer containing file paths and lengths.
    fn read_footer(&mut self) {
        // Seek to end of file metadata
        self.file_in.seek(SeekFrom::Start(self.mta.f_ptr)).unwrap();
        let mut path: Vec<u8> = Vec::with_capacity(64);

        // Get number of files
        let num_files = self.file_in.read_u64();

        for _ in 0..num_files {
            // Get length of next path
            let len = self.file_in.read_byte();

            // Get file path and length
            for _ in 0..len {
                path.push(self.file_in.read_byte());
            }

            let path_string = path.iter().cloned().map(|b| b as char).collect::<String>();
            let file_len = self.file_in.read_u64();

            self.mta.files.push((path_string, file_len));
            path.clear();
        }

        // Get compressed block sizes
        for _ in 0..self.mta.blk_c {
            self.mta.enc_blk_szs.push(self.file_in.read_u64());
        }

        // Seek back to beginning of compressed data
        self.file_in.seek(SeekFrom::Start(48)).unwrap();
    }

    /// Read 48 byte header.
    fn read_header(&mut self) -> Metadata {
        let mut mta: Metadata = Metadata::new();
        mta.mem     = self.file_in.read_u64();
        mta.mgcs    = self.file_in.read_u64();
        mta.blk_sz  = self.file_in.read_u64() as usize;
        mta.fblk_sz = self.file_in.read_u64() as usize;
        mta.blk_c   = self.file_in.read_u64();
        mta.f_ptr   = self.file_in.read_u64();
        mta
    }

    /// Read header and footer of archive.
    pub fn read_metadata(&mut self) {
        self.mta = self.read_header();
        self.verify_magic_number(self.mta.mgcs);
        self.read_footer();
    }

    /// Check for a valid magic number.
    /// * Non-solid archives - 'prsv'
    /// * Solid archives     - 'PRSV'
    fn verify_magic_number(&self, mgc: u64) {
        match mgc {
            0x5653_5250 => {},
            0x7673_7270 => {
                println!();
                println!("Expected solid archive, found non-solid archive.");
                exit(0);
            },
            _ => {
                println!("Not a prisirv archive.");
                exit(0);
            }
        }
    }
}

/// Get the next output file and the input file length, the input file being
/// the original file that was compressed.
/// i.e. output file is foo_d\bar.txt, input file is foo\bar.txt
///
/// The input file length is needed to know when the output file is the 
/// correct size.
///
/// The output file paths are tracked so the final extracted archive size 
/// can be computed at the end of extraction.
fn next_file(file_in_path: &Path, dir_out: &str, file_out_paths: &mut Vec<PathBuf>) -> (u64, BufWriter<File>) {
    let file_in_len   = file_len(file_in_path);
    let file_out_path = fmt_file_out_s_extract(dir_out, file_in_path);
    let file_out      = new_output_file(4096, &file_out_path);
    file_out_paths.push(file_out_path);
    (file_in_len, file_out)
}
