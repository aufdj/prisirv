use crate::statemap::StateMap;
use crate::predictor::next_state;


pub struct ContextModelO1 {
    cxt:        u32,
    pub o1cxt:  u32,
    bits:       usize,
    pub state:  *mut u8,
    sm:         StateMap,
    pub t0:     [u8; 65_536],
}
impl ContextModelO1 {
    pub fn new() -> ContextModelO1 {
        ContextModelO1 {
            cxt:    0,
            o1cxt:  0,
            bits:   0,
            state:  &mut 0,
            sm:     StateMap::new(256),
            t0:     [0; 65_536], 
        }
    }
    pub fn p(&mut self, bit: i32) -> i32 {
        unsafe { self.sm.p(bit, *self.state as i32) }
    }
    pub fn update(&mut self, bit: i32) {
        unsafe { *self.state = next_state(*self.state, bit); }

        self.cxt = (self.cxt << 1) + bit as u32;
        self.bits += 1;

        if self.bits == 8 {
            self.o1cxt = self.cxt << 8;
            self.cxt = 0;
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
    cxt:        u32,
    pub o2cxt:  u32,
    bits:       usize,
    cxt4:       u32,
    pub state:  *mut u8,
    sm:         StateMap,
}
impl ContextModelO2 {
    pub fn new() -> ContextModelO2 {
        ContextModelO2 {
            cxt:    0,
            o2cxt:  0,
            bits:   0,
            cxt4:   0,
            state:  &mut 0,
            sm:     StateMap::new(256),
        }
    }
    pub fn p(&mut self, bit: i32) -> i32 {
        unsafe { self.sm.p(bit, *self.state as i32) }
    }
    pub fn update(&mut self, bit: i32) {
        unsafe { *self.state = next_state(*self.state, bit); }

        self.cxt = (self.cxt << 1) + bit as u32;
        self.bits += 1;

        if self.bits == 8 {
            self.cxt4 = (self.cxt4 << 8) | self.cxt;
            self.o2cxt = (self.cxt4 & 0xFFFF) << 5 | 0x57000000;
            self.cxt = 0;
            self.bits = 0;
        }
        if self.bits > 0 {
            let j = ((bit as usize) + 1) << ((self.bits & 3) - 1);
            unsafe { self.state = self.state.add(j); }
        }
    }
}

pub struct ContextModelO3 {
    cxt:        u32,
    pub o3cxt:  u32,
    bits:       usize,
    cxt4:       u32,
    pub state:  *mut u8,
    sm:         StateMap,
}
impl ContextModelO3 {
    pub fn new() -> ContextModelO3 {
        ContextModelO3 {
            cxt:    0,
            o3cxt:  0,
            bits:   0,
            cxt4:   0,
            state:  &mut 0,
            sm:     StateMap::new(256),
        }
    }
    pub fn p(&mut self, bit: i32) -> i32 {
        unsafe { self.sm.p(bit, *self.state as i32) }
    }
    pub fn update(&mut self, bit: i32) {
        unsafe { *self.state = next_state(*self.state, bit); }

        self.cxt = (self.cxt << 1) + bit as u32;
        self.bits += 1;

        if self.bits == 8 {
            self.cxt4 = (self.cxt4 << 8) | self.cxt;
            self.o3cxt = (self.cxt4 << 8).wrapping_mul(3);
            self.cxt = 0;
            self.bits = 0;
        }
        if self.bits > 0 {
            let j = ((bit as usize) + 1) << ((self.bits & 3) - 1);
            unsafe { self.state = self.state.add(j); }
        }
    }
}

pub struct ContextModelO4 {
    cxt:        u32,
    pub o4cxt:  u32,
    bits:       usize,
    cxt4:       u32,
    pub state:  *mut u8,
    sm:         StateMap,
}
impl ContextModelO4 {
    pub fn new() -> ContextModelO4 {
        ContextModelO4 {
            cxt:    0,
            o4cxt:  0,
            bits:   0,
            cxt4:   0,
            state:  &mut 0,
            sm:     StateMap::new(256),
        }
    }
    pub fn p(&mut self, bit: i32) -> i32 {
        unsafe { self.sm.p(bit, *self.state as i32) }
    }
    pub fn update(&mut self, bit: i32) {
        unsafe { *self.state = next_state(*self.state, bit); }

        self.cxt = (self.cxt << 1) + bit as u32;
        self.bits += 1;

        if self.bits == 8 {
            self.cxt4 = (self.cxt4 << 8) | self.cxt;
            self.o4cxt = self.cxt4.wrapping_mul(5); 
            self.cxt = 0;
            self.bits = 0;
        }
        if self.bits > 0 {
            let j = ((bit as usize) + 1) << ((self.bits & 3) - 1);
            unsafe { self.state = self.state.add(j); }
        }
    }
}

pub struct ContextModelO6 {
    cxt:        u32,
    pub o6cxt:  u32,
    bits:       usize,
    cxt4:       u32,
    pub state:  *mut u8,
    sm:         StateMap,
}
impl ContextModelO6 {
    pub fn new() -> ContextModelO6 {
        ContextModelO6 {
            cxt:    0,
            o6cxt:  0,
            bits:   0,
            cxt4:   0,
            state:  &mut 0,
            sm:     StateMap::new(256),
        }
    }
    pub fn p(&mut self, bit: i32) -> i32 {
        unsafe { self.sm.p(bit, *self.state as i32) }
    }
    pub fn update(&mut self, bit: i32) {
        unsafe { *self.state = next_state(*self.state, bit); }

        self.cxt = (self.cxt << 1) + bit as u32;
        self.bits += 1;

        if self.bits == 8 {
            self.cxt4 = (self.cxt4 << 8) | self.cxt;
            self.o6cxt = (self.o6cxt.wrapping_mul(11 << 5) + self.cxt * 13) & 0x3FFFFFFF;
            self.cxt = 0;
            self.bits = 0;
        }
        if self.bits > 0 {
            let j = ((bit as usize) + 1) << ((self.bits & 3) - 1);
            unsafe { self.state = self.state.add(j); }
        }
    }
}