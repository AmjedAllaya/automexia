//! DwmFlush-driven vsync worker that drives all window repaints.
//!
//! Mirrors the macOS CVDisplayLink model: `Window::request_redraw`
//! sets a per-window `Arc<AtomicBool>` dirty flag and wakes the parked
//! worker, which is the single source of frame timing. The worker sleeps
//! completely while idle; per active composition cycle it iterates the
//! window registry and, for each window where
//! `dirty || should_present_after_input`, fires
//! `RedrawWindow(.., RDW_INVALIDATE)`. The app's `WM_PAINT` /
//! `RedrawRequested` path is unchanged.
//!
//! `should_present_after_input` keeps the loop firing for one
//! second *after a high-rate input burst* (≥ 60 events/sec over
//! a 100 ms window) even if the app never sets the dirty flag —
//! same gate macOS / Wayland / X11 use via `InputRateTracker`.
//! Single keystrokes / lone mouse events no longer force a
//! 1-second redraw storm.
//!
//! When DWM is disabled, the monitor is unplugged, or under some
//! RDP modes, `DwmFlush` returns immediately. The 1 ms threshold
//! catches that and we fall back to `thread::sleep` at the queried
//! refresh interval. Same heuristic as zed's
//! `gpui_windows/src/vsync.rs`.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, RwLock};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use crate::platform_impl::input_rate::InputRateTracker;

use windows_sys::Win32::Foundation::{HWND, S_OK};
use windows_sys::Win32::Graphics::Dwm::{
    DwmFlush, DwmGetCompositionTimingInfo, DWM_TIMING_INFO,
};
use windows_sys::Win32::Graphics::Gdi::{RedrawWindow, RDW_INVALIDATE};
use windows_sys::Win32::System::Performance::QueryPerformanceFrequency;
use windows_sys::Win32::UI::WindowsAndMessaging::IsWindowVisible;

const VSYNC_INTERVAL_THRESHOLD: Duration = Duration::from_millis(1);
const DEFAULT_VSYNC_INTERVAL: Duration = Duration::from_micros(16_666); // ~60Hz

/// State shared between the event loop, window-callback thread,
/// and the DwmFlush worker thread. Holds the per-window dirty-flag
/// registry plus the rate-gated post-input sustain tracker.
pub(crate) struct VSyncSharedState {
    /// HWND (as `usize` for `Hash`/`Eq`) → per-window dirty flag.
    /// `Window::request_redraw` sets the flag; the worker reads
    /// and clears it on each tick.
    windows: RwLock<HashMap<usize, Arc<AtomicBool>>>,
    /// Rate-gated post-input sustain. Only sustained high-rate
    /// input (≥ 60 events/sec over 100 ms) keeps the worker fanning
    /// out redraws after the app stops marking windows dirty.
    /// `Mutex` because the worker thread and the window-message
    /// thread both touch this. See `platform_impl::input_rate`.
    input_rate_tracker: Mutex<InputRateTracker>,
    /// Event-driven idle gate for the vsync worker. A completely idle
    /// terminal parks the thread instead of calling DwmFlush and rebuilding a
    /// window snapshot every monitor refresh. Redraw/input requests wake it;
    /// high-rate input then keeps it running until the sustain window ends.
    work_pending: Mutex<bool>,
    work_ready: Condvar,
}

impl VSyncSharedState {
    pub(crate) fn new() -> Arc<Self> {
        Arc::new(Self {
            windows: RwLock::new(HashMap::new()),
            input_rate_tracker: Mutex::new(InputRateTracker::new()),
            work_pending: Mutex::new(false),
            work_ready: Condvar::new(),
        })
    }

    /// Insert a window and return its dirty-flag handle. The
    /// caller (the `Window` constructor) keeps the returned `Arc`
    /// so `request_redraw` can flip it without taking the registry
    /// lock.
    pub(crate) fn register_window(&self, hwnd: HWND) -> Arc<AtomicBool> {
        let flag = Arc::new(AtomicBool::new(false));
        self.windows
            .write()
            .unwrap()
            .insert(hwnd as usize, flag.clone());
        flag
    }

    pub(crate) fn unregister_window(&self, hwnd: HWND) {
        self.windows.write().unwrap().remove(&(hwnd as usize));
    }

    pub(crate) fn window_count(&self) -> usize {
        self.windows.read().unwrap().len()
    }

    #[inline]
    pub(crate) fn mark_input_received(&self) {
        let entered_high_rate = self.input_rate_tracker.lock().unwrap().record_input();
        if entered_high_rate {
            self.signal_work();
        }
    }

    #[inline]
    pub(crate) fn should_present_after_input(&self) -> bool {
        self.input_rate_tracker.lock().unwrap().is_high_rate()
    }

    #[inline]
    pub(crate) fn mark_redraw_requested(&self, flag: &AtomicBool) {
        flag.store(true, Ordering::Release);
        self.signal_work();
    }

    fn signal_work(&self) {
        let mut pending = self.work_pending.lock().unwrap();
        *pending = true;
        self.work_ready.notify_one();
    }

