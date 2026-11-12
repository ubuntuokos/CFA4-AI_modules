# cfa4-store

CFA4's PostgreSQL client (CFA4-TASKSTORE-PG-001, phases PG-1 and PG-2; plan:
[`docs/ai/postgresql-adatreteg.md`](../../docs/ai/postgresql-adatreteg.md)). Layer L5: it talks to
another process over a socket.

- **Driver:** `tokio-postgres` with the `deadpool-postgres` pool (owner decision 2026-11-11, shared
  with the BacklotHub). PostgreSQL runs as its own process; CFA4 is only a client, in pure Rust.
- **`Store::connect`:** opens the pool and checks one connection. A missing or stopped server is
  `StoreError::Unavailable` with the install hint; editing, playback and saving never need it.
- **Migrations** (`migrate`): embedded steps per component, each with an `up` and a `down`, applied
  in one transaction under an advisory lock and recorded in `cfa4_store.migration`;
  `Store::rollback` takes one step back.
- **`tasks` schema** (`TaskStore`): the `cfa4-tasks` queue in PostgreSQL. Each operation locks the
  rows it needs (`FOR UPDATE`; the scheduler step with `SKIP LOCKED`), runs the queue's own rule
  (`cfa4_tasks::Queue`), writes back the changed tasks (version-checked), their transitions
  (`tasks.transition`) and one Event Fabric outbox row per transition (`tasks.outbox`,
  `cfa4.execution.job.<state>`), and commits. The resource manager changes on a copy that replaces
  the caller's only after the commit. New tasks `NOTIFY cfa4_tasks`.

Tests start a throw-away cluster (`initdb` / `pg_ctl` in a temporary directory, Unix socket only;
as root through `runuser -u postgres`). Without PostgreSQL binaries they print a note and skip
(`CFA4_PG_BIN` points at another `bin` directory).
