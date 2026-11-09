# cfa4-tasks

CFA4-wide task and resource infrastructure, the Adaptive Capability System (ACS; layer L2; no UI,
no FFI, no `unsafe`). Every resource-hungry module (AI, render, proxies, codec conversion, Media
Intelligence) uses this one crate instead of its own resource manager; `cfa4-ai-jobs` will be a
client. Plan and phases: [`docs/ai/acs-adaptiv-eroforras.md`](../../docs/ai/acs-adaptiv-eroforras.md).

```rust
let snap = cfa4_tasks::sample();                       // nominal + current, from /proc, /sys, cgroup v2
let ram = snap.nominal.ram_effective_bytes();          // physical RAM or the cgroup limit, the smaller
let free = snap.ram_free_effective_bytes();            // MemAvailable or what is left under the limit
let floor = cfa4_tasks::ai_floor(&snap.nominal);       // Met / NotMet { reasons } / Unknown { missing }
let mut monitor = cfa4_tasks::Monitor::default();
for event in monitor.observe(&snap) { log::info!("{event:?}"); }
```

## Pieces (ACS-1)

| module | what |
|---|---|
| `resources` | `Nominal` (physical RAM, cgroup `memory.max` and `cpu.max` of this process and its ancestors, online CPUs, every GPU's VRAM and display use) and `Current` (`MemAvailable`, cgroup `memory.current`, swap in use, PSI memory / cpu / io `avg10`, VRAM in use, GPU load) as separate types in one `Snapshot`. `apply_gpu_report` fills VRAM that sysfs does not publish (NVIDIA, Intel) from NVML or a worker. |
| `class` | `ram_profile`: below minimum (< 16 GB), minimum (16 GB), recommended (32 GB), on the effective RAM, so `run-minspec.sh` profiles classify right. `ai_floor`: 16 GB RAM and a 12 GB GPU that does not drive a display; unknown numbers give `Unknown`, never "enough". |
| `monitor` | `Monitor::observe`: nominal changes (GPU added / removed, limits, VRAM) once; per signal (memory / CPU / I/O pressure, low memory, each GPU's VRAM) `Normal → Spike → Sustained` with hysteresis and `sustain_samples`, one event per change of level. Pure state machine; the caller owns the timer. |

GPU sysfs reading (VRAM total / used, load, display connectors) is `cfa4_accel::memory`.

Everything is serde (camelCase) for the control channel and MCP. Every read is bounded, every
number checked; a missing or hostile file is `None` (unknown), never zero and never a panic.
