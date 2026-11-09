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
let mut manager = cfa4_tasks::Manager::new(cfa4_tasks::Budget::new(&snap.nominal, &Default::default()));
let mut monitor = cfa4_tasks::Monitor::default();
for event in monitor.observe(&snap) { log::info!("{event:?}"); }
let request = cfa4_tasks::Request { class: cfa4_tasks::Class::Background, ram_bytes: 4 << 30, cpu_milli: 2000, gpu: None };
match manager.try_reserve(&request, &snap, &monitor) { Ok(grant) => { /* run, then */ manager.release(grant.id); } Err(denied) => log::info!("{denied:?}") }
```

## Pieces

| module | what |
|---|---|
| `resources` | `Nominal` (physical RAM, cgroup `memory.max` and `cpu.max` of this process and its ancestors, online CPUs, every GPU's VRAM and display use) and `Current` (`MemAvailable`, cgroup `memory.current`, swap in use, PSI memory / cpu / io `avg10`, VRAM in use, GPU load) as separate types in one `Snapshot`. `apply_gpu_report` fills VRAM that sysfs does not publish (NVIDIA, Intel) from NVML or a worker. |
| `class` | `ram_profile`: below minimum (< 16 GB), minimum (16 GB), recommended (32 GB), on the effective RAM, so `run-minspec.sh` profiles classify right. `ai_floor`: 16 GB RAM and a 12 GB GPU that does not drive a display; unknown numbers give `Unknown`, never "enough". |
| `monitor` | `Monitor::observe`: nominal changes (GPU added / removed, limits, VRAM) once; per signal (memory / CPU / I/O pressure, low memory, each GPU's VRAM) `Normal → Spike → Sustained` with hysteresis and `sustain_samples`, one event per change of level. Pure state machine; the caller owns the timer. |
| `budget` | (ACS-2) `Budget::new(nominal, settings)`: RAM = effective RAM minus a reserve (default: the larger of 2 GiB and 10 %), optionally capped; CPU = effective CPUs minus a reserve (default one CPU); per GPU its VRAM minus a reserve (default 512 MiB). No budget for a GPU with unknown VRAM, one the user did not select, or one that drives a display while another usable GPU exists (on a single-display-GPU machine only with `allowDisplayGpu`); every exclusion has a reason. |
| `manager` | (ACS-2) `Manager::try_reserve(request, snapshot, monitor)`: RAM, CPU and VRAM on one GPU **all at once or nothing** (no hold and wait). Checks the budget (`NeverFits`: will not run here as asked), the other reservations, the current free memory and VRAM minus the reserves, and pressure by class (memory pressure or low memory hold back everything but `Interactive`; sustained CPU / I/O pressure holds back `Speculative`; a GPU with sustained VRAM pressure takes no new work); otherwise `Wait` with every reason. Picks the GPU with the most room. `release(id)`. |

GPU sysfs reading (VRAM total / used, load, display connectors) is `cfa4_accel::memory`.

Everything is serde (camelCase) for the control channel and MCP. Every read is bounded, every
number checked; a missing or hostile file is `None` (unknown), never zero and never a panic.
