#!/usr/bin/env python3
"""CFA4 F3 – a wav2vec2 felirat-időzítő mérése (docs/ai/modellvalasztas-091.md 4. pont).

Lépések (alparancsonként):
  export   a rögzített safetensors-ellenőrzőpontból ONNX (fp32 és fp16)
  parity   PyTorch és ONNX Runtime egyezése ugyanazon a hangon (logitok, illesztési útvonal)
  speed    valós idejű tényező és csúcs-VRAM egy hosszú hangon, 30 s-os ablakokkal
  accuracy szóhatárok és feliratsorok pontossága kézzel jelölt klipeken

A modellt a felhasználó tölti le (a CFA4 nem terjeszti), például:
  git clone https://huggingface.co/facebook/wav2vec2-base-960h && git -C wav2vec2-base-960h \
      checkout 22aad52d435eb6dbaf354bdad9b0da84ce7d6156
A minimumgépen így fut:  scripts/dev/run-minspec.sh uv run --project scripts/dev/f3-align \
      python scripts/dev/f3-align/measure.py speed --model ... --audio ...
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import threading
import time

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent))
import ctc_align as ca  # noqa: E402

REVISION = "22aad52d435eb6dbaf354bdad9b0da84ce7d6156"
WINDOW_S = 30.0
CONTEXT_S = 1.0


def load_audio(path: Path) -> np.ndarray:
    """Mono, 16 kHz, float32. Más mintavételt átmintavételezünk (a modell csak 16 kHz-en tanult)."""
    import soundfile as sf
    from math import gcd
    from scipy.signal import resample_poly

    data, rate = sf.read(str(path), dtype="float32", always_2d=True)
    mono = data.mean(axis=1)
    if rate != ca.SAMPLE_RATE:
        g = gcd(rate, ca.SAMPLE_RATE)
        mono = resample_poly(mono, ca.SAMPLE_RATE // g, rate // g).astype(np.float32)
    return mono


def normalize_wave(x: np.ndarray) -> np.ndarray:
    # A feature_extractor_config do_normalize=true: nulla átlag, egységnyi szórás.
    return ((x - x.mean()) / np.sqrt(x.var() + 1e-7)).astype(np.float32)


def load_vocab(model_dir: Path) -> dict[str, int]:
    return json.loads((model_dir / "vocab.json").read_text(encoding="utf-8"))


def sha256(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        for block in iter(lambda: f.read(1 << 20), b""):
            h.update(block)
    return h.hexdigest()


class VramSampler:
    """A saját folyamat GPU-memóriája nvidia-smi-vel, 50 ms-onként; a csúcsot tartja meg."""

    def __init__(self) -> None:
        self.peak_mb = 0
        self._stop = threading.Event()
        self._thread = threading.Thread(target=self._run, daemon=True)

    def _run(self) -> None:
        pid = str(os.getpid())
        while not self._stop.is_set():
            try:
                out = subprocess.run(
                    ["nvidia-smi", "--query-compute-apps=pid,used_memory",
                     "--format=csv,noheader,nounits"],
                    capture_output=True, text=True, timeout=5, check=False).stdout
            except (OSError, subprocess.TimeoutExpired):
                return
            for line in out.splitlines():
                parts = [p.strip() for p in line.split(",")]
                if len(parts) == 2 and parts[0] == pid and parts[1].isdigit():
                    self.peak_mb = max(self.peak_mb, int(parts[1]))
            self._stop.wait(0.05)

    def __enter__(self) -> "VramSampler":
        self._thread.start()
        return self

    def __exit__(self, *exc: object) -> None:
        self._stop.set()
        self._thread.join(timeout=10)


def ort_session(onnx_path: Path, device: str):
    import onnxruntime as ort

    providers = ["CPUExecutionProvider"]
    if device == "cuda":
        providers.insert(0, "CUDAExecutionProvider")
    return ort.InferenceSession(str(onnx_path), providers=providers)


def ort_logits(session, wave: np.ndarray) -> np.ndarray:
    dtype = np.float16 if session.get_inputs()[0].type == "tensor(float16)" else np.float32
    out = session.run(["logits"], {"input_values": wave[None, :].astype(dtype)})[0]
    return out[0].astype(np.float32)


def windowed_logits(session, wave: np.ndarray) -> np.ndarray:
    """30 s-os ablakok 1 s-os környezettel; minden ablakból csak a saját szakasza marad meg."""
    rate, step = ca.SAMPLE_RATE, ca.FRAME_SAMPLES
    total_frames = len(wave) // step
    win, ctx = int(WINDOW_S * rate), int(CONTEXT_S * rate)
    parts = []
    start = 0
    while start < len(wave):
        end = min(start + win, len(wave))
        lo, hi = max(0, start - ctx), min(len(wave), end + ctx)
        logits = ort_logits(session, normalize_wave(wave[lo:hi]))
        keep_from = (start - lo) // step
        keep = (end - start) // step
        parts.append(logits[keep_from:keep_from + keep])
        start = end
    out = np.concatenate(parts) if parts else np.zeros((0, 32), np.float32)
    return out[:total_frames]


def log_softmax(logits: np.ndarray) -> np.ndarray:
    peak = logits.max(axis=-1, keepdims=True)
    return logits - peak - np.log(np.exp(logits - peak).sum(axis=-1, keepdims=True))


def cmd_export(args: argparse.Namespace) -> dict:
    import torch
    from transformers import Wav2Vec2ForCTC

    weights = args.model / "model.safetensors"
    report = {"revision": REVISION, "safetensors_sha256": sha256(weights)}
    model = Wav2Vec2ForCTC.from_pretrained(str(args.model)).eval()
    args.out.mkdir(parents=True, exist_ok=True)
    for precision in ("fp32", "fp16"):
        dev = "cuda" if precision == "fp16" else "cpu"
        if dev == "cuda" and not torch.cuda.is_available():
            report[precision] = "kihagyva: nincs CUDA"
            continue
        m = model.to(dev)
        dummy = torch.zeros(1, ca.SAMPLE_RATE * 2, device=dev)
        if precision == "fp16":
            m, dummy = m.half(), dummy.half()
        path = args.out / f"wav2vec2-base-960h-{precision}.onnx"
        with torch.no_grad():
            torch.onnx.export(m, (dummy,), str(path), input_names=["input_values"],
                              output_names=["logits"], opset_version=17,
                              dynamic_axes={"input_values": {1: "samples"},
                                            "logits": {1: "frames"}})
        report[precision] = {"file": path.name, "sha256": sha256(path),
                             "size_mb": round(path.stat().st_size / 2**20)}
    return report


def cmd_parity(args: argparse.Namespace) -> dict:
    import torch
    from transformers import Wav2Vec2ForCTC

    wave = load_audio(args.audio)[: int(WINDOW_S * ca.SAMPLE_RATE)]
    x = normalize_wave(wave)
    model = Wav2Vec2ForCTC.from_pretrained(str(args.model)).eval()
    with torch.no_grad():
        ref = model(torch.from_numpy(x)[None, :]).logits[0].numpy()
    vocab = load_vocab(args.model)
    words, _ = ca.normalize(args.text.read_text(encoding="utf-8"), vocab)
    ref_words = ca.align(log_softmax(ref).tolist(), words, vocab, blank=vocab["<pad>"])
    report = {}
    for onnx_path in sorted(args.onnx_dir.glob("*.onnx")):
        got = ort_logits(ort_session(onnx_path, args.device), x)
        frames = min(len(got), len(ref))
        got_words = ca.align(log_softmax(got).tolist(), words, vocab, blank=vocab["<pad>"])
        report[onnx_path.name] = {
            "max_abs_logit_diff": float(np.abs(got[:frames] - ref[:frames]).max()),
            "same_argmax_ratio": float((got[:frames].argmax(-1) == ref[:frames].argmax(-1)).mean()),
            "same_alignment": [(w.start_frame, w.end_frame) for w in got_words]
                              == [(w.start_frame, w.end_frame) for w in ref_words],
        }
    return report


def cmd_speed(args: argparse.Namespace) -> dict:
    wave = load_audio(args.audio)
    seconds = len(wave) / ca.SAMPLE_RATE
    report = {"audio_s": round(seconds, 1), "device": args.device}
    for onnx_path in sorted(args.onnx_dir.glob("*.onnx")):
        with VramSampler() as vram:
            t0 = time.perf_counter()
            session = ort_session(onnx_path, args.device)
            t1 = time.perf_counter()
            logits = windowed_logits(session, wave)
            t2 = time.perf_counter()
        report[onnx_path.name] = {
            "load_ms": round((t1 - t0) * 1000),
            "rtf": round((t2 - t1) / seconds, 4) if seconds > 0 else None,
            "frames": int(len(logits)),
            "peak_vram_mb": vram.peak_mb if args.device == "cuda" else 0,
        }
    return report


def within(a: float, b: float, tol: float) -> bool:
    return abs(a - b) <= tol


def iou(a: tuple[float, float], b: tuple[float, float]) -> float:
    inter = max(0.0, min(a[1], b[1]) - max(a[0], b[0]))
    union = max(a[1], b[1]) - min(a[0], b[0])
    return inter / union if union > 0 else 0.0


def cmd_accuracy(args: argparse.Namespace) -> dict:
    """A címkefájl (JSON): [{"audio": "...", "lines": [{"text": "...", "start": s, "end": s,
    "words": [{"text": "...", "start": s, "end": s}]}]}]; az időket kézzel jelöltük."""
    labels = json.loads(args.labels.read_text(encoding="utf-8"))
    vocab = load_vocab(args.model)
    session = ort_session(next(iter(sorted(args.onnx_dir.glob("*fp32.onnx")))), args.device)
    word_hits = word_total = line_hits = line_total = 0
    ious, overlaps, failures = [], 0, []
    for clip in labels:
        wave = load_audio(args.labels.parent / clip["audio"])
        lp = log_softmax(windowed_logits(session, wave))
        prev_end = -1.0
        for line in clip["lines"]:
            words, _ = ca.normalize(line["text"], vocab)
            # Soronként, a kézi sor körül 2 s-os tűréssel illesztünk (a termék is így ablakol).
            lo = max(0, int((line["start"] - 2.0) * ca.SAMPLE_RATE / ca.FRAME_SAMPLES))
            hi = min(len(lp), int((line["end"] + 2.0) * ca.SAMPLE_RATE / ca.FRAME_SAMPLES))
            try:
                got = ca.align(lp[lo:hi].tolist(), words, vocab, blank=vocab["<pad>"])
            except ca.AlignError as err:
                failures.append({"audio": clip["audio"], "text": line["text"], "error": str(err)})
                continue
            offset = lo * ca.FRAME_SAMPLES / ca.SAMPLE_RATE
            refs = line.get("words", [])
            if refs and len(refs) != len(got):
                # A kézi szavakat a normalize() szerint kell felírni (például a számokat kiírva).
                failures.append({"audio": clip["audio"], "text": line["text"],
                                 "error": f"szószám eltér: {len(got)} ≠ {len(refs)}"})
                refs = []
            for g, ref in zip(got, refs):
                word_total += 2
                word_hits += within(g.start_s + offset, ref["start"], 0.05)
                word_hits += within(g.end_s + offset, ref["end"], 0.05)
            start, end = got[0].start_s + offset, got[-1].end_s + offset
            line_total += 2
            line_hits += within(start, line["start"], 0.10) + within(end, line["end"], 0.10)
            ious.append(iou((start, end), (line["start"], line["end"])))
            overlaps += start < prev_end
            prev_end = end
    return {
        "word_edges_within_50ms": word_hits / word_total if word_total else None,
        "line_edges_within_100ms": line_hits / line_total if line_total else None,
        "line_iou_mean": sum(ious) / len(ious) if ious else None,
        "overlapping_lines": overlaps,
        "failures": failures,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    sub = parser.add_subparsers(dest="cmd", required=True)
    for name in ("export", "parity", "speed", "accuracy"):
        p = sub.add_parser(name)
        p.add_argument("--model", type=Path, required=True,
                       help=f"a facebook/wav2vec2-base-960h mappája ({REVISION[:7]})")
        p.add_argument("--device", choices=("cuda", "cpu"), default="cuda")
        if name == "export":
            p.add_argument("--out", type=Path, required=True)
        else:
            p.add_argument("--onnx-dir", type=Path, required=True)
        if name in ("parity", "speed"):
            p.add_argument("--audio", type=Path, required=True)
        if name == "parity":
            p.add_argument("--text", type=Path, required=True)
        if name == "accuracy":
            p.add_argument("--labels", type=Path, required=True)
    args = parser.parse_args()
    handler = {"export": cmd_export, "parity": cmd_parity, "speed": cmd_speed,
               "accuracy": cmd_accuracy}[args.cmd]
    try:
        report = handler(args)
    except (ca.AlignError, OSError, ValueError, StopIteration) as err:
        print(json.dumps({"error": str(err)}, ensure_ascii=False), file=sys.stderr)
        return 1
    print(json.dumps(report, ensure_ascii=False, indent=2))
    return 0


if __name__ == "__main__":
    sys.exit(main())
