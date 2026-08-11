//! Rate-gated "keep presenting after input" tracker.
//!
//! Replaces the single-timestamp `last_input_timestamp` pattern with
//! a rolling-window input-rate detector, matching Zed's
//! `InputRateTracker` (`zed/crates/gpui/src/window.rs:987-1027`).
//!
//! The single-timestamp version sustained a 1-second post-input
//! presentation window after *any* input. Combined with very cheap
//! per-frame work (e.g. Rio's new-v4 grid renderer), that forced
//! ~60+ vsync redraws after every keystroke, saturating the Metal
//! drawable pool on macOS and wasting power everywhere.
//!
//! This tracker only extends the sustain window when input arrives
//! at a *high rate* (≥ 60 events/sec in a 100 ms rolling window) —
//! the condition that actually causes ProMotion / VRR displays to
//! downclock. Single keystrokes or isolated mouse events don't
//! trigger the sustain.
//!
//! All platform backends (macOS, Wayland, X11, Windows) share this
//! struct. Single-threaded platforms wrap it in `RefCell`; Windows'
//! DwmFlush worker wraps it in `Mutex`.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct InputRateTracker {
    timestamps: VecDeque<Instant>,
    window: Duration,
    inputs_per_second: u32,
    sustain_until: Instant,
    sustain_duration: Duration,
}

impl Default for InputRateTracker {
    fn default() -> Self {
        Self {
            timestamps: VecDeque::new(),
            window: Duration::from_millis(100),
            inputs_per_second: 60,
            sustain_until: Instant::now(),
            sustain_duration: Duration::from_secs(1),
        }
    }
}

impl InputRateTracker {
    #[inline]
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Record an input event. Extends the post-input sustain window
    /// only when input is arriving at ≥ `inputs_per_second` over the
    /// rolling window — a single keystroke / lone mouse move does
    /// not set the sustain.
    /// Returns `true` only on the transition into high-rate mode so an idle
    /// frame worker needs just one wake-up for the sustained burst.
    pub(crate) fn record_input(&mut self) -> bool {
        let now = Instant::now();
        let was_high_rate = now < self.sustain_until;
        self.timestamps.push_back(now);
        self.prune_old_timestamps(now);

        // `min_events` = inputs_per_second × window_ms / 1000. For
        // defaults (60/s, 100 ms) that's 6 events in the last 100 ms.
        let min_events = self.inputs_per_second as u128 * self.window.as_millis() / 1000;
        if self.timestamps.len() as u128 >= min_events {
            self.sustain_until = now + self.sustain_duration;
        }

        !was_high_rate && now < self.sustain_until
    }

    /// `true` while the sustain window set by a recent high-rate
    /// input burst is still active. Drives the "fire a redraw each
    /// vsync even if the app isn't dirty" logic that keeps VRR
    /// displays from downclocking.
    #[inline]
    pub(crate) fn is_high_rate(&self) -> bool {
        Instant::now() < self.sustain_until
    }

    fn prune_old_timestamps(&mut self, now: Instant) {
        while self
            .timestamps
            .front()
            .is_some_and(|&timestamp| now.duration_since(timestamp) > self.window)
        {
            self.timestamps.pop_front();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn isolated_input_does_not_start_the_sustain_window() {
        let mut tracker = InputRateTracker::new();
        assert!(!tracker.record_input());
        assert!(!tracker.is_high_rate());
    }

    #[test]
    fn high_rate_input_starts_the_sustain_window() {
        let mut tracker = InputRateTracker::new();
        for _ in 0..5 {
            assert!(!tracker.record_input());
        }
        assert!(tracker.record_input());
        assert!(!tracker.record_input());
        assert!(tracker.is_high_rate());
    }

    #[test]
    fn pruning_removes_only_expired_prefix_entries() {
        let mut tracker = InputRateTracker::new();
        let now = Instant::now();
        tracker
            .timestamps
            .push_back(now - tracker.window - Duration::from_millis(1));
        tracker.timestamps.push_back(now);
        tracker.prune_old_timestamps(now);
        assert_eq!(tracker.timestamps.len(), 1);
        assert_eq!(tracker.timestamps.front(), Some(&now));
    }
}
