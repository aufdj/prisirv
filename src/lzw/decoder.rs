use std::cmp::min;

use crate::lzw::{
    entry::Entry,
    cull::Cull,
};

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

struct Dictionary {
    entries:  Vec<Entry>,
    blk:      Vec<u8>,
    string:   Vec<u8>,
    code:     u32,
    cull:     Cull,
}
impl Dictionary {
    fn new(size: u32, cull: Cull) -> Dictionary {
        let mut dict = Dictionary {
            entries:  vec![Entry::default(); size as usize],
            blk:      Vec::new(),
            string:   Vec::new(),
            code:     1,
            cull,
        };
        for i in 0u8..=255 {
            dict.insert(dict.code, vec![i]);
        }
        dict.code += 3;
        dict
    }
    pub fn decompress(&mut self, blk_in: Vec<u8>) {
        let mut stream = BitStream::new(blk_in);
        loop { 
            if let Some(code) = stream.get_code() {
                match code {
                    DATA_END       => break,
                    CODE_LEN_UP    => {},
                    CODE_LEN_RESET => {},
                    _ => {
                        self.output_string(code);
                        self.check_code();
                    }
                }
            }
        } 
    }
    fn output_string(&mut self, code: u32) {
        let entry = self.get_entry(code);

        if entry.is_none() {
            self.string.push(self.string[0]);
            self.insert(code, self.string.clone());

            let entry = self.get_entry(code).unwrap();
            let string = self.entries[entry].string().to_vec();
            for byte in string.iter() {
                self.blk.push(*byte);
            }
            self.string = string;
        }
        else if !self.string.is_empty() {
            let string = self.entries[entry.unwrap()].string().to_vec();
            self.string.push(string[0]);
            self.insert(self.code, self.string.clone());

            for byte in string.iter() {
                self.blk.push(*byte);
            }
            self.string = string;
        }
        else {
            let string = self.entries[entry.unwrap()].string().to_vec();
            for byte in string.iter() {
                self.blk.push(*byte);
            }
            self.string = string;
        }
    }
    fn check_code(&mut self) {
        // if self.code % self.cull.interval == 0 {
        //     self.cull();
        //     self.stream.code_len = pow2(self.code).log2();
        // }

        if self.code >= self.entries.len() as u32 {
            self.reset();
        }
    }
    fn get_entry(&mut self, code: u32) -> Option<usize> {
        if !self.entries[code as usize].is_empty() {
            return Some(code as usize);
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
    // fn cull(&mut self) {
    //     let mut entries = self.entries
    //         .clone()
    //         .into_iter()
    //         .filter(|e| !e.is_empty() && e.code() > 259)
    //         .collect::<Vec<Entry>>();
    //     entries.sort_by(|a, b| a.code().cmp(&b.code()));

    //     self.reset();

    //     entries.retain_mut(|entry| !self.cull.cull(entry));

    //     for entry in entries.into_iter() {
    //         self.insert(self.code, entry.string().to_vec());
    //     }
    // }
}

pub fn decompress(blk_in: Vec<u8>, mem: usize) -> Vec<u8> {
    if blk_in.is_empty() {
        return Vec::new();
    }
    let size = mem as u32 / 4;
    let cull = Cull::settings(4000, 1, size - 0);
    let mut dict = Dictionary::new(size, cull);
    dict.decompress(blk_in);
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



// fn cull(&mut self) {
    //     self.code = 260;
    //     for entry in self.entries.iter_mut() {
    //         if entry.code() > 259 {
    //             if entry.count() < CLEAN_THRESHOLD 
    //             && entry.code() < (self.max_code - RECENCY_THRESHOLD) {
    //                 entry.clear();
    //             }
    //             else {
    //                 entry.set_code(self.code);
    //                 self.code += 1;
    //             }
    //         }
    //     }
    // }