use automexia_extension_api::surface::SemanticTable;
use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
};

#[path = "support/surface_accounting.rs"]
mod surface_accounting;
use surface_accounting::Accounting;

// One isolated test binary, with per-thread accounting. Fixture creation and
// harness allocations stay outside the measured decode/replace/drop interval.
struct TrackingAllocator;
thread_local! {
    static ENABLED: Cell<bool> = const { Cell::new(false) };
    static COUNTS: Cell<Accounting> = const { Cell::new(Accounting::new()) };
}
fn record(change: fn(&mut Accounting, usize), size: usize) {
    if ENABLED.try_with(Cell::get).unwrap_or(false) {
        let _ = COUNTS.try_with(|cell| {
            let mut count = cell.get();
            change(&mut count, size);
            cell.set(count);
        });
    }
}
struct MeasuredWindow;
impl MeasuredWindow {
    fn start() -> Self {
        COUNTS.with(|cell| cell.set(Accounting::new()));
        ENABLED.with(|enabled| enabled.set(true));
        Self
    }
}
impl Drop for MeasuredWindow {
    fn drop(&mut self) {
        // Disable even during assertion unwinding, before unmeasured fixtures drop.
        let _ = ENABLED.try_with(|enabled| enabled.set(false));
    }
}
// SAFETY: Every operation forwards the exact pointer/layout to System, and
// accounting neither allocates nor unwinds or changes results/pointer identity.
unsafe impl GlobalAlloc for TrackingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { System.alloc(layout) };
        if !ptr.is_null() {
            record(Accounting::allocate, layout.size());
        }
        ptr
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        record(Accounting::release, layout.size());
        unsafe {
            System.dealloc(ptr, layout);
        }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        let next = unsafe { System.realloc(ptr, layout, size) };
        if !next.is_null() {
            record(Accounting::release, layout.size());
            record(Accounting::allocate, size);
        }
        next
    }
}
#[global_allocator]
static ALLOCATOR: TrackingAllocator = TrackingAllocator;

#[test]
fn failed_measurement_disables_tracking_before_unmeasured_cleanup() {
    let failure = std::panic::catch_unwind(|| {
        let _measured = MeasuredWindow::start();
        panic!("fictional fixture failure");
    });
    assert!(failure.is_err());
    assert!(!ENABLED.with(Cell::get));
    // A later measurement must not inherit failed-path counters.
    let measured = MeasuredWindow::start();
    drop(measured);
    let count = COUNTS.with(Cell::get);
    assert!(count.valid);
    assert_eq!(count.live, 0);
}

#[test]
fn repeated_large_decode_replace_and_failed_decode_release_owned_allocations() {
    let rows = (1..=20_000)
        .map(|id| format!(r#"{{"id":{id},"resource":null,"cells":[{{"text":"item"}}]}}"#))
        .collect::<Vec<_>>()
        .join(",");
    let wire = format!(
        r#"{{"schema":{{"id":"inventory","columns":[{{"id":"name","title":"Name","min_width":1,"preferred_width":16,"max_width":null,"priority":0,"alignment":"left","overflow":"ellipsis","responsive":"always","data_kind":"text"}}],"row_identity":"stable-handle"}},"rows":[{rows}]}}"#
    );
    let broken = wire.replace("\"id\":20000", "\"id\":1");
    let measured = MeasuredWindow::start();
    let mut current = None;
    for _ in 0..12 {
        current = Some(serde_json::from_str::<SemanticTable>(&wire).unwrap());
        assert!(serde_json::from_str::<SemanticTable>(&broken).is_err());
    }
    drop(current);
    drop(measured);
    let count = COUNTS.with(Cell::get);
    assert!(count.valid, "accounting arithmetic must remain valid");
    assert_eq!(
        count.live, 0,
        "no decoded snapshot or error allocation survives drop"
    );
    assert!(
        count.peak < 16 * 1024 * 1024,
        "two snapshots plus validation scratch stay below the fixture ceiling"
    );
    println!(
        "semantic surface fixture: peak_owned_bytes={}, retained_after_drop=0",
        count.peak
    );
}
