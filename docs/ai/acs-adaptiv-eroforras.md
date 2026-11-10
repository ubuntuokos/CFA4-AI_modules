# Adaptív erőforrás-kezelés (ACS): terv és fázisok

> **Állapot:** elfogadott irány; a forrás István CFA4-ADAPTIVE-CAPABILITY-001 terve (2026-11-07),
> a tulajdonosi döntésekkel (2026-11-07, [ADR 0008](../adr/ADR-0008-ai-processing-layer.md)
> „Lezárt kérdések”) · **ACS-0:** ez a dokumentum · **ACS-1:** kész (`cfa4-tasks`, `cfa4-accel`
> `memory`) · **ACS-2:** kész (`budget`, `manager`) · **ACS-3:** kész (`plan`, `cfa4-platform` `nvml`) · **Utolsó átnézés:** 2026-11-10

## 1. Mi ez

Az ACS a CFA4 egészének közös feladat- és erőforrás-infrastruktúrája, nem AI-modul. Minden
erőforrásigényes modul (AI, render, proxy, kodekkonverzió, Media Intelligence) ezt használja, és
egyik sem épít saját, vele versengő erőforrás-kezelőt. Helye a `cfa4-tasks` crate (L2, felület
nélkül); a hardverfelmérés GPU-s része a meglévő `cfa4-accel` crate bővítése. A `cfa4-ai-jobs`
ennek kliense lesz, és csak az AI-specifikus részt tartja meg (Elemzés → Javaslat → Előnézet →
Alkalmazás, modell-előkészítés, licencelfogadás).

## 2. Alapelvek a tulajdonosi döntések szerint

- **Elérhetőség:** egy funkció elérhető, ha van validált végrehajtási útja a 12 GB VRAM / 16 GB
  RAM alsó határ felett (SDK manifest 5. változat, `paths`). Ha nincs, „jelenleg nem
  végrehajtható”, okkal. Az ismeretlen szám soha nem jelent „elég”-et.
- **Alsó határ:** AI-hoz legalább egy 12 GB-os GPU, amely nem hajt kijelzőt, és legalább 16 GB
  RAM (ajánlott 32 GB). GPU nélküli AI-út nincs; a CPU-s tartalék (VISION 2.4) a médiamotorra
  vonatkozik.
- **OOM-létra két részben:** automatikus csak az eredményt nem változtató lépés (kisebb köteg,
  csempézés átfedéssel, szakaszos futás, várakozás, illetve a manifestben `automatic: true`
  tűrésű út); minden más jóváhagyást kér. Nincs néma minőségcsökkentés.
- **Tartós állapot:** a közös CFA4 PostgreSQL-klaszter `tasks` sémája
  ([PostgreSQL-adatréteg](postgresql-adatreteg.md), PG-2). Új adatbázismotor nincs.
- **Kimi K3:** a fogalmait (memóriakeret, rétegenkénti streaming, gyűrűpuffer, direkt I/O,
  telemetria) Rustban írjuk meg; C-kód a CFA4 folyamatába nem kerül.
- **Ütemezés:** az ACS-1…ACS-4 a Qt 6 átállás alatt is mehet, mert nem AI és nem felület; az
  ACS-5 (felület) az átállás után.

## 3. Fázisok

