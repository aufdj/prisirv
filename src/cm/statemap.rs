/// # State Map 
///
/// A bit history (state) is mapped to a probability using an adaptive table
/// (StateMap). Each table entry has a 22-bit probability (initially p = 0.5) 
/// and 10-bit count (initially n = 0) packed into 32 bits.  After bit y is 
/// predicted, n is incrementedup to the limit (1023) and the probability is 
/// adjusted by p := p + (y - p)/(n + 0.5).  This model is stationary: 
/// p = (n1 + 0.5)/(n + 1), where n1 is the number of times y = 1 out of n.


#[allow(overflowing_literals)]
const PR_MSK: i32 = 0xFFFFFC00; // High 22 bit mask
const LIMIT: usize = 127; // Controls rate of adaptation (higher = slower) (0..1024)

/// A Statemap is used in an indirect context model to map a context to a 
/// state (a 1 byte representation of 0 and 1 counts), which is then mapped 
/// to a prediction. 
#[derive(Clone)]
pub struct StateMap {
    cxt:      usize,    // Context of last prediction
    cxt_map:  Vec<u32>, // Maps a context to a prediction and a count
    rec_t:    Vec<u16>, // Reciprocal table: controls adjustment to cxt_map
}
impl StateMap {
    /// Create a new StateMap.
    pub fn new(n: usize) -> StateMap {
        StateMap {
            cxt:      0,
            cxt_map:  vec![1 << 31; n],
            rec_t:    (0..1024).map(|i| 16_384/(i+i+3)).collect(),
        }
    }

    /// Maps a context, usually a state, to a prediction.
    pub fn p(&mut self, bit: i32, cxt: i32) -> i32 {
        assert!(bit == 0 || bit == 1);
        self.update(bit);
        self.cxt = cxt as usize;
        (self.cxt_map[self.cxt] >> 20) as i32
    }

    /// Update mapping based on prediction error.
    pub fn update(&mut self, bit: i32) {
        let count = (self.cxt_map[self.cxt] & 1023) as usize; // Low 10 bits
        let pr    = (self.cxt_map[self.cxt] >> 10 ) as i32;   // High 22 bits

        if count < LIMIT { self.cxt_map[self.cxt] += 1; }

        // Update cxt_map based on prediction error
        let pr_err = ((bit << 22) - pr) >> 3; // Prediction error
        let rec_v = self.rec_t[count] as i32; // Reciprocal value
        self.cxt_map[self.cxt] = 
        self.cxt_map[self.cxt].wrapping_add(((pr_err * rec_v) & PR_MSK) as u32);
    }
}
