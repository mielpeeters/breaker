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
    postproc::{Effect, FIRBuilder, Reverb},
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
    pub mix: HashMap<String, f32>,
    pub time: u128,
    sample_rate: u32,
    sink: SyncSender<f32>,
    effects: Vec<Effect>,
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

impl Pipeline {
    pub fn from_tree(
        tree: &tree_sitter::Tree,
        source: &str,
        config: Option<&PipelineConfig>,
    ) -> Result<(Self, Receiver<f32>), Box<dyn Error>> {
        let mut playables: HashMap<String, Playable> = HashMap::new();

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

                let property = node.child_by_field_name("property").unwrap();
                let _property = property.utf8_text(source.as_bytes()).unwrap();

                let value = node.child_by_field_name("value").unwrap();
                let _value = value.utf8_text(source.as_bytes()).unwrap();

                // TODO: implement properties for playables
                //       -> all properties should be playable-independent
                //       -> more like post-processing steps than properties!
            }
        }

        rescale_mix(&mut mix);

        let (s_tx, rx) = mpsc::sync_channel(2048);

        let mut effects: Vec<Effect> = vec![];
        // fir lowpass filter with cutoff at 2000 Hz
        let fir = FIRBuilder::new().low_pass(2800.0, 44100.0).build();
        effects.push(Effect::FIR(fir));
        let rev = Reverb::new();
        effects.push(Effect::Reverb(rev));

        Ok((
            Self {
                playables,
                mix,
                time: 0,
                sink: s_tx,
                effects,
                sample_rate: 44100,
            },
            rx,
        ))
    }

    pub fn set_output_config(&mut self, config: &cpal::SupportedStreamConfig) {
        self.sample_rate = config.sample_rate().0;
    }

    pub fn update(&mut self, other: Pipeline) {
        self.playables = other.playables;
        self.mix = other.mix;
    }

    pub fn send_sample(&mut self) -> Result<(), SendError<f32>> {
        // TODO: implement post-processing steps per-playable based on properties
        let mut sample: f32 = 0.0;
        for playable in self.playables.iter_mut() {
            let dry = match playable.1 {
                Playable::Grid(g) => {
                    let s = g.get_sample(self.time, self.sample_rate);
                    s * self.mix[playable.0]
                }
            };
            // NOTE: here, the post-processing pipeline should be traversed
            //       -> keep one per playable!
            //       -> maybe different, when multiple playables are mixed into one effect?
            sample += dry
        }

        self.time += 1;

        for effect in self.effects.iter_mut() {
            sample = match effect {
                Effect::FIR(f) => f.process(sample),
                Effect::Reverb(r) => r.process(sample),
            }
        }

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
