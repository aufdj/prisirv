use std::{
    fs::{File, create_dir},
    path::{Path, PathBuf},
    cmp::{min, Ordering},
    time::Instant,
    iter::repeat,
    env,
    io::{
    Read, Write, BufReader, BufWriter, 
    BufRead, Seek, SeekFrom, ErrorKind
    },
};
    

const MEM: usize = 1 << 23;

// Convenience functions for buffered I/O ---------------------------------------------------------- Convenience functions for buffered I/O
#[derive(PartialEq, Eq)]
enum BufferState {
    NotEmpty,
    Empty,
}

trait BufferedRead {
    fn read_byte(&mut self) -> u8;
    fn read_usize(&mut self) -> usize;
    fn fill_buffer(&mut self) -> BufferState;
}
impl BufferedRead for BufReader<File> {
    fn read_byte(&mut self) -> u8 {
        let mut byte = [0u8; 1];
        match self.read(&mut byte) {
            Ok(_)  => {},
            Err(e) => {
                println!("Function read_byte failed.");
                println!("Error: {}", e);
            },
        };
        if self.buffer().is_empty() {
            self.consume(self.capacity());
            match self.fill_buf() {
                Ok(_)  => {},
                Err(e) => {
                    println!("Function read_byte failed.");
                    println!("Error: {}", e);
                },
            }
        }
        u8::from_le_bytes(byte)
    }
    fn read_usize(&mut self) -> usize {
        let mut bytes = [0u8; 8];
        match self.read(&mut bytes) {
            Ok(_)  => {},
            Err(e) => {
                println!("Function read_usize failed.");
                println!("Error: {}", e);
            },
        };
        if self.buffer().is_empty() {
            self.consume(self.capacity());
            match self.fill_buf() {
                Ok(_)  => {},
                Err(e) => {
                    println!("Function read_usize failed.");
                    println!("Error: {}", e);
                },
            }
        }
        usize::from_le_bytes(bytes)
    }
    fn fill_buffer(&mut self) -> BufferState {
        self.consume(self.capacity());
        match self.fill_buf() {
            Ok(_)  => {},
            Err(e) => {
                println!("Function fill_buffer failed.");
                println!("Error: {}", e);
            },
        }
        if self.buffer().is_empty() {
            return BufferState::Empty;
        }
        BufferState::NotEmpty
    }
}
trait BufferedWrite {
    fn write_byte(&mut self, output: u8);
    fn write_usize(&mut self, output: usize);
    fn flush_buffer(&mut self);
}
impl BufferedWrite for BufWriter<File> {
    fn write_byte(&mut self, output: u8) {
        match self.write(&[output]) {
            Ok(_)  => {},
            Err(e) => {
                println!("Function write_byte failed.");
                println!("Error: {}", e);
            },
        }
        if self.buffer().len() >= self.capacity() {
            match self.flush() {
                Ok(_)  => {},
                Err(e) => {
                    println!("Function write_byte failed.");
                    println!("Error: {}", e);
                },
            }
        }
    }
    fn write_usize(&mut self, output: usize) {
        match self.write(&output.to_le_bytes()[..]) {
            Ok(_)  => {},
            Err(e) => {
                println!("Function write_usize failed.");
                println!("Error: {}", e);
            },
        }
        if self.buffer().len() >= self.capacity() {
            match self.flush() {
                Ok(_)  => {},
                Err(e) => {
                    println!("Function write_usize failed.");
                    println!("Error: {}", e);
                },
            } 
        }
    }
    fn flush_buffer(&mut self) {
        match self.flush() {
            Ok(_)  => {},
            Err(e) => {
                println!("Function flush_buffer failed.");
                println!("Error: {}", e);
            },
        }    
    }
}
fn new_input_file(capacity: usize, file_name: &Path) -> BufReader<File> {
    BufReader::with_capacity(
        capacity, File::open(file_name).unwrap()
    )
}
fn new_output_file(capacity: usize, file_name: &Path) -> BufWriter<File> {
    BufWriter::with_capacity(
        capacity, File::create(file_name).unwrap()
    )
}
fn new_dir(path: &String) {
    let path = Path::new(path);
    match create_dir(path) {
        Ok(_) => {},
        Err(err) => {
            match err.kind() {
                ErrorKind::AlreadyExists => {
                    println!("Directory {} already exists.", path.display());
                    std::process::exit(1);
                },
                ErrorKind::InvalidInput  => {
                    println!("Invalid directory name.");
                },
                _ => 
                    println!("Error"),
            }
        }
    }
}
// ----------------------------------------------------------------------------------------------------------------------------------------


// Logistic Functions -------------------------------------------------------------------------------------------------- Logistic Functions
// Returns p = 1/(1 + exp(-d))
// d = (-2047..2047), p = (0..4095)
fn squash(d: i32) -> i32 {
    const SQ_T: [i32; 33] = [
    1,2,3,6,10,16,27,45,73,120,194,310,488,747,1101,
    1546,2047,2549,2994,3348,3607,3785,3901,3975,4022,
    4050,4068,4079,4085,4089,4092,4093,4094];
    if d > 2047  { return 4095; }
    if d < -2047 { return 0;    }
    let i_w = d & 127;
    let d = ((d >> 7) + 16) as usize;
    (SQ_T[d] * (128 - i_w) + SQ_T[d+1] * i_w + 64) >> 7
}

// Returns p = ln(d/(1-d)) (Inverse of squash)
// d = (0..4095), p = (-2047..2047)
struct Stretch {
    stretch_table: [i16; 4096],
}
impl Stretch {
    fn new() -> Stretch {
        let mut st = Stretch {
            stretch_table: [0; 4096],
        };
        let mut pi = 0;
        for x in -2047..=2047 {
            let i = squash(x);
            for j in pi..=i {
                st.stretch_table[j as usize] = x as i16;
            }
            pi = i + 1;
        }
        st.stretch_table[4095] = 2047;
        st
    }
    fn stretch(&self, p: i32) -> i32 {
        assert!(p < 4096);
        self.stretch_table[p as usize] as i32
    }
}
// ----------------------------------------------------------------------------------------------------------------------------------------


