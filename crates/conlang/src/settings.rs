//! What the user asks for: a style preset and the knobs it fills in.

use serde::{Deserialize, Serialize};

use crate::phoneme::{Backness, Kind, Manner, Phoneme, Place};

/// Overall sound of the language, the starting point for [`Settings`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Style {
    /// Common sounds in common proportions.
    #[default]
    Balanced,
    /// Rough and guttural: uvulars, velar fricatives, glottal stops, heavy closed syllables.
    Harsh,
    /// Flowing and vowel-rich: sonorants, open syllables, long vowels.
    Melodic,
    /// Hissing: sibilants, affricates and lateral fricatives.
    Sibilant,
}

/// Which syllable carries the main stress.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Stress {
    Initial,
    Second,
    Penultimate,
    Final,
}

impl Stress {
    /// Index of the stressed syllable in a word of `syllables` syllables.
    pub fn index(self, syllables: usize) -> usize {
        let last = syllables.saturating_sub(1);
        match self {
            Stress::Initial => 0,
            Stress::Second => 1.min(last),
            Stress::Penultimate => last.saturating_sub(1),
            Stress::Final => last,
        }
    }
}

/// Generation knobs. Out-of-range values are clamped when a language is generated, so any
/// deserialized value is safe.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub style: Style,
    /// Number of vowel qualities, inclusive range (clamped to 3–12).
    pub vowels: (u8, u8),
    /// Number of consonants, inclusive range (clamped to 6–30).
    pub consonants: (u8, u8),
    /// Longest onset and coda, 0–2 consonants each (onset at least 1).
    pub max_onset: u8,
    pub max_coda: u8,
    /// Chance (%) that a syllable has no coda.
    pub open_syllables: u8,
    /// Chance (%) that a language has phonemic vowel length at all.
    pub vowel_length: u8,
    /// Syllables in a content word, inclusive range (clamped to 1–5).
    pub word_syllables: (u8, u8),
    /// Stress position; `None` lets the seed choose.
    pub stress: Option<Stress>,
    /// Extra words (ASCII, any case) the vocabulary must not contain, on top of the built-in list.
    pub avoid: Vec<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self::preset(Style::Balanced)
    }
}

impl Settings {
    pub fn preset(style: Style) -> Self {
        let base = Settings {
            style,
            vowels: (5, 7),
            consonants: (14, 22),
            max_onset: 2,
            max_coda: 1,
            open_syllables: 55,
            vowel_length: 25,
            word_syllables: (1, 3),
            stress: None,
            avoid: Vec::new(),
        };
        match style {
            Style::Balanced => base,
            Style::Harsh => Settings { vowels: (3, 5), consonants: (18, 26), max_coda: 2, open_syllables: 25, vowel_length: 10, ..base },
            Style::Melodic => Settings { vowels: (5, 8), consonants: (10, 15), open_syllables: 80, vowel_length: 50, word_syllables: (2, 4), ..base },
            Style::Sibilant => Settings { vowels: (4, 6), consonants: (14, 20), max_coda: 2, open_syllables: 40, vowel_length: 15, ..base },
        }
    }

    /// The same settings with every number in its valid range.
    pub(crate) fn clamped(&self) -> Settings {
        let range = |(a, b): (u8, u8), lo: u8, hi: u8| {
            let (a, b) = (a.clamp(lo, hi), b.clamp(lo, hi));
            if a <= b { (a, b) } else { (b, a) }
        };
        Settings {
            style: self.style,
            vowels: range(self.vowels, 3, 12),
            consonants: range(self.consonants, 6, 30),
            max_onset: self.max_onset.clamp(1, 2),
            max_coda: self.max_coda.min(2),
            open_syllables: self.open_syllables.min(100),
            vowel_length: self.vowel_length.min(100),
            word_syllables: range(self.word_syllables, 1, 5),
            stress: self.stress,
            avoid: self.avoid.iter().take(MAX_AVOID).map(|w| w.chars().take(64).collect()).collect(),
        }
    }
}

/// Most entries kept from [`Settings::avoid`].
pub const MAX_AVOID: usize = 10_000;

/// Sampling weight multiplier (percent) the style gives a phoneme.
pub(crate) fn style_weight(style: Style, p: &Phoneme) -> u32 {
    match (style, p.kind) {
        (Style::Balanced, _) => 100,
        (Style::Harsh, Kind::Consonant { place, manner, voiced }) => match (place, manner) {
            (Place::Uvular, _) => 600,
            (Place::Velar, Manner::Fricative) => 500,
            (Place::Glottal, _) => 300,
            (_, Manner::Trill) => 300,
            (_, Manner::Affricate) => 150,
            (_, Manner::Stop) if !voiced => 150,
            (_, Manner::Approximant) => 50,
            _ => 100,
        },
        (Style::Harsh, Kind::Vowel { back, round, .. }) => match (back, round) {
            (Backness::Front, true) => 20,
            (Backness::Back | Backness::Central, _) => 200,
            _ => 80,
        },
        (Style::Melodic, Kind::Consonant { place, manner, voiced }) => match (place, manner) {
            (_, Manner::Nasal | Manner::Lateral | Manner::Approximant | Manner::Trill) => 250,
            (Place::Uvular | Place::Glottal, _) => 15,
            (Place::Velar, Manner::Fricative) => 15,
            (_, Manner::Affricate | Manner::LateralFricative) => 20,
            _ if voiced => 130,
            _ => 100,
        },
        (Style::Melodic, Kind::Vowel { .. }) => 100,
        (Style::Sibilant, Kind::Consonant { place, manner, .. }) => match (place, manner) {
            (Place::Alveolar | Place::Postalveolar, Manner::Fricative) => 500,
            (_, Manner::Affricate) => 350,
            (_, Manner::LateralFricative) => 400,
            (Place::Uvular, _) => 40,
            _ => 100,
        },
        (Style::Sibilant, Kind::Vowel { back, .. }) => match back {
            Backness::Front => 180,
            _ => 100,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stress_index_handles_short_words() {
        for s in [Stress::Initial, Stress::Second, Stress::Penultimate, Stress::Final] {
            assert_eq!(s.index(0), 0);
            assert_eq!(s.index(1), 0);
        }
        assert_eq!(Stress::Second.index(4), 1);
        assert_eq!(Stress::Penultimate.index(4), 2);
        assert_eq!(Stress::Final.index(4), 3);
    }

    #[test]
    fn clamping_fixes_hostile_values() {
        let s = Settings {
            vowels: (200, 0),
            consonants: (0, 0),
            max_onset: 9,
            max_coda: 9,
            open_syllables: 255,
            vowel_length: 255,
            word_syllables: (0, 99),
            ..Settings::default()
        };
        let c = s.clamped();
        assert_eq!(c.vowels, (3, 12));
        assert_eq!(c.consonants, (6, 6));
        assert_eq!((c.max_onset, c.max_coda, c.open_syllables, c.vowel_length), (2, 2, 100, 100));
        assert_eq!(c.word_syllables, (1, 5));
    }
}
