
use std::fmt;


// Magic Number
pub const MAGIC: u32 = 0x5653_5250;


#[derive(Default, Debug, Clone, Copy)]
pub struct Version {
    pub major: u16,
    pub minor: u16,
    pub patch: u16,
}
impl Version {
    pub fn current() -> Version {
        Version {
            major: 0,
            minor: 2,
            patch: 0,
        }
    }
}
impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "v{}.{}.{}", self.major, self.minor, self.patch)
    }
}
impl Eq for Version {}
impl PartialEq for Version {
    fn eq(&self, other: &Self) -> bool {
        self.major == other.major 
        && self.minor == other.minor
    }
}