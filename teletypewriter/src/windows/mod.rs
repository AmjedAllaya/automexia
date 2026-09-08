mod child;
mod conpty;
mod pipes;
mod spsc;

use std::ffi::OsStr;
use std::io::{self, Write};
use std::iter::once;
use std::os::windows::ffi::OsStrExt;
use std::os::windows::process::CommandExt;
use std::process::{Command, Stdio};
use std::sync::mpsc::TryRecvError;
use std::time::{Duration, Instant};

use crate::windows::child::ChildExitWatcher;
use crate::{
    ChildEvent, EventedPty, ExactExecutable, ManagedPtyShutdown, ProcessReadWrite,
    Winsize, WinsizeBuilder,
};
use windows_sys::Win32::System::Threading::{CREATE_NEW_PROCESS_GROUP, CREATE_NO_WINDOW};

use conpty::Conpty as Backend;
use pipes::{EventedAnonRead as ReadPipe, EventedAnonWrite as WritePipe};

pub struct Pty {
    // Backend is required to be the first field, to ensure correct drop order. Dropping
    // `conout` before `backend` will cause a deadlock (with Conpty).
    backend: Backend,
    conout: ReadPipe,
    conin: WritePipe,
    read_token: corcovado::Token,
    write_token: corcovado::Token,
    child_event_token: corcovado::Token,
    child_watcher: ChildExitWatcher,
    managed_owned_tree: bool,
}

impl Drop for Pty {
    fn drop(&mut self) {
        // Backend drops first and may wait for conout. A live reader alone is
        // insufficient when its bounded ring is full and VT has stopped.
        self.conout.discard_remaining();
    }
}

// Creates conpty instead of pty
// Windows Pseudo Console (ConPTY)
//
// `env`, when given, is applied on top of the inherited environment,
// overriding inherited variables of the same name. `None` inherits as-is.
//
// `shell` of `None` means no program was configured, and the default console
// host is used. Every ordinary terminal session remains in a kill-on-close Job
// Object so closing its owning route cannot detach the shell or descendants.
pub fn create_pty(
    shell: Option<&str>,
    args: Vec<String>,
    working_directory: &Option<String>,
    env: Option<Vec<(String, String)>>,
    columns: u16,
    rows: u16,
) -> Result<Pty, std::io::Error> {
    let exec = shell.map(|shell| build_command_line(shell, &args));
    conpty::new(
        None,
        exec.as_deref(),
        working_directory,
        env,
        true,
        true,
        columns,
        rows,
    )
}

/// Create one managed PTY from an already-opened exact executable.
///
/// The guard remains live through native process creation. The child receives
/// only the supplied application-owned environment and is suspended until it
/// is assigned to a kill-on-close Job Object.
pub fn create_exact_pty(
    executable: ExactExecutable,
    args: Vec<String>,
    working_directory: &Option<String>,
    environment: Vec<(String, String)>,
    columns: u16,
    rows: u16,
) -> Result<Pty, std::io::Error> {
    let program = executable.path().to_str().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "the exact executable path is not valid Unicode",
        )
    })?;
    let command_line = build_command_line(program, &args);
    let _replacement_guard = &executable.file;
    conpty::new(
        Some(program),
        Some(&command_line),
        working_directory,
        Some(environment),
        false,
        true,
        columns,
        rows,
    )
}

/// Build the single UTF-16 command line consumed by CreateProcessW using the
/// documented CommandLineToArgvW/MSVC escaping rules. Joining argv with spaces
/// corrupts paths, commands, and Unicode-adjacent quoting as soon as an
/// argument contains whitespace or a literal quote.
fn build_command_line(program: &str, args: &[String]) -> String {
    std::iter::once(program)
        .chain(args.iter().map(String::as_str))
        .map(quote_windows_argument)
        .collect::<Vec<_>>()
        .join(" ")
}

