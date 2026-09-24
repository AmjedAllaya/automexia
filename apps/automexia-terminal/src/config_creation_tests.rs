use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc, Mutex};
use std::time::Instant;

fn wake_channel() -> (CompletionSignal, mpsc::Receiver<()>) {
    let (sender, receiver) = mpsc::channel();
    (CompletionSignal::Channel(sender), receiver)
}

fn silent_signal() -> CompletionSignal {
    let (sender, _) = mpsc::channel();
    CompletionSignal::Channel(sender)
}

#[test]
fn welcome_creation_admission_is_nonblocking_and_one_operation_across_windows() {
    let (entered, started) = mpsc::channel();
    let (release, blocked) = mpsc::channel();
    let blocked = Mutex::new(blocked);
    let calls = Arc::new(AtomicUsize::new(0));
    let observed = Arc::clone(&calls);
    let mut owner = ConfigCreation::with_create(move || {
        observed.fetch_add(1, Ordering::SeqCst);
        entered.send(()).unwrap();
        blocked
            .lock()
            .unwrap()
            .recv_timeout(Duration::from_secs(5))
            .unwrap();
        Ok(CreateConfigOutcome::Created)
    });
    let identity = Arc::new(());
    let (wake, woken) = wake_channel();
    let begin = Instant::now();
    let submission = owner.submit(WindowId::from(1), &identity, wake);
    let admission_time = begin.elapsed();
    started.recv_timeout(Duration::from_secs(5)).unwrap();
    let duplicate = owner.submit(WindowId::from(1), &identity, silent_signal());
    let other = owner.submit(WindowId::from(2), &Arc::new(()), silent_signal());
    release.send(()).unwrap();
    woken.recv_timeout(Duration::from_secs(5)).unwrap();
    assert_eq!(submission, RefreshSubmission::Queued);
    assert!(admission_time < Duration::from_secs(1));
    assert_eq!(duplicate, RefreshSubmission::Busy);
    assert_eq!(other, RefreshSubmission::Busy);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert!(owner.take_completion().unwrap().applies_to(
        WindowId::from(1),
        &identity,
        true
    ));
    assert!(!owner.is_pending());
    assert!(owner.shutdown_timeout(Duration::from_secs(2)));
}

#[test]
fn welcome_creation_publishes_complete_file_and_result_before_wake() {
    let directory = tempfile::tempdir().unwrap();
    let destination = directory.path().join("config.toml");
    let path = destination.clone();
    let mut owner =
        ConfigCreation::with_create(move || create_config_file(Some(path.clone())));
    let identity = Arc::new(());
    let (wake, woken) = wake_channel();
    assert_eq!(
        owner.submit(WindowId::from(1), &identity, wake),
        RefreshSubmission::Queued
    );
    woken.recv_timeout(Duration::from_secs(5)).unwrap();
    let completion = owner
        .take_completion()
        .expect("completion must precede wake");
    assert_eq!(
        completion.outcome.unwrap().unwrap(),
        CreateConfigOutcome::Created
    );
    assert_eq!(std::fs::read(&destination).unwrap(), b"# See the configuration reference: https://github.com/AmjedAllaya/automexia-terminal/tree/main/docs\n\n");
    assert!(owner.shutdown_timeout(Duration::from_secs(2)));
}

#[test]
fn welcome_creation_cancelled_or_closed_requests_skip_filesystem_work() {
    for close in [false, true] {
        let identity = Arc::new(());
        let cancellation = CancellationToken::default();
        let (wake, woken) = wake_channel();
        let request = Request {
            operation: 1,
            window: WindowId::from(1),
            identity: Arc::downgrade(&identity),
            cancellation: cancellation.clone(),
            wake,
        };
        if close {
            drop(identity);
        } else {
            cancellation.cancel();
        }
        let (sender, receiver) = mpsc::sync_channel(1);
        execute_request(
            request,
            &|| panic!("obsolete request entered filesystem work"),
            &sender,
        );
        woken.recv_timeout(Duration::from_secs(1)).unwrap();
        assert!(receiver.try_recv().unwrap().outcome.is_none());
    }
}

