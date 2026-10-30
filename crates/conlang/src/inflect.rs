//! Inflected forms and simple clauses built from the grammar sketch.
//!
//! Affixes are the grammatical morphemes of the vocabulary (`PL`, `PST`, `FUT`, `NEG`, `Q`).
//! Where a stem and an affix meet, a linking sound keeps the word speakable and its spelling
//! unambiguous: a linking consonant between two vowels, a linking vowel between consonants when
//! the cluster would grow past two or the spelling could be read two ways. The linking sounds are
//! derived from the seed and the inventory, so they never need storing.

use crate::grammar::{Marking, Position};
use crate::language::{ConlangError, Language};
use crate::phoneme::PhonemeId;
use crate::rng::Rng;
use crate::word::{MAX_SEGMENTS, Segment, Word};

/// Verb tense.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tense {
    #[default]
    Present,
    Past,
    Future,
}

/// A noun phrase: a noun (or pronoun) gloss, number and an optional adjective gloss.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NounPhrase<'a> {
    pub noun: &'a str,
    pub plural: bool,
    pub adjective: Option<&'a str>,
}

impl<'a> NounPhrase<'a> {
    pub fn new(noun: &'a str) -> Self {
        Self { noun, plural: false, adjective: None }
    }
}

/// A plain clause: subject, verb, optional object, tense, negation and yes/no question.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Clause<'a> {
    pub subject: NounPhrase<'a>,
    pub verb: &'a str,
    pub object: Option<NounPhrase<'a>>,
    pub tense: Tense,
    pub negative: bool,
    pub question: bool,
}

/// Words in speaking order.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Phrase {
    pub words: Vec<Word>,
}

impl Phrase {
    /// The words' spellings separated by spaces.
    pub fn spelling(&self) -> String {
        self.words.iter().map(Word::spelling).collect::<Vec<_>>().join(" ")
    }
}

/// Grammatical categories, by the gloss of their morpheme.
const PL: &str = "PL";
const PST: &str = "PST";
const FUT: &str = "FUT";
const NEG: &str = "NEG";
const Q: &str = "Q";

impl Language {
    /// IPA of every word of `phrase`, separated by spaces.
    pub fn phrase_ipa(&self, phrase: &Phrase) -> String {
        phrase.words.iter().map(|w| self.ipa(w)).collect::<Vec<_>>().join(" ")
    }

    /// The noun phrase in this language's order, plural marked as the grammar says.
    pub fn noun_phrase(&self, np: &NounPhrase<'_>) -> Result<Phrase, ConlangError> {
        let noun = self.lookup(np.noun)?;
        let mut words = vec![noun];
        if np.plural {
            self.mark(&mut words, 0, PL, self.grammar.plural)?;
        }
        if let Some(adj) = np.adjective {
            let adj = self.lookup(adj)?;
            match self.grammar.adjective {
                Position::Before => words.insert(0, adj),
                Position::After => words.push(adj),
            }
        }
        Ok(Phrase { words })
    }

    /// The verb with tense and negation: tense closest to the stem, negation outside it.
    pub fn verb_form(&self, verb: &str, tense: Tense, negative: bool) -> Result<Phrase, ConlangError> {
        let mut words = vec![self.lookup(verb)?];
        let mut head = 0;
        match tense {
            Tense::Present => {}
            Tense::Past => head = self.mark(&mut words, head, PST, self.grammar.past)?,
            Tense::Future => head = self.mark(&mut words, head, FUT, self.grammar.future)?,
        }
        if negative {
            self.mark(&mut words, head, NEG, self.grammar.negation)?;
        }
        Ok(Phrase { words })
    }

    /// A whole clause in this language's word order.
    pub fn clause(&self, c: &Clause<'_>) -> Result<Phrase, ConlangError> {
        let subject = self.noun_phrase(&c.subject)?.words;
        let verb = self.verb_form(c.verb, c.tense, c.negative)?.words;
        let object = match &c.object {
            Some(o) => self.noun_phrase(o)?.words,
            None => Vec::new(),
        };
        let mut words: Vec<Word> = self.grammar.word_order.arrange(subject, verb, object).into_iter().flatten().collect();
        if c.question {
            // Intonation only needs no morpheme; an affix joins the first or last word of the
            // clause, a particle stands at its edge.
            match self.grammar.question {
                Marking::Unmarked => {}
                Marking::ParticleBefore => words.insert(0, self.lookup(Q)?),
                Marking::ParticleAfter => words.push(self.lookup(Q)?),
                Marking::Prefix => {
                    let q = self.lookup(Q)?;
                    if let Some(first) = words.first_mut() {
                        *first = self.attach(first, &q, true)?;
                    }
                }
                Marking::Suffix => {
                    let q = self.lookup(Q)?;
                    if let Some(last) = words.last_mut() {
                        *last = self.attach(last, &q, false)?;
                    }
                }
            }
        }
        Ok(Phrase { words })
    }

