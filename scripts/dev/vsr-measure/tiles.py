"""CFA4 #91 – a képkocka csempékre bontása a felskálázáshoz (csak standard könyvtár).

A felskálázó a képkockát átfedő csempékben dolgozza fel, hogy a VRAM-igény a csempe méretétől
függjön, ne a képkockáétól. Minden csempe `pad` képponttal nagyobb a saját részénél (a széleken
kevesebbel), így a csempehatáron a modell lát környezetet; a kimenetből csak a saját részt
tartjuk meg. Ugyanezt a felosztást használja majd a Rust-worker is.
"""
from __future__ import annotations

from dataclasses import dataclass

MAX_SIDE = 8192
MAX_TILES = 4096


@dataclass(frozen=True)
class Tile:
    # A bemeneti kép kivágása (környezettel együtt), képpontban.
    in_y0: int
    in_y1: int
    in_x0: int
    in_x1: int
    # A csempe kimenetéből megtartandó rész, a csempe kimenetének koordinátáiban.
    crop_y0: int
    crop_y1: int
    crop_x0: int
    crop_x1: int
    # Ide kerül a megtartott rész a teljes kimeneti képen.
    out_y0: int
    out_y1: int
    out_x0: int
    out_x1: int


def _spans(size: int, tile: int, pad: int) -> list[tuple[int, int, int, int]]:
    spans = []
    for start in range(0, size, tile):
        end = min(start + tile, size)
        spans.append((start, end, max(start - pad, 0), min(end + pad, size)))
    return spans


def plan(height: int, width: int, tile: int, pad: int, scale: int) -> list[Tile]:
    """A csempék listája; `ValueError`, ha a méretek nem használhatók.

    Kettes nagyításnál a modell a bemenetet 2×2-es blokkokba rendezi (pixel unshuffle), ezért
    minden kivágás széle páros kell legyen: ehhez a kép, a csempe és a környezet mérete páros.
    """
    for name, value in (("height", height), ("width", width), ("tile", tile)):
        if not isinstance(value, int) or isinstance(value, bool) or not 1 <= value <= MAX_SIDE:
            raise ValueError(f"{name}: 1 és {MAX_SIDE} közötti egész kell, nem {value!r}")
    if not isinstance(pad, int) or isinstance(pad, bool) or not 0 <= pad <= tile:
        raise ValueError(f"pad: 0 és a csempeméret közötti egész kell, nem {pad!r}")
    if scale not in (2, 4):
        raise ValueError(f"scale: 2 vagy 4 lehet, nem {scale!r}")
    if scale == 2 and any(v % 2 for v in (height, width, tile, pad)):
        raise ValueError("kettes nagyításnál a kép, a csempe és a környezet mérete páros kell legyen")
    rows, cols = _spans(height, tile, pad), _spans(width, tile, pad)
    if len(rows) * len(cols) > MAX_TILES:
        raise ValueError(f"túl sok csempe ({len(rows) * len(cols)}); legfeljebb {MAX_TILES}")
    tiles = []
    for y0, y1, py0, py1 in rows:
        for x0, x1, px0, px1 in cols:
            tiles.append(Tile(
                in_y0=py0, in_y1=py1, in_x0=px0, in_x1=px1,
                crop_y0=(y0 - py0) * scale, crop_y1=(y1 - py0) * scale,
                crop_x0=(x0 - px0) * scale, crop_x1=(x1 - px0) * scale,
                out_y0=y0 * scale, out_y1=y1 * scale, out_x0=x0 * scale, out_x1=x1 * scale))
    return tiles


def psnr(mse: float, peak: float = 1.0) -> float:
    """PSNR dB-ben; azonos képnél végtelen."""
    import math

    if not mse >= 0:
        raise ValueError(f"az MSE nem lehet negatív vagy NaN: {mse!r}")
    if mse == 0:
        return math.inf
    return 10 * math.log10(peak * peak / mse)
