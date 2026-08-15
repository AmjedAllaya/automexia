use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
    sync::mpsc::Sender,
    time::{Duration, Instant},
};

use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};

use crate::{
    InventoryError, InventorySnapshot, MetadataStore, ScanCancellation, ScanOutcome,
    MAX_SOURCE_FILES,
};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WatchPlan {
    files: BTreeSet<PathBuf>,
    event_paths: BTreeSet<PathBuf>,
}

impl WatchPlan {
    pub fn from_scan(outcome: &ScanOutcome) -> Result<Self, InventoryError> {
        Self::build(outcome.watched_files.iter().cloned())
    }

    pub fn from_scan_and_metadata(
        outcome: &ScanOutcome,
        metadata: &MetadataStore,
    ) -> Result<Self, InventoryError> {
        let metadata_path = metadata.path();
        let metadata_path = match std::fs::symlink_metadata(&metadata_path) {
            Ok(_) => Some(metadata_path),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => {
                return Err(InventoryError::Watch(format!(
                    "owned metadata cannot be inspected ({:?})",
                    error.kind()
                )))
            }
        };
        Self::build(outcome.watched_files.iter().cloned().chain(metadata_path))
    }

    fn build(files: impl IntoIterator<Item = PathBuf>) -> Result<Self, InventoryError> {
        let mut exact = BTreeSet::new();
        let mut event_paths = BTreeSet::new();
        for file in files {
            if exact.len() > MAX_SOURCE_FILES {
                return Err(InventoryError::Watch(
                    "watch plan exceeds the bounded source and metadata set".into(),
                ));
            }
            let metadata = std::fs::symlink_metadata(&file).map_err(|error| {
                InventoryError::Watch(format!(
                    "known SSH source cannot be inspected ({:?})",
                    error.kind()
                ))
            })?;
            if metadata.file_type().is_symlink() || !metadata.is_file() {
                return Err(InventoryError::Watch(
                    "watch plan accepts only exact regular files".into(),
                ));
            }
            let absolute = if file.is_absolute() {
                file.clone()
            } else {
                std::env::current_dir()
                    .map_err(|error| {
                        InventoryError::Watch(format!(
                            "current directory cannot be read ({:?})",
                            error.kind()
                        ))
                    })?
                    .join(&file)
            };
            event_paths.insert(absolute);
            exact.insert(std::fs::canonicalize(file).map_err(|error| {
                InventoryError::Watch(format!(
                    "known SSH source cannot be canonicalized ({:?})",
                    error.kind()
                ))
            })?);
        }
        Ok(Self {
            files: exact,
            event_paths,
        })
    }

    pub fn contains(&self, path: &Path) -> bool {
        self.event_paths.contains(path)
            || self.files.contains(path)
            || std::fs::canonicalize(path)
                .ok()
                .is_some_and(|path| self.files.contains(&path))
    }

    pub fn files(&self) -> impl Iterator<Item = &Path> {
        self.files.iter().map(PathBuf::as_path)
    }
}

pub fn recommended_watcher(
    plan: &WatchPlan,
    sender: Sender<Result<Event, InventoryError>>,
) -> Result<RecommendedWatcher, InventoryError> {
    let handler = move |result: notify::Result<Event>| {
        let result = result.map_err(|error| {
            InventoryError::Watch(format!(
                "watch event failed ({})",
                redacted_notify_error(&error)
            ))
        });
        let _ = sender.send(result);
    };
    let mut watcher = notify::recommended_watcher(handler).map_err(|error| {
        InventoryError::Watch(format!(
            "watcher setup failed ({})",
            redacted_notify_error(&error)
        ))
    })?;
    for file in plan.files() {
        watcher
            .watch(file, RecursiveMode::NonRecursive)
            .map_err(|error| {
                InventoryError::Watch(format!(
                    "file watch failed ({})",
                    redacted_notify_error(&error)
                ))
            })?;
    }
    Ok(watcher)
}

