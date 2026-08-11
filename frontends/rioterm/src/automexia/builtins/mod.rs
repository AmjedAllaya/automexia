use super::api::ExtensionManifest;

pub mod devops;

/// Trusted first-party catalog compiled into the application.
///
/// Keeping registration here prevents the marketplace UI/model from depending
/// on concrete extension implementations. A future package registry can replace
/// this static slice without changing renderer or terminal-engine contracts.
#[cfg(not(target_arch = "wasm32"))]
pub const MANIFESTS: &[ExtensionManifest] = &[devops::MANIFEST];
#[cfg(target_arch = "wasm32")]
pub const MANIFESTS: &[ExtensionManifest] = &[];
