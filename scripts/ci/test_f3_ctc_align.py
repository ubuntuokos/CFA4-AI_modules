#!/usr/bin/env python3
"""CFA4 F3 CTC forced alignment prototype: synthetic model output, standard library only."""
from __future__ import annotations

import math
from pathlib import Path
import sys
import unittest

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/dev/f3-align"))

import check_labels as cl  # noqa: E402
import ctc_align as ca  # noqa: E402
import from_audacity as fa  # noqa: E402

# The facebook/wav2vec2-base-960h vocabulary (vocab.json at revision 22aad52).
VOCAB = {"<pad>": 0, "<s>": 1, "</s>": 2, "<unk>": 3, "|": 4, "E": 5, "T": 6, "A": 7, "O": 8,
         "N": 9, "I": 10, "H": 11, "S": 12, "R": 13, "D": 14, "L": 15, "U": 16, "M": 17,
         "W": 18, "C": 19, "F": 20, "G": 21, "Y": 22, "P": 23, "B": 24, "V": 25, "K": 26,
         "'": 27, "X": 28, "J": 29, "Q": 30, "Z": 31}


def frames_for(plan: list[tuple[str, int]]) -> list[list[float]]:
    """Synthetic log-probabilities: each (symbol, n) is n frames where that symbol wins."""
    rows = []
    for symbol, n in plan:
        idx = VOCAB[symbol]
        for _ in range(n):
            row = [-8.0] * len(VOCAB)
            row[idx] = -0.05
            rows.append(ca.log_softmax(row))
    return rows


class Numbers(unittest.TestCase):
    def test_number_words(self):
        self.assertEqual(ca.number_words(0), "zero")
        self.assertEqual(ca.number_words(21), "twenty one")
        self.assertEqual(ca.number_words(305), "three hundred five")
        self.assertEqual(ca.number_words(2026), "two thousand twenty six")
        self.assertEqual(ca.number_words(1_000_001), "one million one")
        for bad in (-1, 1_000_000_000):
            with self.subTest(bad=bad), self.assertRaises(ca.AlignError):
                ca.number_words(bad)


class Normalize(unittest.TestCase):
    def test_words_numbers_and_dropped(self):
        words, dropped = ca.normalize("Hello, it's 3 o’clock — café!", VOCAB)
        self.assertEqual(words, ["HELLO", "IT'S", "THREE", "O'CLOCK", "CAF"])
        self.assertEqual(dropped, ["É"])
        words, dropped = ca.normalize("call 1234567890 now", VOCAB)
        self.assertEqual((words, dropped), (["CALL", "NOW"], ["1234567890"]))
        words, _ = ca.normalize("wait\u2014what?no.YES,ok it's", VOCAB)
        self.assertEqual(words, ["WAIT", "WHAT", "NO", "YES", "OK", "IT'S"])
        words, dropped = ca.normalize("the 1st and 22ND, 3rd 4th", VOCAB)
        self.assertEqual((words, dropped), (["THE", "AND"], ["1st", "22ND", "3rd", "4th"]))
        words, dropped = ca.normalize("R&D at 50% + tax @ home", VOCAB)
        self.assertEqual((" ".join(words), dropped),
                         ("R AND D AT FIFTY PERCENT PLUS TAX AT HOME", []))
        _, dropped = ca.normalize("costs $5 / day", VOCAB)
        self.assertEqual(dropped, ["$", "/"])
        words, dropped = ca.normalize("pi is 3.14, pages 3-5, minus -5 ok", VOCAB)
        self.assertEqual(words, ["PI", "IS", "PAGES", "MINUS", "OK"])
        self.assertEqual(dropped, ["3.14", "3-5", "-5"])
        self.assertEqual(ca.normalize("at 10:20", VOCAB), (["AT"], ["10:20"]))
        words, _ = ca.normalize("1,000 and 12,500,000 or 3,4", VOCAB)
        self.assertEqual(" ".join(words), "ONE THOUSAND AND TWELVE MILLION FIVE HUNDRED "
                                          "THOUSAND OR THREE FOUR")

    def test_long_text_is_refused(self):
        with self.assertRaises(ca.AlignError):
            ca.normalize("a" * (ca.MAX_TEXT_CHARS + 1), VOCAB)


