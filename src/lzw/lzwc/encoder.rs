use crate::lzw::{
    code::CodeWriter,
    lzwc::{ 
        entry::Entry,
        cull::Cull,
    },
    constant::{
        DATA_END,
        LEN_UP,
        CULL,
    }
};

const PROBE_DIST: usize = 128;

struct Dictionary {
    entries:  Vec<Entry>,
    code:     u32,
    cull:     Cull,
}
impl Dictionary {
    fn new(size: usize, cull: Cull) -> Dictionary {
        let mut dict = Dictionary {
            entries:  vec![Entry::default(); size],
            code:     1,
            cull,
        };
        // Initialize dictionary with all strings of length 1.
        for i in 0u8..=255 {
            dict.insert(vec![i], dict.code);
        }
        // Skip reserved codes.
        dict.code += 4;
        dict
    }
    // FNV-1a
    fn hash(&self, string: &[u8]) -> usize {
        let mut hash = 2166136261usize;
        for s in string.iter() {
            hash = hash ^ *s as usize;
            hash = hash.wrapping_mul(16777619);
        }
        hash & (self.entries.len() - 1)
    }
    fn get_entry(&mut self, string: &[u8]) -> Option<&mut Entry> {
        let hash = self.hash(string);

        if !self.entries[hash].is_empty() {
            if self.entries[hash].string() == string {
                return Some(&mut self.entries[hash]);
            }
            else {
                // Check adjacent slots
                for i in 1..PROBE_DIST {
                    let adj = (hash^i) % self.entries.len();
                    if self.entries[adj].string() == string {
                        return Some(&mut self.entries[adj]);
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
            for i in 1..PROBE_DIST {
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
        self.code = 261;
        for entry in self.entries.iter_mut() {
            if entry.code() > 260 {
                entry.clear();
            }
        }
    }
    fn cull(&mut self) {
        let mut entries = self.entries
            .clone()
            .into_iter()
            .filter(|e| !e.is_empty() && e.code() > 260)
            .collect::<Vec<Entry>>();

        entries.sort_by(|a, b| a.code().cmp(&b.code()));

        self.reset();

        entries.retain(|entry| !self.cull.cull(entry));

        for entry in entries.iter() {
            self.insert(entry.string().to_vec(), self.code);
        }
    }
}

struct Encoder {
    dict:    Dictionary,
    string:  Vec<u8>,
}
impl Encoder {
    fn new(size: usize, cull: Cull) -> Encoder {
        Encoder {
            dict:    Dictionary::new(size, cull),
            string:  Vec::new(),
        }
    }
    fn compress(&mut self, blk: Vec<u8>) -> CodeWriter {
        let mut stream = CodeWriter::new();

        for byte in blk.iter() {
            self.string.push(*byte);

            if self.dict.get_entry(&self.string).is_none() {
                self.dict.insert(self.string.clone(), self.dict.code);
                stream.write(self.output_code());
                
                if self.dict.code >= self.dict.cull.max {
                    stream.write(CULL);
                    self.dict.cull();
                    stream.code_len = self.dict.code.next_power_of_two().log2();
                }

                if self.dict.code == 1 << stream.code_len {
                    stream.write(LEN_UP);
                }
            }
        }

        if let Some(entry) = self.dict.get_entry(&self.string) {
            entry.increase_count();
            stream.write(entry.code());
        }

        stream.write(DATA_END);
        stream
    }
    fn output_code(&mut self) -> u32 {
        let last_char = self.string.pop().unwrap();
        let entry = self.dict.get_entry(&self.string).unwrap();
        entry.increase_count();
        let code = entry.code();

        self.string.clear();
        self.string.push(last_char);

        code
    }
}


pub fn compress(blk_in: Vec<u8>, mem: usize) -> Vec<u8> {
    if blk_in.is_empty() {
        return Vec::new();
    }
    let size = mem / 4;
    let max = (size as f64 * 0.4) as u32;
    let cull = Cull::settings(1, max - 1, max);

    let mut enc = Encoder::new(size, cull);
    enc.compress(blk_in).out
}
