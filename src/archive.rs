use std::{
    path::{Path, PathBuf},
    io::{Seek, SeekFrom},
    time::Instant,
    cmp::min,
};

use crate::{
    file_len, 
    metadata::Metadata,
    encoder::Encoder,
    decoder::Decoder,
    buffered_io::{
        BufferedRead, BufferedWrite, BufferState,
        new_input_file, new_output_file, new_dir_checked,
    },
    formatting::{
        fmt_file_out_ns_archive,
        fmt_file_out_ns_extract,
        fmt_nested_dir_ns_archive,
        fmt_nested_dir_ns_extract,
        fmt_file_out_s_extract,
    },
    Arch,
    parse_args::Config,
};

/// Check for a valid magic number.
/// Non-solid archives - 'prsv'
/// Solid archives     - 'PRSV'
fn verify_magic_number(mgc: usize, arch: Arch) {
    match (arch, mgc) {
        (Arch::Solid, 0x5653_5250) => {},
        (Arch::Solid, 0x7673_7270) => {
            println!();
            println!("Expected solid archive, found non-solid archive.");
            std::process::exit(0);
        },
        (Arch::NonSolid, 0x7673_7270) => {},
        (Arch::NonSolid, 0x5653_5250) => {
            println!();
            println!("Expected non-solid archive, found solid archive.");
            std::process::exit(0);
        }
        (_, _) => {
            println!("Not a prisirv archive.");
            std::process::exit(0);
        }
    }
}


// Non-solid archiving --------------------------------------------------------------------------------------------------------------------
pub struct Archiver {
    cfg:    Config,
    files:  Vec<PathBuf>, // Keep list of files already compressed to prevent accidental clobbering
}
impl Archiver {
    pub fn new(cfg: Config) -> Archiver {
        Archiver {
            cfg,
            files: Vec::with_capacity(32),
        }
    }
    pub fn compress_file(&mut self, file_in_path: &Path, dir_out: &str) -> u64 {
        let mut mta: Metadata = Metadata::new();

        let file_out_path = fmt_file_out_ns_archive(dir_out, file_in_path, self.cfg.clbr, &self.files);
        if self.cfg.clbr { self.files.push(file_out_path.clone()); }
        

        // Create input file with buffer = block size
        let mut file_in = new_input_file(mta.bl_sz, file_in_path);
        let mut enc = Encoder::new(new_output_file(4096, &file_out_path), &self.cfg);

        // Set metadata extension field
        mta.set_ext(file_in_path);

        // Compress
        loop {
            if file_in.fill_buffer() == BufferState::Empty { break; }
            mta.f_bl_sz = file_in.buffer().len();
            enc.compress_block(file_in.buffer());
            mta.bl_c += 1;
        }

        enc.flush();
        enc.write_header(&mta, Arch::NonSolid);
        file_len(&file_out_path)
    }
    
    pub fn compress_dir(&mut self, dir_in: &Path, dir_out: &mut String) {
        let mut dir_out = fmt_nested_dir_ns_archive(dir_out, dir_in);
        new_dir_checked(&dir_out, self.cfg.clbr);

        // Sort files and directories
        let (files, dirs): (Vec<PathBuf>, Vec<PathBuf>) = 
            dir_in.read_dir().unwrap()
            .map(|d| d.unwrap().path())
            .partition(|f| f.is_file());

        // Compress files first, then directories
        for file_in in files.iter() {
            let time = Instant::now();
            if !self.cfg.quiet { println!("Compressing {}", file_in.display()); }
            let file_in_size  = file_len(file_in); 
            let file_out_size = self.compress_file(file_in, &dir_out);
            if !self.cfg.quiet { println!("{} bytes -> {} bytes in {:.2?}\n", 
                file_in_size, file_out_size, time.elapsed()); }
        }
        for dir_in in dirs.iter() {
            self.compress_dir(dir_in, &mut dir_out); 
        }
    }  
}