class Align(unittest.TestCase):
    def test_two_words_get_their_frames(self):
        # 3 blank, H I (2 each), 2 blank, "|", 1 blank, Y O U, 4 blank
        lp = frames_for([("<pad>", 3), ("H", 2), ("I", 2), ("<pad>", 2), ("|", 1),
                         ("<pad>", 1), ("Y", 1), ("O", 1), ("U", 2), ("<pad>", 4)])
        words = ca.align(lp, ["HI", "YOU"], VOCAB)
        self.assertEqual([(w.text, w.start_frame, w.end_frame) for w in words],
                         [("HI", 3, 7), ("YOU", 11, 15)])
        self.assertGreater(words[0].score, 0.9)
        self.assertEqual(words[1].start_ticks, 11 * ca.TICKS_PER_FRAME)
        self.assertEqual(ca.TICKS_PER_FRAME, 5_080_320_000)
        self.assertAlmostEqual(words[1].end_s, 0.30)

    def test_repeated_letters_need_a_blank(self):
        lp = frames_for([("<pad>", 1), ("L", 1), ("<pad>", 1), ("L", 1), ("<pad>", 1)])
        words = ca.align(lp, ["LL"], VOCAB)
        self.assertEqual((words[0].start_frame, words[0].end_frame), (1, 4))
        # Two frames cannot hold L, blank, L.
        with self.assertRaises(ca.AlignError):
            ca.align(frames_for([("L", 2)]), ["LL"], VOCAB)

    def test_hostile_input_raises_instead_of_crashing(self):
        lp = frames_for([("A", 3)])
        cases = {
            "no words": ([], lp),
            "empty word, no frames": ([""], []),
            "no frames": (["A"], []),
            "unknown character": (["É"], lp),
            "short frame row": (["A"], [[0.0]] * 3),
            "too many frames": (["A"], [[0.0] * len(VOCAB)] * (ca.MAX_FRAMES + 1)),
            # Each limit alone holds, but frames × states exceeds the table limit.
            "too many cells": (["AB"] * 2000, [[0.0] * len(VOCAB)] * ca.MAX_FRAMES),
        }
        for why, (words, frames) in cases.items():
            with self.subTest(why), self.assertRaises(ca.AlignError):
                ca.align(frames, words, VOCAB)

    def test_unreachable_path(self):
        lp = [[-math.inf] * len(VOCAB) for _ in range(3)]
        with self.assertRaises(ca.AlignError):
            ca.align(lp, ["A"], VOCAB)


class Labels(unittest.TestCase):
    def test_vocab_copy_matches(self):
        self.assertEqual(ca.WAV2VEC2_VOCAB, VOCAB)

    def test_example_is_valid(self):
        import json
        import tempfile

        example = ROOT / "scripts/dev/f3-align/labels.example.json"
        labels = json.loads(example.read_text(encoding="utf-8"))
        with tempfile.TemporaryDirectory() as tmp:
            (Path(tmp) / "klip01.wav").touch()
            self.assertEqual(cl.check(labels, Path(tmp)), ([], 2, 5))
            errors, _, _ = cl.check(labels, Path(tmp) / "nincs")
            self.assertEqual(len(errors), 1)  # csak a hiányzó hangfájl

    def test_checker_catches_mistakes(self):
        bad = [{"audio": "a.wav", "lines": [
            {"text": "at 10:20", "start": 1.0, "end": 2.0},
            {"text": "go now", "start": 1.5, "end": 3.0,
             "words": [{"text": "now", "start": 1.5, "end": 2.0},
                       {"text": "go", "start": 2.0, "end": 3.5}]}]}]
        errors, _, _ = cl.check(bad, ROOT)
        text = " ".join(errors)
        for part in ("10:20", "korábbi sor", "kilóg", "nem egyeznek", "nem található"):
            with self.subTest(part):
                self.assertIn(part, text)

    def test_audacity_import(self):
        import tempfile

        with tempfile.TemporaryDirectory() as tmp:
            lines, words = Path(tmp) / "s.txt", Path(tmp) / "w.txt"
            lines.write_text("0,5\t1,5\tHi you\n2.0\t3.0\tBye\n", encoding="utf-8")
            words.write_text("0.5\t0.9\tHi\n1.0\t1.5\tyou\n", encoding="utf-8")
            clip = fa.build_clip("a.wav", lines, words)
            self.assertEqual([len(ln.get("words", [])) for ln in clip["lines"]], [2, 0])
            words.write_text("5.0\t6.0\tlost\n", encoding="utf-8")
            with self.assertRaises(ValueError):
                fa.build_clip("a.wav", lines, words)
            lines.write_text("2.0\t1.0\tbackwards\n", encoding="utf-8")
            with self.assertRaises(ValueError):
                fa.read_audacity(lines)


if __name__ == "__main__":
    unittest.main()