fn quote_windows_argument(argument: &str) -> String {
    if !argument.is_empty()
        && !argument
            .chars()
            .any(|character| character.is_whitespace() || character == '"')
    {
        return argument.to_string();
    }

    let mut quoted = String::with_capacity(argument.len() + 2);
    quoted.push('"');
    let mut backslashes = 0usize;
    for character in argument.chars() {
        match character {
            '\\' => backslashes += 1,
            '"' => {
                quoted.extend(std::iter::repeat_n('\\', backslashes * 2 + 1));
                quoted.push('"');
                backslashes = 0;
            }
            _ => {
                quoted.extend(std::iter::repeat_n('\\', backslashes));
                backslashes = 0;
                quoted.push(character);
            }
        }
    }
    // Backslashes immediately before the closing quote must be doubled.
    quoted.extend(std::iter::repeat_n('\\', backslashes * 2));
    quoted.push('"');
    quoted
}

impl Pty {
    fn new(
        backend: impl Into<Backend>,
        conout: impl Into<ReadPipe>,
        conin: impl Into<WritePipe>,
        child_watcher: ChildExitWatcher,
        managed_owned_tree: bool,
    ) -> Self {
        Self {
            backend: backend.into(),
            conout: conout.into(),
            conin: conin.into(),
            read_token: 0.into(),
            write_token: 0.into(),
            child_event_token: 0.into(),
            child_watcher,
            managed_owned_tree,
        }
    }

    pub fn child_watcher(&self) -> &ChildExitWatcher {
        &self.child_watcher
    }
}

impl ProcessReadWrite for Pty {
    type Reader = ReadPipe;
    type Writer = WritePipe;

    #[inline]
    fn register(
        &mut self,
        poll: &corcovado::Poll,
        token: &mut dyn Iterator<Item = corcovado::Token>,
        interest: corcovado::Ready,
        poll_opts: corcovado::PollOpt,
    ) -> io::Result<()> {
        self.read_token = token.next().unwrap();
        self.write_token = token.next().unwrap();

        if interest.is_readable() {
            poll.register(
                &self.conout,
                self.read_token,
                corcovado::Ready::readable(),
                poll_opts,
            )?
        } else {
            poll.register(
                &self.conout,
                self.read_token,
                corcovado::Ready::empty(),
                poll_opts,
            )?
        }
        if interest.is_writable() {
            poll.register(
                &self.conin,
                self.write_token,
                corcovado::Ready::writable(),
                poll_opts,
            )?
        } else {
            poll.register(
                &self.conin,
                self.write_token,
                corcovado::Ready::empty(),
                poll_opts,
            )?
        }

        self.child_event_token = token.next().unwrap();
        poll.register(
            self.child_watcher.event_rx(),
            self.child_event_token,
            corcovado::Ready::readable(),
            poll_opts,
        )?;

        Ok(())
    }

    #[inline]
    fn reregister(
        &mut self,
        poll: &corcovado::Poll,
        interest: corcovado::Ready,
        poll_opts: corcovado::PollOpt,
    ) -> io::Result<()> {
        if interest.is_readable() {
            poll.reregister(
                &self.conout,
                self.read_token,
                corcovado::Ready::readable(),
                poll_opts,
            )?;
        } else {
            poll.reregister(
                &self.conout,
                self.read_token,
                corcovado::Ready::empty(),
                poll_opts,
            )?;
        }
        if interest.is_writable() {
            poll.reregister(
                &self.conin,
                self.write_token,
                corcovado::Ready::writable(),
                poll_opts,
            )?;
        } else {
            poll.reregister(
                &self.conin,
                self.write_token,
                corcovado::Ready::empty(),
                poll_opts,
            )?;
        }

        poll.reregister(
            self.child_watcher.event_rx(),
            self.child_event_token,
            corcovado::Ready::readable(),
            poll_opts,
        )?;

        Ok(())
    }

    #[inline]
    fn deregister(&mut self, poll: &corcovado::Poll) -> io::Result<()> {
        poll.deregister(&self.conout)?;
        poll.deregister(&self.conin)?;
        poll.deregister(self.child_watcher.event_rx())?;
        Ok(())
    }

