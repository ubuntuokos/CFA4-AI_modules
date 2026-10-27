//! Vocabulary: basic concepts, sound changes and the filter against unlucky real words.

use serde::{Deserialize, Serialize};

use crate::phoneme::{Kind, PhonemeId, voicing_partner};
use crate::phonology::Phonology;
use crate::rng::Rng;
use crate::word::{Orthography, Segment, Word};

/// Part of speech of a vocabulary entry; it decides the word length.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WordClass {
    Noun,
    Verb,
    Adjective,
    Pronoun,
    Number,
    Adverb,
    /// Grammatical morphemes (plural, tense, negation, question): one short syllable.
    Grammatical,
}

/// The starting vocabulary: basic, culture-neutral concepts (an original list in the spirit of
/// the public-domain Swadesh lists), English glosses.
pub const BASIC_CONCEPTS: &[(&str, WordClass)] = {
    use WordClass::{Adjective as A, Adverb as D, Noun as N, Number as U, Pronoun as P, Verb as V};
    &[
        ("I", P),
        ("you", P),
        ("he/she/it", P),
        ("we", P),
        ("you (plural)", P),
        ("they", P),
        ("this", P),
        ("that", P),
        ("who", P),
        ("what", P),
        ("where", D),
        ("when", D),
        ("here", D),
        ("there", D),
        ("not", D),
        ("all", A),
        ("many", A),
        ("some", A),
        ("other", A),
        ("one", U),
        ("two", U),
        ("three", U),
        ("four", U),
        ("five", U),
        ("ten", U),
        ("big", A),
        ("long", A),
        ("small", A),
        ("short", A),
        ("wide", A),
        ("narrow", A),
        ("heavy", A),
        ("new", A),
        ("old", A),
        ("good", A),
        ("bad", A),
        ("warm", A),
        ("cold", A),
        ("full", A),
        ("dry", A),
        ("wet", A),
        ("red", A),
        ("green", A),
        ("yellow", A),
        ("white", A),
        ("black", A),
        ("night", N),
        ("day", N),
        ("year", N),
        ("woman", N),
        ("man", N),
        ("person", N),
        ("child", N),
        ("mother", N),
        ("father", N),
        ("friend", N),
        ("enemy", N),
        ("animal", N),
        ("fish", N),
        ("bird", N),
        ("dog", N),
        ("snake", N),
        ("tree", N),
        ("forest", N),
        ("seed", N),
        ("leaf", N),
        ("root", N),
        ("flower", N),
        ("grass", N),
        ("meat", N),
        ("blood", N),
        ("bone", N),
        ("egg", N),
        ("horn", N),
        ("tail", N),
        ("feather", N),
        ("hair", N),
        ("head", N),
        ("ear", N),
        ("eye", N),
        ("nose", N),
        ("mouth", N),
        ("tooth", N),
        ("tongue", N),
        ("hand", N),
        ("foot", N),
        ("heart", N),
        ("sun", N),
        ("moon", N),
        ("star", N),
        ("water", N),
        ("rain", N),
        ("river", N),
        ("lake", N),
        ("sea", N),
        ("salt", N),
        ("stone", N),
        ("sand", N),
        ("earth", N),
        ("cloud", N),
        ("sky", N),
        ("wind", N),
        ("fire", N),
        ("smoke", N),
        ("ash", N),
        ("road", N),
        ("mountain", N),
        ("house", N),
        ("name", N),
        ("word", N),
        ("song", N),
        ("sword", N),
        ("king", N),
        ("god", N),
        ("spirit", N),
        ("war", N),
        ("peace", N),
        ("life", N),
        ("death", N),
        ("time", N),
        ("eat", V),
        ("drink", V),
        ("bite", V),
        ("breathe", V),
        ("sleep", V),
        ("die", V),
        ("kill", V),
        ("fight", V),
        ("hunt", V),
        ("swim", V),
        ("fly", V),
        ("walk", V),
        ("come", V),
        ("go", V),
        ("sit", V),
        ("stand", V),
        ("give", V),
        ("take", V),
        ("hold", V),
        ("see", V),
        ("hear", V),
        ("know", V),
        ("think", V),
        ("say", V),
        ("sing", V),
        ("love", V),
        ("fear", V),
        ("burn", V),
        ("make", V),
        ("have", V),
        ("be", V),
        ("live", V),
    ]
};

/// Most entries a lexicon may hold.
pub const MAX_ENTRIES: usize = 100_000;
/// Longest gloss, in characters.
pub const MAX_GLOSS_CHARS: usize = 128;

