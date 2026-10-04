use rio_backend::{
    crosswords::{archive::DisplayArchive, Crosswords},
    event::{sync::FairMutex, EventListener},
};
use std::sync::Arc;

/// Runtime-only capture capability. Serde skips it; archived input can never
/// manufacture a terminal reference or executable callback.
#[derive(Clone)]
pub struct HistorySource(Arc<dyn Fn(usize) -> Option<DisplayArchive> + Send + Sync>);
impl HistorySource {
    #[cfg(test)]
    pub(super) fn unavailable() -> Self {
        Self(Arc::new(|_| None))
    }
    pub fn new<T: EventListener + Send + 'static>(
        terminal: Arc<FairMutex<Crosswords<T>>>,
    ) -> Self {
        Self(Arc::new(move |budget| {
            let until = std::time::Instant::now() + std::time::Duration::from_millis(100);
            loop {
                if let Some(terminal) = terminal.try_lock_unfair() {
                    let captured = terminal.capture_display(budget);
                    drop(terminal);
                    return Some(captured.into_archive());
                }
                if std::time::Instant::now() >= until {
                    return None;
                }
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
        }))
    }
    pub(super) fn capture(&self, budget: usize) -> Option<DisplayArchive> {
        (self.0)(budget)
    }
}
impl std::fmt::Debug for HistorySource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("HistorySource")
    }
}
impl PartialEq for HistorySource {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for HistorySource {}

pub(super) mod serialized {
    use super::*;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    pub fn serialize<S: Serializer>(
        history: &Option<Arc<DisplayArchive>>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        history.as_deref().serialize(serializer)
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<Arc<DisplayArchive>>, D::Error> {
        Option::<DisplayArchive>::deserialize(deserializer)
            .map(|history| history.map(Arc::new))
    }
}
