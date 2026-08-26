//! Joined, lazy, one-latest-per-route execution for suggestion ranking.

use std::collections::{BTreeMap, VecDeque};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use automexia_command_productivity::suggestions::{
    EditorRequest, RouteIdentity, SourceBatch, SuggestionCapability,
};

use super::{
    BrokerError, BrokerHealth, SuggestionBroker, SuggestionConfig, SuggestionSnapshot,
};

type Completion = Result<SuggestionSnapshot, BrokerError>;
type Wake = Arc<dyn Fn(RouteIdentity) + Send + Sync + 'static>;

#[derive(Default)]
struct CompletionState {
    value: Mutex<Option<Completion>>,
    ready: Condvar,
}

impl CompletionState {
    fn finish(&self, value: Completion) {
        *lock(&self.value) = Some(value);
        self.ready.notify_all();
    }
}

/// A bounded completion mailbox. Dropping it never cancels or blocks shutdown.
#[derive(Clone, Default)]
pub struct SuggestionTicket {
    state: Arc<CompletionState>,
}

impl SuggestionTicket {
    pub fn try_take(&self) -> Option<Completion> {
        lock(&self.state.value).take()
    }

    pub fn wait_timeout(&self, timeout: Duration) -> Option<Completion> {
        let mut value = lock(&self.state.value);
        if value.is_none() {
            let waited = self
                .state
                .ready
                .wait_timeout(value, timeout)
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            value = waited.0;
        }
        value.take()
    }
}

struct PendingWork {
    request: EditorRequest,
    batches: Vec<SourceBatch>,
    completion: Arc<CompletionState>,
}

struct FairLatestQueue<T> {
    pending: BTreeMap<RouteIdentity, T>,
    order: VecDeque<RouteIdentity>,
}

impl<T> Default for FairLatestQueue<T> {
    fn default() -> Self {
        Self {
            pending: BTreeMap::new(),
            order: VecDeque::new(),
        }
    }
}

impl<T> FairLatestQueue<T> {
    fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }

    fn insert(&mut self, route: RouteIdentity, value: T) -> Option<T> {
        let previous = self.pending.insert(route.clone(), value);
        if previous.is_none() {
            self.order.push_back(route);
        }
        previous
    }

    fn remove(&mut self, route: &RouteIdentity) -> Option<T> {
        let removed = self.pending.remove(route);
        if removed.is_some() {
            self.order.retain(|queued| queued != route);
        }
        removed
    }

    fn pop_next(&mut self) -> Option<T> {
        while let Some(route) = self.order.pop_front() {
            if let Some(value) = self.pending.remove(&route) {
                return Some(value);
            }
        }
        None
    }

    fn take_all(&mut self) -> BTreeMap<RouteIdentity, T> {
        self.order.clear();
        std::mem::take(&mut self.pending)
    }
}

#[derive(Default)]
struct WorkerState {
    pending: FairLatestQueue<PendingWork>,
    shutdown: bool,
}

/// Application-owned CP5 service.
///
/// Disabled construction starts no thread and opens no endpoint. Enabling the
/// separately gated preview starts exactly one joined worker. Submission only
/// replaces one in-memory route slot and wakes that worker; ranking and
/// publication never run on input, PTY, renderer, resize, provider,
/// authentication, or shell-output threads.
pub struct SuggestionService {
    lifecycle: Mutex<()>,
    broker: Arc<Mutex<SuggestionBroker>>,
    worker_state: Arc<(Mutex<WorkerState>, Condvar)>,
    worker: Mutex<Option<JoinHandle<()>>>,
    wake: Wake,
}

impl Default for SuggestionService {
    fn default() -> Self {
        Self::new(|_| {})
    }
}

impl SuggestionService {
    pub fn new(wake: impl Fn(RouteIdentity) + Send + Sync + 'static) -> Self {
        Self {
            lifecycle: Mutex::new(()),
            broker: Arc::new(Mutex::new(SuggestionBroker::default())),
            worker_state: Arc::new((Mutex::new(WorkerState::default()), Condvar::new())),
            worker: Mutex::new(None),
            wake: Arc::new(wake),
        }
    }

    pub fn set_config(&self, config: SuggestionConfig) {
        let _lifecycle = lock(&self.lifecycle);
        lock(&self.broker).set_config(config);
        if config.preview {
            let _ = self.ensure_worker();
        } else {
            self.stop_worker(BrokerError::PreviewDisabled);
        }
    }

    pub fn register_route(
        &self,
        route: RouteIdentity,
        capability: SuggestionCapability,
    ) -> Result<(), BrokerError> {
        let _lifecycle = lock(&self.lifecycle);
        lock(&self.broker).register_route(route, capability)
    }

    pub fn try_submit(
        &self,
        request: EditorRequest,
        batches: Vec<SourceBatch>,
    ) -> Result<SuggestionTicket, BrokerError> {
        let _lifecycle = lock(&self.lifecycle);
        if !self.ensure_worker() {
            return Err(BrokerError::Killed);
        }
        lock(&self.broker).reserve(&request)?;
        let route = request.route();
        let completion = Arc::new(CompletionState::default());
        let ticket = SuggestionTicket {
            state: Arc::clone(&completion),
        };
        let (state, ready) = &*self.worker_state;
        let mut state = lock(state);
        if state.shutdown {
            return Err(BrokerError::Killed);
        }
        if let Some(previous) = state.pending.insert(
            route,
            PendingWork {
                request,
                batches,
                completion,
            },
        ) {
            previous.completion.finish(Err(BrokerError::Superseded));
        }
        ready.notify_one();
        Ok(ticket)
    }

