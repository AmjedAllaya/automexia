//! Bounded, non-blocking extension-runtime primitives.
//!
//! This crate knows nothing about frontend events, renderers, PTYs, or provider
//! SDKs. The application injects exact-route wake behavior through WakeRoute.

use std::collections::VecDeque;
use std::hash::Hash;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
#[cfg(not(target_arch = "wasm32"))]
use std::sync::mpsc::{self, SyncSender, TrySendError};
#[cfg(not(target_arch = "wasm32"))]
use std::sync::Condvar;
use std::sync::{Arc, Mutex, MutexGuard};
#[cfg(not(target_arch = "wasm32"))]
use std::thread::{self, JoinHandle};
use std::time::Duration;

use automexia_extension_api::{
    BoundedText, ContractError, EnvironmentCapsule, ExtensionId, OperationId, SessionId,
};

#[derive(Debug)]
pub struct Generation {
    value: AtomicU32,
}

impl Generation {
    pub const fn new(initial: u32) -> Self {
        Self {
            value: AtomicU32::new(initial),
        }
    }

    pub fn current(&self) -> u32 {
        self.value.load(Ordering::Acquire)
    }

    pub fn advance(&self) -> u32 {
        self.value.fetch_add(1, Ordering::AcqRel).wrapping_add(1)
    }
}

#[derive(Clone, Debug, Default)]
pub struct CancellationToken {
    cancelled: Arc<AtomicBool>,
}

impl CancellationToken {
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }
}

/// Route-scoped, one-shot completion injected by the frontend.
pub trait WakeRoute: Send + 'static {
    fn wake(self: Box<Self>);
}

impl<F> WakeRoute for F
where
    F: FnOnce() + Send + 'static,
{
    fn wake(self: Box<Self>) {
        (*self)();
    }
}

pub type CompletionWake = Box<dyn WakeRoute>;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct CacheKey {
    pub extension_id: ExtensionId,
    pub session_id: SessionId,
    pub capsule_revision: u64,
    pub provider_identity: Option<BoundedText>,
    pub source_revision: u64,
    pub request_kind: BoundedText,
}

#[derive(Clone, Debug)]
struct CacheEntry<K, V> {
    key: K,
    value: V,
}

/// Deterministic FIFO-bounded cache. Updating a key keeps one entry and moves
/// it to the newest position so eviction remains predictable.
#[derive(Clone, Debug)]
pub struct BoundedCache<K, V> {
    capacity: usize,
    entries: VecDeque<CacheEntry<K, V>>,
}

impl<K, V> BoundedCache<K, V>
where
    K: Eq,
{
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity: capacity.max(1),
            entries: VecDeque::with_capacity(capacity.max(1)),
        }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }

    pub fn get(&self, key: &K) -> Option<&V> {
        self.entries
            .iter()
            .find(|entry| &entry.key == key)
            .map(|entry| &entry.value)
    }

    pub fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        self.entries
            .iter_mut()
            .find(|entry| &entry.key == key)
            .map(|entry| &mut entry.value)
    }

    pub fn insert(&mut self, key: K, value: V) {
        if let Some(index) = self.entries.iter().position(|entry| entry.key == key) {
            self.entries.remove(index);
        }
        if self.entries.len() >= self.capacity {
            self.entries.pop_front();
        }
        self.entries.push_back(CacheEntry { key, value });
    }

    pub fn find_map_rev<R>(
        &self,
        mut predicate: impl FnMut(&K, &V) -> Option<R>,
    ) -> Option<R> {
        self.entries
            .iter()
            .rev()
            .find_map(|entry| predicate(&entry.key, &entry.value))
    }

    pub fn retain(&mut self, mut predicate: impl FnMut(&K, &mut V) -> bool) {
        self.entries
            .retain_mut(|entry| predicate(&entry.key, &mut entry.value));
    }

    pub fn keys(&self) -> impl Iterator<Item = &K> {
        self.entries.iter().map(|entry| &entry.key)
    }
}

#[derive(Debug, Default)]
pub struct CoalescingSlot<T> {
    latest: Mutex<Option<T>>,
}

impl<T> CoalescingSlot<T> {
    pub fn submit(&self, value: T) -> bool {
        self.latest
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .replace(value)
            .is_some()
    }

