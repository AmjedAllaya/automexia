use std::{
    collections::{BTreeMap, VecDeque},
    fmt,
    path::{Path, PathBuf},
    sync::{mpsc, Arc, Condvar, Mutex, MutexGuard, Weak},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use automexia_devops::connections::{
    AuthState, DirectOpenSshPreparation, EnvironmentRisk, ProviderKind,
};
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

use super::direct_openssh::{
    prepare_inventory_direct_openssh, stable_profile_id, InventoryPreparationError,
};
use super::library::{
    ConnectionLibraryDocument, ConnectionLibraryStore, HubPreferences, LibraryLoadOrigin,
};
use super::receipts::{
    ManagedReceiptDocument, ManagedReceiptLoadOrigin, ManagedReceiptPersistenceState,
    ManagedReceiptRecord, ManagedReceiptSink, ManagedReceiptStore, MAX_MANAGED_RECEIPTS,
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
const WORKER_BUSY_DIAGNOSTIC: &str = "connection-worker-busy";
const WORK_QUEUE_CAPACITY: usize = 2;
const STORE_DIAGNOSTIC: &str = "connection-private-store-unavailable";
const METADATA_DIAGNOSTIC: &str = "connection-metadata-write-failed";
const RECEIPT_STORE_DIAGNOSTIC: &str = "connection-receipt-store-unavailable";
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
    WorkerBusy,
    StoreUnavailable,
    InvalidSelection,
    StaleReview,
    UnknownConnection,
    InvalidMetadata,
    StaleMetadata,
    ConnectionNotReady,
    UnsupportedConnectionRoute,
    InvalidConnection,
    StaleReconnect,
}

impl HubRuntimeErrorCode {
    pub const fn diagnostic_code(self) -> &'static str {
        match self {
            Self::WorkerUnavailable => "connection-worker-unavailable",
            Self::WorkerBusy => WORKER_BUSY_DIAGNOSTIC,
            Self::StoreUnavailable => "connection-private-store-unavailable",
            Self::InvalidSelection => "connection-selection-invalid",
            Self::StaleReview => "connection-selection-review-stale",
            Self::UnknownConnection => "connection-record-unavailable",
            Self::InvalidMetadata => "connection-metadata-invalid",
            Self::StaleMetadata => "connection-metadata-review-stale",
            Self::ConnectionNotReady => "connection-review-not-ready",
            Self::UnsupportedConnectionRoute => "connection-route-not-supported",
            Self::InvalidConnection => "connection-review-invalid",
            Self::StaleReconnect => "connection-reconnect-stale",
        }
    }
}

impl fmt::Display for HubRuntimeErrorCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.diagnostic_code())
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
    pub receipt_store_state: HubStoreState,
    pub receipt_count: usize,
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
    FlushReceipts,
    #[cfg(test)]
    TestBarrier {
        started: mpsc::Sender<()>,
        release: Arc<(Mutex<bool>, Condvar)>,
    },
}

struct WorkRequest {
    request: u64,
    work: Work,
    wake: Option<CompletionWake>,
}

struct WorkerStores {
    metadata: MetadataStore,
    _library: Option<ConnectionLibraryStore>,
    receipts: Option<ManagedReceiptStore>,
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
    receipt_store_state: HubStoreState,
    receipt_records: Arc<Vec<ManagedReceiptRecord>>,
    pending_receipts: VecDeque<ManagedReceiptRecord>,
    library: HubLibrarySnapshot,
}