pub struct Extractor {
    cfg: Config,
}
impl Extractor {
    pub fn new(cfg: Config) -> Extractor {
        Extractor {
            cfg,
        }
    }
    pub fn decompress_file(&self, file_in_path: &Path, dir_out: &str) -> u64 {
        let mut dec = Decoder::new(new_input_file(4096, file_in_path));
        let mta: Metadata = dec.read_header(Arch::NonSolid);

        verify_magic_number(mta.mgc, Arch::NonSolid);

        let file_out_path = fmt_file_out_ns_extract(&mta.get_ext(), dir_out, file_in_path);
        let mut file_out = new_output_file(4096, &file_out_path);
        
        // Call after reading header
        dec.init_x();

        // Decompress full blocks
        for _ in 0..(mta.bl_c - 1) {
            let block = dec.decompress_block(mta.bl_sz);
            for byte in block.iter() {
                file_out.write_byte(*byte);
            }
        }
        // Decompress final variable size block
        let block = dec.decompress_block(mta.f_bl_sz);
        for byte in block.iter() {
            file_out.write_byte(*byte);
        }

        file_out.flush_buffer();
        file_len(&file_out_path)
    }
    pub fn decompress_dir(&self, dir_in: &Path, dir_out: &mut String, root: bool) {
        let mut dir_out = fmt_nested_dir_ns_extract(dir_out, dir_in, root);
        new_dir_checked(&dir_out, self.cfg.clbr);

        // Sort files and directories
        let (files, dirs): (Vec<PathBuf>, Vec<PathBuf>) =
            dir_in.read_dir().unwrap()
            .map(|d| d.unwrap().path())
            .partition(|f| f.is_file());

        // Decompress files first, then directories
        for file_in in files.iter() {
            let time = Instant::now();
            if !self.cfg.quiet { println!("Decompressing {}", file_in.display()); }
            let file_in_size  = file_len(file_in);
            let file_out_size = self.decompress_file(file_in, &dir_out);
            if !self.cfg.quiet { println!("{} bytes -> {} bytes in {:.2?}\n",
                file_in_size, file_out_size, time.elapsed()); }
        }
        for dir_in in dirs.iter() {
            self.decompress_dir(dir_in, &mut dir_out, false); 
        }
    }
}
// ----------------------------------------------------------------------------------------------------------------------------------------


// Solid Archiving ------------------------------------------------------------------------------------------------------------------------
pub struct SolidArchiver {
    pub enc:  Encoder,
    mta:      Metadata,
    cfg:      Config,
}
impl SolidArchiver {
    pub fn new(enc: Encoder, mta: Metadata, cfg: Config) -> SolidArchiver {
        SolidArchiver {
            enc, mta, cfg,
        }
    }
    pub fn create_archive(&mut self) {
        for curr_file in 0..self.mta.files.len() {
            if !self.cfg.quiet { println!("Compressing {}", self.mta.files[curr_file].0); }
            let archive_size = self.compress_file_solid(curr_file);
            if !self.cfg.quiet { println!("Total archive size: {}\n", archive_size); }
        }
        self.enc.flush();
    }
    pub fn compress_file_solid(&mut self, curr_file: usize) -> u64 {
        // Create input file with buffer = block size
        let mut file_in = 
            new_input_file(
                self.mta.bl_sz, 
                Path::new(&self.mta.files[curr_file].0)
            );

        // Compress
        loop {
            if file_in.fill_buffer() == BufferState::Empty { break; }
            self.mta.files[curr_file].2 = file_in.buffer().len();
            self.enc.compress_block(file_in.buffer());
            self.mta.files[curr_file].1 += 1;
        }
        self.enc.file_out.stream_position().unwrap()
    }
    fn write_footer(&mut self) {
        // Get index to end of file metadata
        self.mta.f_ptr =
            self.enc.file_out.stream_position()
            .unwrap() as usize;

        // Output number of files
        self.enc.file_out.write_usize(self.mta.files.len());

        for file in self.mta.files.iter() {
            // Get path as byte slice, truncated if longer than 255 bytes
            let path = &file.0.as_bytes()[..min(file.0.len(), 255)];

            // Output length of file path (for parsing)
            self.enc.file_out.write_byte(path.len() as u8);

            // Output path
            for byte in path.iter() {
                self.enc.file_out.write_byte(*byte);
            }

            // Output block count and final block size
            self.enc.file_out.write_usize(file.1);
            self.enc.file_out.write_usize(file.2);
        }

        // Return final archive size including footer
        if !self.cfg.quiet {
            println!("Final archive size: {}", 
            self.enc.file_out.seek(SeekFrom::End(0)).unwrap());
        }
    }
    // For more info on metadata structure, see metadata.rs
    pub fn write_metadata(&mut self) {
        self.write_footer();
        // Go back to beginning of file and write header
        self.enc.write_header(&self.mta, Arch::Solid);
    }
}

