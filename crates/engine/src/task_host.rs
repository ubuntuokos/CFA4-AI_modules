//! The engine's heavy background work as a client of the CFA4-wide task queue (ACS-6,
//! `docs/ai/acs-adaptiv-eroforras.md`).
//!
//! [`TaskHost`] keeps a [`cfa4_tasks::Queue`] and the resource [`Manager`] for this session. A
//! module asks [`TaskHost::admit`] before it starts heavy work: the task is submitted once, and
//! starts only when the queue's rules and the machine's budget allow it; otherwise it waits, and
//! the reasons are shown to the user. The module stays the owner of its own list and order (the
//! export queue keeps its user-ordered items, "the export job becomes a client, it is not
//! replaced"), and reports the end with [`TaskHost::end`], which releases the reservation.
//!
//! The media engine always works (VISION 2.4, CPU fallback), so a media task never becomes
//! unavailable here: [`TaskHost::fit`] clamps its request to the budget and asks for nothing it
//! cannot measure (no RAM when the free amount is unknown, no CPU when the CPU count is unknown).
//! It may wait for room, never forever for a number nobody can read. AI tasks plan with
//! `cfa4_tasks::plan` instead, where unknown is never enough.
//!
//! The queue lives in memory here: editing, playback and saving never depend on the database
//! (`AGENTS.md` §0). The PostgreSQL store (`cfa4-store`, L5) is wired in by the application.

use std::sync::Arc;

use cfa4_tasks::budget::{Budget, BudgetSettings};
use cfa4_tasks::manager::{Class, Denied, Request, WaitReason};
use cfa4_tasks::{Manager, MissedStart, Monitor, NewTask, Priority, Queue, QueueConfig, QueueEvent, Snapshot, StartMode, State};

/// Reads the machine's resources (tests install a fixed one).
pub type Sampler = Arc<dyn Fn() -> Snapshot + Send + Sync>;

/// The lease owner of every task this session runs.
pub const OWNER: &str = "cfa4-engine";
/// Resources are read again at most this often while tasks wait or run.
const SAMPLE_EVERY: std::time::Duration = std::time::Duration::from_secs(2);
/// Finished tasks are dropped from memory after this long.
const KEEP_FINISHED_MS: u64 = 60 * 60 * 1000;

/// Whether a task may start now.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Admission {
    /// It holds its reservation: start the work, then call [`TaskHost::end`].
    Start,
    /// Not yet; the reasons as the user reads them.
    Wait(Vec<String>),
    /// It cannot be given resources at all (only after a broken restored state).
    Refused(String),
}

/// How a task's work ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Succeeded,
    Failed,
    Cancelled,
}

pub struct TaskHost {
    queue: Queue,
    manager: Manager,
    monitor: Monitor,
    snap: Snapshot,
    sampler: Sampler,
    settings: BudgetSettings,
    sampled: Option<web_time::Instant>,
    next: u64,
}

impl Default for TaskHost {
    fn default() -> Self {
        if cfg!(test) { Self::new(Arc::new(test_snapshot)) } else { Self::new(Arc::new(cfa4_tasks::sample)) }
    }
}

/// A roomy, fully known machine: engine tests never wait on the machine they run on.
fn test_snapshot() -> Snapshot {
    use cfa4_tasks::resources::{Current, Nominal};
    const GIB: u64 = 1 << 30;
    Snapshot {
        nominal: Nominal { ram_physical_bytes: Some(64 * GIB), cpus_online: Some(16), ..Nominal::default() },
        current: Current { ram_available_bytes: Some(60 * GIB), ..Current::default() },
        notes: vec![],
    }
}

fn now_ms() -> u64 {
    web_time::SystemTime::now().duration_since(web_time::UNIX_EPOCH).map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
}

/// One wait reason in words.
pub fn describe(reason: &WaitReason) -> String {
    use cfa4_tasks::manager::Resource;
    let what = |r: &Resource| match r {
        Resource::Ram => "memory".to_owned(),
        Resource::Cpu => "processor".to_owned(),
        Resource::Gpu { node } | Resource::Vram { node } => format!("GPU memory{}", node.as_ref().map(|n| format!(" ({n})")).unwrap_or_default()),
    };
    match reason {
        WaitReason::Reserved { resource, .. } => format!("waiting for other CFA4 tasks to free {}", what(resource)),
        WaitReason::InUse { resource, .. } => format!("waiting: other programs are using the {}", what(resource)),
        WaitReason::Pressure { signal, .. } => format!("waiting: the system is under pressure ({signal:?})"),
        WaitReason::Unknown { resource } => format!("waiting: free {} cannot be read", what(resource)),
    }
}

