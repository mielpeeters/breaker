/*!
* Implement post processing effects like filters, distortion, chorus, reverberation, delay, etc.
*/
include!(concat!(env!("OUT_DIR"), "/data/reverb.rs"));

/// Defines the interface for a post processing effect.
pub enum Effect {
    FIR(FIR),
    Reverb(Reverb),
}

/// A simple low pass FIR filter.
pub struct FIR {
    /// The filter coefficients.
    coeffs: Vec<f32>,
    /// The filter state.
    state: Vec<f32>,
}

/// Builder to create different types of FIR filters.
pub struct FIRBuilder {
    coeffs: Vec<f32>,
}

pub struct Reverb {
    state: Vec<f32>,
}

impl FIRBuilder {
    pub fn new() -> Self {
        Self { coeffs: vec![] }
    }

    pub fn low_pass(mut self, cutoff: f32, sample_rate: f32) -> Self {
        let n = 2.0 * sample_rate / cutoff;
        let n = n as usize;
        let mut coeffs = vec![0.0; n];
        // TODO: check if everything time-wise (sample distance etc) is correct here
        // TODO: apply hamming window
        //       -> find out how they relate to the cutoff and sample frequency
        for (i, item) in coeffs.iter_mut().enumerate() {
            let x = (n - i) as f32 * cutoff / sample_rate;
            *item = x.sin() / (n as f32 * x);
        }
        self.coeffs = coeffs;
        self
    }

    pub fn build(&self) -> FIR {
        FIR::new(self.coeffs.clone())
    }
}

impl Default for FIRBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl FIR {
    /// Create a new FIR filter with the given coefficients.
    fn new(coeffs: Vec<f32>) -> Self {
        let state = vec![0.0; coeffs.len()];
        Self { coeffs, state }
    }

    /// Process a single sample.
    pub fn process(&mut self, input: f32) -> f32 {
        let mut output = 0.0;
        self.state.insert(0, input);
        self.state.pop();
        for i in 0..self.coeffs.len() {
            output += self.coeffs[i] * self.state[i];
        }
        output
    }
}

impl Reverb {
    pub fn new() -> Self {
        Self {
            state: vec![0.0; REVERB_SIZE],
        }
    }

    pub fn process(&mut self, input: f32) -> f32 {
        let mut output = 0.0;
        self.state.insert(0, input);
        self.state.pop();
        for i in 0..REVERB_SIZE {
            output += REVERB_MASK[i] * self.state[i];
        }
        output
    }
}
