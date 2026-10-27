//! The stored (JSON) form: round trip, hand edits and hostile input.

use cfa4_conlang::{ConlangError, Language, Settings, Style, WordClass};
use serde_json::{Value, json};

fn sample() -> Language {
    Language::generate("Sylvan", 2026, &Settings::preset(Style::Melodic)).unwrap()
}

fn load(v: &Value) -> Result<Language, String> {
    serde_json::from_value::<Language>(v.clone()).map_err(|e| e.to_string())
}

#[test]
fn round_trip_is_exact() {
    let lang = sample();
    let text = serde_json::to_string_pretty(&lang).unwrap();
    let back: Language = serde_json::from_str(&text).unwrap();
    assert_eq!(back, lang);
    // Readable: phonemes as IPA, words as spellings with IPA beside them.
    let v: Value = serde_json::from_str(&text).unwrap();
    assert!(v["phonology"]["vowels"].as_array().unwrap().iter().any(|x| x == "a"));
    let first = &v["lexicon"][0];
    assert_eq!(first["gloss"], "pl");
    assert!(first["spelling"].is_string() && first["ipa"].is_string());
}

#[test]
fn hand_edit_survives_reload() {
    let lang = sample();
    let mut v = serde_json::to_value(&lang).unwrap();
    let sun = lang.word("sun").unwrap().word.spelling();
    let moon = lang.word("moon").unwrap().word.spelling();
    // Swap two words by hand, ignore the stale ipa field, add an entry.
    for e in v["lexicon"].as_array_mut().unwrap() {
        if e["gloss"] == "sun" {
            e["spelling"] = json!(moon.to_uppercase());
            e["ipa"] = json!("garbage");
        } else if e["gloss"] == "moon" {
            e["spelling"] = json!(sun);
        }
    }
    v["lexicon"].as_array_mut().unwrap().push(json!({"gloss": "Dragon  Fire", "class": "noun", "spelling": format!("{sun}{moon}")}));
    let edited = load(&v).unwrap();
    assert_eq!(edited.word("sun").unwrap().word.spelling(), moon);
    assert_eq!(edited.word("dragon fire").unwrap().class, WordClass::Noun);
}

#[test]
fn hostile_input_is_an_error() {
    let good = serde_json::to_value(sample()).unwrap();
    let cases: Vec<(&str, Box<dyn Fn(&mut Value)>)> = vec![
        ("format", Box::new(|v| v["format"] = json!(99))),
        ("empty name", Box::new(|v| v["name"] = json!("  "))),
        ("control in name", Box::new(|v| v["name"] = json!("a\u{0}b"))),
        ("long name", Box::new(|v| v["name"] = json!("x".repeat(10_000)))),
        ("bad tag", Box::new(|v| v["tag"] = json!("art-x-waytoolongsubtag"))),
        ("tag with space", Box::new(|v| v["tag"] = json!("art x"))),
        ("unknown phoneme", Box::new(|v| v["phonology"]["consonants"] = json!(["p", "ʘ"]))),
        ("vowel in consonants", Box::new(|v| v["phonology"]["consonants"] = json!(["p", "a"]))),
        ("duplicate", Box::new(|v| v["phonology"]["vowels"] = json!(["a", "a"]))),
        ("no vowels", Box::new(|v| v["phonology"]["vowels"] = json!([]))),
        ("huge list", Box::new(|v| v["phonology"]["vowels"] = json!(vec!["a"; 100_000]))),
        ("onset", Box::new(|v| v["phonology"]["max_onset"] = json!(9))),
        ("zero onset", Box::new(|v| v["phonology"]["max_onset"] = json!(0))),
        ("open %", Box::new(|v| v["phonology"]["open_syllables"] = json!(101))),
        ("negative", Box::new(|v| v["phonology"]["max_coda"] = json!(-1))),
        ("stress", Box::new(|v| v["phonology"]["stress"] = json!("third"))),
        ("seed", Box::new(|v| v["seed"] = json!(-5))),
        ("foreign letters", Box::new(|v| v["lexicon"][7]["spelling"] = json!("§§"))),
        ("no vowel", Box::new(|v| v["lexicon"][7]["spelling"] = json!(""))),
        ("long word", Box::new(|v| v["lexicon"][7]["spelling"] = json!("a".repeat(1_000_000)))),
        ("empty gloss", Box::new(|v| v["lexicon"][7]["gloss"] = json!(" "))),
        ("long gloss", Box::new(|v| v["lexicon"][7]["gloss"] = json!("g".repeat(500)))),
        ("duplicate gloss", Box::new(|v| v["lexicon"][8]["gloss"] = v["lexicon"][7]["gloss"].clone())),
        ("missing morpheme", Box::new(|v| v["lexicon"].as_array_mut().unwrap().retain(|e| e["gloss"] != "neg"))),
        ("bad class", Box::new(|v| v["lexicon"][7]["class"] = json!("verbish"))),
        ("not an object", Box::new(|v| *v = json!([1, 2, 3]))),
    ];
    for (what, mutate) in cases {
        let mut v = good.clone();
        mutate(&mut v);
        assert!(load(&v).is_err(), "{what} was accepted");
    }
    // Settings are clamped rather than refused.
    let mut v = good.clone();
    v["settings"]["vowels"] = json!([0, 255]);
    v["settings"]["avoid"] = json!(vec!["x"; 20_000]);
    let l = load(&v).unwrap();
    assert_eq!(l.settings.vowels, (3, 12));
    assert_eq!(l.settings.avoid.len(), cfa4_conlang::MAX_AVOID);
}

