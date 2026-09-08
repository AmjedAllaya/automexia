use super::*;
use crate::ansi::CursorShape;
use crate::crosswords::CrosswordsSize;
use crate::event::VoidListener;
use std::sync::atomic::{AtomicUsize, Ordering};
use teletypewriter::{
    ChildEvent, EventedPty, ManagedPtyShutdown, ProcessReadWrite, WinsizeBuilder,
};

#[derive(Default)]
struct BoundaryReader {
    final_bytes: Option<Vec<u8>>,
    tail_chunks: VecDeque<Vec<u8>>,
    read_bytes: Option<Arc<AtomicUsize>>,
    cancel_after_read: Option<PtySender>,
    before_eof: Option<mpsc::SyncSender<()>>,
    eof: Option<io::Error>,
}

impl Read for BoundaryReader {
    fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
        if self.final_bytes.is_none() {
            self.final_bytes = self.tail_chunks.pop_front();
        }
        if let Some(bytes) = &mut self.final_bytes {
            let count = bytes.len().min(output.len());
            output[..count].copy_from_slice(&bytes[..count]);
            bytes.drain(..count);
            if bytes.is_empty() {
                self.final_bytes = None;
            }
            if let Some(total) = &self.read_bytes {
                total.fetch_add(count, Ordering::Relaxed);
            }
            if let Some(sender) = self.cancel_after_read.take() {
                sender.send(Msg::Shutdown).unwrap();
            }
            return Ok(count);
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
    write_error: Option<ErrorKind>,
    write_attempts: Option<Arc<AtomicUsize>>,
    child_exit: Option<ChildEvent>,
    exit_on_write: Option<ChildEvent>,
    child_registration: Option<(corcovado::Registration, corcovado::SetReadiness)>,
    tokens: Option<[corcovado::Token; 3]>,
}

impl Write for BoundaryPty {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if let Some(exit) = self.exit_on_write.take() {
            self.child_exit = Some(exit);
        }
        if let Some(attempts) = &self.write_attempts {
            attempts.fetch_add(1, Ordering::Relaxed);
        }
        if let Some(kind) = self.write_error {
            return Err(kind.into());
        }
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
        self.tokens.map_or(corcovado::Token(1), |tokens| tokens[0])
    }
    fn write_token(&self) -> corcovado::Token {
        self.tokens.map_or(corcovado::Token(2), |tokens| tokens[1])
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
        poll: &corcovado::Poll,
        tokens: &mut dyn Iterator<Item = corcovado::Token>,
        _: Ready,
        _: PollOpt,
    ) -> io::Result<()> {
        let assigned = [
            tokens.next().unwrap(),
            tokens.next().unwrap(),
            tokens.next().unwrap(),
        ];
        self.tokens = Some(assigned);
        if self.child_exit.is_some() {
            let (registration, readiness) = corcovado::Registration::new2();
            poll.register(
                &registration,
                assigned[2],
                Ready::readable(),
                PollOpt::level(),
            )?;
            readiness.set_readiness(Ready::readable())?;
            self.child_registration = Some((registration, readiness));
        }
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
    fn deregister(&mut self, poll: &corcovado::Poll) -> io::Result<()> {
        if let Some((registration, _)) = &self.child_registration {
            poll.deregister(registration)?;
        }
        Ok(())
    }
}

impl EventedPty for BoundaryPty {
    fn child_event_token(&self) -> corcovado::Token {
        self.tokens.map_or(corcovado::Token(3), |tokens| tokens[2])
    }
    fn next_child_event(&mut self) -> Option<ChildEvent> {
        self.child_exit.take()
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

#[derive(Clone, Copy)]
enum ExitScenario {
    BeforePoll,
    DuringWrite,
    Absent,
    HostShutdown,
}

fn assert_exit_io_ordering(scenario: ExitScenario, read_error: Option<ErrorKind>) {
    #[derive(Clone)]
    struct ExitEvents(mpsc::SyncSender<(&'static str, Option<i32>)>);
    impl EventListener for ExitEvents {
        fn send_event(&self, event: RioEvent, _: WindowId) {
            let publication = match event {
                RioEvent::ChildExited(_, status) => ("exit", status),
                RioEvent::CloseTerminal(_) => ("close", None),
                RioEvent::Render => ("render", None),
                _ => return,
            };
            self.0.try_send(publication).expect("bounded publication");
        }
    }
    let exited = matches!(
        scenario,
        ExitScenario::BeforePoll | ExitScenario::DuringWrite
    );
    let (tx, rx) = mpsc::sync_channel(4);
    let events = ExitEvents(tx);
    let terminal = Arc::new(FairMutex::new(Crosswords::new(
        CrosswordsSize::new(80, 24),
        CursorShape::Block,
        events.clone(),
        WindowId::from(0),
        0,
        100,
    )));
    let writes = Arc::new(AtomicUsize::new(0));
    let pty = BoundaryPty {
        child_exit: matches!(
            scenario,
            ExitScenario::BeforePoll | ExitScenario::HostShutdown
        )
        .then_some(ChildEvent::Exited(Some(17))),
        exit_on_write: matches!(scenario, ExitScenario::DuringWrite)
            .then_some(ChildEvent::Exited(Some(17))),
        reader: BoundaryReader {
            final_bytes: Some(b"FINAL".to_vec()),
            eof: read_error.map(Into::into),
            ..Default::default()
        },
        write_error: Some(ErrorKind::BrokenPipe),
        write_attempts: Some(writes.clone()),
        ..Default::default()
    };
    let machine =
        Machine::new(terminal.clone(), pty, events, WindowId::from(0), 0).unwrap();
    let sender = machine.channel();
    // Both readiness sources exist before the real worker's first poll. The
    // independent status must win over an input/error path, on every schedule.
    sender
        .send(Msg::Input(Cow::Borrowed(b"ending-input")))
        .unwrap();
    if matches!(scenario, ExitScenario::HostShutdown) {
        sender.send(Msg::Shutdown).unwrap();
    }
    let mut worker = machine.spawn();
    let completed = worker.join_timeout(std::time::Duration::from_secs(5));
    if !completed {
        let _ = sender.send(Msg::Shutdown);
        assert!(
            worker.join_timeout(std::time::Duration::from_secs(5)),
            "fixture cleanup"
        );
    }
    assert!(completed, "worker completion is bounded");
    let publications: Vec<_> = rx.try_iter().collect();
    let expected = if exited {
        vec![("exit", Some(17)), ("close", None), ("render", None)]
    } else {
        vec![]
    };
    assert_eq!(
        publications, expected,
        "exactly one exit precedes close and render"
    );
    if exited {
        assert_eq!(
            writes.load(Ordering::Relaxed),
            usize::from(matches!(scenario, ExitScenario::DuringWrite)),
            "never write after observing the confirmed exit"
        );
        let terminal = terminal.lock();
        let text: String = terminal.grid[crate::crosswords::pos::Line(0)]
            .inner
            .iter()
            .take(5)
            .map(|cell| cell.c())
            .collect();
        assert_eq!(text, "FINAL", "drain final bytes before publishing exit");
    }
}

#[test]
fn confirmed_child_exit_precedes_queued_input_failure() {
    assert_exit_io_ordering(ExitScenario::BeforePoll, None);
}

#[test]
fn confirmed_child_exit_survives_final_read_failure() {
    assert_exit_io_ordering(ExitScenario::BeforePoll, Some(ErrorKind::PermissionDenied));
}

#[test]
fn transport_failure_never_invents_a_confirmed_child_exit() {
    assert_exit_io_ordering(ExitScenario::Absent, None);
}

#[test]
fn child_exit_arriving_during_failed_write_is_published_once() {
    assert_exit_io_ordering(ExitScenario::DuringWrite, None);
}

#[test]
fn explicit_host_shutdown_precedes_child_exit_and_queued_input() {
    assert_exit_io_ordering(ExitScenario::HostShutdown, None);
}

#[test]
fn confirmed_child_exit_retains_tail_after_full_read_batch() {
    let mut machine = machine();
    let terminal = machine.terminal.clone();
    machine.pty.child_exit = Some(ChildEvent::Exited(Some(0)));
    machine.pty.reader = BoundaryReader {
        // CR bytes exercise the parser without evicting the literal tail from
        // scrollback. The transport fragments it at the normal read-batch limit.
        final_bytes: Some(vec![b'\r'; MAX_LOCKED_READ]),
        tail_chunks: VecDeque::from([b"TAIL".to_vec()]),
        ..Default::default()
    };
    let mut worker = machine.spawn();
    assert!(worker.join_timeout(std::time::Duration::from_secs(5)));
    let terminal = terminal.lock();
    let text: String = terminal.grid[crate::crosswords::pos::Line(0)]
        .inner
        .iter()
        .take(4)
        .map(|cell| cell.c())
        .collect();
    assert_eq!(
        text, "TAIL",
        "confirmed exit must retain the next available batch"
    );
}

#[test]
fn final_drain_has_exact_byte_ceiling_and_cancellation_boundary() {
    // Literal boundary values are independent of the production constant.
    for total in [
        0, 1, 65_534, 65_535, 65_536, 4_194_303, 4_194_304, 4_194_305,
    ] {
        let mut machine = machine();
        let consumed = Arc::new(AtomicUsize::new(0));
        machine.pty.child_exit = Some(ChildEvent::Exited(Some(0)));
        machine.pty.reader.final_bytes = Some(vec![b'\r'; total]);
        machine.pty.reader.read_bytes = Some(consumed.clone());
        let mut worker = machine.spawn();
        assert!(worker.join_timeout(std::time::Duration::from_secs(5)));
        assert_eq!(consumed.load(Ordering::Relaxed), total.min(4_194_304));
    }
    let mut machine = machine();
    let consumed = Arc::new(AtomicUsize::new(0));
    machine.pty.child_exit = Some(ChildEvent::Exited(Some(0)));
    machine.pty.reader.final_bytes = Some(vec![b'\r'; 65_535]);
    machine.pty.reader.tail_chunks = VecDeque::from([b"NOT-READ".to_vec()]);
    machine.pty.reader.read_bytes = Some(consumed.clone());
    machine.pty.reader.cancel_after_read = Some(machine.channel());
    let mut worker = machine.spawn();
    assert!(worker.join_timeout(std::time::Duration::from_secs(5)));
    assert_eq!(consumed.load(Ordering::Relaxed), 65_535);
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
            tail_chunks: VecDeque::new(),
            read_bytes: None,
            cancel_after_read: None,
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
    assert_eq!(
        machine.terminal.lock().screen_lines(),
        if cfg!(windows) { 12 } else { 24 },
        "only managed ConPTY delegates grid commits to this worker"
    );
    #[cfg(unix)]
    {
        // Unix geometry belongs to the application resize path, not the PTY
        // acknowledgment. Assert both owners rather than hiding Unix coverage.
        let mut terminal = machine.terminal.lock();
        terminal.resize(CrosswordsSize::new(40, 12));
        assert_eq!(terminal.screen_lines(), 12);
    }
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
