//! Explicit CLI process ownership, separate from GUI/PTY ownership.
use process_wrap::std::{ChildWrapper, CommandWrap};
use std::{
    io::{self, Read},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

const CLEANUP_TIMEOUT: Duration = Duration::from_secs(2);
const POLL_INTERVAL: Duration = Duration::from_millis(5);
mod cancellation;
#[cfg(unix)]
mod unix_foreground;
#[cfg(windows)]
mod windows_completion;
pub(crate) use cancellation::Cancellation;

#[derive(Clone, Copy)]
pub(crate) struct Limits {
    pub timeout: Duration,
    pub stdout: usize,
    pub stderr: usize,
}

pub(crate) struct Captured {
    pub status: std::process::ExitStatus,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

pub(crate) fn capture(
    command: Command,
    limits: Limits,
    cancelled: impl Fn() -> bool,
) -> io::Result<Captured> {
    capture_inner(command, limits, cancelled, CaptureInput::Null)
}

pub(crate) fn capture_leased(
    command: Command,
    limits: Limits,
    cancelled: impl Fn() -> bool,
) -> io::Result<Captured> {
    capture_inner(command, limits, cancelled, CaptureInput::Lease)
}

/// Explicit SSH staging consumes an immutable bounded snapshot, leaves native
/// authentication diagnostics on the terminal, and captures only its receipt.
pub(crate) fn capture_upload(
    command: Command,
    snapshot: std::fs::File,
    cancelled: impl Fn() -> bool,
) -> io::Result<Captured> {
    let metadata = snapshot.metadata()?;
    if !metadata.is_file() || metadata.len() == 0 || metadata.len() > 64 * 1024 * 1024 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "invalid SSH upload snapshot",
        ));
    }
    capture_inner(
        command,
        Limits {
            timeout: Duration::from_secs(120),
            stdout: 8192,
            stderr: 4096,
        },
        cancelled,
        CaptureInput::Upload(snapshot),
    )
}

enum CaptureInput {
    Null,
    Lease,
    Upload(std::fs::File),
}

/// Explicit interactive CLI work borrows the caller's terminal. Enhanced Unix
/// sessions temporarily hand its foreground to their owned process group.
/// Captured-tool budgets do not apply; cancellation cleanup stays bounded.
pub(crate) fn interactive(
    command: Command,
    cancelled: impl Fn() -> bool,
) -> io::Result<std::process::ExitStatus> {
    interactive_inner(command, cancelled, Ownership::Tree, |_| Ok(()))
}

/// The remote helper's one IPC owner may observe its pinned shell leader while
/// this owner retains spawn, terminal foreground, cancellation and retirement.
/// Observers must be bounded/nonblocking; their errors use the same cleanup.
pub(crate) fn interactive_observed(
    command: Command,
    cancelled: impl Fn() -> bool,
    observe: impl FnMut(u32) -> io::Result<()>,
) -> io::Result<std::process::ExitStatus> {
    interactive_inner(command, cancelled, Ownership::Tree, observe)
}

/// Native interactive SSH may deliberately leave a persistent multiplexing
/// master alive. Own and reap its foreground leader without claiming that
/// independent background connection or changing the caller's process group.
pub(crate) fn interactive_native(
    command: Command,
    cancelled: impl Fn() -> bool,
) -> io::Result<std::process::ExitStatus> {
    interactive_inner(command, cancelled, Ownership::Leader, |_| Ok(()))
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Ownership {
    Tree,
    Leader,
}

fn interactive_inner(
    mut command: Command,
    cancelled: impl Fn() -> bool,
    ownership: Ownership,
    mut observe: impl FnMut(u32) -> io::Result<()>,
) -> io::Result<std::process::ExitStatus> {
    if cancelled() {
        return Err(cancelled_error());
    }
    command
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    #[cfg(unix)]
    let mut foreground = if ownership == Ownership::Tree {
        unix_foreground::Foreground::prepare()?
    } else {
        None
    };
    #[cfg(unix)]
    let isolated_group = ownership == Ownership::Tree;
    #[cfg(not(unix))]
    let isolated_group = false;
    let mut child = spawn_owned(command, isolated_group, ownership)?;
    let result = (|| {
        #[cfg(unix)]
        if let Some(foreground) = &mut foreground {
            foreground.attach(child.inner.id())?;
        }
        loop {
            if cancelled() {
                break Err(cancelled_error());
            }
            observe(child.inner.id())?;
            #[cfg(unix)]
            if let Some(foreground) = &mut foreground {
                foreground.poll_stopped(&cancelled)?;
            }
            if leader_exited(child.inner.as_ref())? {
                // Tree owners pin the leader until descendants retire; native SSH
                // owns only this foreground process and preserves persistent peers.
                if ownership == Ownership::Tree {
                    child.retire()?;
                }
                if let Some(status) =
                    child.inner.try_wait().map_err(|_| cleanup_error())?
                {
                    child.reaped = true;
                    break Ok(status);
                }
            }
            std::thread::sleep(POLL_INTERVAL);
        }
    })();
    // Attempt both operations even if either fails. Metadata scope restoration
    // must happen only after this owner has confirmed child retirement.
    let cleanup = child.cleanup();
    #[cfg(unix)]
    let restoration = foreground.as_mut().map(|owner| owner.restore()).transpose();
    if cleanup.is_err() {
        return Err(incomplete_retirement(child));
    }
    #[cfg(unix)]
    restoration?;
    result
}

struct IncompleteRetirement {
    child: std::sync::Mutex<Option<OwnedChild>>,
}

impl std::fmt::Debug for IncompleteRetirement {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("IncompleteRetirement(<retained owner>)")
    }
}

