//! Content-free observation of the real adapter; no fake input, output or exit.
use std::io::{self, Read, Write};
use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Arc, Mutex,
};
use teletypewriter::{
    ChildEvent, EventedPty, ManagedPtyShutdown, ProcessReadWrite, WinsizeBuilder,
};

use std::os::windows::io::{AsRawHandle, BorrowedHandle, OwnedHandle};
use windows_sys::Win32::Foundation::{WAIT_OBJECT_0, WAIT_TIMEOUT};
use windows_sys::Win32::System::Threading::WaitForSingleObject;

/// Pin the exact child object, not a PID that another process could reuse.
pub struct ChildExitProbe(OwnedHandle);
impl ChildExitProbe {
    pub fn new(pty: &teletypewriter::Pty) -> Self {
        // SAFETY: The PTY owns a live process handle throughout this borrow;
        // duplication completes before the PTY is moved to the worker.
        let child =
            unsafe { BorrowedHandle::borrow_raw(pty.child_watcher().raw_handle()) };
        Self(
            child
                .try_clone_to_owned()
                .expect("pin fixture child identity"),
        )
    }
    pub fn exited(&self) -> Option<bool> {
        // SAFETY: The owned duplicate remains valid; zero timeout never waits.
        match unsafe { WaitForSingleObject(self.0.as_raw_handle(), 0) } {
            WAIT_OBJECT_0 => Some(true),
            WAIT_TIMEOUT => Some(false),
            _ => None,
        }
    }
}

#[derive(Debug, Default)]
pub struct Observation {
    // Acceptance into the adapter is not proof of native child consumption.
    accepted_input_bytes: AtomicUsize,
    read_failure: Mutex<Option<io::ErrorKind>>,
    write_failure: Mutex<Option<io::ErrorKind>>,
    child_exit_observed: AtomicBool,
}

pub struct ObservedPty<T> {
    inner: T,
    observation: Arc<Observation>,
}

impl<T> ObservedPty<T> {
    pub fn new(inner: T) -> (Self, Arc<Observation>) {
        let observation = Arc::new(Observation::default());
        (
            Self {
                inner,
                observation: observation.clone(),
            },
            observation,
        )
    }
}

fn record_failure(result: &io::Result<usize>, failure: &Mutex<Option<io::ErrorKind>>) {
    if let Err(error) = result {
        if !matches!(
            error.kind(),
            io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
        ) {
            *failure.lock().unwrap() = Some(error.kind());
        }
    }
}
impl<T: ProcessReadWrite> Read for ObservedPty<T> {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        let result = self.inner.reader().read(bytes);
        record_failure(&result, &self.observation.read_failure);
        result
    }
}
impl<T: ProcessReadWrite> Write for ObservedPty<T> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let result = self.inner.writer().write(bytes);
        if let Ok(count) = &result {
            self.observation
                .accepted_input_bytes
                .fetch_add(*count, Ordering::Relaxed);
        }
        record_failure(&result, &self.observation.write_failure);
        result
    }
    fn flush(&mut self) -> io::Result<()> {
        self.inner.writer().flush()
    }
}
impl<T: ProcessReadWrite> ProcessReadWrite for ObservedPty<T> {
    type Reader = Self;
    type Writer = Self;
    fn reader(&mut self) -> &mut Self {
        self
    }
    fn writer(&mut self) -> &mut Self {
        self
    }
    fn read_token(&self) -> corcovado::Token {
        self.inner.read_token()
    }
    fn write_token(&self) -> corcovado::Token {
        self.inner.write_token()
    }
    fn set_winsize(&mut self, size: WinsizeBuilder) -> io::Result<()> {
        self.inner.set_winsize(size)
    }
    fn register(
        &mut self,
        poll: &corcovado::Poll,
        tokens: &mut dyn Iterator<Item = corcovado::Token>,
        interest: corcovado::Ready,
        options: corcovado::PollOpt,
    ) -> io::Result<()> {
        self.inner.register(poll, tokens, interest, options)
    }
    fn reregister(
        &mut self,
        poll: &corcovado::Poll,
        interest: corcovado::Ready,
        options: corcovado::PollOpt,
    ) -> io::Result<()> {
        self.inner.reregister(poll, interest, options)
    }
    fn deregister(&mut self, poll: &corcovado::Poll) -> io::Result<()> {
        self.inner.deregister(poll)
    }
}
impl<T: EventedPty> EventedPty for ObservedPty<T> {
    fn child_event_token(&self) -> corcovado::Token {
        self.inner.child_event_token()
    }
    fn next_child_event(&mut self) -> Option<ChildEvent> {
        let event = self.inner.next_child_event();
        if event.is_some() {
            self.observation
                .child_exit_observed
                .store(true, Ordering::Relaxed);
        }
        event
    }
    fn shutdown_owned_process_tree(&mut self) -> io::Result<ManagedPtyShutdown> {
        self.inner.shutdown_owned_process_tree()
    }
}
