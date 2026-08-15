// scheduler.rs was retired originally from https://github.com/alacritty/alacritty/blob/e35e5ad14fce8456afdd89f2b392b9924bb27471/alacritty/src/scheduler.rs
// which is licensed under Apache 2.0 license.

use crate::event::EventPayload;
use std::collections::VecDeque;
use std::time::{Duration, Instant};

use rio_window::event_loop::EventLoopProxy;

/// ID uniquely identifying a timer.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct TimerId {
    topic: Topic,
    id: usize,
}

impl TimerId {
    pub fn new(topic: Topic, id: usize) -> Self {
        Self { topic, id }
    }
}

/// Available timer topics.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Topic {
    Render,
    RenderRoute,
    ScheduledRenderRoute,
    UpdateConfig,
    CursorBlinking,
    UpdateTitles,
    SelectionScrolling,
}

/// Event scheduled to be emitted at a specific time.
#[derive(Debug)]
pub struct Timer {
    pub deadline: Instant,
    pub event: EventPayload,
    pub id: TimerId,

    interval: Option<Duration>,
}

/// Scheduler tracking all pending timers.
pub struct Scheduler {
    timers: VecDeque<Timer>,
    event_proxy: EventLoopProxy<EventPayload>,
}

impl Scheduler {
    pub fn new(event_proxy: EventLoopProxy<EventPayload>) -> Self {
        Self {
            timers: VecDeque::new(),
            event_proxy,
        }
    }

    /// Process all pending timers.
    ///
    /// If there are still timers pending after all ready events have been processed, the closest
    /// pending deadline will be returned.
    pub fn update(&mut self) -> Option<Instant> {
        let now = Instant::now();

        while !self.timers.is_empty() && self.timers[0].deadline <= now {
            if let Some(timer) = self.timers.pop_front() {
                // Automatically repeat the event.
                if let Some(interval) = timer.interval {
                    self.schedule(timer.event.clone(), interval, true, timer.id);
                }
                let _ = self.event_proxy.send_event(timer.event);
            }
        }

        self.timers.front().map(|timer| timer.deadline)
    }

    /// Schedule a new event.
    pub fn schedule(
        &mut self,
        event: EventPayload,
        interval: Duration,
        repeat: bool,
        timer_id: TimerId,
    ) {
        let deadline = Instant::now() + interval;

        self.insert_timer(event, interval, repeat, timer_id, deadline);
    }

    /// Schedule a one-shot event unless an equal or earlier event with the same ID exists.
    ///
    /// Render requests commonly share a timer ID so they can be coalesced. Keeping the first
    /// request unconditionally can leave urgent input-driven redraws waiting behind a much slower
    /// maintenance refresh. This method preserves coalescing while allowing the earliest requested
    /// deadline to win.
    pub fn schedule_earliest(
        &mut self,
        event: EventPayload,
        interval: Duration,
        timer_id: TimerId,
    ) -> bool {
        let deadline = Instant::now() + interval;
        let existing_index = self.timers.iter().position(|timer| timer.id == timer_id);

        if let Some(index) = existing_index {
            if !should_replace_timer(self.timers[index].deadline, deadline) {
                return false;
            }

            self.timers.remove(index);
        }

        self.insert_timer(event, interval, false, timer_id, deadline);
        true
    }

    fn insert_timer(
        &mut self,
        event: EventPayload,
        interval: Duration,
        repeat: bool,
        timer_id: TimerId,
        deadline: Instant,
    ) {
        // Get insert position in the schedule.
        let index = self
            .timers
            .iter()
            .position(|timer| timer.deadline > deadline)
            .unwrap_or(self.timers.len());

        // Set the automatic event repeat rate.
        let interval = if repeat { Some(interval) } else { None };

        self.timers.insert(
            index,
            Timer {
                interval,
                deadline,
                event,
                id: timer_id,
            },
        );
    }

    /// Cancel a scheduled event.
    pub fn unschedule(&mut self, id: TimerId) -> Option<Timer> {
        let index = self.timers.iter().position(|timer| timer.id == id)?;
        self.timers.remove(index)
    }

    fn remove_route_timers(timers: &mut VecDeque<Timer>, route_id: usize) {
        timers.retain(|timer| timer.id.id != route_id);
    }

    /// Check if a timer is already scheduled.
    pub fn scheduled(&mut self, id: TimerId) -> bool {
        self.timers.iter().any(|timer| timer.id == id)
    }

    /// Remove all timers scheduled for a tab.
    ///
    /// This must be called when a tab is removed to ensure that timers on intervals do not
    /// stick around forever and cause a memory leak.
    pub fn unschedule_window(&mut self, id: usize) {
        Self::remove_route_timers(&mut self.timers, id);
    }
}

#[inline]
fn should_replace_timer(existing_deadline: Instant, requested_deadline: Instant) -> bool {
    requested_deadline < existing_deadline
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{RioEventType, WindowId};
    use std::time::{Duration, Instant};

    #[test]
    fn urgent_timer_replaces_slower_timer() {
        let now = Instant::now();
        assert!(should_replace_timer(
            now + Duration::from_secs(3),
            now + Duration::from_millis(10),
        ));
    }

    fn timer(topic: Topic, route_id: usize) -> Timer {
        Timer {
            deadline: Instant::now(),
            event: EventPayload::new(RioEventType::Frame, WindowId::from(7)),
            id: TimerId::new(topic, route_id),
            interval: None,
        }
    }

    #[test]
    fn closing_one_window_removes_all_of_its_route_timers_only() {
        let mut timers = VecDeque::from([
            timer(Topic::Render, 11),
            timer(Topic::CursorBlinking, 11),
            timer(Topic::SelectionScrolling, 11),
            timer(Topic::RenderRoute, 22),
            timer(Topic::ScheduledRenderRoute, 22),
        ]);

        Scheduler::remove_route_timers(&mut timers, 11);

        assert_eq!(timers.len(), 2);
        assert!(timers.iter().all(|timer| timer.id.id == 22));
        assert!(timers
            .iter()
            .any(|timer| timer.id.topic == Topic::RenderRoute));
        assert!(timers
            .iter()
            .any(|timer| timer.id.topic == Topic::ScheduledRenderRoute));
    }

    #[test]
    fn slower_or_equal_timer_does_not_postpone_existing_timer() {
        let now = Instant::now();
        let existing = now + Duration::from_millis(10);
        assert!(!should_replace_timer(
            existing,
            now + Duration::from_secs(3),
        ));
        assert!(!should_replace_timer(existing, existing));
    }
}