    fn lookup(&self, gloss: &str) -> Result<Word, ConlangError> {
        self.word(gloss).map(|e| e.word.clone()).ok_or_else(|| ConlangError::UnknownGloss(gloss.chars().take(crate::MAX_GLOSS_CHARS).collect()))
    }

    /// Marks the word group `words` (head at `head`) with the morpheme `gloss` as `marking`
    /// says: an affix joins the head, a particle goes at the start or end of the group, so later
    /// markings sit outside earlier ones. Returns the head's new index.
    fn mark(&self, words: &mut Vec<Word>, head: usize, gloss: &str, marking: Marking) -> Result<usize, ConlangError> {
        if marking == Marking::Unmarked {
            return Ok(head);
        }
        let morpheme = self.lookup(gloss)?;
        match marking {
            Marking::Unmarked => Ok(head),
            Marking::Prefix | Marking::Suffix => {
                if let Some(slot) = words.get_mut(head) {
                    *slot = self.attach(slot, &morpheme, marking == Marking::Prefix)?;
                }
                Ok(head)
            }
            Marking::ParticleBefore => {
                words.insert(0, morpheme);
                Ok(head.saturating_add(1))
            }
            Marking::ParticleAfter => {
                words.push(morpheme);
                Ok(head)
            }
        }
    }

    /// Stem and affix joined into one word, with a linking sound where needed so the result is
    /// speakable and its spelling reads back unambiguously. A form longer than [`MAX_SEGMENTS`]
    /// is an error, never a truncated word.
    pub fn attach(&self, stem: &Word, affix: &Word, prefix: bool) -> Result<Word, ConlangError> {
        let (left, right) = if prefix { (affix, stem) } else { (stem, affix) };
        let (link_vowel, link_consonant) = self.linking();
        let plain = concat(left, &[], right)?;
        let (Some(l), Some(r)) = (left.segments.last(), right.segments.first()) else {
            return Ok(plain);
        };
        let orthography = self.phonology.orthography();
        let candidate = match (l.is_vowel(), r.is_vowel()) {
            (true, true) => concat(left, &link_consonant, right)?,
            (false, false) => {
                let run = trailing_consonants(left) + leading_consonants(right);
                if run > 2 || !self.phonology.reads_back(&orthography, &plain) { concat(left, &link_vowel, right)? } else { plain }
            }
            _ => plain,
        };
        if self.phonology.reads_back(&orthography, &candidate) {
            return Ok(candidate);
        }
        // A vowel next to a consonant never forms a digraph, so a linking vowel between two
        // consonants and a linking consonant between two vowels always read back; anything still
        // ambiguous here was ambiguous in the stem itself, so take the safest join.
        let link = if l.is_vowel() { &link_consonant } else { &link_vowel };
        let joined = concat(left, link, right)?;
        if self.phonology.reads_back(&orthography, &joined) {
            Ok(joined)
        } else {
            Err(ConlangError::Ambiguous(joined.spelling().chars().take(crate::MAX_GLOSS_CHARS).collect()))
        }
    }

    /// The linking vowel and consonant, chosen from the inventory by the seed.
    fn linking(&self) -> (Vec<Segment>, Vec<Segment>) {
        let mut rng = Rng::derive(self.seed, "linking", 0);
        let pick = |rng: &mut Rng, pool: &[PhonemeId], prefer: &[&str]| -> Option<PhonemeId> {
            let preferred: Vec<PhonemeId> = pool.iter().copied().filter(|id| prefer.contains(&id.ipa())).collect();
            let from = if preferred.is_empty() { pool.to_vec() } else { preferred };
            let weights: Vec<u32> = from.iter().map(|id| id.get().map_or(0, |p| u32::from(p.frequency))).collect();
            rng.weighted(&weights).and_then(|i| from.get(i)).copied()
        };
        let vowel = pick(&mut rng, &self.phonology.vowels, &["e", "a", "i", "ɛ", "ɪ"]);
        // The linker always sits inside the word, so any consonant (the glottal stop too) will do.
        let consonant = pick(&mut rng, &self.phonology.consonants, &["n", "j", "t", "h", "r", "l", "w", "s"]);
        (vowel.map(Segment::short).into_iter().collect(), consonant.map(Segment::short).into_iter().collect())
    }
}

