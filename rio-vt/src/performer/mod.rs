pub mod handler;
mod osc;
pub mod parser;

#[cfg(feature = "pty")]
mod sender;
#[cfg(feature = "pty")]
mod workers;
#[cfg(feature = "pty")]
pub use sender::PtySender;
#[cfg(feature = "pty")]
pub use workers::{PtyWorkerLease, PtyWorkerRegistry};

#[cfg(feature = "pty")]
use crate::crosswords::Crosswords;
#[cfg(feature = "pty")]
use crate::event::sync::FairMutex;
#[cfg(feature = "pty")]
use crate::event::RioEvent;
#[cfg(feature = "pty")]
use crate::event::{EventListener, Msg, WindowId};
#[cfg(feature = "pty")]
use corcovado::channel;
#[cfg(all(unix, feature = "pty"))]
use corcovado::unix::UnixReady;
#[cfg(feature = "pty")]
use corcovado::{self, Events, PollOpt, Ready};
#[cfg(feature = "pty")]
use std::borrow::Cow;
#[cfg(feature = "pty")]
use std::collections::VecDeque;
#[cfg(feature = "pty")]
use std::io::{self, ErrorKind, Read, Write};
#[cfg(feature = "pty")]
use std::sync::{mpsc, Arc};
#[cfg(feature = "pty")]
use std::thread::{Builder, JoinHandle};
#[cfg(feature = "pty")]
use std::time::Instant;
#[cfg(feature = "pty")]
use tracing::{error, warn};

/// Like `thread::spawn`, but with a `name` argument.
#[cfg(feature = "pty")]
pub fn spawn_named<F, T, S>(name: S, f: F) -> JoinHandle<T>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
    S: Into<String>,
{
    Builder::new()
        .name(name.into())
        .spawn(f)
        .expect("thread spawn works")
}

/// Join ownership for one PTY worker. Use PtyWorkerRegistry for UI retirement;
/// direct native joins can include platform thread-local destruction.
#[cfg(feature = "pty")]
pub struct PtyWorkerHandle<T> {
    thread: Option<JoinHandle<T>>,
    completion: mpsc::Receiver<()>,
}

#[cfg(feature = "pty")]
impl<T> PtyWorkerHandle<T> {
    /// Wait for worker completion, then join. Native thread-local destruction
    /// can outlive both the body notification and Rust's finished hint. UI
    /// callers must use PtyWorkerRegistry instead of waiting/joining directly.
    pub fn join_timeout(&mut self, timeout: std::time::Duration) -> bool {
        match self.completion.recv_timeout(timeout) {
            Ok(()) | Err(mpsc::RecvTimeoutError::Disconnected) => self
                .thread
                .take()
                .is_none_or(|thread| thread.join().is_ok()),
            Err(mpsc::RecvTimeoutError::Timeout) => false,
        }
    }
}

#[cfg(feature = "pty")]
const READ_BUFFER_SIZE: usize = 0x10_0000;
/// Max bytes to read from the PTY while the terminal is locked.
#[cfg(feature = "pty")]
const MAX_LOCKED_READ: usize = u16::MAX as usize;
/// Closing a child must not let a surviving output producer drain forever.
#[cfg(feature = "pty")]
const MAX_FINAL_OUTPUT_BYTES: usize = 4 * READ_BUFFER_SIZE;

#[cfg(feature = "pty")]
struct PeekableReceiver<T> {
    rx: channel::Receiver<T>,
    peeked: Option<T>,
}

#[cfg(feature = "pty")]
impl<T> PeekableReceiver<T> {
    fn new(rx: channel::Receiver<T>) -> Self {
        Self { rx, peeked: None }
    }

    fn peek(&mut self) -> Option<&T> {
        if self.peeked.is_none() {
            self.peeked = self.rx.try_recv().ok();
        }

        self.peeked.as_ref()
    }

    fn recv(&mut self) -> Option<T> {
        if self.peeked.is_some() {
            self.peeked.take()
        } else {
            self.rx.try_recv().ok()
        }
    }
}

/// Collapse only adjacent resize messages. Input and shutdown are ordering
/// barriers: a pending resize remains before them, while a later resize starts
/// a new coalescing run. This keeps shell line editors synchronized without
/// forwarding a window-drag backlog to ConPTY/Unix PTYs.
#[cfg(feature = "pty")]
fn coalesce_channel_messages(messages: impl IntoIterator<Item = Msg>) -> Vec<Msg> {
    let mut coalesced = Vec::new();
    for message in messages {
        match message {
            Msg::Resize(size) => match coalesced.last_mut() {
                Some(Msg::Resize(pending)) => *pending = size,
                _ => coalesced.push(Msg::Resize(size)),
            },
            Msg::Shutdown => {
                coalesced.push(Msg::Shutdown);
                break;
            }
            input @ Msg::Input(_) => coalesced.push(input),
        }
    }
    coalesced
}

