use std::fmt::Display;

pub struct Chord(Note, Mode, Option<Aug>, Option<Note>);

#[derive(Debug)]
pub struct Note(WhiteNote, Acc, Octave);

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

#[derive(Default, Debug, PartialEq)]
pub enum Octave {
    One,
    Two,
    Three,
    #[default]
    Four,
    Five,
    Six,
    Seven,
}

#[derive(Default, Debug)]
pub enum Acc {
    #[default]
    Natural,
    Flat,
    Sharp,
}

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

#[derive(Debug)]
pub enum Aug {
    Seven,
    Nine,
    Eleven,
    Thirteen,
    MajSeven,
    Five,
}

impl Note {
    pub fn from_node(node: &tree_sitter::Node, source: &str) -> Option<Self> {
        let bass = node.child_by_field_name("bass")?;
        let bass = bass.utf8_text(&source.as_bytes()).unwrap();

        let acc = node.child_by_field_name("acc");
        let acc = match acc {
            Some(acc) => acc.utf8_text(&source.as_bytes()).unwrap().into(),
            None => Acc::default(),
        };

        let octave = node.child_by_field_name("oct");
        let octave = match octave {
            Some(octave) => octave.utf8_text(&source.as_bytes()).unwrap().into(),
            None => Octave::default(),
        };

        Some(Self(bass.into(), acc, octave))
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

impl Chord {
    pub fn from_node(node: &tree_sitter::Node, source: &str) -> Option<Self> {
        let note = node.child_by_field_name("note")?;

        let note = Note::from_node(&note, &source)?;

        let mode = node.child_by_field_name("mode");
        let mode = match mode {
            Some(mode) => mode.utf8_text(&source.as_bytes()).unwrap().into(),
            None => Mode::default(),
        };

        let augm = node.child_by_field_name("augm");
        let augm = match augm {
            Some(augm) => {
                // over is a note
                Some(augm.utf8_text(&source.as_bytes()).unwrap().into())
            }
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

        Some(Self(note, mode, augm, bass))
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

impl From<&str> for Acc {
    fn from(s: &str) -> Self {
        match s {
            "b" => Self::Flat,
            "#" => Self::Sharp,
            _ => Self::Natural,
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

impl From<&str> for Aug {
    fn from(s: &str) -> Self {
        match s {
            "7" => Self::Seven,
            "9" => Self::Nine,
            "11" => Self::Eleven,
            "13" => Self::Thirteen,
            "M7" => Self::MajSeven,
            "5" => Self::Five,
            _ => panic!("Invalid aug"),
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

impl From<&str> for WhiteNote {
    fn from(s: &str) -> Self {
        match s {
            "A" => Self::A,
            "B" => Self::B,
            "C" => Self::C,
            "D" => Self::D,
            "E" => Self::E,
            "F" => Self::F,
            "G" => Self::G,
            _ => panic!("Invalid note"),
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

impl From<&str> for Mode {
    fn from(s: &str) -> Self {
        match s {
            "m" => Self::Minor,
            "dim" => Self::Dim,
            "aug" => Self::Aug,
            "sus4" => Self::Sus4,
            "sus2" => Self::Sus2,
            _ => Self::Major,
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

impl From<&str> for Octave {
    fn from(s: &str) -> Self {
        match s {
            "1" => Self::One,
            "2" => Self::Two,
            "3" => Self::Three,
            "4" => Self::Four,
            "5" => Self::Five,
            "6" => Self::Six,
            "7" => Self::Seven,
            _ => panic!("Invalid octave"),
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
