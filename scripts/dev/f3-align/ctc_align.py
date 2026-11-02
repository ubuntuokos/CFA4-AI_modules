#!/usr/bin/env python3
"""CFA4 F3 – CTC-alapú kényszerített illesztés (felirat-időzítés), mérési prototípus.

Csak a standard könyvtárat használja, hogy a CI külső csomag nélkül tesztelhesse. A termékben
ez Rustban lesz; ez a fájl a mérés és a Rust-változat referenciája.

Bemenet: egy klip modellkimenete (kockánként log-valószínűségek a szókészletre, 20 ms-os
lépéssel) és a szöveg. Kimenet: szavanként kezdő- és végidő, Tickben és másodpercben.
"""
from __future__ import annotations

from dataclasses import dataclass
import math
import re
from typing import Sequence

# cfa4_time::Tick: 254 016 000 000 / s. A wav2vec2 kockalépése 320 minta 16 kHz-en = 20 ms.
TICKS_PER_SECOND = 254_016_000_000
SAMPLE_RATE = 16_000
FRAME_SAMPLES = 320
TICKS_PER_FRAME = TICKS_PER_SECOND * FRAME_SAMPLES // SAMPLE_RATE  # pontos: 5 080 320 000

WORD_SEP = "|"

# A szövegnormalizálás felső korlátai: ellenséges bemenetre se nőjön a munka korlát nélkül.
MAX_TEXT_CHARS = 20_000
MAX_FRAMES = 30_000  # 10 perc egy illesztésben; a hosszabb hangot soronként ablakolva illesztjük
MAX_STATES = 20_001

_ONES = ("zero one two three four five six seven eight nine ten eleven twelve thirteen "
         "fourteen fifteen sixteen seventeen eighteen nineteen").split()
_TENS = "_ _ twenty thirty forty fifty sixty seventy eighty ninety".split()


class AlignError(ValueError):
    """Az illesztés nem végezhető el (üres szöveg, túl hosszú bemenet, kevés kocka)."""


@dataclass(frozen=True)
class Word:
    text: str
    start_frame: int
    end_frame: int  # kizáró
    score: float  # a szó kockáinak átlagos valószínűsége, 0..1

    @property
    def start_ticks(self) -> int:
        return self.start_frame * TICKS_PER_FRAME

    @property
    def end_ticks(self) -> int:
        return self.end_frame * TICKS_PER_FRAME

    @property
    def start_s(self) -> float:
        return self.start_frame * FRAME_SAMPLES / SAMPLE_RATE

    @property
    def end_s(self) -> float:
        return self.end_frame * FRAME_SAMPLES / SAMPLE_RATE


def number_words(n: int) -> str:
    """0..999 999 999 angolul, kötőjel nélkül (a modell szókészletében nincs kötőjel)."""
    if n < 0 or n >= 1_000_000_000:
        raise AlignError(f"szám a tartományon kívül: {n}")
    if n < 20:
        return _ONES[n]
    if n < 100:
        tens, ones = divmod(n, 10)
        return _TENS[tens] + ("" if ones == 0 else " " + _ONES[ones])
    if n < 1000:
        hundreds, rest = divmod(n, 100)
        return _ONES[hundreds] + " hundred" + ("" if rest == 0 else " " + number_words(rest))
    for size, name in ((1_000_000, "million"), (1000, "thousand")):
        if n >= size:
            high, rest = divmod(n, size)
            return number_words(high) + " " + name + ("" if rest == 0 else " " + number_words(rest))
    raise AlignError(f"szám a tartományon kívül: {n}")  # nem érhető el


def normalize(text: str, vocab: dict[str, int]) -> tuple[list[str], list[str]]:
    """A szöveg szavakra bontása a modell szókészlete szerint.

    Visszaadja a normalizált szavakat és az eldobott (szókészleten kívüli) karaktereket, hogy a
    Javaslat jelezhesse a bizonytalan helyeket.
    """
    if len(text) > MAX_TEXT_CHARS:
        raise AlignError(f"a szöveg túl hosszú: {len(text)} karakter (legfeljebb {MAX_TEXT_CHARS})")
    text = text.replace("’", "'")
    words: list[str] = []
    dropped: list[str] = []

    def spell(m: re.Match[str]) -> str:
        if len(m.group()) > 9:
            dropped.append(m.group())
            return " "
        return " " + number_words(int(m.group())) + " "

    text = re.sub(r"\d+", spell, text)
    for raw in text.upper().split():
        kept = []
        for ch in raw:
            if ch in vocab and ch != WORD_SEP:
                kept.append(ch)
            elif ch.isalpha():
                dropped.append(ch)
        word = "".join(kept).strip("'")
        if word:
            words.append(word)
    return words, dropped


