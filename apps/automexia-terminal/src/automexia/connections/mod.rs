//! Application-owned composition for the non-executing Connection Hub.

mod controller;
mod direct_openssh;
mod library;
mod provider_transients;
mod providers;
mod receipts;
mod runtime;
mod workspaces;
mod workspaces_cli;
pub use controller::{
    ConnectionHubController, DisabledHubAction, HubControllerEffect,
    HubControllerPresentation,
};
#[doc(hidden)]
pub use direct_openssh::{CurrentDirectOpenSshReview, CurrentDirectOpenSshReviewError};

pub use library::{
    preview_library_edit, ConnectionLibraryDocument, ConnectionLibraryStore,
    HubPreferences, LibraryEdit, LibraryEditPreview, LibraryError, LibraryErrorCode,
    LibraryExportPreview, LibraryImportPreview, LibraryLoadOrigin, LibraryLoadResult,
    LibraryTransferDocument, CONNECTION_LIBRARY_FILE, CONNECTION_LIBRARY_LOCK_FILE,
    CONNECTION_LIBRARY_PREVIOUS_FILE, CONNECTION_LIBRARY_SCHEMA,
    MAX_CONNECTION_LIBRARY_BYTES,
};

pub use provider_transients::{
    ProviderTransientBinding, ProviderTransientError, ProviderTransientErrorCode,
    ProviderTransientHandle, ProviderTransientManager, MAX_PROVIDER_TRANSIENTS,
    MAX_PROVIDER_TRANSIENT_AGE,
};
pub use providers::{
    ProviderProductError, ProviderProductErrorCode, ProviderProductPublication,
    ProviderProductSnapshot, MAX_PRODUCT_PROVIDERS, MAX_PROVIDER_SCOPE_SUMMARY_BYTES,
    PROVIDER_ACTIVATION_BLOCKER,
};
pub use receipts::{
    ManagedReceiptDocument, ManagedReceiptError, ManagedReceiptErrorCode,
    ManagedReceiptLoadOrigin, ManagedReceiptLoadResult, ManagedReceiptPersistenceState,
    ManagedReceiptRecord, ManagedReceiptSink, ManagedReceiptStore, MANAGED_RECEIPT_FILE,
    MANAGED_RECEIPT_LOCK_FILE, MANAGED_RECEIPT_PREVIOUS_FILE, MANAGED_RECEIPT_SCHEMA,
    MAX_MANAGED_RECEIPTS, MAX_MANAGED_RECEIPT_BYTES,
};
pub use runtime::{
    platform_setup_guidance, ConnectionHubRuntime, GrantReviewState, HubLibrarySnapshot,
    HubMetadataChangeState, HubMetadataValues, HubRuntimeErrorCode, HubRuntimeSnapshot,
    HubRuntimeState, HubStoreState, MetadataChangeReview, PlatformFamily,
    ReviewedGrantFile, SetupGuidance,
};
pub use workspaces::{
    m6_activation_readiness, review_library_broadcast, review_library_recipe,
    review_library_workspace_restore, M6ActivationBlocker, M6ActivationReadiness,
    RecipeReviewRequest, WorkspaceProductError, WorkspaceProductErrorCode,
};
pub use workspaces_cli::{execute_workspaces_command, execute_workspaces_command_at};
