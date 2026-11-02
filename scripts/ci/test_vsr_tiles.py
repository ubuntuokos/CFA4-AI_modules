#!/usr/bin/env python3
"""CFA4 #91 – a felskálázó csempefelosztásának tesztje (csak standard könyvtár, modell nélkül).

Futtatás:  python3 scripts/ci/test_vsr_tiles.py
"""
from __future__ import annotations

import math
from pathlib import Path
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "dev" / "vsr-measure"))
import tiles as tl  # noqa: E402


def coverage(height: int, width: int, tile: int, pad: int, scale: int) -> list[list[int]]:
    hits = [[0] * (width * scale) for _ in range(height * scale)]
    for t in tl.plan(height, width, tile, pad, scale):
        assert 0 <= t.in_y0 < t.in_y1 <= height and 0 <= t.in_x0 < t.in_x1 <= width
        assert t.in_y1 - t.in_y0 <= tile + 2 * pad and t.in_x1 - t.in_x0 <= tile + 2 * pad
        # A megtartott rész a csempe kimenetén belül van, és ugyanakkora, mint a célhelye.
        assert 0 <= t.crop_y0 < t.crop_y1 <= (t.in_y1 - t.in_y0) * scale
        assert 0 <= t.crop_x0 < t.crop_x1 <= (t.in_x1 - t.in_x0) * scale
        assert t.crop_y1 - t.crop_y0 == t.out_y1 - t.out_y0
        assert t.crop_x1 - t.crop_x0 == t.out_x1 - t.out_x0
        # A célhely a kivágás saját részének nagyított képe.
        assert t.out_y0 == t.in_y0 * scale + t.crop_y0 and t.out_x0 == t.in_x0 * scale + t.crop_x0
        if scale == 2:
            assert all(v % 2 == 0 for v in (t.in_y0, t.in_y1, t.in_x0, t.in_x1))
        for y in range(t.out_y0, t.out_y1):
            row = hits[y]
            for x in range(t.out_x0, t.out_x1):
                row[x] += 1
    return hits


class Plan(unittest.TestCase):
    def test_every_output_pixel_exactly_once(self) -> None:
        for h, w, tile, pad, scale in ((60, 96, 32, 10, 2), (54, 50, 16, 4, 4), (30, 30, 64, 10, 2),
                                       (7, 9, 4, 3, 4), (40, 40, 40, 0, 2)):
            with self.subTest(h=h, w=w, tile=tile, pad=pad, scale=scale):
                hits = coverage(h, w, tile, pad, scale)
                self.assertTrue(all(v == 1 for row in hits for v in row))

    def test_2k_output_tile_counts(self) -> None:
        # 1024×540 → 2048×1080 kettes nagyítással: 256-os csempéből 4×3, 512-esből 2×2.
        self.assertEqual(len(tl.plan(540, 1024, 256, 10, 2)), 12)
        self.assertEqual(len(tl.plan(540, 1024, 512, 10, 2)), 4)

    def test_rejects_bad_sizes(self) -> None:
        for args in ((0, 10, 8, 2, 2), (10, 10, 8, 2, 3), (11, 10, 8, 2, 2), (10, 10, 8, 3, 2),
                     (10, 10, 8, 9, 4), (10, 10, 8, -1, 4), (10, 10, True, 2, 4),
                     (tl.MAX_SIDE + 1, 10, 8, 2, 4), (8192, 8192, 2, 0, 4)):
            with self.subTest(args=args), self.assertRaises(ValueError):
                tl.plan(*args)

    def test_psnr(self) -> None:
        self.assertTrue(math.isinf(tl.psnr(0.0)))
        self.assertAlmostEqual(tl.psnr(1e-4), 40.0)
        for bad in (-1.0, math.nan):
            with self.assertRaises(ValueError):
                tl.psnr(bad)


if __name__ == "__main__":
    unittest.main()
