//! Lifecycle-owned CP2.2 service activation and latest-state search worker.

use std::{
    collections::BTreeMap,
    fmt,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Condvar, Mutex, MutexGuard,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use automexia_devops::actions::{
    validate_search_query, ActionIndex, ActionLayer, ActionProvenance, ActionScope,
    ActionSearchHit, IndexError, LayerIdentity, QuickAction, SearchContext,
};
use automexia_extension_runtime::CompletionWake;

use super::{
    QuickActionMonitor, QuickActionService, QuickActionStore, ServiceStatus, StoreError,
    StoreErrorCode, WorkspaceActionStore, WorkspaceTrustStore,
    WORKSPACE_ACTION_DIRECTORY_NAME, WORKSPACE_ACTION_FILE_NAME,
};

const SEARCH_POLL_INTERVAL: Duration = Duration::from_millis(50);
const SEARCH_COALESCE_INTERVAL: Duration = Duration::from_millis(12);
const WATCH_DEBOUNCE: Duration = Duration::from_millis(75);
const WATCH_RECONCILE: Duration = Duration::from_secs(5);
const WORKSPACE_RECONCILE: Duration = Duration::from_millis(250);
#[cfg(test)]
const WORKSPACE_AUTHORIZATION_TTL: Duration = Duration::from_millis(100);
#[cfg(not(test))]
const WORKSPACE_AUTHORIZATION_TTL: Duration = Duration::from_secs(30);
const MAX_WORKSPACE_CACHE_ENTRIES: usize = 32;
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
    workspace_path: Option<PathBuf>,
    wake: CompletionWake,
}

#[derive(Clone, Debug)]
struct RouteWorkspaceAuthorization {
    workspace_path: PathBuf,
    workspace_identity: String,
    checked_at: Instant,
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
    workspace_authorizations: Arc<Mutex<BTreeMap<usize, RouteWorkspaceAuthorization>>>,
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
        let workspace_trust_root = store.root().join("workspace-trust");
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
        let workspace_authorizations = Arc::new(Mutex::new(BTreeMap::new()));
        let handle = spawn_worker(
            monitor,
            initial,
            Arc::clone(&pending),
            Arc::clone(&latest_requested),
            Arc::clone(&results),
            workspace_trust_root,
            Arc::clone(&workspace_authorizations),
        )
        .ok_or(QuickActionRuntimeErrorCode::WorkerUnavailable)?;
        Ok(Self(Arc::new(RuntimeInner {
            service: Some(service),
            disabled: None,
            pending,
            latest_requested,
            results,
            workspace_authorizations,
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
            workspace_authorizations: Arc::new(Mutex::new(BTreeMap::new())),
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
        self.submit_for_workspace(route_id, query, context, None, wake)
    }

    pub fn submit_for_workspace(
        &self,
        route_id: usize,
        query: String,
        context: SearchContext,
        workspace_path: Option<PathBuf>,
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
                workspace_path,
                wake,
            },
        );
        condition.notify_one();
        SearchSubmission::Queued { request_id }
    }

    pub fn workspace_action_is_authorized(
        &self,
        route_id: usize,
        workspace_path: Option<&Path>,
        action: &QuickAction,
    ) -> bool {
        let ActionProvenance::WorkspaceTask {
            workspace_identity, ..
        } = &action.provenance
        else {
            return true;
        };
        let Some(workspace_path) = workspace_path else {
            return false;
        };
        let authorizations = lock(&self.0.workspace_authorizations);
        let Some(authorization) = authorizations.get(&route_id) else {
            return false;
        };
        authorization.checked_at.elapsed() <= WORKSPACE_AUTHORIZATION_TTL
            && authorization.workspace_path == workspace_path
            && authorization.workspace_identity == *workspace_identity
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
        lock(&self.0.workspace_authorizations).remove(&route_id);
    }
}

#[derive(Clone)]
struct CachedWorkspaceIndex {
    checked_at: Instant,
    user_revision: u64,
    index: ActionIndex,
    authorization: Option<RouteWorkspaceAuthorization>,
}

struct WorkspaceIndexCache {
    trust_root: PathBuf,
    entries: BTreeMap<PathBuf, CachedWorkspaceIndex>,
}

impl WorkspaceIndexCache {
    fn new(trust_root: PathBuf) -> Self {
        Self {
            trust_root,
            entries: BTreeMap::new(),
        }
    }

