//! Standalone helper metadata output. The writer completes each active frame;
//! retirement may interrupt it only after the shell and scanner have stopped.
//! Never use this process-termination fallback in the application's UI process.
use std::io;

const MAX_FRAME_BYTES: usize = 6144;

pub(super) struct Publisher {
    inner: platform::Publisher,
    prompt_channel: bool,
}

impl Publisher {
    pub(super) fn new() -> io::Result<Self> {
        Ok(Self {
            inner: platform::Publisher::new()?,
            prompt_channel: false,
        })
    }

    #[cfg(unix)]
    pub(super) fn prompt_channel(
        file: std::fs::File,
        ready: std::fs::File,
    ) -> io::Result<Self> {
        Ok(Self {
            inner: platform::Publisher::prompt_channel(file, ready)?,
            prompt_channel: true,
        })
    }

    /// Replace one bounded pending frame without waiting for terminal capacity.
    /// Replacement never drops a frame that has already started writing.
    pub(super) fn publish(&mut self, frame: &str) -> io::Result<bool> {
        if frame.len() > MAX_FRAME_BYTES
            || (self.prompt_channel && frame.as_bytes().contains(&0))
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "SSH metadata output bound exceeded",
            ));
        }
        if self.prompt_channel {
            // NUL is a transport delimiter, never terminal output. The writer
            // acknowledges only after this entire bounded frame is queued.
            let mut bytes = Vec::with_capacity(frame.len() + 1);
            bytes.extend_from_slice(frame.as_bytes());
            bytes.push(0);
            self.inner.publish(&bytes)
        } else {
            self.inner.publish(frame.as_bytes())
        }
    }

    /// Must follow shell/scanner cleanup and precede signal re-raising. Failure
    /// retains the exact native writer owner until terminate_helper consumes it.
    pub(super) fn finish(mut self) -> Result<(), UnretiredPublisher> {
        if self.inner.retire() {
            Ok(())
        } else {
            Err(UnretiredPublisher {
                _publisher: Box::new(self),
            })
        }
    }
}

pub(super) struct UnretiredPublisher {
    _publisher: Box<Publisher>,
}

impl UnretiredPublisher {
    /// Only for the standalone helper after its other owners have retired.
    /// The exact writer owner remains alive until the process exits.
    pub(super) fn terminate_helper(self) -> ! {
        std::process::exit(70)
    }
}

impl Drop for Publisher {
    fn drop(&mut self) {
        // Normal runtime explicitly calls finish after its other cleanup owners.
        // A setup failure or panic must not detach a still-running writer either.
        if !self.inner.retire() {
            std::process::exit(70);
        }
    }
}

// Publication deliberately declines mutex contention instead of blocking.
// Tests requiring an accepted frame must honor that contract, including the
// initial race with a newly started worker. Errors and sustained refusal fail.
#[cfg(test)]
fn publish_until_accepted(mut publish: impl FnMut() -> io::Result<bool>) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    while !publish().unwrap() {
        assert!(
            std::time::Instant::now() < deadline,
            "writer never accepted frame"
        );
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
}

mod writer {
    use std::{
        fs::File,
        io::{self, Read, Write},
        sync::{
            atomic::{AtomicBool, Ordering},
            Arc, Condvar, Mutex,
        },
        thread::{self, JoinHandle},
        time::{Duration, Instant},
    };

    pub(super) struct State {
        pub(super) pending: Mutex<Option<Vec<u8>>>,
        changed: Condvar,
        stop: AtomicBool,
        failed: AtomicBool,
        #[cfg(test)]
        pub(super) writing: AtomicBool,
    }

    pub(super) struct Writer {
        pub(super) state: Arc<State>,
        pub(super) thread: Option<JoinHandle<()>>,
    }

    impl Writer {
        pub(super) fn start(
            file: Option<File>,
            prepare: fn() -> io::Result<()>,
        ) -> io::Result<Self> {
            Self::start_notified(file, None, prepare)
        }

        pub(super) fn start_notified(
            file: Option<File>,
            mut ready: Option<File>,
            prepare: fn() -> io::Result<()>,
        ) -> io::Result<Self> {
            let state = Arc::new(State {
                pending: Mutex::new(None),
                changed: Condvar::new(),
                stop: AtomicBool::new(false),
                failed: AtomicBool::new(false),
                #[cfg(test)]
                writing: AtomicBool::new(false),
            });
            let Some(mut file) = file else {
                return Ok(Self {
                    state,
                    thread: None,
                });
            };
            let worker = Arc::clone(&state);
            let thread = thread::Builder::new()
                .name("ssh-metadata-output".into())
                .spawn(move || {
                    if prepare().is_ok() {
                        loop {
                            let mut pending = match worker.pending.lock() {
                                Ok(guard) => guard,
                                Err(_) => {
                                    worker.failed.store(true, Ordering::Release);
                                    break;
                                }
                            };
                            while pending.is_none()
                                && !worker.stop.load(Ordering::Acquire)
                            {
                                pending = match worker.changed.wait(pending) {
                                    Ok(guard) => guard,
                                    Err(_) => {
                                        worker.failed.store(true, Ordering::Release);
                                        worker.stop.store(true, Ordering::Release);
                                        return;
                                    }
                                };
                            }
                            if worker.stop.load(Ordering::Acquire) {
                                break;
                            }
                            let frame = pending.take();
                            drop(pending);
                            if let Some(frame) = frame {
                                #[cfg(test)]
                                worker.writing.store(true, Ordering::Release);
                                let result = write_frame(&mut file, &frame, &worker.stop)
                                    .and_then(|()| match ready.as_mut() {
                                        Some(ready) => {
                                            write_frame(ready, b"1", &worker.stop)?;
                                            wait_for_consumption(&mut file, &worker.stop)
                                        }
                                        None => Ok(()),
                                    });
                                #[cfg(test)]
                                worker.writing.store(false, Ordering::Release);
                                if result.is_err() {
                                    if !worker.stop.load(Ordering::Acquire) {
                                        worker.failed.store(true, Ordering::Release);
                                    }
                                    break;
                                }
                            }
                        }
                    } else {
                        worker.failed.store(true, Ordering::Release);
                    }
                    worker.stop.store(true, Ordering::Release);
                })?;
            Ok(Self {
                state,
                thread: Some(thread),
            })
        }

