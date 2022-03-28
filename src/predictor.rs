use crate::{
    logistic::stretch, 
    tables::STATE_TABLE,
    word_model::WordModel,
    match_model::MatchModel, 
    context_model::{
        ContextModelO1,
        ContextModelO2,
        ContextModelO3,
        ContextModelO4,
        ContextModelO6,
    },
    hash_table::HashTable, 
    mixer::Mixer, 
    apm::Apm, 
};


/// # Predictor
///
/// Prisirv is a context mixing compressor with the same model as lpaq1
/// by Matt Mahoney <http://mattmahoney.net/dc/#lpaq>. The model combines 
/// 7 contexts: orders 1, 2, 3, 4, 6, a lowercase unigram word context 
/// (for ASCII text), and a "match" order, which predicts the next bit in 
/// the last matching context. The independent bit predictions of the 7 
/// models are combined using one of 80 neural networks (selected by a 
/// small context), then adjusted using 2 SSE stages (order 0 and 1) and 
/// arithmetic coded.
/// 
/// Prediction is bitwise. This means that an order-n context consists of 
/// the last n whole bytes plus any of the 0 to 7 previously coded bits of 
/// the current byte starting with the most significant bit. The unigram 
/// word context consists of a hash of the last (at most) 11 consecutive 
/// letters (A-Z, a-z) folded to lower case. The context does not include 
/// any nonalphabetic characters nor any characters preceding the last 
/// nonalphabetic character.
/// 
/// The first 6 contexts (orders 1..4, 6, word) are used to index a hash 
/// table to look up a bit-history represented by an 8-bit state. A state 
/// can either represent all histories up to 4 bits long, or a pair of 0,1 
/// counts plus a flag to indicate the most recent bit. The counts are 
/// bounded by (41,0), (40,1), (12,2), (5,3), (4,4) and likewise for 1,0. 
/// If a count is exceeded, the opposite count is reduced to approximately 
/// preserve the count ratio. The last bit flag is present only for states 
/// whose total count is less than 16. There are 253 possible states.
///
/// The 7 predictions are combined using a neural network (Mixer). The
/// inputs p_i, i=0..6 are first stretched: t_i = log(p_i/(1 - p_i)), 
/// then the output is computed: p = squash(SUM_i t_i * w_i), where
/// squash(x) = 1/(1 + exp(-x)) is the inverse of stretch(). The weights
/// are adjusted to reduce the error: w_i := w_i + L * t_i * (y - p) where
/// (y - p) is the prediction error and L ~ 0.002 is the learning rate.
/// This is a standard single layer backpropagation network modified to
/// minimize coding cost rather than RMS prediction error (thus dropping
/// the factors p * (1 - p) from learning).
/// 
/// One of 80 neural networks are selected by a context that depends on
/// the 3 high order bits of the last whole byte plus the context order
/// (quantized to 0, 1, 2, 3, 4, 6, 8, 12, 16, 32). The order is
/// determined by the number of nonzero bit histories and the length of
/// the match from MatchModel.
/// 
/// The Mixer output is adjusted by 2 SSE stages (called APM for adaptive
/// probability map). An APM is a StateMap that accepts both a discrte
/// context and an input probability, pr. pr is stetched and quantized
/// to 24 levels. The output is interpolated between the 2 nearest table 
/// entries, and then only the nearest entry is updated. The entries are 
/// initialized to p = pr and n = 6 (to slow down initial adaptation)
/// with a limit n <= 255. The two stages use a discrete order 0 context 
/// (last 0..7 bits) and a hashed order-1 context (14 bits). Each output 
/// is averaged with its input weighted by 1/4.
/// 
/// The output is arithmetic coded. The code for a string s with probability
/// p(s) is a number between Q and Q+p(x) where Q is the total probability of 
/// all strings lexicographically preceding s. The number is coded as a big-
/// -endian base-256 fraction.

/// Transition to next state in state table.
pub fn next_state(state: u8, bit: i32) -> u8 {
    STATE_TABLE[state as usize][bit as usize]
}

