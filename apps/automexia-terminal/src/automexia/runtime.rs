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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContextStatusError {
    RevisionExhausted,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InventoryStatus {
    Uninitialized,
    Loading,
    Ready,
    Unavailable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InventoryInitialization {
    Queued,
    Ready,
    Pending,
    Unavailable,
}

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
    context_status_preference: bool,
    context_revision: u64,
    inventory_status: InventoryStatus,
    inventory_revision: u64,
    inventory_cancellation: Option<CancellationToken>,
    stopping: bool,
    devops_snapshots: BoundedCache<CacheKey, DevOpsCacheEntry>,
    capsules: BTreeMap<usize, CapsuleState>,
    pending: BTreeMap<usize, (OperationId, CancellationToken)>,
}

impl RuntimeState {
    fn new() -> Self {
        Self {
            installed: BTreeSet::new(),
            context_status_preference: true,
            context_revision: 1,
            inventory_status: InventoryStatus::Uninitialized,
            inventory_revision: 0,
            inventory_cancellation: None,
            stopping: false,
            devops_snapshots: BoundedCache::new(DEVOPS_CONTEXT_CACHE_LIMIT),
            capsules: BTreeMap::new(),
            pending: BTreeMap::new(),
        }
    }

    fn set_context_status_enabled(
        &mut self,
        enabled: bool,
    ) -> Result<bool, ContextStatusError> {
        if self.context_status_preference == enabled {
            return Ok(false);
        }
        let Some(revision) = self.context_revision.checked_add(1) else {
            self.context_status_preference = false;
            self.clear_context();
            return Err(ContextStatusError::RevisionExhausted);
        };
        self.context_revision = revision;
        self.context_status_preference = enabled;
        if !enabled {
            self.clear_context();
        }
        Ok(true)
    }

    fn invalidate_devops_session(&mut self, session_id: usize) -> bool {
        // Retain the capsule as a generation tombstone. Removing it would let
        // unchanged facts recreate revision 1 and admit a delayed registration.
        let mut changed = false;
        if let Some(capsule) = self.capsules.get_mut(&session_id) {
            if capsule.revision != 0 {
                capsule.revision = capsule.revision.checked_add(1).unwrap_or(0);
                changed = true;
            }
        }
        if let Some((_, token)) = self.pending.remove(&session_id) {
            token.cancel();
            changed = true;
        }
        let before = self.devops_snapshots.len();
        let id = SessionId::new(session_id as u64);
        self.devops_snapshots.retain(|key, _| key.session_id != id);
        changed || before != self.devops_snapshots.len()
    }

    fn clear_context(&mut self) {
        self.cancel_all();
        self.devops_snapshots.clear();
        self.capsules.clear();
    }

    fn revoke_context(&mut self) -> Result<(), ContextStatusError> {
        self.clear_context();
        let Some(revision) = self.context_revision.checked_add(1) else {
            self.context_status_preference = false;
            return Err(ContextStatusError::RevisionExhausted);
        };
        self.context_revision = revision;
        Ok(())
    }

    fn context_status_enabled(&self) -> bool {
        !self.stopping
            && self.inventory_status == InventoryStatus::Ready
            && self.context_status_preference
            && self.installed.contains(devops::ID)
    }

    fn accepts_refresh(&self, request: &RefreshRequest) -> bool {
        self.context_status_enabled()
            && self.context_revision == request.context_revision
            && !request.cancellation.is_cancelled()
            && self.accepts(
                request.session.session_id,
                request.operation_id,
                request.capsule_revision,
            )
    }

    fn register_refresh(
        &mut self,
        session_id: usize,
        operation_id: OperationId,
        capsule_revision: u64,
        context_revision: u64,
        cancellation: CancellationToken,
    ) -> bool {
        if capsule_revision == 0
            || !self.context_status_enabled()
            || self.context_revision != context_revision
            || cancellation.is_cancelled()
            || self
                .capsules
                .get(&session_id)
                .is_none_or(|capsule| capsule.revision != capsule_revision)
        {
            cancellation.cancel();
            return false;
        }
        self.register_operation(session_id, operation_id, cancellation);
        true
    }

    fn begin_inventory(&mut self, cancellation: CancellationToken) -> Option<u64> {
        if self.stopping
            || matches!(
                self.inventory_status,
                InventoryStatus::Loading | InventoryStatus::Ready
            )
        {
            return None;
        }
        let Some(revision) = self.inventory_revision.checked_add(1) else {
            self.inventory_status = InventoryStatus::Unavailable;
            return None;
        };
        self.inventory_revision = revision;
        self.inventory_cancellation = Some(cancellation);
        self.inventory_status = InventoryStatus::Loading;
        Some(revision)
    }

    fn finish_inventory(
        &mut self,
        revision: u64,
        installed: Option<BTreeSet<String>>,
    ) -> bool {
        if self.stopping
            || self.inventory_status != InventoryStatus::Loading
            || self.inventory_revision != revision
            || self
                .inventory_cancellation
                .as_ref()
                .is_none_or(CancellationToken::is_cancelled)
        {
            return false;
        }
        self.inventory_cancellation = None;
        if let Some(installed) = installed {
            self.installed = installed;
            self.inventory_status = InventoryStatus::Ready;
        } else {
            self.inventory_status = InventoryStatus::Unavailable;
        }
        true
    }

    fn stop(&mut self) {
        self.stopping = true;
        if let Some(cancellation) = self.inventory_cancellation.take() {
            cancellation.cancel();
        }
        self.inventory_status = InventoryStatus::Unavailable;
        self.clear_context();
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
            // Zero is a permanent exhausted revision for this route, not an
            // initial value. Neither equal nor changed facts may revive it.
            Some(capsule) if capsule.revision == 0 => 0,
            Some(capsule) if same_devops_context(&capsule.session, session) => {
                capsule.revision
            }
            Some(capsule) => {
                capsule.revision = capsule.revision.checked_add(1).unwrap_or(0);
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
        capsule_revision != 0
            && self
                .capsules
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
                && entry.freshness == Freshness::Current
                && entry.completed_at.elapsed() <= DEVOPS_REUSE_MAX_AGE
                && equivalent_shell_context(&entry.session, session))
            .then(|| (entry.snapshot.clone(), entry.observed_at_ms))
        })
    }
}