| Fázis | Tartalom | Elfogadási feltétel | Állapot |
|---|---|---|---|
| ACS-0 | Architektúra-ellenőrzés: az AI-terv, a modellkezelő, a PostgreSQL, az Event Fabric és a meglévő erőforrás-megoldások összevetése, felelősségi határok | Nincs párhuzamos, versengő architektúra | Kész: összevetés 2026-11-07, döntések az ADR 0008-ban, ez a dokumentum |
| ACS-1 | Hardverfelmérés és futásidejű erőforrás-figyelés | A rendszer a névleges és az aktuális erőforrásokat külön kezeli | Kész (4. pont) |
| ACS-2 | Erőforrás-elérhetőség kezelése: dinamikus keretek, tartalékok, CPU/GPU-korlátok, párhuzamos feladatok összehangolása, atomi foglalás | A CFA4 nem veszélyezteti a gazdarendszer stabilitását | Kész (5. pont) |
| ACS-3 | Adaptív végrehajtási terv: alternatív utak (manifest v5 `paths`), szakaszos futás, streaming, részleges modellbetöltés, időbecslés tartománnyal | Egy funkcióhoz több validált út érhető el, ahol a technológia engedi | Kész (6. pont) |
| ACS-4 | Feladatsor és ütemezett végrehajtás: központi sor, P0/P1/P2/R osztály és felhasználói prioritás, négy indítási mód (azonnal, erőforrásra, időpontra, időpontra és erőforrásra), `scheduled` és `paused` állapot, folytatás | Újraindítás után sem vész el feladat, és egyik sem indul kétszer (PG-2: bérlet, `SKIP LOCKED`) | Folyamatban: a sor magja kész (7. pont), a PostgreSQL-tár hátravan |
| ACS-5 | Értesítés és Qt 6 feladatkezelő: előzetes tájékoztatás, becsült idő, késés, döntési lehetőség | A felhasználó indítás előtt és közben is megfelelő információt kap | Qt 6 után |
| ACS-6 | Modulok bekötése: AI-videó és -hang, Media Intelligence, render és kodekek (az `export` jobja kliens lesz, nem cserélődik le) | A modulok a közös infrastruktúrát használják | |
| ACS-7 | Integrációs és stabilitási próbák: hardverkonfigurációk, háttérterhelés, hosszú feladatok, újraindítás, memóriahiány, eredményhelyesség (CPU és GPU arany kimenettel) | A funkciók elérhetők maradnak, ahol van biztonságos út | |

## 4. ACS-1: ami elkészült

**`cfa4-accel` `memory` modul.** GPU-nként a sysfs-ből: VRAM összesen és foglalt
(`mem_info_vram_*`, amdgpu), terhelés (`gpu_busy_percent`), és hogy kapcsolt kijelző lóg-e rajta
(`cardN-*/status`). Az NVIDIA zárt meghajtója és az Intel nem ad VRAM-számlálót a sysfs-ben: ott
az érték „ismeretlen” marad (nem nulla), amíg egy külső mérés (NVML a `cfa4-platform`-ban, vagy a
worker jelentése) ki nem tölti.

**`cfa4-tasks` crate.**

- `Nominal` (névleges): fizikai RAM, a folyamat és őseinek cgroup-korlátja (`memory.max`,
  `cpu.max`, a legszigorúbb számít; az olvashatatlan `memory.max` „ismeretlen korlát”, nem
  „nincs korlát”), online és a folyamat számára engedélyezett CPU-k (affinitás, `cpuset`), GPU-k VRAM-ja és kijelzőhasználata. A
  `ram_effective_bytes` a fizikai RAM és a cgroup-korlát közül a kisebbet adja, így a
  `run-minspec.sh` 16 és 32 GB-os profilja helyesen látszik az 512 GB-os fejlesztőgépen is.
- `Current` (aktuális): `MemAvailable`, a cgroup `memory.current`-je, foglalt swap, PSI-nyomás
  (memória, CPU, I/O, 10 másodperces átlag), foglalt VRAM és GPU-terhelés.
- `class`: RAM-profil (minimum alatt, minimum, ajánlott) és az AI alsó határa
  (`Met` / `NotMet` okokkal / `Unknown` a hiányzó adatokkal).
- `Monitor`: a névleges változásokat (GPU megjelent vagy eltűnt, korlát vagy VRAM változott)
  egyszer jelenti; jelenként (memória-, CPU-, I/O-nyomás, kevés memória, GPU-nkénti VRAM)
  hiszterézissel megkülönbözteti a rövid kiugrást (`Spike`) a tartós korláttól (`Sustained`).
  Tiszta állapotgép: a mintavétel ütemét az ACS-2 erőforrás-kezelője adja.

Minden olvasás korlátos, minden szám ellenőrzött; hiányzó vagy ellenséges fájl „ismeretlen”,
soha nem nulla és soha nem pánik. Minden adat serde, így a vezérlőcsatorna és az MCP olvashatja.

## 5. ACS-2: ami elkészült

**Keret (`budget`).** A névleges értékekből és a felhasználó beállításaiból
(`BudgetSettings`, serde) számol:

- RAM: a hatásos RAM mínusz tartalék (alapból 2 GiB és a 10 % közül a nagyobb), a felhasználó
  ezt felső korláttal szűkítheti;
- CPU: a hatásos CPU-k mínusz tartalék (alapból egy CPU marad az asztalnak);
- GPU-nként a VRAM mínusz tartalék (alapból 512 MiB).

Nem kap keretet az a GPU, amelynek VRAM-ja ismeretlen, amelyet a felhasználó nem választott ki,
vagy amely kijelzőt hajt (vagy nem tudni, hajt-e), miközben van biztosan kijelző nélküli alkalmas kártya. Ha egyetlen kártya van, és az
kijelzőt hajt, akkor csak a felhasználó engedélyével (`allowDisplayGpu`). Minden kizárásnak oka
van.

