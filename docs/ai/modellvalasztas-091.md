# #91 Videó-felskálázás (Video Super-Resolution): modellválasztás az F3 kapuhoz

> **Állapot:** elfogadva; tulajdonosi döntések 2026-10-31-én (1. és 3. pont), 2026-11-02-án (F3: angol felirat-időzítés, 3.3 pont) és 2026-11-07-én (a felhasználó felelőssége, 1.1 pont; magyar felirat-időzítés Parakeet v3-mal, 5. pont) · **Utolsó átnézés:** 2026-11-07 (a lezárt kérdésekkel) ·
> **Változás:** major (első változat) · **Döntés:** [ADR 0008](../adr/ADR-0008-ai-processing-layer.md)
> F3 kapu, manifest v3 licencszabály ([CFA4-sdk `docs/protocol.md`](https://github.com/ubuntuokos/CFA4-sdk/blob/main/docs/protocol.md) 5. pont)

Az F3 kapu célja egy valódi AI-képesség végigvitele a minimumgépen (RTX 3060 12 GB): Előnézet
és Alkalmazás, projektverzió-ellenőrzés, Visszavonás. Eredetileg a #91 lett volna az, mert ez az
egyetlen tisztán ONNX-es, 1. fázisú típus a 200-as listában. A licencvizsgálat után a tulajdonos
**az angol felirat-időzítést választotta F3-nak (wav2vec2, 3.3 pont)**; a #91 később jön.

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

> **Felváltva.** Ezt a döntést előbb az 1.1 pont (2026-11-07, a felhasználó felelőssége), majd
> a 2026-11-08-i tulajdonosi döntés váltotta fel: a CFA4-nek nincs fizetős része, zárt vagy
> fizetős worker nem készül. A lenti szöveg történeti feljegyzés.

**Tulajdonosi döntés (István, 2026-10-31): ilyen modell csak a zárt, fizetős workerben lehet.**
A GPL-es magban és a CFA4-gyel szállított GPL-es workerekben nincs olyan modell, amelynek
bármelyik licencrésze ismeretlen vagy csak kutatási célú. A zárt worker kínálhatja, a
manifestjében a valós licencrészekkel; a felelősség a worker kiadójáé.

Ehhez a gazda befogadási szabálya kivételt kap ([ADR 0008](../adr/ADR-0008-ai-processing-layer.md)
2. pont): a CFA4-gyel nem szállított zárt worker implementációjánál a `weights` és a `dataset`
rész `NOASSERTION`/`unknown` értéke nem blokkol, csak figyelmeztet. A `code` és a
`dependencies` ismeretlen értéke ott is blokkol, kereskedelmi kérésre pedig a `no` vagy
`unknown` rész mindenhol elutasítást ad. A CFA4-gyel szállított workerekre nincs kivétel. A
CFA4-sdk `docs/protocol.md` 5. pontját ugyanígy frissíteni kell (külön SDK-PR).

**Következmény:** a kivétel a kereskedelmi kérést nem nyitja meg. A Real-ESRGAN zárt workeres
útja (DIV2K: `no`, Flickr2K és OST: `NOASSERTION`) ezért **csak nem kereskedelmi kérésre** futhat;
kereskedelmi projektben (`request.commercial = true`) a gazda elutasítja, amíg a tanítóadat
licence nem tisztázódik, vagy a súlyt saját, tiszta adaton újra nem tanítjuk.

Kockázat: a fizetős worker maga kereskedelmi terjesztés, ezért a kutatási célú tanítóadat
(DIV2K, FlyingChairs) ott a legnagyobb jogi kockázat. A zárt worker kiadása előtt jogi
véleményt érdemes kérni, vagy saját tanítású súlyt használni.

### 1.1 Negyedik tulajdonosi döntés: a felhasználó felelőssége (2026-11-07)

**István döntése (2026-11-07):** „ha modelt nem terjesztem akkor az a felhasználó felelőssége”.
Pontosítva (2026-11-07): **a CFA4 a modelleket nem terjeszti, csak az azokat használó
alkalmazásokat. Az alkalmazás felhasználójának felelőssége, hogy az adott modellre vonatkozó
felhasználási feltételeket megismerje, és a modellt az első alkalommal maga töltse le** (a közös
letöltő az ő kérésére, közvetlenül a forrásból, ahogy a legtöbb alkalmazás a Hugging Face-ről). Ez felváltja a
2026-10-31-i döntést, amely az ilyen modelleket a zárt workerbe szorította:

- A `weights` és a `dataset` rész ismeretlen, csak kutatási vagy nem kereskedelmi licence egyik
  workerben sem blokkol, a CFA4-gyel szállított GPL-es workerekben sem.
- A letöltés előtt a felület megmutatja a négy licencrészt, és a felhasználó implementációnként
  (és súlyhash-enként) egyszer elfogadja a figyelmeztetést. Kereskedelmi projektben erősebb a
  figyelmeztetés, de nincs tiltás.
- A `code` és a `dependencies` ismeretlen licence továbbra is blokkol.

**Következmény a #91-re:** a Real-ESRGAN `RealESRGAN_x2plus` az ingyenes, GPL-es workerben is
futhat; a mérése a `scripts/dev/vsr-measure` (CFA4-AI_modules #15).

**Saját modellt nem tanítunk** (István döntése, 2026-11-07: „nem csinálunk saját modellt”). A
lenti szövegek saját tanításra vonatkozó részei (3. pont, 3.1, 3.2) a korábbi állapotot rögzítik,
és nem érvényesek.

## 2. Jelöltek

| Jelölt | Kód | Súly | Tanítóadat | ONNX | Időbeli? | Javaslat |
|---|---|---|---|---|---|---|
| **Real-ESRGAN `RealESRGAN_x4plus` / `x2plus`** (RRDBNet) | BSD-3-Clause | BSD-3-Clause (a repó GitHub-kiadásában) | DIV2K: `no`; Flickr2K, OST: `NOASSERTION` → 2026-11-07 óta egyszeri elfogadással futhat (1.1 pont) | hivatalos `scripts/pytorch2onnx.py` (az x4-hez; az x2-höz `scale=2` kell), csak konvolúció | nem, képkockánként | **a #91 elsődleges jelöltje** (1.1 pont), mérés: `scripts/dev/vsr-measure` |
| Real-ESRGAN `realesr-general-x4v3` (SRVGGNetCompact) | BSD-3-Clause | BSD-3-Clause | nem dokumentált: `unknown` | csak konvolúció és PixelShuffle | nem | tartalék (#91, 1.1 pont szerint) |
| **RealBasicVSR** | Apache-2.0 | a README Dropboxra mutat, külön licenc nélkül: `NOASSERTION` → blokkol | REDS + FlyingChairs: `no` | `grid_sample` (opset 16), kétirányú rekurzió: ablakos export kell | igen | a fizetős workerek kapujában, ha a súlylicenc tisztázódik; GPL-es változathoz az architektúra újratanítása kell FlyingChairs nélkül |
| BasicVSR++ | Apache-2.0 | OpenMMLab modelltár, külön licenc nélkül: `NOASSERTION` → blokkol | REDS + FlyingChairs: `no` | modulált deformálható konvolúció: ORT CUDA-támogatása ellenőrizendő | igen | tartalék |
| RVRT, VRT | CC BY-NC 4.0 | – | – | nehéz | igen | kizárva (a kód nem kereskedelmi) |
| EDVR | – | – | – | DCN | igen | referencia, 2019-es |

## 3. Második tulajdonosi döntés: az F3 más képességgel

A zárt worker az ADR 0008 szerint az első körön kívül van, ezért az F3 nem épülhet rá. A
tulajdonos döntése (István, 2026-10-31): **az F3 olyan ONNX-képességgel teljesüljön, amelynek
mind a négy licencrésze tiszta; a #91 később jön.** ~~A #91 zárt workeres útja (Real-ESRGAN
`RealESRGAN_x2plus`) a fizetős workerek kapujában marad.~~ Fizetős workerkapu már nincs
(2026-11-08): a #91 útját az 1.1 pont (a súly és a tanítóadat licence a felhasználó felelőssége)
szerint, GPL-es workerben kell megtervezni.

A többi ONNX-es képesség jelöltjeit ugyanígy megnéztem. Egyiknek sincs tiszta tanítóadata:

| Típus | Fő jelölt | Tanítóadat | Feltétel |
|---|---|---|---|
| #93 Videózajszűrés | FastDVDnet (MIT kód) | DAVIS 2017 | „Other”, nincs megadott licenc ([hyper.ai](https://hyper.ai/en/datasets/15541)) |
| #101 Képkocka-interpoláció | RIFE (MIT kód) | Vimeo-90K | nincs megadott licenc ([toflow #29](https://www.github.com/anchen1011/toflow/issues/29), válasz nélkül) |
| #137 Rotoszkópia | SAM 2 (Apache-2.0 kód és súly) | SA-V (CC BY 4.0) + SA-1B | SA-1B: „Research purposes only” ([Meta](https://ai.meta.com/datasets/segment-anything/)) |
| #114 Színezés | DeOldify (MIT kód) | ImageNet | csak kutatásra |
| #76 Háttércsere | RVM és társai | VideoMatte240K, COCO stb. | vegyes, részben ismeretlen |

**Következtetés:** a nyilvános, előtanított képi modellek közül egyik sem teljesíti a négy tiszta
részt. Tiszta **képi** képesség csak **saját tanítással** készülhet, tiszta adaton (az F3 végül
előtanított beszédmodellel teljesül, lásd 3.2 és 3.3 pont). A legolcsóbb képi út maga a #91
egyszerű, képkockánkénti változata:

- architektúra: SRVGGNetCompact (a Real-ESRGAN repóból, BSD-3-Clause), kb. 1,2 M paraméter,
  tisztán konvolúciós, ONNX-re triviálisan exportálható;
- tanítóadat: REDS (CC BY 4.0, a szerző feltöltése), előtanított súly nélkül, nulláról;
- a súly saját, a licence a CFA4-é; a REDS miatt forrásmegjelölés kell;
- becslés (mérés nélkül): x2-es modell néhány nap alatt tanítható egy 12 GB-os kártyán.

Ez nem a RealBasicVSR-szintű, hetes munka: kis modell, egyetlen képkockán, nincs mozgásbecslő.

### 3.1 Hangképesség: az első tiszta, előtanított jelölt

A tulajdonos kérésére (István, 2026-10-31: „keress tovább, hang is”) a hangfunkciókat is
átnéztem. A Piper-hangok adatlapja hangonként megadja a tanítóadatot és azt, hogy nulláról vagy
finomhangolással készült a hang, így a négy rész ellenőrizhető.

| Hang | Kód | Függőségek | Súly | Tanítóadat | Eredmény |
|---|---|---|---|---|---|
| **Piper `en_US-ljspeech-high`** (AU-010 Beszédgenerálás) | saját GPL-es ONNX-inferencia | `onnxruntime` (MIT); eSpeak NG (GPL-3.0) külön folyamatban, fonemizálóként | MIT ([rhasspy/piper-voices](https://huggingface.co/rhasspy/piper-voices)) | LJ Speech: közkincs (LibriVox), „no restrictions” ([LJ Speech](https://keithito.com/LJ-Speech-Dataset/)); az adatlap szerint „Trained from scratch” | **mind a négy rész tiszta** |
| Piper `hu_HU-anna`, `hu_HU-imre` | ugyanaz | ugyanaz | MIT | CC0 ([OHF-Voice/voice-datasets](https://huggingface.co/datasets/OHF-Voice/voice-datasets)), de „Finetuned from U.S. English lessac voice” | **nem tiszta**: a lessac alaphang a Blizzard 2013 adatán tanult, ami csak kutatásra engedett (a kereskedelmi felhasználás kizárva) |

Ezzel két tiszta út van az F3-hoz:

1. **Piper `en_US-ljspeech-high` TTS mint F3-képesség.** Azonnal mérhető, nincs tanítás. Az F3
   kilépési feltétele ekkor nem 2K-s klip, hanem: szövegből generált beszéd új hangassetként
   kerül az idővonalra, Előnézettel, Alkalmazással, projektverzió-ellenőrzéssel és Visszavonással,
   a minimumkorlátok alatt. Ehhez az ADR 0008 F3 pontját módosítani kell. Csak angol.
2. **Saját tanítású magyar Piper-hang nulláról**, a CC0 magyar adaton. Tiszta, de tanítást kér
   (becslés mérés nélkül: néhány nap–egy hét egy 12 GB-os kártyán).

A 3. pont elején javasolt saját tanítású SRVGGNetCompact (REDS) harmadik útként megmarad; ez
megtartaná a videós F3-kilépési feltételt.

### 3.2 Harmadik kör: ONNX-en kívül is (2026-11-02)

A tulajdonos kérésére (István, 2026-11-02: „keress tovább”; 2026-11-01: „nincs kőbe vésve az ONNX
használat”) a candle- és PyTorch-modelleket is átnéztem. A futtató nem változtat a licencen, ezért
itt is a tanítóadat dönt.

**Tiszta (mind a négy rész):**

| Jelölt | Képesség | Súly | Tanítóadat | Megjegyzés |
|---|---|---|---|---|
| Piper `en_US-ljspeech-high` | beszédgenerálás (AU-010), angol, 1 hang | MIT | LJ Speech, közkincs; nulláról | lásd 3.1 |
| Piper `en_US-libritts-high` | beszédgenerálás, angol, sok beszélő (LibriTTS: 904) | MIT | LibriTTS train-clean-360, CC BY 4.0 ([adatlap](https://huggingface.co/rhasspy/piper-voices/blob/main/en/en_US/libritts/high/MODEL_CARD): „Trained from scratch”) | forrásmegjelölés kell; a `libritts_r` változat a lessac hangból finomhangolt, az nem tiszta |
| wav2vec2 `facebook/wav2vec2-base-960h` | angol beszédfelismerés és szöveg–hang illesztés (felirat időzítése a klipre) | Apache-2.0 | LibriSpeech 960 h, CC BY 4.0; elő- és finomtanítás is csak ezen ([adatlap](https://huggingface.co/facebook/wav2vec2-base-960h)) | ONNX-re exportálható; forrásmegjelölés kell |

**Majdnem tiszta, tulajdonosi döntést kér:**

- NVIDIA Parakeet TDT 0.6B v3 ([adatlap](https://huggingface.co/nvidia/parakeet-tdt-0.6b-v3)):
  beszédfelismerés 25 nyelven, **magyarral is**; súly CC BY 4.0, a kiadó szerint kereskedelmi
  célra is használható. A tanítóadat nagy része nyílt (LibriSpeech, Common Voice, MLS, VCTK, AMI,
  YouTube-Commons, YODAS), de benne van a Fisher-korpusz (LDC-licenc) és egy belső
  („in-house”) NeMo ASR Set 3.0 is. A szabályunk szerint a `dataset` rész ezért `NOASSERTION`,
  tehát a GPL-es workerben blokkol.

**Nem tiszta:**

- Kokoro-82M: a tanítóadat nincs felsorolva, és zárt TTS-ek szintetikus hangja is benne van
  (`NOASSERTION`).
- Piper `hu_HU` berta is (anna és imre mellett): lessac-finomhangolás.
- Képi helyreállítás: az LSDIR „academic research purpose only”
  ([LSDIR](https://data.vision.ee.ethz.ch/yawli/index.html)), így az erre tanított nyílt
  súlyok (például `4xLSDIRDAT`, CC BY 4.0 súly) adata sem tiszta; a HQ-50K CC BY 4.0-t ír, de a
  képek webes gyűjtésből jönnek, eredeti licenc nélkül.

**Következtetés:** előtanított, tiszta **képi** modellt ONNX-en kívül sem találtam. Tiszta
előtanított modell csak angol beszédre van (generálás és felismerés/illesztés). Videós F3-hoz
továbbra is saját tanítás kell (3. pont), magyar beszédhez saját Piper-hang (3.1).

Mellékes, de fontos: a CFA4 már használja a Whispert (`speech`) és a Kokorót (`tts`). A Whisper
tanítóadata nincs nyilvánosságra hozva, a Kokoróé nincs felsorolva, ezért a manifest v3
szabálya szerint mindkettő `dataset` része `NOASSERTION`. Ha ezek AI-workerbe kerülnek, ugyanez
a döntés kell rájuk is.

### 3.3 Harmadik tulajdonosi döntés: F3 = angol felirat-időzítés (2026-11-02)

István a 3.2 pont tiszta jelöltjei közül a **`facebook/wav2vec2-base-960h`-val végzett angol
felirat-időzítést** választotta. Ez a hanglista 37. sora (AU-037, „Szavankénti időzítés és
feliratigazítás”) és a Kdenlive-terv K27 pontja (kényszerített igazítás); a `cfa4-speech` mai
Whisper-szóidőit pontosítja vagy helyettesíti. A képesség:

- bemenet: egy klip hangja és a hozzá tartozó szöveg (sima szöveg vagy SRT, például forgatókönyv
  vagy kézzel írt felirat);
- előkészítés: a klip hangját monóra keverjük és 16 kHz-re mintavételezzük át (a modell csak 16 kHz-es
  bemenettel tanult). A bemenet a forrásmédia hangja a klip `source_in` és `source_out` pontja
  között, forrásidőben. A szóidőket a `TrackItem` saját leképezésének inverzével számoljuk
  vissza projektidőre (`crates/project`, `moving_source_time_at`): előre játszott klipnél
  t = `start` + (s − `source_in`) / |`speed`|, `Tick`-ben, kerekítéssel. A fordítva lejátszott
  (`reverse`) és a kimerevített (`frame_hold`) klipet a képesség elutasítja, mert ott a beszéd
  nem követhető; a kivágott részre eső szavak nem kerülnek a sávba;
- Elemzés: a modell 20 ms-onként karakter-valószínűséget ad (CTC), a szöveget erre illesztjük
  (Viterbi-alapú kényszerített illesztés, saját GPL-es Rust-kód);
- Javaslat és Előnézet: szavanként és feliratsoronként kezdő- és végidő, alacsony biztonságú
  helyek jelölésével;
- Alkalmazás: új feliratsáv az idővonalon, a klip változatlan, az Undo visszaveszi.

Korlátok: csak angol; a modell szókészlete nagybetűk és aposztróf, ezért a szöveget előbb
normalizálni kell (számok kiírása, írásjelek). Ami a modell számára ismeretlen szó, ott az
illesztés bizonytalanabb, ezt a Javaslat jelzi. A súly és a LibriSpeech miatt
forrásmegjelölés kell (Apache-2.0 NOTICE, CC BY 4.0).

## 4. Mérési terv (F3: felirat-időzítés)

A `scripts/dev/run-minspec.sh` alatt, mindkét profillal (`CFA4_PROFILE=ajanlott`: 32 GB RAM, `CFA4_PROFILE=minimum`: 16 GB RAM; 8 mag, csak a 3060):

- 44,1 és 48 kHz-es forrásklipek is: az átmintavételezés után az időbélyegek ugyanazok, mint
  16 kHz-es forrásnál.
- ONNX-export fp32 és fp16, `onnxruntime` 1.2x CUDA EP-vel; egyezés a PyTorch-csal ugyanazon a
  bemeneten (logitok legnagyobb eltérése, és azonos illesztési útvonal).
- Hosszú hang darabolása: 30 s-os ablakok átfedéssel; az ablakhatárokon nem csúszhat el az
  illesztés.
- Pontosság: saját, CFA4 tulajdonú, kézzel jelölt angol tesztklipeken a szók kezdetének és
  végének hány százaléka esik 50 ms-on belül, a feliratsorok kezdete és vége 100 ms-on belül, és a
  sorok időtartamának átfedése a kézi jelöléssel (IoU). Egymást átfedő vagy levágott sor nem
  lehet. (LDC-s korpuszt, például
  TIMIT-et, nem használunk.)
- Sebesség: 60 perces hang valós idejű tényezője (RTF) és csúcs-VRAM a 3060-on, valamint a CPU-s
  út ideje ugyanarra (a minimumgépen a GPU melletti másik igazolt út, nem GPU nélküli AI-út;
  ADR 0008, 3. pont).
- Az eredmény a manifest `resources.vram_mb` mezőjébe és a képesség sebességjelzőjébe kerül.

**Átmeneti küszöbök az F3 kilépéséhez** (az első mérés után a tulajdonos véglegesíti):

| Mérőszám | Küszöb |
|---|---|
| Szóhatár (kezdet és vég) 50 ms-on belül | legalább 80% |
| Feliratsor-határ (kezdet és vég) 100 ms-on belül | legalább 90% |
| A sorok átlagos IoU-ja a kézi jelöléssel | legalább 0,85 |
| Egymást átfedő vagy levágott sor | 0 |
| Illesztési hiba (`AlignError`) a tesztkészleten | 0 |
| ONNX és PyTorch illesztése | azonos útvonal fp32-n |
| RTF a 3060-on, fp16 | legfeljebb 0,1 (60 perc hang legfeljebb 6 perc alatt) |

A pontossági küszöböknek (az első öt sor) minden olyan pontosságon teljesülniük kell, amelyen a
worker fut: fp32-n, és ha a sebességhez fp16 kell, fp16-on is. Ha egy küszöb nem teljesül, az F3
nem zárul le.

## 4.1 Mérési terv (#91: Real-ESRGAN)

A `scripts/dev/run-minspec.sh` alatt, mindkét profillal (`CFA4_PROFILE=ajanlott`: 32 GB RAM, `CFA4_PROFILE=minimum`: 16 GB RAM; 8 mag, csak a 3060):

A felhasználó által letöltött Real-ESRGAN `RealESRGAN_x2plus`-hoz, a `scripts/dev/vsr-measure`
eszközzel:

- ONNX-export fp32 és fp16, `onnxruntime` 1.2x CUDA EP-vel; az eredmény egyezése a PyTorch-csal
  (PSNR ≥ 45 dB ugyanazon a bemeneten).
- 2048×1080-as kimenet, 10 s-os klip: csúcs-VRAM, képkocka/s, csempemérettel (256, 512).
- A CPU-s út ideje ugyanarra a klipre (a GPU melletti másik igazolt út; ADR 0008, 3. pont).
- Az eredmény a manifest `resources.vram_mb` és `fps_hint` mezőjébe kerül.

## 5. Lezárt kérdések (2026-11-07)

A licencszabály (1.1 pont, ADR 0008 2. pont) szerint a súly és a tanítóadat licence a felhasználó
felelőssége, ezért:

- A Parakeet v3 (magyar beszédfelismerés) a GPL-es workerben is futhat, a Fisher- és a belső
  tanítóadat ellenére, a négy licencrész megjelenítésével és egyszeri elfogadással.
- A már használt Whisper és Kokoro marad; a tanítóadatuk ismeretlen része ugyanígy a felhasználó
  elfogadásával kezelendő.
- Jogi vélemény a kutatási tanítóadatú modellek felkínálása előtt nem kell, mert a CFA4 modellt
  nem terjeszt.

- **Magyar felirat-időzítés (tulajdonosi döntés, 2026-11-07):** lesz, kész modellel, saját tanítás
  nélkül: az NVIDIA Parakeet TDT 0.6B v3 (CC BY 4.0 súly, a felhasználó tölti le, egyszeri
  elfogadással). ONNX-exportból fut, ezért az AI-F4 kapuba kerül; az F3 kilépése (angol,
  wav2vec2) nem változik. A bemenet, a forrásidő-visszaszámolás, a Javaslat–Előnézet–Alkalmazás
  lánc és a feliratsáv az F3 kódja; csak az illesztés módja más: a Parakeet TDT-dekódere nem
  karakterenkénti CTC-valószínűséget ad, ezért a modell szóidős átiratát illesztjük a megadott
  szöveghez (szószintű szövegillesztés, saját GPL-es Rust-kód), és ahol a kettő eltér, ott a hely
  alacsony biztonságúként jelenik meg a Javaslatban. A pontos módszert (például a TDT-időbélyegek
  vagy egy CTC-fej használata) és a küszöböket az AI-F4 mérése rögzíti, a 4. pont mintájára,
  magyar beszédes klippel.
