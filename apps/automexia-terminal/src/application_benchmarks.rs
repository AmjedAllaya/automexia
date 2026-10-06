//! Opt-in observation for controlled application benchmarks, not product telemetry.
//!
//! No polling, filesystem work, text capture, or extra redraws occur on event/render
//! paths. The bounded trace is written once, after the application loop exits.

use rio_window::window::WindowId;
use serde::Serialize;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

const MAX_EVENTS: usize = 32_768;
const MAX_WINDOWS: usize = 8;
static ACTIVE: AtomicBool = AtomicBool::new(false);
static TRACE: OnceLock<Mutex<Trace>> = OnceLock::new();
static FIXTURE_MARKERS: OnceLock<Vec<Vec<char>>> = OnceLock::new();
static SESSION: Mutex<Option<Session>> = Mutex::new(None);

pub(crate) fn start() -> io::Result<()> {
    let session = Session::start()?;
    *SESSION
        .lock()
        .map_err(|_| io::Error::other("benchmark session failed"))? = session;
    Ok(())
}

/// Both native `exiting` and a returning event loop use the same one-shot owner.
pub(crate) fn finish(clean: bool) -> io::Result<()> {
    let session = SESSION
        .lock()
        .map_err(|_| io::Error::other("benchmark session failed"))?
        .take();
    if let Some(session) = session {
        if !clean {
            ACTIVE.store(false, Ordering::Relaxed);
        }
        session.finish()?;
    }
    Ok(())
}

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum Kind {
    Window,
    Input,
    Resize,
    SearchBegin,
    SearchEnd,
    Present,
    Fixture,
}

#[derive(Serialize)]
struct Event {
    at_ns: u64,
    window: u8,
    kind: Kind,
    a: u32,
    b: u32,
    c: u32,
}

struct Trace {
    start: Instant,
    wall_start_ns: u128,
    windows: Vec<WindowId>,
    events: Vec<Event>,
    overflow: bool,
}

impl Trace {
    fn record(&mut self, window: WindowId, kind: Kind, values: [u32; 3]) {
        if self.events.len() >= MAX_EVENTS {
            self.overflow = true;
            return;
        }
        let index = if let Some(index) = self.windows.iter().position(|id| *id == window)
        {
            index
        } else if self.windows.len() < MAX_WINDOWS {
            self.windows.push(window);
            self.windows.len() - 1
        } else {
            self.overflow = true;
            return;
        };
        self.events.push(Event {
            at_ns: self.start.elapsed().as_nanos().min(u64::MAX as u128) as u64,
            window: index as u8,
            kind,
            a: values[0],
            b: values[1],
            c: values[2],
        });
    }
}

pub(crate) fn active() -> bool {
    ACTIVE.load(Ordering::Relaxed)
}

/// Match only fixed fixture sentinels at column zero; never retain terminal text.
/// The per-run nonce prevents unrelated output from completing a measurement.
pub(crate) fn fixture_marker(row: &[rio_backend::crosswords::square::Square]) -> u32 {
    if !active() || row.first().is_none_or(|cell| cell.c() != 'A') {
        return 0;
    }
    let Some(markers) = FIXTURE_MARKERS.get() else {
        return 0;
    };
    markers.iter().enumerate().fold(0, |mask, (index, marker)| {
        if row.len() >= marker.len()
            && row
                .iter()
                .zip(marker)
                .all(|(cell, expected)| cell.c() == *expected)
        {
            mask | (1 << index)
        } else {
            mask
        }
    })
}

pub(crate) fn record(window: WindowId, kind: Kind, values: [u32; 3]) {
    if !active() {
        return;
    }
    if let Some(trace) = TRACE.get() {
        if let Ok(mut trace) = trace.lock() {
            trace.record(window, kind, values);
        } else {
            // A poisoned observation cannot produce trustworthy evidence.
            ACTIVE.store(false, Ordering::Relaxed);
        }
    }
}

pub(crate) struct SearchSpan(Option<WindowId>);

impl SearchSpan {
    pub(crate) fn new(window: WindowId) -> Self {
        let window = active().then_some(window);
        if let Some(window) = window {
            record(window, Kind::SearchBegin, [0; 3]);
        }
        Self(window)
    }
}

impl Drop for SearchSpan {
    fn drop(&mut self) {
        if let Some(window) = self.0 {
            record(window, Kind::SearchEnd, [0; 3]);
        }
    }
}

/// Completion is explicit: a crash or early return must not resemble a full run.
pub(crate) struct Session {
    path: PathBuf,
}

fn wall_ns() -> io::Result<u128> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_nanos())
        .map_err(|_| io::Error::other("benchmark clock is unavailable"))
}

