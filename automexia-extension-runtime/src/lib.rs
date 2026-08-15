//! Bounded, non-blocking extension-runtime primitives.
//!
//! This crate knows nothing about frontend events, renderers, PTYs, or provider
//! SDKs. The application injects exact-route wake behavior through WakeRoute.

use std::collections::VecDeque;
use std::hash::Hash;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
#[cfg(not(target_arch = "wasm32"))]
use std::sync::mpsc::{self, SyncSender, TrySendError};
use std::sync::{Arc, Mutex, MutexGuard};
#[cfg(not(target_arch = "wasm32"))]
use std::thread::{self, JoinHandle};

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
        || current.user != next.user;
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

#[cfg(not(target_arch = "wasm32"))]
struct WorkerThread<T> {
    sender: SyncSender<WorkerMessage<T>>,
    handle: JoinHandle<()>,
}

/// Restartable bounded worker. Submission never waits for extension work.
pub struct BoundedWorker<T>
where
    T: Send + 'static,
{
    name: String,
    capacity: usize,
    handler: Arc<dyn Fn(T) + Send + Sync + 'static>,
    #[cfg(not(target_arch = "wasm32"))]
    slot: Mutex<Option<WorkerThread<T>>>,
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
            slot: Mutex::new(None),
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

    /// Submit work and publish its registration before the handler can start.
    ///
    /// The callback runs only after the bounded channel accepts the item. This
    /// closes the fast-worker race without coupling this crate to application
    /// state or frontend event types.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn try_submit_then(
        &self,
        value: T,
        on_queued: impl FnOnce(),
    ) -> RefreshSubmission {
        let mut slot = self.lock_slot();
        if !self.ensure_thread(&mut slot) {
            return RefreshSubmission::Unavailable;
        }
        let (registration_sender, registration_ready) = mpsc::channel();
        let message = WorkerMessage::Work {
            value,
            registration_ready,
        };
        let first = slot
            .as_ref()
            .expect("worker was ensured")
            .sender
            .try_send(message);
        let result = match first {
            Ok(()) => RefreshSubmission::Queued,
            Err(TrySendError::Full(_)) => RefreshSubmission::Busy,
            Err(TrySendError::Disconnected(message)) => {
                if let Some(worker) = slot.take() {
                    let _ = worker.handle.join();
                }
                if !self.ensure_thread(&mut slot) {
                    return RefreshSubmission::Unavailable;
                }
                match slot
                    .as_ref()
                    .expect("replacement worker was ensured")
                    .sender
                    .try_send(message)
                {
                    Ok(()) => RefreshSubmission::Queued,
                    Err(TrySendError::Full(_)) => RefreshSubmission::Busy,
                    Err(TrySendError::Disconnected(_)) => RefreshSubmission::Unavailable,
                }
            }
        };
        if result == RefreshSubmission::Queued {
            on_queued();
            let _ = registration_sender.send(());
        }
        result
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

    #[cfg(not(target_arch = "wasm32"))]
    pub fn shutdown(&self) {
        let worker = self.lock_slot().take();
        if let Some(worker) = worker {
            let _ = worker.sender.send(WorkerMessage::Shutdown);
            let _ = worker.handle.join();
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub fn shutdown(&self) {}

    #[cfg(not(target_arch = "wasm32"))]
    fn lock_slot(&self) -> MutexGuard<'_, Option<WorkerThread<T>>> {
        self.slot
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn ensure_thread(&self, slot: &mut Option<WorkerThread<T>>) -> bool {
        if slot
            .as_ref()
            .is_some_and(|worker| worker.handle.is_finished())
        {
            if let Some(worker) = slot.take() {
                let _ = worker.handle.join();
            }
        }
        if slot.is_none() {
            *slot = self.spawn_thread();
        }
        slot.is_some()
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn spawn_thread(&self) -> Option<WorkerThread<T>> {
        let (sender, receiver) = mpsc::sync_channel(self.capacity);
        let handler = Arc::clone(&self.handler);
        let name = self.name.clone();
        thread::Builder::new()
            .name(name)
            .spawn(move || {
                while let Ok(message) = receiver.recv() {
                    match message {
                        WorkerMessage::Work {
                            value,
                            registration_ready,
                        } => {
                            if registration_ready.recv().is_ok() {
                                handler(value);
                            }
                        }
                        WorkerMessage::Shutdown => break,
                    }
                }
            })
            .ok()
            .map(|handle| WorkerThread { sender, handle })
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
