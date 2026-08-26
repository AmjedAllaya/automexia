//! Application-owned Quick Action persistence, refresh, and CP4 publication.
//!
//! The typed model remains capability-free in `automexia-command-productivity::actions`.
//! This boundary owns the explicit user-private `actions/` directory, bounded
//! atomic storage, immutable last-known-good action/provider snapshots, and an
//! exact parent-directory watcher. CP4 composes already refreshed public
//! provider capsules; it has no renderer, input, VT, PTY, provider discovery,
//! network, shell-profile, clipboard, secret-store, or execution authority.

mod aliases;
mod aliases_cli;
mod cli;
mod native_import;
mod packs_cli;
#[cfg(not(target_arch = "wasm32"))]
mod providers;
mod refresh;
pub(crate) mod secure_fs;
mod service;
mod store;
mod transfer;
mod worker;
mod workspace;

pub use aliases_cli::execute_aliases_command;
pub use cli::execute_actions_command;
pub use packs_cli::execute_packs_command;

#[cfg(not(target_arch = "wasm32"))]
pub use providers::{
    compose_provider_action_snapshot, ProviderActionCompositionError,
    ProviderActionCompositionErrorCode, ProviderActionPublicationOutcome,
    ProviderActionPublisher, ProviderActionRouteError, ProviderActionRouteErrorCode,
};

pub use native_import::{
    apply_native_alias_import, preview_native_alias_import_file,
    AppliedNativeAliasImport, NativeAliasImportFilePreview, NativeAliasImportSelection,
    NativeImportError,
};

pub use aliases::{
    collect_local_alias_observations, projection_file_name, shell_label,
    AliasDoctorReport, AliasError, AliasErrorCode, AliasHealth, AliasObservationSet,
    AliasProjectionPlan, AliasProjectionStore, AliasPublication, AliasRecovery,
    AliasShellObservations, GenerationExpectation, PreparedAliasPublication,
    ALIAS_CURRENT_FILE, ALIAS_GENERATIONS_DIRECTORY, ALIAS_LOCK_FILE,
    ALIAS_MANIFEST_FILE, ALIAS_MANIFEST_SCHEMA, ALIAS_PREVIOUS_FILE, ALIAS_ROOT_NAME,
    ALIAS_TRANSACTION_FILE, DISABLED_POINTER, MAX_ALIAS_MANIFEST_BYTES,
    MAX_ALIAS_TRANSACTION_BYTES, MAX_GENERATION_DIRECTORY_ENTRIES,
};

pub use refresh::{
    ExactActionWatchPlan, QuickActionMonitor, RefreshCoordinator, RefreshDecision,
    WatchErrorCode, MAX_WATCH_EVENTS_PER_POLL, WATCH_EVENT_CAPACITY,
};
pub use service::{QuickActionService, RefreshOutcome, ServiceStatus};
pub use store::{
    LoadOrigin, LoadResult, QuickActionSnapshot, QuickActionStore, StoreError,
    StoreErrorCode, ACTIONS_FILE_NAME, LOCK_FILE_NAME, MAX_CACHED_ACTION_BYTES,
    PREVIOUS_ACTIONS_FILE_NAME,
};
pub use transfer::{
    apply_import, export_to_path, preview_export, preview_import, ExportPreview,
    ImportPreview, QuickActionTransferDocument, TransferConflictPolicy, TransferError,
    QUICK_ACTION_TRANSFER_SCHEMA,
};
pub use worker::{
    QuickActionRuntime, QuickActionRuntimeErrorCode, QuickActionRuntimeStatus,
    QuickActionSearchResult, SearchSubmission,
};
pub use workspace::{
    WorkspaceActionSnapshot, WorkspaceActionStore, WorkspaceError, WorkspaceErrorCode,
    WorkspaceTaskBridgeInput, WorkspaceTrustSnapshot, WorkspaceTrustStore,
    MAX_WORKSPACE_TRUST_RECEIPTS, WORKSPACE_ACTION_DIRECTORY_NAME,
    WORKSPACE_ACTION_FILE_NAME, WORKSPACE_TRUST_FILE_NAME,
    WORKSPACE_TRUST_SCHEMA_VERSION,
};
