use crate::{
    match_model::MatchModel, 
    hash_table::HashTable, 
    apm::Apm, 
    mixer::Mixer, 
    logistic::Stretch, 
    statemap::{StateMap, next_state}, 
    MEM
};

// Predictor -------------------------------------------------------------------------------------------------------------------- Predictor
pub struct Predictor {
    cxt:   u32,           // Order 0 context
    cxt4:  u32,           // Order 3 context
    bits:  usize,         // Number of bits currently in 'cxt'
    pr:    i32,           // Prediction
    h:     [u32; 6],      // Order 1, 2, 3, 4, 6, and Unigram Word contexts 
    sp:    [*mut u8; 6],  // Pointers to state within a state array
    t0:    [u8; 65_536],  // Order 1 context direct lookup table
    mm:    MatchModel,    // Model for longest context match
    ht:    HashTable,     // Hash table for mapping contexts to state arrays
    apm1:  Apm,           // Adaptive Probability Map for refining Mixer output
    apm2:  Apm,           //
    mxr:   Mixer,         // For weighted averaging of independent predictions
    s:     Stretch,       // Computes stretch(d), or ln(d/(1-d))
    sm:    Vec<StateMap>, // 6 State Maps
}
impl Predictor {
    pub fn new() -> Predictor {
        let mut p = Predictor {
            cxt:   1,            mm:    MatchModel::new(),
            cxt4:  0,            ht:    HashTable::new(MEM*2),
            bits:  0,            apm1:  Apm::new(256),
            pr:    2048,         apm2:  Apm::new(16384),
            h:     [0; 6],       mxr:   Mixer::new(7, 80),
            sp:    [&mut 0; 6],  s:     Stretch::new(),
            t0:    [0; 65_536],  sm:    vec![StateMap::new(256); 6],
        };
        for i in 0..6 {
            p.sp[i] = &mut p.t0[0];
        }
        p
    }

    pub fn p(&mut self) -> i32 {
        assert!(self.pr >= 0 && self.pr < 4096);
        self.pr
    }

    // Set state pointer 'sp[i]' to beginning of new state array
    pub fn update_state_ptrs(&mut self, cxt: [u32; 6], nibble: u32) {
        unsafe {
            for i in 1..6 {
                self.sp[i] = self.ht.hash(cxt[i]+nibble).add(1);
            }
        }
    }

    // Update order 1, 2, 3, 4, 6, and unigram word contexts
    pub fn update_cxts(&mut self, cxt: u32, cxt4: u32) {
        self.h[0] =  cxt << 8;                         // Order 1
        self.h[1] = (cxt4 & 0xFFFF) << 5 | 0x57000000; // Order 2
        self.h[2] = (cxt4 << 8).wrapping_mul(3);       // Order 3
        self.h[3] =  cxt4.wrapping_mul(5);             // Order 4
        self.h[4] =  self.h[4].wrapping_mul(11 << 5)   // Order 6
                     + cxt * 13 & 0x3FFFFFFF;
        
        match self.cxt { // Unigram Word Order
            65..=90 => {
                self.cxt += 32; // Fold to lowercase
                self.h[5] = (self.h[5] + self.cxt).wrapping_mul(7 << 3);
            }
            97..=122 => {
                self.h[5] = (self.h[5] + self.cxt).wrapping_mul(7 << 3);
            },
            _ => self.h[5] = 0,
        }
    }

    pub fn update(&mut self, bit: i32) {
        assert!(bit == 0 || bit == 1);

        // Transition to new states
        unsafe {
            for i in 0..6 {
                *self.sp[i] = next_state(*self.sp[i], bit);
            }
        }
        self.mxr.update(bit);

        // Update order-0 context
        self.cxt += self.cxt + bit as u32;
        self.bits += 1;

        if self.cxt >= 256 { // Byte boundary
            // Update order-3 context
            self.cxt -= 256;
            self.cxt4 = (self.cxt4 << 8) | self.cxt;
            self.update_cxts(self.cxt, self.cxt4);

            self.update_state_ptrs(self.h, 0);

            self.cxt = 1;
            self.bits = 0;
        }
        if self.bits == 4 { // Nibble boundary
            self.update_state_ptrs(self.h, self.cxt);
        }
        else if self.bits > 0 {
            // Calculate new state array index
            let j = ((bit as usize) + 1) << (self.bits & 3) - 1;
            unsafe {
                for i in 1..6 {
                    self.sp[i] = self.sp[i].add(j);
                }
            }
        }
    
        // Update order-1 context
        unsafe { 
        self.sp[0] = ((&mut self.t0[0] as *mut u8)
                     .add(self.h[0] as usize))
                     .add(self.cxt as usize);
        }
        

        // Get prediction and length from match model
        self.mm.p(bit, &mut self.mxr);
        let len = self.mm.len();
        let mut order: u32 = 0;

        // If len is 0, order is determined from 
        // number of non-zero bit histories
        if len == 0 {
            unsafe {
            if *self.sp[4] != 0 { order += 1; }
            if *self.sp[3] != 0 { order += 1; }
            if *self.sp[2] != 0 { order += 1; }
            if *self.sp[1] != 0 { order += 1; }
            }
        }
        else {
            order = 5 +
            if len >= 8  { 1 } else { 0 } +
            if len >= 12 { 1 } else { 0 } +
            if len >= 16 { 1 } else { 0 } +
            if len >= 32 { 1 } else { 0 };
        }

        // Add independent predictions to mixer 
        unsafe {
            for i in 0..6 {
                self.mxr.add(
                    self.s.stretch(
                        self.sm[i].p(bit, *self.sp[i] as i32)
                    )
                );
            }
        }

        // Set weights to be used during mixing
        self.mxr.set(order + 10 * (self.h[0] >> 13));

        // Mix
        self.pr = self.mxr.p();

        // 2 SSE stages
        self.pr = self.pr + 3 * self.apm1.p(bit, 7, self.pr, self.cxt) >> 2;
        self.pr = self.pr + 3 * self.apm2.p(bit, 7, self.pr, self.cxt ^ self.h[0] >> 2) >> 2;
    }
}
// ----------------------------------------------------------------------------------------------------------------------------------------
