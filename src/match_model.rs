use crate::{
    logistic::stretch,
    statemap::StateMap,
    mixer::Mixer,
};


/// Match Model =========================================================================
/// A match model finds the last occurrence of a high order context and predicts 
/// whatever symbol came next. The accuracy of the prediction depends on the length 
/// of the context match. The prediction for a match of L bytes (or 8L bits) is 
/// that the next bit will be the same with probability 1 - 1/8L. Typically a match 
/// model of order 6-8 is mixed with lower order context models. A match model is 
/// faster and uses less memory than a corresponding context model but does not model 
/// well for low orders.
///
/// The model looks up the current context in a
/// hash table, first using a longer context, then a shorter one. If
/// a match is found, then the following bits are predicted until there is
/// a misprediction. The prediction is computed by mapping the predicted
/// bit, the length of the match (1..15 or quantized by 4 in 16..62, max 62),
/// and the last whole byte as context into a StateMap. If no match is found,
/// then the order 0 context (last 0..7 bits of the current byte) is used
/// as context to the StateMap.
/// 
/// One third of memory is used by MatchModel, divided equally between 
/// a rotating input buffer of 2^(N+19) bytes and an index (hash table)
/// with 2^(N+17) entries. Two context hashes are maintained, a long one,
/// h1, of length ceil((N+17)/3) bytes and a shorter one, h2, of length 
/// ceil((N+17)/5) bytes, where ceil() is the ceiling function. The index
/// does not use collision detection. At each byte boundary, if there is 
/// not currently a match, then the bytes before the current byte are
/// compared with the location indexed by h1. If less than 2 bytes match, 
/// then h2 is tried. If a match of length 1 or more is found, the match
/// is maintained until the next bit mismatches the predicted bit.
/// The table is updated at h1 and h2 after every byte.
/// =====================================================================================

const MAX_LEN: usize = 62; // Maximum match length, up to 62

pub struct MatchModel {
    mch_ptr:  usize,    // Pointer to current byte in matched context in buf
    mch_len:  usize,    // Length of match
    cxt:      usize,    // Order-0 context (last 0..7 bits)
    bits:     usize,    // Number of bits in cxt
    hash_s:   usize,    // Short context hash
    hash_l:   usize,    // Long context hash
    buf_pos:  usize,    // Number of bytes in buf
    sm:       StateMap, // Len, bit, last byte -> prediction
    buf:      Vec<u8>,  // Rotating input buffer
    ht:       Vec<u32>, // Context hash -> next byte in buf
    buf_end:  usize,    // Last index of buf (for rotating buffer)
    ht_end:   usize,    // Last index of ht  (for hashing)
}
impl MatchModel {
    pub fn new(n: usize) -> MatchModel {
        MatchModel {
            mch_ptr:  0,    hash_s:   0,
            mch_len:  0,    hash_l:   0,
            cxt:      1,    buf_pos:  0,
            bits:     0,    
            sm:       StateMap::new(56 << 8),
            buf:      vec![0; n / 2],
            ht:       vec![0; n / 8],
            buf_end:  (n / 2) - 1,
            ht_end:   (n / 8) - 1,
        }
    }
    pub fn p(&mut self, bit: i32, mxr: &mut Mixer) {
        self.update(bit);

        let mut cxt = self.cxt;

        // Get n bits of byte at buf[mch_ptr], where n is number of bits in cxt
        // i.e. cxt currently has 3 bits, so get 3 bits of buf[mch_ptr]
        let pr_cxt = ((self.buf[self.mch_ptr] as usize) + 256) >> (8 - self.bits);

        // If the new value of pr_cxt (containing the next "predicted" bit) doesn't
        // match the new value of cxt (containing the next actual bit), reset the match.
        if self.mch_len > 0 && pr_cxt == cxt {
            let pr_bit = (self.buf[self.mch_ptr] >> (7 - self.bits) & 1) as usize;

            // Create new context consisting of the match length, 
            // the next predicted bit, and the previous byte.
            if self.mch_len < 16 { cxt = self.mch_len * 2 + pr_bit; }
            else { cxt = (self.mch_len >> 2) * 2 + pr_bit + 24; }
            
            let prev_byte = self.buf[(self.buf_pos - 1) & self.buf_end];
            cxt = cxt * 256 + prev_byte as usize;
        } 
        else {
            self.mch_len = 0;
        }

        mxr.add(stretch(self.sm.p(bit, cxt as i32)));
    }
    pub fn update(&mut self, bit: i32) {
        // Update order-0 context
        self.cxt += self.cxt + bit as usize; 
        self.bits += 1;                      

        if self.bits == 8 { // Byte boundary
            self.update_long_hash(); 
            self.update_short_hash(); 

            // Add byte to buffer
            self.buf[self.buf_pos] = self.cxt as u8; 
            self.buf_pos += 1;            
            self.buf_pos &= self.buf_end; 

            self.bits = 0; 
            self.cxt = 1;

            if self.mch_len > 0 { 
                self.mch_ptr += 1;
                self.mch_ptr &= self.buf_end;
                if self.mch_len < MAX_LEN { self.mch_len += 1; }
            }
            else { // No current match, try long hash
                self.check_prev_bytes(self.hash_l);
            }

            if self.mch_len < 2 { // Less than 2 bytes match, try short hash
                self.mch_len = 0;
                self.check_prev_bytes(self.hash_s);
            }

            self.ht[self.hash_s] = self.buf_pos as u32;
            self.ht[self.hash_l] = self.buf_pos as u32;
        }
    }
    pub fn check_prev_bytes(&mut self, hash: usize) {
        // Map context hash to index in buffer
        self.mch_ptr = self.ht[hash] as usize; 

        if self.mch_ptr != self.buf_pos {
            // Byte before location indexed by hash (mch_ptr) - match length (mch_len)
            // i.e. if mch_ptr is 50 and mch_len is 3, prev_byte_h is 46 
            let mut prev_byte_h = (self.mch_ptr - self.mch_len - 1) & self.buf_end;
            // Byte before current byte - match length
            let mut prev_byte   = (self.buf_pos - self.mch_len - 1) & self.buf_end;

            // Check subsequent previous bytes, stopping at a mismatch
            while self.mch_len < MAX_LEN   
            && prev_byte_h != self.buf_pos 
            && self.buf[prev_byte] == self.buf[prev_byte_h] {
                self.mch_len += 1;
                prev_byte_h = (prev_byte_h - 1) & self.buf_end; 
                prev_byte   = (prev_byte   - 1) & self.buf_end;  
            }
        }
    }
    fn update_short_hash(&mut self) {
        self.hash_s = (self.hash_s * (5 << 5) + self.cxt) & self.ht_end;
    }
    fn update_long_hash(&mut self) {
        self.hash_l = (self.hash_l * (3 << 3) + self.cxt) & self.ht_end;
    }
    pub fn len(&self) -> usize {
        self.mch_len
    }
}
