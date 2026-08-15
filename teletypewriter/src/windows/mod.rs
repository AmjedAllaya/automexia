mod child;
mod conpty;
mod pipes;
mod spsc;

use std::ffi::OsStr;
use std::io::{self};
use std::iter::once;
use std::os::windows::ffi::OsStrExt;
use std::os::windows::process::CommandExt;
use std::process::{Command, Stdio};
use std::sync::mpsc::TryRecvError;

use crate::windows::child::ChildExitWatcher;
use crate::{ChildEvent, EventedPty, ProcessReadWrite, Winsize, WinsizeBuilder};
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
}

// Creates conpty instead of pty
// Windows Pseudo Console (ConPTY)
//
// `env`, when given, is applied on top of the inherited environment,
// overriding inherited variables of the same name. `None` inherits as-is.
//
// `shell` of `None` means no program was configured, and the default console
// host is used.
pub fn create_pty(
    shell: Option<&str>,
    args: Vec<String>,
    working_directory: &Option<String>,
    env: Option<Vec<(String, String)>>,
    columns: u16,
    rows: u16,
) -> Result<Pty, std::io::Error> {
    let exec = shell.map(|shell| build_command_line(shell, &args));
    conpty::new(exec.as_deref(), working_directory, env, columns, rows)
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
    ) -> Self {
        Self {
            backend: backend.into(),
            conout: conout.into(),
            conin: conin.into(),
            read_token: 0.into(),
            write_token: 0.into(),
            child_event_token: 0.into(),
            child_watcher,
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
        self.backend.on_resize(winsize);
        Ok(())
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
            "/home/lamjed/work tree/项目".to_string(),
            "say \"hello\"".to_string(),
            r"C:\trailing path\".to_string(),
            String::new(),
        ];
        assert_eq!(
            build_command_line(r"C:\Program Files\PowerShell\7\pwsh.exe", &args),
            r#""C:\Program Files\PowerShell\7\pwsh.exe" --cd "/home/lamjed/work tree/项目" "say \"hello\"" "C:\trailing path\\" """#
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
