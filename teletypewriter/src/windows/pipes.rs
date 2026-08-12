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

                    // Write from the temp buffer into the producer
                    let mut written = 0usize;
                    while written < nbytes {
                        // Hold the predicate mutex while checking and waiting.
                        // Without this handshake a consumer could empty the
                        // buffer and notify between `is_full` and `wait`,
                        // leaving ConPTY output asleep until unrelated I/O.
                        let mut wait_tag = inner.wait_tag.lock();
                        while producer.is_full() && !inner.done.load(Ordering::SeqCst) {
                            inner.sig_buffer_not_full.wait(&mut wait_tag);
                        }
                        if inner.done.load(Ordering::SeqCst) {
                            return;
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

        // Pair the buffer mutation and notification with the producer's
        // predicate check so a wakeup cannot be lost.
        let _wait_tag = self.inner.wait_tag.lock();
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

        let thread = self.thread.take().unwrap();

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

        self.thread
            .take()
            .unwrap()
            .join()
            .expect("Could not close EventedAnonWrite worker");
    }
}

#[cfg(test)]
mod tests {
    use super::EventedAnonWrite;
    use std::io::{Read, Write};
    use std::sync::mpsc;
    use std::time::Duration;

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
