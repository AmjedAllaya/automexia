use std::{
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver, SyncSender, TryRecvError},
    time::{Duration, Instant},
};

use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};

use super::{
    service::{QuickActionService, RefreshOutcome},
    store::{QuickActionStore, StoreError, StoreErrorCode},
};

pub const WATCH_EVENT_CAPACITY: usize = 64;
pub const MAX_WATCH_EVENTS_PER_POLL: usize = 64;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WatchErrorCode {
    Generic,
    Io,
    PathNotFound,
    WatchNotFound,
    InvalidConfig,
    WatchLimit,
    Disconnected,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExactActionWatchPlan {
    directory: PathBuf,
    source: PathBuf,
}

impl ExactActionWatchPlan {
    pub fn for_store(store: &QuickActionStore) -> Result<Self, StoreError> {
        let directory = store.root().to_path_buf();
        let metadata = std::fs::symlink_metadata(&directory).map_err(StoreError::io)?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(StoreError::new(StoreErrorCode::LinkRejected));
        }
        Ok(Self {
            source: store.source_path(),
            directory,
        })
    }

    pub fn watched_directory(&self) -> &Path {
        &self.directory
    }

    pub fn source_path(&self) -> &Path {
        &self.source
    }

    pub fn contains(&self, path: &Path) -> bool {
        if path == self.source {
            return true;
        }
        path.is_relative() && self.directory.join(path) == self.source
    }

    pub fn event_affects_source(&self, event: &Event) -> bool {
        event.paths.iter().any(|path| self.contains(path))
    }
}

pub fn recommended_watcher(
    plan: &ExactActionWatchPlan,
    sender: SyncSender<Result<Event, WatchErrorCode>>,
) -> Result<RecommendedWatcher, StoreError> {
    let handler = move |result: notify::Result<Event>| {
        let result = result.map_err(|error| redacted_notify_error(&error));
        // Notification bursts are intentionally lossy. The queue is bounded,
        // exact events are coalesced, and periodic reconciliation restores the
        // final state without blocking a notify backend thread.
        let _ = sender.try_send(result);
    };
    let mut watcher = notify::recommended_watcher(handler)
        .map_err(|_| StoreError::new(StoreErrorCode::Io))?;
    watcher
        .watch(plan.watched_directory(), RecursiveMode::NonRecursive)
        .map_err(|_| StoreError::new(StoreErrorCode::Io))?;
    Ok(watcher)
}