    #[inline]
    fn reader(&mut self) -> &mut Self::Reader {
        &mut self.conout
    }

    #[inline]
    fn read_token(&self) -> corcovado::Token {
        self.read_token
    }

    #[inline]
    fn writer(&mut self) -> &mut Self::Writer {
        &mut self.conin
    }

    #[inline]
    fn write_token(&self) -> corcovado::Token {
        self.write_token
    }

    #[inline]
    fn set_winsize(
        &mut self,
        winsize_builder: WinsizeBuilder,
    ) -> Result<(), std::io::Error> {
        let winsize: Winsize = winsize_builder.build();
        self.backend.on_resize(winsize)
    }
}

impl EventedPty for Pty {
    fn child_event_token(&self) -> corcovado::Token {
        self.child_event_token
    }

    fn next_child_event(&mut self) -> Option<ChildEvent> {
        match self.child_watcher.event_rx().try_recv() {
            Ok(ev) => Some(ev),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => Some(ChildEvent::Exited(None)),
        }
    }

    fn shutdown_owned_process_tree(&mut self) -> io::Result<ManagedPtyShutdown> {
        if !self.managed_owned_tree {
            return Ok(ManagedPtyShutdown::NotManaged);
        }
        self.conout.discard_remaining();
        let _ = self.conin.write_all(&[0x03]);
        if wait_for_job_empty(&self.backend, Duration::from_secs(2))? {
            return Ok(ManagedPtyShutdown::Graceful);
        }
        self.backend.terminate_managed_job()?;
        if wait_for_job_empty(&self.backend, Duration::from_secs(3))? {
            Ok(ManagedPtyShutdown::Forced)
        } else {
            Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "the managed Job Object did not terminate within the force budget",
            ))
        }
    }
}

