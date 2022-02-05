use crate::{
    logistic::stretch,
    statemap::StateMap,
    mixer::Mixer
};


// Match Model ---------------------------------------------------------------------------------------------------------------- Match Model
// The "match" model (MatchModel) looks up the current context in a
// hash table, first using a longer context, then a shorter one.  If
// a match is found, then the following bits are predicted until there is
// a misprediction.  The prediction is computed by mapping the predicted
// bit, the length of the match (1..15 or quantized by 4 in 16..62, max 62),
// and the last whole byte as context into a StateMap.  If no match is found,
// then the order 0 context (last 0..7 bits of the current byte) is used
// as context to the StateMap.
// 
// One third of memory is used by MatchModel, divided equally between 
// a rotating input buffer of 2^(N+19) bytes and an index (hash table)
// with 2^(N+17) entries.  Two context hashes are maintained, a long one,
// h1, of length ceil((N+17)/3) bytes and a shorter one, h2, of length 
// ceil((N+17)/5) bytes, where ceil() is the ceiling function.  The index
// does not use collision detection.  At each byte boundary, if there is 
// not currently a match, then the bytes before the current byte are
// compared with the location indexed by h1.  If less than 2 bytes match,
// then h2 is tried.  If a match of length 1 or more is found, the
// match is maintained until the next bit mismatches the predicted bit.
// The table is updated at h1 and h2 after every byte.


const MAX_LEN: usize = 62;

pub struct MatchModel {
    mch_ptr:  usize,    
    mch_len:  usize,    
    cxt:      usize,    
    bits:     usize,  
    hash_s:   usize,
    hash_l:   usize,
    buf_pos:  usize,  
    sm:       StateMap,
    buf:      Vec<u8>,
    ht:       Vec<u32>,
    buf_end:  usize,
    ht_end:   usize,
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
    pub fn find_or_extend_match(&mut self, hash: usize) {
        self.mch_ptr = self.ht[hash] as usize;
        if self.mch_ptr != self.buf_pos {
            let mut i = self.mch_ptr - self.mch_len - 1 & self.buf_end;
            let mut j = self.buf_pos - self.mch_len - 1 & self.buf_end;

            while i != self.buf_pos 
            && self.mch_len < MAX_LEN 
            && self.buf[i] == self.buf[j] {
                self.mch_len += 1;
                i = (i - 1) & self.buf_end; 
                j = (j - 1) & self.buf_end;  
            }
        }
    }
    pub fn len(&self) -> usize {
        self.mch_len
    }
    pub fn p(&mut self, bit: i32, mxr: &mut Mixer) {
        self.update(bit);

        let mut cxt = self.cxt;

        let a = (self.buf[self.mch_ptr] as usize) + 256 >> (8 - self.bits);

        if self.mch_len > 0 && a == cxt {
            let b = (self.buf[self.mch_ptr] >> 7 - self.bits & 1) as usize;

            if self.mch_len < 16 { cxt = self.mch_len * 2 + b; }
            else { cxt = (self.mch_len >> 2) * 2 + b + 24; }
            
            cxt = cxt * 256 + self.buf[self.buf_pos-1 & self.buf_end] as usize;
        } 
        else {
            self.mch_len = 0;
        }

        mxr.add(stretch(self.sm.p(bit, cxt as i32)));

        if self.bits == 0 {
            self.ht[self.hash_s] = self.buf_pos as u32;
            self.ht[self.hash_l] = self.buf_pos as u32;
        }
    }
    pub fn update(&mut self, bit: i32) {
        self.cxt += self.cxt + bit as usize;
        self.bits += 1;
        if self.bits == 8 {
            self.bits = 0;
            self.hash_s = self.hash_s * (3 << 3) + self.cxt & self.ht_end;
            self.hash_l = self.hash_l * (5 << 5) + self.cxt & self.ht_end;
            self.buf[self.buf_pos] = self.cxt as u8;
            self.buf_pos += 1;
            self.cxt = 1;
            self.buf_pos &= self.buf_end;

            if self.mch_len > 0 {
                self.mch_ptr += 1;
                self.mch_ptr &= self.buf_end;
                if self.mch_len < MAX_LEN { self.mch_len += 1; }
            }
            else {
                self.find_or_extend_match(self.hash_s);
            }

            if self.mch_len < 2 {
                self.mch_len = 0;
                self.find_or_extend_match(self.hash_l);
            }
        }
    }
}
// ----------------------------------------------------------------------------------------------------------------------------------------
