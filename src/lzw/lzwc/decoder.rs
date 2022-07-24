use crate::lzw::{
    code::CodeReader,
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
        for i in 0u8..=255 {
            dict.insert(dict.code, vec![i]);
        }
        dict.code += 4;
        dict
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
            self.insert(self.code, entry.string().to_vec());
        }
    }
}

struct Decoder {
    dict:    Dictionary,
    string:  Vec<u8>,
    blk:     Vec<u8>,
}
impl Decoder {
    fn new(size: usize, cull: Cull) -> Decoder {
        Decoder {
            dict:    Dictionary::new(size, cull),
            string:  Vec::new(),
            blk:     Vec::new(),
        }
    }
    pub fn decompress(&mut self, blk_in: Vec<u8>) {
        let mut stream = CodeReader::new(blk_in);
        loop { 
            if let Some(code) = stream.get_code() {
                assert!(code > 0);
                assert!(code < self.dict.entries.len() as u32);
                match code {
                    DATA_END => {
                        break;
                    }
                    LEN_UP => {
                        stream.code_len += 1;
                    },
                    CULL => {
                        self.dict.cull();
                        stream.code_len = (self.dict.code+1).next_power_of_two().log2();
                    },
                    _ => {
                        self.output_string(code);
                    }
                }
            }
        } 
    }
    fn output_string(&mut self, code: u32) {
        if let Some(entry) = self.dict.get_entry(code) {
            let string = entry.string().to_vec();
            
            if !self.string.is_empty() {
                self.string.push(string[0]);
                self.dict.insert(self.dict.code, self.string.clone());
            }

            for byte in string.iter() {
                self.blk.push(*byte);
            }

            self.string = string;
        }
        else {
            self.string.push(self.string[0]);
            self.dict.insert(code, self.string.clone());

            let entry = self.dict.get_entry(code).unwrap();
            let string = entry.string().to_vec();
            
            for byte in string.iter() {
                self.blk.push(*byte);
            }

            self.string = string;
        }
    }
}

pub fn decompress(blk_in: Vec<u8>, mem: usize) -> Vec<u8> {
    if blk_in.is_empty() {
        return Vec::new();
    }
    let size = mem / 4;
    let max = (size as f64 * 0.4) as u32;
    let cull = Cull::settings(1, max - 1, max);

    let mut dec = Decoder::new(size, cull);
    dec.decompress(blk_in);
    dec.blk
}