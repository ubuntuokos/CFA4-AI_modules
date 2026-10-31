# #91 Videó-felskálázás (Video Super-Resolution): modellválasztás az F3 kapuhoz

> **Állapot:** javaslat; két tulajdonosi döntés 2026-10-31-én (1. és 3. pont), a 3. pont javaslata döntésre vár · **Utolsó átnézés:** 2026-10-31 ·
> **Változás:** major (első változat) · **Döntés:** [ADR 0006](../adr/ADR-0006-ai-processing-layer.md)
> F3 kapu, manifest v3 licencszabály ([CFA4-sdk `docs/protocol.md`](https://github.com/ubuntuokos/CFA4-sdk/blob/main/docs/protocol.md) 5. pont)

Az F3 kapu célja egy valódi AI-képesség végigvitele a minimumgépen (RTX 3060 12 GB): 2K-s klip,
Előnézet és Alkalmazás, projektverzió-ellenőrzés, Visszavonás. A képesség a #91, mert ez az
egyetlen tisztán ONNX-es, 1. fázisú típus a 200-as listában. Tartalék: #93 (videózajszűrés).

A licenceket 2026-10-31-én a repókból és az adathalmazok oldaláról ellenőriztem; a VRAM- és
sebességadatok **becslések**, mérés még nincs (`scripts/dev/run-minspec.sh`).

## 1. A fő megállapítás: tiszta tanítóadatú, nyilvános VSR-súly nincs

A manifest v3 a licencet négy részre bontja (kód, függőségek, súlyok, tanítóadat). A jelöltek
kódja rendben van, de **mindegyik tanítóadatában van ismeretlen licencű vagy csak kutatási
célra engedett rész**:

| Adathalmaz | Ki használja | Feltétel |
|---|---|---|
| DIV2K | Real-ESRGAN (DF2K) | „academic research purpose only” ([DIV2K](https://data.vision.ee.ethz.ch/cvl/DIV2K/)) |
| Flickr2K, OST | Real-ESRGAN | nincs megadott licenc (`NOASSERTION`) |
| REDS | RealBasicVSR, BasicVSR++ | CC BY 4.0 ([a szerző feltöltése](https://huggingface.co/datasets/snah/REDS/blob/main/README.md)) |
| FlyingChairs (a SPyNet előtanítása) | RealBasicVSR, BasicVSR++ | „research purposes only… Any commercial use is prohibited” ([FlyingChairs](https://lmb.informatik.uni-freiburg.de/resources/datasets/FlyingChairs.en.html)) |

Az elfogadott licencszabály szerint a `NOASSERTION` és az `unknown` minden részben blokkol, a
`no` pedig csak akkor, ha a felhasználó kereskedelmi felhasználást kér
(`route_decision.request.commercial`). **Ezzel a szabállyal egyik jelölt sem futhat:** a
Real-ESRGAN tanítóadatában ismeretlen licencű rész van (Flickr2K, OST), a RealBasicVSR és a
BasicVSR++ súlyának pedig nincs megadott licence.

**Tulajdonosi döntés (István, 2026-10-31): ilyen modell csak a zárt, fizetős workerben lehet.**
A GPL-es magban és a CFA4-gyel szállított GPL-es workerekben nincs olyan modell, amelynek
bármelyik licencrésze ismeretlen vagy csak kutatási célú. A zárt worker kínálhatja, a
manifestjében a valós licencrészekkel; a felelősség a worker kiadójáé.

Ehhez a gazda befogadási szabálya kivételt kap ([ADR 0006](../adr/ADR-0006-ai-processing-layer.md)
2. pont): a CFA4-gyel nem szállított zárt worker implementációjánál a `weights` és a `dataset`
rész `NOASSERTION`/`unknown` értéke nem blokkol, csak figyelmeztet. A `code` és a
`dependencies` ismeretlen értéke ott is blokkol, kereskedelmi kérésre pedig a `no` vagy
`unknown` rész mindenhol elutasítást ad. A CFA4-gyel szállított workerekre nincs kivétel. A
CFA4-sdk `docs/protocol.md` 5. pontját ugyanígy frissíteni kell (külön SDK-PR).

Kockázat: a fizetős worker maga kereskedelmi terjesztés, ezért a kutatási célú tanítóadat
(DIV2K, FlyingChairs) ott a legnagyobb jogi kockázat. A zárt worker kiadása előtt jogi
véleményt érdemes kérni, vagy saját tanítású súlyt használni.

## 2. Jelöltek

| Jelölt | Kód | Súly | Tanítóadat | ONNX | Időbeli? | Javaslat |
|---|---|---|---|---|---|---|
| **Real-ESRGAN `RealESRGAN_x4plus` / `x2plus`** (RRDBNet) | BSD-3-Clause | BSD-3-Clause (a repó GitHub-kiadásában) | DIV2K: `no`; Flickr2K, OST: `NOASSERTION` → a GPL-es magban blokkol, zárt workerben figyelmeztet | hivatalos `scripts/pytorch2onnx.py` (az x4-hez; az x2-höz `scale=2` kell), csak konvolúció | nem, képkockánként | zárt workeres út, a fizetős workerek kapujában |
| Real-ESRGAN `realesr-general-x4v3` (SRVGGNetCompact) | BSD-3-Clause | BSD-3-Clause | nem dokumentált: `unknown` | csak konvolúció és PixelShuffle | nem | tartalék a zárt workerben |
| **RealBasicVSR** | Apache-2.0 | a README Dropboxra mutat, külön licenc nélkül: `NOASSERTION` → blokkol | REDS + FlyingChairs: `no` | `grid_sample` (opset 16), kétirányú rekurzió: ablakos export kell | igen | **Második lépés** (F4), ha a súlylicencre a szerzőtől választ kapunk |
| BasicVSR++ | Apache-2.0 | OpenMMLab modelltár, külön licenc nélkül: `NOASSERTION` → blokkol | REDS + FlyingChairs: `no` | modulált deformálható konvolúció: ORT CUDA-támogatása ellenőrizendő | igen | tartalék |
| RVRT, VRT | CC BY-NC 4.0 | – | – | nehéz | igen | kizárva (a kód nem kereskedelmi) |
| EDVR | – | – | – | DCN | igen | referencia, 2019-es |

## 3. Második tulajdonosi döntés: az F3 más képességgel

A zárt worker az ADR 0006 szerint az első körön kívül van, ezért az F3 nem épülhet rá. A
tulajdonos döntése (István, 2026-10-31): **az F3 olyan ONNX-képességgel teljesüljön, amelynek
mind a négy licencrésze tiszta; a #91 később jön.** A #91 zárt workeres útja (Real-ESRGAN
`RealESRGAN_x2plus`) a fizetős workerek kapujában marad.

A többi ONNX-es képesség jelöltjeit ugyanígy megnéztem. Egyiknek sincs tiszta tanítóadata:

| Típus | Fő jelölt | Tanítóadat | Feltétel |
|---|---|---|---|
| #93 Videózajszűrés | FastDVDnet (MIT kód) | DAVIS 2017 | „Other”, nincs megadott licenc ([hyper.ai](https://hyper.ai/en/datasets/15541)) |
| #101 Képkocka-interpoláció | RIFE (MIT kód) | Vimeo-90K | nincs megadott licenc ([toflow #29](https://www.github.com/anchen1011/toflow/issues/29), válasz nélkül) |
| #137 Rotoszkópia | SAM 2 (Apache-2.0 kód és súly) | SA-V (CC BY 4.0) + SA-1B | SA-1B: „Research purposes only” ([Meta](https://ai.meta.com/datasets/segment-anything/)) |
| #114 Színezés | DeOldify (MIT kód) | ImageNet | csak kutatásra |
| #76 Háttércsere | RVM és társai | VideoMatte240K, COCO stb. | vegyes, részben ismeretlen |

**Következtetés:** a nyilvános, előtanított képi modellek közül egyik sem teljesíti a négy tiszta
részt. Tiszta F3-képesség csak **saját tanítással** készülhet, tiszta adaton. A legolcsóbb ilyen út
maga a #91 egyszerű, képkockánkénti változata:

- architektúra: SRVGGNetCompact (a Real-ESRGAN repóból, BSD-3-Clause), kb. 1,2 M paraméter,
  tisztán konvolúciós, ONNX-re triviálisan exportálható;
- tanítóadat: REDS (CC BY 4.0, a szerző feltöltése), előtanított súly nélkül, nulláról;
- a súly saját, a licence a CFA4-é; a REDS miatt forrásmegjelölés kell;
- becslés (mérés nélkül): x2-es modell néhány nap alatt tanítható egy 12 GB-os kártyán.

Ez nem a RealBasicVSR-szintű, hetes munka: kis modell, egyetlen képkockán, nincs mozgásbecslő.

## 4. Mérési terv (F3)

A `scripts/dev/run-minspec.sh` alatt (32 GB RAM, 8 mag, csak a 3060):

- ONNX-export fp32 és fp16, `onnxruntime` 1.2x CUDA EP-vel; az eredmény egyezése a PyTorch-csal
  (PSNR ≥ 45 dB ugyanazon a bemeneten).
- 2048×1080-as kimenet, 10 s-os klip: csúcs-VRAM, képkocka/s, csempemérettel (256, 512).
- CPU-s visszalépés ideje ugyanarra a klipre.
- Az eredmény a manifest `resources.vram_mb` és `fps_hint` mezőjébe kerül.

## 5. Nyitott kérdések

- Saját tanítású SRVGGNetCompact REDS-en legyen-e az F3 képessége (3. pont)?
- Kell-e jogi vélemény a zárt worker kiadása előtt (DIV2K, FlyingChairs)?
- Írjunk-e a RealBasicVSR és a BasicVSR++ szerzőjének a súlylicenc tisztázásáért?
