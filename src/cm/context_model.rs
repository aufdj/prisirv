use std::{
    cell::RefCell,
    rc::Rc,
};

use crate::{
    cm::statemap::StateMap,
    cm::predictor::next_state,
    cm::hash_table::HashTable,
};

type SharedHashTable = Rc<RefCell<HashTable>>;


pub struct ContextModelO1 {
    bits:       usize,
    pub cxt:    u32,
    pub o1cxt:  u32,
    pub state:  *mut u8,
    pub t0:     [u8; 65_536],
    sm:         StateMap,
}
impl ContextModelO1 {
    pub fn new() -> ContextModelO1 {
        ContextModelO1 {
            bits:   0,
            cxt:    1,
            o1cxt:  0,
            state:  &mut 0,
            t0:     [0; 65_536], 
            sm:     StateMap::new(256),
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
            self.o1cxt = self.cxt << 8;
            self.cxt = 1;
            self.bits = 0;
        }

        unsafe { 
            self.state = 
                ((&mut self.t0[0] as *mut u8)
                .add(self.o1cxt as usize))
                .add(self.cxt as usize);
        }
    }
}


pub struct ContextModelO2 {
    bits:       usize,
    cxt:        u32,
    cxt4:       u32,
    pub o2cxt:  u32,
    pub state:  *mut u8,
    sm:         StateMap,
    ht:         SharedHashTable,
}
impl ContextModelO2 {
    pub fn new(ht: SharedHashTable) -> ContextModelO2 {
        ContextModelO2 {
            bits:   0,
            cxt:    1,
            cxt4:   0,
            o2cxt:  0,
            state:  &mut 0,
            sm:     StateMap::new(256),
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
            self.cxt4 = (self.cxt4 << 8) | self.cxt;
            self.o2cxt = (self.cxt4 & 0xFFFF) << 5 | 0x57000000;
            unsafe { self.state = self.ht.borrow_mut().hash(self.o2cxt).add(1); }
            self.cxt = 1;
            self.bits = 0;
        }
        if self.bits == 4 {
            unsafe { self.state = self.ht.borrow_mut().hash(self.o2cxt + self.cxt).add(1); }
        }
        else if self.bits > 0 {
            let j = ((bit as usize) + 1) << ((self.bits & 3) - 1);
            unsafe { self.state = self.state.add(j); }
        }
    }
}

pub struct ContextModelO3 {
    bits:       usize,
    cxt:        u32,
    cxt4:       u32,
    pub o3cxt:  u32,
    pub state:  *mut u8,
    sm:         StateMap,
    ht:         SharedHashTable,
}
impl ContextModelO3 {
    pub fn new(ht: SharedHashTable) -> ContextModelO3 {
        ContextModelO3 {
            bits:   0,
            cxt:    1,
            cxt4:   0,
            o3cxt:  0,
            state:  &mut 0,
            sm:     StateMap::new(256),
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
            self.cxt4 = (self.cxt4 << 8) | self.cxt;
            self.o3cxt = (self.cxt4 << 8).wrapping_mul(3);
            unsafe { self.state = self.ht.borrow_mut().hash(self.o3cxt).add(1); }
            self.cxt = 1;
            self.bits = 0;
        }
        if self.bits == 4 {
            unsafe { self.state = self.ht.borrow_mut().hash(self.o3cxt + self.cxt).add(1); }
        }
        else if self.bits > 0 {
            let j = ((bit as usize) + 1) << ((self.bits & 3) - 1);
            unsafe { self.state = self.state.add(j); }
        }
    }
}

pub struct ContextModelO4 {
    bits:       usize,
    cxt:        u32,
    cxt4:       u32,
    pub o4cxt:  u32,
    pub state:  *mut u8,
    sm:         StateMap,
    ht:         SharedHashTable,
}
impl ContextModelO4 {
    pub fn new(ht: SharedHashTable) -> ContextModelO4 {
        ContextModelO4 {
            bits:   0,
            cxt:    1,
            cxt4:   0,
            o4cxt:  0,
            state:  &mut 0,
            sm:     StateMap::new(256),
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
            self.cxt4 = (self.cxt4 << 8) | self.cxt;
            self.o4cxt = self.cxt4.wrapping_mul(5); 
            unsafe { self.state = self.ht.borrow_mut().hash(self.o4cxt).add(1); }
            self.cxt = 1;
            self.bits = 0;
        }
        if self.bits == 4 {
            unsafe { self.state = self.ht.borrow_mut().hash(self.o4cxt + self.cxt).add(1); }
        }
        else if self.bits > 0 {
            let j = ((bit as usize) + 1) << ((self.bits & 3) - 1);
            unsafe { self.state = self.state.add(j); }
        }
    }
}

pub struct ContextModelO6 {
    bits:       usize,
    cxt:        u32,
    cxt4:       u32,
    pub o6cxt:  u32,
    pub state:  *mut u8,
    sm:         StateMap,
    ht:         SharedHashTable,
}
impl ContextModelO6 {
    pub fn new(ht: SharedHashTable) -> ContextModelO6 {
        ContextModelO6 {
            bits:   0,
            cxt:    1,
            cxt4:   0,
            o6cxt:  0,
            state:  &mut 0,
            sm:     StateMap::new(256),
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
            self.cxt4 = (self.cxt4 << 8) | self.cxt;
            self.o6cxt = (self.o6cxt.wrapping_mul(11 << 5) + self.cxt * 13) & 0x3FFFFFFF;
            unsafe { self.state = self.ht.borrow_mut().hash(self.o6cxt).add(1); }
            self.cxt = 1;
            self.bits = 0;
        }
        if self.bits == 4 {
            unsafe { self.state = self.ht.borrow_mut().hash(self.o6cxt + self.cxt).add(1); }
        }
        else if self.bits > 0 {
            let j = ((bit as usize) + 1) << ((self.bits & 3) - 1);
            unsafe { self.state = self.state.add(j); }
        }
    }
}