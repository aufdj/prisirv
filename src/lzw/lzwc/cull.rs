use crate::lzw::lzwc::entry::Entry;

pub struct Cull {
    pub min_count: u32,
    pub recent:    u32,
    pub max:       u32,
}
impl Cull {
    pub fn settings(min_count: u32, recent: u32, max: u32) -> Cull {
        Cull {
            min_count,
            recent,
            max,
        }
    }
    pub fn cull(&self, entry: &Entry) -> bool {
        entry.count() < self.min_count &&
        entry.code()  < self.recent &&
        entry.code()  > 259
    }
}