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
| ACS-4 | Feladatsor és ütemezett végrehajtás: központi sor, P0/P1/P2/R osztály és felhasználói prioritás, négy indítási mód (azonnal, erőforrásra, időpontra, időpontra és erőforrásra), `scheduled` és `paused` állapot, folytatás | Újraindítás után sem vész el feladat, és egyik sem indul kétszer (PG-2: bérlet, `SKIP LOCKED`) | Kész (7. pont): a sor, a PostgreSQL-tár, az ébresztés, a kimenőtár-továbbító és az időzónás következő időpont |
| ACS-5 | Értesítés és Qt 6 feladatkezelő: előzetes tájékoztatás, becsült idő, késés, döntési lehetőség | A felhasználó indítás előtt és közben is megfelelő információt kap | Blokkolva: a Qt 6 felület még készül, és a CFA4 főágában még nincsenek ACS-crate-ek. Addig a `tasks.list` és a `waitingFor` adja ugyanezt az információt |
| ACS-6 | Modulok bekötése: AI-videó és -hang, Media Intelligence, render és kodekek (az `export` jobja kliens lesz, nem cserélődik le) | A modulok a közös infrastruktúrát használják | Folyamatban: az exportsor és a motor háttérfeladatai már a közös soron mennek (8. pont) |
| ACS-7 | Integrációs és stabilitási próbák: hardverkonfigurációk, háttérterhelés, hosszú feladatok, újraindítás, memóriahiány, eredményhelyesség (CPU és GPU arany kimenettel) | A funkciók elérhetők maradnak, ahol van biztonságos út | Folyamatban: a sor stabilitási próbája kész, két hibát talált és javított (9. pont) |

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

## 7. ACS-4: feladatsor és PostgreSQL-tár

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

**PostgreSQL-tár (`cfa4-store`, PG-1 és PG-2).** A meghajtó a `tokio-postgres` a
`deadpool-postgres` kapcsolatkészlettel (tulajdonosi döntés, 2026-11-11; a BacklotHubbal közös).
A crate L5-ös, mert egy másik folyamattal beszél socketen át, mint a `cfa4-platform`. A terv
L2-t írt, de az L0–L4 WASM-ellenőrzés nem tudná lefordítani.

- **Kapcsolat:** hiányzó vagy leállt szerverre kezelhető hibát ad, a telepítés módjával együtt
  (`Unavailable`). A vágás, a lejátszás és a mentés nem függ tőle.
- **Migráció:** komponensenként beágyazott lépésekből áll, mindegyiknek van visszafelé iránya. Egy
  lépés egy tranzakció, tanácsadó zárral; a nyilvántartás a `cfa4_store.migration` tábla.
- **`tasks` séma:** `task` (indexelt oszlopok és a teljes feladat `jsonb`-ben), `transition`
  (csak hozzáfűzhető napló) és `outbox` (Event Fabric, `cfa4.execution.job.<állapot>`). Az időket
  UTC ezredmásodpercben tárolja, a `cfa4_tasks::UtcMs`-sel egyezően. A terv itt `timestamptz`-t
  írt, ettől eltértem.
- **Egy művelet egy tranzakció:**
  1. zárolja a sorokat (`FOR UPDATE`, az ütemezőlépés `SKIP LOCKED`-del);
  2. lefuttatja rajtuk a sor saját szabályát;
  3. visszaírja a megváltozott feladatokat (verzióra feltételesen), a naplót és a kimenő
     eseményeket;
  4. véglegesít.

  Az erőforrás-kezelő egy másolaton változik, és csak a véglegesítés után lép a hívó helyére.
  Ha a véglegesítés elmarad, semmi nem történt.
- **Tesztek:** eldobható klaszteren futnak (`initdb` és `pg_ctl` ideiglenes könyvtárban, csak
  Unix socketen). Lefedik a migrációt oda-vissza, a két párhuzamos ütemezőt (a feladat pontosan
  egyszer indul), az újraindítás utáni folytatást az ellenőrzőponttól, a verzióellenőrzést, a
  sérült sort (hiba, nem pánik) és a hiányzó szervert.

**Ébresztés (`Store::listen`).** Saját kapcsolat, amely `LISTEN cfa4_tasks`-ra figyel.
- **Ki értesít:** minden beküldés és minden változás, amely más feladatot indíthat (megszakítás,
  folytatás, befejezés). Az értesítés ugyanabban a tranzakcióban megy el, ezért csak a
  véglegesítés után érkezik meg; a visszagörgetett változás nem ébreszt.
- **`Listener::wait(timeout)`:** a megváltozott feladatok azonosítóit adja, vagy `Timeout`-ot,
  vagy `Lost`-ot (új kapcsolat kell).
