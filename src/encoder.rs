use crate::predictor::Predictor;

pub struct Encoder {
    high:       u32,       // Right endpoint of range
    low:        u32,       // Left endpoint of range
    predictor:  Predictor, // Generates predictions
    pub out:    Vec<u8>,   // Compressed block
}
impl Encoder {
    pub fn new(mem: usize, blk_sz: usize) -> Encoder {
        Encoder {
            high: 0xFFFFFFFF,
            low: 0,
            predictor: Predictor::new(mem),
            out: Vec::with_capacity(blk_sz),
        }
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
            self.out.push((self.high >> 24) as u8);
            self.high = (self.high << 8) + 255;
            self.low <<= 8;
        }
    }
    pub fn compress_block(&mut self, block: &[u8]) -> Vec<u8> {
        for byte in block.iter() {
            for i in (0..=7).rev() {
                self.compress_bit(((*byte >> i) & 1) as i32);
            }
        }
        self.flush();
        self.out.clone()
    }
    pub fn flush(&mut self) {
        while ( (self.high ^ self.low) & 0xFF000000) == 0 {
            self.out.push((self.high >> 24) as u8);
            self.high = (self.high << 8) + 255;
            self.low <<= 8;
        }
        self.out.push((self.high >> 24) as u8);
    }
}