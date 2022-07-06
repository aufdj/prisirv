use crate::lzw::entry::Entry;

pub struct Cull {
    pub interval:   u32,
    pub threshold:  u32,
    pub recency:    u32,
}
impl Cull {
    pub fn settings(interval: u32, threshold: u32, recency: u32) -> Cull {
        Cull {
            interval,
            threshold,
            recency,
        }
    }
    pub fn cull(&self, entry: &Entry) -> bool {
        entry.count() < self.threshold &&
        entry.code()  < self.recency
    }
}