/// Stop at input until its bytes have reached the PTY. Merely adding input to
/// a write queue is not an ordering barrier against the next native resize.
#[cfg(feature = "pty")]
fn next_channel_batch(
    receiver: &mut PeekableReceiver<Msg>,
) -> impl Iterator<Item = Msg> + '_ {
    let mut remaining = 128;
    std::iter::from_fn(move || {
        if remaining == 0 {
            return None;
        }
        let message = receiver.recv()?;
        remaining -= 1;
        if matches!(message, Msg::Input(_) | Msg::Shutdown) {
            remaining = 0;
        }
        Some(message)
    })
}

#[cfg(feature = "pty")]
trait PtyMessageSink {
    fn resize(&mut self, size: crate::event::WindowSize) -> io::Result<()>;
    fn input(&mut self, input: Cow<'static, [u8]>);
    fn shutdown(&mut self);
}

#[cfg(feature = "pty")]
struct LivePtyMessageSink<'a, T, U: EventListener> {
    pty: &'a mut T,
    terminal: &'a Arc<FairMutex<Crosswords<U>>>,
    write_list: &'a mut VecDeque<Cow<'static, [u8]>>,
    #[cfg(windows)]
    resize_input_not_before: &'a mut Option<Instant>,
}

#[cfg(feature = "pty")]
impl<T: teletypewriter::EventedPty, U: EventListener> PtyMessageSink
    for LivePtyMessageSink<'_, T, U>
{
    fn resize(&mut self, size: crate::event::WindowSize) -> io::Result<()> {
        self.pty.set_winsize(size.into())?;
        let mut terminal = self.terminal.lock();
        #[cfg(windows)]
        let grid_changed = terminal.columns() != size.cols as usize
            || terminal.screen_lines() != size.rows as usize;
        terminal.commit_worker_resize(size.cols as usize, size.rows as usize);
        drop(terminal);
        // PSReadLine skips cursor reconciliation for 50 ms after rendering.
        // Keep that native fast path from using pre-resize coordinates. The
        // worker continues reading output; neither the UI nor a thread sleeps.
        #[cfg(windows)]
        if grid_changed {
            *self.resize_input_not_before =
                Some(Instant::now() + std::time::Duration::from_millis(50));
        }
        Ok(())
    }

    fn input(&mut self, input: Cow<'static, [u8]>) {
        if !input.is_empty() {
            self.write_list.push_back(input);
        }
    }

    fn shutdown(&mut self) {
        if let Err(error) = self.pty.shutdown_owned_process_tree() {
            warn!(
                "managed PTY process-tree shutdown did not confirm completion: {error}"
            );
        }
    }
}

/// Deliver a coalesced channel batch to the PTY boundary. Keeping this policy
/// in one function makes the exact ordering observable with a recording sink:
/// an effective resize is committed before following input or shutdown, while
/// duplicates never reach the operating-system PTY.
#[cfg(feature = "pty")]
fn deliver_channel_messages(
    messages: impl IntoIterator<Item = Msg>,
    sink: &mut impl PtyMessageSink,
    last_window_size: &mut Option<crate::event::WindowSize>,
) -> bool {
    for msg in coalesce_channel_messages(messages) {
        match msg {
            Msg::Input(input) => sink.input(input),
            Msg::Resize(window_size) => {
                if *last_window_size == Some(window_size) {
                    continue;
                }
                match sink.resize(window_size) {
                    Ok(()) => *last_window_size = Some(window_size),
                    Err(error) => {
                        // A transient ConPTY/PTY resize failure must not
                        // terminate the session. Leave the last successful
                        // size unchanged so a later message can retry.
                        warn!(
                            rows = window_size.rows,
                            cols = window_size.cols,
                            "could not resize PTY; session remains usable: {error}"
                        );
                    }
                }
            }
            Msg::Shutdown => {
                sink.shutdown();
                return false;
            }
        }
    }

    true
}

#[cfg(feature = "pty")]
pub struct Machine<T: teletypewriter::EventedPty, U: EventListener> {
    sender: PtySender,
    shutdown_registration: corcovado::Registration,
    receiver: PeekableReceiver<Msg>,
    pty: T,
    poll: corcovado::Poll,
    terminal: Arc<FairMutex<Crosswords<U>>>,
    event_proxy: U,
    window_id: WindowId,
    route_id: usize,
    last_window_size: Option<crate::event::WindowSize>,
}

#[cfg(feature = "pty")]
#[derive(Default)]
pub struct State {
    write_list: VecDeque<Cow<'static, [u8]>>,
    writing: Option<Writing>,
    parser: handler::Processor,
    resize_input_not_before: Option<Instant>,
}

#[cfg(feature = "pty")]
impl State {
    fn input_ready(&self, now: Instant) -> bool {
        self.resize_input_not_before
            .is_none_or(|deadline| now >= deadline)
    }

