use std::{collections::HashMap, path::Path, sync::Arc};

use dasp_sample::Sample as Sm;

/// SamplePlayer contains a sample reference and the information
/// which is required to play it
#[derive(Debug, PartialEq, Clone)]
pub struct SamplePlayer {
    pub sample: Arc<Sample>,
    start: u128,
    current: u128,
    speed: f32,
}

/// Sample contains the name and data of a single sample
#[derive(Debug, PartialEq)]
pub struct Sample {
    pub name: String,
    data: Vec<f32>,
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

        for s in data.samples().step_by(spec as usize) {
            let s: i16 = s.unwrap();
            let converted: f32 = s.to_sample();
            samples.push(converted)
        }

        Some(Self {
            name: name.to_string(),
            data: samples,
        })
    }
}

fn interpolate(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

impl SamplePlayer {
    pub fn get_sample(&mut self, time: u128) -> f32 {
        // HACK: this is a really hacky way to implement sampleplayback
        //       should probably keep track of the starting time of current playback
        if time > (self.current + 1) {
            self.start = time;
        }

        self.current = time;

        let index = (time as i128 - self.start as i128) as f32 * self.speed;
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

        // WARN: just return interpolate output
        interpolate(low, high, t)
    }

    pub fn new(sample: Arc<Sample>) -> Self {
        Self {
            sample,
            start: 0,
            current: 0,
            speed: 0.8,
        }
    }
}
