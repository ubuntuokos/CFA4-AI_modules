//! A whole generated language and its stored form.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::grammar::{Grammar, Marking};
use crate::lexicon::{BASIC_CONCEPTS, CoinError, Coiner, Entry, MAX_ENTRIES, MAX_GLOSS_CHARS, SoundChange, WordClass, gloss_key, is_blocked};
use crate::phoneme::PhonemeId;
use crate::phonology::Phonology;
use crate::rng::Rng;
use crate::settings::{Settings, Stress};
use crate::word::{MAX_SEGMENTS, Word};

/// Version of the stored form written by this crate.
pub const FORMAT: u32 = 1;
/// Longest language name, in characters.
pub const MAX_NAME_CHARS: usize = 64;

/// Why a language could not be generated, loaded or extended.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ConlangError {
    #[error("unsupported language format {0} (this version reads {FORMAT})")]
    UnsupportedFormat(u32),
    #[error("the name must be 1–{MAX_NAME_CHARS} characters without control characters")]
    InvalidName,
    #[error("invalid language tag \"{0}\"")]
    InvalidTag(String),
    #[error("invalid sound system: {0}")]
    InvalidPhonology(String),
    #[error("invalid vocabulary entry \"{gloss}\": {reason}")]
    InvalidEntry { gloss: String, reason: String },
    #[error("the vocabulary has no word for \"{0}\"")]
    UnknownGloss(String),
    #[error(transparent)]
    Coin(#[from] CoinError),
}

/// A constructed language: sound system, sound changes, grammar sketch and vocabulary. It belongs
/// to the user who generated it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "LanguageData", into = "LanguageData")]
pub struct Language {
    pub name: String,
    /// BCP 47 tag, `art-x-<name>` for generated languages.
    pub tag: String,
    pub seed: u64,
    pub settings: Settings,
    pub phonology: Phonology,
    pub sound_changes: Vec<SoundChange>,
    pub grammar: Grammar,
    entries: Vec<Entry>,
    by_gloss: HashMap<String, usize>,
    /// How many entries use each spelling (hand edits may share one).
    spellings: HashMap<String, usize>,
}

impl Language {
    /// Generates a language. The same name, seed and settings always give the same language.
    pub fn generate(name: &str, seed: u64, settings: &Settings) -> Result<Self, ConlangError> {
        let name = check_name(name)?;
        let settings = settings.clamped();
        let phonology = Phonology::generate(seed, &settings);
        let mut rng = Rng::derive(seed, "sound-changes", 0);
        let sound_changes = SoundChange::ALL.into_iter().filter(|_| rng.chance(45)).collect();
        let grammar = Grammar::generate(seed);
        let tag = format!("art-x-{}", subtag(&name));
        let mut lang =
            Language { name, tag, seed, settings, phonology, sound_changes, grammar, entries: Vec::new(), by_gloss: HashMap::new(), spellings: HashMap::new() };
        for gloss in Grammar::MORPHEMES {
            lang.coin(gloss, WordClass::Grammatical)?;
        }
        for (gloss, class) in BASIC_CONCEPTS {
            lang.coin(gloss, *class)?;
        }
        Ok(lang)
    }

    /// The vocabulary, in the order the words were added.
    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    /// The entry for `gloss` (case and spacing ignored).
    pub fn word(&self, gloss: &str) -> Option<&Entry> {
        self.by_gloss.get(&gloss_key(gloss)).and_then(|&i| self.entries.get(i))
    }

    /// The word for `gloss`, coining a new one if the language has none yet. A new word never
    /// repeats an existing spelling, so the vocabulary stays consistent for the whole film.
    pub fn coin(&mut self, gloss: &str, class: WordClass) -> Result<&Entry, ConlangError> {
        if let Some(&i) = self.by_gloss.get(&gloss_key(gloss)) {
            return self.entries.get(i).ok_or_else(|| CoinError::Exhausted(gloss_key(gloss)).into());
        }
        if self.entries.len() >= MAX_ENTRIES {
            return Err(CoinError::Full.into());
        }
        let orthography = self.phonology.orthography();
        let coiner = Coiner {
            seed: self.seed,
            phonology: &self.phonology,
            orthography: &orthography,
            changes: &self.sound_changes,
            word_syllables: self.settings.word_syllables,
            avoid: &self.settings.avoid,
        };
        let spellings = &self.spellings;
        let word = coiner.coin(gloss, class, &|s| spellings.contains_key(s))?;
        self.push(gloss_key(gloss), class, word)
    }