    fn input_poll_timeout(
        &self,
        now: Instant,
        writable_interest: bool,
    ) -> Option<std::time::Duration> {
        if !self.needs_write() {
            return None;
        }
        if let Some(deadline) = self
            .resize_input_not_before
            .filter(|deadline| *deadline > now)
        {
            return Some(deadline.duration_since(now));
        }
        // The deadline can expire between registration and the next poll.
        // Do not wait forever on a read-only registration with buffered input.
        (!writable_interest).then_some(std::time::Duration::ZERO)
    }
    #[inline]
    fn ensure_next(&mut self) {
        if self.writing.is_none() {
            self.goto_next();
        }
    }

    #[inline]
    fn goto_next(&mut self) {
        self.writing = self.write_list.pop_front().map(Writing::new);
    }

    #[inline]
    fn take_current(&mut self) -> Option<Writing> {
        self.writing.take()
    }

    #[inline]
    fn needs_write(&self) -> bool {
        self.writing.is_some() || !self.write_list.is_empty()
    }

    #[inline]
    fn set_current(&mut self, new: Option<Writing>) {
        self.writing = new;
    }
}

#[cfg(feature = "pty")]
struct Writing {
    source: Cow<'static, [u8]>,
    written: usize,
}

#[cfg(feature = "pty")]
impl Writing {
    #[inline]
    fn new(c: Cow<'static, [u8]>) -> Writing {
        Writing {
            source: c,
            written: 0,
        }
    }

    #[inline]
    fn advance(&mut self, n: usize) {
        self.written += n;
    }

    #[inline]
    fn remaining_bytes(&self) -> &[u8] {
        &self.source[self.written..]
    }

    #[inline]
    fn finished(&self) -> bool {
        self.written >= self.source.len()
    }
}

