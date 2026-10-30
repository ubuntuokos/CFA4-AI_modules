//! Prints a generated language: `cargo run -p cfa4-conlang --example sample -- [style] [seed] [name]`
//! (style: balanced, harsh, melodic, sibilant).

use cfa4_conlang::{Clause, Language, NounPhrase, Settings, Style, Tense};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let style = match args.first().map(String::as_str) {
        Some("harsh") => Style::Harsh,
        Some("melodic") => Style::Melodic,
        Some("sibilant") => Style::Sibilant,
        _ => Style::Balanced,
    };
    let seed = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(1);
    let name = args.get(2).cloned().unwrap_or_else(|| "Sample".to_owned());
    let lang = match Language::generate(&name, seed, &Settings::preset(style)) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };
    let ipa = |ids: &[cfa4_conlang::phoneme::PhonemeId]| ids.iter().map(|i| i.ipa()).collect::<Vec<_>>().join(" ");
    let p = &lang.phonology;
    println!("{} ({}), {style:?}, seed {seed}", lang.name, lang.tag);
    println!("consonants: {}", ipa(&p.consonants));
    println!("vowels:     {}{}", ipa(&p.vowels), if p.long_vowels { " (+ long)" } else { "" });
    println!("stress: {:?}, sound changes: {:?}", p.stress, lang.sound_changes);
    let g = &lang.grammar;
    println!("grammar: {:?}, adjective {:?} noun, adposition {:?} noun", g.word_order, g.adjective, g.adposition);
    for (category, marking) in g.markings() {
        let word = lang.word(category).map(|e| e.word.spelling()).unwrap_or_default();
        println!("  {category:<4} {marking:?} {word}");
    }
    let clause = Clause {
        subject: NounPhrase { noun: "woman", plural: true, adjective: Some("old") },
        verb: "see",
        object: Some(NounPhrase::new("bird")),
        tense: Tense::Past,
        negative: true,
        question: true,
    };
    if let Ok(p) = lang.clause(&clause) {
        println!("\"did the old women not see the bird?\" = {} [{}]", p.spelling(), lang.phrase_ipa(&p));
    }
    for e in lang.entries().iter().filter(|e| e.class != cfa4_conlang::WordClass::Grammatical).take(40) {
        println!("{:<14} {:<14} [{}]", e.gloss, e.word.spelling(), lang.ipa(&e.word));
    }
}