// Adaptive Probability Map -------------------------------------------------------------------------------------- Adaptive Probability Map
struct Apm {
    s:         Stretch,  // For computing stretch(), or ln(d/(1-d))
    bin:       usize,    // A value used for interpolating a new prediction
    num_cxts:  usize,    // Number of possible contexts i.e 256 for order-0
    bin_map:   Vec<u16>, // Table mapping values 0..=32 to squashed 16 bit values
}
impl Apm {
    fn new(n: usize) -> Apm {
        Apm {
            s:         Stretch::new(),
            bin:       0,
            num_cxts:  n,
            bin_map:   repeat( // Map 0..33 to values in closure, create n copies
                       (0..33).map(|i| (squash((i - 16) * 128) * 16) as u16)
                       .collect::<Vec<u16>>().into_iter() )
                       .take(n)
                       .flatten()
                       .collect::<Vec<u16>>(),
        }
    }
    fn p(&mut self, bit: i32, rate: i32, mut pr: i32, cxt: u32) -> i32 {
        assert!(bit == 0 || bit == 1 && pr >= 0 && pr < 4096);
        assert!(cxt < self.num_cxts as u32);
        self.update(bit, rate);
        
        pr = self.s.stretch(pr); // -2047 to 2047
        let i_w = pr & 127;      // Interpolation weight (33 points)
        
        // Compute set of bins from context, and singular bin from prediction
        self.bin = (((pr + 2048) >> 7) + ((cxt as i32) * 33)) as usize;

        let a = self.bin_map[self.bin] as i32;
        let b = self.bin_map[self.bin+1] as i32;
        ((a * (128 - i_w)) + (b * i_w)) >> 11 // Interpolate pr from bin and bin+1
    }
    fn update(&mut self, bit: i32, rate: i32) {
        assert!(bit == 0 || bit == 1 && rate > 0 && rate < 32);
        
        // Controls direction of update (bit = 1: increase, bit = 0: decrease)
        let g: i32 = (bit << 16) + (bit << rate) - bit - bit;

        // Bins used for interpolating previous prediction
        let a = self.bin_map[self.bin] as i32;   // Lower
        let b = self.bin_map[self.bin+1] as i32; // Higher
        self.bin_map[self.bin]   = (a + ((g - a) >> rate)) as u16;
        self.bin_map[self.bin+1] = (b + ((g - b) >> rate)) as u16;
    }
}
// ----------------------------------------------------------------------------------------------------------------------------------------


// State Map -------------------------------------------------------------------------------------------------------------------- State Map
const STATE_TABLE: [[u8; 2]; 256] = [
[  1,  2],[  3,  5],[  4,  6],[  7, 10],[  8, 12],[  9, 13],[ 11, 14], // 0
[ 15, 19],[ 16, 23],[ 17, 24],[ 18, 25],[ 20, 27],[ 21, 28],[ 22, 29], // 7
[ 26, 30],[ 31, 33],[ 32, 35],[ 32, 35],[ 32, 35],[ 32, 35],[ 34, 37], // 14
[ 34, 37],[ 34, 37],[ 34, 37],[ 34, 37],[ 34, 37],[ 36, 39],[ 36, 39], // 21
[ 36, 39],[ 36, 39],[ 38, 40],[ 41, 43],[ 42, 45],[ 42, 45],[ 44, 47], // 28
[ 44, 47],[ 46, 49],[ 46, 49],[ 48, 51],[ 48, 51],[ 50, 52],[ 53, 43], // 35
[ 54, 57],[ 54, 57],[ 56, 59],[ 56, 59],[ 58, 61],[ 58, 61],[ 60, 63], // 42
[ 60, 63],[ 62, 65],[ 62, 65],[ 50, 66],[ 67, 55],[ 68, 57],[ 68, 57], // 49
[ 70, 73],[ 70, 73],[ 72, 75],[ 72, 75],[ 74, 77],[ 74, 77],[ 76, 79], // 56
[ 76, 79],[ 62, 81],[ 62, 81],[ 64, 82],[ 83, 69],[ 84, 71],[ 84, 71], // 63
[ 86, 73],[ 86, 73],[ 44, 59],[ 44, 59],[ 58, 61],[ 58, 61],[ 60, 49], // 70
[ 60, 49],[ 76, 89],[ 76, 89],[ 78, 91],[ 78, 91],[ 80, 92],[ 93, 69], // 77
[ 94, 87],[ 94, 87],[ 96, 45],[ 96, 45],[ 48, 99],[ 48, 99],[ 88,101], // 84
[ 88,101],[ 80,102],[103, 69],[104, 87],[104, 87],[106, 57],[106, 57], // 91
[ 62,109],[ 62,109],[ 88,111],[ 88,111],[ 80,112],[113, 85],[114, 87], // 98
[114, 87],[116, 57],[116, 57],[ 62,119],[ 62,119],[ 88,121],[ 88,121], // 105
[ 90,122],[123, 85],[124, 97],[124, 97],[126, 57],[126, 57],[ 62,129], // 112
[ 62,129],[ 98,131],[ 98,131],[ 90,132],[133, 85],[134, 97],[134, 97], // 119
[136, 57],[136, 57],[ 62,139],[ 62,139],[ 98,141],[ 98,141],[ 90,142], // 126
[143, 95],[144, 97],[144, 97],[ 68, 57],[ 68, 57],[ 62, 81],[ 62, 81], // 133
[ 98,147],[ 98,147],[100,148],[149, 95],[150,107],[150,107],[108,151], // 140
[108,151],[100,152],[153, 95],[154,107],[108,155],[100,156],[157, 95], // 147
[158,107],[108,159],[100,160],[161,105],[162,107],[108,163],[110,164], // 154
[165,105],[166,117],[118,167],[110,168],[169,105],[170,117],[118,171], // 161
[110,172],[173,105],[174,117],[118,175],[110,176],[177,105],[178,117], // 168
[118,179],[110,180],[181,115],[182,117],[118,183],[120,184],[185,115], // 175
[186,127],[128,187],[120,188],[189,115],[190,127],[128,191],[120,192], // 182
[193,115],[194,127],[128,195],[120,196],[197,115],[198,127],[128,199], // 189
[120,200],[201,115],[202,127],[128,203],[120,204],[205,115],[206,127], // 196
[128,207],[120,208],[209,125],[210,127],[128,211],[130,212],[213,125], // 203
[214,137],[138,215],[130,216],[217,125],[218,137],[138,219],[130,220], // 210
[221,125],[222,137],[138,223],[130,224],[225,125],[226,137],[138,227], // 217
[130,228],[229,125],[230,137],[138,231],[130,232],[233,125],[234,137], // 224
[138,235],[130,236],[237,125],[238,137],[138,239],[130,240],[241,125], // 231
[242,137],[138,243],[130,244],[245,135],[246,137],[138,247],[140,248], // 238
[249,135],[250, 69],[ 80,251],[140,252],[249,135],[250, 69],[ 80,251], // 245
[140,252],[  0,  0],[  0,  0],[  0,  0]];  // 252

fn next_state(state: u8, bit: i32) -> u8 {
    STATE_TABLE[state as usize][bit as usize]
}

#[allow(overflowing_literals)]
const PR_MSK: i32 = 0xFFFFFC00; // High 22 bit mask
const LIMIT: usize = 127; // Controls rate of adaptation (higher = slower) (0..1024)

