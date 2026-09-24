//! Dispatch allocation evidence excludes the already-bounded OSC input buffer.
use rio_vt::ansi::CursorShape;
use rio_vt::crosswords::{Crosswords, CrosswordsSize};
use rio_vt::event::{VoidListener, WindowId};
use rio_vt::performer::handler::Processor;
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

struct CountAllocations;
thread_local! {
    static RECORD: Cell<bool> = const { Cell::new(false) };
    static CALLS: Cell<usize> = const { Cell::new(0) };
}
fn record() {
    let _ = RECORD.try_with(|enabled| {
        if enabled.get() {
            let _ = CALLS.try_with(|calls| calls.set(calls.get() + 1));
        }
    });
}
// SAFETY: all allocation and deallocation operations are forwarded unchanged
// to System. Thread-local counters neither allocate nor inspect the payload.
unsafe impl GlobalAlloc for CountAllocations {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        record();
        // SAFETY: preserve the allocator caller's layout contract.
        unsafe { System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        record();
        // SAFETY: preserve the allocator caller's layout contract.
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        record();
        // SAFETY: forward the original allocation and requested layout unchanged.
        unsafe { System.realloc(ptr, layout, size) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: forward the allocation/layout pair to its original allocator.
        unsafe { System.dealloc(ptr, layout) }
    }
}
#[global_allocator]
static ALLOCATOR: CountAllocations = CountAllocations;

fn measured(action: impl FnOnce()) -> usize {
    CALLS.with(|calls| calls.set(0));
    RECORD.with(|record| record.set(true));
    action();
    RECORD.with(|record| record.set(false));
    CALLS.with(Cell::get)
}

#[test]
fn user_var_bounds_allocation_counter_has_a_positive_control() {
    let count = measured(|| {
        std::hint::black_box(vec![1u8; 4096]);
    });
    assert!(count > 0);
}

#[test]
fn user_var_bounds_oversized_name_and_encoding_are_rejected_before_decode_allocation() {
    for body in [
        format!("\x1b]1337;SetUserVar={}={}", "key", "A".repeat(10_925)),
        format!("\x1b]1337;SetUserVar={}=eA==", "n".repeat(129)),
    ] {
        let mut terminal = Crosswords::new(
            CrosswordsSize::new(80, 8),
            CursorShape::Block,
            VoidListener {},
            WindowId::from(0),
            0,
            128,
        );
        let mut processor = Processor::default();
        // Buffer first; record only termination and OSC dispatch allocations.
        processor.advance(&mut terminal, body.as_bytes());
        let allocations = measured(|| {
            processor.advance(&mut terminal, b"\x07");
        });
        assert_eq!(
            allocations, 0,
            "oversized metadata reached an allocating decoder"
        );
        assert!(terminal.user_vars.is_empty());
    }
}

#[test]
fn user_var_chronology_reads_and_raw_overflow_rejection_do_not_allocate() {
    let mut terminal = Crosswords::new(
        CrosswordsSize::new(80, 8),
        CursorShape::Block,
        VoidListener {},
        WindowId::from(0),
        0,
        128,
    );
    let mut processor = Processor::default();
    processor.advance(&mut terminal, b"\x1b]1337;SetUserVar=known=b2xk\x07");
    assert!(terminal.user_var_write_stamp("known").is_some());
    let reads = measured(|| {
        for _ in 0..512 {
            std::hint::black_box(terminal.user_var_write_stamp("known"));
            std::hint::black_box(terminal.user_var_write_stamp("unknown"));
            std::hint::black_box(terminal.user_var_chronology_valid());
            std::hint::black_box(terminal.last_user_var_rejection());
        }
    });
    assert_eq!(reads, 0);
    for prefix in [
        b"\x1b]1337;SetUserVar=known=".as_slice(),
        b"\x1b]0;".as_slice(),
    ] {
        let mut body = prefix.to_vec();
        body.extend(vec![b'A'; 1024 * 1024 + 64]);
        processor.advance(&mut terminal, &body);
        let allocations = measured(|| {
            processor.advance(&mut terminal, b"\x07");
        });
        assert_eq!(allocations, 0, "overflow notification allocated a buffer");
    }
    assert_eq!(
        terminal
            .last_user_var_rejection()
            .map(std::num::NonZeroU64::get),
        Some(2)
    );
    assert_eq!(terminal.user_vars["known"], "old");
}

#[test]
fn user_var_chronology_quota_rejection_does_not_allocate_tracking() {
    let mut terminal = Crosswords::new(
        CrosswordsSize::new(80, 8),
        CursorShape::Block,
        VoidListener {},
        WindowId::from(0),
        0,
        128,
    );
    let mut processor = Processor::default();
    for i in 0..128 {
        let frame = format!("\x1b]1337;SetUserVar=k{i:03}=eA==\x07");
        processor.advance(&mut terminal, frame.as_bytes());
    }
    let name = "overflow".to_string();
    let value = "value".to_string();
    let rejected = measured(|| {
        rio_vt::performer::handler::Handler::set_user_var(&mut terminal, name, value);
    });
    assert_eq!(rejected, 0);
    assert_eq!(
        terminal
            .last_user_var_rejection()
            .map(std::num::NonZeroU64::get),
        Some(129)
    );
    let name = "k000".to_string();
    let value = "updated".to_string();
    let replacement = measured(|| {
        rio_vt::performer::handler::Handler::set_user_var(&mut terminal, name, value);
    });
    assert_eq!(replacement, 0);
    let stamp = terminal
        .user_var_write_stamp("k000")
        .expect("replacement is tracked");
    assert_eq!(stamp.latest.get(), 130);
    assert_eq!(stamp.previous.map(std::num::NonZeroU64::get), Some(1));
}
