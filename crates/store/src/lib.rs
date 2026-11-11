//! CFA4's PostgreSQL client (CFA4-TASKSTORE-PG-001, PG-1 and PG-2): the connection pool, the
//! migrations and the typed `tasks` schema of the shared CFA4 cluster.
//!
//! The driver is `tokio-postgres` with the `deadpool-postgres` pool (owner decision,
//! 2026-11-11; shared with the BacklotHub). PostgreSQL is a separate process (PostgreSQL License,
//! GPL-compatible); CFA4 only connects to it as a client, in pure Rust.
//!
//! - [`Store::connect`] opens the pool and checks one connection. A missing or stopped server is
//!   [`StoreError::Unavailable`] with how to install it: editing, playback and saving never
//!   depend on the database, only the queue and the AI do (`AGENTS.md` §0).
//! - [`Store::migrate`] brings every component's schema up to date, [`Store::rollback`] takes
//!   one step back; each step runs in its own transaction.
//! - [`tasks::TaskStore`] keeps the `cfa4-tasks` queue in the `tasks` schema. The queue's rules
//!   live in [`cfa4_tasks::Queue`] only: every operation locks the rows it touches
//!   (`FOR UPDATE`, the scheduler with `SKIP LOCKED`), runs the same rule on them, and writes back
//!   the changed tasks, their transitions and the outbox events in the same transaction. If the
//!   transaction does not commit, nothing happened, so a task never starts twice.
//!
//! The layer is L5 (like `cfa4-platform`): it talks to another process over a socket, which the
//! L0–L4 WASM check cannot build.

#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

pub mod migrate;
pub mod tasks;

use std::time::Duration;

use deadpool_postgres::{Pool, PoolConfig, Runtime, Timeouts};
use serde::{Deserialize, Serialize};
use tokio_postgres::NoTls;

pub use tasks::TaskStore;

/// How to reach the CFA4 cluster. The defaults are the local Unix socket of a Debian/Ubuntu
/// PostgreSQL (`/var/run/postgresql`, port 5432) and the `cfa4` database and role.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct StoreConfig {
    /// A host name, or a directory for a Unix socket (starts with `/`).
    pub host: String,
    pub port: u16,
    pub user: String,
    pub dbname: String,
    /// Not needed with peer authentication over the local socket.
    #[serde(skip_serializing)]
    pub password: Option<String>,
    /// Connections in the pool (few: the 16 GB minimum machine, PG-001 §8).
    pub pool_size: usize,
    pub connect_timeout_ms: u64,
}

impl Default for StoreConfig {
    fn default() -> Self {
        StoreConfig {
            host: "/var/run/postgresql".into(),
            port: 5432,
            user: "cfa4".into(),
            dbname: "cfa4".into(),
            password: None,
            pool_size: 4,
            connect_timeout_ms: 3_000,
        }
    }
}

/// Every failure is a value: the caller shows it, CFA4 keeps running.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", rename_all_fields = "camelCase", tag = "kind")]
pub enum StoreError {
    /// No server to talk to (not installed, not running, wrong socket or credentials).
    Unavailable { detail: String, hint: String },
    /// A migration step failed; the transaction was rolled back.
    Migration { component: String, version: u32, detail: String },
    /// A query failed (the transaction was rolled back).
    Query { detail: String },
    /// A stored row does not decode: reported, never trusted, never a panic.
    Corrupt { id: String, detail: String },
    /// The queue refused the operation.
    Queue { error: cfa4_tasks::QueueError },
}

/// Shown when the server cannot be reached.
pub const INSTALL_HINT: &str = "CFA4's task queue and AI features need a running PostgreSQL: on Ubuntu `sudo apt install postgresql`, then `sudo -u postgres createuser cfa4 && sudo -u postgres createdb -O cfa4 cfa4`. Editing, playback and saving work without it.";

impl StoreError {
    pub(crate) fn query(e: impl std::fmt::Display) -> Self {
        StoreError::Query { detail: e.to_string() }
    }
}

impl From<cfa4_tasks::QueueError> for StoreError {
    fn from(error: cfa4_tasks::QueueError) -> Self {
        StoreError::Queue { error }
    }
}

impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StoreError::Unavailable { detail, hint } => write!(f, "PostgreSQL is not available ({detail}). {hint}"),
            StoreError::Migration { component, version, detail } => write!(f, "migration {component} {version} failed: {detail}"),
            StoreError::Query { detail } => write!(f, "query failed: {detail}"),
            StoreError::Corrupt { id, detail } => write!(f, "stored row {id} does not decode: {detail}"),
            StoreError::Queue { error } => write!(f, "queue: {error:?}"),
        }
    }
}

impl std::error::Error for StoreError {}

/// The pool over the CFA4 cluster.
#[derive(Clone)]
pub struct Store {
    pool: Pool,
}

impl Store {
    /// Open the pool and check that one connection works.
    pub async fn connect(config: &StoreConfig) -> Result<Store, StoreError> {
        let unavailable = |detail: String| StoreError::Unavailable { detail, hint: INSTALL_HINT.into() };
        let timeout = Duration::from_millis(config.connect_timeout_ms.max(1));
        let mut pg = deadpool_postgres::Config::new();
        pg.host = Some(config.host.clone());
        pg.port = Some(config.port);
        pg.user = Some(config.user.clone());
        pg.dbname = Some(config.dbname.clone());
        pg.password = config.password.clone();
        pg.application_name = Some("cfa4".into());
        pg.connect_timeout = Some(timeout);
        let mut pool_config = PoolConfig::new(config.pool_size.clamp(1, 64));
        pool_config.timeouts = Timeouts { wait: Some(timeout), create: Some(timeout), recycle: Some(timeout) };
        pg.pool = Some(pool_config);
        let pool = pg.create_pool(Some(Runtime::Tokio1), NoTls).map_err(|e| unavailable(e.to_string()))?;
        let client = pool.get().await.map_err(|e| unavailable(e.to_string()))?;
        client.simple_query("SELECT 1").await.map_err(|e| unavailable(e.to_string()))?;
        Ok(Store { pool })
    }

    pub(crate) async fn client(&self) -> Result<deadpool_postgres::Object, StoreError> {
        self.pool.get().await.map_err(|e| StoreError::Unavailable { detail: e.to_string(), hint: INSTALL_HINT.into() })
    }

    /// Bring every component's schema up to date; the versions applied now.
    pub async fn migrate(&self) -> Result<Vec<(String, u32)>, StoreError> {
        migrate::up(self).await
    }

    /// Undo the newest migration step of `component`; the version undone, `None` when nothing
    /// was applied.
    pub async fn rollback(&self, component: &str) -> Result<Option<u32>, StoreError> {
        migrate::down(self, component).await
    }

    /// The task queue in the `tasks` schema.
    pub fn tasks(&self, config: cfa4_tasks::QueueConfig) -> TaskStore {
        TaskStore::new(self.clone(), config)
    }
}

#[cfg(test)]
pub(crate) mod testdb;
