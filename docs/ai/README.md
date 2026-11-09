# CFA4 AI – besorolási táblák

> **Állapot:** javaslat, egyetlen projekt–típus pár sincs még tesztelve · **Utolsó átnézés:**
> 2026-11-07 · **Változás:** minor (a „Fázis” oszlopból „Kapu” lett: AI-F0…AI-F6; a nem kereskedelmi súlyú modellek választhatók) · **Döntés:**
> [ADR 0008](../adr/ADR-0008-ai-processing-layer.md)

| Fájl | Tartalom |
|---|---|
| [`200-tipus-besorolas.csv`](200-tipus-besorolas.csv) | A 200 funkciótípus: csoport, motorcsalád (M1–M12), besorolás (képesség, változat, sablon, kísérleti), futtató, 12 GB-os futtathatóság, AI-kapu. A „Jelölt repók” oszlop a `projektjeloltek.csv` azonosítóira hivatkozik (típusonként legfeljebb 5 hely). A „P-kódok” az eredeti 200-as lista projektkódjai, ezek feloldása még hiányzik |
| [`projektjeloltek.csv`](projektjeloltek.csv) | A 107 jelölt repó: motorcsalád és javaslat (elsődleges, értékelendő Rust, tartalék, referencia, kihagyás) |
| [`hang-besorolas.csv`](hang-besorolas.csv) | 60 hang-, zene- és beszédfunkció: motorcsalád, besorolás, ami a CFA4-ben már megvan, elsődleges és tartalék jelölt, futtató, kód- és súlylicenc külön, kereskedelmi használat, 12 GB-os futtathatóság, AI-kapu |
| [`modellvalasztas-091.md`](modellvalasztas-091.md) | Az F3 kapu modellválasztása: a #91 videó-felskálázás jelöltjei négyrészes licenccel (kód, függőség, súly, tanítóadat), a tiszta licencű alternatívák keresése, a döntés (F3 = angol felirat-időzítés wav2vec2-vel) és a mérési terv |
| [`postgresql-adatreteg.md`](postgresql-adatreteg.md) | A feladatsor, az ütemezés, az AI-jobok, a licencelfogadások, a modellrekordok és a gyorsítótár-index PostgreSQL-adatrétege (CFA4-TASKSTORE-PG-001, elfogadva 2026-11-07): séma, kétszeri indítás elleni védelem, adatbázis-kiesés, PG-0…PG-4 fázisok az F1–F2 kapukban |
| [`acs-adaptiv-eroforras.md`](acs-adaptiv-eroforras.md) | Az adaptív erőforrás-kezelés (ACS, CFA4-ADAPTIVE-CAPABILITY-001) terve a tulajdonosi döntésekkel: ACS-0…ACS-7 fázisok, az ACS-1 (névleges és aktuális erőforrások, figyelő) leírása a `cfa4-tasks` crate-hez |

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

A manifestben a hangfunkció azonosítója `AU-nnn`, ahol `nnn` a tábla sorszáma (például `AU-010` a
beszédgenerálás), a 200-as lista `AI-nnn` azonosítója mellett.

A „Kapu” oszlop (2026-11-07 előtt „Fázis”, a régi havi sorszámmal) az
[ADR 0008](../adr/ADR-0008-ai-processing-layer.md) kapuját adja, amelyet a futtató határoz meg:
ONNX → AI-F4 (a felirat-időzítő AI-F3), candle és tract → AI-F5, PyTorch, helyi LLM és az M1/M7
motorcsaládra épülő funkciók → AI-F6. Több futtatónál a legkésőbbi kapu számít. A „nem AI” sor
a meglévő Rust-kóddal, modell nélkül készül.

