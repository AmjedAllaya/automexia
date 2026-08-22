//! Application-owned composition for the non-executing Connection Hub.

mod controller;
mod direct_openssh;
mod library;
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

pub use runtime::{
    platform_setup_guidance, ConnectionHubRuntime, GrantReviewState, HubLibrarySnapshot,
    HubMetadataChangeState, HubMetadataValues, HubRuntimeErrorCode, HubRuntimeSnapshot,
    HubRuntimeState, HubStoreState, MetadataChangeReview, PlatformFamily,
    ReviewedGrantFile, SetupGuidance,
};
