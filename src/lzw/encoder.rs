use std::cmp::min;

use crate::lzw::{
    entry::Entry,
    cull::{Cull, pow2},
};

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

struct Dictionary {
    entries:  Vec<Entry>,
    stream:   BitStream,
    string:   Vec<u8>,
    code:     u32,
    cull:     Cull,
    entry:    usize,
}
impl Dictionary {
    fn new(size: u32, cull: Cull) -> Dictionary {
        let mut dict = Dictionary {
            entries:  vec![Entry::default(); size as usize],
            stream:   BitStream::new(),
            string:   Vec::new(),
            code:     1,
            cull,
            entry:    0,
        };
        // Initialize dictionary with all strings of length 1.
        for i in 0u8..=255 {
            dict.insert(vec![i], dict.code);
        }
        // Skip reserved codes.
        dict.code += 3;
        dict
    }
    fn compress(&mut self, blk: Vec<u8>) {
        for byte in blk.iter() {
            self.update_string(*byte);

            if self.get_entry(&self.string).is_none() {
                self.cull.count += 1;

                self.insert(self.string.clone(), self.code);
                self.output_code();

                if self.code >= self.entries.len() as u32 {
                    self.stream.write(CODE_LEN_RESET);
                    self.reset();
                }

                // if self.cull.count == self.cull.interval {
                //     self.cull.count = 0;
                //     self.cull();
                //     self.stream.code_len = pow2(self.code).log2();
                // }
            
                if self.code == 1 << self.stream.code_len {
                    self.stream.write(CODE_LEN_UP);
                }
            }
        }

        if let Some(entry) = self.get_entry(&self.string) {
            self.entries[entry].increase_count();
            self.stream.write(self.entries[entry].code());
        }
        self.stream.write(DATA_END);
    }
    fn update_string(&mut self, byte: u8) {
        self.string.push(byte);
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
    fn output_code(&mut self) {
        let last_char = self.string.pop().unwrap();
        let entry = self.get_entry(&self.string).unwrap();
        self.entry = entry;
        self.entries[entry].increase_count();
        self.stream.write(self.entries[entry].code());

        self.string.clear();
        self.string.push(last_char);
    }
    fn get_entry(&self, string: &[u8]) -> Option<usize> {
        let hash = self.hash(&string);
        if !self.entries[hash].is_empty() {
            if self.entries[hash].string() == string {
                return Some(hash);
            }
            else {
                // Check adjacent slots
                for i in 1..16 {
                    let adj = (hash^i) % self.entries.len();
                    if self.entries[adj].string() == string {
                        return Some(adj);
                    }
                }
            }
        }
        None
    }
    // Insert a new entry into hash table if selected
    // slot is empty. If slot is not empty, search up to 16 
    // adjacent slots. If no adjacent slots are empty, don't insert.
    fn insert(&mut self, string: Vec<u8>, code: u32) {
        assert!(code != 0);
        let hash = self.hash(&string);
        if self.entries[hash].is_empty() {
            self.entries[hash] = Entry::new(code, string);
        }
        else {
            // Check adjacent slots
            for i in 1..16 {
                let adj = (hash^i) % self.entries.len();
                if self.entries[adj].is_empty() {
                    self.entries[adj] = Entry::new(code, string);
                    break;
                }
            }
        }
        self.code += 1;
    }
    fn reset(&mut self) {
        self.code = 260;
        for entry in self.entries.iter_mut() {
            if entry.code() > 259 {
                entry.clear();
            }
        }
        // println!("{:#?}", self.entries);
    }
    fn cull(&mut self) {
        let mut entries = self.entries
            .clone()
            .into_iter()
            .filter(|e| !e.is_empty() && e.code() > 259)
            .collect::<Vec<Entry>>();
        entries.sort_by(|a, b| a.code().cmp(&b.code()));

        // for entry in entries.iter() {
        //     println!("{}", entry.count());
        // }

        self.reset();

        entries.retain_mut(|entry| !self.cull.cull(entry));

        // println!("culled {} entries", len - entries.len());

        // for entry in entries.iter() {
        //     println!("{entry}");
        // }

        for entry in entries.iter() {
            self.insert(entry.string().to_vec(), self.code);
        }

        // let mut entries = self.entries
        //     .clone()
        //     .into_iter()
        //     .filter(|e| !e.is_empty() && e.code() > 259)
        //     .collect::<Vec<Entry>>();
        // entries.sort_by(|a, b| a.code().cmp(&b.code()));

        // for entry in entries.iter() {
        //     println!("{entry}");
        // }
    }
}


pub fn compress(blk_in: Vec<u8>, mem: usize) -> Vec<u8> {
    if blk_in.is_empty() {
        return Vec::new();
    }
    let size = mem as u32 / 4;
    let interval = 1 << 19;
    let cull = Cull::settings(interval, 1, (interval + 260) - 1);
    let mut dict = Dictionary::new(size, cull);
    dict.compress(blk_in);
    dict.stream.out
}
