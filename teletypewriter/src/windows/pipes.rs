use crate::windows::spsc::*;
use corcovado::{
    event::Evented, Poll, PollOpt, Ready, Registration, SetReadiness, Token,
};
use miow::pipe::{AnonRead, AnonWrite};
use parking_lot::{Condvar, Mutex};
use windows_sys::Win32::System::IO::CancelSynchronousIo;

use std::io;
use std::os::windows::io::AsRawHandle;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc::{channel, Receiver, TryRecvError},
    Arc,
};
use std::thread::{spawn, JoinHandle};

struct WaitTag {}

struct EventedAnonReadInner {
    registration: Registration,
    readiness: SetReadiness,
    done: AtomicBool,
    discard: AtomicBool,
    sig_buffer_not_full: Condvar,
    wait_tag: Mutex<WaitTag>,
}

/// Wraps an AnonRead pipe so that it can be read asynchronously using mio.
///
/// This is achieved by spawning a worker thread which continuously attempts
/// to read from the pipe into a buffer, which reads from the EventedAnonRead
/// object will be directed to.
///
/// This should only be considered if your application architecture requires
/// a synchronous anonymous pipe; an asynchronous NamedPipe will likely be
/// more performant.
pub struct EventedAnonRead {
    // Is an Option so it can be moved out and joined in the Drop impl.
    thread: Option<JoinHandle<()>>,
    consumer: SpscBufferReader,
    inner: Arc<EventedAnonReadInner>,
    error_receiver: Receiver<String>,
}

// Helper to send an error string from the worker threads
macro_rules! try_or_send {
    ($e:expr, $sender:ident) => {
        match $e {
            Ok(value) => value,
            Err(e) => {
                $sender
                    .send(format!("{}", e))
                    .expect("Could not send error");
                return;
            }
        }
    };
}

impl EventedAnonRead {
    #[cfg(test)]
    pub(super) fn is_saturated(&self) -> bool {
        self.consumer.is_full()
    }

    /// Irreversible consumer retirement. Keep draining the native pipe while
    /// ConPTY closes, even when the ring has no remaining VT consumer.
    pub(super) fn discard_remaining(&self) {
        let _wait_tag = self.inner.wait_tag.lock();
        self.inner.discard.store(true, Ordering::SeqCst);
        self.inner.sig_buffer_not_full.notify_one();
    }

    pub fn new(mut pipe: AnonRead) -> Self {
        let (registration, readiness) = Registration::new2();

        let (mut producer, consumer) = spsc_buffer(65536);

        let done = AtomicBool::new(false);

        let sig_buffer_not_full = Condvar::new();
        let wait_tag = Mutex::new(WaitTag {});

        let (error_sender, error_receiver) = channel();

        let inner = Arc::new(EventedAnonReadInner {
            registration,
            readiness,
            done,
            discard: AtomicBool::new(false),
            sig_buffer_not_full,
            wait_tag,
        });

        let thread = {
            let inner = inner.clone();
            spawn(move || {
                use std::io::Read;

                let mut tmp_buf = [0u8; 65535];

                loop {
                    if inner.done.load(Ordering::SeqCst) {
                        return;
                    }

                    // Read into temp buffer
                    let nbytes = try_or_send!(pipe.read(&mut tmp_buf[..]), error_sender);
                    if nbytes == 0 {
                        return;
                    }

                    // Write from the temp buffer into the producer
                    let mut written = 0usize;
                    while written < nbytes {
                        // Hold the predicate mutex while checking and waiting.
                        // Without this handshake a consumer could empty the
                        // buffer and notify between `is_full` and `wait`,
                        // leaving ConPTY output asleep until unrelated I/O.
                        let mut wait_tag = inner.wait_tag.lock();
                        while producer.is_full()
                            && !inner.done.load(Ordering::SeqCst)
                            && !inner.discard.load(Ordering::SeqCst)
                        {
                            inner.sig_buffer_not_full.wait(&mut wait_tag);
                        }
                        if inner.done.load(Ordering::SeqCst) {
                            return;
                        }
                        if inner.discard.load(Ordering::SeqCst) {
                            break;
                        }

                        written += producer.write_from_slice(&tmp_buf[written..nbytes]);

                        if !inner.readiness.readiness().is_readable() {
                            try_or_send!(
                                inner.readiness.set_readiness(Ready::readable()),
                                error_sender
                            );
                        }
                    }
                }
            })
        };

        Self {
            thread: Some(thread),
            consumer,
            inner,
            error_receiver,
        }
    }
}

impl io::Read for EventedAnonRead {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        if self.thread.is_none() {
            return Err(io::Error::new(io::ErrorKind::BrokenPipe, ""));
        }