**Foglalás (`manager`).** A `try_reserve` egy feladat RAM-, CPU- és VRAM-igényét **egyszerre
adja meg, vagy semmit** (nincs „tartok és várok”, nincs holtpont). Négy ellenőrzés:

1. A kereten túli igény `NeverFits`: így ezen a gépen nem fut, a tervező (ACS-3) másik validált
   utat választ. Ismeretlen keretre semmi nem fér.
2. A többi foglalás és az új igény együtt sem lépheti túl a keretet.
3. A pillanatnyi szabad memória és VRAM (a tartalék nélkül) fedezze az igényt, mert más programok
   is használják. cgroup-korlátnál a korlát és a cgroupok saját `memory.current`-je közti rés is
   számít; ha a pillanatnyi szabad érték nem olvasható, a feladat vár (`unknown`).
   Egy friss foglalás még nem látszik a terhelésben (a feladat még foglal), ezért három
   mintavételig (`observe_sample`) a pillanatnyi szabad értékből is levonódik: két kérés ugyanarra
   a pillanatképre nem osztozhat ugyanazon a helyen.
4. Nyomás osztályonként:
   - memórianyomásnál vagy kevés memóriánál (kiugrás vagy tartós) csak az `Interactive` (P0)
     indul, mert a felhasználó vár rá;
   - tartós CPU- vagy I/O-nyomásnál a `Speculative` (P2) vár, a többi csak lassabb lesz;
   - tartósan tele VRAM-ú GPU új munkát nem kap.

Ha valami nem teljesül, `Wait` jön, minden okkal; az újrapróbálást a sor (ACS-4) végzi. A GPU-t a
kezelő választja: amelyiken a legtöbb valódi hely van (a keret és a pillanatnyi szabad VRAM közül a
kisebb). Ha a foglalási azonosítók elfogynának, `IdsExhausted` jön, ismételt azonosító soha.

## 6. ACS-3: ami elkészült

**Tervező (`plan`).** A `plan(job, paths, manager, snapshot, monitor)` egy feladat igazolt útjait
(SDK manifest 5. változat, `paths`) veszi sorra. A számokat a kliens (`cfa4-ai-jobs`, render,
proxy) adja meg a konkrét feladatra: a VRAM-ot a feladat felbontásához, és a munkaegységek számát
(képkocka, hangmásodperc). Így a crate AI- és manifestfüggetlen marad.

- **Sorrend, a kétrészes OOM-létra szerint:** előbb a referencia-út, utána az `automatic: true`
  tűrésű utak, végül az eredményt megváltoztatók, a manifest sorrendjében. Az utóbbiakat a tervező
  csak felajánlja, a felhasználó hagyja jóvá; néma minőségcsökkentés nincs.
- **Döntés:**
  - `Run`: azonnal fut. A `switched` jelzi, ha nem a referencia-út fut; erről a felhasználó
    tájékoztatást kap.
  - `Wait`: az első, valaha is elférő automatikus út most nem fér el. Az `approvalNow` megnevezi
    azt az eredményt megváltoztató utat, amely azonnal futhatna, ha a felhasználó a várakozás
    helyett azt választja.
  - `NeedsApproval`: csak eredményt megváltoztató út fér el; a tűrésével együtt ajánlja fel.
  - `Unavailable`: „jelenleg nem végrehajtható”; minden út értékelése okkal együtt megmarad.
  - `InvalidPaths`: hibás útlista (nincs út, nem pontosan egy referencia-út van, tűrés hiányzik
    vagy hibás, ismétlődő azonosító).
- **Ismeretlen igény:** ha egy út nem adja meg a RAM- vagy a VRAM-igényét, nem tervezhető
  (`NotStated`); az ismeretlen soha nem elég.
- **Eszköz:** a futtató (`cuda`, `tensorrt`: NVIDIA; `rocm`, `migraphx`: AMD; `openvino`: Intel;
  `cpu`) csak a megfelelő gyártó kártyáján kap helyet (`GpuRequest.vendor`). A foglalás nélküli
  ellenőrzést a `Manager::check` végzi.
