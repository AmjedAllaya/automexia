//! Package-owned temporary remote helper. OpenSSH still owns transport and PTY;
//! the existing CLI process owner owns the shell and each isolated scanner.
mod discovery;
mod output;
#[cfg(unix)]
mod unix_endpoint;
mod worker;
#[cfg(unix)]
use unix_endpoint::Endpoint;
#[cfg(windows)]
mod windows_endpoint;
#[cfg(windows)]
use windows_endpoint::Endpoint;

use super::{cli_process, private_fs};
use automexia_ssh_integration::{
    bootstrap,
    helper::{DiscoveryRequest, HELPER_DESCRIPTION, MAX_DISCOVERY_REQUEST_BYTES},
    GenerationKey, RemoteShell,
};
use std::{
    ffi::OsString,
    io::{self, Write},
    path::PathBuf,
    process::{Command, ExitStatus},
    time::{Duration, Instant},
};

fn failure() -> io::Error {
    io::Error::other("SSH helper request or runtime unavailable")
}

/// Internal versioned executable contract. Paths and requests are never logged.
pub fn run(arguments: Vec<OsString>) -> io::Result<Option<ExitStatus>> {
    if arguments.as_slice() == [OsString::from("--describe-v1")] {
        writeln!(io::stdout().lock(), "{HELPER_DESCRIPTION}")?;
        return Ok(None);
    }
    if arguments.len() != 4 {
        return Err(failure());
    }
    let mode = arguments[0].to_str().ok_or_else(failure)?;
    let (shell, name) = match arguments[1].to_str() {
        Some("bash") => (RemoteShell::Bash, "bash"),
        Some("zsh") => (RemoteShell::Zsh, "zsh"),
        Some("fish") => (RemoteShell::Fish, "fish"),
        Some("powershell") => (RemoteShell::PowerShell, "powershell"),
        Some("pwsh") => (RemoteShell::Pwsh, "pwsh"),
        _ => return Err(failure()),
    };
    let number = |index: usize| {
        arguments
            .get(index)
            .and_then(|value| value.to_str())
            .and_then(|value| value.parse::<u64>().ok())
            .ok_or_else(failure)
    };
    let key = GenerationKey::new(number(2)?, number(3)?).map_err(|_| failure())?;
    match mode {
        "--scan-v1" => {
            let value =
                std::env::var_os("AUTOMEXIA_SSH_SCAN_REQUEST").ok_or_else(failure)?;
            if value.as_encoded_bytes().len() > MAX_DISCOVERY_REQUEST_BYTES - 2 {
                return Err(failure());
            }
            let mut frame = vec![0];
            frame.extend_from_slice(value.to_str().ok_or_else(failure)?.as_bytes());
            frame.push(0);
            let request = DiscoveryRequest::decode(key, &frame)
                .map_err(|_| failure())?
                .ok_or_else(failure)?;
            let output = discovery::detect(&request, name)
                .map_err(|_| failure())?
                .encode();
            io::stdout().lock().write_all(output.as_bytes())?;
            Ok(None)
        }
        "--session-v1" => session(shell, name, key).map(Some),
        _ => Err(failure()),
    }
}

