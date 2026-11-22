# CFA4-MODEL-MANAGER-001: az egységes modellkezelő és -letöltő, F0-audit és adaptáció

> **Állapot:** a 7. pont döntéseit (D1–D8) **a tulajdonos a javaslatok szerint elfogadta (István, 2026-11-22)**; a CFA3-összevetésből jött **D9 (a, b) is elfogadva (István, 2026-11-22)**; az llmfit beépítése (6a pont, D10) **a CFA4 része, nem külön alkalmazás (István, 2026-11-22)**; a megvalósítás indítása külön engedélyt kér · **Készült:** 2026-11-22 · **Nyelv:** magyar
> **Nem indít el semmit:** a megvalósítás az F0 kapu (Qt6 kész, egui kint, [ADR 0008](../adr/ADR-0008-ai-processing-layer.md))
> és a tulajdonos jóváhagyása után kezdődik. Ez a dokumentum a megbízás (CFA4-MODEL-MANAGER-001, 2026-11-22)
> **F0 – Architecture & Audit** szakasza és az abból következő, CFA4-hez igazított terv.

## 0. Röviden

- A megbízás nagy része **már le van fektetve** az ADR 0008 2. pontjában (`cfa4-ai-models`), a
  [PostgreSQL-tervben](postgresql-adatreteg.md) (`ai.model`, `ai.model_use`), az SDK manifestjében (pontos revízió, SHA-256,
  négyrészes licenc) és a CFA4 meglévő `cfa4-fetch` letöltőjében. Ezeket **bővítem, nem duplikálom**.
- **Új** a megbízásból: a katalógus (keresés, szűrés, adatlap), az importfolyamat és a karantén, a
  tárolóterületek szétválasztása (store, cache, staging, quarantine), a frissítés és az eltávolítás szabályai, a
  kompatibilitási címkék, a Qt6 felület hét lapja.
- **Eltérés a már döntött szabályoktól** (3. pont): négy helyen a megbízás szigorúbb vagy mást ír, mint a
  tulajdonos 2026-11-07-i döntése. Ezeket nem írom át magamtól, a 7. pontban kérek döntést.
- **CFA3-összevetés** (2026-11-22, a tulajdonos megadta a repót): a 2. pont után külön táblázatban (2a). Két hiányt találtam, amelyek a tervből
  kimaradtak; ezeket a D9 pont rögzíti (elfogadva).

## 1. Mit vizsgáltam

CFA3 `docs/plans/accepted/CFA3-AI-VIDEO-TOOLS-V2-FINAL.md` (6–8., 20.3 pont) és `docs/architecture/AI-VIDEO-TOOLS-CFA3-INTEGRATION.md` (a CFA3 repó többi részét nem néztem át), ADR 0008 (2., 5., 6., 7. pont, F1–F2 kapu), `docs/ai/postgresql-adatreteg.md`, `docs/ai/modellvalasztas-091.md`,
`docs/ai/acs-adaptiv-eroforras.md`, a CFA4-SDK `manifest.schema.json` (6. verzió) és `host.schema.json`
(jobállapotok), a CFA4 `apps/cfa4-fetch` (CFA4-SECURITY-001 D5), a CFA4-AI-MODEL-TRAINING-001 modellre vonatkozó
pontjai (3. és a nyilvántartás-hivatkozás). **Nem vizsgáltam:** a CFA3 repó modellkezelésen kívüli részeit, a `plan/` mappát (gép-lokális),
és nem futtattam semmit; ez dokumentumaudit, kódot nem érint.

## 2. Összevetés: követelmény → meglévő → döntés

