use std::{
    collections::HashMap,
    fs,
    sync::{
        mpsc::{self, Receiver, SendError, SyncSender},
        Arc,
    },
};

use crate::{
    grid::{Grid, Sample, SampleSet},
    util::FromNode,
};

const PLAYABLES: [&str; 1] = ["grid"];

#[derive(Debug)]
pub enum Playable {
    Grid(Grid),
}

// TODO: find a better name
pub struct Pipeline {
    pub playables: HashMap<String, Playable>,
    pub mix: HashMap<String, f32>,
    pub time: u128,
    sink: SyncSender<f32>,
}

pub struct PipelineConfig {
    pub samples_dir: String,
}

fn get_samples(config: &PipelineConfig) -> HashMap<String, Arc<Sample>> {
    let paths = fs::read_dir(config.samples_dir.clone()).unwrap();
    let mut samples = HashMap::new();

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

// HACK: this is a hacky way to set the mix, but it works
//       should be so that the given mix values correspond to the mix ratios
fn set_mix(mix: &mut HashMap<String, f32>, index: &str, value: f32) {
    let old = mix[index];
    let old_remainder = 1.0 - old;

    let default = 1.0 / (mix.len() as f32);

    let mut value = value * default;

    if value > 1.0 {
        value = 1.0;
    }

    let new_remainder = 1.0 - value;

    mix.iter_mut().for_each(|(i, x)| {
        if i != index {
            *x = *x * (new_remainder / old_remainder);
        } else {
            *x = value;
        }
    });

    println!("{:?}", mix);

    let val = mix.iter().fold(0.0, |acc, (_, x)| acc + x) - 1.0;
    assert!(val.abs() < 0.001, "Value wasnt 1.0, but had error {}", val);
}

impl Pipeline {
    pub fn from_tree(
        tree: &tree_sitter::Tree,
        source: &str,
        config: Option<&PipelineConfig>,
    ) -> (Self, Receiver<f32>) {
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

        let mix = 1.0 / (playables.len() as f32);
        let mut mix = playables
            .iter()
            .map(|(i, _x)| (i.to_string(), mix))
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

                set_mix(&mut mix, target, value);
            }
        }

        let (s_tx, rx) = mpsc::sync_channel(512);

        (
            Self {
                playables,
                mix,
                time: 0,
                sink: s_tx,
            },
            rx,
        )
    }

    pub fn update(&mut self, other: Pipeline) {
        self.playables = other.playables;
        self.mix = other.mix;
    }

    pub fn send_sample(&mut self) -> Result<(), SendError<f32>> {
        let mut sample: f32 = 0.0;
        for playable in self.playables.iter_mut() {
            match playable.1 {
                Playable::Grid(g) => {
                    let s = g.get_sample(self.time);
                    sample += s * self.mix[playable.0];
                }
            }
        }

        self.time += 1;

        self.sink.send(sample)
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

        let tree = parser.parse(&source, None).unwrap();

        (source.to_string(), tree)
    }

    #[test]
    fn named_grid() {
        let (source, tree) = get_test_tree();

        let playables = Pipeline::from_tree(&tree, &source, None).0.playables;

        assert!(
            playables.len() == 1,
            "Playables length is not 1, but {}",
            playables.len()
        );
        assert!(playables.contains_key("veryfunname"));
    }
}
