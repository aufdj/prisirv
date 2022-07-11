use crate::lzw::entry::Entry;

pub struct Cull {
    pub count:      u32,
    pub interval:   u32,
    pub threshold:  u32,
    pub recency:    u32,
}
impl Cull {
    pub fn settings(interval: u32, threshold: u32, recency: u32) -> Cull {
        Cull {
            count: 0,
            interval,
            threshold,
            recency,
        }
    }
    pub fn cull(&self, entry: &Entry) -> bool {
        if entry.count() < self.threshold 
        && entry.code()  < self.recency {
            true
        }
        else {
            false
        }
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