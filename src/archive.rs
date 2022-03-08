use std::{
    path::{Path, PathBuf},
    io::Seek,
    fs::File,
    io::BufWriter,
};

use crate::{
    Mode,
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
        let prg = Progress::new(&cfg, Mode::Compress);
        Archiver {
            cfg, prg, files: Vec::with_capacity(32),
        }
    }

    /// Compress all files.
    pub fn create_archive(&mut self) {
        new_dir_checked(&self.cfg.dir_out, self.cfg.clbr);

        let (files, dirs): (Vec<PathBuf>, Vec<PathBuf>) = 
            self.cfg.inputs.clone().into_iter().partition(|f| f.is_file());

        let mut dir_out = self.cfg.dir_out.clone();

        for file_in in files.iter() {
            if !self.cfg.quiet { println!("Compressing {}", file_in.display()); }
            self.compress_file(file_in, &dir_out);
        }
        for dir_in in dirs.iter() {
            self.compress_dir(dir_in, &mut dir_out);      
        }
    }

    /// Compress a single file.
    pub fn compress_file(&mut self, file_in_path: &Path, dir_out: &str) {
        self.prg.get_input_size_enc(file_in_path);

        let mut mta: Metadata = Metadata::new();
        mta.blk_sz = self.cfg.blk_sz;
        mta.mem = self.cfg.mem;

        let file_out_path = fmt_file_out_ns_archive(dir_out, file_in_path, self.cfg.clbr, &self.files);
        if self.cfg.clbr { self.files.push(file_out_path.clone()); }
        
        // Create input file with buffer = block size
        let mut file_in  = new_input_file(mta.blk_sz, file_in_path);

        // Create output file and write metadata placeholder
        let mut file_out = new_output_file(4096, &file_out_path);
        for _ in 0..7 { file_out.write_u64(0); }

        // Set metadata extension field
        mta.set_ext(file_in_path);
        
        let mut index: u64 = 0;
        let mut blks_wrtn: u64 = 0;
        let mut tp = ThreadPool::new(self.cfg.threads, self.cfg.mem, self.prg);

        while file_in.fill_buffer() == BufferState::NotEmpty {
            mta.fblk_sz = file_in.buffer().len();
            tp.compress_block(file_in.buffer().to_vec(), index, self.cfg.blk_sz);
            index += 1;
            mta.blk_c += 1;
        }
        while blks_wrtn != mta.blk_c {
            blks_wrtn += tp.bq.lock().unwrap().try_write_block_enc(&mut mta, &mut file_out);
        }   

        self.write_footer(&mut file_out, &mut mta);
        self.write_header(&mut file_out, &mta);

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
        for file_in in files.iter() {
            if !self.cfg.quiet { println!("Compressing {}", file_in.display()); } 
            self.compress_file(file_in, &dir_out);
        }
        for dir_in in dirs.iter() {
            self.compress_dir(dir_in, &mut dir_out);
        }
    } 

    /// Rewind to the beginning of the file and write a 56 byte header.
    fn write_header(&mut self, file_out: &mut BufWriter<File>, mta: &Metadata) {
        file_out.rewind().unwrap();
        file_out.write_u64(mta.mem);
        file_out.write_u64(mta.mgc);
        file_out.write_u64(mta.ext);
        file_out.write_u64(mta.fblk_sz as u64);
        file_out.write_u64(mta.blk_sz as u64);
        file_out.write_u64(mta.blk_c);
        file_out.write_u64(mta.f_ptr);
    }

    /// Write compressed block sizes to archive so that compressed blocks 
    /// can be parsed ahead of time during decompression.
    fn write_footer(&mut self, file_out: &mut BufWriter<File>, mta: &mut Metadata) {
        // Get index to end of file metadata
        mta.f_ptr = file_out.stream_position().unwrap();

        for blk_sz in mta.enc_blk_szs.iter() {
            file_out.write_u64(*blk_sz);
        }
    } 
}
