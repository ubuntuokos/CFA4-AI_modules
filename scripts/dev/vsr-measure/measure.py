#!/usr/bin/env python3
"""CFA4 #91 – a Real-ESRGAN felskálázó mérése (docs/ai/modellvalasztas-091.md 4.1 pont).

Lépések (alparancsonként):
  export   a rögzített .pth-súlyból ONNX (fp32 CPU-n, fp16 CUDA-n)
  parity   PyTorch és ONNX Runtime egyezése ugyanazon a képkockán (PSNR)
  speed    képkocka/s és csúcs-VRAM egy klipen, csempemérettel (256, 512)

A súlyt a felhasználó tölti le (a CFA4 nem terjeszti); csak a zárt, fizetős workerhez mérjük, mert
a tanítóadat licence a GPL-es magban blokkol (modellválasztás 1. pont). A bemenet nyers rgb24
képkockák sora, például:
  ffmpeg -i klip.mov -t 10 -vf scale=1024:540:flags=area -pix_fmt rgb24 -f rawvideo klip.rgb
A minimumgépen így fut:  scripts/dev/run-minspec.sh uv run --project scripts/dev/vsr-measure \
      python scripts/dev/vsr-measure/measure.py speed --weights ... --input klip.rgb ...
"""
from __future__ import annotations

import argparse
import contextlib
import gc
import hashlib
import json
import math
import os
from pathlib import Path
import subprocess
import sys
import threading
import time

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent))
import tiles as tl  # noqa: E402

# A Real-ESRGAN GitHub-kiadásainak súlyai (BSD-3-Clause); csak ezeket fogadjuk el.
WEIGHTS = {
    "49fafd45f8fd7aa8d31ab2a22d14d91b536c34494a5cfe31eb5d89c2fa266abb": ("RealESRGAN_x2plus", 2),
    "4fa0d38905f75ac06eb49a7951b426670021be3018265fd191d2125df9d682f1": ("RealESRGAN_x4plus", 4),
}
MAX_WEIGHTS_BYTES = 256 << 20
MAX_FRAMES = 2000
MIN_SIDE = 16
MAX_OUT_SIDE = 8192
TILE_PAD = 10
DEFAULT_TILES = (256, 512)
PARITY_MIN_PSNR_DB = 45.0
PARITY_MAX_FRAMES = 8