    /// Park until a redraw/input arrives or shutdown is requested.
    fn wait_for_work(&self, stop: &AtomicBool) -> bool {
        let mut pending = self.work_pending.lock().unwrap();
        while !*pending && !stop.load(Ordering::Acquire) {
            pending = self.work_ready.wait(pending).unwrap();
        }
        *pending = false;
        !stop.load(Ordering::Acquire)
    }
}

/// Owns the worker thread. Drop signals stop and joins.
pub(super) struct VSyncThread {
    stop: Arc<AtomicBool>,
    state: Arc<VSyncSharedState>,
    handle: Option<JoinHandle<()>>,
}

impl VSyncThread {
    pub(super) fn spawn(state: Arc<VSyncSharedState>) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let stop_worker = stop.clone();
        let worker_state = state.clone();

        let handle = std::thread::Builder::new()
            .name("rio-window::vsync".to_owned())
            .spawn(move || {
                let provider = VSyncProvider::new();
                let mut snapshot: Vec<(usize, Arc<AtomicBool>)> = Vec::new();
                while !stop_worker.load(Ordering::Acquire) {
                    if !worker_state.should_present_after_input()
                        && !worker_state.wait_for_work(&stop_worker)
                    {
                        break;
                    }
                    provider.wait_for_vsync();
                    if stop_worker.load(Ordering::Acquire) {
                        break;
                    }

                    let present_after_input = worker_state.should_present_after_input();

                    // Snapshot HWND + flag pairs so we don't hold
                    // the registry lock across `RedrawWindow`. Reuse the
                    // allocation across frames and bursts.
                    snapshot.clear();
                    snapshot.extend(
                        worker_state
                            .windows
                            .read()
                            .unwrap()
                            .iter()
                            .map(|(&hwnd, flag)| (hwnd, flag.clone())),
                    );

                    for (hwnd_bits, flag) in &snapshot {
                        let was_dirty = flag.swap(false, Ordering::AcqRel);
                        if !(was_dirty || present_after_input) {
                            continue;
                        }
                        let hwnd = *hwnd_bits as HWND;
                        // SAFETY: `IsWindowVisible` and
                        // `RedrawWindow` are documented thread-safe.
                        unsafe {
                            if IsWindowVisible(hwnd) != 0 {
                                RedrawWindow(
                                    hwnd,
                                    std::ptr::null(),
                                    std::ptr::null_mut(),
                                    RDW_INVALIDATE,
                                );
                            }
                        }
                    }
                }
            })
            .expect("failed to spawn rio-window vsync thread");

        Self {
            stop,
            state,
            handle: Some(handle),
        }
    }
}

impl Drop for VSyncThread {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        self.state.signal_work();
        if let Some(handle) = self.handle.take() {
            // Worker exits at the start of the next iteration after
            // the current DwmFlush returns (typically <16 ms).
            let _ = handle.join();
        }
    }
}

struct VSyncProvider {
    interval: Duration,
}

impl VSyncProvider {
    fn new() -> Self {
        let interval = query_dwm_interval().unwrap_or(DEFAULT_VSYNC_INTERVAL);
        Self { interval }
    }

    fn wait_for_vsync(&self) {
        let start = Instant::now();
        let hr = unsafe { DwmFlush() };
        let elapsed = start.elapsed();
        if hr != S_OK || elapsed < VSYNC_INTERVAL_THRESHOLD {
            std::thread::sleep(self.interval);
        }
    }
}

fn query_dwm_interval() -> Option<Duration> {
    let mut frequency: i64 = 0;
    if unsafe { QueryPerformanceFrequency(&mut frequency) } == 0 || frequency <= 0 {
        return None;
    }
    let qpc_per_second = frequency as u64;

    let mut info: DWM_TIMING_INFO = unsafe { std::mem::zeroed() };
    info.cbSize = std::mem::size_of::<DWM_TIMING_INFO>() as u32;
    if unsafe { DwmGetCompositionTimingInfo(std::ptr::null_mut(), &mut info) } != S_OK {
        return None;
    }

    let interval = ticks_to_duration(info.qpcRefreshPeriod, qpc_per_second);
    if interval >= VSYNC_INTERVAL_THRESHOLD {
        return Some(interval);
    }
    if info.rateRefresh.uiNumerator == 0 {
        return None;
    }
    Some(ticks_to_duration(
        info.rateRefresh.uiDenominator as u64,
        info.rateRefresh.uiNumerator as u64,
    ))
}

#[inline]
fn ticks_to_duration(counts: u64, ticks_per_second: u64) -> Duration {
    let ticks_per_microsecond = (ticks_per_second / 1_000_000).max(1);
    Duration::from_micros(counts / ticks_per_microsecond)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redraw_request_marks_the_window_and_wakes_the_idle_worker() {
        let state = VSyncSharedState::new();
        let window_flag = AtomicBool::new(false);

        state.mark_redraw_requested(&window_flag);

        assert!(window_flag.load(Ordering::Acquire));
        assert!(*state.work_pending.lock().unwrap());
    }
}
