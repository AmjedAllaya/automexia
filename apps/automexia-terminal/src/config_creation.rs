//! One application-owned, bounded starter-configuration action.
//!
//! Filesystem policy belongs to the backend. This owner only admits one action,
//! retains its route identity through completion, and keeps I/O off input paths.

use automexia_extension_runtime::{BoundedWorker, CancellationToken, RefreshSubmission};
use rio_backend::config::{create_config_file, CreateConfigError, CreateConfigOutcome};
use rio_backend::event::{EventProxy, RioEvent, RioEventType, WindowId};
use std::fmt;
use std::sync::{mpsc, Arc, Weak};
use std::time::Duration;

/// Only the application event proxy can wake production code. In particular,
/// unwinding must never invoke an arbitrary callback that could panic again.
pub(crate) enum CompletionSignal {
    Event(EventProxy),
    #[cfg(test)]
    Channel(mpsc::Sender<()>),
}

impl CompletionSignal {
    fn wake(self, window: WindowId) {
        match self {
            Self::Event(proxy) => {
                proxy.send_event(RioEventType::Rio(RioEvent::Render), window)
            }
            #[cfg(test)]
            Self::Channel(sender) => {
                let _ = sender.send(());
            }
        }
    }
}

#[derive(Debug)]
pub(crate) enum CreationFailure {
    Publication(CreateConfigError),
    WorkerFailed,
}

impl fmt::Display for CreationFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Publication(error) => fmt::Display::fmt(error, formatter),
            Self::WorkerFailed => formatter.write_str("Configuration creation did not finish. Restart Automexia or use the command line to create it."),
        }
    }
}

struct Request {
    operation: u64,
    window: WindowId,
    identity: Weak<()>,
    cancellation: CancellationToken,
    wake: CompletionSignal,
}

pub(crate) struct Completion {
    operation: u64,
    pub(crate) window: WindowId,
    identity: Weak<()>,
    pub(crate) outcome: Option<Result<CreateConfigOutcome, CreationFailure>>,
}

impl Completion {
    pub(crate) fn applies_to(
        &self,
        window: WindowId,
        identity: &Arc<()>,
        still_welcome: bool,
    ) -> bool {
        still_welcome
            && self.window == window
            && self.identity.ptr_eq(&Arc::downgrade(identity))
            && self.outcome.is_some()
    }
}

struct Pending {
    operation: u64,
    window: WindowId,
    cancellation: CancellationToken,
}

pub(crate) struct ConfigCreation {
    worker: BoundedWorker<Request>,
    completed: mpsc::Receiver<Completion>,
    pending: Option<Pending>,
    next_operation: Option<u64>,
    closing: bool,
}

/// The runtime retains panic-payload disposal and actual join ownership. This
/// guard only guarantees a bounded app result when the creation handler unwinds.
struct CompletionPublisher<'a> {
    completed: &'a mpsc::SyncSender<Completion>,
    completion: Option<Completion>,
    wake: Option<CompletionSignal>,
}

impl Drop for CompletionPublisher<'_> {
    fn drop(&mut self) {
        let Some(completion) = self.completion.take() else {
            return;
        };
        let window = completion.window;
        // No arbitrary user callback or panic payload is owned here. Sending
        // typed events to a closed application/test channel is a harmless error.
        // One retained admission and one result slot prevent a full channel.
        if self.completed.try_send(completion).is_ok() {
            if let Some(wake) = self.wake.take() {
                wake.wake(window);
            }
        }
    }
}

