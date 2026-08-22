//! Application-owned composition for the non-executing Connection Hub.

mod controller;
mod direct_openssh;
mod library;
mod private_fs;
mod receipts;
mod runtime;
pub use controller::{
    ConnectionHubController, DisabledHubAction, HubControllerEffect,
    HubControllerPresentation,
};
pub use library::{
    ConnectionLibraryDocument, ConnectionLibraryStore, HubPreferences, LibraryError,
    LibraryErrorCode, LibraryLoadOrigin, LibraryLoadResult, LibraryTransferDocument,
    CONNECTION_LIBRARY_FILE, CONNECTION_LIBRARY_LOCK_FILE,
    CONNECTION_LIBRARY_PREVIOUS_FILE, CONNECTION_LIBRARY_SCHEMA,
    MAX_CONNECTION_LIBRARY_BYTES,
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
