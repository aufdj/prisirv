use std::{
    io::BufReader,
    fs::File,
};
use crate::{
    buffered_io::BufferedRead,
    predictor::Predictor,
    Metadata,
};


pub struct Decoder {
    high:         u32,
    low:          u32,
    predictor:    Predictor,
    pub file_in:  BufReader<File>,
    x:            u32, // 4 byte sliding window of compressed data
}
impl Decoder {
    pub fn new(mut file_in: BufReader<File>) -> Decoder {
        let mem: usize = file_in.read_usize();
        Decoder {
            high: 0xFFFFFFFF,
            low: 0,
            x: 0,
            predictor: Predictor::new(mem),
            file_in,
        }   
    }
    fn decompress_bit(&mut self) -> i32 {
        let mut p = self.predictor.p() as u32;
        if p < 2048 { p += 1; }

        let range = self.high - self.low;
        let mid: u32 = self.low + (range >> 12) * p
                       + ((range & 0x0FFF) * p >> 12);

        let mut bit: i32 = 0;
        if self.x <= mid {
            bit = 1;
            self.high = mid;
        }
        else {
            self.low = mid + 1;
        }
        self.predictor.update(bit);
        
        while ( (self.high ^ self.low) & 0xFF000000) == 0 {
            self.high = (self.high << 8) + 255;
            self.low <<= 8;
            self.x = (self.x << 8) + self.file_in.read_byte() as u32;
        }
        bit
    }
    pub fn decompress_block(&mut self, block_size: usize) -> Vec<u8> {
        let mut block: Vec<u8> = Vec::with_capacity(block_size);
        while block.len() < block.capacity() {
            let mut byte: i32 = 1;
            while byte < 256 {
                byte += byte + self.decompress_bit();
            }
            byte -= 256;
            block.push(byte as u8);
        }
        block
    }
    pub fn read_header(&mut self, solid: bool) -> Metadata {
        let mut mta: Metadata = Metadata::new();
        if solid {
            mta.mgc =   self.file_in.read_usize();
            mta.bl_sz = self.file_in.read_usize();
            mta.f_ptr = self.file_in.read_usize();
        }
        else {
            mta.mgc =     self.file_in.read_usize();
            mta.ext =     self.file_in.read_usize();
            mta.f_bl_sz = self.file_in.read_usize();
            mta.bl_sz =   self.file_in.read_usize();
            mta.bl_c =    self.file_in.read_usize();
        }
        mta
    }
    // Inititialize decoder with first 4 bytes of compressed data
    pub fn init_x(&mut self) {
        for _ in 0..4 {
            self.x = (self.x << 8) + self.file_in.read_byte() as u32;
        }
    }
}
// ----------------------------------------------------------------------------------------------------------------------------------------
