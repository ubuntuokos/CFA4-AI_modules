//! A language's sound system and the words it allows.

use crate::phoneme::{CATALOG, Kind, Manner, Phoneme, PhonemeId, Place};
use crate::rng::Rng;
use crate::settings::{Settings, Stress, style_weight};
use crate::word::{Orthography, Segment, Word};

/// The sound system: inventory, syllable shape and stress.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Phonology {
    /// Consonants and vowels, in catalogue order.
    pub consonants: Vec<PhonemeId>,
    pub vowels: Vec<PhonemeId>,
    /// Whether vowel length is phonemic (long vowels are written doubled).
    pub long_vowels: bool,
    pub max_onset: u8,
    pub max_coda: u8,
    /// Chance (%) that a syllable has no coda.
    pub open_syllables: u8,
    pub stress: Stress,
}

impl Phonology {
    /// Generates a sound system from `seed` and (clamped) `settings`.
    pub fn generate(seed: u64, settings: &Settings) -> Self {
        let s = settings.clamped();
        let mut rng = Rng::derive(seed, "phonology", 0);
        let weight = |id: &PhonemeId| id.get().map_or(0, |p| u32::from(p.frequency) * style_weight(s.style, p));

        let vowel_pool: Vec<PhonemeId> = PhonemeId::all().filter(|id| id.get().is_some_and(|p| p.is_vowel())).collect();
        let n_vowels = usize::from(rng.range(s.vowels.0, s.vowels.1));
        let core_vowels: Vec<PhonemeId> = ["a", "i", "u"].iter().filter_map(|v| PhonemeId::from_ipa(v)).collect();
        let vowels = pick(&mut rng, &vowel_pool, &core_vowels, n_vowels, &weight);

        let consonant_pool: Vec<PhonemeId> = PhonemeId::all().filter(|id| id.get().is_some_and(|p| !p.is_vowel())).collect();
        let n_consonants = usize::from(rng.range(s.consonants.0, s.consonants.1));
        // Every language gets a voiceless stop and a nasal, so even a tiny inventory is speakable.
        let class = |f: fn(&Kind) -> bool| -> Vec<PhonemeId> { consonant_pool.iter().copied().filter(|id| id.get().is_some_and(|p| f(&p.kind))).collect() };
        let stops = class(|k| matches!(k, Kind::Consonant { manner: Manner::Stop, voiced: false, place } if *place != Place::Glottal));
        let nasals = class(|k| matches!(k, Kind::Consonant { manner: Manner::Nasal, place: Place::Bilabial | Place::Alveolar, .. }));
        let mut core = Vec::new();
        for group in [&stops, &nasals] {
            let weights: Vec<u32> = group.iter().map(&weight).collect();
            if let Some(id) = rng.weighted(&weights).and_then(|i| group.get(i)) {
                core.push(*id);
            }
        }
        let consonants = pick(&mut rng, &consonant_pool, &core, n_consonants, &weight);

        let long_vowels = rng.chance(s.vowel_length);
        let stress = s.stress.unwrap_or_else(|| {
            const STRESS: [Stress; 4] = [Stress::Initial, Stress::Second, Stress::Penultimate, Stress::Final];
            rng.weighted(&[35, 5, 40, 20]).and_then(|i| STRESS.get(i).copied()).unwrap_or(Stress::Penultimate)
        });
        Phonology { consonants, vowels, long_vowels, max_onset: s.max_onset, max_coda: s.max_coda, open_syllables: s.open_syllables, stress }
    }

    /// Every phoneme in the language, consonants first.
    pub fn inventory(&self) -> impl Iterator<Item = PhonemeId> + '_ {
        self.consonants.iter().chain(self.vowels.iter()).copied()
    }

    pub fn orthography(&self) -> Orthography {
        Orthography::new(self.inventory(), self.long_vowels)
    }

    /// A random word of `syllables` syllables that obeys the syllable rules. Its spelling is not
    /// yet checked for ambiguity (see [`Phonology::reads_back`]).
    pub(crate) fn random_word(&self, rng: &mut Rng, syllables: u8) -> Word {
        let mut segments = Vec::new();
        for s in 0..syllables.max(1) {
            // Onset: optional only word-initially, so vowels never meet across a boundary.
            if !(s == 0 && rng.chance(20)) {
                let cluster = if self.max_onset >= 2 && rng.chance(15) { self.onset_cluster(rng) } else { None };
                match cluster {
                    Some((a, b)) => segments.extend([Segment::short(a), Segment::short(b)]),
                    None => segments.extend(self.consonant(rng, |p| s > 0 || p.at_word_edge()).map(Segment::short)),
                }
            }
            if let Some(v) = self.vowel(rng) {
                let long = self.long_vowels && v.get().is_some_and(|p| p.can_be_long()) && rng.chance(20);
                segments.push(Segment { phoneme: v, long });
            }
            if self.max_coda > 0 && !rng.chance(self.open_syllables) {
                let cluster = if self.max_coda >= 2 && rng.chance(15) { self.coda_cluster(rng) } else { None };
                match cluster {
                    Some((a, b)) => segments.extend([Segment::short(a), Segment::short(b)]),
                    None => {
                        let last = s + 1 == syllables.max(1);
                        let coda = self.consonant(rng, |p| {
                            (!last || p.at_word_edge())
                                && !matches!(
                                    p.kind,
                                    Kind::Consonant { manner: Manner::Approximant, .. }
                                        | Kind::Consonant { place: Place::Glottal, manner: Manner::Fricative, .. }
                                )
                        });
                        segments.extend(coda.map(Segment::short));
                    }
                }
            }
        }
        Word { segments }
    }

    /// Whether the spelling of `word` reads back to exactly the same sounds (no digraph or
    /// long-vowel ambiguity).
    pub(crate) fn reads_back(&self, orthography: &Orthography, word: &Word) -> bool {
        orthography.read(&word.spelling()).as_ref() == Some(word)
    }

    fn vowel(&self, rng: &mut Rng) -> Option<PhonemeId> {
        let weights: Vec<u32> = self.vowels.iter().map(|id| id.get().map_or(0, |p| u32::from(p.frequency))).collect();
        rng.weighted(&weights).and_then(|i| self.vowels.get(i)).copied()
    }

    fn consonant(&self, rng: &mut Rng, allow: impl Fn(&Phoneme) -> bool) -> Option<PhonemeId> {
        let weights: Vec<u32> = self.consonants.iter().map(|id| id.get().filter(|p| allow(p)).map_or(0, |p| u32::from(p.frequency))).collect();
        rng.weighted(&weights).and_then(|i| self.consonants.get(i)).copied()
    }

    fn onset_cluster(&self, rng: &mut Rng) -> Option<(PhonemeId, PhonemeId)> {
        let a =
            self.consonant(rng, |p| matches!(p.kind, Kind::Consonant { manner: Manner::Stop | Manner::Fricative, place, .. } if place != Place::Glottal))?;
        let b = self.consonant(rng, |p| matches!(p.kind, Kind::Consonant { manner: Manner::Lateral | Manner::Trill | Manner::Approximant, .. }))?;
        Some((a, b))
    }

    fn coda_cluster(&self, rng: &mut Rng) -> Option<(PhonemeId, PhonemeId)> {
        let a = self.consonant(rng, |p| matches!(p.kind, Kind::Consonant { manner: Manner::Nasal | Manner::Lateral | Manner::Trill, .. }))?;
        let b =
            self.consonant(rng, |p| matches!(p.kind, Kind::Consonant { manner: Manner::Stop | Manner::Fricative, place, .. } if place != Place::Glottal))?;
        Some((a, b))
    }
}

