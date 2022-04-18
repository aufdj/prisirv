use std::collections::HashMap;

struct Dictionary {
    pub map: HashMap<Vec<u8>, u16>,
    pub max_code: u16,
    pub code: u16,
    pub string: Vec<u8>,
}
impl Dictionary {
    fn new(byte: u8) -> Dictionary {
        let mut d = Dictionary {
            map: HashMap::new(),
            max_code: 65535,
            code: 256,
            string: vec![byte],
        };
        for i in 0..256 { 
            d.map.insert(vec![i as u8], i);
        }
        d
    }
    fn reset(&mut self) {
        self.code = 257;
        self.map.clear(); 
        for i in 0..256 {
            self.map.insert(vec![i as u8], i);
        }
    }
    fn output_code(&mut self, blk: &mut Vec<u8>) {
        if self.code <= self.max_code {
            self.map.insert(self.string.clone(), self.code); 
            self.code += 1;
        }

        let last_char = self.string.pop().unwrap();

        let code = *self.map.get(&self.string).unwrap();
        blk.push((code & 0xFF) as u8);
        blk.push((code >> 8)   as u8);

        self.string.clear();
        self.string.push(last_char); 

        if self.code >= self.max_code {
            self.reset();
        }
    }
    fn output_last_code(&mut self, blk: &mut Vec<u8>) {
        if !self.string.is_empty() {
            let code = *self.map.get(&self.string).unwrap();
            blk.push((code & 0xFF) as u8);
            blk.push((code >> 8)   as u8);
        } 
    }
    fn update_string(&mut self, byte: u8) {
        self.string.push(byte);
    }
    fn contains_string(&self) -> bool {
        self.map.contains_key(&self.string)
    }
}


pub fn compress(blk_in: &[u8]) -> Vec<u8> {
    let mut blk = blk_in.iter();
    let mut blk_out = Vec::new();

    let byte = blk.next();
    if byte == None { return blk_out; }
    let mut dict = Dictionary::new(*byte.unwrap());

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
        dict.output_code(&mut blk_out);
    }
    dict.output_last_code(&mut blk_out);
    blk_out
}

