/*!
* This module defines notes and chords, and functions for parsing them.
*/

use std::{
    fmt::Display,
    ops::{Add, AddAssign},
};

use num_derive::FromPrimitive;

use crate::util::FromNode;

/// A chord consists of a root note, a mode, an optional augmentation, and an optional bass note.
#[derive(Debug, PartialEq, Clone)]
// TODO: add inversions support!
pub struct Chord(Note, Mode, Augs, Option<Note>);

/// A note consists of a white note, an accidental, and an octave.
#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Note(PitchClass, Octave);

/// A pitch class is a white note with an accidental.
#[derive(Debug, FromPrimitive, Clone, Copy, PartialEq)]
pub enum PitchClass {
    C = 0,
    Cs = 1,
    D = 2,
    Ds = 3,
    E = 4,
    F = 5,
    Fs = 6,
    G = 7,
    Gs = 8,
    A = 9,
    As = 10,
    B = 11,
}

/// Octaves one through seven (as for an 88 key piano)
#[derive(Default, Debug, PartialEq, Clone, Copy)]
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
#[derive(Default, Debug, Copy, Clone)]
pub enum Acc {
    #[default]
    Natural = 0,
    Flat = -1,
    Sharp = 1,
}

/// Possible modes for chords (of course, there are a lot more)
#[derive(Default, Debug, PartialEq, Clone, Copy)]
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
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Aug {
    Six,
    MajSix,
    Seven,
    MajSeven,
    Nine,
    MajNine,
    Eleven,
    MajEleven,
    Thirteen,
    MajThirteen,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Augs(Vec<Aug>);

impl FromNode for Note {
    /// Parse a note from a tree-sitter node, with given source string (which generated the
    /// treesitter node).
    fn from_node(node: &tree_sitter::Node, source: &str) -> Option<Self> {
        let bass = node.child_by_field_name("bass");
        let bass = if let Some(bass) = bass {
            bass
        } else {
            node.child_by_field_name("small_bass").unwrap()
        };

        let bass = bass.utf8_text(source.as_bytes()).unwrap();
        let mut bass: PitchClass = bass.try_into().ok()?;

        let acc = node.child_by_field_name("acc");
        let acc = match acc {
            Some(acc) => acc
                .utf8_text(source.as_bytes())
                .unwrap()
                .try_into()
                .unwrap_or_default(),
            None => Acc::default(),
        };

        bass += acc;

        let octave = node.child_by_field_name("oct");
        let octave = match octave {
            Some(octave) => octave
                .utf8_text(source.as_bytes())
                .unwrap()
                .try_into()
                .unwrap_or_default(),
            None => Octave::default(),
        };

        Some(Self(bass, octave))
    }
}

impl Display for Note {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.1 != Octave::default() {
            write!(f, "[{}]", self.1)?;
        }
        write!(f, "{}", self.0)
    }
}

impl PitchClass {
    pub fn to_freq(&self) -> f64 {
        let a = 220_f64;
        let a2 = 440_f64;
        match self {
            PitchClass::C => a * 2.0f64.powf(3.0 / 12.0),
            PitchClass::Cs => a * 2.0f64.powf(4.0 / 12.0),
            PitchClass::D => a * 2.0f64.powf(5.0 / 12.0),
            PitchClass::Ds => a * 2.0f64.powf(6.0 / 12.0),
            PitchClass::E => a * 2.0f64.powf(7.0 / 12.0),
            PitchClass::F => a * 2.0f64.powf(8.0 / 12.0),
            PitchClass::Fs => a * 2.0f64.powf(9.0 / 12.0),
            PitchClass::G => a * 2.0f64.powf(10.0 / 12.0),
            PitchClass::Gs => a * 2.0f64.powf(11.0 / 12.0),
            PitchClass::A => a2,
            PitchClass::As => a2 * 2.0f64.powf(1.0 / 12.0),
            PitchClass::B => a2 * 2.0f64.powf(2.0 / 12.0),
        }
    }
}

impl Note {
    pub fn to_freq(&self) -> f64 {
        let pitch_freq = self.0.to_freq();

        match self.1 {
            Octave::One => pitch_freq / 8.0,
            Octave::Two => pitch_freq / 4.0,
            Octave::Three => pitch_freq / 2.0,
            Octave::Four => pitch_freq,
            Octave::Five => pitch_freq * 2.0,
            Octave::Six => pitch_freq * 4.0,
            Octave::Seven => pitch_freq * 8.0,
        }
    }

