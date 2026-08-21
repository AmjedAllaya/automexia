use std::{
    collections::BTreeMap,
    fmt,
    path::{Path, PathBuf},
    sync::{mpsc, Arc, Condvar, Mutex, MutexGuard, Weak},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use automexia_devops::connections::{AuthState, EnvironmentRisk, ProviderKind};
use automexia_devops_ssh::{
    scan_inventory_cancellable, ConnectionMetadata, ConnectionRecord, GrantKind,
    IdentityHint, InventoryError, InventoryGrant, InventoryLimits, MetadataDocument,
    MetadataLoadOrigin, MetadataLoadResult, MetadataStore, ScanCancellation, SourceKind,
    MAX_SOURCE_FILES,
};
use automexia_extension_runtime::CompletionWake;
use automexia_ui_model::connection_hub::{
    ConnectionCatalogEntry, ConnectionSummary, HubCatalogSource,
};

use super::library::{
    ConnectionLibraryDocument, ConnectionLibraryStore, HubPreferences, LibraryLoadOrigin,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlatformFamily {
    Windows,
    MacOs,
    Linux,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SetupGuidance {
    pub message: &'static str,
    pub candidate_locations: &'static [&'static str],
    pub requires_explicit_scan: bool,
    pub opens_network_connections: bool,
    pub launches_processes: bool,
}

pub fn platform_setup_guidance(platform: PlatformFamily) -> SetupGuidance {
    let candidate_locations = match platform {
        PlatformFamily::Windows => &[
            "%USERPROFILE%\\.ssh\\config",
            "%PROGRAMDATA%\\ssh\\ssh_config",
        ][..],
        PlatformFamily::MacOs => &["~/.ssh/config", "/etc/ssh/ssh_config"][..],
        PlatformFamily::Linux => &["~/.ssh/config", "/etc/ssh/ssh_config"][..],
    };
    SetupGuidance {
        message: "Choose exact OpenSSH configuration files to scan locally. No connection, login, or command will run.",
        candidate_locations,
        requires_explicit_scan: true,
        opens_network_connections: false,
        launches_processes: false,
    }
}

const SELECTION_DIAGNOSTIC: &str = "ssh-inventory-selection-rejected";
const REFRESH_DIAGNOSTIC: &str = "ssh-inventory-refresh-failed";
const STORE_DIAGNOSTIC: &str = "connection-private-store-unavailable";
const METADATA_DIAGNOSTIC: &str = "connection-metadata-write-failed";
const MAX_REVIEW_PATH_DISPLAY_BYTES: usize = 1_024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HubStoreState {
    Initializing,
    Ready,
    Recovered,
    Unavailable { diagnostic_code: &'static str },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HubRuntimeErrorCode {
    WorkerUnavailable,
    StoreUnavailable,
    InvalidSelection,
    StaleReview,
    UnknownConnection,
    InvalidMetadata,
    StaleMetadata,
}

impl fmt::Display for HubRuntimeErrorCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::WorkerUnavailable => "connection-worker-unavailable",
            Self::StoreUnavailable => "connection-private-store-unavailable",
            Self::InvalidSelection => "connection-selection-invalid",
            Self::StaleReview => "connection-selection-review-stale",
            Self::UnknownConnection => "connection-record-unavailable",
            Self::InvalidMetadata => "connection-metadata-invalid",
            Self::StaleMetadata => "connection-metadata-review-stale",
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReviewedGrantFile {
    pub display_path: String,
    pub kind: GrantKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GrantReviewState {
    None,
    Reviewing {
        request: u64,
    },
    Ready {
        request: u64,
        files: Arc<Vec<ReviewedGrantFile>>,
    },
    Error {
        request: u64,
        diagnostic_code: &'static str,
    },
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HubMetadataValues {
    pub favorite: bool,
    pub tags: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MetadataChangeReview {
    pub connection_id: String,
    pub expected_revision: u64,
    pub before: HubMetadataValues,
    pub after: HubMetadataValues,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HubMetadataChangeState {
    Idle,
    Applying {
        request: u64,
    },
    Applied {
        request: u64,
        revision: u64,
    },
    Conflict {
        request: u64,
        current_revision: u64,
    },
    Error {
        request: u64,
        diagnostic_code: &'static str,
    },
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HubLibrarySnapshot {
    pub revision: u64,
    pub profile_count: usize,
    pub recipe_count: usize,
    pub preferences: HubPreferences,
    pub recovered: bool,
}

impl HubLibrarySnapshot {
    fn from_document(document: &ConnectionLibraryDocument, recovered: bool) -> Self {
        Self {
            revision: document.revision,
            profile_count: document.profiles.profiles.len(),
            recipe_count: document.recipes.recipes.len(),
            preferences: document.preferences.clone(),
            recovered,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HubRuntimeSnapshot {
    pub state: HubRuntimeState,
    pub catalog: Arc<Vec<ConnectionCatalogEntry>>,
    pub grant_review: GrantReviewState,
    pub metadata_change: HubMetadataChangeState,
    pub metadata_revision: u64,
    pub store_state: HubStoreState,
    pub library: HubLibrarySnapshot,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HubRuntimeState {
    Initializing,
    InitialSetup,
    Loading {
        request: u64,
    },
    Ready {
        generation: u64,
    },
    Stale {
        generation: u64,
        diagnostic_code: &'static str,
        failed_request: u64,
    },
    Error {
        diagnostic_code: &'static str,
        failed_request: u64,
    },
    Shutdown,
}

enum Startup {
    Preloaded {
        metadata_store: MetadataStore,
        metadata: MetadataLoadResult,
    },
    Root(PathBuf),
}

enum Work {
    Initialize(Startup),
    ReviewFiles {
        paths: Vec<PathBuf>,
        kind: GrantKind,
    },
    Scan {
        grants: Vec<InventoryGrant>,
        cancellation: ScanCancellation,
    },
    ApplyMetadata(MetadataChangeReview),
}

struct WorkRequest {
    request: u64,
    work: Work,
    wake: Option<CompletionWake>,
}

struct WorkerStores {
    metadata: MetadataStore,
    _library: Option<ConnectionLibraryStore>,
}

struct RuntimeData {
    requested: u64,
    completed: u64,
    successful_generation: u64,
    initialized: bool,
    shutdown: bool,
    active_cancellation: Option<ScanCancellation>,
    state: HubRuntimeState,
    catalog: Arc<Vec<ConnectionCatalogEntry>>,
    records: Arc<Vec<ConnectionRecord>>,
    metadata: MetadataDocument,
    grant_review: GrantReviewState,
    reviewed_grants: Option<(u64, Vec<InventoryGrant>)>,
    metadata_change: HubMetadataChangeState,
    store_state: HubStoreState,
    library: HubLibrarySnapshot,
}

struct RuntimeInner {
    data: Mutex<RuntimeData>,
    settled: Condvar,
    sender: Mutex<Option<mpsc::Sender<WorkRequest>>>,
    handle: Mutex<Option<JoinHandle<()>>>,
}

impl Drop for RuntimeInner {
    fn drop(&mut self) {
        let cancellation = {
            let mut data = lock(&self.data);
            data.shutdown = true;
            data.active_cancellation.take()
        };
        if let Some(cancellation) = cancellation {
            cancellation.cancel();
        }
        lock(&self.sender).take();
        if let Some(handle) = lock(&self.handle).take() {
            if handle.thread().id() != thread::current().id() {
                let _ = handle.join();
            }
        }
    }
}

#[derive(Clone)]
pub struct ConnectionHubRuntime {
    inner: Arc<RuntimeInner>,
}

impl ConnectionHubRuntime {
    pub fn open(metadata_store: MetadataStore) -> Result<Self, InventoryError> {
        let metadata = metadata_store.load_with_recovery()?;
        Self::spawn(
            Startup::Preloaded {
                metadata_store,
                metadata,
            },
            0,
        )
        .and_then(|runtime| {
            if runtime.wait_for_settled(Duration::from_secs(5)) {
                Ok(runtime)
            } else {
                Err(InventoryError::Persistence(
                    "connection inventory worker initialization timed out".into(),
                ))
            }
        })
    }

    pub fn open_at_root(root: impl AsRef<Path>) -> Self {
        Self::spawn(Startup::Root(root.as_ref().to_path_buf()), 1)
            .unwrap_or_else(|_| Self::disabled())
    }

    pub fn open_default() -> Self {
        Self::open_at_root(rio_backend::config::config_dir_path())
    }

    fn spawn(
        startup: Startup,
        initialization_request: u64,
    ) -> Result<Self, InventoryError> {
        let (sender, receiver) = mpsc::channel();
        let inner = Arc::new(RuntimeInner {
            data: Mutex::new(RuntimeData {
                requested: initialization_request,
                completed: 0,
                successful_generation: 0,
                initialized: false,
                shutdown: false,
                active_cancellation: None,
                state: HubRuntimeState::Initializing,
                catalog: Arc::new(Vec::new()),
                records: Arc::new(Vec::new()),
                metadata: MetadataDocument::default(),
                grant_review: GrantReviewState::None,
                reviewed_grants: None,
                metadata_change: HubMetadataChangeState::Idle,
                store_state: HubStoreState::Initializing,
                library: HubLibrarySnapshot::default(),
            }),
            settled: Condvar::new(),
            sender: Mutex::new(Some(sender.clone())),
            handle: Mutex::new(None),
        });
        let weak = Arc::downgrade(&inner);
        let handle = thread::Builder::new()
            .name("automexia-connection-inventory".into())
            .spawn(move || worker_loop(weak, receiver))
            .map_err(|_| {
                InventoryError::Persistence(
                    "connection inventory worker could not start".into(),
                )
            })?;
        *lock(&inner.handle) = Some(handle);
        sender
            .send(WorkRequest {
                request: initialization_request,
                work: Work::Initialize(startup),
                wake: None,
            })
            .map_err(|_| {
                InventoryError::Persistence(
                    "connection inventory worker could not initialize".into(),
                )
            })?;
        Ok(Self { inner })
    }

    fn disabled() -> Self {
        Self {
            inner: Arc::new(RuntimeInner {
                data: Mutex::new(RuntimeData {
                    requested: 1,
                    completed: 1,
                    successful_generation: 0,
                    initialized: true,
                    shutdown: false,
                    active_cancellation: None,
                    state: HubRuntimeState::Error {
                        diagnostic_code: STORE_DIAGNOSTIC,
                        failed_request: 1,
                    },
                    catalog: Arc::new(Vec::new()),
                    records: Arc::new(Vec::new()),
                    metadata: MetadataDocument::default(),
                    grant_review: GrantReviewState::None,
                    reviewed_grants: None,
                    metadata_change: HubMetadataChangeState::Idle,
                    store_state: HubStoreState::Unavailable {
                        diagnostic_code: STORE_DIAGNOSTIC,
                    },
                    library: HubLibrarySnapshot::default(),
                }),
                settled: Condvar::new(),
                sender: Mutex::new(None),
                handle: Mutex::new(None),
            }),
        }
    }

    fn enqueue(&self, request: WorkRequest) -> Result<(), HubRuntimeErrorCode> {
        let sender = lock(&self.inner.sender)
            .as_ref()
            .cloned()
            .ok_or(HubRuntimeErrorCode::WorkerUnavailable)?;
        sender
            .send(request)
            .map_err(|_| HubRuntimeErrorCode::WorkerUnavailable)
    }

    fn begin_request(&self) -> Result<u64, HubRuntimeErrorCode> {
        if lock(&self.inner.handle)
            .as_ref()
            .is_none_or(JoinHandle::is_finished)
        {
            return Err(HubRuntimeErrorCode::WorkerUnavailable);
        }
        let mut data = lock(&self.inner.data);
        if data.shutdown {
            return Err(HubRuntimeErrorCode::WorkerUnavailable);
        }
        if let Some(active) = data.active_cancellation.take() {
            active.cancel();
        }
        data.requested = data.requested.saturating_add(1);
        Ok(data.requested)
    }

    pub fn request_explicit_scan(&self, grants: Vec<InventoryGrant>) -> u64 {
        let Ok(request) = self.begin_request() else {
            return 0;
        };
        let cancellation = ScanCancellation::default();
        {
            let mut data = lock(&self.inner.data);
            data.active_cancellation = Some(cancellation.clone());
            data.state = HubRuntimeState::Loading { request };
        }
        if self
            .enqueue(WorkRequest {
                request,
                work: Work::Scan {
                    grants,
                    cancellation,
                },
                wake: None,
            })
            .is_err()
        {
            complete_scan_failure(&self.inner, request);
        }
        request
    }

    pub fn review_exact_files(
        &self,
        paths: Vec<PathBuf>,
        kind: GrantKind,
        wake: CompletionWake,
    ) -> Result<u64, HubRuntimeErrorCode> {
        if paths.is_empty()
            || paths.len() > MAX_SOURCE_FILES
            || paths.iter().any(|path| !path.is_absolute())
        {
            return Err(HubRuntimeErrorCode::InvalidSelection);
        }
        let request = self.begin_request()?;
        {
            let mut data = lock(&self.inner.data);
            data.reviewed_grants = None;
            data.grant_review = GrantReviewState::Reviewing { request };
        }
        if let Err(error) = self.enqueue(WorkRequest {
            request,
            work: Work::ReviewFiles { paths, kind },
            wake: Some(wake),
        }) {
            complete_review_error(&self.inner, request);
            return Err(error);
        }
        Ok(request)
    }

    pub fn confirm_reviewed_scan(
        &self,
        reviewed_request: u64,
        wake: CompletionWake,
    ) -> Result<u64, HubRuntimeErrorCode> {
        let grants = {
            let mut data = lock(&self.inner.data);
            match data.reviewed_grants.take() {
                Some((request, grants)) if request == reviewed_request => {
                    data.grant_review = GrantReviewState::None;
                    grants
                }
                Some(reviewed) => {
                    data.reviewed_grants = Some(reviewed);
                    return Err(HubRuntimeErrorCode::StaleReview);
                }
                None => return Err(HubRuntimeErrorCode::StaleReview),
            }
        };
        let request = self.begin_request()?;
        let cancellation = ScanCancellation::default();
        {
            let mut data = lock(&self.inner.data);
            data.active_cancellation = Some(cancellation.clone());
            data.state = HubRuntimeState::Loading { request };
        }
        if let Err(error) = self.enqueue(WorkRequest {
            request,
            work: Work::Scan {
                grants,
                cancellation,
            },
            wake: Some(wake),
        }) {
            complete_scan_failure(&self.inner, request);
            return Err(error);
        }
        Ok(request)
    }

    pub fn discard_review(&self) {
        let mut data = lock(&self.inner.data);
        if data.shutdown || matches!(data.grant_review, GrantReviewState::None) {
            return;
        }
        data.requested = data.requested.saturating_add(1);
        data.completed = data.requested;
        data.reviewed_grants = None;
        data.grant_review = GrantReviewState::None;
        self.inner.settled.notify_all();
    }

    pub fn review_metadata_change(
        &self,
        connection_id: &str,
        favorite: Option<bool>,
        tags: Option<Vec<String>>,
    ) -> Result<MetadataChangeReview, HubRuntimeErrorCode> {
        let data = lock(&self.inner.data);
        if !data
            .catalog
            .iter()
            .any(|entry| entry.summary.id == connection_id)
        {
            return Err(HubRuntimeErrorCode::UnknownConnection);
        }
        let current = data
            .metadata
            .connections
            .iter()
            .find(|item| item.connection_id == connection_id);
        let before = HubMetadataValues {
            favorite: current.is_some_and(|item| item.favorite),
            tags: current.map_or_else(Vec::new, |item| item.tags.clone()),
        };
        let after = HubMetadataValues {
            favorite: favorite.unwrap_or(before.favorite),
            tags: tags.unwrap_or_else(|| before.tags.clone()),
        };
        let candidate = metadata_with_change(
            &data.metadata,
            connection_id,
            &after,
            current.and_then(|item| item.display_name.clone()),
            current.and_then(|item| item.last_used_at_ms),
        );
        candidate
            .validate()
            .map_err(|_| HubRuntimeErrorCode::InvalidMetadata)?;
        Ok(MetadataChangeReview {
            connection_id: connection_id.to_owned(),
            expected_revision: data.metadata.revision,
            before,
            after,
        })
    }

    pub fn apply_metadata_change(
        &self,
        review: MetadataChangeReview,
        wake: CompletionWake,
    ) -> Result<u64, HubRuntimeErrorCode> {
        {
            let data = lock(&self.inner.data);
            if review.expected_revision != data.metadata.revision {
                return Err(HubRuntimeErrorCode::StaleMetadata);
            }
            let current = data
                .metadata
                .connections
                .iter()
                .find(|item| item.connection_id == review.connection_id);
            let values = HubMetadataValues {
                favorite: current.is_some_and(|item| item.favorite),
                tags: current.map_or_else(Vec::new, |item| item.tags.clone()),
            };
            if values != review.before {
                return Err(HubRuntimeErrorCode::StaleMetadata);
            }
        }
        let request = self.begin_request()?;
        lock(&self.inner.data).metadata_change =
            HubMetadataChangeState::Applying { request };
        if let Err(error) = self.enqueue(WorkRequest {
            request,
            work: Work::ApplyMetadata(review),
            wake: Some(wake),
        }) {
            complete_metadata_error(&self.inner, request);
            return Err(error);
        }
        Ok(request)
    }

    pub fn state(&self) -> HubRuntimeState {
        lock(&self.inner.data).state.clone()
    }

    pub fn catalog(&self) -> Arc<Vec<ConnectionCatalogEntry>> {
        Arc::clone(&lock(&self.inner.data).catalog)
    }

    pub fn snapshot(&self) -> HubRuntimeSnapshot {
        let data = lock(&self.inner.data);
        HubRuntimeSnapshot {
            state: data.state.clone(),
            catalog: Arc::clone(&data.catalog),
            grant_review: data.grant_review.clone(),
            metadata_change: data.metadata_change.clone(),
            metadata_revision: data.metadata.revision,
            store_state: data.store_state,
            library: data.library.clone(),
        }
    }

    pub fn wait_for_settled(&self, timeout: Duration) -> bool {
        let deadline = Instant::now() + timeout;
        let mut data = lock(&self.inner.data);
        loop {
            if data.initialized && data.completed >= data.requested {
                return true;
            }
            let Some(remaining) = deadline.checked_duration_since(Instant::now()) else {
                return false;
            };
            let (next, result) = self
                .inner
                .settled
                .wait_timeout(data, remaining)
                .unwrap_or_else(|error| error.into_inner());
            data = next;
            if result.timed_out()
                && (!data.initialized || data.completed < data.requested)
            {
                return false;
            }
        }
    }

    pub fn shutdown(&self) {
        let cancellation = {
            let mut data = lock(&self.inner.data);
            if data.shutdown {
                return;
            }
            data.shutdown = true;
            data.state = HubRuntimeState::Shutdown;
            data.completed = data.requested;
            data.reviewed_grants = None;
            data.grant_review = GrantReviewState::None;
            self.inner.settled.notify_all();
            data.active_cancellation.take()
        };
        if let Some(cancellation) = cancellation {
            cancellation.cancel();
        }
        lock(&self.inner.sender).take();
        if let Some(handle) = lock(&self.inner.handle).take() {
            if handle.thread().id() != thread::current().id() {
                let _ = handle.join();
            }
        }
    }
}

fn worker_loop(inner: Weak<RuntimeInner>, receiver: mpsc::Receiver<WorkRequest>) {
    let mut stores = None;
    while let Ok(work_request) = receiver.recv() {
        let Some(inner) = inner.upgrade() else {
            return;
        };
        let WorkRequest {
            request,
            work,
            wake,
        } = work_request;
        let is_initialization = matches!(work, Work::Initialize(_));
        if !is_initialization && !is_current_request(&inner, request) {
            continue;
        }
        let published = match work {
            Work::Initialize(startup) => {
                complete_initialization(&inner, request, startup, &mut stores)
            }
            Work::ReviewFiles { paths, kind } => {
                complete_file_review(&inner, request, paths, kind, stores.is_some())
            }
            Work::Scan {
                grants,
                cancellation,
            } => complete_scan(&inner, request, grants, cancellation, stores.is_some()),
            Work::ApplyMetadata(review) => {
                complete_metadata_change(&inner, request, review, stores.as_ref())
            }
        };
        if published {
            if let Some(wake) = wake {
                wake.wake();
            }
        }
        if lock(&inner.data).shutdown {
            return;
        }
    }
}

fn is_current_request(inner: &RuntimeInner, request: u64) -> bool {
    let data = lock(&inner.data);
    !data.shutdown && request == data.requested
}

fn complete_initialization(
    inner: &RuntimeInner,
    request: u64,
    startup: Startup,
    stores: &mut Option<WorkerStores>,
) -> bool {
    let initialized = initialize_stores(startup);
    let mut data = lock(&inner.data);
    data.initialized = true;
    data.completed = data.completed.max(request);
    match initialized {
        Ok((worker_stores, metadata, library, recovered)) => {
            data.metadata = metadata;
            data.library = library;
            data.store_state = if recovered {
                HubStoreState::Recovered
            } else {
                HubStoreState::Ready
            };
            if matches!(data.state, HubRuntimeState::Initializing) {
                data.state = HubRuntimeState::InitialSetup;
            }
            *stores = Some(worker_stores);
        }
        Err(()) => {
            data.store_state = HubStoreState::Unavailable {
                diagnostic_code: STORE_DIAGNOSTIC,
            };
            data.state = HubRuntimeState::Error {
                diagnostic_code: STORE_DIAGNOSTIC,
                failed_request: request,
            };
        }
    }
    inner.settled.notify_all();
    true
}

fn initialize_stores(
    startup: Startup,
) -> Result<(WorkerStores, MetadataDocument, HubLibrarySnapshot, bool), ()> {
    match startup {
        Startup::Preloaded {
            metadata_store,
            metadata,
        } => {
            let recovered = metadata.origin == MetadataLoadOrigin::PreviousRecovery
                || metadata.rejected_primary;
            Ok((
                WorkerStores {
                    metadata: metadata_store,
                    _library: None,
                },
                metadata.document,
                HubLibrarySnapshot::default(),
                recovered,
            ))
        }
        Startup::Root(root) => {
            let metadata_store =
                MetadataStore::new(root.join("extensions").join("devops-ssh"))
                    .map_err(|_| ())?;
            let metadata = metadata_store.load_with_recovery().map_err(|_| ())?;
            let library_store =
                ConnectionLibraryStore::open(root.join("connections")).map_err(|_| ())?;
            let library = library_store.load().map_err(|_| ())?;
            let recovered = metadata.origin == MetadataLoadOrigin::PreviousRecovery
                || metadata.rejected_primary
                || library.origin == LibraryLoadOrigin::PreviousRecovery
                || library.rejected_primary;
            let snapshot =
                HubLibrarySnapshot::from_document(&library.document, recovered);
            Ok((
                WorkerStores {
                    metadata: metadata_store,
                    _library: Some(library_store),
                },
                metadata.document,
                snapshot,
                recovered,
            ))
        }
    }
}

fn complete_file_review(
    inner: &RuntimeInner,
    request: u64,
    paths: Vec<PathBuf>,
    kind: GrantKind,
    stores_ready: bool,
) -> bool {
    if !stores_ready {
        return complete_review_failure(inner, request, STORE_DIAGNOSTIC);
    }
    match review_files(paths, kind) {
        Ok((grants, files)) => {
            let mut data = lock(&inner.data);
            if data.shutdown || request != data.requested {
                return false;
            }
            data.reviewed_grants = Some((request, grants));
            data.grant_review = GrantReviewState::Ready {
                request,
                files: Arc::new(files),
            };
            data.completed = data.completed.max(request);
            inner.settled.notify_all();
            true
        }
        Err(()) => complete_review_failure(inner, request, SELECTION_DIAGNOSTIC),
    }
}

fn review_files(
    paths: Vec<PathBuf>,
    kind: GrantKind,
) -> Result<(Vec<InventoryGrant>, Vec<ReviewedGrantFile>), ()> {
    if paths.is_empty() || paths.len() > MAX_SOURCE_FILES {
        return Err(());
    }
    let mut reviewed = BTreeMap::new();
    for (index, path) in paths.into_iter().enumerate() {
        if !path.is_absolute() {
            return Err(());
        }
        let parent = path.parent().ok_or(())?;
        let grant = InventoryGrant::new(
            format!("selected-{}", index.saturating_add(1)),
            parent,
            [&path],
            kind,
        )
        .map_err(|_| ())?;
        let canonical = grant.entries().first().cloned().ok_or(())?;
        reviewed.entry(canonical.clone()).or_insert_with(|| {
            (
                grant,
                ReviewedGrantFile {
                    display_path: safe_path_display(&canonical),
                    kind,
                },
            )
        });
    }
    let (grants, files): (Vec<_>, Vec<_>) = reviewed.into_values().unzip();
    Ok((grants, files))
}

fn safe_path_display(path: &Path) -> String {
    let mut display = String::with_capacity(MAX_REVIEW_PATH_DISPLAY_BYTES.min(256));
    for character in path.to_string_lossy().chars() {
        let character = if character.is_control() || unsafe_format_character(character) {
            '\u{fffd}'
        } else {
            character
        };
        if display.len().saturating_add(character.len_utf8())
            > MAX_REVIEW_PATH_DISPLAY_BYTES.saturating_sub(3)
        {
            display.push_str("...");
            break;
        }
        display.push(character);
    }
    display
}

fn unsafe_format_character(character: char) -> bool {
    let codepoint = character as u32;
    codepoint == 0x061c
        || (0x200b..=0x200f).contains(&codepoint)
        || (0x202a..=0x202e).contains(&codepoint)
        || (0x2060..=0x206f).contains(&codepoint)
        || codepoint == 0xfeff
}

fn complete_review_error(inner: &RuntimeInner, request: u64) -> bool {
    complete_review_failure(inner, request, SELECTION_DIAGNOSTIC)
}

fn complete_review_failure(
    inner: &RuntimeInner,
    request: u64,
    diagnostic_code: &'static str,
) -> bool {
    let mut data = lock(&inner.data);
    if data.shutdown || request != data.requested {
        return false;
    }
    data.reviewed_grants = None;
    data.grant_review = GrantReviewState::Error {
        request,
        diagnostic_code,
    };
    data.completed = data.completed.max(request);
    inner.settled.notify_all();
    true
}

fn complete_scan(
    inner: &RuntimeInner,
    request: u64,
    grants: Vec<InventoryGrant>,
    cancellation: ScanCancellation,
    stores_ready: bool,
) -> bool {
    if !stores_ready {
        return complete_scan_failure_with(inner, request, STORE_DIAGNOSTIC);
    }
    let outcome = scan_inventory_cancellable(
        &grants,
        InventoryLimits::default(),
        request,
        cancellation,
    );
    let Ok(outcome) = outcome else {
        return complete_scan_failure(inner, request);
    };
    let records = outcome.snapshot.records;
    let metadata = lock(&inner.data).metadata.clone();
    let catalog = compose_catalog(&records, &metadata, request);
    let mut data = lock(&inner.data);
    if data.shutdown || request != data.requested {
        return false;
    }
    data.records = Arc::new(records);
    data.catalog = Arc::new(catalog);
    data.successful_generation = request;
    data.completed = data.completed.max(request);
    data.active_cancellation = None;
    data.state = HubRuntimeState::Ready {
        generation: request,
    };
    inner.settled.notify_all();
    true
}

fn complete_scan_failure(inner: &RuntimeInner, request: u64) -> bool {
    complete_scan_failure_with(inner, request, REFRESH_DIAGNOSTIC)
}

fn complete_scan_failure_with(
    inner: &RuntimeInner,
    request: u64,
    diagnostic_code: &'static str,
) -> bool {
    let mut data = lock(&inner.data);
    if data.shutdown || request != data.requested {
        return false;
    }
    data.completed = data.completed.max(request);
    data.active_cancellation = None;
    data.state = if data.catalog.is_empty() {
        HubRuntimeState::Error {
            diagnostic_code,
            failed_request: request,
        }
    } else {
        HubRuntimeState::Stale {
            generation: data.successful_generation,
            diagnostic_code,
            failed_request: request,
        }
    };
    inner.settled.notify_all();
    true
}

fn complete_metadata_change(
    inner: &RuntimeInner,
    request: u64,
    review: MetadataChangeReview,
    stores: Option<&WorkerStores>,
) -> bool {
    let Some(stores) = stores else {
        return complete_metadata_failure(inner, request, STORE_DIAGNOSTIC);
    };
    let current = lock(&inner.data).metadata.clone();
    if current.revision != review.expected_revision {
        return complete_metadata_conflict(inner, request, current);
    }
    let existing = current
        .connections
        .iter()
        .find(|item| item.connection_id == review.connection_id);
    let candidate = metadata_with_change(
        &current,
        &review.connection_id,
        &review.after,
        existing.and_then(|item| item.display_name.clone()),
        existing.and_then(|item| item.last_used_at_ms),
    );
    if candidate.validate().is_err() {
        return complete_metadata_failure(inner, request, METADATA_DIAGNOSTIC);
    }
    match stores
        .metadata
        .compare_and_swap(review.expected_revision, &candidate)
    {
        Ok(next) => complete_metadata_success(inner, request, next),
        Err(_) => match stores.metadata.load_with_recovery() {
            Ok(current) if current.document.revision != review.expected_revision => {
                complete_metadata_conflict(inner, request, current.document)
            }
            Ok(_) | Err(_) => {
                complete_metadata_failure(inner, request, METADATA_DIAGNOSTIC)
            }
        },
    }
}

fn complete_metadata_success(
    inner: &RuntimeInner,
    request: u64,
    metadata: MetadataDocument,
) -> bool {
    let mut data = lock(&inner.data);
    if data.shutdown || request != data.requested {
        return false;
    }
    data.catalog = Arc::new(compose_catalog(
        &data.records,
        &metadata,
        data.successful_generation,
    ));
    let revision = metadata.revision;
    data.metadata = metadata;
    data.metadata_change = HubMetadataChangeState::Applied { request, revision };
    data.completed = data.completed.max(request);
    inner.settled.notify_all();
    true
}

fn complete_metadata_conflict(
    inner: &RuntimeInner,
    request: u64,
    metadata: MetadataDocument,
) -> bool {
    let mut data = lock(&inner.data);
    if data.shutdown || request != data.requested {
        return false;
    }
    data.catalog = Arc::new(compose_catalog(
        &data.records,
        &metadata,
        data.successful_generation,
    ));
    let current_revision = metadata.revision;
    data.metadata = metadata;
    data.metadata_change = HubMetadataChangeState::Conflict {
        request,
        current_revision,
    };
    data.completed = data.completed.max(request);
    inner.settled.notify_all();
    true
}

fn complete_metadata_error(inner: &RuntimeInner, request: u64) -> bool {
    complete_metadata_failure(inner, request, METADATA_DIAGNOSTIC)
}

fn complete_metadata_failure(
    inner: &RuntimeInner,
    request: u64,
    diagnostic_code: &'static str,
) -> bool {
    let mut data = lock(&inner.data);
    if data.shutdown || request != data.requested {
        return false;
    }
    data.metadata_change = HubMetadataChangeState::Error {
        request,
        diagnostic_code,
    };
    data.completed = data.completed.max(request);
    inner.settled.notify_all();
    true
}

fn metadata_with_change(
    current: &MetadataDocument,
    connection_id: &str,
    values: &HubMetadataValues,
    display_name: Option<String>,
    last_used_at_ms: Option<u64>,
) -> MetadataDocument {
    let mut next = current.clone();
    if let Some(item) = next
        .connections
        .iter_mut()
        .find(|item| item.connection_id == connection_id)
    {
        item.favorite = values.favorite;
        item.tags.clone_from(&values.tags);
    } else {
        next.connections.push(ConnectionMetadata {
            connection_id: connection_id.to_owned(),
            display_name,
            tags: values.tags.clone(),
            favorite: values.favorite,
            last_used_at_ms,
        });
    }
    next.connections
        .sort_by(|left, right| left.connection_id.cmp(&right.connection_id));
    next
}

fn compose_catalog(
    records: &[ConnectionRecord],
    metadata: &MetadataDocument,
    generation: u64,
) -> Vec<ConnectionCatalogEntry> {
    let metadata_by_id = metadata
        .connections
        .iter()
        .map(|item| (item.connection_id.as_str(), item))
        .collect::<BTreeMap<_, _>>();
    records
        .iter()
        .map(|record| {
            let metadata_item = metadata_by_id.get(record.id.as_str()).copied();
            let tags = metadata_item.map_or_else(Vec::new, |item| item.tags.clone());
            let production = tags
                .iter()
                .any(|tag| tag.eq_ignore_ascii_case("production"));
            let (environment, risk) = if production {
                ("Production".to_owned(), EnvironmentRisk::Production)
            } else {
                ("Unclassified".to_owned(), EnvironmentRisk::Development)
            };
            let source = match record.source {
                SourceKind::OpenSshUser | SourceKind::AutomexiaMetadata => {
                    HubCatalogSource::OpenSshUser
                }
                SourceKind::OpenSshSystem => HubCatalogSource::OpenSshSystem,
            };
            ConnectionCatalogEntry {
                summary: ConnectionSummary {
                    id: record.id.clone(),
                    display_name: metadata_item
                        .and_then(|item| item.display_name.clone())
                        .unwrap_or_else(|| record.alias.clone()),
                    provider: ProviderKind::Ssh,
                    target: public_target(record),
                    identity: identity_label(record.identity_hint).to_owned(),
                    environment,
                    risk,
                    auth_state: AuthState::Unknown,
                    favorite: metadata_item.is_some_and(|item| item.favorite),
                },
                tags,
                source,
                source_revision: metadata.revision.max(generation),
                last_used_at_ms: metadata_item.and_then(|item| item.last_used_at_ms),
            }
        })
        .collect()
}

fn public_target(record: &ConnectionRecord) -> String {
    let host = record.hostname.as_deref().unwrap_or(&record.alias);
    let mut target = record
        .username
        .as_ref()
        .map_or_else(|| host.to_owned(), |user| format!("{user}@{host}"));
    if let Some(port) = record.port {
        target.push(':');
        target.push_str(&port.to_string());
    }
    target
}

const fn identity_label(hint: IdentityHint) -> &'static str {
    match hint {
        IdentityHint::AgentOrDefault => "OpenSSH agent or default identity",
        IdentityHint::FileReferencePresent => "OpenSSH identity file reference",
        IdentityHint::CertificateReferencePresent => "OpenSSH certificate reference",
        IdentityHint::HardwareOrProviderReferencePresent => {
            "OpenSSH hardware or provider reference"
        }
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|error| error.into_inner())
}
