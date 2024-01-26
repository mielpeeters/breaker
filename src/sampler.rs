use std::{collections::HashMap, path::Path, sync::Arc};

use dasp_sample::Sample as Sm;

/// SamplePlayer contains a sample reference and the information
/// which is required to play it
#[derive(Debug, PartialEq, Clone)]
pub struct SamplePlayer {
    pub sample: Arc<Sample>,
    start: u128,
    speed: f32,
}

/// Sample contains the name and data of a single sample
#[derive(Debug, PartialEq)]
pub struct Sample {
    pub name: String,
    data: Vec<f32>,
    sample_rate: u32,
}

#[derive(Debug)]
pub struct SampleSet {
    pub samples: HashMap<String, Arc<Sample>>,
}

impl Sample {
    pub fn try_new(file: &Path) -> Option<Self> {
        let name = file.file_name().unwrap().to_str().unwrap();
        let Ok(mut data) = hound::WavReader::open(file) else {
            return None;
        };
        let mut samples = vec![];

        let spec = data.spec().channels;
        let sample_rate = data.spec().sample_rate;

        for s in data.samples().step_by(spec as usize) {
            let s: i16 = s.unwrap();
            let converted: f32 = s.to_sample();
            samples.push(converted)
        }

        Some(Self {
            name: name.to_string(),
            data: samples,
            sample_rate,
        })
    }
}

fn interpolate(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

impl SamplePlayer {
    pub fn get_sample(&mut self, time: u128, sample_rate: u32) -> f32 {
        // NOTE: there may be better ways to interpolate than just linear interpolation

        // index of the destination sample rate
        let mut index = (time as i128 - self.start as i128) as f32 * self.speed;
        // if the sample rate is different, we need to adjust the index
        if sample_rate != self.sample.sample_rate {
            index *= self.sample.sample_rate as f32 / sample_rate as f32;
        }

        // index at this point is a float, so we need to interpolate between two samples, which we
        // will call low and high
        let index_low = index.floor() as usize;
        let t = index.fract();

        let low = match self.sample.data.get(index_low) {
            Some(s) => *s,
            None => 0.0,
        };

        // no need to interpolate if the target time falls on the grid
        if t < 0.0001 {
            return low;
        }

        let high = match self.sample.data.get(index_low + 1) {
            Some(s) => *s,
            None => 0.0,
        };

        // no need to interpolate if the target time falls on the grid
        if t > 0.9999 {
            return high;
        }

        interpolate(low, high, t)
    }

    /// Hit this sample, i.e. reset the start time
    pub fn hit(&mut self, time: u128) {
        self.start = time;
    }

    pub fn new(sample: Arc<Sample>) -> Self {
        Self {
            sample,
            start: 0,
            speed: 0.8,
        }
    }
}