#[cfg(feature = "pty")]
impl<T, U> Machine<T, U>
where
    T: teletypewriter::EventedPty + Send + 'static,
    U: EventListener + Send + 'static,
{
    pub fn new(
        terminal: Arc<FairMutex<Crosswords<U>>>,
        pty: T,
        event_proxy: U,
        window_id: WindowId,
        route_id: usize,
    ) -> Result<Machine<T, U>, Box<dyn std::error::Error>> {
        let (sender, receiver) = channel::channel();
        let (sender, shutdown_registration) = PtySender::managed(sender);
        let poll = corcovado::Poll::new()?;

        // All Windows PTY sessions (including wsl.exe) use ConPTY. Its mutable
        // viewport cannot pull prior history back when the window grows.
        #[cfg(windows)]
        {
            let mut terminal = terminal.lock();
            terminal.set_resize_policy(crate::crosswords::ResizePolicy::Conpty);
            terminal.enable_worker_resize();
        }

        Ok(Machine {
            sender,
            shutdown_registration,
            receiver: PeekableReceiver::new(receiver),
            poll,
            pty,
            terminal,
            event_proxy,
            window_id,
            route_id,
            last_window_size: None,
        })
    }

    /// Read from the PTY and parse into the terminal.
    ///
    /// With `drain_fully` the read loop only stops on `WouldBlock` (or
    /// `MAX_LOCKED_READ`). Without it, a read shorter than the available
    /// buffer is treated as "PTY drained" and the loop stops early, skipping
    /// the extra read that would confirm it with `WouldBlock`. The early stop
    /// is only sound while the PTY stays registered level-triggered (if more
    /// data raced in, the next poll returns immediately), so callers that
    /// exit the event loop right after (the child-exit drain) must pass
    /// `drain_fully: true`.
    #[inline]
    fn pty_read(
        &mut self,
        state: &mut State,
        buf: &mut [u8],
        drain_fully: bool,
    ) -> io::Result<()> {
        self.pty_read_bounded(state, buf, drain_fully, usize::MAX)
            .map(|_| ())
    }

    fn pty_read_bounded(
        &mut self,
        state: &mut State,
        buf: &mut [u8],
        drain_fully: bool,
        byte_limit: usize,
    ) -> io::Result<usize> {
        let mut unprocessed = 0;
        let mut processed = 0;

        // Reserve the next terminal lock for PTY reading.
        let _terminal_lease = Some(self.terminal.lease());
        let mut terminal = None;

        loop {
            if processed == byte_limit {
                break;
            }
            let read_limit = buf.len().min(byte_limit - processed);
            let drained;
            let mut eof = false;

            // Read from the PTY.
            match self.pty.reader().read(&mut buf[unprocessed..read_limit]) {
                // This is received on Windows/macOS when no more data is readable from the PTY.
                Ok(0) if unprocessed == 0 => break,
                Ok(got) => {
                    eof = got == 0;
                    drained = got < read_limit - unprocessed;
                    unprocessed += got;
                }
                Err(err) => match err.kind() {
                    ErrorKind::Interrupted | ErrorKind::WouldBlock => {
                        // Go back to mio if we're caught up on parsing and the PTY would block.
                        if unprocessed == 0 {
                            break;
                        }
                        // An interrupted read says nothing about the PTY
                        // being drained; only `WouldBlock` does.
                        drained = err.kind() == ErrorKind::WouldBlock;
                    }
                    _ if teletypewriter::is_pty_eof_error(&err) => {
                        if unprocessed == 0 {
                            break;
                        }
                        // A resize/frame may have held the terminal lock while
                        // the final bytes arrived. Parse them before closing.
                        eof = true;
                        drained = true;
                    }
                    _ => return Err(err),
                },
            }

            // Attempt to lock the terminal.
            let terminal = match &mut terminal {
                Some(terminal) => terminal,
                None => terminal.insert(match self.terminal.try_lock_unfair() {
                    // EOF cannot produce another readiness event for pending
                    // bytes. Wait on this worker, never on the input thread.
                    None if unprocessed == read_limit || eof => {
                        self.terminal.lock_unfair()
                    }
                    None => continue,
                    Some(terminal) => terminal,
                }),
            };

            // Parse the incoming bytes.
            state.parser.advance(&mut **terminal, &buf[..unprocessed]);

            processed += unprocessed;
            unprocessed = 0;

            // Assure we're not blocking the terminal too long unnecessarily,
            // and stop as soon as the PTY looks drained.
            if eof || processed >= MAX_LOCKED_READ || (drained && !drain_fully) {
                break;
            }
        }

        // Notify renderer that new damage is available.
        // Only send if no event is already in flight: the renderer will
        // extract all accumulated damage when it locks the terminal.
        if state.parser.sync_bytes_count() < processed && processed > 0 {
            if let Some(ref mut term) = terminal {
                if !term.damage_event_in_flight && term.peek_damage_event().is_some() {
                    term.damage_event_in_flight = true;
                    self.event_proxy.send_event(
                        RioEvent::TerminalDamaged(self.route_id),
                        self.window_id,
                    );
                }
            }
        }

        Ok(processed)
    }

    /// Drain the channel.
    ///
    /// Returns `false` when a shutdown message was received.
    fn drain_recv_channel(&mut self, state: &mut State) -> bool {
        // Explicit cancellation need not wait for unsent input or a full pipe.
        if self.sender.shutdown_requested() {
            if let Err(error) = self.pty.shutdown_owned_process_tree() {
                warn!("managed PTY process-tree shutdown did not confirm completion: {error}");
            }
            return false;
        }
        if state.needs_write() {
            return true;
        }
        let messages = next_channel_batch(&mut self.receiver);
        let mut sink = LivePtyMessageSink {
            pty: &mut self.pty,
            terminal: &self.terminal,
            write_list: &mut state.write_list,
            #[cfg(windows)]
            resize_input_not_before: &mut state.resize_input_not_before,
        };
        deliver_channel_messages(messages, &mut sink, &mut self.last_window_size)
    }

    #[inline]
    fn pty_write(&mut self, state: &mut State) -> io::Result<()> {
        if !state.input_ready(Instant::now()) {
            return Ok(());
        }
        state.ensure_next();

        'write_many: while let Some(mut current) = state.take_current() {
            'write_one: loop {
                match self.pty.writer().write(current.remaining_bytes()) {
                    Ok(0) => {
                        state.set_current(Some(current));
                        break 'write_many;
                    }
                    Ok(n) => {
                        current.advance(n);
                        if current.finished() {
                            state.goto_next();
                            break 'write_one;
                        }
                    }
                    Err(err) => {
                        state.set_current(Some(current));
                        match err.kind() {
                            ErrorKind::Interrupted | ErrorKind::WouldBlock => {
                                break 'write_many
                            }
                            _ => return Err(err),
                        }
                    }
                }
            }
        }
        Ok(())
    }

    pub fn channel(&self) -> PtySender {
        self.sender.clone()
    }

    /// Consume only the adapter's authoritative child notification. An I/O
    /// failure alone cannot establish a child status. Keep final output and
    /// exit/close publication in one owner, including transport-error races.
    fn finish_child_exit(&mut self, state: &mut State, buf: &mut [u8]) -> bool {
        let Some(teletypewriter::ChildEvent::Exited(status)) =
            self.pty.next_child_event()
        else {
            return false;
        };
        let mut remaining = MAX_FINAL_OUTPUT_BYTES;
        while remaining > 0 && !self.sender.shutdown_requested() {
            // Release the terminal lease between normal-sized batches. The
            // byte budget also bounds a hostile surviving output producer.
            match self.pty_read_bounded(state, buf, true, remaining) {
                Ok(0) => break,
                Ok(processed) => remaining -= processed,
                Err(err) => {
                    tracing::debug!("PTY drain after child exit failed: {err}");
                    break;
                }
            }
        }
        if remaining == 0 {
            warn!("PTY final-output drain reached its byte ceiling");
        }
        self.event_proxy
            .send_event(RioEvent::ChildExited(self.route_id, status), self.window_id);
        self.terminal.lock().exit();
        self.event_proxy
            .send_event(RioEvent::Render, self.window_id);
        true
    }

    pub fn spawn(mut self) -> PtyWorkerHandle<()> {
        let (completion_tx, completion) = mpsc::sync_channel(1);
        let thread = spawn_named("PTY reader", move || {
            let mut state = State::default();
            let mut buf = [0u8; READ_BUFFER_SIZE];

            let mut tokens = (0..).map(Into::into);

            let shutdown_token = tokens.next().unwrap();
            self.poll
                .register(
                    &self.shutdown_registration,
                    shutdown_token,
                    Ready::readable(),
                    PollOpt::edge(),
                )
                .unwrap();

            // Remaining channel batches use an immediate poll once pending
            // writes drain; this retains edge-triggered wakeups without losing
            // an input/resize boundary or spinning while a write is deferred.
            let channel_token = tokens.next().unwrap();
            self.poll
                .register(
                    &self.receiver.rx,
                    channel_token,
                    Ready::readable(),
                    PollOpt::edge(),
                )
                .unwrap();

            // The PTY is level-triggered: pty_read may stop before draining
            // the fd (MAX_LOCKED_READ), which would lose an edge, and level
            // registrations stay armed so no re-registration is needed after
            // each event. The write interest must be dropped as soon as the
            // write queue drains or the poll would keep waking up for the
            // writable PTY.
            let poll_opts = PollOpt::level();

            // Register TTY through EventedRW interface.
            self.pty
                .register(&self.poll, &mut tokens, Ready::readable(), poll_opts)
                .unwrap();

            let mut events = Events::with_capacity(1024);
            let mut last_interest = Ready::readable();

            'event_loop: loop {
                // Wakeup the event loop when a synchronized update timeout was reached.
                let handler = state.parser.sync_timeout();
                let mut timeout = handler
                    .sync_timeout()
                    .map(|st| st.saturating_duration_since(Instant::now()));
                if let Some(wait) =
                    state.input_poll_timeout(Instant::now(), last_interest.is_writable())
                {
                    timeout = Some(timeout.map_or(wait, |sync| sync.min(wait)));
                }
                if self.sender.shutdown_requested()
                    || (!state.needs_write() && self.receiver.peek().is_some())
                {
                    timeout = Some(std::time::Duration::ZERO);
                }

                events.clear();
                if let Err(err) = self.poll.poll(&mut events, timeout) {
                    match err.kind() {
                        ErrorKind::Interrupted => continue,
                        _ => {
                            error!("Event loop polling error: {err}");
                            break 'event_loop;
                        }
                    }
                }

                // A native input deadline must not postpone synchronized-output
                // expiry or turn an expired sync timeout into a busy poll.
                if state
                    .parser
                    .sync_timeout()
                    .sync_timeout()
                    .is_some_and(|deadline| deadline <= Instant::now())
                {
                    let mut terminal = self.terminal.lock();
                    state.parser.stop_sync(&mut *terminal);

                    // Notify renderer if damage available and no event in flight
                    if !terminal.damage_event_in_flight
                        && terminal.peek_damage_event().is_some()
                    {
                        terminal.damage_event_in_flight = true;
                        self.event_proxy.send_event(
                            RioEvent::TerminalDamaged(self.route_id),
                            self.window_id,
                        );
                    }
                }

                // Handle channel events, if there are any.
                if self.sender.shutdown_requested() {
                    self.drain_recv_channel(&mut state);
                    break;
                }
                // A confirmed exit wins over obsolete input/resize work in the
                // same poll batch. Explicit host cancellation still wins above.
                if events
                    .iter()
                    .any(|event| event.token() == self.pty.child_event_token())
                    && self.finish_child_exit(&mut state, &mut buf)
                {
                    break;
                }
                #[cfg(windows)]
                if self.receiver.peek().is_some() {
                    // Parse available old-size output before committing a new
                    // native/grid size; the SPSC adapter keeps this nonblocking.
                    if let Err(err) = self.pty_read(&mut state, &mut buf, true) {
                        error!("Error draining output before native resize: {err}");
                        self.finish_child_exit(&mut state, &mut buf);
                        break;
                    }
                }
                if !self.drain_recv_channel(&mut state) {
                    break;
                }

                // Opportunistically flush newly queued input before waiting for
                // another writable-readiness notification. Anonymous ConPTY
                // pipes are backed by a bounded nonblocking buffer, so this
                // normally completes immediately. If the buffer is full,
                // `pty_write` retains the remainder and writable interest below
                // resumes it without blocking. This removes an avoidable poll
                // cycle from every key press and is especially important on
                // Windows, where re-registering an already-writable synthetic
                // readiness source can otherwise defer delivery noticeably.
                if state.needs_write() && state.input_ready(Instant::now()) {
                    if let Err(err) = self.pty_write(&mut state) {
                        error!("Error writing queued input to PTY: {err}");
                        self.finish_child_exit(&mut state, &mut buf);
                        break 'event_loop;
                    }
                }

                for event in events.iter() {
                    match event.token() {
                        // Channel messages were already drained above.
                        token if token == channel_token => (),
                        // Child readiness was consumed before queued I/O.
                        token if token == self.pty.child_event_token() => (),

                        token
                            if token == self.pty.read_token()
                                || token == self.pty.write_token() =>
                        {
                            #[cfg(unix)]
                            if UnixReady::from(event.readiness()).is_hup() {
                                // Don't try to do I/O on a dead PTY.
                                continue;
                            }
                            if event.readiness().is_readable() {
                                if let Err(err) =
                                    self.pty_read(&mut state, &mut buf, false)
                                {
                                    error!(
                                        "Error reading from PTY in event loop: {}",
                                        err
                                    );
                                    self.finish_child_exit(&mut state, &mut buf);
                                    break 'event_loop;
                                }
                            }

                            if event.readiness().is_writable() {
                                if let Err(err) = self.pty_write(&mut state) {
                                    error!("Error writing to PTY in event loop: {}", err);
                                    self.finish_child_exit(&mut state, &mut buf);
                                    break 'event_loop;
                                }
                            }
                        }
                        _ => (),
                    }
                }

                // Update the PTY registration when write interest changed.
                let mut interest = Ready::readable();
                if state.needs_write() && state.input_ready(Instant::now()) {
                    interest.insert(Ready::writable());
                }
                if interest != last_interest {
                    self.pty
                        .reregister(&self.poll, interest, poll_opts)
                        .unwrap();
                    last_interest = interest;
                }
            }

            // The evented instances are not dropped here so deregister them explicitly.
            let _ = self.poll.deregister(&self.receiver.rx);
            let _ = self.poll.deregister(&self.shutdown_registration);
            let _ = self.pty.deregister(&self.poll);

            drop((self, state));
            let _ = completion_tx.send(());
        });
        PtyWorkerHandle {
            thread: Some(thread),
            completion,
        }
    }
}

