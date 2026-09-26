//! Non-executing preparation. Native and enhanced argv require distinct reviews.
use crate::{
    bootstrap, Error, GenerationKey, Invocation, InvocationClass, PassReason,
    MAX_ARGUMENTS, MAX_ARGUMENT_BYTES,
};
use std::ffi::OsString;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Mode {
    #[default]
    Auto,
    Off,
    Required,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RemoteShell {
    #[default]
    Unknown,
    Bash,
    Zsh,
    Fish,
    PowerShell,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Startup {
    #[default]
    Login,
    Interactive,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ConfigEvidence {
    #[default]
    Unknown,
    /// Host-supplied planning fact. Not an authorization capability. The caller
    /// must review effective config/RemoteCommand before live launch is added.
    CompatibleForPreview,
}
#[derive(Clone, Copy, Debug)]
pub struct Options {
    pub mode: Mode,
    pub terminal_input: bool,
    pub terminal_output: bool,
    pub shell: RemoteShell,
    pub startup: Startup,
    pub posix_account_shell: bool,
    pub permit_session_files: bool,
    pub configuration: ConfigEvidence,
    pub policy_denied: bool,
    pub key: GenerationKey,
}
impl Options {
    pub fn conservative(key: GenerationKey) -> Self {
        Self {
            mode: Mode::Auto,
            terminal_input: false,
            terminal_output: false,
            shell: RemoteShell::Unknown,
            startup: Startup::Login,
            posix_account_shell: false,
            permit_session_files: false,
            configuration: ConfigEvidence::Unknown,
            policy_denied: false,
            key,
        }
    }
}
#[derive(Clone, PartialEq, Eq)]
pub struct Candidate {
    original: Invocation,
    arguments: Vec<OsString>,
    key: GenerationKey,
}
impl std::fmt::Debug for Candidate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Candidate")
            .field("key", &self.key)
            .field("argument_count", &self.arguments.len())
            .field("requires_new_review", &true)
            .finish()
    }
}
impl Candidate {
    pub fn original(&self) -> &Invocation {
        &self.original
    }
    /// NOT executable authority. Never append this to an existing LaunchBinding.
    pub fn proposed_arguments_for_new_review(&self) -> &[OsString] {
        &self.arguments
    }
    pub const fn key(&self) -> GenerationKey {
        self.key
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Decision {
    /// Explicit policy denial is never converted to native passthrough.
    Denied,
    Passthrough {
        original: Invocation,
        reason: PassReason,
    },
    Unavailable {
        reason: PassReason,
    },
    Candidate(Candidate),
}
fn fallback(invocation: Invocation, mode: Mode, reason: PassReason) -> Decision {
    if mode == Mode::Required {
        Decision::Unavailable { reason }
    } else {
        Decision::Passthrough {
            original: invocation,
            reason,
        }
    }
}
pub fn plan(invocation: Invocation, options: Options) -> Result<Decision, Error> {
    if options.policy_denied {
        return Ok(Decision::Denied);
    }
    if options.mode == Mode::Off {
        return Ok(fallback(invocation, Mode::Off, PassReason::Disabled));
    }
    if let InvocationClass::Passthrough(reason) =
        invocation.classify(options.terminal_input, options.terminal_output)
    {
        return Ok(fallback(invocation, options.mode, reason));
    }
    if options.configuration != ConfigEvidence::CompatibleForPreview {
        return Ok(fallback(
            invocation,
            options.mode,
            PassReason::UnknownConfiguration,
        ));
    }
    if options.shell != RemoteShell::Bash || !options.posix_account_shell {
        return Ok(fallback(
            invocation,
            options.mode,
            PassReason::UnsupportedShell,
        ));
    }
    // Login-shell semantics and other dialects must not be silently approximated.
    if options.startup != Startup::Interactive {
        return Ok(fallback(
            invocation,
            options.mode,
            PassReason::UnsupportedStartup,
        ));
    }
    if !options.permit_session_files {
        return Ok(fallback(
            invocation,
            options.mode,
            PassReason::RemoteWritesDenied,
        ));
    }
    let command = bootstrap::bash_interactive_candidate(options.key)?;
    let mut arguments = Vec::with_capacity(invocation.arguments().len() + 2);
    arguments.push(OsString::from("-t"));
    arguments.extend_from_slice(invocation.arguments());
    arguments.push(command.into());
    // Bound the fully quoted launch, not only the source payload.
    if arguments.len() > MAX_ARGUMENTS
        || arguments
            .iter()
            .map(|a| a.as_encoded_bytes().len())
            .sum::<usize>()
            > MAX_ARGUMENT_BYTES
    {
        return Err(Error::BootstrapLimit);
    }
    Ok(Decision::Candidate(Candidate {
        original: invocation,
        arguments,
        key: options.key,
    }))
}
