use crate::lzw::{
    code::CodeReader,
    constant::{
        DATA_END,
        LEN_UP,
        RESET,
    },
};

struct HashTable {
    strings:      Vec<u8>,
    codes:        Vec<u32>,
    pub code:     u32,
    pub max_code: u32,
}
impl HashTable {
    fn new(size: usize) -> HashTable {
        HashTable {
            strings:   Vec::with_capacity(size),
            codes:     vec![0; size],
            code:      1,
            max_code:  size as u32,
        }
    }
    fn get(&mut self, code: u32) -> Option<&[u8]> {
        let code = code as usize;

        if self.codes[code] != 0 {
            let pos = (self.codes[code] & 0x07FFFFFF) as usize;
            let len = (self.codes[code] >> 27) as usize;
            return Some(&self.strings[pos..pos+len]);
        }
        None
    }
    fn insert(&mut self, code: u32, string: Vec<u8>) {
        assert!(self.strings.len() < 0x07FFFFFF);
        assert!(string.len() < 32);
        self.codes[code as usize] = ((string.len() << 27) + self.strings.len()) as u32;
        for s in string.iter() {
            self.strings.push(*s);
        }
        self.code += 1;
    }
    fn reset(&mut self) {
        // Skip code 0
        self.code = 1;
        for i in self.strings.iter_mut() {
            *i = 0;
        }
        for i in self.codes.iter_mut() {
            *i = 0;
        }
        for i in 0u8..=255 {
            self.insert(self.code, vec![i]);
        }
        // Skip reserved codes
        self.code += 3;
    }
}

struct Dictionary {
    pub map:     HashTable,
    pub string:  Vec<u8>, // Current string
    pub blk:     Vec<u8>,
}
impl Dictionary {
    fn new(mem: usize) -> Dictionary {
        let mut map = HashTable::new(mem/4);
        map.reset();

        Dictionary {
            map,
            string:  Vec::new(),
            blk:     Vec::new(),
        }
    }
    fn output_string(&mut self, code: u32) {
        if self.map.get(code).is_none() {
            self.string.push(self.string[0]);
            self.map.insert(code, self.string.clone());
        }
        else if !self.string.is_empty() {
            self.string.push((self.map.get(code).unwrap())[0]);
            self.map.insert(self.map.code, self.string.clone());
        }
        
        let string = self.map.get(code).unwrap();
        for byte in string.iter() {
            self.blk.push(*byte);
        }

        self.string = string.to_vec();

        if self.map.code >= self.map.max_code {
            self.map.reset();
        }
    }
}

pub fn decompress(blk_in: Vec<u8>, mem: usize) -> Vec<u8> {
    if blk_in.is_empty() { 
        return Vec::new(); 
    }
    let mut dict = Dictionary::new(mem);
    let mut stream = CodeReader::new(blk_in);
    
    loop { 
        if let Some(code) = stream.get_code() {
            match code {
                DATA_END => {
                    break;
                }
                LEN_UP => {
                    stream.code_len += 1;
                }
                RESET => {
                    stream.code_len = 9;
                }
                _ => {
                    dict.output_string(code);
                }
            }
        }
    }  
    dict.blk
}