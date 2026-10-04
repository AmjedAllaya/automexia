use super::{Checkpoint, Snapshot, MAX_BYTES};
use crate::automexia::private_fs::{self, WriteLock};
use automexia_extension_runtime::{BoundedWorker, RefreshSubmission};
use std::{
    path::{Path, PathBuf},
    sync::{mpsc, Arc, Mutex},
    time::{Duration, Instant},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StoreError {
    Invalid,
    Version,
    Busy,
    Io,
    Worker,
    Timeout,
    Protection,
    Capture,
}
impl From<private_fs::PrivateFsError> for StoreError {
    fn from(_: private_fs::PrivateFsError) -> Self {
        Self::Io
    }
}

pub enum StoreOutcome {
    Loaded {
        checkpoint: Checkpoint,
        restore: Snapshot,
        recovered: bool,
    },
    Prepared {
        snapshot: Snapshot,
        missing_folders: usize,
    },
    Saved {
        retired: bool,
    },
}
enum Operation {
    Load,
    Save {
        checkpoint: Checkpoint,
        retire: bool,
    },
    Prepare(Snapshot),
}
struct Request {
    operation: Operation,
}
trait CloneIntoCheckpoint {
    fn checkpoint(&self) -> Checkpoint;
}
impl CloneIntoCheckpoint for Snapshot {
    fn checkpoint(&self) -> Checkpoint {
        self.clone().into()
    }
}
impl CloneIntoCheckpoint for Checkpoint {
    fn checkpoint(&self) -> Checkpoint {
        self.clone()
    }
}

struct Store {
    directory: PathBuf,
    _lock: WriteLock,
    protection: std::cell::RefCell<super::protection::Protection>,
    digest: std::cell::Cell<Option<[u8; 32]>>,
}
fn cleanup_staged(directory: &Path) -> Result<(), StoreError> {
    // The lifetime lock excludes live writers. Only reserved, private regular
    // staging files are ours; cap startup work even in a hostile directory.
    for entry in std::fs::read_dir(directory)
        .map_err(|_| StoreError::Io)?
        .take(128)
    {
        let entry = entry.map_err(|_| StoreError::Io)?;
        if !entry
            .file_name()
            .to_str()
            .is_some_and(|n| n.starts_with(".session-staged-"))
        {
            continue;
        }
        let path = entry.path();
        if private_fs::inspect_private_file(&path).is_ok() {
            std::fs::remove_file(path).map_err(|_| StoreError::Io)?;
        }
    }
    Ok(())
}
impl Store {
    fn open(root: &Path) -> Result<Self, StoreError> {
        private_fs::inspect_directory(root)?;
        let state = root.join("state");
        private_fs::ensure_private_child_directory(&state)?;
        let directory = state.join("session-v1");
        private_fs::ensure_private_child_directory(&directory)?;
        let file = private_fs::open_private_lock(&directory.join("owner.lock"))?;
        let lock = WriteLock::try_acquire(file).map_err(|_| StoreError::Busy)?;
        cleanup_staged(&directory)?;
        Ok(Self {
            directory,
            _lock: lock,
            protection: std::cell::RefCell::new(super::protection::Protection::new(root)),
            digest: std::cell::Cell::new(None),
        })
    }
    fn read(&self, name: &str) -> Result<Option<Checkpoint>, StoreError> {
        private_fs::validate_private_child_directory(&self.directory)?;
        private_fs::read_bounded_regular(&self.directory.join(name), MAX_BYTES)?
            .map(|bytes| {
                self.protection
                    .borrow_mut()
                    .open(&bytes)
                    .and_then(|plain| Checkpoint::decode(&plain))
            })
            .transpose()
    }
    fn load(&self) -> Result<(Checkpoint, bool), StoreError> {
        match self.read("workspace.json") {
            Ok(Some(snapshot)) => Ok((snapshot, false)),
            Err(error @ (StoreError::Version | StoreError::Protection)) => Err(error),
            primary => match self.read("previous.json") {
                Ok(Some(snapshot)) => Ok((snapshot, true)),
                Ok(None) if matches!(primary, Ok(None)) => {
                    Ok((Snapshot::default().into(), false))
                }
                Err(error @ (StoreError::Version | StoreError::Protection)) => Err(error),
                _ => Err(StoreError::Invalid),
            },
        }
    }
    /// Promote before starting a new workspace. Ordinary periodic saves never
    /// replace this manual candidate. A trivial visit cannot displace real work.
    fn begin(&self) -> Result<(Checkpoint, Snapshot, bool), StoreError> {
        let (current, recovered) = self.load()?;
        let archived = self.read("restore.json")?;
        let promote = !current.incomplete_restore
            && !current.snapshot.windows.is_empty()
            && archived
                .as_ref()
                .is_none_or(|old| !old.noteworthy() || current.noteworthy());
        let restore = if promote {
            if archived.as_ref() != Some(&current) {
                self.write("restore.json", &current.encode()?)?;
            }
            current.snapshot.clone()
        } else {
            archived.map(|c| c.snapshot).unwrap_or_default()
        };
        Ok((current, restore, recovered))
    }
    fn write(&self, name: &str, bytes: &[u8]) -> Result<(), StoreError> {
        let encrypted = self.protection.borrow_mut().seal(bytes)?;
        private_fs::atomic_write_private(
            &self.directory,
            &self.directory.join(name),
            &encrypted,
            MAX_BYTES,
            ".session-staged-",
        )
        .map_err(Into::into)
    }
    fn retire_previous(&self) -> Result<(), StoreError> {
        // Tombstones first prevent an interrupted deletion from resurrecting an
        // obsolete candidate. Never enumerate or delete another owner's files.
        let empty = Checkpoint::from(Snapshot::default()).encode()?;
        for name in ["restore.json", "previous.json"] {
            self.write(name, &empty)?;
            let path = self.directory.join(name);
            private_fs::inspect_private_file(&path)?;
            std::fs::remove_file(path).map_err(|_| StoreError::Io)?;
        }
        Ok(())
    }

    fn save(&self, snapshot: &impl CloneIntoCheckpoint) -> Result<(), StoreError> {
        let mut snapshot = snapshot.checkpoint();
        let budget = (4_000_000 / snapshot.snapshot.session_count().max(1))
            .min(rio_backend::crosswords::archive::MAX_CELLS);
        for session in snapshot
            .snapshot
            .windows
            .iter_mut()
            .flat_map(|w| &mut w.tabs)
            .flat_map(|t| &mut t.nodes)
            .filter_map(|n| match n {
                super::Node::Pane { sessions, .. } => Some(sessions),
                _ => None,
            })
            .flatten()
        {
            if let Some(source) = session.source.take() {
                session.history =
                    Some(Arc::new(source.capture(budget).ok_or(StoreError::Capture)?));
            }
        }
        let candidate = zeroize::Zeroizing::new(snapshot.encode()?);
        use sha2::Digest;
        let digest: [u8; 32] = sha2::Sha256::digest(candidate.as_slice()).into();
        let (previous, recovered) = self.load()?;
        match std::fs::symlink_metadata(self.directory.join("previous.json")) {
            Ok(_) => {
                private_fs::inspect_private_file(&self.directory.join("previous.json"))?
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => return Err(StoreError::Io),
        }
        if !recovered && self.digest.get() == Some(digest) {
            return Ok(());
        }
        if snapshot.snapshot.windows.is_empty() {
            // A successful clean decision is authoritative even if backup
            // replacement fails or the process dies immediately afterwards.
            self.digest.set(None);
            self.write("workspace.json", &candidate)?;
            return self.write("previous.json", &candidate);
        }
        if previous.snapshot.windows.is_empty() {
            self.write("previous.json", &candidate)?;
        } else {
            self.write("previous.json", &previous.encode()?)?;
        }
        self.write("workspace.json", &candidate)?;
        self.digest.set(Some(digest));
        Ok(())
    }
}

/// One disk owner and one admitted operation. A newer checkpoint replaces the
/// pending checkpoint in memory; it never creates another worker or disk queue.
pub struct RecoveryService {
    worker: BoundedWorker<Request>,
    completed: mpsc::Receiver<Result<StoreOutcome, StoreError>>,
    pending: bool,
    deadline: Option<Instant>,
    failed: bool,
    latest: Option<Checkpoint>,
    retire_pending: bool,
}
impl RecoveryService {
    pub fn new(root: PathBuf, wake: Arc<dyn Fn() + Send + Sync>) -> Self {
        let (sender, completed) = mpsc::sync_channel(1);
        let store = Mutex::new(None::<Store>);
        let worker =
            BoundedWorker::new("session-recovery", 1, move |request: Request| {
                let result = (|| {
                    let mut owner = store.lock().map_err(|_| StoreError::Worker)?;
                    match request.operation {
                        Operation::Load => {
                            if owner.is_none() {
                                *owner = Some(Store::open(&root)?);
                            }
                            let (checkpoint, restore, recovered) =
                                owner.as_ref().ok_or(StoreError::Worker)?.begin()?;
                            Ok(StoreOutcome::Loaded {
                                checkpoint,
                                restore,
                                recovered,
                            })
                        }
                        Operation::Save { checkpoint, retire } => {
                            let store = owner.as_ref().ok_or(StoreError::Worker)?;
                            store.save(&checkpoint)?;
                            if retire {
                                store.retire_previous()?;
                            }
                            Ok(StoreOutcome::Saved { retired: retire })
                        }
                        Operation::Prepare(mut snapshot) => {
                            if !snapshot.validate() {
                                return Err(StoreError::Invalid);
                            }
                            let mut missing_folders = 0;
                            for session in snapshot
                                .windows
                                .iter_mut()
                                .flat_map(|w| &mut w.tabs)
                                .flat_map(|t| &mut t.nodes)
                                .filter_map(|node| match node {
                                    super::Node::Pane { sessions, .. } => Some(sessions),
                                    _ => None,
                                })
                                .flatten()
                            {
                                // Guest paths are never probed through a network share
                                // or a process. WSL receives a typed --cd only after consent.
                                if !matches!(session.profile, super::Profile::Wsl { .. })
                                    && session
                                        .cwd
                                        .as_deref()
                                        .is_some_and(|cwd| !Path::new(cwd).is_dir())
                                {
                                    session.cwd = None;
                                    missing_folders += 1;
                                }
                            }
                            Ok(StoreOutcome::Prepared {
                                snapshot,
                                missing_folders,
                            })
                        }
                    }
                })();
                if sender.try_send(result).is_ok() {
                    wake();
                }
            });
        Self {
            worker,
            completed,
            pending: false,
            deadline: None,
            failed: false,
            latest: None,
            retire_pending: false,
        }
    }
    fn submit(&mut self, operation: Operation) -> bool {
        if self.failed || self.pending {
            return false;
        }
        if self.worker.try_submit(Request { operation }) != RefreshSubmission::Queued {
            self.failed = true;
            return false;
        }
        self.pending = true;
        self.deadline = Some(Instant::now() + Duration::from_secs(5));
        true
    }
    pub fn load(&mut self) -> bool {
        self.submit(Operation::Load)
    }
    pub fn prepare(&mut self, snapshot: Snapshot) -> bool {
        self.submit(Operation::Prepare(snapshot))
    }
    pub fn save(&mut self, snapshot: impl Into<Checkpoint>) {
        let snapshot = snapshot.into();
        if self.failed {
            return;
        }
        if self.pending {
            self.latest = Some(snapshot);
        } else {
            self.submit(Operation::Save {
                checkpoint: snapshot,
                retire: self.retire_pending,
            });
        }
    }
    pub fn save_and_retire(&mut self, snapshot: Checkpoint) {
        self.retire_pending = true;
        self.save(snapshot);
    }
    pub fn retiring(&self) -> bool {
        self.retire_pending
    }
    pub fn take(&mut self, now: Instant) -> Option<Result<StoreOutcome, StoreError>> {
        if self.failed {
            return None;
        }
        if let Ok(result) = self.completed.try_recv() {
            self.pending = false;
            self.deadline = None;
            if result.as_ref().is_err_and(|e| *e != StoreError::Capture) {
                self.failed = true;
                self.latest = None;
            } else {
                if matches!(result, Ok(StoreOutcome::Saved { retired: true })) {
                    self.retire_pending = false;
                }
                // A busy terminal is transient. Keep the retirement intent and
                // continue with the newest capture; never consume on failure.
                if let Some(snapshot) = self.latest.take() {
                    self.submit(Operation::Save {
                        checkpoint: snapshot,
                        retire: self.retire_pending,
                    });
                }
            }
            return Some(result);
        }
        if self.deadline.is_some_and(|time| now >= time) {
            self.failed = true;
            self.latest = None;
            self.deadline = None;
            self.worker.request_shutdown();
            return Some(Err(StoreError::Timeout));
        }
        None
    }
    pub fn deadline(&self) -> Option<Instant> {
        self.deadline
    }
    pub fn shutdown(&mut self, timeout: Duration) -> bool {
        let until = Instant::now() + timeout;
        while self.pending && !self.failed && Instant::now() < until {
            let _ = self.take(Instant::now());
            if self.pending {
                std::thread::sleep(Duration::from_millis(5));
            }
        }
        self.worker
            .shutdown_timeout(until.saturating_duration_since(Instant::now()))
    }
}
impl Drop for RecoveryService {
    fn drop(&mut self) {
        self.worker.request_shutdown();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn abandoned_staging_cleanup_never_deletes_unrelated_files_or_directories() {
        let root = tempfile::tempdir().unwrap();
        let store = Store::open(root.path()).unwrap();
        store.write(".session-staged-fixture", b"unused").unwrap();
        store.write("unrelated.json", b"keep").unwrap();
        std::fs::create_dir(store.directory.join(".session-staged-directory")).unwrap();
        let directory = store.directory.clone();
        drop(store);
        let _store = Store::open(root.path()).unwrap();
        assert!(!directory.join(".session-staged-fixture").exists());
        assert!(directory.join("unrelated.json").exists());
        assert!(directory.join(".session-staged-directory").is_dir());
    }
    #[test]
    fn successful_recovery_retires_only_consumed_versions_after_latest_save() {
        let root = tempfile::tempdir().unwrap();
        let store = Store::open(root.path()).unwrap();
        let old = super::super::tests::fixture();
        store.save(&old).unwrap();
        store.begin().unwrap();
        assert!(store.read("restore.json").unwrap().is_some());
        let mut latest = old.clone();
        latest.windows[0].width = 1200;
        store.save(&latest).unwrap();
        store.retire_previous().unwrap();
        assert_eq!(store.load().unwrap().0.snapshot, latest);
        assert!(!store.directory.join("restore.json").exists());
        assert!(!store.directory.join("previous.json").exists());
        assert!(store.directory.join("owner.lock").exists());
    }
    #[test]
    fn disk_checkpoint_is_protected_and_legacy_topology_migrates() {
        let root = tempfile::tempdir().unwrap();
        let store = Store::open(root.path()).unwrap();
        let mut saved = super::super::tests::fixture();
        saved.windows[0].tabs[0].title = Some("RECOVERY_FIXTURE_SENTINEL".into());
        // Exercise the same no-follow private writer used by old releases.
        private_fs::atomic_write_private(
            &store.directory,
            &store.directory.join("workspace.json"),
            &saved.encode().unwrap(),
            MAX_BYTES,
            ".fixture-",
        )
        .unwrap();
        assert_eq!(store.load().unwrap().0.snapshot, saved);
        store.save(&saved).unwrap();
        let bytes = std::fs::read(store.directory.join("workspace.json")).unwrap();
        assert!(!bytes
            .windows(b"RECOVERY_FIXTURE_SENTINEL".len())
            .any(|w| w == b"RECOVERY_FIXTURE_SENTINEL"));
        assert_eq!(store.load().unwrap().0.snapshot, saved);
    }
    #[test]
    fn unchanged_checkpoints_do_not_reencrypt_or_rewrite_files() {
        let root = tempfile::tempdir().unwrap();
        let store = Store::open(root.path()).unwrap();
        let snapshot = super::super::tests::fixture();
        store.save(&snapshot).unwrap();
        let before = std::fs::read(store.directory.join("workspace.json")).unwrap();
        store.save(&snapshot).unwrap();
        assert_eq!(
            std::fs::read(store.directory.join("workspace.json")).unwrap(),
            before
        );
    }

    #[test]
    fn failed_capture_without_a_followup_never_consumes_retry_data() {
        let root = tempfile::tempdir().unwrap();
        let original = super::super::tests::fixture();
        {
            let store = Store::open(root.path()).unwrap();
            store.save(&original).unwrap();
        }
        let mut service = RecoveryService::new(root.path().into(), Arc::new(|| {}));
        assert!(service.load());
        assert!(completion(&mut service).is_ok());
        let mut failed = original.clone();
        if let super::super::Node::Pane { sessions, .. } =
            &mut failed.windows[0].tabs[0].nodes[0]
        {
            sessions[0].source = Some(super::super::HistorySource::unavailable());
        }
        service.save_and_retire(failed.into());
        assert!(matches!(completion(&mut service), Err(StoreError::Capture)));
        assert!(service.retiring());
        assert!(service.shutdown(Duration::from_secs(3)));
        drop(service);
        let store = Store::open(root.path()).unwrap();
        assert_eq!(store.read("restore.json").unwrap(), Some(original.into()));
    }
    #[test]
    fn manual_candidate_survives_quiet_startup_and_current_checkpoints() {
        let root = tempfile::tempdir().unwrap();
        let mut important = super::super::tests::fixture();
        let second = important.windows[0].tabs[0].clone();
        important.windows[0].tabs.push(second);
        {
            let store = Store::open(root.path()).unwrap();
            store.save(&important).unwrap();
        }
        let mut service = RecoveryService::new(root.path().into(), Arc::new(|| {}));
        assert!(service.load());
        assert!(completion(&mut service).is_ok());
        service.save(super::super::tests::fixture());
        assert!(service.shutdown(Duration::from_secs(3)));
        drop(service);
        let store = Store::open(root.path()).unwrap();
        assert_eq!(store.read("restore.json").unwrap(), Some(important.into()));
    }
    #[test]
    fn failed_partial_restore_never_replaces_retry_candidate_on_restart() {
        let root = tempfile::tempdir().unwrap();
        let original = super::super::tests::fixture();
        {
            let store = Store::open(root.path()).unwrap();
            store.save(&original).unwrap();
            store.begin().unwrap();
            let mut partial: Checkpoint = original.clone().into();
            partial.snapshot.windows[0].width = 1200;
            partial.significant_activity = true;
            partial.incomplete_restore = true;
            store.save(&partial).unwrap();
        }
        let store = Store::open(root.path()).unwrap();
        let (checkpoint, retry, _) = store.begin().unwrap();
        assert!(checkpoint.incomplete_restore);
        assert_eq!(retry, original);
    }
    #[test]
    fn exclusive_owner_and_restart_preserve_snapshot() {
        let root = tempfile::tempdir().unwrap();
        let store = Store::open(root.path()).unwrap();
        assert!(matches!(Store::open(root.path()), Err(StoreError::Busy)));
        let snapshot = super::super::tests::fixture();
        store.save(&snapshot).unwrap();
        drop(store);
        assert_eq!(
            Store::open(root.path()).unwrap().load().unwrap().0.snapshot,
            snapshot
        );
    }
    #[test]
    fn corrupted_primary_recovers_previous_but_future_schema_is_preserved() {
        let root = tempfile::tempdir().unwrap();
        let store = Store::open(root.path()).unwrap();
        let snapshot = super::super::tests::fixture();
        store.save(&snapshot).unwrap();
        store.save(&snapshot).unwrap();
        store.write("workspace.json", b"interrupted").unwrap();
        assert_eq!(store.load().unwrap(), (snapshot.clone().into(), true));
        store
            .write("workspace.json", br#"{"version":999}"#)
            .unwrap();
        assert_eq!(store.load(), Err(StoreError::Version));
        assert_eq!(store.save(&snapshot), Err(StoreError::Version));
    }
    #[test]
    fn start_clean_does_not_resurrect_backup() {
        let root = tempfile::tempdir().unwrap();
        let store = Store::open(root.path()).unwrap();
        store.save(&super::super::tests::fixture()).unwrap();
        store.save(&Snapshot::default()).unwrap();
        assert_eq!(store.load().unwrap().0.snapshot, Snapshot::default());
        store.write("workspace.json", b"bad").unwrap();
        assert_eq!(store.load().unwrap().0.snapshot, Snapshot::default());
    }
    fn completion(service: &mut RecoveryService) -> Result<StoreOutcome, StoreError> {
        let until = Instant::now() + Duration::from_secs(3);
        loop {
            if let Some(result) = service.take(Instant::now()) {
                return result;
            }
            assert!(
                Instant::now() < until,
                "recovery worker did not publish completion"
            );
            std::thread::sleep(Duration::from_millis(2));
        }
    }
    #[test]
    fn worker_coalesces_checkpoints_and_preserves_the_latest_on_shutdown() {
        let root = tempfile::tempdir().unwrap();
        let mut service = RecoveryService::new(root.path().into(), Arc::new(|| {}));
        assert!(service.load());
        assert!(matches!(
            completion(&mut service),
            Ok(StoreOutcome::Loaded { .. })
        ));
        // A macOS dock reopen reuses the same lifetime lock, never competes
        // with itself or creates a second disk owner.
        assert!(service.load());
        assert!(matches!(
            completion(&mut service),
            Ok(StoreOutcome::Loaded { .. })
        ));
        let mut snapshot = super::super::tests::fixture();
        for width in 400..600 {
            snapshot.windows[0].width = width;
            service.save(snapshot.clone());
        }
        assert!(service.shutdown(Duration::from_secs(3)));
        drop(service);
        assert_eq!(
            Store::open(root.path()).unwrap().load().unwrap().0.snapshot,
            snapshot
        );
    }
    #[test]
    fn capture_failure_keeps_candidate_until_a_queued_checkpoint_succeeds() {
        let root = tempfile::tempdir().unwrap();
        let original = super::super::tests::fixture();
        {
            let store = Store::open(root.path()).unwrap();
            store.save(&original).unwrap();
        }
        let mut service = RecoveryService::new(root.path().into(), Arc::new(|| {}));
        assert!(service.load());
        assert!(completion(&mut service).is_ok());
        let mut failed = original.clone();
        if let super::super::Node::Pane { sessions, .. } =
            &mut failed.windows[0].tabs[0].nodes[0]
        {
            sessions[0].source = Some(super::super::HistorySource::unavailable());
        }
        service.save_and_retire(failed.into());
        let mut latest = original.clone();
        latest.windows[0].width = 1200;
        service.save(latest.clone());
        assert!(matches!(completion(&mut service), Err(StoreError::Capture)));
        assert!(service.shutdown(Duration::from_secs(3)));
        drop(service);
        let store = Store::open(root.path()).unwrap();
        assert_eq!(store.load().unwrap().0.snapshot, latest);
        assert!(!store.directory.join("restore.json").exists());
        assert!(!store.directory.join("previous.json").exists());
    }
    #[test]
    fn restore_preparation_drops_missing_local_cwd_without_probing_guest_paths() {
        let root = tempfile::tempdir().unwrap();
        let mut service = RecoveryService::new(root.path().into(), Arc::new(|| {}));
        assert!(service.load());
        assert!(completion(&mut service).is_ok());
        let mut snapshot = super::super::tests::fixture();
        if let super::super::Node::Pane { sessions, .. } =
            &mut snapshot.windows[0].tabs[0].nodes[0]
        {
            sessions[0].cwd =
                Some(root.path().join("deleted-folder").to_str().unwrap().into());
            sessions.push(super::super::Session {
                history: None,
                source: None,
                profile: super::super::Profile::Wsl {
                    distribution: Some("Example".into()),
                },
                cwd: Some("/unavailable/guest/folder".into()),
                disconnected: false,
            });
        }
        assert!(service.prepare(snapshot));
        let StoreOutcome::Prepared {
            snapshot,
            missing_folders,
        } = completion(&mut service).unwrap()
        else {
            panic!("wrong operation")
        };
        assert_eq!(missing_folders, 1);
        let super::super::Node::Pane { sessions, .. } =
            &snapshot.windows[0].tabs[0].nodes[0]
        else {
            panic!("wrong topology")
        };
        assert_eq!(sessions[0].cwd, None);
        assert_eq!(
            sessions[1].cwd.as_deref(),
            Some("/unavailable/guest/folder")
        );
    }
    #[test]
    fn oversized_or_nonregular_storage_never_replaces_the_checkpoint() {
        let root = tempfile::tempdir().unwrap();
        let store = Store::open(root.path()).unwrap();
        let snapshot = super::super::tests::fixture();
        store.save(&snapshot).unwrap();
        std::fs::remove_file(store.directory.join("previous.json")).unwrap();
        std::fs::create_dir(store.directory.join("previous.json")).unwrap();
        assert!(store.save(&snapshot).is_err());
        assert_eq!(store.read("workspace.json").unwrap(), Some(snapshot.into()));
        assert!(store
            .write("workspace.json", &vec![0; MAX_BYTES + 1])
            .is_err());
    }
}