| # | Megbízás | Ma a CFA4-ben | Döntés |
|---|---|---|---|
| 1 | Központi nyilvántartás (Registry) | ADR 0008 2. pont: `cfa4-ai-models`; PG `ai.model` | **Bővítés**: a rekord kap revíziót, artifact-azonosságot, forrást, megbízhatósági szintet (3.1) |
| 2 | Katalógus, keresés, szűrés, adatlap | `cfa4-ai-registry` a manifestekből szűr, nincs adatlap/katalógusnézet | **Új** réteg a manifestekre és egy opcionális katalógusindexre (4. pont) |
| 3 | Hivatalos forrás és letöltés | közös letöltő, felhasználó indítja; `cfa4-fetch` (HTTPS 443, csak a célkönyvtárba ír, SHA-256, `.part`) | **Újrahasznosítás**: a „Managed Download” a `cfa4-fetch`-et hívja, külön letöltő nem készül |
| 4 | Download Queue | `cfa4-tasks` (ACS) feladatsor, a feladattípusok között „modellbetöltés” | **Nem új sor**: a letöltés a közös feladatsor egy feladattípusa; a „Downloads” lap a Task Manager nézete (ACS-5) |
| 5 | Integritás | SHA-256, rögzített revízió, „latest” tilos | **Kész szabály**; kiegészül méret-, formátum- és manifest-ellenőrzéssel, és a karanténnal |
| 6 | Shared Store, deduplikáció | „közös modelltár”, SHA-256 szerinti deduplikáció, egyetlen közös letöltés | **Kész szabály**; fizikai azonosság csak hash alapján (megbízás: fájlnév nem elég) |
| 7 | Store, cache, staging, karantén | cache van, a staging/karantén nincs leírva | **Új**: négy terület; a tár helyét a felhasználó adja meg (5. pont, tulajdonosi döntés 2026-11-22) |
| 8 | Kompatibilitás | ACS: hardverfelmérés, végrehajtási terv, `Decision` (Run/Wait/NeedsApproval/Unavailable) | **Integráció**: a Compatibility Engine az ACS tervezőjének nézete, nem új motor |
| 9 | Függőségek, használati kapcsolatok | PG `ai.model_use` | **Kész**; kiegészül a törlés előtti ellenőrzéssel |
| 10 | Provenance | manifest: forrás URL, revízió, hash | **Bővítés** forrásmegbízhatósággal és ellenőrzési rekorddal |
| 11 | Licenckezelés | négy licencrész, egyszeri elfogadás modellenként és hash-enként | **Kész szabály**, az adatlap megmutatja (7. pont: nem blokkol) |
| 12 | Frissítés, eltávolítás | „törlés csak a felhasználó döntésével” | **Új** részletszabály (6. pont) |
| 13 | Qt6 felület, 7 lap | ACS-5 Task Manager már a mainben; modellfelület nincs | **Új**, az F4-ben (Qt6, nem egui) |
| 14 | AI-modulok közös használata | ADR 0008: minden képesség és worker a közös tárból olvas | **Kész elv**; a workerek nem tölthetnek és nem törölhetnek (az SDK-protokollban is rögzítendő) |
| 15 | Offline-first | helyi-első alapelv | **Kész**; a 5. pont felsorolja az offline és online műveleteket |
| 16 | PostgreSQL | `ai` séma (PG-3), tokio-postgres + deadpool | **Bővítés** a megbízás 17 entitására, de a meglévő táblákra építve (3.2) |

## 2a. Összevetés a CFA3 modellkezelővel (6–8. fejezet)

| CFA3 elv | CFA4-MODEL-MANAGER-001 | Döntés |
|---|---|---|
| 6.1 kizárólagos modellkezelés, modul nem készít sajátot | 12. pont, AC-10, AC-17–19 | átvéve |
| 6.2 modell nélküli telepítés | ADR 0008, „a CFA4 nem szállít modellt” | átvéve |
| 6.3 **hiányzó modellt a Model Manager automatikusan beszerzi** a generátor indításakor | AC-03, AC-21: soha nincs automatikus letöltés; AC-26: javaslat igen | **eltérés, szándékos** (István, 2026-11-22): nem tölt le, de javaslatot tesz a modellre, és ha a modell már nem létezik, helyettesítőt javasol |
| 6.4 azonos modell ismételt letöltése tilos | deduplikáció hash szerint (5. pont) | átvéve |
| 6.5 több kérés egyetlen közös letöltést kap | egy letöltési feladat, a többi kérés ahhoz kapcsolódik | átvéve (a `cfa4-tasks` feladatsor azonosító szerint összevon) |
| 6.6 azonosság csak tartalom szerint | 3.2 | átvéve |
| 7.1 frissítés, 8. lépés: a kompatibilis alkalmazások hivatkozásait a rendszer átállítja | 5. pont: frissítés csak jóváhagyással | **eltérés**: az átállás csak a felhasználó aktiválása után, **modulonként** (D9/b) |
| 7.2 kompatibilitásvédelem: inkompatibilis frissítésnél a működő verzió megmarad, használatban lévő fájl nem íródik felül | 5. pont: régi revízió megmarad | **bővítés** modulonkénti rögzítéssel (D9/b) |
| 7.3 sikertelen frissítés nem sérti a működőt | 5. pont, 20. pont | átvéve |
| 8. megszűnt/nem karbantartott modell: **megtartás vagy eltávolítás**, a forrás állapotát külön kezelni (átmeneti hiba, tartós elérhetetlenség, hivatalosan megszűnt, archivált, visszavont kiadás) | csak `retired` állapot | **hiány** (D9/a) |
| 20.3 elfogadási tesztek | AC-01…22 | lefedve; „megszűnt modellnél választás” új AC (D9/a) |
| Model Router, Temporal, Host Resource Broker | `cfa4-ai-registry`/ACS tervező, `cfa4-tasks`, nincs Temporal | CFA4-ben már másként megoldva, nem veszem át |

