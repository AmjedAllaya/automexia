use super::*;
use std::time::Duration;

#[test]
fn registration_callback_can_reenter_the_worker() {
    let worker = Arc::new(BoundedWorker::new("reentrant-registration", 1, |()| {}));
    let submitted = Arc::clone(&worker);
    let (finished, completed) = mpsc::channel();
    let submitter = thread::spawn(move || {
        let result = submitted.try_submit_then((), || {
            assert!(submitted.ensure_started());
        });
        finished.send(result).unwrap();
    });
    assert_eq!(
        completed.recv_timeout(Duration::from_secs(2)).unwrap(),
        RefreshSubmission::Queued
    );
    submitter.join().unwrap();
    worker.shutdown();
}

#[test]
fn full_queue_shutdown_is_bounded_and_restart_waits_for_the_old_handler() {
    let (entered, started) = mpsc::channel();
    let (release, gate) = mpsc::channel();
    let gate = Mutex::new(gate);
    let worker = BoundedWorker::new("blocked-worker", 1, move |value| {
        entered.send(value).unwrap();
        gate.lock().unwrap().recv().unwrap();
    });
    assert_eq!(worker.try_submit(1), RefreshSubmission::Queued);
    assert_eq!(started.recv_timeout(Duration::from_secs(2)).unwrap(), 1);
    assert_eq!(worker.try_submit(2), RefreshSubmission::Queued);
    let before = std::time::Instant::now();
    worker.request_shutdown();
    assert!(!worker.shutdown_timeout(Duration::from_millis(20)));
    assert!(before.elapsed() < Duration::from_millis(500));
    assert!(!worker.ensure_started());
    assert_eq!(worker.try_submit(3), RefreshSubmission::Unavailable);
    release.send(()).unwrap();
    assert!(worker.shutdown_timeout(Duration::from_secs(2)));
    assert!(
        started.try_recv().is_err(),
        "retired queue executed stale work"
    );
    assert!(worker.ensure_started());
    assert_eq!(worker.try_submit(4), RefreshSubmission::Queued);
    assert_eq!(started.recv_timeout(Duration::from_secs(2)).unwrap(), 4);
    release.send(()).unwrap();
    assert!(worker.shutdown_timeout(Duration::from_secs(2)));
}

#[test]
fn callback_retirement_refuses_replacement_until_registration_returns() {
    let (finished, completed) = mpsc::channel();
    let worker = BoundedWorker::new("callback-retirement", 1, move |value| {
        finished.send(value).unwrap();
    });
    assert_eq!(
        worker.try_submit_then(1, || {
            assert!(!worker.shutdown_timeout(Duration::from_millis(20)));
            assert!(!worker.ensure_started());
        }),
        RefreshSubmission::Unavailable
    );
    assert!(worker.shutdown_timeout(Duration::from_secs(2)));
    assert_eq!(worker.try_submit(2), RefreshSubmission::Queued);
    assert_eq!(completed.recv_timeout(Duration::from_secs(2)).unwrap(), 2);
    assert!(completed.try_recv().is_err());
    assert!(worker.shutdown_timeout(Duration::from_secs(2)));
}

#[test]
fn panicking_registration_does_not_poison_worker_or_run_unregistered_item() {
    let (finished, completed) = mpsc::channel();
    let worker = BoundedWorker::new("registration-panic", 1, move |value| {
        finished.send(value).unwrap();
    });
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        worker.try_submit_then(1, || panic!("fictional registration failure"))
    }));
    assert!(result.is_err());
    assert!(worker.shutdown_timeout(Duration::from_secs(2)));
    assert!(completed.try_recv().is_err());
    assert_eq!(worker.try_submit(2), RefreshSubmission::Queued);
    assert_eq!(completed.recv_timeout(Duration::from_secs(2)).unwrap(), 2);
    assert!(worker.shutdown_timeout(Duration::from_secs(2)));
}

struct NativeDestructorGate {
    entered: mpsc::Sender<()>,
    release: mpsc::Receiver<()>,
}

impl Drop for NativeDestructorGate {
    fn drop(&mut self) {
        self.entered.send(()).unwrap();
        self.release.recv().unwrap();
    }
}

thread_local! {
    static NATIVE_GATE: std::cell::RefCell<Option<NativeDestructorGate>> = const { std::cell::RefCell::new(None) };
}