pub struct Predictor {
    cxt:   u32,            // Order 0 context
    bits:  usize,          // Number of bits currently in 'cxt'
    pr:    i32,            // Prediction
    wm:    WordModel,      // Lowercase unigram word model
    mm:    MatchModel,     // Match model
    cm1:   ContextModelO1, // Order 1 context model
    cm2:   ContextModelO2, // Order 2 context model 
    cm3:   ContextModelO3, // Order 3 context model
    cm4:   ContextModelO4, // Order 4 context model
    cm6:   ContextModelO6, // Order 6 context model
    ht:    HashTable,      // Hash table for mapping contexts to state arrays
    mxr:   Mixer,          // For weighted averaging of independent predictions
    apm1:  Apm,            // Adaptive Probability Map for refining Mixer output
    apm2:  Apm,            //
}
impl Predictor {
    /// Create a new Predictor.
    pub fn new(mem: usize) -> Predictor {
        let mut p = Predictor {
            cxt:   1,                               
            bits:  0,            
            pr:    2048,         
            wm:    WordModel::new(),
            mm:    MatchModel::new(mem),
            cm1:   ContextModelO1::new(),
            cm2:   ContextModelO2::new(),
            cm3:   ContextModelO3::new(),
            cm4:   ContextModelO4::new(),
            cm6:   ContextModelO6::new(),
            ht:    HashTable::new(mem*2),
            mxr:   Mixer::new(7, 80),
            apm1:  Apm::new(256),
            apm2:  Apm::new(16384),
        };
        
        p.wm.state  = &mut p.cm1.t0[0];
        p.cm1.state = &mut p.cm1.t0[0];
        p.cm2.state = &mut p.cm1.t0[0];
        p.cm3.state = &mut p.cm1.t0[0];
        p.cm4.state = &mut p.cm1.t0[0];
        p.cm6.state = &mut p.cm1.t0[0];
        p
    }

    /// Return current prediction.
    pub fn p(&mut self) -> i32 {
        assert!(self.pr >= 0 && self.pr < 4096);
        self.pr
    }

    /// Update contexts and states, map states to predictions, and mix
    /// predictions in Mixer.
    pub fn update(&mut self, bit: i32) {
        assert!(bit == 0 || bit == 1);
        
        self.mxr.update(bit);
        self.wm.update(bit);
        self.cm1.update(bit);
        self.cm2.update(bit);
        self.cm3.update(bit);
        self.cm4.update(bit);
        self.cm6.update(bit);
        
        // Update order-0 context
        self.cxt += self.cxt + bit as u32;
        self.bits += 1;

        if self.cxt >= 256 { // Byte boundary
            self.new_state_arr(0);
            self.cxt = 1;
            self.bits = 0;
        }
        if self.bits == 4 { // Nibble boundary
            self.new_state_arr(self.cxt);
        }

        
        // Get prediction and length from match model
        self.mm.p(bit, &mut self.mxr);
        let len = self.mm.len();
        let order = self.order(len);

        // Add independent predictions to mixer 
        self.mxr.add(stretch(self.wm.p(bit)));
        self.mxr.add(stretch(self.cm1.p(bit)));
        self.mxr.add(stretch(self.cm2.p(bit)));
        self.mxr.add(stretch(self.cm3.p(bit)));
        self.mxr.add(stretch(self.cm4.p(bit)));
        self.mxr.add(stretch(self.cm6.p(bit)));
        
        // Set weights to be used during mixing
        self.mxr.set(order + 10 * (self.cm1.o1cxt >> 13));

        // Mix
        self.pr = self.mxr.p();

        // 2 SSE stages
        self.pr = (self.pr + 3 * self.apm1.p(bit, 7, self.pr, self.cxt)) >> 2;
        self.pr = (self.pr + 3 * self.apm2.p(bit, 7, self.pr, self.cxt ^ self.cm1.o1cxt >> 2)) >> 2;
    }
    /// Map context hashes to state arrays
    pub fn new_state_arr(&mut self, nibble: u32) {
        unsafe {
            self.wm.state  = self.ht.hash(self.wm.word_cxt + nibble).add(1);
            self.cm2.state = self.ht.hash(self.cm2.o2cxt + nibble).add(1);
            self.cm3.state = self.ht.hash(self.cm3.o3cxt + nibble).add(1);
            self.cm4.state = self.ht.hash(self.cm4.o4cxt + nibble).add(1);
            self.cm6.state = self.ht.hash(self.cm6.o6cxt + nibble).add(1);
        }
    }
    fn order(&mut self, len: usize) -> u32 {
        let mut order: u32 = 0;

        // If len is 0, order is determined from 
        // number of non-zero bit histories
        if len == 0 {
            unsafe {
                if *self.cm2.state != 0 { order += 1; }
                if *self.cm3.state != 0 { order += 1; }
                if *self.cm4.state != 0 { order += 1; }
                if *self.cm6.state != 0 { order += 1; }
            }
        }
        else {
            order = 5 +
            if len >= 8  { 1 } else { 0 } +
            if len >= 12 { 1 } else { 0 } +
            if len >= 16 { 1 } else { 0 } +
            if len >= 32 { 1 } else { 0 };
        }
        order
    }
}
