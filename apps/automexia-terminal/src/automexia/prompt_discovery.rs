//! Killable isolation for WSL-backed prompt configuration reads. This is not a
//! general provider runner: only the current application executable is launched.
use std::io::{Read, Write};
use std::os::windows::{io::AsRawHandle, process::CommandExt};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use automexia_devops::{kubernetes, KubernetesContext};
use automexia_extension_api::SessionFacts;
use automexia_extension_runtime::CancellationToken;
use windows_sys::Win32::System::Pipes::PeekNamedPipe;

const HELPER_FLAG: &str = "--internal-prompt-context-v1";
const MAX_REQUEST: usize = 8192;
const MAX_RESPONSE: usize = 4096;
const DEADLINE: Duration = Duration::from_millis(1500);
const POLL: Duration = Duration::from_millis(10);

/// Called before application configuration, logging, migration or GUI startup.
pub fn dispatch_helper() -> Option<i32> {
    let mut arguments = std::env::args_os().skip(1);
    if arguments.next().as_deref() != Some(std::ffi::OsStr::new(HELPER_FLAG)) {
        return None;
    }
    let result = (|| {
        let request = arguments.next()?;
        if arguments.next().is_some() {
            return None;
        }
        let request = request.to_str()?;
        if request.len() > MAX_REQUEST {
            return None;
        }
        let session: SessionFacts = serde_json::from_str(request).ok()?;
        let paths = kubernetes::wsl_paths(&session)?;
        let context = kubernetes::from_files(&paths);
        let bytes = serde_json::to_vec(&context).ok()?;
        if bytes.len() > MAX_RESPONSE {
            return None;
        }
        std::io::stdout().lock().write_all(&bytes).ok()?;
        Some(())
    })();
    // No error, path or parser excerpt can escape on stderr/stdout.
    Some(if result.is_some() { 0 } else { 2 })
}

pub(super) fn refresh(
    session: &SessionFacts,
    cancellation: &CancellationToken,
) -> Option<KubernetesContext> {
    if cancellation.is_cancelled() || kubernetes::wsl_paths(session).is_none() {
        return None;
    }
    let mut request = session.clone();
    request.title.clear();
    // Generic SessionFacts serialization excludes local hints. Only this
    // application-owned, non-persistent transport opts in to the two names.
    let mut wire = serde_json::to_value(&request).ok()?;
    let hints = request
        .environment
        .iter()
        .filter(|(name, _)| matches!(name.as_str(), "HOME" | "KUBECONFIG"))
        .collect::<std::collections::BTreeMap<_, _>>();
    wire["environment"] = serde_json::to_value(hints).ok()?;
    let request = serde_json::to_string(&wire).ok()?;
    if request.len() > MAX_REQUEST {
        return None;
    }
    let executable = std::env::current_exe().ok()?;
    let mut command = Command::new(executable);
    command.args([HELPER_FLAG, &request]);
    let bytes = run_child(&mut command, cancellation, DEADLINE).ok()?;
    let result: Option<KubernetesContext> = serde_json::from_slice(&bytes).ok()?;
    result.filter(|context| {
        [&context.context, &context.namespace].iter().all(|label| {
            !label.is_empty()
                && label.chars().count() <= 96
                && automexia_devops::sanitize_label(label) == **label
        })
    })
}

#[derive(Debug, PartialEq, Eq)]
enum Failure {
    Spawn,
    Io,
    Cancelled,
    Timeout,
    Oversized,
    Exit,
}

fn stop_and_reap(child: &mut Child) {
    // The child never spawns descendants. Keep ownership through native reaping;
    // all of this runs on the discovery worker, never the input/event thread.
    let _ = child.kill();
    let _ = child.wait();
}

