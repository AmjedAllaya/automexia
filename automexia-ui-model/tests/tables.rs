use automexia_ui_model::tables::{Table, TableError, TableViewport};

fn cells(text: &str) -> usize {
    // Independent fixture widths; the native adapter supplies terminal widths.
    text.chars()
        .map(|c| match c {
            '界' | '語' => 2,
            '\u{301}' => 0,
            _ => 1,
        })
        .sum()
}

fn pod_rows() -> Vec<String> {
    [
        "NAME       READY  STATUS",
        "api        1/1    Running",
        "batch      0/1    Completed",
    ]
    .map(String::from)
    .into()
}

#[test]
fn table_columns_keep_original_gaps_and_source_order() {
    let rows = pod_rows();
    let table = Table::detect(rows.clone(), cells).unwrap();
    assert_eq!(table.source(), rows);
    assert_eq!(table.column_starts(), &[0, 11, 18]);
    assert_eq!(table.width(), 27);
    assert_eq!(table.separators(), &[8, 17]);
}

#[test]
fn ordinary_prose_single_rows_and_ragged_words_are_not_tables() {
    for rows in [
        vec![],
        vec!["one  row".into()],
        vec!["ordinary prose".into(), "another sentence".into()],
        vec!["one  two".into(), "unaligned  data".into()],
    ] {
        assert_eq!(
            Table::detect(rows, cells).unwrap_err(),
            TableError::NotTable
        );
    }
}

#[test]
fn empty_last_cells_do_not_remove_columns_with_other_evidence() {
    let rows = vec!["NAME  VALUE".into(), "one   data".into(), "two".into()];
    let table = Table::detect(rows.clone(), cells).unwrap();
    assert_eq!(table.source(), rows);
    assert_eq!(table.column_starts(), &[0, 6]);
}

#[test]
fn controls_and_bidi_cannot_become_interactive_table_labels() {
    for value in [
        '\x1b', '\t', '\0', '\u{061c}', '\u{200e}', '\u{200f}', '\u{202e}', '\u{2066}',
        '\u{7f}',
    ] {
        let mut rows = pod_rows();
        rows[1].push(value);
        assert_eq!(
            Table::detect(rows, cells).unwrap_err(),
            TableError::InvalidText
        );
    }
}

#[test]
fn source_limits_are_checked_before_column_detection() {
    assert_eq!(
        Table::detect(vec!["a  b".into(); 257], cells).unwrap_err(),
        TableError::Capacity
    );
    assert_eq!(
        Table::detect(vec!["x".repeat(4097); 2], cells).unwrap_err(),
        TableError::Capacity
    );
    assert_eq!(
        Table::detect(vec!["界".repeat(2049); 2], cells).unwrap_err(),
        TableError::Capacity
    );
    assert_eq!(
        Table::detect(vec!["x".repeat(2048); 129], cells).unwrap_err(),
        TableError::Capacity
    );
}

#[test]
fn cell_slices_never_wrap_or_split_wide_and_combining_graphemes() {
    let table =
        Table::detect(vec!["界e\u{301}  A".into(), "語b   B".into()], cells).unwrap();
    let slice = table.visible_range(0, 1, 3, cells).unwrap();
    assert_eq!(&table.source()[0][slice.bytes], "e\u{301} ");
    assert_eq!(slice.leading_cells, 1);
    assert!(table.visible_range(0, 0, 1, cells).is_none());
    assert!(table.visible_range(0, 99, 8, cells).is_none());
    assert!(table.visible_range(99, 0, 8, cells).is_none());
    assert!(table.visible_range(0, 0, 0, cells).is_none());
}

#[test]
fn every_horizontal_position_can_reach_the_last_column_without_mutation() {
    let table = Table::detect(pod_rows(), cells).unwrap();
    for width in [1, 2, 3, 8, 26, 27, 28, 512] {
        let mut view = TableViewport::default();
        view.fit(table.width(), table.source().len(), width, 2);
        view.scroll(isize::MAX, isize::MAX);
        assert_eq!(view.column(), table.width().saturating_sub(width));
        assert_eq!(view.row(), 1);
        assert!(!view.scroll(1, 1));
        view.scroll(isize::MIN, isize::MIN);
        assert_eq!((view.column(), view.row()), (0, 0));
        assert!(!view.scroll(-1, -1));
    }
    assert_eq!(table.source(), pod_rows());
}

#[test]
fn viewport_resize_clamps_offsets_and_empty_geometry_is_safe() {
    let mut view = TableViewport::default();
    view.fit(200, 100, 10, 5);
    assert!(view.scroll(150, 80));
    view.fit(200, 100, 1, 1);
    assert_eq!((view.column(), view.row()), (150, 80));
    view.fit(200, 100, 300, 200);
    assert_eq!((view.column(), view.row()), (0, 0));
    view.fit(0, 0, 0, 0);
    assert!(!view.scroll(isize::MAX, isize::MAX));
    assert_eq!(view.horizontal_thumb(100.0), None);
}

#[test]
fn scrollbar_is_bounded_and_indicates_both_horizontal_extremes() {
    let mut view = TableViewport::default();
    view.fit(100, 10, 10, 5);
    let first = view.horizontal_thumb(200.0).unwrap();
    assert_eq!(first, (0.0, 20.0));
    view.scroll(90, 0);
    assert_eq!(view.horizontal_thumb(200.0), Some((180.0, 20.0)));
    for track in [0.0, -1.0, f32::NAN, f32::INFINITY] {
        assert_eq!(view.horizontal_thumb(track), None);
    }
}

#[test]
fn scrollbars_and_ranges_remain_bounded_through_repeated_extreme_sizes() {
    let table = Table::detect(pod_rows(), cells).unwrap();
    let mut view = TableViewport::default();
    for width in (1..=512).chain((1..=512).rev()) {
        view.fit(table.width(), 3, width, 2);
        view.scroll(3, 1);
        if let Some((start, len)) = view.horizontal_thumb(width as f32) {
            assert!(start >= 0.0 && len > 0.0);
            assert!(start + len <= width as f32 + f32::EPSILON * 512.0);
        }
        for row in 0..3 {
            if let Some(slice) = table.visible_range(row, view.column(), width, cells) {
                let text = &table.source()[row][slice.bytes];
                assert!(slice.leading_cells + cells(text) <= width);
                assert!(!text.contains('\n'));
            }
        }
    }
}
