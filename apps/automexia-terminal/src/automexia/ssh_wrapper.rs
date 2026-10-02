//! Explicit system-OpenSSH CLI adapter. The terminal keeps its existing PTY;
//! OpenSSH owns authentication, trust, configuration, proxies and networking.
use super::{cli_process, local_tools::ToolSession};
use automexia_ssh_integration::{self as integration, Invocation, InvocationClass};
use clap::{Args, ValueEnum};
use std::{
    ffi::OsString,
    io::{self, IsTerminal},
    path::PathBuf,
    process::{Command as ProcessCommand, ExitStatus},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

#[derive(Args)]
#[command(
    after_help = "Examples:\n  automexia +ssh -- host\n  automexia +ssh --shell bash -- host\n  automexia +ssh --integration off -- -p 2222 host\n\nAuto preserves native SSH when the remote shell is unknown and isolates its display metadata from local paths. If OpenSSH configuration cannot be classified, Auto stops before connecting; choose --integration off for exact native passthrough. Select --shell only when you know the account shell. Required checks local integration prerequisites; it cannot guarantee remote shell startup. Integration uses temporary session files and never edits your remote profile. Native SSH options and commands follow --.\n\nOn Windows, use amx +ssh so the console launcher preserves input and waits for completion. For WSL, use the native Linux Automexia CLI for keyboard signals and window resizing. --force-tty enables integration through bridged streams, but does not repair their local terminal behavior."
)]
pub struct Command {
    /// Known remote account shell. Auto preserves native startup without guessing.
    #[arg(long, value_enum, default_value = "auto")]
    pub shell: Shell,
    /// Auto classifies safe native fallbacks; off preserves native behavior; required checks local prerequisites.
    #[arg(long, value_enum, default_value = "auto")]
    pub integration: Integration,
    /// Request a remote TTY for integration, like OpenSSH -tt; bridged signals and resizing remain limited.
    #[arg(long)]
    pub force_tty: bool,
    /// Upload this compatible Automexia helper for this session using two SSH invocations (120-second staging deadline).
    #[arg(
        long,
        value_name = "PATH",
        long_help = "Upload a compatible Automexia helper for this session. Supports Bash/Zsh on Unix and PowerShell/pwsh on Windows; Fish helper upload is unavailable. Requires integration auto or required. Uses two SSH invocations and may authenticate twice, with a 120-second staging deadline. The file must be a nonempty regular file at most 64 MiB. No download or persistent installation. Interrupted setup, disconnect or forced termination can leave temporary files on the remote host."
    )]
    pub helper_upload: Option<PathBuf>,
    /// Exact OpenSSH arguments, including the destination, after --.
    #[arg(last = true)]
    pub arguments: Vec<OsString>,
}

impl std::fmt::Debug for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SshWrapperCommand")
            .field("shell", &self.shell)
            .field("integration", &self.integration)
            .field("force_tty", &self.force_tty)
            .field("helper_upload", &self.helper_upload.is_some())
            .field("argument_count", &self.arguments.len())
            .finish()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum Shell {
    Auto,
    Bash,
    Zsh,
    Fish,
    #[value(name = "powershell")]
    PowerShell,
    Pwsh,
}