fn equivalent_shell_context(left: &SessionFacts, right: &SessionFacts) -> bool {
    left.shell_integration == right.shell_integration
        && left.cwd == right.cwd
        && left.distro == right.distro
        && left.os_version == right.os_version
        && left.shell_name == right.shell_name
        && left.shell_user == right.shell_user
        && left.shell_path == right.shell_path
        && left.environment == right.environment
}

/// Discovery uses explicit shell facts, never the mutable OSC window title.
/// Route and native process identity still invalidate in-flight publication.
pub fn same_devops_context(left: &SessionFacts, right: &SessionFacts) -> bool {
    left.session_id == right.session_id
        && left.shell_pid == right.shell_pid
        && equivalent_shell_context(left, right)
}

fn runtime() -> &'static RwLock<RuntimeState> {
    static RUNTIME: OnceLock<RwLock<RuntimeState>> = OnceLock::new();
    RUNTIME.get_or_init(|| RwLock::new(RuntimeState::new()))
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
    session.distro.hash(&mut hasher);
    session.os_version.hash(&mut hasher);
    session.shell_name.hash(&mut hasher);
    session.shell_user.hash(&mut hasher);
    session.shell_path.hash(&mut hasher);
    session.environment.hash(&mut hasher);
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

/// Cached membership only: this does not initialize filesystem state or workers.
pub fn output_highlighting_available() -> bool {
    is_installed(devops::ID)
}

pub fn inventory_status() -> InventoryStatus {
    read_runtime().inventory_status
}

