# cfa4-ai-models

CFA4 model manager, phase MM-F1 (plan: `docs/ai/CFA4-MODEL-MANAGER-001.md`, ADR 0008 §2). Layer L2.

This crate is the registry: model records, pinned revisions, artifacts with SHA-256, source trust and
status, the four licence parts, the 13 stored model states, format detection from the bytes, and an
in-memory catalogue with filters. It has **no network and never downloads**: download is a `cfa4-tasks`
task type that calls `cfa4-fetch` only on the user's explicit command.

Next phases: MM-F2 store, staging, quarantine and the user-chosen storage location (AC-25, PostgreSQL
`ai.model*` tables); MM-F3 import, scan (modelscan, D11) and managed download; llmfit verdicts (D10, AC-27).