impl TaskHost {
    pub fn new(sampler: Sampler) -> Self {
        TaskHost {
            queue: Queue::new(QueueConfig::default()),
            manager: Manager::default(),
            monitor: Monitor::default(),
            snap: Snapshot::default(),
            sampler,
            settings: BudgetSettings::default(),
            sampled: None,
            next: 0,
        }
    }

    /// A new task id under `prefix` (unique in this session).
    pub fn new_id(&mut self, prefix: &str) -> String {
        self.next = self.next.saturating_add(1);
        format!("{prefix}-{}", self.next)
    }

    fn refresh(&mut self, force: bool) {
        if !force && self.sampled.is_some_and(|t| t.elapsed() < SAMPLE_EVERY) {
            return;
        }
        self.snap = (self.sampler)();
        self.manager.set_budget(Budget::new(&self.snap.nominal, &self.settings));
        self.monitor.observe(&self.snap);
        self.manager.observe_sample();
        self.sampled = Some(web_time::Instant::now());
    }

    /// What a media task may ask for: `ram_bytes` and `cpu_milli` clamped to the budget, and
    /// nothing for a resource whose amount cannot be read (see the module notes).
    pub fn fit(&mut self, class: Class, ram_bytes: u64, cpu_milli: u64) -> Request {
        if self.sampled.is_none() {
            self.refresh(true);
        }
        let budget = self.manager.budget();
        let ram = match (budget.ram_bytes, self.snap.ram_free_effective_bytes()) {
            (Some(b), Some(_)) => ram_bytes.min(b),
            _ => 0,
        };
        let cpu = budget.cpu_milli.map_or(0, |b| cpu_milli.min(b));
        Request { class, ram_bytes: ram, cpu_milli: cpu, gpu: None }
    }

    /// Submit `id` once (with `request`), run a scheduler step, and say whether it may start.
    pub fn admit(&mut self, id: &str, request: impl FnOnce(&mut Self) -> Request) -> Admission {
        if self.queue.get(id).is_none() {
            let request = request(self);
            let new = NewTask { id: id.to_owned(), request, priority: Priority::Normal, start: StartMode::Now, missed: MissedStart::RunLate, resumable: false };
            if let Err(e) = self.queue.submit(new, now_ms()) {
                return Admission::Refused(format!("{e:?}"));
            }
        }
        self.tick();
        match self.queue.get(id) {
            Some(t) if t.state == State::Running => Admission::Start,
            Some(t) if t.state == State::Blocked => Admission::Refused(match &t.blocked_by {
                Some(Denied::NeverFits { resource, .. }) => format!("not enough {resource:?} on this machine"),
                Some(d) => format!("{d:?}"),
                None => "no resources".into(),
            }),
            Some(t) => Admission::Wait(t.wait_reasons.iter().map(describe).collect()),
            None => Admission::Refused("the task was dropped".into()),
        }
    }

    /// Renew the leases of the running tasks and run one scheduler step.
    pub fn tick(&mut self) -> Vec<QueueEvent> {
        let busy = self.queue.tasks().iter().any(|t| matches!(t.state, State::Queued | State::Waiting | State::Running | State::Cancelling));
        if !busy {
            return Vec::new();
        }
        self.refresh(false);
        let now = now_ms();
        // this process runs them: its leases lapse only when it stops
        let running: Vec<String> = self.queue.tasks().iter().filter(|t| t.state == State::Running).map(|t| t.id.clone()).collect();
        for id in running {
            let _ = self.queue.renew(&id, OWNER, now);
        }
        let events = match self.queue.tick(now, OWNER, &mut self.manager, &self.snap, &self.monitor) {
            Ok(events) => events,
            Err(e) => {
                log::warn!("task queue step failed: {e:?}");
                Vec::new()
            }
        };
        self.queue.prune(now.saturating_sub(KEEP_FINISHED_MS));
        events
    }

    /// The work of `id` ended: release its resources. Unknown ids are ignored.
    pub fn end(&mut self, id: &str, outcome: Outcome) {
        let now = now_ms();
        let Some(t) = self.queue.get(id) else { return };
        let (state, version) = (t.state, t.version);
        let r = match (state, outcome) {
            (State::Running, Outcome::Succeeded) => self.queue.finish(id, OWNER, State::Succeeded, now, "done", &mut self.manager),
            (State::Running, Outcome::Failed) => self.queue.finish(id, OWNER, State::Failed, now, "failed", &mut self.manager),
            (State::Running, Outcome::Cancelled) => self.queue.cancel(id, version, now).and_then(|_| self.queue.cancelled(id, OWNER, now, &mut self.manager)),
            (State::Cancelling, _) => self.queue.cancelled(id, OWNER, now, &mut self.manager),
            (s, _) if s.is_terminal() => Ok(()),
            // not started yet: it will not run
            _ => self.queue.cancel(id, version, now).map(|_| ()),
        };
        if let Err(e) = r {
            log::warn!("task {id}: could not record its end: {e:?}");
        }
    }

