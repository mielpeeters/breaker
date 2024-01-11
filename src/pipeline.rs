use std::{
    collections::HashMap,
    sync::mpsc::{self, Receiver, SendError, SyncSender},
};

use crate::{grid::Grid, util::FromNode};

const PLAYABLES: [&str; 1] = ["grid"];

#[derive(Debug)]
pub enum Playable {
    Grid(Grid),
}

// TODO: find a better name
pub struct Pipeline {
    pub playables: HashMap<String, Playable>,
    pub mix: Vec<f32>,
    pub time: u128,
    sink: SyncSender<f32>,
}

impl Pipeline {
    pub fn from_tree(tree: &tree_sitter::Tree, source: &str) -> (Self, Receiver<f32>) {
        let mut playables: HashMap<String, Playable> = HashMap::new();

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

        let mut cursor = tree.root_node().walk();
        for node in tree.root_node().children(&mut cursor) {
            if node.kind() == "map" {
                println!("Found a map");
                let target = node.child_by_field_name("name").unwrap();
                let target = target.utf8_text(source.as_bytes()).unwrap();

                let playable = playables.get_mut(target).unwrap();
                match playable {
                    Playable::Grid(g) => g.map_from_node(&node, source),
                }
            }
        }

        let mix = 1.0 / (playables.len() as f32);
        let mix = vec![mix; playables.len()];

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
        for (i, playable) in self.playables.iter_mut().enumerate() {
            match playable.1 {
                Playable::Grid(g) => {
                    let s = g.get_sample(self.time);
                    sample += s * self.mix[i];
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

        let playables = Pipeline::from_tree(&tree, &source).0.playables;

        assert!(
            playables.len() == 1,
            "Playables length is not 1, but {}",
            playables.len()
        );
        assert!(playables.contains_key("veryfunname"));
    }
}
