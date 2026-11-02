#!/usr/bin/env python3
"""CFA4 F3 – Audacity-címkékből címkefájl (csak standard könyvtár).

Használat:
  python3 scripts/dev/f3-align/from_audacity.py cimkek/labels.json klip01.wav \\
      klip01-sorok.txt [klip01-szavak.txt]

Az Audacity „Címkék exportálása” fájlja soronként `kezdet<TAB>vég<TAB>szöveg`. A sorok fájlja
kötelező; a szavak fájlja elhagyható, ilyenkor csak a sorok idejét mérjük. A szavakat a soraikhoz
idő alapján rendeljük (a szó közepe a sorba esik). A klip bejegyzését a címkefájlban hozzáadja
vagy lecseréli, a többit nem bántja. Utána futtasd a check_labels.py-t.
"""
from __future__ import annotations

import json
from pathlib import Path
import sys

MAX_FILE_BYTES = 4 << 20


def read_text(path: Path) -> str:
    with path.open("rb") as f:
        data = f.read(MAX_FILE_BYTES + 1)
    if len(data) > MAX_FILE_BYTES:
        raise ValueError(f"{path}: a fájl nagyobb {MAX_FILE_BYTES} bájtnál")
    return data.decode("utf-8-sig")


def read_audacity(path: Path) -> list[dict]:
    """Az Audacity címkeexportja; a spektrális címkék `\\` kezdetű sorait kihagyjuk."""
    items = []
    for n, raw in enumerate(read_text(path).splitlines(), start=1):
        if not raw.strip() or raw.startswith("\\"):
            continue
        parts = raw.split("\t", 2)
        if len(parts) != 3:
            raise ValueError(f"{path}:{n}: nem `kezdet<TAB>vég<TAB>szöveg` sor")
        try:
            start, end = float(parts[0].replace(",", ".")), float(parts[1].replace(",", "."))
        except ValueError as err:
            raise ValueError(f"{path}:{n}: hibás időérték") from err
        if not (0 <= start < end) or end != end or end == float("inf"):
            raise ValueError(f"{path}:{n}: a kezdet legyen kisebb a végnél, nemnegatív")
        text = parts[2].strip()
        if not text:
            raise ValueError(f"{path}:{n}: üres címke")
        items.append({"text": text, "start": round(start, 3), "end": round(end, 3)})
    items.sort(key=lambda x: (x["start"], x["end"]))
    return items


def build_clip(audio: str, lines_path: Path, words_path: Path | None) -> dict:
    lines = read_audacity(lines_path)
    if not lines:
        raise ValueError(f"{lines_path}: nincs egyetlen sor sem")
    if words_path is not None:
        for line in lines:
            line["words"] = []
        for word in read_audacity(words_path):
            mid = (word["start"] + word["end"]) / 2
            owner = next((ln for ln in lines if ln["start"] <= mid <= ln["end"]), None)
            if owner is None:
                raise ValueError(f"{words_path}: a(z) {word['text']!r} szó "
                                 f"({word['start']} s) egyik sorba sem esik")
            owner["words"].append(word)
        for line in lines:
            if not line["words"]:
                del line["words"]  # e sor szavait nem jelölték: csak a sor idejét mérjük
    return {"audio": audio, "lines": lines}


def main() -> int:
    if len(sys.argv) not in (4, 5):
        print(__doc__.strip().split("\n\n")[1], file=sys.stderr)
        return 2
    out, audio, lines_path = Path(sys.argv[1]), sys.argv[2], Path(sys.argv[3])
    words_path = Path(sys.argv[4]) if len(sys.argv) == 5 else None
    try:
        clip = build_clip(audio, lines_path, words_path)
        labels = json.loads(read_text(out)) if out.exists() else []
        if not isinstance(labels, list):
            raise ValueError(f"{out}: a gyökér lista kell legyen")
        labels = [c for c in labels if not (isinstance(c, dict) and c.get("audio") == audio)]
        labels.append(clip)
        out.parent.mkdir(parents=True, exist_ok=True)
        tmp = out.with_suffix(out.suffix + ".tmp")
        tmp.write_text(json.dumps(labels, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
        tmp.replace(out)  # félbeszakadt írás ne rontsa el a meglévő fájlt
    except (OSError, ValueError, RecursionError) as err:
        print(f"HIBA: {err}", file=sys.stderr)
        return 1
    n_words = sum(len(ln.get("words", [])) for ln in clip["lines"])
    print(json.dumps({"audio": audio, "lines": len(clip["lines"]), "words": n_words}))
    return 0


if __name__ == "__main__":
    sys.exit(main())