    /// Withdraw every task under `prefix` that has not started, except `keep`.
    pub fn withdraw(&mut self, prefix: &str, keep: Option<&str>) {
        let ids: Vec<String> = self
            .queue
            .tasks()
            .iter()
            .filter(|t| {
                t.id.starts_with(prefix)
                    && Some(t.id.as_str()) != keep
                    && matches!(t.state, State::Queued | State::Waiting | State::Blocked | State::Scheduled | State::Paused)
            })
            .map(|t| t.id.clone())
            .collect();
        for id in ids {
            self.end(&id, Outcome::Cancelled);
        }
    }

    pub fn state(&self, id: &str) -> Option<State> {
        self.queue.get(id).map(|t| t.state)
    }

    /// Reservations held now.
    pub fn held(&self) -> usize {
        self.manager.held_count()
    }

    pub fn queue(&self) -> &Queue {
        &self.queue
    }

    /// The tasks, their states and wait reasons, for `tasks.list`.
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::Value::Array(
            self.queue
                .tasks()
                .iter()
                .map(|t| {
                    serde_json::json!({
                        "id": t.id,
                        "state": t.state,
                        "class": t.request.class,
                        "ramBytes": t.request.ram_bytes,
                        "cpuMilli": t.request.cpu_milli,
                        "waitingFor": t.wait_reasons.iter().map(describe).collect::<Vec<_>>(),
                    })
                })
                .collect(),
        )
    }

    /// Read the resources again at the next step (after the machine or its settings changed).
    pub fn resample(&mut self) {
        self.sampled = None;
    }

    /// The budget's settings (Preferences ▸ Resources, later); applied at the next sample.
    pub fn set_settings(&mut self, settings: BudgetSettings) {
        self.settings = settings;
        self.sampled = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GIB: u64 = 1 << 30;

    fn host_with(ram_available: Option<u64>) -> TaskHost {
        use cfa4_tasks::resources::{Current, Nominal};
        TaskHost::new(Arc::new(move || Snapshot {
            nominal: Nominal { ram_physical_bytes: Some(16 * GIB), cpus_online: Some(8), ..Nominal::default() },
            current: Current { ram_available_bytes: ram_available, ..Current::default() },
            notes: vec![],
        }))
    }

    #[test]
    fn a_task_waits_for_room_and_starts_when_the_other_ends() {
        let mut h = host_with(Some(14 * GIB));
        let big = |h: &mut TaskHost| h.fit(Class::FinalRender, 10 * GIB, 4000);
        assert_eq!(h.admit("export-1", big), Admission::Start);
        let second = h.admit("export-2", big);
        assert!(matches!(&second, Admission::Wait(r) if r.iter().any(|r| r.contains("memory"))), "{second:?}");
        assert_eq!(h.held(), 1);
        h.end("export-1", Outcome::Succeeded);
        assert_eq!(h.held(), 0);
        assert_eq!(h.admit("export-2", big), Admission::Start);
        h.end("export-2", Outcome::Cancelled);
        assert_eq!((h.state("export-2"), h.held()), (Some(State::Cancelled), 0));
    }

    #[test]
    fn media_work_never_waits_on_a_number_nobody_can_read() {
        // free RAM unreadable, and a request larger than the whole budget: clamped, not refused
        let mut h = host_with(None);
        let r = h.fit(Class::FinalRender, 999 * GIB, 999_000);
        assert_eq!((r.ram_bytes, r.cpu_milli), (0, 7000));
        assert_eq!(h.admit("export-1", |_| r), Admission::Start);
        let mut unknown = TaskHost::new(Arc::new(Snapshot::default));
        let r = unknown.fit(Class::FinalRender, GIB, 1000);
        assert_eq!((r.ram_bytes, r.cpu_milli), (0, 0));
        assert_eq!(unknown.admit("t", |_| r), Admission::Start);
    }

    #[test]
    fn withdrawn_and_ended_tasks_release_everything() {
        let mut h = host_with(Some(14 * GIB));
        let big = |h: &mut TaskHost| h.fit(Class::FinalRender, 10 * GIB, 1000);
        assert_eq!(h.admit("export-1", big), Admission::Start);
        assert!(matches!(h.admit("export-2", big), Admission::Wait(_)));
        assert!(matches!(h.admit("export-3", big), Admission::Wait(_)));
        h.withdraw("export-", Some("export-3"));
        assert_eq!(h.state("export-2"), Some(State::Cancelled));
        assert_eq!(h.state("export-1"), Some(State::Running), "a running task is not withdrawn");
        h.end("export-1", Outcome::Failed);
        h.end("export-1", Outcome::Failed); // twice: ignored
        h.end("missing", Outcome::Succeeded);
        assert_eq!(h.admit("export-3", big), Admission::Start);
        assert_eq!(h.held(), 1);
    }
}
