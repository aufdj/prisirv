
// const HT_SIZE: usize = 1 << 20;

// pub struct Predictor {
//     cxt:   i32,
//     bits:  i32,
//     hash:  usize,
//     max:   i32,
//     map:   Vec<u32>,
// }
// impl Predictor {
//     pub fn new() -> Predictor {
//         Predictor {
//             cxt:   0, 
//             bits:  0,
//             hash:  0,
//             max:   9,
//             map:   vec![32768; HT_SIZE],
//         }
//     }
//     pub fn p(&mut self) -> u32 {
//         (self.map[self.hash] >> 4) as u32
//     } 
//     pub fn update(&mut self, bit: i32) {
//         if bit == 1 { 
//             self.map[self.hash] += 65535 - self.map[self.hash] >> 5;
//         } 
//         else { 
//             self.map[self.hash] -= self.map[self.hash] >> 5;
//         }
//         self.cxt |= bit << self.bits;

//         self.hash = self.hash(self.cxt);
//         self.bits += 1;
        
//         if self.bits >= self.max {
//             match self.cxt {
//                 258 => {
//                     self.max += 1;
//                 },
//                 259 => {
//                     self.max = 9;
//                 },
//                 _ => {}
//             }
//             self.cxt = 0;
//             self.bits = 0;
//         } 
//     }
//     fn hash(&self, cxt: i32) -> usize {
//         cxt.wrapping_mul(123456791) as usize & (HT_SIZE - 1)
//     }
// }