fn incomplete_retirement(child: OwnedChild) -> io::Error {
    io::Error::other(IncompleteRetirement {
        child: std::sync::Mutex::new(Some(child)),
    })
}

impl std::fmt::Display for IncompleteRetirement {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("owned child retirement could not be confirmed")
    }
}

impl std::error::Error for IncompleteRetirement {}

pub(crate) fn retirement_incomplete(error: &io::Error) -> bool {
    error
        .get_ref()
        .is_some_and(|source| source.is::<IncompleteRetirement>())
}

/// Retry the same pinned owner after a bounded cleanup failure. The error must
/// remain owned by the caller until this succeeds or its explicit CLI process
/// ends fail-closed. No replacement child or detached cleanup thread is created.
/// False identifies an unrelated error; true confirms retirement.
pub(crate) fn retry_retirement(error: &io::Error) -> io::Result<bool> {
    let Some(owner) = error
        .get_ref()
        .and_then(|source| source.downcast_ref::<IncompleteRetirement>())
    else {
        return Ok(false);
    };
    let mut child = owner.child.lock().map_err(|_| cleanup_error())?;
    if let Some(child) = child.as_mut() {
        child.cleanup()?;
    }
    child.take();
    Ok(true)
}

fn spawn_owned(
    command: Command,
    isolated_group: bool,
    ownership: Ownership,
) -> io::Result<OwnedChild> {
    let mut command = CommandWrap::from(command);
    #[cfg(windows)]
    let completion = if ownership == Ownership::Tree {
        Some(windows_completion::CompletionJob::new()?)
    } else {
        None
    };
    #[cfg(not(windows))]
    let _ = ownership;
    #[cfg(unix)]
    if isolated_group {
        command.wrap(process_wrap::std::ProcessGroup::leader());
    }
    #[cfg(windows)]
    {
        if isolated_group {
            let mut flags = process_wrap::std::CreationFlags(Default::default());
            flags.0 .0 = windows_sys::Win32::System::Threading::CREATE_NEW_PROCESS_GROUP;
            command.wrap(flags);
        }
        if let Some(completion) = &completion {
            command
                .wrap(process_wrap::std::JobObject)
                .wrap(completion.clone());
        }
    }
    Ok(OwnedChild {
        inner: command.spawn().map_err(|_| {
            io::Error::new(io::ErrorKind::NotFound,
            "local tool could not be started; check that the required tool is installed")
        })?,
        reaped: false,
        killed: false,
        #[cfg(windows)]
        completion,
        #[cfg(windows)]
        members: Vec::new(),
    })
}

fn capture_inner(
    mut command: Command,
    limits: Limits,
    cancelled: impl Fn() -> bool,
    input: CaptureInput,
) -> io::Result<Captured> {
    let upload = matches!(input, CaptureInput::Upload(_));
    if limits.timeout.is_zero()
        || (!upload && limits.timeout > Duration::from_secs(60))
        || (upload && limits.timeout > Duration::from_secs(120))
        || limits.stdout == 0
        || limits.stdout > 4 * 1024 * 1024
        || limits.stderr == 0
        || limits.stderr > 64 * 1024
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "invalid local tool resource limits",
        ));
    }
    if cancelled() {
        return Err(cancelled_error());
    }
    command
        .stdin(match input {
            CaptureInput::Null => Stdio::null(),
            CaptureInput::Lease => Stdio::piped(),
            CaptureInput::Upload(snapshot) => Stdio::from(snapshot),
        })
        .stdout(Stdio::piped())
        .stderr(if upload {
            Stdio::inherit()
        } else {
            Stdio::piped()
        });
    #[cfg(unix)]
    let mut foreground = if upload {
        unix_foreground::Foreground::prepare()?
    } else {
        None
    };
    let mut child = spawn_owned(command, true, Ownership::Tree)?;
    let result = (|| {
        #[cfg(unix)]
        if let Some(foreground) = &mut foreground {
            foreground.attach(child.inner.id())?;
        }
        let mut stdout = child.inner.stdout().take().ok_or_else(pipe_error)?;
        let mut stderr = child.inner.stderr().take();
        if !upload && stderr.is_none() {
            return Err(pipe_error());
        }
        prepare_pipe(&stdout)?;
        if let Some(stderr) = &stderr {
            prepare_pipe(stderr)?;
        }
        let mut out = Vec::new();
        let mut err = Vec::new();
        let mut out_eof = false;
        let mut err_eof = upload;
        let mut status = None;
        let deadline = Instant::now() + limits.timeout;
        loop {
            if cancelled() {
                return Err(cancelled_error());
            }
            #[cfg(unix)]
            if let Some(foreground) = &mut foreground {
                foreground.poll_stopped(&cancelled)?;
            }
            if Instant::now() >= deadline {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "local tool exceeded its deadline",
                ));
            }
            if !out_eof {
                out_eof = drain(&mut stdout, &mut out, limits.stdout)?;
            }
            if !err_eof {
                err_eof = drain(
                    stderr.as_mut().ok_or_else(pipe_error)?,
                    &mut err,
                    limits.stderr,
                )?;
            }
            // Observe exit without reaping the leader. Its native identity stays
            // pinned while the complete group/job is retired, even if a helper
            // still holds stdout. Never signal a potentially reused process ID.
            if !child.killed && leader_exited(child.inner.as_ref())? {
                child.retire()?;
            }
            if child.killed && status.is_none() {
                status = child.inner.try_wait().map_err(|_| cleanup_error())?;
                child.reaped = status.is_some();
            }
            if let Some(status) = status {
                if out_eof && err_eof {
                    return Ok(Captured {
                        status,
                        stdout: out,
                        stderr: err,
                    });
                }
            }
            std::thread::sleep(POLL_INTERVAL);
        }
    })();
    let cleanup = child.cleanup();
    #[cfg(unix)]
    let restoration = foreground.as_mut().map(|owner| owner.restore()).transpose();
    if cleanup.is_err() {
        return Err(incomplete_retirement(child));
    }
    #[cfg(unix)]
    restoration?;
    result
}

