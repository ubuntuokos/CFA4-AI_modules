//! The outbox relay (ACS-4, PG-2): hands the `cfa4.execution.job.*` events written with each
//! state change to the Event Fabric.
//!
//! Delivery is **at least once**. A batch is locked (`FOR UPDATE SKIP LOCKED`, so several relays
//! never send the same row at the same time), handed to the publisher, and the rows it confirms
//! are marked published in the same transaction. If CFA4 stops between the publish and the commit,
//! those events go out again: consumers drop repeats by [`OutboxEvent::id`]. The order is the
//! outbox order (the commit order of the state changes) within one relay.
//!
//! The Event Fabric client (NATS JetStream, `CFA4-EVENT-FABRIC-001`) is the publisher's business;
//! this crate does not depend on it.

use std::future::Future;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{Store, StoreError};

/// Most events handed over in one batch.
pub const MAX_BATCH: usize = 1000;

/// One outbox row.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OutboxEvent {
    /// Increasing; the deduplication key for consumers.
    pub id: i64,
    pub task_id: String,
    /// `cfa4.execution.job.<state>` or `cfa4.execution.job.event`.
    pub subject: String,
    pub event: Value,
}

/// Relays the outbox of a store.
pub struct Relay {
    store: Store,
}

impl Store {
    pub fn outbox(&self) -> Relay {
        Relay { store: self.clone() }
    }
}

impl Relay {
    /// Hand up to `limit` unpublished events, oldest first, to `publish`. It answers how many of
    /// them, from the first, are now published (a failed publish answers how many went before
    /// it, or an error); exactly those are marked. Returns that number.
    pub async fn relay_once<F, Fut>(&self, limit: usize, publish: F) -> Result<usize, StoreError>
    where
        F: FnOnce(Vec<OutboxEvent>) -> Fut,
        Fut: Future<Output = Result<usize, String>>,
    {
        let limit = i64::try_from(limit.clamp(1, MAX_BATCH)).unwrap_or(1);
        let mut client = self.store.client().await?;
        let tx = client.transaction().await.map_err(StoreError::query)?;
        let rows = tx
            .query("SELECT id, task_id, subject, event FROM tasks.outbox WHERE published_at IS NULL ORDER BY id LIMIT $1 FOR UPDATE SKIP LOCKED", &[&limit])
            .await
            .map_err(StoreError::query)?;
        let mut events = Vec::with_capacity(rows.len());
        for row in rows {
            let id: i64 = row.try_get("id").map_err(StoreError::query)?;
            let corrupt = |e: tokio_postgres::Error| StoreError::Corrupt { id: format!("outbox {id}"), detail: e.to_string() };
            events.push(OutboxEvent {
                id,
                task_id: row.try_get("task_id").map_err(corrupt)?,
                subject: row.try_get("subject").map_err(corrupt)?,
                event: row.try_get("event").map_err(corrupt)?,
            });
        }
        if events.is_empty() {
            return Ok(0);
        }
        let ids: Vec<i64> = events.iter().map(|e| e.id).collect();
        let sent = publish(events).await.map_err(|detail| StoreError::Query { detail: format!("publish: {detail}") })?.min(ids.len());
        let done = ids.get(..sent).unwrap_or_default();
        if !done.is_empty() {
            tx.execute("UPDATE tasks.outbox SET published_at = now() WHERE id = ANY($1)", &[&done]).await.map_err(StoreError::query)?;
        }
        tx.commit().await.map_err(StoreError::query)?;
        Ok(sent)
    }

    /// Unpublished events waiting.
    pub async fn pending(&self) -> Result<i64, StoreError> {
        let client = self.store.client().await?;
        client.query_one("SELECT count(*) FROM tasks.outbox WHERE published_at IS NULL", &[]).await.and_then(|r| r.try_get(0)).map_err(StoreError::query)
    }

    /// Delete events published more than `older_than_days` days ago; how many went.
    pub async fn prune(&self, older_than_days: u32) -> Result<u64, StoreError> {
        let client = self.store.client().await?;
        let days = i32::try_from(older_than_days).unwrap_or(i32::MAX);
        client
            .execute("DELETE FROM tasks.outbox WHERE published_at IS NOT NULL AND published_at < now() - make_interval(days => $1)", &[&days])
            .await
            .map_err(StoreError::query)
    }
}
