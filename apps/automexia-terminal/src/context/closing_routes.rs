//! Recent close acknowledgements are an optimization, not route authority.
//! Older unknown exits still resolve against the live/parked topology and can
//! never close the selected sibling. Shutdown need not emit a child-exit event.

use std::collections::VecDeque;

const MAX_RECENT_CLOSES: usize = 256;

#[derive(Default)]
pub(super) struct ClosingRoutes(VecDeque<usize>);

impl ClosingRoutes {
    pub(super) fn insert(&mut self, id: usize) {
        if self.0.contains(&id) {
            return;
        }
        if self.0.len() == MAX_RECENT_CLOSES {
            self.0.pop_front();
        }
        self.0.push_back(id);
    }

    pub(super) fn extend(&mut self, ids: impl IntoIterator<Item = usize>) {
        for id in ids {
            self.insert(id);
        }
    }

    pub(super) fn remove(&mut self, id: &usize) -> bool {
        if let Some(index) = self.0.iter().position(|candidate| candidate == id) {
            self.0.remove(index);
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_acknowledgements_and_duplicate_closes_stay_bounded() {
        let mut closes = ClosingRoutes::default();
        for id in 1..10_000 {
            closes.extend([id, id]);
            assert!(closes.0.len() <= MAX_RECENT_CLOSES);
        }
        assert!(!closes.remove(&1));
        assert!(closes.remove(&9_999));
        assert!(!closes.remove(&9_999));
        assert_eq!(closes.0.len(), MAX_RECENT_CLOSES - 1);
    }
}
