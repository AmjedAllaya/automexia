//! One cancellable WSL clone check per window. No process work on input/render.
use super::launch::SessionLaunchDescriptor;
use automexia_extension_runtime::{BoundedWorker, RefreshSubmission};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc, Arc,
};

type CheckResult = Result<(), &'static str>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Action {
    Split { down: bool },
    LocalTab,
}

pub(super) struct Request {
    pub route: usize,
    pub launch: SessionLaunchDescriptor,
    pub action: Action,
    pub rich_text: usize,
}

struct Work {
    id: u64,
    launch: SessionLaunchDescriptor,
    cancelled: Arc<AtomicBool>,
    wake: Box<dyn FnOnce() + Send>,
}

struct Completion {
    id: u64,
    result: CheckResult,
}

struct Pending {
    id: u64,
    request: Request,
    cancelled: Arc<AtomicBool>,
}

pub(super) struct WslPreflight {
    worker: BoundedWorker<Work>,
    completed: mpsc::Receiver<Completion>,
    pending: Option<Pending>,
    next: Option<u64>,
}

// Even a checker panic publishes failure before waking the window. The runtime
// owns worker retirement; dropping a window never joins a foreign thread.
struct Publisher<'a> {
    sender: &'a mpsc::SyncSender<Completion>,
    completion: Option<Completion>,
    wake: Option<Box<dyn FnOnce() + Send>>,
}

impl Drop for Publisher<'_> {
    fn drop(&mut self) {
        if self
            .completion
            .take()
            .is_some_and(|value| self.sender.try_send(value).is_ok())
        {
            if let Some(wake) = self.wake.take() {
                wake();
            }
        }
    }
}

impl WslPreflight {
    #[cfg(windows)]
    pub fn new() -> Self {
        Self::with_checker(check_distribution)
    }

    fn with_checker(
        check: impl Fn(&SessionLaunchDescriptor, &AtomicBool) -> CheckResult
            + Send
            + Sync
            + 'static,
    ) -> Self {
        let (sender, completed) = mpsc::sync_channel(1);
        Self {
            worker: BoundedWorker::new("wsl-clone-check", 1, move |work: Work| {
                let mut publisher = Publisher {
                    sender: &sender,
                    completion: Some(Completion {
                        id: work.id,
                        result: Err("The WSL check could not complete. Retry the clone."),
                    }),
                    wake: Some(work.wake),
                };
                let result = check(&work.launch, &work.cancelled);
                publisher.completion = Some(Completion {
                    id: work.id,
                    result,
                });
            }),
            completed,
            pending: None,
            next: Some(1),
        }
    }

    pub fn is_pending(&self) -> bool {
        self.pending.is_some()
    }

    pub fn start(
        &mut self,
        request: Request,
        wake: impl FnOnce() + Send + 'static,
    ) -> RefreshSubmission {
        if self.pending.is_some() {
            return RefreshSubmission::Busy;
        }
        let Some(id) = self.next else {
            return RefreshSubmission::Unavailable;
        };
        self.next = id.checked_add(1);
        let cancelled = Arc::new(AtomicBool::new(false));
        let work = Work {
            id,
            launch: request.launch.clone(),
            cancelled: cancelled.clone(),
            wake: Box::new(wake),
        };
        self.pending = Some(Pending {
            id,
            request,
            cancelled,
        });
        let result = self.worker.try_submit(work);
        if result != RefreshSubmission::Queued {
            self.pending = None;
        }
        result
    }

    pub fn take(
        &mut self,
        route: usize,
        launch: Option<&SessionLaunchDescriptor>,
    ) -> Option<(Request, CheckResult)> {
        let pending = self.pending.as_ref()?;
        if pending.request.route != route || launch != Some(&pending.request.launch) {
            pending.cancelled.store(true, Ordering::Release);
        }
        let completion = self.completed.try_recv().ok()?;
        if completion.id != pending.id {
            return None;
        }
        let pending = self.pending.take()?;
        if pending.cancelled.load(Ordering::Acquire) {
            return None;
        }
        Some((pending.request, completion.result))
    }
}