A „12 GB-on” oszlopban a generatív videófunkciók (M1) a 12 GB-os gépen 480–720p-ben generálnak, és
a 2K-s kimenetet (az ADR 0008 9. pontjának teljesítményígérete) a #91 felskálázó adja; ez az
igazolt végrehajtási útjuk a minimumgépen. A „nagy VRAM” sorok a minimumgépen nem érhetők el, a
nagyobb kártya nyitja meg őket (ADR 0008, 3. és 9. pont). A „kész” azt jelenti, hogy a funkció AI nélkül, a meglévő
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
- **Nyelvenkénti modellválasztás, többnyelvű filmek (István, 2026-10-26).** Egy filmben a
  párbeszéd nyelve váltakozhat, ezért a nyelv nem a projekt vagy a fájl tulajdonsága, hanem
  szakaszé:
  1. **Átirat.** Ma a `cfa4-speech` az első 30 másodpercből egyszer becsül nyelvet, és az egész
     fájlra azt használja. Helyette ablakonként kell becsülni, és minden átiratszakasz saját
     `language` mezőt kap. A Text panelen ez kézzel javítható, mint a beszélőcímke.
  2. **Modellválasztás.** A Registry (képesség, nyelv) pár alapján választ a telepített modellek
     közül, a VRAM-szint szerint. A sorrend: a felhasználó választása, aztán a minőségi
     rangsor, végül az eSpeak NG tartalék. Ha egy nyelvhez nincs modell, a felület jelzi, és a
     K18 letöltővel pótolható.
  3. **Beszédhang.** A szkriptben soronként jelölhető a nyelv a `[pause 1s]` mintájára, például
     `[lang hu]`. Ha nincs jelölés, a felirat- vagy átiratszakasz nyelve az alapértelmezés.
  4. **Szereplők.** Szereplőnként nyelvenként rendelhető hang (szereplő → nyelv → hang). Ha
     ugyanannak a szereplőnek minden nyelven ugyanúgy kell szólnia, az csak klónozással
     (Chatterbox Multilingual) oldható meg, hozzájárulással.
  5. **Fordítás.** A forrásnyelv szakaszonként jön az átiratból.
  6. **Protokoll.** A manifestben `capability.languages` (BCP 47), a `run` paramétereiben
     `language`. A worker szakaszonként kapja a nyelvet, így egy kötegben több nyelv is lehet.
- **Példa (István): angol, kínai és magyar szereplő, köztük tolmács.**
  - A tolmács beszélőváltás nélkül vált nyelvet. Ezért a nyelvet nem szabad a beszélőből
    levezetni: a beszélő és a nyelv két külön címke minden szakaszon. A nyelvváltást a
    mondathatáron is keresni kell, nem csak az ablakhatáron.
  - Az átirat mindhárom nyelvet a meglévő Whisperrel kezeli; szakaszonként kell nyelvet becsülni.
  - Beszédhangnál a Kokoro angolul és kínaiul tud, magyarul a Piper. Ha a tolmácsnak mindhárom
    nyelven ugyanúgy kell szólnia, az ma nem megoldott: a Chatterbox Multilingual nem tud
    magyarul. Ez nyitott modellválasztási kérdés.
  - A tolmács mondatai a másik szereplő mondatának fordításai. Ha a szakasz „tolmácsolás”
    jelölést kap, és mutat az eredeti szakaszra, akkor a felirat és a szinkron kihagyhatja a
    felesleges ismétlést, vagy szándékosan megtarthatja.
  - A felirat célnyelvenként készül. Minden szakasz az eredeti nyelvéből fordítódik, nem a
    tolmács fordításából, hogy a hibák ne halmozódjanak.
