//! Capability-free D7/CP6 domain contracts.
//!
//! This crate performs no filesystem, process, network, provider, credential,
//! PTY, renderer, clipboard, or model I/O. The product release gate remains
//! disabled; callers must not interpret these types as activation authority.

mod capability;
mod distribution;
mod lifecycle;
mod manifest;
mod model_suggestion;
mod package;
mod strict_json;
mod ui;

pub use capability::*;
pub use distribution::*;
pub use lifecycle::*;
pub use manifest::*;
pub use model_suggestion::*;
pub use package::*;
pub use strict_json::*;
pub use ui::*;

/// Exact accepted schema-1 contract digest. The accepted artifact is immutable;
/// acceptance and nonactivation are recorded in a separate receipt.
pub const ACCEPTED_CONTRACT_SHA256: &str =
    "fdd765ec52cf043ebbf2132177a94ae8be5bcb8175d4f1b566bcac4b96b346b4";

pub struct Limits;

impl Limits {
    pub const BUNDLE_BYTES: usize = 16 * 1024 * 1024;
    pub const EXPANDED_BYTES: usize = 32 * 1024 * 1024;
    pub const PACKAGE_FILES: usize = 32;
    pub const PATH_BYTES: usize = 512;
    pub const MANIFEST_BYTES: usize = 64 * 1024;
    pub const MANIFEST_DEPTH: usize = 16;
    pub const MANIFEST_STRING_BYTES: usize = 4 * 1024;
    pub const COMPONENT_BYTES: usize = 8 * 1024 * 1024;
    pub const WIT_IMPORTS: usize = 64;
    pub const CAPABILITY_REQUESTS: usize = 32;
    pub const LINEAR_MEMORY_BYTES: usize = 64 * 1024 * 1024;
    pub const TABLE_ELEMENTS: usize = 100_000;
    pub const INSTANCES_PER_EXTENSION: usize = 2;
    pub const FUEL_PER_CALL: u64 = 10_000_000;
    pub const HOST_TRANSFER_BYTES: usize = 1024 * 1024;
    pub const INTERACTIVE_DEADLINE_MS: u64 = 250;
    pub const EXPLICIT_DEADLINE_MS: u64 = 2_000;
    pub const OUTPUT_BYTES: usize = 1024 * 1024;
    pub const LOG_BYTES: usize = 64 * 1024;
    pub const QUEUED_CALLS_PER_EXTENSION: usize = 16;
    pub const CONCURRENT_CALLS_PER_EXTENSION: usize = 2;
    pub const CONCURRENT_CALLS_GLOBAL: usize = 8;
    pub const CRASHES_PER_FIVE_MINUTES: usize = 3;
    pub const SELECTED_MODEL_INPUT_BYTES: usize = 16 * 1024;
    pub const MODEL_RESPONSE_BYTES: usize = 64 * 1024;
    pub const CACHE_BYTES: usize = 256 * 1024 * 1024;
    pub const RETAINED_VERSIONS: usize = 2;
    pub const INSTALLED_EXTENSIONS: usize = 128;
}