#[cfg(all(test, feature = "pty"))]
mod tests {
    use super::*;
    use crate::event::WindowSize;
    use proptest::prelude::*;

    mod resize_worker;

    #[test]
    fn native_resize_input_deadline_is_bounded_and_does_not_delay_normal_input() {
        let now = Instant::now();
        let mut state = State::default();
        assert!(state.input_ready(now));
        state.resize_input_not_before = Some(now + std::time::Duration::from_millis(50));
        assert!(!state.input_ready(now));
        assert!(!state.input_ready(now + std::time::Duration::from_millis(49)));
        assert!(state.input_ready(now + std::time::Duration::from_millis(50)));
        assert!(state.input_ready(now + std::time::Duration::from_secs(1)));
        assert!(!state.needs_write(), "a resize never fabricates input");
    }

    #[test]
    fn elapsed_resize_deadline_rearms_writes_without_busy_or_infinite_polling() {
        use std::time::Duration;
        let now = Instant::now();
        let mut state = State {
            resize_input_not_before: Some(now + Duration::from_millis(50)),
            ..State::default()
        };
        assert_eq!(state.input_poll_timeout(now, false), None);
        state.write_list.push_back(Cow::Borrowed(b"a"));
        assert_eq!(
            state.input_poll_timeout(now, false),
            Some(Duration::from_millis(50))
        );
        assert_eq!(
            state.input_poll_timeout(now + Duration::from_millis(49), true),
            Some(Duration::from_millis(1))
        );
        assert_eq!(
            state.input_poll_timeout(now + Duration::from_millis(50), false),
            Some(Duration::ZERO)
        );
        assert_eq!(
            state.input_poll_timeout(now + Duration::from_millis(51), true),
            None
        );
        state.write_list.clear();
        assert_eq!(
            state.input_poll_timeout(now + Duration::from_millis(51), false),
            None
        );
    }