#[test]
fn native_tls_must_finish_before_join_acknowledgement_or_restart() {
    let (entered, started) = mpsc::channel();
    let (release, gate) = mpsc::channel();
    let setup = Mutex::new(Some(NativeDestructorGate {
        entered,
        release: gate,
    }));
    let (installed, installation) = mpsc::channel();
    let worker = BoundedWorker::new("native-cleanup", 1, move |()| {
        NATIVE_GATE.with(|tls| *tls.borrow_mut() = setup.lock().unwrap().take());
        installed.send(()).unwrap();
    });
    assert_eq!(worker.try_submit(()), RefreshSubmission::Queued);
    installation.recv_timeout(Duration::from_secs(2)).unwrap();
    worker.request_shutdown();
    started.recv_timeout(Duration::from_secs(2)).unwrap();
    assert!(!worker.shutdown_timeout(Duration::from_millis(20)));
    assert!(!worker.ensure_started());
    release.send(()).unwrap();
    assert!(worker.shutdown_timeout(Duration::from_secs(2)));
    assert!(worker.ensure_started());
    assert!(worker.shutdown_timeout(Duration::from_secs(2)));
}

#[test]
fn handler_panic_is_joined_before_restart() {
    let (entered, started) = mpsc::channel();
    let worker = BoundedWorker::new("handler-panic", 1, move |()| {
        entered.send(()).unwrap();
        panic!("fictional failure");
    });
    assert_eq!(worker.try_submit(()), RefreshSubmission::Queued);
    started.recv_timeout(Duration::from_secs(2)).unwrap();
    assert!(worker.shutdown_timeout(Duration::from_secs(2)));
    assert!(worker.ensure_started());
    assert!(worker.shutdown_timeout(Duration::from_secs(2)));
}

#[test]
fn cleanup_service_is_reused_across_restarts() {
    let worker = BoundedWorker::new("reused-cleanup", 1, |()| {});
    assert!(worker.ensure_started());
    let first = worker
        .lock_slot()
        .cleanup
        .as_ref()
        .unwrap()
        ._handle
        .thread()
        .id();
    for _ in 0..24 {
        assert!(worker.shutdown_timeout(Duration::from_secs(2)));
        assert!(worker.ensure_started());
        let current = worker
            .lock_slot()
            .cleanup
            .as_ref()
            .unwrap()
            ._handle
            .thread()
            .id();
        assert_eq!(current, first);
    }
    assert!(worker.shutdown_timeout(Duration::from_secs(2)));
}

#[test]
fn admission_is_shared_across_owners_and_retained_after_last_owner_drop() {
    let live = Arc::new(AtomicUsize::new(0));
    let budget = OwnerBudget::Isolated(Arc::clone(&live), 1);
    let (entered, started) = mpsc::channel();
    let (release, gate) = mpsc::channel();
    let gate = Mutex::new(gate);
    let mut worker = BoundedWorker::new("dropped-owner", 1, move |()| {
        entered.send(()).unwrap();
        gate.lock().unwrap().recv().unwrap();
    });
    worker.budget = budget.clone();
    let mut replacement = BoundedWorker::new("replacement-owner", 1, |()| {});
    replacement.budget = budget;
    assert_eq!(worker.try_submit(()), RefreshSubmission::Queued);
    started.recv_timeout(Duration::from_secs(2)).unwrap();
    assert!(!replacement.ensure_started());
    let before = std::time::Instant::now();
    drop(worker);
    assert!(before.elapsed() < Duration::from_millis(500));
    assert_eq!(live.load(Ordering::Acquire), 1);
    assert!(!replacement.ensure_started());
    release.send(()).unwrap();
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    while !replacement.ensure_started() {
        assert!(std::time::Instant::now() < deadline);
        thread::sleep(Duration::from_millis(1));
    }
    assert_eq!(live.load(Ordering::Acquire), 1);
    assert!(replacement.shutdown_timeout(Duration::from_secs(2)));
    drop(replacement);
    while live.load(Ordering::Acquire) != 0 {
        assert!(std::time::Instant::now() < deadline);
        thread::sleep(Duration::from_millis(1));
    }
}

struct DropCallback(Option<Box<dyn FnOnce() + Send>>);

impl Drop for DropCallback {
    fn drop(&mut self) {
        if let Some(callback) = self.0.take() {
            callback();
        }
    }
}

