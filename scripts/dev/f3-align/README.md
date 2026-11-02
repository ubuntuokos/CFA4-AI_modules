# F3 mérés: angol felirat-időzítés (wav2vec2)

> **Állapot:** mérőeszköz, nem része a terméknek · **Utolsó átnézés:** 2026-11-02 ·
> **Döntés:** [ADR 0006](../../../docs/adr/ADR-0006-ai-processing-layer.md) F3,
> [modellválasztás](../../../docs/ai/modellvalasztas-091.md) 3.3 és 4. pont

Az F3 képessége a `facebook/wav2vec2-base-960h` modellel végzett CTC-alapú kényszerített
illesztés. Ez a mappa a mérését végzi el, mielőtt a Rust-worker elkészül (az F0 kapu, vagyis a
Qt 6 átállás vége előtt AI-crate nem indul).

| Fájl | Tartalom |
|---|---|
| `ctc_align.py` | Szövegnormalizálás és Viterbi-alapú illesztés, csak standard könyvtárral; a Rust-változat referenciája. Teszt: `scripts/ci/test_f3_ctc_align.py`. |
| `measure.py` | `export`, `parity`, `speed`, `accuracy` alparancs, JSON-jelentéssel. |
| `pyproject.toml`, `uv.lock` | Rögzített környezet: egy CUDA 12.x (torch 2.5.1, onnxruntime-gpu 1.20.2). |

## Futtatás a minimumgépen

A modellt a felhasználó tölti le, a CFA4 nem terjeszti:

```sh
git clone https://huggingface.co/facebook/wav2vec2-base-960h ~/models/wav2vec2-base-960h
git -C ~/models/wav2vec2-base-960h checkout 22aad52d435eb6dbaf354bdad9b0da84ce7d6156
```

Minden mérés a `run-minspec.sh` alatt fut (32 GB RAM, 8 mag, csak a 3060):

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

## Mit mér, és hová kerül

- `export`: a safetensors SHA-256-ja és a két ONNX-fájl (fp32 CPU-n, fp16 CUDA-n) hash-e és mérete.
  Ez kerül a manifest `[[weights]]` részébe (CFA4-sdk `examples/manifests/align-wav2vec2.toml`).
- Minden alparancs, amely a modellmappát használja, elutasítja a más commitra állt vagy helyben
  módosított (követett fájljában eltérő) checkoutot.
- `parity`: legfeljebb 30 s-os klipen és a hozzá tartozó átiraton fut (hosszabb hangot elutasít).
  Jelenti a modell rögzített változatát, a safetensors SHA-256-ját, a kockaszámot mindkét
  oldalon, a logitok legnagyobb eltérését, az egyező argmax arányát, és azt, hogy az illesztés
  ugyanaz-e, mint a PyTorch-csal.
  Eltérő kockaszám esetén a `parity_ok` hamis.
- A hangfájl legfeljebb 2 óra és legfeljebb 2 óra × 48 000 bemeneti kocka lehet, 64 csatornáig;
  a hosszt dekódolás előtt ellenőrizzük.
- `speed`: betöltési idő, valós idejű tényező (RTF) és csúcs-VRAM 30 s-os ablakokkal, 1 s
  környezettel. Ez a manifest `resources` része (`vram_mb` kulcsa `any@fp32`, `any@fp16`).
- `accuracy`: a kézi `words` elemek `text` mezője a `normalize()` szerinti egy-egy szó, az átirat
  sorrendjében; eltérésnél a sor szóhatárait nem pontozzuk, hanem hibaként jelentjük.
  Minden mért pontosságon (CUDA-n fp32 és fp16, CPU-n fp32) a szók kezdete és vége 50 ms-on, a feliratsorok kezdete és vége 100 ms-on belül,
  a sorok átfedése (IoU) a kézi jelöléssel, és az egymást átfedő sorok száma (0 kell).

A címkézett klipeknek CFA4 tulajdonú, saját felvételeknek kell lenniük; LDC-s vagy más
korlátozott licencű korpusz (például TIMIT) nem használható. A címkefájl formáját a
`measure.py accuracy` leírása adja meg; a kézi szavakat a `ctc_align.normalize()` szerint kell
felírni (a számokat kiírva).