fn redacted_notify_error(error: &notify::Error) -> String {
    use notify::ErrorKind;

    match &error.kind {
        ErrorKind::Generic(_) => "generic".into(),
        ErrorKind::Io(error) => format!("io:{:?}", error.kind()),
        ErrorKind::PathNotFound => "path-not-found".into(),
        ErrorKind::WatchNotFound => "watch-not-found".into(),
        ErrorKind::InvalidConfig(_) => "invalid-config".into(),
        ErrorKind::MaxFilesWatch => "watch-limit".into(),
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RefreshStatus {
    Empty,
    Fresh { generation: u64 },
    Stale { generation: u64, error: String },
}

#[derive(Clone, Debug)]
pub enum RefreshDecision {
    Idle,
    Run {
        generation: u64,
        cancellation: ScanCancellation,
    },
}

#[derive(Clone, Debug)]
pub struct RefreshCoordinator {
    debounce: Duration,
    reconcile_interval: Duration,
    requested_generation: u64,
    completed_generation: u64,
    pending_at: Option<Instant>,
    next_reconcile_at: Instant,
    active_cancellation: Option<ScanCancellation>,
    last_good: Option<InventorySnapshot>,
    status: RefreshStatus,
}

impl Drop for RefreshCoordinator {
    fn drop(&mut self) {
        if let Some(active) = self.active_cancellation.take() {
            active.cancel();
        }
    }
}

impl RefreshCoordinator {
    pub fn new(now: Instant, debounce: Duration, reconcile_interval: Duration) -> Self {
        Self {
            debounce,
            reconcile_interval,
            requested_generation: 1,
            completed_generation: 0,
            pending_at: Some(now),
            next_reconcile_at: now + reconcile_interval,
            active_cancellation: None,
            last_good: None,
            status: RefreshStatus::Empty,
        }
    }

    pub fn request(&mut self, now: Instant) -> u64 {
        if let Some(active) = self.active_cancellation.take() {
            active.cancel();
        }
        self.requested_generation = self.requested_generation.saturating_add(1);
        self.pending_at = Some(now + self.debounce);
        self.requested_generation
    }

    pub fn request_event(&mut self, plan: &WatchPlan, path: &Path, now: Instant) -> bool {
        if !plan.contains(path) {
            return false;
        }
        self.request(now);
        true
    }

    pub fn poll(&mut self, now: Instant) -> RefreshDecision {
        if now >= self.next_reconcile_at {
            self.request(now);
            self.pending_at = Some(now);
            self.next_reconcile_at = now + self.reconcile_interval;
        }
        match self.pending_at {
            Some(due)
                if now >= due
                    && self.requested_generation > self.completed_generation =>
            {
                self.pending_at = None;
                let cancellation = ScanCancellation::default();
                self.active_cancellation = Some(cancellation.clone());
                RefreshDecision::Run {
                    generation: self.requested_generation,
                    cancellation,
                }
            }
            _ => RefreshDecision::Idle,
        }
    }

    pub fn complete(
        &mut self,
        generation: u64,
        result: Result<ScanOutcome, InventoryError>,
    ) -> bool {
        if generation != self.requested_generation
            || generation <= self.completed_generation
        {
            return false;
        }
        self.active_cancellation = None;
        self.completed_generation = generation;
        match result {
            Ok(mut outcome) => {
                outcome.snapshot.generation = generation;
                self.last_good = Some(outcome.snapshot);
                self.status = RefreshStatus::Fresh { generation };
            }
            Err(error) => {
                let retained = self
                    .last_good
                    .as_ref()
                    .map_or(0, |snapshot| snapshot.generation);
                self.status = RefreshStatus::Stale {
                    generation: retained,
                    error: error.to_string(),
                };
            }
        }
        true
    }

    pub fn snapshot(&self) -> Option<&InventorySnapshot> {
        self.last_good.as_ref()
    }

    pub fn status(&self) -> &RefreshStatus {
        &self.status
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn success(generation: u64) -> ScanOutcome {
        ScanOutcome {
            snapshot: InventorySnapshot {
                generation,
                observed_files: 1,
                ..InventorySnapshot::default()
            },
            ..ScanOutcome::default()
        }
    }

    fn observed(files: impl IntoIterator<Item = PathBuf>) -> ScanOutcome {
        ScanOutcome {
            watched_files: files.into_iter().collect(),
            ..ScanOutcome::default()
        }
    }

    #[test]
    fn burst_is_coalesced_and_obsolete_completion_is_discarded() {
        let now = Instant::now();
        let mut coordinator = RefreshCoordinator::new(
            now,
            Duration::from_millis(50),
            Duration::from_secs(60),
        );
        let RefreshDecision::Run {
            generation,
            cancellation: first_scan,
        } = coordinator.poll(now)
        else {
            panic!("initial refresh was not due");
        };
        assert_eq!(generation, 1);
        coordinator.request(now);
        assert!(first_scan.is_cancelled());
        let latest = coordinator.request(now + Duration::from_millis(10));
        assert!(matches!(
            coordinator.poll(now + Duration::from_millis(49)),
            RefreshDecision::Idle
        ));
        let RefreshDecision::Run { generation, .. } =
            coordinator.poll(now + Duration::from_millis(60))
        else {
            panic!("coalesced refresh was not due");
        };
        assert_eq!(generation, latest);
        assert!(!coordinator.complete(latest - 1, Ok(success(latest - 1))));
        assert!(coordinator.complete(latest, Ok(success(latest))));
        assert_eq!(coordinator.snapshot().unwrap().generation, latest);
    }

    #[test]
    fn failed_refresh_retains_last_known_good_and_reports_stale() {
        let now = Instant::now();
        let mut coordinator =
            RefreshCoordinator::new(now, Duration::ZERO, Duration::from_secs(60));
        coordinator.request(now);
        let RefreshDecision::Run { generation, .. } = coordinator.poll(now) else {
            panic!("refresh was not due");
        };
        coordinator.complete(generation, Ok(success(generation)));
        let retained = coordinator.snapshot().cloned();
        coordinator.request(now);
        let RefreshDecision::Run { generation, .. } = coordinator.poll(now) else {
            panic!("second refresh was not due");
        };
        coordinator.complete(
            generation,
            Err(InventoryError::Limit("bounded failure".into())),
        );
        assert_eq!(coordinator.snapshot(), retained.as_ref());
        assert!(matches!(coordinator.status(), RefreshStatus::Stale { .. }));
    }

    #[test]
    fn watch_plan_accepts_only_exact_known_files() {
        let root = tempfile::tempdir().unwrap();
        let known = root.path().join("config");
        let unknown = root.path().join("other");
        std::fs::write(&known, b"Host prod").unwrap();
        std::fs::write(&unknown, b"Host other").unwrap();
        let plan = WatchPlan::from_scan(&observed([known.clone()])).unwrap();
        assert!(plan.contains(&known));
        assert!(!plan.contains(&unknown));
        assert_eq!(plan.files().count(), 1);
        std::fs::remove_file(&known).unwrap();
        assert!(plan.contains(&known));
    }

    #[test]
    fn recommended_watchers_release_resources_repeatedly() {
        let root = tempfile::tempdir().unwrap();
        let known = root.path().join("config");
        std::fs::write(&known, b"Host prod").unwrap();
        let plan = WatchPlan::from_scan(&observed([known])).unwrap();
        for _ in 0..16 {
            let (sender, _receiver) = std::sync::mpsc::channel();
            let watcher = recommended_watcher(&plan, sender).unwrap();
            drop(watcher);
        }
    }

    #[test]
    fn watcher_errors_never_echo_messages_or_paths() {
        let error = notify::Error::generic("TOP-SECRET")
            .add_path(PathBuf::from("C:/private/ssh/config"));
        let redacted = redacted_notify_error(&error);
        assert_eq!(redacted, "generic");
        assert!(!redacted.contains("TOP-SECRET"));
        assert!(!redacted.contains("private"));
    }

    #[test]
    fn watch_plan_adds_only_owned_metadata_to_observed_sources() {
        let root = tempfile::tempdir().unwrap();
        let known = root.path().join("config");
        std::fs::write(&known, b"Host prod").unwrap();
        let store = MetadataStore::new(root.path().join("devops-ssh")).unwrap();
        store.save(&crate::MetadataDocument::default()).unwrap();
        let plan = WatchPlan::from_scan_and_metadata(&observed([known.clone()]), &store)
            .unwrap();
        assert!(plan.contains(&known));
        assert!(plan.contains(&store.path()));
        assert_eq!(plan.files().count(), 2);
    }

    #[test]
    fn dropping_coordinator_cancels_active_scan() {
        let cancellation = {
            let now = Instant::now();
            let mut coordinator =
                RefreshCoordinator::new(now, Duration::ZERO, Duration::from_secs(60));
            let RefreshDecision::Run { cancellation, .. } = coordinator.poll(now) else {
                panic!("initial refresh was not due");
            };
            cancellation
        };
        assert!(cancellation.is_cancelled());
    }

    #[cfg(unix)]
    #[test]
    fn watch_plan_rejects_symlinked_owned_metadata() {
        use std::os::unix::fs::symlink;

        let root = tempfile::tempdir().unwrap();
        let known = root.path().join("config");
        std::fs::write(&known, b"Host prod").unwrap();
        let store = MetadataStore::new(root.path().join("devops-ssh")).unwrap();
        let outside = root.path().join("outside.json");
        std::fs::write(&outside, b"{}").unwrap();
        symlink(&outside, store.path()).unwrap();
        assert!(WatchPlan::from_scan_and_metadata(&observed([known]), &store).is_err());
    }
}