    #[test]
    fn channel_batch_stops_before_resize_can_overtake_buffered_input() {
        let (sender, receiver) = channel::channel();
        let mut receiver = PeekableReceiver::new(receiver);
        let size = WindowSize {
            cols: 80,
            rows: 24,
            width: 0,
            height: 0,
        };
        sender.send(Msg::Resize(size)).unwrap();
        sender.send(Msg::Input(Cow::Borrowed(b"typed"))).unwrap();
        sender
            .send(Msg::Resize(WindowSize { rows: 12, ..size }))
            .unwrap();
        let batch: Vec<_> = next_channel_batch(&mut receiver).collect();
        assert_eq!(batch.len(), 2);
        assert!(matches!(batch[0], Msg::Resize(_)));
        assert!(matches!(batch[1], Msg::Input(_)));
        assert!(matches!(receiver.peek(), Some(Msg::Resize(next)) if next.rows == 12));
    }

    #[test]
    fn channel_batch_has_a_fairness_bound_and_keeps_shutdown_order() {
        let (sender, receiver) = channel::channel();
        let mut receiver = PeekableReceiver::new(receiver);
        let size = WindowSize {
            cols: 80,
            rows: 24,
            width: 0,
            height: 0,
        };
        for _ in 0..129 {
            sender.send(Msg::Resize(size)).unwrap();
        }
        sender.send(Msg::Shutdown).unwrap();
        sender
            .send(Msg::Input(Cow::Borrowed(b"not delivered")))
            .unwrap();
        assert_eq!(next_channel_batch(&mut receiver).count(), 128);
        let tail: Vec<_> = next_channel_batch(&mut receiver).collect();
        assert_eq!(tail.len(), 2);
        assert!(matches!(tail[0], Msg::Resize(_)));
        assert!(matches!(tail[1], Msg::Shutdown));
        assert!(matches!(receiver.peek(), Some(Msg::Input(_))));
    }

