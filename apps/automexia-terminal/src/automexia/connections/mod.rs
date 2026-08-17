//! Application-owned composition for the non-executing Connection Hub.

mod runtime;

pub use runtime::{
    platform_setup_guidance, ConnectionHubRuntime, HubRuntimeState, PlatformFamily,
    SetupGuidance,
};
