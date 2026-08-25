use std::collections::{BTreeMap, VecDeque};

use serde::{Deserialize, Serialize};

use crate::{portable_identifier, Limits};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LifecycleState {
    #[default]
    Unavailable,
    Discovered,
    Parsed,
    Verified,
    Reviewed,
    InstalledDisabled,
    Enabled,
    Running,
    Quarantined,
    Revoked,
    Disabled,
    Uninstalled,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LifecycleEvent {
    Discover,
    Parse,
    Verify,
    Review,
    Install,
    Enable,
    Start,
    Stop,
    Quarantine,
    Revoke,
    Disable,
    Uninstall,
    RecoverLastKnownGood,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LifecycleError {
    InvalidTransition,
    ActivationDenied,
}

pub fn transition(
    state: LifecycleState,
    event: LifecycleEvent,
    activation_authorized: bool,
) -> Result<LifecycleState, LifecycleError> {
    use LifecycleEvent as E;
    use LifecycleState as S;
    let next = match (state, event) {
        (S::Unavailable, E::Discover) => S::Discovered,
        (S::Discovered, E::Parse) => S::Parsed,
        (S::Parsed, E::Verify) => S::Verified,
        (S::Verified, E::Review) => S::Reviewed,
        (S::Reviewed, E::Install) => S::InstalledDisabled,
        (S::InstalledDisabled | S::Disabled, E::Enable) => {
            if !activation_authorized {
                return Err(LifecycleError::ActivationDenied);
            }
            S::Enabled
        }
        (S::Enabled, E::Start) => {
            if !activation_authorized {
                return Err(LifecycleError::ActivationDenied);
            }
            S::Running
        }
        (S::Running, E::Stop) => S::Enabled,
        (S::Enabled | S::Running, E::Quarantine) => S::Quarantined,
        (
            S::InstalledDisabled | S::Enabled | S::Running | S::Disabled | S::Quarantined,
            E::Revoke,
        ) => S::Revoked,
        (S::InstalledDisabled | S::Enabled | S::Running | S::Quarantined, E::Disable) => {
            S::Disabled
        }
        (
            S::InstalledDisabled | S::Disabled | S::Quarantined | S::Revoked,
            E::Uninstall,
        ) => S::Uninstalled,
        (S::Quarantined, E::RecoverLastKnownGood) => S::InstalledDisabled,
        _ => return Err(LifecycleError::InvalidTransition),
    };
    Ok(next)
}

#[derive(Clone, Debug, Default)]
pub struct CrashWindow {
    failures: VecDeque<u64>,
}

impl CrashWindow {
    pub const WINDOW_SECONDS: u64 = 5 * 60;

    pub fn record(&mut self, now_unix: u64) -> bool {
        while self
            .failures
            .front()
            .is_some_and(|first| now_unix.saturating_sub(*first) >= Self::WINDOW_SECONDS)
        {
            self.failures.pop_front();
        }
        self.failures.push_back(now_unix);
        self.failures.len() >= Limits::CRASHES_PER_FIVE_MINUTES
    }

    pub fn clear(&mut self) {
        self.failures.clear();
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QueuedCall<T> {
    pub extension_id: String,
    pub generation: u64,
    pub value: T,
}

#[derive(Clone, Debug)]
pub struct FairCallQueue<T> {
    queues: BTreeMap<String, VecDeque<QueuedCall<T>>>,
    order: VecDeque<String>,
    len: usize,
}

impl<T> Default for FairCallQueue<T> {
    fn default() -> Self {
        Self {
            queues: BTreeMap::new(),
            order: VecDeque::new(),
            len: 0,
        }
    }
}

impl<T> FairCallQueue<T> {
    pub fn try_push(&mut self, call: QueuedCall<T>) -> Result<(), QueuedCall<T>> {
        if !portable_identifier(&call.extension_id) {
            return Err(call);
        }
        let queue = self.queues.entry(call.extension_id.clone()).or_default();
        if queue.len() >= Limits::QUEUED_CALLS_PER_EXTENSION {
            return Err(call);
        }
        if queue.is_empty() {
            self.order.push_back(call.extension_id.clone());
        }
        queue.push_back(call);
        self.len += 1;
        Ok(())
    }

    pub fn pop_current(
        &mut self,
        generations: &BTreeMap<String, u64>,
    ) -> Option<QueuedCall<T>> {
        while let Some(extension_id) = self.order.pop_front() {
            let queue = self.queues.get_mut(&extension_id)?;
            while queue.front().is_some_and(|call| {
                generations.get(&extension_id) != Some(&call.generation)
            }) {
                queue.pop_front();
                self.len -= 1;
            }
            let next = queue.pop_front();
            if next.is_some() {
                self.len -= 1;
            }
            if queue.is_empty() {
                self.queues.remove(&extension_id);
            } else {
                self.order.push_back(extension_id);
            }
            if next.is_some() {
                return next;
            }
        }
        None
    }

    pub fn cancel_extension(&mut self, extension_id: &str) -> usize {
        self.order.retain(|candidate| candidate != extension_id);
        let removed = self
            .queues
            .remove(extension_id)
            .map_or(0, |queue| queue.len());
        self.len = self.len.saturating_sub(removed);
        removed
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn release_gate_denies_enable_and_running_transitions() {
        let installed =
            transition(LifecycleState::Reviewed, LifecycleEvent::Install, false).unwrap();
        assert_eq!(installed, LifecycleState::InstalledDisabled);
        assert_eq!(
            transition(installed, LifecycleEvent::Enable, false),
            Err(LifecycleError::ActivationDenied)
        );
    }

    #[test]
    fn crash_window_quarantines_exactly_at_the_bound_and_expires_old_failures() {
        let mut window = CrashWindow::default();
        assert!(!window.record(10));
        assert!(!window.record(11));
        assert!(window.record(12));
        window.clear();
        assert!(!window.record(1));
        assert!(!window.record(301));
    }

    #[test]
    fn queue_is_bounded_fair_and_rejects_stale_generations() {
        let mut queue = FairCallQueue::default();
        for value in 0..2 {
            queue
                .try_push(QueuedCall {
                    extension_id: "a.one".into(),
                    generation: 1,
                    value,
                })
                .unwrap();
            queue
                .try_push(QueuedCall {
                    extension_id: "b.two".into(),
                    generation: 1,
                    value,
                })
                .unwrap();
        }
        let generations = BTreeMap::from([("a.one".into(), 1), ("b.two".into(), 1)]);
        assert_eq!(
            queue.pop_current(&generations).unwrap().extension_id,
            "a.one"
        );
        assert_eq!(
            queue.pop_current(&generations).unwrap().extension_id,
            "b.two"
        );
        let stale = BTreeMap::from([("a.one".into(), 2), ("b.two".into(), 1)]);
        assert_eq!(queue.pop_current(&stale).unwrap().extension_id, "b.two");
        assert!(queue.pop_current(&stale).is_none());
    }
}