/// Words the vocabulary must never contain (ASCII-folded, matched anywhere in a word). Kept short
/// and partial on purpose: the user adds more in [`crate::Settings::avoid`].
const BLOCKED_SUBSTRINGS: &[&str] = &[
    "fuck", "shit", "cunt", "nigg", "fagg", "whore", "slut", "twat", "piss", "porn", "rape", "dick", "cock", "fasz", "kurv", "geci", "picsa", "punci", "segg",
    "bazd", "buzi", "cigany", "puta", "merde", "scheiss", "fick", "hure", "blyat", "suka", "hitler", "nazi",
];
/// Short words blocked only as a whole word.
const BLOCKED_WORDS: &[&str] = &["ass", "fag", "kys", "szar", "fos", "kaka", "pisi", "cul", "con", "ano"];

/// A change applied to every newly coined word, so the vocabulary looks like it has a history.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SoundChange {
    /// Voiceless obstruents between vowels become voiced (`ata` → `ada`).
    IntervocalicVoicing,
    /// A voiced obstruent at the end of a word becomes voiceless (`ab` → `ap`).
    FinalDevoicing,
    /// `k`, `t`, `s` before a close front vowel become `tʃ`, `ts`, `ʃ` (`ki` → `chi`).
    Palatalization,
}

impl SoundChange {
    pub const ALL: [SoundChange; 3] = [SoundChange::IntervocalicVoicing, SoundChange::FinalDevoicing, SoundChange::Palatalization];

    /// Applies the change; a target sound missing from the language leaves the segment alone.
    pub fn apply(self, word: &mut Word, phonology: &Phonology) {
        let has = |id: PhonemeId| phonology.consonants.contains(&id);
        let snapshot = word.segments.clone();
        let vowel_at = |i: Option<usize>| i.and_then(|i| snapshot.get(i)).is_some_and(Segment::is_vowel);
        let last = snapshot.len().checked_sub(1);
        for (i, seg) in word.segments.iter_mut().enumerate() {
            let Some(p) = seg.phoneme.get() else { continue };
            let Kind::Consonant { voiced, .. } = p.kind else { continue };
            let target = match self {
                SoundChange::IntervocalicVoicing if !voiced && p.is_obstruent() && vowel_at(i.checked_sub(1)) && vowel_at(i.checked_add(1)) => {
                    voicing_partner(seg.phoneme)
                }
                SoundChange::FinalDevoicing if voiced && p.is_obstruent() && Some(i) == last => voicing_partner(seg.phoneme),
                SoundChange::Palatalization if snapshot.get(i + 1).is_some_and(|n| matches!(n.phoneme.ipa(), "i" | "ɪ" | "y")) => {
                    let to = match seg.phoneme.ipa() {
                        "k" => "tʃ",
                        "t" => "ts",
                        "s" => "ʃ",
                        _ => "",
                    };
                    PhonemeId::from_ipa(to)
                }
                _ => None,
            };
            if let Some(t) = target.filter(|t| has(*t)) {
                seg.phoneme = t;
            }
        }
    }
}

/// One vocabulary entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub gloss: String,
    pub class: WordClass,
    pub word: Word,
}

/// Why a word could not be coined.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CoinError {
    #[error("the gloss is empty or longer than {MAX_GLOSS_CHARS} characters")]
    BadGloss,
    #[error("the vocabulary already has {MAX_ENTRIES} entries")]
    Full,
    #[error("no new word found for \"{0}\"; the sound system is too small for this vocabulary")]
    Exhausted(String),
}

/// Normalized gloss used as the lookup key and as the word's sub-seed: trimmed, inner whitespace
/// collapsed, lower case.
pub(crate) fn gloss_key(gloss: &str) -> String {
    gloss.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase()
}

/// Everything needed to coin words deterministically.
pub(crate) struct Coiner<'a> {
    pub seed: u64,
    pub phonology: &'a Phonology,
    pub orthography: &'a Orthography,
    pub changes: &'a [SoundChange],
    pub word_syllables: (u8, u8),
    pub avoid: &'a [String],
}

impl Coiner<'_> {
    /// A new word for `gloss` that differs from every spelling in `taken`. The same seed, gloss
    /// and `taken` always give the same word.
    pub fn coin(&self, gloss: &str, class: WordClass, taken: &dyn Fn(&str) -> bool) -> Result<Word, CoinError> {
        let key = gloss_key(gloss);
        if key.is_empty() || key.chars().count() > MAX_GLOSS_CHARS {
            return Err(CoinError::BadGloss);
        }
        let (lo, hi) = match class {
            WordClass::Pronoun | WordClass::Grammatical => (1, 1),
            WordClass::Number | WordClass::Adverb => (1, 2),
            _ => self.word_syllables,
        };
        const ATTEMPTS: u32 = 96;
        for attempt in 0..ATTEMPTS {
            let mut rng = Rng::derive(self.seed, &format!("word:{key}"), attempt);
            // Short words run out quickly in small sound systems: allow one more syllable every
            // 24 failed attempts.
            let extra = (attempt / 24) as u8;
            let n = rng.range(lo, hi).saturating_add(extra);
            let mut word = self.phonology.random_word(&mut rng, n);
            for change in self.changes {
                change.apply(&mut word, self.phonology);
            }
            if class == WordClass::Grammatical && word.segments.len() > 3 {
                continue;
            }
            if !self.phonology.reads_back(self.orthography, &word) {
                continue;
            }
            let spelling = word.spelling();
            if taken(&spelling) || is_blocked(&spelling, self.avoid) {
                continue;
            }
            return Ok(word);
        }
        Err(CoinError::Exhausted(key))
    }
}