- **Az időkorlát marad:** a bérletek, az időzített indítások és a CFA4-en kívül felszabaduló
  erőforrások miatt.

**Kimenőtár-továbbító (`outbox::Relay`).**
- **Sorrend és kizárás:** a még el nem küldött `cfa4.execution.job.*` eseményeket a kimenőtár
  sorrendjében adja át a küldőnek. A `FOR UPDATE SKIP LOCKED` miatt két továbbító nem küldi
  ugyanazt egyszerre.
- **Jelölés:** csak azt jelöli elküldöttnek, amit a küldő visszaigazolt.
- **Legalább egyszeri kézbesítés:** ha a CFA4 a küldés és a véglegesítés között áll le, az
  esemény újra kimegy. A fogyasztó az `id` alapján szűri az ismétlést.
- **Az Event Fabric kliense** (NATS JetStream) a küldő dolga, a crate nem függ tőle.

**Következő helyi időpont (`schedule`, `jiff`).** A feladat megadhat egy IANA-időzónát
(`timeZone`, például `Europe/Budapest`); beküldéskor ellenőrizzük.
- **Új időpont:** a `nextSlot` szabályú, elmaradt indítás magától átkerül ugyanarra a helyi
  időpontra azon az első napon, amikor az még előttünk van. Időzóna nélkül UTC szerint számol.
- **Állapot és esemény:** a feladat `scheduled` marad, a `Missed` esemény `nextAt` mezője
  hordozza az új időt.
- **Óraátállítás:** a helyi idő megmarad. Ha az adott idő aznap nem létezik (előreállítás), a
  feladat a kihagyott óra utáni első pillanatban indul; ha kétszer létezik (visszaállítás), az
  elsőnél.
- **Ismeretlen időzóna:** a feladat szünetel, a felhasználó dönt, nincs találgatás.
- **Az időzóna-adatbázis** a rendszeré, tartaléknak beépített másolattal.
- **Új függőség:** `jiff` (Unlicense OR MIT, tiszta Rust).

## 8. ACS-6: modulok bekötése

**Az első kliens az exportsor.** Az `engine` crate `task_host` modulja (`TaskHost`) a munkamenet
saját `cfa4_tasks::Queue`-ja és erőforrás-kezelője. Az exportsor sorrendje és elemei maradnak,
ahogy a felhasználó megadta (a terv szerint az `export` jobja kliens lesz, nem cserélődik le).

- **Indítás:** a sor elején álló elem feladatot kér (`FinalRender` osztály), és csak akkor indul,
  ha a sor szabályai és a gép kerete engedik. Addig `ready` marad, és a `waitingFor` mezőben
  olvasható az ok, például hogy más programok használják a memóriát.
- **Erőforrás-kérés:** nyolc képkocka a kimeneti méretben teljes pontossággal, plusz 256 MiB és
  legfeljebb négy CPU.
- **Befejezés:** ha az export véget ér (kész, hiba vagy megszakítás), a foglalás felszabadul.
  Felszabadul akkor is, ha az elem helyet veszít a sorban, törlik, vagy a sor leáll.
- **A médiamotor mindig működik (VISION 2.4).** Egy médiafeladat ezért itt sosem lesz „nem
  végrehajtható”:
  - a kérést a keretre vágjuk;
  - amit nem lehet mérni (szabad RAM, CPU-szám), azt nem kéri;
  - várhat helyre, de soha nem vár örökké egy olvashatatlan számra.

  Az AI-feladatok a tervezőn át kérnek (ACS-3), ott az ismeretlen soha nem „elég”.
- **Blokkoló export:** a parancssori vagy szkriptből indított (`wait`) export azonnal indul,
  mert a hívó most kérte.
- **Vezérlés:** a `tasks.list` parancs mutatja a feladatokat, az állapotukat, a kérésüket és a
  várakozás okát.
- **Tárolás:** a sor memóriában van, mert a vágás, a lejátszás és a mentés nem függ az
  adatbázistól. A `cfa4-store` bekötése az alkalmazásban következik, a 7. pont hátralévő
  részével együtt.

**A többi háttérfeladat (`engine::background`).** Ugyanígy kérnek erőforrást, mielőtt a
száluk elindul:

| Feladat | Kérés |
|---|---|
| Proxy, ingest és átkódolás | 1 GiB, 2 CPU |
| Projektkezelő | 1 GiB, 2 CPU |
| Előnézet-renderelés (videó) | 1 GiB, 2 CPU |
| Hang-előnézet | 256 MiB, 1 CPU |
| Jelenetfelismerés | 512 MiB, 1 CPU |
| Maszkkövetés | 512 MiB, 1 CPU |
| Narráció | 512 MiB, 1 CPU |

