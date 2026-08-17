//! Application-owned CP2.1 Quick Action persistence and refresh services.
//!
//! The typed model remains capability-free in `automexia-devops::actions`.
//! This boundary owns only the explicit user-private `actions/` directory,
//! bounded atomic storage, immutable last-known-good snapshots, and an exact
//! parent-directory watcher. It has no renderer, input, VT, PTY, provider,
//! network, shell-profile, clipboard, secret-store, or execution authority.

mod aliases;
mod aliases_cli;
mod cli;
mod refresh;
mod secure_fs;
mod service;
mod store;
mod transfer;
mod worker;

pub use aliases_cli::execute_aliases_command;
pub use cli::execute_actions_command;

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