impl Shell {
    fn remote(self) -> integration::RemoteShell {
        match self {
            Self::Auto => integration::RemoteShell::Unknown,
            Self::Bash => integration::RemoteShell::Bash,
            Self::Zsh => integration::RemoteShell::Zsh,
            Self::Fish => integration::RemoteShell::Fish,
            Self::PowerShell => integration::RemoteShell::PowerShell,
            Self::Pwsh => integration::RemoteShell::Pwsh,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum Integration {
    Auto,
    Off,
    Required,
}

fn unavailable(reason: &str) -> io::Error {
    io::Error::new(io::ErrorKind::Unsupported,
        format!("required SSH integration is unavailable: {reason}; check --shell and interactive SSH options, or choose --integration off"))
}

fn native_or_required(
    command: &Command,
    session: &ToolSession,
    reason: &str,
) -> io::Result<ExitStatus> {
    if command.helper_upload.is_some() {
        return Err(upload_unavailable(reason));
    }
    if command.integration == Integration::Required {
        return Err(unavailable(reason));
    }
    run_native(session.interactive_command("ssh", &command.arguments)?)
}

fn upload_unavailable(reason: &str) -> io::Error {
    io::Error::new(io::ErrorKind::Unsupported,
        format!("SSH helper upload is unavailable: {reason}; remove --helper-upload to use native SSH"))
}

fn run_native(command: ProcessCommand) -> io::Result<ExitStatus> {
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        let mut command = command;
        // Replacement preserves native signal, foreground-job and exit behavior.
        let _ = command.exec();
        Err(io::Error::new(
            io::ErrorKind::NotFound,
            "system OpenSSH could not be started; install the native SSH client",
        ))
    }
    #[cfg(not(unix))]
    {
        // Native -f/control operations may intentionally retain a background
        // client. Preserve the original terminal's process ownership here.
        let mut command = command;
        command.status().map_err(|_| {
            io::Error::new(
                io::ErrorKind::NotFound,
                "system OpenSSH could not be started; install the native SSH client",
            )
        })
    }
}

fn invocation_key() -> io::Result<integration::GenerationKey> {
    let generation = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| io::Error::other("SSH invocation clock is unavailable"))?
        .as_nanos();
    // Advisory replay separation only. Neither PID nor time grants authority.
    let generation = (generation as u64).max(1);
    integration::GenerationKey::new(u64::from(std::process::id()).max(1), generation)
        .map_err(io::Error::other)
}

pub fn execute(command: &Command, session: &ToolSession) -> io::Result<ExitStatus> {
    if command.helper_upload.is_some() && command.shell == Shell::Fish {
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "Fish helper upload is unavailable; omit --helper-upload to use ordinary Fish shell integration",
        ));
    }
    if command.helper_upload.is_some()
        && (command.integration == Integration::Off || command.shell == Shell::Auto)
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "helper upload requires a known --shell and integration auto or required",
        ));
    }
    let invocation =
        Invocation::new(command.arguments.clone()).map_err(io::Error::other)?;
    if command.integration == Integration::Off {
        return native_or_required(command, session, "integration is disabled");
    }
    let interactive = invocation.classify_for_wrapper(
        command.force_tty || io::stdin().is_terminal(),
        command.force_tty || (io::stdout().is_terminal() && io::stderr().is_terminal()),
    );
    if !matches!(interactive, InvocationClass::Interactive { .. }) {
        let reason = if !command.force_tty
            && (!io::stdin().is_terminal()
                || !io::stdout().is_terminal()
                || !io::stderr().is_terminal())
        {
            "the client does not own an interactive terminal"
        } else {
            "this OpenSSH invocation requires native behavior"
        };
        return native_or_required(command, session, reason);
    }

    // This explicit user command alone evaluates config. The bounded observation
    // is never performed by a renderer, startup hook or passive inventory.
    let cancellation = cli_process::Cancellation::install()?;
    let mut inspect_arguments = Vec::with_capacity(command.arguments.len() + 1);
    inspect_arguments.push(OsString::from("-G"));
    inspect_arguments.extend(command.arguments.iter().cloned());
    let observed = cli_process::capture(
        session.interactive_command("ssh", &inspect_arguments)?,
        cli_process::Limits {
            timeout: Duration::from_secs(10),
            stdout: integration::MAX_EFFECTIVE_CONFIG_BYTES,
            stderr: 4096,
        },
        || cancellation.cancelled(),
    );
    if let Err(error) = &observed {
        if cli_process::retirement_incomplete(error)
            && cli_process::retry_retirement(error).is_err()
        {
            // Preserve the exact cleanup owner through the CLI failure path.
            // Do not discard it while translating config errors or launch SSH.
            return observed.map(|output| output.status);
        }
    }
    if cancellation.cancelled() {
        propagate_cancellation(cancellation);
        return Err(io::Error::new(
            io::ErrorKind::Interrupted,
            "SSH configuration inspection cancelled",
        ));
    }
    let configuration = match observed {
        Err(_) => Err("OpenSSH configuration inspection could not complete"),
        Ok(result) if !result.status.success() => {
            Err("OpenSSH configuration inspection failed")
        }
        Ok(result) => match integration::EffectiveConfig::parse(&result.stdout) {
            Err(_) => Err("OpenSSH configuration evidence is incomplete or unsupported"),
            Ok(config) => Ok(config),
        },
    };
    let configuration = configuration.map_err(|reason| {
        if command.helper_upload.is_some() {
            upload_unavailable(reason)
        } else if command.integration == Integration::Required {
            unavailable(reason)
        } else {
            io::Error::new(io::ErrorKind::Unsupported,
                "unable to determine safe interactive SSH mode; use --integration off for native passthrough")
        }
    })?;
    if command.shell == Shell::Auto || configuration.passthrough_reason().is_some() {
        let reason = if command.shell == Shell::Auto {
            "select the remote account shell with --shell"
        } else {
            "OpenSSH configuration requires native behavior"
        };
        return native_from_config(
            command,
            session,
            cancellation,
            configuration.native_interactive_eligible(),
            reason,
        );
    }

    let key = invocation_key()?;
    if let Some(path) = command.helper_upload.as_deref() {
        let snapshot = super::ssh_upload::Snapshot::read(path)?;
        return run_in_scope(key, command.shell.remote(), cancellation, |cancellation| {
            super::ssh_upload::run(
                snapshot,
                &invocation,
                session,
                command.shell.remote(),
                key,
                command.force_tty,
                cancellation,
            )
        });
    }
    let prepared = prepare_arguments(command, key);
    let prepared = match prepared {
        Ok(prepared) => prepared,
        Err(_) => {
            return native_from_config(
                command,
                session,
                cancellation,
                configuration.native_interactive_eligible(),
                "the remote shell adapter exceeds its supported limits",
            );
        }
    };
    // xterm-256color is the portable fallback; no remote package/profile install.
    let process = session.interactive_command_with_environment(
        "ssh",
        prepared.arguments(),
        &[("TERM", "xterm-256color")],
    )?;
    run_scoped(process, key, command.shell.remote(), cancellation, false)
}

