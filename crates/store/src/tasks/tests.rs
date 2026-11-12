use cfa4_accel::Vendor;
use cfa4_tasks::budget::{Budget, BudgetSettings};
use cfa4_tasks::manager::{Class, GpuRequest, Request};
use cfa4_tasks::resources::{Current, GpuCurrent, GpuNominal, Nominal};
use cfa4_tasks::{Manager, MissedStart, Monitor, NewTask, Priority, QueueConfig, QueueError, QueueEvent, Snapshot, StartMode, State};

use crate::testdb::TestDb;
use crate::{Store, StoreConfig, StoreError};

const GIB: u64 = 1 << 30;

fn snap() -> Snapshot {
    Snapshot {
        nominal: Nominal {
            ram_physical_bytes: Some(32 * GIB),
            cpus_online: Some(16),
            gpus: vec![GpuNominal {
                node: "renderD128".into(),
                vendor: Vendor::Nvidia,
                driver: "nvidia".into(),
                vram_total_bytes: Some(12 * GIB),
                drives_display: Some(false),
            }],
            ..Nominal::default()
        },
        current: Current {
            ram_available_bytes: Some(28 * GIB),
            gpus: vec![GpuCurrent { node: "renderD128".into(), vram_used_bytes: Some(0), busy_percent: None }],
            ..Current::default()
        },
        notes: vec![],
    }
}

fn manager() -> Manager {
    Manager::new(Budget::new(&snap().nominal, &BudgetSettings::default()))
}

fn task(id: &str, vram_gib: u64) -> NewTask {
    NewTask {
        id: id.into(),
        request: Request {
            class: Class::Background,
            ram_bytes: GIB,
            cpu_milli: 1000,
            gpu: Some(GpuRequest { vram_bytes: vram_gib * GIB, node: None, vendor: None }),
        },
        priority: Priority::Normal,
        start: StartMode::Now,
        missed: MissedStart::RunLate,
        resumable: true,
    }
}

async fn setup() -> Option<(TestDb, Store)> {
    let db = TestDb::start()?;
    let store = match Store::connect(&db.config()).await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("cfa4-store tests: cannot connect to the test cluster: {e}; skipped");
            return None;
        }
    };
    if let Err(e) = store.migrate().await {
        eprintln!("cfa4-store tests: migration failed: {e}");
        return None;
    }
    Some((db, store))
}

async fn count(store: &Store, sql: &str) -> i64 {
    let Ok(client) = store.client().await else { return -1 };
    client.query_one(sql, &[]).await.and_then(|r| r.try_get(0)).unwrap_or(-1)
}

#[tokio::test]
async fn a_missing_server_is_a_handled_error_with_the_install_hint() {
    let config = StoreConfig { host: "/nonexistent/cfa4-socket".into(), connect_timeout_ms: 200, ..StoreConfig::default() };
    match Store::connect(&config).await {
        Err(StoreError::Unavailable { hint, .. }) => assert!(hint.contains("apt install postgresql")),
        Err(e) => panic!("expected Unavailable, got {e}"),
        Ok(_) => panic!("connected to a socket that does not exist"),
    }
}

#[tokio::test]
async fn migrations_go_up_once_and_back_one_step() {
    let Some((_db, store)) = setup().await else { return };
    assert_eq!(store.migrate().await.unwrap(), vec![], "already up to date");
    assert_eq!(count(&store, "SELECT count(*) FROM tasks.task").await, 0);
    assert_eq!(store.rollback("tasks").await.unwrap(), Some(2));
    assert_eq!(count(&store, "SELECT count(*) FROM pg_indexes WHERE indexname = 'task_open' AND indexdef LIKE '%cancelling%'").await, 0);
    assert_eq!(store.rollback("tasks").await.unwrap(), Some(1));
    assert_eq!(count(&store, "SELECT count(*) FROM information_schema.schemata WHERE schema_name = 'tasks'").await, 0);
    assert_eq!(store.rollback("tasks").await.unwrap(), None);
    assert_eq!(store.migrate().await.unwrap(), vec![("tasks".to_owned(), 1), ("tasks".to_owned(), 2)]);
    assert_eq!(count(&store, "SELECT count(*) FROM pg_indexes WHERE indexname = 'task_open' AND indexdef LIKE '%cancelling%'").await, 1);
    assert!(store.rollback("nope").await.is_err());
}

#[tokio::test]
async fn two_schedulers_start_a_task_exactly_once() {
    let Some((_db, store)) = setup().await else { return };
    let (a, b) = (store.tasks(QueueConfig::default()), store.tasks(QueueConfig::default()));
    a.submit(task("t1", 4), 0).await.unwrap();
    assert!(matches!(a.submit(task("t1", 4), 0).await, Err(StoreError::Queue { error: QueueError::DuplicateId { .. } })));
    let (s, mon) = (snap(), Monitor::default());
    let (mut ma, mut mb) = (manager(), manager());
    let (ea, eb) = tokio::join!(a.tick(1, "sup-a", &mut ma, &s, &mon), b.tick(1, "sup-b", &mut mb, &s, &mon));
    let started = |e: &Vec<QueueEvent>| e.iter().filter(|e| matches!(e, QueueEvent::Started { .. })).count();
    assert_eq!(started(&ea.unwrap()) + started(&eb.unwrap()), 1, "exactly one scheduler started it");
    assert_eq!(ma.held_count() + mb.held_count(), 1, "and only that one holds the reservation");
    // a later step by the other scheduler sees it running under a live lease
    let again = b.tick(2, "sup-b", &mut mb, &s, &mon).await.unwrap();
    assert_eq!(started(&again), 0);
    let t = a.get("t1").await.unwrap().unwrap();
    assert_eq!((t.state, t.attempts), (State::Running, 1));
    assert_eq!(count(&store, "SELECT count(*) FROM tasks.transition WHERE task_id = 't1'").await, 1);
    assert_eq!(count(&store, "SELECT count(*) FROM tasks.outbox WHERE subject = 'cfa4.execution.job.running'").await, 1);
}

