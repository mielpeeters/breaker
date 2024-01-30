use std::{
    collections::HashMap,
    error::Error,
    fs,
    sync::{
        mpsc::{self, Receiver, SendError, SyncSender},
        Arc,
    },
    time::{SystemTime, UNIX_EPOCH},
};

use crate::{
    grid::Grid,
    postproc::{Effect, FIRBuilder, Gain},
    sampler::{Sample, SampleSet},
    util::FromNode,
};

const PLAYABLES: [&str; 1] = ["grid"];

#[derive(Debug)]
pub enum Playable {
    Grid(Grid),
}

pub struct Pipeline {
    pub playables: HashMap<String, Playable>,
    effects: HashMap<String, Vec<Effect>>,
    pub mix: HashMap<String, f32>,
    pub time: u128,
    bar_length: u128,
    sample_rate: u32,
    sink: SyncSender<f32>,
    next: Option<Box<Pipeline>>,
}

pub struct PipelineConfig {
    pub samples_dir: String,
}

fn get_samples(config: &PipelineConfig) -> HashMap<String, Arc<Sample>> {
    let mut samples = HashMap::new();

    let Ok(paths) = fs::read_dir(config.samples_dir.clone()) else {
        log::info!("No samples directory found, skipping sample loading");
        return samples;
    };

    for path in paths {
        let path = path.unwrap();

        let name = path.path();
        let name = name.file_stem().unwrap().to_str().unwrap();

        let Some(sample) = Sample::try_new(&path.path()) else {
            continue;
        };
        let sample = Arc::new(sample);

        samples.insert(name.to_string(), sample);
    }

    samples
}

fn rescale_mix(mix: &mut HashMap<String, f32>) {
    let total: f32 = mix.values().sum();

    for (_name, value) in mix.iter_mut() {
        *value /= total;
    }
}

fn samples_per_bar(tempo: f32, time_signature: (u32, u32), sample_rate: u32) -> u32 {
    60 * sample_rate * time_signature.0 / tempo as u32
}

impl Pipeline {
    pub fn from_tree(
        tree: &tree_sitter::Tree,
        source: &str,
        config: Option<&PipelineConfig>,
    ) -> Result<(Self, Receiver<f32>), Box<dyn Error>> {
        // initialize playables and effects
        let mut playables: HashMap<String, Playable> = HashMap::new();
        let mut effects: HashMap<String, Vec<Effect>> = HashMap::new();

        let sample_rate = 48000;

        let mut bar_length = samples_per_bar(120.0, (4, 4), sample_rate);

        // NOTE: should probably only load those samples that haven't been loaded yet...
        //       because this function runs every time the declaration file changes

        let mut samples = HashMap::new();

        if let Some(config) = config {
            samples = get_samples(config);
        }

        let samples = SampleSet { samples };

        let mut cursor = tree.root_node().walk();
        for node in tree.root_node().children(&mut cursor) {
            if PLAYABLES.iter().any(|&p| p == node.kind()) {
                let name = node.child_by_field_name("name").unwrap();
                let name = name.utf8_text(source.as_bytes()).unwrap();

                let playable = match node.kind() {
                    "grid" => {
                        let grid = Grid::from_node(&node, source).unwrap();
                        Playable::Grid(grid)
                    }
                    _ => panic!("Unknown playable"),
                };

                playables.insert(name.to_string(), playable);
            }
        }

        let mut mix = playables
            .keys()
            .map(|i| (i.to_string(), 1.0))
            .collect::<HashMap<String, f32>>();

        let mut cursor = tree.root_node().walk();
        for node in tree.root_node().children(&mut cursor) {
            if node.kind() == "map" {
                let target = node.child_by_field_name("name").unwrap();
                let target = target.utf8_text(source.as_bytes()).unwrap();

                let playable = playables.get_mut(target).unwrap();

                match playable {
                    Playable::Grid(g) => g.map_from_node(&node, source, &samples),
                }
            } else if node.kind() == "tempo" {
                let bpm = node.child_by_field_name("bpm").unwrap();
                let bpm = bpm.utf8_text(source.as_bytes()).unwrap();

                let count = node.child_by_field_name("count").unwrap();
                let count = count.utf8_text(source.as_bytes()).unwrap();

                let note = node.child_by_field_name("note").unwrap();
                let note = note.utf8_text(source.as_bytes()).unwrap();

                bar_length = samples_per_bar(
                    bpm.parse().unwrap(),
                    (count.parse().unwrap(), note.parse().unwrap()),
                    sample_rate,
                );

                // set this information in all grids
                playables
                    .iter_mut()
                    .filter(|x| match x.1 {
                        Playable::Grid(_) => true,
                    })
                    .for_each(|(_, x)| match x {
                        Playable::Grid(g) => g.set_tempo_and_time(
                            bpm.parse().unwrap(),
                            (count.parse().unwrap(), note.parse().unwrap()),
                        ),
                    });
            } else if node.kind() == "speed" {
                let target = node.child_by_field_name("name").unwrap();
                let target = target.utf8_text(source.as_bytes()).unwrap();

                let playable = playables.get_mut(target).unwrap();

                let sign = node.child(2).unwrap();
                let numer = sign.child_by_field_name("numer").unwrap();
                let numer = numer.utf8_text(source.as_bytes()).unwrap();
                let denom = sign.child_by_field_name("denom").unwrap();
                let denom = denom.utf8_text(source.as_bytes()).unwrap();

                let numer: i16 = numer.parse().unwrap();
                let denom: i16 = denom.parse().unwrap();

                match playable {
                    Playable::Grid(g) => g.set_note_length((numer as u32, denom as u32)),
                }
            } else if node.kind() == "mix" {
                let target = node.child_by_field_name("name").unwrap();
                let target = target.utf8_text(source.as_bytes()).unwrap();

                let value = node.child_by_field_name("value").unwrap();
                let value = value.utf8_text(source.as_bytes()).unwrap();
                let value = value.parse().unwrap();

                mix.insert(target.to_string(), value);
            } else if node.kind() == "setter" {
                let target = node.child_by_field_name("name").unwrap();
                let target = target.utf8_text(source.as_bytes()).unwrap();

                let _playable = playables.get_mut(target).unwrap();

                let property = node.child_by_field_name("prop").unwrap();
                let property = property.utf8_text(source.as_bytes()).unwrap();

                let value = node.child_by_field_name("value").unwrap();
                let value = value.utf8_text(source.as_bytes()).unwrap();

                match property {
                    "lp_cutoff" => {
                        let value = value.parse().unwrap();
                        // TODO: implement variable sample rate (set_output_config should propagate to all effects)
                        let fir = FIRBuilder::new()
                            .low_pass(value, sample_rate as f32)
                            .build();

                        // add the effect to the list of effects (or create new list if none exists)
                        effects
                            .entry(target.to_string())
                            .or_default()
                            .push(Effect::FIR(fir));
                    }
                    "hp_cutoff" => {
                        let value = value.parse().unwrap();
                        let fir = FIRBuilder::new()
                            .high_pass(value, sample_rate as f32)
                            .build();

                        // add the effect to the list of effects (or create new list if none exists)
                        effects
                            .entry(target.to_string())
                            .or_default()
                            .push(Effect::FIR(fir));
                    }
                    // other effects will come here
                    "gain" => {
                        let value = value.parse().unwrap();
                        let gain = Gain::new(value);

                        effects
                            .entry(target.to_string())
                            .or_default()
                            .push(Effect::Gain(gain));
                    }
                    _ => (),
                }
            }
        }

        rescale_mix(&mut mix);

        let (s_tx, rx) = mpsc::sync_channel(2048);

        Ok((
            Self {
                playables,
                mix,
                time: 0,
                bar_length: bar_length as u128,
                sink: s_tx,
                effects,
                sample_rate,
                next: None,
            },
            rx,
        ))
    }

