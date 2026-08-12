use std::collections::{BTreeSet, VecDeque};
use std::sync::atomic::{AtomicU32, Ordering};
#[cfg(not(target_arch = "wasm32"))]
use std::sync::mpsc::{self, SyncSender, TrySendError};
#[cfg(not(target_arch = "wasm32"))]
use std::sync::{Mutex, MutexGuard};
use std::sync::{OnceLock, RwLock, RwLockReadGuard, RwLockWriteGuard};
#[cfg(not(target_arch = "wasm32"))]
use std::thread::{self, JoinHandle};
#[cfg(not(target_arch = "wasm32"))]
use std::time::{Duration, Instant};

use super::api::{SemanticSeverity, SessionFacts};
use super::builtins::devops::{self, DevOpsSnapshot};
use super::marketplace::{self, MarketItem};
use super::state;

static ACTIVATION_GENERATION: AtomicU32 = AtomicU32::new(1);
static DEVOPS_GENERATION: AtomicU32 = AtomicU32::new(1);
static DEVOPS_COMPLETION_COUNTER: AtomicU32 = AtomicU32::new(1);
const DEVOPS_CONTEXT_CACHE_LIMIT: usize = 32;
#[cfg(not(target_arch = "wasm32"))]
const MAX_SESSION_TITLE_BYTES: usize = 4 * 1024;
#[cfg(not(target_arch = "wasm32"))]
const DEVOPS_REUSE_MAX_AGE: Duration = Duration::from_secs(5);

#[derive(Debug)]
struct DevOpsCacheEntry {
    session_id: usize,
    revision: u32,
    session: SessionFacts,
    snapshot: DevOpsSnapshot,
    #[cfg(not(target_arch = "wasm32"))]
    completed_at: Instant,
}

#[derive(Debug)]
struct RuntimeState {
    installed: BTreeSet<String>,
    /// Bounded cache keyed by terminal session/route id. Each entry carries
    /// its own completion revision so another pane finishing discovery cannot
    /// accidentally acknowledge this pane's pending refresh.
    devops_snapshots: VecDeque<DevOpsCacheEntry>,
}

impl RuntimeState {
    fn load() -> Self {
        let installed = marketplace::descriptors()
            .iter()
            .filter(|manifest| state::is_installed(manifest))
            .map(|manifest| manifest.id.to_owned())
            .collect();
        Self {
            installed,
            devops_snapshots: VecDeque::with_capacity(DEVOPS_CONTEXT_CACHE_LIMIT),
        }
    }

    fn devops_snapshot(
        &self,
        session_id: usize,
    ) -> (u32, Option<SessionFacts>, DevOpsSnapshot) {
        self.devops_snapshots
            .iter()
            .find(|entry| entry.session_id == session_id)
            .map(|entry| {
                (
                    entry.revision,
                    Some(entry.session.clone()),
                    entry.snapshot.clone(),
                )
            })
            .unwrap_or_default()
    }

