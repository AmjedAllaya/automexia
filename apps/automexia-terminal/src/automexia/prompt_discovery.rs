//! Killable isolation for WSL-backed prompt configuration reads. This is not a
//! general provider runner: only package-owned application entrypoints are launched.
use std::io::Read;
use std::os::windows::{io::AsRawHandle, process::CommandExt};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use automexia_devops::kubernetes;
pub use automexia_devops::prompt_helper::dispatch_helper;
use automexia_devops::prompt_helper::{Context, HELPER_FLAG, MAX_REQUEST, MAX_RESPONSE};
use automexia_extension_api::SessionFacts;
use automexia_extension_runtime::CancellationToken;
use windows_sys::Win32::System::Pipes::PeekNamedPipe;

const DEADLINE: Duration = Duration::from_millis(1500);
const POLL: Duration = Duration::from_millis(10);

fn helper_executable(current: &std::path::Path) -> std::path::PathBuf {
    use std::os::windows::fs::MetadataExt;
    let console = current.with_file_name("amx.exe");
    // Never select a PATH entry or a reparse-point executable. Single-binary
    // developer builds retain the existing desktop dispatch as a fallback.
    if std::fs::symlink_metadata(&console).is_ok_and(|metadata| {
        metadata.is_file() && metadata.file_attributes() & 0x400 == 0
    }) {
        console
    } else {
        current.to_path_buf()
    }
}

pub(super) fn refresh(
    session: &SessionFacts,
    include_git: bool,
    include_kubernetes: bool,
    cancellation: &CancellationToken,
) -> Option<Context> {
    if cancellation.is_cancelled()
        || !kubernetes::is_wsl_session(session)
        || !(include_git || include_kubernetes)
    {
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
        .filter(|(name, _)| {
            include_kubernetes && matches!(name.as_str(), "HOME" | "KUBECONFIG")
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    wire["environment"] = serde_json::to_value(hints).ok()?;
    let request = serde_json::to_string(&serde_json::json!({
        "session": wire, "git": include_git, "kubernetes": include_kubernetes,
    }))
    .ok()?;
    if request.len() > MAX_REQUEST {
        return None;
    }
    let executable = helper_executable(&std::env::current_exe().ok()?);
    let mut command = Command::new(executable);
    command.args([HELPER_FLAG, &request]);
    let bytes = run_child(&mut command, cancellation, DEADLINE).ok()?;
    let mut result: Context = serde_json::from_slice(&bytes).ok()?;
    if !include_git {
        result.git_branch = None;
    }
    if !include_kubernetes {
        result.kubernetes = None;
    }
    valid_context(&result).then_some(result)
}

fn valid_context(context: &Context) -> bool {
    context
        .git_branch
        .iter()
        .chain(
            context
                .kubernetes
                .iter()
                .flat_map(|context| [&context.context, &context.namespace]),
        )
        .all(|label| {
            !label.is_empty()
                && label.chars().count() <= 96
                && automexia_devops::sanitize_label(label) == *label
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
    use std::io::Write;

    #[test]
    fn helper_scope_and_projection_are_explicit_and_bounded() {
        for label in ["", "bad\nlabel", &"x".repeat(97)] {
            assert!(!valid_context(&Context {
                git_branch: Some(label.into()),
                ..Context::default()
            }));
        }
        assert!(valid_context(&Context {
            git_branch: Some("fixture-branch".into()),
            ..Context::default()
        }));
        assert!(serde_json::from_str::<Context>(
            r#"{"git_branch":null,"kubernetes":null,"extra":1}"#
        )
        .is_err());
    }

    #[test]
    fn helper_selects_only_a_regular_adjacent_console_entrypoint() {
        let directory = tempfile::tempdir().unwrap();
        let desktop = directory.path().join("automexia.exe");
        let console = directory.path().join("amx.exe");
        assert_eq!(helper_executable(&desktop), desktop);
        std::fs::create_dir(&console).unwrap();
        assert_eq!(helper_executable(&desktop), desktop);
        std::fs::remove_dir(&console).unwrap();
        std::fs::write(&console, b"fixture").unwrap();
        assert_eq!(helper_executable(&desktop), console);
    }

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