struct RuntimeInner {
    data: Mutex<RuntimeData>,
    settled: Condvar,
    sender: Mutex<Option<mpsc::SyncSender<WorkRequest>>>,
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
        let (sender, receiver) = mpsc::sync_channel(WORK_QUEUE_CAPACITY);
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
                receipt_store_state: HubStoreState::Initializing,
                receipt_records: Arc::new(Vec::new()),
                pending_receipts: VecDeque::with_capacity(MAX_MANAGED_RECEIPTS),
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
                    receipt_store_state: HubStoreState::Unavailable {
                        diagnostic_code: RECEIPT_STORE_DIAGNOSTIC,
                    },
                    receipt_records: Arc::new(Vec::new()),
                    pending_receipts: VecDeque::with_capacity(MAX_MANAGED_RECEIPTS),
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
        sender.try_send(request).map_err(|error| match error {
            mpsc::TrySendError::Full(_) => HubRuntimeErrorCode::WorkerBusy,
            mpsc::TrySendError::Disconnected(_) => HubRuntimeErrorCode::WorkerUnavailable,
        })
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
        if let Err(error) = self.enqueue(WorkRequest {
            request,
            work: Work::Scan {
                grants,
                cancellation,
            },
            wake: None,
        }) {
            complete_scan_failure_with(&self.inner, request, error.diagnostic_code());
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
            complete_review_error(&self.inner, request, error.diagnostic_code());
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
            complete_scan_failure_with(&self.inner, request, error.diagnostic_code());
            return Err(error);
        }
        Ok(request)
    }

    pub fn discard_review(&self, reviewed_request: u64) -> bool {
        let mut data = lock(&self.inner.data);
        let owns_review = matches!(
            data.grant_review,
            GrantReviewState::Reviewing { request }
                | GrantReviewState::Ready { request, .. }
                | GrantReviewState::Error { request, .. }
                if request == reviewed_request
        );
        if data.shutdown || !owns_review {
            return false;
        }
        data.requested = data.requested.saturating_add(1);
        data.completed = data.requested;
        data.reviewed_grants = None;
        data.grant_review = GrantReviewState::None;
        self.inner.settled.notify_all();
        true
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
            complete_metadata_error(&self.inner, request, error.diagnostic_code());
            return Err(error);
        }
        Ok(request)
    }

    /// Compose one current inventory record into a pure pending F2 plan.
    /// The runtime lock is held only while cloning bounded in-memory inputs;
    /// composition performs no I/O and creates no runtime authority.
    pub fn prepare_direct_openssh(
        &self,
        connection_id: &str,
    ) -> Result<DirectOpenSshPreparation, HubRuntimeErrorCode> {
        let (record, metadata, generation, metadata_revision) = {
            let data = lock(&self.inner.data);
            let generation = match data.state {
                HubRuntimeState::Ready { generation }
                    if generation > 0 && generation == data.successful_generation =>
                {
                    generation
                }
                _ => return Err(HubRuntimeErrorCode::ConnectionNotReady),
            };
            let record = data
                .records
                .iter()
                .find(|record| record.id == connection_id)
                .cloned()
                .ok_or(HubRuntimeErrorCode::UnknownConnection)?;
            let metadata = data
                .metadata
                .connections
                .iter()
                .find(|item| item.connection_id == connection_id)
                .cloned();
            (record, metadata, generation, data.metadata.revision)
        };
        prepare_inventory_direct_openssh(
            &record,
            metadata.as_ref(),
            generation,
            metadata_revision,
        )
        .map_err(|error| match error {
            InventoryPreparationError::UnsupportedRoute => {
                HubRuntimeErrorCode::UnsupportedConnectionRoute
            }
            InventoryPreparationError::InvalidRecord
            | InventoryPreparationError::InvalidModel => {
                HubRuntimeErrorCode::InvalidConnection
            }
        })
    }

    /// Rebuild a reconnect from the current inventory and reject any receipt
    /// whose opaque profile identity or source revision no longer matches.
    /// The returned preparation still requires a new executable observation,
    /// host-trust observation, review, and explicit approval.
    pub fn prepare_managed_reconnect(
        &self,
        receipt: &ManagedReceiptRecord,
    ) -> Result<DirectOpenSshPreparation, HubRuntimeErrorCode> {
        let (public_connection_id, source_revision) = receipt
            .reconnect_identity()
            .ok_or(HubRuntimeErrorCode::UnsupportedConnectionRoute)?;
        let (record, metadata, generation, metadata_revision) = {
            let data = lock(&self.inner.data);
            let generation = match data.state {
                HubRuntimeState::Ready { generation }
                    if generation > 0 && generation == data.successful_generation =>
                {
                    generation
                }
                _ => return Err(HubRuntimeErrorCode::ConnectionNotReady),
            };
            let record = data
                .records
                .iter()
                .find(|record| stable_profile_id(record) == public_connection_id)
                .cloned()
                .ok_or(HubRuntimeErrorCode::StaleReconnect)?;
            let metadata = data
                .metadata
                .connections
                .iter()
                .find(|item| item.connection_id == record.id)
                .cloned();
            (record, metadata, generation, data.metadata.revision)
        };
        let preparation = prepare_inventory_direct_openssh(
            &record,
            metadata.as_ref(),
            generation,
            metadata_revision,
        )
        .map_err(|_| HubRuntimeErrorCode::InvalidConnection)?;
        if preparation.profile().source.revision != source_revision {
            return Err(HubRuntimeErrorCode::StaleReconnect);
        }
        Ok(preparation)
    }
    pub fn managed_receipt_records(&self) -> Arc<Vec<ManagedReceiptRecord>> {
        Arc::clone(&lock(&self.inner.data).receipt_records)
    }

    pub fn wait_for_managed_receipts(
        &self,
        minimum_count: usize,
        timeout: Duration,
    ) -> bool {
        let deadline = Instant::now() + timeout;
        let mut data = lock(&self.inner.data);
        loop {
            if data.receipt_records.len() >= minimum_count {
                return true;
            }
            if matches!(data.receipt_store_state, HubStoreState::Unavailable { .. }) {
                return false;
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
            if result.timed_out() && data.receipt_records.len() < minimum_count {
                return false;
            }
        }
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
            receipt_store_state: data.receipt_store_state,
            receipt_count: data.receipt_records.len(),
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

impl ManagedReceiptSink for ConnectionHubRuntime {
    fn try_persist(
        &self,
        record: ManagedReceiptRecord,
    ) -> ManagedReceiptPersistenceState {
        let sender = match lock(&self.inner.sender).as_ref().cloned() {
            Some(sender) => sender,
            None => return ManagedReceiptPersistenceState::Unavailable,
        };
        let request = {
            let mut data = lock(&self.inner.data);
            if data.shutdown
                || matches!(data.receipt_store_state, HubStoreState::Unavailable { .. })
            {
                return ManagedReceiptPersistenceState::Unavailable;
            }
            if data.pending_receipts.len() == MAX_MANAGED_RECEIPTS {
                data.pending_receipts.pop_front();
            }
            data.pending_receipts.push_back(record);
            data.requested
        };
        match sender.try_send(WorkRequest {
            request,
            work: Work::FlushReceipts,
            wake: None,
        }) {
            Ok(()) | Err(mpsc::TrySendError::Full(_)) => {
                ManagedReceiptPersistenceState::Queued
            }
            Err(mpsc::TrySendError::Disconnected(_)) => {
                let mut data = lock(&self.inner.data);
                data.pending_receipts.clear();
                data.receipt_store_state = HubStoreState::Unavailable {
                    diagnostic_code: RECEIPT_STORE_DIAGNOSTIC,
                };
                ManagedReceiptPersistenceState::Unavailable
            }
        }
    }
}

fn worker_loop(inner: Weak<RuntimeInner>, receiver: mpsc::Receiver<WorkRequest>) {
    let mut stores = None;
    while let Ok(work_request) = receiver.recv() {
        let Some(runtime) = inner.upgrade() else {
            return;
        };
        let WorkRequest {
            request,
            work,
            wake,
        } = work_request;
        let is_initialization = matches!(&work, Work::Initialize(_));
        let is_receipt_flush = matches!(&work, Work::FlushReceipts);
        if !is_initialization
            && !is_receipt_flush
            && !is_current_request(&runtime, request)
        {
            let _ = flush_pending_receipts(&runtime, stores.as_ref());
            if lock(&runtime.data).shutdown {
                return;
            }
            continue;
        }
        let published = match work {
            Work::Initialize(startup) => {
                complete_initialization(&runtime, request, startup, &mut stores)
            }
            Work::ReviewFiles { paths, kind } => {
                complete_file_review(&runtime, request, paths, kind, stores.is_some())
            }
            Work::Scan {
                grants,
                cancellation,
            } => complete_scan(&runtime, request, grants, cancellation, stores.is_some()),
            Work::ApplyMetadata(review) => {
                complete_metadata_change(&runtime, request, review, stores.as_ref())
            }
            Work::FlushReceipts => flush_pending_receipts(&runtime, stores.as_ref()),
            #[cfg(test)]
            Work::TestBarrier { started, release } => {
                let _ = started.send(());
                let (released, ready) = &*release;
                let mut released = lock(released);
                while !*released {
                    released = ready
                        .wait(released)
                        .unwrap_or_else(|error| error.into_inner());
                }
                true
            }
        };
        if !is_receipt_flush {
            let _ = flush_pending_receipts(&runtime, stores.as_ref());
        }
        if published {
            if let Some(wake) = wake {
                wake.wake();
            }
        }
        if lock(&runtime.data).shutdown {
            return;
        }
    }
    if let Some(runtime) = inner.upgrade() {
        let _ = flush_pending_receipts(&runtime, stores.as_ref());
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
        Ok((
            worker_stores,
            metadata,
            library,
            recovered,
            receipt_document,
            receipt_store_state,
        )) => {
            data.metadata = metadata;
            data.library = library;
            data.store_state = if recovered {
                HubStoreState::Recovered
            } else {
                HubStoreState::Ready
            };
            data.receipt_records = Arc::new(receipt_document.records);
            data.receipt_store_state = receipt_store_state;
            if matches!(data.state, HubRuntimeState::Initializing) {
                data.state = HubRuntimeState::InitialSetup;
            }
            *stores = Some(worker_stores);
        }
        Err(()) => {
            data.store_state = HubStoreState::Unavailable {
                diagnostic_code: STORE_DIAGNOSTIC,
            };
            data.receipt_store_state = HubStoreState::Unavailable {
                diagnostic_code: RECEIPT_STORE_DIAGNOSTIC,
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
) -> Result<
    (
        WorkerStores,
        MetadataDocument,
        HubLibrarySnapshot,
        bool,
        ManagedReceiptDocument,
        HubStoreState,
    ),
    (),
> {
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
                    receipts: None,
                },
                metadata.document,
                HubLibrarySnapshot::default(),
                recovered,
                ManagedReceiptDocument::default(),
                HubStoreState::Unavailable {
                    diagnostic_code: RECEIPT_STORE_DIAGNOSTIC,
                },
            ))
        }
        Startup::Root(root) => {
            let metadata_store =
                MetadataStore::new(root.join("extensions").join("devops-ssh"))
                    .map_err(|_| ())?;
            let metadata = metadata_store.load_with_recovery().map_err(|_| ())?;
            let connections_root = root.join("connections");
            let library_store =
                ConnectionLibraryStore::open(&connections_root).map_err(|_| ())?;
            let library = library_store.load().map_err(|_| ())?;
            let recovered = metadata.origin == MetadataLoadOrigin::PreviousRecovery
                || metadata.rejected_primary
                || library.origin == LibraryLoadOrigin::PreviousRecovery
                || library.rejected_primary;
            let snapshot =
                HubLibrarySnapshot::from_document(&library.document, recovered);
            let (receipt_store, receipt_document, receipt_store_state) =
                match ManagedReceiptStore::open(&connections_root)
                    .and_then(|store| store.load().map(|loaded| (store, loaded)))
                {
                    Ok((store, loaded)) => {
                        let state = if loaded.origin
                            == ManagedReceiptLoadOrigin::PreviousRecovery
                            || loaded.rejected_primary
                        {
                            HubStoreState::Recovered
                        } else {
                            HubStoreState::Ready
                        };
                        (Some(store), loaded.document, state)
                    }
                    Err(_) => (
                        None,
                        ManagedReceiptDocument::default(),
                        HubStoreState::Unavailable {
                            diagnostic_code: RECEIPT_STORE_DIAGNOSTIC,
                        },
                    ),
                };
            Ok((
                WorkerStores {
                    metadata: metadata_store,
                    _library: Some(library_store),
                    receipts: receipt_store,
                },
                metadata.document,
                snapshot,
                recovered,
                receipt_document,
                receipt_store_state,
            ))
        }
    }
}

