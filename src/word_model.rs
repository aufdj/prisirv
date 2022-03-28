use crate::statemap::StateMap;
use crate::predictor::next_state;

pub struct WordModel {
    cxt:           u32,
    bits:          usize,
    pub word_cxt:  u32,
    pub state:     *mut u8,
    sm:            StateMap,
}
impl WordModel {
    pub fn new() -> WordModel {
        WordModel {
            cxt:       0,
            bits:      0,
            word_cxt:  0,
            state:     &mut 0,
            sm:        StateMap::new(256),
        }
    }
    pub fn p(&mut self, bit: i32) -> i32 {
        unsafe { self.sm.p(bit, *self.state as i32) }
    }
    pub fn update(&mut self, bit: i32) {
        unsafe { *self.state = next_state(*self.state, bit); }

        self.cxt += self.cxt + bit as u32;
        self.bits += 1;

        if self.bits == 8 {
            self.word_cxt = match self.cxt { // Unigram Word Order
                65..=90 => {
                    self.cxt += 32; // Fold to lowercase
                    (self.word_cxt + self.cxt).wrapping_mul(7 << 3)
                },
                97..=122 => {
                    (self.word_cxt + self.cxt).wrapping_mul(7 << 3)
                },
                _ => 0,
            };
            self.cxt = 0;
            self.bits = 0;
        }
        if self.bits > 0 {
            let j = ((bit as usize) + 1) << ((self.bits & 3) - 1);
            unsafe { self.state = self.state.add(j); }
        }
    }
}