fn redacted_notify_error(error: &notify::Error) -> WatchErrorCode {
    use notify::ErrorKind;
    match &error.kind {
        ErrorKind::Generic(_) => WatchErrorCode::Generic,
        ErrorKind::Io(_) => WatchErrorCode::Io,
        ErrorKind::PathNotFound => WatchErrorCode::PathNotFound,
        ErrorKind::WatchNotFound => WatchErrorCode::WatchNotFound,
        ErrorKind::InvalidConfig(_) => WatchErrorCode::InvalidConfig,
        ErrorKind::MaxFilesWatch => WatchErrorCode::WatchLimit,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RefreshDecision {
    Idle,
    Run { generation: u64 },
}

#[derive(Clone, Debug)]
pub struct RefreshCoordinator {
    debounce: Duration,
    reconcile_interval: Duration,
    requested_generation: u64,
    completed_generation: u64,
    pending_at: Option<Instant>,
    next_reconcile_at: Instant,
}

impl RefreshCoordinator {
    pub fn new(now: Instant, debounce: Duration, reconcile_interval: Duration) -> Self {
        Self {
            debounce,
            reconcile_interval,
            requested_generation: 0,
            completed_generation: 0,
            pending_at: None,
            next_reconcile_at: now + reconcile_interval,
        }
    }

    pub fn request(&mut self, now: Instant) -> u64 {
        self.requested_generation = self.requested_generation.saturating_add(1);
        self.pending_at = Some(now + self.debounce);
        self.requested_generation
    }

    pub fn request_event(
        &mut self,
        plan: &ExactActionWatchPlan,
        event: &Event,
        now: Instant,
    ) -> bool {
        if !plan.event_affects_source(event) {
            return false;
        }
        self.request(now);
        true
    }

    pub fn poll(&mut self, now: Instant) -> RefreshDecision {
        if now >= self.next_reconcile_at {
            self.requested_generation = self.requested_generation.saturating_add(1);
            self.pending_at = Some(now);
            self.next_reconcile_at = now + self.reconcile_interval;
        }
        match self.pending_at {
            Some(due)
                if now >= due
                    && self.requested_generation > self.completed_generation =>
            {
                self.pending_at = None;
                RefreshDecision::Run {
                    generation: self.requested_generation,
                }
            }
            _ => RefreshDecision::Idle,
        }
    }

    pub fn complete(&mut self, generation: u64) -> bool {
        if generation != self.requested_generation
            || generation <= self.completed_generation
        {
            return false;
        }
        self.completed_generation = generation;
        true
    }
}

#[derive(Debug)]
pub struct QuickActionMonitor {
    service: QuickActionService,
    plan: ExactActionWatchPlan,
    coordinator: RefreshCoordinator,
    receiver: Receiver<Result<Event, WatchErrorCode>>,
    _watcher: RecommendedWatcher,
    last_watch_error: Option<WatchErrorCode>,
}

impl QuickActionMonitor {
    pub fn start(
        service: QuickActionService,
        now: Instant,
        debounce: Duration,
        reconcile_interval: Duration,
    ) -> Result<Self, StoreError> {
        let plan = ExactActionWatchPlan::for_store(service.store())?;
        let (sender, receiver) = mpsc::sync_channel(WATCH_EVENT_CAPACITY);
        let watcher = recommended_watcher(&plan, sender)?;
        Ok(Self {
            service,
            plan,
            coordinator: RefreshCoordinator::new(now, debounce, reconcile_interval),
            receiver,
            _watcher: watcher,
            last_watch_error: None,
        })
    }

    pub fn service(&self) -> &QuickActionService {
        &self.service
    }

    pub const fn last_watch_error(&self) -> Option<WatchErrorCode> {
        self.last_watch_error
    }

    /// Drain a bounded event batch and perform at most one coalesced refresh.
    /// Callers must drive this from an application worker, never renderer,
    /// terminal-input, VT, or PTY threads.
    pub fn poll(&mut self, now: Instant) -> Option<RefreshOutcome> {
        for _ in 0..MAX_WATCH_EVENTS_PER_POLL {
            match self.receiver.try_recv() {
                Ok(Ok(event)) => {
                    self.coordinator.request_event(&self.plan, &event, now);
                }
                Ok(Err(error)) => self.last_watch_error = Some(error),
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    self.last_watch_error = Some(WatchErrorCode::Disconnected);
                    break;
                }
            }
        }
        let RefreshDecision::Run { generation } = self.coordinator.poll(now) else {
            return None;
        };
        let outcome = self.service.refresh();
        self.coordinator.complete(generation);
        Some(outcome)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use notify::EventKind;

    #[test]
    fn exact_plan_accepts_only_the_canonical_source_path() {
        let root = tempfile::tempdir().unwrap();
        let store =
            QuickActionStore::open_or_create(root.path().join("actions")).unwrap();
        let plan = ExactActionWatchPlan::for_store(&store).unwrap();
        assert!(plan.contains(&store.source_path()));
        assert!(!plan.contains(&store.previous_path()));
        assert!(!plan.contains(&store.lock_path()));
        assert!(!plan.contains(&store.root().join("untrusted.toml")));

        let exact = Event::new(EventKind::Any).add_path(store.source_path());
        let unrelated =
            Event::new(EventKind::Any).add_path(store.root().join("untrusted.toml"));
        assert!(plan.event_affects_source(&exact));
        assert!(!plan.event_affects_source(&unrelated));
    }

    #[test]
    fn burst_coalescing_runs_only_the_latest_generation() {
        let root = tempfile::tempdir().unwrap();
        let store =
            QuickActionStore::open_or_create(root.path().join("actions")).unwrap();
        let plan = ExactActionWatchPlan::for_store(&store).unwrap();
        let now = Instant::now();
        let mut coordinator = RefreshCoordinator::new(
            now,
            Duration::from_millis(50),
            Duration::from_secs(60),
        );
        let event = Event::new(EventKind::Any).add_path(store.source_path());
        for offset in 0..1_000 {
            coordinator.request_event(&plan, &event, now + Duration::from_micros(offset));
        }
        assert_eq!(
            coordinator.poll(now + Duration::from_millis(49)),
            RefreshDecision::Idle
        );
        let RefreshDecision::Run { generation } =
            coordinator.poll(now + Duration::from_millis(51))
        else {
            panic!("coalesced refresh was not ready");
        };
        assert_eq!(generation, 1_000);
        assert!(coordinator.complete(generation));
        assert_eq!(
            coordinator.poll(now + Duration::from_millis(52)),
            RefreshDecision::Idle
        );
    }

    #[test]
    fn periodic_reconciliation_recovers_dropped_events() {
        let now = Instant::now();
        let mut coordinator = RefreshCoordinator::new(
            now,
            Duration::from_millis(50),
            Duration::from_secs(5),
        );
        assert_eq!(
            coordinator.poll(now + Duration::from_secs(4)),
            RefreshDecision::Idle
        );
        assert!(matches!(
            coordinator.poll(now + Duration::from_secs(5)),
            RefreshDecision::Run { generation: 1 }
        ));
    }

    #[test]
    fn monitor_periodic_reconciliation_publishes_cross_window_changes() {
        use automexia_devops::actions::{
            ActionProvenance, ActionScope, ActionTemplate, ExecutionMode, QuickAction,
            RiskClass, ShellKind, WorkingDirectoryPolicy,
        };

        let root = tempfile::tempdir().unwrap();
        let store =
            QuickActionStore::open_or_create(root.path().join("actions")).unwrap();
        let service = QuickActionService::open(store.clone()).unwrap();
        let now = Instant::now();
        let mut monitor = QuickActionMonitor::start(
            service,
            now,
            Duration::from_millis(25),
            Duration::from_secs(5),
        )
        .unwrap();
        store
            .create(
                0,
                QuickAction {
                    id: "external".into(),
                    display_name: "External".into(),
                    description: String::new(),
                    tags: Vec::new(),
                    scope: ActionScope::GlobalUser,
                    shells: vec![ShellKind::Powershell],
                    template: ActionTemplate::TypedArgv {
                        executable_id: "git".into(),
                        arguments: Vec::new(),
                    },
                    placeholders: Vec::new(),
                    working_directory_policy: WorkingDirectoryPolicy::Inherit,
                    risk: RiskClass::ReadOnly,
                    execution: ExecutionMode::Insert,
                    provenance: ActionProvenance::User,
                    enabled: true,
                    alias_projection: None,
                },
            )
            .unwrap();

        let outcome = monitor
            .poll(now + Duration::from_secs(5))
            .expect("periodic reconciliation must run without a watcher event");
        assert_eq!(outcome, RefreshOutcome::Published { revision: 1 });
        assert_eq!(monitor.service().snapshot().revision(), 1);
    }
    #[test]
    fn watcher_errors_are_redacted_to_fixed_codes() {
        let error = notify::Error::generic("TOP-SECRET")
            .add_path(PathBuf::from("C:/private/actions.toml"));
        assert_eq!(redacted_notify_error(&error), WatchErrorCode::Generic);
        let rendered = format!("{:?}", redacted_notify_error(&error));
        assert!(!rendered.contains("TOP-SECRET"));
        assert!(!rendered.contains("private"));
    }

    #[test]
    fn watcher_handles_release_across_repeated_start_stop_cycles() {
        let root = tempfile::tempdir().unwrap();
        let store =
            QuickActionStore::open_or_create(root.path().join("actions")).unwrap();
        let plan = ExactActionWatchPlan::for_store(&store).unwrap();
        for _ in 0..24 {
            let (sender, _receiver) = mpsc::sync_channel(WATCH_EVENT_CAPACITY);
            let watcher = recommended_watcher(&plan, sender).unwrap();
            drop(watcher);
        }
    }
}