        // Synchronize final output and EOF: never report closure ahead of
        // bytes that the producer already committed to the bounded ring.
        let _wait_tag = self.inner.wait_tag.lock();
        if self.consumer.is_empty() {
            match self.error_receiver.try_recv() {
                Ok(err) => {
                    // Other thread will be closing
                    self.thread.take().unwrap().join().unwrap();
                    return Err(io::Error::new(io::ErrorKind::BrokenPipe, err));
                }
                Err(TryRecvError::Disconnected) => {
                    return Err(io::Error::new(io::ErrorKind::BrokenPipe, ""))
                }
                Err(TryRecvError::Empty) => {}
            }
        }

        // Pair the buffer mutation and notification with the producer's
        // predicate check so a wakeup cannot be lost.
        let nbytes = self.consumer.read_to_slice(buf);

        if self.consumer.is_empty() {
            self.inner.readiness.set_readiness(Ready::empty())?;

            // Possible race: the consumer may think the queue is empty but by the time
            // the readiness is set the producer thread may have written data
            //
            // We avoid the race by re-checking the queue is empty like this, and undo the
            // readiness setting if necessary.
            if !self.consumer.is_empty() {
                self.inner.readiness.set_readiness(Ready::readable())?;
            }
        }

        self.inner.sig_buffer_not_full.notify_one();
        Ok(nbytes)
    }
}

impl Evented for EventedAnonRead {
    fn register(
        &self,
        poll: &Poll,
        token: Token,
        interest: Ready,
        opts: PollOpt,
    ) -> io::Result<()> {
        poll.register(&self.inner.registration, token, interest, opts)
    }

    fn reregister(
        &self,
        poll: &Poll,
        token: Token,
        interest: Ready,
        opts: PollOpt,
    ) -> io::Result<()> {
        poll.reregister(&self.inner.registration, token, interest, opts)
    }

    fn deregister(&self, poll: &Poll) -> io::Result<()> {
        poll.deregister(&self.inner.registration)
    }
}

impl Drop for EventedAnonRead {
    fn drop(&mut self) {
        {
            let _wait_tag = self.inner.wait_tag.lock();
            self.inner.done.store(true, Ordering::SeqCst);
            self.inner.sig_buffer_not_full.notify_one();
        }

        // The read-error path may already have joined this worker.
        let Some(thread) = self.thread.take() else {
            return;
        };

        // Stop reader thread waiting for pipe contents
        unsafe {
            CancelSynchronousIo(thread.as_raw_handle());
        }

        thread
            .join()
            .expect("Could not close EventedAnonRead worker");
    }
}

struct EventedAnonWriteInner {
    registration: Registration,
    readiness: SetReadiness,
    done: AtomicBool,
    sig_buffer_not_empty: Condvar,
    wait_tag: Mutex<WaitTag>,
}

/// Wraps an AnonWrite pipe so that it can be written asynchronously using mio.
///
/// This is achieved by spawning a worker thread which continuously attempts
/// to write to the pipe from a buffer, which writes to the EventedAnonWrite
/// object will be directed to.
///
/// This should only be considered if your application architecture requires
/// a synchronous anonymous pipe; an asynchronous NamedPipe will likely be
/// more performant.
pub struct EventedAnonWrite {
    // Is an Option so it can be moved out and joined in the Drop impl
    thread: Option<JoinHandle<()>>,
    producer: SpscBufferWriter,
    inner: Arc<EventedAnonWriteInner>,
    error_receiver: Receiver<String>,
}

impl EventedAnonWrite {
    pub fn new(mut pipe: AnonWrite) -> Self {
        let (registration, readiness) = Registration::new2();

        let (producer, mut consumer) = spsc_buffer(65536);

        let done = AtomicBool::new(false);

        let sig_buffer_not_empty = Condvar::new();
        let wait_tag = Mutex::new(WaitTag {});

        let inner = Arc::new(EventedAnonWriteInner {
            registration,
            readiness,
            done,
            sig_buffer_not_empty,
            wait_tag,
        });

        let (error_sender, error_receiver) = channel();

        let thread = {
            let inner = inner.clone();
            spawn(move || {
                use std::io::Write;
                let mut tmp_buf = [0u8; 65535];

                try_or_send!(
                    inner.readiness.set_readiness(Ready::writable()),
                    error_sender
                );

                loop {
                    if inner.done.load(Ordering::SeqCst) {
                        return;
                    }

                    // Check the empty predicate while holding the same mutex
                    // used by the producer. This closes the check-then-wait
                    // race which could leave freshly queued keyboard input
                    // stuck until a later key or resize woke the thread.
                    let nbytes = {
                        let mut wait_tag = inner.wait_tag.lock();
                        while consumer.is_empty() && !inner.done.load(Ordering::SeqCst) {
                            inner.sig_buffer_not_empty.wait(&mut wait_tag);
                        }
                        if inner.done.load(Ordering::SeqCst) {
                            return;
                        }

                        let nbytes = consumer.read_to_slice(&mut tmp_buf);

                        if !inner.readiness.readiness().is_writable() {
                            try_or_send!(
                                inner.readiness.set_readiness(Ready::writable()),
                                error_sender
                            );
                        }

                        nbytes
                    };

                    let mut written = 0usize;
                    while written < nbytes {
                        written += try_or_send!(
                            pipe.write(&tmp_buf[written..nbytes]),
                            error_sender
                        );
                    }
                }
            })
        };

        Self {
            thread: Some(thread),
            producer,
            inner,
            error_receiver,
        }
    }
}

