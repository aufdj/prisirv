use std::{
    fs::{File, create_dir, OpenOptions},
    path::Path,
    io::{
        self, Read, Write, BufReader, 
        BufWriter, BufRead, ErrorKind
    },
};

use crate::filedata::FileData;

// Indicates an empty or non-empty buffer. 
#[derive(PartialEq, Eq)]
pub enum BufferState {
    NotEmpty,
    Empty,
}

/// A trait for handling buffered reading.
pub trait BufferedRead {
    fn read_byte(&mut self) -> u8;
    fn read_u16(&mut self) -> u16;
    fn read_u32(&mut self) -> u32;
    fn read_u64(&mut self) -> u64;
    fn fill_buffer(&mut self) -> BufferState;
}
impl BufferedRead for BufReader<File> {
    /// Read one byte from an input file.
    fn read_byte(&mut self) -> u8 {
        let mut byte = [0u8; 1];

        if self.read(&mut byte).is_ok() {
            if self.buffer().is_empty() {
                self.consume(self.capacity());

                if let Err(e) = self.fill_buf() {
                    println!("Function read_byte failed.");
                    println!("Error: {}", e);
                }
            }
        }
        else {
            println!("Function read_byte failed.");
        }
        u8::from_le_bytes(byte)
    }
    fn read_u16(&mut self) -> u16 {
        let mut bytes = [0u8; 2];

        if let Ok(len) = self.read(&mut bytes) {
            if self.buffer().is_empty() {
                self.consume(self.capacity());

                if let Err(e) = self.fill_buf() {
                    println!("Function read_u16 failed.");
                    println!("Error: {}", e);
                }
                if len < 2 {
                    self.read_exact(&mut bytes[len..]).unwrap();
                }
            }
        }
        else {
            println!("Function read_u16 failed.");
        }
        u16::from_le_bytes(bytes)
    }
    fn read_u32(&mut self) -> u32 {
        let mut bytes = [0u8; 4];

        if let Ok(len) = self.read(&mut bytes) {
            if self.buffer().is_empty() {
                self.consume(self.capacity());

                if let Err(e) = self.fill_buf() {
                    println!("Function read_u32 failed.");
                    println!("Error: {}", e);
                }
                if len < 4 {
                    self.read_exact(&mut bytes[len..]).unwrap();
                }
            }
        }
        else {
            println!("Function read_u32 failed.");
        }
        u32::from_le_bytes(bytes)
    }
    /// Read 8 bytes from an input file, taking care 
    /// to handle reading across buffer boundaries.
    fn read_u64(&mut self) -> u64 {
        let mut bytes = [0u8; 8];

        if let Ok(len) = self.read(&mut bytes) {
            if self.buffer().is_empty() {
                self.consume(self.capacity());
                
                if let Err(e) = self.fill_buf() {
                    println!("Function read_u64 failed.");
                    println!("Error: {}", e);
                }
                if len < 8 {
                    self.read_exact(&mut bytes[len..]).unwrap();
                }
            }
        }
        else {
            println!("Function read_u64 failed.");
        }
        u64::from_le_bytes(bytes)
    }

    /// Fills the input buffer, returning the buffer's state.
    fn fill_buffer(&mut self) -> BufferState {
        self.consume(self.capacity());
        if let Err(e) = self.fill_buf() {
            println!("Function fill_buffer failed.");
            println!("Error: {}", e);
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
        if let Err(e) = self.write(&[output]) {
            println!("Function write_byte failed.");
            println!("Error: {}", e);
        }
        
        if self.buffer().len() >= self.capacity() {
            if let Err(e) = self.flush() {
                println!("Function write_byte failed.");
                println!("Error: {}", e);
            }
        }
    }
    fn write_u16(&mut self, output: u16) {
        if let Err(e) = self.write(&output.to_le_bytes()[..]) {
            println!("Function write_u16 failed.");
            println!("Error: {}", e);
        }
        
        if self.buffer().len() >= self.capacity() {
            if let Err(e) = self.flush() {
                println!("Function write_u16 failed.");
                println!("Error: {}", e);
            }
        }
    }
    fn write_u32(&mut self, output: u32) {
        if let Err(e) = self.write(&output.to_le_bytes()[..]) {
            println!("Function write_32 failed.");
            println!("Error: {}", e);
        }
        
        if self.buffer().len() >= self.capacity() {
            if let Err(e) = self.flush() {
                println!("Function write_32 failed.");
                println!("Error: {}", e);
            }
        }
    }
    /// Write 8 bytes to an output file.
    fn write_u64(&mut self, output: u64) {
        if let Err(e) = self.write(&output.to_le_bytes()[..]) {
            println!("Function write_u64 failed.");
            println!("Error: {}", e);
        }
        
        if self.buffer().len() >= self.capacity() {
            if let Err(e) = self.flush() {
                println!("Function write_u64 failed.");
                println!("Error: {}", e);
            }
        }
    }

    /// Flush buffer to file.
    fn flush_buffer(&mut self) {
        if let Err(e) = self.flush() {
            println!("Function flush_buffer failed.");
            println!("Error: {}", e);
        }    
    }
}


/// Takes a file path and returns an input file wrapped in a BufReader.
pub fn new_input_file(path: &Path) -> io::Result<BufReader<File>> {
    match File::open(path) {
        Ok(file) => Ok(BufReader::with_capacity(4096, file)),
        Err(err) => Err(err),
    }
}

pub fn new_output_file(file: &FileData, clobber: bool) -> io::Result<BufWriter<File>> {
    if !file.path.exists() || file.len == 0 || clobber {
        match OpenOptions::new()
            .write(true)
            .truncate(file.seg_beg == 0)
            .create(true)
            .open(&file.path) {
                Ok(file) => {
                    Ok(BufWriter::with_capacity(4096, file))
                },
                Err(err) => {
                    Err(err)
                }
        }
    }
    else {
        Err(io::Error::from(ErrorKind::AlreadyExists))
    }
    
}

/// Create a new directory if it doesn't already exist.
pub fn new_dir(out: &FileData) -> io::Result<()> {
    if !out.path.exists() {
        if let Err(err) = create_dir(&out.path) {
            return Err(err);
        }
    }
    Ok(())
}