fn run_child(
    command: &mut Command,
    cancellation: &CancellationToken,
    deadline: Duration,
) -> Result<Vec<u8>, Failure> {
    if cancellation.is_cancelled() {
        return Err(Failure::Cancelled);
    }
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .creation_flags(0x0800_0000)
        .spawn()
        .map_err(|_| Failure::Spawn)?;
    let start = Instant::now();
    let Some(mut stdout) = child.stdout.take() else {
        stop_and_reap(&mut child);
        return Err(Failure::Io);
    };
    let mut bytes = Vec::new();
    let result = (|| loop {
        if cancellation.is_cancelled() {
            return Err(Failure::Cancelled);
        }
        if start.elapsed() >= deadline {
            return Err(Failure::Timeout);
        }
        let mut available = 0;
        // SAFETY: the pipe handle remains owned by stdout; this single worker is
        // its only reader. Peek never consumes bytes or waits for a full message.
        let readable = unsafe {
            PeekNamedPipe(
                stdout.as_raw_handle() as _,
                std::ptr::null_mut(),
                0,
                std::ptr::null_mut(),
                &mut available,
                std::ptr::null_mut(),
            )
        } != 0;
        if readable && available > 0 {
            if bytes.len().saturating_add(available as usize) > MAX_RESPONSE {
                return Err(Failure::Oversized);
            }
            let mut chunk = [0u8; 1024];
            let count = stdout
                .read(&mut chunk[..(available as usize).min(1024)])
                .map_err(|_| Failure::Io)?;
            bytes.extend_from_slice(&chunk[..count]);
            continue;
        }
        if let Some(status) = child.try_wait().map_err(|_| Failure::Io)? {
            // Inspect once again after exit: publication can race the preceding
            // empty peek. Only this child owns a write end; no blocking EOF read.
            let mut remaining = 0;
            // SAFETY: the same exclusively owned pipe is still live after exit.
            let readable = unsafe {
                PeekNamedPipe(
                    stdout.as_raw_handle() as _,
                    std::ptr::null_mut(),
                    0,
                    std::ptr::null_mut(),
                    &mut remaining,
                    std::ptr::null_mut(),
                )
            } != 0;
            if readable && remaining > 0 {
                continue;
            }
            return if status.success() {
                Ok(bytes)
            } else {
                Err(Failure::Exit)
            };
        }
        std::thread::sleep(POLL.min(deadline.saturating_sub(start.elapsed())));
    })();
    if result.is_err() {
        stop_and_reap(&mut child);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn helper_process_fixture() {
        let Ok(mode) = std::env::var("AUTOMEXIA_PROMPT_FIXTURE") else {
            return;
        };
        match mode.as_str() {
            "stall" => loop {
                std::thread::park();
            },
            "overflow" => {
                std::io::stdout().write_all(&vec![b'x'; 4097]).unwrap();
            }
            "output" => {
                std::io::stdout().write_all(b"fixture-response").unwrap();
            }
            _ => std::process::exit(7),
        }
    }

    fn fixture(mode: &str) -> Command {
        let mut command = Command::new(std::env::current_exe().unwrap());
        command
            .args([
                "--exact",
                "automexia::prompt_discovery::tests::helper_process_fixture",
                "--nocapture",
            ])
            .env("AUTOMEXIA_PROMPT_FIXTURE", mode);
        command
    }

    #[test]
    fn native_helper_bounds_output_deadline_and_cancellation() {
        let token = CancellationToken::default();
        assert_eq!(
            run_child(&mut fixture("overflow"), &token, DEADLINE),
            Err(Failure::Oversized)
        );
        assert_eq!(
            run_child(&mut fixture("stall"), &token, Duration::from_millis(100)),
            Err(Failure::Timeout)
        );
        let bytes = run_child(&mut fixture("output"), &token, DEADLINE).unwrap();
        assert!(bytes
            .windows(b"fixture-response".len())
            .any(|slice| slice == b"fixture-response"));
        token.cancel();
        assert_eq!(
            run_child(&mut fixture("output"), &token, DEADLINE),
            Err(Failure::Cancelled)
        );
    }
}
