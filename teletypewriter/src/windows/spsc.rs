// Single-producer single-consumer buffer for Rust

use parking_lot::Mutex;
use std::io::{self, Read, Write};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

// The pipe predicate mutex, when present, is acquired before this storage
// mutex. No operation under this mutex performs I/O, invokes a callback, or
// acquires a pipe lock. Atomic length queries remain advisory snapshots.
struct SpscBuffer {
    buf: Mutex<Box<[u8]>>,
    len: AtomicUsize,
}

impl SpscBuffer {
    fn new(size: usize) -> Self {
        Self {
            buf: Mutex::new(vec![0; size].into_boxed_slice()),
            len: AtomicUsize::new(0),
        }
    }

    fn len(&self) -> usize {
        self.len.load(Ordering::SeqCst)
    }

    fn capacity(&self) -> usize {
        self.buf.lock().len()
    }

    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn is_full(&self) -> bool {
        self.len() == self.capacity()
    }
}

/// Consumer of the ringbuffer.
pub struct SpscBufferReader {
    start: usize,
    buffer: Arc<SpscBuffer>,
}

impl SpscBufferReader {
    /// Get length of contents currently in the buffer
    #[allow(unused)]
    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    /// Get total capacity of the buffer
    #[allow(unused)]
    pub fn capacity(&self) -> usize {
        self.buffer.capacity()
    }

    /// Check whether the buffer is currently empty
    #[allow(unused)]
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    /// Check whether the buffer is currently full
    #[allow(unused)]
    pub fn is_full(&self) -> bool {
        self.buffer.is_full()
    }

    /// Read data from the buffer. Returns number of bytes read.
    pub fn read_to_slice(&mut self, buf: &mut [u8]) -> usize {
        use std::cmp::min;

        let ringbuf = self.buffer.buf.lock();

        let ringbuf_capacity = ringbuf.len();
        let ringbuf_len = self.buffer.len.load(Ordering::SeqCst);

        // Max number of bytes we might read
        let max_read_size = min(buf.len(), ringbuf_len);
        let contents_until_end = ringbuf_capacity - self.start;
        let read_size = min(max_read_size, contents_until_end);
        if read_size == 0 {
            return 0;
        }

        buf[..read_size].copy_from_slice(&ringbuf[self.start..self.start + read_size]);
        self.start = (self.start + read_size) % ringbuf_capacity;
        self.buffer.len.fetch_sub(read_size, Ordering::SeqCst);

        read_size
    }
}

impl Read for SpscBufferReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        Ok(self.read_to_slice(buf))
    }
}

/// Producer for the ringbuffer
pub struct SpscBufferWriter {
    end: usize,
    buffer: Arc<SpscBuffer>,
}

impl SpscBufferWriter {
    /// Get length of contents currently in the buffer
    #[allow(unused)]
    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    /// Get total capacity of the buffer
    #[allow(unused)]
    pub fn capacity(&self) -> usize {
        self.buffer.capacity()
    }

    /// Check whether the buffer is currently empty
    #[allow(unused)]
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    /// Check whether the buffer is currently full
    pub fn is_full(&self) -> bool {
        self.buffer.is_full()
    }

    /// Write data to the buffer. Returns number of bytes written.
    pub fn write_from_slice(&mut self, buf: &[u8]) -> usize {
        use std::cmp::min;

        let mut ringbuf = self.buffer.buf.lock();

        let ringbuf_capacity = ringbuf.len();
        let ringbuf_len = self.buffer.len.load(Ordering::SeqCst);

        // Max number of bytes we can accept without growing the ring.
        let max_write_size = min(buf.len(), ringbuf_capacity - ringbuf_len);
        let space_until_end = ringbuf_capacity - self.end;
        let write_size = min(max_write_size, space_until_end);
        if write_size == 0 {
            return 0;
        }

        ringbuf[self.end..self.end + write_size].copy_from_slice(&buf[..write_size]);
        self.end = (self.end + write_size) % ringbuf_capacity;
        self.buffer.len.fetch_add(write_size, Ordering::SeqCst);

        write_size
    }
}

