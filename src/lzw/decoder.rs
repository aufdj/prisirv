use std::collections::HashMap;

struct Dictionary {
    pub map: HashMap<u16, Vec<u8>>,
    pub max_code: u16,
    pub code: u16,
    pub string: Vec<u8>,
}
impl Dictionary {
    fn new() -> Dictionary {
        let mut d = Dictionary {
            map: HashMap::new(),
            max_code: 65535,
            code: 256,
            string: vec![],
        };
        for i in 0..256 { 
            d.map.insert(i, vec![i as u8]);
        }
        d
    }
    fn reset(&mut self) {
        self.code = 257;
        self.map.clear(); 
        for i in 0..256 {
            self.map.insert(i, vec![i as u8]);
        }
    }
    fn output_string(&mut self, blk: &mut Vec<u8>, code: u16) {
        if !self.map.contains_key(&code) && self.code < self.max_code { // Didn't recognize code
            self.string.push(self.string[0]);
            
            self.map.insert(code, self.string.clone());
            self.code += 1;
        }
        else if !self.string.is_empty() && self.code < self.max_code {
            self.string.push((&self.map.get(&code).unwrap())[0]);
            
            self.map.insert(self.code, self.string.clone());
            self.code += 1;
        }

        let string = self.map.get(&code).unwrap();
        for byte in string.iter() {
            blk.push(*byte);
        }

        self.string = string.to_vec();

        if self.code >= self.max_code {
            self.reset();
        }
    }
}

pub fn decompress(blk_in: &[u8]) -> Vec<u8> {
    let mut blk = blk_in.iter();
    let mut blk_out = Vec::new();

    let mut dict = Dictionary::new();

    loop { 
        let mut code: u16 = 0;
        match blk.next() {
            Some(byte) => code += *byte as u16,
            None => break,
        }
        match blk.next() {
            Some(byte) => code += (*byte as u16) * 256,
            None => break,
        }

        dict.output_string(&mut blk_out, code);
    }  
    blk_out  
}