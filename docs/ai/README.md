# CFA4 AI – besorolási táblák

> **Állapot:** javaslat, egyetlen projekt–típus pár sincs még tesztelve · **Utolsó átnézés:**
> 2026-10-26 · **Változás:** minor (hangfunkciók táblája, M13–M15) · **Döntés:**
> [ADR 0006](../adr/ADR-0006-ai-processing-layer.md)

| Fájl | Tartalom |
|---|---|
| [`200-tipus-besorolas.csv`](200-tipus-besorolas.csv) | A 200 funkciótípus: csoport, motorcsalád (M1–M12), besorolás (képesség, változat, sablon, kísérleti), futtató, 12 GB-os futtathatóság, fázis. A „Jelölt repók” oszlop a `projektjeloltek.csv` azonosítóira hivatkozik (típusonként legfeljebb 5 hely). A „P-kódok” az eredeti 200-as lista projektkódjai, ezek feloldása még hiányzik |
| [`projektjeloltek.csv`](projektjeloltek.csv) | A 107 jelölt repó: motorcsalád és javaslat (elsődleges, értékelendő Rust, tartalék, referencia, kihagyás) |
| [`hang-besorolas.csv`](hang-besorolas.csv) | 59 hang-, zene- és beszédfunkció: motorcsalád, besorolás, ami a CFA4-ben már megvan, elsődleges és tartalék jelölt, futtató, kód- és súlylicenc külön, kereskedelmi használat, 12 GB-os futtathatóság, fázis |

A licenc- és VRAM-adatok becslések, és egyetlen projekt sincs még kipróbálva. Mielőtt egy projekt
bekerül, ellenőrizni kell a repójában a licencét, és le kell mérni a
[`scripts/dev/run-minspec.sh`](../../scripts/dev/run-minspec.sh) alatt. A hang-, zene- és
zajfunkciók a `hang-besorolas.csv` táblában vannak.

## Hangfunkciók (2026-10-26)

A hanglista miatt három új motorcsalád kell; az M10 és az M12 a 200-as listából marad:

| Család | Tartalom | Fő futtató |
|---|---|---|
| M10 Hang- és zeneanalízis | átirat, diarizáció, VAD, címkézés, MIDI, akkord, keresés | candle, ONNX |
| M12 Hagyományos algoritmus | hangosság, de-esser, tempó, hangnem, térhang, mastering | Rust DSP (`audio-dsp`) |
| M13 Beszédszintézis és hangátalakítás | TTS, hangklónozás, beszéd- és énekhang-konverzió | candle, PyTorch |
| M14 Zene- és hanggenerálás | zene, dal, effekt, Foley | PyTorch |
| M15 Hangleválasztás és -helyreállítás | stemek, beszéd–zaj szétválasztás, zajszűrés, sávszélesség | Rust (tract), PyTorch |

A „Fázis” oszlop az [ADR 0006](../adr/ADR-0006-ai-processing-layer.md) hónapjait jelenti, a
Qt 6 átállás lezárásától számolva. A „kész” azt jelenti, hogy a funkció AI nélkül, a meglévő
crate-ekkel már működik. A „K…” a [Kdenlive-terv](../kdenlive-parity.md) teendőjére utal.

**Ami már megvan, ezért nem kell rá modell:**

- `cfa4-speech`: Whisper candle-on, szóidőkkel és nyelvfelismeréssel, valamint klasszikus diarizáció.
- `cfa4-tts`: Kokoro-82M candle-on.
- `audio-dsp`: R128 hangosság, denoise, dehum, declick, dereverb, de-esser, mastering,
  binauralizer.
- `remix.rs`: BPM, ütem és kroma.

Hat funkció (hangosság, de-esser, térhang, tempó, nyelvfelismerés, zenei restaurálás) így kész,
és több másiknak is ezek adják a nem-AI alapját.

**Döntést igénylő megállapítások:**

- **Többnyelvű beszédhang (István, 2026-10-26: minél több nyelv).** A szűk keresztmetszet a
  fonemizálás, mert a CFA4 saját fonemizálója csak angolul és spanyolul tud. Javasolt rétegek:
  1. Kokoro: 9 nyelv, candle-on, folyamaton belül.
  2. Piper-hangok: kb. 40 nyelv, köztük a magyar (`hu_HU`), ONNX-en. A licencük hangonként eltér.
  3. Chatterbox Multilingual: 23 nyelv és hangklónozás, MIT, PyTorch-workerben. Magyarul nem tud.
  4. eSpeak NG fonemizáló: kb. 100 nyelv, GPL-3.0, C nyelvű, ezért külön folyamatban fut. Ez
     látja el fonémával a Kokorót és a Pipert, és robotos végső tartalékhangként minden
     nyelvre ad hangot.

  A nyelv alapján a Registry választ, ehhez a manifestnek nyelvmező kell. Kizárt: MMS-TTS
  (1100+ nyelv, de CC-BY-NC) és XTTS v2 (CPML).
- **Kizárt modellek.** Nem kereskedelmi a súlyuk, ezért a fizetős oldalon nem használhatók:
  MusicGen, AudioGen, MERT, F5-TTS, Fish Speech, XTTS v2, Audio Flamingo, MMS, SeamlessM4T.
- **Tiltott függőség.** Az Essentia AGPL-3.0, és a CFA4 licencpolitikája ezt tiltja.
- **Kerülendő könyvtárak.** A `whisper-rs` és a `sherpa-onnx` C/C++ kötés, a `sherpa-onnx`
  ráadásul saját ONNX Runtime-ot hoz. A CFA4 csak a modelljeiket használja, a közös ONNX
  workerben.
- **Rust-jelöltek.** A DeepFilterNet és a Basic Pitch tract-tal tiszta Rustban fut. A
  `cargo deny` és a pánikmentesség ellenőrzése után folyamaton belül is futhatnak.
- **Szabályzat kell.** A hangklónozáshoz, a beszéd- és énekhang-konverzióhoz és a beszéd
  átírásához hozzájárulás és vízjel kell, mint az arccseréhez (38. típus). A beszélőazonosítás
  biometrikus adatot kezel, ezért GDPR-szabály is kell hozzá.
- **Python 3.14.** Az AI-workerek nem a rendszer Pythonját használják, hanem a `uv` által
  rögzített interpretert a közös PyTorch-környezetben (ADR 0006, 3. pont). Ezért például az
  AudioCraft régi Python-igénye nem akadály, de a kizárt modellek így sem jönnek szóba.
- **Modellkatalógus.** A worker-manifest ([CFA4-sdk](https://github.com/ubuntuokos/CFA4-sdk))
  már tárolja a kód és a súly licencét külön, a letöltési címet, a méretet, a VRAM-ot és a
  RAM-ot. A támogatott nyelvek mezője (`capability.languages`, BCP 47) a CFA4-sdk-ba kerül.