fn cancelled_error() -> io::Error {
    io::Error::new(io::ErrorKind::Interrupted, "local tool cancelled")
}
fn pipe_error() -> io::Error {
    io::Error::other("local tool pipe unavailable")
}
fn cleanup_error() -> io::Error {
    io::Error::other("local tool cleanup could not be confirmed")
}

struct OwnedChild {
    inner: Box<dyn ChildWrapper>,
    reaped: bool,
    killed: bool,
    #[cfg(windows)]
    completion: Option<windows_completion::CompletionJob>,
    #[cfg(windows)]
    members: Vec<std::os::windows::io::OwnedHandle>,
}

impl OwnedChild {
    fn retire(&mut self) -> io::Result<()> {
        if !self.killed && !self.reaped {
            // Pin live members before requesting termination. A zero active-job
            // count may precede the final native process-handle signal.
            #[cfg(windows)]
            let members = self
                .completion
                .as_ref()
                .map(|completion| completion.pin_members())
                .transpose();
            self.inner.start_kill().map_err(|_| cleanup_error())?;
            self.killed = true;
            #[cfg(windows)]
            {
                self.members = members?.unwrap_or_default();
            }
        }
        Ok(())
    }

    fn cleanup(&mut self) -> io::Result<()> {
        let lease = self.inner.stdin().take();
        let had_lease = lease.is_some();
        drop(lease);
        if had_lease && !self.reaped {
            // EOF is the guest supervisor's cancellation handshake. Give it a
            // bounded chance to retire its Linux group before the Windows job.
            let deadline = Instant::now() + Duration::from_millis(500);
            while !leader_exited(self.inner.as_ref())? && Instant::now() < deadline {
                std::thread::sleep(POLL_INTERVAL);
            }
        }
        self.retire()?;
        let deadline = Instant::now() + CLEANUP_TIMEOUT;
        loop {
            if !self.reaped
                && self
                    .inner
                    .try_wait()
                    .map_err(|_| cleanup_error())?
                    .is_some()
            {
                self.reaped = true;
            }
            // Leader exit and pipe EOF are not job quiescence. In particular a
            // descendant may close both pipes while kernel teardown is pending.
            #[cfg(windows)]
            let tree_empty = self
                .completion
                .as_ref()
                .map(|completion| completion.is_empty())
                .transpose()?
                .unwrap_or(true)
                && windows_completion::members_stopped(&self.members)?;
            #[cfg(unix)]
            let tree_empty = true;
            if self.reaped && tree_empty {
                return Ok(());
            }
            if Instant::now() >= deadline {
                return Err(cleanup_error());
            }
            std::thread::sleep(POLL_INTERVAL);
        }
    }
}

impl Drop for OwnedChild {
    fn drop(&mut self) {
        // Best effort on panic/error. Normal paths explicitly verify cleanup.
        if !self.reaped && !self.killed {
            let _ = self.inner.start_kill();
        }
    }
}

#[cfg(unix)]
trait NativePipe: Read + std::os::fd::AsRawFd {}
#[cfg(unix)]
impl<T: Read + std::os::fd::AsRawFd> NativePipe for T {}
#[cfg(windows)]
trait NativePipe: Read + std::os::windows::io::AsRawHandle {}
#[cfg(windows)]
impl<T: Read + std::os::windows::io::AsRawHandle> NativePipe for T {}

#[cfg(unix)]
fn prepare_pipe(pipe: &impl NativePipe) -> io::Result<()> {
    let fd = pipe.as_raw_fd();
    // The owned child pipe stays live; preserve its existing descriptor flags.
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    if flags < 0
        || unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0
    {
        return Err(pipe_error());
    }
    Ok(())
}
#[cfg(windows)]
fn prepare_pipe(_pipe: &impl NativePipe) -> io::Result<()> {
    Ok(())
}

fn drain(
    pipe: &mut impl NativePipe,
    output: &mut Vec<u8>,
    limit: usize,
) -> io::Result<bool> {
    let mut buffer = [0u8; 8192];
    // Fair service for both streams and cancellation even under an output storm.
    for _ in 0..8 {
        #[cfg(unix)]
        let available = buffer.len();
        #[cfg(windows)]
        let available = {
            use windows_sys::Win32::{
                Foundation::ERROR_BROKEN_PIPE, System::Pipes::PeekNamedPipe,
            };
            let mut available = 0;
            // Peek never blocks; only read bytes currently owned by this pipe.
            if unsafe {
                PeekNamedPipe(
                    pipe.as_raw_handle(),
                    std::ptr::null_mut(),
                    0,
                    std::ptr::null_mut(),
                    &mut available,
                    std::ptr::null_mut(),
                )
            } == 0
            {
                if io::Error::last_os_error().raw_os_error()
                    == Some(ERROR_BROKEN_PIPE as i32)
                {
                    return Ok(true);
                }
                return Err(pipe_error());
            }
            if available == 0 {
                return Ok(false);
            }
            (available as usize).min(buffer.len())
        };
        let count = match pipe.read(&mut buffer[..available]) {
            Ok(0) => return Ok(true),
            Ok(count) => count,
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => return Ok(false),
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(_) => return Err(pipe_error()),
        };
        if output.len().saturating_add(count) > limit {
            return Err(io::Error::new(
                io::ErrorKind::FileTooLarge,
                "local tool output exceeded its byte limit",
            ));
        }
        output.extend_from_slice(&buffer[..count]);
    }
    Ok(false)
}

