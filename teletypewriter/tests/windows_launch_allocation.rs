#![cfg(windows)]
//! Public launch-boundary allocation evidence, independent of builder internals.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::hint::black_box;

struct ObserveAllocations;
thread_local! {
    static TRACKING: Cell<bool> = const { Cell::new(false) };
    static LARGEST: Cell<usize> = const { Cell::new(0) };
}

fn record(size: usize) {
    if TRACKING.try_with(Cell::get).unwrap_or(false) {
        let _ = LARGEST.try_with(|largest| largest.set(largest.get().max(size)));
    }
}

// SAFETY: This test-only observer forwards every pointer, layout and requested
// size unchanged to System. The allocation-free counters are thread-local and
// never take ownership of returned memory or interfere with another test thread.
unsafe impl GlobalAlloc for ObserveAllocations {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        record(layout.size());
        unsafe { System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        record(layout.size());
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        record(size);
        unsafe { System.realloc(pointer, layout, size) }
    }
    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        unsafe { System.dealloc(pointer, layout) }
    }
}

#[global_allocator]
static ALLOCATOR: ObserveAllocations = ObserveAllocations;

struct TrackingGuard;
impl Drop for TrackingGuard {
    fn drop(&mut self) {
        TRACKING.set(false);
    }
}

fn largest_allocation<T>(operation: impl FnOnce() -> T) -> (T, usize) {
    LARGEST.set(0);
    TRACKING.set(true);
    let guard = TrackingGuard;
    let result = operation();
    drop(guard);
    (result, LARGEST.get())
}

#[test]
fn allocation_observer_detects_real_allocations_and_restores_after_unwind() {
    let (_, largest) = largest_allocation(|| {
        black_box(vec![0u8; black_box(8_192)]);
    });
    assert!(largest >= 8_192);
    let _ = std::panic::catch_unwind(|| {
        largest_allocation(|| panic!("fictional allocation fixture unwind"));
    });
    assert!(!TRACKING.get());
}

#[test]
fn oversized_quoted_command_is_rejected_before_expansion_allocations() {
    // Quoting doubles trailing backslashes. The supplied argument is smaller
    // than the native limit; its expanded command exceeds the UTF-16 budget.
    let argument = format!(" {}", "\\".repeat(16_380));
    let arguments = vec![argument];
    let (result, largest) = largest_allocation(|| {
        teletypewriter::create_pty(Some("fixture.exe"), arguments, &None, None, 80, 24)
    });
    let error = result.err().expect("oversized command must fail");
    assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
    assert!(
        largest <= 4_096,
        "oversized command allocated an expanded buffer"
    );
}

#[test]
fn oversized_environment_batch_is_rejected_before_wide_copy_allocations() {
    // Each value fits individually, but their two-entry block exceeds the
    // documented aggregate 16 Mi UTF-16-unit budget even before inheritance.
    let entries = vec![
        ("AMX_A".into(), "x".repeat(8 * 1024 * 1024)),
        ("AMX_B".into(), "y".repeat(8 * 1024 * 1024)),
    ];
    let (result, largest) = largest_allocation(|| {
        teletypewriter::create_pty(None, Vec::new(), &None, Some(entries), 80, 24)
    });
    let error = result.err().expect("oversized environment must fail");
    assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
    assert!(
        largest <= 4_096,
        "oversized environment allocated a wide copy"
    );
}
