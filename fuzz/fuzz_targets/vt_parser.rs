#![no_main]

use libfuzzer_sys::fuzz_target;
use rio_vt::ansi::CursorShape;
use rio_vt::crosswords::{Crosswords, CrosswordsSize};
use rio_vt::event::{VoidListener, WindowId};
use rio_vt::performer::handler::Processor;

fuzz_target!(|data: &[u8]| {
    let mut terminal = Crosswords::new(
        CrosswordsSize::new(80, 24),
        CursorShape::Block,
        VoidListener {},
        WindowId::from(0),
        0,
        1_000,
    );
    Processor::default().advance(&mut terminal, data);
});