// A Statemap is used in an indirect context model to map a context to a 
// state (a 1 byte representation of 0 and 1 counts), which is then mapped 
// to a prediction. 
#[derive(Clone)]
struct StateMap {
    cxt:      usize,    // Context of last prediction
    cxt_map:  Vec<u32>, // Maps a context to a prediction and a count
    rec_t:    Vec<u16>, // Reciprocal table: controls adjustment to cxt_map
}
impl StateMap {
    fn new(n: usize) -> StateMap {
        StateMap {
            cxt:      0,
            cxt_map:  vec![1 << 31; n],
            rec_t:    (0..1024).map(|i| 16_384/(i+i+3)).collect(),
        }
    }
    fn p(&mut self, bit: i32, cxt: i32) -> i32 {
        assert!(bit == 0 || bit == 1);
        self.update(bit);
        self.cxt = cxt as usize;
        (self.cxt_map[self.cxt] >> 20) as i32
    }
    fn update(&mut self, bit: i32) {
        let count = (self.cxt_map[self.cxt] & 1023) as usize; // Low 10 bits
        let pr    = (self.cxt_map[self.cxt] >> 10 ) as i32;   // High 22 bits

        if count < LIMIT { self.cxt_map[self.cxt] += 1; }

        // Update cxt_map based on prediction error
        let pr_err = ((bit << 22) - pr) >> 3; // Prediction error
        let rec_v = self.rec_t[count] as i32; // Reciprocal value
        self.cxt_map[self.cxt] = 
        self.cxt_map[self.cxt].wrapping_add((pr_err * rec_v & PR_MSK) as u32);
    }
}
// ----------------------------------------------------------------------------------------------------------------------------------------


// Mixer ---------------------------------------------------------------------------------------------------------------------------- Mixer
fn train(inputs: &[i32], weights: &mut [i32], error: i32) {
    for (input, weight) in inputs.iter().zip(weights.iter_mut()) {
        *weight += ((*input * error) + 0x8000) >> 16;
    }
}
fn dot_product(inputs: &[i32], weights: &[i32]) -> i32 {
    (inputs.iter().zip(weights.iter())
    .map(|(i, w)| i * w).sum::<i32>()) >> 16
}

struct Mixer {
    max_in:   usize,    // Maximum number of inputs
    inputs:   Vec<i32>, // Current inputs
    weights:  Vec<i32>, // Weights used for weighted averaging
    wht_set:  usize,    // Single set of weights chosen by a context
    pr:       i32,      // Current prediction
}
impl Mixer {
    fn new(n: usize, m: usize) -> Mixer {
        Mixer {
            max_in:   n,                     
            inputs:   Vec::with_capacity(n), 
            weights:  vec![0; n * m],        
            wht_set:  0,                     
            pr:       2048,                  
        }
    }
    fn add(&mut self, pr: i32) {
        assert!(self.inputs.len() < self.inputs.capacity());
        self.inputs.push(pr);
    }
    fn set(&mut self, cxt: u32) {
        // Calculate set of weights to be used for mixing
        self.wht_set = (cxt as usize) * self.max_in; 
    }
    fn p(&mut self) -> i32 {
        let d = dot_product(&self.inputs[..], &self.weights[self.wht_set..]);
        self.pr = squash(d);
        self.pr
    }
    fn update(&mut self, bit: i32) {
        let error: i32 = ((bit << 12) - self.pr) * 7;
        assert!(error >= -32768 && error < 32768);
        train(&self.inputs[..], &mut self.weights[self.wht_set..], error);
        self.inputs.clear();
    }
}
// ----------------------------------------------------------------------------------------------------------------------------------------


// Match Model ---------------------------------------------------------------------------------------------------------------- Match Model
const BUF_END: usize = (MEM / 2) - 1;
const HT_END:  usize = (MEM / 8) - 1;
const MAX_LEN: usize = 62;

struct MatchModel {
    mch_ptr:  usize,    
    mch_len:  usize,    
    cxt:      usize,    
    bits:     usize,    
    sm:       StateMap,
    buf:      Vec<u8>,
    ht:       Vec<u32>,
    hash_s:   usize,
    hash_l:   usize,
    buf_pos:  usize,
    s:        Stretch,
}
impl MatchModel {
    fn new() -> MatchModel {
        MatchModel {
            mch_ptr:  0,    hash_s:   0,
            mch_len:  0,    hash_l:   0,
            cxt:      1,    buf_pos:  0,
            bits:     0,    s:        Stretch::new(),
            sm:       StateMap::new(56 << 8),
            buf:      vec![0; BUF_END + 1],
            ht:       vec![0;  HT_END + 1],
        }
    }
    fn find_or_extend_match(&mut self, hash: usize) {
        self.mch_ptr = self.ht[hash] as usize;
        if self.mch_ptr != self.buf_pos {
            let mut i = self.mch_ptr - self.mch_len - 1 & BUF_END;
            let mut j = self.buf_pos - self.mch_len - 1 & BUF_END;

            while i != self.buf_pos 
            && self.mch_len < MAX_LEN 
            && self.buf[i] == self.buf[j] {
                self.mch_len += 1;
                i = (i - 1) & BUF_END; 
                j = (j - 1) & BUF_END;  
            }
        }
    }
    fn len(&self) -> usize {
        self.mch_len
    }
    fn p(&mut self, bit: i32, mxr: &mut Mixer) {
        self.update(bit);

        let mut cxt = self.cxt;

        let a = (self.buf[self.mch_ptr] as usize) + 256 >> (8 - self.bits);
        if self.mch_len > 0 && a == cxt {
            let b = (self.buf[self.mch_ptr] >> 7 - self.bits & 1) as usize;
            if self.mch_len < 16 {
                cxt = self.mch_len * 2 + b;
            }
            else {
                cxt = (self.mch_len >> 2) * 2 + b + 24;
            }
            cxt = cxt * 256 + self.buf[self.buf_pos-1 & BUF_END] as usize;
        } 
        else {
            self.mch_len = 0;
        }

        mxr.add(self.s.stretch(self.sm.p(bit, cxt as i32)));

        if self.bits == 0 {
            self.ht[self.hash_s] = self.buf_pos as u32;
            self.ht[self.hash_l] = self.buf_pos as u32;
        }
    }
    fn update(&mut self, bit: i32) {
        self.cxt += self.cxt + bit as usize;
        self.bits += 1;
        if self.bits == 8 {
            self.bits = 0;
            self.hash_s = self.hash_s * (3 << 3) + self.cxt & HT_END;
            self.hash_l = self.hash_l * (5 << 5) + self.cxt & HT_END;
            self.buf[self.buf_pos] = self.cxt as u8;
            self.buf_pos += 1;
            self.cxt = 1;
            self.buf_pos &= BUF_END;

            if self.mch_len > 0 {
                self.mch_ptr += 1;
                self.mch_ptr &= BUF_END;
                if self.mch_len < MAX_LEN { self.mch_len += 1; }
            }
            else {
                self.find_or_extend_match(self.hash_s);
            }

            if self.mch_len < 2 {
                self.mch_len = 0;
                self.find_or_extend_match(self.hash_l);
            }
        }
    }
}
// ----------------------------------------------------------------------------------------------------------------------------------------


