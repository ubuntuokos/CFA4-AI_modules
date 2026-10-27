//! The phoneme catalogue every generated language draws from.
//!
//! Each entry has its IPA symbol, its romanization (the letters the language is written with)
//! and its eSpeak NG phoneme mnemonic in the `az` phoneme table, the table that says every entry
//! here as written in every position (checked against eSpeak NG 1.51 by `tests/espeak_oracle.rs`).
//! Long vowels are written with a doubled letter, `ː` in IPA and `:` in eSpeak NG.

use serde::{Deserialize, Serialize};

/// Where a consonant is made.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Place {
    Bilabial,
    Labiodental,
    Dental,
    Alveolar,
    Postalveolar,
    Palatal,
    Velar,
    LabialVelar,
    Uvular,
    Glottal,
}

/// How a consonant is made.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Manner {
    Stop,
    Fricative,
    Affricate,
    Nasal,
    Lateral,
    LateralFricative,
    Trill,
    Approximant,
}

/// Vowel height, close to open.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Height {
    Close,
    NearClose,
    CloseMid,
    OpenMid,
    NearOpen,
    Open,
}

/// Vowel backness.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Backness {
    Front,
    Central,
    Back,
}

/// Phonetic description of a catalogue entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Consonant { place: Place, manner: Manner, voiced: bool },
    Vowel { height: Height, back: Backness, round: bool },
}

/// One catalogue entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Phoneme {
    pub ipa: &'static str,
    pub roman: &'static str,
    pub espeak: &'static str,
    pub kind: Kind,
    /// Rough share of the world's languages that have it (0–100), the base sampling weight.
    pub frequency: u8,
}

impl Phoneme {
    pub fn is_vowel(&self) -> bool {
        matches!(self.kind, Kind::Vowel { .. })
    }

    /// Whether the vowel has a long form (eSpeak NG cannot lengthen `ɯ`).
    pub fn can_be_long(&self) -> bool {
        self.is_vowel() && self.ipa != "ɯ"
    }

    /// Whether the consonant may start or end a word (eSpeak NG drops a word-edge `ʔ`).
    pub fn at_word_edge(&self) -> bool {
        self.ipa != "ʔ"
    }

    pub fn is_obstruent(&self) -> bool {
        matches!(self.kind, Kind::Consonant { manner: Manner::Stop | Manner::Fricative | Manner::Affricate | Manner::LateralFricative, .. })
    }

    /// Nasals, liquids and glides: the consonants allowed second in an onset cluster or first in
    /// a coda cluster.
    pub fn is_sonorant(&self) -> bool {
        matches!(self.kind, Kind::Consonant { manner: Manner::Nasal | Manner::Lateral | Manner::Trill | Manner::Approximant, .. })
    }

    pub fn is_liquid_or_glide(&self) -> bool {
        matches!(self.kind, Kind::Consonant { manner: Manner::Lateral | Manner::Trill | Manner::Approximant, .. })
    }
}

/// Index into [`CATALOG`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PhonemeId(pub(crate) u8);

impl PhonemeId {
    pub fn get(self) -> Option<&'static Phoneme> {
        CATALOG.get(usize::from(self.0))
    }

    /// The catalogue entry with this IPA symbol (without the length mark).
    pub fn from_ipa(ipa: &str) -> Option<Self> {
        CATALOG.iter().position(|p| p.ipa == ipa).and_then(|i| u8::try_from(i).ok()).map(PhonemeId)
    }

    pub fn ipa(self) -> &'static str {
        self.get().map_or("", |p| p.ipa)
    }

    /// Every id in the catalogue, in catalogue order.
    pub fn all() -> impl Iterator<Item = PhonemeId> {
        (0..CATALOG.len()).filter_map(|i| u8::try_from(i).ok()).map(PhonemeId)
    }
}

const fn c(ipa: &'static str, roman: &'static str, espeak: &'static str, place: Place, manner: Manner, voiced: bool, frequency: u8) -> Phoneme {
    Phoneme { ipa, roman, espeak, kind: Kind::Consonant { place, manner, voiced }, frequency }
}

const fn v(ipa: &'static str, roman: &'static str, espeak: &'static str, height: Height, back: Backness, round: bool, frequency: u8) -> Phoneme {
    Phoneme { ipa, roman, espeak, kind: Kind::Vowel { height, back, round }, frequency }
}

use Backness::{Back, Central, Front};
use Height::{Close, CloseMid, NearClose, NearOpen, Open, OpenMid};
use Manner::{Affricate, Approximant, Fricative, Lateral, LateralFricative, Nasal, Stop, Trill};
use Place::{Alveolar, Bilabial, Dental, Glottal, LabialVelar, Labiodental, Palatal, Postalveolar, Uvular, Velar};

