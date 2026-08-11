#![no_main]

use libfuzzer_sys::fuzz_target;
use rio_vt::ansi::CursorShape;
use rio_vt::crosswords::{Crosswords, CrosswordsSize};
use rio_vt::event::{VoidListener, WindowId};
use rio_vt::performer::handler::Processor;

fuzz_target!(|payload: &[u8]| {
    let mut terminal = Crosswords::new(
        CrosswordsSize::new(40, 8),
        CursorShape::Block,
        VoidListener {},
        WindowId::from(0),
        0,
        128,
    );
    let mut sequence = b"\x1b]1337;SetUserVar=fuzz=".to_vec();
    sequence.extend_from_slice(payload);
    sequence.push(0x07);
    Processor::default().advance(&mut terminal, &sequence);
});
