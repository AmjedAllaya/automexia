use std::collections::hash_map::DefaultHasher;
use std::collections::{BTreeMap, BTreeSet};
use std::hash::{Hash, Hasher};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{OnceLock, RwLock, RwLockReadGuard, RwLockWriteGuard};
#[cfg(not(target_arch = "wasm32"))]
use std::time::{Duration, Instant};
use std::time::{SystemTime, UNIX_EPOCH};

use automexia_extension_api::{
    BoundedText, ContextContribution, Freshness, OperationId, SemanticSeverity,
    SessionFacts, SessionId,
};
pub use automexia_extension_runtime::RefreshSubmission;
use automexia_extension_runtime::{
    BoundedCache, BoundedWorker, CacheKey, CancellationToken, CompletionWake, Generation,
};

use super::builtins::devops::{self, DevOpsSnapshot};
use super::marketplace::{self, MarketItem};
use super::state;

static ACTIVATION_GENERATION: Generation = Generation::new(1);
static DEVOPS_GENERATION: Generation = Generation::new(1);
static DEVOPS_COMPLETION_COUNTER: Generation = Generation::new(1);
static OPERATION_COUNTER: AtomicU32 = AtomicU32::new(1);
const DEVOPS_CONTEXT_CACHE_LIMIT: usize = 32;
const MAX_SESSION_TITLE_BYTES: usize = 4 * 1024;
#[cfg(not(target_arch = "wasm32"))]
const DEVOPS_REUSE_MAX_AGE: Duration = Duration::from_secs(5);

#[derive(Clone, Debug)]
struct DevOpsCacheEntry {
    revision: u32,
    session: SessionFacts,
    snapshot: DevOpsSnapshot,
    freshness: Freshness,
    observed_at_ms: u64,
    #[cfg(not(target_arch = "wasm32"))]
    completed_at: Instant,
}

#[derive(Clone, Debug)]
struct CapsuleState {
    session: SessionFacts,
    revision: u64,
}

