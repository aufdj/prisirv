use std::cmp::min;

const DATA_END: u32 = 257;
const CODE_LEN_UP: u32 = 258;
const CODE_LEN_RESET: u32 = 259;

struct BitStream {
    stream:        Box<dyn Iterator<Item = u8>>,
    pub code_len:  u32,
    code:          u32,
    count:         u32,
}
impl BitStream {
    fn new(blk_in: Vec<u8>) -> BitStream {
        BitStream {
            stream:    Box::new(blk_in.into_iter()),
            code_len:  9,
            code:      0,
            count:     0,
        }
    }
    fn get_code(&mut self) -> Option<u32> {
        loop {
            match self.stream.next() {
                Some(byte) => {
                    let rem_len = self.code_len - self.count;

                    let codel = byte as u32 & ((1 << rem_len) - 1);
                    let codel_len = min(8, rem_len);

                    let codeu = byte as u32 >> codel_len;
                    let codeu_len = 8 - codel_len;

                    if self.count == self.code_len {
                        if self.code == CODE_LEN_UP { 
                            self.code_len += 1; 
                        }
                        if self.code == CODE_LEN_RESET { 
                            self.code_len = 9;  
                        }
                        let out = self.code;
                        self.code = 0;
                        self.count = 0;
                        self.code |= codel;
                        self.count += codel_len;
                        return Some(out);
                    }
                    else {
                        self.code |= codel << self.count;
                        self.count += codel_len;
                    }

                    if self.count == self.code_len {
                        if self.code == CODE_LEN_UP {
                            self.code_len += 1;
                        }
                        if self.code == CODE_LEN_RESET {
                            self.code_len = 9;
                        }
                        let out = self.code;
                        self.code = 0;
                        self.count = 0;
                        self.code |= codeu;
                        self.count += codeu_len;
                        return Some(out);
                    }
                    else {
                        self.code |= codeu << self.count;
                        self.count += codeu_len;
                    }
                }
                None => return None,
            }
        }
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

struct HashTable {
    entries:      Vec<Entry>,
    pub code:     u32,
    pub max_code: u32,
}
impl HashTable {
    fn new(size: usize) -> HashTable {
        HashTable {
            entries:   vec![Entry::default(); size],
            code:      1,
            max_code:  size as u32,
        }
    }
    fn get(&mut self, code: u32) -> Option<&[u8]> {
        let code = code as usize;

        if !self.entries[code].is_empty() {
            self.entries[code].count_up();
            return Some(&self.entries[code].string());
        }
        None
    }
    fn insert(&mut self, code: u32, string: Vec<u8>) {
        self.entries[code as usize] = Entry::new(code, string);
        self.code += 1;
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
    //             if entry.count() < 4 && entry.code() < (self.max_code - 1024) {
    //                 entry.clear();
    //             }
    //             else {
    //                 entry.set_code(self.code);
    //                 self.code += 1;
    //             }
    //         }
    //     }
    // }
    fn init(&mut self) {
        for i in 0u8..=255 {
            self.insert(self.code, vec![i]);
        }
        // Skip reserved codes
        self.code += 3;
    }
}

struct Dictionary {
    pub map:     HashTable,
    pub string:  Vec<u8>,
    pub blk:     Vec<u8>,
    pub stream:  BitStream,
}
impl Dictionary {
    fn new(mem: usize, blk_in: Vec<u8>) -> Dictionary {
        let mut map = HashTable::new(mem/4);
        map.init();

        Dictionary {
            map,
            string:  Vec::new(),
            blk:     Vec::new(),
            stream:  BitStream::new(blk_in),
        }
    }
    pub fn decompress(&mut self) {
        loop { 
            if let Some(code) = self.stream.get_code() {
                if code == DATA_END { 
                    break; 
                }
                if code != CODE_LEN_UP 
                && code != CODE_LEN_RESET {
                    self.output_string(code);
                }
            }
        } 
    }
    fn output_string(&mut self, code: u32) {
        let string = self.map.get(code);
        if string.is_none() {
            self.string.push(self.string[0]);
            self.map.insert(code, self.string.clone());
        }
        else if !self.string.is_empty() {
            self.string.push((string.unwrap())[0]);
            self.map.insert(self.map.code, self.string.clone());
        }
        
        let string = self.map.get(code).unwrap();
        for byte in string.iter() {
            self.blk.push(*byte);
        }

        self.string = string.to_vec();

        // if self.map.code % 524288 == 0 {
        //     self.map.clean();
        //     self.stream.code_len = pow2(self.map.code).log2();
        // }

        if self.map.code >= self.map.max_code {
            self.map.reset();
        }
    }
}

pub fn decompress(blk_in: Vec<u8>, mem: usize) -> Vec<u8> {
    if blk_in.is_empty() {
        return Vec::new();
    }
    let mut dict = Dictionary::new(mem, blk_in);
    dict.decompress();
    dict.blk
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