//! Lifecycle-owned CP2.2 service activation and latest-state search worker.

use std::{
    collections::BTreeMap,
    fmt,
    path::Path,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Condvar, Mutex, MutexGuard,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use automexia_devops::actions::{
    validate_search_query, ActionIndex, ActionLayer, ActionScope, ActionSearchHit,
    IndexError, LayerIdentity, SearchContext,
};
use automexia_extension_runtime::CompletionWake;

use super::{
    QuickActionMonitor, QuickActionService, QuickActionStore, ServiceStatus, StoreError,
    StoreErrorCode,
};

const SEARCH_POLL_INTERVAL: Duration = Duration::from_millis(50);
const SEARCH_COALESCE_INTERVAL: Duration = Duration::from_millis(12);
const WATCH_DEBOUNCE: Duration = Duration::from_millis(75);
const WATCH_RECONCILE: Duration = Duration::from_secs(5);
const MAX_RESULT_ROUTES: usize = 32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QuickActionRuntimeErrorCode {
    Store(StoreErrorCode),
    Index,
    InvalidQuery,
    RouteCapacity,
    WorkerUnavailable,
}

impl fmt::Display for QuickActionRuntimeErrorCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Store(code) => write!(formatter, "store-{}", code.as_str()),
            Self::Index => formatter.write_str("index-error"),
            Self::InvalidQuery => formatter.write_str("invalid-query"),
            Self::RouteCapacity => formatter.write_str("route-capacity"),
            Self::WorkerUnavailable => formatter.write_str("worker-unavailable"),
        }
    }
}