    fn index_for(
        &mut self,
        workspace_path: Option<&Path>,
        service: &QuickActionService,
        base: &ActionIndex,
        now: Instant,
    ) -> (ActionIndex, Option<RouteWorkspaceAuthorization>) {
        let Some(workspace_path) = workspace_path.filter(|path| path.is_absolute())
        else {
            return (base.clone(), None);
        };
        let key = workspace_path.to_path_buf();
        let user_revision = service.snapshot().revision();
        if let Some(cached) = self.entries.get(&key) {
            if cached.user_revision == user_revision
                && now.saturating_duration_since(cached.checked_at) < WORKSPACE_RECONCILE
            {
                return (cached.index.clone(), cached.authorization.clone());
            }
        }

        let base = index_for_service(service).unwrap_or_else(|_| base.clone());
        let (index, authorization) =
            match resolve_trusted_workspace(workspace_path, &self.trust_root) {
                Some((layer, workspace_identity)) => {
                    let index = index_for_service_with_workspace(service, Some(layer))
                        .unwrap_or_else(|_| base.clone());
                    let authorization = RouteWorkspaceAuthorization {
                        workspace_path: key.clone(),
                        workspace_identity,
                        checked_at: now,
                    };
                    (index, Some(authorization))
                }
                None => (base, None),
            };
        if !self.entries.contains_key(&key)
            && self.entries.len() >= MAX_WORKSPACE_CACHE_ENTRIES
        {
            if let Some(oldest) = self
                .entries
                .iter()
                .min_by_key(|(_, entry)| entry.checked_at)
                .map(|(path, _)| path.clone())
            {
                self.entries.remove(&oldest);
            }
        }
        self.entries.insert(
            key,
            CachedWorkspaceIndex {
                checked_at: now,
                user_revision,
                index: index.clone(),
                authorization: authorization.clone(),
            },
        );
        (index, authorization)
    }
}

fn resolve_trusted_workspace(
    workspace_path: &Path,
    trust_root: &Path,
) -> Option<(ActionLayer, String)> {
    for workspace_root in workspace_path.ancestors().take(64) {
        let source = workspace_root
            .join(WORKSPACE_ACTION_DIRECTORY_NAME)
            .join(WORKSPACE_ACTION_FILE_NAME);
        match std::fs::symlink_metadata(&source) {
            Ok(metadata) if !metadata.file_type().is_symlink() && metadata.is_file() => {}
            Ok(_) => return None,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(_) => return None,
        }
        let workspace = WorkspaceActionStore::open(workspace_root)
            .ok()?
            .load()
            .ok()?;
        let trust = WorkspaceTrustStore::open_existing_read_only(trust_root).ok()??;
        let layer = trust.trusted_layer(&workspace).ok()?;
        let LayerIdentity::TrustedWorkspace { identity } = &layer.identity else {
            return None;
        };
        return Some((layer.clone(), identity.clone()));
    }
    None
}

fn spawn_worker(
    mut monitor: QuickActionMonitor,
    mut index: ActionIndex,
    pending: Arc<(Mutex<PendingState>, Condvar)>,
    latest_requested: Arc<Mutex<BTreeMap<usize, u64>>>,
    results: Arc<Mutex<BTreeMap<usize, QuickActionSearchResult>>>,
    workspace_trust_root: PathBuf,
    workspace_authorizations: Arc<Mutex<BTreeMap<usize, RouteWorkspaceAuthorization>>>,
) -> Option<JoinHandle<()>> {
    let mut workspace_cache = WorkspaceIndexCache::new(workspace_trust_root);
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
                let (request_index, authorization) = workspace_cache.index_for(
                    request.workspace_path.as_deref(),
                    monitor.service(),
                    &index,
                    Instant::now(),
                );
                let mut context = request.context;
                context.workspace_identity = authorization
                    .as_ref()
                    .map(|authorization| authorization.workspace_identity.clone());
                context.workspace_trusted = authorization.is_some();
                let hits = request_index
                    .search(&request.query, &context)
                    .unwrap_or_default();
                if lock(&latest_requested).get(&request.route_id).copied()
                    != Some(request.request_id)
                {
                    continue;
                }
                let mut route_authorizations = lock(&workspace_authorizations);
                if let Some(authorization) = authorization {
                    route_authorizations.insert(request.route_id, authorization);
                } else {
                    route_authorizations.remove(&request.route_id);
                }
                drop(route_authorizations);
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
    index_for_service_with_workspace(service, None)
}

fn index_for_service_with_workspace(
    service: &QuickActionService,
    workspace: Option<ActionLayer>,
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
    let mut layers = vec![
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
    ];
    if let Some(workspace) = workspace {
        layers.push(workspace);
    }
    ActionIndex::build(layers).map_err(|_: IndexError| QuickActionRuntimeErrorCode::Index)
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

    #[test]
    fn trusted_workspace_tasks_are_cached_off_thread_and_revocation_fails_closed() {
        let root = tempfile::tempdir().unwrap();
        let actions_root = root.path().join("actions");
        let runtime = QuickActionRuntime::open(&actions_root).unwrap();
        let workspace = root.path().join("project");
        std::fs::create_dir(&workspace).unwrap();
        let workspace_store =
            super::super::WorkspaceActionStore::open(&workspace).unwrap();
        let saved = workspace_store
            .put_task_bridge(
                super::super::WorkspaceTaskBridgeInput {
                    action_id: "workspace.build".into(),
                    display_name: "Build workspace".into(),
                    description: "Explicit task bridge".into(),
                    runner: automexia_devops::actions::TaskRunner::Just,
                    task_name: "build".into(),
                    shells: vec![ShellKind::Bash],
                    risk: RiskClass::Mutating,
                },
                0,
                false,
            )
            .unwrap();
        let trust = super::super::WorkspaceTrustStore::open_or_create(
            actions_root.join("workspace-trust"),
        )
        .unwrap();
        trust.trust(&saved, 0).unwrap();

        let (sender, receiver) = mpsc::channel();
        runtime.submit_for_workspace(
            77,
            String::new(),
            context(),
            Some(workspace.clone()),
            Box::new(move || {
                let _ = sender.send(());
            }),
        );
        receiver.recv_timeout(Duration::from_secs(2)).unwrap();
        let result = runtime.take_result(77, 0).unwrap();
        let action = result
            .hits
            .iter()
            .find(|hit| hit.action.id == "workspace.build")
            .unwrap()
            .action
            .clone();
        assert!(runtime.workspace_action_is_authorized(77, Some(&workspace), &action));

        trust
            .revoke(workspace_store.workspace_identity(), 1)
            .unwrap();
        std::thread::sleep(WORKSPACE_RECONCILE + Duration::from_millis(25));
        assert!(!runtime.workspace_action_is_authorized(77, Some(&workspace), &action));
        let (sender, receiver) = mpsc::channel();
        runtime.submit_for_workspace(
            77,
            String::new(),
            context(),
            Some(workspace),
            Box::new(move || {
                let _ = sender.send(());
            }),
        );
        receiver.recv_timeout(Duration::from_secs(2)).unwrap();
        assert!(runtime
            .take_result(77, 0)
            .unwrap()
            .hits
            .iter()
            .all(|hit| hit.action.id != "workspace.build"));
    }
}
