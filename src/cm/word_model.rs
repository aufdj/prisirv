use std::{
    cell::RefCell,
    rc::Rc,
};

use crate::{
    cm::statemap::StateMap,
    cm::predictor::next_state,
    cm::hash_table::HashTable,
};


pub struct WordModel {
    cxt:           u32,
    bits:          usize,
    pub word_cxt:  u32,
    pub state:     *mut u8,
    sm:            StateMap,
    ht:            Rc<RefCell<HashTable>>,
}
impl WordModel {
    pub fn new(ht: Rc<RefCell<HashTable>>) -> WordModel {
        WordModel {
            cxt:       1,
            bits:      0,
            word_cxt:  0,
            state:     &mut 0,
            sm:        StateMap::new(256),
            ht,
        }
    }

    pub fn p(&mut self, bit: i32) -> i32 {
        self.update(bit);
        unsafe { self.sm.p(bit, *self.state as i32) }
    }

    pub fn update(&mut self, bit: i32) {
        unsafe { *self.state = next_state(*self.state, bit); }

        self.cxt = (self.cxt << 1) + bit as u32;
        self.bits += 1;

        if self.cxt >= 256 {
            self.cxt -= 256;
            self.word_cxt = match self.cxt {
                65..=90 => {
                    self.cxt += 32; // Fold to lowercase
                    (self.word_cxt + self.cxt).wrapping_mul(7 << 3)
                },
                97..=122 => {
                    (self.word_cxt + self.cxt).wrapping_mul(7 << 3)
                },
                _ => 0,
            };
            unsafe { self.state = self.ht.borrow_mut().hash(self.word_cxt).add(1); }
            self.cxt = 1;
            self.bits = 0;
        }
        if self.bits == 4 {
            unsafe { self.state = self.ht.borrow_mut().hash(self.word_cxt + self.cxt).add(1); }
        }
        else if self.bits > 0 {
            let j = ((bit as usize) + 1) << ((self.bits & 3) - 1);
            unsafe { self.state = self.state.add(j); }
        }
    }
}