use std::fmt;

#[derive(Default, Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub code:  u32,
    string:    Vec<u8>,
}
impl Entry {
    pub fn new(code: u32, string: Vec<u8>) -> Entry {
        Entry { 
            code, 
            string
        }
    }
    pub fn code(&self) -> u32 {
        self.code & 0x07FFFFFF 
    }
    pub fn count(&self) -> u32 {
        self.code >> 27
    }
    pub fn increase_count(&mut self) {
        if self.count() < 31 {
            self.code += 1 << 27;
        }
    }
    pub fn string(&self) -> &[u8] {
        &self.string
    }
    pub fn clear(&mut self) {
        self.code = 0;
        self.string.clear();
    }
    pub fn is_empty(&self) -> bool {
        self.code == 0
    }
}
impl fmt::Display for Entry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "
            \rCode:  {},
            \rCount: {},
            \r{:#?}",
            self.code(),
            self.count(),
            self.string
        )
    }
}