#[test]
fn welcome_creation_close_retains_busy_slot_until_entered_io_finishes() {
    let (entered, started) = mpsc::channel();
    let (release, blocked) = mpsc::channel();
    let blocked = Mutex::new(blocked);
    let mut owner = ConfigCreation::with_create(move || {
        entered.send(()).unwrap();
        blocked
            .lock()
            .unwrap()
            .recv_timeout(Duration::from_secs(5))
            .unwrap();
        Ok(CreateConfigOutcome::Created)
    });
    let identity = Arc::new(());
    let (wake, woken) = wake_channel();
    assert_eq!(
        owner.submit(WindowId::from(1), &identity, wake),
        RefreshSubmission::Queued
    );
    started.recv_timeout(Duration::from_secs(5)).unwrap();
    owner.cancel_window(WindowId::from(1));
    let busy = owner.submit(WindowId::from(2), &Arc::new(()), silent_signal());
    release.send(()).unwrap();
    woken.recv_timeout(Duration::from_secs(5)).unwrap();
    assert_eq!(busy, RefreshSubmission::Busy);
    assert!(owner.take_completion().unwrap().outcome.is_none());
    assert!(!owner.is_pending());
    assert!(owner.shutdown_timeout(Duration::from_secs(2)));
}

#[test]
fn welcome_creation_completion_rejects_wrong_closed_reused_and_nonwelcome_routes() {
    let identity = Arc::new(());
    let replacement = Arc::new(());
    let completion = Completion {
        operation: 1,
        window: WindowId::from(1),
        identity: Arc::downgrade(&identity),
        outcome: Some(Ok(CreateConfigOutcome::Created)),
    };
    assert!(completion.applies_to(WindowId::from(1), &identity, true));
    assert!(!completion.applies_to(WindowId::from(2), &identity, true));
    assert!(!completion.applies_to(WindowId::from(1), &replacement, true));
    assert!(!completion.applies_to(WindowId::from(1), &identity, false));
    drop(identity);
    assert!(!completion.applies_to(WindowId::from(1), &replacement, true));
}

#[test]
fn welcome_creation_error_retires_pending_and_allows_a_new_operation() {
    let directory = tempfile::tempdir().unwrap();
    let absent = directory.path().join("absent").join("config.toml");
    let mut owner =
        ConfigCreation::with_create(move || create_config_file(Some(absent.clone())));
    let identity = Arc::new(());
    for _ in 0..2 {
        let (wake, woken) = wake_channel();
        assert_eq!(
            owner.submit(WindowId::from(1), &identity, wake),
            RefreshSubmission::Queued
        );
        woken.recv_timeout(Duration::from_secs(5)).unwrap();
        let error = owner
            .take_completion()
            .unwrap()
            .outcome
            .unwrap()
            .unwrap_err();
        assert!(!error.to_string().contains("absent"));
        assert!(!owner.is_pending());
    }
    assert!(owner.shutdown_timeout(Duration::from_secs(2)));
}

#[test]
fn welcome_creation_stale_operation_cannot_retire_current_registration() {
    let mut owner = ConfigCreation::with_create(|| Ok(CreateConfigOutcome::Created));
    let identity = Arc::new(());
    owner.pending = Some(Pending {
        operation: 2,
        window: WindowId::from(1),
        cancellation: CancellationToken::default(),
    });
    let stale = Completion {
        operation: 1,
        window: WindowId::from(1),
        identity: Arc::downgrade(&identity),
        outcome: Some(Ok(CreateConfigOutcome::Created)),
    };
    assert!(owner.accept_completion(stale).is_none());
    assert!(owner.is_pending());
}

#[test]
fn welcome_creation_shutdown_is_bounded_and_permanently_closes_admission() {
    let (entered, started) = mpsc::channel();
    let (release, blocked) = mpsc::channel();
    let blocked = Mutex::new(blocked);
    let mut owner = ConfigCreation::with_create(move || {
        entered.send(()).unwrap();
        blocked
            .lock()
            .unwrap()
            .recv_timeout(Duration::from_secs(5))
            .unwrap();
        Ok(CreateConfigOutcome::Created)
    });
    let identity = Arc::new(());
    let (wake, woken) = wake_channel();
    assert_eq!(
        owner.submit(WindowId::from(1), &identity, wake),
        RefreshSubmission::Queued
    );
    started.recv_timeout(Duration::from_secs(5)).unwrap();
    let begin = Instant::now();
    let finished = owner.shutdown_timeout(Duration::ZERO);
    let elapsed = begin.elapsed();
    let rejected = owner.submit(WindowId::from(1), &identity, silent_signal());
    release.send(()).unwrap();
    woken.recv_timeout(Duration::from_secs(5)).unwrap();
    assert!(!finished);
    assert!(elapsed < Duration::from_secs(1));
    assert_eq!(rejected, RefreshSubmission::Unavailable);
    assert!(owner.take_completion().unwrap().outcome.is_none());
    assert!(owner.shutdown_timeout(Duration::from_secs(2)));
}