def sha256(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        for block in iter(lambda: f.read(1 << 20), b""):
            h.update(block)
    return h.hexdigest()


def identify_weights(path: Path) -> tuple[str, int, str]:
    """A súlyfájl neve, nagyítása és hash-e; ismeretlen vagy módosított fájlt nem fogadunk el."""
    size = path.stat().st_size
    if size > MAX_WEIGHTS_BYTES:
        raise ValueError(f"{path}: a súlyfájl nagyobb {MAX_WEIGHTS_BYTES} bájtnál")
    digest = sha256(path)
    known = WEIGHTS.get(digest)
    if known is None:
        raise ValueError(f"{path}: ismeretlen súlyfájl (sha256 {digest}); a README-ben leírt "
                         "RealESRGAN_x2plus.pth vagy RealESRGAN_x4plus.pth kell")
    return known[0], known[1], digest


def load_torch_model(path: Path):
    import torch

    from rrdbnet import RRDBNet

    name, scale, digest = identify_weights(path)
    try:
        # weights_only: a .pth pickle; így csak tenzorokat és alaptípusokat tölt be, kódot nem.
        state = torch.load(str(path), map_location="cpu", weights_only=True)
    except Exception as err:  # noqa: BLE001 – a torch.load sokféle kivételt ad sérült fájlra
        raise ValueError(f"{path}: a súlyfájl nem tölthető be: {err}") from err
    if isinstance(state, dict):
        state = state.get("params_ema", state.get("params", state))
    if not isinstance(state, dict):
        raise ValueError(f"{path}: a súlyfájl nem állapotszótár")
    model = RRDBNet(scale)
    try:
        model.load_state_dict(state, strict=True)
    except (RuntimeError, KeyError) as err:
        raise ValueError(f"{path}: a súlyok nem illenek az RRDBNet-re: {err}") from err
    return model.eval(), name, scale, digest


class VramSampler:
    """A saját folyamat GPU-memóriája nvidia-smi-vel, 50 ms-onként (ugyanaz, mint az F3-mérésben)."""

    def __init__(self) -> None:
        self.peak_mb: int | None = None
        self.error: str | None = None
        self._stop = threading.Event()
        self._thread = threading.Thread(target=self._run, daemon=True)

    def _run(self) -> None:
        pid = str(os.getpid())
        while not self._stop.is_set():
            try:
                proc = subprocess.run(
                    ["nvidia-smi", "--query-compute-apps=pid,used_memory",
                     "--format=csv,noheader,nounits"],
                    capture_output=True, text=True, timeout=5, check=False)
            except (OSError, subprocess.TimeoutExpired) as err:
                self.error = f"nvidia-smi nem futott: {err}"
                return
            if proc.returncode != 0:
                self.error = (f"nvidia-smi hibával lépett ki ({proc.returncode}): "
                              f"{proc.stderr.strip()[:200]}")
                return
            for line in proc.stdout.splitlines():
                parts = [p.strip() for p in line.split(",")]
                if len(parts) == 2 and parts[0] == pid and parts[1].isdigit():
                    self.peak_mb = max(self.peak_mb or 0, int(parts[1]))
            self._stop.wait(0.05)

    def __enter__(self) -> "VramSampler":
        self._thread.start()
        return self

    def __exit__(self, *exc: object) -> None:
        self._stop.set()
        self._thread.join(timeout=10)
        if self.peak_mb is None and self.error is None:
            self.error = "az nvidia-smi nem adott használható sort ehhez a folyamathoz"


def ort_session(onnx_path: Path, device: str):
    if device == "cuda":
        # Az ONNX Runtime CUDA-futtatója a torch-csal érkező CUDA/cuDNN-könyvtárakat így találja meg.
        try:
            import torch  # noqa: F401
        except ImportError:
            pass
    import onnxruntime as ort

    providers = ["CPUExecutionProvider"]
    if device == "cuda":
        providers.insert(0, "CUDAExecutionProvider")
    try:
        session = ort.InferenceSession(str(onnx_path), providers=providers)
    except Exception as err:  # noqa: BLE001 – az ONNX Runtime saját kivételtípusai
        raise ValueError(f"{onnx_path.name}: az ONNX-modell nem tölthető be: {err}") from err
    active = session.get_providers()
    if device == "cuda" and (not active or active[0] != "CUDAExecutionProvider"):
        raise ValueError(f"{onnx_path.name}: a CUDA nem aktív (aktív: {active})")
    inputs, outputs = session.get_inputs(), session.get_outputs()
    if [i.name for i in inputs] != ["image"] or [o.name for o in outputs] != ["upscaled"]:
        raise ValueError(f"{onnx_path.name}: nem a várt image→upscaled felületű modell")
    return session


def onnx_models(onnx_dir: Path, device: str) -> list[Path]:
    paths = sorted(onnx_dir.glob("*.onnx"))
    if device == "cpu":
        paths = [p for p in paths if "fp16" not in p.name]
    if not paths:
        raise ValueError(f"nincs mérhető ONNX-fájl itt: {onnx_dir}")
    return paths


def onnx_scale(path: Path) -> int:
    for name, scale in WEIGHTS.values():
        if path.name.startswith(name + "-"):
            return scale
    raise ValueError(f"{path.name}: a fájlnévből nem derül ki a nagyítás (export kimenete kell)")


def open_frames(path: Path, width: int, height: int, scale: int, limit: int | None) -> np.ndarray:
    """Nyers rgb24 képkockák memóriába képezve: [N, H, W, 3] uint8."""
    for name, value in (("width", width), ("height", height)):
        if not MIN_SIDE <= value <= MAX_OUT_SIDE // scale:
            raise ValueError(f"--{name}: {MIN_SIDE} és {MAX_OUT_SIDE // scale} között kell legyen "
                             f"({scale}× nagyításnál), nem {value}")
    frame_bytes = width * height * 3
    size = path.stat().st_size
    if size == 0 or size % frame_bytes:
        raise ValueError(f"{path}: a méret ({size} bájt) nem egész számú {width}×{height}-es "
                         "rgb24 képkocka; jó-e a --width és a --height?")
    count = size // frame_bytes
    if count > MAX_FRAMES:
        raise ValueError(f"{path}: {count} képkocka; legfeljebb {MAX_FRAMES} (vágd le a klipet)")
    if limit is not None:
        count = min(count, limit)
    return np.memmap(path, dtype=np.uint8, mode="r", shape=(count, height, width, 3))


def to_input(frame: np.ndarray, dtype) -> np.ndarray:
    """[H, W, 3] uint8 → [1, 3, H, W], 0..1 között."""
    return np.ascontiguousarray(frame.transpose(2, 0, 1)[None].astype(np.float32) / 255.0,
                                dtype=dtype)


def upscale(run, frame: np.ndarray, tile: int, scale: int, dtype) -> np.ndarray:
    """Egy képkocka felskálázása csempénként; [3, H*s, W*s] float32, 0..1-re vágva."""
    height, width = frame.shape[:2]
    out = np.empty((3, height * scale, width * scale), dtype=np.float32)
    for t in tl.plan(height, width, tile, TILE_PAD, scale):
        x = to_input(frame[t.in_y0:t.in_y1, t.in_x0:t.in_x1], dtype)
        y = run(x)
        want = (1, 3, (t.in_y1 - t.in_y0) * scale, (t.in_x1 - t.in_x0) * scale)
        if tuple(y.shape) != want:
            raise ValueError(f"a modell kimenete {tuple(y.shape)}, várt: {want}")
        y = y[0, :, t.crop_y0:t.crop_y1, t.crop_x0:t.crop_x1].astype(np.float32)
        if not np.isfinite(y).all():
            raise ValueError("a modell kimenetében nem véges érték van")
        out[:, t.out_y0:t.out_y1, t.out_x0:t.out_x1] = y
    return np.clip(out, 0.0, 1.0)


def ort_runner(session):
    dtype = np.float16 if session.get_inputs()[0].type == "tensor(float16)" else np.float32

    def run(x: np.ndarray) -> np.ndarray:
        try:
            return session.run(["upscaled"], {"image": x})[0]
        except Exception as err:  # noqa: BLE001 – az ONNX Runtime futási hibái
            raise ValueError(f"az ONNX Runtime futása hibát adott: {err}") from err

    return run, dtype


def cmd_export(args: argparse.Namespace) -> dict:
    import torch

    model, name, scale, digest = load_torch_model(args.weights)
    report: dict = {"weights": name, "scale": scale, "weights_sha256": digest}
    args.out.mkdir(parents=True, exist_ok=True)
    for precision in ("fp32", "fp16"):
        dev = "cuda" if precision == "fp16" else "cpu"
        path = args.out / f"{name}-{precision}.onnx"
        path.unlink(missing_ok=True)  # egy korábbi export maradékát ne mérjük
        if dev == "cuda" and not torch.cuda.is_available():
            report[precision] = "kihagyva: nincs CUDA"
            continue
        m = model.to(dev)
        dummy = torch.zeros(1, 3, 64, 64, device=dev)
        if precision == "fp16":
            m, dummy = m.half(), dummy.half()
        try:
            with torch.no_grad():
                # dynamo=False: a TorchScript-alapú exportáló (a torch 2.5-ben ez az alapértelmezés).
                torch.onnx.export(m, (dummy,), str(path), input_names=["image"],
                                  output_names=["upscaled"], opset_version=17, dynamo=False,
                                  dynamic_axes={"image": {2: "height", 3: "width"},
                                                "upscaled": {2: "out_height", 3: "out_width"}})
        except Exception as err:  # noqa: BLE001 – az exportáló sokféle kivételt ad
            path.unlink(missing_ok=True)
            raise ValueError(f"{path.name}: az ONNX-export nem sikerült: {err}") from err
        report[precision] = {"file": path.name, "sha256": sha256(path),
                             "size_mb": round(path.stat().st_size / 2**20)}
        model = model.float().to("cpu")
    return report


def cmd_parity(args: argparse.Namespace) -> dict:
    import torch

    model, name, scale, digest = load_torch_model(args.weights)
    frames = open_frames(args.input, args.width, args.height, scale, args.frames)
    ref_dev = "cuda" if torch.cuda.is_available() else "cpu"
    # A referencia tiszta fp32 legyen: a cuDNN alapból TF32-vel számolná a konvolúciót.
    torch.backends.cudnn.allow_tf32 = False
    torch.backends.cuda.matmul.allow_tf32 = False
    model = model.to(ref_dev)

    def torch_run(x: np.ndarray) -> np.ndarray:
        with torch.no_grad():
            return model(torch.from_numpy(x).to(ref_dev)).cpu().numpy()

    refs = [upscale(torch_run, f, args.tile, scale, np.float32) for f in frames]
    report: dict = {"weights": name, "weights_sha256": digest, "reference_device": ref_dev,
                    "frames": len(refs), "tile": args.tile, "min_psnr_db": PARITY_MIN_PSNR_DB}
    for onnx_path in onnx_models(args.onnx_dir, args.device):
        if onnx_scale(onnx_path) != scale:
            continue  # másik súly exportja
        run, dtype = ort_runner(ort_session(onnx_path, args.device))
        psnrs = []
        for frame, ref in zip(frames, refs):
            got = upscale(run, frame, args.tile, scale, dtype)
            psnrs.append(tl.psnr(float(np.mean((got.astype(np.float64) - ref) ** 2))))
        worst = min(psnrs)
        report[onnx_path.name] = {
            "psnr_db_min": None if math.isinf(worst) else round(worst, 2),
            "psnr_db_mean": None if any(math.isinf(p) for p in psnrs)
            else round(sum(psnrs) / len(psnrs), 2),
            "identical": math.isinf(worst),
            "parity_ok": worst >= PARITY_MIN_PSNR_DB,
        }
    return report


def cmd_speed(args: argparse.Namespace) -> dict:
    _, scale, digest = identify_weights(args.weights)
    frames = open_frames(args.input, args.width, args.height, scale, args.frames)
    report: dict = {"weights_sha256": digest, "device": args.device, "frames": len(frames),
                    "input": f"{args.width}x{args.height}",
                    "output": f"{args.width * scale}x{args.height * scale}"}
    for onnx_path in onnx_models(args.onnx_dir, args.device):
        if onnx_scale(onnx_path) != scale:
            continue
        for tile in args.tiles:
            vram = VramSampler() if args.device == "cuda" else None
            with vram if vram is not None else contextlib.nullcontext():
                t0 = time.perf_counter()
                session = ort_session(onnx_path, args.device)
                run, dtype = ort_runner(session)
                upscale(run, frames[0], tile, scale, dtype)  # bemelegítés: a CUDA-kernelek kiválasztása
                t1 = time.perf_counter()
                for frame in frames:
                    upscale(run, frame, tile, scale, dtype)
                t2 = time.perf_counter()
            elapsed = t2 - t1
            report[f"{onnx_path.name}@tile{tile}"] = {
                "load_and_warmup_ms": round((t1 - t0) * 1000),
                "fps": round(len(frames) / elapsed, 3) if elapsed > 0 else None,
                "s_per_frame": round(elapsed / len(frames), 3),
                "peak_vram_mb": vram.peak_mb if vram is not None else 0,
                "vram_error": vram.error if vram is not None and vram.peak_mb is None else None,
            }
            del session, run
            gc.collect()
    return report


def tile_size(text: str) -> int:
    value = int(text)
    if not 32 <= value <= 2048 or value % 2:
        raise argparse.ArgumentTypeError("a csempe 32 és 2048 közötti páros szám legyen")
    return value


def frame_limit(text: str) -> int:
    value = int(text)
    if not 1 <= value <= MAX_FRAMES:
        raise argparse.ArgumentTypeError(f"1 és {MAX_FRAMES} között legyen")
    return value


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    sub = parser.add_subparsers(dest="cmd", required=True)
    for name in ("export", "parity", "speed"):
        p = sub.add_parser(name)
        p.add_argument("--weights", type=Path, required=True,
                       help="RealESRGAN_x2plus.pth vagy RealESRGAN_x4plus.pth")
        if name == "export":
            p.add_argument("--out", type=Path, required=True)
            continue
        p.add_argument("--device", choices=("cuda", "cpu"), default="cuda")
        p.add_argument("--onnx-dir", type=Path, required=True)
        p.add_argument("--input", type=Path, required=True, help="nyers rgb24 képkockák")
        p.add_argument("--width", type=int, required=True)
        p.add_argument("--height", type=int, required=True)
        if name == "parity":
            p.add_argument("--tile", type=tile_size, default=512)
            p.add_argument("--frames", type=frame_limit, default=3,
                           help=f"ennyi képkockán (legfeljebb {PARITY_MAX_FRAMES})")
        else:
            p.add_argument("--tiles", type=tile_size, nargs="+", default=list(DEFAULT_TILES))
            p.add_argument("--frames", type=frame_limit, default=None,
                           help="csak az első N képkocka (CPU-n érdemes)")
    args = parser.parse_args()
    if args.cmd == "parity" and args.frames > PARITY_MAX_FRAMES:
        parser.error(f"--frames: a parity legfeljebb {PARITY_MAX_FRAMES} képkockán fut")
    handler = {"export": cmd_export, "parity": cmd_parity, "speed": cmd_speed}[args.cmd]
    try:
        report = handler(args)
    except (OSError, ValueError, OverflowError, MemoryError) as err:
        print(json.dumps({"error": str(err)}, ensure_ascii=False), file=sys.stderr)
        return 1
    print(json.dumps(report, ensure_ascii=False, indent=2))
    return 0


if __name__ == "__main__":
    sys.exit(main())
