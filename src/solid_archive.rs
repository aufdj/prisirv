use std::{
    path::{Path, PathBuf},
    io::{Seek, BufWriter},
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
    pub archive:  BufWriter<File>,
    cfg:          Config,
    prg:          Progress,
}
impl SolidArchiver {
    /// Create a new SolidArchiver.
    pub fn new(cfg: Config) -> SolidArchiver {
        let prg = Progress::new(&cfg, Mode::Compress);

        let mut archive = new_output_file_checked(&cfg.dir_out, cfg.clbr);
        for _ in 0..6 { archive.write_u64(0); }

        SolidArchiver { archive, cfg, prg }
    }

    /// Parse files into blocks and compress blocks.
    pub fn create_archive(&mut self) {
        let mut mta: Metadata = Metadata::new_with_cfg(&self.cfg);
        // Group files and directories 
        let (files, dirs): (Vec<PathBuf>, Vec<PathBuf>) =
            self.cfg.inputs.clone().into_iter().partition(|f| f.is_file());

        // Walk through directories and collect all files
        for file in files.iter() {
            mta.files.push(
                (file.display().to_string(), file_len(file))
            );
        }
        for dir in dirs.iter() {
            collect_files(dir, &mut mta);
        }

        // Sort files to potentially improve compression of solid archives
        match self.cfg.sort {
            Sort::None => {},
            _ => mta.files.sort_by(|f1, f2| sort_files(&f1.0, &f2.0, self.cfg.sort)),
        }

        self.prg.get_archive_size_enc(&mta.files);
        let mut tp = ThreadPool::new(self.cfg.threads, self.cfg.mem, self.prg);
        let mut blk = Vec::with_capacity(self.cfg.blk_sz);

        for file in mta.files.iter() {
            let file_path = Path::new(&file.0);
            let file_len = file_len(file_path);
            let mut file_in = new_input_file(blk.capacity(), file_path);

            for _ in 0..file_len {
                blk.push(file_in.read_byte());
                
                // Compress full block
                if blk.len() == blk.capacity() {
                    tp.compress_block(blk.clone(), mta.blk_c, blk.len());
                    mta.blk_c += 1;
                    blk.clear();
                }
            }
        }
        mta.fblk_sz = 
            if blk.is_empty() { blk.capacity() } 
            else { blk.len() };

        // Compress final block
        tp.compress_block(blk.clone(), mta.blk_c, blk.len());
        mta.blk_c += 1;

        // Output blocks
        let mut blks_wrtn: u64 = 0;
        while blks_wrtn != mta.blk_c {
            blks_wrtn += tp.bq.lock().unwrap().try_write_block_enc(&mut mta, &mut self.archive);
        }
        self.archive.flush_buffer();

        self.write_metadata(&mut mta);
    }

    /// Write footer containing file paths and lengths.
    fn write_metadata(&mut self, mta: &mut Metadata) {
        // Get index to footer
        mta.f_ptr = self.archive.stream_position().unwrap();

        // Output number of files
        self.archive.write_u64(mta.files.len() as u64);

        for file in mta.files.iter() {
            // Get path as byte slice, truncated if longer than 255 bytes
            let path = &file.0.as_bytes()[..min(file.0.len(), 255)];

            // Output length of file path (for parsing)
            self.archive.write_byte(path.len() as u8);

            // Output path
            for byte in path.iter() {
                self.archive.write_byte(*byte);
            }

            // Output file length
            self.archive.write_u64(file.1);
        }

        // Write compressed block sizes
        for blk_sz in mta.enc_blk_szs.iter() {
            self.archive.write_u64(*blk_sz);
        }

        // Return final archive size including footer
        self.prg.print_archive_stats(self.archive.stream_position().unwrap());

        self.archive.rewind().unwrap();
        self.archive.write_u64(mta.mem);     
        self.archive.write_u64(mta.mgcs);
        self.archive.write_u64(mta.blk_sz as u64);
        self.archive.write_u64(mta.fblk_sz as u64);
        self.archive.write_u64(mta.blk_c);
        self.archive.write_u64(mta.f_ptr);
    }
}
