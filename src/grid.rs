/*!
* Grid module implements the grid sequencer grid parsing
*/

use std::{collections::HashMap, fmt::Display};

use dasp_sample::Sample as Smp;
use rand::Rng;

use crate::{chromatic::Chord, util::FromNode};

/// The possible entries in a grid item, each with a different meaning.
#[derive(Default, Debug, PartialEq)]
pub enum GridToken {
    // hit a sample with given name
    Hit(Sample),
    #[default]
    Pause,
    Prob(f32, Sample),
    Chord(Chord),
    Repeat,
    Todo(String),
}

#[derive(Debug)]
pub struct Grid {
    pub tokens: Vec<GridToken>,
    // maps the remaining characters to GridTokens
    pub map: HashMap<String, GridToken>,
}

#[derive(Debug, PartialEq)]
pub struct Sample {
    name: String,
    data: Vec<f32>,
    start: i128,
    current: i128,
}

impl Sample {
    pub fn new(name: &str) -> Self {
        let name = format!(
            "/home/mielpeeters/coding/breaker/testdata/samples/{}.wav",
            name
        );
        let mut data = hound::WavReader::open(&name).unwrap();
        let mut samples = vec![];

        for s in data.samples() {
            let s: i16 = s.unwrap();
            let converted: f32 = s.to_sample();
            samples.push(converted)
        }

        Self {
            name,
            data: samples,
            start: -100,
            current: 0,
        }
    }

    fn get_sample(&mut self, time: u128) -> f32 {
        if time > (self.current + 1) as u128 {
            self.start = time as i128;
        }

        self.current = time as i128;

        let index = (time as i128 - self.start) as usize;
        match self.data.get(index) {
            Some(s) => *s,
            None => 0.0,
        }
    }
}

impl GridToken {
    fn get_sample(&mut self, time: u128) -> f32 {
        match self {
            GridToken::Hit(s) => s.get_sample(time),
            GridToken::Pause => 0.0,
            GridToken::Prob(p, _) => {
                let mut rng = rand::thread_rng();
                let sample = rng.gen_bool((*p).into());
                if sample {
                    1.0
                } else {
                    0.0
                }
            }
            GridToken::Chord(c) => c.get_sample(time),
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
                        if let Some(chord) = res {
                            Some(GridToken::Chord(chord))
                        } else {
                            None
                        }
                    }
                    &_ => None,
                }
            })
            // replace None with default value (Pause)
            .map(|token| token.unwrap_or_default())
            .collect();

        let map = HashMap::new();

        Some(Self { tokens, map })
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
        }
    }
}

impl Grid {
    pub fn get_sample(&mut self, time: u128) -> f32 {
        if self.tokens.is_empty() {
            return 0.0;
        }
        // TODO: implement variable grid speed
        let index = ((time / 22050) % (self.tokens.len() as u128)) as usize;

        match &mut self.tokens[index] {
            GridToken::Repeat => {
                let mut i = index - 1;
                while self.tokens[i] == GridToken::Repeat {
                    i -= 1;
                }
                self.tokens[i].get_sample(time)
            }
            GridToken::Todo(key) => {
                // println!("Map: {:?}", self.map);
                let Some(value) = self.map.get_mut(key) else {
                    return 0.0;
                };
                value.get_sample(time)
            }
            _ => self.tokens[index].get_sample(time),
        }
    }

    pub fn map_from_node(&mut self, node: &tree_sitter::Node, source: &str) {
        let mut walk = node.walk();
        let map_entry_iter = node.children_by_field_name("pair", &mut walk);

        map_entry_iter.for_each(|entry| {
            println!("Entry: {}", entry.to_sexp());
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
                    let value_text = value.utf8_text(source.as_bytes()).unwrap();
                    self.map.insert(
                        key_text.to_string(),
                        GridToken::Hit(Sample::new(&value_text)),
                    );
                }
                "chord" => {
                    let res = Chord::from_node(&value, source);
                    if let Some(chord) = res {
                        self.map
                            .insert(key_text.to_string(), GridToken::Chord(chord));
                    }
                }
                &_ => {}
            }
        })
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
