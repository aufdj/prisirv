use std::{
    fs::{File, create_dir, OpenOptions},
    process::exit,
    path::Path,
    io::{
        Read, Write, BufReader, BufWriter, 
        BufRead, ErrorKind
    },
};

// Indicates an empty or non-empty buffer. 
#[derive(PartialEq, Eq)]
pub enum BufferState {
    NotEmpty,
    Empty,
}

/// A trait for handling buffered reading.
pub trait BufferedRead {
    fn read_byte(&mut self) -> u8;
    fn read_u32(&mut self) -> u32;
    fn read_u64(&mut self) -> u64;
    fn fill_buffer(&mut self) -> BufferState;
}
impl BufferedRead for BufReader<File> {
    /// Read one byte from an input file.
    fn read_byte(&mut self) -> u8 {
        let mut byte = [0u8; 1];
        match self.read(&mut byte) {
            Ok(_)  => {},
            Err(e) => {
                println!("Function read_byte failed.");
                println!("Error: {}", e);
            },
        };
        if self.buffer().is_empty() {
            self.consume(self.capacity());
            match self.fill_buf() {
                Ok(_)  => {},
                Err(e) => {
                    println!("Function read_byte failed.");
                    println!("Error: {}", e);
                },
            }
        }
        u8::from_le_bytes(byte)
    }
    fn read_u32(&mut self) -> u32 {
        let mut bytes = [0u8; 4];
        let len = match self.read(&mut bytes) {
            Ok(len)  => { len },
            Err(e) => {
                println!("Function read_u64 failed.");
                println!("Error: {}", e);
                0
            },
        };
        if self.buffer().is_empty() {
            self.consume(self.capacity());
            match self.fill_buf() {
                Ok(_)  => {},
                Err(e) => {
                    println!("Function read_u64 failed.");
                    println!("Error: {}", e);
                },
            }
            if len < 4 {
                self.read_exact(&mut bytes[len..]).unwrap();
            }
        }
        u32::from_le_bytes(bytes)
    }
    /// Read 8 bytes from an input file, taking care to handle reading 
    /// across buffer boundaries.
    fn read_u64(&mut self) -> u64 {
        let mut bytes = [0u8; 8];
        let len = match self.read(&mut bytes) {
            Ok(len)  => { len },
            Err(e) => {
                println!("Function read_u64 failed.");
                println!("Error: {}", e);
                0
            },
        };
        if self.buffer().is_empty() {
            self.consume(self.capacity());
            match self.fill_buf() {
                Ok(_)  => {},
                Err(e) => {
                    println!("Function read_u64 failed.");
                    println!("Error: {}", e);
                },
            }
            if len < 8 {
                self.read_exact(&mut bytes[len..]).unwrap();
            }
        }
        u64::from_le_bytes(bytes)
    }

    /// Fills the input buffer, returning the buffer's state.
    fn fill_buffer(&mut self) -> BufferState {
        self.consume(self.capacity());
        match self.fill_buf() {
            Ok(_)  => {},
            Err(e) => {
                println!("Function fill_buffer failed.");
                println!("Error: {}", e);
            },
        }
        if self.buffer().is_empty() {
            return BufferState::Empty;
        }
        BufferState::NotEmpty
    }
}

/// A trait for handling buffered writing.
pub trait BufferedWrite {
    fn write_byte(&mut self, output: u8);
    fn write_u16(&mut self, output: u16);
    fn write_u32(&mut self, output: u32);
    fn write_u64(&mut self, output: u64);
    fn flush_buffer(&mut self);
}
impl BufferedWrite for BufWriter<File> {
    /// Write one byte to an output file.
    fn write_byte(&mut self, output: u8) {
        match self.write(&[output]) {
            Ok(_)  => {},
            Err(e) => {
                println!("Function write_byte failed.");
                println!("Error: {}", e);
            },
        }
        if self.buffer().len() >= self.capacity() {
            match self.flush() {
                Ok(_)  => {},
                Err(e) => {
                    println!("Function write_byte failed.");
                    println!("Error: {}", e);
                },
            }
        }
    }
    fn write_u16(&mut self, output: u16) {
        match self.write(&output.to_le_bytes()[..]) {
            Ok(_)  => {},
            Err(e) => {
                println!("Function write_u16 failed.");
                println!("Error: {}", e);
            },
        }
        if self.buffer().len() >= self.capacity() {
            match self.flush() {
                Ok(_)  => {},
                Err(e) => {
                    println!("Function write_u16 failed.");
                    println!("Error: {}", e);
                },
            }
        }
    }
    fn write_u32(&mut self, output: u32) {
        match self.write(&output.to_le_bytes()[..]) {
            Ok(_)  => {},
            Err(e) => {
                println!("Function write_u32 failed.");
                println!("Error: {}", e);
            },
        }
        if self.buffer().len() >= self.capacity() {
            match self.flush() {
                Ok(_)  => {},
                Err(e) => {
                    println!("Function write_u32 failed.");
                    println!("Error: {}", e);
                },
            }
        }
    }
    /// Write 8 bytes to an output file.
    fn write_u64(&mut self, output: u64) {
        match self.write(&output.to_le_bytes()[..]) {
            Ok(_)  => {},
            Err(e) => {
                println!("Function write_u64 failed.");
                println!("Error: {}", e);
            },
        }
        if self.buffer().len() >= self.capacity() {
            match self.flush() {
                Ok(_)  => {},
                Err(e) => {
                    println!("Function write_u64 failed.");
                    println!("Error: {}", e);
                },
            } 
        }
    }

