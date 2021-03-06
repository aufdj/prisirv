use crate::cm::predictor::Predictor;

/// A block based arithmetic encoder. Accepts an uncompressed block and
/// returns a compressed block.
pub struct Encoder {
    high:         u32,       // Right endpoint of range
    low:          u32,       // Left endpoint of range
    predictor:    Predictor, // Generates predictions
    pub blk_out:  Vec<u8>,   // Compressed block
}
impl Encoder {
    /// Create a new Encoder.
    pub fn new(mem: usize, blk_sz: usize) -> Encoder {
        Encoder {
            high:       0xFFFFFFFF,
            low:        0,
            predictor:  Predictor::new(mem),
            blk_out:    Vec::with_capacity(blk_sz),
        }
    }

    /// Compress one bit with the latest prediction.
    pub fn compress_bit(&mut self, bit: i32) {
        let mut p = self.predictor.p() as u32;
        if p < 2048 { 
            p += 1; 
        }
        
        let range = self.high - self.low;
        let mid: u32 = self.low + (range >> 12) * p
                       + (((range & 0x0FFF) * p) >> 12);
                       
        if bit == 1 {
            self.high = mid;
        }
        else {
            self.low = mid + 1;
        }
        self.predictor.update(bit);
        
        while ( (self.high ^ self.low) & 0xFF000000) == 0 {
            self.blk_out.push((self.high >> 24) as u8);
            self.high = (self.high << 8) + 255;
            self.low <<= 8;
        }
    }

    /// Compress one block.
    pub fn compress_block(&mut self, blk_in: &[u8]) {
        for byte in blk_in.iter() {
            for i in (0..=7).rev() {
                self.compress_bit(((*byte >> i) & 1) as i32);
            }
        }
        self.flush();
    }

    /// Flush any remaining equal Most Significant Bytes (MSBs), then flush
    /// the first non-equal MSB, marking the end of compression.
    pub fn flush(&mut self) {
        while ( (self.high ^ self.low) & 0xFF000000) == 0 {
            self.blk_out.push((self.high >> 24) as u8);
            self.high = (self.high << 8) + 255;
            self.low <<= 8;
        }
        if !self.blk_out.is_empty() {
            self.blk_out.push((self.high >> 24) as u8);
        }
    }
}