fn native_from_config(
    command: &Command,
    session: &ToolSession,
    cancellation: cli_process::Cancellation,
    native_interactive: bool,
    reason: &str,
) -> io::Result<ExitStatus> {
    if command.helper_upload.is_some() {
        return Err(upload_unavailable(reason));
    }
    if command.integration == Integration::Required {
        return Err(unavailable(reason));
    }
    let process = session.interactive_command("ssh", &command.arguments)?;
    if native_interactive {
        run_scoped(
            process,
            invocation_key()?,
            integration::RemoteShell::Unknown,
            cancellation,
            true,
        )
    } else {
        drop(cancellation);
        run_native(process)
    }
}

fn run_scoped(
    process: ProcessCommand,
    key: integration::GenerationKey,
    shell: integration::RemoteShell,
    cancellation: cli_process::Cancellation,
    native_child: bool,
) -> io::Result<ExitStatus> {
    run_in_scope(key, shell, cancellation, |cancellation| {
        if native_child {
            cli_process::interactive_native(process, || cancellation.cancelled())
        } else {
            cli_process::interactive(process, || cancellation.cancelled())
        }
    })
}

fn run_in_scope(
    key: integration::GenerationKey,
    shell: integration::RemoteShell,
    cancellation: cli_process::Cancellation,
    work: impl FnOnce(&cli_process::Cancellation) -> io::Result<ExitStatus>,
) -> io::Result<ExitStatus> {
    let mut scope = super::ssh_scope::ScopeGuard::enter(key, shell)?;
    let result = work(&cancellation);
    if result.as_ref().is_err_and(|error| {
        cli_process::retirement_incomplete(error)
            && cli_process::retry_retirement(error).is_err()
    }) {
        // A child whose retirement is unconfirmed may still emit remote bytes.
        // Never publish the secret that restores the pane's local authority.
        scope.abandon();
    }
    let finish = scope.finish();
    // Drop retries a failed end write before signal propagation. A failed
    // display channel must not rewrite an already observed OpenSSH status.
    drop(scope);
    if finish.is_err() {
        use std::io::Write;
        let _ = writeln!(
            io::stderr().lock(),
            "Automexia: SSH closed; terminal scope restoration could not be displayed."
        );
    }
    propagate_cancellation(cancellation);
    result
}

fn propagate_cancellation(cancellation: cli_process::Cancellation) {
    #[cfg(unix)]
    {
        let signal = cancellation.signal();
        drop(cancellation);
        if let Some(signal) = signal {
            let _ = signal_hook::low_level::emulate_default_handler(signal);
            std::process::exit(128 + signal);
        }
    }
    #[cfg(not(unix))]
    drop(cancellation);
}

fn prepare_arguments(
    command: &Command,
    key: integration::GenerationKey,
) -> Result<Invocation, integration::Error> {
    let source =
        integration::bootstrap::interactive_candidate(command.shell.remote(), key)?;
    let mut arguments = Vec::with_capacity(command.arguments.len() + 2);
    arguments.push(OsString::from(if command.force_tty { "-tt" } else { "-t" }));
    arguments.extend(command.arguments.iter().cloned());
    arguments.push(OsString::from(source));
    Invocation::new(arguments)
}

/// Preserve the system client's status, including Unix signal termination.
pub fn exit_with_status(status: ExitStatus) -> ! {
    if let Some(code) = status.code() {
        std::process::exit(code);
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        if let Some(signal) = status.signal() {
            let _ = signal_hook::low_level::emulate_default_handler(signal);
            std::process::exit(128 + signal);
        }
    }
    std::process::exit(1);
}
