extern crate corcovado;
extern crate criterion;

use corcovado::*;
use criterion::{criterion_group, criterion_main, Criterion};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

fn bench_poll(c: &mut Criterion) {
    const NUM: usize = 10_000;
    const THREADS: usize = 4;

    let poll = Poll::new().unwrap();
    let mut events = Events::with_capacity(1024);

    let mut registrations = vec![];
    let mut set_readiness = vec![];

    for i in 0..NUM {
        let (r, s) =
            Registration::new(&poll, Token(i), Ready::readable(), PollOpt::edge());

        registrations.push(r);
        set_readiness.push(s);
    }

    let set_readiness = Arc::new(set_readiness);
    let mut seen = vec![false; NUM];

    c.bench_function("bench_poll", |b| {
        b.iter(|| {
            seen.fill(false);
            let mut workers = Vec::with_capacity(THREADS);
            for mut i in 0..THREADS {
                let set_readiness = set_readiness.clone();
                workers.push(thread::spawn(move || {
                    while i < NUM {
                        set_readiness[i].set_readiness(Ready::readable()).unwrap();
                        i += THREADS;
                    }
                }));
            }

            let mut n = 0;
            let deadline = Instant::now() + Duration::from_secs(10);
            while n < NUM {
                let remaining = deadline.saturating_duration_since(Instant::now());
                assert!(!remaining.is_zero(), "poll benchmark readiness deadline");
                poll.poll(&mut events, Some(remaining.min(Duration::from_millis(50))))
                    .unwrap();
                for event in &events {
                    let index = event.token().0;
                    assert!(index < NUM, "unknown benchmark registration");
                    assert!(!seen[index], "duplicate benchmark readiness");
                    seen[index] = true;
                    n += 1;
                }
            }
            // Counting events alone can hide a duplicated token and leave a
            // worker alive across iterations. Exact identities plus native
            // joins prove one complete round, including thread-local cleanup.
            for worker in workers {
                worker.join().expect("benchmark readiness worker");
            }
            assert!(seen.iter().all(|observed| *observed));
        })
    });
}

criterion_group!(benches, bench_poll);
criterion_main!(benches);