// Hash Table ------------------------------------------------------------------------------------------------------------------ Hash Table
const B: usize = 16;

struct HashTable {
    t:     Vec<u8>, // Hash table mapping index to state array
    size:  usize,   // Size of hash table in bytes
}
impl HashTable {
    fn new(n: usize) -> HashTable {
        assert!(B >= 2       && B.is_power_of_two());
        assert!(n >= (B * 4) && n.is_power_of_two());
        HashTable {
            t:     vec![0; n + B * 4 + 64],
            size:  n,
        }
    }
    // Maps context i to 2nd value of state array (" ")
    // A state array is a set of states corresponding to possible future contexts
    // [checksum, " ", 0, 1, 00, 10, 01, 11, 000, 100, 010, 110, 001, 101, 011, 111]
    fn hash(&mut self, mut i: u32) -> *mut u8 {
        i = i.wrapping_mul(123456791).rotate_right(16).wrapping_mul(234567891);
        let chksum = (i >> 24) as u8;
        let mut i = i as usize; // Convert to usize for indexing
        i = (i * B) & (self.size - B); // Restrict i to valid ht index
        if self.t[i]     == chksum { return &mut self.t[i];     }
        if self.t[i^B]   == chksum { return &mut self.t[i^B];   }
        if self.t[i^B*2] == chksum { return &mut self.t[i^B*2]; }
        if self.t[i+1] > self.t[i+1^B]
        || self.t[i+1] > self.t[i+1^B*2] { i ^= B; }
        if self.t[i+1] > self.t[i+1^B^B*2] { i ^= B ^ (B * 2); }
        for x in self.t[i..i+B].iter_mut() {
            *x = 0;
        }
        self.t[i] = chksum;
        &mut self.t[i]
    }
}
// ----------------------------------------------------------------------------------------------------------------------------------------


// Predictor -------------------------------------------------------------------------------------------------------------------- Predictor
struct Predictor {
    cxt:   u32,           // Order 0 context
    cxt4:  u32,           // Order 3 context
    bits:  usize,         // Number of bits currently in 'cxt'
    pr:    i32,           // Prediction
    h:     [u32; 6],      // Order 1, 2, 3, 4, 6, and Unigram Word contexts 
    sp:    [*mut u8; 6],  // Pointers to state within a state array
    t0:    [u8; 65_536],  // Order 1 context direct lookup table
    mm:    MatchModel,    // Model for longest context match
    ht:    HashTable,     // Hash table for mapping contexts to state arrays
    apm1:  Apm,           // Adaptive Probability Map for refining Mixer output
    apm2:  Apm,           //
    mxr:   Mixer,         // For weighted averaging of independent predictions
    s:     Stretch,       // Computes stretch(d), or ln(d/(1-d))
    sm:    Vec<StateMap>, // 6 State Maps
}
impl Predictor {
    fn new() -> Predictor {
        let mut p = Predictor {
            cxt:   1,            mm:    MatchModel::new(),
            cxt4:  0,            ht:    HashTable::new(MEM*2),
            bits:  0,            apm1:  Apm::new(256),
            pr:    2048,         apm2:  Apm::new(16384),
            h:     [0; 6],       mxr:   Mixer::new(7, 80),
            sp:    [&mut 0; 6],  s:     Stretch::new(),
            t0:    [0; 65_536],  sm:    vec![StateMap::new(256); 6],
        };
        for i in 0..6 {
            p.sp[i] = &mut p.t0[0];
        }
        p
    }

    fn p(&mut self) -> i32 {
        assert!(self.pr >= 0 && self.pr < 4096);
        self.pr
    }

    // Set state pointer 'sp[i]' to beginning of new state array
    fn update_state_ptrs(&mut self, cxt: [u32; 6], nibble: u32) {
        unsafe {
            for i in 1..6 {
                self.sp[i] = self.ht.hash(cxt[i]+nibble).add(1);
            }
        }
    }

    // Update order 1, 2, 3, 4, 6, and unigram word contexts
    fn update_cxts(&mut self, cxt: u32, cxt4: u32) {
        self.h[0] =  cxt << 8;                         // Order 1
        self.h[1] = (cxt4 & 0xFFFF) << 5 | 0x57000000; // Order 2
        self.h[2] = (cxt4 << 8).wrapping_mul(3);       // Order 3
        self.h[3] =  cxt4.wrapping_mul(5);             // Order 4
        self.h[4] =  self.h[4].wrapping_mul(11 << 5)   // Order 6
                     + cxt * 13 & 0x3FFFFFFF;
        
        match self.cxt { // Unigram Word Order
            65..=90 => {
                self.cxt += 32; // Fold to lowercase
                self.h[5] = (self.h[5] + self.cxt).wrapping_mul(7 << 3);
            }
            97..=122 => {
                self.h[5] = (self.h[5] + self.cxt).wrapping_mul(7 << 3);
            },
            _ => self.h[5] = 0,
        }
    }

    fn update(&mut self, bit: i32) {
        assert!(bit == 0 || bit == 1);

        // Transition to new states
        unsafe {
            for i in 0..6 {
                *self.sp[i] = next_state(*self.sp[i], bit);
            }
        }
        self.mxr.update(bit);

        // Update order-0 context
        self.cxt += self.cxt + bit as u32;
        self.bits += 1;

        if self.cxt >= 256 { // Byte boundary
            // Update order-3 context
            self.cxt -= 256;
            self.cxt4 = (self.cxt4 << 8) | self.cxt;
            self.update_cxts(self.cxt, self.cxt4);

            self.update_state_ptrs(self.h, 0);

            self.cxt = 1;
            self.bits = 0;
        }
        if self.bits == 4 { // Nibble boundary
            self.update_state_ptrs(self.h, self.cxt);
        }
        else if self.bits > 0 {
            // Calculate new state array index
            let j = ((bit as usize) + 1) << (self.bits & 3) - 1;
            unsafe {
                for i in 1..6 {
                    self.sp[i] = self.sp[i].add(j);
                }
            }
        }
    
        // Update order-1 context
        unsafe { 
        self.sp[0] = ((&mut self.t0[0] as *mut u8)
                     .add(self.h[0] as usize))
                     .add(self.cxt as usize);
        }
        

        // Get prediction and length from match model
        self.mm.p(bit, &mut self.mxr);
        let len = self.mm.len();
        let mut order: u32 = 0;

        // If len is 0, order is determined from 
        // number of non-zero bit histories
        if len == 0 {
            unsafe {
            if *self.sp[4] != 0 { order += 1; }
            if *self.sp[3] != 0 { order += 1; }
            if *self.sp[2] != 0 { order += 1; }
            if *self.sp[1] != 0 { order += 1; }
            }
        }
        else {
            order = 5 +
            if len >= 8  { 1 } else { 0 } +
            if len >= 12 { 1 } else { 0 } +
            if len >= 16 { 1 } else { 0 } +
            if len >= 32 { 1 } else { 0 };
        }

        // Add independent predictions to mixer 
        unsafe {
            for i in 0..6 {
                self.mxr.add(
                    self.s.stretch(
                        self.sm[i].p(bit, *self.sp[i] as i32)
                    )
                );
            }
        }

        // Set weights to be used during mixing
        self.mxr.set(order + 10 * (self.h[0] >> 13));

        // Mix
        self.pr = self.mxr.p();

        // 2 SSE stages
        self.pr = self.pr + 3 * self.apm1.p(bit, 7, self.pr, self.cxt) >> 2;
        self.pr = self.pr + 3 * self.apm2.p(bit, 7, self.pr, self.cxt ^ self.h[0] >> 2) >> 2;
    }
}
// ----------------------------------------------------------------------------------------------------------------------------------------


