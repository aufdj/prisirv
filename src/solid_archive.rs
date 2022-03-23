use std::{
    path::{Path, PathBuf},
    io::{Write, Seek, BufWriter},
    fs::File,
};

use crate::{
    Mode,
    sort::sort_files,
    metadata::Metadata,
    threads::ThreadPool,
    progress::Progress,
    config::Config,
    buffered_io::{
        BufferedRead, BufferedWrite, file_len,
        new_input_file, new_output_file_checked,
    },
};


/// A solid archiver creates solid archives, or an archive containing files 
/// compressed as one stream. Solid archives take advantage of redundancy 
/// across files and therefore achieve better compression ratios than non-
/// solid archives, but don't allow for extracting individual files.
pub struct SolidArchiver {
    pub archive:  BufWriter<File>,
    cfg:          Config,
    prg:          Progress,
    mta:          Metadata,
}
impl SolidArchiver {
    /// Create a new SolidArchiver.
    pub fn new(cfg: Config) -> SolidArchiver {
        let mut mta = Metadata::new_with_cfg(&cfg);
        
        // Collect and sort files.
        collect_files(&cfg.inputs, &mut mta);
        mta.files.sort_by(|f1, f2| sort_files(&f1.0, &f2.0, cfg.sort));

        let mut prg = Progress::new(&cfg, Mode::Compress);
        prg.get_archive_size_enc(&mta.files);

        let mut archive = new_output_file_checked(&cfg.dir_out, cfg.clbr);
        for _ in 0..6 { archive.write_u64(0); }

        SolidArchiver { 
            archive, cfg, prg, mta
        }
    }

    /// Parse files into blocks and compress blocks.
    pub fn create_archive(&mut self) {
        let mut tp = ThreadPool::new(self.cfg.threads, self.cfg.mem, self.prg);
        let mut blk = Vec::with_capacity(self.cfg.blk_sz);

        // Read files into blocka and compress
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
            blks_wrtn += tp.bq.lock().unwrap().try_write_block_enc(&mut self.mta, &mut self.archive);
        }
        self.archive.flush_buffer();

        self.write_metadata();
    }

    /// Write footer containing file paths and lengths.
    fn write_metadata(&mut self) {
        // Get index to footer
        self.mta.f_ptr = self.archive.stream_position().unwrap();

        self.archive.write_u64(self.mta.files.len() as u64);

        for file in self.mta.files.iter() {
            // Output null terminated path string.
            let path = file.0.to_str().unwrap().as_bytes();
            self.archive.write_all(path).unwrap();
            self.archive.write_byte(0);

            // Output file length
            self.archive.write_u64(file.1);
        }

        // Write compressed block sizes
        for blk_sz in self.mta.enc_blk_szs.iter() {
            self.archive.write_u64(*blk_sz);
        }

        // Return final archive size including footer
        self.prg.print_archive_stats(self.archive.stream_position().unwrap());

        self.archive.rewind().unwrap();
        self.archive.write_u64(self.mta.mem);     
        self.archive.write_u64(self.mta.mgcs);
        self.archive.write_u64(self.mta.blk_sz as u64);
        self.archive.write_u64(self.mta.fblk_sz as u64);
        self.archive.write_u64(self.mta.blk_c);
        self.archive.write_u64(self.mta.f_ptr);
    }
}

/// Recursively collect all files into a vector for sorting before compression.
fn collect_files(inputs: &[PathBuf], mta: &mut Metadata) {
    // Group files and directories 
    let (files, dirs): (Vec<PathBuf>, Vec<PathBuf>) =
        inputs.iter().cloned()
        .partition(|f| f.is_file());

    // Walk through directories and collect all files
    for file in files.iter() {
        mta.files.push((file.clone(), file_len(file)));
    }
    for dir in dirs.iter() {
        collect(dir, mta);
    }
}
fn collect(dir_in: &Path, mta: &mut Metadata) {
    let (files, dirs): (Vec<PathBuf>, Vec<PathBuf>) =
        dir_in.read_dir().unwrap()
        .map(|d| d.unwrap().path())
        .partition(|f| f.is_file());

    for file in files.iter() {
        mta.files.push((file.clone(), file_len(file)));
    }
    for dir in dirs.iter() {
        collect(dir, mta);
    }
}
