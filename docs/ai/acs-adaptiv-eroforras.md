# Adaptív erőforrás-kezelés (ACS): terv és fázisok

> **Állapot:** elfogadott irány; a forrás István CFA4-ADAPTIVE-CAPABILITY-001 terve (2026-11-07),
> a tulajdonosi döntésekkel (2026-11-07, [ADR 0008](../adr/ADR-0008-ai-processing-layer.md)
> „Lezárt kérdések”) · **ACS-0:** ez a dokumentum · **ACS-1:** kész (`cfa4-tasks`, `cfa4-accel`
> `memory`) · **ACS-2:** kész (`budget`, `manager`) · **Utolsó átnézés:** 2026-11-09

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
| ACS-3 | Adaptív végrehajtási terv: alternatív utak (manifest v5 `paths`), szakaszos futás, streaming, részleges modellbetöltés, időbecslés tartománnyal | Egy funkcióhoz több validált út érhető el, ahol a technológia engedi | |
| ACS-4 | Feladatsor és ütemezett végrehajtás: központi sor, P0/P1/P2/R osztály és felhasználói prioritás, négy indítási mód (azonnal, erőforrásra, időpontra, időpontra és erőforrásra), `scheduled` és `paused` állapot, folytatás | Újraindítás után sem vész el feladat, és egyik sem indul kétszer (PG-2: bérlet, `SKIP LOCKED`) | |
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
  `cpu.max`, a legszigorúbb számít), online CPU-k, GPU-k VRAM-ja és kijelzőhasználata. A
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
vagy amely kijelzőt hajt, miközben van másik alkalmas kártya. Ha egyetlen kártya van, és az
kijelzőt hajt, akkor csak a felhasználó engedélyével (`allowDisplayGpu`). Minden kizárásnak oka
van.

**Foglalás (`manager`).** A `try_reserve` egy feladat RAM-, CPU- és VRAM-igényét **egyszerre
adja meg, vagy semmit** (nincs „tartok és várok”, nincs holtpont). Négy ellenőrzés:

1. A kereten túli igény `NeverFits`: így ezen a gépen nem fut, a tervező (ACS-3) másik validált
   utat választ. Ismeretlen keretre semmi nem fér.
2. A többi foglalás és az új igény együtt sem lépheti túl a keretet.
3. A pillanatnyi szabad memória és VRAM (a tartalék nélkül) fedezze az igényt, mert más programok
   is használják.
4. Nyomás osztályonként:
   - memórianyomásnál vagy kevés memóriánál (kiugrás vagy tartós) csak az `Interactive` (P0)
     indul, mert a felhasználó vár rá;
   - tartós CPU- vagy I/O-nyomásnál a `Speculative` (P2) vár, a többi csak lassabb lesz;
   - tartósan tele VRAM-ú GPU új munkát nem kap.

Ha valami nem teljesül, `Wait` jön, minden okkal; az újrapróbálást a sor (ACS-4) végzi. A GPU-t a
kezelő választja: amelyiken a legtöbb hely van.

## 6. Következő lépés: ACS-3

Adaptív végrehajtási terv: a manifest v5 `paths` útjai közül az válik futtathatóvá, amelynek
`Request`-je a keretbe fér. A `NeverFits` a következő utat próbálja (automatikusan csak
`tolerance.automatic=true` esetén, egyébként jóváhagyással). Időbecslés tartománnyal
(`estimate {min_s, max_s, confidence}`). NVML-olvasó a `cfa4-platform`-ban az NVIDIA VRAM-jához,
a `GpuMemoryReport` felületen át.
