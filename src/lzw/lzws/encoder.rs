use crate::lzw::{
    code::CodeWriter,
    constant::{
        DATA_END,
        LEN_UP,
        RESET,
    },
};

struct Dictionary {
    codes:         Vec<u32>,
    strings:       Vec<Vec<u8>>,
    prev:          usize,
    pub code:      u32,
    pub max_code:  u32,
}
impl Dictionary {
    fn new(size: usize) -> Dictionary {
        let mut dict = Dictionary {
            codes:     vec![0; size],
            strings:   vec![Vec::new(); size],
            prev:      0,
            code:      1,
            max_code:  size as u32,
        };
        dict.reset();
        dict
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
        self.insert(string.to_vec(), hash);
        None
    }
    // Insert a new key-value pair into hash table if selected slot is 
    // empty, or if it is not empty but doesn't contain any strings of 
    // length 1. Because a new unseen string added to the dictionary may 
    // overwrite an existing value, a problem can occur if two values 
    // have the same hash.
    // 
    // If 'a' and 'aa' are both hashed to the same slot, the encoder could 
    // insert 'aa' into the dictionary, overwriting 'a', and then attempt to 
    // output the code for 'a', which is no longer in the table. To address
    // this, the previous hash is recorded and, if it equals the new hash, 
    // the current value is not replaced.
    fn insert(&mut self, string: Vec<u8>, hash: usize) {
        if self.codes[hash] != 0 {
            if self.codes[hash] > 259 && hash != self.prev && string.len() < 31 {
                self.codes[hash] = self.code;
                self.strings[hash] = string;
            }
        }
        else {
            if string.len() < 31 {
                self.codes[hash] = self.code;
                self.strings[hash] = string;
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


struct Encoder {
    dict:   Dictionary,
    string: Vec<u8>,
}
impl Encoder {
    fn new(mem: usize) -> Encoder {
        Encoder {
            dict:   Dictionary::new(mem/4),
            string: Vec::new(),
        }
    }
    fn compress(&mut self, blk_in: Vec<u8>) -> CodeWriter {
        let mut stream = CodeWriter::new();

        for byte in blk_in.iter() {
            self.string.push(*byte);

            if self.dict.get(&self.string).is_none() {
                stream.write(self.output_code());

                if self.dict.code == 1 << stream.code_len {
                    stream.write(LEN_UP);
                }
        
                if self.dict.code >= self.dict.max_code {
                    stream.write(RESET);
                    self.dict.reset();
                }
            }
        }

        if !self.string.is_empty() {
            stream.write(
                self.dict.get(&self.string).unwrap()
            );
        }
        stream.write(DATA_END);
        stream
    }
    fn output_code(&mut self) -> u32 {
        let last_char = self.string.pop().unwrap();
        let code = self.dict.get(&self.string).unwrap();

        self.string.clear();
        self.string.push(last_char);

        code
    }
}


pub fn compress(blk_in: Vec<u8>, mem: usize) -> Vec<u8> {
    if blk_in.is_empty() {
        return Vec::new();
    }
    Encoder::new(mem).compress(blk_in).out
}