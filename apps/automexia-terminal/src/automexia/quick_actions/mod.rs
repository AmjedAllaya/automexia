//! Application-owned CP2.1 Quick Action persistence and refresh services.
//!
//! The typed model remains capability-free in `automexia-devops::actions`.
//! This boundary owns only the explicit user-private `actions/` directory,
//! bounded atomic storage, immutable last-known-good snapshots, and an exact
//! parent-directory watcher. It has no renderer, input, VT, PTY, provider,
//! network, shell-profile, clipboard, secret-store, or execution authority.

mod refresh;
mod secure_fs;
mod service;
mod store;

pub use refresh::{
    ExactActionWatchPlan, QuickActionMonitor, RefreshCoordinator, RefreshDecision,
    WatchErrorCode, MAX_WATCH_EVENTS_PER_POLL, WATCH_EVENT_CAPACITY,
};
pub use service::{QuickActionService, RefreshOutcome, ServiceStatus};
pub use store::{
    LoadOrigin, LoadResult, QuickActionSnapshot, QuickActionStore, StoreError,
    StoreErrorCode, ACTIONS_FILE_NAME, LOCK_FILE_NAME, MAX_CACHED_ACTION_BYTES,
    PREVIOUS_ACTIONS_FILE_NAME,
};