- **Időbecslés:** mindig tartomány (`minS`, `maxS`), bizonyossággal együtt.
  - Ha az utat ugyanazzal a `run-minspec.sh`-profillal mérték, mint amilyen a gép:
    `Medium` bizonyosság, az alapidő 0,8–1,5-szerese.
  - Ha más profillal mérték, vagy a profil nem ismert: `Low` bizonyosság, az alapidő 0,5–3-szorosa.
  - Tartós CPU- vagy I/O-nyomásnál a felső határ megduplázódik.
  - Ellenséges számra (nulla, negatív vagy végtelen sebesség) nincs becslés.

**NVML-olvasó (`cfa4-platform` `nvml`).** Az NVIDIA zárt meghajtója nem ad VRAM-számlálót a
sysfs-ben, ezért ez a modul a `libnvidia-ml.so.1`-ből olvassa ki a VRAM-ot, futásidőben
betöltve: ha a meghajtó hiányzik, egyszerűen nincs jelentés, nem hiba. A leolvasott értékeket a
PCI-cím köti a renderelőcsomóponthoz (`cfa4-accel` `reports_by_pci`), az eredmény pedig
`GpuMemoryReport` lesz, amelyet a hívó `Snapshot::apply_gpu_report`-tal alkalmaz. A sysfs saját
számlálója mindig elsőbbséget kap.

## 7. ACS-4: a sor magja (folyamatban)

**Sor (`queue`).** A közös CFA4-feladatsor szabályai tár nélkül. Ezek a szabályok kötelezők minden
tárra: a PostgreSQL-tár (`tasks` séma, PG-2, a `cfa4-store`-on át) ugyanezeket a műveleteket
tranzakcióban végzi (`FOR UPDATE SKIP LOCKED`, bérlet, verziószám), és ugyanezeket a teszteket
kell teljesítenie. Az időt a hívó adja meg (UTC ezredmásodperc), az ütemező maga nem olvas órát.

- **Állapotok** (SDK gazdajelentés, 2. változat): `scheduled`, `queued`, `waiting` (erőforrásra),
  `blocked` (egyik út sem fér el: a kliens újratervez, vagy a felhasználó másik utat hagy jóvá),
  `running`, `paused`, `cancelling`, `succeeded`, `failed`, `cancelled`. Az AI-alszakaszok
  (`preparing_model`, `loading`) a `cfa4-ai-jobs`-ban maradnak. Az állapotváltást az átmenettábla
  ellenőrzi, a napló legfeljebb 256 sort tart meg.
- **Sorrend:**
  - Elsőként az osztály számít: P0 (interaktív), R (végső render), P1 (háttér), P2 (spekulatív).
    Az R a P1 elé kerül, mert a felhasználó kifejezetten kiadásra indította; ezt én döntöttem
    el, az ADR nem rögzíti.
  - Osztályon belül a felhasználói prioritás, azon belül a beküldés ideje dönt.
  - Ha egy feladat erőforrásra vár, az alatta lévő elindulhat helyette, de csak amíg a várakozó
    10 percnél rövidebb ideje vár (`starvationMs`). Utána ő következik, így a nagy feladat nem
    éhezik ki.
- **Négy indítási mód:** azonnal, erőforrásra, időpontra, időpontra és erőforrásra. Ha az
  „azonnal” vagy az „időpontra” módú feladatnak várnia kell, a felhasználó értesítést kap; a másik
  kettő csendben vár.
- **Egyszeri indítás:**
  - A futó feladat bérletét a felügyelő újítja meg (alapból 30 másodpercenként).
  - Ha a bérlet lejár, a folytatható feladat az ellenőrzőpontjától kerül vissza a sorba, minden
    más `failed` lesz. Elejéről magától soha nem indul újra.
- **Verziószám:** a felhasználói művelet (szüneteltetés, folytatás, megszakítás, átütemezés, új
  út) a látott verzióra feltételes. Az elavult kérést a sor elutasítja.
- **Elmaradt indítás:** ha a késés több mint 5 perc, a feladat saját szabálya dönt:
  - `runLate`: elindul késve;
  - `nextSlot`: a hívó a feladat IANA-időzónájában kiszámolja a következő időpontot
    (`reschedule`);
  - `askUser`: a felhasználó dönt.
- **Ellenséges bemenet:** üres vagy túl hosszú azonosító, legfeljebb 100 000 feladat, szaturáló
  idők; hibát ad, pánikot nem.

**Hátralévő rész:** a PostgreSQL-tár (PG-1 `cfa4-store` és PG-2 `tasks` séma), a meghajtó
kiválasztása után. A meghajtót a BacklotHub G2 kapujával közösen kell kiválasztani.
