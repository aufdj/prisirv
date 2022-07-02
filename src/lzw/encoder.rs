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

#[derive(Default, Clone)]
struct Entry {
    code:    u32,
    string:  Vec<u8>,
}
impl Entry {
    fn new(code: u32, string: Vec<u8>) -> Entry {
        Entry { 
            code, 
            string
        }
    }
    fn code(&self) -> u32 {
        self.code & 0x07FFFFFF 
    }
    fn count(&self) -> u32 {
        self.code >> 27
    }
    fn count_up(&mut self) {
        if self.count() < 31 {
            self.code += 1 << 27;
        }
    }
    fn string(&self) -> &[u8] {
        &self.string
    }
    fn clear(&mut self) {
        self.code = 0;
        self.string.clear();
    }
    // fn set_code(&mut self, code: u32) {
    //     self.code = code;
    // }
    fn is_empty(&self) -> bool {
        self.code == 0
    }
}

#[repr(align(64))]
struct HashTable {
    entries:       Vec<Entry>,
    pub code:      u32,
    pub max_code:  u32,
}
impl HashTable {
    fn new(size: usize) -> HashTable {
        HashTable {
            entries:   vec![Entry::default(); size],
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
        hash & (self.entries.len() - 1)
    }
    fn get(&mut self, string: &[u8]) -> Option<u32> {
        let hash = self.hash(string);
        
        if !self.entries[hash].is_empty() {
            if self.entries[hash].string() == string {
                self.entries[hash].count_up();
                return Some(self.entries[hash].code());
            }
            else {
                // Check adjacent slots
                for i in 1..16 {
                    let adj = (hash^i) % self.entries.len();
                    if self.entries[adj].string() == string {
                        self.entries[adj].count_up();
                        return Some(self.entries[adj].code());
                    }
                }
            }
        }
        // Pass hash in to avoid recomputing.
        self.insert(string, hash);
        None
    }
    // Insert a new entry into hash table if selected
    // slot is empty. If slot is not empty, search up to 16 
    // adjacent slots. If no adjacent slots are empty, don't insert.
    fn insert(&mut self, string: &[u8], hash: usize) {
        if self.entries[hash].is_empty() {
            self.entries[hash] = Entry::new(self.code, string.to_vec());
        }
        else {
            // Check adjacent slots
            for i in 1..16 {
                let adj = (hash^i) % self.entries.len();
                if self.entries[adj].is_empty() {
                    self.entries[adj] = Entry::new(self.code, string.to_vec());
                    break;
                }
            }
        }
    
        // Increment code unconditionally; in the event no empty slot is
        // found, increment code anyway to keep in sync with decoder.
        self.code += 1;
    }
    fn init(&mut self) {
        // Initialize dictionary with all strings of length 1.
        for i in 0u8..=255 {
            let hash = self.hash(&[i]);
            self.entries[hash] = Entry::new(self.code, vec![i]);
            self.code += 1;
        }
        // Skip reserved codes.
        self.code += 3;
    }
    fn reset(&mut self) {
        self.code = 260;
        for entry in self.entries.iter_mut() {
            if entry.code() > 259 {
                entry.clear();
            }
        }
    }
    // fn clean(&mut self) {
    //     self.code = 260;
    //     for entry in self.entries.iter_mut() {
    //         if entry.code() > 259 {
    //             if entry.count() < 1 && entry.code() < (self.max_code - 512) {
    //                 entry.clear();
    //             }
    //             else {
    //                 entry.set_code(self.code);
    //                 self.code += 1;
    //             }
    //         }
    //     }
    // }
}

// fn pow2(x: u32) -> u32 {
//     let mut y = x + 1;
//     y |= y >> 1;
//     y |= y >> 2;
//     y |= y >> 4;
//     y |= y >> 8;
//     y |= y >> 16;
//     y + 1
// }

struct Dictionary {
    pub map:     HashTable,
    pub string:  Vec<u8>,
    pub stream:  BitStream,
}
impl Dictionary {
    fn new(byte: u8, mem: usize) -> Dictionary {
        let mut map = HashTable::new(mem/4);
        map.init();
        
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

        // if self.map.code % 524288 == 0 {
        //     self.map.clean();
        //     self.stream.code_len = pow2(self.map.code).log2();
        // }

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

