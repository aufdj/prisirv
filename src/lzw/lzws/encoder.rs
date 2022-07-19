use crate::lzw::{
    code::CodeWriter,
    constant::{
        DATA_END,
        LEN_UP,
        RESET,
    },
};

struct HashTable {
    codes:         Vec<u32>,
    strings:       Vec<Vec<u8>>,
    prev:          usize,
    pub code:      u32,
    pub max_code:  u32,
}
impl HashTable {
    fn new(size: usize) -> HashTable {
        HashTable {
            codes:     vec![0; size],
            strings:   vec![Vec::new(); size],
            prev:      0,
            code:      1,
            max_code:  size as u32,
        }
    }
    // FNV-1a
    fn hash(&self, string: &[u8]) -> usize {
        let mut hash = 2166136261usize;
        for s in string.iter() {
            hash = hash * 16777619;
            hash = hash ^ *s as usize;
        }
        hash & (self.codes.len() - 1)
    }
    fn get(&mut self, string: &[u8]) -> Option<u32> {
        let hash = self.hash(string);
        
        if self.codes[hash] != 0 {
            // Verify that string hasn't been overwritten
            if self.strings[hash] == string {
                self.prev = hash;
                return Some(self.codes[hash]);
            }
        }
        // Pass hash in to avoid recomputing
        self.insert(string, hash);
        None
    }
    // Insert a new key-value pair into hash table if selected slot is 
    // empty, or if it is not empty but doesn't contain any strings of 
    // length 1. Because a new unseen string added to the hashtable may 
    // overwrite an existing value, a problem can occur if two values 
    // have the same hash.
    // 
    // If 'a' and 'aa' are both hashed to the same slot, the encoder could 
    // insert 'aa' into the hashtable, overwriting 'a', and then attempt to 
    // output the code for 'a', which is no longer in the table. To address
    // this, the previous hash is recorded and, if it equals the new hash, 
    // the current value is not replaced.
    fn insert(&mut self, string: &[u8], hash: usize) {
        if self.codes[hash] != 0 {
            if self.codes[hash] > 259 && hash != self.prev && string.len() < 31 {
                self.codes[hash] = self.code;
                self.strings[hash] = string.to_vec();
            }
        }
        else {
            if string.len() < 31 {
                self.codes[hash] = self.code;
                self.strings[hash] = string.to_vec();
            }
            
        }
        // Increment code unconditionally; in the event of a hash collision with 
        // first 256, increment code anyway to keep in sync with decoder.
        self.code += 1;
    }
    fn reset(&mut self) {
        // Skip code 0
        self.code = 1;
        for i in self.codes.iter_mut() {
            *i = 0;
        }
        for i in 0u8..=255 {
            let hash = self.hash(&[i]);

            assert!(self.codes[hash] == 0);
            self.codes[hash] = self.code;
            self.strings[hash] = vec![i];
            self.code += 1;
        }
        // Skip reserved codes
        self.code += 3;
    }
}

struct Dictionary {
    pub map:     HashTable,
    pub string:  Vec<u8>,
    pub stream:  CodeWriter,
}
impl Dictionary {
    fn new(byte: u8, mem: usize) -> Dictionary {
        let mut map = HashTable::new(mem/4);
        map.reset();
        
        Dictionary {
            map,
            string:  vec![byte],
            stream:  CodeWriter::new(),
        }
    }
    fn output_code(&mut self) {
        let last_char = self.string.pop().unwrap();
        
        self.stream.write(
            self.map.get(&self.string).unwrap()
        );

        self.string.clear();
        self.string.push(last_char);

        if self.map.code == 1 << self.stream.code_len {
            self.stream.write(LEN_UP);
        }

        if self.map.code >= self.map.max_code {
            self.stream.write(RESET);
            self.map.reset();
        }
    }
    fn output_last_code(&mut self) {
        if !self.string.is_empty() {
            self.stream.write(
                self.map.get(&self.string).unwrap()
            );
        }
        self.stream.write(DATA_END);
    }
    fn update_string(&mut self, byte: u8) {
        self.string.push(byte);
    }
    fn contains_string(&mut self) -> bool {
        self.map.get(&self.string).is_some()
    }
}


pub fn compress(blk_in: Vec<u8>, mem: usize) -> Vec<u8> {
    if blk_in.is_empty() { 
        return Vec::new();
    }
    let mut blk = blk_in.iter();
    let byte = blk.next().unwrap();
    let mut dict = Dictionary::new(*byte, mem);

    'c: loop {
        while dict.contains_string() {
            match blk.next() {
                Some(byte) => {
                    dict.update_string(*byte);
                }
                None => {
                    break 'c;
                }
            } 
        }
        dict.output_code();
    }
    dict.output_last_code();
    dict.stream.out
}