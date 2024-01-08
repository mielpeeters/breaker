/*!
* This module defines notes and chords, and functions for parsing them.
*/

use std::fmt::Display;

use crate::util::FromNode;

/// A chord consists of a root note, a mode, an optional augmentation, and an optional bass note.
pub struct Chord(Note, Mode, Option<Aug>, Option<Note>);

/// A note consists of a white note, an accidental, and an octave.
#[derive(Debug)]
pub struct Note(WhiteNote, Acc, Octave);

/// A white note, like found on a piano.
#[derive(Debug)]
pub enum WhiteNote {
    A,
    B,
    C,
    D,
    E,
    F,
    G,
}

/// Octaves one through seven (as for an 88 key piano)
#[derive(Default, Debug, PartialEq)]
pub enum Octave {
    One = 1,
    Two = 2,
    Three = 3,
    #[default]
    Four = 4,
    Five = 5,
    Six = 6,
    Seven = 7,
}

/// The accent for a note, either natural, flat, or sharp.
#[derive(Default, Debug)]
pub enum Acc {
    #[default]
    Natural,
    Flat,
    Sharp,
}

/// Possible modes for chords (of course, there are a lot more)
#[derive(Default)]
pub enum Mode {
    #[default]
    Major,
    Minor,
    Dim,
    Aug,
    Sus4,
    Sus2,
}

/// Possible augmentations for chords (of course, there are a lot more)
#[derive(Debug)]
pub enum Aug {
    Seven,
    Nine,
    Eleven,
    Thirteen,
    MajSeven,
    Five,
}

impl FromNode for Note {
    /// Parse a note from a tree-sitter node, with given source string (which generated the
    /// treesitter node).
    fn from_node(node: &tree_sitter::Node, source: &str) -> Option<Self> {
        let bass = node.child_by_field_name("bass")?;
        let bass = bass.utf8_text(source.as_bytes()).unwrap();
        let bass = bass.try_into().ok()?;

        let acc = node.child_by_field_name("acc");
        let acc = match acc {
            Some(acc) => acc
                .utf8_text(source.as_bytes())
                .unwrap()
                .try_into()
                .unwrap_or_default(),
            None => Acc::default(),
        };

        let octave = node.child_by_field_name("oct");
        let octave = match octave {
            Some(octave) => octave
                .utf8_text(source.as_bytes())
                .unwrap()
                .try_into()
                .unwrap_or_default(),
            None => Octave::default(),
        };

        Some(Self(bass, acc, octave))
    }
}

impl Display for Note {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.2 != Octave::default() {
            write!(f, "[{}]", self.2)?;
        }
        write!(f, "{}{}", self.0, self.1)
    }
}

impl FromNode for Chord {
    fn from_node(node: &tree_sitter::Node, source: &str) -> Option<Self> {
        let root = node.child_by_field_name("root")?;

        let root = Note::from_node(&root, &source)?;

        let mode = node.child_by_field_name("mode");
        let mode = match mode {
            Some(mode) => mode
                .utf8_text(source.as_bytes())
                .unwrap()
                .try_into()
                .unwrap_or_default(),
            None => Mode::default(),
        };

        let augm = node.child_by_field_name("augm");
        let augm = match augm {
            Some(augm) => augm.utf8_text(source.as_bytes()).unwrap().try_into().ok(),
            None => None,
        };

        let bass = node.child_by_field_name("bass");
        let bass = match bass {
            Some(bass) => {
                // over is a note
                Note::from_node(&bass, source)
            }
            None => None,
        };

        Some(Self(root, mode, augm, bass))
    }
}

impl Display for Chord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}", self.0, self.1)?;
        if let Some(aug) = &self.2 {
            write!(f, "{}", aug)?;
        }
        if let Some(over) = &self.3 {
            write!(f, "/{}", over)?;
        }
        Ok(())
    }
}

impl TryFrom<&str> for Acc {
    type Error = &'static str;
    fn try_from(s: &str) -> Result<Acc, &'static str> {
        match s {
            "b" => Ok(Self::Flat),
            "#" => Ok(Self::Sharp),
            _ => Err("Invalid acc"),
        }
    }
}

impl Display for Acc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Flat => write!(f, "b"),
            Self::Sharp => write!(f, "#"),
            Self::Natural => Ok(()),
        }
    }
}

impl TryFrom<&str> for Aug {
    type Error = &'static str;

    fn try_from(s: &str) -> Result<Self, &'static str> {
        match s {
            "7" => Ok(Self::Seven),
            "9" => Ok(Self::Nine),
            "11" => Ok(Self::Eleven),
            "13" => Ok(Self::Thirteen),
            "M7" => Ok(Self::MajSeven),
            "5" => Ok(Self::Five),
            _ => Err("Invalid aug"),
        }
    }
}

impl Display for Aug {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Seven => write!(f, "7"),
            Self::Nine => write!(f, "9"),
            Self::Eleven => write!(f, "11"),
            Self::Thirteen => write!(f, "13"),
            Self::MajSeven => write!(f, "M7"),
            Self::Five => write!(f, "5"),
        }
    }
}

impl TryFrom<&str> for WhiteNote {
    type Error = &'static str;

    fn try_from(s: &str) -> Result<Self, &'static str> {
        match s {
            "A" => Ok(Self::A),
            "B" => Ok(Self::B),
            "C" => Ok(Self::C),
            "D" => Ok(Self::D),
            "E" => Ok(Self::E),
            "F" => Ok(Self::F),
            "G" => Ok(Self::G),
            _ => Err("Invalid note"),
        }
    }
}

impl Display for WhiteNote {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::A => write!(f, "A"),
            Self::B => write!(f, "B"),
            Self::C => write!(f, "C"),
            Self::D => write!(f, "D"),
            Self::E => write!(f, "E"),
            Self::F => write!(f, "F"),
            Self::G => write!(f, "G"),
        }
    }
}

impl TryFrom<&str> for Mode {
    type Error = &'static str;

    fn try_from(s: &str) -> Result<Self, &'static str> {
        match s {
            "m" => Ok(Self::Minor),
            "dim" => Ok(Self::Dim),
            "aug" => Ok(Self::Aug),
            "sus4" => Ok(Self::Sus4),
            "sus2" => Ok(Self::Sus2),
            _ => Err("Unrecognized mode"),
        }
    }
}

impl Display for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Major => Ok(()),
            Self::Minor => write!(f, "m"),
            Self::Dim => write!(f, "dim"),
            Self::Aug => write!(f, "aug"),
            Self::Sus4 => write!(f, "sus4"),
            Self::Sus2 => write!(f, "sus2"),
        }
    }
}

impl TryFrom<&str> for Octave {
    type Error = &'static str;

    fn try_from(s: &str) -> Result<Self, &'static str> {
        match s {
            "1" => Ok(Self::One),
            "2" => Ok(Self::Two),
            "3" => Ok(Self::Three),
            "4" => Ok(Self::Four),
            "5" => Ok(Self::Five),
            "6" => Ok(Self::Six),
            "7" => Ok(Self::Seven),
            _ => Err("Invalid octave"),
        }
    }
}

impl Display for Octave {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::One => write!(f, "1"),
            Self::Two => write!(f, "2"),
            Self::Three => write!(f, "3"),
            Self::Four => write!(f, "4"),
            Self::Five => write!(f, "5"),
            Self::Six => write!(f, "6"),
            Self::Seven => write!(f, "7"),
        }
    }
}