- **Mesterséges nyelvek (István, 2026-10-26; főleg fantasy és sci-fi).**
  - **Jelölés.** Ahol van ISO 639-3 kód, azt kell használni: `tlh` (klingon), `qya` (quenya),
    `sjn` (sinda), `eo` (eszperantó), `jbo` (lojban), `tok` (toki pona). A kód nélküli nyelvek
    privát BCP 47 címkét kapnak: `art-x-<név>`, például `art-x-dothrak` vagy `art-x-navi`. Az
    `art` a mesterséges nyelvek gyűjtőkódja. A projekt saját nyelvdefiníciót tárol: név, címke,
    kiejtési szabályok, szótár és opcionálisan írásrendszer.
  - **Átirat.** A Whisper ezeket nem ismeri fel, a nyelvfelismerés rossz nyelvet tippel. Ezért az
    ilyen szakaszt kézzel kell jelölni, vagy alacsony biztonság esetén „ismeretlen nyelvűként”
    megjelölni. A szöveg a forgatókönyvből jön, az időzítést a kényszerített igazítás (K27) adja
    a kiejtési szabályok alapján.
  - **Beszédhang.** Az eSpeak NG-ben már van kiejtési szabály a quenyához, a sindához, a
    klingonhoz (`piqd`), a lojbanhoz, az eszperantóhoz és az interlinguához. Más nyelvhez a
    felhasználó adhat szabályt eSpeak NG formátumban vagy egyszerű betű–hang táblában. A fonémák
    a Kokoro, a Piper vagy egy klónozott hang bemenetei lehetnek, így a színész hangján is
    megszólalhat a mesterséges nyelv.
  - **Felirat és fordítás.** Gépi fordító nem ismeri ezeket a nyelveket. A szakasz fordítása a
    forgatókönyv glosszájából vagy kézzel jön, a gépi fordítás le van tiltva.
  - **Írás.** A tengwar vagy a pIqaD betűtípusa a külső betűtípus-mappába kerül, a repóba nem
    (AGENTS.md §1.9). Ezek az írások a Unicode magánhasználatú tartományában vannak, a CSUR
    szerint.
  - **Jogok (István, 2026-10-26).** A jogtulajdonoshoz tartozó nyelvet (klingon, na'vi, dothraki,
    valyrian) a felhasználó maga szerzi be vagy licenceli. A CFA4 nem szállít hozzá szótárt vagy
    szabályt, csak importálni tudja a felhasználó adatait a projekt nyelvdefiníciójába. A CFA4
    csak nyílt licencű adatot szállíthat, például az eSpeak NG GPL-3.0 adatait.
- **Új nyelv kitalálása (István, 2026-10-26: ez a legfontosabb).** A CFA4 saját nyelvalkotót kap
  (`AU-060`). Az eredmény a felhasználóé, a CFA4 semmilyen jogot nem tart fenn rá. A lépések:
  1. **Hangzás.** A felhasználó leírja a karaktert („érdes, torokhangú harcos nép”, „dallamos,
     sok magánhangzó”), vagy beállításokat választ. Ebből Rustban, magból (seed) generálva
     születik a hangkészlet, a szótagszerkezet, a hangsúly és a hanglejtés. Ugyanaz a mag
     ugyanazt a nyelvet adja.
  2. **Írás és kiejtés.** Latin betűs átírás és IPA-megfeleltetés készül. Ebből automatikusan
     keletkeznek az eSpeak NG formátumú kiejtési szabályok, így a beszédhang azonnal megszólal
     az új nyelven, akár a színész klónozott hangján is.
  3. **Szótár.** Egy alapfogalom-listához (Swadesh-típusú lista) generál szavakat, hangváltozási
     szabályokkal, hogy a nyelv „élőnek” hasson. Szűri a valódi nyelvek sértő vagy nevetséges
     egybeeséseit. A szótár a projektben tárolódik, kézzel szerkeszthető, és új szó igény
     szerint keletkezik.
  4. **Nyelvtan.** Szórend, ragozás és toldalékok sablonokból választva.
  5. **Fordítás.** A forgatókönyv sorait a helyi LLM (M8) a projekt szótára és nyelvtana alapján
     fordítja. Minden új szót és alakot visszaír a szótárba, így a nyelv a film végéig
     következetes marad.
  6. **Írásrendszer (később).** Generált jelkészlet, amely betűtípusként a felhasználó
     betűtípus-mappájába kerül, nem a repóba.
  7. **Kimenet.** Nyelvdefiníció `art-x-<név>` címkével, valamint szótár, nyelvtani összefoglaló,
     kiejtési szabályok és feliratfordítások. Exportálható, hogy más projekt vagy a stáb
     (például a színészek nyelvi edzője) is használhassa.

  A meglévő nyelvalkotó programok (például a Vulgarlang zárt, a többi licence változó) csak
  ötletforrások lehetnek, kódjuk és adatuk nem kerülhet be. A nyelvalkotó magja nem igényel AI-t
  és GPU-t, ezért korán elkészülhet. Az LLM csak a fordítást segíti.

  **Állapot (2026-10-30).** Az 1–4. és a 7. lépés magja kész a `crates/conlang` crate-ben (L2, tiszta
  Rust, felület nélkül; István döntése: GUI csak Qt6-ban készülhet). Négy hangzás-előbeállítás
  van (kiegyensúlyozott, érdes, dallamos, sziszegő). A kiejtési szabályokat az eSpeak NG `az`
  hangtáblájára írja. Toldalékolt alakokat (többes szám, múlt és jövő idő, tagadás) és egyszerű
  mondatokat (alany, ige, tárgy, jelző, eldöntendő kérdés) is képez a nyelvtan szerint; a
  szóhatáron kötőhang tartja kiejthetőnek és egyértelműen olvashatónak a szót. Egy teszt kb.
  16 000 szón (szótár és toldalékolt alakok) ellenőrzi, hogy az eSpeak NG pontosan a számolt
  IPA-t és hangsúlyt mondja. Hiányzik még: leírásból (szövegből) választott hangzás, esetek,
  névutók és személyragok, fordítás helyi LLM-mel (5.), írásrendszer (6.) és a Qt6-os
  szerkesztőfelület.
