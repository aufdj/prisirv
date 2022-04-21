use std::collections::HashMap;
use std::cmp::min;

const DATA_END: u32 = 256;
const CODE_LEN_UP: u32 = 257;
const CODE_LEN_RESET: u32 = 258;

struct BitStream {
    pck:      u32,
    pck_len:  u32,
    pub out:  Vec<u8>,
    pub code_len: u32,
}
impl BitStream {
    fn new() -> BitStream {
        BitStream {
            pck:      0,
            pck_len:  0,
            out:      Vec::new(),
            code_len: 9,
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
        let codeu_len = self.code_len.saturating_sub(codel_len);

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

        if code == CODE_LEN_UP    { self.code_len += 1; }
        if code == CODE_LEN_RESET { self.code_len = 9;  }

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
    fn new(byte: u8) -> Dictionary {
        let mut d = Dictionary {
            map:       HashMap::new(),
            max_code:  0x40000,
            code:      259,
            string:    vec![byte],
            stream:    BitStream::new(),
        };
        for i in 0..256 { 
            d.map.insert(vec![i as u8], i);
        }
        d
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

        let code = *self.map.get(&self.string).unwrap();
        self.stream.write(code);

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
            let code = *self.map.get(&self.string).unwrap();
            self.stream.write(code);
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


pub fn compress(blk_in: &[u8]) -> Vec<u8> {
    if blk_in.is_empty() { return Vec::new(); }
    let mut blk = blk_in.iter();

    let mut dict = Dictionary::new(*blk.next().unwrap());

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

