use std::hint::black_box;
use std::io::{ErrorKind, Read};
use std::time::{Duration, Instant};

use corcovado::{event::Events, Poll, PollOpt, Ready, Token};
use criterion::{criterion_group, criterion_main, Criterion, Throughput};
use teletypewriter::{is_pty_eof_error, ChildEvent, EventedPty, ProcessReadWrite, Pty};

const READY: &str = "AMX_PTY_READY";
const THROUGHPUT_BYTES: usize = 1024 * 1024;

#[cfg(windows)]
fn spawn_output_pty(bytes: usize) -> Pty {
    teletypewriter::create_pty(
        Some("powershell.exe"),
        vec![
            "-NoLogo".to_owned(),
            "-NoProfile".to_owned(),
            "-NonInteractive".to_owned(),
            "-Command".to_owned(),
            format!("[Console]::Out.Write(('x' * {bytes}) + '{READY}')"),
        ],
        &None,
        None,
        120,
        40,
    )
    .expect("ConPTY benchmark child should start")
}

#[cfg(not(windows))]
fn spawn_output_pty(bytes: usize) -> Pty {
    // BSD head rejects a zero byte count. Keep startup free of an error message
    // and avoid measuring producer processes when there is no payload.
    let producer = if bytes == 0 {
        String::new()
    } else {
        format!("head -c {bytes} /dev/zero | tr '\\0' x; ")
    };
    teletypewriter::create_pty_with_spawn(
        Some("/bin/sh"),
        vec!["-c".to_owned(), format!("{producer}printf '%s' '{READY}'")],
        &None,
        None,
        120,
        40,
        0,
        0,
    )
    .expect("Unix PTY benchmark child should start")
}

fn drain_to_exit(mut pty: Pty, expected_bytes: usize) -> usize {
    let deadline = Instant::now() + Duration::from_secs(30);
    let poll = Poll::new().expect("PTY benchmark readiness poll should start");
    let mut events = Events::with_capacity(4);
    pty.register(
        &poll,
        &mut [Token(0), Token(1), Token(2)].into_iter(),
        Ready::readable(),
        PollOpt::level(),
    )
    .expect("PTY benchmark readiness sources should register");
    let mut reads = 0_usize;
    let mut waits = 0_usize;
    let mut received = 0_usize;
    let mut tail = Vec::with_capacity(READY.len() * 2);
    let mut exited = false;
    let mut marker_seen = false;
    let mut read_closed = false;
    let mut buffer = [0_u8; 16 * 1024];

    while Instant::now() < deadline {
        let mut progressed = false;
        if !read_closed {
            match pty.reader().read(&mut buffer) {
                Ok(0) => {}
                Ok(read) => {
                    progressed = true;
                    reads += 1;
                    received = received.saturating_add(read);
                    tail.extend_from_slice(&buffer[..read]);
                    marker_seen |= tail
                        .windows(READY.len())
                        .any(|bytes| bytes == READY.as_bytes());
                    if tail.len() > READY.len() * 2 {
                        tail.drain(..tail.len() - READY.len() * 2);
                    }
                }
                Err(error) if error.kind() == ErrorKind::WouldBlock => {}
                Err(error) if is_pty_eof_error(&error) => read_closed = true,
                Err(error) => panic!("PTY benchmark read failed: {error}"),
            }
        }
        if matches!(pty.next_child_event(), Some(ChildEvent::Exited(_))) {
            exited = true;
        }
        if exited && marker_seen {
            break;
        }
        // A timer poll caps throughput when the producer writes small chunks.
        // Use the PTY owner's native readiness source, including child exit,
        // and retain the same total deadline and complete-output assertions.
        if !progressed {
            waits += 1;
            match poll.poll(
                &mut events,
                Some(deadline.saturating_duration_since(Instant::now())),
            ) {
                Ok(_) => {}
                Err(error) if error.kind() == ErrorKind::Interrupted => {}
                Err(error) => panic!("PTY benchmark readiness failed: {error}"),
            }
        }
    }
    pty.deregister(&poll)
        .expect("PTY benchmark readiness sources should deregister");

    assert!(
        exited,
        "PTY benchmark child did not exit before its deadline: received={received}, expected={expected_bytes}, marker={marker_seen}, read_closed={read_closed}, reads={reads}, waits={waits}"
    );
    assert!(marker_seen, "PTY benchmark completion marker was lost");
    assert!(
        received >= expected_bytes + READY.len(),
        "PTY benchmark lost output: expected at least {}, received {received}",
        expected_bytes + READY.len()
    );
    received
}

fn pty_io(c: &mut Criterion) {
    let mut startup = c.benchmark_group("pty_process");
    // Process startup is noisy; retain enough samples to compare changes
    // without confusing one scheduler/antivirus outlier with a PTY regression.
    startup.sample_size(30);
    startup.bench_function("startup_to_output_and_clean_exit", |b| {
        b.iter(|| black_box(drain_to_exit(spawn_output_pty(0), 0)))
    });
    startup.finish();

    let mut throughput = c.benchmark_group("pty_output");
    throughput.sample_size(30);
    throughput.throughput(Throughput::Bytes(THROUGHPUT_BYTES as u64));
    throughput.bench_function("sustained_1_mib_and_clean_exit", |b| {
        b.iter(|| {
            black_box(drain_to_exit(
                spawn_output_pty(THROUGHPUT_BYTES),
                THROUGHPUT_BYTES,
            ))
        })
    });
    throughput.finish();
}

criterion_group!(benches, pty_io);
criterion_main!(benches);
