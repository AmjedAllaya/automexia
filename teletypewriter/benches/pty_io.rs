use std::hint::black_box;
use std::io::{ErrorKind, Read};
use std::thread;
use std::time::{Duration, Instant};

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
    teletypewriter::create_pty_with_spawn(
        Some("/bin/sh"),
        vec![
            "-c".to_owned(),
            format!("head -c {bytes} /dev/zero | tr '\\0' x; printf '%s' '{READY}'"),
        ],
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
    let mut received = 0_usize;
    let mut tail = Vec::with_capacity(READY.len() * 2);
    let mut exited = false;
    let mut marker_seen = false;
    let mut read_closed = false;
    let mut buffer = [0_u8; 16 * 1024];

    while Instant::now() < deadline {
        if !read_closed {
            match pty.reader().read(&mut buffer) {
                Ok(0) => {}
                Ok(read) => {
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
        thread::sleep(Duration::from_millis(1));
    }

    assert!(
        exited,
        "PTY benchmark child did not exit before its deadline"
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