        pub(super) fn publish(&mut self, frame: &[u8]) -> io::Result<bool> {
            if self.state.failed.load(Ordering::Acquire) {
                return Err(io::Error::new(
                    io::ErrorKind::BrokenPipe,
                    "SSH metadata writer failed",
                ));
            }
            if self.thread.is_none() || self.state.stop.load(Ordering::Acquire) {
                return Ok(false);
            }
            let Ok(mut pending) = self.state.pending.try_lock() else {
                return Ok(false);
            };
            *pending = Some(frame.to_vec());
            self.state.changed.notify_one();
            Ok(true)
        }

        pub(super) fn retire(&mut self, cancel: impl Fn(&JoinHandle<()>)) -> bool {
            self.state.stop.store(true, Ordering::Release);
            self.state.changed.notify_one();
            let Some(thread) = self.thread.as_ref() else {
                return true;
            };
            let deadline = Instant::now() + Duration::from_secs(2);
            while !thread.is_finished() {
                // Holding the predicate lock closes the check/wait lost-wake race.
                if let Ok(mut pending) = self.state.pending.try_lock() {
                    pending.take();
                    self.state.changed.notify_one();
                }
                // Repeat exact-thread cancellation to close the check/write race.
                // Retirement is only permitted after live shell output has ceased.
                cancel(thread);
                if Instant::now() >= deadline {
                    return false;
                }
                thread::sleep(Duration::from_millis(2));
            }
            if let Some(thread) = self.thread.take() {
                let _ = thread.join();
            }
            true
        }
    }

    /// Keep at most one completed frame in the private channel. While the
    /// shell is idle, publication replaces the pending frame with the newest
    /// result instead of queueing stale context in the socket buffer.
    fn wait_for_consumption(file: &mut File, stop: &AtomicBool) -> io::Result<()> {
        let mut ack = [0; 1];
        loop {
            if stop.load(Ordering::Acquire) {
                return Err(io::ErrorKind::Interrupted.into());
            }
            #[cfg(unix)]
            {
                use std::os::fd::AsRawFd;
                let mut descriptor = libc::pollfd {
                    fd: file.as_raw_fd(),
                    events: libc::POLLIN,
                    revents: 0,
                };
                // SAFETY: one initialized pollfd, owned live descriptor, and a
                // bounded timeout. Kernel readiness avoids polling an idle
                // shell at the retry frequency used for active writes.
                match unsafe { libc::poll(&mut descriptor, 1, 200) } {
                    -1 => {
                        let error = io::Error::last_os_error();
                        if error.kind() == io::ErrorKind::Interrupted {
                            continue;
                        }
                        return Err(error);
                    }
                    0 => continue,
                    _ => {}
                }
            }
            match file.read(&mut ack) {
                Ok(1) if ack == *b"1" => return Ok(()),
                Ok(0) => return Err(io::ErrorKind::UnexpectedEof.into()),
                Ok(_) => return Err(io::ErrorKind::InvalidData.into()),
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(10));
                }
                Err(error) => return Err(error),
            }
        }
    }

    fn write_frame(
        file: &mut impl Write,
        mut frame: &[u8],
        stop: &AtomicBool,
    ) -> io::Result<()> {
        // Unlike write_all, EINTR must not be retried after shutdown. While the
        // shell is alive, preserve the frame through its terminating byte.
        while !frame.is_empty() {
            if stop.load(Ordering::Acquire) {
                return Err(io::ErrorKind::Interrupted.into());
            }
            match file.write(frame) {
                Ok(0) => return Err(io::ErrorKind::WriteZero.into()),
                Ok(count) => frame = &frame[count..],
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                    // Only private nonblocking channels use this path. Sleep
                    // rather than spin, and observe retirement before retrying.
                    thread::sleep(Duration::from_millis(10));
                }
                Err(error) => return Err(error),
            }
        }
        Ok(())
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        #[cfg(unix)]
        #[test]
        fn consumption_acknowledgement_rejects_eof_invalid_and_cancelled_reads() {
            use std::os::fd::OwnedFd;
            use std::os::unix::net::UnixStream;
            let stop = AtomicBool::new(false);
            for (value, expected) in [
                (b"1".as_slice(), None),
                (b"".as_slice(), Some(io::ErrorKind::UnexpectedEof)),
                (b"x".as_slice(), Some(io::ErrorKind::InvalidData)),
            ] {
                let (mut send, receive) = UnixStream::pair().unwrap();
                receive.set_nonblocking(true).unwrap();
                send.write_all(value).unwrap();
                drop(send);
                let mut file = File::from(OwnedFd::from(receive));
                assert_eq!(
                    wait_for_consumption(&mut file, &stop)
                        .err()
                        .map(|error| error.kind()),
                    expected
                );
            }
            let (_send, receive) = UnixStream::pair().unwrap();
            receive.set_nonblocking(true).unwrap();
            stop.store(true, Ordering::Release);
            assert_eq!(
                wait_for_consumption(&mut File::from(OwnedFd::from(receive)), &stop)
                    .unwrap_err()
                    .kind(),
                io::ErrorKind::Interrupted
            );
        }

        #[test]
        fn permanent_prepare_and_native_write_failures_are_reported() {
            for needs_frame in [true, false] {
                let prepare: fn() -> io::Result<()> = if needs_frame {
                    || Ok(())
                } else {
                    || Err(io::ErrorKind::Unsupported.into())
                };
                // Read-only handle makes a real native write fail on both OSes;
                // the second iteration exercises failure before any I/O starts.
                let file = File::open(std::env::current_exe().unwrap()).unwrap();
                let mut writer = Writer::start(Some(file), prepare).unwrap();
                if needs_frame {
                    super::super::publish_until_accepted(|| {
                        writer.publish(b"bounded-frame")
                    });
                }
                let deadline = Instant::now() + Duration::from_secs(1);
                while !writer.thread.as_ref().unwrap().is_finished() {
                    assert!(
                        Instant::now() < deadline,
                        "failed native writer did not stop"
                    );
                    thread::sleep(Duration::from_millis(2));
                }
                assert_eq!(
                    writer.publish(b"retry").unwrap_err().kind(),
                    io::ErrorKind::BrokenPipe
                );
                assert!(writer.retire(|_| {}));
            }
        }

        #[test]
        fn short_writes_complete_the_frame_and_cancelled_interrupts_stop() {
            struct Short(Vec<u8>);
            impl Write for Short {
                fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
                    let count = bytes.len().min(3);
                    self.0.extend_from_slice(&bytes[..count]);
                    Ok(count)
                }
                fn flush(&mut self) -> io::Result<()> {
                    Ok(())
                }
            }
            let mut short = Short(Vec::new());
            let stop = AtomicBool::new(false);
            write_frame(&mut short, b"\x1b]frame\x07", &stop).unwrap();
            assert_eq!(short.0, b"\x1b]frame\x07");
            stop.store(true, Ordering::Release);
            assert_eq!(
                write_frame(&mut short, b"ignored", &stop)
                    .unwrap_err()
                    .kind(),
                io::ErrorKind::Interrupted
            );
        }
    }
}

