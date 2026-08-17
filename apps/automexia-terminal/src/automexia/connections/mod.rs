//! Application-owned composition for the non-executing Connection Hub.

mod library;
mod runtime;
pub use library::{
    ConnectionLibraryDocument, ConnectionLibraryStore, HubPreferences, LibraryError,
    LibraryErrorCode, LibraryLoadOrigin, LibraryLoadResult, LibraryTransferDocument,
    CONNECTION_LIBRARY_FILE, CONNECTION_LIBRARY_LOCK_FILE,
    CONNECTION_LIBRARY_PREVIOUS_FILE, CONNECTION_LIBRARY_SCHEMA,
    MAX_CONNECTION_LIBRARY_BYTES,
};

pub use runtime::{
    platform_setup_guidance, ConnectionHubRuntime, HubRuntimeState, PlatformFamily,
    SetupGuidance,
};
