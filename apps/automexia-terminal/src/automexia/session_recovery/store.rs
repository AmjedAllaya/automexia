use super::{Snapshot, MAX_BYTES};
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
}
impl From<private_fs::PrivateFsError> for StoreError {
    fn from(_: private_fs::PrivateFsError) -> Self {
        Self::Io
    }
}

pub enum StoreOutcome {
    Loaded {
        snapshot: Snapshot,
        recovered: bool,
    },
    Prepared {
        snapshot: Snapshot,
        missing_folders: usize,
    },
    Saved,
}
enum Operation {
    Load,
    Save(Snapshot),
    Prepare(Snapshot),
}
struct Request {
    operation: Operation,
}
struct Store {
    directory: PathBuf,
    _lock: WriteLock,
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
        Ok(Self {
            directory,
            _lock: lock,
        })
    }
    fn read(&self, name: &str) -> Result<Option<Snapshot>, StoreError> {
        private_fs::validate_private_child_directory(&self.directory)?;
        private_fs::read_bounded_regular(&self.directory.join(name), MAX_BYTES)?
            .map(|bytes| Snapshot::decode(&bytes))
            .transpose()
    }
    fn load(&self) -> Result<(Snapshot, bool), StoreError> {
        match self.read("workspace.json") {
            Ok(Some(snapshot)) => Ok((snapshot, false)),
            Err(StoreError::Version) => Err(StoreError::Version),
            primary => match self.read("previous.json") {
                Ok(Some(snapshot)) => Ok((snapshot, true)),
                Ok(None) if matches!(primary, Ok(None)) => {
                    Ok((Snapshot::default(), false))
                }
                Err(StoreError::Version) => Err(StoreError::Version),
                _ => Err(StoreError::Invalid),
            },
        }
    }
    fn write(&self, name: &str, bytes: &[u8]) -> Result<(), StoreError> {
        private_fs::atomic_write_private(
            &self.directory,
            &self.directory.join(name),
            bytes,
            MAX_BYTES,
            ".session-staged-",
        )
        .map_err(Into::into)
    }
    fn save(&self, snapshot: &Snapshot) -> Result<(), StoreError> {
        let candidate = snapshot.encode()?;
        let (previous, _) = self.load()?;
        if snapshot.windows.is_empty() {
            // A successful clean decision is authoritative even if backup
            // replacement fails or the process dies immediately afterwards.
            self.write("workspace.json", &candidate)?;
            return self.write("previous.json", &candidate);
        }
        self.write("previous.json", &previous.encode()?)?;
        self.write("workspace.json", &candidate)
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
    latest: Option<Snapshot>,
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
                            let (snapshot, recovered) =
                                owner.as_ref().ok_or(StoreError::Worker)?.load()?;
                            Ok(StoreOutcome::Loaded {
                                snapshot,
                                recovered,
                            })
                        }
                        Operation::Save(snapshot) => {
                            owner.as_ref().ok_or(StoreError::Worker)?.save(&snapshot)?;
                            Ok(StoreOutcome::Saved)
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
    pub fn save(&mut self, snapshot: Snapshot) {
        if self.failed {
            return;
        }
        if self.pending {
            self.latest = Some(snapshot);
        } else {
            self.submit(Operation::Save(snapshot));
        }
    }
    pub fn take(&mut self, now: Instant) -> Option<Result<StoreOutcome, StoreError>> {
        if self.failed {
            return None;
        }
        if let Ok(result) = self.completed.try_recv() {
            self.pending = false;
            self.deadline = None;
            if result.is_err() {
                self.failed = true;
                self.latest = None;
            } else if let Some(snapshot) = self.latest.take() {
                self.submit(Operation::Save(snapshot));
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
    fn exclusive_owner_and_restart_preserve_snapshot() {
        let root = tempfile::tempdir().unwrap();
        let store = Store::open(root.path()).unwrap();
        assert!(matches!(Store::open(root.path()), Err(StoreError::Busy)));
        let snapshot = super::super::tests::fixture();
        store.save(&snapshot).unwrap();
        drop(store);
        assert_eq!(
            Store::open(root.path()).unwrap().load().unwrap().0,
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
        assert_eq!(store.load().unwrap(), (snapshot.clone(), true));
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
        assert_eq!(store.load().unwrap().0, Snapshot::default());
        store.write("workspace.json", b"bad").unwrap();
        assert_eq!(store.load().unwrap().0, Snapshot::default());
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
            Store::open(root.path()).unwrap().load().unwrap().0,
            snapshot
        );
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
        assert_eq!(store.read("workspace.json").unwrap(), Some(snapshot));
        assert!(store
            .write("workspace.json", &vec![0; MAX_BYTES + 1])
            .is_err());
    }
}