#[cfg(unix)]
mod platform {
    use super::writer::Writer;
    use std::{
        fs::{File, OpenOptions},
        io,
        os::unix::{fs::OpenOptionsExt, thread::JoinHandleExt},
        sync::atomic::{AtomicBool, Ordering},
    };

    const CANCEL_SIGNAL: libc::c_int = libc::SIGUSR2;
    static SIGNAL_OWNED: AtomicBool = AtomicBool::new(false);
    #[cfg(test)]
    pub(super) static TEST_SIGNAL_OWNER: std::sync::Mutex<()> = std::sync::Mutex::new(());
    pub(super) struct Publisher {
        writer: Writer,
        signal: Option<CancelSignal>,
    }

    impl Publisher {
        pub(super) fn new() -> io::Result<Self> {
            // Darwin releases the terminal lock under backpressure, allowing
            // other output inside an OSC even during one blocking write.
            // Its helper must use the prompt-owned channel instead.
            if cfg!(target_os = "macos") {
                return Err(io::Error::new(
                    io::ErrorKind::Unsupported,
                    "macOS SSH metadata requires the prompt channel",
                ));
            }
            // Independent open: never change flags on the shell's shared file.
            // Linux retains its terminal write lock while waiting for capacity.
            let file = match OpenOptions::new()
                .write(true)
                .custom_flags(libc::O_CLOEXEC | libc::O_NOCTTY)
                .open("/dev/tty")
            {
                Ok(file) => Some(file),
                Err(error)
                    if matches!(
                        error.raw_os_error(),
                        Some(libc::ENOENT | libc::ENXIO | libc::ENODEV | libc::ENOTTY)
                    ) =>
                {
                    None
                }
                Err(error) => return Err(error),
            };
            Self::start(file)
        }
        fn start(file: Option<File>) -> io::Result<Self> {
            let signal = if file.is_some() {
                Some(CancelSignal::acquire()?)
            } else {
                None
            };
            let writer = Writer::start(file, mask_writer_signals)?;
            Ok(Self { writer, signal })
        }
        pub(super) fn prompt_channel(file: File, ready: File) -> io::Result<Self> {
            use std::os::fd::AsRawFd;
            for channel in [&file, &ready] {
                // SAFETY: both files own live descriptors; flags are read only.
                let flags = unsafe { libc::fcntl(channel.as_raw_fd(), libc::F_GETFL) };
                if flags < 0 {
                    return Err(io::Error::last_os_error());
                }
                if flags & libc::O_NONBLOCK == 0 {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "SSH prompt channels must be nonblocking",
                    ));
                }
            }
            Ok(Self {
                writer: Writer::start_notified(Some(file), Some(ready), || Ok(()))?,
                signal: None,
            })
        }
        pub(super) fn publish(&mut self, frame: &[u8]) -> io::Result<bool> {
            self.writer.publish(frame)
        }
        pub(super) fn retire(&mut self) -> bool {
            let needs_interrupt = self.signal.is_some();
            let retired = self.writer.retire(|thread| {
                if !needs_interrupt {
                    return;
                }
                // SAFETY: JoinHandle pins this thread until join. The reserved
                // no-restart handler interrupts only its write; pthread_cancel
                // and foreign unwinding are never used across Rust stack frames.
                unsafe {
                    libc::pthread_kill(thread.as_pthread_t(), CANCEL_SIGNAL);
                }
            });
            if !retired {
                return false;
            }
            if let Some(signal) = self.signal.as_mut() {
                if !signal.restore() {
                    return false;
                }
            }
            self.signal.take();
            true
        }
    }

    extern "C" fn interrupt_write(_: libc::c_int) {}
    struct CancelSignal {
        original: libc::sigaction,
        active: bool,
    }

    impl CancelSignal {
        fn acquire() -> io::Result<Self> {
            if SIGNAL_OWNED
                .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                .is_err()
            {
                return Err(io::Error::new(
                    io::ErrorKind::AlreadyExists,
                    "SSH metadata writer already exists",
                ));
            }
            // SAFETY: native POD action storage; sigaction retains no borrowed pointers.
            let (mut original, mut action): (libc::sigaction, libc::sigaction) =
                unsafe { (std::mem::zeroed(), std::mem::zeroed()) };
            if unsafe { libc::sigaction(CANCEL_SIGNAL, std::ptr::null(), &mut original) }
                != 0
            {
                SIGNAL_OWNED.store(false, Ordering::Release);
                return Err(io::Error::last_os_error());
            }
            if original.sa_sigaction != libc::SIG_DFL {
                SIGNAL_OWNED.store(false, Ordering::Release);
                return Err(io::Error::new(
                    io::ErrorKind::Unsupported,
                    "SSH metadata cancellation signal is already in use",
                ));
            }
            action.sa_sigaction = interrupt_write as *const () as usize;
            // SAFETY: valid native storage. Absence of SA_RESTART makes the
            // shutdown signal interrupt a blocked syscall on the writer thread.
            if unsafe { libc::sigemptyset(&mut action.sa_mask) } != 0
                || unsafe {
                    libc::sigaction(CANCEL_SIGNAL, &action, std::ptr::null_mut())
                } != 0
            {
                SIGNAL_OWNED.store(false, Ordering::Release);
                return Err(io::Error::last_os_error());
            }
            Ok(Self {
                original,
                active: true,
            })
        }
        fn restore(&mut self) -> bool {
            if !self.active {
                return true;
            }
            // SAFETY: captured native action; the writer is joined before restoration.
            if unsafe {
                libc::sigaction(CANCEL_SIGNAL, &self.original, std::ptr::null_mut())
            } != 0
            {
                return false;
            }
            self.active = false;
            SIGNAL_OWNED.store(false, Ordering::Release);
            true
        }
    }
    impl Drop for CancelSignal {
        fn drop(&mut self) {
            if !self.restore() {
                std::process::exit(70);
            }
        }
    }

    fn mask_writer_signals() -> io::Result<()> {
        // This dedicated thread never spawns children. Block ordinary signals
        // (including SIGTTOU while the shell owns the foreground terminal) so
        // only the explicit retirement interrupt or an unrecoverable process
        // fault can truncate a native write. Synchronous fault delivery remains
        // enabled: masking those faults would have undefined POSIX behavior.
        // SAFETY: valid local signal-set storage, changed only on this thread.
        let mut mask = unsafe { std::mem::zeroed() };
        if unsafe { libc::sigfillset(&mut mask) } != 0 {
            return Err(io::Error::last_os_error());
        }
        for signal in [
            CANCEL_SIGNAL,
            libc::SIGSEGV,
            libc::SIGBUS,
            libc::SIGILL,
            libc::SIGFPE,
            libc::SIGTRAP,
            libc::SIGABRT,
            libc::SIGSYS,
        ] {
            // SAFETY: initialized signal-set storage and native signal values.
            if unsafe { libc::sigdelset(&mut mask, signal) } != 0 {
                return Err(io::Error::last_os_error());
            }
        }
        let error = unsafe {
            libc::pthread_sigmask(libc::SIG_SETMASK, &mask, std::ptr::null_mut())
        };
        if error != 0 {
            return Err(io::Error::from_raw_os_error(error));
        }
        Ok(())
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use std::{
            io::{Read, Write},
            os::fd::{AsRawFd, FromRawFd, OwnedFd},
            thread,
            time::{Duration, Instant},
        };
        struct Terminal {
            // Retain both native endpoints even where only the path is used.
            #[cfg_attr(target_os = "macos", allow(dead_code))]
            master: File,
            #[cfg_attr(target_os = "macos", allow(dead_code))]
            slave: OwnedFd,
            path: String,
        }
        impl Terminal {
            fn new() -> Self {
                let (mut master, mut slave) = (-1, -1);
                // SAFETY: writable descriptor slots, null optional default settings.
                assert_eq!(
                    unsafe {
                        libc::openpty(
                            &mut master,
                            &mut slave,
                            std::ptr::null_mut(),
                            std::ptr::null_mut(),
                            std::ptr::null_mut(),
                        )
                    },
                    0
                );
                // SAFETY: successful openpty transferred unique owned descriptors.
                let master = unsafe { File::from_raw_fd(master) };
                let slave = unsafe { OwnedFd::from_raw_fd(slave) };
                let mut path = [0_u8; 256];
                // SAFETY: live fd and bounded writable filename storage.
                assert_eq!(
                    unsafe {
                        libc::ttyname_r(
                            slave.as_raw_fd(),
                            path.as_mut_ptr().cast(),
                            path.len(),
                        )
                    },
                    0
                );
                let end = path.iter().position(|&byte| byte == 0).unwrap();
                let path = std::str::from_utf8(&path[..end]).unwrap().to_string();
                // SAFETY: these descriptors stay owned throughout the test.
                assert_eq!(
                    unsafe {
                        libc::fcntl(master.as_raw_fd(), libc::F_SETFL, libc::O_NONBLOCK)
                    },
                    0
                );
                let mut modes = unsafe { std::mem::zeroed() };
                assert_eq!(unsafe { libc::tcgetattr(slave.as_raw_fd(), &mut modes) }, 0);
                modes.c_oflag &= !libc::OPOST;
                assert_eq!(
                    unsafe { libc::tcsetattr(slave.as_raw_fd(), libc::TCSANOW, &modes) },
                    0
                );
                Self {
                    master,
                    slave,
                    path,
                }
            }
            fn writer(&self, nonblocking: bool) -> File {
                OpenOptions::new()
                    .write(true)
                    .custom_flags(
                        libc::O_NOCTTY | if nonblocking { libc::O_NONBLOCK } else { 0 },
                    )
                    .open(&self.path)
                    .unwrap()
            }
            fn fill(&self) -> usize {
                let mut filler = self.writer(true);
                let mut count = 0;
                loop {
                    match filler.write(&[b'x'; 4096]) {
                        Ok(written) => count += written,
                        Err(error) if error.kind() == io::ErrorKind::WouldBlock => break,
                        other => panic!("unexpected terminal fill result: {other:?}"),
                    }
                    assert!(count < 1024 * 1024);
                }
                count
            }
        }
        fn wait_for_write(publisher: &Publisher) {
            let deadline = Instant::now() + Duration::from_secs(1);
            while !publisher.writer.state.writing.load(Ordering::Acquire) {
                assert!(Instant::now() < deadline, "native write did not start");
                thread::sleep(Duration::from_millis(2));
            }
            thread::sleep(Duration::from_millis(20));
        }
        fn prompt_channel_pair() -> (Publisher, File, File) {
            let (send, receive) = std::os::unix::net::UnixStream::pair().unwrap();
            let (notify, ready) = std::os::unix::net::UnixDatagram::pair().unwrap();
            for socket in [&send, &receive] {
                socket.set_nonblocking(true).unwrap();
            }
            for socket in [&notify, &ready] {
                socket.set_nonblocking(true).unwrap();
            }
            (
                Publisher::prompt_channel(
                    File::from(OwnedFd::from(send)),
                    File::from(OwnedFd::from(notify)),
                )
                .unwrap(),
                File::from(OwnedFd::from(receive)),
                File::from(OwnedFd::from(ready)),
            )
        }

        #[test]
        fn prompt_channel_rejects_a_blocking_endpoint_before_starting_a_worker() {
            for blocking_data in [false, true] {
                let (send, _receive) = std::os::unix::net::UnixStream::pair().unwrap();
                let (notify, _ready) = std::os::unix::net::UnixDatagram::pair().unwrap();
                send.set_nonblocking(!blocking_data).unwrap();
                notify.set_nonblocking(blocking_data).unwrap();
                assert!(matches!(
                    Publisher::prompt_channel(
                        File::from(OwnedFd::from(send)),
                        File::from(OwnedFd::from(notify))
                    ),
                    Err(error) if error.kind() == io::ErrorKind::InvalidInput
                ));
            }
        }

        #[test]
        fn prompt_channel_contention_declines_without_blocking_or_mutating_pending() {
            let (mut publisher, mut receive, mut ready) = prompt_channel_pair();
            let state = std::sync::Arc::clone(&publisher.writer.state);
            let pending = state.pending.lock().unwrap();
            let (sent, received) = std::sync::mpsc::channel();
            let attempt = thread::spawn(move || {
                let result = publisher
                    .publish(b"deferred\0")
                    .map_err(|error| error.kind());
                sent.send(result).unwrap();
                publisher
            });
            let declined = received.recv_timeout(Duration::from_secs(1));
            let unchanged = pending.is_none();
            // Release before joining/asserting, so a blocking-lock regression
            // reports failure rather than deadlocking the test runner.
            drop(pending);
            let mut publisher = attempt.join().unwrap();
            assert_eq!(declined, Ok(Ok(false)));
            assert!(unchanged);
            super::super::publish_until_accepted(|| publisher.publish(b"complete\0"));
            let deadline = Instant::now() + Duration::from_secs(2);
            let mut ack = [0; 1];
            loop {
                match ready.read(&mut ack) {
                    Ok(1) => break,
                    Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                        assert!(Instant::now() < deadline);
                        thread::sleep(Duration::from_millis(1));
                    }
                    result => panic!("unexpected readiness result: {result:?}"),
                }
            }
            assert_eq!(ack, *b"1");
            let mut frame = [0; 9];
            receive.read_exact(&mut frame).unwrap();
            assert_eq!(frame, *b"complete\0");
            assert_eq!(
                receive.read(&mut [0; 1]).unwrap_err().kind(),
                io::ErrorKind::WouldBlock
            );
            assert!(publisher.retire());
        }

        #[test]
        fn prompt_channel_acknowledges_only_complete_frames_and_retires_without_signals()
        {
            let (mut publisher, mut receive, mut ready) = prompt_channel_pair();
            let frame = [b'A'; super::super::MAX_FRAME_BYTES + 1];
            super::super::publish_until_accepted(|| publisher.publish(&frame));
            let deadline = Instant::now() + Duration::from_secs(2);
            let mut ack = [0; 1];
            loop {
                match ready.read(&mut ack) {
                    Ok(1) => break,
                    Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                        assert!(Instant::now() < deadline);
                        thread::sleep(Duration::from_millis(1));
                    }
                    result => panic!("unexpected readiness result: {result:?}"),
                }
            }
            assert_eq!(ack, *b"1");
            let mut actual = vec![0; frame.len()];
            // Ack means a complete frame is present, so a nonblocking read
            // must not encounter EAGAIN halfway through it.
            receive.read_exact(&mut actual).unwrap();
            assert_eq!(actual, frame);
            assert!(publisher.signal.is_none());
            assert!(publisher.retire());
        }

        #[test]
        fn prompt_channel_waits_for_consumption_and_coalesces_idle_updates() {
            let (mut publisher, mut receive, mut ready) = prompt_channel_pair();
            super::super::publish_until_accepted(|| publisher.publish(b"first\0"));
            let mut ack = [0; 1];
            let deadline = Instant::now() + Duration::from_secs(2);
            loop {
                match ready.read(&mut ack) {
                    Ok(1) => break,
                    Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                        assert!(Instant::now() < deadline);
                        thread::sleep(Duration::from_millis(1));
                    }
                    result => panic!("unexpected readiness result: {result:?}"),
                }
            }
            let mut first = [0; 6];
            receive.read_exact(&mut first).unwrap();
            assert_eq!(first, *b"first\0");
            // The consumer has not acknowledged completion. Idle refreshes
            // must replace pending state instead of filling a stale FIFO.
            for index in 0..256 {
                super::super::publish_until_accepted(|| {
                    publisher.publish(format!("latest-{index}\0").as_bytes())
                });
                thread::sleep(Duration::from_millis(1));
            }
            assert_eq!(
                ready.read(&mut ack).unwrap_err().kind(),
                io::ErrorKind::WouldBlock
            );
            assert_eq!(
                receive.read(&mut [0; 1]).unwrap_err().kind(),
                io::ErrorKind::WouldBlock
            );
            receive.write_all(b"1").unwrap();
            let deadline = Instant::now() + Duration::from_secs(2);
            loop {
                match ready.read(&mut ack) {
                    Ok(1) => break,
                    Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                        assert!(Instant::now() < deadline);
                        thread::sleep(Duration::from_millis(1));
                    }
                    result => panic!("unexpected readiness result: {result:?}"),
                }
            }
            let mut latest = [0; 11];
            receive.read_exact(&mut latest).unwrap();
            assert_eq!(latest, *b"latest-255\0");
            assert!(publisher.retire());
        }

        #[test]
        fn full_prompt_channel_never_acknowledges_partial_output_and_cancels() {
            let (send, receive) = std::os::unix::net::UnixStream::pair().unwrap();
            send.set_nonblocking(true).unwrap();
            receive.set_nonblocking(true).unwrap();
            let mut file = File::from(OwnedFd::from(send));
            let mut filled = 0;
            loop {
                match file.write(&[b'x'; 4096]) {
                    Ok(count) => filled += count,
                    Err(error) if error.kind() == io::ErrorKind::WouldBlock => break,
                    other => panic!("unexpected channel fill: {other:?}"),
                }
                assert!(filled < 4 * 1024 * 1024);
            }
            let (notify, ready) = std::os::unix::net::UnixDatagram::pair().unwrap();
            notify.set_nonblocking(true).unwrap();
            ready.set_nonblocking(true).unwrap();
            let mut ready = File::from(OwnedFd::from(ready));
            let mut publisher =
                Publisher::prompt_channel(file, File::from(OwnedFd::from(notify)))
                    .unwrap();
            super::super::publish_until_accepted(|| {
                publisher.publish(&[b'A'; super::super::MAX_FRAME_BYTES + 1])
            });
            wait_for_write(&publisher);
            assert_eq!(
                ready.read(&mut [0; 1]).unwrap_err().kind(),
                io::ErrorKind::WouldBlock
            );
            for index in 0..256 {
                super::super::publish_until_accepted(|| {
                    publisher.publish(format!("latest-{index}").as_bytes())
                });
            }
            assert_eq!(
                publisher.writer.state.pending.lock().unwrap().as_deref(),
                Some(b"latest-255".as_slice())
            );
            let started = Instant::now();
            assert!(publisher.retire());
            assert!(started.elapsed() < Duration::from_secs(2));
            assert!(publisher.writer.thread.is_none());
            assert!(publisher.signal.is_none());
            drop(receive);
        }

        #[cfg(target_os = "macos")]
        #[test]
        fn darwin_rejects_competing_background_terminal_metadata_writer() {
            assert!(
                matches!(Publisher::new(), Err(error) if error.kind() == io::ErrorKind::Unsupported)
            );
        }

        #[cfg(not(target_os = "macos"))]
        #[test]
        fn full_terminal_finishes_osc_before_following_command_output() {
            let _serial = TEST_SIGNAL_OWNER
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            let mut terminal = Terminal::new();
            let filled = terminal.fill();
            let before =
                unsafe { libc::fcntl(terminal.slave.as_raw_fd(), libc::F_GETFL) };
            let mut publisher = Publisher::start(Some(terminal.writer(false))).unwrap();
            let frame = format!("\x1b]1337;SetUserVar=example={}\x07", "A".repeat(6098));
            super::super::publish_until_accepted(|| publisher.publish(frame.as_bytes()));
            wait_for_write(&publisher);
            let mut peer = terminal.writer(false);
            let command =
                thread::spawn(move || peer.write_all(b"FOLLOWING_COMMAND_OUTPUT\n"));
            let expected_len = filled + frame.len() + b"FOLLOWING_COMMAND_OUTPUT\n".len();
            let mut received = Vec::new();
            let mut bytes = [0; 4096];
            let deadline = Instant::now() + Duration::from_secs(2);
            while received.len() < expected_len && Instant::now() < deadline {
                match terminal.master.read(&mut bytes) {
                    Ok(count) => received.extend_from_slice(&bytes[..count]),
                    Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(1))
                    }
                    other => panic!("unexpected terminal read: {other:?}"),
                }
            }
            assert!(publisher.retire());
            // Closing master releases a failing peer before an assertion unwinds.
            drop(terminal.master);
            command.join().unwrap().unwrap();
            let mut expected = vec![b'x'; filled];
            expected.extend_from_slice(frame.as_bytes());
            expected.extend_from_slice(b"FOLLOWING_COMMAND_OUTPUT\n");
            assert_eq!(
                received, expected,
                "an OSC prefix consumed following command output"
            );
            assert_eq!(
                unsafe { libc::fcntl(terminal.slave.as_raw_fd(), libc::F_GETFL) },
                before
            );
        }
        #[test]
        fn full_terminal_writer_cancels_and_joins_without_a_reader() {
            let _serial = TEST_SIGNAL_OWNER
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            let terminal = Terminal::new();
            terminal.fill();
            let mut publisher = Publisher::start(Some(terminal.writer(false))).unwrap();
            super::super::publish_until_accepted(|| {
                publisher.publish(&[b'x'; super::super::MAX_FRAME_BYTES])
            });
            wait_for_write(&publisher);
            for index in 0..256 {
                super::super::publish_until_accepted(|| {
                    publisher.publish(format!("latest-{index}").as_bytes())
                });
            }
            assert_eq!(
                publisher.writer.state.pending.lock().unwrap().as_deref(),
                Some(b"latest-255".as_slice())
            );
            let started = Instant::now();
            assert!(
                publisher.retire(),
                "native terminal write did not acknowledge cancellation"
            );
            assert!(started.elapsed() < Duration::from_secs(2));
            assert!(publisher.writer.thread.is_none());
            assert!(!SIGNAL_OWNED.load(Ordering::Acquire));
        }
        #[test]
        fn writer_restores_signal_ownership_and_repeated_idle_shutdown() {
            let _serial = TEST_SIGNAL_OWNER
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            let terminal = Terminal::new();
            let mut before: libc::sigaction = unsafe { std::mem::zeroed() };
            assert_eq!(
                unsafe { libc::sigaction(CANCEL_SIGNAL, std::ptr::null(), &mut before) },
                0
            );
            for _ in 0..64 {
                let mut publisher =
                    Publisher::start(Some(terminal.writer(false))).unwrap();
                assert!(Publisher::start(Some(terminal.writer(false))).is_err());
                assert!(publisher.retire());
            }
            let mut after: libc::sigaction = unsafe { std::mem::zeroed() };
            assert_eq!(
                unsafe { libc::sigaction(CANCEL_SIGNAL, std::ptr::null(), &mut after) },
                0
            );
            assert_eq!(before.sa_sigaction, after.sa_sigaction);
        }

        #[test]
        fn existing_signal_handler_is_preserved_and_declines_writer_setup() {
            let _serial = TEST_SIGNAL_OWNER
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            let mut original: libc::sigaction = unsafe { std::mem::zeroed() };
            let mut occupied: libc::sigaction = unsafe { std::mem::zeroed() };
            occupied.sa_sigaction = libc::SIG_IGN;
            // SAFETY: serial test exclusively owns this signal, restoring its
            // captured disposition immediately after the attempted acquisition.
            assert_eq!(unsafe { libc::sigemptyset(&mut occupied.sa_mask) }, 0);
            assert_eq!(
                unsafe { libc::sigaction(CANCEL_SIGNAL, &occupied, &mut original) },
                0
            );
            let rejected = CancelSignal::acquire().is_err();
            let mut after: libc::sigaction = unsafe { std::mem::zeroed() };
            assert_eq!(
                unsafe { libc::sigaction(CANCEL_SIGNAL, &original, &mut after) },
                0
            );
            assert!(rejected);
            assert_eq!(after.sa_sigaction, libc::SIG_IGN);
            assert!(!SIGNAL_OWNED.load(Ordering::Acquire));
        }
    }
}