#[test]
fn welcome_creation_operation_exhaustion_fails_closed() {
    let mut owner = ConfigCreation::with_create(|| panic!("exhausted operation ran"));
    owner.next_operation = None;
    assert_eq!(
        owner.submit(WindowId::from(1), &Arc::new(()), silent_signal()),
        RefreshSubmission::Unavailable
    );
    assert!(!owner.is_pending());
}

#[test]
fn welcome_creation_cancellation_after_publication_still_suppresses_route_result() {
    let mut owner = ConfigCreation::with_create(|| Ok(CreateConfigOutcome::Created));
    let identity = Arc::new(());
    let (wake, woken) = wake_channel();
    assert_eq!(
        owner.submit(WindowId::from(1), &identity, wake),
        RefreshSubmission::Queued
    );
    woken.recv_timeout(Duration::from_secs(5)).unwrap();
    owner.cancel_window(WindowId::from(1));
    assert!(owner.take_completion().unwrap().outcome.is_none());
    assert!(!owner.is_pending());
    assert!(owner.shutdown_timeout(Duration::from_secs(2)));
}

#[test]
fn welcome_creation_unrelated_window_close_does_not_cancel_target() {
    let mut owner =
        ConfigCreation::with_create(|| Ok(CreateConfigOutcome::AlreadyExists));
    let identity = Arc::new(());
    let (wake, woken) = wake_channel();
    assert_eq!(
        owner.submit(WindowId::from(1), &identity, wake),
        RefreshSubmission::Queued
    );
    owner.cancel_window(WindowId::from(2));
    woken.recv_timeout(Duration::from_secs(5)).unwrap();
    assert_eq!(
        owner.take_completion().unwrap().outcome.unwrap().unwrap(),
        CreateConfigOutcome::AlreadyExists
    );
    assert!(owner.shutdown_timeout(Duration::from_secs(2)));
}

#[test]
fn welcome_creation_handler_failure_reports_and_closes_admission_without_losing_payload()
{
    struct TrackedPayload(Arc<AtomicUsize>);
    impl Drop for TrackedPayload {
        fn drop(&mut self) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }
    let dropped = Arc::new(AtomicUsize::new(0));
    let payload_dropped = Arc::clone(&dropped);
    let mut owner = ConfigCreation::with_create(move || {
        std::panic::panic_any(TrackedPayload(Arc::clone(&payload_dropped)))
    });
    let identity = Arc::new(());
    let (wake, woken) = wake_channel();
    assert_eq!(
        owner.submit(WindowId::from(1), &identity, wake),
        RefreshSubmission::Queued
    );
    let wake_result = woken.recv_timeout(Duration::from_secs(5));
    let completion = owner.take_completion();
    let rejected = owner.submit(WindowId::from(1), &identity, silent_signal());
    let joined = owner.shutdown_timeout(Duration::from_secs(2));
    assert!(wake_result.is_ok(), "handler failure must publish and wake");
    assert!(completion
        .expect("handler failure must retire pending")
        .outcome
        .unwrap()
        .is_err());
    assert_eq!(rejected, RefreshSubmission::Unavailable);
    assert!(joined);
    assert_eq!(dropped.load(Ordering::SeqCst), 1);
}

#[test]
fn welcome_creation_closed_wake_receiver_does_not_lose_completion_or_panic() {
    let (entered, started) = mpsc::channel();
    let mut owner = ConfigCreation::with_create(move || {
        entered.send(()).unwrap();
        Ok(CreateConfigOutcome::Created)
    });
    let identity = Arc::new(());
    assert_eq!(
        owner.submit(WindowId::from(1), &identity, silent_signal()),
        RefreshSubmission::Queued
    );
    started.recv_timeout(Duration::from_secs(5)).unwrap();
    assert!(owner.worker.shutdown_timeout(Duration::from_secs(2)));
    assert_eq!(
        owner.take_completion().unwrap().outcome.unwrap().unwrap(),
        CreateConfigOutcome::Created
    );
}
