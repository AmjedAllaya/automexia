use super::*;
use std::cell::RefCell;
use std::sync::mpsc;

fn blocked_worker() -> (PtyWorkerHandle<()>, mpsc::SyncSender<()>) {
    let (release, gate) = mpsc::sync_channel(0);
    let (done, completion) = mpsc::sync_channel(1);
    let thread = std::thread::spawn(move || {
        let _ = gate.recv();
        let _ = done.send(());
    });
    (
        PtyWorkerHandle {
            thread: Some(thread),
            completion,
        },
        release,
    )
}

#[test]
fn retirement_returns_while_worker_is_blocked_and_keeps_join_ownership() {
    let registry = PtyWorkerRegistry::default();
    let (worker, release) = blocked_worker();
    let lease = registry.reserve().unwrap().attach(worker);
    assert!(
        registry.poll_cleanup().is_none(),
        "no timer for an active session"
    );
    drop(lease);
    assert!(registry.poll_cleanup().is_some());
    assert_eq!(registry.finish_shutdown(Duration::ZERO), 1);
    // The independent gate has not been released: successful return above
    // proves retirement neither joined the worker nor discarded its owner.
    assert_eq!(registry.state().entries.len(), 1);
    release.send(()).unwrap();
    assert_eq!(registry.finish_shutdown(Duration::from_secs(1)), 0);
    assert!(registry.poll_cleanup().is_none());
}

#[test]
fn capacity_counts_reservations_and_closing_workers_then_recovers() {
    let registry = PtyWorkerRegistry::default();
    let reservations: Vec<_> = (0..MAX_WORKERS)
        .map(|_| registry.reserve().unwrap())
        .collect();
    assert!(registry.reserve().is_err());
    drop(reservations);
    assert!(registry.state().entries.is_empty());
    let (worker, release) = blocked_worker();
    drop(registry.reserve().unwrap().attach(worker));
    let reservations: Vec<_> = (1..MAX_WORKERS)
        .map(|_| registry.reserve().unwrap())
        .collect();
    assert!(
        registry.reserve().is_err(),
        "retiring workers retain capacity"
    );
    release.send(()).unwrap();
    // Unattached reservations remain owned through shutdown as well.
    assert_eq!(
        registry.finish_shutdown(Duration::from_secs(1)),
        MAX_WORKERS - 1
    );
    assert!(registry.reserve().is_ok());
    drop(reservations);
    assert_eq!(registry.finish_shutdown(Duration::ZERO), 0);
}

#[test]
fn repeated_cleanup_is_isolated_and_does_not_accumulate_workers() {
    let registry = PtyWorkerRegistry::default();
    let unrelated = PtyWorkerRegistry::default();
    let reservation = unrelated.reserve().unwrap();
    let mut cleanup_thread = None;
    for _ in 0..64 {
        let (worker, release) = blocked_worker();
        drop(registry.reserve().unwrap().attach(worker));
        assert_eq!(registry.state().entries.len(), 1);
        release.send(()).unwrap();
        {
            let mut state = registry.state();
            let id = state.reaper.as_ref().unwrap().thread.thread().id();
            assert_eq!(*cleanup_thread.get_or_insert(id), id);
            state.finish(Duration::from_secs(1));
            assert!(state.entries.is_empty());
        }
        assert!(registry.poll_cleanup().is_none());
        assert_eq!(unrelated.state().entries.len(), 1);
    }
    drop(reservation);
    assert_eq!(registry.finish_shutdown(Duration::from_secs(1)), 0);
}

#[test]
fn panicked_worker_is_joined_and_capacity_is_released() {
    let (done, completion) = mpsc::sync_channel(1);
    let thread = std::thread::spawn(move || {
        drop(done);
        panic!("intentional worker panic fixture");
    });
    let registry = PtyWorkerRegistry::default();
    drop(registry.reserve().unwrap().attach(PtyWorkerHandle {
        thread: Some(thread),
        completion,
    }));
    assert_eq!(registry.finish_shutdown(Duration::from_secs(1)), 0);
    assert!(registry.poll_cleanup().is_none());
}

#[test]
fn disconnected_cleanup_queue_retains_the_exact_worker_until_recovery() {
    let registry = PtyWorkerRegistry::default();
    let (worker, release) = blocked_worker();
    let lease = registry.reserve().unwrap().attach(worker);
    let reaper = registry.state().reaper.take().unwrap();
    drop(reaper.jobs);
    reaper.thread.join().unwrap();
    let mut replacement = Reaper::start().unwrap();
    let (disconnected, receiver) = mpsc::sync_channel(1);
    drop(receiver);
    let working_queue = std::mem::replace(&mut replacement.jobs, disconnected);
    registry.state().reaper = Some(replacement);
    drop(lease);
    assert!(registry.state().entries[0].worker.is_some());
    assert!(registry.state().cleanup_fault_reported);
    assert_eq!(registry.finish_shutdown(Duration::ZERO), 1);
    registry.state().reaper.as_mut().unwrap().jobs = working_queue;
    release.send(()).unwrap();
    assert_eq!(registry.finish_shutdown(Duration::from_secs(1)), 0);
}

struct DestructorGate {
    entered: mpsc::SyncSender<()>,
    release: mpsc::Receiver<()>,
}
impl Drop for DestructorGate {
    fn drop(&mut self) {
        let _ = self.entered.send(());
        let _ = self.release.recv();
    }
}
thread_local! { static DESTRUCTOR_GATE: RefCell<Option<DestructorGate>> = const { RefCell::new(None) }; }

#[test]
fn retirement_and_shutdown_deadline_do_not_wait_for_native_tls_destructors() {
    let (entered, ready) = mpsc::sync_channel(1);
    let (release, gate) = mpsc::channel();
    let (done, completion) = mpsc::sync_channel(1);
    let thread = std::thread::spawn(move || {
        DESTRUCTOR_GATE.with(|cell| {
            *cell.borrow_mut() = Some(DestructorGate {
                entered,
                release: gate,
            })
        });
        done.send(()).unwrap();
    });
    ready.recv_timeout(Duration::from_secs(1)).unwrap();
    let worker = PtyWorkerHandle {
        thread: Some(thread),
        completion,
    };
    let registry = PtyWorkerRegistry::default();
    drop(registry.reserve().unwrap().attach(worker));
    // The actual native thread destructor is still held at the independent
    // gate. Even is_finished() can be true here on Windows; neither UI close
    // nor the final acknowledgement deadline is allowed to call join itself.
    assert_eq!(registry.finish_shutdown(Duration::ZERO), 1);
    assert!(registry.poll_cleanup().is_some());
    release.send(()).unwrap();
    assert_eq!(registry.finish_shutdown(Duration::from_secs(1)), 0);
}