impl io::Write for EventedAnonWrite {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if self.thread.is_none() {
            return Err(io::Error::new(io::ErrorKind::BrokenPipe, ""));
        }

        match self.error_receiver.try_recv() {
            Ok(err) => {
                // Other thread will be closing
                self.thread.take().unwrap().join().unwrap();
                return Err(io::Error::new(io::ErrorKind::BrokenPipe, err));
            }
            Err(TryRecvError::Disconnected) => {
                return Err(io::Error::new(io::ErrorKind::BrokenPipe, ""))
            }
            Err(TryRecvError::Empty) => {}
        }

        // Mutate the predicate and notify while holding the consumer's mutex;
        // `Condvar::wait` releases it atomically, so the notification cannot
        // arrive in the gap before the consumer actually sleeps.
        let _wait_tag = self.inner.wait_tag.lock();
        let nbytes = self.producer.write_from_slice(buf);
        if self.producer.is_full() {
            self.inner.readiness.set_readiness(Ready::empty())?;

            // Possible race: the producer may think the buffer is full but by the time
            // the readiness is set the consumer thread may have read data
            //
            // It is sufficient to re-check the buffer is empty, and undo the readiness
            // setting to work around this.
            if !self.producer.is_full() {
                self.inner.readiness.set_readiness(Ready::writable())?;
            }
        }

        self.inner.sig_buffer_not_empty.notify_one();
        Ok(nbytes)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Evented for EventedAnonWrite {
    fn register(
        &self,
        poll: &Poll,
        token: Token,
        interest: Ready,
        opts: PollOpt,
    ) -> io::Result<()> {
        poll.register(&self.inner.registration, token, interest, opts)
    }

    fn reregister(
        &self,
        poll: &Poll,
        token: Token,
        interest: Ready,
        opts: PollOpt,
    ) -> io::Result<()> {
        poll.reregister(&self.inner.registration, token, interest, opts)
    }

    fn deregister(&self, poll: &Poll) -> io::Result<()> {
        poll.deregister(&self.inner.registration)
    }
}

impl Drop for EventedAnonWrite {
    fn drop(&mut self) {
        {
            let _wait_tag = self.inner.wait_tag.lock();
            self.inner.done.store(true, Ordering::SeqCst);

            // Stop the writer thread waiting for contents.
            self.inner.sig_buffer_not_empty.notify_one();
        }

        // The write-error path may already have joined this worker.
        if let Some(thread) = self.thread.take() {
            thread
                .join()
                .expect("Could not close EventedAnonWrite worker");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{EventedAnonRead, EventedAnonWrite};
    use std::io::{Read, Write};
    use std::sync::mpsc;
    use std::time::{Duration, Instant};

    #[test]
    fn saturated_output_drains_after_the_terminal_consumer_retires() {
        let (pipe_reader, mut pipe_writer) = miow::pipe::anonymous(0).unwrap();
        let mut reader = EventedAnonRead::new(pipe_reader);
        let (sent, received) = mpsc::channel();
        let writer = std::thread::spawn(move || {
            let result = pipe_writer.write_all(&[b'x'; 1024 * 1024]);
            sent.send(result.is_ok()).unwrap();
        });
        // Reproduce a full native output ring, not a mocked worker completion.
        // ConPTY may produce more bytes while its owner is being closed.
        let deadline = Instant::now() + Duration::from_secs(5);
        while !reader.consumer.is_full() && Instant::now() < deadline {
            std::thread::yield_now();
        }
        let saturated = reader.consumer.is_full();
        let started = Instant::now();
        reader.discard_remaining();
        reader.discard_remaining();
        let completed = received.recv_timeout(Duration::from_millis(500));
        let elapsed = started.elapsed();
        // Keep the pre-fix failure self-cleaning: restore consumption before
        // asserting. Otherwise the reproduction itself would leak a writer.
        if completed.is_err() {
            let deadline = Instant::now() + Duration::from_secs(5);
            let mut buffer = [0; 8192];
            while !writer.is_finished() && Instant::now() < deadline {
                let _ = reader.read(&mut buffer);
                std::thread::yield_now();
            }
        }
        drop(reader);
        writer.join().unwrap();
        eprintln!(
            "saturated native close drain: {} microseconds",
            elapsed.as_micros()
        );
        assert!(saturated, "fixture reached the real ring capacity");
        assert_eq!(
            completed,
            Ok(true),
            "retired consumer blocked native output drain"
        );
    }

    fn wait_for_pipe_worker(worker: &std::thread::JoinHandle<()>) {
        let deadline = Instant::now() + Duration::from_secs(5);
        while !worker.is_finished() {
            assert!(Instant::now() < deadline, "closed pipe worker exit");
            std::thread::sleep(Duration::from_millis(1));
        }
    }

    #[test]
    fn closed_read_pipe_error_then_drop_joins_only_once() {
        let (pipe_reader, pipe_writer) = miow::pipe::anonymous(0).unwrap();
        let mut reader = EventedAnonRead::new(pipe_reader);
        drop(pipe_writer);
        wait_for_pipe_worker(reader.thread.as_ref().unwrap());
        // Error delivery already joins the native thread. Destruction must not
        // take a second join handle or panic while a PTY worker is unwinding.
        assert_eq!(
            reader.read(&mut [0_u8; 1]).unwrap_err().kind(),
            std::io::ErrorKind::BrokenPipe
        );
        assert!(reader.thread.is_none());
        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| drop(reader)))
                .is_ok()
        );
    }