/// The catalogue. Frequencies are rounded impressions from cross-linguistic surveys, used only
/// as sampling weights.
pub const CATALOG: &[Phoneme] = &[
    c("p", "p", "p", Bilabial, Stop, false, 85),
    c("b", "b", "b", Bilabial, Stop, true, 65),
    c("t", "t", "t", Alveolar, Stop, false, 90),
    c("d", "d", "d", Alveolar, Stop, true, 65),
    c("k", "k", "k", Velar, Stop, false, 90),
    c("g", "g", "g", Velar, Stop, true, 55),
    c("q", "q", "q", Uvular, Stop, false, 15),
    c("ʔ", "'", "?", Glottal, Stop, false, 40),
    c("f", "f", "f", Labiodental, Fricative, false, 45),
    c("v", "v", "v", Labiodental, Fricative, true, 30),
    c("θ", "th", "T", Dental, Fricative, false, 8),
    c("ð", "dh", "D", Dental, Fricative, true, 8),
    c("s", "s", "s", Alveolar, Fricative, false, 80),
    c("z", "z", "z", Alveolar, Fricative, true, 30),
    c("ʃ", "sh", "S", Postalveolar, Fricative, false, 45),
    c("ʒ", "zh", "Z", Postalveolar, Fricative, true, 15),
    c("x", "kh", "x", Velar, Fricative, false, 25),
    c("ɣ", "gh", "Q", Velar, Fricative, true, 15),
    c("χ", "qh", "X", Uvular, Fricative, false, 8),
    c("h", "h", "h", Glottal, Fricative, false, 60),
    c("ts", "c", "ts", Alveolar, Affricate, false, 20),
    c("dz", "dz", "dz", Alveolar, Affricate, true, 8),
    c("tʃ", "ch", "tS", Postalveolar, Affricate, false, 40),
    c("dʒ", "j", "dZ", Postalveolar, Affricate, true, 25),
    c("m", "m", "m", Bilabial, Nasal, true, 95),
    c("n", "n", "n", Alveolar, Nasal, true, 95),
    c("ɲ", "ny", "n^", Palatal, Nasal, true, 40),
    c("ŋ", "ng", "N", Velar, Nasal, true, 60),
    c("l", "l", "l", Alveolar, Lateral, true, 75),
    c("ɬ", "lh", "l#", Alveolar, LateralFricative, false, 8),
    c("r", "r", "R2", Alveolar, Trill, true, 45),
    c("j", "y", "j", Palatal, Approximant, true, 85),
    c("w", "w", "w", LabialVelar, Approximant, true, 75),
    v("a", "a", "a", Open, Central, false, 90),
    v("e", "e", "e", CloseMid, Front, false, 60),
    v("i", "i", "i", Close, Front, false, 90),
    v("o", "o", "o", CloseMid, Back, true, 60),
    v("u", "u", "u", Close, Back, true, 85),
    v("ɛ", "è", "E", OpenMid, Front, false, 35),
    v("ɔ", "ò", "O", OpenMid, Back, true, 30),
    v("ɪ", "ì", "I", NearClose, Front, false, 15),
    v("ʊ", "ù", "U", NearClose, Back, true, 12),
    v("y", "ü", "y", Close, Front, true, 8),
    v("ø", "ö", "Y", CloseMid, Front, true, 5),
    v("œ", "œ", "W", OpenMid, Front, true, 3),
    v("ɯ", "ï", "@", Close, Back, false, 6),
    v("æ", "ä", "&", NearOpen, Front, false, 10),
];

/// The counterpart differing only in voicing, if the catalogue has one.
pub fn voicing_partner(id: PhonemeId) -> Option<PhonemeId> {
    let Kind::Consonant { place, manner, voiced } = id.get()?.kind else {
        return None;
    };
    PhonemeId::all().find(|other| {
        matches!(other.get().map(|p| p.kind), Some(Kind::Consonant { place: p2, manner: m2, voiced: v2 }) if p2 == place && m2 == manner && v2 != voiced)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn catalogue_symbols_are_unique() {
        let mut ipa = HashSet::new();
        let mut roman = HashSet::new();
        let mut espeak = HashSet::new();
        for p in CATALOG {
            assert!(ipa.insert(p.ipa), "duplicate IPA {}", p.ipa);
            assert!(roman.insert(p.roman), "duplicate romanization {}", p.roman);
            assert!(espeak.insert(p.espeak), "duplicate eSpeak {}", p.espeak);
            assert!(!p.roman.is_empty() && p.roman.chars().all(|c| c == '\'' || c.is_lowercase()));
        }
        assert!(CATALOG.len() < usize::from(u8::MAX));
    }

    #[test]
    fn voicing_partners() {
        let id = |s| PhonemeId::from_ipa(s).map(|i| i.ipa());
        let partner = |s| PhonemeId::from_ipa(s).and_then(voicing_partner).map(PhonemeId::ipa);
        assert_eq!(partner("p"), Some("b"));
        assert_eq!(partner("ʒ"), Some("ʃ"));
        assert_eq!(partner("tʃ"), Some("dʒ"));
        assert_eq!(partner("h"), None);
        assert_eq!(partner("a"), None);
        assert_eq!(id("nope"), None);
    }
}
