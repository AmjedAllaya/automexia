use std::collections::{BTreeSet, VecDeque};
use std::sync::atomic::{AtomicU32, Ordering};
#[cfg(not(target_arch = "wasm32"))]
use std::sync::mpsc::{self, SyncSender, TrySendError};
use std::sync::{OnceLock, RwLock, RwLockReadGuard, RwLockWriteGuard};
#[cfg(not(target_arch = "wasm32"))]
use std::thread;

use super::api::{SemanticSeverity, SessionFacts};
use super::builtins::devops::{self, DevOpsSnapshot};
use super::marketplace::{self, MarketItem};
use super::state;

static ACTIVATION_GENERATION: AtomicU32 = AtomicU32::new(1);
static DEVOPS_GENERATION: AtomicU32 = AtomicU32::new(1);
static DEVOPS_COMPLETION_COUNTER: AtomicU32 = AtomicU32::new(1);
const DEVOPS_CONTEXT_CACHE_LIMIT: usize = 32;

#[derive(Debug)]
struct RuntimeState {
    installed: BTreeSet<String>,
    /// Bounded cache keyed by terminal session/route id. Each entry carries
    /// its own completion revision so another pane finishing discovery cannot
    /// accidentally acknowledge this pane's pending refresh.
    devops_snapshots: VecDeque<(usize, u32, SessionFacts, DevOpsSnapshot)>,
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

    fn devops_snapshot(&self, session_id: usize) -> (u32, Option<SessionFacts>, DevOpsSnapshot) {
        self.devops_snapshots
            .iter()
            .find(|(key, _, _, _)| *key == session_id)
            .map(|(_, revision, session, snapshot)| {
                (*revision, Some(session.clone()), snapshot.clone())
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
        if let Some((_, existing_revision, existing_session, existing)) = self
            .devops_snapshots
            .iter_mut()
            .find(|(key, _, _, _)| *key == session_id)
        {
            *existing_revision = revision;
            *existing_session = session;
            *existing = snapshot;
            return;
        }

        if self.devops_snapshots.len() >= DEVOPS_CONTEXT_CACHE_LIMIT {
            self.devops_snapshots.pop_front();
        }
        self.devops_snapshots
            .push_back((session_id, revision, session, snapshot));
    }

}

fn runtime() -> &'static RwLock<RuntimeState> {
    static RUNTIME: OnceLock<RwLock<RuntimeState>> = OnceLock::new();
    RUNTIME.get_or_init(|| RwLock::new(RuntimeState::load()))
}

fn read_runtime() -> RwLockReadGuard<'static, RuntimeState> {
    runtime().read().unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn write_runtime() -> RwLockWriteGuard<'static, RuntimeState> {
    runtime().write().unwrap_or_else(|poisoned| poisoned.into_inner())
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
    let manifest = marketplace::descriptor(id).ok_or_else(|| format!("unknown extension: {id}"))?;
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

pub fn devops_snapshot(
    session_id: usize,
) -> (u32, Option<SessionFacts>, DevOpsSnapshot) {
    read_runtime().devops_snapshot(session_id)
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug)]
struct RefreshRequest {
    session: SessionFacts,
}

#[cfg(not(target_arch = "wasm32"))]
struct Worker {
    sender: SyncSender<RefreshRequest>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RefreshSubmission {
    Queued,
    Busy,
    Unavailable,
}

#[cfg(not(target_arch = "wasm32"))]
fn worker() -> Option<&'static Worker> {
    static WORKER: OnceLock<Option<Worker>> = OnceLock::new();
    WORKER
        .get_or_init(|| {
            // Capacity 1 provides backpressure. Rendering never blocks waiting
            // for extension discovery; redundant refreshes are coalesced.
            let (sender, receiver) = mpsc::sync_channel::<RefreshRequest>(1);
            match thread::Builder::new()
                .name("automexia-extension-worker".to_owned())
                .spawn(move || {
                    while let Ok(request) = receiver.recv() {
                        if !read_runtime().installed.contains(devops::ID) {
                            continue;
                        }
                        let session = request.session;
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
                        // Give every completed request a session-local revision.
                        // A global generation is still used as the cheap renderer
                        // wake-up signal, while the revision prevents another pane's
                        // completion from acknowledging this pane's pending refresh.
                        let revision = DEVOPS_COMPLETION_COUNTER
                            .fetch_add(1, Ordering::AcqRel)
                            .wrapping_add(1);
                        {
                            let mut runtime = write_runtime();
                            runtime.put_devops_snapshot(session, revision, snapshot);
                        }
                        // Publish only after the cache entry is visible.
                        DEVOPS_GENERATION.fetch_add(1, Ordering::Release);
                    }
                }) {
                Ok(_) => Some(Worker { sender }),
                Err(error) => {
                    // Extensions are optional application services. Failure to
                    // start their worker must degrade the HUD, not crash the
                    // terminal engine or prevent a shell from opening.
                    tracing::error!("failed to start Automexia extension worker: {error}");
                    None
                }
            }
        })
        .as_ref()
}

#[cfg(not(target_arch = "wasm32"))]
pub fn ensure_background_services() {
    let _ = worker();
}

/// Request asynchronous context discovery. This function never waits for IO.
/// `Busy` tells a renderer to retry soon rather than waiting a full refresh
/// interval; this prevents one active window from starving another.
#[cfg(not(target_arch = "wasm32"))]
pub fn request_devops_refresh(session: &SessionFacts) -> RefreshSubmission {
    let Some(worker) = worker() else {
        return RefreshSubmission::Unavailable;
    };
    let request = RefreshRequest {
        session: session.clone(),
    };
    match worker.sender.try_send(request) {
        Ok(()) => RefreshSubmission::Queued,
        Err(TrySendError::Full(_)) => RefreshSubmission::Busy,
        Err(TrySendError::Disconnected(_)) => {
            tracing::warn!("Automexia extension worker disconnected");
            RefreshSubmission::Unavailable
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub fn request_devops_refresh(_session: &SessionFacts) -> RefreshSubmission {
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
        assert_eq!(state.devops_snapshot(101).2.environment.as_deref(), Some("a"));
        assert_eq!(state.devops_snapshot(202).0, 20);
        assert_eq!(state.devops_snapshot(202).2.environment.as_deref(), Some("b"));
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
        state.put_devops_snapshot(session(7, "user@host:/mnt/d/work"), 2, snapshot("wsl"));
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
            .all(|(session_id, _, _, _)| *session_id != 0));
        assert_eq!(
            state
                .devops_snapshot(DEVOPS_CONTEXT_CACHE_LIMIT + 4)
                .2
                .environment,
            Some((DEVOPS_CONTEXT_CACHE_LIMIT + 4).to_string())
        );
    }
}
