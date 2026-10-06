//! One bounded, off-event-thread font preparation owner.
use automexia_extension_runtime::{BoundedWorker, RefreshSubmission};
use rio_backend::event::{EventProxy, RioEvent, RioEventType, WindowId};
use rio_backend::sugarloaf::font::{FontLibrary, SugarloafFonts};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc,
    },
    time::{Duration, Instant},
};

pub(crate) const LOAD_TIMEOUT: Duration = Duration::from_secs(30);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Failure {
    Missing,
    Worker,
    TimedOut,
    Cancelled,
}
pub(crate) enum Prepared {
    Library(FontLibrary),
    Inventory(Vec<String>, bool),
}
enum Job {
    Library(Box<SugarloafFonts>),
    Inventory(FontLibrary),
}
struct Request {
    id: u64,
    job: Job,
    cancelled: Arc<AtomicBool>,
    window: WindowId,
}
struct Completion {
    id: u64,
    result: Result<Prepared, Failure>,
}
struct Pending {
    id: u64,
    cancelled: Arc<AtomicBool>,
    deadline: Instant,
    timed_out: bool,
}
pub(crate) struct FontPreparation {
    worker: BoundedWorker<Request>,
    completed: mpsc::Receiver<Completion>,
    pending: Option<Pending>,
    next: Option<u64>,
}

/// Publish before waking even if the loader unwinds. The runtime retains actual
/// worker join/panic cleanup ownership; this guard only sends a bounded result.
struct Publisher<'a> {
    sender: &'a mpsc::SyncSender<Completion>,
    value: Option<Completion>,
    proxy: Option<&'a EventProxy>,
    window: WindowId,
}
impl Drop for Publisher<'_> {
    fn drop(&mut self) {
        if self
            .value
            .take()
            .is_some_and(|value| self.sender.try_send(value).is_ok())
        {
            if let Some(proxy) = self.proxy {
                proxy.send_event(RioEventType::Rio(RioEvent::Render), self.window);
            }
        }
    }
}
fn prepare_library(mut fonts: SugarloafFonts) -> Result<FontLibrary, Failure> {
    // Warning suppression is a configuration display choice, not permission to
    // save a font that failed to load from the Fonts editor.
    fonts.disable_warnings_not_found = false;
    let (library, errors) = FontLibrary::new(fonts);
    if errors.is_some() {
        Err(Failure::Missing)
    } else {
        Ok(library)
    }
}

