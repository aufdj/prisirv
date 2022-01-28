use crate::logistic::squash;

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

pub struct Mixer {
    max_in:   usize,    // Maximum number of inputs
    inputs:   Vec<i32>, // Current inputs
    weights:  Vec<i32>, // Weights used for weighted averaging
    wht_set:  usize,    // Single set of weights chosen by a context
    pr:       i32,      // Current prediction
}
impl Mixer {
    pub fn new(n: usize, m: usize) -> Mixer {
        Mixer {
            max_in:   n,                     
            inputs:   Vec::with_capacity(n), 
            weights:  vec![0; n * m],        
            wht_set:  0,                     
            pr:       2048,                  
        }
    }
    pub fn add(&mut self, pr: i32) {
        assert!(self.inputs.len() < self.inputs.capacity());
        self.inputs.push(pr);
    }
    pub fn set(&mut self, cxt: u32) {
        // Calculate set of weights to be used for mixing
        self.wht_set = (cxt as usize) * self.max_in; 
    }
    pub fn p(&mut self) -> i32 {
        let d = dot_product(&self.inputs[..], &self.weights[self.wht_set..]);
        self.pr = squash(d);
        self.pr
    }
    pub fn update(&mut self, bit: i32) {
        let error: i32 = ((bit << 12) - self.pr) * 7;
        assert!(error >= -32768 && error < 32768);
        train(&self.inputs[..], &mut self.weights[self.wht_set..], error);
        self.inputs.clear();
    }
}
// ----------------------------------------------------------------------------------------------------------------------------------------
