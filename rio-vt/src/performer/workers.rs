//! One bounded application-owned join service. Process/pipe ownership stays
//! with each PTY worker; no worker join or platform teardown runs on input.

use super::PtyWorkerHandle;
use std::io;
use std::sync::{mpsc, Arc, Mutex, MutexGuard};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

const MAX_WORKERS: usize = 256;
const REAP_INTERVAL: Duration = Duration::from_millis(50);
const FINAL_JOIN_BUDGET: Duration = Duration::from_secs(10);

#[derive(Clone, Default)]
pub struct PtyWorkerRegistry(Arc<Mutex<State>>);

#[derive(Default)]
struct State {
    entries: Vec<Entry>,
    next_id: u64,
    next_poll: Option<Instant>,
    retired: usize,
    reaper: Option<Reaper>,
    cleanup_fault_reported: bool,
}

struct Entry {
    id: u64,
    worker: Option<PtyWorkerHandle<()>>,
    retired: bool,
}

struct JoinJob {
    id: u64,
    worker: PtyWorkerHandle<()>,
}

struct Reaper {
    jobs: mpsc::SyncSender<JoinJob>,
    completed: mpsc::Receiver<(u64, bool)>,
    thread: JoinHandle<()>,
}

impl Reaper {
    fn start() -> io::Result<Self> {
        let (jobs, receive): (_, mpsc::Receiver<JoinJob>) =
            mpsc::sync_channel(MAX_WORKERS);
        let (finished, completed) = mpsc::sync_channel(MAX_WORKERS);
        let thread = std::thread::Builder::new()
            .name("PTY cleanup".into())
            .spawn(move || {
                while let Ok(mut job) = receive.recv() {
                    // is_finished/body notifications can precede native TLS cleanup.
                    // Always perform the actual join here, never on the event thread.
                    let success = job
                        .worker
                        .thread
                        .take()
                        .is_none_or(|thread| thread.join().is_ok());
                    let _ = finished.send((job.id, success));
                }
            })?;
        Ok(Self {
            jobs,
            completed,
            thread,
        })
    }
}

/// Reserves capacity before a PTY is created; errors release the reservation.
pub struct PtyWorkerReservation {
    registry: PtyWorkerRegistry,
    id: Option<u64>,
}

/// Dropping a lease retires its worker without waiting for native shutdown.
pub struct PtyWorkerLease {
    registry: PtyWorkerRegistry,
    id: u64,
}

impl PtyWorkerRegistry {
    fn state(&self) -> MutexGuard<'_, State> {
        self.0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    pub fn reserve(&self) -> io::Result<PtyWorkerReservation> {
        let mut state = self.state();
        state.reap_finished();
        if state.entries.len() >= MAX_WORKERS {
            return Err(io::Error::other(
                "terminal session capacity reached; wait for closing sessions to finish",
            ));
        }
        let id = state.next_id;
        let next = id.checked_add(1).ok_or_else(|| {
            io::Error::other("terminal worker identity capacity reached")
        })?;
        if state.reaper.is_none() {
            state.reaper = Some(Reaper::start()?);
            state.cleanup_fault_reported = false;
        }
        state.next_id = next;
        state.entries.push(Entry {
            id,
            worker: None,
            retired: false,
        });
        Ok(PtyWorkerReservation {
            registry: self.clone(),
            id: Some(id),
        })
    }

    /// Reclaim completed bookkeeping only. Active sessions create no timer;
    /// the cleanup thread sleeps on its channel whenever there is no join job.
    pub fn poll_cleanup(&self) -> Option<Instant> {
        let mut state = self.state();
        if state.retired == 0 {
            return None;
        }
        let now = Instant::now();
        if state.next_poll.is_some_and(|deadline| now < deadline) {
            return state.next_poll;
        }
        state.reap_finished();
        state.queue_retired();
        state.next_poll = (state.retired != 0).then_some(now + REAP_INTERVAL);
        state.next_poll
    }