impl From<StoreError> for QuickActionRuntimeErrorCode {
    fn from(value: StoreError) -> Self {
        Self::Store(value.code())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QuickActionRuntimeStatus {
    Ready {
        revision: u64,
    },
    Recovered {
        revision: u64,
    },
    Stale {
        revision: u64,
        error: StoreErrorCode,
    },
    Disabled {
        error: QuickActionRuntimeErrorCode,
    },
}

#[derive(Clone, Debug)]
pub struct QuickActionSearchResult {
    pub request_id: u64,
    pub route_id: usize,
    pub query: String,
    pub hits: Vec<ActionSearchHit>,
    pub status: QuickActionRuntimeStatus,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SearchSubmission {
    Queued { request_id: u64 },
    Disabled { error: QuickActionRuntimeErrorCode },
}

struct SearchRequest {
    request_id: u64,
    route_id: usize,
    query: String,
    context: SearchContext,
    wake: CompletionWake,
}

#[derive(Default)]
struct PendingState {
    latest_by_route: BTreeMap<usize, SearchRequest>,
    shutdown: bool,
}

struct RuntimeInner {
    service: Option<QuickActionService>,
    disabled: Option<QuickActionRuntimeErrorCode>,
    pending: Arc<(Mutex<PendingState>, Condvar)>,
    latest_requested: Arc<Mutex<BTreeMap<usize, u64>>>,
    results: Arc<Mutex<BTreeMap<usize, QuickActionSearchResult>>>,
    next_request: AtomicU64,
    handle: Mutex<Option<JoinHandle<()>>>,
}

impl Drop for RuntimeInner {
    fn drop(&mut self) {
        {
            let (pending, condition) = &*self.pending;
            let mut state = lock(pending);
            state.shutdown = true;
            state.latest_by_route.clear();
            condition.notify_all();
        }
        if let Some(handle) = lock(&self.handle).take() {
            let _ = handle.join();
        }
    }
}

#[derive(Clone)]
pub struct QuickActionRuntime(Arc<RuntimeInner>);

impl fmt::Debug for QuickActionRuntime {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("QuickActionRuntime")
            .field("status", &self.status())
            .finish_non_exhaustive()
    }
}

impl QuickActionRuntime {
    pub fn open_default() -> Self {
        let root = rio_backend::config::config_dir_path().join("actions");
        Self::open(root).unwrap_or_else(Self::disabled)
    }

    pub fn open(root: impl AsRef<Path>) -> Result<Self, QuickActionRuntimeErrorCode> {
        let store = QuickActionStore::open_or_create(root)?;
        let service = QuickActionService::open(store)?;
        let monitor = QuickActionMonitor::start(
            service.clone(),
            Instant::now(),
            WATCH_DEBOUNCE,
            WATCH_RECONCILE,
        )?;
        let initial = index_for_service(&service)?;
        let pending = Arc::new((Mutex::new(PendingState::default()), Condvar::new()));
        let latest_requested = Arc::new(Mutex::new(BTreeMap::new()));
        let results = Arc::new(Mutex::new(BTreeMap::new()));
        let handle = spawn_worker(
            monitor,
            initial,
            Arc::clone(&pending),
            Arc::clone(&latest_requested),
            Arc::clone(&results),
        )
        .ok_or(QuickActionRuntimeErrorCode::WorkerUnavailable)?;
        Ok(Self(Arc::new(RuntimeInner {
            service: Some(service),
            disabled: None,
            pending,
            latest_requested,
            results,
            next_request: AtomicU64::new(1),
            handle: Mutex::new(Some(handle)),
        })))
    }

    pub fn disabled(error: QuickActionRuntimeErrorCode) -> Self {
        Self(Arc::new(RuntimeInner {
            service: None,
            disabled: Some(error),
            pending: Arc::new((Mutex::new(PendingState::default()), Condvar::new())),
            latest_requested: Arc::new(Mutex::new(BTreeMap::new())),
            results: Arc::new(Mutex::new(BTreeMap::new())),
            next_request: AtomicU64::new(1),
            handle: Mutex::new(None),
        }))
    }

    pub fn service(&self) -> Option<&QuickActionService> {
        self.0.service.as_ref()
    }

    pub fn status(&self) -> QuickActionRuntimeStatus {
        if let Some(error) = self.0.disabled {
            return QuickActionRuntimeStatus::Disabled { error };
        }
        if lock(&self.0.handle)
            .as_ref()
            .is_none_or(JoinHandle::is_finished)
        {
            return QuickActionRuntimeStatus::Disabled {
                error: QuickActionRuntimeErrorCode::WorkerUnavailable,
            };
        }
        let Some(service) = &self.0.service else {
            return QuickActionRuntimeStatus::Disabled {
                error: QuickActionRuntimeErrorCode::WorkerUnavailable,
            };
        };
        status_from_service(service.status())
    }

    pub fn submit(
        &self,
        route_id: usize,
        query: String,
        context: SearchContext,
        wake: CompletionWake,
    ) -> SearchSubmission {
        if let Some(error) = self.0.disabled {
            return SearchSubmission::Disabled { error };
        }
        if validate_search_query(&query).is_err() {
            return SearchSubmission::Disabled {
                error: QuickActionRuntimeErrorCode::InvalidQuery,
            };
        }
        if lock(&self.0.handle)
            .as_ref()
            .is_none_or(JoinHandle::is_finished)
        {
            return SearchSubmission::Disabled {
                error: QuickActionRuntimeErrorCode::WorkerUnavailable,
            };
        }
        let (pending, condition) = &*self.0.pending;
        let mut state = lock(pending);
        if state.shutdown {
            return SearchSubmission::Disabled {
                error: QuickActionRuntimeErrorCode::WorkerUnavailable,
            };
        }
        let mut latest_requested = lock(&self.0.latest_requested);
        if !latest_requested.contains_key(&route_id)
            && latest_requested.len() >= MAX_RESULT_ROUTES
        {
            return SearchSubmission::Disabled {
                error: QuickActionRuntimeErrorCode::RouteCapacity,
            };
        }
        let request_id = self.0.next_request.fetch_add(1, Ordering::AcqRel);
        latest_requested.insert(route_id, request_id);
        drop(latest_requested);
        state.latest_by_route.insert(
            route_id,
            SearchRequest {
                request_id,
                route_id,
                query,
                context,
                wake,
            },
        );
        condition.notify_one();
        SearchSubmission::Queued { request_id }
    }

    pub fn take_result(
        &self,
        route_id: usize,
        minimum_request_id: u64,
    ) -> Option<QuickActionSearchResult> {
        let mut results = lock(&self.0.results);
        let result = results.remove(&route_id)?;
        (result.request_id >= minimum_request_id).then_some(result)
    }

    pub fn forget_route(&self, route_id: usize) {
        let (pending, _) = &*self.0.pending;
        lock(pending).latest_by_route.remove(&route_id);
        lock(&self.0.latest_requested).remove(&route_id);
        lock(&self.0.results).remove(&route_id);
    }
}

fn spawn_worker(
    mut monitor: QuickActionMonitor,
    mut index: ActionIndex,
    pending: Arc<(Mutex<PendingState>, Condvar)>,
    latest_requested: Arc<Mutex<BTreeMap<usize, u64>>>,
    results: Arc<Mutex<BTreeMap<usize, QuickActionSearchResult>>>,
) -> Option<JoinHandle<()>> {
    thread::Builder::new()
        .name("automexia-quick-actions".into())
        .spawn(move || loop {
            // Rebuild after every exact-source reconciliation outcome. A
            // mutation performed through this process's shared service is
            // already published before the watcher sees it, so its subsequent
            // load is legitimately `Unchanged` even though the worker's index
            // still needs the newer generation.
            if monitor.poll(Instant::now()).is_some() {
                if let Ok(candidate) = index_for_service(monitor.service()) {
                    index = candidate;
                }
            }
            let request = {
                let (slot, condition) = &*pending;
                let state = lock(slot);
                let (mut state, _) = condition
                    .wait_timeout_while(state, SEARCH_POLL_INTERVAL, |state| {
                        !state.shutdown && state.latest_by_route.is_empty()
                    })
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                if state.shutdown {
                    break;
                }
                // Keep one short, absolute coalescing window. Repeated wakeups
                // replace the slot but never extend the deadline, so rapid
                // typing publishes only the newest generation without letting
                // a malicious producer postpone search indefinitely.
                let deadline = Instant::now() + SEARCH_COALESCE_INTERVAL;
                while !state.shutdown {
                    let now = Instant::now();
                    if now >= deadline {
                        break;
                    }
                    let (next, _) = condition
                        .wait_timeout(state, deadline.saturating_duration_since(now))
                        .unwrap_or_else(|poisoned| poisoned.into_inner());
                    state = next;
                }
                if state.shutdown {
                    break;
                }
                std::mem::take(&mut state.latest_by_route)
            };
            if request.is_empty() {
                continue;
            }
            for (_, request) in request {
                let hits = index
                    .search(&request.query, &request.context)
                    .unwrap_or_default();
                if lock(&latest_requested).get(&request.route_id).copied()
                    != Some(request.request_id)
                {
                    continue;
                }
                let result = QuickActionSearchResult {
                    request_id: request.request_id,
                    route_id: request.route_id,
                    query: request.query,
                    hits,
                    status: status_from_service(monitor.service().status()),
                };
                lock(&results).insert(request.route_id, result);
                request.wake.wake();
            }
        })
        .ok()
}

fn index_for_service(
    service: &QuickActionService,
) -> Result<ActionIndex, QuickActionRuntimeErrorCode> {
    let snapshot = service.snapshot();
    let mut shell_user = Vec::new();
    let mut global_user = Vec::new();
    for action in &snapshot.actions().document().actions {
        match action.scope {
            ActionScope::ShellUser => shell_user.push(action.clone()),
            ActionScope::GlobalUser => global_user.push(action.clone()),
            _ => return Err(QuickActionRuntimeErrorCode::Index),
        }
    }
    ActionIndex::build(vec![
        ActionLayer {
            identity: LayerIdentity::ShellUser,
            revision: snapshot.revision(),
            actions: shell_user,
        },
        ActionLayer {
            identity: LayerIdentity::GlobalUser,
            revision: snapshot.revision(),
            actions: global_user,
        },
    ])
    .map_err(|_: IndexError| QuickActionRuntimeErrorCode::Index)
}

fn status_from_service(status: ServiceStatus) -> QuickActionRuntimeStatus {
    match status {
        ServiceStatus::Empty => QuickActionRuntimeStatus::Ready { revision: 0 },
        ServiceStatus::Fresh { revision } => QuickActionRuntimeStatus::Ready { revision },
        ServiceStatus::Recovered { revision, .. } => {
            QuickActionRuntimeStatus::Recovered { revision }
        }
        ServiceStatus::Stale { revision, error } => {
            QuickActionRuntimeStatus::Stale { revision, error }
        }
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;
    use automexia_devops::actions::{
        ActionProvenance, ActionScope, ActionTemplate, ExecutionMode, QuickAction,
        RiskClass, ShellKind, WorkingDirectoryPolicy,
    };
    use std::sync::mpsc;

    fn action(index: usize) -> QuickAction {
        QuickAction {
            id: format!("search.action-{index}"),
            display_name: format!("Search action {index}"),
            description: String::new(),
            tags: vec!["test".into()],
            scope: ActionScope::GlobalUser,
            shells: vec![ShellKind::Bash],
            template: ActionTemplate::TypedArgv {
                executable_id: "git".into(),
                arguments: vec![],
            },
            placeholders: vec![],
            working_directory_policy: WorkingDirectoryPolicy::Inherit,
            risk: RiskClass::ReadOnly,
            execution: ExecutionMode::Insert,
            provenance: ActionProvenance::User,
            enabled: true,
            alias_projection: None,
        }
    }

    fn context() -> SearchContext {
        SearchContext {
            session_id: 8,
            capsule_revision: 1,
            workspace_identity: None,
            workspace_trusted: false,
            shell: ShellKind::Bash,
        }
    }

    #[test]
    fn latest_query_wins_and_wakes_only_its_route() {
        let root = tempfile::tempdir().unwrap();
        let runtime = QuickActionRuntime::open(root.path().join("actions")).unwrap();
        runtime
            .service()
            .unwrap()
            .replace(
                0,
                automexia_devops::actions::QuickActionDocument {
                    schema_version: 1,
                    revision: 1,
                    actions: (0..64).map(action).collect(),
                },
            )
            .unwrap();
        let (sender, receiver) = mpsc::channel();
        for index in 0..500 {
            let sender = sender.clone();
            runtime.submit(
                8,
                format!("action {index}"),
                context(),
                Box::new(move || {
                    let _ = sender.send(index);
                }),
            );
        }
        let woke = receiver.recv_timeout(Duration::from_secs(3)).unwrap();
        assert_eq!(woke, 499);
        let result = runtime.take_result(8, 1).unwrap();
        assert_eq!(result.query, "action 499");
    }

    #[test]
    fn repeated_open_search_drop_cycles_join_without_hanging() {
        for cycle in 0..24 {
            let root = tempfile::tempdir().unwrap();
            let runtime = QuickActionRuntime::open(root.path().join("actions")).unwrap();
            let (sender, receiver) = mpsc::channel();
            runtime.submit(
                cycle + 1,
                String::new(),
                context(),
                Box::new(move || {
                    let _ = sender.send(());
                }),
            );
            receiver.recv_timeout(Duration::from_secs(2)).unwrap();
            drop(runtime);
        }
    }

    #[test]
    fn forgetting_a_closed_route_discards_its_published_result() {
        let root = tempfile::tempdir().unwrap();
        let runtime = QuickActionRuntime::open(root.path().join("actions")).unwrap();
        let (sender, receiver) = mpsc::channel();
        runtime.submit(
            55,
            String::new(),
            context(),
            Box::new(move || {
                let _ = sender.send(());
            }),
        );
        receiver.recv_timeout(Duration::from_secs(2)).unwrap();
        runtime.forget_route(55);
        assert!(runtime.take_result(55, 0).is_none());
    }

    #[test]
    fn simultaneous_routes_are_coalesced_independently_without_starvation() {
        let root = tempfile::tempdir().unwrap();
        let runtime = QuickActionRuntime::open(root.path().join("actions")).unwrap();
        let (sender, receiver) = mpsc::channel();
        for route_id in [11, 22] {
            let sender = sender.clone();
            assert!(matches!(
                runtime.submit(
                    route_id,
                    String::new(),
                    context(),
                    Box::new(move || {
                        let _ = sender.send(route_id);
                    }),
                ),
                SearchSubmission::Queued { .. }
            ));
        }
        let mut woke = [
            receiver.recv_timeout(Duration::from_secs(2)).unwrap(),
            receiver.recv_timeout(Duration::from_secs(2)).unwrap(),
        ];
        woke.sort_unstable();
        assert_eq!(woke, [11, 22]);
        assert!(runtime.take_result(11, 0).is_some());
        assert!(runtime.take_result(22, 0).is_some());
    }

    #[test]
    fn route_capacity_and_query_limits_fail_closed_and_recover_after_close() {
        let root = tempfile::tempdir().unwrap();
        let runtime = QuickActionRuntime::open(root.path().join("actions")).unwrap();
        for route_id in 1..=MAX_RESULT_ROUTES {
            assert!(matches!(
                runtime.submit(route_id, String::new(), context(), Box::new(|| {})),
                SearchSubmission::Queued { .. }
            ));
        }
        assert_eq!(
            runtime.submit(
                MAX_RESULT_ROUTES + 1,
                String::new(),
                context(),
                Box::new(|| {}),
            ),
            SearchSubmission::Disabled {
                error: QuickActionRuntimeErrorCode::RouteCapacity
            }
        );
        runtime.forget_route(1);
        assert!(matches!(
            runtime.submit(
                MAX_RESULT_ROUTES + 1,
                String::new(),
                context(),
                Box::new(|| {}),
            ),
            SearchSubmission::Queued { .. }
        ));
        assert_eq!(
            runtime.submit(2, "bad\u{202e}query".into(), context(), Box::new(|| {})),
            SearchSubmission::Disabled {
                error: QuickActionRuntimeErrorCode::InvalidQuery
            }
        );
    }
}
