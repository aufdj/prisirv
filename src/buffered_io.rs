use std::{
    fs::{File, create_dir},
    path::Path,
    io::{
    Read, Write, BufReader, BufWriter, 
    BufRead, ErrorKind
    },
};

// Convenience functions for buffered I/O ---------------------------------------------------------- Convenience functions for buffered I/O
#[derive(PartialEq, Eq)]
pub enum BufferState {
    NotEmpty,
    Empty,
}

pub trait BufferedRead {
    fn read_byte(&mut self) -> u8;
    fn read_usize(&mut self) -> usize;
    fn fill_buffer(&mut self) -> BufferState;
}
impl BufferedRead for BufReader<File> {
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
    fn read_usize(&mut self) -> usize {
        let mut bytes = [0u8; 8];
        match self.read(&mut bytes) {
            Ok(_)  => {},
            Err(e) => {
                println!("Function read_usize failed.");
                println!("Error: {}", e);
            },
        };
        if self.buffer().is_empty() {
            self.consume(self.capacity());
            match self.fill_buf() {
                Ok(_)  => {},
                Err(e) => {
                    println!("Function read_usize failed.");
                    println!("Error: {}", e);
                },
            }
        }
        usize::from_le_bytes(bytes)
    }
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
pub trait BufferedWrite {
    fn write_byte(&mut self, output: u8);
    fn write_usize(&mut self, output: usize);
    fn flush_buffer(&mut self);
}
impl BufferedWrite for BufWriter<File> {
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
    fn write_usize(&mut self, output: usize) {
        match self.write(&output.to_le_bytes()[..]) {
            Ok(_)  => {},
            Err(e) => {
                println!("Function write_usize failed.");
                println!("Error: {}", e);
            },
        }
        if self.buffer().len() >= self.capacity() {
            match self.flush() {
                Ok(_)  => {},
                Err(e) => {
                    println!("Function write_usize failed.");
                    println!("Error: {}", e);
                },
            } 
        }
    }
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
pub fn new_input_file(capacity: usize, file_name: &Path) -> BufReader<File> {
    BufReader::with_capacity(
        capacity, File::open(file_name).unwrap()
    )
}
pub fn new_output_file(capacity: usize, file_name: &Path) -> BufWriter<File> {
    BufWriter::with_capacity(
        capacity, File::create(file_name).unwrap()
    )
}
pub fn new_dir(path: &str) {
    let path = Path::new(path);
    match create_dir(path) {
        Ok(_) => {},
        Err(err) => {
            match err.kind() {
                ErrorKind::AlreadyExists => {
                    println!("Directory {} already exists.", path.display());
                    std::process::exit(1);
                },
                ErrorKind::InvalidInput  => {
                    println!("Invalid directory name.");
                },
                _ => 
                    println!("Error"),
            }
        }
    }
}
// ----------------------------------------------------------------------------------------------------------------------------------------