use std::iter::repeat;

use crate::logistic::{stretch, squash};

// Adaptive Probability Map -------------------------------------------------------------------------------------- Adaptive Probability Map
pub struct Apm {
    bin:       usize,    // A value used for interpolating a new prediction
    num_cxts:  usize,    // Number of possible contexts i.e 256 for order-0
    bin_map:   Vec<u16>, // Table mapping values 0..=32 to squashed 16 bit values
}
impl Apm {
    pub fn new(n: usize) -> Apm {
        Apm {
            bin:       0,
            num_cxts:  n,
            bin_map:   repeat( // Map 0..33 to values in closure, create n copies
                       (0..33).map(|i| (squash((i - 16) * 128) * 16) as u16)
                       .collect::<Vec<u16>>().into_iter() )
                       .take(n)
                       .flatten()
                       .collect::<Vec<u16>>(),
        }
    }
    pub fn p(&mut self, bit: i32, rate: i32, mut pr: i32, cxt: u32) -> i32 {
        assert!(bit == 0 || bit == 1 && pr >= 0 && pr < 4096);
        assert!(cxt < self.num_cxts as u32);
        self.update(bit, rate);
        
        pr = stretch(pr);   // -2047 to 2047
        let i_w = pr & 127; // Interpolation weight (33 points)
        
        // Compute set of bins from context, and singular bin from prediction
        self.bin = (((pr + 2048) >> 7) + ((cxt as i32) * 33)) as usize;

        let l = self.bin_map[self.bin] as i32;   // Lower bin
        let u = self.bin_map[self.bin+1] as i32; // Upper bin
        ((l * (128 - i_w)) + (u * i_w)) >> 11 // Interpolate pr from bin and bin+1
    }
    pub fn update(&mut self, bit: i32, rate: i32) {
        assert!(bit == 0 || bit == 1 && rate > 0 && rate < 32);
        
        // Controls direction of update (bit = 1: increase, bit = 0: decrease)
        let g: i32 = (bit << 16) + (bit << rate) - bit - bit;

        // Bins used for interpolating previous prediction
        let l = self.bin_map[self.bin] as i32;   // Lower
        let u = self.bin_map[self.bin+1] as i32; // Upper
        self.bin_map[self.bin]   = (l + ((g - l) >> rate)) as u16;
        self.bin_map[self.bin+1] = (u + ((g - u) >> rate)) as u16;
    }
}
// ----------------------------------------------------------------------------------------------------------------------------------------
