//! Non-executing SSH enhancement plans. A plan is NOT permission to launch.
//!
//! The application retains executable identity, authorization, PTY, process,
//! credential and GUI ownership. This crate neither implements SSH nor performs
//! filesystem, environment, network, process, clock, or terminal I/O.
//! Bootstrap candidates need a NEW review of their complete argument vector;
//! they must never be appended to an existing native-SSH launch binding.
#![forbid(unsafe_code)]

pub mod bootstrap;
pub mod invocation;
pub mod plan;
pub mod session;

pub use invocation::{Invocation, InvocationClass, PassReason};
pub use plan::{
    plan, Candidate, ConfigEvidence, Decision, Mode, Options, RemoteShell, Startup,
};
pub use session::{Capabilities, GenerationKey, Negotiation, Phase, RemotePath};

/// Independently versioned wire format, not the product release version.
pub const PROTOCOL_VERSION: u32 = 1;
pub const MAX_ARGUMENTS: usize = 128;
pub const MAX_ARGUMENT_BYTES: usize = 16 * 1024;
// The reviewed connection model remains the destination-budget owner.
pub use automexia_connectivity::connections::MAX_DIRECT_OPENSSH_DESTINATION_BYTES as MAX_DESTINATION_BYTES;
pub const MAX_BOOTSTRAP_BYTES: usize = 12 * 1024;
pub const MAX_FRAME_BYTES: usize = 128;
pub const MAX_REMOTE_PATH_BYTES: usize = 4096;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    ArgumentLimit,
    InvalidArgument,
    InvalidGeneration,
    UnsupportedShell,
    InvalidFrame,
    InvalidPath,
    ClockRegression,
    DeadlineOverflow,
    BootstrapLimit,
    RequiredUnavailable,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Do not put hosts, paths, argv or remote-supplied strings in errors.
        f.write_str(match self {
            Self::ArgumentLimit => "SSH integration argument budget exceeded",
            Self::InvalidArgument => {
                "SSH integration argument cannot be represented safely"
            }
            Self::InvalidGeneration => "invalid SSH session generation",
            Self::UnsupportedShell => "this shell/startup adapter is not implemented",
            Self::InvalidFrame => "invalid SSH integration receipt",
            Self::InvalidPath => "invalid bounded remote path",
            Self::ClockRegression => "monotonic time moved backwards",
            Self::DeadlineOverflow => "invalid negotiation deadline",
            Self::BootstrapLimit => "SSH bootstrap exceeds its fixed budget",
            Self::RequiredUnavailable => "required SSH integration is unavailable",
        })
    }
}
impl std::error::Error for Error {}