// Encoder ------------------------------------------------------------------------------------------------------------------------ Encoder
struct Encoder {
    high:       u32,       // Right endpoint of range
    low:        u32,       // Left endpoint of range
    predictor:  Predictor, // Generates predictions
    file_out:   BufWriter<File>, 
}
impl Encoder {
    fn new(file_out: BufWriter<File>) -> Encoder {
        let mut enc = Encoder {
            high: 0xFFFFFFFF,
            low: 0,
            predictor: Predictor::new(),
            file_out,
        };
        // Metadata placeholder
        for _ in 0..5 {
            enc.file_out.write_usize(0);
        }
        enc
    }
    fn compress_bit(&mut self, bit: i32) {
        let mut p = self.predictor.p() as u32;
        if p < 2048 { p += 1; }
        
        let range = self.high - self.low;
        let mid: u32 = self.low + (range >> 12) * p
                       + ((range & 0x0FFF) * p >> 12);
                       
        if bit == 1 {
            self.high = mid;
        }
        else {
            self.low = mid + 1;
        }
        self.predictor.update(bit);
        
        while ( (self.high ^ self.low) & 0xFF000000) == 0 {
            self.file_out.write_byte((self.high >> 24) as u8);
            self.high = (self.high << 8) + 255;
            self.low <<= 8;
        }
    }
    fn compress_block(&mut self, block: &[u8]) {
        for byte in block.iter() {
            for i in (0..=7).rev() {
                self.compress_bit(((*byte >> i) & 1) as i32);
            }
        }
    }
    // Write 40 byte header
    fn write_header(&mut self, mta: &Metadata) {
        self.file_out.get_ref().rewind().unwrap();
        self.file_out.write_usize(mta.ext);
        self.file_out.write_usize(mta.f_bl_sz);
        self.file_out.write_usize(mta.bl_sz);
        self.file_out.write_usize(mta.bl_c);
        self.file_out.write_usize(mta.f_ptr);
    }
    fn flush(&mut self) {
        while ( (self.high ^ self.low) & 0xFF000000) == 0 {
            self.file_out.write_byte((self.high >> 24) as u8);
            self.high = (self.high << 8) + 255;
            self.low <<= 8;
        }
        self.file_out.write_byte((self.high >> 24) as u8);
        self.file_out.flush_buffer();
    }
}
struct Decoder {
    high:       u32,
    low:        u32,
    predictor:  Predictor,
    file_in:    BufReader<File>,
    x:          u32, // 4 byte sliding window of compressed data
}
impl Decoder {
    fn new(file_in: BufReader<File>) -> Decoder {
        Decoder {
            high: 0xFFFFFFFF,
            low: 0,
            x: 0,
            predictor: Predictor::new(),
            file_in,
        }
    }
    fn decompress_bit(&mut self) -> i32 {
        let mut p = self.predictor.p() as u32;
        if p < 2048 { p += 1; }

        let range = self.high - self.low;
        let mid: u32 = self.low + (range >> 12) * p
                       + ((range & 0x0FFF) * p >> 12);

        let mut bit: i32 = 0;
        if self.x <= mid {
            bit = 1;
            self.high = mid;
        }
        else {
            self.low = mid + 1;
        }
        self.predictor.update(bit);
        
        while ( (self.high ^ self.low) & 0xFF000000) == 0 {
            self.high = (self.high << 8) + 255;
            self.low <<= 8;
            self.x = (self.x << 8) + self.file_in.read_byte() as u32;
        }
        bit
    }
    fn decompress_block(&mut self, block_size: usize) -> Vec<u8> {
        let mut block: Vec<u8> = Vec::with_capacity(block_size);
        while block.len() < block.capacity() {
            let mut byte: i32 = 1;
            while byte < 256 {
                byte += byte + self.decompress_bit();
            }
            byte -= 256;
            block.push(byte as u8);
        }
        block
    }
    fn read_header(&mut self) -> Metadata {
        let mut mta: Metadata = Metadata::new();
        mta.ext =     self.file_in.read_usize();
        mta.f_bl_sz = self.file_in.read_usize();
        mta.bl_sz =   self.file_in.read_usize();
        mta.bl_c =    self.file_in.read_usize();
        mta.f_ptr =   self.file_in.read_usize();
        mta
    }
    // Inititialize decoder with first 4 bytes of compressed data
    fn init_x(&mut self) {
        for _ in 0..4 {
            self.x = (self.x << 8) + self.file_in.read_byte() as u32;
        }
    }
}
// ----------------------------------------------------------------------------------------------------------------------------------------


#[derive(Debug)]
struct Metadata {
    ext:      usize, // Extension
    f_bl_sz:  usize, // Final block size
    bl_sz:    usize, // Block size
    bl_c:     usize, // Block count

    // Solid archives only ---------------
    // Path, block_count, final_block_size
    files:  Vec<(String, usize, usize)>,     
    f_ptr:  usize, // Pointer to 'files'
}
impl Metadata {
    fn new() -> Metadata {
        Metadata {
            ext:      0,
            f_bl_sz:  0,
            bl_sz:    1 << 20,
            bl_c:     0,
            files:    Vec::new(),
            f_ptr:    0,
        }
    }
    // Set metadata extension field
    fn set_ext(&mut self, path: &Path) {
        for byte in path.extension().unwrap()
        .to_str().unwrap().as_bytes().iter().rev() {
            self.ext = (self.ext << 8) | *byte as usize;
        }
    }
    // Get metadata extension field
    fn get_ext(&self) -> String {
        String::from_utf8(
            self.ext.to_le_bytes().iter().cloned()
            .filter(|&i| i != 0).collect()
        ).unwrap()
    }
}


// Get file name or path without extension
fn file_name_no_ext(path: &Path) -> &str {
    path.file_name().unwrap()
    .to_str().unwrap()
    .split(".").next().unwrap()
}
fn file_path_no_ext(path: &Path) -> &str {
    path.to_str().unwrap()
    .split(".").next().unwrap()
}
// Get file name or path with extension 
fn file_name_ext(path: &Path) -> &str {
    path.file_name().unwrap()
    .to_str().unwrap()
}
fn file_path_ext(path: &Path) -> String {
    path.to_str().unwrap().to_string()
}
fn file_len(path: &Path) -> u64 {
    path.metadata().unwrap().len()
}