    #[derive(Clone, Debug)]
    enum ResizeQueueModelMessage {
        Resize(u16, u16),
        Input(u8),
        Shutdown,
    }

    #[derive(Debug, Eq, PartialEq)]
    enum ResizeQueueModelOutput {
        Resize(u16, u16),
        Input(u8),
        Shutdown,
    }

    fn reference_resize_queue(
        messages: &[ResizeQueueModelMessage],
    ) -> Vec<ResizeQueueModelOutput> {
        let mut output = Vec::new();
        let mut pending_resize = None;
        for message in messages {
            match *message {
                ResizeQueueModelMessage::Resize(cols, rows) => {
                    pending_resize = Some((cols, rows));
                }
                ResizeQueueModelMessage::Input(byte) => {
                    if let Some((cols, rows)) = pending_resize.take() {
                        output.push(ResizeQueueModelOutput::Resize(cols, rows));
                    }
                    output.push(ResizeQueueModelOutput::Input(byte));
                }
                ResizeQueueModelMessage::Shutdown => {
                    if let Some((cols, rows)) = pending_resize.take() {
                        output.push(ResizeQueueModelOutput::Resize(cols, rows));
                    }
                    output.push(ResizeQueueModelOutput::Shutdown);
                    break;
                }
            }
        }
        if let Some((cols, rows)) = pending_resize {
            output.push(ResizeQueueModelOutput::Resize(cols, rows));
        }
        output
    }

    fn run_resize_queue_model(
        messages: &[ResizeQueueModelMessage],
    ) -> Vec<ResizeQueueModelOutput> {
        coalesce_channel_messages(messages.iter().map(|message| match *message {
            ResizeQueueModelMessage::Resize(cols, rows) => Msg::Resize(size(cols, rows)),
            ResizeQueueModelMessage::Input(byte) => Msg::Input(Cow::Owned(vec![byte])),
            ResizeQueueModelMessage::Shutdown => Msg::Shutdown,
        }))
        .into_iter()
        .map(|message| match message {
            Msg::Resize(window_size) => {
                ResizeQueueModelOutput::Resize(window_size.cols, window_size.rows)
            }
            Msg::Input(input) => ResizeQueueModelOutput::Input(input[0]),
            Msg::Shutdown => ResizeQueueModelOutput::Shutdown,
        })
        .collect()
    }

    #[derive(Debug, PartialEq, Eq)]
    enum RecordedPtyEvent {
        Resize(WindowSize),
        Input(Vec<u8>),
        Shutdown,
    }

    #[derive(Default)]
    struct RecordingPty {
        events: Vec<RecordedPtyEvent>,
        fail_next_resize: bool,
    }

    impl PtyMessageSink for RecordingPty {
        fn resize(&mut self, size: WindowSize) -> io::Result<()> {
            if self.fail_next_resize {
                self.fail_next_resize = false;
                return Err(io::Error::other("injected transient resize failure"));
            }
            self.events.push(RecordedPtyEvent::Resize(size));
            Ok(())
        }

        fn input(&mut self, input: Cow<'static, [u8]>) {
            self.events
                .push(RecordedPtyEvent::Input(input.into_owned()));
        }

        fn shutdown(&mut self) {
            self.events.push(RecordedPtyEvent::Shutdown);
        }
    }

    fn size(cols: u16, rows: u16) -> WindowSize {
        WindowSize {
            rows,
            cols,
            width: cols.saturating_mul(8),
            height: rows.saturating_mul(16),
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(512))]

