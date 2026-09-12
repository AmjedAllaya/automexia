//! Bounded one-shot CLI process capture, separate from GUI/PTY ownership.
use process_wrap::std::{ChildWrapper, CommandWrap};
use std::{
    io::{self, Read},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

const CLEANUP_TIMEOUT: Duration = Duration::from_secs(2);
const POLL_INTERVAL: Duration = Duration::from_millis(5);
mod cancellation;
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
    capture_inner(command, limits, cancelled, false)
}

pub(crate) fn capture_leased(
    command: Command,
    limits: Limits,
    cancelled: impl Fn() -> bool,
) -> io::Result<Captured> {
    capture_inner(command, limits, cancelled, true)
}

fn capture_inner(
    mut command: Command,
    limits: Limits,
    cancelled: impl Fn() -> bool,
    lease: bool,
) -> io::Result<Captured> {
    if limits.timeout.is_zero()
        || limits.timeout > Duration::from_secs(60)
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
        .stdin(if lease { Stdio::piped() } else { Stdio::null() })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut command = CommandWrap::from(command);
    #[cfg(unix)]
    command.wrap(process_wrap::std::ProcessGroup::leader());
    #[cfg(windows)]
    {
        // JobObject intentionally replaces Command's raw creation flags. Use
        // its composable flag owner so cancellation never targets the caller's
        // shared console group and job assignment remains suspended/atomic.
        let mut flags = process_wrap::std::CreationFlags(Default::default());
        flags.0 .0 = windows_sys::Win32::System::Threading::CREATE_NEW_PROCESS_GROUP;
        command.wrap(flags).wrap(process_wrap::std::JobObject);
    }
    let mut child = OwnedChild { inner: command.spawn().map_err(|_| io::Error::new(io::ErrorKind::NotFound, "local tool could not be started; check that the required tool is installed"))?, reaped: false, killed: false };
    let result = (|| {
        let mut stdout = child.inner.stdout().take().ok_or_else(pipe_error)?;
        let mut stderr = child.inner.stderr().take().ok_or_else(pipe_error)?;
        prepare_pipe(&stdout)?;
        prepare_pipe(&stderr)?;
        let mut out = Vec::new();
        let mut err = Vec::new();
        let mut out_eof = false;
        let mut err_eof = false;
        let mut status = None;
        let deadline = Instant::now() + limits.timeout;
        loop {
            if cancelled() {
                return Err(cancelled_error());
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
                err_eof = drain(&mut stderr, &mut err, limits.stderr)?;
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
    child.cleanup()?;
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
}

impl OwnedChild {
    fn retire(&mut self) -> io::Result<()> {
        if !self.killed && !self.reaped {
            self.inner.start_kill().map_err(|_| cleanup_error())?;
            self.killed = true;
        }
        Ok(())
    }

    fn cleanup(&mut self) -> io::Result<()> {
        if self.reaped {
            return Ok(());
        }
        let lease = self.inner.stdin().take();
        let had_lease = lease.is_some();
        drop(lease);
        if had_lease {
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
            if self
                .inner
                .try_wait()
                .map_err(|_| cleanup_error())?
                .is_some()
            {
                self.reaped = true;
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
            let mut command = fixture("wait");
            command.env("AMX_PROCESS_READY", &ready);
            let limit = Limits {
                timeout: Duration::from_secs(3),
                ..limits()
            };
            let begin = Instant::now();
            let result = capture(command, limit, || !timed_out && ready.exists());
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
            // The identity is produced only in an isolated native fixture. It is
            // consumed locally and never emitted in reports or persisted examples.
            let pid: u32 = std::fs::read_to_string(&ready).unwrap().parse().unwrap();
            assert_child_stopped(pid);
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
        let result = capture(command, limits(), || {
            if !sent.get() && ready.is_file() {
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
        let pid: u32 = std::fs::read_to_string(&ready).unwrap().parse().unwrap();
        assert_child_stopped(pid);
    }

    #[test]
    fn amx_process_leader_exit_retires_descendant_holding_pipes() {
        let temporary = tempfile::tempdir().unwrap();
        let ready = temporary.path().join("ready");
        let mut command = fixture("descendant");
        command.env("AMX_PROCESS_READY", &ready);
        let output = capture(command, limits(), || false).unwrap();
        assert!(output.status.success());
        assert!(ready.is_file());
        let pid: u32 = std::fs::read_to_string(&ready).unwrap().parse().unwrap();
        assert_child_stopped(pid);
    }

    fn assert_child_stopped(pid: u32) {
        #[cfg(windows)]
        {
            use windows_sys::Win32::{
                Foundation::{CloseHandle, WAIT_OBJECT_0},
                System::Threading::{
                    OpenProcess, WaitForSingleObject, PROCESS_SYNCHRONIZE,
                },
            };
            // Open by the captured identity immediately; never terminate by PID.
            let handle = unsafe { OpenProcess(PROCESS_SYNCHRONIZE, 0, pid) };
            if !handle.is_null() {
                let stopped = unsafe { WaitForSingleObject(handle, 0) } == WAIT_OBJECT_0;
                unsafe {
                    CloseHandle(handle);
                }
                assert!(stopped, "native fixture process is still running");
            }
        }
        #[cfg(unix)]
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
            Ok("success") => {
                print!("AMX_PAYLOAD_BEGIN\nfixture & output\nAMX_PAYLOAD_END");
                std::io::stderr().write_all(b"fixture diagnostic").unwrap();
            }
            Ok("failure") => std::process::exit(7),
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
            Ok("descendant") => {
                let ready = std::env::var_os("AMX_PROCESS_READY").unwrap();
                let mut command = fixture("wait");
                command.env("AMX_PROCESS_READY", &ready);
                let _child = command.spawn().unwrap();
                let deadline = Instant::now() + Duration::from_secs(5);
                while !std::path::Path::new(&ready).is_file() {
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
