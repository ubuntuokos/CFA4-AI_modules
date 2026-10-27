//! eSpeak NG as an external oracle: the generated rules must make eSpeak NG say exactly the IPA
//! this crate computes, with the stress on the same syllable.
//!
//! Needs the `espeak-ng` program (Debian/Ubuntu package `espeak-ng`); without it the test only
//! prints a note. The system data directory is copied into the target directory, so nothing
//! outside it is written.

use std::path::{Path, PathBuf};
use std::process::Command;

use cfa4_conlang::{Language, PHONEME_TABLE, Settings, Stress, Style};

fn espeak_data_dir() -> Option<PathBuf> {
    let out = Command::new("espeak-ng").arg("--version").output().ok()?;
    let text = String::from_utf8_lossy(&out.stdout).into_owned();
    let dir = text.split("Data at:").nth(1)?.trim();
    Some(PathBuf::from(dir))
}

fn copy_dir(from: &Path, to: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir(&entry.path(), &target)?;
        } else {
            std::fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

/// IPA without stress marks, in eSpeak NG's spelling of `g`.
fn bare(ipa: &str) -> String {
    ipa.chars().filter(|c| !matches!(c, 'ˈ' | 'ˌ')).map(|c| if c == 'g' { 'ɡ' } else { c }).collect()
}

const VOWELS: &[char] = &['a', 'e', 'i', 'o', 'u', 'ɛ', 'ɔ', 'ɪ', 'ʊ', 'y', 'ø', 'œ', 'ɯ', 'æ'];

/// Index of the vowel that follows eSpeak NG's primary stress mark (it marks the nucleus).
fn stressed_vowel(ipa: &str) -> Option<usize> {
    let mut seen = 0;
    let mut marked = false;
    for c in ipa.chars() {
        if c == 'ˈ' {
            marked = true;
        } else if VOWELS.contains(&c) {
            if marked {
                return Some(seen);
            }
            seen += 1;
        }
    }
    None
}

#[test]
fn espeak_says_what_we_compute() {
    let Some(system) = espeak_data_dir() else {
        eprintln!("espeak-ng not installed: oracle test skipped");
        return;
    };
    let data = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("conlang-espeak-data");
    let _ = std::fs::remove_dir_all(&data);
    copy_dir(&system, &data).unwrap();
    assert!(data.join("phontab").exists(), "no phoneme tables in {}", data.display());
    let src = data.join("conlang-src");
    std::fs::create_dir_all(data.join("lang/art")).unwrap();

    let mut checked = 0;
    let mut wrong = Vec::new();
    let mut stresses = std::collections::HashSet::new();
    for (i, style) in [Style::Balanced, Style::Harsh, Style::Melodic, Style::Sibilant].into_iter().enumerate() {
        for seed in 0..6u64 {
            let mut settings = Settings::preset(style);
            // Cover every stress rule explicitly, plus the seeded choice.
            settings.stress =
                [None, Some(Stress::Initial), Some(Stress::Second), Some(Stress::Penultimate), Some(Stress::Final), None].get(seed as usize).copied().flatten();
            settings.vowel_length = if seed % 2 == 0 { 100 } else { 0 };
            let name = format!("Oracle{i}{seed}");
            let lang = Language::generate(&name, seed * 31 + i as u64, &settings).unwrap();
            stresses.insert(lang.phonology.stress);
            let files = lang.espeak();
            assert!(files.voice.contains(&format!("phonemes {PHONEME_TABLE}")));
            let _ = std::fs::remove_dir_all(&src);
            std::fs::create_dir_all(&src).unwrap();
            std::fs::write(src.join(format!("{}_rules", files.dictionary)), &files.rules).unwrap();
            std::fs::write(src.join(format!("{}_list", files.dictionary)), &files.list).unwrap();
            std::fs::write(data.join("lang/art").join(&files.dictionary), &files.voice).unwrap();
            let out =
                Command::new("espeak-ng").current_dir(&src).env("ESPEAK_DATA_PATH", &data).arg(format!("--compile={}", files.dictionary)).output().unwrap();
            let log = String::from_utf8_lossy(&out.stderr).into_owned() + &String::from_utf8_lossy(&out.stdout);
            assert!(out.status.success() && !log.contains("Bad phoneme") && !log.contains("rror"), "{name}: {log}\n{}", files.rules);

            let words: Vec<_> = lang.entries().iter().map(|e| e.word.clone()).collect();
            let input: String = words.iter().map(|w| format!("{}.\n", w.spelling())).collect();
            let txt = src.join("words.txt");
            std::fs::write(&txt, input).unwrap();
            let out = Command::new("espeak-ng").env("ESPEAK_DATA_PATH", &data).args(["-q", "--ipa", "-v", &lang.tag, "-f"]).arg(&txt).output().unwrap();
            assert!(out.status.success(), "{name}: {}", String::from_utf8_lossy(&out.stderr));
            let spoken: Vec<String> = String::from_utf8_lossy(&out.stdout).lines().map(|l| l.trim().to_owned()).filter(|l| !l.is_empty()).collect();
            assert_eq!(spoken.len(), words.len(), "{name}: one line per word");
            for (w, said) in words.iter().zip(&spoken) {
                let ours = lang.ipa(w);
                let stress_ok = w.syllable_count() < 2 || stressed_vowel(said) == Some(lang.phonology.stress.index(w.syllable_count()));
                if bare(said) != bare(&ours) || !stress_ok {
                    wrong.push(format!("{name} {:?} \"{}\": eSpeak NG {said}, ours {ours}", lang.phonology.stress, w.spelling()));
                }
                checked += 1;
            }
        }
    }
    assert!(wrong.is_empty(), "{} of {checked} words differ:\n{}", wrong.len(), wrong.join("\n"));
    assert_eq!(stresses.len(), 4, "every stress rule exercised");
    eprintln!("eSpeak NG oracle: {checked} words agree");
}