fn session(
    shell: RemoteShell,
    name: &'static str,
    key: GenerationKey,
) -> io::Result<ExitStatus> {
    let supported = (cfg!(unix) && matches!(shell, RemoteShell::Bash | RemoteShell::Zsh))
        || (cfg!(windows)
            && matches!(shell, RemoteShell::PowerShell | RemoteShell::Pwsh));
    if !supported {
        // Helper producers need a verified nonblocking native IPC path.
        // Ordinary Fish and PowerShell-on-Unix integration remains available.
        return Err(failure());
    }
    let cancellation = cli_process::Cancellation::install_interactive_shell()?;
    let mut endpoint = Endpoint::new()?;
    let files = StartupFiles::create(shell, key)?;
    let mut command = files.command(shell)?;
    endpoint.configure(&mut command);
    let worker = worker::Worker::start(std::env::current_exe()?, name)?;
    let mut framing = Framing::default();
    let revoke =
        bootstrap::user_var_frame("automexia_ssh_revision", "").map_err(|_| failure())?;
    #[cfg(unix)]
    let mut output = match endpoint.prompt_output()? {
        Some((file, ready)) => output::Publisher::prompt_channel(file, ready)?,
        None => output::Publisher::new()?,
    };
    #[cfg(windows)]
    let mut output = output::Publisher::new()?;
    let mut disabled = false;
    let mut next_revoke = Instant::now();
    let mut bytes = [0; MAX_DISCOVERY_REQUEST_BYTES];
    let result = cli_process::interactive_observed(
        command,
        || cancellation.cancelled(),
        |child| {
            if worker.failed() && !disabled {
                disabled = true;
                worker.stop();
                next_revoke = Instant::now();
            }
            if disabled {
                if Instant::now() >= next_revoke {
                    // A transient full terminal must not permanently lose the
                    // only revocation. Retry at the normal discovery interval.
                    let _ = output.publish(&revoke);
                    next_revoke = Instant::now() + Duration::from_secs(3);
                }
                return Ok(());
            }
            match endpoint.poll(child, &mut bytes) {
                Ok(count) => {
                    framing
                        .accept(&bytes[..count], key, |request| worker.submit(request))?;
                }
                Err(_) => {
                    disabled = true;
                    worker.stop();
                    let _ = output.publish(&revoke);
                    next_revoke = Instant::now() + Duration::from_secs(3);
                    return Ok(());
                }
            }
            if let Some(result) = worker.receive() {
                let frame = bootstrap::user_var_frame(
                    "automexia_ssh_context_v2",
                    &result.encode(),
                )
                .map_err(|_| failure())?;
                // The bounded publisher owns completion of accepted frames.
                if output.publish(&frame).is_err() {
                    disabled = true;
                    worker.stop();
                }
            }
            Ok(())
        },
    );
    let scanner_retirement = worker.finish();
    let shell_unretired = result.as_ref().is_err_and(|error| {
        cli_process::retirement_incomplete(error)
            && !matches!(cli_process::retry_retirement(error), Ok(true))
    });
    let scanner_unretired = scanner_retirement.as_ref().is_err_and(|error| {
        cli_process::retirement_incomplete(error)
            && !matches!(cli_process::retry_retirement(error), Ok(true))
    });
    // Keep the private receiving lifeline through writer retirement, so
    // closing the shell cannot turn an in-flight channel write into SIGPIPE.
    // StartupFiles removes only exact owned names after shell retirement.
    if shell_unretired {
        // A still-running shell may need these files. Preserve the bounded
        // owner until process exit instead of claiming normal session cleanup.
        std::mem::forget(files);
    } else {
        drop(files);
    }
    if let Err(unretired) = output.finish() {
        // Only the standalone helper owns this writer. A native driver that
        // refuses cancellation must not leave it detached from a live owner.
        // Shell/scanner retirement and exact-file cleanup precede process exit.
        unretired.terminate_helper();
    }
    drop(endpoint);
    if shell_unretired || scanner_unretired {
        eprintln!("Automexia SSH helper could not confirm child cleanup; temporary files may remain.");
        // Both errors retain the exact process owners until helper termination.
        std::process::exit(70);
    }
    scanner_retirement?;
    #[cfg(unix)]
    {
        let signal = cancellation.signal();
        drop(cancellation);
        if let Some(signal) = signal {
            let _ = signal_hook::low_level::emulate_default_handler(signal);
            return Err(io::Error::new(
                io::ErrorKind::Interrupted,
                "SSH helper cancelled",
            ));
        }
    }
    result
}

#[derive(Default)]
struct Framing {
    bytes: Vec<u8>,
    overflow: bool,
}
impl Framing {
    fn accept(
        &mut self,
        input: &[u8],
        key: GenerationKey,
        mut accept: impl FnMut(DiscoveryRequest) -> io::Result<()>,
    ) -> io::Result<()> {
        for &byte in input {
            if byte == 0 {
                if !self.overflow && !self.bytes.is_empty() {
                    let mut frame = Vec::with_capacity(self.bytes.len() + 2);
                    frame.push(0);
                    frame.extend_from_slice(&self.bytes);
                    frame.push(0);
                    if let Ok(Some(request)) = DiscoveryRequest::decode(key, &frame) {
                        accept(request)?;
                    }
                }
                self.bytes.clear();
                self.overflow = false;
            } else if !self.overflow {
                if self.bytes.len() >= MAX_DISCOVERY_REQUEST_BYTES - 2 {
                    self.bytes.clear();
                    self.overflow = true;
                } else {
                    self.bytes.push(byte);
                }
            }
        }
        Ok(())
    }
}

struct StartupFiles {
    directory: PathBuf,
    names: Vec<&'static str>,
}
impl StartupFiles {
    fn create(shell: RemoteShell, key: GenerationKey) -> io::Result<Self> {
        let hook = match shell {
            RemoteShell::Bash => include_str!(
                "../../../../automexia-ssh-integration/resources/helper-bash.bash"
            ),
            RemoteShell::Zsh => include_str!(
                "../../../../automexia-ssh-integration/resources/helper-zsh.zsh"
            ),
            RemoteShell::PowerShell | RemoteShell::Pwsh => include_str!(
                "../../../../automexia-ssh-integration/resources/helper-powershell.ps1"
            ),
            RemoteShell::Unknown | RemoteShell::Fish => return Err(failure()),
        };
        let contents =
            bootstrap::helper_shell_files(shell, key, hook).map_err(|_| failure())?;
        let temporary = tempfile::Builder::new()
            .prefix("automexia-ssh-shell-")
            .tempdir()?;
        private_fs::validate_private_child_directory(temporary.path())
            .map_err(|_| failure())?;
        let mut files = Self {
            directory: temporary.keep(),
            names: Vec::new(),
        };
        for (name, source) in contents {
            let path = files.directory.join(name);
            let mut file =
                private_fs::create_private_file(&path).map_err(|_| failure())?;
            files.names.push(name);
            file.write_all(source.as_bytes())?;
        }
        Ok(files)
    }

