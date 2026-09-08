use super::*;
use crate::ansi::CursorShape;
use crate::crosswords::CrosswordsSize;
use crate::event::VoidListener;
use teletypewriter::{
    ChildEvent, EventedPty, ManagedPtyShutdown, ProcessReadWrite, WinsizeBuilder,
};

#[derive(Default)]
struct BoundaryReader {
    final_bytes: Option<Vec<u8>>,
    before_eof: Option<mpsc::SyncSender<()>>,
    eof: Option<io::Error>,
}

impl Read for BoundaryReader {
    fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
        if let Some(bytes) = self.final_bytes.take() {
            assert!(bytes.len() <= output.len());
            output[..bytes.len()].copy_from_slice(&bytes);
            return Ok(bytes.len());
        }
        if let Some(before_eof) = self.before_eof.take() {
            before_eof.send(()).unwrap();
        }
        self.eof.take().map_or(Ok(0), Err)
    }
}

#[derive(Default)]
struct BoundaryPty {
    reader: BoundaryReader,
    output: Vec<u8>,
    writable_bytes: usize,
    sizes: Vec<(u16, u16)>,
    fail_resize: bool,
    shutdowns: usize,
}

impl Write for BoundaryPty {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if self.writable_bytes == 0 {
            return Err(ErrorKind::WouldBlock.into());
        }
        let count = bytes.len().min(self.writable_bytes);
        self.output.extend_from_slice(&bytes[..count]);
        self.writable_bytes -= count;
        Ok(count)
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl ProcessReadWrite for BoundaryPty {
    type Reader = BoundaryReader;
    type Writer = Self;
    fn reader(&mut self) -> &mut Self::Reader {
        &mut self.reader
    }
    fn writer(&mut self) -> &mut Self::Writer {
        self
    }
    fn read_token(&self) -> corcovado::Token {
        corcovado::Token(1)
    }
    fn write_token(&self) -> corcovado::Token {
        corcovado::Token(2)
    }
    fn set_winsize(&mut self, size: WinsizeBuilder) -> io::Result<()> {
        if std::mem::take(&mut self.fail_resize) {
            return Err(io::Error::other("injected resize rejection"));
        }
        self.sizes.push((size.cols, size.rows));
        Ok(())
    }
    fn register(
        &mut self,
        _: &corcovado::Poll,
        _: &mut dyn Iterator<Item = corcovado::Token>,
        _: Ready,
        _: PollOpt,
    ) -> io::Result<()> {
        Ok(())
    }
    fn reregister(
        &mut self,
        _: &corcovado::Poll,
        _: Ready,
        _: PollOpt,
    ) -> io::Result<()> {
        Ok(())
    }
    fn deregister(&mut self, _: &corcovado::Poll) -> io::Result<()> {
        Ok(())
    }
}

impl EventedPty for BoundaryPty {
    fn child_event_token(&self) -> corcovado::Token {
        corcovado::Token(3)
    }
    fn next_child_event(&mut self) -> Option<ChildEvent> {
        None
    }
    fn shutdown_owned_process_tree(&mut self) -> io::Result<ManagedPtyShutdown> {
        self.shutdowns += 1;
        Ok(ManagedPtyShutdown::Graceful)
    }
}

fn machine() -> Machine<BoundaryPty, VoidListener> {
    let terminal = Crosswords::new(
        CrosswordsSize::new(80, 24),
        CursorShape::Block,
        VoidListener {},
        WindowId::from(0),
        0,
        100,
    );
    Machine::new(
        Arc::new(FairMutex::new(terminal)),
        BoundaryPty::default(),
        VoidListener {},
        WindowId::from(0),
        0,
    )
    .unwrap()
}

#[test]
fn final_output_survives_eof_while_resize_owns_the_terminal_lock() {
    let mut endings = vec![None, Some(ErrorKind::WouldBlock.into())];
    if cfg!(windows) {
        endings.push(Some(ErrorKind::BrokenPipe.into()));
    }
    #[cfg(target_os = "linux")]
    endings.push(Some(io::Error::from_raw_os_error(libc::EIO)));
    for eof in endings {
        let label = format!("{eof:?}");
        let mut machine = machine();
        let terminal = machine.terminal.clone();
        let (before_eof, eof_reached) = mpsc::sync_channel(0);
        machine.pty.reader = BoundaryReader {
            final_bytes: Some(b"FINAL-OUTPUT".to_vec()),
            before_eof: Some(before_eof),
            eof,
        };
        // Hold the same core lock as a resize or frame extraction. The reader
        // must reach EOF with bytes pending, not parse before contention exists.
        let resize_lock = terminal.lock();
        let worker = std::thread::spawn(move || {
            machine.pty_read(&mut State::default(), &mut [0; 128], true)
        });
        eof_reached
            .recv_timeout(std::time::Duration::from_secs(5))
            .unwrap();
        drop(resize_lock);
        worker
            .join()
            .unwrap()
            .expect("EOF after final bytes is not an I/O failure");
        let terminal = terminal.lock();
        let text: String = terminal.grid[crate::crosswords::pos::Line(0)]
            .inner
            .iter()
            .take(12)
            .map(|cell| cell.c())
            .collect();
        assert_eq!(text, "FINAL-OUTPUT", "pending bytes survive {label}");
    }
}

#[test]
fn unrelated_reader_failure_is_not_hidden_as_eof() {
    let mut machine = machine();
    machine.pty.reader.eof = Some(ErrorKind::PermissionDenied.into());
    let error = machine
        .pty_read(&mut State::default(), &mut [0; 128], true)
        .expect_err("non-EOF failures remain observable");
    assert_eq!(error.kind(), ErrorKind::PermissionDenied);
}

#[test]
fn failed_duplicate_and_pixel_only_resizes_do_not_start_an_input_deadline() {
    let mut machine = machine();
    let sender = machine.channel();
    let mut state = State::default();
    machine.pty.fail_resize = true;
    sender.send(Msg::Resize(size(40, 12))).unwrap();
    assert!(machine.drain_recv_channel(&mut state));
    assert!(state.resize_input_not_before.is_none());
    assert_eq!(machine.terminal.lock().screen_lines(), 24);
    assert!(machine.last_window_size.is_none());
    sender.send(Msg::Resize(size(40, 12))).unwrap();
    let before = Instant::now();
    assert!(machine.drain_recv_channel(&mut state));
    assert_eq!(machine.terminal.lock().screen_lines(), 12);
    let deadline = state.resize_input_not_before;
    if cfg!(windows) {
        assert!(deadline.unwrap() >= before + std::time::Duration::from_millis(50));
        assert!(
            deadline.unwrap() <= Instant::now() + std::time::Duration::from_millis(50)
        );
    } else {
        assert!(deadline.is_none(), "Unix input has no compatibility delay");
    }
    for next in [
        size(40, 12),
        WindowSize {
            width: 321,
            ..size(40, 12)
        },
    ] {
        sender.send(Msg::Resize(next)).unwrap();
        assert!(machine.drain_recv_channel(&mut state));
        assert_eq!(state.resize_input_not_before, deadline);
    }
    assert_eq!(
        machine.pty.sizes,
        [(40, 12), (40, 12)],
        "duplicate omitted, pixel-only update delivered"
    );
}

#[test]
fn empty_input_does_not_hold_a_following_resize_behind_a_zero_byte_write() {
    let mut machine = machine();
    let sender = machine.channel();
    let mut state = State::default();
    sender.send(Msg::Input(Cow::Borrowed(b""))).unwrap();
    sender.send(Msg::Resize(size(40, 12))).unwrap();
    assert!(machine.drain_recv_channel(&mut state));
    assert!(!state.needs_write());
    assert!(machine.drain_recv_channel(&mut state));
    assert_eq!(machine.pty.sizes, [(40, 12)]);
    assert!(machine.pty.output.is_empty());
}

#[test]
fn partial_writes_finish_before_resize_but_shutdown_bypasses_blocked_input() {
    let mut machine = machine();
    let sender = machine.channel();
    let mut state = State::default();
    sender.send(Msg::Input(Cow::Borrowed(b"abcd"))).unwrap();
    sender.send(Msg::Resize(size(40, 12))).unwrap();
    assert!(machine.drain_recv_channel(&mut state));
    machine.pty.writable_bytes = 2;
    machine.pty_write(&mut state).unwrap();
    assert_eq!(machine.pty.output, b"ab");
    assert!(machine.drain_recv_channel(&mut state));
    assert!(
        machine.pty.sizes.is_empty(),
        "resize cannot overtake the partial write"
    );
    machine.pty.writable_bytes = 2;
    machine.pty_write(&mut state).unwrap();
    assert_eq!(machine.pty.output, b"abcd");
    assert!(machine.drain_recv_channel(&mut state));
    assert_eq!(machine.pty.sizes, [(40, 12)]);
    sender.send(Msg::Input(Cow::Borrowed(b"blocked"))).unwrap();
    sender.send(Msg::Resize(size(80, 24))).unwrap();
    sender.send(Msg::Input(Cow::Borrowed(b"later"))).unwrap();
    assert!(machine.drain_recv_channel(&mut state));
    // Neither the compatibility deadline nor an unwritable native pipe may
    // strand a close request behind queued resize and input messages.
    sender.clone().send(Msg::Shutdown).unwrap();
    assert!(!machine.drain_recv_channel(&mut state));
    assert_eq!(machine.pty.shutdowns, 1);
    assert_eq!(machine.pty.sizes, [(40, 12)]);
    assert_eq!(machine.pty.output, b"abcd");
}
