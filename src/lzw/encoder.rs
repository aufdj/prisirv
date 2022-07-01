use std::cmp::min;

const DATA_END: u32 = 257;
const CODE_LEN_UP: u32 = 258;
const CODE_LEN_RESET: u32 = 259;

struct BitStream {
    pck:           u32,
    pck_len:       u32,
    pub out:       Vec<u8>,
    pub code_len:  u32,
}
impl BitStream {
    fn new() -> BitStream {
        BitStream {
            pck:       0,
            pck_len:   0,
            out:       Vec::new(),
            code_len:  9,
        }
    }
    fn write(&mut self, code: u32) {
        // Split code in two, assuming it crosses a packed code boundary.
        // If the entire code fits in the current packed code, codeu will
        // simply be 0. Otherwise, add the first part of the code (codel)
        // to the current packed code, output the packed code, reset it,
        // and add the remaining part of the code (codeu).
        let rem_len = 32 - self.pck_len;

        let codel = code & (0xFFFFFFFF >> self.pck_len);
        let codel_len = min(self.code_len, rem_len);

        let codeu = code >> codel_len;
        let codeu_len = self.code_len - codel_len;

        self.pck |= codel << self.pck_len;
        self.pck_len += codel_len;

        if self.pck_len == 32 {
            self.write_code(self.pck);
            self.pck = 0;
            self.pck_len = 0;
        }

        self.pck |= codeu << self.pck_len;
        self.pck_len += codeu_len;

        if self.pck_len == 32 {
            self.write_code(self.pck);
            self.pck = 0;
            self.pck_len = 0;
        }

        match code {
            CODE_LEN_UP => {
                self.code_len += 1;
            }
            CODE_LEN_RESET => {
                self.code_len = 9;
            }
            DATA_END => {
                self.write_code(self.pck);
            }
            _ => {},
        }
    }
    fn write_code(&mut self, code: u32) {
        self.out.push((code & 0xFF) as u8);
        self.out.push(((code >> 8) & 0xFF) as u8);
        self.out.push(((code >> 16) & 0xFF) as u8);
        self.out.push((code >> 24) as u8);
    }
}

#[repr(align(16))]
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
            hash = hash ^ *s as usize;
            hash = hash * 16777619;
        }
        hash & (self.codes.len() - 1)
    }
    fn get(&mut self, string: &[u8]) -> Option<u32> {
        let hash = self.hash(string);
        
        if self.codes[hash] != 0 {
            if self.strings[hash] == string {
                self.prev = hash;
                return Some(self.codes[hash]);
            }
            else {
                // Check adjacent slots
                for i in 1..16 {
                    let idx = (hash^i) % self.strings.len();
                    if self.strings[idx] == string {
                        self.prev = idx;
                        return Some(self.codes[idx]);
                    }
                }
            }
        }
        // Pass hash in to avoid recomputing.
        self.insert(string, hash);
        None
    }
    // Insert a new key-value pair into hash table if selected 
    // slot is empty. If slot is not empty, search up to 16 
    // adjacent slots. If no adjacent slots are empty, don't insert.
    fn insert(&mut self, string: &[u8], hash: usize) {
        if string.len() < 31 {
            if self.codes[hash] == 0 {
                self.codes[hash] = self.code;
                self.strings[hash] = string.to_vec();
            }
            else {
                if hash != self.prev {
                    // Check adjacent slots
                    for i in 1..16 {
                        let idx = (hash^i) % self.codes.len();
                        if self.codes[idx] == 0  {
                            self.codes[idx] = self.code;
                            self.strings[idx] = string.to_vec();
                            break;
                        }
                    }
                }
            }
        }
        
        // Increment code unconditionally; in the event no empty slot is
        // found, increment code anyway to keep in sync with decoder.
        self.code += 1;
    }
    fn reset(&mut self) {
        // Skip code 0.
        self.code = 1;
        for i in self.codes.iter_mut() {
            *i = 0;
        }
        for string in self.strings.iter_mut() {
            string.clear();
        }
        // Initialize dictionary with all strings of length 1.
        for i in 0u8..=255 {
            let hash = self.hash(&[i]);

            assert!(self.codes[hash] == 0);
            self.codes[hash] = self.code;
            self.strings[hash] = vec![i];
            self.code += 1;
        }
        // Skip reserved codes.
        self.code += 3;
    }
}

struct Dictionary {
    pub map:     HashTable,
    pub string:  Vec<u8>,
    pub stream:  BitStream,
}
impl Dictionary {
    fn new(byte: u8, mem: usize) -> Dictionary {
        let mut map = HashTable::new(mem/4);
        map.reset();
        
        Dictionary {
            map,
            string:  vec![byte],
            stream:  BitStream::new(),
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
            self.stream.write(CODE_LEN_UP);
        }

        if self.map.code >= self.map.max_code {
            self.stream.write(CODE_LEN_RESET);
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

