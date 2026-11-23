# cfa4-ai-models

CFA4 model manager, phase MM-F1 (plan: `docs/ai/CFA4-MODEL-MANAGER-001.md`, ADR 0008 §2). Layer L2.

This crate is the registry: model records, pinned revisions, artifacts with SHA-256, source trust and
status, the four licence parts, the 13 stored model states, format detection from the bytes, and an
in-memory catalogue with filters. It has **no network and never downloads**: download is a `cfa4-tasks`
task type that calls `cfa4-fetch` only on the user's explicit command.

MM-F2 (first part): `location` — the user-chosen store (no silent default, free-space and writable checks, XDG only as a suggestion), `cfa4-models/`, `.staging/`, `.quarantine/` under the chosen root, hash-checked promotion by rename, one copy per hash, quarantine that never overwrites.

MM-F3 (first part): `import` — `import_staged` checks size, SHA-256, the real format and structure of a staged file, runs the `Scanner` (modelscan, D11) for pickle and HDF5, then promotes, quarantines (findings, per-hash user override) or refuses (file stays in staging); no scanner installed means pickle/HDF5 import is refused, other formats work (AC-28).

Next: PostgreSQL `ai.model*` tables (PG-3), explicit move of a store, per-model store choice; MM-F3 import, scan (modelscan, D11) and managed download; llmfit verdicts (D10, AC-27).