**D9 (kiegészítés a CFA3-ból, elfogadva, István, 2026-11-22):**
a) **Forrásállapot és megszűnt modellek:** a modellforrás állapota külön mező: `ok`, `temporarily_unreachable`, `permanently_unreachable`,
   `discontinued`, `archived`, `withdrawn`. Átmeneti hiba nem jelent megszűnést; az állapotot ellenőrzött információ
   alapján lehet átállítani. Megszűnt modellnél a helyi példányt a CFA4 soha nem törli automatikusan: a felhasználó
   **megtartja** (marad a tárban, használható, a megszűnt állapot látszik) vagy **eltávolítja** (a függőségek és a
   következmények bemutatása után, megerősítéssel). Új elfogadási feltétel: **AC-23** megszűnt modellnél a felhasználó
   választhat megtartás és eltávolítás között; teszt: generált forrásállapotok.
b) **Modulonkénti revízió-rögzítés:** a `ai.model_use` kapcsolat revíziót is rögzít. Frissítés után egy modul csak a felhasználó
   aktiválásakor áll át az új revízióra; ha az új revízió egy modullal inkompatibilis, az a modul a régi, működő revízión
   marad, és a használatban lévő fájl nem íródik felül. Új elfogadási feltétel: **AC-24**.

## 3. Adatmodell

### 3.1 Állapotok

A megbízás 19 állapotot kér. A jobállapotok (SDK `host.schema.json`) külön vannak és a szerződés részei; a
modellállapot a modellrekordé. **Javaslat:** a tárolt állapotok: `discovered`, `installable`, `approved`
(a felhasználó a licencet elfogadta), `acquiring`, `paused`, `verifying`, `installed`, `update_available`,
`incompatible`, `blocked`, `quarantined`, `retired`, `removing`, `removed`. A megbízás többi állapota **származtatott,
nem tárolt**: `AVAILABLE`/`AVAILABLE_LOCAL` (= `installed`), `VERIFIED`/`ADMITTED` (az ellenőrzési rekord és a
`installed` átmenet), `IN_USE` (az `ai.model_use` és a futó jobok alapján). Így nem kerül két helyre ugyanaz az
igazság. A felület egyszerű címkéket mutat.

### 3.2 Entitások

A megbízás 17 entitása a meglévő `ai.model` és `ai.model_use` köré épül: `ai.model_family`, `ai.model_revision`,
`ai.model_artifact` (hash, méret, formátum, precízió, kvantálás, tárolási út), `ai.model_source` (URL, megbízhatóság:
`attributed`, `validated`, `canonical`, `unverified`, `blocked`), `ai.model_license` (a négy rész és az ellenőrzés
dátuma), `ai.model_capability`, `ai.model_storage`, `ai.verification` és `ai.model_event` (életciklus-napló). A letöltési
és importfeladat a `tasks.task` táblában van, nem külön táblában. Séma és migráció: PG-3 kapu, a meglévő PG-terv szerint.
Fájlnévre vagy útra nem épül azonosság; az azonosság (modell, revízió, formátum, változat, hash).

## 4. Katalógus

- **Forrás:** (1) a telepített workerek manifestjei (`cfa4-ai-registry`), (2) egy **opcionális katalógusindex-fájl**
  (aláírt, a felhasználó kéri le, ha online akar frissíteni), (3) a felhasználó saját importja. A katalógusba
  kerülés nem engedély és nem licenc-jóváhagyás.
- **Szűrők:** capability, modalitás, család, paraméterszám, méret, kvantálás, formátum, kompatibilitás, RAM/VRAM,
  licenc, offline használhatóság, modul, telepített-e. A szűrés az adatbázisban fut, nem a felületen.
- **Adatlap:** a megbízás 5. pontjának mezői, a manifestből és a rekordból; hiányzó adat „ismeretlen”, nem kitalált.
- **Kategóriák:** Video, Audio, Image, Vision & Analysis, Text & LLM, Embeddings, Multimodal; a lista adat, nem kód.

## 5. Tárolás, import, életciklus

