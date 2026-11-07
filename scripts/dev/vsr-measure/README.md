# #91 mérés: videó-felskálázás (Real-ESRGAN)

> **Állapot:** mérőeszköz, nem része a terméknek · **Utolsó átnézés:** 2026-11-02 ·
> **Döntés:** [modellválasztás](../../../docs/ai/modellvalasztas-091.md) 1.1, 2. és 4.1 pont

A #91 (videó-felskálázás) elsődleges jelöltje a Real-ESRGAN. Kódja és súlya BSD-3-Clause, de a
tanítóadata nem tiszta (DIV2K: csak kutatásra, Flickr2K és OST: nincs megadott licenc). István
döntése szerint (2026-11-07) ez a felhasználó felelőssége, mert a CFA4 nem terjeszt modellt: a
modell egyszeri licencelfogadás után az ingyenes, GPL-es workerben is futhat (modellválasztás
1.1 pont). Ez a mappa azt méri, elfér-e és milyen gyors a minimumgépen 2K-s kimenettel. Ugyanez a
szkript méri majd a saját, tiszta adaton tanított súlyt is, ha az ugyanilyen RRDBNet-architektúrájú.

| Fájl | Tartalom |
|---|---|
| `measure.py` | `export`, `parity`, `speed` alparancs, JSON-jelentéssel. |
| `rrdbnet.py` | Az RRDBNet-generátor PyTorch-ban; a Real-ESRGAN súlyai szigorúan betölthetők bele. |
| `tiles.py` | A képkocka csempékre bontása (csak standard könyvtár); a Rust-worker referenciája. Teszt: `scripts/ci/test_vsr_tiles.py`. |
| `pyproject.toml`, `uv.lock` | Rögzített környezet, ugyanaz a CUDA 12.x-es csomagsor, mint az F3-é (torch 2.5.1, onnxruntime-gpu 1.20.2). |

## A súlyok

A súlyt a felhasználó tölti le, a CFA4 nem terjeszti. A szkript csak ezt a két fájlt fogadja el,
a SHA-256 alapján:

| Fájl | Forrás | SHA-256 |
|---|---|---|
| `RealESRGAN_x2plus.pth` | [Real-ESRGAN v0.2.1](https://github.com/xinntao/Real-ESRGAN/releases/download/v0.2.1/RealESRGAN_x2plus.pth) | `49fafd45f8fd7aa8d31ab2a22d14d91b536c34494a5cfe31eb5d89c2fa266abb` |
| `RealESRGAN_x4plus.pth` | [Real-ESRGAN v0.1.0](https://github.com/xinntao/Real-ESRGAN/releases/download/v0.1.0/RealESRGAN_x4plus.pth) | `4fa0d38905f75ac06eb49a7951b426670021be3018265fd191d2125df9d682f1` |

```sh
mkdir -p ~/models/realesrgan && cd ~/models/realesrgan
curl -LO https://github.com/xinntao/Real-ESRGAN/releases/download/v0.2.1/RealESRGAN_x2plus.pth
curl -LO https://github.com/xinntao/Real-ESRGAN/releases/download/v0.1.0/RealESRGAN_x4plus.pth
sha256sum RealESRGAN_x*.pth
```

## A bemeneti klip

A mérés 2048×1080-as (DCI 2K) kimenetet céloz, 10 s-os klipen. A szkript nyers rgb24 képkockákat
olvas; ezeket az ffmpeg állítja elő (csak tesztkészletet készít, a termék nem használja). A kimenet
a `target/fixtures/` alá kerül, a repóba nem:

```sh
mkdir -p target/fixtures
# kettes nagyításhoz 1024×540-es, négyeshez 512×270-es bemenet kell a 2048×1080-as kimenethez
ffmpeg -i klip.mov -t 10 -vf scale=1024:540:flags=area -pix_fmt rgb24 -f rawvideo \
    target/fixtures/vsr-1024x540.rgb
ffmpeg -i klip.mov -t 10 -vf scale=512:270:flags=area -pix_fmt rgb24 -f rawvideo \
    target/fixtures/vsr-512x270.rgb
```

Valódi filmanyag legyen (arcok, szöveg, zaj, mozgás), ne szintetikus kép.

## Futtatás a minimumgépen

Minden mérés a `run-minspec.sh` alatt fut (32 GB RAM, 8 mag, egyetlen referencia-GPU; a végleges a
3060). Az RTX A4000-en ugyanaz érvényes, mint az F3-nál (`scripts/dev/f3-align/README.md`): a
`CFA4_REF_GPU=<az A4000 indexe>` kell, a parity végleges, a VRAM ideiglenes, a sebesség csak
tájékoztató.

```sh
W=~/models/realesrgan/RealESRGAN_x2plus.pth
I="--input target/fixtures/vsr-1024x540.rgb --width 1024 --height 540"
R="scripts/dev/run-minspec.sh uv run --project scripts/dev/vsr-measure python scripts/dev/vsr-measure/measure.py"
$R export --weights $W --out target/vsr-measure/onnx
$R parity --weights $W --onnx-dir target/vsr-measure/onnx $I
$R speed  --weights $W --onnx-dir target/vsr-measure/onnx $I
$R speed  --weights $W --onnx-dir target/vsr-measure/onnx $I --device cpu --frames 5
```

A négyes nagyításhoz ugyanez a `RealESRGAN_x4plus.pth`-val és az 512×270-es bemenettel. Az
`export` mindkét súlyt ugyanabba a mappába teheti; a `parity` és a `speed` a fájlnévből választja
ki a megadott súlyhoz tartozó ONNX-fájlt.

## Mit mér, és hová kerül

- `export`: a súly SHA-256-ja és a két ONNX-fájl (fp32 CPU-n, fp16 CUDA-n) hash-e és mérete. A worker
  manifestjének `[[weights]]` részébe kerül. A licencrészek ott: kód és súly BSD-3-Clause,
  tanítóadat DIV2K `no`, Flickr2K és OST `NOASSERTION`.
- `parity`: legfeljebb 8 képkockán ugyanazzal a csempézéssel fut a PyTorch (tiszta fp32, TF32
  nélkül) és az ONNX Runtime. A `parity_ok` akkor igaz, ha a legrosszabb képkocka PSNR-je is
  legalább 45 dB.
- `speed`: csempeméretenként (alapból 256 és 512, 10 képpontos környezettel) a betöltés és a
  bemelegítés ideje, a képkocka/s és a csúcs-VRAM. Ez a manifest `resources` része: `vram_mb`
  (`any@fp32`, `any@fp16`) és `fps_hint`. CPU-n nagyon lassú, ezért ott a `--frames 5` elég.
- A bemenet legfeljebb 2000 képkocka, a kimenet legfeljebb 8192 képpont oldalanként; a fájlméretnek
  pontosan a megadott képméret egész többszörösének kell lennie.