    pub fn get_sample(&self, time: u128) -> f32 {
        let freq = self.to_freq();

        let n_overtones = 20;
        let mut sample: f32 = 0.0;
        for i in 0..n_overtones {
            let freq_fact = 1.0 + 2.0 * i as f64;
            let val = ((time as f64 / 44100.0) * (freq * freq_fact) * 2.0 * std::f64::consts::PI)
                .sin() as f32;
            // triangle wave
            // sample += val / (freq_fact as f32).powi(2);
            // square wave
            sample += val / freq_fact as f32;
        }

        sample /= n_overtones as f32;

        sample
    }
}

impl Add<u8> for Note {
    type Output = Self;

    fn add(self, rhs: u8) -> Self {
        let mut note = self;
        let notenum = note.0 as u8;
        if notenum + rhs > 11 {
            note.1 = note.1 + 1;
        }
        for _ in 0..rhs {
            note.0 += Acc::Sharp;
        }
        note
    }
}

impl FromNode for Chord {
    fn from_node(node: &tree_sitter::Node, source: &str) -> Option<Self> {
        let root = node.child_by_field_name("root")?;

        let root = Note::from_node(&root, source)?;

        let mode = node.child_by_field_name("mode");
        let mode = match mode {
            Some(mode) => mode
                .utf8_text(source.as_bytes())
                .unwrap()
                .try_into()
                .unwrap_or_default(),
            None => Mode::default(),
        };

        // there could be multiple augmentations, if there are none, this will be an empty vector
        let mut walk = node.walk();
        let augm_nodes = node.children_by_field_name("augm", &mut walk);
        let aug_vec: Vec<Aug> = augm_nodes
            .filter_map(|aug| aug.utf8_text(source.as_bytes()).unwrap().try_into().ok())
            .collect();
        let augm = Augs(aug_vec);

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
        for a in &self.2 .0 {
            write!(f, "{}", a)?;
        }
        if let Some(over) = &self.3 {
            write!(f, "/{}", over)?;
        }
        Ok(())
    }
}

impl Chord {
    pub fn to_notes(&self) -> Vec<Note> {
        let mut notes = Vec::new();

        let mut relatives = Vec::new();

        relatives.append(&mut self.1.as_relatives());
        relatives.append(&mut self.2.as_relatives());

        // root note
        notes.push(self.0);

        // mode and augmentation notes
        for relative in relatives {
            let note = self.0 + relative;
            notes.push(note);
        }

        // bass note
        if let Some(bass) = &self.3 {
            let mut bass = *bass;

            // lower the bass to the octave below the root
            bass.1 = notes[0].1 + -1;

            notes.push(bass);
        }

        notes
    }

    pub fn as_freqs(&self) -> Vec<f64> {
        let mut freqs = Vec::new();

        for note in self.to_notes() {
            freqs.push(note.to_freq());
        }

        freqs
    }

    pub fn get_sample(&self, time: u128) -> f32 {
        let freqs = self.as_freqs();

        let mut sample: f32 = 0.0;

        // TODO: write functionally with fold?
        for freq in &freqs {
            let n_overtones = 15;
            for i in 0..n_overtones {
                let freq_fact = 1.0 + 2.0 * i as f64;
                let val =
                    ((time as f64 / 44100.0) * (freq * freq_fact) * 2.0 * std::f64::consts::PI)
                        .sin() as f32;
                // triangle wave
                // sample += val / (freq_fact as f32).powi(2);
                // square wave
                sample += val / freq_fact as f32;
            }
        }

        sample /= freqs.len() as f32;

        sample
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
            "6" => Ok(Self::Six),
            "M6" => Ok(Self::MajSix),
            "7" => Ok(Self::Seven),
            "M7" => Ok(Self::MajSeven),
            "9" => Ok(Self::Nine),
            "M9" => Ok(Self::MajNine),
            "11" => Ok(Self::Eleven),
            "M11" => Ok(Self::MajEleven),
            "13" => Ok(Self::Thirteen),
            "M13" => Ok(Self::MajThirteen),
            _ => Err("Invalid aug"),
        }
    }
}

