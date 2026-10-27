//! Basic grammar choices, drawn with real-world frequencies as weights.

use serde::{Deserialize, Serialize};

use crate::rng::Rng;

/// Order of subject, verb and object in a plain statement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum WordOrder {
    Sov,
    Svo,
    Vso,
    Vos,
    Ovs,
    Osv,
}

impl WordOrder {
    pub const ALL: [WordOrder; 6] = [WordOrder::Sov, WordOrder::Svo, WordOrder::Vso, WordOrder::Vos, WordOrder::Ovs, WordOrder::Osv];

    /// Whether the object follows the verb.
    pub fn verb_before_object(self) -> bool {
        matches!(self, WordOrder::Svo | WordOrder::Vso | WordOrder::Vos)
    }

    /// Puts subject, verb and object in this order.
    pub fn arrange<T>(self, subject: T, verb: T, object: T) -> [T; 3] {
        match self {
            WordOrder::Sov => [subject, object, verb],
            WordOrder::Svo => [subject, verb, object],
            WordOrder::Vso => [verb, subject, object],
            WordOrder::Vos => [verb, object, subject],
            WordOrder::Ovs => [object, verb, subject],
            WordOrder::Osv => [object, subject, verb],
        }
    }
}

/// Where a modifier goes relative to its head.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Position {
    Before,
    After,
}

/// How a grammatical category is expressed. The morpheme itself is the lexicon entry with the
/// category's gloss ([`Grammar::MORPHEMES`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Marking {
    /// Not marked at all (context only).
    Unmarked,
    Prefix,
    Suffix,
    /// A separate word before the head (the noun, the verb, or the clause for questions).
    ParticleBefore,
    /// A separate word after the head.
    ParticleAfter,
}

/// The grammar sketch of a language.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Grammar {
    pub word_order: WordOrder,
    /// Adjective relative to its noun.
    pub adjective: Position,
    /// Adposition relative to its noun: `Before` is a preposition, `After` a postposition.
    pub adposition: Position,
    pub plural: Marking,
    pub past: Marking,
    pub future: Marking,
    pub negation: Marking,
    /// Yes/no questions; `Unmarked` means intonation only.
    pub question: Marking,
}

impl Grammar {
    /// Glosses of the grammatical morphemes, in the order of the fields above.
    pub const MORPHEMES: [&'static str; 5] = ["PL", "PST", "FUT", "NEG", "Q"];

    pub fn generate(seed: u64) -> Self {
        let mut rng = Rng::derive(seed, "grammar", 0);
        // Rough shares of the world's languages with a dominant order.
        let word_order = pick(&mut rng, &WordOrder::ALL, &[41, 35, 7, 2, 1, 1]).unwrap_or(WordOrder::Sov);
        let vo = word_order.verb_before_object();
        // Verb–object order goes with prepositions, object–verb with postpositions, about 9 times
        // in 10.
        let adposition = if rng.chance(90) == vo { Position::Before } else { Position::After };
        let adjective = pick(&mut rng, &[Position::Before, Position::After], &[30, 70]).unwrap_or(Position::After);
        use Marking::*;
        let affixes = [Unmarked, Prefix, Suffix, ParticleBefore, ParticleAfter];
        let plural = pick(&mut rng, &affixes, &[10, 15, 55, 10, 10]).unwrap_or(Suffix);
        let past = pick(&mut rng, &affixes, &[5, 15, 55, 15, 10]).unwrap_or(Suffix);
        let future = pick(&mut rng, &affixes, &[10, 10, 40, 30, 10]).unwrap_or(ParticleBefore);
        let negation = pick(&mut rng, &affixes, &[0, 20, 15, 50, 15]).unwrap_or(ParticleBefore);
        let question = pick(&mut rng, &affixes, &[30, 0, 0, 30, 40]).unwrap_or(ParticleAfter);
        Grammar { word_order, adjective, adposition, plural, past, future, negation, question }
    }

    /// Category name and marking for each of [`Grammar::MORPHEMES`].
    pub fn markings(&self) -> [(&'static str, Marking); 5] {
        let [pl, pst, fut, neg, q] = Self::MORPHEMES;
        [(pl, self.plural), (pst, self.past), (fut, self.future), (neg, self.negation), (q, self.question)]
    }
}

fn pick<T: Copy>(rng: &mut Rng, items: &[T], weights: &[u32]) -> Option<T> {
    rng.weighted(weights).and_then(|i| items.get(i)).copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_and_plausible() {
        assert_eq!(Grammar::generate(5), Grammar::generate(5));
        let grammars: Vec<Grammar> = (0..2000).map(Grammar::generate).collect();
        let share = |f: &dyn Fn(&Grammar) -> bool| grammars.iter().filter(|g| f(g)).count() * 100 / grammars.len();
        // SOV and SVO dominate, rare orders still appear.
        assert!(share(&|g| matches!(g.word_order, WordOrder::Sov | WordOrder::Svo)) > 65);
        assert!(grammars.iter().any(|g| g.word_order == WordOrder::Osv));
        // Adposition order follows verb–object order most of the time.
        assert!(share(&|g| g.word_order.verb_before_object() == (g.adposition == Position::Before)) > 80);
        assert!(grammars.iter().all(|g| g.negation != Marking::Unmarked && g.question != Marking::Prefix));
    }

    #[test]
    fn arrange_orders() {
        assert_eq!(WordOrder::Sov.arrange("S", "V", "O"), ["S", "O", "V"]);
        assert_eq!(WordOrder::Vos.arrange("S", "V", "O"), ["V", "O", "S"]);
    }
}
