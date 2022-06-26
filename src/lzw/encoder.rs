use std::collections::HashMap;
use std::cmp::min;

const DATA_END: u32 = 256;
const CODE_LEN_UP: u32 = 257;
const CODE_LEN_RESET: u32 = 258;

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

        let codeu = code >> (codel_len % 32);
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

        if code == CODE_LEN_UP { 
            self.code_len += 1; 
        }
        if code == CODE_LEN_RESET { 
            self.code_len = 9;  
        }

        if code == DATA_END {
            self.write_code(self.pck);
        }
    }
    fn write_code(&mut self, code: u32) {
        self.out.push((code & 0xFF) as u8);
        self.out.push(((code >> 8) & 0xFF) as u8);
        self.out.push(((code >> 16) & 0xFF) as u8);
        self.out.push((code >> 24) as u8);
    }
}

struct Dictionary {
    pub map:       HashMap<Vec<u8>, u32>,
    pub max_code:  u32,
    pub code:      u32,
    pub string:    Vec<u8>,
    pub stream:    BitStream,
}
impl Dictionary {
    fn new(byte: u8, mem: usize) -> Dictionary {
        let mut map = HashMap::with_capacity(mem/4);
        for i in 0..256 { 
            map.insert(vec![i as u8], i);
        }

        Dictionary {
            map,
            max_code:  (mem/4) as u32,
            code:      259,
            string:    vec![byte],
            stream:    BitStream::new(),
        }
    }
    fn reset(&mut self) {
        self.code = 259;
        self.map.clear(); 
        for i in 0..256 {
            self.map.insert(vec![i as u8], i);
        }
    }
    fn output_code(&mut self) {
        if self.code <= self.max_code {
            self.map.insert(self.string.clone(), self.code);
            self.code += 1;  
        }
        
        let last_char = self.string.pop().unwrap();

        self.stream.write(
            *self.map.get(&self.string).unwrap()
        );

        self.string.clear();
        self.string.push(last_char); 

        if self.code == 1 << self.stream.code_len {
            self.stream.write(CODE_LEN_UP);
        }

        if self.code >= self.max_code {
            self.stream.write(CODE_LEN_RESET);
            self.reset();
        }
    }
    fn output_last_code(&mut self) {
        if !self.string.is_empty() {
            self.stream.write(
                *self.map.get(&self.string).unwrap()
            );
        }
        self.stream.write(DATA_END);
    }
    fn update_string(&mut self, byte: u8) {
        self.string.push(byte);
    }
    fn contains_string(&self) -> bool {
        self.map.contains_key(&self.string)
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

// use std::cmp::min;

// const DATA_END: u32 = 257;
// const CODE_LEN_UP: u32 = 258;
// const CODE_LEN_RESET: u32 = 259;

// struct BitStream {
//     pck:           u32,
//     pck_len:       u32,
//     pub out:       Vec<u8>,
//     pub code_len:  u32,
// }
// impl BitStream {
//     fn new() -> BitStream {
//         BitStream {
//             pck:       0,
//             pck_len:   0,
//             out:       Vec::new(),
//             code_len:  9,
//         }
//     }
//     fn write(&mut self, code: u32) {
//         // Split code in two, assuming it crosses a packed code boundary.
//         // If the entire code fits in the current packed code, codeu will
//         // simply be 0. Otherwise, add the first part of the code (codel)
//         // to the current packed code, output the packed code, reset it,
//         // and add the remaining part of the code (codeu).
//         let rem_len = 32 - self.pck_len;

//         let codel = code & (0xFFFFFFFF >> self.pck_len);
//         let codel_len = min(self.code_len, rem_len);

//         let codeu = code >> (codel_len % 32);
//         let codeu_len = self.code_len - codel_len;

//         self.pck |= codel << self.pck_len;
//         self.pck_len += codel_len;

//         if self.pck_len == 32 {
//             self.write_code(self.pck);
//             self.pck = 0;
//             self.pck_len = 0;
//         }

//         self.pck |= codeu << self.pck_len;
//         self.pck_len += codeu_len;

//         if self.pck_len == 32 {
//             self.write_code(self.pck);
//             self.pck = 0;
//             self.pck_len = 0;
//         }

//         // match code {
//         //     CODE_LEN_UP    => self.code_len += 1,
//         //     CODE_LEN_RESET => self.code_len = 9,
//         //     DATA_END       => self.write_code(self.pck),
//         //     _ => {}
//         // }
//         if code == CODE_LEN_UP { 
//             self.code_len += 1; 
//         }
//         if code == CODE_LEN_RESET { 
//             self.code_len = 9;  
//         }
//         if code == DATA_END {
//             self.write_code(self.pck);
//         }
//     }
//     fn write_code(&mut self, code: u32) {
//         self.out.push((code & 0xFF) as u8);
//         self.out.push(((code >> 8) & 0xFF) as u8);
//         self.out.push(((code >> 16) & 0xFF) as u8);
//         self.out.push((code >> 24) as u8);
//     }
// }


// struct HashTable {
//     map:  Vec<u32>,
//     keys: Vec<Vec<u8>>,
//     prev: usize,
// }
// impl HashTable {
//     fn new(size: usize) -> HashTable {
//         HashTable {
//             map:  vec![0; size],
//             keys: vec![Vec::new(); size],
//             prev: 0,
//         }
//     }
//     fn hash(&self, string: &[u8]) -> usize {
//         let mut hash = 2166136261usize;
//         for s in string.iter() {
//             hash = hash * 16777619;
//             hash = hash ^ *s as usize;
//         }
//         hash &= self.map.len() - 1;
//         hash
//     }
//     fn get(&mut self, string: &[u8]) -> Option<u32> {
//         let hash = self.hash(string);
//         self.prev = hash;

//         if self.map[hash] != 0 {
//             if &self.keys[hash] == &string {
//                 return Some(self.map[hash]);
//             }
//         }
//         None
//     }
//     // Insert a new key-value pair into hash table. Because a new unseen 
//     // string added to the hashtable may overwrite an existing value, a 
//     // problem can occur if two values have the same hash.
//     // 
//     // If 'a' and 'aa' are both hashed to the same slot, the encoder could 
//     // insert 'aa' into the hashtable, overwriting 'a', and then attempt to 
//     // output the code for 'a', which no longer exists. 
//     fn insert(&mut self, string: &[u8], code: u32) {
//         let hash = self.hash(string);
        
//         // If slot is not empty, only overwrite if 
//         // existing code is not one of the first 259.
//         if self.map[hash] != 0 {
//                                     // See problem above
//             if self.map[hash] > 259 /*&& hash != self.prev*/ {
//                 self.map[hash] = code;
//                 self.keys[hash] = string.to_vec();
//             }
//         }
//         // If slot is empty, insert new key-value pair.
//         else {
//             self.map[hash] = code;
//             self.keys[hash] = string.to_vec();
//         }
//     }
//     fn init(&mut self, string: &[u8], code: u32) {
//         let hash = self.hash(string);

//         self.map[hash] = code;
//         self.keys[hash] = string.to_vec();
//     }
//     fn reset(&mut self) {
//         for i in self.map.iter_mut() {
//             *i = 0;
//         }
//         for vec in self.keys.iter_mut() {
//             vec.clear();
//         }
//         for i in 0u8..=255 {
//             self.init(&[i], i as u32 + 1);
//         }
//     }
// }

// struct Dictionary {
//     pub map:       HashTable,
//     pub max_code:  u32,
//     pub code:      u32,
//     pub string:    Vec<u8>,
//     pub stream:    BitStream,
// }
// impl Dictionary {
//     fn new(byte: u8, mem: usize) -> Dictionary {
//         let mut map = HashTable::new(mem/4);
//         map.reset();
        
//         Dictionary {
//             map,
//             max_code:  (mem/4) as u32,
//             code:      260,
//             string:    vec![byte],
//             stream:    BitStream::new(),
//         }
//     }
//     fn reset(&mut self) {
//         self.code = 260;
//         self.map.reset();
//     }
//     fn output_code(&mut self) {
//         if self.code <= self.max_code {
//             self.map.insert(&self.string, self.code);
//             self.code += 1;  
//         }
        
//         let last_char = self.string.pop().unwrap();
        
//         self.stream.write(
//             self.map.get(&self.string).unwrap()
//         );

//         self.string.clear();
//         self.string.push(last_char);

//         if self.code == 1 << self.stream.code_len {
//             self.stream.write(CODE_LEN_UP);
//         }

//         if self.code >= self.max_code {
//             self.stream.write(CODE_LEN_RESET);
//             self.reset();
//         }
//     }
//     fn output_last_code(&mut self) {
//         if !self.string.is_empty() {
//             self.stream.write(
//                 self.map.get(&self.string).unwrap()
//             );
//         }
//         self.stream.write(DATA_END);
//     }
//     fn update_string(&mut self, byte: u8) {
//         self.string.push(byte);
//     }
//     fn contains_string(&mut self) -> bool {
//         self.map.get(&self.string).is_some()
//     }
// }


// pub fn compress(blk_in: Vec<u8>, mem: usize) -> Vec<u8> {
//     if blk_in.is_empty() { 
//         return Vec::new();
//     }
//     let mut blk = blk_in.iter();
//     let byte = blk.next().unwrap();
//     let mut dict = Dictionary::new(*byte, mem);

//     'c: loop {
//         while dict.contains_string() {
//             match blk.next() {
//                 Some(byte) => {
//                     dict.update_string(*byte);
//                 }
//                 None => {
//                     break 'c;
//                 }
//             } 
//         }
//         dict.output_code();
//     }
//     dict.output_last_code();
//     dict.stream.out
// }

