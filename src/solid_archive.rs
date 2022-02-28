use std::{
    path::{Path, PathBuf},
    io::{Seek, SeekFrom, BufWriter},
    fs::File,
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

        for file in self.mta.files.iter() {
            let file_path = Path::new(&file.0);
            let mut file_in = new_input_file(blk.capacity(), file_path);

            while file_in.fill_buffer() == BufferState::NotEmpty {
                blk.append(&mut file_in.buffer().to_vec());
                self.mta.fblk_sz = blk.len();
                
                // Compress full block
                if blk.capacity() - blk.len() == 0 { 
                    tp.compress_block(blk.clone(), self.mta.blk_c, blk.len());
                    self.mta.blk_c += 1;
                    blk.clear();
                }
            }
        }
        // Compress final block
        tp.compress_block(blk.clone(), self.mta.blk_c, blk.len());
        self.mta.blk_c += 1;

        // Output blocks
        let mut blks_wrtn = 0;
        while blks_wrtn != self.mta.blk_c {
            blks_wrtn += tp.bq.lock().unwrap().try_write_block_enc(&mut self.mta, &mut self.enc);
        }
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

            // Output file length
            self.enc.file_out.write_u64(file.1);
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

        let mut tp = ThreadPool::new(self.cfg.threads, self.dec.mem, self.prg);
        let mut index = 0;
        
        // Decompress blocks ----------------------------------------
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
        // ----------------------------------------------------------

        let mut file_in_paths = self.mta.files.iter().map(|f| PathBuf::from(f.0.clone()));   
        let mut file_out_paths = Vec::new();

        let (mut file_in_len, mut file_out) = 
            next_file(
                &file_in_paths.next().unwrap_or_else(|| exit(0)), 
                dir_out, &mut file_out_paths
            );

        let mut file_out_pos = 0;
        let mut blks_wrtn = 0;
        // Write blocks to output -----------------------------------
        while blks_wrtn != self.mta.blk_c {
            match tp.bq.lock().unwrap().try_get_block() {
                Some(block) => {
                    for byte in block.iter() {
                        if file_out_pos == file_in_len {
                            (file_in_len, file_out) = 
                                next_file(
                                    &file_in_paths.next().unwrap_or_else(|| exit(0)), 
                                    dir_out, &mut file_out_paths
                                );
                            file_out_pos = 0;
                        }
                        file_out.write_byte(*byte);
                        file_out_pos += 1;
                    }
                    blks_wrtn += 1;
                }
                None => {},
            }
        }
        // ----------------------------------------------------------

        file_out.flush_buffer();
        self.prg.print_archive_stats(file_out_paths.iter().map(|f| file_len(f)).sum());
    }
    fn read_footer(&mut self) {
        // Seek to end of file metadata
        self.dec.file_in.seek(SeekFrom::Start(self.mta.f_ptr as u64)).unwrap();
        let mut path: Vec<u8> = Vec::new();

        // Get number of files
        let num_files = self.dec.file_in.read_usize();

        for _ in 0..num_files {
            // Get length of next path
            let len = self.dec.file_in.read_byte();

            // Get file path and length
            for _ in 0..len {
                path.push(self.dec.file_in.read_byte());
            }
            self.mta.files.push(
                (path.iter().cloned()
                    .map(|b| b as char)
                    .collect::<String>(),
                self.dec.file_in.read_u64())
            );
            path.clear();
        }

        // Get compressed block sizes
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
    let file_in_len   = file_len(&file_in_path);
    let file_out_path = fmt_file_out_s_extract(dir_out, &file_in_path);
    let file_out      = new_output_file(4096, &file_out_path);
    file_out_paths.push(file_out_path);
    (file_in_len, file_out)
}