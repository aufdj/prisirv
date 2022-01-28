// Hash Table ------------------------------------------------------------------------------------------------------------------ Hash Table
const B: usize = 16;

pub struct HashTable {
    t:     Vec<u8>, // Hash table mapping index to state array
    size:  usize,   // Size of hash table in bytes
}
impl HashTable {
    pub fn new(n: usize) -> HashTable {
        assert!(B >= 2       && B.is_power_of_two());
        assert!(n >= (B * 4) && n.is_power_of_two());
        HashTable {
            t:     vec![0; n + B * 4 + 64],
            size:  n,
        }
    }
    // Maps context i to 2nd value of state array (" ")
    // A state array is a set of states corresponding to possible future contexts
    // [checksum, " ", 0, 1, 00, 10, 01, 11, 000, 100, 010, 110, 001, 101, 011, 111]
    pub fn hash(&mut self, mut i: u32) -> *mut u8 {
        i = i.wrapping_mul(123456791).rotate_right(16).wrapping_mul(234567891);
        let chksum = (i >> 24) as u8;
        let mut i = i as usize; // Convert to usize for indexing
        i = (i * B) & (self.size - B); // Restrict i to valid ht index
        if self.t[i]     == chksum { return &mut self.t[i];     }
        if self.t[i^B]   == chksum { return &mut self.t[i^B];   }
        if self.t[i^B*2] == chksum { return &mut self.t[i^B*2]; }
        if self.t[i+1] > self.t[i+1^B]
        || self.t[i+1] > self.t[i+1^B*2] { i ^= B; }
        if self.t[i+1] > self.t[i+1^B^B*2] { i ^= B ^ (B * 2); }
        for x in self.t[i..i+B].iter_mut() {
            *x = 0;
        }
        self.t[i] = chksum;
        &mut self.t[i]
    }
}
// ----------------------------------------------------------------------------------------------------------------------------------------
