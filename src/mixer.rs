use crate::logistic::squash;

/// Predictions are combined using a neural network (Mixer). The inputs p_i, 
/// i=0..6 are first stretched: t_i = log(p_i/(1 - p_i)), then the output is 
/// computed: p = squash(SUM_i t_i * w_i), where squash(x) = 1/(1 + exp(-x)) 
/// is the inverse of stretch().  The weights are adjusted to reduce the 
/// error: w_i := w_i + L * t_i * (y - p) where (y - p) is the prediction 
/// error and L ~ 0.002 is the learning rate. This is a standard single layer 
/// backpropagation network modified to minimize coding cost rather than RMS 
/// prediction error (thus dropping the factors p * (1 - p) from learning).
pub struct Mixer {
    max_in:   usize,    // Maximum number of inputs
    inputs:   Vec<i32>, // Current inputs
    weights:  Vec<i32>, // Weights used for weighted averaging
    wht_set:  usize,    // Single set of weights chosen by a context
    pr:       i32,      // Current prediction
}
impl Mixer {
    /// Create a new Mixer with m sets of n weights.
    pub fn new(n: usize, m: usize) -> Mixer {
        Mixer {
            max_in:   n,                     
            inputs:   Vec::with_capacity(n), 
            weights:  vec![0; n * m],        
            wht_set:  0,                     
            pr:       2048,                  
        }
    }

    /// Add an input prediction to the Mixer.
    pub fn add(&mut self, pr: i32) {
        assert!(self.inputs.len() < self.inputs.capacity());
        self.inputs.push(pr);
    }

    /// Choose the set of weights to be used for averaging.
    pub fn set(&mut self, cxt: u32) {
        self.wht_set = (cxt as usize) * self.max_in; 
    }

    /// Compute weighted average of input predictions.
    pub fn p(&mut self) -> i32 {
        let d = dot_product(&self.inputs[..], &self.weights[self.wht_set..]);
        self.pr = squash(d);
        self.pr
    }

    /// Update weights based on prediction error.
    pub fn update(&mut self, bit: i32) {
        let error: i32 = ((bit << 12) - self.pr) * 7;
        assert!(error >= -32768 && error < 32768);
        train(&self.inputs[..], &mut self.weights[self.wht_set..], error);
        self.inputs.clear();
    }
}

/// Update weights based on prediction error.
fn train(inputs: &[i32], weights: &mut [i32], error: i32) {
    for (input, weight) in inputs.iter().zip(weights.iter_mut()) {
        *weight += ((*input * error) + 0x8000) >> 16;
    }
}

/// Compute dot product.
fn dot_product(inputs: &[i32], weights: &[i32]) -> i32 {
    (inputs.iter().zip(weights.iter())
    .map(|(i, w)| i * w).sum::<i32>()) >> 16
}