#[test]
fn rejected_work_destructor_can_reenter_the_worker() {
    let (entered, started) = mpsc::channel();
    let (release, gate) = mpsc::channel();
    let gate = Mutex::new(gate);
    let worker = Arc::new(BoundedWorker::new(
        "rejected-destructor",
        1,
        move |_: DropCallback| {
            entered.send(()).unwrap();
            gate.lock().unwrap().recv().unwrap();
        },
    ));
    assert_eq!(
        worker.try_submit(DropCallback(None)),
        RefreshSubmission::Queued
    );
    started.recv_timeout(Duration::from_secs(2)).unwrap();
    assert_eq!(
        worker.try_submit(DropCallback(None)),
        RefreshSubmission::Queued
    );
    let submitted = Arc::clone(&worker);
    let (finished, completed) = mpsc::channel();
    let submitter = thread::spawn(move || {
        let reentrant = Arc::clone(&submitted);
        let value = DropCallback(Some(Box::new(move || {
            assert!(reentrant.ensure_started());
        })));
        finished.send(submitted.try_submit(value)).unwrap();
    });
    let result = completed.recv_timeout(Duration::from_secs(2));
    // Always release the handler before asserting, including a regression failure.
    worker.request_shutdown();
    release.send(()).unwrap();
    assert_eq!(result.unwrap(), RefreshSubmission::Busy);
    submitter.join().unwrap();
    assert!(worker.shutdown_timeout(Duration::from_secs(2)));
}

#[test]
fn blocked_registration_retains_generation_even_after_native_join() {
    let (entered, started) = mpsc::channel();
    let (release, gate) = mpsc::channel();
    let worker = Arc::new(BoundedWorker::new("blocked-registration", 1, |()| {}));
    let submitted = Arc::clone(&worker);
    let submitter = thread::spawn(move || {
        submitted.try_submit_then((), || {
            entered.send(()).unwrap();
            gate.recv().unwrap();
        })
    });
    started.recv_timeout(Duration::from_secs(2)).unwrap();
    worker.request_shutdown();
    let completion = Arc::clone(&worker.lock_slot().worker.as_ref().unwrap().completion);
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    while !completion.finished.load(Ordering::Acquire) {
        assert!(std::time::Instant::now() < deadline);
        thread::sleep(Duration::from_millis(1));
    }
    assert!(!worker.shutdown_timeout(Duration::ZERO));
    assert!(!worker.ensure_started());
    release.send(()).unwrap();
    assert_eq!(submitter.join().unwrap(), RefreshSubmission::Unavailable);
    assert!(worker.shutdown_timeout(Duration::from_secs(2)));
    assert!(worker.ensure_started());
    assert!(worker.shutdown_timeout(Duration::from_secs(2)));
}

struct PanickingPayload;

impl Drop for PanickingPayload {
    fn drop(&mut self) {
        panic!("fictional payload destructor failure");
    }
}

#[test]
fn panic_payload_destructor_failure_is_explicit_and_keeps_restart_closed() {
    let (entered, started) = mpsc::channel();
    let worker = BoundedWorker::new("payload-failure", 1, move |()| {
        entered.send(()).unwrap();
        std::panic::panic_any(PanickingPayload);
    });
    assert_eq!(worker.try_submit(()), RefreshSubmission::Queued);
    started.recv_timeout(Duration::from_secs(2)).unwrap();
    assert!(!worker.shutdown_timeout(Duration::from_secs(2)));
    assert_eq!(
        worker.shutdown_status(),
        WorkerShutdownStatus::CleanupFailed
    );
    assert!(!worker.ensure_started());
    assert_eq!(worker.try_submit(()), RefreshSubmission::Unavailable);
}

struct PayloadWithNativeCleanup(Option<NativeDestructorGate>);

impl Drop for PayloadWithNativeCleanup {
    fn drop(&mut self) {
        NATIVE_GATE.with(|tls| *tls.borrow_mut() = self.0.take());
    }
}

#[test]
fn panic_payload_native_tls_is_part_of_worker_join_not_cleanup_service_drop() {
    let (entered, started) = mpsc::channel();
    let (release, gate) = mpsc::channel();
    let setup = Mutex::new(Some(NativeDestructorGate {
        entered,
        release: gate,
    }));
    let (invoked, invocation) = mpsc::channel();
    let worker = BoundedWorker::new("payload-native-cleanup", 1, move |()| {
        invoked.send(()).unwrap();
        std::panic::panic_any(PayloadWithNativeCleanup(setup.lock().unwrap().take()));
    });
    assert_eq!(worker.try_submit(()), RefreshSubmission::Queued);
    invocation.recv_timeout(Duration::from_secs(2)).unwrap();
    worker.request_shutdown();
    started.recv_timeout(Duration::from_secs(2)).unwrap();
    assert!(!worker.shutdown_timeout(Duration::from_millis(20)));
    assert!(!worker.ensure_started());
    release.send(()).unwrap();
    assert!(worker.shutdown_timeout(Duration::from_secs(2)));
}