    pub fn set_output_config(&mut self, config: &cpal::SupportedStreamConfig) {
        self.sample_rate = config.sample_rate().0;
    }

    pub fn update(&mut self, other: Pipeline) {
        self.next = Some(Box::new(other));
    }

    fn set_to_new(&mut self) {
        if let Some(next) = self.next.take() {
            self.time = 0;
            self.playables = next.playables;
            self.effects = next.effects;
            self.mix = next.mix;
            self.bar_length = next.bar_length;
        }
    }

    pub fn send_sample(&mut self) -> Result<(), SendError<f32>> {
        // check if we need to update the pipeline
        if self.time % (4 * self.bar_length) == 0 {
            self.set_to_new();
        }

        let mut sample: f32 = 0.0;
        for playable in self.playables.iter_mut() {
            let dry = match playable.1 {
                Playable::Grid(g) => {
                    let s = g.get_sample(self.time, self.sample_rate);
                    s * self.mix[playable.0]
                }
            };

            let wet = {
                let mut output = dry;
                match self.effects.get_mut(playable.0) {
                    None => output,
                    Some(effects) => {
                        for effect in effects {
                            output = match effect {
                                Effect::FIR(fir) => fir.process(output),
                                Effect::Gain(gain) => gain.process(output),
                                // not yet implemented effects
                                _ => output,
                            }
                        }

                        output
                    }
                }
            };

            sample += wet;
        }

        self.time += 1;

        let res = self.sink.send(sample);
        log::trace!(
            "pipeline, {}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );

        res
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tree_sitter::Parser;

    fn get_test_tree() -> (String, tree_sitter::Tree) {
        let source = include_str!("../testdata/pipeline_test.br");

        let mut parser = Parser::new();
        parser
            .set_language(tree_sitter_breaker::language())
            .unwrap();

        let tree = parser.parse(source, None).unwrap();

        (source.to_string(), tree)
    }

    #[test]
    fn named_grid() {
        let (source, tree) = get_test_tree();

        let playables = Pipeline::from_tree(&tree, &source, None)
            .unwrap()
            .0
            .playables;

        assert!(
            playables.len() == 1,
            "Playables length is not 1, but {}",
            playables.len()
        );
        assert!(playables.contains_key("veryfunname"));
    }
}
