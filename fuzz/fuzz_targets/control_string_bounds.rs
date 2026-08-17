#![no_main]

use libfuzzer_sys::fuzz_target;
use rio_vt::ansi::CursorShape;
use rio_vt::crosswords::{Crosswords, CrosswordsSize};
use rio_vt::event::{VoidListener, WindowId};
use rio_vt::performer::handler::Processor;

fuzz_target!(|data: &[u8]| {
    let selector = data.first().copied().unwrap_or_default() % 3;
    let chunk_width = usize::from(data.get(1).copied().unwrap_or(31)) + 1;
    let seed = data
        .get(2..)
        .filter(|seed| !seed.is_empty())
        .unwrap_or(b"A");
    let (prefix, retained_limit) = match selector {
        0 => (b"\x1b]52;s;".as_slice(), 1024 * 1024),
        1 => (b"\x1b_25a1;s;".as_slice(), 96 * 1024),
        _ => (b"\x1bP+q".as_slice(), 4 * 1024),
    };

    let mut stream = Vec::with_capacity(retained_limit + 512);
    stream.extend_from_slice(prefix);
    stream.extend(
        seed.iter()
            .cycle()
            .take(retained_limit + 257)
            .map(|byte| b'A' + (byte % 26)),
    );
    stream.extend_from_slice(b"\x1b\\");
    // Every oversized case is followed by ordinary text and a valid OSC. A
    // parser stuck in discard mode, or one retaining stale parameter offsets,
    // will be exercised immediately in the same iteration.
    stream.extend_from_slice(b"recovered\r\n\x1b]2;Automexia recovered\x07");

    let mut terminal = Crosswords::new(
        CrosswordsSize::new(80, 24),
        CursorShape::Block,
        VoidListener {},
        WindowId::from(0),
        0,
        1_000,
    );
    let mut processor = Processor::default();
    for chunk in stream.chunks(chunk_width) {
        processor.advance(&mut terminal, chunk);
    }
});
