//! Independent allocation evidence for the application default palette owner.
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::hint::black_box;

struct CountAllocations;
thread_local! {
    static TRACK: Cell<bool> = const { Cell::new(false) };
    static ALLOCATIONS: Cell<usize> = const { Cell::new(0) };
}
fn record_allocation() {
    if TRACK.try_with(Cell::get).unwrap_or(false) {
        let _ = ALLOCATIONS.try_with(|count| count.set(count.get() + 1));
    }
}
// SAFETY: This test-only adapter forwards each original pointer, layout and
// requested size unchanged to System. Thread-local, allocation-free counters
// observe only the calling test thread; the adapter never owns returned memory.
unsafe impl GlobalAlloc for CountAllocations {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        record_allocation();
        unsafe { System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        record_allocation();
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        unsafe { System.dealloc(pointer, layout) }
    }
    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        record_allocation();
        unsafe { System.realloc(pointer, layout, size) }
    }
}
#[global_allocator]
static ALLOCATOR: CountAllocations = CountAllocations;

struct Tracking;
impl Drop for Tracking {
    fn drop(&mut self) {
        TRACK.set(false);
    }
}
fn allocations(operation: impl FnOnce()) -> usize {
    ALLOCATIONS.set(0);
    TRACK.set(true);
    let guard = Tracking;
    operation();
    drop(guard);
    ALLOCATIONS.get()
}

#[test]
fn counter_detects_allocations_and_resets_after_unwinding() {
    assert!(
        allocations(|| {
            black_box(vec![0u8; black_box(32)]);
        }) > 0
    );
    let _ = std::panic::catch_unwind(|| allocations(|| panic!("fixture unwind")));
    assert!(!TRACK.get());
}

#[test]
fn first_and_repeated_unified_palette_setup_allocate_nothing() {
    std::thread::spawn(|| {
        assert_eq!(
            allocations(|| {
                let palette = rio_backend::config::defaults::unified_colors();
                assert_eq!(
                    palette.foreground,
                    [216.0 / 255.0, 222.0 / 255.0, 233.0 / 255.0, 1.0]
                );
                assert_eq!(
                    palette.background.0,
                    [2.0 / 255.0, 11.0 / 255.0, 22.0 / 255.0, 1.0]
                );
                black_box(palette);
            }),
            0
        );
        assert_eq!(
            allocations(|| {
                for _ in 0..64 {
                    black_box(rio_backend::config::defaults::unified_colors());
                }
            }),
            0
        );
    })
    .join()
    .unwrap();
}