/// Queue the single bounded inventory load after publishing desired preferences.
/// Completion is global and runs only after the corresponding state is published.
pub fn schedule_initialize(completion: CompletionWake) -> InventoryInitialization {
    let cancellation = CancellationToken::default();
    let revision = {
        let mut runtime = write_runtime();
        if runtime.stopping {
            return InventoryInitialization::Unavailable;
        }
        match runtime.inventory_status {
            InventoryStatus::Ready => return InventoryInitialization::Ready,
            InventoryStatus::Loading => return InventoryInitialization::Pending,
            _ => {}
        }
        let Some(revision) = runtime.begin_inventory(cancellation.clone()) else {
            return InventoryInitialization::Unavailable;
        };
        revision
    };
    let request = InventoryRequest {
        revision,
        cancellation,
        completion,
    };
    if worker().try_submit(RuntimeWork::Inventory(request)) == RefreshSubmission::Queued {
        InventoryInitialization::Queued
    } else {
        let _ = write_runtime().finish_inventory(revision, None);
        InventoryInitialization::Unavailable
    }
}

/// Publish only the presentation choice; installation membership is unchanged.
pub fn set_context_status_enabled(enabled: bool) -> Result<bool, ContextStatusError> {
    let result = write_runtime().set_context_status_enabled(enabled);
    if !matches!(result, Ok(false)) {
        DEVOPS_GENERATION.advance();
        ACTIVATION_GENERATION.advance();
    }
    result
}

pub fn context_status_enabled() -> bool {
    read_runtime().context_status_enabled()
}

pub fn is_installed(id: &str) -> bool {
    let runtime = read_runtime();
    !runtime.stopping
        && runtime.inventory_status == InventoryStatus::Ready
        && runtime.installed.contains(id)
}