    /// Adds a word the user wrote (a hand edit, or data the user licensed for a rights-held
    /// language). It must use only the language's letters; it replaces an existing entry with the
    /// same gloss.
    pub fn set_word(&mut self, gloss: &str, class: WordClass, spelling: &str) -> Result<&Entry, ConlangError> {
        let key = gloss_key(gloss);
        let word = self.parse_entry(&key, spelling)?;
        if let Some(&i) = self.by_gloss.get(&key) {
            let old = self.entries.get(i).map(|e| e.word.spelling());
            if let Some(old) = old {
                self.release(&old);
            }
            *self.spellings.entry(word.spelling()).or_insert(0) += 1;
            let entry = self.entries.get_mut(i).ok_or_else(|| invalid(&key, "index out of range"))?;
            entry.class = class;
            entry.word = word;
            return Ok(entry);
        }
        if self.entries.len() >= MAX_ENTRIES {
            return Err(CoinError::Full.into());
        }
        self.push(key, class, word)
    }

    /// IPA of `word` with this language's stress.
    pub fn ipa(&self, word: &Word) -> String {
        word.ipa(self.phonology.stress, self.phonology.max_onset)
    }

    /// Whether a spelling would be filtered as an unlucky real word.
    pub fn is_blocked(&self, spelling: &str) -> bool {
        is_blocked(spelling, &self.settings.avoid)
    }

    /// Drops one use of `spelling`; it stays reserved while another entry uses it.
    fn release(&mut self, spelling: &str) {
        if let Some(n) = self.spellings.get_mut(spelling) {
            *n = n.saturating_sub(1);
            if *n == 0 {
                self.spellings.remove(spelling);
            }
        }
    }

    /// Appends an entry and returns it.
    fn push(&mut self, key: String, class: WordClass, word: Word) -> Result<&Entry, ConlangError> {
        let i = self.entries.len();
        *self.spellings.entry(word.spelling()).or_insert(0) += 1;
        self.by_gloss.insert(key.clone(), i);
        self.entries.push(Entry { gloss: key.clone(), class, word });
        self.entries.get(i).ok_or_else(|| invalid(&key, "was not stored"))
    }

    fn parse_entry(&self, key: &str, spelling: &str) -> Result<Word, ConlangError> {
        if key.is_empty() || key.chars().count() > MAX_GLOSS_CHARS {
            return Err(invalid(key, "the gloss is empty or too long"));
        }
        let word = self.phonology.orthography().read(spelling).ok_or_else(|| invalid(key, "uses letters this language does not have, or is too long"))?;
        if word.syllable_count() == 0 {
            return Err(invalid(key, "has no vowel"));
        }
        Ok(word)
    }
}

fn invalid(gloss: &str, reason: &str) -> ConlangError {
    ConlangError::InvalidEntry { gloss: gloss.chars().take(MAX_GLOSS_CHARS).collect(), reason: reason.to_owned() }
}

fn check_name(name: &str) -> Result<String, ConlangError> {
    let name = name.trim();
    let n = name.chars().count();
    if n == 0 || n > MAX_NAME_CHARS || name.chars().any(char::is_control) {
        return Err(ConlangError::InvalidName);
    }
    Ok(name.to_owned())
}

/// The private-use subtag for a name: its letters and digits with accents dropped, at most 8,
/// `conlang` if none.
pub(crate) fn subtag(name: &str) -> String {
    let s: String = crate::lexicon::fold(name).chars().take(8).collect();
    if s.is_empty() { "conlang".to_owned() } else { s }
}

/// A BCP 47 tag in the simple form used here: 1–8 ASCII letters or digits per subtag, at most 64
/// characters. (The SDK schema checks full RFC 5646 syntax.)
fn check_tag(tag: &str) -> Result<(), ConlangError> {
    let ok = !tag.is_empty() && tag.len() <= 64 && tag.split('-').all(|s| (1..=8).contains(&s.len()) && s.bytes().all(|b| b.is_ascii_alphanumeric()));
    if ok { Ok(()) } else { Err(ConlangError::InvalidTag(tag.chars().take(64).collect())) }
}

// ---- stored form ----