#[cfg(unix)]
fn leader_exited(child: &dyn ChildWrapper) -> io::Result<bool> {
    // WNOWAIT observes, but does not release, the owned leader's PID identity.
    let mut info: libc::siginfo_t = unsafe { std::mem::zeroed() };
    if unsafe {
        libc::waitid(
            libc::P_PID,
            child.id() as libc::id_t,
            &mut info,
            libc::WEXITED | libc::WNOHANG | libc::WNOWAIT,
        )
    } != 0
    {
        return Err(cleanup_error());
    }
    Ok(unsafe { info.si_pid() } != 0)
}

#[cfg(windows)]
fn leader_exited(child: &dyn ChildWrapper) -> io::Result<bool> {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::{
        Foundation::{WAIT_OBJECT_0, WAIT_TIMEOUT},
        System::Threading::WaitForSingleObject,
    };
    let handle = child.process_handle().ok_or_else(cleanup_error)?;
    // A borrowed live process handle, never a process-name or PID lookup.
    match unsafe { WaitForSingleObject(handle.as_raw_handle(), 0) } {
        WAIT_OBJECT_0 => Ok(true),
        WAIT_TIMEOUT => Ok(false),
        _ => Err(cleanup_error()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn amx_incomplete_retirement_retains_exact_owner_until_retry_confirms_exit() {
        use std::sync::{
            atomic::{AtomicBool, AtomicUsize, Ordering},
            Arc,
        };
        #[derive(Debug)]
        struct DelayedRetirement {
            ready: Arc<AtomicBool>,
            drops: Arc<AtomicUsize>,
            stdin: Option<std::process::ChildStdin>,
        }
        impl Drop for DelayedRetirement {
            fn drop(&mut self) {
                self.drops.fetch_add(1, Ordering::AcqRel);
            }
        }
        impl ChildWrapper for DelayedRetirement {
            fn inner(&self) -> &dyn ChildWrapper {
                self
            }
            fn inner_mut(&mut self) -> &mut dyn ChildWrapper {
                self
            }
            fn into_inner(self: Box<Self>) -> Box<dyn ChildWrapper> {
                self
            }
            fn stdin(&mut self) -> &mut Option<std::process::ChildStdin> {
                &mut self.stdin
            }
            fn start_kill(&mut self) -> io::Result<()> {
                if self.ready.load(Ordering::Acquire) {
                    Ok(())
                } else {
                    Err(cleanup_error())
                }
            }
            fn try_wait(&mut self) -> io::Result<Option<std::process::ExitStatus>> {
                #[cfg(unix)]
                use std::os::unix::process::ExitStatusExt;
                #[cfg(windows)]
                use std::os::windows::process::ExitStatusExt;
                Ok(Some(std::process::ExitStatus::from_raw(0)))
            }
        }
        let ready = Arc::new(AtomicBool::new(false));
        let drops = Arc::new(AtomicUsize::new(0));
        let mut child = OwnedChild {
            inner: Box::new(DelayedRetirement {
                ready: Arc::clone(&ready),
                drops: Arc::clone(&drops),
                stdin: None,
            }),
            reaped: false,
            killed: false,
            #[cfg(windows)]
            completion: None,
            #[cfg(windows)]
            members: Vec::new(),
        };
        assert!(child.cleanup().is_err());
        let error = incomplete_retirement(child);
        assert!(retirement_incomplete(&error));
        assert!(retry_retirement(&error).is_err());
        assert_eq!(
            drops.load(Ordering::Acquire),
            0,
            "failed cleanup must retain the same child"
        );
        assert!(!format!("{error:?}").contains("DelayedRetirement"));
        ready.store(true, Ordering::Release);
        assert!(retry_retirement(&error).unwrap());
        assert_eq!(drops.load(Ordering::Acquire), 1);
        assert!(
            retry_retirement(&error).unwrap(),
            "completed retry is idempotent"
        );
        assert!(!retry_retirement(&cleanup_error()).unwrap());
    }

    fn fixture(mode: &str) -> Command {
        let mut command = Command::new(std::env::current_exe().unwrap());
        command.args([
            "--exact",
            "automexia::cli_process::tests::amx_process_child_fixture",
            "--ignored",
            "--nocapture",
        ]);
        command.env("AMX_PROCESS_FIXTURE", mode);
        command
    }

    fn limits() -> Limits {
        Limits {
            timeout: Duration::from_secs(10),
            stdout: 16384,
            stderr: 1024,
        }
    }

    #[test]
    fn amx_process_captures_actual_child_streams_and_exit() {
        let result = capture(fixture("success"), limits(), || false).unwrap();
        assert!(result.status.success());
        assert!(result
            .stdout
            .windows(b"AMX_PAYLOAD_BEGIN\nfixture & output\nAMX_PAYLOAD_END".len())
            .any(
                |value| value == b"AMX_PAYLOAD_BEGIN\nfixture & output\nAMX_PAYLOAD_END"
            ));
        assert_eq!(result.stderr, b"fixture diagnostic");
    }

    #[test]
    fn amx_process_pre_cancel_never_spawns_and_errors_are_content_free() {
        let command = Command::new("missing-private-fixture-executable");
        let error = capture(command, limits(), || true).err().unwrap();
        assert_eq!(error.kind(), io::ErrorKind::Interrupted);
        assert!(!error.to_string().contains("private-fixture"));
    }

    #[test]
    fn amx_upload_capture_transfers_binary_snapshot_and_preserves_exit() {
        use std::io::Seek;
        let payload = b"AMX_UPLOAD_BEGIN\x00\xff\r\n\x1bAMX_UPLOAD_END";
        let mut snapshot = tempfile::tempfile().unwrap();
        snapshot.write_all(payload).unwrap();
        snapshot.rewind().unwrap();
        let output = capture_upload(fixture("upload-echo"), snapshot, || false).unwrap();
        assert_eq!(output.status.code(), Some(7));
        assert!(output
            .stdout
            .windows(payload.len())
            .any(|bytes| bytes == payload));
        assert!(
            output.stderr.is_empty(),
            "native stderr must not be captured"
        );
    }

    #[test]
    fn amx_upload_capture_rejects_empty_snapshot_and_bounded_receipt_overflow() {
        use std::io::Seek;
        let empty = tempfile::tempfile().unwrap();
        assert_eq!(
            capture_upload(Command::new("missing-upload-fixture"), empty, || false)
                .err()
                .unwrap()
                .kind(),
            io::ErrorKind::InvalidInput
        );
        let mut snapshot = tempfile::tempfile().unwrap();
        snapshot.write_all(b"fixture").unwrap();
        snapshot.rewind().unwrap();
        let error = capture_upload(fixture("stdout-flood"), snapshot, || false)
            .err()
            .unwrap();
        assert_eq!(error.kind(), io::ErrorKind::FileTooLarge);
    }

    #[test]
    fn amx_interactive_preserves_child_failure_status() {
        let result = interactive(fixture("failure"), || false).unwrap();
        assert_eq!(result.code(), Some(7));
    }

    #[test]
    fn amx_interactive_cancellation_reaps_the_owned_child() {
        let directory = tempfile::tempdir().unwrap();
        let ready = directory.path().join("ready");
        let identity = std::cell::RefCell::new(None);
        let mut command = fixture("wait");
        command.env("AMX_PROCESS_READY", &ready);
        let started = Instant::now();
        let error = interactive(command, || {
            observe_identity(&ready, &identity)
                || started.elapsed() > Duration::from_secs(5)
        })
        .unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::Interrupted);
        assert!(ready.is_file(), "child never reached the actual wait");
        assert!(started.elapsed() < Duration::from_secs(8));
        identity
            .borrow()
            .as_ref()
            .expect("child identity was not captured")
            .assert_stopped();
    }

    #[test]
    fn amx_interactive_observer_error_retires_its_exact_shell_leader() {
        let directory = tempfile::tempdir().unwrap();
        let ready = directory.path().join("ready");
        let identity = std::cell::RefCell::new(None);
        let mut command = fixture("wait");
        command.env("AMX_PROCESS_READY", &ready);
        let started = Instant::now();
        let mut observed_pid = None;
        let error = interactive_observed(
            command,
            || started.elapsed() > Duration::from_secs(5),
            |pid| {
                assert!(pid > 1);
                assert_eq!(*observed_pid.get_or_insert(pid), pid);
                if observe_identity(&ready, &identity) {
                    Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "fixture IPC fault",
                    ))
                } else {
                    Ok(())
                }
            },
        )
        .unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        assert!(!retirement_incomplete(&error));
        identity
            .borrow()
            .as_ref()
            .expect("observed child was pinned")
            .assert_stopped();
    }

    #[cfg(unix)]
    #[test]
    fn amx_native_interactive_keeps_the_foreground_process_group() {
        let directory = tempfile::tempdir().unwrap();
        let ready = directory.path().join("group");
        let mut command = fixture("interactive-group");
        command.env("AMX_PROCESS_READY", &ready);
        assert!(interactive_native(command, || false).unwrap().success());
        // SAFETY: getpgrp has no pointers, allocation or ownership transfer.
        let group = unsafe { libc::getpgrp() };
        assert_eq!(std::fs::read_to_string(ready).unwrap(), group.to_string());
    }

    #[cfg(unix)]
    #[test]
    fn amx_interactive_owns_a_separate_process_group() {
        let directory = tempfile::tempdir().unwrap();
        let ready = directory.path().join("group");
        let mut command = fixture("interactive-group");
        command.env("AMX_PROCESS_READY", &ready);
        assert!(interactive(command, || false).unwrap().success());
        // SAFETY: getpgrp has no pointers, allocation or ownership transfer.
        let caller = unsafe { libc::getpgrp() };
        let child_group: libc::pid_t =
            std::fs::read_to_string(ready).unwrap().parse().unwrap();
        assert!(child_group > 1);
        assert_ne!(child_group, caller);
    }

    #[test]
    fn amx_native_interactive_cancellation_reaps_only_the_owned_leader() {
        let directory = tempfile::tempdir().unwrap();
        let ready = directory.path().join("ready");
        let identity = std::cell::RefCell::new(None);
        let mut command = fixture("wait");
        command.env("AMX_PROCESS_READY", &ready);
        let started = Instant::now();
        let error = interactive_native(command, || {
            observe_identity(&ready, &identity)
                || started.elapsed() > Duration::from_secs(5)
        })
        .unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::Interrupted);
        assert!(started.elapsed() < Duration::from_secs(8));
        identity
            .borrow()
            .as_ref()
            .expect("native leader was pinned")
            .assert_stopped();
    }

    #[test]
    fn amx_native_interactive_preserves_intentional_background_connection() {
        let directory = tempfile::tempdir().unwrap();
        let ready = directory.path().join("ready");
        let release = directory.path().join("release-leader");
        let background_release = directory.path().join("release-background");
        let retired = directory.path().join("background-finished");
        let identity = std::cell::RefCell::new(None);
        let mut command = fixture("native-persistent-parent");
        command
            .env("AMX_PROCESS_READY", &ready)
            .env("AMX_PROCESS_RELEASE", &release)
            .env("AMX_PROCESS_BACKGROUND_RELEASE", &background_release)
            .env("AMX_PROCESS_RETIRED", &retired);
        let started = Instant::now();
        let status = interactive_native(command, || {
            if observe_identity(&ready, &identity) {
                std::fs::write(&release, b"release").unwrap();
            }
            started.elapsed() > Duration::from_secs(5)
        })
        .unwrap();
        assert_eq!(status.code(), Some(7));
        // This helper is owned by the fixture handshake, like a persistent SSH
        // master. A post-leader-exit acknowledgment proves it was not retired.
        std::fs::write(&background_release, b"release").unwrap();
        let deadline = Instant::now() + Duration::from_secs(3);
        while !retired.exists() && Instant::now() < deadline {
            std::thread::sleep(POLL_INTERVAL);
        }
        assert!(
            retired.exists(),
            "native background connection was killed with its leader"
        );
        let identity = identity.borrow();
        let identity = identity.as_ref().expect("background identity was pinned");
        #[cfg(windows)]
        {
            use std::os::windows::io::AsRawHandle;
            // SAFETY: the captured process handle remains owned and identifies
            // this exact fixture helper while its normal teardown completes.
            assert_eq!(
                unsafe {
                    windows_sys::Win32::System::Threading::WaitForSingleObject(
                        identity.handle.as_raw_handle(),
                        2000,
                    )
                },
                windows_sys::Win32::Foundation::WAIT_OBJECT_0
            );
        }
        #[cfg(unix)]
        identity.assert_stopped();
    }

    #[test]
    fn amx_process_rejects_invalid_resource_contracts_before_spawn() {
        for limit in [
            Limits {
                timeout: Duration::ZERO,
                ..limits()
            },
            Limits {
                timeout: Duration::from_secs(61),
                ..limits()
            },
            Limits {
                stdout: 0,
                ..limits()
            },
            Limits {
                stdout: 4 * 1024 * 1024 + 1,
                ..limits()
            },
            Limits {
                stderr: 0,
                ..limits()
            },
            Limits {
                stderr: 65537,
                ..limits()
            },
        ] {
            assert_eq!(
                capture(Command::new("missing-fixture"), limit, || false)
                    .err()
                    .unwrap()
                    .kind(),
                io::ErrorKind::InvalidInput
            );
        }
    }

    #[test]
    fn amx_process_overflow_in_either_stream_and_nonzero_exit_are_not_success() {
        for mode in ["stdout-flood", "stderr-flood"] {
            assert_eq!(
                capture(fixture(mode), limits(), || false)
                    .err()
                    .unwrap()
                    .kind(),
                io::ErrorKind::FileTooLarge
            );
        }
        let result = capture(fixture("failure"), limits(), || false).unwrap();
        assert!(!result.status.success());
        assert_eq!(result.status.code(), Some(7));
    }

    #[test]
    fn amx_process_running_child_cancellation_and_timeout_retire_native_handle() {
        for timed_out in [false, true] {
            let temporary = tempfile::tempdir().unwrap();
            let ready = temporary.path().join("ready");
            let identity = std::cell::RefCell::new(None);
            let mut command = fixture("wait");
            command.env("AMX_PROCESS_READY", &ready);
            let limit = Limits {
                timeout: Duration::from_secs(3),
                ..limits()
            };
            let begin = Instant::now();
            let result = capture(command, limit, || {
                observe_identity(&ready, &identity) && !timed_out
            });
            assert!(ready.is_file(), "child never acknowledged readiness");
            assert_eq!(
                result.err().unwrap().kind(),
                if timed_out {
                    io::ErrorKind::TimedOut
                } else {
                    io::ErrorKind::Interrupted
                }
            );
            assert!(begin.elapsed() < Duration::from_secs(6));
            identity
                .borrow()
                .as_ref()
                .expect("native readiness did not publish a complete identity")
                .assert_stopped();
        }
    }

    #[test]
    fn amx_process_guest_lease_closes_before_forced_cleanup() {
        let temporary = tempfile::tempdir().unwrap();
        let ready = temporary.path().join("ready");
        let retired = temporary.path().join("retired");
        let mut command = fixture("lease");
        command
            .env("AMX_PROCESS_READY", &ready)
            .env("AMX_PROCESS_RETIRED", &retired);
        let error = capture_leased(command, limits(), || ready.exists())
            .err()
            .unwrap();
        assert_eq!(error.kind(), io::ErrorKind::Interrupted);
        assert!(
            retired.is_file(),
            "lease EOF was not delivered before force cleanup"
        );
    }

    #[cfg(windows)]
    #[test]
    fn amx_process_native_console_cancel_retires_the_owned_tool() {
        use windows_sys::Win32::System::Console::{
            GenerateConsoleCtrlEvent, CTRL_BREAK_EVENT,
        };
        let temporary = tempfile::tempdir().unwrap();
        let ready = temporary.path().join("ready");
        let owner = temporary.path().join("owner");
        let mut command = fixture("signal-owner");
        command
            .env("AMX_PROCESS_READY", &ready)
            .env("AMX_PROCESS_OWNER", &owner);
        let sent = std::cell::Cell::new(false);
        let identity = std::cell::RefCell::new(None);
        let result = capture(command, limits(), || {
            if !sent.get() && observe_identity(&ready, &identity) {
                let pid: u32 = std::fs::read_to_string(&owner).unwrap().parse().unwrap();
                // Send only to the live fixture's distinct console group, never
                // to the test runner's console or an unrelated terminal session.
                assert_ne!(
                    unsafe { GenerateConsoleCtrlEvent(CTRL_BREAK_EVENT, pid) },
                    0
                );
                sent.set(true);
            }
            false
        })
        .unwrap();
        assert!(sent.get());
        assert!(result.status.success());
        identity.borrow().as_ref().unwrap().assert_stopped();
    }

    #[test]
    fn amx_process_leader_exit_retires_descendant_holding_pipes() {
        std::thread::scope(|scope| {
            for _ in 0..4 {
                scope.spawn(|| {
                    for iteration in 0..25 {
                        let temporary = tempfile::tempdir().unwrap();
                        let ready = temporary.path().join("ready");
                        let release = temporary.path().join("release");
                        let identity = std::cell::RefCell::new(None);
                        let mut command = fixture(if iteration % 2 == 0 {
                            "descendant"
                        } else {
                            "descendant-no-pipes"
                        });
                        command
                            .env("AMX_PROCESS_READY", &ready)
                            .env("AMX_PROCESS_RELEASE", &release);
                        let output = capture(command, limits(), || {
                            if identity.borrow().is_none() {
                                if let Some(pinned) = PinnedIdentity::from_ready(&ready) {
                                    *identity.borrow_mut() = Some(pinned);
                                    std::fs::write(&release, b"release").unwrap();
                                }
                            }
                            false
                        })
                        .unwrap();
                        assert!(output.status.success());
                        identity
                            .borrow()
                            .as_ref()
                            .expect("native child identity was not pinned")
                            .assert_stopped();
                    }
                });
            }
        });
    }

    #[cfg(windows)]
    #[test]
    fn amx_process_completion_recovers_native_handles_and_measures_capture() {
        // Isolate the counter from other concurrently executing test fixtures.
        let result =
            capture(fixture("completion-resources"), limits(), || false).unwrap();
        assert!(
            result.status.success(),
            "isolated native resource fixture failed"
        );
        let text = String::from_utf8(result.stdout).unwrap();
        let report = text
            .lines()
            .find(|line| line.starts_with("AMX_CAPTURE_RESOURCES "))
            .unwrap();
        println!("{report}");
    }

    #[cfg(windows)]
    fn completion_resources() {
        use windows_sys::Win32::System::Threading::{
            GetCurrentProcess, GetProcessHandleCount,
        };
        fn handles() -> u32 {
            let mut count = 0;
            assert_ne!(
                unsafe { GetProcessHandleCount(GetCurrentProcess(), &mut count) },
                0
            );
            count
        }
        // Warm the test runtime, then require exact recovery after each lifecycle.
        for _ in 0..3 {
            assert!(capture(fixture("success"), limits(), || false)
                .unwrap()
                .status
                .success());
        }
        let baseline = handles();
        let mut timings = Vec::new();
        for _ in 0..20 {
            let start = Instant::now();
            let result = capture(fixture("success"), limits(), || false).unwrap();
            assert!(result.status.success());
            assert_eq!(result.stderr, b"fixture diagnostic");
            assert!(result
                .stdout
                .windows(b"fixture & output".len())
                .any(|text| text == b"fixture & output"));
            timings.push(start.elapsed().as_micros());
            assert!(
                capture(Command::new("missing-resource-fixture"), limits(), || false)
                    .is_err()
            );
            assert_eq!(
                handles(),
                baseline,
                "native handles grew after completed capture"
            );
        }
        timings.sort_unstable();
        println!("AMX_CAPTURE_RESOURCES cycles=20 handle_delta=0 median_us={} p95_us={}; native fixture capture, not interactive latency", timings[10], timings[19]);
    }

    struct PinnedIdentity {
        #[cfg(windows)]
        handle: std::os::windows::io::OwnedHandle,
        #[cfg(unix)]
        pid: u32,
    }

    fn observe_identity(
        ready: &std::path::Path,
        identity: &std::cell::RefCell<Option<PinnedIdentity>>,
    ) -> bool {
        let mut pinned = identity.borrow_mut();
        if pinned.is_none() {
            *pinned = PinnedIdentity::from_ready(ready);
        }
        pinned.is_some()
    }

    impl PinnedIdentity {
        fn from_ready(ready: &std::path::Path) -> Option<Self> {
            let pid: u32 = std::fs::read_to_string(ready).ok()?.parse().ok()?;
            #[cfg(windows)]
            {
                use std::os::windows::io::AsRawHandle;
                use std::os::windows::io::FromRawHandle;
                use windows_sys::Win32::System::Threading::{
                    OpenProcess, PROCESS_SYNCHRONIZE,
                };
                let handle = unsafe { OpenProcess(PROCESS_SYNCHRONIZE, 0, pid) };
                if handle.is_null() {
                    return None;
                }
                let handle =
                    unsafe { std::os::windows::io::OwnedHandle::from_raw_handle(handle) };
                assert_eq!(
                    unsafe {
                        windows_sys::Win32::System::Threading::WaitForSingleObject(
                            handle.as_raw_handle(),
                            0,
                        )
                    },
                    windows_sys::Win32::Foundation::WAIT_TIMEOUT,
                    "fixture must remain live until the parent pins its identity"
                );
                // Open while the acknowledged child is still alive. A PID read
                // only after capture returns can identify a reused process.
                Some(Self { handle })
            }
            #[cfg(unix)]
            {
                Some(Self { pid })
            }
        }

        fn assert_stopped(&self) {
            #[cfg(windows)]
            {
                use std::os::windows::io::AsRawHandle;
                let immediate = unsafe {
                    windows_sys::Win32::System::Threading::WaitForSingleObject(
                        self.handle.as_raw_handle(),
                        0,
                    )
                };
                if immediate != windows_sys::Win32::Foundation::WAIT_OBJECT_0 {
                    let later = unsafe {
                        windows_sys::Win32::System::Threading::WaitForSingleObject(
                            self.handle.as_raw_handle(),
                            2000,
                        )
                    };
                    panic!("pinned native descendant remained live after capture; later completion={}", later == windows_sys::Win32::Foundation::WAIT_OBJECT_0);
                }
            }
            #[cfg(unix)]
            assert_child_stopped(self.pid);
        }
    }

    #[cfg(unix)]
    fn assert_child_stopped(pid: u32) {
        {
            let deadline = Instant::now() + Duration::from_secs(2);
            loop {
                // A reparented descendant can remain a zombie briefly until its
                // system reaper runs; this test requires disappearance as well.
                if unsafe { libc::kill(pid as libc::pid_t, 0) } != 0 {
                    break;
                }
                assert!(
                    Instant::now() < deadline,
                    "native fixture identity remained live"
                );
                std::thread::sleep(Duration::from_millis(5));
            }
        }
    }

    #[test]
    #[ignore = "only executed by the bounded process regression parent"]
    fn amx_process_child_fixture() {
        match std::env::var("AMX_PROCESS_FIXTURE").as_deref() {
            #[cfg(unix)]
            Ok("interactive-group") => {
                // SAFETY: getpgrp reads the calling process's group identity.
                let group = unsafe { libc::getpgrp() };
                std::fs::write(
                    std::env::var_os("AMX_PROCESS_READY").unwrap(),
                    group.to_string(),
                )
                .unwrap();
            }
            #[cfg(windows)]
            Ok("completion-resources") => completion_resources(),
            Ok("success") => {
                print!("AMX_PAYLOAD_BEGIN\nfixture & output\nAMX_PAYLOAD_END");
                std::io::stderr().write_all(b"fixture diagnostic").unwrap();
            }
            Ok("failure") => std::process::exit(7),
            Ok("upload-echo") => {
                let mut input = Vec::new();
                std::io::stdin()
                    .take(65537)
                    .read_to_end(&mut input)
                    .unwrap();
                assert!(input.len() <= 65536);
                std::io::stdout().write_all(&input).unwrap();
                std::io::stdout().flush().unwrap();
                std::process::exit(7);
            }
            Ok("stdout-flood") => {
                std::io::stdout()
                    .write_all(&vec![b'x'; 1024 * 1024])
                    .unwrap();
            }
            Ok("stderr-flood") => {
                std::io::stderr()
                    .write_all(&vec![b'x'; 1024 * 1024])
                    .unwrap();
            }
            Ok("wait") => {
                let ready = std::env::var_os("AMX_PROCESS_READY").unwrap();
                std::fs::write(ready, std::process::id().to_string()).unwrap();
                std::thread::park_timeout(Duration::from_secs(30));
            }
            Ok("lease") => {
                std::fs::write(std::env::var_os("AMX_PROCESS_READY").unwrap(), b"ready")
                    .unwrap();
                let mut input = Vec::new();
                std::io::stdin().read_to_end(&mut input).unwrap();
                assert!(input.is_empty());
                std::fs::write(
                    std::env::var_os("AMX_PROCESS_RETIRED").unwrap(),
                    b"retired",
                )
                .unwrap();
            }
            Ok("signal-owner") => {
                let cancellation = Cancellation::install().unwrap();
                std::fs::write(
                    std::env::var_os("AMX_PROCESS_OWNER").unwrap(),
                    std::process::id().to_string(),
                )
                .unwrap();
                let error =
                    capture(fixture("wait"), limits(), || cancellation.cancelled())
                        .err()
                        .unwrap();
                assert_eq!(error.kind(), io::ErrorKind::Interrupted);
            }
            Ok("native-persistent-parent") => {
                let mut child = fixture("native-persistent-helper");
                child
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null());
                let _child = child.spawn().unwrap();
                let release = std::env::var_os("AMX_PROCESS_RELEASE").unwrap();
                let deadline = Instant::now() + Duration::from_secs(5);
                while !std::path::Path::new(&release).is_file() {
                    assert!(Instant::now() < deadline, "native leader release timed out");
                    std::thread::sleep(POLL_INTERVAL);
                }
                std::process::exit(7);
            }
            Ok("native-persistent-helper") => {
                std::fs::write(
                    std::env::var_os("AMX_PROCESS_READY").unwrap(),
                    std::process::id().to_string(),
                )
                .unwrap();
                let release = std::env::var_os("AMX_PROCESS_BACKGROUND_RELEASE").unwrap();
                let deadline = Instant::now() + Duration::from_secs(5);
                while !std::path::Path::new(&release).is_file() {
                    if Instant::now() >= deadline {
                        return;
                    }
                    std::thread::sleep(POLL_INTERVAL);
                }
                std::fs::write(
                    std::env::var_os("AMX_PROCESS_RETIRED").unwrap(),
                    b"retired",
                )
                .unwrap();
            }
            Ok("descendant") | Ok("descendant-no-pipes") => {
                let ready = std::env::var_os("AMX_PROCESS_READY").unwrap();
                let release = std::env::var_os("AMX_PROCESS_RELEASE").unwrap();
                let mut command = fixture("wait");
                command.env("AMX_PROCESS_READY", &ready);
                if std::env::var("AMX_PROCESS_FIXTURE").unwrap() == "descendant-no-pipes"
                {
                    command.stdout(Stdio::null()).stderr(Stdio::null());
                }
                let _child = command.spawn().unwrap();
                let deadline = Instant::now() + Duration::from_secs(5);
                while !std::path::Path::new(&release).is_file() {
                    assert!(Instant::now() < deadline, "descendant readiness timed out");
                    std::thread::sleep(Duration::from_millis(5));
                }
                // Intentionally exit before the descendant: the supervising
                // parent must retire the whole native group, not only its leader.
                std::process::exit(0);
            }
            _ => panic!("fixture mode required"),
        }
    }
}
