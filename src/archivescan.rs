use crate::{
    block::Block,
    filedata::FileData,
    buffered_io::new_input_file,
};

use std::io::{Seek, SeekFrom};

/// Functions for reading archive metadata, for the purpose of displaying
/// or modifying an archive.

/// Count number of blocks in an archive.
pub fn block_count(ex_arch: &FileData) -> usize {
    let mut count = 0;
    let mut blk = Block::default();
    let mut archive = new_input_file(&ex_arch.path).unwrap();
    loop {
        blk.read_header_from(&mut archive);
        if blk.sizeo == 0 { 
            return count; 
        }

        archive.seek(
            SeekFrom::Current(blk.sizeo as i64)
        ).unwrap();

        count += 1;
        blk.next();
    }
}

/// Return id of the first block that contains 'file', or none if the file 
/// isn't in the archive.
pub fn find_file(file: &FileData, ex_arch: &FileData) -> Option<u32> {
    let mut blk = Block::default();
    let mut archive = new_input_file(&ex_arch.path).unwrap();
    loop {
        blk.read_header_from(&mut archive);
        if blk.sizeo == 0 {
            return None;
        }
        if blk.files.iter().any(|f| f.path == file.path) {
            return Some(blk.id);
        }

        archive.seek(
            SeekFrom::Current(blk.sizeo as i64)
        ).unwrap();

        blk.next();
    }
}

/// Print archive information.
pub fn list_archive(ex_arch: &FileData) -> ! {
    let mut blk = Block::default();
    let mut archive = new_input_file(&ex_arch.path).unwrap();
    loop {
        blk.read_header_from(&mut archive);
        if blk.sizeo == 0 { 
            break; 
        }

        archive.seek(
            SeekFrom::Current(blk.sizeo as i64)
        ).unwrap();

        blk.print();
        blk.next();
    }
    std::process::exit(0);
}