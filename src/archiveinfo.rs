use crate::{
    block::Block,
    filedata::FileData,
    buffered_io::new_input_file,
    error::ExtractError,
};

use std::{
    io::{Seek, SeekFrom},
    fmt,
};

pub struct ArchiveInfo {
    eod: u64,
    blks: Vec<Block>,
}
impl ArchiveInfo {
    pub fn new(ex_arch: &FileData) -> Result<ArchiveInfo, ExtractError> {
        let mut info = ArchiveInfo {
            eod:  0,
            blks: Vec::new(),
        };
        let mut blk = Block::default();
        let mut archive = new_input_file(&ex_arch.path)?;
        
        loop {
            info.eod = archive.stream_position()?;
            blk.read_header_from(&mut archive)?;
            if blk.sizeo == 0 {
                break;
            }
            info.blks.push(blk.clone());

            archive.seek(SeekFrom::Current(blk.sizeo as i64))?;

            blk.next();
        }
        Ok(info)
    }
    pub fn block_count(&self) -> u32 {
        self.blks.len() as u32
    }
    pub fn end_of_data(&self) -> u64 {
        self.eod
    }
}
impl fmt::Display for ArchiveInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for blk in self.blks.iter() {
            write!(f, "{blk}")?;
        }
        write!(f, "")
    }
}

// pub fn archive_info(ex_arch: &FileData) -> Result<ArchiveInfo, ExtractError> {
//     let mut info = ArchiveInfo::new();
//     let mut blk = Block::default();
//     let mut archive = new_input_file(&ex_arch.path)?;
//     loop {
//         info.eod = archive.stream_position()?;
//         blk.read_header_from(&mut archive)?;
//         if blk.sizeo == 0 {
//             break;
//         }
//         info.blks.push(blk.clone());

//         archive.seek(SeekFrom::Current(blk.sizeo as i64))?;

//         blk.next();
//     }
//     Ok(info)
// }

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