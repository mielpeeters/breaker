/*!
* Grid module implements the grid sequencer grid parsing
*/

use std::fmt::Display;

use crate::util::FromNode;

/// The possible entries in a grid item, each with a different meaning.
#[derive(Default, Debug)]
pub enum GridToken {
    Hit,
    #[default]
    Pause,
    Prob,
}

pub struct Grid {
    tokens: Vec<GridToken>,
}

impl TryFrom<&str> for GridToken {
    type Error = &'static str;

    fn try_from(value: &str) -> Result<GridToken, &'static str> {
        match value {
            "x" => Ok(Self::Hit),
            "_" => Ok(Self::Pause),
            "?" => Ok(Self::Prob),
            _ => Err("Invalid grid token"),
        }
    }
}

impl Into<&str> for GridToken {
    fn into(self) -> &'static str {
        match self {
            GridToken::Hit => "x",
            GridToken::Pause => "_",
            GridToken::Prob => "?",
        }
    }
}

impl<'a> Into<&'a str> for &'a GridToken {
    fn into(self) -> &'a str {
        match self {
            GridToken::Hit => "x",
            GridToken::Pause => "_",
            GridToken::Prob => "?",
        }
    }
}

impl FromNode for Grid {
    fn from_node(node: &tree_sitter::Node, source: &str) -> Option<Self>
    where
        Self: Sized,
    {
        // TODO: implement
        let mut walk = node.walk();
        let token_iter = node.children_by_field_name("tokens", &mut walk);

        let tokens: Vec<GridToken> = token_iter
            .map(|token| {
                let token = token.utf8_text(source.as_bytes()).unwrap();
                token.try_into().unwrap_or_default()
            })
            .collect();

        Some(Self { tokens })
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
        let strg: &str = self.into();
        write!(f, "{}", strg)
    }
}
