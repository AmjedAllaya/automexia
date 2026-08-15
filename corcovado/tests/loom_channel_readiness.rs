#![allow(unexpected_cfgs)]
#![cfg(loom)]

use loom::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use loom::sync::Arc;
use loom::thread;

fn sender_publish(pending: &AtomicUsize, readable: &AtomicBool) {
    let previous = pending.fetch_add(1, Ordering::Acquire);
    if previous == 0 {
        readable.store(true, Ordering::Release);
    }
}

fn receiver_consume(pending: &AtomicUsize, readable: &AtomicBool) {
    let first = pending.load(Ordering::Acquire);
    if first == 1 {
        readable.store(false, Ordering::Release);
    }
    let second = pending.fetch_sub(1, Ordering::AcqRel);
    if first == 1 && second > 1 {
        readable.store(true, Ordering::Release);
    }
}

#[test]
fn send_racing_last_receive_never_loses_readiness() {
    loom::model(|| {
        let pending = Arc::new(AtomicUsize::new(1));
        let readable = Arc::new(AtomicBool::new(true));

        let receiver_pending = Arc::clone(&pending);
        let receiver_readable = Arc::clone(&readable);
        let receiver = thread::spawn(move || {
            receiver_consume(&receiver_pending, &receiver_readable);
        });

        let sender_pending = Arc::clone(&pending);
        let sender_readable = Arc::clone(&readable);
        let sender = thread::spawn(move || {
            sender_publish(&sender_pending, &sender_readable);
        });

        receiver.join().unwrap();
        sender.join().unwrap();
        assert_eq!(pending.load(Ordering::Acquire), 1);
        assert!(readable.load(Ordering::Acquire));
    });
}

#[test]
fn idle_send_publishes_pending_before_waking_receiver() {
    loom::model(|| {
        let pending = Arc::new(AtomicUsize::new(0));
        let readable = Arc::new(AtomicBool::new(false));
        let sender_pending = Arc::clone(&pending);
        let sender_readable = Arc::clone(&readable);

        let sender = thread::spawn(move || {
            sender_publish(&sender_pending, &sender_readable);
        });
        sender.join().unwrap();

        if readable.load(Ordering::Acquire) {
            assert_eq!(pending.load(Ordering::Acquire), 1);
        }
    });
}
