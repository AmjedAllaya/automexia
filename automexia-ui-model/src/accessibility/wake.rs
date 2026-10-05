//! Publish-before-wake lifecycle shared by the native callback adapters.
//! The identical protocol runs with Loom atomics in unit tests.
#[cfg(test)]
use loom::sync::atomic::{AtomicBool, Ordering};
#[cfg(not(test))]
use std::sync::atomic::{AtomicBool, Ordering};

pub struct WakeState {
    active: AtomicBool,
    alive: AtomicBool,
    pending: AtomicBool,
}

impl Default for WakeState {
    fn default() -> Self {
        Self {
            active: AtomicBool::new(false),
            alive: AtomicBool::new(true),
            pending: AtomicBool::new(false),
        }
    }
}

impl WakeState {
    pub fn activate(&self) -> bool {
        self.active.store(true, Ordering::Release);
        self.request_wake()
    }

    pub fn deactivate(&self) -> bool {
        self.active.store(false, Ordering::Release);
        self.request_wake()
    }

    pub fn is_alive(&self) -> bool {
        self.alive.load(Ordering::Acquire)
    }

    pub fn request_wake(&self) -> bool {
        self.is_alive() && !self.pending.swap(true, Ordering::AcqRel)
    }

    /// Clear pending before reading the published state: an activation racing
    /// this frame is either observed here or schedules the following frame.
    pub fn begin_frame(&self) -> bool {
        // Acquire the callback's wake publication before consuming its active
        // flag. A release-only clear can lose activation on weak memory.
        self.pending.swap(false, Ordering::AcqRel);
        self.is_alive() && self.active.load(Ordering::Acquire)
    }

    pub fn close(&self) {
        self.alive.store(false, Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use loom::{sync::Arc, thread};

    #[test]
    fn activation_racing_frame_cannot_lose_the_published_state() {
        loom::model(|| {
            let state = Arc::new(WakeState::default());
            let callback = Arc::clone(&state);
            let activation = thread::spawn(move || callback.activate());
            let observed = state.begin_frame();
            activation.join().unwrap();
            assert!(observed || state.pending.load(Ordering::Acquire));
            assert!(state.begin_frame());
        });
    }

    #[test]
    fn teardown_makes_late_activation_and_actions_inert() {
        loom::model(|| {
            let state = Arc::new(WakeState::default());
            let callback = Arc::clone(&state);
            let activation = thread::spawn(move || {
                callback.activate();
                callback.request_wake();
            });
            state.close();
            activation.join().unwrap();
            assert!(!state.begin_frame());
            assert!(!state.request_wake());
        });
    }

    #[test]
    fn repeated_callbacks_coalesce_and_deactivation_clears_visibility() {
        loom::model(|| {
            let state = WakeState::default();
            assert!(state.activate());
            assert!(!state.request_wake());
            assert!(state.begin_frame());
            assert!(state.deactivate());
            assert!(!state.begin_frame());
        });
    }
}