pub fn toggle(id: &str) -> Result<bool, String> {
    let manifest =
        marketplace::descriptor(id).ok_or_else(|| format!("unknown extension: {id}"))?;
    let next = {
        let runtime = read_runtime();
        if runtime.stopping || runtime.inventory_status != InventoryStatus::Ready {
            return Err("extension inventory is not ready".into());
        }
        !runtime.installed.contains(id)
    };

    state::set_installed(manifest, next)?;

    let mut runtime = write_runtime();
    let cleared_devops = if next {
        runtime.installed.insert(id.to_owned());
        false
    } else {
        runtime.installed.remove(id);
        if id == devops::ID {
            if runtime.revoke_context().is_err() {
                tracing::warn!("Local context disabled after revision exhaustion");
            }
            true
        } else {
            false
        }
    };
    drop(runtime);
    if cleared_devops {
        DEVOPS_GENERATION.advance();
    }
    if next && id == devops::ID && context_status_enabled() {
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

/// Revoke one route's cached and in-flight discovery without starting a worker.
/// Called after releasing the terminal snapshot lock. Other routes and
/// completed prompt history remain owned by their existing components.
pub fn invalidate_devops_session(session_id: usize) -> bool {
    let changed = write_runtime().invalidate_devops_session(session_id);
    if changed {
        DEVOPS_GENERATION.advance();
    }
    changed
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
    context_revision: u64,
    operation_id: OperationId,
    session: SessionFacts,
    capsule_revision: u64,
    source_revision: u64,
    cancellation: CancellationToken,
    completion: Option<DevOpsRefreshCompletion>,
}

fn process_refresh(mut request: RefreshRequest) {
    if request.cancellation.is_cancelled() || !read_runtime().accepts_refresh(&request) {
        return;
    }

    let result = catch_unwind(AssertUnwindSafe(|| {
        #[cfg(feature = "visual-test-hooks")]
        if let Some(snapshot) = super::visual_test_hooks::visual_test_snapshot() {
            return snapshot;
        }
        let snapshot = devops::detect(&request.session);
        #[cfg(target_os = "windows")]
        let snapshot = {
            let mut snapshot = snapshot;
            if automexia_devops::kubernetes::is_wsl_session(&request.session) {
                publish_devops_progress(&request, &snapshot);
                let context = super::prompt_discovery::refresh(
                    &request.session,
                    &request.cancellation,
                );
                devops::attach_kubernetes_context(&mut snapshot, context);
            }
            snapshot
        };
        snapshot
    }));
    match result {
        Ok(snapshot) => {
            tracing::debug!(
                session_id = request.session.session_id,
                capsule_revision = request.capsule_revision,
                "Automexia local context discovery completed"
            );
            if request.cancellation.is_cancelled()
                || !read_runtime().accepts_refresh(&request)
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

struct InventoryRequest {
    revision: u64,
    cancellation: CancellationToken,
    completion: CompletionWake,
}

enum RuntimeWork {
    Inventory(InventoryRequest),
    Refresh(Box<RefreshRequest>),
}

fn process_inventory(request: InventoryRequest) {
    if request.cancellation.is_cancelled() {
        return;
    }
    let installed = catch_unwind(AssertUnwindSafe(|| {
        marketplace::descriptors()
            .iter()
            .filter(|manifest| state::is_installed(manifest))
            .map(|manifest| manifest.id.to_owned())
            .collect()
    }))
    .ok();
    let published = write_runtime().finish_inventory(request.revision, installed);
    if published {
        ACTIVATION_GENERATION.advance();
        request.completion.wake();
    }
}

fn process_work(request: RuntimeWork) {
    match request {
        RuntimeWork::Inventory(request) => process_inventory(request),
        RuntimeWork::Refresh(request) => process_refresh(*request),
    }
}

fn worker() -> &'static BoundedWorker<RuntimeWork> {
    static WORKER: OnceLock<BoundedWorker<RuntimeWork>> = OnceLock::new();
    WORKER
        .get_or_init(|| BoundedWorker::new("automexia-extension-worker", 1, process_work))
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
        if !runtime.accepts_refresh(&request) {
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

#[cfg(target_os = "windows")]
fn publish_devops_progress(request: &RefreshRequest, snapshot: &DevOpsSnapshot) {
    let mut runtime = write_runtime();
    if runtime.put_devops_progress(request, snapshot) {
        // The existing 100 ms pending-context timer observes this progress.
        // Keep the one-shot route wake for completion, not intermediate state.
        DEVOPS_GENERATION.advance();
    }
}

impl RuntimeState {
    #[cfg(any(target_os = "windows", test))]
    fn put_devops_progress(
        &mut self,
        request: &RefreshRequest,
        snapshot: &DevOpsSnapshot,
    ) -> bool {
        let session_id = request.session.session_id;
        if !self.accepts_refresh(request) || self.devops_snapshot(session_id).is_some() {
            // Refreshing an existing result must never clear its namespace.
            return false;
        }
        self.put_devops_snapshot(
            cache_key(
                &request.session,
                request.capsule_revision,
                request.source_revision,
            ),
            DEVOPS_COMPLETION_COUNTER.advance(),
            request.session.clone(),
            snapshot.clone(),
            Freshness::Refreshing,
            unix_time_ms(),
        );
        true
    }
}

fn publish_devops_failure(request: &mut RefreshRequest) {
    let session_id = request.session.session_id;
    let revision = DEVOPS_COMPLETION_COUNTER.advance();
    {
        let mut runtime = write_runtime();
        if !runtime.accepts_refresh(request) {
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
    context_revision: u64,
) -> bool {
    {
        let mut runtime = write_runtime();
        if capsule_revision == 0
            || !runtime.context_status_enabled()
            || runtime.context_revision != context_revision
            || runtime
                .capsules
                .get(&session.session_id)
                .is_none_or(|capsule| capsule.revision != capsule_revision)
            || runtime.devops_snapshot(session.session_id).is_some()
        {
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
    _context_revision: u64,
) -> bool {
    false
}

pub fn ensure_background_services() {
    if context_status_enabled() {
        let _ = worker().ensure_started();
    }
}

pub fn shutdown_background_services() {
    {
        let mut runtime = write_runtime();
        runtime.stop();
    }
    if !worker().shutdown_timeout(std::time::Duration::from_secs(2)) {
        tracing::warn!(
            "Background service cleanup did not complete: {:?}",
            worker().shutdown_status()
        );
    }
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
    if !context_status_enabled() {
        return RefreshSubmission::Rejected;
    }
    let source_revision = source_revision(session);
    let (capsule_revision, context_revision) = {
        let mut runtime = write_runtime();
        if !runtime.context_status_enabled() {
            return RefreshSubmission::Rejected;
        }
        (runtime.capsule_revision(session), runtime.context_revision)
    };
    if capsule_revision == 0 {
        return RefreshSubmission::Rejected;
    }
    let _ = seed_devops_snapshot(
        session,
        capsule_revision,
        source_revision,
        context_revision,
    );
    let operation_id = OperationId::new(u64::from(
        OPERATION_COUNTER
            .fetch_add(1, Ordering::AcqRel)
            .wrapping_add(1),
    ));
    let cancellation = CancellationToken::default();
    let request = RefreshRequest {
        context_revision,
        operation_id,
        session: session.clone(),
        capsule_revision,
        source_revision,
        cancellation: cancellation.clone(),
        completion,
    };
    let submission =
        worker().try_submit_then(RuntimeWork::Refresh(Box::new(request)), || {
            write_runtime().register_refresh(
                session.session_id,
                operation_id,
                capsule_revision,
                context_revision,
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
            installed: BTreeSet::from([devops::ID.to_owned()]),
            context_status_preference: true,
            context_revision: 1,
            inventory_status: InventoryStatus::Ready,
            inventory_revision: 1,
            inventory_cancellation: None,
            stopping: false,
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
            environment: Default::default(),
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
        let mut changed = session(7, "user@host:/mnt/d/work");
        changed.cwd = Some("/fixture/work".into());
        put(&mut state, changed, 2, snapshot("wsl"));
        let (key, entry) = state.devops_snapshot(7).unwrap();
        assert_eq!(entry.revision, 2);
        assert_eq!(entry.session.title, "user@host:/mnt/d/work");
        assert_eq!(entry.snapshot.environment.as_deref(), Some("wsl"));
        assert_eq!(key.capsule_revision, 2);
    }

    #[test]
    fn prompt_title_storm_preserves_pending_discovery_and_cache() {
        let mut state = state();
        let mut facts = session(51, "starting");
        put(&mut state, facts.clone(), 1, snapshot("sandbox"));
        let capsule = state.capsule_revision(&facts);
        let source = source_revision(&facts);
        let operation = OperationId::new(9);
        let cancellation = CancellationToken::default();
        state.register_operation(51, operation, cancellation.clone());
        // OSC titles can change between shell metadata frames without changing
        // a discovery input. They must not starve the one background worker.
        for index in 0..256 {
            facts.title = format!("command-{index}");
            assert_eq!(state.capsule_revision(&facts), capsule);
            assert_eq!(source_revision(&facts), source);
            assert!(!cancellation.is_cancelled());
            assert!(state.accepts(51, operation, capsule));
            assert!(state.devops_snapshot(51).is_some());
        }
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

        let mut changed = session(50, "alice@host:/work");
        changed.cwd = Some("/fixture/work".into());
        let new_capsule = state.capsule_revision(&changed);
        assert_ne!(old_capsule, new_capsule);
        assert!(cancellation.is_cancelled());
        assert!(!state.accepts(50, operation, old_capsule));
        assert!(!state.accepts(50, operation, new_capsule));
        assert!(state.devops_snapshot(50).is_none());
    }

    #[test]
    fn every_discovery_input_still_invalidates_pending_work() {
        let original = session(61, "fixture");
        for field in 0..10 {
            let mut state = state();
            let capsule = state.capsule_revision(&original);
            let operation = OperationId::new(1);
            let cancellation = CancellationToken::default();
            state.register_operation(61, operation, cancellation.clone());
            let mut changed = original.clone();
            match field {
                0 => changed.cwd = Some("/fixture/changed".into()),
                1 => changed.distro = Some("Fixture-Distro".into()),
                2 => changed.os_version = Some("1".into()),
                3 => changed.shell_name = Some("bash".into()),
                4 => changed.shell_user = Some("alice".into()),
                5 => changed.shell_path = Some("/bin/bash".into()),
                6 => changed.shell_integration = false,
                7 => changed.shell_pid = 12,
                8 => {
                    changed
                        .environment
                        .insert("HOME".into(), "/fixture/home".into());
                }
                _ => {
                    changed
                        .environment
                        .insert("KUBECONFIG".into(), "/fixture/config".into());
                }
            }
            assert!(!same_devops_context(&original, &changed));
            assert_ne!(source_revision(&original), source_revision(&changed));
            assert_ne!(state.capsule_revision(&changed), capsule);
            assert!(cancellation.is_cancelled());
            assert!(!state.accepts(61, operation, capsule));
        }
    }

    #[test]
    fn initial_progress_keeps_operation_pending_and_never_replaces_a_cached_namespace() {
        let mut state = state();
        let facts = session(62, "fixture");
        let capsule_revision = state.capsule_revision(&facts);
        let cancellation = CancellationToken::default();
        let request = RefreshRequest {
            context_revision: 1,
            operation_id: OperationId::new(2),
            source_revision: source_revision(&facts),
            session: facts.clone(),
            capsule_revision,
            cancellation: cancellation.clone(),
            completion: None,
        };
        state.register_operation(62, request.operation_id, cancellation.clone());
        let initial = DevOpsSnapshot {
            git_branch: Some("fixture-branch".into()),
            ..Default::default()
        };
        assert!(state.put_devops_progress(&request, &initial));
        let (_, entry) = state.devops_snapshot(62).unwrap();
        assert_eq!(entry.freshness, Freshness::Refreshing);
        assert_eq!(entry.snapshot.git_branch.as_deref(), Some("fixture-branch"));
        assert!(state.accepts(62, request.operation_id, capsule_revision));
        let mut completed = initial.clone();
        completed.kubernetes = Some(automexia_devops::KubernetesContext {
            context: "fixture".into(),
            namespace: "sandbox".into(),
        });
        put(&mut state, facts, 100, completed);
        assert!(!state.put_devops_progress(&request, &initial));
        assert_eq!(
            state
                .devops_snapshot(62)
                .unwrap()
                .1
                .snapshot
                .kubernetes
                .unwrap()
                .namespace,
            "sandbox"
        );
    }

    #[test]
    fn obsolete_or_cancelled_progress_cannot_publish() {
        for cancel in [false, true] {
            let mut state = state();
            let facts = session(63, "fixture");
            let capsule_revision = state.capsule_revision(&facts);
            let request = RefreshRequest {
                context_revision: 1,
                operation_id: OperationId::new(3),
                source_revision: source_revision(&facts),
                session: facts,
                capsule_revision,
                cancellation: CancellationToken::default(),
                completion: None,
            };
            state.register_operation(
                63,
                if cancel {
                    request.operation_id
                } else {
                    OperationId::new(4)
                },
                request.cancellation.clone(),
            );
            if cancel {
                request.cancellation.cancel();
            }
            assert!(!state.put_devops_progress(&request, &snapshot("must-not-appear")));
            assert!(state.devops_snapshot(63).is_none());
        }
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
    fn incomplete_or_failed_snapshots_are_not_reused_by_another_pane() {
        for freshness in [
            Freshness::Refreshing,
            Freshness::Error,
            Freshness::Unavailable,
        ] {
            let mut state = state();
            let source = session(305, "fixture");
            put(&mut state, source.clone(), 1, snapshot("sandbox"));
            let key = state.devops_snapshot(305).unwrap().0;
            state.devops_snapshots.get_mut(&key).unwrap().freshness = freshness;
            let mut other = source;
            other.session_id = 306;
            assert!(state.reusable_devops_snapshot(&other).is_none());
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn equivalent_new_session_reuses_a_fresh_snapshot() {
        let mut state = state();
        let mut source = session(301, "alice@host:/mnt/d/work");
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

    mod context_settings {
        include!("runtime_context_settings_tests.rs");
    }

    #[cfg(not(target_arch = "wasm32"))]
    mod inventory_process {
        include!("runtime_inventory_process_tests.rs");
    }
    mod metadata_readiness {
        include!("runtime_metadata_readiness_tests.rs");
    }
}