#[tokio::test]
async fn a_restart_resumes_from_the_checkpoint_never_from_the_start() {
    let Some((_db, store)) = setup().await else { return };
    let q = store.tasks(QueueConfig::default());
    q.submit(task("long", 4), 0).await.unwrap();
    q.submit(NewTask { resumable: false, ..task("oneshot", 2) }, 0).await.unwrap();
    let (s, mon) = (snap(), Monitor::default());
    let mut m = manager();
    q.tick(0, "sup-a", &mut m, &s, &mon).await.unwrap();
    q.checkpoint("long", "sup-a", 7).await.unwrap();
    assert_eq!(q.renew("long", "sup-a", 10_000).await.unwrap(), 40_000);
    assert!(q.renew("long", "intruder", 10_000).await.is_err());
    // CFA4 restarts: a new process with a fresh manager, after the leases lapsed
    let mut fresh = manager();
    let e = q.tick(60_000, "sup-b", &mut fresh, &s, &mon).await.unwrap();
    assert!(e.contains(&QueueEvent::Started { id: "long".into(), gpu_node: Some("renderD128".into()), resumed_from: Some(7) }), "{e:?}");
    assert!(e.contains(&QueueEvent::LeaseLost { id: "oneshot".into(), resumed: false }));
    assert_eq!(q.get("oneshot").await.unwrap().map(|t| t.state), Some(State::Failed));
    // finish: the reservation is released only once the commit went through
    q.finish("long", "sup-b", State::Succeeded, 61_000, "done", &mut fresh).await.unwrap();
    assert_eq!(fresh.held_count(), 0);
    assert!(q.finish("long", "sup-b", State::Succeeded, 61_000, "again", &mut fresh).await.is_err());
}

#[tokio::test]
async fn user_actions_check_the_version() {
    let Some((_db, store)) = setup().await else { return };
    let q = store.tasks(QueueConfig::default());
    q.submit(NewTask { start: StartMode::At { at: 1_000 }, ..task("later", 2) }, 0).await.unwrap();
    assert!(matches!(q.pause("later", 7, 1).await, Err(StoreError::Queue { error: QueueError::Stale { .. } })));
    q.pause("later", 1, 1).await.unwrap();
    q.reschedule("later", 2, 5_000, 2).await.unwrap();
    let t = q.get("later").await.unwrap().unwrap();
    assert_eq!((t.state, t.version, t.start), (State::Scheduled, 3, StartMode::At { at: 5_000 }));
    assert_eq!(q.cancel("later", 3, 3).await.unwrap(), State::Cancelled);
    assert!(matches!(q.cancel("missing", 1, 3).await, Err(StoreError::Queue { error: QueueError::NotFound { .. } })));
}

#[tokio::test]
async fn a_corrupt_row_is_an_error_not_a_panic() {
    let Some((_db, store)) = setup().await else { return };
    let q = store.tasks(QueueConfig::default());
    q.submit(task("bad", 2), 0).await.unwrap();
    let client = store.client().await.unwrap();
    client.execute("UPDATE tasks.task SET doc = '{\"id\": 42}'::jsonb WHERE id = 'bad'", &[]).await.unwrap();
    assert!(matches!(q.get("bad").await, Err(StoreError::Corrupt { .. })));
    let mut m = manager();
    assert!(matches!(q.tick(1, "sup", &mut m, &snap(), &Monitor::default()).await, Err(StoreError::Corrupt { .. })));
    assert_eq!(m.held_count(), 0);
}

#[tokio::test]
async fn a_task_whose_worker_dies_while_cancelling_frees_its_resources() {
    let Some((_db, store)) = setup().await else { return };
    let q = store.tasks(QueueConfig::default());
    q.submit(task("stuck", 4), 0).await.unwrap();
    let (s, mon) = (snap(), Monitor::default());
    let mut m = manager();
    q.tick(0, "sup-a", &mut m, &s, &mon).await.unwrap();
    let v = q.get("stuck").await.unwrap().unwrap().version;
    assert_eq!(q.cancel("stuck", v, 1).await.unwrap(), State::Cancelling);
    // cancelling again changes nothing
    assert_eq!(q.cancel("stuck", v + 1, 2).await.unwrap(), State::Cancelling);
    assert_eq!(m.held_count(), 1);
    // the worker never reports; once the lease lapses the scheduler cancels it and frees the VRAM
    let e = q.tick(60_000, "sup-a", &mut m, &s, &mon).await.unwrap();
    assert!(e.contains(&QueueEvent::LeaseLost { id: "stuck".into(), resumed: false }), "{e:?}");
    assert_eq!(q.get("stuck").await.unwrap().map(|t| t.state), Some(State::Cancelled));
    assert_eq!(m.held_count(), 0);
}
