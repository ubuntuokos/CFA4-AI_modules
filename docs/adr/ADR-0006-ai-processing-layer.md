# CFA4 ADR 0006 – AI-feldolgozó réteg

**Dátum:** 2026-10-25  
**Állapot:** JAVASLAT; KÓD MÉG NINCS  
**Kiadás:** HOLD

> **Utolsó átnézés:** 2026-10-25 · **Változás:** minor (Qt 6 utáni sorrend a szakaszokban) · **Részletes terv:**
> [CFA4 AI tervdokumentum](https://claude.ai/artifact/MLd6UfHG1uPBdJ16B2JZBh)

A számozás azért 0006, mert a 0004 (FFI-kivétel elve) és a 0005 (`cfa4-route`) már foglalt
az [ADR 0003](ADR-0003-aswf-and-conversion-sources.md) és a
[Kdenlive-paritás](../kdenlive-parity.md) tervében.

## Háttér

A CFA4 hibrid szerkesztő lesz: a hagyományos vágás mellett körülbelül 200 AI-funkciótípust kap
(lista: [`docs/ai/200-tipus-besorolas.csv`](../ai/200-tipus-besorolas.csv)). A 200 típus
12 motorcsaládba (M1–M12) és 32 valódi képességbe sűríthető. A többi ezek változata, sablonja
vagy kísérleti ága. A 107 jelölt projekt értékelése:
[`docs/ai/projektjeloltek.csv`](../ai/projektjeloltek.csv).

A rétegnek ezeket a meglévő szabályokat kell betartania: soha ne omoljon össze (AGENTS.md §0),
minden művelet parancs legyen (`crates/engine`), tartsa a rétegezést (`cargo xtask layers`),
a licenc GPL-3.0-or-later maradjon ([`docs/licensing.md`](../licensing.md)), és ne legyen
futásidejű hálózati függőség ([ADR F8](ADR-F8-QT6-RUST-ENGINE-2026-10-23.md)).

## Döntés

### 1. Öt komponens

| Komponens | Feladat |
|---|---|
| Képességjegyzék (Capability Registry) | A TOML-manifestek beolvasása, és a képességek szűrése a gépen ténylegesen elérhető VRAM, gyártó és telepített modell szerint |
| Gráf- és feladattervező | Az elemzés → javaslat → előnézet → alkalmazás lánc, szakaszolás (chunking), függőségek |
| Ütemező és erőforrás-kezelő | P0/P1/P2/R osztályok, GPU-nkénti VRAM-nyilvántartás (NVML), LRU modellmegtartás, OOM-visszalépési létra |
| Futtatók (Runtime Hosts) | candle a CFA4 folyamatán belül; ONNX Runtime és PyTorch külön worker-folyamatban |
| Médiahíd és eredménytár | Képkockák átadása a `media`/`frame` crate-ekből, eredmények `MediaSource`-ként, tartalomcímzett gyorsítótár |

### 2. Crate-ek és rétegek

A tervezett rétegbesorolás a crate-ek létrehozásakor az `xtask layers` szabályaiba is bekerül.

| Crate | Réteg | Tartalom |
|---|---|---|
| `cfa4-ai-types` | L1 | Közös típusok: képességazonosító, alak (frame/window/clip/generative), hibakódok |
| `cfa4-ai-registry` | L2 | Manifestolvasás, sémaellenőrzés, hardver szerinti szűrés |
| `cfa4-ai-jobs` | L2 | Feladatgráf, ütemező, VRAM-nyilvántartás, gyorsítótár-kulcs |
| `cfa4-ai-workers` | L3 | Worker-folyamatok indítása, felügyelete és újraindítása, az IPC kliensoldala |
| `ai.*` parancsok | L4 | A `crates/engine` parancsregiszterében, nem a `ui-egui`-ban |

Az AI-panel csak a Qt 6 felületbe kerül, mert az egui kivezetés alatt áll. A beállítási
panelek a manifest JSON Schemájából generált natív Qt-vezérlők. Összetett szerkesztéshez
beágyazott QtWebEngine ablak is használható, ennek feltételei: csak helyi fájlok, QWebChannel,
és tilos a távoli navigáció. Az idővonalra csak a kész kép vagy videó kerül, az Alkalmazás
paranccsal.

### 3. Futtatási sorrend és függőségek

1. **candle** a CFA4 folyamatán belül. Ezt használja már a `speech` (Whisper) és a `tts` (Kokoro).
2. **ONNX Runtime** külön Rust worker-folyamatban, futtatókörnyezetekkel (EP): CUDA/TensorRT,
   MIGraphX, OpenVINO. Ez az út a gyártófüggetlen funkcióké, például a SAM2-é (Kdenlive-paritás
   K4). Mivel natív könyvtár, előfeltétele az ADR 0004 (FFI-kivétel elve).
3. **Egyetlen közös PyTorch-környezet**, uv-vel rögzítve, egy CUDA 12.x-szel. Nincs
   modellenkénti venv, és nincs Kdenlive-stílusú CUDA-verziónkénti requirements-fájl. Külön
   környezetet csak írásos indoklással kaphat egy modell.

A modelleket a CFA4 nem szállítja: a felhasználó külön szerzi be őket, a jegyzék pedig csak a
telepítetteket kínálja fel. Az FFmpeg és az MLT marad, nem töröljük őket: az FFmpeg tesztorákulum
(és a tervezett `cfa4-ffmpeg` háttér a `cfa4-route` mögött), a `melt` szintén csak teszt.
Egyikük sem az AI-réteg motorja.

### 4. Worker-protokoll

- **Folyamatmodell:** minden worker külön folyamat, a Rust oldal felügyeli, összeomlása
  hibaüzenet és nem a CFA4 összeomlása.
- **Csatorna:** JSON-lines egy 0700-as jogú könyvtárban lévő Unix socketen. A hitelesítés
  nélküli 127.0.0.1-es vezérlőcsatornát (`docs/control-protocol.md`) nem használjuk erre.
- **Képkockák:** osztott memóriában utaznak, később CUDA IPC-vel.
- **Kézfogás:** `hello`/`caps`, benne a protokollverzió, a képességek és a korlátok.
- **Üzenetek:** `load`, `run`, `cancel`, `unload`, `progress`, `result`, `stats` és `error`.
  Az `error` kódjai: `oom`, `invalid_input`, `model_error`, `crash`.
- **Eszköz:** a `load` és a `run` megnevezi (`"cpu"` vagy GPU-index), így a CPU-s visszalépés és később a több GPU is működik.
- **Kódolás:** MessagePackre csak akkor váltunk, ha a mérések ezt indokolják.

**A protokoll leírása, a manifest JSON Schemája és a worker SDK külön repóban készül**
([CFA4-sdk](https://github.com/ubuntuokos/CFA4-sdk)),
megengedő licenccel (MIT OR Apache-2.0, frissen írt kóddal). Erre azért van szükség, mert
a fizetős workerek vegyesek: van köztük GPL-es és zárt is. A CFA4 ebben a repóban csak
fogyasztja a sémát.

### 5. Licenc és ingyenes/fizetős határ

- Az alapprogram és minden itteni crate GPL-3.0-or-later. AGPL és GPL-2.0-only kód nem kerülhet
  be, a modellek licencét képességenként rögzíteni kell a manifest `license.commercial` mezőjében.
- Fizetős extra csak külön folyamatként futó worker lehet. Használati korlát (például feldolgozott
  időtartam vagy 720p-s kimeneti felbontás) csak a zárt workerekben élhet, a GPL-es magban soha.
  Az alapprogram képességeit nem csonkítjuk.

### 6. Hardverminimum és tesztelés

- **Működési minimum** (alsó határ, nem felső): Linux, NVIDIA Ampere 12 GB (RTX 3060 12 GB),
  CUDA 12.x-es illesztő, 32 GB RAM, 8 mag AVX2-vel, 150 GB NVMe. Minden ingyenes funkciónak
  1080p-n működnie kell.
- **Nagyobb gépek:** a nagyobb VRAM nagyobb modelleket és több funkciót nyit meg, a több GPU
  pedig lehetőség, nem követelmény.
- **Gyártók:** első körben NVIDIA, utána 12 GB-os AMD és Intel kártyák az ONNX úton.
- **Minimumtesztek:** a [`scripts/dev/run-minspec.sh`](../../scripts/dev/run-minspec.sh)
  alatt futnak (32 GB RAM swap nélkül, 8 CPU, egyetlen referencia-GPU), mert a fejlesztőgép
  512 GB-os RAM-ja elfedné a memórianyomásból eredő hibákat. A burkoló szabályait a
  `scripts/ci/test-run-minspec.sh` ellenőrzi GPU nélkül, a gyors CI-kapuban.

## Elvetett alternatívák

- **ComfyUI mint futtatómotor:** sok egymásnak ellentmondó függőséget hoz, és nem
  ütemezhető a CFA4 parancsmotorján keresztül.
- **Modellenkénti Python-környezet vagy CUDA-verzió:** ez a „38 féle CUDA” helyzet.
- **AI-hívás közvetlenül a felületből:** megkerülné a parancsmotort, a visszavonást és a naplót.
- **Felhős AI alapértelmezésként:** ellentmond a helyi alapelvnek. A távoli futtatás később
  jöhet, saját ADR-rel.
- **Python a CFA4 folyamatán belül:** egy modellhiba a szerkesztőt is magával vinné.

## Szakaszok (havi lépések)

**Sorrend (tulajdonosi döntés, 2026-10-25):** előbb a teljes Qt 6 átállásnak kell
elkészülnie és az egui-nak ki kell kerülnie ([ADR F8](ADR-F8-QT6-RUST-ENGINE-2026-10-23.md)).
Az átállás 2026-10-25-én kb. 65%-on állt. Addig ebben a repóban **nem indul AI-crate**
(`cfa4-ai-types`, `-registry`, `-jobs`, `-workers`), és nem kerül `ai.*` parancs a motorba.
A hónapokat az átállás lezárásától kell számolni.

Az átállás alatt is folytatható, mert nem érinti a felületet:

- dokumentáció: ez az ADR, a [besorolási táblák](../ai/README.md), a Kdenlive-terv 11.3 pontja;
- a minimumteszt-burkoló (`scripts/dev/run-minspec.sh`) és a tesztje;
- a worker-protokoll, a sémák és a `cfa4-ai-protocol` crate a külön
  [CFA4-sdk](https://github.com/ubuntuokos/CFA4-sdk) repóban (MIT OR Apache-2.0);
- a K2 (Whisper bekapcsolása), mert kész kód, és a [Kdenlive-terv](../kdenlive-parity.md)
  szerint független.

Minden hónap végén legyen valami, ami az idővonalon használható.

1. **1. hónap:** a `cfa4-ai-types` és `cfa4-ai-registry` crate, a `cfa4-ai-protocol` bekötése.
   Az első képesség a jelenetfelismerés candle-on.
2. **2. hónap:** `cfa4-ai-jobs` és `cfa4-ai-workers`, a Qt shell ↔ motor adapter, az első
   PyTorch worker.
3. **3. hónap:** átirat és szövegalapú vágás. Ehhez a meglévő `transcript.generate` funkciót kell
   bekapcsolni a kiadott buildekben (K2, G19), új Whisper-modulra nincs szükség.
4. **4. hónap:** maszkolás és háttéreltávolítás (K4), SAM2 ONNX-en, gyártófüggetlenül.
5. **5. hónaptól:** felskálázás, képkocka-interpoláció, restaurálás, majd a generatív családok
   (M1 videódiffúzió és a többi) a VRAM-szintek szerint.

## Nyitott kérdések

- Melyik funkció legyen ingyenes, és melyik fizetős?
- Melyik korlát legyen a fizetős workerekben: időtartam, kimeneti felbontás, vagy mindkettő?
- Milyen hozzájárulási és vízjelezési szabály vonatkozzon az arccserére (38. típus)?
- Mi helyettesítse a SeamlessM4T v2 fordítót, amelynek a licence nem kereskedelmi?
