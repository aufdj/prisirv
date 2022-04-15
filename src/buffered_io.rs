use std::{
    fs::{File, create_dir, OpenOptions},
    path::Path,
    io::{
        Read, Write, BufReader, BufWriter, 
        BufRead, ErrorKind
    },
};

use crate::{
    error,
    filedata::FileData,
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
pub fn new_input_file(capacity: usize, path: &Path) -> BufReader<File> {
    BufReader::with_capacity(
        capacity, 
        match File::open(path) {
            Ok(file) => file,
            Err(e) => match e.kind() {
                ErrorKind::NotFound => error::file_not_found(path),
                ErrorKind::PermissionDenied => error::permission_denied(path),
                _ => error::file_general(path),
            }
        }
    )
}

/// Takes a file path and returns an output file wrapped in a BufWriter.
pub fn new_output_file_no_trunc(capacity: usize, path: &Path) -> BufWriter<File> {
    BufWriter::with_capacity(
        capacity, 
        OpenOptions::new().write(true).open(path).unwrap(),
    )
}


/// Takes a file path and returns an output file wrapped in a BufWriter.
pub fn new_output_file(capacity: usize, path: &Path) -> BufWriter<File> {
    BufWriter::with_capacity(
        capacity, 
        match File::create(path) {
            Ok(file) => file,
            Err(e) => match e.kind() {
                ErrorKind::NotFound => error::file_not_found(path),
                ErrorKind::PermissionDenied => error::permission_denied(path),
                _ => error::file_general(path),
            }
        }
    )
}

/// Create a new file only if the clobber flag is set or the existing file 
/// is empty.
pub fn new_output_file_checked(file: &FileData, clbr: bool) -> BufWriter<File> {
    // If file doesn't exist or is empty, ignore clobber flag.
    if !file.path.exists() || file.len == 0 {}
    // If file exists or is not empty, abort if user disallowed clobbering (default)
    else if !clbr { error::file_already_exists(&file.path); }
    // If file exists and is not empty and user allowed clobbering, continue as normal.
    else {}
    new_output_file(4096, &file.path)
}

/// Create a new directory only if the clobber flag is set or the existing 
/// directory is empty.
pub fn new_dir(out: &FileData, clbr: bool) {
    // Create output directory if it doesn't exist.
    if !out.path.exists() {
        match create_dir(&out.path) {
            Ok(_) => {},
            Err(err) => {
                match err.kind() {
                    ErrorKind::InvalidInput => error::invalid_input(&out.path),
                    _ => error::dir_general(&out.path),
                }
            }
        }
    }
    // If directory exists but is empty, ignore clobber option.
    else if out.path.read_dir().unwrap().count() == 0 {}
    // If directory exists and is not empty, abort if user disallowed clobbering (default)
    else if !clbr { error::dir_already_exists(&out.path); }
    // If directory exists and is not empty and user allowed clobbering, continue as normal.
    else {}
}


