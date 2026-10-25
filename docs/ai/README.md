# CFA4 AI – besorolási táblák

> **Állapot:** javaslat, egyetlen projekt–típus pár sincs még tesztelve · **Utolsó átnézés:**
> 2026-10-25 · **Változás:** major (első változat) · **Döntés:**
> [ADR 0006](../adr/ADR-0006-ai-processing-layer.md)

| Fájl | Tartalom |
|---|---|
| [`200-tipus-besorolas.csv`](200-tipus-besorolas.csv) | A 200 funkciótípus: csoport, jelölt projektek, motorcsalád (M1–M12), besorolás (képesség, változat, sablon, kísérleti), futtató, 12 GB-os futtathatóság, fázis |
| [`projektjeloltek.csv`](projektjeloltek.csv) | A 107 jelölt repó: motorcsalád és javaslat (elsődleges, értékelendő Rust, tartalék, referencia, kihagyás) |

A licenc- és VRAM-adatok becslések, és egyetlen projekt sincs még kipróbálva. Mielőtt egy projekt
bekerül, ellenőrizni kell a repójában a licencét, és le kell mérni a
[`scripts/dev/run-minspec.sh`](../../scripts/dev/run-minspec.sh) alatt. A hang-, zene- és
zajfunkciók listája külön táblába kerül.