Mindegyik `Background` (P1) osztályú, így az export (R) megelőzi őket.

- **Várakozás:** a feladat a kéréstől kezdve látszik a `jobs.list`-ben. Ha nincs hely, az
  állapotsora „Waiting: …” az okkal, és a képkockánkénti munkában (`poll_persistence`) indul el,
  amint elfér.
- **Megszakítás várakozás közben:** a feladat azonnal véget ér, el sem indul.
- **Befejezés:** amikor a szál befejezi, a foglalás felszabadul.
- **Kivétel:** a blokkoló és a wasm32-es futás most is azonnal indul.

**Következik az ACS-6-ban:**
- a `cfa4-store` bekötése az alkalmazásba, az AI-workerekkel együtt (a motor feladatai újraindítás
  után nem folytathatók, a vágás pedig nem függhet az adatbázistól);
- az AI-modulok, amikor elkészülnek (a Qt 6 után).

## 9. ACS-7: stabilitási próbák

**A sor véletlenszerű terhelése (`crates/tasks/tests/stability.rs`).**
- Nyolc rögzített kiinduló értékkel fut, mindegyikkel 5000 lépést tesz.
- A lépések véletlenszerűek:
  - beküldés, köztük olyan kérés is, amely sosem fér el;
  - ütemezőlépés két felügyelővel;
  - bérlet megújítása és ellenőrzőpont;
  - befejezés és megszakítás;
  - szüneteltetés, folytatás, átütemezés.
- A műveletek gyakran rossz tulajdonossal vagy elavult verzióval jönnek.
- Közben a gép változik:
  - a szabad memória és a VRAM ingadozik, néha olvashatatlan;
  - időnként memórianyomás lép fel;
  - az idő néha annyit ugrik, hogy minden bérlet lejár, mintha a CFA4 leállt volna.

Minden lépés után ellenőrzi:
- a foglalások soha nem lépik túl a keretet (RAM, CPU, minden GPU VRAM-ja);
- foglalást pontosan a futó és a leálló feladatok tartanak, mindegyik egyet;
- nem folytatható feladat soha nem indul kétszer;
- semmi nem pánikol.

**Amit talált, és a javítás (a `cfa4-store` ugyanazt a szabályt használja, így ott is javult):**
- **Kétszeri megszakítás:** egy leálló (`cancelling`) feladat második megszakítása eddig rögtön
  `cancelled`-be tette, a foglalása pedig örökre megmaradt. Most nem változtat semmit, a feladatot
  a munkása zárja le.
- **Futó feladat folytatása:** egy futó feladatra kért „folytatás” eddig visszatette a sorba a
  foglalásával együtt, így kétszer indulhatott. Most csak szüneteltetett feladat folytatható.
- **Halott munkás leállás közben:** ha egy leálló feladat munkása meghal, a bérlet lejártával az
  ütemező `cancelled`-re állítja, és felszabadítja az erőforrásait. A PostgreSQL-tár ütemezője
  ezért a `cancelling` sorokat is betölti. A 2. migráció a részleges indexet erre bővíti, és
  visszafelé is lefut.

**Valódi gépen: memóriaterhelés (`crates/tasks/tests/machine.rs`).**
- **Mit csinál:** magától lefoglalja a szabad memória legfeljebb 60%-át, minden lapját írva.
- **Mit ellenőriz:**
  - a hatásos RAM a cgroup-korlát;
  - a szabad memória a terheléssel csökken;
  - a már el nem férő kérés várakozik, nem indul;
  - a felszabadítás után minden visszaáll.
- **Futtatás:** alapból ki van hagyva (`#[ignore]`). Ebben a környezetben (16 GB-os korlát) zöld:
  9 GB foglalásnál a szabad memória 15,3 GB-ról 6,1 GB-ra csökkent, és a kérés várakozott.

**A referenciagépen (`scripts/dev/acs7-stability.sh`).** A `run-minspec.sh` két profiljában
(minimum: 16 GB, ajánlott: 32 GB, swap nélkül, 8 mag, a referencia-GPU) futtatja:
- a sor hosszú üzemű véletlenszerű próbáját (`CFA4_ACS7_STEPS`, alapból 200 000 lépés kiinduló
  értékenként);
- a memóriaterhelési próbát, a cgroup-korlát ellenőrzésével;
- a PostgreSQL-tár tesztjeit;
- a motor háttérfeladat-tesztjeit.

A napló a `target/acs7/` könyvtárba kerül. A tulajdonos gépén kell lefuttatni, mert a felhőben
nincs referencia-GPU.

**Még hátravan:** az eredményhelyesség CPU-n és GPU-n, arany kimenettel. Ez az AI-workerekkel
jön, mert most nincs GPU-s feladat, amelyet ellenőrizni lehetne.
