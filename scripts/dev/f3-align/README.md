# F3 mérés: angol felirat-időzítés (wav2vec2)

> **Állapot:** mérőeszköz, nem része a terméknek · **Utolsó átnézés:** 2026-11-02 ·
> **Döntés:** [ADR 0008](../../../docs/adr/ADR-0008-ai-processing-layer.md) F3,
> [modellválasztás](../../../docs/ai/modellvalasztas-091.md) 3.3 és 4. pont

Az F3 képessége a `facebook/wav2vec2-base-960h` modellel végzett CTC-alapú kényszerített
illesztés. Ez a mappa a mérését végzi el, mielőtt a Rust-worker elkészül (az F0 kapu, vagyis a
Qt 6 átállás vége előtt AI-crate nem indul).

| Fájl | Tartalom |
|---|---|
| `ctc_align.py` | Szövegnormalizálás és Viterbi-alapú illesztés, csak standard könyvtárral; a Rust-változat referenciája. Teszt: `scripts/ci/test_f3_ctc_align.py`. |
| `measure.py` | `export`, `parity`, `speed`, `accuracy` alparancs, JSON-jelentéssel. |
| `check_labels.py` | A kézi címkefájl ellenőrzése a mérés előtt (modell nélkül, csak standard könyvtár). |
| `from_audacity.py` | Audacity-címkeexportból (sorok és szavak) címkefájl-bejegyzés. |
| `labels.example.json` | A címkefájl mintája. |
| `JELOLES.md` | Jelölési útmutató: milyen hang kell, hogyan írd a szöveget, hogyan jelölj Audacityben. |
| `pyproject.toml`, `uv.lock` | Rögzített környezet: egy CUDA 12.x (torch 2.5.1, onnxruntime-gpu 1.20.2). |

## Futtatás a minimumgépen

A modellt a felhasználó tölti le, a CFA4 nem terjeszti:

```sh
git clone https://huggingface.co/facebook/wav2vec2-base-960h ~/models/wav2vec2-base-960h
git -C ~/models/wav2vec2-base-960h checkout 22aad52d435eb6dbaf354bdad9b0da84ce7d6156
```

Minden mérés a `run-minspec.sh` alatt fut, mindkét profillal (`CFA4_PROFILE=ajanlott`: 32 GB RAM, alap; `CFA4_PROFILE=minimum`: 16 GB RAM, ott csak az számít, hogy összeomlás nélkül lefut), 8 maggal, egyetlen referencia-GPU-val (a végleges a 3060):

```sh
M=~/models/wav2vec2-base-960h
R="scripts/dev/run-minspec.sh uv run --project scripts/dev/f3-align python scripts/dev/f3-align/measure.py"
$R export   --model $M --out target/f3-align/onnx
$R parity   --model $M --onnx-dir target/f3-align/onnx --audio klip.wav --text klip.txt
$R speed    --model $M --onnx-dir target/f3-align/onnx --audio egy-ora.wav
$R speed    --model $M --onnx-dir target/f3-align/onnx --audio egy-ora.wav --device cpu
$R accuracy --model $M --onnx-dir target/f3-align/onnx --labels cimkek/labels.json
```

A hang bármilyen mintavételű lehet: a szkript monóra keveri és 16 kHz-re mintavételezi át.

### Amíg nincs RTX 3060: mérés az RTX A4000-en

A referenciagép (CinemaFundArchitecture4 `docs/development/reference-machine.md`) számítási
kártyája most RTX A4000 (16 GB); az RTX A1000 csak a kijelzőt hajtja. A `run-minspec.sh` a 3060-at
keresi, ezért addig a kártyát kézzel kell megadni:

```sh
nvidia-smi --query-gpu=index,name --format=csv   # az A4000 indexe
export CFA4_REF_GPU=<az A4000 indexe>
```

Mi érvényes így, és mi nem:

| Mérés | A4000-en | Indok |
|---|---|---|
| `export`, `parity` | végleges | nem függ a kártyától |
| `accuracy` (fp32 és fp16) | végleges | az A4000 és a 3060 ugyanaz az Ampere-generáció (8.6-os számítási képesség), így az fp16-os számítás is ugyanaz |
| `speed`: csúcs-VRAM | ideiglenes, a manifestbe beírható | ugyanazzal a futtatóval és architektúrával a foglalás közel azonos; a 3060-on újramérjük |
| `speed`: RTF | csak tájékoztató | az A4000 gyorsabb a 3060-nál, ezért a ≤ 0,1-es küszöb teljesítése itt nem bizonyítja a 3060-at |

A jegyzőkönyvbe kerüljön az `lscpu` kimenete, a rögzített magok, a kártya neve és a terhelés.

## Mit mér, és hová kerül

- `export`: a safetensors SHA-256-ja és a két ONNX-fájl (fp32 CPU-n, fp16 CUDA-n) hash-e és mérete.
  Ez kerül a manifest `[[weights]]` részébe (CFA4-sdk `examples/manifests/align-wav2vec2.toml`).
- Minden alparancs, amely a modellmappát használja, elutasítja a más commitra állt vagy helyben
  módosított (követett fájljában eltérő) checkoutot.
- `parity`: legfeljebb 30 s-os klipen és a hozzá tartozó átiraton fut (hosszabb hangot elutasít).
  Jelenti a modell rögzített változatát, a safetensors SHA-256-ját, a kockaszámot mindkét
  oldalon, a logitok legnagyobb eltérését, az egyező argmax arányát, és azt, hogy az illesztés
  ugyanaz-e, mint a PyTorch-csal.
  A `parity_ok` csak akkor igaz, ha a kockaszám és az illesztés egyezik, és a logit-eltérés
  meg az argmax-egyezés a `PARITY_LIMITS` küszöbén belül van (fp32: legfeljebb 0,001 és
  legalább 99,9%; fp16: legfeljebb 0,25 és legalább 98%). A küszöbök ideiglenesek: az első
  3060-as mérés után pontosítjuk őket.
- A hangfájl legfeljebb 2 óra és legfeljebb 2 óra × 48 000 bemeneti kocka lehet, 64 csatornáig;
  a hosszt dekódolás előtt ellenőrizzük.
- `speed`: betöltési idő, valós idejű tényező (RTF) és csúcs-VRAM 30 s-os ablakokkal, 1 s
  környezettel. Ez a manifest `resources` része (`vram_mb` kulcsa `any@fp32`, `any@fp16`).
- `accuracy`: a kézi `words` elemek `text` mezője a `normalize()` szerinti egy-egy szó, az átirat
  sorrendjében; eltérésnél a sor szóhatárait nem pontozzuk, hanem hibaként jelentjük.
  Minden mért pontosságon (CUDA-n fp32 és fp16, CPU-n fp32) a szók kezdete és vége 50 ms-on, a feliratsorok kezdete és vége 100 ms-on belül,
  a sorok átfedése (IoU) a kézi jelöléssel, és az egymást átfedő sorok száma (0 kell).

A címkézett klipeknek CFA4 tulajdonú, saját felvételeknek kell lenniük; LDC-s vagy más
korlátozott licencű korpusz (például TIMIT) nem használható. A jelölés menetét, a címkefájl
formáját és az ellenőrzést a [JELOLES.md](JELOLES.md) írja le; a kézi szavakat a
`ctc_align.normalize()` szerint kell felírni (a számokat kiírva).