- **Nem kereskedelmi súlyú modellek.** MusicGen, AudioGen, MERT, F5-TTS, Fish Speech, XTTS v2, Audio
  Flamingo, MMS, SeamlessM4T: a CFA4 nem szállítja őket; a felhasználó tölti le, és egyszer elfogadja
  a feltételeiket (licencszabály, 2026-11-07, ADR 0008 2. pont). Funkciót nem tiltanak.
- **Tiltott függőség.** Az Essentia AGPL-3.0, és a CFA4 licencpolitikája ezt tiltja.
- **Kerülendő könyvtárak.** A `whisper-rs` és a `sherpa-onnx` C/C++ kötés, a `sherpa-onnx`
  ráadásul saját ONNX Runtime-ot hoz. A CFA4 csak a modelljeiket használja, a közös ONNX
  workerben.
- **Rust-jelöltek.** A DeepFilterNet és a Basic Pitch tract-tal tiszta Rustban fut. A
  `cargo deny` és a pánikmentesség ellenőrzése után folyamaton belül is futhatnak.
- **Hozzájárulás és jelölés.** A hangklónozásra, a beszéd- és énekhang-konverzióra és a beszéd
  átírására ugyanaz a szabály vonatkozik, mint az arccserére (ADR 0008, lezárt kérdések,
  2026-11-07): projektenként kötelező nyilatkozat az érintett hozzájárulásáról, és az exportban
  nem kikapcsolható „AI által módosított” jelölés és metaadat. A beszélőazonosítás biometrikus
  adat, ezért az arcindexeléshez hasonlóan csak projektenkénti bekapcsolással fut, és egy lépésben
  törölhető (PostgreSQL-adatréteg, 4. pont).
- **Python 3.14.** Az AI-workerek nem a rendszer Pythonját használják, hanem a `uv` által
  rögzített interpretert a közös PyTorch-környezetben (ADR 0008, 6. pont). Ezért például az
  AudioCraft régi Python-igénye nem akadály.
- **Modellkatalógus.** A worker-manifest ([CFA4-sdk](https://github.com/ubuntuokos/CFA4-sdk))
  már tárolja a kód és a súly licencét külön, a letöltési címet, a méretet, a VRAM-ot és a
  RAM-ot. A támogatott nyelvek mezője (`capability.languages`, BCP 47) a CFA4-sdk-ba kerül.
