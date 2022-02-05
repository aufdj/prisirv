use std::{
    path::{Path, PathBuf},
    io::{Seek, SeekFrom},
    time::Instant,
    cmp::min,
    ffi::OsString,
};

use crate::{
    file_name_no_ext, file_path_no_ext,
    file_name_ext, file_len, new_dir_checked,
    metadata::Metadata,
    encoder::Encoder,
    decoder::Decoder,
    buffered_io::{
        BufferedRead, BufferedWrite, BufferState,
        new_input_file, new_output_file, new_dir
    }
};


// Non-solid archiving --------------------------------------------------------------------------------------------------------------------
pub struct Archiver {
    dup:    u32, // For handling duplicate file names
    quiet:  bool,
    mem:    usize,
}
impl Archiver {
    pub fn new(quiet: bool, mem: usize) -> Archiver {
        Archiver {
            dup: 1,
            quiet,
            mem,
        }
    }
    pub fn compress_file(&mut self, file_in_path: &Path, dir_out: &str) -> u64 {
        let mut mta: Metadata = Metadata::new();

        // Create output file path from current output directory
        // and input file name without extension
        // i.e. foo/bar.txt -> foo/bar.pri
        let mut file_out_path = 
            PathBuf::from(
                &format!("{}\\{}.pri",  
                dir_out, file_name_no_ext(file_in_path))
            ); 

        // Modify file path if it already exists due to extension change
        // i.e foo/bar.txt -> foo/bar.pri
        //     foo/bar.bin -> foo/bar.pri -> foo/bar(1).pri
        while file_out_path.exists() {
            file_out_path = 
            if self.dup == 1 {
                PathBuf::from(
                    &format!("{}({}).pri", 
                    file_path_no_ext(&file_out_path), self.dup)
                )
            }
            else {
                let file_path = file_path_no_ext(&file_out_path);
                PathBuf::from(
                    &format!("{}({}).pri", 
                    &file_path[..file_path.len()-3], self.dup)
                )
            };
            self.dup += 1;
        }
        self.dup = 1;

        // Create input file with buffer = block size
        let mut file_in = new_input_file(mta.bl_sz, file_in_path);
        let mut enc = Encoder::new(new_output_file(4096, &file_out_path), self.mem);

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
        enc.write_header(&mta, false);
        file_len(&file_out_path)
    }
    
    pub fn compress_dir(&mut self, dir_in: &Path, dir_out: &mut String) {
        // Create new nested directory from current output 
        // directory and input directory name 
        let mut dir_out = 
            format!("{}\\{}", 
            dir_out, file_name_ext(dir_in));
        new_dir(&dir_out);

        // Sort files and directories
        let (files, dirs): (Vec<PathBuf>, Vec<PathBuf>) = 
            dir_in.read_dir().unwrap()
            .map(|d| d.unwrap().path())
            .partition(|f| f.is_file());

        // Compress files first, then directories
        for file_in in files.iter() {
            let time = Instant::now();
            if !self.quiet { println!("Compressing {}", file_in.display()); }
            let file_in_size  = file_len(file_in); 
            let file_out_size = self.compress_file(file_in, &dir_out);
            if !self.quiet { println!("{} bytes -> {} bytes in {:.2?}\n", 
                file_in_size, file_out_size, time.elapsed()); }
        }
        for dir_in in dirs.iter() {
            self.compress_dir(dir_in, &mut dir_out); 
        }
    }  
}