fn execute_request(
    request: Request,
    create: &impl Fn() -> Result<CreateConfigOutcome, CreateConfigError>,
    completed: &mpsc::SyncSender<Completion>,
) {
    let mut publisher = CompletionPublisher {
        completed,
        completion: Some(Completion {
            operation: request.operation,
            window: request.window,
            identity: request.identity.clone(),
            outcome: Some(Err(CreationFailure::WorkerFailed)),
        }),
        wake: Some(request.wake),
    };
    // Weak identity does not keep a closed/replaced Welcome route alive. Once
    // publication starts it may finish; no replacement writer is admitted while
    // that operation retires. Unwind retains the default WorkerFailed result.
    let mut outcome =
        if request.cancellation.is_cancelled() || request.identity.strong_count() == 0 {
            None
        } else {
            Some(create().map_err(CreationFailure::Publication))
        };
    if request.cancellation.is_cancelled() || request.identity.strong_count() == 0 {
        outcome = None;
    }
    if let Some(completion) = &mut publisher.completion {
        completion.outcome = outcome;
    }
}

impl ConfigCreation {
    pub(crate) fn new() -> Self {
        Self::with_create(|| create_config_file(None))
    }

    fn with_create(
        create: impl Fn() -> Result<CreateConfigOutcome, CreateConfigError>
            + Send
            + Sync
            + 'static,
    ) -> Self {
        let (completed, receiver) = mpsc::sync_channel(1);
        Self {
            worker: BoundedWorker::new("starter-config", 1, move |request| {
                execute_request(request, &create, &completed);
            }),
            completed: receiver,
            pending: None,
            next_operation: Some(1),
            closing: false,
        }
    }

    pub(crate) fn is_pending(&self) -> bool {
        self.pending.is_some()
    }

    pub(crate) fn submit(
        &mut self,
        window: WindowId,
        identity: &Arc<()>,
        wake: CompletionSignal,
    ) -> RefreshSubmission {
        if self.closing {
            return RefreshSubmission::Unavailable;
        }
        if self.pending.is_some() {
            return RefreshSubmission::Busy;
        }
        let Some(operation) = self.next_operation else {
            return RefreshSubmission::Unavailable;
        };
        self.next_operation = operation.checked_add(1);
        let cancellation = CancellationToken::default();
        // Registration is event-thread-owned and complete before the worker
        // can run or wake. Admission failure retires this exact registration.
        self.pending = Some(Pending {
            operation,
            window,
            cancellation: cancellation.clone(),
        });
        let result = self.worker.try_submit(Request {
            operation,
            window,
            identity: Arc::downgrade(identity),
            cancellation,
            wake,
        });
        if result != RefreshSubmission::Queued {
            self.pending = None;
        }
        result
    }

    pub(crate) fn take_completion(&mut self) -> Option<Completion> {
        let completion = self.completed.try_recv().ok()?;
        self.accept_completion(completion)
    }

    fn accept_completion(&mut self, mut completion: Completion) -> Option<Completion> {
        let pending = self.pending.as_ref()?;
        if pending.operation != completion.operation
            || pending.window != completion.window
        {
            return None;
        }
        if matches!(
            &completion.outcome,
            Some(Err(CreationFailure::WorkerFailed))
        ) {
            // Publication can precede the runtime's actual native join. Do not
            // let retry race that retiring generation, including slow payload
            // destruction. Recovery is the terminal diagnostic, CLI, or restart.
            self.closing = true;
            self.worker.request_shutdown();
        }
        // Cancellation can occur after the worker publishes but before the
        // event loop drains the result; it still suppresses route publication.
        if pending.cancellation.is_cancelled() {
            completion.outcome = None;
        }
        self.pending = None;
        Some(completion)
    }

    pub(crate) fn cancel_window(&self, window: WindowId) {
        if let Some(pending) = &self.pending {
            if pending.window == window {
                pending.cancellation.cancel();
            }
        }
    }

    pub(crate) fn request_shutdown(&mut self) {
        self.closing = true;
        if let Some(pending) = &self.pending {
            pending.cancellation.cancel();
        }
        self.worker.request_shutdown();
    }

    pub(crate) fn shutdown_timeout(&mut self, budget: Duration) -> bool {
        self.request_shutdown();
        self.worker.shutdown_timeout(budget)
    }
}

impl Drop for ConfigCreation {
    fn drop(&mut self) {
        self.request_shutdown();
    }
}

#[cfg(test)]
#[path = "config_creation_tests.rs"]
mod tests;
