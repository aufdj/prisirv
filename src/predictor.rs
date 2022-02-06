use crate::{
    match_model::MatchModel, 
    hash_table::HashTable, 
    apm::Apm, 
    mixer::Mixer, 
    logistic::stretch, 
    statemap::StateMap, 
    tables::STATE_TABLE,
};


// Predictor -------------------------------------------------------------------------------------------------------------------- Predictor
// Prisirv is a context mixing compressor with the same model as lpaq1
// by Matt Mahoney (http://mattmahoney.net/dc/#lpaq). The model combines 7 
// contexts: orders 1, 2, 3, 4, 6, a lowercase unigram word context 
// (for ASCII text), and a "match" order, which predicts the next
// bit in the last matching context. The independent bit predictions of
// the 7 models are combined using one of 80 neural networks (selected by
// a small context), then adjusted using 2 SSE stages (order 0 and 1)
// and arithmetic coded.
// 
// Prediction is bitwise. This means that an order-n context consists
// of the last n whole bytes plus any of the 0 to 7 previously coded
// bits of the current byte starting with the most significant bit.
// The unigram word context consists of a hash of the last (at most) 11
// consecutive letters (A-Z, a-z) folded to lower case. The context
// does not include any nonalphabetic characters nor any characters
// preceding the last nonalphabetic character.
// 
// The first 6 contexts (orders 1..4, 6, word) are used to index a
// hash table to look up a bit-history represented by an 8-bit state.
// A state can either represent all histories up to 4 bits long, or a 
// pair of 0,1 counts plus a flag to indicate the most recent bit. The 
// counts are bounded by (41,0), (40,1), (12,2), (5,3), (4,4) and likewise
// for 1,0. When a count is exceeded, the opposite count is reduced to 
// approximately preserve the count ratio. The last bit flag is present
// only for states whose total count is less than 16. There are 253
// possible states.
//
// The 7 predictions are combined using a neural network (Mixer). The
// inputs p_i, i=0..6 are first stretched: t_i = log(p_i/(1 - p_i)), 
// then the output is computed: p = squash(SUM_i t_i * w_i), where
// squash(x) = 1/(1 + exp(-x)) is the inverse of stretch(). The weights
// are adjusted to reduce the error: w_i := w_i + L * t_i * (y - p) where
// (y - p) is the prediction error and L ~ 0.002 is the learning rate.
// This is a standard single layer backpropagation network modified to
// minimize coding cost rather than RMS prediction error (thus dropping
// the factors p * (1 - p) from learning).
// 
// One of 80 neural networks are selected by a context that depends on
// the 3 high order bits of the last whole byte plus the context order
// (quantized to 0, 1, 2, 3, 4, 6, 8, 12, 16, 32). The order is
// determined by the number of nonzero bit histories and the length of
// the match from MatchModel.
// 
// The Mixer output is adjusted by 2 SSE stages (called APM for adaptive
// probability map). An APM is a StateMap that accepts both a discrte
// context and an input probability, pr. pr is stetched and quantized
// to 24 levels. The output is interpolated between the 2 nearest
// table entries, and then only the nearest entry is updated. The entries
// are initialized to p = pr and n = 6 (to slow down initial adaptation)
// with a limit n <= 255. The two stages use a discrete order 0 context 
// (last 0..7 bits) and a hashed order-1 context (14 bits). Each output 
// is averaged with its input weighted by 1/4.
// 
// The output is arithmetic coded. The code for a string s with probability
// p(s) is a number between Q and Q+p(x) where Q is the total probability
// of all strings lexicographically preceding s. The number is coded as
// a big-endian base-256 fraction.


fn next_state(state: u8, bit: i32) -> u8 {
    STATE_TABLE[state as usize][bit as usize]
}

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
    sm:    Vec<StateMap>, // 6 State Maps
}
impl Predictor {
    pub fn new(mem: usize) -> Predictor {
        let mut p = Predictor {
            cxt:   1,            mm:    MatchModel::new(mem),
            cxt4:  0,            ht:    HashTable::new(mem*2),
            bits:  0,            apm1:  Apm::new(256),
            pr:    2048,         apm2:  Apm::new(16384),
            h:     [0; 6],       mxr:   Mixer::new(7, 80),
            sp:    [&mut 0; 6],  sm:    vec![StateMap::new(256); 6],
            t0:    [0; 65_536],  
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
    pub fn new_state_arr(&mut self, cxt: [u32; 6], nibble: u32) {
        unsafe {
            for (i, cxt) in cxt.iter().enumerate().skip(1) {
                self.sp[i] = self.ht.hash(cxt + nibble).add(1);
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
        
        self.h[5] = match self.cxt { // Unigram Word Order
            65..=90 => {
                self.cxt += 32; // Fold to lowercase
                (self.h[5] + self.cxt).wrapping_mul(7 << 3)
            },
            97..=122 => {
                (self.h[5] + self.cxt).wrapping_mul(7 << 3)
            },
            _ => 0,
        };
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

            self.new_state_arr(self.h, 0);

            self.cxt = 1;
            self.bits = 0;
        }
        if self.bits == 4 { // Nibble boundary
            self.new_state_arr(self.h, self.cxt);
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
                    stretch(self.sm[i].p(bit, *self.sp[i] as i32))
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
