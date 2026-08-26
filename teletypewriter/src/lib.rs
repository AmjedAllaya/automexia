extern crate libc;

#[cfg(not(windows))]
mod unix;
#[cfg(not(windows))]
pub use self::unix::*;

#[cfg(windows)]
pub mod windows;
#[cfg(windows)]
pub use self::windows::*;

use std::io;
use std::path::{Path, PathBuf};

/// An executable opened by absolute path and held stable until native process
/// creation finishes. The opaque identity lets an application policy compare
/// the file it reviewed with the file this launch seam will execute.
pub struct ExactExecutable {
    file: std::fs::File,
    path: PathBuf,
    identity: ExactExecutableIdentity,
}

#[derive(Clone, PartialEq, Eq)]
pub struct ExactExecutableIdentity {
    canonical_path: PathBuf,
    platform: ExactExecutablePlatformIdentity,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum ExactExecutablePlatformIdentity {
    #[cfg(unix)]
    Unix {
        device: u64,
        inode: u64,
        size: u64,
        modified_seconds: i64,
        modified_nanoseconds: i64,
    },
    #[cfg(windows)]
    Windows {
        volume_serial: u32,
        file_index: u64,
        size: u64,
        last_write: u64,
    },
}

impl std::fmt::Debug for ExactExecutable {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ExactExecutable")
            .field("path", &"<redacted>")
            .field("identity", &"<redacted>")
            .finish()
    }
}

impl std::fmt::Debug for ExactExecutableIdentity {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("ExactExecutableIdentity(<redacted>)")
    }
}

impl ExactExecutable {
    pub fn open(path: &Path) -> io::Result<Self> {
        exact_executable::open(path)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn identity(&self) -> &ExactExecutableIdentity {
        &self.identity
    }
}

mod exact_executable;

#[repr(C)]
pub struct Winsize {
    ws_row: libc::c_ushort,
    ws_col: libc::c_ushort,
    ws_xpixel: libc::c_ushort,
    ws_ypixel: libc::c_ushort,
}

pub trait ProcessReadWrite {
    type Reader: io::Read;
    type Writer: io::Write;
    fn reader(&mut self) -> &mut Self::Reader;
    fn read_token(&self) -> corcovado::Token;
    fn writer(&mut self) -> &mut Self::Writer;
    fn write_token(&self) -> corcovado::Token;
    fn set_winsize(&mut self, _: WinsizeBuilder) -> Result<(), io::Error>;

    fn register(
        &mut self,
        _: &corcovado::Poll,
        _: &mut dyn Iterator<Item = corcovado::Token>,
        _: corcovado::Ready,
        _: corcovado::PollOpt,
    ) -> io::Result<()>;
    fn reregister(
        &mut self,
        _: &corcovado::Poll,
        _: corcovado::Ready,
        _: corcovado::PollOpt,
    ) -> io::Result<()>;
    fn deregister(&mut self, _: &corcovado::Poll) -> io::Result<()>;
}

#[derive(Debug, PartialEq, Eq)]
pub enum ChildEvent {
    /// Indicates the child has exited, with the raw wait status when the
    /// platform makes it available (interpret with
    /// `std::process::ExitStatus::from_raw` / `ExitStatusExt`).
    Exited(Option<i32>),
}

/// Confirmed terminal state for an application-owned exact-launch process tree.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ManagedPtyShutdown {
    /// The PTY was not created through the guarded exact-launch seam.
    NotManaged,
    /// The owned tree exited within the graceful cancellation interval.
    Graceful,
    /// The owned tree required the platform containment primitive's force phase.
    Forced,
}

pub trait EventedPty: ProcessReadWrite {
    fn child_event_token(&self) -> corcovado::Token;

    /// Tries to retrieve an event.
    ///
    /// Returns `Some(event)` on success, or `None` if there are no events to retrieve.
    fn next_child_event(&mut self) -> Option<ChildEvent>;

    /// Gracefully stop, then forcibly contain, one guarded exact-launch tree.
    fn shutdown_owned_process_tree(&mut self) -> io::Result<ManagedPtyShutdown>;
}

#[derive(Debug, Clone)]
pub struct WinsizeBuilder {
    pub rows: u16,
    pub cols: u16,
    pub width: u16,
    pub height: u16,
}

impl WinsizeBuilder {
    fn build(&self) -> Winsize {
        let ws_row = self.rows as libc::c_ushort;
        let ws_col = self.cols as libc::c_ushort;
        let ws_xpixel = self.width as libc::c_ushort;
        let ws_ypixel = self.height as libc::c_ushort;

        Winsize {
            ws_row,
            ws_col,
            ws_xpixel,
            ws_ypixel,
        }
    }
}

#[cfg(test)]
mod exact_executable_tests {
    use super::ExactExecutable;

    #[test]
    fn rejects_relative_executable_paths() {
        let error = ExactExecutable::open(std::path::Path::new("ssh")).unwrap_err();
        assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
    }

    #[cfg(windows)]
    #[test]
    fn windows_guard_prevents_replacement_until_launch_finishes() {
        use std::io::Write;

        let directory = tempfile::tempdir().unwrap();
        let executable = directory.path().join("ssh.exe");
        std::fs::File::create(&executable)
            .unwrap()
            .write_all(b"first")
            .unwrap();

        let guarded = ExactExecutable::open(&executable).unwrap();
        assert!(std::fs::remove_file(&executable).is_err());
        drop(guarded);
        std::fs::remove_file(&executable).unwrap();
    }
}
