#![allow(unexpected_cfgs)]
#![cfg(loom)]

use loom::sync::atomic::{fence, AtomicBool, AtomicUsize, Ordering};
use loom::sync::Arc;
use loom::thread;

#[derive(Clone, Copy)]
enum Publication {
    SnapshotFirst,
    PublishFirst,
}

// Independent model of the registration handshake, not the native poller or
// message queue. State 0/1/2 represents AtomicLazyCell NONE/LOCK/SOME with its
// actual acquire reservation, release publication and acquire borrow.
fn registration_model(
    publication: Publication,
    registrar_fence: bool,
    sender_fence: bool,
) {
    loom::model(move || {
        let pending = Arc::new(AtomicUsize::new(0));
        let handle = Arc::new(AtomicUsize::new(0));
        let readable = Arc::new(AtomicBool::new(false));
        let reg_pending = Arc::clone(&pending);
        let reg_handle = Arc::clone(&handle);
        let reg_readable = Arc::clone(&readable);
        let registrar = thread::spawn(move || {
            let snapshot = match publication {
                Publication::SnapshotFirst => Some(reg_pending.load(Ordering::Relaxed)),
                Publication::PublishFirst => None,
            };
            reg_handle
                .compare_exchange(0, 1, Ordering::Acquire, Ordering::Acquire)
                .unwrap();
            reg_handle
                .compare_exchange(1, 2, Ordering::Release, Ordering::Relaxed)
                .unwrap();
            if registrar_fence {
                fence(Ordering::SeqCst);
            }
            let pending = snapshot.unwrap_or_else(|| reg_pending.load(Ordering::Acquire));
            if pending > 0 {
                reg_readable.store(true, Ordering::Release);
            }
        });
        let send_pending = Arc::clone(&pending);
        let send_handle = Arc::clone(&handle);
        let send_readable = Arc::clone(&readable);
        let sender = thread::spawn(move || {
            let previous = send_pending.fetch_add(1, Ordering::Acquire);
            if previous == 0 {
                if sender_fence {
                    fence(Ordering::SeqCst);
                }
                if send_handle.load(Ordering::Acquire) == 2 {
                    send_readable.store(true, Ordering::Release);
                }
            }
        });
        registrar.join().unwrap();
        sender.join().unwrap();
        assert_eq!(pending.load(Ordering::Acquire), 1);
        assert!(
            readable.load(Ordering::Acquire),
            "registration lost pending readiness"
        );
    });
}

#[test]
fn send_racing_first_registration_is_observable() {
    registration_model(Publication::PublishFirst, true, true);
}

#[test]
#[should_panic(expected = "registration lost pending readiness")]
fn model_rejects_snapshot_before_publication_even_with_fences() {
    registration_model(Publication::SnapshotFirst, true, true);
}

#[test]
#[should_panic(expected = "registration lost pending readiness")]
fn model_rejects_missing_registrar_fence() {
    registration_model(Publication::PublishFirst, false, true);
}

#[test]
#[should_panic(expected = "registration lost pending readiness")]
fn model_rejects_missing_sender_fence() {
    registration_model(Publication::PublishFirst, true, false);
}

#[test]
#[should_panic(expected = "registration lost pending readiness")]
fn model_rejects_release_acquire_publication_without_fences() {
    registration_model(Publication::PublishFirst, false, false);
}
