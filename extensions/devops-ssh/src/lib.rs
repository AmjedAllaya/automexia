//! Bounded, non-executing OpenSSH inventory for Automexia.
//!
//! This crate intentionally does not implement SSH, resolve effective OpenSSH
//! configuration, spawn processes, or open sockets. It indexes a conservative
//! static subset from explicitly granted files and stores only public metadata.

mod inventory;
mod model;
mod persistence;
mod refresh;

use automexia_extension_api::{Capability, ExtensionManifest};

pub use inventory::{
    scan_inventory, Diagnostic, GrantKind, InventoryError, InventoryGrant,
    InventoryLimits, ScanOutcome,
};
pub use model::{
    ConnectionMetadata, ConnectionRecord, IdentityHint, InventorySnapshot,
    MetadataDocument, SourceKind, SCHEMA_VERSION,
};
pub use persistence::{MetadataStore, CONNECTIONS_FILE_NAME};
pub use refresh::{
    recommended_watcher, RefreshCoordinator, RefreshDecision, RefreshStatus, WatchPlan,
};

pub const ID: &str = "automexia.devops-ssh";
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The package can read only paths granted by the host. Extension-owned state
/// is written under the exact root provided by the host and is not a general
/// filesystem capability.
pub const MANIFEST: ExtensionManifest = ExtensionManifest {
    id: ID,
    name: "Automexia OpenSSH Inventory",
    description: "Bounded static OpenSSH aliases and private Automexia metadata.",
    version: VERSION,
    default_enabled: false,
    capabilities: &[Capability::FilesystemRead],
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_is_nonactivated_and_least_privilege() {
        let manifest = std::hint::black_box(MANIFEST);
        assert!(!manifest.default_enabled);
        assert_eq!(manifest.capabilities, &[Capability::FilesystemRead]);
    }
}
