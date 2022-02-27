use std::{
    path::{Path, PathBuf},
    io::{Seek, SeekFrom},
    process::exit,
    cmp::min,
};

use crate::{
    Arch, Mode,
    metadata::Metadata,
    encoder::Encoder,
    decoder::Decoder,
    threads::ThreadPool,
    progress::Progress,
    formatting::fmt_file_out_s_extract,
    parse_args::Config,
    buffered_io::{
        BufferedRead, BufferedWrite, BufferState, file_len,
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
            exit(0);
        },
        (Arch::NonSolid, 0x7673_7270) => {},
        (Arch::NonSolid, 0x5653_5250) => {
            println!();
            println!("Expected non-solid archive, found solid archive.");
            exit(0);
        }
        (_, _) => {
            println!("Not a prisirv archive.");
            exit(0);
        }
    }
}


/// A solid archiver creates solid archives. A solid archive is an archive containing
/// files compressed as one stream. Solid archives can take advantage of redundancy
/// across files and therefore achieve better compression ratios than non-solid
/// archives, but don't allow for extracting individual files.
pub struct SolidArchiver {
    pub enc:  Encoder,
    mta:      Metadata,
    cfg:      Config,
    prg:      Progress,
}
impl SolidArchiver {
    pub fn new(enc: Encoder, mta: Metadata, cfg: Config) -> SolidArchiver {
        let prg = Progress::new(&cfg, Mode::Compress);
        SolidArchiver {
            enc, mta, cfg, prg,
        }
    }
    pub fn create_archive(&mut self) {
        self.prg.get_input_size_solid_enc(&self.mta.files);
        let mut tp = ThreadPool::new(self.cfg.threads, self.cfg.mem, self.prg);

        let mut blk = Vec::with_capacity(self.cfg.blk_sz);
        let mut rem_cap = blk.capacity();
        let mut index = 0;
        let mut blks_wrtn = 0;

        for curr_file in 0..self.mta.files.len() {
            //let file_len = file_len(file_path);
            let file_path = Path::new(&self.mta.files[curr_file].0);
            let mut file_in = new_input_file(rem_cap, file_path);

            while file_in.fill_buffer() == BufferState::NotEmpty {
                blk.append(&mut file_in.buffer().to_vec());
                rem_cap = blk.capacity() - blk.len();
                self.mta.fblk_sz = blk.len();
                // Compress full block
                if rem_cap == 0 {
                    tp.compress_block(blk.clone(), index, blk.len());
                    self.mta.blk_c += 1;
                    index += 1;
                    blk.clear();
                    rem_cap = blk.capacity();
                }
            }
        }
        // Compress final block
        tp.compress_block(blk.clone(), index, blk.len());
        self.mta.blk_c += 1;

        // Output blocks
        while blks_wrtn != self.mta.blk_c {
            blks_wrtn += tp.bq.lock().unwrap().try_write_block_enc(&mut self.mta, &mut self.enc);
        }
        //self.enc.flush();
        self.enc.file_out.flush_buffer();
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
        // Write compressed block sizes
        for blk_sz in self.mta.enc_blk_szs.iter() {
            self.enc.file_out.write_usize(*blk_sz);
        }

        // Return final archive size including footer
        self.prg.print_archive_stats(self.enc.file_out.seek(SeekFrom::End(0)).unwrap());
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
    dec:  Decoder,
    mta:  Metadata,
    cfg:  Config,
    prg:  Progress,
}
impl SolidExtractor {
    pub fn new(dec: Decoder, mta: Metadata, cfg: Config) -> SolidExtractor {
        let prg = Progress::new(&cfg, Mode::Decompress);
        SolidExtractor {
            dec, mta, cfg, prg,
        }
    }
    pub fn extract_archive(&mut self, dir_out: &str) {
        self.prg.get_input_size_solid_dec(&self.cfg.inputs, self.mta.blk_c);
        new_dir_checked(dir_out, self.cfg.clbr);

        let mut index = 0;
        let mut blks_wrtn = 0;
        let mut tp = ThreadPool::new(self.cfg.threads, self.dec.mem, self.prg);

        // Decompress blocks --------------------------------------
        for _ in 0..self.mta.blk_c-1 {
            let mut blk_in = Vec::with_capacity(self.mta.blk_sz);
            for _ in 0..self.mta.enc_blk_szs[index] {
                blk_in.push(self.dec.file_in.read_byte());
            }
            tp.decompress_block(blk_in, index, self.mta.blk_sz);
            index += 1;
        }
        let mut blk_in = Vec::with_capacity(self.mta.blk_sz);
        for _ in 0..self.mta.enc_blk_szs[index] {
            blk_in.push(self.dec.file_in.read_byte());
        }
        tp.decompress_block(blk_in, index, self.mta.fblk_sz);
        // --------------------------------------------------------

        let mut file_in_paths = self.mta.files.iter().map(|f| PathBuf::from(f.0.clone()));
            
        let mut file_in_path = file_in_paths.next().unwrap_or_else(|| exit(0));
        let mut file_in_len = file_len(&file_in_path);
        let mut file_out_path = fmt_file_out_s_extract(dir_out, &file_in_path);
        let mut file_out = new_output_file(4096, &file_out_path);
        let mut file_out_pos = 0;
        let mut file_out_paths = vec![file_out_path];
        
        while blks_wrtn != self.mta.blk_c {
            match tp.bq.lock().unwrap().try_get_block() {
                Some(block) => {
                    blks_wrtn += 1;
                    for byte in block.iter() {
                        if file_out_pos == file_in_len {
                            file_in_path = file_in_paths.next().unwrap_or_else(|| exit(0));
                            file_in_len = file_len(&file_in_path);
                            file_out_path = fmt_file_out_s_extract(dir_out, &file_in_path);
                            file_out = new_output_file(4096, &file_out_path);
                            file_out_paths.push(file_out_path);
                            file_out_pos = 0;
                        }
                        file_out.write_byte(*byte);
                        file_out_pos += 1;
                    }
                }
                None => {},
            }
        }
        file_out.flush_buffer();
        self.prg.print_archive_stats(dir_size(&file_out_paths));
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
        for _ in 0..self.mta.blk_c {
            self.mta.enc_blk_szs.push(self.dec.file_in.read_usize());
        }

        // Seek back to beginning of compressed data
        #[cfg(target_pointer_width = "64")]
        self.dec.file_in.seek(SeekFrom::Start(48)).unwrap();

        #[cfg(target_pointer_width = "32")]
        self.dec.file_in.seek(SeekFrom::Start(24)).unwrap();
    }
    // For more info on metadata structure, see metadata.rs
    pub fn read_metadata(&mut self) {
        self.mta = self.dec.read_header(Arch::Solid);
        verify_magic_number(self.mta.mgcs, Arch::Solid);
        self.read_footer();
    }
}

fn dir_size(files: &[PathBuf]) -> u64 {
    let mut size: u64 = 0;
    for file in files.iter() {
        size += file_len(file);
    }
    size
}