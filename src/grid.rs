/*!
* Grid module implements the grid sequencer grid parsing
*/

use std::{collections::HashMap, fmt::Display, time::Instant};

use rand::Rng;

use crate::{
    chromatic::{Chord, Note},
    sampler::{SamplePlayer, SampleSet},
    util::FromNode,
};

/// The possible entries in a grid item, each with a different meaning.
#[derive(Default, Debug, PartialEq, Clone)]
pub enum GridToken {
    // hit a sample with given name
    Hit(SamplePlayer),
    #[default]
    Pause,
    Prob(f32, SamplePlayer),
    Chord(Chord),
    Note(Note),
    Repeat,
    Todo(String),
}

#[derive(Debug)]
pub struct Grid {
    pub tokens: Vec<GridToken>,
    next_scheduled: usize,
    now_playing: usize,
    tempo: f32,
    time_sign: (u32, u32),
    // the note length of one token
    note_length: (u32, u32),
    samples_per_hit: Option<u32>,
}

impl GridToken {
    fn get_sample(&mut self, time: u128) -> f32 {
        match self {
            GridToken::Hit(s) | GridToken::Prob(_, s) => s.get_sample(time),
            GridToken::Pause => 0.0,
            GridToken::Chord(c) => c.get_sample(time),
            GridToken::Note(n) => n.get_sample(time),
            _ => panic!("This token doesn't have a sample"),
        }
    }
}

impl TryFrom<&str> for GridToken {
    type Error = &'static str;

    fn try_from(value: &str) -> Result<GridToken, &'static str> {
        match value {
            "_" => Ok(Self::Pause),
            "&" => Ok(Self::Repeat),
            x => Ok(Self::Todo(x.to_string())),
        }
    }
}

impl Default for Grid {
    fn default() -> Self {
        Self {
            tokens: Default::default(),
            tempo: 120.0,
            note_length: (1, 16),
            time_sign: (4, 4),
            samples_per_hit: None,
            now_playing: 0,
            next_scheduled: 0,
        }
    }
}

impl FromNode for Grid {
    fn from_node(node: &tree_sitter::Node, source: &str) -> Option<Self>
    where
        Self: Sized,
    {
        let mut walk = node.walk();
        let token_iter = node.children_by_field_name("token", &mut walk);

        let tokens: Vec<GridToken> = token_iter
            .map(|token| -> Option<GridToken> {
                let Some(token) = token.child(0) else {
                    return None;
                };
                let kind = token.kind();
                let token_text = token.utf8_text(source.as_bytes()).unwrap();

                match kind {
                    "raw_token" => token_text.try_into().ok(),
                    "chord" => {
                        let res = Chord::from_node(&token, source);
                        res.map(GridToken::Chord)
                    }
                    "single_note" => {
                        let res = Note::from_node(&token, source);
                        res.map(GridToken::Note)
                    }
                    &_ => None,
                }
            })
            // replace None with default value (Pause)
            .map(|token| token.unwrap_or_default())
            .collect();

        Some(Self {
            tokens,
            ..Default::default()
        })
    }
}

impl Display for Grid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{{")?;
        for i in &self.tokens {
            writeln!(f, "   {}", i)?;
        }
        write!(f, "}}")
    }
}

impl Display for GridToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GridToken::Hit(_) => write!(f, "x"),
            GridToken::Pause => write!(f, "_"),
            GridToken::Prob(_, _) => write!(f, "?"),
            GridToken::Chord(c) => write!(f, "{}", c),
            GridToken::Repeat => write!(f, "&"),
            GridToken::Todo(s) => write!(f, "{}", s),
            GridToken::Note(n) => write!(f, "{}", n),
        }
    }
}

impl Grid {
    pub fn get_sample(&mut self, time: u128) -> f32 {
        if self.samples_per_hit.is_none() {
            self.calc_samples_per_token();
        }

        if self.tokens.is_empty() {
            return 0.0;
        }
        // TODO: implement variable grid speed

        let index =
            ((time / self.samples_per_hit.unwrap() as u128) % (self.tokens.len() as u128)) as usize;

        if index == self.next_scheduled {
            // we need to check whether we should play the next token or not
            match &mut self.tokens[index] {
                GridToken::Prob(p, _) => {
                    let mut rng = rand::thread_rng();
                    let should_play = rng.gen_bool((*p / 100.0).into());
                    if should_play {
                        self.now_playing = index;
                    }
                }
                GridToken::Repeat => {}
                _ => {
                    self.now_playing = index;
                }
            }

            self.next_scheduled = (index + 1) % self.tokens.len();

            // if let GridToken::Hit(s) | GridToken::Prob(_, s) = &self.tokens[index] {
            //     println!("{}", s.sample.name);
            // } else if let GridToken::Chord(c) = &self.tokens[index] {
            //     println!("{}", c);
            // }
        }

        self.tokens[self.now_playing].get_sample(time)
    }