// Non-solid archiving --------------------------------------------------------------------------------------------------------------------
fn compress_file(file_in_path: &Path, dir_out: &String) -> u64 {
    let mut mta: Metadata = Metadata::new();
    
    // Create output file path from current output directory
    // and input file name without extension
    // i.e. foo/bar.txt -> foo/bar.pri
    let file_out_path = 
        PathBuf::from(
            &format!("{}\\{}.pri",  
            dir_out, file_name_no_ext(file_in_path))
        );  

    // Create input file with buffer = block size
    let mut file_in = new_input_file(mta.bl_sz, file_in_path);
    let mut enc = Encoder::new(new_output_file(4096, &file_out_path));
    
    // Set metadata extension field
    mta.set_ext(file_in_path);

    // Compress
    loop {
        if file_in.fill_buffer() == BufferState::Empty { break; }
        mta.f_bl_sz = file_in.buffer().len();
        enc.compress_block(&file_in.buffer());
        mta.bl_c += 1;
    } 
    
    enc.flush();
    enc.write_header(&mta);
    file_len(&file_out_path)
}
fn decompress_file(file_in_path: &Path, dir_out: &String) -> u64 {
    let mut dec = Decoder::new(new_input_file(4096, file_in_path));
    let mta: Metadata = dec.read_header();

    // Create output file path from current output directory,
    // input file name without extension, and file's original
    // extension (stored in header)
    // i.e. foo/bar.pri -> foo/bar.txt
    let file_out_path =
        PathBuf::from(
            &format!("{}\\{}.{}",
            dir_out,
            file_name_no_ext(file_in_path),
            mta.get_ext())
        );
    let mut file_out = new_output_file(4096, &file_out_path);
    
    // Call after reading header
    dec.init_x();

    // Decompress
    for _ in 0..(mta.bl_c - 1) {
        let block = dec.decompress_block(mta.bl_sz);
        for byte in block.iter() {
            file_out.write_byte(*byte);
        }
    }
    let block = dec.decompress_block(mta.f_bl_sz);
    for byte in block.iter() {
        file_out.write_byte(*byte);
    }

    file_out.flush_buffer();
    file_len(&file_out_path)
}
fn compress_dir(dir_in: &Path, dir_out: &mut String) {
    // Create new nested directory from current output 
    // directory and input directory name 
    let mut dir_out = 
        format!("{}\\{}", 
        dir_out, file_name_ext(dir_in));
    new_dir(&dir_out);

    // Sort files and directories
    let (files, dirs): (Vec<PathBuf>, Vec<PathBuf>) = 
        dir_in.read_dir().unwrap()
        .map(|d| d.unwrap().path())
        .partition(|f| f.is_file());

    // Compress files first, then directories
    for file_in in files.iter() {
        let time = Instant::now();
        println!("Compressing {}", file_in.display());
        let file_in_size  = file_len(&file_in); 
        let file_out_size = compress_file(&file_in, &dir_out);
        println!("{} bytes -> {} bytes in {:.2?}\n", 
            file_in_size, file_out_size, time.elapsed());  
    }
    for dir_in in dirs.iter() {
        compress_dir(dir_in, &mut dir_out); 
    }
}
fn decompress_dir(dir_in: &Path, dir_out: &mut String, root: bool) {
    // Create new nested directory from current output 
    // directory and input directory name; if current output
    // directory is root, replace rather than nest
    let mut dir_out = 
        if root { dir_out.to_string() }
        else { 
            format!("{}\\{}", 
            dir_out, file_name_ext(dir_in)) 
        };
    if !Path::new(&dir_out).is_dir() {
        new_dir(&dir_out);
    }
    
    // Sort files and directories
    let (files, dirs): (Vec<PathBuf>, Vec<PathBuf>) =
        dir_in.read_dir().unwrap()
        .map(|d| d.unwrap().path())
        .partition(|f| f.is_file());

    // Decompress files first, then directories
    for file_in in files.iter() {
        let time = Instant::now();
        println!("Decompressing {}", file_in.display());
        let file_in_size  = file_len(&file_in);
        let file_out_size = decompress_file(&file_in, &dir_out);
        println!("{} bytes -> {} bytes in {:.2?}\n",
            file_in_size, file_out_size, time.elapsed());
    }
    for dir_in in dirs.iter() {
        decompress_dir(&dir_in, &mut dir_out, false); 
    }
}
// ----------------------------------------------------------------------------------------------------------------------------------------


// Solid archiving ------------------------------------------------------------------------------------------------------------------------
enum Sort {   // Sort By:
    None,     // No sorting
    Ext,      // Extension
    PrtDir,   // Parent Directory
    Created,  // Creation Time
    Accessed, // Last Access Time
    Modified, // Last Modification Time
}
fn sort_ext(f1: &String, f2: &String) -> Ordering {
        (Path::new(f1).extension().unwrap())
    .cmp(Path::new(f2).extension().unwrap())
}
fn sort_prtdir(f1: &String, f2: &String) -> Ordering {
        (Path::new(f1).parent().unwrap())
    .cmp(Path::new(f2).parent().unwrap())
}
fn sort_created(f1: &String, f2: &String) -> Ordering {
         (Path::new(f1).metadata().unwrap().created().unwrap())
    .cmp(&Path::new(f2).metadata().unwrap().created().unwrap())
}
fn sort_accessed(f1: &String, f2: &String) -> Ordering {
         (Path::new(f1).metadata().unwrap().accessed().unwrap())
    .cmp(&Path::new(f2).metadata().unwrap().accessed().unwrap())
}
fn sort_modified(f1: &String, f2: &String) -> Ordering {
         (Path::new(f1).metadata().unwrap().modified().unwrap())
    .cmp(&Path::new(f2).metadata().unwrap().modified().unwrap())
}
fn collect_files(dir_in: &Path, mta: &mut Metadata) {
    let (files, dirs): (Vec<PathBuf>, Vec<PathBuf>) =
        dir_in.read_dir().unwrap()
        .map(|d| d.unwrap().path())
        .partition(|f| f.is_file());

    for file in files.iter() {
        mta.files.push(
            (file_path_ext(file), 0, 0)
        );
    }
    if !dirs.is_empty() {
        for dir in dirs.iter() {
            collect_files(dir, mta);
        }
    }
}
fn compress_file_solid(enc: &mut Encoder, mta: &mut Metadata, curr_file: usize) {
    // Create input file with buffer = block size
    let mut file_in = new_input_file(mta.bl_sz, Path::new(&mta.files[curr_file].0));

    // Compress
    loop {
        if file_in.fill_buffer() == BufferState::Empty { break; }
        mta.files[curr_file].2 = file_in.buffer().len();
        enc.compress_block(&file_in.buffer());
        mta.files[curr_file].1 += 1;
    }
    println!("Total archive size: {}\n", 
    enc.file_out.stream_position().unwrap());
}
fn decompress_file_solid(dec: &mut Decoder, mta: &mut Metadata, dir_out: &String, curr_file: usize) {
    let file_out_name =
        format!("{}\\{}",
            dir_out,
            file_name_ext(
                Path::new(&mta.files[curr_file].0)
            ),
        );
    let mut file_out = new_output_file(4096, Path::new(&file_out_name));

    // Decompress
    for _ in 0..((mta.files[curr_file].1) - 1) {
        let block = dec.decompress_block(mta.bl_sz);
        for byte in block.iter() {
            file_out.write_byte(*byte);
        }
    }
    let block = dec.decompress_block(mta.files[curr_file].2);
    for byte in block.iter() {
        file_out.write_byte(*byte);
    }
    file_out.flush_buffer();
}
// ----------------------------------------------------------------------------------------------------------------------------------------