impl Display for Aug {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Six => write!(f, "6"),
            Self::MajSix => write!(f, "M6"),
            Self::Seven => write!(f, "7"),
            Self::MajSeven => write!(f, "M7"),
            Self::Nine => write!(f, "9"),
            Self::MajNine => write!(f, "M9"),
            Self::Eleven => write!(f, "11"),
            Self::MajEleven => write!(f, "M11"),
            Self::Thirteen => write!(f, "13"),
            Self::MajThirteen => write!(f, "M13"),
        }
    }
}

impl Aug {
    fn as_relatives(&self) -> Vec<u8> {
        match self {
            Aug::Six => vec![8],
            Aug::MajSix => vec![9],
            Aug::Seven => vec![10],
            Aug::MajSeven => vec![11],
            Aug::Nine => vec![13],
            Aug::MajNine => vec![14],
            Aug::Eleven => vec![16],
            Aug::MajEleven => vec![17],
            Aug::Thirteen => vec![20],
            Aug::MajThirteen => vec![21],
        }
    }
}

impl Augs {
    fn as_relatives(&self) -> Vec<u8> {
        let mut relatives = Vec::new();

        for aug in &self.0 {
            relatives.append(&mut aug.as_relatives());
        }

        relatives
    }
}

impl TryFrom<&str> for PitchClass {
    type Error = &'static str;

    fn try_from(s: &str) -> Result<Self, &'static str> {
        match s {
            "A" | "a" => Ok(Self::A),
            "B" | "t" => Ok(Self::B),
            "C" | "c" => Ok(Self::C),
            "D" | "d" => Ok(Self::D),
            "E" | "e" => Ok(Self::E),
            "F" | "f" => Ok(Self::F),
            "G" | "g" => Ok(Self::G),
            _ => Err("Invalid note"),
        }
    }
}

impl Display for PitchClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PitchClass::A => write!(f, "A"),
            PitchClass::As => write!(f, "As"),
            PitchClass::B => write!(f, "B"),
            PitchClass::C => write!(f, "C"),
            PitchClass::Cs => write!(f, "Cs"),
            PitchClass::D => write!(f, "D"),
            PitchClass::Ds => write!(f, "Ds"),
            PitchClass::E => write!(f, "E"),
            PitchClass::F => write!(f, "F"),
            PitchClass::Fs => write!(f, "Fs"),
            PitchClass::G => write!(f, "G"),
            PitchClass::Gs => write!(f, "Gs"),
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

impl Mode {
    fn as_relatives(&self) -> Vec<u8> {
        match self {
            Self::Major => vec![4, 7],
            Self::Minor => vec![3, 7],
            Self::Dim => vec![3, 6],
            Self::Aug => vec![4, 8],
            Self::Sus4 => vec![5, 7],
            Self::Sus2 => vec![2, 7],
        }
    }
}

impl Add<Acc> for PitchClass {
    type Output = Self;

    fn add(self, rhs: Acc) -> Self {
        let pitch: PitchClass =
            num::FromPrimitive::from_i32((self as i32 + rhs as i32) % 12).unwrap();

        pitch
    }
}

impl AddAssign<Acc> for PitchClass {
    fn add_assign(&mut self, rhs: Acc) {
        // HACK: this is a bit of a hack, but it works
        //       i'd rather not have to clone self here...
        let pitch: PitchClass =
            num::FromPrimitive::from_i32((*self as i32 + rhs as i32).rem_euclid(12)).unwrap();
        *self = pitch;
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

impl TryFrom<i8> for Octave {
    type Error = &'static str;

    fn try_from(i: i8) -> Result<Self, &'static str> {
        match i {
            1 => Ok(Self::One),
            2 => Ok(Self::Two),
            3 => Ok(Self::Three),
            4 => Ok(Self::Four),
            5 => Ok(Self::Five),
            6 => Ok(Self::Six),
            7 => Ok(Self::Seven),
            // HACK: when higher than 7, just return 7
            //       also returns 7 for 0 and negative numbers
            _ => Ok(Self::Seven),
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

impl Add<i8> for Octave {
    type Output = Self;

    fn add(self, rhs: i8) -> Self {
        let mut octave_num = self as i8;
        octave_num += rhs;

        octave_num.try_into().unwrap()
    }
}