pub struct SolidExtractor {
    dec:    Decoder,
    mta:    Metadata,
    cfg:    Config,
}
impl SolidExtractor {
    pub fn new(dec: Decoder, mta: Metadata, cfg: Config) -> SolidExtractor {
        SolidExtractor {
            dec, mta, cfg,
        }
    }
    pub fn extract_archive(&mut self, dir_out: &str) {
        new_dir_checked(dir_out, self.cfg.clbr);
        for curr_file in 0..self.mta.files.len() {
            if !self.cfg.quiet { println!("Decompressing {}", self.mta.files[curr_file].0); }
            self.decompress_file_solid(dir_out, curr_file);
        }
    }
    pub fn decompress_file_solid(&mut self, dir_out: &str, curr_file: usize) {
        let file_out_path = fmt_file_out_s_extract(dir_out, Path::new(&self.mta.files[curr_file].0));
        let mut file_out = new_output_file(4096, &file_out_path);

        // Decompress full blocks
        for _ in 0..((self.mta.files[curr_file].1) - 1) {
            let block = self.dec.decompress_block(self.mta.bl_sz);
            for byte in block.iter() {
                file_out.write_byte(*byte);
            }
        }
        // Decompress final variable size block
        let block = self.dec.decompress_block(self.mta.files[curr_file].2);
        for byte in block.iter() {
            file_out.write_byte(*byte);
        }
        file_out.flush_buffer();
    }
    fn read_footer(&mut self) {
        // Seek to end of file metadata
        self.dec.file_in.seek(SeekFrom::Start(self.mta.f_ptr as u64)).unwrap();
        let mut path: Vec<u8> = Vec::new();

        // Get number of files
        let num_files = self.dec.file_in.read_usize();

        // Parse files and lengths
        for _ in 0..num_files {
            // Get length of next path
            let len = self.dec.file_in.read_byte();

            // Get path, block count, final block size
            for _ in 0..len {
                path.push(self.dec.file_in.read_byte());
            }
            self.mta.files.push(
                (path.iter().cloned()
                    .map(|b| b as char)
                    .collect::<String>(),
                self.dec.file_in.read_usize(),
                self.dec.file_in.read_usize())
            );
            path.clear();
        }

        // Seek back to beginning of compressed data
        #[cfg(target_pointer_width = "64")]
        self.dec.file_in.seek(SeekFrom::Start(32)).unwrap();

        #[cfg(target_pointer_width = "32")]
        self.dec.file_in.seek(SeekFrom::Start(16)).unwrap();
    }
    // For more info on metadata structure, see metadata.rs
    pub fn read_metadata(&mut self) {
        self.mta = self.dec.read_header(Arch::Solid);
        verify_magic_number(self.mta.mgcs, Arch::Solid);
        self.read_footer();
        self.dec.init_x();
    }
}
// ----------------------------------------------------------------------------------------------------------------------------------------

