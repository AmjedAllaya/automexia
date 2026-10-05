// Copyright 2026 Automexia contributors.
// Licensed under MIT OR Apache-2.0, like the adapted AccessKit source.

//! Bounded event transport with a separate, authoritative window registry.
//! Overflow requests a fresh native tree; it never discards window removal.

use async_channel::{Receiver, Sender};
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    sync::{Arc, Mutex},
};

pub(crate) const MAX_ADAPTERS: usize = 64;
const MAX_EVENTS: usize = 8192;
const MAX_EVENT_BYTES: usize = 4 * 1024 * 1024;
const BATCH_EVENTS: usize = 64;

struct State<T, E> {
    live: BTreeSet<usize>,
    added: BTreeMap<usize, T>,
    events: VecDeque<(E, usize)>,
    bytes: usize,
    reset: bool,
    closed: bool,
}

pub(crate) struct Queue<T, E> {
    state: Arc<Mutex<State<T, E>>>,
    wake: Sender<()>,
}

impl<T, E> std::fmt::Debug for Queue<T, E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Bounded accessibility queue")
    }
}

impl<T, E> Clone for Queue<T, E> {
    fn clone(&self) -> Self {
        Self {
            state: Arc::clone(&self.state),
            wake: self.wake.clone(),
        }
    }
}

pub(crate) struct Batch<T, E> {
    pub(crate) live: BTreeSet<usize>,
    pub(crate) added: BTreeMap<usize, T>,
    pub(crate) events: Vec<E>,
    pub(crate) reset: bool,
}

impl<T, E> Queue<T, E> {
    pub(crate) fn new() -> (Self, Receiver<()>) {
        let (wake, receiver) = async_channel::bounded(1);
        (
            Self {
                state: Arc::new(Mutex::new(State {
                    live: BTreeSet::new(),
                    added: BTreeMap::new(),
                    events: VecDeque::new(),
                    bytes: 0,
                    reset: false,
                    closed: false,
                })),
                wake,
            },
            receiver,
        )
    }

    pub(crate) fn add(&self, id: usize, entry: T) -> bool {
        let Ok(mut state) = self.state.lock() else {
            return false;
        };
        if state.closed || state.live.len() == MAX_ADAPTERS || state.live.contains(&id) {
            return false;
        }
        state.live.insert(id);
        state.added.insert(id, entry);
        let _ = self.wake.try_send(());
        true
    }

    pub(crate) fn remove(&self, id: usize) {
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        state.live.remove(&id);
        // A not-yet-activated entry has no foreign provider resources. Active
        // entries remain owned by the worker until it reconciles `live`.
        state.added.remove(&id);
        let _ = self.wake.try_send(());
    }

    pub(crate) fn event(&self, event: E, bytes: usize) {
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        if state.closed || state.reset {
            return;
        }
        if state.events.len() == MAX_EVENTS
            || bytes > MAX_EVENT_BYTES.saturating_sub(state.bytes)
        {
            state.events.clear();
            state.bytes = 0;
            state.reset = true;
        } else {
            state.bytes += bytes;
            state.events.push_back((event, bytes));
        }
        let _ = self.wake.try_send(());
    }

    pub(crate) fn reset_pending(&self) -> bool {
        self.state
            .lock()
            .map_or(true, |state| state.reset || state.closed)
    }

    pub(crate) fn idle(&self) -> bool {
        self.state
            .lock()
            .is_ok_and(|state| !state.closed && !state.reset && state.events.is_empty())
    }

    pub(crate) fn wake(&self) {
        let _ = self.wake.try_send(());
    }

    /// Called only after the worker deactivates all trees and drops the old bus.
    /// Inactive adapters cannot publish new tree events until reactivation.
    pub(crate) fn reset_complete(&self) {
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        state.events.clear();
        state.bytes = 0;
        state.reset = false;
    }

    pub(crate) fn batch(&self) -> Option<Batch<T, E>> {
        let Ok(mut state) = self.state.lock() else {
            return None;
        };
        if state.closed {
            return None;
        }
        let mut events = Vec::with_capacity(BATCH_EVENTS);
        for _ in 0..BATCH_EVENTS {
            let Some((event, bytes)) = state.events.pop_front() else {
                break;
            };
            state.bytes -= bytes;
            events.push(event);
        }
        if !state.events.is_empty() {
            let _ = self.wake.try_send(());
        }
        Some(Batch {
            live: state.live.clone(),
            added: std::mem::take(&mut state.added),
            events,
            reset: state.reset,
        })
    }

    pub(crate) fn close(&self) {
        if let Ok(mut state) = self.state.lock() {
            state.closed = true;
            state.live.clear();
            state.added.clear();
            state.events.clear();
            state.bytes = 0;
        }
        self.wake.close();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overflow_preserves_removal_and_out_of_order_registration() {
        let (queue, wakes) = Queue::<usize, usize>::new();
        assert!(queue.add(9, 90));
        assert!(queue.add(2, 20));
        let first = queue.batch().unwrap();
        assert_eq!(first.live.into_iter().collect::<Vec<_>>(), [2, 9]);
        for n in 0..=MAX_EVENTS {
            queue.event(n, 1);
        }
        queue.remove(9);
        assert!(queue.add(3, 30));
        let batch = queue.batch().unwrap();
        assert!(batch.reset);
        assert!(batch.events.is_empty());
        assert_eq!(batch.live.into_iter().collect::<Vec<_>>(), [2, 3]);
        assert_eq!(batch.added.into_iter().collect::<Vec<_>>(), [(3, 30)]);
        assert_eq!(wakes.len(), 1);
        queue.reset_complete();
        queue.event(42, 1);
        assert_eq!(queue.batch().unwrap().events, [42]);
    }

    #[test]
    fn byte_limit_cannot_overflow_or_retain_oversized_events() {
        let (queue, _) = Queue::<(), usize>::new();
        queue.event(1, MAX_EVENT_BYTES);
        queue.event(2, usize::MAX);
        assert!(queue.reset_pending());
        assert!(queue.batch().unwrap().events.is_empty());
        queue.event(3, 0);
        assert!(queue.batch().unwrap().events.is_empty());
    }

    #[test]
    fn lifecycle_capacity_releases_before_another_worker_tick() {
        let (queue, wakes) = Queue::<(), ()>::new();
        for id in 0..MAX_ADAPTERS {
            assert!(queue.add(id, ()));
        }
        assert!(!queue.add(MAX_ADAPTERS, ()));
        queue.remove(0);
        assert!(queue.add(MAX_ADAPTERS, ()));
        assert_eq!(queue.batch().unwrap().live.len(), MAX_ADAPTERS);
        queue.close();
        assert!(wakes.is_closed());
        assert!(queue.batch().is_none());
        assert!(!queue.add(0, ()));
        queue.event((), usize::MAX);
        assert!(queue.reset_pending());
    }

    #[test]
    fn batches_are_bounded_and_rearm_the_coalesced_wake() {
        let (queue, wakes) = Queue::<(), usize>::new();
        for n in 0..BATCH_EVENTS + 1 {
            queue.event(n, 1);
        }
        assert_eq!(wakes.try_recv(), Ok(()));
        assert_eq!(queue.batch().unwrap().events.len(), BATCH_EVENTS);
        assert_eq!(wakes.try_recv(), Ok(()));
        assert_eq!(queue.batch().unwrap().events, [BATCH_EVENTS]);
        assert!(wakes.try_recv().is_err());
    }
}
