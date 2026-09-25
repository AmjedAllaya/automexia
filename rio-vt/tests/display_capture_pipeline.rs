use rio_vt::{
    ansi::CursorShape,
    crosswords::{
        pos::{Column, Line, Pos},
        Crosswords, CrosswordsSize,
    },
    event::{VoidListener, WindowId},
    performer::handler::Processor,
};
fn terminal(columns: usize) -> Crosswords<VoidListener> {
    Crosswords::new(
        CrosswordsSize::new(columns, 64),
        CursorShape::Block,
        VoidListener {},
        WindowId::from(0),
        0,
        2000,
    )
}
fn read(term: &Crosswords<VoidListener>, last_row: i32) -> String {
    term.bounds_to_display_string_bounded(
        Pos::new(Line(0), Column(0)),
        Pos::new(Line(last_row), Column(term.columns() - 1)),
        64 * 1024,
    )
    .unwrap()
}

#[test]
fn inline_pipeline_display_preserves_blank_soft_wrap_segments() {
    for columns in [5, 8, 16] {
        for gap in [columns - 1, columns, columns + 1, columns * 3] {
            let value = format!("A{}Z", " ".repeat(gap));
            let mut term = terminal(columns);
            Processor::default().advance(&mut term, format!("{value}\r\n").as_bytes());
            let before = term.cursor();
            assert_eq!(read(&term, ((value.len() - 1) / columns) as i32), value);
            assert_eq!(term.cursor(), before);
        }
    }
}

#[test]
fn inline_pipeline_display_does_not_join_hard_newlines() {
    let mut term = terminal(16);
    Processor::default().advance(&mut term, b"HEADER\r\n      TAIL\r\n");
    assert_eq!(read(&term, 1), "HEADER\n      TAIL");
}

#[test]
fn inline_pipeline_display_expands_actual_custom_tab_stops_without_changing_copy() {
    let mut term = terminal(32);
    Processor::default().advance(&mut term, b"\x1b[3g\x1b[7G\x1bH\rA\tB\r\n");
    let start = Pos::new(Line(0), Column(0));
    let end = Pos::new(Line(0), Column(31));
    let before = term.bounds_to_string(start, end);
    assert_eq!(read(&term, 0), "A     B");
    assert_eq!(term.bounds_to_string(start, end), before);
}

#[test]
fn inline_pipeline_display_rejects_invalid_ranges_and_caps_expanded_bytes() {
    let mut term = terminal(8);
    Processor::default().advance(&mut term, "éééééééé\r\n".as_bytes());
    let start = Pos::new(Line(0), Column(0));
    assert!(term
        .bounds_to_display_string_bounded(start, Pos::new(Line(0), Column(7)), 8)
        .is_err());
    assert!(term
        .bounds_to_display_string_bounded(start, Pos::new(Line(0), Column(8)), 100)
        .is_err());
    assert!(term
        .bounds_to_display_string_bounded(start, Pos::new(Line(1000), Column(0)), 100)
        .is_err());
    assert!(term
        .bounds_to_display_string_bounded(Pos::new(Line(1), Column(0)), start, 100)
        .is_err());
    assert_eq!(
        term.bounds_to_display_string_bounded(start, Pos::new(Line(0), Column(7)), 16)
            .unwrap(),
        "éééééééé"
    );
}

#[test]
fn inline_pipeline_display_preserves_combining_and_wide_characters() {
    let mut term = terminal(16);
    Processor::default().advance(&mut term, "A界e\u{301}   Z\r\n".as_bytes());
    assert_eq!(read(&term, 0), "A界e\u{301}   Z");
}

#[test]
fn inline_pipeline_display_discards_only_trailing_blank_lines_and_padding() {
    let mut term = terminal(16);
    Processor::default().advance(&mut term, b"\r\nA   \r\n\r\nB\r\n\r\n");
    assert_eq!(read(&term, 5), "\nA\n\nB");
}