/// `n` distinct items from `pool`: all of `core` first, then weighted draws without replacement.
/// The result is in catalogue order.
fn pick(rng: &mut Rng, pool: &[PhonemeId], core: &[PhonemeId], n: usize, weight: &impl Fn(&PhonemeId) -> u32) -> Vec<PhonemeId> {
    let mut chosen: Vec<PhonemeId> = Vec::with_capacity(n.min(CATALOG.len()));
    for id in core {
        if chosen.len() < n && !chosen.contains(id) {
            chosen.push(*id);
        }
    }
    let mut rest: Vec<PhonemeId> = pool.iter().copied().filter(|id| !chosen.contains(id)).collect();
    while chosen.len() < n {
        let weights: Vec<u32> = rest.iter().map(weight).collect();
        let Some(i) = rng.weighted(&weights) else { break };
        if i < rest.len() {
            chosen.push(rest.swap_remove(i));
        }
    }
    chosen.sort();
    chosen
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::Style;

    #[test]
    fn same_seed_same_sounds() {
        let s = Settings::default();
        assert_eq!(Phonology::generate(42, &s), Phonology::generate(42, &s));
        assert_ne!(Phonology::generate(42, &s), Phonology::generate(43, &s));
    }

    #[test]
    fn inventories_respect_settings_and_core() {
        for style in [Style::Balanced, Style::Harsh, Style::Melodic, Style::Sibilant] {
            let s = Settings::preset(style);
            for seed in 0..200 {
                let p = Phonology::generate(seed, &s);
                assert!((usize::from(s.vowels.0)..=usize::from(s.vowels.1)).contains(&p.vowels.len()), "{style:?} {seed}");
                assert!((usize::from(s.consonants.0)..=usize::from(s.consonants.1)).contains(&p.consonants.len()));
                for v in ["a", "i", "u"] {
                    assert!(p.vowels.iter().any(|id| id.ipa() == v));
                }
                assert!(p.consonants.iter().any(|id| matches!(id.ipa(), "m" | "n")));
                assert!(p.vowels.iter().all(|id| id.get().is_some_and(|x| x.is_vowel())));
                assert!(p.consonants.iter().all(|id| id.get().is_some_and(|x| !x.is_vowel())));
            }
        }
    }

    #[test]
    fn styles_sound_different() {
        // Over many seeds, harsh languages have more uvulars and velar fricatives than melodic ones.
        let count = |style, ipa: &[&str]| -> usize {
            (0..300).map(|seed| Phonology::generate(seed, &Settings::preset(style)).consonants.iter().filter(|id| ipa.contains(&id.ipa())).count()).sum()
        };
        let guttural = ["q", "χ", "x", "ɣ"];
        assert!(count(Style::Harsh, &guttural) > 3 * count(Style::Melodic, &guttural));
        let sibilant = ["s", "z", "ʃ", "ʒ", "ts", "dz", "tʃ", "dʒ", "ɬ"];
        assert!(count(Style::Sibilant, &sibilant) > count(Style::Melodic, &sibilant));
    }

    #[test]
    fn random_words_follow_syllable_rules() {
        for seed in 0..50 {
            let p = Phonology::generate(seed, &Settings::preset(Style::Harsh));
            let mut rng = Rng::new(seed);
            for n in 1..=4 {
                let w = p.random_word(&mut rng, n);
                assert_eq!(w.syllable_count(), usize::from(n));
                assert!(w.segments.iter().all(|s| p.inventory().any(|id| id == s.phoneme)));
                assert!(w.segments.iter().all(|s| !s.long || p.long_vowels));
            }
        }
    }
}
