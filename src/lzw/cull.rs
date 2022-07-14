use crate::lzw::entry::Entry;

pub struct Cull {
    pub threshold:  u32,
    pub recency:    u32,
    pub max:        u32,
}
impl Cull {
    pub fn settings(threshold: u32, recency: u32, max: u32) -> Cull {
        Cull {
            threshold,
            recency,
            max,
        }
    }
    pub fn cull(&self, entry: &Entry) -> bool {
        entry.count() < self.threshold && 
        entry.code()  < self.recency
    }
}

pub fn pow2(x: u32) -> u32 {
    let mut y = x + 1;
    y |= y >> 1;
    y |= y >> 2;
    y |= y >> 4;
    y |= y >> 8;
    y |= y >> 16;
    y + 1
}