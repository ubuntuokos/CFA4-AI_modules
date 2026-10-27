# cfa4-conlang

Constructed-language generator for CFA4 (L2, function `AU-060` in
[`docs/ai/hang-besorolas.csv`](../../docs/ai/hang-besorolas.csv)). It invents a
new language for a film: sounds, spelling, pronunciation, vocabulary and a
grammar sketch. The core needs no AI, no GPU and no UI. The generated language
belongs to the user.

```rust
use cfa4_conlang::{Language, Settings, Style, WordClass};

let mut lang = Language::generate("Kh'zar", 2026, &Settings::preset(Style::Harsh))?;
let fire = lang.word("fire").map(|e| e.word.clone());   // from the basic vocabulary
let dragon = lang.coin("dragon", WordClass::Noun)?;     // new word, never a duplicate
println!("{} [{}]", dragon.word.spelling(), lang.ipa(&dragon.word));
let files = lang.espeak();                              // eSpeak NG voice + rules
let json = serde_json::to_string_pretty(&lang)?;        // readable, hand-editable
```

## What it generates

- **Sound system** (`Phonology`): consonants and vowels drawn from a
  47-phoneme catalogue, weighted by how common each sound is in the world's
  languages and by the chosen `Style` (balanced, harsh, melodic, sibilant).
  Every language has a, i, u, a voiceless stop and m or n. Syllables allow
  onsets of up to two consonants (obstruent + liquid or glide) and codas of up
  to two (sonorant + obstruent). Optional phonemic vowel length. Stress on the
  first, second, penultimate or last syllable.
- **Spelling and IPA**: each phoneme has one Latin spelling (`sh`, `kh`, `ny`,
  `'` for the glottal stop, `ü`, `ö`, `è`…); long vowels are doubled. Words
  whose spelling could be read two ways (`s`+`h` next to `sh`) are never
  generated, so every spelling reads back to exactly one pronunciation.
- **Vocabulary**: 152 basic concepts plus the grammatical morphemes, and any
  new word on demand with `Language::coin`. Each word is seeded by the language
  seed and its gloss, so the same request gives the same word. Up to three
  sound changes (intervocalic voicing, final devoicing, palatalization) give
  the vocabulary a history. A filter drops words that match a short built-in
  list of offensive real words (English, Hungarian and a few others) or any
  word the user lists in `Settings::avoid`.
- **Grammar sketch** (`Grammar`): word order (SOV, SVO, … with real-world
  frequencies), adjective and adposition position (adpositions follow the
  verb–object order 9 times in 10), and how plural, past, future, negation and
  yes/no questions are marked (prefix, suffix, particle or nothing).
- **eSpeak NG files** (`Language::espeak`): a voice file and
  `<dictionary>_rules`, so eSpeak NG (a separate GPL-3.0 program; CFA4 only
  writes its input) speaks the language at once. They use eSpeak NG's `az`
  phoneme table, the best fit of the 130 tables in eSpeak NG 1.51 (tied with
  `tk`): it says every catalogue phoneme as written in every position tested,
  except three handled here. The trill uses the `R2` phoneme, `ɯ` has no long
  form, and the glottal stop is kept off word edges, where eSpeak NG drops it.

## Stored form

`Language` serializes to JSON with phonemes as IPA and words as spellings
(plus an `ipa` field for readers, ignored on load), so a person can edit it.
Loading validates everything: unknown or misplaced phonemes, duplicates, out of
range syllable limits, words with letters the language lacks or without a
vowel, duplicate glosses, missing grammatical morphemes, over-long strings and
more than 100 000 entries are errors; generation settings are clamped. Nothing
panics on hostile input.

## Limits and next steps

- Style comes from presets; choosing it from a free-text description ("rough,
  guttural warrior people") is for the later LLM step.
- No inflected forms yet: the grammar names the morphemes and where they go,
  but does not build words with them.
- Translation of script lines (local LLM using this vocabulary and grammar), a
  generated writing system (a font in the user's font directory) and the Qt6
  editor come later.
- Rights-held languages (Klingon, Na'vi, Dothraki…) are never shipped: the
  user imports data they licensed with `Language::set_word` or a JSON file.

## Tests

`cargo test -p cfa4-conlang`. `tests/espeak_oracle.rs` copies the system
eSpeak NG data into the target directory, compiles the rules of 24 generated
languages (every style and stress rule, with and without long vowels) and
checks that eSpeak NG says all about 3800 words with exactly the IPA and the
stressed syllable this crate computes. Without `espeak-ng` installed
(`apt install espeak-ng`) it prints a note and passes.