impl Write for SpscBufferWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        Ok(self.write_from_slice(buf))
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Create a new SPSC buffer pair.
///
/// The producer and consumer can safely be transferred between threads; the
/// expected use case is that one thread will be writing and one will be reading.
///
/// Storage is shared through an atomically reference-counted mutex; both
/// endpoints acquire a guard before borrowing the bytes. Transfers allocate
/// no memory and hold the guard only for one bounded contiguous copy. A
/// zero-capacity ring accepts and returns zero bytes.
///
/// The underlying buffer's size is synchronised using an atomic. The producer
/// and consumer have methods to query the size and the capacity, which is
/// guaranteed to be consistent between threads but may not be sufficient to
/// prevent races depending on what you are trying to achieve.
///
/// See the mio-anonymous-pipes crate for example usage.
pub fn spsc_buffer(size: usize) -> (SpscBufferWriter, SpscBufferReader) {
    let buffer = Arc::new(SpscBuffer::new(size));

    let producer = SpscBufferWriter {
        end: 0,
        buffer: buffer.clone(),
    };
    let consumer = SpscBufferReader { start: 0, buffer };

    (producer, consumer)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn zero_capacity_is_a_bounded_empty_buffer() {
        let (mut producer, mut consumer) = spsc_buffer(0);
        assert!(producer.is_empty());
        assert!(producer.is_full());
        assert_eq!(producer.write_from_slice(b"x"), 0);
        assert_eq!(producer.write_from_slice(&[]), 0);
        assert_eq!(consumer.read_to_slice(&mut [0; 1]), 0);
        assert_eq!(consumer.read_to_slice(&mut []), 0);
    }

    #[test]
    fn transfers_match_a_fifo_across_capacity_and_wrap_boundaries() {
        use std::collections::VecDeque;

        for capacity in [1, 2, 3, 7, 64, 65536] {
            let (mut producer, mut consumer) = spsc_buffer(capacity);
            let mut expected = VecDeque::new();
            let mut seed = 0xA17C_39D2u32;
            for step in 0..2048 {
                seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                let input = [step as u8; 83];
                let request = seed as usize % (input.len() + 1);
                let written = producer.write_from_slice(&input[..request]);
                assert!(written <= request.min(capacity - expected.len()));
                expected.extend(&input[..written]);
                let mut output = [0xFF; 97];
                let request = (seed >> 8) as usize % (output.len() + 1);
                let read = consumer.read_to_slice(&mut output[..request]);
                assert!(read <= request.min(expected.len()));
                for actual in &output[..read] {
                    assert_eq!(Some(*actual), expected.pop_front());
                }
                assert!(output[read..].iter().all(|byte| *byte == 0xFF));
                assert_eq!(producer.len(), expected.len());
                assert_eq!(consumer.len(), expected.len());
                assert_eq!(producer.capacity(), capacity);
            }
            while !expected.is_empty() {
                let mut output = [0; 97];
                let read = consumer.read_to_slice(&mut output);
                assert!(read > 0);
                for actual in &output[..read] {
                    assert_eq!(Some(*actual), expected.pop_front());
                }
            }
        }
    }

    #[test]
    fn endpoints_transfer_and_drop_on_separate_threads() {
        use std::sync::mpsc;
        use std::time::Duration;

        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<SpscBufferWriter>();
        assert_send_sync::<SpscBufferReader>();

        // Bounded acknowledgments select the schedule and expose a failed
        // peer instead of hanging the test at an unconditional barrier.
        for capacity in [1, 7, 65536] {
            let (mut producer, mut consumer) = spsc_buffer(capacity);
            let lifetime = Arc::downgrade(&producer.buffer);
            let (published, ready) = mpsc::sync_channel(1);
            let (drained, empty) = mpsc::sync_channel(1);
            let writer = std::thread::spawn(move || {
                let input = vec![0xA5; capacity];
                for _ in 0..128 {
                    assert_eq!(producer.write_from_slice(&input), capacity);
                    assert_eq!(producer.write_from_slice(b"x"), 0);
                    published.send(()).unwrap();
                    empty.recv_timeout(Duration::from_secs(5)).unwrap();
                }
                drop(producer);
            });
            let reader = std::thread::spawn(move || {
                let mut output = vec![0; capacity];
                for _ in 0..128 {
                    ready.recv_timeout(Duration::from_secs(5)).unwrap();
                    assert_eq!(consumer.read_to_slice(&mut output), capacity);
                    assert!(output.iter().all(|byte| *byte == 0xA5));
                    assert_eq!(consumer.read_to_slice(&mut output), 0);
                    drained.send(()).unwrap();
                }
                drop(consumer);
            });
            let writer_result = writer.join();
            let reader_result = reader.join();
            writer_result.unwrap();
            reader_result.unwrap();
            assert!(lifetime.upgrade().is_none());
        }
    }

    #[test]
    fn test_spsc_buffer() {
        let buf = [1u8; 100];

        let (mut producer, mut consumer) = spsc_buffer(60);

        assert!(producer.is_empty());
        assert!(consumer.is_empty());

        assert_eq!(producer.len(), 0);
        assert_eq!(consumer.len(), 0);

        assert_eq!(producer.capacity(), 60);
        assert_eq!(consumer.capacity(), 60);

        let mut out_buf = [0u8; 100];

        assert_eq!(producer.write_from_slice(&buf), 60);
        assert_eq!(producer.len(), 60);
        assert_eq!(consumer.len(), 60);

        assert_eq!(consumer.read_to_slice(&mut out_buf), 60);
        assert_eq!(producer.len(), 0);
        assert_eq!(consumer.len(), 0);

        assert_eq!(producer.write_from_slice(&buf[60..]), 40);
        assert_eq!(producer.len(), 40);
        assert_eq!(consumer.len(), 40);

        assert_eq!(consumer.read_to_slice(&mut out_buf[60..]), 40);
        assert_eq!(producer.len(), 0);
        assert_eq!(consumer.len(), 0);

        assert_eq!(&buf[..], &out_buf[..]);
    }
}