impl FontPreparation {
    pub fn new(proxy: EventProxy) -> Self {
        Self::with_loader(Some(proxy), prepare_library)
    }
    fn with_loader(
        proxy: Option<EventProxy>,
        load: impl Fn(SugarloafFonts) -> Result<FontLibrary, Failure> + Send + Sync + 'static,
    ) -> Self {
        let (sender, completed) = mpsc::sync_channel(1);
        Self {
            worker: BoundedWorker::new("font-preparation", 1, move |request: Request| {
                let mut publisher = Publisher {
                    sender: &sender,
                    value: Some(Completion {
                        id: request.id,
                        result: Err(Failure::Worker),
                    }),
                    proxy: proxy.as_ref(),
                    window: request.window,
                };
                let result = if request.cancelled.load(Ordering::Acquire) {
                    Err(Failure::Cancelled)
                } else {
                    match request.job {
                        Job::Library(fonts) => load(*fonts).map(Prepared::Library),
                        Job::Inventory(library) => {
                            let (entries, limited) =
                                normalize_inventory(library.family_names());
                            Ok(Prepared::Inventory(entries, limited))
                        }
                    }
                };
                // A late loaded library is disposed on its worker when cancelled.
                let result = if request.cancelled.load(Ordering::Acquire) {
                    Err(Failure::Cancelled)
                } else {
                    result
                };
                publisher.value = Some(Completion {
                    id: request.id,
                    result,
                });
            }),
            completed,
            pending: None,
            next: Some(1),
        }
    }
    pub fn submit(
        &mut self,
        fonts: SugarloafFonts,
        window: WindowId,
        now: Instant,
    ) -> RefreshSubmission {
        self.submit_job(Job::Library(Box::new(fonts)), window, now)
    }
    pub fn submit_inventory(
        &mut self,
        library: FontLibrary,
        window: WindowId,
        now: Instant,
    ) -> RefreshSubmission {
        self.submit_job(Job::Inventory(library), window, now)
    }
    pub fn busy(&self) -> bool {
        self.pending.is_some()
    }
    fn submit_job(
        &mut self,
        job: Job,
        window: WindowId,
        now: Instant,
    ) -> RefreshSubmission {
        if self.pending.is_some() {
            return RefreshSubmission::Busy;
        }
        let Some(id) = self.next else {
            return RefreshSubmission::Unavailable;
        };
        self.next = id.checked_add(1);
        let cancelled = Arc::new(AtomicBool::new(false));
        self.pending = Some(Pending {
            id,
            cancelled: cancelled.clone(),
            deadline: now + LOAD_TIMEOUT,
            timed_out: false,
        });
        let result = self.worker.try_submit(Request {
            id,
            job,
            cancelled,
            window,
        });
        if result != RefreshSubmission::Queued {
            self.pending = None;
        }
        result
    }
    pub fn cancel(&mut self) {
        if let Some(pending) = &self.pending {
            pending.cancelled.store(true, Ordering::Release);
        }
    }
    pub fn deadline(&self) -> Option<Instant> {
        self.pending
            .as_ref()
            .filter(|p| !p.timed_out && !p.cancelled.load(Ordering::Acquire))
            .map(|p| p.deadline)
    }
    pub fn take(&mut self, now: Instant) -> Option<Result<Prepared, Failure>> {
        if let Ok(completion) = self.completed.try_recv() {
            let pending = self.pending.take()?;
            if completion.id == pending.id && !pending.cancelled.load(Ordering::Acquire) {
                return Some(completion.result);
            }
            return None;
        }
        if let Some(pending) = self.pending.as_mut() {
            if !pending.timed_out
                && !pending.cancelled.load(Ordering::Acquire)
                && now >= pending.deadline
            {
                pending.timed_out = true;
                pending.cancelled.store(true, Ordering::Release);
                return Some(Err(Failure::TimedOut));
            }
        }
        None
    }
}
pub(crate) const MAX_INSTALLED_FAMILIES: usize = 4096;
fn normalize_inventory(names: Vec<String>) -> (Vec<String>, bool) {
    use crate::automexia::font_preferences::valid_family;
    let mut limited = names.len() > MAX_INSTALLED_FAMILIES;
    let mut entries: Vec<_> = names
        .into_iter()
        .take(MAX_INSTALLED_FAMILIES * 4)
        .filter(|name| {
            valid_family(name)
                && !name.chars().any(
                    |c| matches!(c, '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}'),
                )
        })
        .map(|name| (name.to_lowercase(), name))
        .collect();
    entries.sort_unstable();
    entries.dedup_by(|a, b| a.0 == b.0);
    limited |= entries.len() > MAX_INSTALLED_FAMILIES;
    entries.truncate(MAX_INSTALLED_FAMILIES);
    let mut entries: Vec<_> = entries.into_iter().map(|(_, name)| name).collect();
    let bundled = rio_backend::sugarloaf::font::constants::DEFAULT_FONT_FAMILY;
    if !entries
        .iter()
        .any(|name| name.eq_ignore_ascii_case(bundled))
    {
        entries.insert(0, bundled.to_owned());
    }
    (entries, limited)
}
impl Drop for FontPreparation {
    fn drop(&mut self) {
        self.cancel();
        self.worker.request_shutdown();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn installed_font_inventory_is_bounded_sorted_deduplicated_and_safe() {
        let (names, limited) = normalize_inventory(vec![
            "Zulu Mono".into(),
            "Alpha Mono".into(),
            "alpha mono".into(),
            "../font.ttf".into(),
            "Bad\nName".into(),
            "Bad\u{202e}Name".into(),
            "a".repeat(129),
            "".into(),
        ]);
        assert!(!limited);
        assert_eq!(names, ["cascadiacode", "Alpha Mono", "Zulu Mono"]);
        let (names, limited) = normalize_inventory(
            (0..MAX_INSTALLED_FAMILIES * 5)
                .map(|i| format!("Font {i:05}"))
                .collect(),
        );
        assert!(limited);
        assert_eq!(names.len(), MAX_INSTALLED_FAMILIES + 1);
    }

    #[test]
    fn installed_font_inventory_uses_native_discovery_off_thread() {
        let mut owner = FontPreparation::with_loader(None, prepare_library);
        let (library, _) = FontLibrary::new(Default::default());
        assert_eq!(
            owner.submit_inventory(library, WindowId::from(0), Instant::now()),
            RefreshSubmission::Queued
        );
        assert!(owner.busy());
        let result = await_result(&mut owner);
        let Ok(Prepared::Inventory(names, _)) = result else {
            panic!("Expected font inventory");
        };
        assert!(names.iter().any(|name| name == "cascadiacode"));
        assert!(!owner.busy());
    }
    fn await_result(owner: &mut FontPreparation) -> Result<Prepared, Failure> {
        let until = Instant::now() + Duration::from_secs(5);
        loop {
            if let Some(result) = owner.take(Instant::now()) {
                return result;
            }
            assert!(Instant::now() < until, "font preparation did not publish");
            std::thread::sleep(Duration::from_millis(5));
        }
    }
    #[test]
    fn fonts_real_loader_accepts_bundled_faces_and_rejects_missing_even_with_warnings_off(
    ) {
        let fonts = SugarloafFonts {
            features: Some(vec!["liga=0".into()]),
            ..Default::default()
        };
        let library = prepare_library(fonts).expect("bundled font must load");
        assert!(library.inner.read().try_get(&0).is_some());
        let missing = SugarloafFonts {
            family: Some("Automexia Missing Font Fixture 7F9A".into()),
            disable_warnings_not_found: true,
            ..Default::default()
        };
        assert!(matches!(prepare_library(missing), Err(Failure::Missing)));
    }
    #[test]
    fn fonts_missing_failure_is_returned_without_publishing_a_library() {
        let mut owner = FontPreparation::with_loader(None, |_| Err(Failure::Missing));
        assert_eq!(
            owner.submit(Default::default(), WindowId::from(0), Instant::now()),
            RefreshSubmission::Queued
        );
        assert!(matches!(await_result(&mut owner), Err(Failure::Missing)));
        assert!(owner.deadline().is_none());
    }
    #[test]
    fn fonts_timeout_keeps_admission_until_the_worker_really_finishes() {
        let (go, wait) = mpsc::sync_channel(1);
        let wait = std::sync::Mutex::new(wait);
        let mut owner = FontPreparation::with_loader(None, move |_| {
            let _ = wait.lock().unwrap().recv();
            Err(Failure::Missing)
        });
        let start = Instant::now();
        assert_eq!(
            owner.submit(Default::default(), WindowId::from(0), start),
            RefreshSubmission::Queued
        );
        assert_eq!(
            owner.submit(Default::default(), WindowId::from(0), start),
            RefreshSubmission::Busy
        );
        assert!(matches!(
            owner.take(start + LOAD_TIMEOUT),
            Some(Err(Failure::TimedOut))
        ));
        assert!(owner.deadline().is_none());
        assert_eq!(
            owner.submit(Default::default(), WindowId::from(0), start),
            RefreshSubmission::Busy
        );
        go.send(()).unwrap();
        let until = Instant::now() + Duration::from_secs(5);
        while owner.pending.is_some() {
            assert!(owner.take(Instant::now()).is_none());
            assert!(Instant::now() < until);
            std::thread::sleep(Duration::from_millis(5));
        }
    }
    #[test]
    fn fonts_cancelled_or_panicked_load_never_publishes_success() {
        let mut owner =
            FontPreparation::with_loader(None, |_| panic!("fixture font loader"));
        assert_eq!(
            owner.submit(Default::default(), WindowId::from(0), Instant::now()),
            RefreshSubmission::Queued
        );
        assert!(matches!(await_result(&mut owner), Err(Failure::Worker)));
        let mut owner = FontPreparation::with_loader(None, |_| Err(Failure::Missing));
        owner.submit(Default::default(), WindowId::from(0), Instant::now());
        owner.cancel();
        let until = Instant::now() + Duration::from_secs(5);
        while owner.pending.is_some() {
            assert!(owner.take(Instant::now()).is_none());
            assert!(Instant::now() < until);
            std::thread::sleep(Duration::from_millis(5));
        }
    }
}