    pub fn take(&self) -> Option<T> {
        self.latest
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RebindDisposition {
    MetadataOnly,
    RelaunchRequired,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RebindPlan {
    pub previous_session_id: SessionId,
    pub next_session_id: SessionId,
    pub previous_revision: u64,
    pub next_revision: u64,
    pub disposition: RebindDisposition,
}

pub fn plan_rebind(
    current: &EnvironmentCapsule,
    next: &EnvironmentCapsule,
) -> Result<RebindPlan, ContractError> {
    if next.revision <= current.revision {
        return Err(ContractError::InvalidCharacter(
            "capsule revision must advance",
        ));
    }
    let requires_relaunch = current.cwd != next.cwd
        || current.shell != next.shell
        || current.distribution != next.distribution
        || current.user != next.user
        || current.providers != next.providers;
    let disposition = if requires_relaunch {
        if current.session_id == next.session_id {
            return Err(ContractError::InvalidCharacter(
                "unsafe capsule changes require a new session",
            ));
        }
        RebindDisposition::RelaunchRequired
    } else {
        if current.session_id != next.session_id {
            return Err(ContractError::InvalidCharacter(
                "metadata-only rebind must keep its session",
            ));
        }
        RebindDisposition::MetadataOnly
    };
    Ok(RebindPlan {
        previous_session_id: current.session_id,
        next_session_id: next.session_id,
        previous_revision: current.revision,
        next_revision: next.revision,
        disposition,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RefreshSubmission {
    Queued,
    Busy,
    Rejected,
    Unavailable,
}

#[cfg(not(target_arch = "wasm32"))]
enum WorkerMessage<T> {
    Work {
        value: T,
        registration_ready: mpsc::Receiver<()>,
    },
    Shutdown,
}

/// Maximum live cleanup owners, including owners dropped during blocked work.
#[cfg(not(target_arch = "wasm32"))]
const MAX_WORKER_OWNERS: usize = 64;
#[cfg(not(target_arch = "wasm32"))]
static LIVE_WORKER_OWNERS: AtomicUsize = AtomicUsize::new(0);
#[cfg(not(target_arch = "wasm32"))]
const WORKER_POLL_INTERVAL: Duration = Duration::from_millis(10);
const WORKER_SHUTDOWN_BUDGET: Duration = Duration::from_secs(2);

#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Default)]
enum OwnerBudget {
    #[default]
    Global,
    #[cfg(test)]
    Isolated(Arc<AtomicUsize>, usize),
}

#[cfg(not(target_arch = "wasm32"))]
impl OwnerBudget {
    fn counter_and_limit(&self) -> (&AtomicUsize, usize) {
        match self {
            Self::Global => (&LIVE_WORKER_OWNERS, MAX_WORKER_OWNERS),
            #[cfg(test)]
            Self::Isolated(counter, limit) => (counter, *limit),
        }
    }

    fn acquire(&self) -> Option<OwnerPermit> {
        let (counter, limit) = self.counter_and_limit();
        counter
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |live| {
                (live < limit).then_some(live + 1)
            })
            .ok()
            .map(|_| OwnerPermit(self.clone()))
    }
}

#[cfg(not(target_arch = "wasm32"))]
struct OwnerPermit(OwnerBudget);

#[cfg(not(target_arch = "wasm32"))]
impl Drop for OwnerPermit {
    fn drop(&mut self) {
        self.0.counter_and_limit().0.fetch_sub(1, Ordering::AcqRel);
    }
}

/// Observable retirement state. Failed cleanup never authorizes replacement.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkerShutdownStatus {
    Running,
    Retiring,
    Complete,
    CleanupFailed,
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Default)]
struct JoinCompletion {
    finished: AtomicBool,
    failed: AtomicBool,
    registrations: AtomicUsize,
    notification: Mutex<()>,
    changed: Condvar,
}

#[cfg(not(target_arch = "wasm32"))]
impl JoinCompletion {
    fn finish(&self) {
        let _notification = self
            .notification
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        self.finished.store(true, Ordering::Release);
        self.changed.notify_all();
    }

    fn fail(&self) {
        let _notification = self.notification.lock().unwrap_or_else(|p| p.into_inner());
        self.failed.store(true, Ordering::Release);
        self.changed.notify_all();
    }

    fn complete(&self) -> bool {
        self.finished.load(Ordering::Acquire)
            && self.registrations.load(Ordering::Acquire) == 0
            && !self.failed.load(Ordering::Acquire)
    }

    fn wait(&self, timeout: Duration) -> bool {
        let guard = self
            .notification
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let _guard = self
            .changed
            .wait_timeout_while(guard, timeout, |_| {
                !self.complete() && !self.failed.load(Ordering::Acquire)
            })
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        self.complete()
    }
}

#[cfg(not(target_arch = "wasm32"))]
struct RegistrationLease(Arc<JoinCompletion>);

#[cfg(not(target_arch = "wasm32"))]
impl RegistrationLease {
    fn new(completion: &Arc<JoinCompletion>) -> Self {
        completion.registrations.fetch_add(1, Ordering::AcqRel);
        Self(Arc::clone(completion))
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl Drop for RegistrationLease {
    fn drop(&mut self) {
        let _notification = self
            .0
            .notification
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        self.0.registrations.fetch_sub(1, Ordering::AcqRel);
        self.0.changed.notify_all();
    }
}

// Panic payload destructors run inside the worker's actual native join boundary,
// including TLS they create. A destructor failure is explicit and closes restart.
// Recovery is bounded: recursively panicking Rust destructors beyond two attempts
// resume unwinding; this in-process primitive is not an arbitrary-code sandbox.
#[cfg(not(target_arch = "wasm32"))]
fn dispose_panic_payload(
    mut payload: Box<dyn std::any::Any + Send>,
    completion: &JoinCompletion,
) {
    for _ in 0..2 {
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| drop(payload))) {
            Ok(()) => return,
            Err(next) => {
                completion.fail();
                payload = next;
            }
        }
    }
    std::panic::resume_unwind(payload);
}

#[cfg(not(target_arch = "wasm32"))]
struct JoinJob {
    handle: JoinHandle<()>,
    completion: Arc<JoinCompletion>,
}

/// One-slot mailbox: a new generation is admitted only after the old join ack.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Default)]
struct CleanupMailbox {
    job: Mutex<Option<JoinJob>>,
    closing: AtomicBool,
    changed: Condvar,
}

#[cfg(not(target_arch = "wasm32"))]
struct CleanupService {
    mailbox: Arc<CleanupMailbox>,
    // Drop does not join this service on a latency-sensitive caller. The service
    // retains its permit and sole worker handle until actual cleanup completes.
    _handle: JoinHandle<()>,
}

#[cfg(not(target_arch = "wasm32"))]
impl CleanupService {
    fn new(name: &str, permit: OwnerPermit) -> Option<Self> {
        let mailbox = Arc::new(CleanupMailbox::default());
        let worker_mailbox = Arc::clone(&mailbox);
        let handle = thread::Builder::new()
            .name(format!("{name}-cleanup"))
            .spawn(move || {
                let _permit = permit;
                loop {
                    let job = {
                        let guard = worker_mailbox
                            .job
                            .lock()
                            .unwrap_or_else(|poisoned| poisoned.into_inner());
                        let mut guard = worker_mailbox
                            .changed
                            .wait_while(guard, |job| {
                                job.is_none()
                                    && !worker_mailbox.closing.load(Ordering::Acquire)
                            })
                            .unwrap_or_else(|poisoned| poisoned.into_inner());
                        guard.take()
                    };
                    let Some(job) = job else { break };
                    // Actual join includes native TLS destruction. No caller or
                    // slot-lock holder executes foreign thread cleanup.
                    match job.handle.join() {
                        Ok(()) => job.completion.finish(),
                        Err(payload) => {
                            // No opaque destructor executes on the cleanup service.
                            // An unrecoverable outer panic closes this owner; it is
                            // propagated with the ordinary Rust unwind contract.
                            job.completion.fail();
                            std::panic::resume_unwind(payload);
                        }
                    }
                }
            })
            .ok()?;
        Some(Self {
            mailbox,
            _handle: handle,
        })
    }

    fn own(&self, job: JoinJob) {
        let mut pending = self
            .mailbox
            .job
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        // The slot admits at most one generation before its join acknowledgement.
        // The cleanup loop removes this job before publishing that acknowledgement.
        debug_assert!(pending.is_none());
        *pending = Some(job);
        self.mailbox.changed.notify_one();
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl Drop for CleanupService {
    fn drop(&mut self) {
        let _pending = self
            .mailbox
            .job
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        self.mailbox.closing.store(true, Ordering::Release);
        self.mailbox.changed.notify_one();
    }
}

#[cfg(not(target_arch = "wasm32"))]
struct WorkerThread<T> {
    sender: SyncSender<WorkerMessage<T>>,
    stopping: Arc<AtomicBool>,
    completion: Arc<JoinCompletion>,
}

#[cfg(not(target_arch = "wasm32"))]
impl<T> WorkerThread<T> {
    fn request_shutdown(&self) {
        self.stopping.store(true, Ordering::Release);
        // A full queue already guarantees a wake. This never waits for capacity
        // and only drops this control message, never a caller-owned work value.
        let _ = self.sender.try_send(WorkerMessage::Shutdown);
    }
}

#[cfg(not(target_arch = "wasm32"))]
struct WorkerSlot<T> {
    worker: Option<WorkerThread<T>>,
    cleanup: Option<CleanupService>,
}

/// Restartable bounded worker. Submission never waits for extension work.
///
/// At most 64 owners can start across the process. Each owner lazily starts one
/// cleanup service, reused across generations, and at most one worker. A retiring
/// generation keeps its slot until its actual native join is acknowledged.
/// Handlers own their cancellation and generation checks before publication.
pub struct BoundedWorker<T>
where
    T: Send + 'static,
{
    name: String,
    capacity: usize,
    handler: Arc<dyn Fn(T) + Send + Sync + 'static>,
    #[cfg(not(target_arch = "wasm32"))]
    slot: Mutex<WorkerSlot<T>>,
    #[cfg(not(target_arch = "wasm32"))]
    budget: OwnerBudget,
    #[cfg(target_arch = "wasm32")]
    _marker: std::marker::PhantomData<T>,
}

impl<T> BoundedWorker<T>
where
    T: Send + 'static,
{
    pub fn new(
        name: impl Into<String>,
        capacity: usize,
        handler: impl Fn(T) + Send + Sync + 'static,
    ) -> Self {
        Self {
            name: name.into(),
            capacity: capacity.max(1),
            handler: Arc::new(handler),
            #[cfg(not(target_arch = "wasm32"))]
            slot: Mutex::new(WorkerSlot {
                worker: None,
                cleanup: None,
            }),
            #[cfg(not(target_arch = "wasm32"))]
            budget: OwnerBudget::Global,
            #[cfg(target_arch = "wasm32")]
            _marker: std::marker::PhantomData,
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn ensure_started(&self) -> bool {
        self.ensure_thread(&mut self.lock_slot())
    }

    #[cfg(target_arch = "wasm32")]
    pub fn ensure_started(&self) -> bool {
        false
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn try_submit(&self, value: T) -> RefreshSubmission {
        self.try_submit_then(value, || {})
    }

    /// Publish registration after queue admission and before the handler starts.
    ///
    /// The callback executes without runtime locks and may re-enter the worker.
    /// Retirement during registration returns Unavailable; callers must retire
    /// that registration too. In-flight handlers must reject obsolete publication
    /// using their own operation/context generation and cancellation token.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn try_submit_then(
        &self,
        value: T,
        on_queued: impl FnOnce(),
    ) -> RefreshSubmission {
        let (sender, stopping, _registration) = {
            let mut slot = self.lock_slot();
            if !self.ensure_thread(&mut slot) {
                return RefreshSubmission::Unavailable;
            }
            let Some(worker) = slot.worker.as_ref() else {
                return RefreshSubmission::Unavailable;
            };
            (
                worker.sender.clone(),
                Arc::clone(&worker.stopping),
                RegistrationLease::new(&worker.completion),
            )
        };
        let (registration_sender, registration_ready) = mpsc::channel();
        let message = WorkerMessage::Work {
            value,
            registration_ready,
        };
        let result = sender.try_send(message);
        match result {
            Ok(()) => {
                on_queued();
                if stopping.load(Ordering::Acquire)
                    || registration_sender.send(()).is_err()
                {
                    RefreshSubmission::Unavailable
                } else {
                    RefreshSubmission::Queued
                }
            }
            Err(TrySendError::Full(_)) => RefreshSubmission::Busy,
            Err(TrySendError::Disconnected(_)) => {
                stopping.store(true, Ordering::Release);
                RefreshSubmission::Unavailable
            }
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub fn try_submit(&self, _value: T) -> RefreshSubmission {
        RefreshSubmission::Unavailable
    }

    #[cfg(target_arch = "wasm32")]
    pub fn try_submit_then(
        &self,
        _value: T,
        _on_queued: impl FnOnce(),
    ) -> RefreshSubmission {
        RefreshSubmission::Unavailable
    }

    /// Cancel queued work and request retirement without joining or waiting for
    /// queue capacity. A running handler must finish its own bounded operation.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn request_shutdown(&self) {
        if let Some(worker) = self.lock_slot().worker.as_ref() {
            worker.request_shutdown();
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub fn request_shutdown(&self) {}

    /// Request retirement and wait at most `timeout` for this generation's actual
    /// join acknowledgement and admitted registration callbacks. False retains
    /// ownership and refuses replacement. A registration callback cannot wait for
    /// its own completion: request retirement and return instead. No slot lock is
    /// held while waiting. Use request_shutdown on input paths.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn shutdown_timeout(&self, timeout: Duration) -> bool {
        let completion = {
            let slot = self.lock_slot();
            let Some(worker) = slot.worker.as_ref() else {
                return true;
            };
            worker.request_shutdown();
            Arc::clone(&worker.completion)
        };
        completion.wait(timeout)
    }

    #[cfg(target_arch = "wasm32")]
    pub fn shutdown_timeout(&self, _timeout: Duration) -> bool {
        true
    }

    /// Inspect retirement without waiting. CleanupFailed is permanent for this
    /// owner and must not be interpreted as a successful join.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn shutdown_status(&self) -> WorkerShutdownStatus {
        let slot = self.lock_slot();
        let Some(worker) = slot.worker.as_ref() else {
            return WorkerShutdownStatus::Complete;
        };
        if worker.completion.failed.load(Ordering::Acquire) {
            WorkerShutdownStatus::CleanupFailed
        } else if worker.completion.complete() {
            WorkerShutdownStatus::Complete
        } else if worker.stopping.load(Ordering::Acquire)
            || worker.completion.finished.load(Ordering::Acquire)
        {
            WorkerShutdownStatus::Retiring
        } else {
            WorkerShutdownStatus::Running
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub fn shutdown_status(&self) -> WorkerShutdownStatus {
        WorkerShutdownStatus::Complete
    }

    /// Compatibility shutdown with a two-second acknowledgement budget. Use
    /// shutdown_timeout when the caller needs to report incomplete cleanup.
    pub fn shutdown(&self) {
        let _ = self.shutdown_timeout(WORKER_SHUTDOWN_BUDGET);
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn lock_slot(&self) -> MutexGuard<'_, WorkerSlot<T>> {
        self.slot
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn ensure_thread(&self, slot: &mut WorkerSlot<T>) -> bool {
        if slot
            .worker
            .as_ref()
            .is_some_and(|worker| worker.completion.complete())
        {
            slot.worker = None;
        }
        if let Some(worker) = slot.worker.as_ref() {
            return !worker.stopping.load(Ordering::Acquire)
                && !worker.completion.finished.load(Ordering::Acquire)
                && !worker.completion.failed.load(Ordering::Acquire);
        }
        if slot.cleanup.is_none() {
            let Some(permit) = self.budget.acquire() else {
                return false;
            };
            slot.cleanup = CleanupService::new(&self.name, permit);
        }
        let Some(cleanup) = slot.cleanup.as_ref() else {
            return false;
        };
        let Some((worker, job)) = self.spawn_thread() else {
            return false;
        };
        cleanup.own(job);
        slot.worker = Some(worker);
        true
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn spawn_thread(&self) -> Option<(WorkerThread<T>, JoinJob)> {
        let (sender, receiver) = mpsc::sync_channel::<WorkerMessage<T>>(self.capacity);
        let handler = Arc::clone(&self.handler);
        let stopping = Arc::new(AtomicBool::new(false));
        let worker_stopping = Arc::clone(&stopping);
        let completion = Arc::new(JoinCompletion::default());
        let worker_completion = Arc::clone(&completion);
        let handle = thread::Builder::new()
            .name(self.name.clone())
            .spawn(move || {
                let result =
                    std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
                        run_worker(receiver, handler, worker_stopping);
                    }));
                if let Err(payload) = result {
                    dispose_panic_payload(payload, &worker_completion);
                }
            })
            .ok()?;
        let job = JoinJob {
            handle,
            completion: Arc::clone(&completion),
        };
        Some((
            WorkerThread {
                sender,
                stopping,
                completion,
            },
            job,
        ))
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn run_worker<T>(
    receiver: mpsc::Receiver<WorkerMessage<T>>,
    handler: Arc<dyn Fn(T) + Send + Sync + 'static>,
    stopping: Arc<AtomicBool>,
) {
    // All user-owned values are parameters of this call so their ordinary Drop
    // runs inside the caller's unwind guard, before native join acknowledgement.
    while !stopping.load(Ordering::Acquire) {
        let Ok(WorkerMessage::Work {
            value,
            registration_ready,
        }) = receiver.recv()
        else {
            break;
        };
        while !stopping.load(Ordering::Acquire) {
            match registration_ready.recv_timeout(WORKER_POLL_INTERVAL) {
                Ok(()) => {
                    if !stopping.load(Ordering::Acquire) {
                        handler(value);
                    }
                    break;
                }
                Err(mpsc::RecvTimeoutError::Timeout) => continue,
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
    }
}

impl<T> Drop for BoundedWorker<T>
where
    T: Send + 'static,
{
    fn drop(&mut self) {
        // Closing the cleanup mailbox retains the background owner until its
        // pending/running join completes. Blocking native work is not detached
        // from its join owner, nor interpreted as successful cleanup.
        self.request_shutdown();
    }
}

impl<T> fmt::Debug for BoundedWorker<T>
where
    T: Send + 'static,
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("BoundedWorker")
            .field("name", &self.name)
            .field("capacity", &self.capacity)
            .finish_non_exhaustive()
    }
}

use std::fmt;

#[derive(Clone, Debug)]
pub struct OperationContext {
    pub operation_id: OperationId,
    pub cancellation: CancellationToken,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};
    use std::sync::mpsc;
    use std::time::Duration;

    fn key(session: u64, revision: u64) -> CacheKey {
        CacheKey {
            extension_id: ExtensionId::new("automexia.devops").unwrap(),
            session_id: SessionId::new(session),
            capsule_revision: revision,
            provider_identity: None,
            source_revision: revision,
            request_kind: BoundedText::new("context-refresh").unwrap(),
        }
    }

    #[test]
    fn cache_is_bounded_keyed_and_updates_move_to_newest() {
        let mut cache = BoundedCache::new(2);
        cache.insert(key(1, 1), "one");
        cache.insert(key(2, 1), "two");
        cache.insert(key(1, 1), "updated");
        cache.insert(key(3, 1), "three");
        assert_eq!(cache.len(), 2);
        assert_eq!(cache.get(&key(1, 1)), Some(&"updated"));
        assert_eq!(cache.get(&key(2, 1)), None);
        assert_eq!(cache.get(&key(3, 1)), Some(&"three"));
    }

    #[test]
    fn worker_wakes_only_the_submitted_route_and_joins_cleanly() {
        let (sender, receiver) = mpsc::channel();
        let worker = BoundedWorker::new("runtime-test", 1, move |route| {
            sender.send(route).unwrap();
        });
        assert!(worker.ensure_started());
        assert_eq!(worker.try_submit(77_u64), RefreshSubmission::Queued);
        assert_eq!(receiver.recv_timeout(Duration::from_secs(2)).unwrap(), 77);
        worker.shutdown();
    }

    #[test]
    fn worker_restarts_after_every_shutdown_without_stale_work_or_threads() {
        const CYCLES: usize = 24;
        let (sender, receiver) = mpsc::channel();
        let worker = BoundedWorker::new("restart-lifecycle", 1, move |route| {
            sender.send(route).unwrap();
        });

        for route in 0..CYCLES {
            assert!(worker.ensure_started());
            assert_eq!(worker.try_submit(route), RefreshSubmission::Queued);
            assert_eq!(
                receiver.recv_timeout(Duration::from_secs(2)).unwrap(),
                route
            );
            worker.shutdown();
            assert!(receiver.try_recv().is_err(), "stale work crossed a restart");
        }
    }

    #[test]
    fn worker_capacity_is_clamped_and_repeated_saturation_recovers() {
        let (release_sender, release_receiver) = mpsc::channel::<()>();
        let release_receiver = Arc::new(Mutex::new(release_receiver));
        let handler_release = Arc::clone(&release_receiver);
        let (started_sender, started_receiver) = mpsc::channel();
        let worker = BoundedWorker::new("zero-capacity", 0, move |route| {
            started_sender.send(route).unwrap();
            handler_release.lock().unwrap().recv().unwrap();
        });

        assert_eq!(worker.try_submit(1), RefreshSubmission::Queued);
        assert_eq!(
            started_receiver
                .recv_timeout(Duration::from_secs(2))
                .unwrap(),
            1
        );
        assert_eq!(worker.try_submit(2), RefreshSubmission::Queued);
        for route in 3..64 {
            assert_eq!(
                worker.try_submit(route),
                RefreshSubmission::Busy,
                "bounded queue accepted route {route} while saturated"
            );
        }
        release_sender.send(()).unwrap();
        assert_eq!(
            started_receiver
                .recv_timeout(Duration::from_secs(2))
                .unwrap(),
            2
        );
        release_sender.send(()).unwrap();
        worker.shutdown();
    }

    #[test]
    fn accepted_work_observes_registration_before_the_handler_runs() {
        let phase = Arc::new(AtomicUsize::new(0));
        let handler_phase = Arc::clone(&phase);
        let (sender, receiver) = mpsc::channel();
        let worker = BoundedWorker::new("registration-order", 1, move |()| {
            sender
                .send(handler_phase.load(AtomicOrdering::Acquire))
                .unwrap();
        });
        assert_eq!(
            worker.try_submit_then((), || phase.store(1, AtomicOrdering::Release)),
            RefreshSubmission::Queued
        );
        assert_eq!(receiver.recv_timeout(Duration::from_secs(2)).unwrap(), 1);
        worker.shutdown();
    }

    #[test]
    fn saturation_is_non_blocking_and_does_not_register_rejected_work() {
        let (started_sender, started_receiver) = mpsc::channel();
        let worker = BoundedWorker::new("saturation-order", 1, move |value| {
            started_sender.send(value).unwrap();
            thread::sleep(Duration::from_millis(150));
        });
        let registrations = AtomicUsize::new(0);
        assert_eq!(
            worker.try_submit_then(1, || {
                registrations.fetch_add(1, AtomicOrdering::Relaxed);
            }),
            RefreshSubmission::Queued
        );
        assert_eq!(
            started_receiver
                .recv_timeout(Duration::from_secs(2))
                .unwrap(),
            1
        );
        assert_eq!(
            worker.try_submit_then(2, || {
                registrations.fetch_add(1, AtomicOrdering::Relaxed);
            }),
            RefreshSubmission::Queued
        );
        let started = std::time::Instant::now();
        assert_eq!(
            worker.try_submit_then(3, || {
                registrations.fetch_add(1, AtomicOrdering::Relaxed);
            }),
            RefreshSubmission::Busy
        );
        assert!(started.elapsed() < Duration::from_millis(100));
        assert_eq!(registrations.load(AtomicOrdering::Relaxed), 2);
        worker.shutdown();
    }

    #[test]
    fn cancellation_is_monotonic() {
        let token = CancellationToken::default();
        let clone = token.clone();
        assert!(!token.is_cancelled());
        clone.cancel();
        assert!(token.is_cancelled());
    }

    #[test]
    fn generation_publishes_monotonic_revisions() {
        let generation = Generation::new(4);
        assert_eq!(generation.current(), 4);
        assert_eq!(generation.advance(), 5);
        assert_eq!(generation.current(), 5);
    }

    #[test]
    fn loom_models_cancel_visibility_without_a_lost_transition() {
        loom::model(|| {
            let cancelled =
                loom::sync::Arc::new(loom::sync::atomic::AtomicBool::new(false));
            let writer = loom::sync::Arc::clone(&cancelled);
            let thread = loom::thread::spawn(move || {
                writer.store(true, loom::sync::atomic::Ordering::Release);
            });
            thread.join().unwrap();
            assert!(cancelled.load(loom::sync::atomic::Ordering::Acquire));
        });
    }

    #[test]
    fn coalescing_slot_keeps_only_the_latest_submission() {
        let slot = CoalescingSlot::default();
        assert!(!slot.submit(1));
        assert!(slot.submit(2));
        assert_eq!(slot.take(), Some(2));
        assert_eq!(slot.take(), None);
    }

    #[test]
    fn rebind_requires_new_session_for_shell_or_directory_changes() {
        let current = EnvironmentCapsule::new(SessionId::new(1), 1, Vec::new()).unwrap();
        let mut changed =
            EnvironmentCapsule::new(SessionId::new(2), 2, Vec::new()).unwrap();
        changed.cwd = Some(BoundedText::new("/srv/new").unwrap());
        let plan = plan_rebind(&current, &changed).unwrap();
        assert_eq!(plan.disposition, RebindDisposition::RelaunchRequired);

        changed.session_id = SessionId::new(1);
        assert!(plan_rebind(&current, &changed).is_err());
    }

    #[test]
    fn metadata_rebind_is_atomic_within_the_existing_session() {
        let current = EnvironmentCapsule::new(SessionId::new(5), 3, Vec::new()).unwrap();
        let next = EnvironmentCapsule::new(SessionId::new(5), 4, Vec::new()).unwrap();
        assert_eq!(
            plan_rebind(&current, &next).unwrap().disposition,
            RebindDisposition::MetadataOnly
        );
    }

    #[test]
    fn provider_context_rebind_requires_a_fresh_session() {
        let current = EnvironmentCapsule::new(SessionId::new(5), 3, Vec::new()).unwrap();
        let mut next = EnvironmentCapsule::new(
            SessionId::new(6),
            4,
            vec![automexia_extension_api::ProviderIdentity {
                provider: BoundedText::new("aws").unwrap(),
                account: Some(BoundedText::new("development").unwrap()),
                region: Some(BoundedText::new("eu-west-3").unwrap()),
            }],
        )
        .unwrap();
        assert_eq!(
            plan_rebind(&current, &next).unwrap().disposition,
            RebindDisposition::RelaunchRequired
        );

        next.session_id = current.session_id;
        assert!(plan_rebind(&current, &next).is_err());
    }
    #[test]
    fn loom_models_coalescing_to_the_newest_revision() {
        loom::model(|| {
            let latest = loom::sync::Arc::new(loom::sync::atomic::AtomicUsize::new(0));
            let mut threads = Vec::new();
            for revision in [1_usize, 2] {
                let latest = loom::sync::Arc::clone(&latest);
                threads.push(loom::thread::spawn(move || {
                    let mut observed = latest.load(loom::sync::atomic::Ordering::Acquire);
                    while observed < revision {
                        match latest.compare_exchange(
                            observed,
                            revision,
                            loom::sync::atomic::Ordering::AcqRel,
                            loom::sync::atomic::Ordering::Acquire,
                        ) {
                            Ok(_) => break,
                            Err(actual) => observed = actual,
                        }
                    }
                }));
            }
            for thread in threads {
                thread.join().unwrap();
            }
            assert_eq!(latest.load(loom::sync::atomic::Ordering::Acquire), 2);
        });
    }

    #[test]
    fn loom_models_publish_before_exact_route_wake() {
        loom::model(|| {
            let snapshot = loom::sync::Arc::new(loom::sync::Mutex::new(None));
            let woke = loom::sync::Arc::new(loom::sync::atomic::AtomicBool::new(false));
            let writer_snapshot = loom::sync::Arc::clone(&snapshot);
            let writer_woke = loom::sync::Arc::clone(&woke);
            let publisher = loom::thread::spawn(move || {
                *writer_snapshot.lock().unwrap() = Some(42_u32);
                writer_woke.store(true, loom::sync::atomic::Ordering::Release);
            });
            publisher.join().unwrap();
            if woke.load(loom::sync::atomic::Ordering::Acquire) {
                assert_eq!(*snapshot.lock().unwrap(), Some(42));
            }
        });
    }

    #[test]
    fn loom_models_disable_and_cancel_preserving_last_known_good() {
        loom::model(|| {
            let snapshot = loom::sync::Arc::new(loom::sync::Mutex::new(7_u32));
            let enabled = loom::sync::Arc::new(loom::sync::atomic::AtomicBool::new(true));
            let cancelled =
                loom::sync::Arc::new(loom::sync::atomic::AtomicBool::new(false));

            enabled.store(false, loom::sync::atomic::Ordering::Release);
            cancelled.store(true, loom::sync::atomic::Ordering::Release);
            if enabled.load(loom::sync::atomic::Ordering::Acquire)
                && !cancelled.load(loom::sync::atomic::Ordering::Acquire)
            {
                *snapshot.lock().unwrap() = 99;
            }
            assert_eq!(*snapshot.lock().unwrap(), 7);
        });
    }

    #[test]
    fn loom_models_shutdown_as_a_terminal_lifecycle_state() {
        loom::model(|| {
            let shutdown =
                loom::sync::Arc::new(loom::sync::atomic::AtomicBool::new(false));
            let worker = loom::sync::Arc::clone(&shutdown);
            let handle = loom::thread::spawn(move || {
                worker.store(true, loom::sync::atomic::Ordering::Release);
            });
            handle.join().unwrap();
            assert!(shutdown.load(loom::sync::atomic::Ordering::Acquire));
        });
    }

    #[test]
    fn completion_wake_is_one_shot() {
        let count = Arc::new(AtomicUsize::new(0));
        let target = Arc::clone(&count);
        let wake: CompletionWake = Box::new(move || {
            target.fetch_add(1, AtomicOrdering::SeqCst);
        });
        wake.wake();
        assert_eq!(count.load(AtomicOrdering::SeqCst), 1);
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod worker_tests;
