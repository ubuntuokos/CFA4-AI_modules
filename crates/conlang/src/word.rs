//! Words as phoneme sequences: spelling, IPA and syllables.

use crate::phoneme::{Kind, Manner, PhonemeId};
use crate::settings::Stress;

/// Longest word kept, in segments.
pub const MAX_SEGMENTS: usize = 64;

/// One sound: a catalogue phoneme, long or short (only vowels are ever long).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Segment {
    pub phoneme: PhonemeId,
    pub long: bool,
}

impl Segment {
    pub fn short(phoneme: PhonemeId) -> Self {
        Self { phoneme, long: false }
    }

    pub fn is_vowel(&self) -> bool {
        self.phoneme.get().is_some_and(|p| p.is_vowel())
    }

    fn roman(&self) -> String {
        let r = self.phoneme.get().map_or("", |p| p.roman);
        if self.long { r.repeat(2) } else { r.to_owned() }
    }

    fn ipa(&self) -> String {
        let mut s = self.phoneme.ipa().to_owned();
        if self.long {
            s.push('ː');
        }
        s
    }

    pub(crate) fn espeak(&self) -> String {
        let mut s = self.phoneme.get().map_or("", |p| p.espeak).to_owned();
        if self.long {
            s.push(':');
        }
        s
    }
}

/// A word of the language.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct Word {
    pub segments: Vec<Segment>,
}

impl Word {
    /// The romanized spelling.
    pub fn spelling(&self) -> String {
        self.segments.iter().map(Segment::roman).collect()
    }

    /// Number of syllables (one per vowel).
    pub fn syllable_count(&self) -> usize {
        self.segments.iter().filter(|s| s.is_vowel()).count()
    }

    /// Start index (into `segments`) of every syllable, by the maximal-onset principle with
    /// onsets of at most `max_onset` consonants, a two-consonant onset being an obstruent
    /// followed by a liquid or glide.
    pub fn syllable_starts(&self, max_onset: u8) -> Vec<usize> {
        let vowels: Vec<usize> = self.segments.iter().enumerate().filter(|(_, s)| s.is_vowel()).map(|(i, _)| i).collect();
        let mut starts = Vec::with_capacity(vowels.len());
        if vowels.is_empty() {
            return starts;
        }
        starts.push(0);
        for pair in vowels.windows(2) {
            let &[a, b] = pair else { continue };
            let between = self.segments.get(a + 1..b).unwrap_or(&[]);
            let mut k = between.len().min(usize::from(max_onset));
            while k > 1 && !valid_onset(between.get(between.len() - k..).unwrap_or(&[])) {
                k -= 1;
            }
            starts.push(b - k);
        }
        starts
    }

    /// IPA with the primary stress mark before the stressed syllable.
    pub fn ipa(&self, stress: Stress, max_onset: u8) -> String {
        let starts = self.syllable_starts(max_onset);
        let mark = if starts.len() > 1 { starts.get(stress.index(starts.len())).copied() } else { None };
        let mut out = String::new();
        for (i, seg) in self.segments.iter().enumerate() {
            if Some(i) == mark {
                out.push('ˈ');
            }
            out.push_str(&seg.ipa());
        }
        out
    }
}

/// Whether `cluster` may begin a syllable.
pub(crate) fn valid_onset(cluster: &[Segment]) -> bool {
    match cluster {
        [] | [_] => true,
        [a, b] => {
            let a = a.phoneme.get();
            let b = b.phoneme.get();
            a.is_some_and(|a| a.is_obstruent() && !matches!(a.kind, Kind::Consonant { manner: Manner::Affricate, .. }))
                && b.is_some_and(|b| b.is_liquid_or_glide())
        }
        _ => false,
    }
}

/// Reads a spelling back into segments with greedy longest match over a fixed set of
/// graphemes (the language's own letters).
#[derive(Debug, Clone)]
pub struct Orthography {
    /// (grapheme as chars, segment), longest graphemes first.
    graphemes: Vec<(Vec<char>, Segment)>,
}

impl Orthography {
    /// The letters of an inventory; vowels get their long (doubled) form when `long_vowels`.
    pub fn new(inventory: impl IntoIterator<Item = PhonemeId>, long_vowels: bool) -> Self {
        let mut graphemes = Vec::new();
        for id in inventory {
            let Some(p) = id.get() else { continue };
            graphemes.push((p.roman.chars().collect(), Segment::short(id)));
            if long_vowels && p.can_be_long() {
                graphemes.push((p.roman.repeat(2).chars().collect(), Segment { phoneme: id, long: true }));
            }
        }
        graphemes.sort_by_key(|g: &(Vec<char>, Segment)| std::cmp::Reverse(g.0.len()));
        Self { graphemes }
    }

    /// The segments of `text` (any case), or `None` if a letter is not in the language or the
    /// word is longer than [`MAX_SEGMENTS`].
    pub fn read(&self, text: &str) -> Option<Word> {
        let chars: Vec<char> = text.chars().flat_map(char::to_lowercase).collect();
        let mut rest = chars.as_slice();
        let mut segments = Vec::new();
        while !rest.is_empty() {
            if segments.len() >= MAX_SEGMENTS {
                return None;
            }
            let (g, seg) = self.graphemes.iter().find(|(g, _)| rest.starts_with(g))?;
            segments.push(*seg);
            rest = rest.get(g.len()..)?;
        }
        Some(Word { segments })
    }

    /// Every grapheme with its segment, longest first.
    pub fn graphemes(&self) -> impl Iterator<Item = (String, Segment)> + '_ {
        self.graphemes.iter().map(|(g, s)| (g.iter().collect(), *s))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(ipa: &[&str]) -> Vec<PhonemeId> {
        ipa.iter().filter_map(|s| PhonemeId::from_ipa(s)).collect()
    }

    #[test]
    fn greedy_reading_prefers_digraphs() {
        let o = Orthography::new(ids(&["s", "h", "ʃ", "a", "k", "x"]), true);
        let w = o.read("Shaakha").unwrap_or_default();
        let ipa: Vec<_> = w.segments.iter().map(|s| (s.phoneme.ipa(), s.long)).collect();
        assert_eq!(ipa, [("ʃ", false), ("a", true), ("x", false), ("a", false)]);
        assert!(o.read("shq").is_none());
        assert!(o.read(&"a".repeat(1000)).is_none());
        assert_eq!(o.read(""), Some(Word::default()));
    }

    #[test]
    fn syllables_and_ipa() {
        let o = Orthography::new(ids(&["t", "k", "r", "s", "a", "i", "o"]), true);
        let w = o.read("astrakoo").unwrap_or_default();
        // a.stra.koː with max onset 2: "s" + "tr" — only "tr" is a valid onset.
        assert_eq!(w.syllable_starts(2), [0, 2, 5]);
        assert_eq!(w.syllable_starts(1), [0, 3, 5]);
        assert_eq!(w.ipa(Stress::Penultimate, 2), "asˈtrakoː");
        assert_eq!(w.ipa(Stress::Initial, 2), "ˈastrakoː");
        assert_eq!(w.ipa(Stress::Final, 2), "astraˈkoː");
        let one = o.read("tak").unwrap_or_default();
        assert_eq!(one.ipa(Stress::Final, 2), "tak");
        assert_eq!(one.syllable_count(), 1);
    }
}