fn regular_parent(path: &Path) -> io::Result<()> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .ok_or_else(|| io::Error::other("benchmark output needs an existing parent"))?;
    for ancestor in parent.ancestors() {
        if ancestor.as_os_str().is_empty() {
            continue;
        }
        let metadata = fs::symlink_metadata(ancestor)?;
        #[cfg(windows)]
        let linked = {
            use std::os::windows::fs::MetadataExt;
            metadata.file_attributes() & 0x400 != 0
        };
        #[cfg(not(windows))]
        let linked = metadata.file_type().is_symlink();
        if linked || !metadata.is_dir() {
            return Err(io::Error::other("benchmark parent must not traverse links"));
        }
    }
    Ok(())
}

impl Session {
    pub(crate) fn start() -> io::Result<Option<Self>> {
        let Some(path) = std::env::var_os("AUTOMEXIA_BENCHMARK_TRACE") else {
            return Ok(None);
        };
        if cfg!(feature = "native-gui-test-hooks") {
            return Err(io::Error::other(
                "GUI test hooks invalidate benchmark timing",
            ));
        }
        let path = PathBuf::from(path);
        regular_parent(&path)?;
        if path.try_exists()? {
            return Err(io::Error::other("benchmark output must be new"));
        }
        if let Ok(token) = std::env::var("AUTOMEXIA_BENCHMARK_TOKEN") {
            if token.len() != 32 || !token.bytes().all(|b| b.is_ascii_hexdigit()) {
                return Err(io::Error::other("invalid benchmark fixture token"));
            }
            let markers = ["OUTPUT", "KITTY", "ITERM2", "SIXEL"]
                .iter()
                .map(|name| format!("AMX_BENCH_{token}_DONE_{name}").chars().collect())
                .collect();
            FIXTURE_MARKERS
                .set(markers)
                .map_err(|_| io::Error::other("benchmark fixture already initialized"))?;
        }
        let trace = Trace {
            start: Instant::now(),
            wall_start_ns: wall_ns()?,
            windows: Vec::with_capacity(MAX_WINDOWS),
            events: Vec::with_capacity(MAX_EVENTS),
            overflow: false,
        };
        TRACE
            .set(Mutex::new(trace))
            .map_err(|_| io::Error::other("benchmark observation already started"))?;
        ACTIVE.store(true, Ordering::Relaxed);
        Ok(Some(Self { path }))
    }

    pub(crate) fn finish(self) -> io::Result<()> {
        let active = ACTIVE.swap(false, Ordering::Relaxed);
        let trace = TRACE
            .get()
            .ok_or_else(|| io::Error::other("benchmark observation missing"))?
            .lock()
            .map_err(|_| io::Error::other("benchmark observation failed"))?;
        let duration_ns = trace.start.elapsed().as_nanos();
        let wall_end_ns = wall_ns()?;
        #[derive(Serialize)]
        struct Document<'a> {
            schema: u8,
            kind: &'static str,
            complete: bool,
            os: &'static str,
            arch: &'static str,
            optimized: bool,
            wall_start_ns: u128,
            wall_end_ns: u128,
            duration_ns: u128,
            events: &'a [Event],
        }
        let payload = serde_json::to_vec(&Document {
            schema: 1,
            kind: "application-observation",
            complete: active && !trace.overflow,
            os: std::env::consts::OS,
            arch: std::env::consts::ARCH,
            optimized: !cfg!(debug_assertions),
            wall_start_ns: trace.wall_start_ns,
            wall_end_ns,
            duration_ns,
            events: &trace.events,
        })?;
        drop(trace);
        regular_parent(&self.path)?;
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(&self.path)?;
        file.write_all(&payload)?;
        file.write_all(b"\n")?;
        file.sync_all()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn trace() -> Trace {
        Trace {
            start: Instant::now(),
            wall_start_ns: 0,
            windows: Vec::new(),
            events: Vec::new(),
            overflow: false,
        }
    }

    #[test]
    fn overflow_is_incomplete_instead_of_silently_dropping_observations() {
        let mut trace = trace();
        let window = WindowId::from(1_u64);
        for _ in 0..MAX_EVENTS + 1 {
            trace.record(window, Kind::Input, [0; 3]);
        }
        assert_eq!(trace.events.len(), MAX_EVENTS);
        assert!(trace.overflow);
    }

    #[test]
    fn trace_contains_only_numeric_geometry_and_fixed_event_kinds() {
        let mut trace = trace();
        trace.record(WindowId::from(1_u64), Kind::Present, [4, 1200, 800]);
        let value = serde_json::to_value(&trace.events).unwrap();
        assert_eq!(value[0]["kind"], "present");
        assert_eq!(value[0]["window"], 0);
        assert_eq!(value[0]["a"], 4);
        assert_eq!(value[0].as_object().unwrap().len(), 6);
    }
}
