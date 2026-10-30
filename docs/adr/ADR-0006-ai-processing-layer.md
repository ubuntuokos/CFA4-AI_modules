# CFA4 ADR 0006 – AI-feldolgozó réteg

**Dátum:** 2026-10-25, átdolgozva 2026-10-30  
**Állapot:** ELFOGADOTT TERV; KÓD MÉG NINCS  
**Kiadás:** HOLD

> **Utolsó átnézés:** 2026-10-30 · **Változás:** major (CFA4-AI-PLAN-001 és a CFA3/FA3 elődtervek
> beépítése: F0–F6 kapuk, ONNX-es első kör, modellkezelő, erőforrás-kezelési szabályok, tartós
> jobtár, bővített jobállapotok, szétbontott licencmezők) · **Részletes terv:**
> [CFA4 AI tervdokumentum](https://claude.ai/artifact/MLd6UfHG1uPBdJ16B2JZBh) ·
> **Összevetés és döntések:** [CFA4-AI-PLAN-001 összevetése](https://claude.ai/artifact/Yavio9epPDgKA6SmxQ8BGJ)

A számozás azért 0006, mert a 0004 (FFI-kivétel elve) és a 0005 (`cfa4-route`) már foglalt
az [ADR 0003](ADR-0003-aswf-and-conversion-sources.md) és a
[Kdenlive-paritás](../kdenlive-parity.md) tervében.

## Háttér

A CFA4 hibrid szerkesztő lesz: a hagyományos vágás mellett körülbelül 200 AI-funkciótípust kap
(lista: [`docs/ai/200-tipus-besorolas.csv`](../ai/200-tipus-besorolas.csv)). A 200 típus
12 motorcsaládba (M1–M12) és 32 valódi képességbe sűríthető. A többi ezek változata, sablonja
vagy kísérleti ága. A 107 jelölt projekt értékelése:
[`docs/ai/projektjeloltek.csv`](../ai/projektjeloltek.csv). A hang-, zene- és beszédfunkciók külön
táblában vannak ([`docs/ai/hang-besorolas.csv`](../ai/hang-besorolas.csv)): 60 funkció, amelyhez három
új motorcsalád kell (M13 beszédszintézis, M14 zene- és hanggenerálás, M15 hangleválasztás és
-helyreállítás).

A rétegnek ezeket a meglévő szabályokat kell betartania: soha ne omoljon össze (AGENTS.md §0),
minden művelet parancs legyen (`crates/engine`), tartsa a rétegezést (`cargo xtask layers`),
a licenc GPL-3.0-or-later maradjon ([`docs/licensing.md`](../licensing.md)), és ne legyen
futásidejű hálózati függőség ([ADR F8](ADR-F8-QT6-RUST-ENGINE-2026-10-23.md)).

**Átdolgozás, 2026-10-30.** A tulajdonos két forrás alapján kérte a terv frissítését:

- a CFA4-AI-PLAN-001 bírálati tervből: kapuk a hónapok helyett, egyetlen futtatókörnyezetes első
  kör, Előnézet lépés és projektverzió-ellenőrzés, explicit jobállapotok, szétbontott licencmezők;
- a CFA3 és az FA3 elődtervekből: modellkezelő
  ([CFA3 AI Video Tools v2.0 FINAL](https://github.com/ubuntuokos/CFA3-CinenemFoundArchitecture3/blob/main/docs/plans/accepted/CFA3-AI-VIDEO-TOOLS-V2-FINAL.md),
  6–8. és 10–13. fejezet) és az erőforrás-befogadás szabályai
  ([FA3](https://github.com/ubuntuokos/Final-Architecture-v3.0), `canonical/FA3-RESOURCE-ADMISSION-CONTRACTS-001.json`,
  `canonical/model-manager-v2-enforcement.json`).

Az elődtervekből csak a szabályokat vesszük át. A kódjuk Python, a CFA4 terméke tiszta Rust, ezért
újraírjuk.

## Orchesztrátor és komponenstérkép

**Az orchesztrátor** az a rész, amely egy AI-feladatot a kéréstől az idővonalig végigvisz: eldönti,
mi fusson, megszerzi hozzá a modellt és a hardvert, elindítja, figyeli, és az eredményt a
felhasználó elé teszi. A CFA4-ben ez nem egyetlen nagy komponens, hanem a `cfa4-ai-jobs` crate
feladattervezője és erőforrás-kezelője, amely a többi hat elemet hívja. Mindegyik elemnek egy
felelőse van, és egyik sem végzi a másik munkáját.

### A nyolc elem egy helyen

| # | Elem | Crate | Réteg | Felelős ezért | Nem felelős ezért | Előd (CFA3/FA3) |
|---|---|---|---|---|---|---|
| 1 | Képességjegyzék és útválasztó | `cfa4-ai-registry` | L2 | A manifestek beolvasása; a képesség, a nyelv, a modell és a futtató kiválasztása a gép VRAM-ja, gyártója és a telepített modellek szerint; „mi futna és miért” lekérdezés futtatás nélkül | Modellfájlok, hardverfoglalás | Model Router |
| 2 | Modellkezelő | `cfa4-ai-models` | L2 | Közös modelltár, SHA-256 ellenőrzés és deduplikáció, verziók, licenc, frissítés, törlés csak a felhasználó döntésével | Melyik modell fusson, melyik GPU-n | Model Manager |
| 3 | Feladattervező (az orchesztrátor egyik fele) | `cfa4-ai-jobs` | L2 | Elemzés → Javaslat → Előnézet → Alkalmazás lánc, szakaszolás (chunking), függőségek, jobállapotok | Modellválasztás, hardver | workflow-réteg |
| 4 | Erőforrás-kezelő (az orchesztrátor másik fele) | `cfa4-ai-jobs` | L2 | Befogadás és atomi foglalás, P0/P1/P2/R osztályok, GPU-nkénti VRAM-nyilvántartás (NVML), LRU modellmegtartás, OOM-visszalépési létra | Modellfájlok, a feladat tartalma | Host Resource Broker |
| 5 | Tartós jobtár | `cfa4-ai-jobs` (redb) | L2 | A jobok állapota és naplója, amely a CFA4 újraindítása után is megmarad | Futtatás | Temporal |
| 6 | Worker-felügyelő | `cfa4-ai-workers` | L3 | Worker-folyamatok indítása, felügyelete, újraindítása, az IPC kliensoldala | Ütemezés, modellválasztás | Package Management izolációja |
| 7 | Médiahíd és eredménytár | `cfa4-ai-workers` + `media` | L3 | Bemenetek átadása fájl- és asset-hivatkozással, eredmények `MediaSource`-ként, tartalomcímzett gyorsítótár | A projekt módosítása | Shared Asset Management |
| 8 | Naplózás és diagnosztika | `cfa4-ai-jobs` | L2 | Worker-hibák, VRAM-csúcsok, a minimumtesztek mérései, a CFA4 meglévő naplójába | Döntések | Evidence (egyszerűsítve) |

Mindegyik fölött az `ai.*` parancsok állnak (L4, `crates/engine`), és csak ők módosíthatják a
projektet: a felület, a CLI, a vezérlőcsatorna és az MCP is ezeken át éri el az AI-réteget. A
közös típusok (képességazonosító, jobállapotok, hibakódok) a `cfa4-ai-types` crate-ben (L1)
vannak.

```
Qt 6 felület / CLI / MCP
        │  ai.* parancsok (L4, crates/engine)
        ▼
┌──────────────── orchesztrátor: cfa4-ai-jobs ────────────────┐
│  feladattervező ◄──► erőforrás-kezelő      tartós jobtár    │
│        │                    │               naplózás        │
└────────┼────────────────────┼───────────────────────────────┘
         ▼                    ▼
 cfa4-ai-registry      cfa4-ai-models        cfa4-ai-workers ──► worker-folyamat
 (mi fusson)           (modellfájl)          (indítás, IPC)      (ONNX Runtime…)
                                                    │
                                             médiahíd és eredménytár
```

### Egy job útja

1. A felhasználó a Qt 6 felületen elindít egy funkciót; ez egy `ai.*` parancs.
2. A **feladattervező** létrehozza a jobot (`queued`), és megkérdezi a **jegyzéket**, melyik
   képesség, modell és futtató felel meg. Ha egyik sem, a job `blocked`, és a felület a jegyzék
   indoklását mutatja.
3. A **modellkezelő** ellenőrzi a modellt (hash, licenc, formátum): `preparing_model`. Hiányzó
   vagy ismeretlen licencű modellnél a job `blocked`.
4. Az **erőforrás-kezelő** befogadja a deklarált igényt, és atomin lefoglalja a GPU-t:
   `waiting_for_resources`, amíg nincs elég hely.
5. A **worker-felügyelő** elindítja vagy újrahasznosítja a workert, betölti a modellt
   (`loading`), majd futtatja (`running`); a bemenet a **médiahídon** át, hivatkozással megy.
6. Az eredmény az **eredménytárba** kerül új assetként (`succeeded`), a hiba kóddal (`failed`),
   a megszakítás `cancelling`, majd `cancelled`. Minden állapotváltás a **tartós jobtárba** és a
   **naplóba** kerül.
7. A felhasználó az Előnézetben megnézi az eredményt; az Alkalmazás parancs a projektverzió
   ellenőrzése után, egyetlen visszavonható lépésként teszi az idővonalra.

## Döntés

### 1. Komponensek

A nyolc elemet, a crate-jeiket és az orchesztrátor működését az
[Orchesztrátor és komponenstérkép](#orchesztrátor-és-komponenstérkép) rész írja le. Az alábbi
pontok az egyes elemek szabályai.

### 2. Modellkezelő

A CFA4 nem szállít modellt, és nincs automatikus letöltés. Modell csak a felhasználó kifejezett
műveletével kerül a gépre: kézi telepítéssel vagy a közös letöltővel ([Kdenlive-terv](../kdenlive-parity.md)
K18), jóváhagyással. A modellkezelő szabályai:

- **Közös modelltár** a felhasználó adatkönyvtárában. Minden képesség és worker innen olvas; a
  workerek nem tárolhatnak saját másolatot.
- **Tartalom szerinti deduplikáció:** a fájlok azonosítója a SHA-256. Ugyanaz a tartalom egyszer
  van meg, és ha több képesség egyszerre kér egy hiányzó modellt, egyetlen közös letöltés fut.
- **Modellrekord mezői:** azonosító, család, revízió, változat (például kvantálás), formátum,
  SHA-256, forrás, a négy licencmező (6. pont), a modellt használó képességek.
- **Rögzített revízió:** a manifest pontos revíziót és hash-t ad meg. „Latest” nem fogadható el.
- **Ismeretlen licenc vagy eredet blokkol:** az ilyen modellt a jegyzék nem kínálja fel, a job
  `blocked` állapotba kerül, és a felület megmondja, mi hiányzik.
- **Biztonságos formátum:** az első körben csak `.onnx` és `.safetensors`. Pickle-alapú formátum
  (`.pt`, `.bin`, `.ckpt`) csak a PyTorch-kapuban, külön szabállyal jöhet.
- **Frissítés:** az új verzió mellé kerül, ellenőrzés után lesz aktív. Sikertelen frissítés nem
  sértheti a működő verziót, és amíg egy képesség a régit igényli, a régi megmarad.
- **Törlés csak a felhasználó döntésével:** megszűnt vagy nem karbantartott modellt a CFA4 nem
  töröl magától, csak megmutatja, mely képességek használják.
- **Hardvert nem választ:** a modell csak az igényét deklarálja (VRAM, futtató), az elhelyezésről
  az erőforrás-kezelő dönt.

### 3. Erőforrás-kezelés szabályai

- A feladat deklarálja az igényeit (például `vram_gib >= 6`, futtató: ONNX Runtime CUDA EP), és
  **mindnek teljesülnie kell**. Hiányzó mérés vagy ismeretlen feltétel elutasítást jelent.
- **Nincs néma visszalépés.** GPU-ról CPU-ra vagy egyik futtatóról a másikra csak akkor vált a
  rendszer, ha a felhasználó vagy a manifest kifejezetten engedi. Ami nem fér el, az nem
  elérhető (a 7. pont szerint).
- **Stabil GPU-azonosítás:** UUID vagy PCI-cím szerint, nem a CUDA sorszáma szerint.
- **A kijelzőt hajtó GPU** alapból nem kap AI-feladatot, ha van másik alkalmas kártya. Egykártyás
  gépen a felhasználó engedélyezheti.
- **Atomi foglalás:** egy feladat az összes szükséges erőforrást egyszerre kapja meg, vagy semmit.
  Részleges foglalással nem várakozhat („tartok és várok”), így nem alakulhat ki holtpont.

### 4. Feladatok és jobállapotok

**Munkafolyamat:** Elemzés → Javaslat → Előnézet → Alkalmazás. Az AI soha nem írja közvetlenül a
projektet: az eredmény új asset vagy javaslat, amelyet a felhasználó előnézetben megnéz, és az
Alkalmazás parancs tesz az idővonalra. Az Alkalmazás előtt a motor ellenőrzi, hogy a projekt
verziója azonos-e azzal, amelyre a javaslat készült. Ha közben változott, a javaslatot újra kell
számolni vagy a felhasználónak jóvá kell hagynia. Az Alkalmazás egyetlen visszavonható lépés
(Undo/Redo).

**Jobállapotok:**

| Állapot | Jelentés |
|---|---|
| `queued` | Sorban áll |
| `preparing_model` | A modellkezelő ellenőrzi vagy előkészíti a modellt |
| `waiting_for_resources` | Az erőforrás-kezelő még nem tudja befogadni |
| `loading` | A worker tölti a modellt |
| `running` | Fut, `progress` üzenetekkel |
| `cancelling` | Megszakítás kérve, a worker még nem állt le |
| `succeeded` | Kész, az eredmény az eredménytárban |
| `failed` | Hiba (kóddal: `oom`, `invalid_input`, `model_error`, `crash`) |
| `cancelled` | Megszakítva |
| `blocked` | Nem indítható: hiányzó modell, ismeretlen licenc vagy nem teljesülő hardverigény |

A worker a kézfogásban deklarálja, hogy támogatja-e a megszakítást, az előrehaladás jelzését, az
előnézetet és a folytatást. A felület csak azt kínálja fel, amit a worker ténylegesen tud.

**Tartós jobtár:** a `cfa4-ai-jobs` tiszta Rust kulcs–érték tárban (redb) őrzi a jobokat. A
CFA4 újraindítása után a kész eredmények megmaradnak, a félbemaradt jobok `failed` vagy (ha a
worker támogatja) folytatható állapotba kerülnek. Külső workflow-szerver (Temporal) nincs.

### 5. Crate-ek és rétegek

A tervezett rétegbesorolás a crate-ek létrehozásakor az `xtask layers` szabályaiba is bekerül.

| Crate | Réteg | Tartalom |
|---|---|---|
| `cfa4-ai-types` | L1 | Közös típusok: képességazonosító, alak (frame/window/clip/generative), jobállapotok, hibakódok |
| `cfa4-ai-models` | L2 | Modelltár, modellrekordok, SHA-256 ellenőrzés, deduplikáció, verziók |
| `cfa4-ai-registry` | L2 | Manifestolvasás, sémaellenőrzés, hardver szerinti szűrés, útválasztás |
| `cfa4-ai-jobs` | L2 | Feladatgráf, erőforrás-kezelő, VRAM-nyilvántartás, gyorsítótár-kulcs, tartós jobtár |
| `cfa4-ai-workers` | L3 | Worker-folyamatok indítása, felügyelete és újraindítása, az IPC kliensoldala |
| `ai.*` parancsok | L4 | A `crates/engine` parancsregiszterében, nem a `ui-egui`-ban |

Az AI-panel csak a Qt 6 felületbe kerül, mert az egui kivezetés alatt áll. A beállítási
panelek a manifest JSON Schemájából generált natív Qt-vezérlők. Összetett szerkesztéshez
beágyazott QtWebEngine ablak is használható, ennek feltételei: csak helyi fájlok, QWebChannel,
és tilos a távoli navigáció. Az idővonalra csak a kész kép vagy videó kerül, az Alkalmazás
paranccsal.

### 6. Futtatókörnyezetek sorrendje

**Az első kör egyetlen futtatókörnyezet: ONNX Runtime, külön Rust worker-folyamatban**
(tulajdonosi döntés, 2026-10-30). Ezzel a csővezetéket igazoljuk (worker, protokoll, jobállapotok,
Előnézet és Alkalmazás, Undo) úgy, hogy egyszerre csak egy futtató romolhat el. Ez csak az első
lépcső, nem a végleges kör: a 200 típusból 25 fut tisztán ONNX-szel, 104-hez PyTorch, 28-hoz
helyi LLM kell. A többi futtató a későbbi kapukban jön, ugyanarra a protokollra.

1. **ONNX Runtime** (F2–F4): futtatókörnyezetek (EP) közül az első körben a CUDA és a CPU. Ez az út
   a gyártófüggetlen funkcióké, például a SAM2-é (Kdenlive-paritás K4). Mivel natív könyvtár,
   előfeltétele az ADR 0004 (FFI-kivétel elve).
2. **candle** (F5): a CFA4 folyamatán belül. Ezt használja már a `speech` (Whisper) és a `tts`
   (Kokoro).
3. **Egyetlen közös PyTorch-környezet** (F6), uv-vel rögzítve, egy CUDA 12.x-szel. Nincs
   modellenkénti venv, és nincs Kdenlive-stílusú CUDA-verziónkénti requirements-fájl. Külön
   környezetet csak írásos indoklással kaphat egy modell, és az is a közös modelltárat használja.

Az FFmpeg és az MLT marad, nem töröljük őket: az FFmpeg tesztorákulum (és a tervezett
`cfa4-ffmpeg` háttér a `cfa4-route` mögött), a `melt` szintén csak teszt. Egyikük sem az
AI-réteg motorja.

### 7. Worker-protokoll

- **Folyamatmodell:** minden worker külön folyamat, a Rust oldal felügyeli, összeomlása
  hibaüzenet és nem a CFA4 összeomlása.
- **Csatorna:** verziózott JSON-lines egy 0700-as jogú könyvtárban lévő Unix socketen. A
  hitelesítés nélküli 127.0.0.1-es vezérlőcsatornát (`docs/control-protocol.md`) nem használjuk erre.
- **Adatátadás:** az első körben csak asset- és fájlhivatkozással. Az osztott memória a
  protokollban megmarad, de csak mérés után kapcsoljuk be; a CUDA IPC későbbi kapu.
- **Kézfogás:** `hello`/`caps`, benne a protokollverzió, a képességek, a korlátok és a támogatott
  műveletek (megszakítás, előrehaladás, előnézet, folytatás).
- **Üzenetek:** `load`, `run`, `cancel`, `unload`, `progress`, `result`, `stats` és `error`.
  Az `error` kódjai: `oom`, `invalid_input`, `model_error`, `crash`.
- **Eszköz:** a `load` és a `run` megnevezi (`"cpu"` vagy GPU), az erőforrás-kezelő döntése szerint.
- **Kódolás:** MessagePackre csak akkor váltunk, ha a mérések ezt indokolják.

**A protokoll leírása, a manifest JSON Schemája és a worker SDK külön repóban készül**
([CFA4-sdk](https://github.com/ubuntuokos/CFA4-sdk)),
megengedő licenccel (MIT OR Apache-2.0, frissen írt kóddal). Erre azért van szükség, mert
a fizetős workerek vegyesek: van köztük GPL-es és zárt is. A CFA4 ebben a repóban csak
fogyasztja a sémát. A jobállapotok és a manifest v3 licencmezői az SDK-ban kerülnek a sémába.

### 8. Licenc és ingyenes/fizetős határ

- Az alapprogram és minden itteni crate GPL-3.0-or-later. AGPL és GPL-2.0-only kód nem kerülhet be.
- **A licenc négy külön mező** a manifestben (v3), mindegyik SPDX-azonosítóval és a kereskedelmi
  használat jelzésével: `code` (a worker kódja), `dependencies` (a futtató és a könyvtárak),
  `weights` (a modell súlyai), `dataset` (a tanítóadat, ha ismert). Egyetlen „kereskedelmi” igen/nem
  nem elég, mert a kód és a súly licence gyakran eltér. Ismeretlen mező blokkol (2. pont).
- Fizetős extra csak külön folyamatként futó worker lehet. Használati korlát (például feldolgozott
  időtartam vagy 720p-s kimeneti felbontás) csak a zárt workerekben élhet, a GPL-es magban soha.
  Az alapprogram képességeit nem csonkítjuk. A freemium az első körben nem szerepel.

### 9. Hardverminimum és tesztelés

- **Működési minimum** (alsó határ, nem felső; a tulajdonos 2026-10-30-án megerősítette): Linux,
  NVIDIA Ampere 12 GB (RTX 3060 12 GB), CUDA 12.x-es illesztő, 32 GB RAM, 8 mag AVX2-vel,
  150 GB NVMe. Minden ingyenes funkciónak 1080p-n működnie kell. 8 GB-os profil nincs.
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
- **Temporal mint workflow-motor** (CFA3): külön szerver, egy helyi szerkesztőhöz túl nehéz.
- **Az FA3 bizonyíték- és kapurendszere** (receipt-ek, kanonikus JSON-szabályok): a CFA4-ben a
  tesztek és a minimumtesztek a bizonyíték.
- **8 GB-os alapprofil** (PLAN-001): a tulajdonos a 12 GB-os minimum mellett döntött.

## Szakaszok (F0–F6 kapuk)

A hónapok helyett kapuk vannak: a következő kapu csak akkor nyílik, ha az előző kilépési
feltételei teljesültek. Minden kapu végén legyen valami, ami ellenőrizhetően működik.

**F0 – Előfeltétel.** A teljes Qt 6 átállás kész és az egui kikerült
([ADR F8](ADR-F8-QT6-RUST-ENGINE-2026-10-23.md)); az átállás 2026-10-25-én kb. 65%-on állt.
Addig ebben a repóban **nem indul AI-crate**, és nem kerül `ai.*` parancs a motorba. Az átállás
alatt is folytatható, mert nem érinti a felületet:

- dokumentáció: ez az ADR, a [besorolási táblák](../ai/README.md), a Kdenlive-terv 11.3 pontja;
- a minimumteszt-burkoló (`scripts/dev/run-minspec.sh`) és a tesztje;
- a worker-protokoll, a sémák (jobállapotok, manifest v3) és a `cfa4-ai-protocol` crate a külön
  [CFA4-sdk](https://github.com/ubuntuokos/CFA4-sdk) repóban (MIT OR Apache-2.0);
- a K2 (Whisper bekapcsolása), mert kész kód, és a [Kdenlive-terv](../kdenlive-parity.md)
  szerint független;
- a nyelvalkotó mag (`crates/conlang`), mert nem AI, nem GPU és nem felület.

*Kilépés:* az egui nincs a buildben, a Qt 6 shell a valódi motorral dolgozik.

**F1 – Alaptípusok és modellkezelő.** `cfa4-ai-types`, `cfa4-ai-models`, `cfa4-ai-registry`, a
`cfa4-ai-protocol` bekötése. *Kilépés:* a jegyzék beolvassa a manifesteket és a referenciagépen
helyesen szűr; a modellkezelő hash-t ellenőriz, deduplikál, és a hiányzó vagy ismeretlen licencű
modellt `blocked`-ként jelzi; a jegyzék futtatás nélkül is megmondja, mi futna és miért (vagy miért
nem), és a felület ezt mutatja; minden hibás bemenetre hibát ad, pánik nélkül.

**F2 – Jobok, erőforrás-kezelő, ONNX worker.** `cfa4-ai-jobs` (erőforrás-kezelő, tartós jobtár),
`cfa4-ai-workers`, az ONNX Runtime worker egy egyszerű tesztmodellel. *Kilépés:* a
`run-minspec.sh` alatt a job végigmegy az összes állapoton; a worker összeomlása, a megszakítás,
az OOM és a CFA4 újraindítása tesztelten kezelt.

**F3 – Első képesség.** #91 Video Super-Resolution ONNX-en, a teljes Elemzés → Javaslat →
Előnézet → Alkalmazás lánccal, projektverzió-ellenőrzéssel, Undo/Redo-val és Qt 6 panellel.
*Kilépés:* egy 1080p-s klip a referenciakártyán, a minimumkorlátok alatt, új assetként kerül az
idővonalra, az eredeti klip változatlan.

**F4 – Többi ONNX-képesség.** Zajszűrés, képkocka-interpoláció és a többi tisztán ONNX-es
képjavító (a 25-ből), valamint a SAM2-alapú maszkolás és háttéreltávolítás (K4).

**F5 – candle.** Átirat és szövegalapú vágás a meglévő `transcript.generate` funkcióval
(K2, G19), beszédszintézis (`cfa4-tts`), hangfunkciók.

**F6 – PyTorch és LLM.** A közös PyTorch-worker és a generatív családok (M1 videódiffúzió és a
többi) a VRAM-szintek szerint, a helyi LLM-es funkciók. A helyi LLM méretét (kis vagy nagy modell)
kérésenként egy kis ONNX-es szövegbeágyazó és pontozó választja, nem LLM-prompt; ez a meglévő
ONNX-workeren fut (ötlet: [weave-os/router](https://github.com/weave-os/router), Apache-2.0, csak a módszer).

**Az első körön kívül** (saját döntéssel, későbbi kapuban): freemium és fizetős workerek,
ComfyUI-kompatibilitás, több GPU, AMD és Intel, TensorRT, CUDA IPC, osztott memória
bekapcsolása, felhős futtatás.

A [besorolási táblák](../ai/README.md) „Fázis” oszlopa még a régi havi sorszámokat tartalmazza;
a kaput a futtató határozza meg: ONNX → F3/F4, candle → F5, PyTorch és helyi LLM → F6.

## Nyitott kérdések

- Melyik funkció legyen ingyenes, és melyik fizetős?
- Melyik korlát legyen a fizetős workerekben: időtartam, kimeneti felbontás, vagy mindkettő?
- Milyen hozzájárulási és vízjelezési szabály vonatkozzon az arccserére (38. típus)?
- Mi helyettesítse a SeamlessM4T v2 fordítót, amelynek a licence nem kereskedelmi?
- Melyik ONNX-modell legyen a #91 alapja (licenc és 1080p-s sebesség alapján)?