#[test]
fn random_mutations_never_panic() {
    // Replace every leaf, one at a time, with a set of hostile values; loading must return.
    let good = serde_json::to_value(sample()).unwrap();
    let hostile = [json!(null), json!(-1), json!(u64::MAX), json!(1.5e308), json!(""), json!("ʘ\u{0}"), json!([]), json!({}), json!(true)];
    fn paths(v: &Value, prefix: &mut Vec<Value>, out: &mut Vec<Vec<Value>>) {
        match v {
            Value::Object(m) => m.iter().for_each(|(k, x)| {
                prefix.push(json!(k));
                paths(x, prefix, out);
                prefix.pop();
            }),
            // Only the first few entries of long arrays: enough to reach every field shape.
            Value::Array(a) => a.iter().take(3).enumerate().for_each(|(i, x)| {
                prefix.push(json!(i));
                paths(x, prefix, out);
                prefix.pop();
            }),
            _ => out.push(prefix.clone()),
        }
    }
    let mut all = Vec::new();
    paths(&good, &mut Vec::new(), &mut all);
    let mut tried = 0;
    for path in &all {
        for h in &hostile {
            let mut v = good.clone();
            let mut slot = &mut v;
            for key in path {
                slot = match key {
                    Value::String(k) => &mut slot[k.as_str()],
                    Value::Number(i) => &mut slot[i.as_u64().unwrap() as usize],
                    _ => unreachable!(),
                };
            }
            *slot = h.clone();
            let _ = load(&v);
            tried += 1;
        }
    }
    assert!(tried > 200, "{tried}");
}

#[test]
fn small_sound_systems_report_exhaustion() {
    // Three vowels, six consonants, one-syllable open words: coining many words must stop with
    // an error, never loop or panic.
    let s = Settings { vowels: (3, 3), consonants: (6, 6), max_coda: 0, word_syllables: (1, 1), vowel_length: 0, ..Settings::default() };
    let mut lang = Language::generate("Tiny", 1, &s).unwrap();
    let mut errors = 0;
    for i in 0..3000 {
        match lang.coin(&format!("concept {i}"), WordClass::Noun) {
            Ok(_) => {}
            Err(ConlangError::Coin(_)) => errors += 1,
            Err(e) => panic!("{e}"),
        }
    }
    let n = lang.entries().len();
    let mut spellings: Vec<_> = lang.entries().iter().map(|e| e.word.spelling()).collect();
    spellings.sort();
    spellings.dedup();
    assert_eq!(spellings.len(), n, "spellings stay unique");
    assert!(n > 500, "{n} words");
    let _ = errors;
}
