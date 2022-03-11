use std::{
    path::{Path, PathBuf},
    io::{Seek, SeekFrom, BufWriter},
    fs::File,
    cmp::min,
};

use crate::{
    Mode,
    sort::{Sort, sort_files},
    metadata::Metadata,
    threads::ThreadPool,
    progress::Progress,
    config::Config,
    buffered_io::{
        BufferedRead, BufferedWrite, file_len,
        new_input_file, new_output_file_checked,
    },
};

/// Recursively collect all files into a vector for sorting before compression.
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

/// A solid archiver creates solid archives, or an archive containing files 
/// compressed as one stream. Solid archives take advantage of redundancy 
/// across files and therefore achieve better compression ratios than non-
/// solid archives, but don't allow for extracting individual files.
pub struct SolidArchiver {
    pub file_out:  BufWriter<File>,
    mta:           Metadata,
    cfg:           Config,
    prg:           Progress,
}
impl SolidArchiver {
    /// Create a new SolidArchiver.
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

    /// Parse files into blocks and compress blocks.
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

        self.prg.get_archive_size_enc(&self.mta.files);
        let mut tp = ThreadPool::new(self.cfg.threads, self.cfg.mem, self.prg);
        let mut blk = Vec::with_capacity(self.cfg.blk_sz);

        for file in self.mta.files.iter() {
            let file_path = Path::new(&file.0);
            let file_len = file_len(file_path);
            let mut file_in = new_input_file(blk.capacity(), file_path);

            for _ in 0..file_len {
                blk.push(file_in.read_byte());
                
                // Compress full block
                if blk.len() == blk.capacity() {
                    tp.compress_block(blk.clone(), self.mta.blk_c, blk.len());
                    self.mta.blk_c += 1;
                    blk.clear();
                }
            }
        }
        self.mta.fblk_sz = 
            if blk.is_empty() { blk.capacity() } 
            else { blk.len() };

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

    /// Write footer containing file paths and lengths.
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

    /// Write 48 byte header.
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
