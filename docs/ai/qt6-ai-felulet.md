# Az AI-réteg Qt 6 felülete: előkészítés

> **Állapot:** tervezet, kód nincs · **Utolsó átnézés:** 2026-11-22 · **Forrás:**
> [ADR 0008](../adr/ADR-0008-ai-processing-layer.md) 3., 10. és 11. pont · **Kapu:** az F0 után
> (az AI-felület csak a Qt 6 shellbe kerül, az egui-ba nem)

Ez a dokumentum azt rögzíti, amit a felületnek a Qt 6 átállás során már most tudnia kell, hogy az
AI-réteg később ne kényszerítsen átépítést. Új képernyőt vagy `ai.*` parancsot nem ír elő a
mostani kódba; az F0 kapu (ADR 0008) addig változatlan.

## 1. Alapszabályok

- Minden `ai.*` parancs a motor parancsregiszterén át megy (L4, `crates/engine`); a felület nem hív
  közvetlenül workert, így a visszavonás, a napló és a vezérlőcsatorna ugyanazt látja.
- Minden interaktív elemnek automation azonosítója van, az állapot serde-alapú, tehát a
  vezérlőcsatorna olvassa és állítja (`AGENTS.md`).
- A beállítópaneleket a manifest sémájából generált natív Qt-elemek adják; külső böngésző nincs.
  Összetett szerkesztéshez a beágyazott, bővítmény nélküli, csak helyi fájlokat nyitó
  Qt-webnézet felugró ablaka használható, `QWebChannel`-lel (helyi port nélkül); az idővonalra csak a
  kész kép vagy videó kerül, az Alkalmazás paranccsal.

## 2. Panelek és a mögöttük álló adat

| Panel | Mit mutat | Honnan |
|---|---|---|
| Funkcióválasztó | Mi futna ezen a gépen és miért (vagy miért nem), futtatás nélkül | jegyzék (`cfa4-ai-registry`), ADR 3. pont (elérhetőség) |
| Feladatkezelő | Jobok, állapotok, várakozás oka, becsült idő tartománnyal, döntési gombok | `tasks.list`, `tasks.decide` (a Qt Task Manager már él: `python/cfa4_qt/panels/tasks.py`) |
| Előnézet | Elemzés → Javaslat → Előnézet → Alkalmazás lánc, egyetlen visszavonható Alkalmazás | jobállapotok, projektverzió-ellenőrzés |
| Modell és licenc | Az egyszeri, modellenkénti elfogadás: a négy licencrész (kód, függőségek, súly, tanítóadat) és a letöltés forrása | modellkezelő, ADR 2. és 8. pont |
| Erőforrás-jelző | GPU-nkénti VRAM-keret és -használat, RAM, a kijelzőt hajtó GPU jelzése | VRAM-könyvelés (GPU-nként, eszközazonosítóval) |
| Node-gráf | Típusos portok, mentett gráf (JSON), importált ComfyUI-workflow | manifest portjai (`type`, `color`, `format`), ADR 11. pont |
| ComfyUI-telepítés | Egyszeri jóváhagyás: forrás, verzió, méret, licenc (GPL-3.0); állapot, javítás, újratelepítés | `ai.comfy.repair`, ADR 11. pont |

## 3. Amit a felületnek külön kezelnie kell

- **Nincs néma visszalépés:** az automatikus OOM-lépések a naplóba kerülnek, a kisebb pontosság, a
  kisebb felbontás vagy a másik modell jóváhagyást kér, és a becsült időt a felület mutatja.
- **Nem elérhető funkció** magyarázattal jelenik meg (VRAM, RAM, hiányzó modell), nem tűnik el.
- **Licencfigyelmeztetés:** nem kereskedelmi licencű modellnél minden használatkor jelzés; a
  szintetikus arc „AI által módosított” jelölése nem kikapcsolható.
- **Szín és idő:** a felület a médiahíd színkonverzióját (port `color`, `format`) és a `Tick`
  alapú időt használja, `f64` másodpercet nem.
- **Távoli futás** később jöhet; a felület feladatként kezeli, nem helyi eszközként.

## 4. Mi kell a Qt 6 oldalán, mielőtt az AI-felület elkészülhet

1. A Qt shell a valódi motorral dolgozik (a `cfa4-cli` lekérdezésen túl); ez az F0 kilépési feltétele.
2. Az ACS crate-ek (`cfa4-tasks`) a CFA4 főrepóban is megjelennek; ettől a `tasks.list` ott is él.
3. A parancsregiszter `ai.*` névtere és a séma-generált beállítópanel alapja.

Az első két pont a CFA4 főrepó és a Qt 6 szál feladata; ez a dokumentum csak a követelményeket
adja át nekik.