fn flush_pending_receipts(inner: &RuntimeInner, stores: Option<&WorkerStores>) -> bool {
    let pending = {
        let mut data = lock(&inner.data);
        if data.pending_receipts.is_empty() {
            return true;
        }
        data.pending_receipts.drain(..).collect::<Vec<_>>()
    };
    let Some(store) = stores.and_then(|stores| stores.receipts.as_ref()) else {
        let mut data = lock(&inner.data);
        data.receipt_store_state = HubStoreState::Unavailable {
            diagnostic_code: RECEIPT_STORE_DIAGNOSTIC,
        };
        inner.settled.notify_all();
        return false;
    };
    match store.append_batch(&pending) {
        Ok(document) => {
            let mut data = lock(&inner.data);
            data.receipt_records = Arc::new(document.records);
            data.receipt_store_state = HubStoreState::Ready;
            inner.settled.notify_all();
            true
        }
        Err(_) => {
            let mut data = lock(&inner.data);
            data.receipt_store_state = HubStoreState::Unavailable {
                diagnostic_code: RECEIPT_STORE_DIAGNOSTIC,
            };
            inner.settled.notify_all();
            false
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

fn complete_review_error(
    inner: &RuntimeInner,
    request: u64,
    diagnostic_code: &'static str,
) -> bool {
    complete_review_failure(inner, request, diagnostic_code)
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

fn complete_metadata_error(
    inner: &RuntimeInner,
    request: u64,
    diagnostic_code: &'static str,
) -> bool {
    complete_metadata_failure(inner, request, diagnostic_code)
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

#[cfg(test)]
mod tests {
    use super::*;
    use automexia_devops::connections::{
        ConnectionReceipt, OpaqueReference, OperationResultState,
        CONNECTION_SCHEMA_VERSION,
    };

    fn managed_receipt(index: usize) -> ManagedReceiptRecord {
        ManagedReceiptRecord::new(
            ConnectionReceipt {
                schema_version: CONNECTION_SCHEMA_VERSION,
                operation_id: format!("operation-{index}"),
                session_id: format!("session-{index}"),
                capsule_id: format!("capsule-{index}"),
                approved_intent_digest: format!("{index:064x}"),
                source_revision: format!("source-{index}"),
                process_ownership_references: vec![OpaqueReference::new(format!(
                    "process-{index}"
                ))],
                route_ownership_references: vec![OpaqueReference::new(format!(
                    "route-{index}"
                ))],
                tunnel_ownership_references: Vec::new(),
                started_at_ms: index as u64,
                outcome: OperationResultState::Succeeded,
            },
            Some((format!("profile-{index}"), format!("source-{index}"))),
            index as u64 + 1,
        )
        .unwrap()
    }

    #[test]
    fn saturated_work_queue_rejects_immediately_and_remains_bounded() {
        let temporary = tempfile::tempdir().unwrap();
        let runtime = ConnectionHubRuntime::open_at_root(temporary.path());
        assert!(runtime.wait_for_settled(Duration::from_secs(5)));

        let request = {
            let mut data = lock(&runtime.inner.data);
            data.requested = data.requested.saturating_add(1);
            data.requested
        };
        let (started, observed_start) = mpsc::channel();
        let release = Arc::new((Mutex::new(false), Condvar::new()));
        let work = || WorkRequest {
            request,
            work: Work::TestBarrier {
                started: started.clone(),
                release: Arc::clone(&release),
            },
            wake: None,
        };

        assert_eq!(runtime.enqueue(work()), Ok(()));
        observed_start
            .recv_timeout(Duration::from_secs(2))
            .expect("worker must enter the deterministic barrier");
        for _ in 0..WORK_QUEUE_CAPACITY {
            assert_eq!(runtime.enqueue(work()), Ok(()));
        }
        assert_eq!(
            runtime.enqueue(work()),
            Err(HubRuntimeErrorCode::WorkerBusy)
        );
        assert_eq!(
            runtime.try_persist(managed_receipt(1)),
            ManagedReceiptPersistenceState::Queued
        );

        let (released, ready) = &*release;
        *lock(released) = true;
        ready.notify_all();
        runtime.shutdown();
        assert_eq!(runtime.state(), HubRuntimeState::Shutdown);
        let persisted = ManagedReceiptStore::open(temporary.path().join("connections"))
            .unwrap()
            .load()
            .unwrap();
        assert_eq!(persisted.document.records.len(), 1);
    }

    #[test]
    fn managed_receipts_survive_restart_with_only_fresh_review_identity() {
        let temporary = tempfile::tempdir().unwrap();
        let runtime = ConnectionHubRuntime::open_at_root(temporary.path());
        assert!(runtime.wait_for_settled(Duration::from_secs(5)));
        assert_eq!(
            runtime.try_persist(managed_receipt(7)),
            ManagedReceiptPersistenceState::Queued
        );
        assert!(runtime.wait_for_managed_receipts(1, Duration::from_secs(5)));
        let records = runtime.managed_receipt_records();
        assert_eq!(records.len(), 1);
        assert_eq!(
            records[0].reconnect_identity(),
            Some(("profile-7", "source-7"))
        );
        runtime.shutdown();

        let restarted = ConnectionHubRuntime::open_at_root(temporary.path());
        assert!(restarted.wait_for_settled(Duration::from_secs(5)));
        let records = restarted.managed_receipt_records();
        assert_eq!(records.len(), 1);
        assert_eq!(
            records[0].reconnect_identity(),
            Some(("profile-7", "source-7"))
        );
        assert_eq!(restarted.snapshot().receipt_count, 1);
        restarted.shutdown();
    }
    #[test]
    fn direct_review_preparation_requires_a_current_exact_runtime_record() {
        let temporary = tempfile::tempdir().unwrap();
        let runtime = ConnectionHubRuntime::open_at_root(temporary.path());
        assert!(runtime.wait_for_settled(Duration::from_secs(5)));
        let record = ConnectionRecord {
            id: "openssh:prod".into(),
            alias: "prod".into(),
            hostname: Some("prod.example.invalid".into()),
            username: None,
            port: None,
            proxy_jump_configured: false,
            identity_hint: IdentityHint::AgentOrDefault,
            source: SourceKind::OpenSshUser,
        };
        {
            let mut data = lock(&runtime.inner.data);
            data.records = Arc::new(vec![record.clone()]);
            data.catalog = Arc::new(compose_catalog(&[record], &data.metadata, 9));
            data.successful_generation = 9;
            data.state = HubRuntimeState::Ready { generation: 9 };
        }

        let prepared = runtime.prepare_direct_openssh("openssh:prod").unwrap();
        assert_eq!(prepared.profile().revision, 9);
        let reconnect_receipt = ManagedReceiptRecord::new(
            ConnectionReceipt {
                schema_version: CONNECTION_SCHEMA_VERSION,
                operation_id: "operation-reconnect".into(),
                session_id: "session-reconnect".into(),
                capsule_id: "capsule-reconnect".into(),
                approved_intent_digest: "d".repeat(64),
                source_revision: prepared.profile().source.revision.clone(),
                process_ownership_references: vec![OpaqueReference::new(
                    "process-reconnect",
                )],
                route_ownership_references: vec![OpaqueReference::new("route-reconnect")],
                tunnel_ownership_references: Vec::new(),
                started_at_ms: 10,
                outcome: OperationResultState::Succeeded,
            },
            Some((
                prepared.profile().id.clone(),
                prepared.profile().source.revision.clone(),
            )),
            20,
        )
        .unwrap();
        assert_eq!(
            runtime
                .prepare_managed_reconnect(&reconnect_receipt)
                .unwrap()
                .profile(),
            prepared.profile()
        );
        lock(&runtime.inner.data).metadata.revision = 1;
        assert_eq!(
            runtime.prepare_managed_reconnect(&reconnect_receipt),
            Err(HubRuntimeErrorCode::StaleReconnect)
        );
        lock(&runtime.inner.data).metadata.revision = 0;
        assert_eq!(
            runtime.prepare_direct_openssh("openssh:missing"),
            Err(HubRuntimeErrorCode::UnknownConnection)
        );
        lock(&runtime.inner.data).state = HubRuntimeState::Stale {
            generation: 9,
            diagnostic_code: REFRESH_DIAGNOSTIC,
            failed_request: 10,
        };
        assert_eq!(
            runtime.prepare_direct_openssh("openssh:prod"),
            Err(HubRuntimeErrorCode::ConnectionNotReady)
        );
    }
}