impl Drop for WslPreflight {
    fn drop(&mut self) {
        if let Some(pending) = &self.pending {
            pending.cancelled.store(true, Ordering::Release);
        }
        self.worker.request_shutdown();
    }
}

#[cfg(windows)]
fn check_distribution(
    launch: &SessionLaunchDescriptor,
    cancelled: &AtomicBool,
) -> CheckResult {
    let Some(distro) = launch.wsl_distro() else {
        return Ok(());
    };
    crate::automexia::wsl::validate_distribution(
        launch.program().unwrap_or("wsl.exe"),
        distro,
        cancelled,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn request(route: usize) -> Request {
        Request {
            route,
            launch: SessionLaunchDescriptor::new(
                Some("wsl.exe".into()),
                vec!["-d".into(), "Fixture".into()],
                vec![],
                None,
                None,
            ),
            action: Action::LocalTab,
            rich_text: 42,
        }
    }

    #[test]
    fn wsl_completion_is_published_before_wake_and_is_consumed_once() {
        for action in [
            Action::LocalTab,
            Action::Split { down: false },
            Action::Split { down: true },
        ] {
            let (sender, receiver) = mpsc::channel();
            let mut owner = WslPreflight::with_checker(|_, _| Ok(()));
            let launch = request(7).launch;
            let mut submitted = request(7);
            submitted.action = action;
            assert_eq!(
                owner.start(submitted, move || {
                    sender.send(()).unwrap();
                }),
                RefreshSubmission::Queued
            );
            receiver.recv_timeout(Duration::from_secs(2)).unwrap();
            let (request, result) = owner.take(7, Some(&launch)).unwrap();
            assert_eq!(request.action, action);
            assert_eq!(request.rich_text, 42);
            assert_eq!(result, Ok(()));
            assert!(!owner.is_pending());
            assert!(owner.take(7, Some(&launch)).is_none());
        }
    }

    #[test]
    fn wsl_stale_route_or_launch_cancels_without_reusing_the_busy_slot() {
        for changed_route in [false, true] {
            let (entered_tx, entered_rx) = mpsc::channel();
            let (wake_tx, wake_rx) = mpsc::channel();
            let mut owner = WslPreflight::with_checker(move |_, cancelled| {
                entered_tx.send(()).unwrap();
                while !cancelled.load(Ordering::Acquire) {
                    std::thread::sleep(Duration::from_millis(1));
                }
                Ok(()) // A stale success must never be applied.
            });
            let launch = request(7).launch;
            assert_eq!(
                owner.start(request(7), move || {
                    wake_tx.send(()).unwrap();
                }),
                RefreshSubmission::Queued
            );
            entered_rx.recv_timeout(Duration::from_secs(2)).unwrap();
            assert_eq!(owner.start(request(8), || {}), RefreshSubmission::Busy);
            assert!(owner
                .take(
                    if changed_route { 8 } else { 7 },
                    if changed_route { Some(&launch) } else { None }
                )
                .is_none());
            wake_rx.recv_timeout(Duration::from_secs(2)).unwrap();
            assert!(owner.take(7, Some(&launch)).is_none());
            assert!(!owner.is_pending());
        }
    }

    #[test]
    fn wsl_window_drop_cancels_its_running_check() {
        let (entered_tx, entered_rx) = mpsc::channel();
        let (retired_tx, retired_rx) = mpsc::channel();
        let mut owner = WslPreflight::with_checker(move |_, cancelled| {
            entered_tx.send(()).unwrap();
            while !cancelled.load(Ordering::Acquire) {
                std::thread::sleep(Duration::from_millis(1));
            }
            retired_tx.send(()).unwrap();
            Ok(())
        });
        assert_eq!(owner.start(request(7), || {}), RefreshSubmission::Queued);
        entered_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        drop(owner);
        retired_rx.recv_timeout(Duration::from_secs(2)).unwrap();
    }
}
