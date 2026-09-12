//! Public configuration paths and independent channel/grammar oracles.
use rio_vt::config::colors::{
    hex_to_color_arr, hex_to_color_wgpu, ColorBuilder, Colors, Format,
};
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

#[test]
fn allocation_counter_detects_temporary_buffers() {
    ALLOCATIONS.set(0);
    TRACK.set(true);
    let buffer = black_box(vec![0_u8; black_box(32)]);
    TRACK.set(false);
    assert!(ALLOCATIONS.get() > 0);
    black_box(buffer);
}
// Test-only forwarding preserves System's exact pointer/layout contract.
// Thread-local counters exclude unrelated parallel test activity.
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

#[test]
fn unicode_digit_colour_is_rejected_without_panicking() {
    // Six scalars but eight UTF-8 bytes: the old regex accepts this before
    // alpha radix conversion panics on the trailing Arabic digit.
    assert!(
        ColorBuilder::from_hex("0000\u{660}\u{660}".into(), Format::SRGB0_1).is_err()
    );
    assert!(serde_json::from_str::<Colors>(r#"{"cursor":"0000\u0660\u0660"}"#).is_err());
}

#[test]
fn colour_grammar_rejects_malformed_and_oversized_values() {
    for value in [
        "",
        "#",
        "abc",
        "00000",
        "0000000",
        "000000000",
        "##123456",
        "123#456",
        "000000#",
        " 123456",
        "123456\n",
        "12345g",
        "\0abcde",
        "１２３４５６",
        "a\u{301}bcdef",
        "\u{202e}123456",
        "🦀12345",
    ] {
        assert!(ColorBuilder::from_hex(value.into(), Format::SRGB0_1).is_err());
    }
    assert!(ColorBuilder::from_hex("f".repeat(1_048_576), Format::SRGB0_1).is_err());
    assert_eq!(hex_to_color_arr("invalid"), [0.0, 0.0, 0.0, 1.0]);
}

#[test]
fn all_channel_values_preserve_exact_rgb_rgba_conversions() {
    for channel in 0..=255u8 {
        let channels = [channel, 255 - channel, channel ^ 0x5a];
        for (text, alpha) in [
            (
                format!("#{:02X}{:02x}{:02X}", channels[0], channels[1], channels[2]),
                1.0,
            ),
            (
                format!(
                    "{:02x}{:02X}{:02x}{channel:02X}",
                    channels[0], channels[1], channels[2]
                ),
                f64::from(channel) / 255.0,
            ),
        ] {
            for (format, divisor) in [(Format::SRGB0_1, 255.0), (Format::SRGB0_255, 1.0)]
            {
                let actual = ColorBuilder::from_hex(text.clone(), format).unwrap();
                assert_eq!(
                    [actual.red, actual.green, actual.blue, actual.alpha],
                    [
                        f64::from(channels[0]) / divisor,
                        f64::from(channels[1]) / divisor,
                        f64::from(channels[2]) / divisor,
                        alpha
                    ]
                );
            }
        }
    }
}

#[test]
fn serde_palette_preserves_exact_values_and_defaults() {
    let colors: Colors = serde_json::from_str(
        r##"{"background":"#12345678","cursor":"aAbBcC","dim-blue":"#01020304"}"##,
    )
    .unwrap();
    assert_eq!(
        colors.background.0,
        [
            (18.0 / 255.0) as f32,
            (52.0 / 255.0) as f32,
            (86.0 / 255.0) as f32,
            (120.0 / 255.0) as f32
        ]
    );
    assert_eq!(colors.background.1.r, 18.0 / 255.0);
    assert_eq!(colors.background.1.a, 120.0 / 255.0);
    assert_eq!(
        colors.cursor,
        [
            (170.0 / 255.0) as f32,
            (187.0 / 255.0) as f32,
            (204.0 / 255.0) as f32,
            1.0
        ]
    );
    assert_eq!(
        colors.dim_blue,
        Some([
            (1.0 / 255.0) as f32,
            (2.0 / 255.0) as f32,
            (3.0 / 255.0) as f32,
            (4.0 / 255.0) as f32
        ])
    );
    assert_eq!(colors.foreground, [1.0; 4]);
}

#[test]
fn colour_helpers_and_defaults_do_not_allocate() {
    struct Reset;
    impl Drop for Reset {
        fn drop(&mut self) {
            TRACK.set(false);
        }
    }
    ALLOCATIONS.set(0);
    TRACK.set(true);
    let reset = Reset;
    // Include first use, so a lazily initialized regex cannot hide its cost.
    for _ in 0..32 {
        black_box(Colors::default());
        black_box(hex_to_color_arr(black_box("#12345678")));
        black_box(hex_to_color_wgpu(black_box("aAbBcC")));
    }
    drop(reset);
    assert_eq!(ALLOCATIONS.get(), 0);
}

proptest::proptest! {
    #![proptest_config(proptest::test_runner::Config {
        cases: 64,
        rng_seed: proptest::test_runner::RngSeed::Fixed(0x434f_4c4f_5552),
        failure_persistence: Some(Box::new(proptest::test_runner::FileFailurePersistence::Direct(
            "tests/proptest-regressions/color_conversion.txt",
        ))),
        ..proptest::test_runner::Config::default()
    })]
    #[test]
    fn arbitrary_unicode_colour_never_panics(value in ".{0,32}") {
        let grammar = regex::Regex::new(r"\A#?[0-9a-fA-F]{6}(?:[0-9a-fA-F]{2})?\z").unwrap();
        let accepted = grammar.is_match(&value);
        let result = ColorBuilder::from_hex(value, Format::SRGB0_1);
        proptest::prop_assert_eq!(result.is_ok(), accepted);
    }
}