    /// After broadcasting shutdown, await acknowledgements of actual joins
    /// against one shared deadline. A timeout retains ownership, including
    /// queued jobs; it never frees capacity or fabricates successful cleanup.
    pub fn finish_shutdown(&self, budget: Duration) -> usize {
        let mut state = self.state();
        state.finish(budget);
        if state.entries.is_empty() {
            state.stop_idle_reaper();
        }
        state.entries.len()
    }
}

impl PtyWorkerReservation {
    pub fn attach(mut self, worker: PtyWorkerHandle<()>) -> PtyWorkerLease {
        let id = self.id.take().expect("one worker per reservation");
        self.registry
            .state()
            .entries
            .iter_mut()
            .find(|entry| entry.id == id)
            .expect("live worker reservation")
            .worker = Some(worker);
        PtyWorkerLease {
            registry: self.registry.clone(),
            id,
        }
    }
}

impl Drop for PtyWorkerReservation {
    fn drop(&mut self) {
        if let Some(id) = self.id {
            self.registry.state().entries.retain(|entry| entry.id != id);
        }
    }
}

impl Drop for PtyWorkerLease {
    fn drop(&mut self) {
        let mut state = self.registry.state();
        if let Some(entry) = state.entries.iter_mut().find(|entry| entry.id == self.id) {
            entry.retired = true;
            state.retired += 1;
            state.next_poll = None;
            state.queue_retired();
        }
    }
}

impl State {
    fn queue_retired(&mut self) {
        let Some(reaper) = &self.reaper else { return };
        for entry in &mut self.entries {
            if !entry.retired {
                continue;
            }
            let Some(worker) = entry.worker.take() else {
                continue;
            };
            if let Err(error) = reaper.jobs.try_send(JoinJob {
                id: entry.id,
                worker,
            }) {
                // Reservation accounting prevents a full queue in normal use.
                // Retain the exact owner on failure; never block or detach.
                let (mpsc::TrySendError::Full(job)
                | mpsc::TrySendError::Disconnected(job)) = error;
                entry.worker = Some(job.worker);
                if !self.cleanup_fault_reported {
                    tracing::error!(
                        "PTY cleanup queue unavailable; retaining worker ownership"
                    );
                    self.cleanup_fault_reported = true;
                }
            }
        }
    }

    fn acknowledge(&mut self, id: u64, success: bool) {
        if !success {
            tracing::warn!("PTY worker panicked during cleanup");
        }
        if let Some(index) = self
            .entries
            .iter()
            .position(|entry| entry.id == id && entry.retired)
        {
            self.entries.swap_remove(index);
            self.retired -= 1;
        }
    }

    fn reap_finished(&mut self) {
        while let Some(result) = self
            .reaper
            .as_ref()
            .and_then(|reaper| reaper.completed.try_recv().ok())
        {
            self.acknowledge(result.0, result.1);
        }
    }

    fn finish(&mut self, budget: Duration) {
        self.queue_retired();
        self.reap_finished();
        let started = Instant::now();
        while self.retired != 0 {
            let Some(reaper) = &self.reaper else { break };
            let remaining = budget.saturating_sub(started.elapsed());
            if remaining.is_zero() {
                break;
            }
            match reaper.completed.recv_timeout(remaining) {
                Ok((id, success)) => self.acknowledge(id, success),
                Err(_) => break,
            }
        }
    }

    fn stop_idle_reaper(&mut self) {
        if let Some(reaper) = self.reaper.take() {
            drop(reaper.jobs);
            // All foreign worker joins already acknowledged. The service has
            // no callbacks/TLS destructors and only exits its receive loop.
            if reaper.thread.join().is_err() {
                tracing::error!("PTY cleanup service panicked");
            }
        }
    }
}

impl Drop for State {
    fn drop(&mut self) {
        self.finish(FINAL_JOIN_BUDGET);
        if self.entries.is_empty() {
            self.stop_idle_reaper();
        } else {
            // Only final owner destruction can reach this. Runtime retirement
            // always retains the service. A stuck OS primitive is a failure,
            // not successful shutdown; normal application exit reports it too.
            tracing::error!(
                pending = self.entries.len(),
                "PTY cleanup exceeded final shutdown budget"
            );
        }
    }
}

#[cfg(test)]
mod tests;