- **A tárolási helyet a felhasználó adja meg** (István, 2026-11-22: ő tudja, melyik meghajtóján van elég hely). Az első
  modellművelet (import vagy letöltés) előtt a CFA4 megkérdezi a modelltár helyét; addig nincs letöltés és nincs
  alapértelmezett, csendben használt útvonal. A választóban meghajtónként látszik a szabad hely, az írhatóság és a
  fájlrendszer, és a kért modell becsült mérete; az XDG-útvonal (`$XDG_DATA_HOME/cfa4/models/`) csak **javaslat**, amelyet
  a felhasználó elfogadhat vagy felülírhat. Több tár megadható, és a felhasználó modellenként választja, melyikre kerüljön; a
  hely utólag a felhasználó kifejezett parancsával áthelyezhető. A választott helyet a CFA4 ellenőrzi (létezik, írható, elég
  szabad hely a kért modellhez és a staginghez), és nem hoz létre könyvtárat a megadott gyökéren kívül.
- **Négy terület a `cfa4` névtérben** (békés együttélés: saját könyvtárak, nem a Hugging Face vagy más upstream program
  gyorsítótára, nincs közös port, konfiguráció vagy csomag): tartós store, staging és karantén a **választott gyökér alatt**
  (`<gyökér>/cfa4-models/`, `.staging/`, `.quarantine/`), így az előléptetés ugyanazon a fájlrendszeren egy átnevezés, nem
  másolás; a runtime cache `$XDG_CACHE_HOME/cfa4/models/` (ez ideiglenes, a felhasználó saját helye nem kell hozzá). A cache
  törlése a storet soha nem érinti. Ha egy tár meghajtója nincs csatolva, az abban lévő modellek `unavailable` jelzést kapnak,
  de a nyilvántartásból nem törlődnek, és a CFA4 nem tölti le őket máshová.
- **Beszerzés:** (A) hivatalos oldal megnyitása és a felhasználó saját letöltése, utána import; (B) kezelt letöltés a
  `cfa4-fetch`-csel, **csak a felhasználó kifejezett indítására**: staging, méret és SHA-256, a licenc megmutatása és
  egyszeri elfogadása, ellenőrzés, majd előléptetés a storeba. Automatikus letöltés **soha**: nem modulnyitáskor, nem
  ajánláskor, nem kompatibilitás-vizsgálatkor, nem katalógusfrissítéskor, és nem fejlesztés vagy teszt közben (a tesztek
  helyi, generált modellekkel futnak).
- **Javaslattétel hiányzó vagy már nem létező modellnél** (István, 2026-11-22): ha egy generátor vagy képesség modellje hiányzik, a CFA4
  **nem tölti le magától**, de **javaslatot tesz**: javaslatkártya a modellről (név, hivatalos forrás, méret, tárhelyigény,
  kompatibilitás, licenc), amelyen a letöltés vagy az import gombját a felhasználó nyomja meg. Ugyanígy javaslatot tesz, ha a
  szükséges modell **már nem létezik** (a forrás megszűnt, archivált vagy visszavont; lásd D9/a): ilyenkor a kártya azt mondja
  meg, hogy a modell nem érhető el, és **helyettesítő modelleket ajánl** ugyanahhoz a képességhez (először a már telepítettek, utána
  a katalógusból a kompatibilisek, kompatibilitás és méret szerint). A javaslat csak megjelenik; semmit nem indít, nem tölt le,
  és nem választ a felhasználó helyett.
- **Importlépések:** azonosítás, valódi formátum (a kiterjesztés nem elég; pickle nem fogadható el), metaadat, méret,
  hash, forrás és licenc, kompatibilitás, majd felvétel vagy karantén. Részleges, sérült vagy ellenőrizetlen modell nem
  aktiválható; a sikertelen letöltés a meglévő telepítést nem érinti.
- **Frissítés:** nem automatikus. A revízióellenőrzés csak jelez (`update_available`); új letöltéshez kifejezett
  jóváhagyás kell; a régi revízió megmarad, amíg a felhasználó nem törli; csendes felülírás nincs.
- **Eltávolítás két külön művelet:** „Eltávolítás a modulból” (csak a kapcsolat szűnik meg) és „Eltávolítás a
  CFA4-ből” (használat-, függőség-, futó-job- és megosztottságellenőrzés, megerősítés, biztonságos törlés).
  Megosztott modell nem törölhető úgy, hogy másik képesség működése váratlanul megszakad.
- **Zárolás:** a közös letöltés zárolása a feladatsor `SKIP LOCKED` bérleti mechanizmusával megy (PG-terv), nem új zárral.
- **Offline:** telepített modellek, adatlap, kompatibilitás, tárhely, eltávolítás, modulhozzárendelés, összehasonlítás,
  előzmények, import működik; katalógusfrissítés, felfedezés, revízióellenőrzés, forrásmegnyitás és kezelt letöltés
  online.