/// The JSON form of a [`Language`]: phonemes as IPA, words as spellings, so a person can read
/// and edit it.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct LanguageData {
    format: u32,
    name: String,
    tag: String,
    seed: u64,
    #[serde(default)]
    settings: Settings,
    phonology: PhonologyData,
    #[serde(default)]
    sound_changes: Vec<SoundChange>,
    grammar: Grammar,
    lexicon: Vec<EntryData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PhonologyData {
    consonants: Vec<String>,
    vowels: Vec<String>,
    long_vowels: bool,
    max_onset: u8,
    max_coda: u8,
    open_syllables: u8,
    stress: Stress,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EntryData {
    gloss: String,
    class: WordClass,
    spelling: String,
    /// Written for readers; ignored when loading.
    #[serde(default, skip_deserializing)]
    ipa: String,
}

impl From<Language> for LanguageData {
    fn from(l: Language) -> Self {
        let ipa = |ids: &[PhonemeId]| ids.iter().map(|id| id.ipa().to_owned()).collect();
        let lexicon =
            l.entries.iter().map(|e| EntryData { gloss: e.gloss.clone(), class: e.class, spelling: e.word.spelling(), ipa: l.ipa(&e.word) }).collect();
        LanguageData {
            format: FORMAT,
            phonology: PhonologyData {
                consonants: ipa(&l.phonology.consonants),
                vowels: ipa(&l.phonology.vowels),
                long_vowels: l.phonology.long_vowels,
                max_onset: l.phonology.max_onset,
                max_coda: l.phonology.max_coda,
                open_syllables: l.phonology.open_syllables,
                stress: l.phonology.stress,
            },
            name: l.name,
            tag: l.tag,
            seed: l.seed,
            settings: l.settings,
            sound_changes: l.sound_changes,
            grammar: l.grammar,
            lexicon,
        }
    }
}

impl TryFrom<LanguageData> for Language {
    type Error = ConlangError;

    fn try_from(d: LanguageData) -> Result<Self, ConlangError> {
        if d.format != FORMAT {
            return Err(ConlangError::UnsupportedFormat(d.format));
        }
        let name = check_name(&d.name)?;
        check_tag(&d.tag)?;
        let bad = |m: &str| ConlangError::InvalidPhonology(m.to_owned());
        let ids = |list: &[String], vowel: bool| -> Result<Vec<PhonemeId>, ConlangError> {
            let mut out: Vec<PhonemeId> = Vec::new();
            for s in list.iter().take(crate::phoneme::CATALOG.len().saturating_add(1)) {
                let id = PhonemeId::from_ipa(s).ok_or_else(|| bad(&format!("unknown phoneme \"{}\"", s.chars().take(16).collect::<String>())))?;
                if id.get().is_some_and(|p| p.is_vowel()) != vowel {
                    return Err(bad(&format!("\"{}\" is in the wrong list", id.ipa())));
                }
                if out.contains(&id) {
                    return Err(bad(&format!("\"{}\" is listed twice", id.ipa())));
                }
                out.push(id);
            }
            out.sort();
            Ok(out)
        };
        let p = &d.phonology;
        let consonants = ids(&p.consonants, false)?;
        let vowels = ids(&p.vowels, true)?;
        if consonants.is_empty() || vowels.is_empty() {
            return Err(bad("needs at least one consonant and one vowel"));
        }
        if !(1..=2).contains(&p.max_onset) || p.max_coda > 2 || p.open_syllables > 100 {
            return Err(bad("syllable limits out of range"));
        }
        let phonology = Phonology {
            consonants,
            vowels,
            long_vowels: p.long_vowels,
            max_onset: p.max_onset,
            max_coda: p.max_coda,
            open_syllables: p.open_syllables,
            stress: p.stress,
        };
        if d.lexicon.len() > MAX_ENTRIES {
            return Err(CoinError::Full.into());
        }
        let mut sound_changes = d.sound_changes;
        sound_changes.dedup();
        sound_changes.truncate(SoundChange::ALL.len());
        let mut lang = Language {
            name,
            tag: d.tag,
            seed: d.seed,
            settings: d.settings.clamped(),
            phonology,
            sound_changes,
            grammar: d.grammar,
            entries: Vec::with_capacity(d.lexicon.len()),
            by_gloss: HashMap::new(),
            spellings: HashMap::new(),
        };
        for e in d.lexicon {
            let key = gloss_key(&e.gloss);
            if e.spelling.chars().count() > MAX_SEGMENTS * 4 {
                return Err(invalid(&key, "is too long"));
            }
            let word = lang.parse_entry(&key, &e.spelling)?;
            if lang.by_gloss.contains_key(&key) {
                return Err(invalid(&key, "appears twice"));
            }
            lang.push(key, e.class, word)?;
        }
        for (gloss, marking) in lang.grammar.markings() {
            if marking != Marking::Unmarked && lang.word(gloss).is_none() {
                return Err(invalid(gloss, "the grammar uses this morpheme but the vocabulary lacks it"));
            }
        }
        Ok(lang)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::Style;

    #[test]
    fn generation_is_deterministic() {
        let a = Language::generate("Dwarvish", 7, &Settings::default());
        assert!(a.is_ok());
        assert_eq!(a, Language::generate("Dwarvish", 7, &Settings::default()));
        assert_ne!(a, Language::generate("Dwarvish", 8, &Settings::default()));
    }

    #[test]
    fn names_and_tags() {
        assert_eq!(subtag("Ősi Tünde nyelv"), "ositunde");
        assert_eq!(subtag("ÁÉ"), "ae");
        assert_eq!(subtag("ㄱ—!"), "conlang");
        assert!(check_name("").is_err() && check_name("a\u{7}").is_err() && check_name(&"x".repeat(65)).is_err());
        assert!(check_tag("art-x-elf").is_ok() && check_tag("tlh").is_ok());
        assert!(check_tag("art-x-toolongsubtag").is_err() && check_tag("a--b").is_err() && check_tag("").is_err());
        let l = Language::generate("  Ősi Tünde nyelv ", 1, &Settings::preset(Style::Melodic));
        assert_eq!(l.map(|l| (l.name, l.tag)), Ok(("Ősi Tünde nyelv".to_owned(), "art-x-ositunde".to_owned())));
    }

    #[test]
    fn coin_and_set_word() -> Result<(), ConlangError> {
        let mut l = Language::generate("Test", 3, &Settings::default())?;
        let n = l.entries().len();
        assert_eq!(n, Grammar::MORPHEMES.len() + BASIC_CONCEPTS.len());
        let sun = l.word("Sun").map(|e| e.word.clone());
        assert_eq!(l.coin("sun", WordClass::Noun)?.word, sun.unwrap_or_default());
        assert_eq!(l.entries().len(), n);
        let dragon = l.coin("dragon", WordClass::Noun)?.word.spelling();
        assert_eq!(l.entries().len(), n + 1);
        assert!(l.entries().iter().filter(|e| e.word.spelling() == dragon).count() == 1);
        // A hand-written word replaces the generated one.
        let letters: String = l.word("sun").map(|e| e.word.spelling()).unwrap_or_default();
        let new = format!("{letters}{letters}");
        assert_eq!(l.set_word("dragon", WordClass::Noun, &new)?.word.spelling(), new);
        assert_eq!(l.entries().len(), n + 1);
        assert!(matches!(l.set_word("x", WordClass::Noun, "§"), Err(ConlangError::InvalidEntry { .. })));
        Ok(())
    }

    #[test]
    fn shared_spelling_stays_reserved_after_replacement() -> Result<(), ConlangError> {
        let mut l = Language::generate("Test", 3, &Settings::default())?;
        let sun = l.word("sun").map(|e| e.word.spelling()).unwrap_or_default();
        // "star" shares the spelling of "sun", then "sun" gets a new word.
        l.set_word("star", WordClass::Noun, &sun)?;
        let other = format!("{sun}{sun}");
        l.set_word("sun", WordClass::Noun, &other)?;
        // "star" still uses the old spelling, so no new word may take it.
        assert!(l.spellings.contains_key(&sun));
        for i in 0..300 {
            let w = l.coin(&format!("thing {i}"), WordClass::Noun)?.word.spelling();
            assert!(w != sun && w != other);
        }
        Ok(())
    }

    #[test]
    fn huge_spelling_is_refused_without_reading_it_all() -> Result<(), ConlangError> {
        let mut l = Language::generate("Test", 3, &Settings::default())?;
        let huge = "a".repeat(50_000_000);
        assert!(matches!(l.set_word("x", WordClass::Noun, &huge), Err(ConlangError::InvalidEntry { .. })));
        Ok(())
    }
}
