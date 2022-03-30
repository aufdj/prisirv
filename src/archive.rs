use std::{
    path::{Path, PathBuf},
    io::{Write, Seek},
    fs::File,
    io::BufWriter,
};

use crate::{
    progress::Progress,
    metadata::Metadata,
    config::Config,
    threads::ThreadPool,
    buffered_io::{
        BufferedRead, BufferedWrite, BufferState, file_len, 
        new_input_file, new_output_file, new_dir_checked,
    },
    formatting::{
        fmt_file_out_ns_archive,
        fmt_nested_dir_ns_archive,
    },
};

/// Size of header in bytes
const PLACEHOLDER: [u8; 56] = [0; 56];

/// An archiver creates non-solid archives. A non-solid archive is an 
/// archive containing independently compressed files. Non-solid archiving 
/// typically results in worse compression ratios than solid archiving, but 
/// allows for extracting individual files.
pub struct Archiver {
    cfg:    Config,
    prg:    Progress,
    files:  Vec<PathBuf>, // Keep list of files already compressed to prevent accidental clobbering
}
impl Archiver {
    /// Create a new Archiver.
    pub fn new(cfg: Config) -> Archiver {
        let prg = Progress::new(&cfg);
        
        Archiver {
            cfg, prg, files: Vec::new(),
        }
    }

    /// Compress all files.
    pub fn create_archive(&mut self) {
        new_dir_checked(&self.cfg.dir_out, self.cfg.clbr);

        let (files, dirs): (Vec<PathBuf>, Vec<PathBuf>) = 
            self.cfg.inputs.clone().into_iter()
            .partition(|f| f.is_file());

        let mut dir_out = self.cfg.dir_out.clone();

        for file in files.iter() {
            self.prg.print_file_name(file);
            self.compress_file(file, &dir_out);
        }
        for dir in dirs.iter() {
            self.compress_dir(dir, &mut dir_out);      
        }
    }

    /// Compress a single file.
    pub fn compress_file(&mut self, file_in_path: &Path, dir_out: &str) {
        self.prg.get_file_size_enc(file_in_path);

        let mut mta: Metadata = Metadata::new_with_cfg(&self.cfg);

        // Create input file with buffer = block size
        let mut file_in = new_input_file(mta.blk_sz, file_in_path);

        // Create output file and write metadata placeholder
        let file_out_path = fmt_file_out_ns_archive(dir_out, file_in_path, self.cfg.clbr, &self.files);
        if self.cfg.clbr { self.files.push(file_out_path.clone()); }
        let mut file_out = new_output_file(4096, &file_out_path);
        file_out.write_all(&PLACEHOLDER).unwrap();

        // Set metadata extension field
        mta.set_ext(file_in_path);
        
        let mut blks_wrtn: u64 = 0;
        let mut tp = ThreadPool::new(self.cfg.threads, self.cfg.mem, self.prg);

        while file_in.fill_buffer() == BufferState::NotEmpty {
            mta.fblk_sz = file_in.buffer().len();
            tp.compress_block(file_in.buffer().to_vec(), mta.blk_c);
            mta.blk_c += 1;
        }

        while blks_wrtn != mta.blk_c {
            blks_wrtn += tp.bq.lock().unwrap().try_write_block_enc(&mut mta, &mut file_out);
        }   

        self.write_metadata(&mut file_out, &mut mta);

        self.prg.print_file_stats(file_len(&file_out_path));
    }
    
    /// Compress all files in a directory.
    pub fn compress_dir(&mut self, dir_in: &Path, dir_out: &mut String) {
        let mut dir_out = fmt_nested_dir_ns_archive(dir_out, dir_in);
        new_dir_checked(&dir_out, self.cfg.clbr);

        // Sort files and directories
        let (files, dirs): (Vec<PathBuf>, Vec<PathBuf>) = 
            dir_in.read_dir().unwrap()
            .map(|d| d.unwrap().path())
            .partition(|f| f.is_file());

        // Compress files first, then directories
        for file in files.iter() {
            self.prg.print_file_name(file);
            self.compress_file(file, &dir_out);
        }
        for dir in dirs.iter() {
            self.compress_dir(dir, &mut dir_out);
        }
    } 

    /// Rewind to the beginning of the file and write a 56 byte header.
    fn write_metadata(&mut self, file_out: &mut BufWriter<File>, mta: &mut Metadata) {
        // Get index to end of file metadata
        mta.f_ptr = file_out.stream_position().unwrap();

        for blk_sz in mta.enc_blk_szs.iter() {
            file_out.write_u64(*blk_sz);
        }

        file_out.rewind().unwrap();
        file_out.write_u64(mta.mem);
        file_out.write_u64(mta.mgc);
        file_out.write_u64(mta.ext);
        file_out.write_u64(mta.fblk_sz as u64);
        file_out.write_u64(mta.blk_sz as u64);
        file_out.write_u64(mta.blk_c);
        file_out.write_u64(mta.f_ptr);
    }
}
