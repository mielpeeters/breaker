/*!
* Implement post processing effects like filters, distortion, chorus, reverberation, delay, etc.
*/

use std::f32::consts::PI;
/// Defines the interface for a post processing effect.
pub enum Effect {
    FIR(FIR),
    Reverb(Reverb),
    Gain(Gain),
    Compressor(Compressor),
}

/// Arbitrary maximum length for FIR filters.
const MAX_FIR_LENGTH: usize = 100;

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

#[allow(unused)]
pub struct Reverb {
    state: f32,
}

pub struct Gain {
    amount: f32,
}

pub struct Compressor {
    ratio: f32,
    threshold: f32,
    energy: AudioEnergy,
    current: f32,
}

pub struct AudioEnergy {
    state: Vec<f32>,
    energy: f32,
}

impl FIRBuilder {
    pub fn new() -> Self {
        Self { coeffs: vec![] }
    }

    pub fn low_pass(mut self, cutoff: f32, sample_rate: f32) -> Self {
        let n = 2.0 * sample_rate / cutoff;
        let mut n = n as usize;
        n = n.min(MAX_FIR_LENGTH);
        let mut coeffs = vec![0.0; n];
        for (i, item) in coeffs.iter_mut().enumerate() {
            let x = i as f32 * cutoff / sample_rate;
            if x == 0.0 {
                *item = 1.0;
                continue;
            }
            *item = x.sin() / (x);
            // Hann window
            *item *= (PI * i as f32 / n as f32).sin().powi(2);
        }

        self.coeffs = coeffs;
        self
    }

    pub fn high_pass(mut self, cutoff: f32, sample_rate: f32) -> Self {
        let n = 2.0 * sample_rate / cutoff;
        let mut n = n as usize;
        n = n.min(MAX_FIR_LENGTH);
        let mut coeffs = vec![0.0; n];
        for (i, item) in coeffs.iter_mut().enumerate() {
            let x = i as f32 * cutoff / sample_rate;
            if x == 0.0 {
                *item = 1.0;
                continue;
            }
            *item = (-1.0_f32).powi(i as i32) * x.sin() / x;
            // Hann window
            // *item *= (PI * i as f32 / n as f32).sin().powi(2);
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
        Self { state: 0.0 }
    }

    pub fn process(&mut self, input: f32) -> f32 {
        // TODO: implement a proper reverb algorithm
        input
    }
}

impl Default for Reverb {
    fn default() -> Self {
        Self::new()
    }
}

impl Gain {
    pub fn new(amount: f32) -> Self {
        Self { amount }
    }

    pub fn process(&mut self, input: f32) -> f32 {
        input * self.amount
    }
}

impl AudioEnergy {
    fn new(len: usize) -> Self {
        // maybe change length to be a parameter
        Self {
            state: vec![0.0; len],
            energy: 0.0,
        }
    }

    #[allow(unused)]
    fn add(&mut self, input: f32) {
        self.state.insert(0, input);
        self.energy += input.powi(2);
        let out = self.state.pop();
        self.energy -= out.unwrap().powi(2);
    }

    /// Returns decibels of energy in the state.
    fn add_and_get(&mut self, input: f32) -> f32 {
        self.add(input);
        10.0 * (100.0 * self.energy / self.state.len() as f32).log10()
    }
}

impl Compressor {
    pub fn new(ratio: f32, threshold: f32, len: usize) -> Self {
        Self {
            ratio,
            threshold,
            energy: AudioEnergy::new(len),
            current: 1.0,
        }
    }

    pub fn process(&mut self, input: f32) -> f32 {
        // TODO: implement a proper compressor algorithm
        input
    }
}