    fn put_devops_snapshot(
        &mut self,
        session: SessionFacts,
        revision: u32,
        snapshot: DevOpsSnapshot,
    ) {
        let session_id = session.session_id;
        if let Some(existing) = self
            .devops_snapshots
            .iter_mut()
            .find(|entry| entry.session_id == session_id)
        {
            existing.revision = revision;
            existing.session = session;
            existing.snapshot = snapshot;
            #[cfg(not(target_arch = "wasm32"))]
            {
                existing.completed_at = Instant::now();
            }
            return;
        }

        if self.devops_snapshots.len() >= DEVOPS_CONTEXT_CACHE_LIMIT {
            self.devops_snapshots.pop_front();
        }
        self.devops_snapshots.push_back(DevOpsCacheEntry {
            session_id,
            revision,
            session,
            snapshot,
            #[cfg(not(target_arch = "wasm32"))]
            completed_at: Instant::now(),
        });
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn reusable_devops_snapshot(&self, session: &SessionFacts) -> Option<DevOpsSnapshot> {
        if !session.shell_integration {
            return None;
        }

        self.devops_snapshots
            .iter()
            .rev()
            .filter(|entry| entry.session_id != session.session_id)
            .find(|entry| {
                entry.completed_at.elapsed() <= DEVOPS_REUSE_MAX_AGE
                    && equivalent_shell_context(&entry.session, session)
            })
            .map(|entry| entry.snapshot.clone())
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn equivalent_shell_context(left: &SessionFacts, right: &SessionFacts) -> bool {
    left.shell_integration
        && left.cwd == right.cwd
        && left.title == right.title
        && left.distro == right.distro
        && left.os_version == right.os_version
        && left.shell_name == right.shell_name
}

fn runtime() -> &'static RwLock<RuntimeState> {
    static RUNTIME: OnceLock<RwLock<RuntimeState>> = OnceLock::new();
    RUNTIME.get_or_init(|| RwLock::new(RuntimeState::load()))
}

fn read_runtime() -> RwLockReadGuard<'static, RuntimeState> {
    runtime()
        .read()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn write_runtime() -> RwLockWriteGuard<'static, RuntimeState> {
    runtime()
        .write()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

pub fn generation() -> u32 {
    ACTIVATION_GENERATION.load(Ordering::Acquire)
}

pub fn is_installed(id: &str) -> bool {
    let installed = read_runtime().installed.contains(id);
    #[cfg(not(target_arch = "wasm32"))]
    if installed && id == devops::ID {
        // Renderer construction asks this before the paint loop starts, so the
        // worker is created outside the first HUD refresh.
        ensure_background_services();
    }
    installed
}

pub fn toggle(id: &str) -> Result<bool, String> {
    let manifest =
        marketplace::descriptor(id).ok_or_else(|| format!("unknown extension: {id}"))?;
    let next = !read_runtime().installed.contains(id);

    // Persistence may touch disk. Never hold the shared runtime lock while
    // performing IO or the renderer could be delayed waiting for a cached
    // model read. UI command routing serializes activation changes today.
    state::set_installed(manifest, next)?;

    let mut runtime = write_runtime();
    let cleared_devops = if next {
        runtime.installed.insert(id.to_owned());
        false
    } else {
        runtime.installed.remove(id);
        if id == devops::ID {
            runtime.devops_snapshots.clear();
            true
        } else {
            false
        }
    };
    drop(runtime);
    if cleared_devops {
        DEVOPS_GENERATION.fetch_add(1, Ordering::AcqRel);
    }
    #[cfg(not(target_arch = "wasm32"))]
    if next && id == devops::ID {
        ensure_background_services();
    }
    ACTIVATION_GENERATION.fetch_add(1, Ordering::AcqRel);
    Ok(next)
}

pub fn market_items() -> Vec<MarketItem> {
    let runtime = read_runtime();
    marketplace::descriptors()
        .iter()
        .map(|manifest| MarketItem {
            id: manifest.id.to_owned(),
            name: manifest.name.to_owned(),
            description: manifest.description.to_owned(),
            installed: runtime.installed.contains(manifest.id),
        })
        .collect()
}

pub fn classify_row_text(text: &str) -> Option<SemanticSeverity> {
    devops::classify_row_text(text)
}

pub fn devops_generation() -> u32 {
    DEVOPS_GENERATION.load(Ordering::Acquire)
}

pub fn devops_snapshot(session_id: usize) -> (u32, Option<SessionFacts>, DevOpsSnapshot) {
    read_runtime().devops_snapshot(session_id)
}

/// One-shot notification invoked after a refreshed snapshot is visible.
///
/// The UI supplies a route-scoped event-loop wake-up here. Keeping the
/// callback renderer-agnostic lets the extension runtime stay usable without
/// depending on the desktop event types.
pub type DevOpsRefreshCompletion = Box<dyn FnOnce() + Send + 'static>;

#[cfg(not(target_arch = "wasm32"))]
struct RefreshRequest {
    session: SessionFacts,
    completion: Option<DevOpsRefreshCompletion>,
}

#[cfg(not(target_arch = "wasm32"))]
enum WorkerMessage {
    Refresh(RefreshRequest),
    Shutdown,
}

#[cfg(not(target_arch = "wasm32"))]
struct Worker {
    sender: SyncSender<WorkerMessage>,
    handle: JoinHandle<()>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RefreshSubmission {
    Queued,
    Busy,
    Rejected,
    Unavailable,
}

#[cfg(not(target_arch = "wasm32"))]
fn spawn_worker() -> Option<Worker> {
    // Capacity 1 provides backpressure. Rendering never blocks waiting for
    // extension discovery; redundant refreshes are coalesced.
    let (sender, receiver) = mpsc::sync_channel::<WorkerMessage>(1);
    match thread::Builder::new()
        .name("automexia-extension-worker".to_owned())
        .spawn(move || {
            while let Ok(message) = receiver.recv() {
                match message {
                    WorkerMessage::Shutdown => break,
                    WorkerMessage::Refresh(request) => {
                        if !read_runtime().installed.contains(devops::ID) {
                            continue;
                        }
                        let RefreshRequest {
                            session,
                            completion,
                        } = request;
                        let snapshot = devops::detect(&session);
                        tracing::debug!(
                            session_id = session.session_id,
                            terminal_title = %session.title,
                            ?snapshot,
                            "Automexia DevOps discovery completed"
                        );
                        // Activation can change while bounded discovery is in
                        // flight. Never publish a snapshot after the user has
                        // disabled the extension.
                        if !read_runtime().installed.contains(devops::ID) {
                            continue;
                        }
                        publish_devops_snapshot(session, snapshot, completion);
                    }
                }
            }
        }) {
        Ok(handle) => Some(Worker { sender, handle }),
        Err(error) => {
            // Extensions are optional application services. Failure to start
            // their worker must degrade the HUD, not crash the terminal engine
            // or prevent a shell from opening.
            tracing::error!("failed to start Automexia extension worker: {error}");
            None
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn publish_devops_snapshot(
    session: SessionFacts,
    snapshot: DevOpsSnapshot,
    completion: Option<DevOpsRefreshCompletion>,
) {
    // Give every completed request a session-local revision. A global
    // generation is still used as the cheap renderer synchronization signal,
    // while the revision prevents another pane's completion from acknowledging
    // this pane's pending refresh.
    let revision = DEVOPS_COMPLETION_COUNTER
        .fetch_add(1, Ordering::AcqRel)
        .wrapping_add(1);
    {
        let mut runtime = write_runtime();
        runtime.put_devops_snapshot(session, revision, snapshot);
    }
    // Publish before waking the window. The render triggered by `completion`
    // must always be able to observe the new cache entry on its first frame.
    DEVOPS_GENERATION.fetch_add(1, Ordering::Release);
    if let Some(completion) = completion {
        completion();
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn seed_devops_snapshot(session: &SessionFacts) -> bool {
    {
        let mut runtime = write_runtime();
        if runtime
            .devops_snapshots
            .iter()
            .any(|entry| entry.session == *session)
        {
            return false;
        }
        let Some(snapshot) = runtime.reusable_devops_snapshot(session) else {
            return false;
        };
        let revision = DEVOPS_COMPLETION_COUNTER
            .fetch_add(1, Ordering::AcqRel)
            .wrapping_add(1);
        runtime.put_devops_snapshot(session.clone(), revision, snapshot);
    }
    DEVOPS_GENERATION.fetch_add(1, Ordering::Release);
    true
}

#[cfg(not(target_arch = "wasm32"))]
fn worker_slot() -> &'static Mutex<Option<Worker>> {
    static WORKER: OnceLock<Mutex<Option<Worker>>> = OnceLock::new();
    WORKER.get_or_init(|| Mutex::new(None))
}

#[cfg(not(target_arch = "wasm32"))]
fn lock_worker() -> MutexGuard<'static, Option<Worker>> {
    worker_slot()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[cfg(not(target_arch = "wasm32"))]
fn ensure_worker(slot: &mut Option<Worker>) -> bool {
    if slot
        .as_ref()
        .is_some_and(|worker| worker.handle.is_finished())
    {
        if let Some(worker) = slot.take() {
            let _ = worker.handle.join();
        }
    }
    if slot.is_none() {
        *slot = spawn_worker();
    }
    slot.is_some()
}

#[cfg(not(target_arch = "wasm32"))]
pub fn ensure_background_services() {
    let _ = ensure_worker(&mut lock_worker());
}

/// Stop and join the optional worker before process teardown. This is called
/// only after the event loop exits, so waiting here cannot delay rendering.
#[cfg(not(target_arch = "wasm32"))]
pub fn shutdown_background_services() {
    let worker = lock_worker().take();
    if let Some(worker) = worker {
        let _ = worker.sender.send(WorkerMessage::Shutdown);
        if worker.handle.join().is_err() {
            tracing::warn!("Automexia extension worker panicked during shutdown");
        }
    }
}

/// Request asynchronous context discovery. This function never waits for IO.
/// `Busy` tells a renderer to retry soon rather than waiting a full refresh
/// interval; this prevents one active window from starving another.
#[cfg(not(target_arch = "wasm32"))]
pub fn request_devops_refresh(
    session: &SessionFacts,
    completion: Option<DevOpsRefreshCompletion>,
) -> RefreshSubmission {
    if session.title.len() > MAX_SESSION_TITLE_BYTES {
        tracing::warn!(
            session_id = session.session_id,
            title_bytes = session.title.len(),
            "refusing oversized extension refresh context"
        );
        return RefreshSubmission::Rejected;
    }

    // A new tab/split usually starts in the same shell and directory as its
    // source. Reuse only a very recent equivalent snapshot for the first frame,
    // then still queue live discovery below. This removes duplicate WSL/Docker
    // startup latency without allowing one pane's context to leak into another.
    let _ = seed_devops_snapshot(session);

    let mut slot = lock_worker();
    if !ensure_worker(&mut slot) {
        return RefreshSubmission::Unavailable;
    }
    let message = WorkerMessage::Refresh(RefreshRequest {
        session: session.clone(),
        completion,
    });
    match slot
        .as_ref()
        .expect("worker was ensured")
        .sender
        .try_send(message)
    {
        Ok(()) => RefreshSubmission::Queued,
        Err(TrySendError::Full(_)) => RefreshSubmission::Busy,
        Err(TrySendError::Disconnected(message)) => {
            tracing::warn!("Automexia extension worker disconnected");
            if let Some(worker) = slot.take() {
                let _ = worker.handle.join();
            }
            if !ensure_worker(&mut slot) {
                return RefreshSubmission::Unavailable;
            }
            match slot
                .as_ref()
                .expect("replacement worker was ensured")
                .sender
                .try_send(message)
            {
                Ok(()) => RefreshSubmission::Queued,
                Err(TrySendError::Full(_)) => RefreshSubmission::Busy,
                Err(TrySendError::Disconnected(_)) => RefreshSubmission::Unavailable,
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub fn request_devops_refresh(
    _session: &SessionFacts,
    _completion: Option<DevOpsRefreshCompletion>,
) -> RefreshSubmission {
    RefreshSubmission::Unavailable
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session(id: usize, title: &str) -> SessionFacts {
        SessionFacts {
            session_id: id,
            cwd: None,
            title: title.to_owned(),
            distro: None,
            os_version: None,
            shell_name: None,
            shell_integration: true,
            shell_pid: 0,
        }
    }

    fn snapshot(label: &str) -> DevOpsSnapshot {
        DevOpsSnapshot {
            environment: Some(label.to_owned()),
            ..DevOpsSnapshot::default()
        }
    }

    #[test]
    fn devops_cache_is_scoped_by_terminal_session() {
        let mut state = RuntimeState {
            installed: BTreeSet::new(),
            devops_snapshots: VecDeque::new(),
        };
        state.put_devops_snapshot(session(101, "one"), 10, snapshot("a"));
        state.put_devops_snapshot(session(202, "two"), 20, snapshot("b"));

        assert_eq!(state.devops_snapshot(101).0, 10);
        assert_eq!(state.devops_snapshot(101).1.unwrap().title, "one");
        assert_eq!(
            state.devops_snapshot(101).2.environment.as_deref(),
            Some("a")
        );
        assert_eq!(state.devops_snapshot(202).0, 20);
        assert_eq!(
            state.devops_snapshot(202).2.environment.as_deref(),
            Some("b")
        );
        assert_eq!(
            state.devops_snapshot(999),
            (0, None, DevOpsSnapshot::default())
        );
    }

    #[test]
    fn cache_keeps_the_session_facts_that_produced_the_snapshot() {
        let mut state = RuntimeState {
            installed: BTreeSet::new(),
            devops_snapshots: VecDeque::new(),
        };
        state.put_devops_snapshot(session(7, "powershell"), 1, snapshot("host"));
        state.put_devops_snapshot(
            session(7, "user@host:/mnt/d/work"),
            2,
            snapshot("wsl"),
        );
        let (revision, facts, value) = state.devops_snapshot(7);
        assert_eq!(revision, 2);
        assert_eq!(facts.unwrap().title, "user@host:/mnt/d/work");
        assert_eq!(value.environment.as_deref(), Some("wsl"));
    }

    #[test]
    fn devops_cache_is_bounded() {
        let mut state = RuntimeState {
            installed: BTreeSet::new(),
            devops_snapshots: VecDeque::new(),
        };
        for index in 0..(DEVOPS_CONTEXT_CACHE_LIMIT + 5) {
            state.put_devops_snapshot(
                session(index, &format!("session-{index}")),
                index as u32 + 1,
                snapshot(&index.to_string()),
            );
        }
        assert_eq!(state.devops_snapshots.len(), DEVOPS_CONTEXT_CACHE_LIMIT);
        assert!(state
            .devops_snapshots
            .iter()
            .all(|entry| entry.session_id != 0));
        assert_eq!(
            state
                .devops_snapshot(DEVOPS_CONTEXT_CACHE_LIMIT + 4)
                .2
                .environment,
            Some((DEVOPS_CONTEXT_CACHE_LIMIT + 4).to_string())
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn bounded_worker_queue_reports_pressure_and_disconnects() {
        let (sender, receiver) = mpsc::sync_channel(1);
        sender.send(WorkerMessage::Shutdown).unwrap();
        assert!(matches!(
            sender.try_send(WorkerMessage::Shutdown),
            Err(TrySendError::Full(_))
        ));
        drop(receiver);
        assert!(matches!(
            sender.try_send(WorkerMessage::Shutdown),
            Err(TrySendError::Disconnected(_))
        ));
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn oversized_context_is_rejected_before_worker_submission() {
        let mut facts = session(88, "small");
        facts.title = "x".repeat(MAX_SESSION_TITLE_BYTES + 1);
        assert_eq!(
            request_devops_refresh(&facts, None),
            RefreshSubmission::Rejected
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn completion_runs_after_the_snapshot_is_visible() {
        let session = session(usize::MAX - 17, "amjed@host:/mnt/d/work");
        let expected_session = session.clone();
        let (sender, receiver) = mpsc::channel();

        publish_devops_snapshot(
            session,
            snapshot("wsl"),
            Some(Box::new(move || {
                let cached = devops_snapshot(expected_session.session_id);
                sender.send(cached).unwrap();
            })),
        );

        let (revision, cached_session, cached_snapshot) = receiver.recv().unwrap();
        assert!(revision > 0);
        assert_eq!(cached_session, Some(expected_session));
        assert_eq!(cached_snapshot.environment.as_deref(), Some("wsl"));
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn equivalent_new_session_reuses_a_fresh_snapshot() {
        let mut state = RuntimeState {
            installed: BTreeSet::new(),
            devops_snapshots: VecDeque::new(),
        };
        let mut source = session(301, "amjed@host:/mnt/d/work");
        source.cwd = Some("D:\\work".into());
        source.distro = Some("Ubuntu".to_string());
        source.shell_name = Some("bash".to_string());
        let mut target = source.clone();
        target.session_id = 302;

        state.put_devops_snapshot(source, 44, snapshot("docker"));

        assert_eq!(
            state
                .reusable_devops_snapshot(&target)
                .and_then(|value| value.environment),
            Some("docker".to_string())
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn snapshot_reuse_rejects_other_paths_and_unintegrated_shells() {
        let mut state = RuntimeState {
            installed: BTreeSet::new(),
            devops_snapshots: VecDeque::new(),
        };
        let mut source = session(401, "PowerShell - D:/work");
        source.cwd = Some("D:\\work".into());
        source.shell_name = Some("PowerShell".to_string());
        state.put_devops_snapshot(source.clone(), 55, snapshot("staging"));

        let mut other_path = source.clone();
        other_path.session_id = 402;
        other_path.cwd = Some("D:\\production".into());
        other_path.title = "PowerShell - D:/production".to_string();
        assert!(state.reusable_devops_snapshot(&other_path).is_none());

        let mut unintegrated = source;
        unintegrated.session_id = 403;
        unintegrated.shell_integration = false;
        assert!(state.reusable_devops_snapshot(&unintegrated).is_none());

        let mut expired = unintegrated;
        expired.session_id = 404;
        expired.shell_integration = true;
        state.devops_snapshots[0].completed_at =
            Instant::now() - DEVOPS_REUSE_MAX_AGE - Duration::from_millis(1);
        assert!(state.reusable_devops_snapshot(&expired).is_none());
    }
}