    #[test]
    fn native_pipe_exit_preserves_final_buffered_output_before_eof() {
        for read_size in [1, 7, 14, 64] {
            assert_final_output_before_eof(read_size);
        }
    }

    fn assert_final_output_before_eof(read_size: usize) {
        let (pipe_reader, mut pipe_writer) = miow::pipe::anonymous(0).unwrap();
        let mut reader = EventedAnonRead::new(pipe_reader);
        let expected = b"final output\r\n";
        pipe_writer.write_all(expected).unwrap();
        drop(pipe_writer);
        // Make final bytes and EOF both ready before the VT consumer reads.
        wait_for_pipe_worker(reader.thread.as_ref().unwrap());
        let mut buffer = vec![0; read_size];
        assert_eq!(reader.read(&mut []).unwrap(), 0);
        let mut received = Vec::new();
        while received.len() < expected.len() {
            let count = reader.read(&mut buffer).unwrap();
            assert!(count > 0, "buffered final output makes progress");
            received.extend_from_slice(&buffer[..count]);
            assert_eq!(reader.read(&mut []).unwrap(), 0);
        }
        assert_eq!(received, expected);
        assert_eq!(
            reader.read(&mut buffer).unwrap_err().kind(),
            std::io::ErrorKind::BrokenPipe
        );
    }

    #[test]
    fn closed_write_pipe_error_then_drop_joins_only_once() {
        let (pipe_reader, pipe_writer) = miow::pipe::anonymous(0).unwrap();
        let mut writer = EventedAnonWrite::new(pipe_writer);
        drop(pipe_reader);
        writer.write_all(b"closed").unwrap();
        wait_for_pipe_worker(writer.thread.as_ref().unwrap());
        assert_eq!(
            writer.write(b"again").unwrap_err().kind(),
            std::io::ErrorKind::BrokenPipe
        );
        assert!(writer.thread.is_none());
        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| drop(writer)))
                .is_ok()
        );
    }

    #[test]
    fn idle_input_writer_wakes_for_every_small_message() {
        let (mut pipe_reader, pipe_writer) = miow::pipe::anonymous(0).unwrap();
        let mut evented_writer = EventedAnonWrite::new(pipe_writer);
        let (received_tx, received_rx) = mpsc::channel();

        let reader = std::thread::spawn(move || {
            for _ in 0..128 {
                let mut byte = [0_u8; 1];
                pipe_reader.read_exact(&mut byte).unwrap();
                received_tx.send(byte[0]).unwrap();
            }
        });

        for byte in 0_u8..128 {
            // Exercise the idle transition repeatedly instead of relying on a
            // continuously non-empty buffer to keep the worker awake.
            std::thread::sleep(Duration::from_millis(2));
            evented_writer.write_all(&[byte]).unwrap();
            assert_eq!(
                received_rx.recv_timeout(Duration::from_millis(250)),
                Ok(byte),
                "ConPTY input writer did not wake for byte {byte}",
            );
        }

        drop(evented_writer);
        reader.join().unwrap();
    }
}