fn main() {
    // Get arguments, skipping over program name
    let mut args = env::args().skip(1).peekable();

    // Print program info
    if args.peek() == None {
        println!();
        println!("      ______   ______     ________  ______    ________  ______    __   __     
     /_____/\\ /_____/\\   /_______/\\/_____/\\  /_______/\\/_____/\\  /_/\\ /_/\\    
     \\:::_ \\ \\\\:::_ \\ \\  \\__.::._\\/\\::::_\\/_ \\__.::._\\/\\:::_ \\ \\ \\:\\ \\\\ \\ \\   
      \\:(_) \\ \\\\:(_) ) )_   \\::\\ \\  \\:\\/___/\\   \\::\\ \\  \\:(_) ) )_\\:\\ \\\\ \\ \\  
       \\: ___\\/ \\: __ `\\ \\  _\\::\\ \\__\\_::._\\:\\  _\\::\\ \\__\\: __ `\\ \\\\:\\_/.:\\ \\ 
        \\ \\ \\    \\ \\ `\\ \\ \\/__\\::\\__/\\ /____\\:\\/__\\::\\__/\\\\ \\ `\\ \\ \\\\ ..::/ / 
         \\_\\/     \\_\\/ \\_\\/\\________\\/ \\_____\\/\\________\\/ \\_\\/ \\_\\/ \\___/_(  
                                                                                 ");
        println!();
        println!("Prisirv is a context mixing archiver based on lpaq1");
        println!("Source code available at https://github.com/aufdj/prisirv");
        println!();
        println!("USAGE: PROG_NAME [c|d] [-out [path]] [-sld] [-sort [..]] [files|dirs]");
        println!();
        println!("OPTIONS:");
        println!("   c      Compress");
        println!("   d      Decompress");
        println!("  -out    Specify output path");
        println!("  -sld    Create solid archive");
        println!("  -sort   Sort files (solid archives only)");
        println!();
        println!("      Sorting Methods:");
        println!("          -sort ext      Sort by extension");
        println!("          -sort prtdir   Sort by parent directory");
        println!("          -sort crtd     Sort by creation time");
        println!("          -sort accd     Sort by last access time");
        println!("          -sort mod      Sort by last modification time");
        println!();
        println!("EXAMPLE:");
        println!("  Compress file [\\foo\\bar.txt] and directory [baz] into solid archive [\\foo\\arch], \n  sorting files by creation time:");
        println!();
        println!("      prisirv c -out arch -sld -sort crtd \\foo\\bar.txt \\baz");
        println!();
        println!("  Decompress the archive:");
        println!();
        println!("      prisirv d -sld \\foo.pri");
        std::process::exit(0);
    }

    // Get mode
    let mode = args.next().unwrap();

    // Get user specified output path
    let mut user_out = String::new();
    if args.peek().unwrap() == "-out" {
        args.next();
        user_out = args.next().unwrap();
    }

    // Determine if solid or non-solid archive
    let mut solid = false;
    if args.peek().unwrap() == "-sld" { 
        solid = true;
        args.next();
    }

    // Select sorting option
    let mut sort = Sort::None;
    if args.peek().unwrap() == "-sort" { 
        args.next();
        sort = match args.next().unwrap().as_str() {
            "ext"    => Sort::Ext,
            "prtdir" => Sort::PrtDir,
            "crtd"   => Sort::Created,
            "accd"   => Sort::Accessed,
            "mod"    => Sort::Modified,
            _ => { 
                println!("Not a valid sort criteria.");
                std::process::exit(1);
            }
        }
    }

    let mut dir_out = String::new();

    if solid {
        let mut enc;
        let mut dec;
        let mut mta: Metadata = Metadata::new();
        match mode.as_str() {
            "c" => {
                let arg = PathBuf::from(&args.peek().unwrap());
                if arg.is_file() || arg.is_dir() {
                    // Use first input file name if user doesn't specify a path
                    if user_out.is_empty() {
                        dir_out = format!("{}.pri", file_path_no_ext(&arg));
                    }
                    else {
                        // An -out option containing \'s will be treated as an absolute path.
                        // An -out option with no \'s creates a new archive inside the same directory as the first input.
                        // i.e. Compressing \foo\bar.txt with option '-out \baz\arch' creates archive \baz\arch,
                        // while option '-out arch' creates archive \foo\arch.
                        if user_out.contains("\\") {
                            dir_out = format!("{}.pri", user_out);
                        }
                        else {
                            let s: Vec<String> = 
                                file_path_ext(&arg)
                                .split("\\").skip(1)
                                .map(|s| s.to_string())
                                .collect();
                            for i in s.iter().take(s.len()-1) {
                                dir_out.push_str(format!("\\{}", i).as_str());
                            }
                            dir_out = format!("{}\\{}.pri", dir_out, user_out);
                        }   
                    }
                    enc = Encoder::new(new_output_file(4096, Path::new(&dir_out)));
                }
                else {
                    println!("No files or directories found.");
                    std::process::exit(1);
                }
                
                // Sort files and directories
                let (files, dirs): (Vec<PathBuf>, Vec<PathBuf>) =
                    args.map(|f| PathBuf::from(f))
                    .partition(|f| f.is_file());

                // Add file paths and lengths to metadata
                for file in files.iter() {
                    mta.files.push(
                        (file_path_ext(file), 0, 0)
                    );
                }
                
                // Walk through directories and gather rest of files
                if !dirs.is_empty() {
                    for dir in dirs.iter() {
                        collect_files(dir, &mut mta);
                    }
                }

                // Sort files to potentially improve compression of solid archives
                match sort {
                    Sort::None     => {},
                    Sort::Ext      => mta.files.sort_by(|f1, f2| sort_ext(&f1.0, &f2.0)),
                    Sort::PrtDir   => mta.files.sort_by(|f1, f2| sort_prtdir(&f1.0, &f2.0)),
                    Sort::Created  => mta.files.sort_by(|f1, f2| sort_created(&f1.0, &f2.0)),
                    Sort::Accessed => mta.files.sort_by(|f1, f2| sort_accessed(&f1.0, &f2.0)),
                    Sort::Modified => mta.files.sort_by(|f1, f2| sort_modified(&f1.0, &f2.0)),
                }
                
                for curr_file in 0..mta.files.len() {
                    println!("Compressing {}", mta.files[curr_file].0);
                    compress_file_solid(&mut enc, &mut mta, curr_file);
                }
                enc.flush();
                
                // Get index to end of file metadata
                mta.f_ptr =
                    enc.file_out.stream_position()
                    .unwrap() as usize;

                // Output number of files
                enc.file_out.write_usize(mta.files.len());

                for file in mta.files.iter() {
                    // Get path as byte slice, truncated if longer than 255 bytes
                    let path = &file.0.as_bytes()[..min(file.0.len(), 255)];

                    // Output length of file path (for parsing)
                    enc.file_out.write_byte(path.len() as u8);

                    // Output path
                    for byte in path.iter() {
                        enc.file_out.write_byte(*byte);
                    }

                    // Output block count and final block size
                    enc.file_out.write_usize(file.1);
                    enc.file_out.write_usize(file.2);
                }

                // Go back to beginning of file and write header
                enc.file_out.rewind().unwrap();
                enc.write_header(&mta);
            }
            "d" => {
                let arg = PathBuf::from(&args.peek().unwrap());
                if arg.is_file() {
                    // Use first input file name if user doesn't specify a path
                    if user_out.is_empty() {
                        dir_out = format!("{}_d", file_path_no_ext(&arg));
                    }
                    else {
                        if user_out.contains("\\") {
                            dir_out = format!("{}_d", user_out);
                        }
                        else {
                            let s: Vec<String> = 
                                file_path_ext(&arg)
                                .split("\\").skip(1)
                                .map(|s| s.to_string())
                                .collect();
                            for i in s.iter().take(s.len()-1) {
                                dir_out.push_str(format!("\\{}", i).as_str());
                            }
                            dir_out = format!("{}\\{}_d", dir_out, user_out);
                        }   
                    }
                    new_dir(&dir_out);
                    dec = Decoder::new(new_input_file(4096, &arg));
                }
                else {
                    println!("No solid archives found.");
                    std::process::exit(1);
                }

                mta = dec.read_header();

                // Seek to end of file metadata
                dec.file_in.seek(SeekFrom::Start(mta.f_ptr as u64)).unwrap();

                // Parse files and lengths
                let mut path: Vec<u8> = Vec::new();

                // Get number of files
                let num_files = dec.file_in.read_usize();

                for _ in 0..num_files {
                    // Get length of next path
                    let len = dec.file_in.read_byte();

                    // Get path, block count, final block size
                    for _ in 0..len {
                        path.push(dec.file_in.read_byte());
                    }
                    mta.files.push(
                        (path.iter().cloned()
                            .map(|b| b as char)
                            .collect::<String>(),
                         dec.file_in.read_usize(),
                         dec.file_in.read_usize())
                    );
                    path.clear();
                }

                // Seek back to beginning of compressed data
                dec.file_in.seek(SeekFrom::Start(40)).unwrap();

                dec.init_x();
                
                for curr_file in 0..mta.files.len() {
                    println!("Decompressing {}", mta.files[curr_file].0);
                    decompress_file_solid(&mut dec, &mut mta, &dir_out, curr_file);
                }
            }
            _ => {
                println!("To Compress: c input");
                println!("To Decompress: d input");
            }
        }
    }
    else {
        match mode.as_str() {
            "c" => {
                // Create archive with same name as first file/dir
                let arg = PathBuf::from(&args.peek().unwrap());
                if arg.is_file() || arg.is_dir() {
                    // Use first input file name if user doesn't specify a path
                    if user_out.is_empty() {
                        dir_out = format!("{}_pri", file_path_no_ext(&arg));
                    }
                    else {
                        if user_out.contains("\\") {
                            dir_out = format!("{}_pri", user_out);
                        }
                        else {
                            let s: Vec<String> = 
                                file_path_ext(&arg)
                                .split("\\").skip(1)
                                .map(|s| s.to_string())
                                .collect();
                            for i in s.iter().take(s.len()-1) {
                                dir_out.push_str(format!("\\{}", i).as_str());
                            }
                            dir_out = format!("{}\\{}_pri", dir_out, user_out);
                        }   
                    }
                    new_dir(&dir_out);
                }
                else {
                    println!("No files or directories found.");
                    std::process::exit(1);
                }

                let (files, dirs): (Vec<PathBuf>, Vec<PathBuf>) = 
                    args.map(|f| PathBuf::from(f))
                    .partition(|f| f.is_file());
            
                for file_in in files.iter() {
                    let time = Instant::now();
                    println!("Compressing {}", file_in.display());
                    let file_in_size  = file_len(&file_in); 
                    let file_out_size = compress_file(&file_in, &dir_out);
                    println!("{} bytes -> {} bytes in {:.2?}\n", 
                    file_in_size, file_out_size, time.elapsed());  
                }
                for dir_in in dirs.iter() {
                    compress_dir(dir_in, &mut dir_out);      
                }
            }
            "d" => {
                // Create archive with same name as first file/dir
                let arg = PathBuf::from(&args.peek().unwrap());
                if arg.is_file() || arg.is_dir() {
                    // Use first input file name if user doesn't specify a path
                    if user_out.is_empty() {
                        dir_out = format!("{}_d", file_path_no_ext(&arg));
                    }
                    else {
                        if user_out.contains("\\") {
                            dir_out = format!("{}_d", user_out);
                        }
                        else {
                            let s: Vec<String> = 
                                file_path_ext(&arg)
                                .split("\\").skip(1)
                                .map(|s| s.to_string())
                                .collect();
                            for i in s.iter().take(s.len()-1) {
                                dir_out.push_str(format!("\\{}", i).as_str());
                            }
                            dir_out = format!("{}\\{}_d", dir_out, user_out);
                        }   
                    }
                    println!("creating directory {} in main()", dir_out);
                    new_dir(&dir_out);
                }
                else {
                    println!("No files or directories found.");
                    std::process::exit(1);
                }

                let (files, dirs): (Vec<PathBuf>, Vec<PathBuf>) = 
                    args.map(|f| PathBuf::from(f))
                    .partition(|f| f.is_file());
            
                for file_in in files.iter() {
                    let time = Instant::now();
                    println!("Decompressing {}", file_in.display());
                    let file_in_size  = file_len(&file_in); 
                    let file_out_size = decompress_file(&file_in, &dir_out);
                    println!("{} bytes -> {} bytes in {:.2?}\n", 
                    file_in_size, file_out_size, time.elapsed());  
                }
                for dir_in in dirs.iter() {
                    decompress_dir(dir_in, &mut dir_out, true);      
                }
            }
            _ => {
                println!("To Compress: c input");
                println!("To Decompress: d input");
            }
        }
    }        
}