fn wait_for_job_empty(backend: &Backend, timeout: Duration) -> io::Result<bool> {
    let deadline = Instant::now() + timeout;
    loop {
        if backend.managed_job_is_empty()? {
            return Ok(true);
        }
        if Instant::now() >= deadline {
            return Ok(false);
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn cmdline(shell: Option<&str>) -> String {
    if let Some(shell) = shell.filter(|shell| !shell.is_empty()) {
        return shell.to_string();
    }

    once("powershell")
        // .chain(shell.args().iter().map(|a| a.as_ref()))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Converts the string slice into a Windows-standard representation for "W"-
/// suffixed function variants, which accept UTF-16 encoded string values.
pub fn win32_string<S: AsRef<OsStr> + ?Sized>(value: &S) -> Vec<u16> {
    OsStr::new(value).encode_wide().chain(once(0)).collect()
}

pub fn spawn_daemon<I, S>(program: &str, args: I) -> io::Result<()>
where
    I: IntoIterator<Item = S> + Copy,
    S: AsRef<OsStr>,
{
    Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .creation_flags(CREATE_NEW_PROCESS_GROUP | CREATE_NO_WINDOW)
        .spawn()
        .map(|_| ())
}

#[cfg(test)]
mod command_line_tests {
    use super::*;

    #[test]
    fn quotes_program_arguments_spaces_unicode_and_embedded_quotes() {
        let args = vec![
            "--cd".to_string(),
            "/home/alice/work tree/项目".to_string(),
            "say \"hello\"".to_string(),
            r"C:\trailing path\".to_string(),
            String::new(),
        ];
        assert_eq!(
            build_command_line(r"C:\Program Files\PowerShell\7\pwsh.exe", &args),
            r#""C:\Program Files\PowerShell\7\pwsh.exe" --cd "/home/alice/work tree/项目" "say \"hello\"" "C:\trailing path\\" """#
        );
    }

    #[test]
    fn leaves_simple_arguments_unquoted() {
        assert_eq!(
            build_command_line("wsl.exe", &["--distribution".into(), "Ubuntu".into()]),
            "wsl.exe --distribution Ubuntu"
        );
    }
}

#[cfg(test)]
mod exact_spawn_tests {
    use std::path::PathBuf;
    use std::time::Duration;

    use corcovado::{event::Events, Poll, PollOpt, Ready, Token};

    use super::*;

    #[test]
    fn native_conpty_saturated_shutdown_and_drop_release_exact_processes() {
        use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
        use windows_sys::Win32::Foundation::WAIT_OBJECT_0;
        use windows_sys::Win32::System::Threading::{OpenProcess, WaitForSingleObject};

        let script = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/shutdown-output.ps1");
        for explicit_shutdown in [false, true, false, true] {
            let mut pty = create_pty(
                Some("powershell.exe"),
                vec![
                    "-NoLogo".into(),
                    "-NoProfile".into(),
                    "-NonInteractive".into(),
                    "-File".into(),
                    script.to_string_lossy().into_owned(),
                ],
                &None,
                None,
                80,
                24,
            )
            .expect("native output fixture starts");
            // Preserve the kernel identity before teardown, not a later PID
            // lookup. Querying saturation observes the actual native pipe.
            // SAFETY: the live watcher supplies the just-created child identity;
            // request synchronization only, with no borrowed handle ownership.
            let process = unsafe {
                OpenProcess(0x0010_0000, 0, pty.child_watcher.pid().unwrap().get())
            };
            assert!(!process.is_null(), "fixture synchronization handle");
            // SAFETY: OpenProcess transferred one owned non-null handle.
            let process = unsafe { OwnedHandle::from_raw_handle(process) };
            let deadline = Instant::now() + Duration::from_secs(10);
            while !pty.conout.is_saturated() && Instant::now() < deadline {
                std::thread::sleep(Duration::from_millis(1));
            }
            let saturated = pty.conout.is_saturated();
            let started = Instant::now();
            if explicit_shutdown {
                assert_ne!(
                    pty.shutdown_owned_process_tree().unwrap(),
                    ManagedPtyShutdown::NotManaged
                );
                assert!(pty.backend.managed_job_is_empty().unwrap());
                // Repeated requests must not add another grace wait.
                assert_eq!(
                    pty.shutdown_owned_process_tree().unwrap(),
                    ManagedPtyShutdown::Graceful
                );
            }
            drop(pty);
            let remaining = Duration::from_secs(10).saturating_sub(started.elapsed());
            assert_eq!(
                unsafe {
                    WaitForSingleObject(
                        process.as_raw_handle(),
                        remaining.as_millis() as u32,
                    )
                },
                WAIT_OBJECT_0
            );
            assert!(saturated, "real ConPTY output filled its consumer ring");
            assert!(started.elapsed() < Duration::from_secs(10));
            eprintln!("native saturated ConPTY close (explicit={explicit_shutdown}): {} microseconds; exact process exited", started.elapsed().as_micros());
        }
    }

    #[test]
    fn exact_spawn_uses_explicit_program_and_does_not_inherit_path() {
        let system_root = std::env::var("SystemRoot").unwrap();
        let executable = crate::ExactExecutable::open(
            &PathBuf::from(&system_root).join("System32/cmd.exe"),
        )
        .unwrap();
        let pty = create_exact_pty(
            executable,
            vec![
                "/D".into(),
                "/S".into(),
                "/C".into(),
                "if defined PATH (exit 73) else (exit 0)".into(),
            ],
            &None,
            vec![("SystemRoot".into(), system_root)],
            80,
            24,
        )
        .unwrap();

        let mut events = Events::with_capacity(1);
        let poll = Poll::new().unwrap();
        poll.register(
            pty.child_watcher().event_rx(),
            Token::from(0usize),
            Ready::readable(),
            PollOpt::oneshot(),
        )
        .unwrap();
        poll.poll(&mut events, Some(Duration::from_secs(5)))
            .unwrap();

        assert!(matches!(
            pty.child_watcher().event_rx().try_recv(),
            Ok(ChildEvent::Exited(Some(0)))
        ));
    }
}
