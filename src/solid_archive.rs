use std::{
    path::{Path, PathBuf},
    io::{Seek, SeekFrom, BufWriter, BufReader},
    fs::File,
    process::exit,
    cmp::min,
};

use crate::{
    Mode,
    sort::{Sort, sort_files},
    metadata::Metadata,
    threads::ThreadPool,
    progress::Progress,
    formatting::fmt_file_out_s_extract,
    config::Config,
    buffered_io::{
        BufferedRead, BufferedWrite, file_len,
        new_input_file, new_output_file, 
        new_dir_checked, new_output_file_checked,
    },
};

// Recursively collect all files into a vector for sorting before compression.
fn collect_files(dir_in: &Path, mta: &mut Metadata) {
    let (files, dirs): (Vec<PathBuf>, Vec<PathBuf>) =
        dir_in.read_dir().unwrap()
        .map(|d| d.unwrap().path())
        .partition(|f| f.is_file());

    for file in files.iter() {
        mta.files.push(
            (file.display().to_string(), file_len(file))
        );
    }
    for dir in dirs.iter() {
        collect_files(dir, mta);
    }
}

/// A solid archiver creates solid archives. A solid archive is an archive containing
/// files compressed as one stream. Solid archives can take advantage of redundancy
/// across files and therefore achieve better compression ratios than non-solid
/// archives, but don't allow for extracting individual files.
pub struct SolidArchiver {
    pub file_out:  BufWriter<File>,
    mta:           Metadata,
    cfg:           Config,
    prg:           Progress,
}
impl SolidArchiver {
    pub fn new(cfg: Config) -> SolidArchiver {
        let mut mta: Metadata = Metadata::new();
        mta.blk_sz = cfg.blk_sz;
        mta.mem = cfg.mem;

        let prg = Progress::new(&cfg, Mode::Compress);

        let mut file_out = new_output_file_checked(&cfg.dir_out, cfg.clbr);
        for _ in 0..6 { file_out.write_u64(0); }

        SolidArchiver {
            file_out, mta, cfg, prg,
        }
    }
    pub fn create_archive(&mut self) {
        // Group files and directories 
        let (files, dirs): (Vec<PathBuf>, Vec<PathBuf>) =
            self.cfg.inputs.clone().into_iter().partition(|f| f.is_file());

        // Walk through directories and collect all files
        for file in files.iter() {
            self.mta.files.push(
                (file.display().to_string(), file_len(file))
            );
        }
        for dir in dirs.iter() {
            collect_files(dir, &mut self.mta);
        }

        // Sort files to potentially improve compression of solid archives
        let sort_method = self.cfg.sort;
        match self.cfg.sort {
            Sort::None => {},
            _ => self.mta.files.sort_by(|f1, f2| sort_files(&f1.0, &f2.0, &sort_method)),
        }

        self.prg.get_input_size_solid_enc(&self.mta.files);
        let mut tp = ThreadPool::new(self.cfg.threads, self.cfg.mem, self.prg);
        let mut blk = Vec::with_capacity(self.cfg.blk_sz);

        for file in self.mta.files.iter() {
            let file_path = Path::new(&file.0);
            let file_len = file_len(file_path);
            let mut file_in = new_input_file(blk.capacity(), file_path);

            for _ in 0..file_len {
                blk.push(file_in.read_byte());
                
                // Compress full block
                if blk.len() == self.cfg.blk_sz {
                    tp.compress_block(blk.clone(), self.mta.blk_c, blk.len());
                    self.mta.blk_c += 1;
                    blk.clear();
                }
            }
        }
        self.mta.fblk_sz = blk.len(); // FIXME: flbk_sz will be 0 if block size is equal to or multiple of archive size
        // Compress final block
        tp.compress_block(blk.clone(), self.mta.blk_c, blk.len());
        self.mta.blk_c += 1;

        // Output blocks
        let mut blks_wrtn: u64 = 0;
        while blks_wrtn != self.mta.blk_c {
            blks_wrtn += tp.bq.lock().unwrap().try_write_block_enc(&mut self.mta, &mut self.file_out);
        }
        self.file_out.flush_buffer();

        self.write_metadata();
    }

    /// Write footer, then go back to beginning of file and write header.
    pub fn write_metadata(&mut self) {
        self.write_footer();
        self.write_header();
    }

    fn write_footer(&mut self) {
        // Get index to footer
        self.mta.f_ptr = self.file_out.stream_position().unwrap();

        // Output number of files
        self.file_out.write_u64(self.mta.files.len() as u64);

        for file in self.mta.files.iter() {
            // Get path as byte slice, truncated if longer than 255 bytes
            let path = &file.0.as_bytes()[..min(file.0.len(), 255)];

            // Output length of file path (for parsing)
            self.file_out.write_byte(path.len() as u8);

            // Output path
            for byte in path.iter() {
                self.file_out.write_byte(*byte);
            }

            // Output file length
            self.file_out.write_u64(file.1);
        }

        // Write compressed block sizes
        for blk_sz in self.mta.enc_blk_szs.iter() {
            self.file_out.write_u64(*blk_sz);
        }

        // Return final archive size including footer
        self.prg.print_archive_stats(self.file_out.seek(SeekFrom::End(0)).unwrap());
    }
    fn write_header(&mut self) {
        self.file_out.rewind().unwrap();
        self.file_out.write_u64(self.mta.mem);     
        self.file_out.write_u64(self.mta.mgcs);
        self.file_out.write_u64(self.mta.blk_sz as u64);
        self.file_out.write_u64(self.mta.fblk_sz as u64);
        self.file_out.write_u64(self.mta.blk_c);
        self.file_out.write_u64(self.mta.f_ptr);
    }
}


/// A SolidExtractor extracts solid archives.
pub struct SolidExtractor {
    file_in:  BufReader<File>,
    mta:      Metadata,
    cfg:      Config,
    prg:      Progress,
}
impl SolidExtractor {
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
                // Output block
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

    // For more info on metadata structure, see metadata.rs
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

/// Get the next output file and the input file length,
/// the input file being the original file that was compressed.
/// i.e. output file is foo_d\bar.txt, input file is foo\bar.txt
///
/// The input file length is needed to know when the output file
/// is the correct size.
///
/// The output file paths are tracked so the final extracted archive
/// size can be computed at the end of extraction.
fn next_file(file_in_path: &Path, dir_out: &str, file_out_paths: &mut Vec<PathBuf>) -> (u64, BufWriter<File>) {
    let file_in_len   = file_len(file_in_path);
    let file_out_path = fmt_file_out_s_extract(dir_out, file_in_path);
    let file_out      = new_output_file(4096, &file_out_path);
    file_out_paths.push(file_out_path);
    (file_in_len, file_out)
}