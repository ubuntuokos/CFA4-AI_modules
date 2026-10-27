//! Constructed-language generator for CFA4 (`AU-060`, the core without AI).
//!
//! [`Language::generate`] builds a language from a name, a seed and [`Settings`] (a [`Style`]
//! preset plus knobs): a sound system ([`Phonology`]: inventory, syllable shape, stress), a Latin
//! romanization with IPA, a few [`SoundChange`]s that give the vocabulary a history, a grammar
//! sketch ([`Grammar`]) and a starting vocabulary of basic concepts. The same name, seed and
//! settings always give the same language, on every platform. New words are coined on demand with
//! [`Language::coin`] and never repeat an existing spelling; hand-written words go in with
//! [`Language::set_word`]. Every spelling reads back to exactly one pronunciation.
//!
//! [`Language::espeak`] writes eSpeak NG pronunciation files, so the language can be spoken right
//! away. The stored form (serde) keeps phonemes as IPA and words as spellings, so a person can read
//! and edit it; loading validates everything (hostile input is an error, never a panic).
//!
//! The generated language belongs to the user. Translating script lines into it with a local LLM
//! is a later, separate step that uses this crate's vocabulary and grammar.

#![cfg_attr(
    not(test),
    deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable, clippy::indexing_slicing)
)]

mod espeak;
mod grammar;
mod language;
mod lexicon;
pub mod phoneme;
mod phonology;
mod rng;
mod settings;
mod word;

pub use espeak::{EspeakFiles, PHONEME_TABLE};
pub use grammar::{Grammar, Marking, Position, WordOrder};
pub use language::{ConlangError, FORMAT, Language, MAX_NAME_CHARS};
pub use lexicon::{BASIC_CONCEPTS, CoinError, Entry, MAX_ENTRIES, MAX_GLOSS_CHARS, SoundChange, WordClass};
pub use phonology::Phonology;
pub use settings::{MAX_AVOID, Settings, Stress, Style};
pub use word::{MAX_SEGMENTS, Orthography, Segment, Word};