    fn calc_samples_per_token(&mut self) {
        let note_len = self.note_length.0 as f32 / self.note_length.1 as f32;
        let beat = self.time_sign.1 as f32;
        // TODO: variable sample rate
        let val = note_len * beat * 60.0 * 44100.0 / self.tempo;
        self.samples_per_hit = Some(val as u32);
    }

    pub fn map_from_node(&mut self, node: &tree_sitter::Node, source: &str, sampleset: &SampleSet) {
        let mut walk = node.walk();
        let map_entry_iter = node.children_by_field_name("pair", &mut walk);

        let mut map = HashMap::new();

        map_entry_iter.for_each(|entry| {
            // entry is a pair of key and value
            let Some(key) = entry.child_by_field_name("key") else {
                return;
            };
            let Some(value) = entry.child_by_field_name("value") else {
                return;
            };
            let Some(value) = value.child(0) else {
                return;
            };

            let key_text = key.utf8_text(source.as_bytes()).unwrap();

            // value is either a sample, or a chord
            match value.kind() {
                "sample" => {
                    // get the sample name
                    let name = value.child_by_field_name("name").unwrap();
                    let value_text = name.utf8_text(source.as_bytes()).unwrap();
                    let sample = sampleset.samples.get(value_text).unwrap();
                    let sampleplayer = SamplePlayer::new(sample.clone());

                    if let Some(p) = value.child_by_field_name("probability") {
                        let Some(p) = p.child(0) else {
                            return;
                        };
                        let p_text = p.utf8_text(source.as_bytes()).unwrap();
                        let p: f32 = p_text.parse().unwrap();
                        map.insert(key_text.to_string(), GridToken::Prob(p, sampleplayer));
                    } else {
                        map.insert(key_text.to_string(), GridToken::Hit(sampleplayer));
                    }
                }
                "chord" => {
                    let res = Chord::from_node(&value, source);
                    if let Some(chord) = res {
                        map.insert(key_text.to_string(), GridToken::Chord(chord));
                    }
                }
                &_ => {}
            }
        });

        let start = Instant::now();
        // set the todos to the mapped values
        self.tokens.iter_mut().for_each(|token| {
            if let GridToken::Todo(key) = token {
                let Some(value) = map.get(key) else {
                    return;
                };
                *token = value.clone();
            }
        });
        // println!("Cloning map values took {:?}", start.elapsed());
    }

    pub fn set_tempo(&mut self, tempo: f32) {
        self.tempo = tempo;
    }

    pub fn set_note_length(&mut self, note_length: (u32, u32)) {
        self.note_length = note_length;
    }

    pub fn set_time_sign(&mut self, time_sign: (u32, u32)) {
        self.time_sign = time_sign;
    }

    pub fn set_tempo_and_time(&mut self, tempo: f32, time_sign: (u32, u32)) {
        self.tempo = tempo;
        self.time_sign = time_sign;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tree_sitter::Parser;

    fn get_test_tree() -> (String, tree_sitter::Tree) {
        let source = include_str!("../testdata/grid_test.br");

        let mut parser = Parser::new();
        parser
            .set_language(tree_sitter_breaker::language())
            .unwrap();

        let tree = parser.parse(&source, None).unwrap();

        (source.to_string(), tree)
    }

    #[test]
    fn test_grid() {
        let (source, tree) = get_test_tree();

        let Some(grid_node) = tree.root_node().child(0) else {
            panic!("No grid node found");
        };

        let Some(grid) = Grid::from_node(&grid_node, &source) else {
            panic!("Grid parsing failed");
        };

        assert!(
            grid.tokens.len() == 4,
            "Grid length is not 4, but {}",
            grid.tokens.len()
        );
        assert!(
            grid.tokens[1] == GridToken::Pause,
            "Grid token 1 is not Hit, but {}",
            grid.tokens[1]
        );
        assert!(matches!(grid.tokens[3], GridToken::Chord(_)));
    }
}
