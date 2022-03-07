/// # Hash Table
///
/// Two thirds of the memory (2 * 2^N MB) is used for a hash table mapping
/// the 6 regular contexts (orders 1-4, 6, word) to state arrays. A lookup
/// occurs every 4 bits. The input is a byte-oriented context plus possibly
/// the first nibble of the next byte. The output is an array of 15 bit
/// histories, or states, (1 byte each) for all possible contexts formed 
/// by appending 0..3 more bits. The table entries have the format:
/// 
///  [checksum, "", 0, 1, 00, 10, 01, 11, 000, 100, 010, 110, 001, 101, 011, 111]
/// 
/// The second byte is the bit history for the context ending on a nibble
/// boundary. It also serves as a priority for replacement. The states are 
/// ordered by increasing total count, where state 0 represents the initial 
/// state (no history). When a context is looked up, the 8 bit checksum 
/// (part of the hash) is compared with 3 adjacent entries, and if there 
/// is no match, the entry with the lowest priority is cleared and the new 
/// checksum is stored.
/// 
/// Given a 32-bit hash h of the context, 8 bits are used for the checksum 
/// and 17 + N bits are used for the index i. Then the entries i, i XOR 1, 
/// and i XOR 2 are tried. The hash h is actually a collision-free permut-
/// -ation, consisting of multiplying the input by a large odd number mod 
/// 2^32, a 16-bit rotate, and another multiply.
/// 
/// The order-1 context is mapped to a bit history using a 64K direct
/// lookup table, not a hash table.

/// State array length.
const B: usize = 16;

pub struct HashTable {
    t:     Vec<u8>, // Hash table mapping index to state array
    size:  usize,   // Size of hash table in bytes
}
impl HashTable {
    /// Create a new HashTable.
    pub fn new(n: usize) -> HashTable {
        assert!(B.is_power_of_two());
        assert!(n.is_power_of_two());
        assert!(n >= (B * 4)); 
        HashTable {
            t:     vec![0; n + B * 4 + 64],
            size:  n,
        }
    }

    /// Map context i to element 0 of state array. A state array is a set 
    /// of states corresponding to possible future contexts.
    pub fn hash(&mut self, mut i: u32) -> *mut u8 {
        i = i.wrapping_mul(123456791).rotate_right(16).wrapping_mul(234567891);
        let chksum = (i >> 24) as u8;
        let mut i = i as usize; // Convert to usize for indexing
        i = (i * B) & (self.size - B);

        if self.t[i]       == chksum { return &mut self.t[i];       }
        if self.t[i^B]     == chksum { return &mut self.t[i^B];     }
        if self.t[i^(B*2)] == chksum { return &mut self.t[i^(B*2)]; }

        if self.t[i+1] > self.t[(i+1)^B]
        || self.t[i+1] > self.t[(i+1)^(B*2)] { i ^= B; }

        if self.t[i+1] > self.t[(i+1)^B^(B*2)] { i ^= B ^ (B * 2); }

        for x in self.t[i..i+B].iter_mut() {
            *x = 0;
        }

        self.t[i] = chksum;
        &mut self.t[i]
    }
}
