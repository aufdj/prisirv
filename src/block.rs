use std::{
    fs::File,
    io::{BufWriter, BufReader, Write},
    path::PathBuf,
    str,
    fmt,
};
use crate::{
    filedata::FileData,
    config::{Config, Method},
    buffered_io::{BufferedWrite, BufferedRead},
    error::ArchiveError,
    constant::{MAGIC, Version},
};

#[derive(Clone, Default)]
pub struct Block {
    pub mem:     u64,           // Memory usage
    pub blk_sz:  usize,         // Block size
    pub id:      u32,           // Block id
    pub chksum:  u32,           // Input block checksum
    pub sizeo:   u64,           // Output data size
    pub sizei:   u64,           // Input data size
    pub crtd:    u64,           // Creation time
    pub files:   Vec<FileData>, // Files in this block
    pub ver:     Version,       // Version number
    pub data:    Vec<u8>,       // Block data 
    pub method:  Method,        // Context Mixing, LZW, or Uncompressed
}
impl Block {
    pub fn new(cfg: &Config) -> Block {
        Block {
            mem:     cfg.mem,
            blk_sz:  cfg.blk_sz,
            method:  cfg.method,
            id:      0,
            chksum:  0,
            sizeo:   0,
            sizei:   0,
            crtd:    0,  
            ver:     Version::current(),
            files:   Vec::new(),
            data:    Vec::with_capacity(cfg.blk_sz),
        }
    }
    pub fn next(&mut self) {
        self.data.clear();
        self.files.clear();
        self.sizei = 0;
        self.id += 1;
    }
    pub fn write_to(&self, archive: &mut BufWriter<File>) {
        archive.write_u32(MAGIC);
        archive.write_u16(self.ver.major);
        archive.write_u16(self.ver.minor);
        archive.write_u16(self.ver.patch);
        archive.write_u64(self.mem);
        archive.write_u64(self.blk_sz as u64);
        archive.write_byte(self.method as u8);
        archive.write_u32(self.id);
        archive.write_u32(self.chksum);
        archive.write_u64(self.sizeo);
        archive.write_u64(self.sizei);
        archive.write_u64(self.crtd);
        archive.write_u32(self.files.len() as u32);

        for file in self.files.iter() {
            let path = file.path.to_str().unwrap().as_bytes();
            archive.write_all(path).unwrap();
            archive.write_byte(0);
            archive.write_u64(file.len);
            archive.write_u64(file.seg_beg);
            archive.write_u64(file.seg_end);
            archive.write_u64(file.blk_pos);
        }

        for byte in self.data.iter() {
            archive.write_byte(*byte);
        }
    }
    /// Read entire block
    pub fn read_from(&mut self, archive: &mut BufReader<File>) -> Result<(), ArchiveError>  {
        self.read_header_from(archive)?;

        self.data.reserve(self.blk_sz);
        
        // Read compressed data
        for _ in 0..self.sizeo {
            self.data.push(archive.read_byte());
        }
        Ok(())
    }
    /// Read block header
    pub fn read_header_from(&mut self, archive: &mut BufReader<File>) -> Result<(), ArchiveError> {
        let magic      = archive.read_u32();
        self.ver.major = archive.read_u16();
        self.ver.minor = archive.read_u16();
        self.ver.patch = archive.read_u16();
        self.mem       = archive.read_u64();
        self.blk_sz    = archive.read_u64() as usize;
        self.method    = Method::from(archive.read_byte());
        self.id        = archive.read_u32();
        self.chksum    = archive.read_u32();
        self.sizeo     = archive.read_u64();
        self.sizei     = archive.read_u64();
        self.crtd      = archive.read_u64();
        let num_files  = archive.read_u32();

        if magic != MAGIC { 
            return Err(ArchiveError::InvalidMagicNumber(self.id));
        }

        let mut path: Vec<u8> = Vec::with_capacity(64);

        // Read null terminated path strings and lengths
        for _ in 0..num_files {
            loop {
                match archive.read_byte() {
                    0 => {
                        self.files.push(
                            FileData {
                                path:     PathBuf::from(str::from_utf8(&path)?),
                                len:      archive.read_u64(), 
                                seg_beg:  archive.read_u64(), 
                                seg_end:  archive.read_u64(), 
                                blk_pos:  archive.read_u64(), 
                                ..Default::default() 
                            }
                        );
                        path.clear();
                        break;
                    }
                    byte => path.push(byte),
                }
            }
        }
        Ok(())
    }
    pub fn size(&self) -> u64 {
        self.files.iter().map(|file| file.size() + 1).sum::<u64>()
        + 63 
        + self.data.len() as u64
    }
}
impl fmt::Display for Block {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for file in self.files.iter() {
            if file.seg_beg != file.seg_end && file.seg_beg == 0 {
                write!(f, "
                    \r{}", 
                    file.path.display()
                )?;
            }
        }
        Ok(())
    }
}
impl fmt::Debug for Block {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "
            \rBlock {}:
            \r==========================================
            \rUncompressed Size: {}
            \rCompressed Size:   {}
            \rCRC32 Checksum:    {:x}
            \rCreation time:     {}
            \rMemory Usage:      {}\n\n",
            self.id,
            self.sizei, 
            self.sizeo,
            self.chksum, 
            self.crtd,
            self.mem
        )?;
        write!(f, "\rFiles:")?;
        for file in self.files.iter() {
            if file.seg_beg != file.seg_end {
                write!(f, "
                    \r  Path:   {}
                    \r  Length: {}
                    \r  Segment Begin:  {}
                    \r  Segment End:    {}
                    \r  Block Position: {}\n",
                    file.path.display(), 
                    file.len,
                    file.seg_beg, 
                    file.seg_end,
                    file.blk_pos
                )?;
            }
        }
        writeln!(f, "\r==========================================")
    }
}