pub struct Extractor {
    quiet: bool,
}
impl Extractor {
    pub fn new(quiet: bool) -> Extractor {
        Extractor {
            quiet,
        }
    }
    pub fn decompress_file(&self, file_in_path: &Path, dir_out: &str) -> u64 {
        let mut dec = Decoder::new(new_input_file(4096, file_in_path));
        let mta: Metadata = dec.read_header();

        // Verify magic number
        match mta.mgc {
            0x76_7269_7369_7270 => {},
            0x53767_2697_3697_270 => {
                println!();
                println!("Expected non-solid archive, found solid archive.");
                std::process::exit(0);
            },
            _ => {
                println!("Not a prisirv archive.");
                std::process::exit(0);
            },
        }

        // Create output file path from current output directory,
        // input file name without extension, and file's original
        // extension (stored in header)
        // i.e. foo/bar.pri -> foo/bar.txt
        let file_out_path = 
            // Handle no extension
            if mta.get_ext().is_empty() {
                PathBuf::from(
                    &format!("{}\\{}",
                    dir_out,
                    file_name_no_ext(file_in_path))
                )
            }
            else {
                PathBuf::from(
                    &format!("{}\\{}.{}",
                    dir_out,
                    file_name_no_ext(file_in_path),
                    mta.get_ext())
                )
            };
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
        // Create new nested directory from current output 
        // directory and input directory name; if current output
        // directory is root, replace rather than nest
        let mut dir_out = 
            if root { dir_out.to_string() }
            else { 
                format!("{}\\{}", 
                dir_out, file_name_ext(dir_in)) 
            };
        if !Path::new(&dir_out).is_dir() {
            new_dir(&dir_out);
        }

        // Sort files and directories
        let (files, dirs): (Vec<PathBuf>, Vec<PathBuf>) =
            dir_in.read_dir().unwrap()
            .map(|d| d.unwrap().path())
            .partition(|f| f.is_file());

        // Decompress files first, then directories
        for file_in in files.iter() {
            let time = Instant::now();
            if !self.quiet { println!("Decompressing {}", file_in.display()); }
            let file_in_size  = file_len(file_in);
            let file_out_size = self.decompress_file(file_in, &dir_out);
            if !self.quiet { println!("{} bytes -> {} bytes in {:.2?}\n",
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
    enc:    Encoder,
    mta:    Metadata,
    quiet:  bool
}
impl SolidArchiver {
    pub fn new(enc: Encoder, mta: Metadata, quiet: bool) -> SolidArchiver {
        SolidArchiver {
            enc, mta, quiet,
        }
    }
    pub fn create_archive(&mut self) {
        for curr_file in 0..self.mta.files.len() {
            if !self.quiet { println!("Compressing {}", self.mta.files[curr_file].0); }
            let archive_size = self.compress_file_solid(curr_file);
            if !self.quiet { println!("Total archive size: {}\n", archive_size); }
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
    // For more info on metadata structure, see metadata.rs
    pub fn write_metadata(&mut self) {
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

        // Go back to beginning of file and write header
        self.enc.file_out.rewind().unwrap();
        self.enc.write_header(&self.mta, true);
    }
}

pub struct SolidExtractor {
    dec:    Decoder,
    mta:    Metadata,
    quiet:  bool,
    clbr:   bool,
}
impl SolidExtractor {
    pub fn new(dec: Decoder, mta: Metadata, quiet: bool, clbr: bool) -> SolidExtractor {
        SolidExtractor {
            dec, mta, quiet, clbr,
        }
    }
    pub fn extract_archive(&mut self, dir_out: &str) {
        new_dir_checked(dir_out, self.clbr);
        for curr_file in 0..self.mta.files.len() {
            if !self.quiet { println!("Decompressing {}", self.mta.files[curr_file].0); }
            self.decompress_file_solid(dir_out, curr_file);
        }
    }
    pub fn decompress_file_solid(&mut self, dir_out: &str, curr_file: usize) {
        // Create new output file from output directory and the file
        // currently being decompressed. The code below currently does
        // not reconstruct the original directory structure, all files
        // are just placed in the same directory.
        let file_out_name =
            format!("{}\\{}",
                dir_out,
                file_name_ext(
                    Path::new(&self.mta.files[curr_file].0)
                ),
            );
        let mut file_out = new_output_file(4096, Path::new(&file_out_name));

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
    // For more info on metadata structure, see metadata.rs
    pub fn read_metadata(&mut self) {
        self.mta = self.dec.read_header();

        // Verify magic number
        match self.mta.mgc {
            0x76_7269_7369_7270 => {
                println!();
                println!("Expected solid archive, found non-solid archive.");
                std::process::exit(0);
            },
            0x53767_2697_3697_270 => {},
            _ => {
                println!("Not a prisirv archive.");
                std::process::exit(0);
            },
        }

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
        self.dec.file_in.seek(SeekFrom::Start(56)).unwrap();

        self.dec.init_x();
    }
}
// ----------------------------------------------------------------------------------------------------------------------------------------