fn concat(left: &Word, link: &[Segment], right: &Word) -> Result<Word, ConlangError> {
    let len = left.segments.len().saturating_add(link.len()).saturating_add(right.segments.len());
    if len > MAX_SEGMENTS {
        return Err(ConlangError::FormTooLong(format!("{}+{}", left.spelling(), right.spelling()).chars().take(crate::MAX_GLOSS_CHARS).collect()));
    }
    let segments = left.segments.iter().chain(link).chain(&right.segments).copied().collect();
    Ok(Word { segments })
}

fn trailing_consonants(w: &Word) -> usize {
    w.segments.iter().rev().take_while(|s| !s.is_vowel()).count()
}

fn leading_consonants(w: &Word) -> usize {
    w.segments.iter().take_while(|s| !s.is_vowel()).count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Grammar, Settings, Style, WordOrder};

    fn lang(seed: u64) -> Language {
        Language::generate("Test", seed, &Settings::default()).unwrap_or_else(|e| panic!("{e}"))
    }

    #[test]
    fn every_join_reads_back() {
        for style in [Style::Balanced, Style::Harsh, Style::Melodic, Style::Sibilant] {
            for seed in 0..30 {
                let l = Language::generate("Test", seed, &Settings::preset(style)).unwrap_or_else(|e| panic!("{e}"));
                let o = l.phonology.orthography();
                let morphemes: Vec<Word> = Grammar::MORPHEMES.iter().filter_map(|g| l.word(g)).map(|e| e.word.clone()).collect();
                for e in l.entries() {
                    for m in &morphemes {
                        for prefix in [false, true] {
                            let w = l.attach(&e.word, m, prefix).unwrap_or_else(|e| panic!("{e}"));
                            assert_eq!(o.read(&w.spelling()).as_ref(), Some(&w), "{style:?} {seed} {} + {}", e.word.spelling(), m.spelling());
                            // Never more than two consonants in a row across the boundary.
                            let edge = if prefix { m.segments.len() } else { e.word.segments.len() };
                            let window = w.segments.get(edge.saturating_sub(2)..(edge + 3).min(w.segments.len())).unwrap_or(&[]);
                            assert!(window.windows(3).all(|t| t.iter().any(Segment::is_vowel)), "{}", w.spelling());
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn markings_follow_the_grammar() {
        for seed in 0..40 {
            let mut l = lang(seed);
            let stem = l.word("sun").map(|e| e.word.clone()).unwrap_or_default();
            let pl = l.word("PL").map(|e| e.word.clone()).unwrap_or_default();
            let np = l.noun_phrase(&NounPhrase { noun: "sun", plural: true, adjective: None }).unwrap_or_default();
            match l.grammar.plural {
                Marking::Unmarked => assert_eq!(np.words, std::slice::from_ref(&stem)),
                Marking::Suffix => assert_eq!(np.words, [l.attach(&stem, &pl, false).unwrap_or_default()]),
                Marking::Prefix => assert_eq!(np.words, [l.attach(&stem, &pl, true).unwrap_or_default()]),
                Marking::ParticleBefore => assert_eq!(np.words, [pl.clone(), stem.clone()]),
                Marking::ParticleAfter => assert_eq!(np.words, [stem.clone(), pl.clone()]),
            }
            // Force particles everywhere and check the order of tense and negation.
            l.grammar.past = Marking::ParticleBefore;
            l.grammar.negation = Marking::ParticleBefore;
            let v = l.verb_form("see", Tense::Past, true).unwrap_or_default();
            let [neg, pst, see] = ["NEG", "PST", "see"].map(|s| l.word(s).map(|e| e.word.clone()).unwrap_or_default());
            assert_eq!(v.words, [neg.clone(), pst.clone(), see.clone()]);
            l.grammar.past = Marking::Suffix;
            l.grammar.negation = Marking::ParticleAfter;
            let v = l.verb_form("see", Tense::Past, true).unwrap_or_default();
            assert_eq!(v.words, [l.attach(&see, &pst, false).unwrap_or_default(), neg.clone()]);
            l.grammar.past = Marking::ParticleAfter;
            let v = l.verb_form("see", Tense::Past, true).unwrap_or_default();
            assert_eq!(v.words, [see, pst, neg]);
        }
    }

    #[test]
    fn clauses_follow_word_order_and_question_marking() {
        let mut l = lang(4);
        let g = |l: &Language, s: &str| l.word(s).map(|e| e.word.clone()).unwrap_or_default();
        l.grammar.adjective = Position::After;
        l.grammar.question = Marking::ParticleAfter;
        for order in WordOrder::ALL {
            l.grammar.word_order = order;
            let c = Clause {
                subject: NounPhrase::new("woman"),
                verb: "see",
                object: Some(NounPhrase { noun: "bird", plural: false, adjective: Some("red") }),
                tense: Tense::Present,
                negative: false,
                question: true,
            };
            let p = l.clause(&c).unwrap_or_default();
            let s = vec![g(&l, "woman")];
            let v = vec![g(&l, "see")];
            let o = vec![g(&l, "bird"), g(&l, "red")];
            let mut expected: Vec<Word> = order.arrange(s, v, o).into_iter().flatten().collect();
            expected.push(g(&l, "Q"));
            assert_eq!(p.words, expected, "{order:?}");
        }
        assert_eq!(p_err(&l), Err(ConlangError::UnknownGloss("dragon".to_owned())));
    }

    fn p_err(l: &Language) -> Result<Phrase, ConlangError> {
        l.clause(&Clause { subject: NounPhrase::new("dragon"), verb: "see", object: None, tense: Tense::Past, negative: false, question: false })
    }

    #[test]
    fn question_affixes_and_intonation() {
        let mut l = lang(6);
        let g = |l: &Language, s: &str| l.word(s).map(|e| e.word.clone()).unwrap_or_default();
        let c = Clause { subject: NounPhrase::new("woman"), verb: "sleep", object: None, tense: Tense::Present, negative: false, question: true };
        l.grammar.word_order = WordOrder::Svo;
        l.grammar.question = Marking::Suffix;
        let p = l.clause(&c).unwrap_or_default();
        assert_eq!(p.words, [g(&l, "woman"), l.attach(&g(&l, "sleep"), &g(&l, "Q"), false).unwrap_or_default()]);
        l.grammar.question = Marking::Prefix;
        let p = l.clause(&c).unwrap_or_default();
        assert_eq!(p.words, [l.attach(&g(&l, "woman"), &g(&l, "Q"), true).unwrap_or_default(), g(&l, "sleep")]);
        // Intonation-only questions work without a Q entry in the vocabulary.
        l.grammar.question = Marking::Unmarked;
        let mut v = serde_json::to_value(&l).unwrap_or_default();
        if let Some(a) = v["lexicon"].as_array_mut() {
            a.retain(|e| e["gloss"] != "q");
        }
        let loaded: Language = serde_json::from_value(v).unwrap_or_else(|e| panic!("{e}"));
        assert!(loaded.word("Q").is_none());
        assert_eq!(loaded.clause(&c).map(|p| p.words.len()), Ok(2));
    }

    #[test]
    fn overlong_forms_are_errors_not_truncated() {
        let mut l = lang(8);
        let a = l.phonology.vowels.first().map(|v| v.get().map_or("a", |p| p.roman)).unwrap_or("a");
        let longest = a.repeat(MAX_SEGMENTS);
        l.set_word("long", crate::WordClass::Noun, &longest).unwrap_or_else(|e| panic!("{e}"));
        l.grammar.plural = Marking::Suffix;
        let np = l.noun_phrase(&NounPhrase { noun: "long", plural: true, adjective: None });
        assert!(matches!(np, Err(ConlangError::FormTooLong(_))), "{np:?}");
        l.grammar.plural = Marking::Prefix;
        assert!(matches!(l.noun_phrase(&NounPhrase { noun: "long", plural: true, adjective: None }), Err(ConlangError::FormTooLong(_))));
    }

    #[test]
    fn glottal_stop_can_link_vowels() -> Result<(), ConlangError> {
        // A loaded language whose only consonant is the glottal stop, with long vowels: "a" + "a"
        // must not become "aa" (one long vowel) but "a'a".
        let l = lang(1);
        let mut v = serde_json::to_value(&l).map_err(|e| ConlangError::InvalidPhonology(e.to_string()))?;
        v["phonology"]["consonants"] = serde_json::json!(["ʔ"]);
        v["phonology"]["vowels"] = serde_json::json!(["a"]);
        v["phonology"]["long_vowels"] = serde_json::json!(true);
        v["lexicon"] = serde_json::json!([]);
        v["grammar"]["plural"] = serde_json::json!("unmarked");
        for k in ["past", "future", "negation", "question"] {
            v["grammar"][k] = serde_json::json!("unmarked");
        }
        let tiny: Language = serde_json::from_value(v).map_err(|e| ConlangError::InvalidPhonology(e.to_string()))?;
        let a = tiny.phonology.orthography().read("a").unwrap_or_default();
        let joined = tiny.attach(&a, &a, false)?;
        assert_eq!(joined.spelling(), "a'a");
        assert_eq!(tiny.phonology.orthography().read(&joined.spelling()), Some(joined));
        Ok(())
    }
}
