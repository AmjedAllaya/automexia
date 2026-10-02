//! Read-only SSH-integration CLI and the app's reviewed-preparation adapter.
//! No transport activation, shell replacement, SSH process, filesystem probe or
//! remote write happens here. The existing launch broker remains authoritative.
use automexia_ssh_integration as integration;
use clap::{Args, Subcommand, ValueEnum};
use std::ffi::OsString;

#[derive(Args)]
pub struct Command {
    #[command(subcommand)]
    pub action: Action,
}
impl std::fmt::Debug for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SshIntegrationCommand")
            .field("arguments", &"<redacted>")
            .finish()
    }
}
#[derive(Subcommand)]
pub enum Action {
    /// Inspect a fictional/declared launch plan without evaluating SSH config.
    Inspect {
        #[arg(long, value_enum, default_value = "unknown")]
        shell: Shell,
        #[arg(long, value_enum, default_value = "login")]
        startup: Startup,
        #[arg(long, value_enum, default_value = "auto")]
        mode: Mode,
        /// Declare a terminal session for this preview; this is not a live probe.
        #[arg(long)]
        tty: bool,
        /// Declare POSIX account-shell syntax for this preview only.
        #[arg(long)]
        posix_account_shell: bool,
        /// Describe explicit permission for temporary startup files in the plan.
        #[arg(long)]
        permit_session_files: bool,
        /// Simulate compatible config. Does not execute ssh -G or approve a launch.
        #[arg(long)]
        assume_compatible_config: bool,
        /// SSH arguments after --. Values and destination are redacted in output.
        #[arg(last = true)]
        arguments: Vec<OsString>,
    },
    /// Explain the execution gate and the supported planning scope.
    Status,
}
#[derive(Clone, Copy, ValueEnum)]
pub enum Shell {
    Unknown,
    Bash,
    Zsh,
    Fish,
    #[value(name = "powershell")]
    PowerShell,
}
#[derive(Clone, Copy, ValueEnum)]
pub enum Startup {
    Login,
    Interactive,
}
#[derive(Clone, Copy, ValueEnum)]
pub enum Mode {
    Auto,
    Off,
    Required,
}

pub fn report(
    command: &Command,
    native_broker_enabled: bool,
) -> Result<serde_json::Value, integration::Error> {
    let mut result = serde_json::json!({
        "schema": 1,
        "native_broker_enabled": native_broker_enabled,
        "enhanced_execution_enabled": false,
        "reason": "enhanced bootstrap requires a separately reviewed launch contract",
        "evidence": "declared-preview-not-observed",
        "protocol": integration::PROTOCOL_VERSION,
        "supported_bootstrap_candidate": "POSIX account shell, Bash >=5.1, non-login interactive, explicit temporary-file permission",
        "implemented_runtime_capabilities": [],
        "candidate_hook_capabilities": ["prompt-boundaries", "cwd"],
        "remote_command_status": "not implemented",
        "remote_cwd_transport": "scoped user-variable candidate, not connected to live sessions",
        "remote_automexia_prompt": "not implemented"
    });
    if let Action::Inspect {
        shell,
        startup,
        mode,
        tty,
        posix_account_shell,
        permit_session_files,
        assume_compatible_config,
        arguments,
    } = &command.action
    {
        let key = integration::GenerationKey::new(1, 1)?;
        let mut options = integration::Options::conservative(key);
        options.mode = match mode {
            Mode::Auto => integration::Mode::Auto,
            Mode::Off => integration::Mode::Off,
            Mode::Required => integration::Mode::Required,
        };
        options.shell = match shell {
            Shell::Unknown => integration::RemoteShell::Unknown,
            Shell::Bash => integration::RemoteShell::Bash,
            Shell::Zsh => integration::RemoteShell::Zsh,
            Shell::Fish => integration::RemoteShell::Fish,
            Shell::PowerShell => integration::RemoteShell::PowerShell,
        };
        options.startup = match startup {
            Startup::Login => integration::Startup::Login,
            Startup::Interactive => integration::Startup::Interactive,
        };
        options.terminal_input = *tty;
        options.terminal_output = *tty;
        options.posix_account_shell = *posix_account_shell;
        options.permit_session_files = *permit_session_files;
        options.configuration = if *assume_compatible_config {
            integration::ConfigEvidence::CompatibleForPreview
        } else {
            integration::ConfigEvidence::Unknown
        };
        let invocation = integration::Invocation::new(arguments.clone())?;
        result["original_argument_count"] = arguments.len().into();
        match integration::plan(invocation, options)? {
            integration::Decision::Denied => {
                result["decision"] = "denied".into();
            }
            integration::Decision::Passthrough { reason, .. } => {
                result["decision"] = "native-passthrough-plan-only".into();
                result["fallback"] = format!("{reason:?}").into();
            }
            integration::Decision::Unavailable { reason } => {
                result["decision"] = "required-integration-unavailable".into();
                result["fallback"] = format!("{reason:?}").into();
            }
            integration::Decision::Candidate(candidate) => {
                result["decision"] = "candidate-needs-new-review".into();
                result["proposed_argument_count"] =
                    candidate.proposed_arguments_for_new_review().len().into();
                result["proposed_argument_bytes"] = candidate
                    .proposed_arguments_for_new_review()
                    .iter()
                    .map(|a| a.as_encoded_bytes().len())
                    .sum::<usize>()
                    .into();
            }
        }
    }
    Ok(result)
}
pub fn execute(
    command: &Command,
    native_broker_enabled: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    use std::io::Write;
    let report = report(command, native_broker_enabled)?;
    let mut out = std::io::stdout().lock();
    serde_json::to_writer_pretty(&mut out, &report)?;
    writeln!(out)?;
    Ok(())
}