#[derive(Debug)]
struct RuntimeState {
    installed: BTreeSet<String>,
    devops_snapshots: BoundedCache<CacheKey, DevOpsCacheEntry>,
    capsules: BTreeMap<usize, CapsuleState>,
    pending: BTreeMap<usize, (OperationId, CancellationToken)>,
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
            devops_snapshots: BoundedCache::new(DEVOPS_CONTEXT_CACHE_LIMIT),
            capsules: BTreeMap::new(),
            pending: BTreeMap::new(),
        }
    }

    fn devops_snapshot(&self, session_id: usize) -> Option<(CacheKey, DevOpsCacheEntry)> {
        self.devops_snapshots.find_map_rev(|key, entry| {
            (key.session_id == SessionId::new(session_id as u64))
                .then(|| (key.clone(), entry.clone()))
        })
    }

    fn put_devops_snapshot(
        &mut self,
        key: CacheKey,
        revision: u32,
        session: SessionFacts,
        snapshot: DevOpsSnapshot,
        freshness: Freshness,
        observed_at_ms: u64,
    ) {
        self.devops_snapshots.insert(
            key,
            DevOpsCacheEntry {
                revision,
                session,
                snapshot,
                freshness,
                observed_at_ms,
                #[cfg(not(target_arch = "wasm32"))]
                completed_at: Instant::now(),
            },
        );
    }

    fn capsule_revision(&mut self, session: &SessionFacts) -> u64 {
        match self.capsules.get_mut(&session.session_id) {
            Some(capsule) if capsule.session == *session => capsule.revision,
            Some(capsule) => {
                capsule.revision = capsule.revision.wrapping_add(1).max(1);
                capsule.session = session.clone();
                let revision = capsule.revision;
                if let Some((_, token)) = self.pending.remove(&session.session_id) {
                    token.cancel();
                }
                let session_id = SessionId::new(session.session_id as u64);
                self.devops_snapshots
                    .retain(|key, _| key.session_id != session_id);
                revision
            }
            None => {
                self.capsules.insert(
                    session.session_id,
                    CapsuleState {
                        session: session.clone(),
                        revision: 1,
                    },
                );
                1
            }
        }
    }

    fn accepts(
        &self,
        session_id: usize,
        operation_id: OperationId,
        capsule_revision: u64,
    ) -> bool {
        self.capsules
            .get(&session_id)
            .is_some_and(|capsule| capsule.revision == capsule_revision)
            && self
                .pending
                .get(&session_id)
                .is_some_and(|(current, _)| *current == operation_id)
    }

    fn finish_operation(&mut self, session_id: usize, operation_id: OperationId) {
        if self
            .pending
            .get(&session_id)
            .is_some_and(|(current, _)| *current == operation_id)
        {
            self.pending.remove(&session_id);
        }
    }

    fn register_operation(
        &mut self,
        session_id: usize,
        operation_id: OperationId,
        token: CancellationToken,
    ) {
        if let Some((_, previous)) =
            self.pending.insert(session_id, (operation_id, token))
        {
            previous.cancel();
        }
    }

    fn cancel_all(&mut self) {
        for (_, token) in self.pending.values() {
            token.cancel();
        }
        self.pending.clear();
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn reusable_devops_snapshot(
        &self,
        session: &SessionFacts,
    ) -> Option<(DevOpsSnapshot, u64)> {
        if !session.shell_integration {
            return None;
        }
        self.devops_snapshots.find_map_rev(|_, entry| {
            (entry.session.session_id != session.session_id
                && entry.completed_at.elapsed() <= DEVOPS_REUSE_MAX_AGE
                && equivalent_shell_context(&entry.session, session))
            .then(|| (entry.snapshot.clone(), entry.observed_at_ms))
        })
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
        && left.shell_user == right.shell_user
        && left.shell_path == right.shell_path
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

fn unix_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| u64::try_from(duration.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or(0)
}

fn source_revision(session: &SessionFacts) -> u64 {
    let mut hasher = DefaultHasher::new();
    session.session_id.hash(&mut hasher);
    session.cwd.hash(&mut hasher);
    session.title.hash(&mut hasher);
    session.distro.hash(&mut hasher);
    session.os_version.hash(&mut hasher);
    session.shell_name.hash(&mut hasher);
    session.shell_user.hash(&mut hasher);
    session.shell_path.hash(&mut hasher);
    session.shell_integration.hash(&mut hasher);
    session.shell_pid.hash(&mut hasher);
    hasher.finish()
}

fn cache_key(
    session: &SessionFacts,
    capsule_revision: u64,
    source_revision: u64,
) -> CacheKey {
    CacheKey {
        extension_id: automexia_extension_api::ExtensionId::new(devops::ID)
            .expect("built-in extension id is valid"),
        session_id: SessionId::new(session.session_id as u64),
        capsule_revision,
        provider_identity: None,
        source_revision,
        request_kind: BoundedText::new("context-refresh")
            .expect("built-in request kind is valid"),
    }
}

pub fn generation() -> u32 {
    ACTIVATION_GENERATION.current()
}

pub fn context_status_enabled() -> bool {
    is_installed(devops::ID)
}

pub fn is_installed(id: &str) -> bool {
    let installed = read_runtime().installed.contains(id);
    if installed && id == devops::ID {
        ensure_background_services();
    }
    installed
}

pub fn toggle(id: &str) -> Result<bool, String> {
    let manifest =
        marketplace::descriptor(id).ok_or_else(|| format!("unknown extension: {id}"))?;
    let next = !read_runtime().installed.contains(id);

    state::set_installed(manifest, next)?;

    let mut runtime = write_runtime();
    let cleared_devops = if next {
        runtime.installed.insert(id.to_owned());
        false
    } else {
        runtime.installed.remove(id);
        if id == devops::ID {
            runtime.cancel_all();
            runtime.devops_snapshots.clear();
            runtime.capsules.clear();
            true
        } else {
            false
        }
    };
    drop(runtime);
    if cleared_devops {
        DEVOPS_GENERATION.advance();
    }
    if next && id == devops::ID {
        ensure_background_services();
    }
    ACTIVATION_GENERATION.advance();
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
    DEVOPS_GENERATION.current()
}

pub fn context_contribution(
    session_id: usize,
) -> (u32, Option<SessionFacts>, ContextContribution) {
    let Some((key, entry)) = read_runtime().devops_snapshot(session_id) else {
        return (0, None, devops::empty_contribution(session_id));
    };
    let contribution = devops::contribution(
        &entry.snapshot,
        &entry.session,
        key.capsule_revision,
        key.source_revision,
        entry.observed_at_ms,
        entry.freshness,
    )
    .unwrap_or_else(|error| {
        tracing::warn!(
            session_id,
            error = %error,
            "discarding invalid local context contribution"
        );
        let mut empty = devops::empty_contribution(session_id);
        empty.freshness = Freshness::Error;
        empty
    });
    (entry.revision, Some(entry.session), contribution)
}

pub type DevOpsRefreshCompletion = CompletionWake;

struct RefreshRequest {
    operation_id: OperationId,
    session: SessionFacts,
    capsule_revision: u64,
    source_revision: u64,
    cancellation: CancellationToken,
    completion: Option<DevOpsRefreshCompletion>,
}

fn process_refresh(mut request: RefreshRequest) {
    if request.cancellation.is_cancelled()
        || !read_runtime().installed.contains(devops::ID)
    {
        return;
    }

    let result = catch_unwind(AssertUnwindSafe(|| {
        #[cfg(feature = "visual-test-hooks")]
        if let Some(snapshot) = super::visual_test_hooks::visual_test_snapshot() {
            return snapshot;
        }
        devops::detect(&request.session)
    }));
    match result {
        Ok(snapshot) => {
            tracing::debug!(
                session_id = request.session.session_id,
                capsule_revision = request.capsule_revision,
                "Automexia local context discovery completed"
            );
            if request.cancellation.is_cancelled()
                || !read_runtime().installed.contains(devops::ID)
            {
                return;
            }
            publish_devops_snapshot(request, snapshot);
        }
        Err(_) => {
            tracing::error!(
                session_id = request.session.session_id,
                "Automexia local context provider panicked; preserving last truthful snapshot"
            );
            publish_devops_failure(&mut request);
        }
    }
}

fn worker() -> &'static BoundedWorker<RefreshRequest> {
    static WORKER: OnceLock<BoundedWorker<RefreshRequest>> = OnceLock::new();
    WORKER.get_or_init(|| {
        BoundedWorker::new("automexia-extension-worker", 1, process_refresh)
    })
}

fn publish_devops_snapshot(request: RefreshRequest, snapshot: DevOpsSnapshot) {
    let session_id = request.session.session_id;
    let revision = DEVOPS_COMPLETION_COUNTER.advance();
    let key = cache_key(
        &request.session,
        request.capsule_revision,
        request.source_revision,
    );
    {
        let mut runtime = write_runtime();
        if !runtime.accepts(session_id, request.operation_id, request.capsule_revision) {
            return;
        }
        runtime.put_devops_snapshot(
            key,
            revision,
            request.session,
            snapshot,
            Freshness::Current,
            unix_time_ms(),
        );
        runtime.finish_operation(session_id, request.operation_id);
    }
    DEVOPS_GENERATION.advance();
    if let Some(completion) = request.completion {
        completion.wake();
    }
}

fn publish_devops_failure(request: &mut RefreshRequest) {
    let session_id = request.session.session_id;
    let revision = DEVOPS_COMPLETION_COUNTER.advance();
    {
        let mut runtime = write_runtime();
        if !runtime.accepts(session_id, request.operation_id, request.capsule_revision) {
            return;
        }
        let (snapshot, observed_at_ms) = runtime
            .devops_snapshot(session_id)
            .map(|(_, entry)| (entry.snapshot, entry.observed_at_ms))
            .unwrap_or_else(|| (DevOpsSnapshot::default(), unix_time_ms()));
        let key = cache_key(
            &request.session,
            request.capsule_revision,
            request.source_revision,
        );
        runtime.put_devops_snapshot(
            key,
            revision,
            request.session.clone(),
            snapshot,
            Freshness::Error,
            observed_at_ms,
        );
        runtime.finish_operation(session_id, request.operation_id);
    }
    DEVOPS_GENERATION.advance();
    if let Some(completion) = request.completion.take() {
        completion.wake();
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn seed_devops_snapshot(
    session: &SessionFacts,
    capsule_revision: u64,
    source_revision: u64,
) -> bool {
    {
        let mut runtime = write_runtime();
        if runtime.devops_snapshot(session.session_id).is_some() {
            return false;
        }
        let Some((snapshot, observed_at_ms)) = runtime.reusable_devops_snapshot(session)
        else {
            return false;
        };
        let revision = DEVOPS_COMPLETION_COUNTER.advance();
        let key = cache_key(session, capsule_revision, source_revision);
        runtime.put_devops_snapshot(
            key,
            revision,
            session.clone(),
            snapshot,
            Freshness::Refreshing,
            observed_at_ms,
        );
    }
    DEVOPS_GENERATION.advance();
    true
}

#[cfg(target_arch = "wasm32")]
fn seed_devops_snapshot(
    _session: &SessionFacts,
    _capsule_revision: u64,
    _source_revision: u64,
) -> bool {
    false
}

pub fn ensure_background_services() {
    let _ = worker().ensure_started();
}

pub fn shutdown_background_services() {
    {
        let mut runtime = write_runtime();
        runtime.cancel_all();
    }
    worker().shutdown();
}

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

    let source_revision = source_revision(session);
    let capsule_revision = {
        let mut runtime = write_runtime();
        runtime.capsule_revision(session)
    };
    let _ = seed_devops_snapshot(session, capsule_revision, source_revision);

    let operation_id = OperationId::new(u64::from(
        OPERATION_COUNTER
            .fetch_add(1, Ordering::AcqRel)
            .wrapping_add(1),
    ));
    let cancellation = CancellationToken::default();
    let request = RefreshRequest {
        operation_id,
        session: session.clone(),
        capsule_revision,
        source_revision,
        cancellation: cancellation.clone(),
        completion,
    };
    let submission = worker().try_submit_then(request, || {
        write_runtime().register_operation(
            session.session_id,
            operation_id,
            cancellation.clone(),
        );
    });
    if submission != RefreshSubmission::Queued {
        cancellation.cancel();
    }
    submission
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state() -> RuntimeState {
        RuntimeState {
            installed: BTreeSet::new(),
            devops_snapshots: BoundedCache::new(DEVOPS_CONTEXT_CACHE_LIMIT),
            capsules: BTreeMap::new(),
            pending: BTreeMap::new(),
        }
    }

    fn session(id: usize, title: &str) -> SessionFacts {
        SessionFacts {
            session_id: id,
            cwd: None,
            title: title.to_owned(),
            distro: None,
            os_version: None,
            shell_name: None,
            shell_user: None,
            shell_path: None,
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

    fn put(
        state: &mut RuntimeState,
        facts: SessionFacts,
        revision: u32,
        value: DevOpsSnapshot,
    ) {
        let capsule = state.capsule_revision(&facts);
        let source = source_revision(&facts);
        state.put_devops_snapshot(
            cache_key(&facts, capsule, source),
            revision,
            facts,
            value,
            Freshness::Current,
            unix_time_ms(),
        );
    }

    #[test]
    fn devops_cache_is_scoped_by_terminal_session_and_complete_key() {
        let mut state = state();
        put(&mut state, session(101, "one"), 10, snapshot("a"));
        put(&mut state, session(202, "two"), 20, snapshot("b"));

        let (_, one) = state.devops_snapshot(101).unwrap();
        assert_eq!(one.revision, 10);
        assert_eq!(one.session.title, "one");
        assert_eq!(one.snapshot.environment.as_deref(), Some("a"));
        let (two_key, two) = state.devops_snapshot(202).unwrap();
        assert_eq!(two.revision, 20);
        assert_eq!(two.snapshot.environment.as_deref(), Some("b"));
        assert_eq!(two_key.request_kind.as_str(), "context-refresh");
        assert!(state.devops_snapshot(999).is_none());
    }

    #[test]
    fn cache_keeps_only_the_latest_capsule_for_a_changed_session() {
        let mut state = state();
        put(&mut state, session(7, "powershell"), 1, snapshot("host"));
        put(
            &mut state,
            session(7, "user@host:/mnt/d/work"),
            2,
            snapshot("wsl"),
        );
        let (key, entry) = state.devops_snapshot(7).unwrap();
        assert_eq!(entry.revision, 2);
        assert_eq!(entry.session.title, "user@host:/mnt/d/work");
        assert_eq!(entry.snapshot.environment.as_deref(), Some("wsl"));
        assert_eq!(key.capsule_revision, 2);
    }

    #[test]
    fn devops_cache_is_bounded() {
        let mut state = state();
        for index in 0..(DEVOPS_CONTEXT_CACHE_LIMIT + 5) {
            put(
                &mut state,
                session(index, &format!("session-{index}")),
                index as u32 + 1,
                snapshot(&index.to_string()),
            );
        }
        assert_eq!(state.devops_snapshots.len(), DEVOPS_CONTEXT_CACHE_LIMIT);
        assert!(state.devops_snapshot(0).is_none());
        assert_eq!(
            state
                .devops_snapshot(DEVOPS_CONTEXT_CACHE_LIMIT + 4)
                .unwrap()
                .1
                .snapshot
                .environment,
            Some((DEVOPS_CONTEXT_CACHE_LIMIT + 4).to_string())
        );
    }

    #[test]
    fn stale_operations_are_rejected_after_capsule_rebind() {
        let mut state = state();
        let original = session(50, "PowerShell");
        let old_capsule = state.capsule_revision(&original);
        let operation = OperationId::new(1);
        let cancellation = CancellationToken::default();
        state.register_operation(50, operation, cancellation.clone());

        let changed = session(50, "amjed@host:/work");
        let new_capsule = state.capsule_revision(&changed);
        assert_ne!(old_capsule, new_capsule);
        assert!(cancellation.is_cancelled());
        assert!(!state.accepts(50, operation, old_capsule));
        assert!(!state.accepts(50, operation, new_capsule));
        assert!(state.devops_snapshot(50).is_none());
    }

    #[test]
    fn replacing_an_operation_cancels_the_obsolete_request() {
        let mut state = state();
        let old = CancellationToken::default();
        state.register_operation(8, OperationId::new(1), old.clone());
        state.register_operation(8, OperationId::new(2), CancellationToken::default());
        assert!(old.is_cancelled());
        assert!(!state.accepts(8, OperationId::new(1), 0));
    }

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
    fn equivalent_new_session_reuses_a_fresh_snapshot() {
        let mut state = state();
        let mut source = session(301, "amjed@host:/mnt/d/work");
        source.cwd = Some("D:\\work".into());
        source.distro = Some("Ubuntu".to_string());
        source.shell_name = Some("bash".to_string());
        let mut target = source.clone();
        target.session_id = 302;
        put(&mut state, source, 44, snapshot("docker"));

        assert_eq!(
            state
                .reusable_devops_snapshot(&target)
                .and_then(|(value, _)| value.environment),
            Some("docker".to_string())
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn snapshot_reuse_rejects_other_paths_unintegrated_and_expired() {
        let mut state = state();
        let mut source = session(401, "PowerShell - D:/work");
        source.cwd = Some("D:\\work".into());
        source.shell_name = Some("PowerShell".to_string());
        put(&mut state, source.clone(), 55, snapshot("staging"));

        let mut other_path = source.clone();
        other_path.session_id = 402;
        other_path.cwd = Some("D:\\production".into());
        other_path.title = "PowerShell - D:/production".to_string();
        assert!(state.reusable_devops_snapshot(&other_path).is_none());

        let mut unintegrated = source;
        unintegrated.session_id = 403;
        unintegrated.shell_integration = false;
        assert!(state.reusable_devops_snapshot(&unintegrated).is_none());

        let (_, entry) = state.devops_snapshot(401).unwrap();
        let key = cache_key(&entry.session, 1, source_revision(&entry.session));
        state.devops_snapshots.get_mut(&key).unwrap().completed_at =
            Instant::now() - DEVOPS_REUSE_MAX_AGE - Duration::from_millis(1);
        let mut expired = entry.session;
        expired.session_id = 404;
        assert!(state.reusable_devops_snapshot(&expired).is_none());
    }
}
