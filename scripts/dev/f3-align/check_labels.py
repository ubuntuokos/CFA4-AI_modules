#!/usr/bin/env python3
"""CFA4 F3 – a kézi címkefájl ellenőrzése a mérés előtt (csak standard könyvtár, modell nélkül).

Használat:  python3 scripts/dev/f3-align/check_labels.py cimkek/labels.json

Hibát ad, ha a fájl szerkezete rossz, ha egy hangfájl hiányzik, ha a sorok vagy szavak ideje
nem növekvő vagy kilóg a sorból, vagy ha a szöveg normalizálásakor bármi kiesne (az ilyen sort a
mérés kihagyná). A formátumot és a jelölés módját a JELOLES.md írja le.
"""
from __future__ import annotations

import json
from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent))
import ctc_align as ca  # noqa: E402

MAX_LABELS_BYTES = 16 << 20
MAX_LABEL_S = 24 * 3600.0
EDGE_TOLERANCE_S = 0.02  # egy modellkocka: a szó szélei ennyivel lóghatnak ki a sorból


def is_time(x: object) -> bool:
    return isinstance(x, (int, float)) and not isinstance(x, bool) and 0 <= x <= MAX_LABEL_S


def check(labels: object, base: Path) -> tuple[list[str], int, int]:
    """Visszaadja a hibákat, a sorok és a szavak számát."""
    errors: list[str] = []
    if not isinstance(labels, list):
        return ["a gyökér lista kell legyen"], 0, 0
    n_lines = n_words = 0
    for ci, clip in enumerate(labels):
        if not isinstance(clip, dict) or not isinstance(clip.get("audio"), str) \
                or not isinstance(clip.get("lines"), list):
            errors.append(f"{ci}. klip: kell egy 'audio' szöveg és egy 'lines' lista")
            continue
        audio = clip["audio"]
        if not (base / audio).is_file():
            errors.append(f"{audio}: a hangfájl nem található ({base / audio})")
        prev_end = -1.0
        for li, line in enumerate(clip["lines"]):
            where = f"{audio}, {li + 1}. sor"
            if not isinstance(line, dict) or not isinstance(line.get("text"), str) \
                    or not is_time(line.get("start")) or not is_time(line.get("end")) \
                    or line["end"] <= line["start"]:
                errors.append(f"{where}: kell 'text', és 'start' < 'end' másodpercben")
                continue
            n_lines += 1
            if line["start"] < prev_end:
                errors.append(f"{where}: a sor a korábbi sor vége ({prev_end:.3f} s) előtt kezdődik")
            prev_end = max(prev_end, line["end"])
            try:
                words, dropped = ca.normalize(line["text"], ca.WAV2VEC2_VOCAB)
            except ca.AlignError as err:
                errors.append(f"{where}: {err}")
                continue
            if dropped:
                errors.append(f"{where}: a szövegből kiesne: {dropped} (írd ki szóval)")
            if not words:
                errors.append(f"{where}: a szövegben nincs illeszthető szó")
            refs = line.get("words", [])
            if not isinstance(refs, list):
                errors.append(f"{where}: a 'words' lista kell legyen")
                continue
            if not refs:
                continue
            ref_words: list[str] = []
            prev_word_end = -1.0
            for wi, ref in enumerate(refs):
                wwhere = f"{where}, {wi + 1}. szó"
                if not isinstance(ref, dict) or not isinstance(ref.get("text"), str) \
                        or not is_time(ref.get("start")) or not is_time(ref.get("end")) \
                        or ref["end"] <= ref["start"]:
                    errors.append(f"{wwhere}: kell 'text', és 'start' < 'end' másodpercben")
                    continue
                n_words += 1
                if ref["start"] < prev_word_end:
                    errors.append(f"{wwhere}: az előző szó vége előtt kezdődik")
                prev_word_end = ref["end"]
                if ref["start"] < line["start"] - EDGE_TOLERANCE_S \
                        or ref["end"] > line["end"] + EDGE_TOLERANCE_S:
                    errors.append(f"{wwhere}: kilóg a sorból")
                normalized, ref_dropped = ca.normalize(ref["text"], ca.WAV2VEC2_VOCAB)
                if ref_dropped or len(normalized) != 1:
                    errors.append(f"{wwhere}: {ref['text']!r} nem egyetlen kiírt szó "
                                  f"(normalizálva: {normalized}, kiesne: {ref_dropped})")
                ref_words.extend(normalized)
            if ref_words and ref_words != words:
                errors.append(f"{where}: a szavak nem egyeznek a sor szövegével: "
                              f"{ref_words} ≠ {words}")
    if n_lines == 0:
        errors.append("a címkefájlban nincs egyetlen sor sem")
    return errors, n_lines, n_words


def main() -> int:
    if len(sys.argv) != 2:
        print(__doc__.strip().splitlines()[2].strip(), file=sys.stderr)
        return 2
    path = Path(sys.argv[1])
    try:
        with path.open("rb") as f:
            data = f.read(MAX_LABELS_BYTES + 1)
        if len(data) > MAX_LABELS_BYTES:
            raise ValueError(f"a fájl nagyobb {MAX_LABELS_BYTES} bájtnál")
        labels = json.loads(data.decode("utf-8"))
    except (OSError, ValueError, RecursionError) as err:
        print(f"{path}: nem olvasható címkefájl: {err}", file=sys.stderr)
        return 1
    errors, n_lines, n_words = check(labels, path.parent)
    for e in errors:
        print(f"HIBA: {e}", file=sys.stderr)
    print(json.dumps({"lines": n_lines, "words": n_words, "errors": len(errors)}))
    return 1 if errors else 0


if __name__ == "__main__":
    sys.exit(main())
