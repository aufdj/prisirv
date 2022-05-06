use std::collections::HashMap;
use std::cmp::min;

const DATA_END: u32 = 256;
const CODE_LEN_UP: u32 = 257;
const CODE_LEN_RESET: u32 = 258;

struct BitStream {
    stream:        Box<dyn Iterator<Item = u8>>,
    pub code_len:  u32,
    code:          u32,
    count:         u32,
    out:           u32,
}
impl BitStream {
    fn new(stream: Box<dyn Iterator<Item = u8>>) -> BitStream {
        BitStream {
            stream,
            code_len:  9,
            code:      0,
            count:     0,
            out:       0,
        }
    }
    fn get_code(&mut self) -> Option<u32> {
        loop {
            match self.stream.next() {
                Some(byte) => {
                    let rem_len = self.code_len - self.count;

                    let codel = byte as u32 & ((1 << rem_len) - 1);
                    let codel_len = min(8, rem_len);

                    let codeu = byte as u32 >> (codel_len);
                    let codeu_len = 8 - codel_len;

                    if self.count == self.code_len {
                        if self.code == CODE_LEN_UP { 
                            self.code_len += 1; 
                        }
                        if self.code == CODE_LEN_RESET { 
                            self.code_len = 9;  
                        }
                        self.out = self.code;
                        self.code = 0;
                        self.count = 0;
                        self.code |= codel;
                        self.count += codel_len;
                        return Some(self.out);
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
                        self.out = self.code;
                        self.code = 0;
                        self.count = 0;
                        self.code |= codeu;
                        self.count += codeu_len;
                        return Some(self.out);
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
    pub map:       HashMap<u32, Vec<u8>>,
    pub max_code:  u32,
    pub code:      u32,
    pub string:    Vec<u8>, // Current string
    pub blk:       Vec<u8>,
}
impl Dictionary {
    fn new(mem: usize) -> Dictionary {
        let mut map = HashMap::with_capacity(mem/4);
        for i in 0..256 {
            map.insert(i, vec![i as u8]);
        }

        Dictionary {
            map,
            max_code:  (mem/4) as u32,
            code:      259,
            string:    Vec::new(),
            blk:       Vec::new(),
        }
    }
    fn reset(&mut self) {
        self.code = 259;
        self.map.clear(); 
        for i in 0..256 {
            self.map.insert(i, vec![i as u8]);
        }
    }
    fn output_string(&mut self, code: u32) {
        if self.code < self.max_code {
            if !self.map.contains_key(&code) {
                self.string.push(self.string[0]);
                self.map.insert(code, self.string.clone());
                self.code += 1;
            }
            else if !self.string.is_empty() {
                self.string.push((self.map.get(&code).unwrap())[0]);
                self.map.insert(self.code, self.string.clone());
                self.code += 1;
            }
        }
        
        let string = self.map.get(&code).unwrap();
        for byte in string.iter() {
            self.blk.push(*byte);
        }

        self.string = string.to_vec();

        if self.code >= self.max_code {
            self.reset();
        }
    }
}

pub fn decompress(blk_in: &[u8], mem: usize) -> Vec<u8> {
    if blk_in.is_empty() { return Vec::new(); }
    let mut stream = BitStream::new(Box::new(blk_in.to_vec().into_iter()));
    let mut dict = Dictionary::new(mem);

    loop { 
        if let Some(code) = stream.get_code() {
            if code == DATA_END { break; }
            if code != 257 && code != 258 {
                dict.output_string(code);
            }
        }
    }  
    dict.blk
}