#![no_main]

use libfuzzer_sys::fuzz_target;
use rio_vt::ansi::CursorShape;
use rio_vt::crosswords::pos::Line;
use rio_vt::crosswords::{Crosswords, CrosswordsSize};
use rio_vt::event::{VoidListener, WindowId};
use rio_vt::performer::handler::Processor;

const MAX_INPUT_BYTES: usize = 4096;

fn terminal() -> Crosswords<VoidListener> {
    Crosswords::new(
        CrosswordsSize::new(80, 24),
        CursorShape::Block,
        VoidListener {},
        WindowId::from(0),
        0,
        1_000,
    )
}

fn assert_user_variable_bounds(terminal: &Crosswords<VoidListener>) {
    assert!(terminal.user_vars.len() <= 128);
    let mut bytes = 0usize;
    for (name, value) in &terminal.user_vars {
        assert!(!name.is_empty() && name.len() <= 128);
        assert!(value.len() <= 8192);
        bytes += name.len() + value.len();
    }
    assert!(bytes <= 64 * 1024);
}

fuzz_target!(|data: &[u8]| {
    // Preserve the existing whole-input coverage, including large inputs.
    let mut whole = terminal();
    Processor::default().advance(&mut whole, data);
    assert_user_variable_bounds(&whole);

    // Bound only the additional fragmentation work. Large inputs get a fresh
    // prefix reference so the comparison always concerns identical bytes.
    if data.len() > MAX_INPUT_BYTES {
        whole = terminal();
        Processor::default().advance(&mut whole, &data[..MAX_INPUT_BYTES]);
    }
    let data = &data[..data.len().min(MAX_INPUT_BYTES)];
    for chunk_size in [1, 7, 31] {
        let mut fragmented = terminal();
        let mut processor = Processor::default();
        for chunk in data.chunks(chunk_size) {
            processor.advance(&mut fragmented, chunk);
        }

        assert_user_variable_bounds(&fragmented);

        // Ground-state text/control semantics must not depend on read sizes.
        // OSC lifecycle timestamps and synchronized-update deadlines are not
        // compared; exact framed-protocol oracles live in the owner unit tests.
        if !data.contains(&0x1b) {
            assert_eq!(fragmented.grid.cursor.pos, whole.grid.cursor.pos);
            assert_eq!(fragmented.mode(), whole.mode());
            for row in 0..24 {
                let expected = whole.grid[Line(row)].inner.iter().map(|cell| cell.c());
                let actual = fragmented.grid[Line(row)].inner.iter().map(|cell| cell.c());
                assert!(
                    actual.eq(expected),
                    "ground row {row} differs after fragmentation"
                );
            }
        }
    }
});