/// Fold used by the filter and for tags: lower case, accents dropped, only ASCII letters and digits
/// kept.
pub(crate) fn fold(s: &str) -> String {
    s.chars()
        .flat_map(char::to_lowercase)
        .filter_map(|c| match c {
            'à' | 'á' | 'â' | 'ä' | 'ã' | 'å' => Some('a'),
            'è' | 'é' | 'ê' | 'ë' => Some('e'),
            'ì' | 'í' | 'î' | 'ï' => Some('i'),
            'ò' | 'ó' | 'ô' | 'ö' | 'ő' | 'õ' | 'ø' | 'œ' => Some('o'),
            'ù' | 'ú' | 'û' | 'ü' | 'ű' => Some('u'),
            c if c.is_ascii_alphanumeric() => Some(c),
            _ => None,
        })
        .collect()
}

/// Whether `spelling` contains a blocked word or equals an avoided one.
pub fn is_blocked(spelling: &str, avoid: &[String]) -> bool {
    let w = fold(spelling);
    BLOCKED_SUBSTRINGS.iter().any(|b| w.contains(b)) || BLOCKED_WORDS.contains(&w.as_str()) || avoid.iter().any(|a| fold(a) == w)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::Settings;
    use std::collections::HashSet;

    fn word(o: &Orthography, s: &str) -> Word {
        o.read(s).unwrap_or_default()
    }

    fn phon(ipa: &[&str]) -> Phonology {
        let ids = |f: bool| ipa.iter().filter_map(|s| PhonemeId::from_ipa(s)).filter(|id| id.get().is_some_and(|p| p.is_vowel()) == f).collect();
        Phonology {
            consonants: ids(false),
            vowels: ids(true),
            long_vowels: false,
            max_onset: 2,
            max_coda: 1,
            open_syllables: 50,
            stress: crate::Stress::Initial,
        }
    }

    #[test]
    fn sound_changes() {
        let p = phon(&["p", "b", "t", "d", "k", "s", "ʃ", "tʃ", "ts", "a", "i"]);
        let o = p.orthography();
        let run = |c: SoundChange, s: &str| {
            let mut w = word(&o, s);
            c.apply(&mut w, &p);
            w.spelling()
        };
        assert_eq!(run(SoundChange::IntervocalicVoicing, "apatak"), "abadak");
        assert_eq!(run(SoundChange::FinalDevoicing, "bad"), "bat");
        assert_eq!(run(SoundChange::Palatalization, "kisitak"), "chishitak");
        // No voiced partner of k in this language: nothing happens.
        assert_eq!(run(SoundChange::IntervocalicVoicing, "aka"), "aka");
    }

    #[test]
    fn filter_folds_and_matches() {
        assert!(is_blocked("Fùck", &[]));
        assert!(is_blocked("kaka", &[]));
        assert!(!is_blocked("kakar", &[]));
        assert!(is_blocked("Bambi", &["bambì".to_owned()]));
        assert!(!is_blocked("tala", &[]));
    }

    #[test]
    fn coining_is_deterministic_unique_and_readable() {
        let settings = Settings::default();
        let p = Phonology::generate(9, &settings);
        let o = p.orthography();
        let coiner = Coiner { seed: 9, phonology: &p, orthography: &o, changes: &SoundChange::ALL, word_syllables: (1, 3), avoid: &[] };
        let mut seen = HashSet::new();
        for (gloss, class) in BASIC_CONCEPTS {
            let w = coiner.coin(gloss, *class, &|s| seen.contains(s)).unwrap_or_default();
            assert_eq!(o.read(&w.spelling()).as_ref(), Some(&w), "{gloss}");
            assert!(seen.insert(w.spelling()), "duplicate {gloss}");
        }
        let again = coiner.coin("sun", WordClass::Noun, &|_| false);
        assert_eq!(again, coiner.coin("  SUN ", WordClass::Noun, &|_| false));
        assert_eq!(coiner.coin(" ", WordClass::Noun, &|_| false), Err(CoinError::BadGloss));
        assert_eq!(coiner.coin(&"x".repeat(200), WordClass::Noun, &|_| false), Err(CoinError::BadGloss));
        assert!(matches!(coiner.coin("sun", WordClass::Noun, &|_| true), Err(CoinError::Exhausted(_))));
    }

    #[test]
    fn concept_glosses_are_unique() {
        let mut seen = HashSet::new();
        assert!(BASIC_CONCEPTS.iter().all(|(g, _)| seen.insert(gloss_key(g))));
    }
}