#[cfg(windows)]
mod platform {
    use super::writer::Writer;
    use std::{
        fs::File,
        io,
        os::windows::io::{AsHandle, AsRawHandle},
    };
    use windows_sys::Win32::{
        Storage::FileSystem::{GetFileType, FILE_TYPE_CHAR, FILE_TYPE_PIPE},
        System::{
            Console::GetConsoleMode,
            Pipes::{GetNamedPipeHandleStateW, PIPE_NOWAIT},
            IO::CancelSynchronousIo,
        },
    };
    pub(super) struct Publisher {
        writer: Writer,
    }
    impl Publisher {
        pub(super) fn new() -> io::Result<Self> {
            let file: File = io::stderr().as_handle().try_clone_to_owned()?.into();
            let mut console_mode = 0;
            // SAFETY: owned live handle and valid writable output slots.
            let kind = unsafe { GetFileType(file.as_raw_handle()) };
            let supported = if kind == FILE_TYPE_PIPE {
                let mut mode = 0;
                if unsafe {
                    GetNamedPipeHandleStateW(
                        file.as_raw_handle(),
                        &mut mode,
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                        0,
                    )
                } == 0
                {
                    return Err(io::Error::last_os_error());
                }
                if mode & PIPE_NOWAIT != 0 {
                    return Err(io::Error::new(
                        io::ErrorKind::Unsupported,
                        "SSH metadata output requires a blocking pipe",
                    ));
                }
                true
            } else {
                kind == FILE_TYPE_CHAR
                    && unsafe { GetConsoleMode(file.as_raw_handle(), &mut console_mode) }
                        != 0
            };
            Self::start(supported.then_some(file))
        }
        fn start(file: Option<File>) -> io::Result<Self> {
            Ok(Self {
                writer: Writer::start(file, || Ok(()))?,
            })
        }
        pub(super) fn publish(&mut self, frame: &[u8]) -> io::Result<bool> {
            self.writer.publish(frame)
        }
        pub(super) fn retire(&mut self) -> bool {
            self.writer.retire(|thread| {
                // SAFETY: JoinHandle pins only this dedicated thread. Native
                // cancellation is requested after all shell output has ceased.
                unsafe {
                    CancelSynchronousIo(thread.as_raw_handle());
                }
            })
        }
    }
    #[cfg(test)]
    mod tests {
        use super::*;
        use std::{
            io::{Read, Write},
            os::windows::io::{FromRawHandle, OwnedHandle},
            sync::atomic::Ordering,
            thread,
            time::{Duration, Instant},
        };
        use windows_sys::Win32::System::Pipes::{CreatePipe, PeekNamedPipe};
        fn pipe() -> (File, File) {
            let (mut read, mut write) = (std::ptr::null_mut(), std::ptr::null_mut());
            // SAFETY: writable output slots; CreatePipe transfers unique handles.
            assert_ne!(
                unsafe { CreatePipe(&mut read, &mut write, std::ptr::null(), 4096) },
                0
            );
            unsafe {
                (
                    File::from(OwnedHandle::from_raw_handle(read)),
                    File::from(OwnedHandle::from_raw_handle(write)),
                )
            }
        }
        fn wait_for_pipe(read: &File) {
            let deadline = Instant::now() + Duration::from_secs(1);
            let mut available = 0;
            while available == 0 && Instant::now() < deadline {
                // SAFETY: live pipe, bounded query with no data read.
                assert_ne!(
                    unsafe {
                        PeekNamedPipe(
                            read.as_raw_handle(),
                            std::ptr::null_mut(),
                            0,
                            std::ptr::null_mut(),
                            &mut available,
                            std::ptr::null_mut(),
                        )
                    },
                    0
                );
                thread::sleep(Duration::from_millis(2));
            }
            assert!(available > 0, "native blocking write never started");
        }
        #[test]
        fn full_native_pipe_writer_cancels_and_joins_without_a_reader() {
            let (read, file) = pipe();
            let mut publisher = Publisher::start(Some(file)).unwrap();
            super::super::publish_until_accepted(|| {
                publisher.publish(&vec![b'x'; super::super::MAX_FRAME_BYTES])
            });
            wait_for_pipe(&read);
            assert!(publisher.writer.state.writing.load(Ordering::Acquire));
            for index in 0..256 {
                super::super::publish_until_accepted(|| {
                    publisher.publish(format!("latest-{index}").as_bytes())
                });
            }
            assert_eq!(
                publisher.writer.state.pending.lock().unwrap().as_deref(),
                Some(b"latest-255".as_slice())
            );
            assert!(
                publisher.retire(),
                "native pipe write did not acknowledge cancellation"
            );
            assert!(publisher.writer.thread.is_none());
        }
        #[test]
        fn full_native_pipe_completes_frame_before_following_text() {
            let (mut read, file) = pipe();
            let mut peer = file.try_clone().unwrap();
            let mut publisher = Publisher::start(Some(file)).unwrap();
            let frame = format!("\x1b]1337;SetUserVar=example={}\x07", "A".repeat(6098));
            super::super::publish_until_accepted(|| publisher.publish(frame.as_bytes()));
            wait_for_pipe(&read);
            let command =
                thread::spawn(move || peer.write_all(b"FOLLOWING_COMMAND_OUTPUT\n"));
            let expected_len = frame.len() + b"FOLLOWING_COMMAND_OUTPUT\n".len();
            let mut received = Vec::new();
            let deadline = Instant::now() + Duration::from_secs(2);
            let mut bytes = [0; 4096];
            while received.len() < expected_len && Instant::now() < deadline {
                let mut available = 0;
                // SAFETY: owned read handle and bounded output-only size query.
                assert_ne!(
                    unsafe {
                        PeekNamedPipe(
                            read.as_raw_handle(),
                            std::ptr::null_mut(),
                            0,
                            std::ptr::null_mut(),
                            &mut available,
                            std::ptr::null_mut(),
                        )
                    },
                    0
                );
                if available == 0 {
                    thread::sleep(Duration::from_millis(1));
                    continue;
                }
                let count = (available as usize).min(bytes.len());
                let count = read.read(&mut bytes[..count]).unwrap();
                received.extend_from_slice(&bytes[..count]);
            }
            let retired = publisher.retire();
            drop(read); // Release a failed peer write before joining or asserting.
            let command_result = command.join().unwrap();
            assert!(retired);
            command_result.unwrap();
            let mut expected = frame.into_bytes();
            expected.extend_from_slice(b"FOLLOWING_COMMAND_OUTPUT\n");
            assert_eq!(received, expected);
        }
        #[test]
        fn repeated_idle_shutdown_does_not_lose_the_stop_notification() {
            for _ in 0..64 {
                let (_read, file) = pipe();
                let mut publisher = Publisher::start(Some(file)).unwrap();
                assert!(publisher.retire());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn oversized_frame_is_rejected_before_native_output() {
        #[cfg(unix)]
        let mut publisher = {
            let (send, _receive) = std::os::unix::net::UnixStream::pair().unwrap();
            let (notify, _ready) = std::os::unix::net::UnixDatagram::pair().unwrap();
            send.set_nonblocking(true).unwrap();
            notify.set_nonblocking(true).unwrap();
            Publisher::prompt_channel(
                std::fs::File::from(std::os::fd::OwnedFd::from(send)),
                std::fs::File::from(std::os::fd::OwnedFd::from(notify)),
            )
            .unwrap()
        };
        #[cfg(windows)]
        let mut publisher = Publisher::new().unwrap();
        assert_eq!(
            publisher
                .publish(&"x".repeat(MAX_FRAME_BYTES + 1))
                .unwrap_err()
                .kind(),
            io::ErrorKind::InvalidInput
        );
        #[cfg(unix)]
        assert_eq!(
            publisher.publish("frame\0injected").unwrap_err().kind(),
            io::ErrorKind::InvalidInput
        );
        assert!(publisher.finish().is_ok());
    }
}
