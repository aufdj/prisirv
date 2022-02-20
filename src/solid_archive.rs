use std::{
    path::Path,
    io::{Seek, SeekFrom},
    cmp::min,
};

use crate::{
    Arch,
    metadata::Metadata,
    encoder::Encoder,
    decoder::Decoder,
    formatting::fmt_file_out_s_extract,
    parse_args::Config,
    buffered_io::{
        BufferedRead, BufferedWrite, BufferState,
        new_input_file, new_output_file, new_dir_checked,
    },
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

/// Solid Archiver ======================================================================
///
/// A solid archiver creates solid archives. A solid archive is an archive containing 
/// files compressed as one stream. Solid archives can take advantage of redundancy 
/// across files and therefore achieve better compression ratios than non-solid 
/// archives, but don't allow for extracting individual files.
///
/// =====================================================================================
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
                self.mta.blk_sz, 
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


/// A SolidExtractor extracts solid archives.
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
            let block = self.dec.decompress_block(self.mta.blk_sz);
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