    /// Flush buffer to file.
    fn flush_buffer(&mut self) {
        match self.flush() {
            Ok(_)  => {},
            Err(e) => {
                println!("Function flush_buffer failed.");
                println!("Error: {}", e);
            },
        }    
    }
}


/// Takes a file path and returns an input file wrapped in a BufReader.
pub fn new_input_file(capacity: usize, file_path: &Path) -> BufReader<File> {
    BufReader::with_capacity(
        capacity, 
        match File::open(file_path) {
            Ok(file) => file,
            Err(e) => match e.kind() {
                ErrorKind::NotFound => {
                    println!("Couldn't open file {}: Not Found", file_path.display());
                    exit(0); 
                }
                ErrorKind::PermissionDenied => {
                    println!("Couldn't open file {}: Permission Denied", file_path.display());
                    exit(0);
                }
                _ => {
                    println!("Couldn't open file {}", file_path.display());
                    exit(0);
                }
            }
        }
    )
}

/// Takes a file path and returns an output file wrapped in a BufWriter.
pub fn new_output_file_no_trunc(capacity: usize, file_path: &Path) -> BufWriter<File> {
    BufWriter::with_capacity(
        capacity, 
        OpenOptions::new().write(true).open(file_path).unwrap(),
    )
}


/// Takes a file path and returns an output file wrapped in a BufWriter.
pub fn new_output_file(capacity: usize, file_path: &Path) -> BufWriter<File> {
    BufWriter::with_capacity(
        capacity, 
        match File::create(file_path) {
            Ok(file) => file,
            Err(e) => match e.kind() {
                ErrorKind::NotFound => {
                    println!("Couldn't open file {}: Not Found", file_path.display());
                    exit(0); 
                }
                ErrorKind::PermissionDenied => {
                    println!("Couldn't open file {}: Permission Denied", file_path.display());
                    exit(0);
                }
                _ => {
                    println!("Couldn't open file {}", file_path.display());
                    exit(0);
                }  
            }
        }
    )
}

/// Create a new directory.
pub fn new_dir(path: &str) {
    let path = Path::new(path);
    match create_dir(path) {
        Ok(_) => {},
        Err(err) => {
            match err.kind() {
                ErrorKind::AlreadyExists => {
                    println!("Directory {} already exists.", path.display());
                    exit(0);
                },
                ErrorKind::InvalidInput  => {
                    println!("Invalid directory name.");
                },
                _ => 
                    println!("Error: {}", err),
            }
        }
    }
}

/// Create a new directory only if the clobber flag is set or the existing 
/// directory is empty.
pub fn new_dir_checked(dir_out: &str, clbr: bool) {
    let path = Path::new(dir_out);
    // Create output directory if it doesn't exist.
    if !path.exists() {
        new_dir(dir_out);
    }
    // If directory exists but is empty, ignore clobber option.
    else if path.read_dir().unwrap().count() == 0 {}
    // If directory exists and is not empty, abort if user disallowed clobbering (default)
    else if !clbr {
        println!("Directory {} already exists.", dir_out);
        println!("To overwrite existing directories, enable flag '-clb'.");
        exit(0);
    }
    // If directory exists and is not empty and user allowed clobbering, continue as normal.
    else {}
}

/// Create a new file only if the clobber flag is set or the existing file 
/// is empty.
pub fn new_output_file_checked(dir_out: &str, clbr: bool) -> BufWriter<File> {
    let path = Path::new(&dir_out);
    // If file doesn't exist or is empty, ignore clobber option.
    if !path.exists() || file_len(path) == 0 {}
    // If file exists or is not empty, abort if user disallowed clobbering (default)
    else if !clbr {
        println!("Archive {} already exists.", dir_out);
        println!("To overwrite existing archives, enable flag '-clb'.");
        exit(0);
    }
    // If file exists and is not empty and user allowed clobbering, continue as normal.
    else {}
    new_output_file(4096, path)
}

/// Return the length of a file.
pub fn file_len(path: &Path) -> u64 {
    path.metadata().unwrap().len()
}