        #[test]
        fn resize_queue_model_preserves_barriers_and_latest_resize(
            messages in proptest::collection::vec(
                prop_oneof![
                    (1_u16..=1_000, 1_u16..=1_000)
                        .prop_map(|(cols, rows)| ResizeQueueModelMessage::Resize(cols, rows)),
                    any::<u8>().prop_map(ResizeQueueModelMessage::Input),
                    Just(ResizeQueueModelMessage::Shutdown),
                ],
                0..=256,
            ),
        ) {
            let expected = reference_resize_queue(&messages);
            let actual = run_resize_queue_model(&messages);

            prop_assert_eq!(&actual, &expected);
            prop_assert!(actual.len() <= messages.len());
            prop_assert!(actual.windows(2).all(|pair| !matches!(
                pair,
                [ResizeQueueModelOutput::Resize(_, _), ResizeQueueModelOutput::Resize(_, _)]
            )));
            if let Some(shutdown) = actual
                .iter()
                .position(|message| *message == ResizeQueueModelOutput::Shutdown)
            {
                prop_assert_eq!(shutdown + 1, actual.len());
            }
        }
    }

    #[test]
    fn resize_stress_coalesces_one_thousand_adjacent_messages() {
        let messages = (1..=1_000).map(|index| Msg::Resize(size(index, 40)));
        let coalesced = coalesce_channel_messages(messages);

        assert_eq!(coalesced.len(), 1);
        assert!(matches!(
            coalesced.as_slice(),
            [Msg::Resize(window_size)] if *window_size == size(1_000, 40)
        ));
    }

    #[test]
    fn resize_stress_preserves_input_and_shutdown_barriers() {
        let messages = vec![
            Msg::Resize(size(80, 24)),
            Msg::Resize(size(100, 30)),
            Msg::Input(Cow::Borrowed(b"a")),
            Msg::Resize(size(120, 40)),
            Msg::Resize(size(140, 50)),
            Msg::Input(Cow::Borrowed(b"b")),
            Msg::Resize(size(160, 60)),
            Msg::Shutdown,
            Msg::Resize(size(200, 70)),
        ];
        let coalesced = coalesce_channel_messages(messages);

        assert_eq!(coalesced.len(), 6);
        assert!(matches!(coalesced[0], Msg::Resize(value) if value == size(100, 30)));
        assert!(matches!(coalesced[1], Msg::Input(ref value) if value.as_ref() == b"a"));
        assert!(matches!(coalesced[2], Msg::Resize(value) if value == size(140, 50)));
        assert!(matches!(coalesced[3], Msg::Input(ref value) if value.as_ref() == b"b"));
        assert!(matches!(coalesced[4], Msg::Resize(value) if value == size(160, 60)));
        assert!(matches!(coalesced[5], Msg::Shutdown));
    }

    #[test]
    fn resize_stress_recording_pty_observes_final_sizes_and_input_order() {
        let messages = vec![
            Msg::Resize(size(80, 24)),
            Msg::Resize(size(100, 30)),
            Msg::Input(Cow::Borrowed(b"first")),
            Msg::Resize(size(100, 30)),
            Msg::Resize(size(140, 50)),
            Msg::Input(Cow::Borrowed(b"second")),
            Msg::Resize(size(160, 60)),
            Msg::Shutdown,
            Msg::Input(Cow::Borrowed(b"after-shutdown")),
        ];
        let mut recording_pty = RecordingPty::default();
        let mut last_window_size = None;

        assert!(!deliver_channel_messages(
            messages,
            &mut recording_pty,
            &mut last_window_size,
        ));
        assert_eq!(last_window_size, Some(size(160, 60)));
        assert_eq!(
            recording_pty.events,
            vec![
                RecordedPtyEvent::Resize(size(100, 30)),
                RecordedPtyEvent::Input(b"first".to_vec()),
                RecordedPtyEvent::Resize(size(140, 50)),
                RecordedPtyEvent::Input(b"second".to_vec()),
                RecordedPtyEvent::Resize(size(160, 60)),
                RecordedPtyEvent::Shutdown,
            ]
        );
    }

    #[test]
    fn resize_stress_recording_pty_retries_after_resize_error() {
        let requested = size(220, 70);
        let mut recording_pty = RecordingPty {
            fail_next_resize: true,
            ..RecordingPty::default()
        };
        let mut last_window_size = Some(size(100, 30));

        assert!(deliver_channel_messages(
            [Msg::Resize(requested)],
            &mut recording_pty,
            &mut last_window_size,
        ));
        assert_eq!(last_window_size, Some(size(100, 30)));
        assert!(recording_pty.events.is_empty());

        assert!(deliver_channel_messages(
            [Msg::Resize(requested), Msg::Resize(requested)],
            &mut recording_pty,
            &mut last_window_size,
        ));
        assert_eq!(last_window_size, Some(requested));
        assert_eq!(
            recording_pty.events,
            vec![RecordedPtyEvent::Resize(requested)]
        );
    }

    #[test]
    fn pty_worker_join_is_bounded_and_can_complete_after_readiness() {
        let (release_tx, release_rx) = std::sync::mpsc::sync_channel(0);
        let (completion_tx, completion_rx) = std::sync::mpsc::sync_channel(1);
        let thread = std::thread::spawn(move || {
            release_rx.recv().unwrap();
            completion_tx.send(()).unwrap();
            7_u8
        });
        let mut worker = PtyWorkerHandle {
            thread: Some(thread),
            completion: completion_rx,
        };

        assert!(!worker.join_timeout(std::time::Duration::ZERO));
        release_tx.send(()).unwrap();
        assert!(worker.join_timeout(std::time::Duration::from_secs(1)));
        assert!(worker.thread.is_none());
    }
}
