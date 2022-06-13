use std::{
    io::{Seek, SeekFrom},
    fmt,
};

use crate::{
    block::Block,
    filedata::FileData,
    buffered_io::new_input_file,
    error::ArchiveError,
    constant::Version,
};

#[derive(Default, Clone)]
pub struct ArchiveInfo {
    eod:      u64,
    blks:     Vec<Block>,
    pub ver:  Version,
    next_id:  u32,
}
impl ArchiveInfo {
    pub fn new(ex_arch: &FileData) -> Result<ArchiveInfo, ArchiveError> {
        let mut info = ArchiveInfo::default();
        let mut archive = new_input_file(&ex_arch.path)?;
        let mut blk = Block::default();
        
        loop {
            info.eod = archive.stream_position()?;
            blk.read_header_from(&mut archive)?;
            if blk.sizeo == 0 {
                break;
            }
            info.ver = blk.ver;
            info.blks.push(blk.clone());

            archive.seek(SeekFrom::Current(blk.sizeo as i64))?;

            blk.next();
        }
        info.next_id = info.blks.len() as u32;
        Ok(info)
    }
    pub fn end_of_data(&self) -> u64 {
        self.eod
    }
    pub fn next_id(&mut self) -> u32 {
        self.next_id += 1;
        self.next_id - 1
    }
}
impl fmt::Display for ArchiveInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Archive created with Prisirv {}", self.ver)?;
        for blk in self.blks.iter() {
            write!(f, "{blk}")?;
        }
        Ok(())
    }
}
impl fmt::Debug for ArchiveInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Archive created with Prisirv {}", self.ver)?;
        for blk in self.blks.iter() {
            write!(f, "{blk:?}")?;
        }
        Ok(())
    }
}