    fn command(&self, shell: RemoteShell) -> io::Result<Command> {
        let mut command = match shell {
            RemoteShell::Bash => {
                let mut command = Command::new("bash");
                command
                    .args(["--noprofile", "--rcfile"])
                    .arg(self.directory.join("rc.bash"))
                    .arg("-i");
                command
            }
            RemoteShell::Zsh => {
                let mut command = Command::new("zsh");
                command.arg("-i");
                let original = std::env::var_os("ZDOTDIR");
                command.env(
                    "AUTOMEXIA_SSH_ZDOTDIR_SET",
                    if original.is_some() { "x" } else { "" },
                );
                command.env(
                    "AUTOMEXIA_SSH_ZDOTDIR",
                    original
                        .unwrap_or_else(|| std::env::var_os("HOME").unwrap_or_default()),
                );
                command.env("ZDOTDIR", &self.directory);
                command
            }
            RemoteShell::PowerShell | RemoteShell::Pwsh => {
                let mut command = Command::new(if shell == RemoteShell::PowerShell {
                    "powershell.exe"
                } else {
                    "pwsh"
                });
                command
                    .args(["-NoLogo", "-NoExit", "-File"])
                    .arg(self.directory.join("rc.ps1"));
                command
            }
            RemoteShell::Unknown | RemoteShell::Fish => return Err(failure()),
        };
        command.env_remove("AUTOMEXIA_SSH_SCAN_REQUEST");
        Ok(command)
    }
}
impl Drop for StartupFiles {
    fn drop(&mut self) {
        // No recursive cleanup: user startup may create unrelated files here.
        // A nonempty directory is left intact instead of deleting their work.
        for name in &self.names {
            let _ = std::fs::remove_file(self.directory.join(name));
        }
        let _ = std::fs::remove_dir(&self.directory);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn startup_files_remove_only_bundled_names_after_retirement() {
        for shell in [RemoteShell::Bash, RemoteShell::Zsh, RemoteShell::Pwsh] {
            let files =
                StartupFiles::create(shell, GenerationKey::new(3, 7).unwrap()).unwrap();
            let directory = files.directory.clone();
            assert!(files
                .names
                .iter()
                .all(|name| directory.join(name).is_file()));
            drop(files);
            assert!(!directory.exists());
        }
    }

    #[test]
    fn startup_files_preserve_user_history_and_completion_data() {
        let files =
            StartupFiles::create(RemoteShell::Zsh, GenerationKey::new(3, 7).unwrap())
                .unwrap();
        let directory = files.directory.clone();
        let user_files = [".zsh_history", ".zcompdump", ".zcompdump.zwc", "user-note"];
        for name in user_files {
            std::fs::write(directory.join(name), b"user-owned fixture").unwrap();
        }
        drop(files);
        assert!(!directory.join(".zshenv").exists());
        assert!(!directory.join(".zshrc").exists());
        for name in user_files {
            assert_eq!(
                std::fs::read(directory.join(name)).unwrap(),
                b"user-owned fixture"
            );
            std::fs::remove_file(directory.join(name)).unwrap();
        }
        std::fs::remove_dir(directory).unwrap();
    }

    #[test]
    fn request_framing_recovers_after_fragmentation_oversize_and_partial_records() {
        let key = GenerationKey::new(1, 2).unwrap();
        let request =
            DiscoveryRequest::new(key, 3, "/fixture", Default::default()).unwrap();
        let encoded = request.encode();
        let mut framing = Framing::default();
        let mut seen = Vec::new();
        framing
            .accept(&vec![b'x'; MAX_DISCOVERY_REQUEST_BYTES * 2], key, |value| {
                seen.push(value);
                Ok(())
            })
            .unwrap();
        assert!(framing.bytes.len() <= MAX_DISCOVERY_REQUEST_BYTES);
        framing
            .accept(b"\0AMXREQ1|partial", key, |value| {
                seen.push(value);
                Ok(())
            })
            .unwrap();
        for chunk in encoded.chunks(7) {
            framing
                .accept(chunk, key, |value| {
                    seen.push(value);
                    Ok(())
                })
                .unwrap();
        }
        assert_eq!(seen, vec![request]);
    }
}
