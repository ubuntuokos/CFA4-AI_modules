# F3 – kézi jelölés a pontossági méréshez

> **Állapot:** útmutató · **Utolsó átnézés:** 2026-11-02 · **Mire kell:** `measure.py accuracy`
> ([modellválasztás](../../../docs/ai/modellvalasztas-091.md) 4. pont: szóhatár ≤ 50 ms,
> sorhatár ≤ 100 ms, IoU ≥ 0,85)

A mérés a modell illesztését veti össze a kézi jelöléssel. A kézi jelölés a viszonyítási alap, ezért
annak kell pontosnak lennie: egy elcsúszott kézi határ hibának látszik a modellnél.

## 1. Milyen hang kell

- **Angol beszéd, 10–20 klip, egyenként 1–5 perc.** Összesen legalább 200 sor; ebből legalább 50
  sorban a szavakat is jelöld (kb. 300 szó). A többi sornál elég a sor eleje és vége.
- **Változatos legyen:** több beszélő, férfi és női hang, csendes szoba és zajos jelenet, alatta
  zene, gyors párbeszéd egymásba érő mondatokkal. A szerkesztő valódi anyaga ilyen.
- **Ne legyen LibriVox-hangoskönyv vagy LibriSpeech-részlet:** a modell ezeken tanult, ezért ott
  jobbnak látszana, mint a valóságban.
- **Jó forrás:** saját felvétel, vagy olyan film-, sorozat- és interjúrészlet, amelyhez van jogod.
  A hangfájlok a gépeden maradnak, a repóba nem kerülnek (a CFA4 nem tárol médiát).
- A formátum bármi lehet, amit a `soundfile` olvas (WAV, FLAC, OGG); a mérés monóra keveri és
  16 kHz-re mintavételezi át.

## 2. Hogyan írd a szöveget

A modell csak az angol ábécét és az aposztrófot ismeri. Amit nem tud kiolvasni, azt a mérés
kihagyja, ezért úgy írd, ahogy elhangzik:

| Elhangzik | Így írd | Ne így |
|---|---|---|
| „twenty past ten” | `twenty past ten` | `10:20` |
| „the first time” | `the first time` | `the 1st time` |
| „three point one four” | `three point one four` | `3.14` |
| „five dollars” | `five dollars` | `$5` |
| „café” | `cafe` | `café` |

Az egész számok (`2026`, `1,000`) és az `&`, `%`, `+`, `@` maradhat, ezeket a szkript kiírja; az
írásjelek (pont, vessző, kérdőjel, gondolatjel) is maradhatnak. A `check_labels.py` szól, ha
valami kiesne.

## 3. Hogyan jelölj Audacityben

1. Nyisd meg a klipet. Adj hozzá két címkesávot: **Sávok → Új hozzáadása → Címkesáv**; az egyik a
   soroké, a másik a szavaké.
2. **Sor:** jelöld ki egérrel a sort az első szó első hangjától az utolsó szó utolsó hangjáig,
   majd **Ctrl+B**, és írd be a szöveget. Ne hagyj csendet az elején vagy a végén: a mérés a
   kimondás határát veti össze, nem a felirat megjelenési idejét.
3. **Szó:** ugyanígy a szavak sávjában, szavanként. Nagyíts rá (Ctrl+1), és a hullámformán és a
   spektrogramon (sáv neve → Spektrogram) keresd a hang elejét és végét. Ha két szó egybeolvad,
   a határ a hangátmenet közepe.
4. **Exportálás:** **Fájl → Exportálás → Címkék exportálása**, sávonként külön fájlba
   (például `klip01-sorok.txt` és `klip01-szavak.txt`). Ha egyszerre több sáv van, az Audacity
   mindegyiket egy fájlba teszi: ilyenkor a másikat ideiglenesen töröld vagy némítsd az exportnál.

## 4. Címkefájl és ellenőrzés

```sh
D=cimkek   # a hangfájlok és a címkefájl mappája
python3 scripts/dev/f3-align/from_audacity.py $D/labels.json klip01.wav \
    $D/klip01-sorok.txt $D/klip01-szavak.txt
python3 scripts/dev/f3-align/check_labels.py $D/labels.json
```

A `from_audacity.py` hozzáadja (vagy lecseréli) a klipet a címkefájlban; a szavakat idő szerint
rendeli a soraikhoz. A `check_labels.py` a mérés előtt jelzi a hiányzó hangfájlt, az egymásba
lógó sorokat, a sorból kilógó szavakat és a kiírandó számokat. A formátum mintája:
[`labels.example.json`](labels.example.json). Ha a hiba szám nulla, jöhet a mérés:

```sh
$R accuracy --model $M --onnx-dir target/f3-align/onnx --labels $D/labels.json
```

## 5. Ellenőrizd magad is

Jelölj meg újra egy klipet egy nappal később, és vesd össze a két jelölést. Ha a saját két
jelölésed szóhatárai is gyakran 50 ms-nál többel térnek el, a küszöb a jelölés pontosságát méri,
nem a modellét; ezt írd be a mérési jegyzőkönyvbe.