## 6. Kompatibilitás és az ACS

A Compatibility Engine az ACS-3 végrehajtási tervének (`cfa4-tasks::plan`) a modellre vetített nézete: a hardverfelmérés
(`accel`, NVML), a RAM/VRAM-költségvetés és a manifest `paths[]` adja a címkéket: *Fully compatible*,
*Compatible – reduced performance*, *CPU only*, *GPU recommended*, *Memory limited*, *Unknown*, *Incompatible*. A **12 GB
VRAM és 16 GB RAM alsó határ** (tulajdonosi döntés) érvényes marad: alatta nincs CPU-n futó AI-út. A CFA4 nem
választja automatikusan a legnagyobb modellt, és hiányzó modellt nem telepít; a hiányzó modellről a felület a hivatalos
forrást és a tárhelyigényt mutatja.

## 6a. llmfit: a modellkezelő kötelező funkciója (István, 2026-11-22)

**Döntés (István):** az [llmfit](https://github.com/AlexsJones/llmfit) a modellkezelő **kötelező funkciója**. Ez azt jelenti: a
Compatibility Engine minden nyelvi (LLM) modellre az llmfit ítéletét is megadja, és ez a funkció nem kapcsolható ki és nem
helyettesíthető.

**Amit az llmfit-ről ellenőriztem** (a `main` 26450cc, 2026-11-18, v1.1.20): licenc **MIT**; Rust; a `llmfit-core` önálló
könyvtár (hardverfelismerés, `ModelFit::analyze`, `estimate_model_plan`, `HardwareProfile`, szimulált hardver, tokenek/másodperc
becslés, kvantálás-választás: GGUF, AWQ, GPTQ, EXL2). Az adatbázisa **LLM-központú** (van kis ONNX-lista is, a videó-, hang- és
képmodelleket nem fedi). A magban `unsafe` csak tesztben és megjegyzésben van. **Hálózati felülete:** a `bench`, `providers`, `update`
és `share` modul HTTP-t használ (`ureq`), a `share` és a benchmark-megosztás eredményeket küld a projektnek, a `providers` helyi
szolgáltatókat (Ollama, llama.cpp stb.) kérdez. Nem vizsgáltam: a teljes forrást soronként, a `llmfit-web` és `llmfit-desktop` részt.

**Beépítés (D10, elfogadva: István, 2026-11-22 — „a CFA4 része legyen, ne külön alkalmazás”):** az llmfit a CFA4 része, nem külön program
vagy alkalmazás, ezért a 1. pont szerint könyvtárként épül be:
1. **Felhasználás könyvtárként** (`llmfit-core`, rögzített verzió, `cargo deny` zöld, a licenc a `docs/licensing.md` nyilvántartásában),
   nem külső programként és nem átírva. Így az ítélet ugyanaz, mint az upstreamé, és a frissítés egy verziólépés.
2. **Csak a számító részt hívjuk:** modelladatbázis, `ModelFit::analyze`, `estimate_model_plan`, `HardwareProfile`. A `bench`, `providers`,
   `update` és `share` modult a CFA4 **soha nem hívja**; ezt egy CI-teszt ellenőrzi (a tiltott modulok nem hivatkozottak), mert a
   hálózati és megosztó funkciók ütköznének az offline-first és a „semmi automatikus” szabállyal.
3. **Egyetlen hardverigazság:** a `SystemSpecs` bemenetet a CFA4 hardverfelmérése (`accel`, NVML, ACS) tölti, nem az llmfit saját
   felismerése; az llmfit saját felismerése csak összevetésre használható (eltérés esetén naplózott figyelmeztetés).
4. **Modelladatbázis frissítése:** csak a felhasználó kifejezett katalógusfrissítésére, a CFA4 saját letöltőjén (`cfa4-fetch`) át; az llmfit
   saját hálózati frissítője nem fut.
5. **Hatókör:** a kötelező ítélet a nyelvi modellekre (Text & LLM, Embeddings, ahol az llmfit ismeri) érvényes. Videó-, hang- és képmodellekre
   az ACS `paths[]` adja a kompatibilitást, az llmfit-oszlop „nem alkalmazható”. Ismeretlen modellnél az ítélet „ismeretlen”, nem kitalált.
6. **A CFA4 szabályai változatlanok:** a 12 GB VRAM és 16 GB RAM alsó határ érvényes akkor is, ha az llmfit kisebb gépen „elfér” ítéletet
   adna; az llmfit pontszáma nem választ modellt a felhasználó helyett, és hiányzó modellt nem telepít.
7. **Függőségi kockázat:** új külső függőség, amelyet a CFA4 nem ellenőriz; verzióváltás előtt a kimenetet ismert hardverprofilokon (golden)
   össze kell vetni, és a `cargo deny` zöld kell. Ha az llmfit megszűnik vagy a licence változik, a rögzített verzió marad használható.

Új elfogadási feltétel: **AC-27** minden LLM-modell adatlapján és kompatibilitás-nézetében látszik az llmfit ítélete (illeszkedés,
becsült memória, becsült sebesség) a CFA4 hardverfelméréséből; a tiltott hálózati modulok nem hivatkozottak; offline is működik.

## 6b. Biztonsági vizsgálat letöltéskor és importkor: modelscan (István, 2026-11-22)

**Kérés (István):** a modell letöltéséhez épüljön be a [ProtectAI modelscan](https://github.com/protectai/modelscan). (A megkapott link
csonkolt volt, „modelscan-t”; a `protectai/modelscan` projektre értelmeztem. Ha másra gondoltál, szólj.)

**Amit a modelscanről ellenőriztem** (`main` 61fcec9, 2026-02-18): licenc **Apache-2.0** (a GPL-3.0-or-later CFA4-gyel
összeegyeztethető, nyilvántartási sor kell a `docs/licensing.md` §5-be); **Python** (3.10–3.12), parancssori program; **statikusan elemzi** a
modellfájlt (nem tölti be), ezért maga a vizsgálat nem futtat modellkódot. **Amit vizsgál:** a Pickle-alapú formátumokat (PyTorch
`torch.save`, sklearn, XGBoost, joblib, dill, cloudpickle), a Keras H5 és az újabb Keras formátumot, és a TensorFlow SavedModel-t; ezek
tartalmazhatnak betöltéskor lefutó kódot. **Amit nem vizsgál:** a `safetensors`, ONNX és GGUF fájlokat (ezekre a program „nincs támogatott
fájl” kilépési kódot ad), ezért ezeknél a CFA4 saját formátum-ellenőrzése marad (7. pont, importlépések). A vizsgálat nem bizonyítja, hogy a
fájl ártalmatlan; csak ismert veszélyes műveleteket jelez, ezért a „nincs találat” címke nem jelent jóváhagyást. Utolsó upstream commit: 2026-02,
ezért a karbantartás üteme külön kockázat.

**Beépítés (javaslat, a döntés D11):**
1. **Az importfolyamat új lépése** a formátumfelismerés és a hash-ellenőrzés után, a **staging** területen, **a promóció előtt**:
   `scan`. Eredmény: `clean` (nincs találat), `issues` (súlyosság szerint: kritikus, magas, közepes, alacsony), `unsupported`
   (a formátumot a modelscan nem fedi), `error`. Az eredmény az ellenőrzési rekord része (`ai.verification`), a modell adatlapján látszik.
2. **Külön folyamatban fut**, a `cfa4-fetch`-hez hasonló zárt környezetben (nincs hálózat, csak a vizsgált fájl olvasható, nincs program
   indítás), rögzített verzióval és hash-sel. A CFA4 főfolyamatába nem kerül Python.
3. **A modelscan telepítése nem automatikus.** Egyszeri felhasználói jóváhagyás után telepíti a CFA4 egy külön, rögzített (uv) környezetbe,
   ugyanazzal a mechanizmussal, mint a többi választható komponenst (hash-ellenőrzött, a CFA4 generálta beállítás, javítóparancs); a
   jóváhagyás előtt a Pickle/H5/SavedModel fájlok importja **nem engedélyezett**, a többi formátumé igen.
4. **Kötelező, ahol alkalmazható:** minden Pickle-alapú, H5 és SavedModel fájl vizsgálaton megy át az aktiválás előtt; ellenőrizetlen ilyen
   fájl nem aktiválható. A `safetensors`, ONNX és GGUF fájlokra a CFA4 saját szerkezeti ellenőrzése (fejléc, méretek, ONNX külső adat útja) fut.
5. **Találat esetén:** a modell **karanténba** kerül, nem aktiválható; a felület megmutatja a találatot és a kockázatot (lásd D11).
6. **Mikor fut:** letöltés vagy import után, újra frissítéskor és a felhasználó kérésére; futás közben nem, és a CFA4 nem tölt le semmit vizsgálat
   céljából.
7. **Ami nem változik:** a licenc továbbra sem blokkol (D1); a vizsgálat biztonsági, nem jogi kapu.

Új elfogadási feltétel: **AC-28** Pickle-alapú fájl vizsgálat nélkül nem aktiválható; ártalmatlan és szándékosan veszélyes mintafájlon (generált, a
CFA4 nem tölt le valódi támadó mintát) a várt `clean` és `issues` eredmény, a találat karanténba visz; a vizsgálat hálózat nélkül fut; ha a
modelscan nincs telepítve, a Pickle/H5/SavedModel import tiltott, a többi működik.

## 7. Eltérések a már döntött szabályoktól (eldöntve 2026-11-22: a javaslatok érvényesek)

| # | A megbízás szövege | A döntött szabály | Javaslat |
|---|---|---|---|
| D1 | `BLOCKED` és `QUARANTINED` állapot, forrásmegbízhatóság, „licenc-figyelmeztetés hiányzó vagy bizonytalan licencnél” | 2026-11-07: a súly- és adatlicenc `NOASSERTION`/`unknown`/`no` értéke **soha nem blokkol**; a felület figyelmeztet, és modellenként egyszer elfogadást kér | Hash-hiba, sérült fájl, ismeretlen **forrás** blokkolhat/karanténba kerülhet; a **licenc** csak figyelmeztet és elfogadást kér. Az `unverified` forrás is telepíthető kifejezett jóváhagyással |
| D2 | „Commercial use status”, „redistribution” mezők az adatlapon | a CFA4 modellt nem terjeszt, a felhasználó felelőssége | Az adatlap a négy licencrészt és a forrást mutatja; a „kereskedelmi” mező az, amit a forrás állít, a CFA4 jogi értékelést nem ad |
| D3 | Külön „Download Queue” és saját letöltési motor | közös feladatsor (`cfa4-tasks`), `cfa4-fetch` letöltő | Nem készül második sor vagy letöltő (2. pont, 3–4. sor) |
| D4 | „Egyetlen modell, egyetlen fizikai példány”, több store | közös modelltár egy helyen | Több store megengedett, de egy modell egy hash-hez egy példány az egész rendszerben; hardlink/reflink engedélyezett, másolat nem |

További, kisebb nyitott pontok: **D5** a katalógusindex forrása és aláírása (nincs ma; nélküle a katalógus a
manifestekből áll); **D6** a meglévő Hugging Face-gyorsítótárból importál-e a CFA4 (javaslat: csak másolással vagy
hardlinkkel, a forrást nem módosítja); **D7** a modell-összehasonlítás és a kedvencek az F4-be vagy későbbre kerüljenek
(javaslat: F4, alacsony prioritás); **D8** a modellrekord állapotai (3.1) elfogadhatók-e.

## 8. Fázisok az ADR 0008 kapuihoz igazítva

A megbízás F0–F7 szakaszai nem új kapuk, hanem a meglévő **AI-F1** (modellkezelő és adattár) és **AI-F2** (jobok, ONNX
worker) tartalmának finomítása, plusz az ACS-5 és a Qt6 felület. Azonosító: **MM-F0…MM-F7** (az AI-F, ACS-, TR-F és PG-
előtagok mellett).

| Fázis | Tartalom | Kapu | Függ |
|---|---|---|---|
| MM-F0 | ez az audit | a terv leképezve, nincs párhuzamos alrendszer | kész, jóváhagyásra vár |
| MM-F1 | Registry és katalógus: azonosítók, revízió, artifact, forrás, licenc, képesség, adatlap | a katalógus és a nyilvántartás elválik; hibás bemenetre hiba, pánik nélkül | AI-F1, F0 kapu |
| MM-F2 | Store, staging, karantén, deduplikáció, tárhelykezelés | közös modell egyszeri példányban; a cache törlése nem érinti a storet | MM-F1, PG-3 |
| MM-F3 | Beszerzés és import: hivatalos forrás, kezelt letöltés (`cfa4-fetch`), feladatsor, megszakítás, újrapróbálás, folytatás ahol a forrás támogatja | biztonságos, felhasználó által indított beszerzés | MM-F2, ACS-4 |
| MM-F4 | Qt6 Modellkezelő: Katalógus, Letöltések, Telepítettek, Kompatibilitás, Tárhely, Előzmények, Beállítások | teljes kezelés a Qt6 felületről | MM-F3, Qt6 kész |
| MM-F5 | AI-modulok bekötése: Video, Audio, Media Intelligence, használatkövetés, modellválasztó API | minden modul a közös kezelőt használja | MM-F4, AI-F3+ |
| MM-F6 | Életciklus: frissítés, revízió, eltávolítás, függőségellenőrzés, kivonás | teljes életciklus | MM-F5 |
| MM-F7 | Validáció: integritás, import, megszakadt letöltés, helyreállítás, deduplikáció, egyidejű használat, offline, licenc, UI, hardver, **együttélés upstream programokkal** | végponttól végpontig igazolt | MM-F6 |

Minden fázis kilépési bizonyítéka: CI-napló, a futtatott tesztek kimenete, és (UI-nál) a GUI-automatizálás naplója.
Nem futtatott tesztet nem jelölök sikeresnek, és részleges megvalósítást nem nevezek késznek.

## 9. Az elfogadási feltételek (AC-01…AC-22) leképezése

| AC | Mivel igazolható | Fázis |
|---|---|---|
| AC-01, 02, 20 | registry- és katalógusteszt: azonosság, forrás, licenc, revízió, artifact nyomon követhető | MM-F1 |
| AC-03, 21 | teszt: modulnyitás, ajánlás, kompatibilitás-vizsgálat és katalógusfrissítés **nem** indít hálózati kérést (hálózatmentes tesztkörnyezet, a letöltő meghívása számlálva) | MM-F3 |
| AC-26 | hiányzó modellnél javaslatkártya jelenik meg, hálózati kérés és letöltés nélkül; megszűnt modellnél helyettesítő modelleket ajánl (a telepítettek elöl); a javaslat nem indít semmit | MM-F3/F5 |
| AC-04, 07, 08 | importteszt generált kis modellekkel: jó, hibás hash, részleges, hamis formátum; ellenőrzés nélkül aktiválás elutasítva | MM-F3 |
| AC-28 | vizsgálati teszt generált ártalmatlan és szándékosan veszélyes mintafájlokon (nincs valódi támadó minta, nincs letöltés); zárt, hálózat nélküli futás; hiányzó modelscannél a Pickle-import tiltott | MM-F3 |
| AC-05, 06 | kezelt letöltés generált helyi HTTPS-forrással: megszakítás, újrapróbálás, folytatás | MM-F3 |
| AC-09, 22 | kompatibilitási címkék ismert hardverprofilokra, az ACS tervezőjével | MM-F1/F5 |
| AC-27 | llmfit-ítélet LLM-modellekre ismert hardverprofilokon (golden), offline futás, CI-teszt: a `bench`/`providers`/`update`/`share` modul nem hivatkozott | MM-F1/F4 |
| AC-10, 11, 17–19 | két képesség ugyanazt a modellt használja egy példányból; hash szerinti deduplikáció | MM-F2/F5 |
| AC-12, 13, 24 | frissítési és törlési teszt: csendes felülírás nincs, megosztott modell nem törölhető, az inkompatibilis modul a régi revízión marad | MM-F6 |
| AC-23 | forrásállapot-teszt: megszűnt modellnél megtartás vagy eltávolítás választható, automatikus törlés nincs | MM-F6 |
| AC-14 | offline teszt: telepített modellek kezelése hálózat nélkül | MM-F7 |
| AC-15 | cache törlése után a store változatlan | MM-F2 |
| AC-25 | a tár helyét a felhasználó adja meg: választás nélkül nincs letöltés és import; meghajtónként szabad hely látszik; elégtelen helyet a CFA4 elutasít; több tár; az áthelyezés csak kifejezett paranccsal | MM-F2 |
| AC-16 | Qt6 GUI-teszt (`python/tests/`), képernyőkép a referenciagépen | MM-F4 |

A licenc- és hardverfüggő tesztek a `run-minspec.sh` alatt futnak, a referenciakártyán; valós letöltést a fejlesztés nem végez.

## 10. Hatókörön kívül (megerősítve)

Automatikus letöltés, telepítés, frissítés vagy licencelfogadás; háttérletöltés beleegyezés nélkül; modulonkénti
letöltő vagy modelltár; kötelező felhő vagy fiók; új adatbázis-motor; Windows, macOS, Android, ARM; az eredeti
FilmCraft projekt módosítása; aktiválás ellenőrzés nélkül; jóváhagyás nélküli tárhely-áthelyezés.

## 11. Mit nem tudtam ellenőrizni

1. A CFA3 repóból csak az AI Video Tools terv 6–8. és 20.3 pontját és az integrációs összefoglalót olvastam; a repó többi részét (Model Router, Host Resource Broker, a canonical nyilvántartások) nem. A CFA3-ban nincs modellkezelő kód, csak a terv.
2. A CFA4 könyvtárpolitikája modellre: a repóban csak más alkalmazásokra vonatkozó XDG-szabályokat találtam.
3. A `cfa4-fetch` folytatási képessége (Range kérés): a kódban nem találtam; a „folytatás ahol támogatott” ezért új
   munka a letöltőben, nem meglévő képesség.
4. Nem futtattam semmit, nincs kód- vagy tesztváltozás.
