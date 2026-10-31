# #91 Videó-felskálázás (Video Super-Resolution): modellválasztás az F3 kapuhoz

> **Állapot:** javaslat; az 1. pont tulajdonosi döntése 2026-10-31-én megszületett · **Utolsó átnézés:** 2026-10-31 ·
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
| **Real-ESRGAN `RealESRGAN_x4plus` / `x2plus`** (RRDBNet) | BSD-3-Clause | BSD-3-Clause (a repó GitHub-kiadásában) | DIV2K: `no`; Flickr2K, OST: `NOASSERTION` → a GPL-es magban blokkol, zárt workerben figyelmeztet | hivatalos `scripts/pytorch2onnx.py` (az x4-hez; az x2-höz `scale=2` kell), csak konvolúció | nem, képkockánként | **Elsődleges az F3-hoz, csak a zárt workerben** |
| Real-ESRGAN `realesr-general-x4v3` (SRVGGNetCompact) | BSD-3-Clause | BSD-3-Clause | nem dokumentált: `unknown` | csak konvolúció és PixelShuffle | nem | tartalék a zárt workerben |
| **RealBasicVSR** | Apache-2.0 | a README Dropboxra mutat, külön licenc nélkül: `NOASSERTION` → blokkol | REDS + FlyingChairs: `no` | `grid_sample` (opset 16), kétirányú rekurzió: ablakos export kell | igen | **Második lépés** (F4), ha a súlylicencre a szerzőtől választ kapunk |
| BasicVSR++ | Apache-2.0 | OpenMMLab modelltár, külön licenc nélkül: `NOASSERTION` → blokkol | REDS + FlyingChairs: `no` | modulált deformálható konvolúció: ORT CUDA-támogatása ellenőrizendő | igen | tartalék |
| RVRT, VRT | CC BY-NC 4.0 | – | – | nehéz | igen | kizárva (a kód nem kereskedelmi) |
| EDVR | – | – | – | DCN | igen | referencia, 2019-es |

## 3. Javaslat a döntés után

1. **F3 a zárt workerrel:** a lánc (Előnézet, Alkalmazás, projektverzió, Visszavonás) a gazdában
   készül, a #91-et a zárt fizetős worker futtatja Real-ESRGAN `RealESRGAN_x2plus`-szal (2K
   kimenet 1024×540-es bemenetből), ONNX Runtime CUDA EP-n, fp16-ban, csempézéssel. A kód és a
   súly BSD-3, az ONNX-export a repó saját szkriptje, a modell tisztán konvolúciós, így az AMD
   (MIGraphX) és Intel (OpenVINO) EP is futtatja. Hátránya, hogy képkockánként dolgozik, ezért
   villoghat.
2. **GPL-es #91:** csak tiszta licencű súllyal. Ilyen nyilvános súly most nincs, ezért saját
   tanítás kell: RealBasicVSR-architektúra (Apache-2.0) REDS-en (CC BY 4.0), a SPyNet helyett
   tiszta adaton tanított mozgásbecslővel. Ez külön feladat, az F3-at nem tartja fel.
3. **F4: RealBasicVSR** az időbeli stabilitásért (a zárt workerben), ha a súly licence
   tisztázódik.
4. **Tartalék (#93): FastDVDnet** (MIT kód, tisztán konvolúciós). A súly- és tanítóadat-licenc
   (DAVIS) még ellenőrizendő.

A manifest helykitöltő mintája a CFA4-sdk `examples/manifests/vsr-onnx.toml` fájlja; a valódi
manifest az F3-ban, a mérés után készül.

## 4. Mérési terv (F3)

A `scripts/dev/run-minspec.sh` alatt (32 GB RAM, 8 mag, csak a 3060):

- ONNX-export fp32 és fp16, `onnxruntime` 1.2x CUDA EP-vel; az eredmény egyezése a PyTorch-csal
  (PSNR ≥ 45 dB ugyanazon a bemeneten).
- 2048×1080-as kimenet, 10 s-os klip: csúcs-VRAM, képkocka/s, csempemérettel (256, 512).
- CPU-s visszalépés ideje ugyanarra a klipre.
- Az eredmény a manifest `resources.vram_mb` és `fps_hint` mezőjébe kerül.

## 5. Nyitott kérdések

- Kell-e jogi vélemény a zárt worker kiadása előtt (DIV2K, FlyingChairs)?
- Írjunk-e a RealBasicVSR és a BasicVSR++ szerzőjének a súlylicenc tisztázásáért?