    pub fn snapshot(&self, route: &RouteIdentity) -> Option<SuggestionSnapshot> {
        lock(&self.broker).snapshot(route).cloned()
    }

    pub fn health(&self) -> BrokerHealth {
        lock(&self.broker).health()
    }

    pub fn worker_running(&self) -> bool {
        lock(&self.worker).is_some()
    }

    pub fn close_route(&self, route: &RouteIdentity) -> bool {
        let _lifecycle = lock(&self.lifecycle);
        if let Some(pending) = lock(&self.worker_state.0).pending.remove(route) {
            pending.completion.finish(Err(BrokerError::Superseded));
        }
        lock(&self.broker).close_route(route)
    }

    pub fn kill(&self) {
        let _lifecycle = lock(&self.lifecycle);
        lock(&self.broker).kill();
        self.stop_worker(BrokerError::Killed);
    }

    pub fn reset(&self) {
        let _lifecycle = lock(&self.lifecycle);
        self.cancel_pending(BrokerError::Superseded);
        lock(&self.broker).reset();
    }

    pub fn disable(&self) {
        let _lifecycle = lock(&self.lifecycle);
        lock(&self.broker).disable();
        self.stop_worker(BrokerError::PreviewDisabled);
    }

    pub fn uninstall(&self) {
        let _lifecycle = lock(&self.lifecycle);
        lock(&self.broker).uninstall();
        self.stop_worker(BrokerError::PreviewDisabled);
    }

    fn ensure_worker(&self) -> bool {
        if !lock(&self.broker).health().config.preview {
            return false;
        }
        let mut worker = lock(&self.worker);
        if worker.as_ref().is_some_and(|handle| handle.is_finished()) {
            if let Some(handle) = worker.take() {
                let _ = handle.join();
            }
        }
        if worker.is_some() {
            return true;
        }
        lock(&self.worker_state.0).shutdown = false;
        let broker = Arc::clone(&self.broker);
        let shared = Arc::clone(&self.worker_state);
        let wake = Arc::clone(&self.wake);
        *worker = thread::Builder::new()
            .name("automexia-suggestions".into())
            .spawn(move || worker_loop(broker, shared, wake))
            .ok();
        if worker.is_none() {
            lock(&self.worker_state.0).shutdown = true;
        }
        worker.is_some()
    }

    fn cancel_pending(&self, reason: BrokerError) {
        let pending = lock(&self.worker_state.0).pending.take_all();
        for (_, work) in pending {
            work.completion.finish(Err(reason));
        }
    }

    fn stop_worker(&self, reason: BrokerError) {
        self.cancel_pending(reason);
        let (state, ready) = &*self.worker_state;
        lock(state).shutdown = true;
        ready.notify_all();
        if let Some(worker) = lock(&self.worker).take() {
            let _ = worker.join();
        }
        lock(state).shutdown = false;
    }
}

impl Drop for SuggestionService {
    fn drop(&mut self) {
        self.cancel_pending(BrokerError::Killed);
        let (state, ready) = &*self.worker_state;
        lock(state).shutdown = true;
        ready.notify_all();
        let worker = self
            .worker
            .get_mut()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take();
        if let Some(worker) = worker {
            let _ = worker.join();
        }
        lock(&self.broker).kill();
    }
}

fn worker_loop(
    broker: Arc<Mutex<SuggestionBroker>>,
    shared: Arc<(Mutex<WorkerState>, Condvar)>,
    wake: Wake,
) {
    loop {
        let work = {
            let (state, ready) = &*shared;
            let mut state = lock(state);
            while state.pending.is_empty() && !state.shutdown {
                state = ready
                    .wait(state)
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
            }
            if state.shutdown {
                return;
            }
            state.pending.pop_next()
        };
        let Some(work) = work else {
            continue;
        };
        let route = work.request.route();
        let result = lock(&broker).complete(work.request, &work.batches);
        if result.is_ok() {
            wake(route);
        }
        work.completion.finish(result);
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[cfg(test)]
mod tests {
    use super::FairLatestQueue;
    use automexia_command_productivity::suggestions::RouteIdentity;

    fn route(pane_id: u64) -> RouteIdentity {
        RouteIdentity {
            application_generation: 1,
            window_id: 1,
            tab_id: 1,
            pane_id,
            session_id: pane_id,
            shell: automexia_command_productivity::suggestions::ShellKind::Bash,
            editor_version: "5.2".into(),
            endpoint_instance: 1,
        }
    }

    #[test]
    fn latest_per_route_queue_is_fifo_across_routes() {
        let mut queue = FairLatestQueue::default();
        assert_eq!(queue.insert(route(9), "first"), None);
        assert_eq!(queue.insert(route(1), "second"), None);
        assert_eq!(queue.insert(route(9), "latest"), Some("first"));
        assert_eq!(queue.pop_next(), Some("latest"));
        assert_eq!(queue.pop_next(), Some("second"));
        assert_eq!(queue.pop_next(), None);
    }

    #[test]
    fn removing_a_route_removes_its_fifo_slot() {
        let mut queue = FairLatestQueue::default();
        queue.insert(route(1), 1);
        queue.insert(route(2), 2);
        assert_eq!(queue.remove(&route(1)), Some(1));
        assert_eq!(queue.pop_next(), Some(2));
        assert!(queue.is_empty());
    }
}
