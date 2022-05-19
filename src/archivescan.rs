use crate::{
    block::Block,
    filedata::FileData,
    buffered_io::new_input_file,
    error::ExtractError,
};

use std::io::{Seek, SeekFrom};

/// Functions for reading archive metadata, for the purpose of displaying
/// or modifying an archive.

/// Count number of blocks in an archive.
pub fn block_count(ex_arch: &FileData) -> Result<u32, ExtractError> {
    let mut count = 0u32;
    let mut blk = Block::default();
    let mut archive = new_input_file(&ex_arch.path)?;
    loop {
        
        blk.read_header_from(&mut archive)?;
        if blk.sizeo == 0 {
            break;
        }

        archive.seek(SeekFrom::Current(blk.sizeo as i64))?;

        count += 1;
        blk.next();
    }
    Ok(count)
}

pub fn find_eod(ex_arch: &FileData) -> Result<u64, ExtractError> {
    let mut blk = Block::default();
    let mut archive = new_input_file(&ex_arch.path)?;
    loop {
        let eod = archive.stream_position()?;
        blk.read_header_from(&mut archive)?;
        if blk.sizeo == 0 {
            return Ok(eod);
        }

        archive.seek(SeekFrom::Current(blk.sizeo as i64))?;

        blk.next();
    }
}

/// Return id of the first block that contains 'file', or none if the file 
/// isn't in the archive.
pub fn find_file(file: &FileData, ex_arch: &FileData) -> Result<Option<u32>, ExtractError> {
    let mut blk = Block::default();
    let mut archive = new_input_file(&ex_arch.path)?;
    loop {
        blk.read_header_from(&mut archive)?;
        if blk.sizeo == 0 {
            break;
        }
        if blk.files.iter().any(|f| f.path == file.path) {
            return Ok(Some(blk.id));
        }

        archive.seek(SeekFrom::Current(blk.sizeo as i64))?;

        blk.next();
    }
    Ok(None)
}

/// Print archive information.
pub fn list_archive(ex_arch: &FileData) -> Result<(), ExtractError> {
    let mut blk = Block::default();
    let mut archive = new_input_file(&ex_arch.path)?;
    loop {
        blk.read_header_from(&mut archive)?;
        if blk.sizeo == 0 { 
            break; 
        }

        archive.seek(SeekFrom::Current(blk.sizeo as i64))?;

        println!("{}", blk);
        blk.next();
    }
    std::process::exit(0);
}

//pub fn verify_archive() {
//    let mut blk = Block::default();
//    let mut archive = new_input_file(&ex_arch.path).unwrap();
//    loop {
//        blk.read_header_from(&mut archive);
//        if blk.
//
//        archive.seek(
//            SeekFrom::Current(blk.sizeo as i64)
//        ).unwrap();
//
//        blk.print();
//        blk.next();
//    }
//}