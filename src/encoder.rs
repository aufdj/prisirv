use std::{
    io::{Seek, BufWriter},
    fs::File,
};
use crate::{
    predictor::Predictor,
    buffered_io::BufferedWrite,
    parse_args::Config,
    Metadata,
    Arch,
};


// Encoder ------------------------------------------------------------------------------------------------------------------------ Encoder
pub struct Encoder {
    high:          u32,       // Right endpoint of range
    low:           u32,       // Left endpoint of range
    predictor:     Predictor, // Generates predictions
    mem:           usize,     // Memory option
    pub file_out:  BufWriter<File>, 
}
impl Encoder {
    pub fn new(file_out: BufWriter<File>, cfg: &Config) -> Encoder {
        let mut enc = Encoder {
            high: 0xFFFFFFFF,
            low: 0,
            predictor: Predictor::new(cfg.mem),
            file_out,
            mem: cfg.mem,
        };
        // Metadata placeholder
        for _ in match cfg.arch {
            Arch::Solid    => { 0..4 } 
            Arch::NonSolid => { 0..6 }
        } { enc.file_out.write_usize(0); }
        enc
    }
    pub fn compress_bit(&mut self, bit: i32) {
        let mut p = self.predictor.p() as u32;
        if p < 2048 { p += 1; }
        
        let range = self.high - self.low;
        let mid: u32 = self.low + (range >> 12) * p
                       + ((range & 0x0FFF) * p >> 12);
                       
        if bit == 1 {
            self.high = mid;
        }
        else {
            self.low = mid + 1;
        }
        self.predictor.update(bit);
        
        while ( (self.high ^ self.low) & 0xFF000000) == 0 {
            self.file_out.write_byte((self.high >> 24) as u8);
            self.high = (self.high << 8) + 255;
            self.low <<= 8;
        }
    }
    pub fn compress_block(&mut self, block: &[u8]) {
        for byte in block.iter() {
            for i in (0..=7).rev() {
                self.compress_bit(((*byte >> i) & 1) as i32);
            }
        }
    }
    pub fn write_header(&mut self, mta: &Metadata, arch: Arch) {
        self.file_out.rewind().unwrap();
        self.file_out.write_usize(self.mem);
        match arch {
            Arch::Solid => {
                self.file_out.write_usize(mta.mgcs);
                self.file_out.write_usize(mta.bl_sz);
                self.file_out.write_usize(mta.f_ptr);
            }
            Arch::NonSolid => {
                self.file_out.write_usize(mta.mgc);
                self.file_out.write_usize(mta.ext);
                self.file_out.write_usize(mta.f_bl_sz);
                self.file_out.write_usize(mta.bl_sz);
                self.file_out.write_usize(mta.bl_c);
            }
        }
    }
    pub fn flush(&mut self) {
        while ( (self.high ^ self.low) & 0xFF000000) == 0 {
            self.file_out.write_byte((self.high >> 24) as u8);
            self.high = (self.high << 8) + 255;
            self.low <<= 8;
        }
        self.file_out.write_byte((self.high >> 24) as u8);
        self.file_out.flush_buffer();
    }
}