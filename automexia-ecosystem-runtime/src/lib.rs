//! D7 adapters for explicit local packages and the disabled component host.

mod action_pack;
mod package;
mod store;

#[cfg(feature = "component-host")]
mod sandbox;

pub use action_pack::*;
pub use package::*;
pub use store::*;

#[cfg(feature = "component-host")]
pub use sandbox::*;

/// Source implementation exists, but product activation remains denied until
/// the protected release gates recorded by ADR 0029 are satisfied.
pub const RUNTIME_ACTIVATION_AUTHORIZED: bool = false;
