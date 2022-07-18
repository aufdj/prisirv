use std::cmp::min;

use crate::lzw::{
    entry::Entry,
    cull::Cull,
};

const DATA_END: u32 = 257;
const LEN_UP: u32 = 258;
const CULL: u32 = 259;

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
                assert!(code > 0);
                assert!(code < self.entries.len() as u32);
                match code {
                    DATA_END => {
                        break;
                    }
                    LEN_UP => {
                        stream.code_len += 1;
                    },
                    CULL => {
                        self.cull();
                        stream.code_len = (self.code+1).next_power_of_two().log2();
                    },
                    _ => {
                        self.output_string(code);
                    }
                }
            }
        } 
    }
    fn output_string(&mut self, code: u32) {
        if let Some(entry) = self.get_entry(code) {
            let string = entry.string().to_vec();
            
            if !self.string.is_empty() {
                self.string.push(string[0]);
                self.insert(self.code, self.string.clone());
            }

            for byte in string.iter() {
                self.blk.push(*byte);
            }

            self.string = string;
        }
        else {
            self.string.push(self.string[0]);
            self.insert(code, self.string.clone());

            let entry = self.get_entry(code).unwrap();
            let string = entry.string().to_vec();
            
            for byte in string.iter() {
                self.blk.push(*byte);
            }

            self.string = string;
        }
    }
    fn get_entry(&mut self, code: u32) -> Option<&Entry> {
        let entry = &mut self.entries[code as usize];
        if !entry.is_empty() {
            entry.increase_count();
            return Some(entry);
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
    fn cull(&mut self) {
        let mut entries = self.entries
            .clone()
            .into_iter()
            .filter(|e| !e.is_empty() && e.code() > 259)
            .collect::<Vec<Entry>>();
        entries.sort_by(|a, b| a.code().cmp(&b.code()));

        self.reset();

        entries.retain(|entry| !self.cull.cull(entry));

        for entry in entries.iter() {
            self.insert(self.code, entry.string().to_vec());
        }
    }
}

pub fn decompress(blk_in: Vec<u8>, mem: usize) -> Vec<u8> {
    if blk_in.is_empty() {
        return Vec::new();
    }
    let size = mem as u32 / 4;
    let max = (size as f64 * 0.4) as u32;
    let cull = Cull::settings(1, max - 1, max);
    let mut dict = Dictionary::new(size, cull);
    dict.decompress(blk_in);
    dict.blk
}