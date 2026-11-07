# #91 Videó-felskálázás: saját tanítás terve

> **Állapot:** javaslat, tulajdonosi döntésre vár (9. pont) · **Utolsó átnézés:** 2026-11-07 ·
> **Változás:** major (első változat) · **Döntés:** [modellválasztás](modellvalasztas-091.md)
> 1–3. pont, [ADR 0006](../adr/ADR-0006-ai-processing-layer.md)

A nyilvános felskálázó-súlyok egyike sem teljesíti a manifest v3 négy tiszta licencrészét (kód,
függőségek, súly, tanítóadat), ezért a GPL-es magban és a CFA4-gyel szállított workerekben nem
futhatnak (modellválasztás 1. pont). A Real-ESRGAN a zárt, fizetős workerbe kerülhet, ott is csak
nem kereskedelmi kérésre; ennek mérőeszköze a `scripts/dev/vsr-measure` (PR #15).

Ez a terv a másik út: **saját tanítású felskálázó, tiszta adaton, nulláról**, előtanított súly
nélkül. Ha elkészül, a #91 ingyenes, kereskedelmi projektben is használható funkció lehet, és
mind a négy licencrésze tiszta.

A számok ebben a dokumentumban **becslések**, mérés még nincs.

## 1. Cél

| Szempont | Cél |
|---|---|
| Licenc | mind a négy rész tiszta: saját kód (GPL-3.0-or-later) vagy GPL-kompatibilis átvett kód, tiszta függőségek, saját súly, kereskedelmi felhasználást engedő tanítóadat |
| Feladat | 2× nagyítás (például 1024×540 → 2048×1080, 1080p → 2160p), később 4× |
| Minimumgép | RTX 3060 12 GB, `run-minspec.sh` alatt; 2K-s kimenet (DCI 2048×1080) |
| Futtató | ONNX Runtime (fp16 CUDA, fp32 CPU-s visszalépés), csempézve (`tiles.py`) |
| Minőség | valódi filmanyagon látható javulás a bicubic nagyításhoz képest, csempehatár és villódzás nélkül |

## 2. Tanítóadat

Szabály: minden tanítófájlnak ismert, kereskedelmi felhasználást engedő licence van, és a
forrását fájlonként nyilvántartjuk (forrás URL, licenc, szerző, letöltés dátuma, SHA-256). A
nyilvántartás a repóba kerül (`docs/ai/vsr-tanitoadat.csv`), maguk a fájlok nem. CC BY típusú
licencnél a súly kiadásakor forrásmegjelölés kell, ahogy az F3-nál a LibriSpeech esetén.

A licenceket 2026-11-07-én a forrásoldalakon ellenőriztem:

| Forrás | Tartalom | Licenc (idézet) | Kereskedelmi | Javaslat |
|---|---|---|---|---|
| **Saját felvétel** (István, CFA4 tulajdon) | a valódi célanyag, kameránként | a CFA4-é | igen | **elsődleges**, és ebből lesz a tesztkészlet is |
| **REDS** (NTIRE 2019) | 300 videó, 720p, utcai és mozgó jelenetek; a VSR-kutatás alapkészlete | CC BY 4.0: „REDS dataset is released under CC BY 4.0 license” ([szerzői oldal](https://seungjunnah.github.io/Datasets/reds.html), a [Hugging Face-kártya](https://huggingface.co/datasets/snah/REDS) ugyanígy) | igen, forrásmegjelöléssel | **elsődleges** |
| **Netflix Open Content** (Meridian, Sol Levante, Sparks, Chimera…) | 4K, részben HDR, profi filmanyag, élőszereplős és animált | CC BY 4.0: „Our open source content is available under the Creative Commons Attribution 4.0 International Public License” ([opencontent.netflix.com](https://opencontent.netflix.com/)); címenként külön licencfájl is van, azt letöltéskor fájlonként rögzítjük | igen, forrásmegjelöléssel | **elsődleges** (ez a legközelebb a CFA4 anyagához) |
| Tears of Steel (Blender, Mango) | 4K élőszereplős nyersanyag | CC BY 3.0: „you can freely reuse and distribute this content, also commercially, for as long you provide a proper attribution” ([mango.blender.org/sharing](https://mango.blender.org/sharing/)) | igen, de a szereplők képmáshoz fűződő joga megmarad („not to use the actor for making a commercial”) | tartalék: tanításra valószínűleg rendben van, mert a súly nem adja vissza a szereplőt, de ezt jogi szempontból érdemes megerősíteni |
| LSDIR | 85 ezer nagy felbontású kép | „made available for academic research purpose only” ([lsdir-data](https://ofsoundof.github.io/lsdir-data/)) | **nem** | kizárva |
| Wikimedia Commons (CC0 és közkincs) | vegyes képek és videók | fájlonként eltérő | fájltól függ | nem ellenőriztem (az oldal nem volt elérhető); csak fájlonkénti licencszűréssel jöhet szóba |

Képmás: a REDS és a Netflix-anyag is mutat felismerhető embereket. A felskálázó súlya nem tárolja
és nem adja vissza az arcokat, ezért a kockázatot alacsonynak tartom, de a súly kiadása előtti
jogi átnézésbe ez is bekerül.

Kizárva: LSDIR, DIV2K, Flickr2K, OST, Vimeo-90K, DAVIS, FlyingChairs, ImageNet és minden olyan halmaz,
amelynek licence ismeretlen vagy csak kutatásra enged (modellválasztás 1. és 3. pont). Stockoldalak
saját licencei (Pexels, Unsplash, Pixabay) sem: ezek nem szabad licencek, és a gépi tanulásra
vonatkozó feltételük nem egyértelmű.

**Validáló és tesztkészlet:** a tanítóadattól elkülönített klipek, lehetőleg saját felvétel. A
mérés ezekből az alacsony felbontású bemenetet ugyanúgy állítja elő, mint tanításkor, és van egy
valódi, nem leskálázott alacsony felbontású készlet is (régi felvétel, telefonvideó), amelyen csak
szemrevételezés lehetséges.

## 3. Architektúra

Két lépcső, a kisebb kockázatúval kezdve:

**A) Képkockánkénti kompakt modell: SRVGGNetCompact, 2×.** Kb. 1,2 M paraméter (64 csatorna,
32 konvolúció, PReLU, a végén PixelShuffle, és a bemenet legközelebbi-szomszédos nagyítása
hozzáadódik). A Real-ESRGAN repóban BSD-3-Clause; a definíció rövid, ezért az RRDBNet-hez
hasonlóan saját megírást javaslok. Tisztán konvolúciós, ONNX-re és később Rustra is egyszerű.
Becslés a 3060-ra, fp16, 2K-s kimenet: 10–20 képkocka/s, 2 GB alatti VRAM 512-es csempével.

**B) Időbeli (rekurrens) modell.** Az időbeli modell kevesebbet villódzik, és több részletet
állít helyre. A RealBasicVSR és a BasicVSR++ előtanított SPyNet-mozgásbecslőt használ, amely a
FlyingChairsen tanult (csak kutatásra), ezért mozgásbecslő nélküli vagy együtt, nulláról
tanított mozgásbecslős változat kell. Javaslat: az A modell törzse, rejtett állapottal, amely
képkockáról képkockára továbbadódik (egyirányú rekurzió, így élő előnézetnél is működik). Ez csak
az A modell eredménye után indul.

Az RRDBNet-et (Real-ESRGAN x2plus, 16,7 M paraméter) nem tanítjuk újra: becslés szerint 2K-n
2–5 képkocka/s a 3060-on. A PR #15 mérése megadja a pontos értéket; ez lesz a minőségi és
sebességi felső viszonyítási pont.

## 4. Degradáció és veszteségfüggvény

**Degradáció.** A tanítópár a jó minőségű képkockából és annak mesterségesen rontott, kicsinyített
változatából áll. A Real-ESRGAN cikkében leírt kétlépcsős rontást javaslom (elmosás, átméretezés,
zaj, JPEG-tömörítés kétszer, véletlen sorrendben és erősséggel), kiegészítve a videóra jellemző
hibákkal: H.264/H.265 tömörítés, sorváltás (interlace) maradványa, élesítési glória. Saját
megírás numpyval és torch-csal; a videókodekes rontáshoz az ffmpeg csak a tanítókészlet előállításán
fut (tesztkészlet-előállító, ahogy a CLAUDE.md engedi), a termékbe nem kerül.

**Veszteség.** Első lépés: L1 + MS-SSIM (PSNR-orientált modell). Második lépés: GAN-finomhangolás,
nulláról tanított U-Net diszkriminátorral. A szokásos perceptuális veszteség (VGG19, LPIPS) az
ImageNeten előtanított hálót használja, ami csak kutatásra engedett adat; ezt a tanításból
kihagyom (tulajdonosi kérdés, 9. pont). Méréshez az LPIPS használható, mert az nem kerül bele a
súlyba.

## 5. Kód és függőségek

- `scripts/dev/vsr-train/`: adatnyilvántartás-ellenőrző, degradáció, tanítóciklus, export. A
  `basicsr` csomagot nem használjuk (karbantartatlan, és a friss torchvision-nel nem fut); a
  tanítóciklus kicsi, saját.
- Ugyanaz a rögzített környezet, mint a mérésé (torch 2.5.1, onnxruntime-gpu 1.20.2, egy CUDA
  12.x), csak `numpy` és esetleg `scipy` pluszban.
- A `vsr-measure` bővül: SRVGGNetCompact-architektúra, és a saját súly hash-e a rögzített listába.
- Átvett kódot csak GPL-3.0-kompatibilis licenccel, ép fejléccel és nyilvántartási bejegyzéssel
  veszünk át (CFA4 `docs/licensing.md` 3. és 5. pont).

## 6. Mérési kapuk

A mérés a `run-minspec.sh` alatt fut, a `vsr-measure` eszközzel. Javasolt küszöbök (az első mérés
után a tulajdonos véglegesíti):

| Mérőszám | Küszöb |
|---|---|
| PSNR a tesztkészleten (rontott bemenet → eredeti) | legalább 1,5 dB-lel jobb a bicubic nagyításnál |
| Szemrevételezés valódi alacsony felbontású anyagon | István jóváhagyása, a Real-ESRGAN x2plus mellett, vakon összevetve |
| Villódzás (egymást követő képkockák különbsége a kimeneten és a bemeneten) | a kimeneté legfeljebb 10%-kal nagyobb |
| Csempehatár | nem látható; a csempézett és a teljes képkockás kimenet PSNR-je legalább 45 dB |
| ONNX és PyTorch egyezése | PSNR legalább 45 dB (`vsr-measure parity`) |
| Sebesség a 3060-on, fp16, 2K-s kimenet | legalább 10 képkocka/s (az A modellnél) |
| Csúcs-VRAM a 3060-on | legfeljebb 4 GB, hogy más AI-funkció is elférjen mellette |

## 7. Számítási igény és ütemezés

A tanítás nem a minimumgépen fut: ez fejlesztői munka a referenciagépen (RTX A4000 16 GB), és így
is jelöljük. A minimumgép csak a kész modell futtatására vonatkozik.

| Lépés | Tartalom | Becslés |
|---|---|---|
| V0 | Adatgyűjtés és nyilvántartás, tesztkészlet kiválasztása | 2–3 nap (István anyaga kell) |
| V1 | `vsr-train`: degradáció, tanítóciklus, tesztek; `vsr-measure` bővítése | 3–5 nap |
| V2 | A modell L1 + MS-SSIM tanítása (kb. 400 ezer lépés, 16-os köteg, 128-as csempe) | 2–4 nap gépidő az A4000-en |
| V3 | GAN-finomhangolás, mérés, szemrevételezés | 2–4 nap gépidő |
| V4 | Időbeli B modell | a V3 eredménye után tervezzük |
| V5 | Súlykiadás: ONNX, hash a manifestbe, forrásmegjelölés | 1 nap |

Sorrend az ADR 0006 szerint: a tanítóeszköz és a tanítás a terméken kívül fut, ezért az F0 kapu
(az egui eltávolítása) előtt is végezhető. A Rust-worker és a Qt-felület csak az F0 után jön.

## 8. A súly kiadása

- A súlyt a CFA4 nem szállítja: külön letöltés, a manifest `[[weights]]` részében rögzített
  SHA-256-tal, ahogy a többi modellnél.
- Licencrészek a manifestben: kód GPL-3.0-or-later, függőségek a mérőkörnyezet szerint, súly a
  tulajdonos által választott licenc (9. pont), tanítóadat a nyilvántartás összesítése (például
  CC BY 4.0 forrásmegjelöléssel).
- A tanítás pontosan visszakereshető: rögzített kódváltozat, adatnyilvántartás, véletlenmag és
  tanítási napló a kiadás mellé.

## 9. Tulajdonosi kérdések

1. **Saját felvétel.** Adsz-e saját, jó minőségű felvételt (például RED-anyagot vagy más 4K-s
   forrást) tanító- és tesztanyagnak? Ez a legtisztább adat.
2. **Perceptuális veszteség.** Kimaradjon-e az ImageNeten előtanított VGG a tanításból (tiszta
   út, javaslatom), vagy elfogadható, mert maga a VGG nem kerül bele a súlyba?
3. **A súly licence.** GPL-3.0-or-later (mint a CFA4), CC BY 4.0 vagy Apache-2.0? Javaslatom a
   CC BY 4.0, mert a tanítóadat jó része is ilyen, és a súly így más projektben is használható.
4. **Sorrend.** A tanítás indulhat-e az F0 kapu előtt (a terméket nem érinti), vagy várjon?
