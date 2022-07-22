use crate::lzw::{
    code::CodeReader,
    constant::{
        DATA_END,
        LEN_UP,
        RESET,
    },
};

struct Dictionary {
    strings:      Vec<u8>,
    codes:        Vec<u32>,
    pub code:     u32,
}
impl Dictionary {
    fn new(size: usize) -> Dictionary {
        let mut dict = Dictionary {
            strings:   Vec::with_capacity(size),
            codes:     vec![0; size],
            code:      1,
        };
        dict.reset();
        dict
    }
    fn get(&self, code: u32) -> Option<&[u8]> {
        let code = self.codes[code as usize];

        if code != 0 {
            let pos = (code & 0x07FFFFFF) as usize;
            let len = (code >> 27) as usize;
            return Some(&self.strings[pos..pos+len]);
        }
        None
    }
    fn insert(&mut self, string: &[u8], code: u32) {
        assert!(self.strings.len() < 0x07FFFFFF);
        assert!(string.len() < 32);

        let len = string.len() << 27;
        let pos = self.strings.len();
        self.codes[code as usize] = (len + pos) as u32;

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
            self.insert(&[i], self.code);
        }
        // Skip reserved codes
        self.code += 3;
    }
}

struct Decoder {
    dict:    Dictionary,
    string:  Vec<u8>,
    code:    u32,
    pub blk: Vec<u8>,
}
impl Decoder {
    fn new(mem: usize) -> Decoder {
        Decoder {
            dict:   Dictionary::new(mem/4),
            string: Vec::new(),
            code:   0,
            blk:    Vec::new(),
        }
    }
    fn decompress(&mut self, blk_in: Vec<u8>) {
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
                        self.dict.reset();
                        self.string.clear();
                    }
                    _ => {
                        self.output_string(code);
                    }
                }
            }
        }  
    }
    fn output_string(&mut self, code: u32) {
        let string = self.dict.get(code);

        if string.is_some() {
            let string = string.unwrap().to_vec();

            if !self.string.is_empty() {
                self.string.push(string[0]);
                self.dict.insert(&self.string, self.dict.code);
            }

            for byte in string.iter() {
                self.blk.push(*byte);
            }

            self.string = string;
        }
        else {
            self.string.push(self.string[0]);
            self.dict.insert(&self.string, code);

            let string = self.dict.get(code).unwrap();

            for byte in string.iter() {
                self.blk.push(*byte);
            }

            self.string = string.to_vec();
        }
    }
}

pub fn decompress(blk_in: Vec<u8>, mem: usize) -> Vec<u8> {
    if blk_in.is_empty() { 
        return Vec::new(); 
    }
    
    let mut dec = Decoder::new(mem);
    dec.decompress(blk_in);
    dec.blk
}