def align(log_probs: Sequence[Sequence[float]], words: Sequence[str], vocab: dict[str, int],
          blank: int = 0) -> list[Word]:
    """Viterbi-alapú CTC kényszerített illesztés.

    `log_probs[t][v]`: a t-edik kocka log-valószínűsége a v-edik szókészletelemre.
    """
    if not words:
        raise AlignError("nincs illeszthető szó")
    tokens: list[int] = []
    owner: list[int] = []  # melyik szóhoz tartozik a token
    sep = vocab.get(WORD_SEP)
    for wi, word in enumerate(words):
        if wi > 0 and sep is not None:
            tokens.append(sep)
            owner.append(-1)
        for ch in word:
            idx = vocab.get(ch)
            if idx is None:
                raise AlignError(f"szókészleten kívüli karakter: {ch!r}")
            tokens.append(idx)
            owner.append(wi)
    frames = len(log_probs)
    states = 2 * len(tokens) + 1
    if frames > MAX_FRAMES or states > MAX_STATES:
        raise AlignError(f"túl hosszú illesztés: {frames} kocka, {states} állapot")
    # A CTC-nek ismételt token között üres kocka kell, ezért ennyi kocka a minimum.
    repeats = sum(1 for a, b in zip(tokens, tokens[1:]) if a == b)
    if frames < len(tokens) + repeats:
        raise AlignError(f"kevés kocka: {frames} a {len(tokens)} tokenhez")
    vocab_size = max(vocab.values()) + 1
    for t, row in enumerate(log_probs):
        if len(row) < vocab_size:
            raise AlignError(f"a {t}. kocka rövidebb a szókészletnél")

    ext = [blank if s % 2 == 0 else tokens[s // 2] for s in range(states)]
    neg = -math.inf
    prev = [neg] * states
    prev[0] = log_probs[0][ext[0]]
    if states > 1:
        prev[1] = log_probs[0][ext[1]]
    back: list[bytearray] = [bytearray(states)]
    for t in range(1, frames):
        row = log_probs[t]
        cur = [neg] * states
        bt = bytearray(states)
        for s in range(states):
            best, step = prev[s], 0
            if s >= 1 and prev[s - 1] > best:
                best, step = prev[s - 1], 1
            if s >= 2 and ext[s] != blank and ext[s] != ext[s - 2] and prev[s - 2] > best:
                best, step = prev[s - 2], 2
            if best != neg:
                cur[s] = best + row[ext[s]]
                bt[s] = step
        prev = cur
        back.append(bt)

    end = states - 1
    if states > 1 and prev[states - 2] > prev[end]:
        end = states - 2
    if prev[end] == neg:
        raise AlignError("nincs érvényes illesztési útvonal")

    path = [0] * frames
    s = end
    for t in range(frames - 1, -1, -1):
        path[t] = s
        s -= back[t][s]

    first: dict[int, int] = {}
    last: dict[int, int] = {}
    score_sum: dict[int, float] = {}
    count: dict[int, int] = {}
    for t, s in enumerate(path):
        if s % 2 == 0:
            continue
        wi = owner[s // 2]
        if wi < 0:
            continue
        first.setdefault(wi, t)
        last[wi] = t
        score_sum[wi] = score_sum.get(wi, 0.0) + math.exp(log_probs[t][ext[s]])
        count[wi] = count.get(wi, 0) + 1
    result = []
    for wi, word in enumerate(words):
        if wi not in first:
            raise AlignError(f"a(z) {word!r} szó nem kapott kockát")
        result.append(Word(word, first[wi], last[wi] + 1, score_sum[wi] / count[wi]))
    return result


def log_softmax(row: Sequence[float]) -> list[float]:
    peak = max(row)
    total = math.log(sum(math.exp(x - peak) for x in row)) + peak
    return [x - total for x in row]
