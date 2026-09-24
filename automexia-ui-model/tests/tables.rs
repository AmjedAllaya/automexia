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

fn narrow_pod_rows() -> Vec<String> {
    [
        "NAME                    READY  STATUS   RESTARTS      AGE",
        "api-8f7c9d6b-x2abc      0/1    Running  94 (17s ago)  21d",
        "worker-54bd781c-z8xyz   1/1    Unknown  28 (1m ago)   2d",
    ]
    .map(String::from)
    .into()
}

#[test]
fn bordered_inline_pods_wrap_inside_aligned_cells_in_a_narrow_pane() {
    use automexia_ui_model::tables::HeaderConfidence;
    let source = narrow_pod_rows();
    let table = Table::detect(source.clone(), cells).unwrap();
    assert_eq!(table.header_confidence(), HeaderConfidence::UppercaseLabels);
    assert_eq!(table.column_starts(), &[0, 24, 31, 40, 54]);
    let wrapped = table.wrap(37, cells).unwrap();
    assert_eq!(wrapped.width, 37);
    assert_eq!(wrapped.columns.len(), 5);
    assert_eq!(
        wrapped
            .columns
            .iter()
            .map(|c| c.content_width)
            .collect::<Vec<_>>(),
        vec![7, 5, 6, 6, 3]
    );
    assert_eq!(
        wrapped.rows.iter().map(|r| r.height).collect::<Vec<_>>(),
        vec![2, 3, 3]
    );
    let restarts = &wrapped.rows[1].cells[3];
    assert_eq!(restarts.source_cells, 40..52);
    assert_eq!(&source[1][restarts.source_bytes.clone()], "94 (17s ago)");
    let fragments = restarts
        .fragments
        .iter()
        .map(|f| &source[1][f.bytes.clone()])
        .collect::<Vec<_>>();
    assert_eq!(fragments, ["94 ", "(17s ", "ago)"]);
    assert_eq!(restarts.fragments[0].source_cells, 40..43);
    assert_eq!(restarts.fragments[1].source_cells, 43..48);
    assert_eq!(table.source(), source);
}

#[test]
fn inline_header_confidence_does_not_turn_every_first_row_into_a_header() {
    use automexia_ui_model::tables::{HeaderConfidence, WrapError};
    for rows in [
        vec!["alpha  running".into(), "beta   stopped".into()],
        vec!["Alpha  Running".into(), "Beta   Stopped".into()],
        vec!["UP     READY".into(), "DOWN   WAIT".into()],
        vec!["NAME   NAME".into(), "one    two".into()],
        vec!["123    456".into(), "789    012".into()],
    ] {
        let table = Table::detect(rows, cells).unwrap();
        assert_eq!(table.header_confidence(), HeaderConfidence::None);
        assert_eq!(
            table.wrap(40, cells).unwrap_err(),
            WrapError::UncertainHeader
        );
    }
    let table = Table::detect(vec!["NAME  PORT(S)".into(), "web   80/TCP".into()], cells)
        .unwrap();
    assert_eq!(table.header_confidence(), HeaderConfidence::UppercaseLabels);
}

#[test]
fn wrapped_table_geometry_shares_borders_and_preserves_each_source_cell() {
    let table = Table::detect(narrow_pod_rows(), cells).unwrap();
    for width in 15..=100 {
        let layout = table.wrap(width, cells).unwrap();
        assert!(layout.width <= width);
        let mut border = 0;
        for column in &layout.columns {
            assert_eq!(column.x, border);
            assert_eq!(column.content_x, column.x + 1);
            assert_eq!(column.width, column.content_width + 2);
            border += column.width;
        }
        assert_eq!(border, layout.width);
        for (row, source) in layout.rows.iter().zip(table.source()) {
            assert_eq!(row.cells.len(), 5);
            assert_eq!(
                row.height,
                row.cells
                    .iter()
                    .map(|cell| cell.fragments.len())
                    .max()
                    .unwrap()
                    .max(1)
            );
            for (cell, column) in row.cells.iter().zip(&layout.columns) {
                let mut text = String::new();
                let mut source_column = cell.source_cells.start;
                let mut source_byte = cell.source_bytes.start;
                for fragment in &cell.fragments {
                    assert_eq!(fragment.bytes.start, source_byte);
                    assert_eq!(fragment.source_cells.start, source_column);
                    let fragment_text = &source[fragment.bytes.clone()];
                    assert!(!fragment_text.is_empty());
                    assert_eq!(cells(fragment_text), fragment.source_cells.len());
                    assert!(fragment.source_cells.len() <= column.content_width);
                    text.push_str(fragment_text);
                    source_column = fragment.source_cells.end;
                    source_byte = fragment.bytes.end;
                }
                assert_eq!(text, source[cell.source_bytes.clone()]);
                assert_eq!(source_column, cell.source_cells.end);
                assert_eq!(source_byte, cell.source_bytes.end);
            }
        }
    }
}

#[test]
fn wrapped_cells_keep_wide_and_combining_graphemes_intact() {
    let table = Table::detect(
        vec!["NAME    VALUE".into(), "界e\u{301}語   x".into()],
        cells,
    )
    .unwrap();
    let layout = table.wrap(7, cells).unwrap();
    assert_eq!(
        layout
            .columns
            .iter()
            .map(|c| c.content_width)
            .collect::<Vec<_>>(),
        [2, 1]
    );
    let value = &layout.rows[1].cells[0];
    let parts = value
        .fragments
        .iter()
        .map(|f| &table.source()[1][f.bytes.clone()])
        .collect::<Vec<_>>();
    assert_eq!(parts, ["界", "e\u{301}", "語"]);
    assert_eq!(
        value
            .fragments
            .iter()
            .map(|f| f.source_cells.clone())
            .collect::<Vec<_>>(),
        [0..2, 2..3, 3..5]
    );
    assert_eq!(layout.rows[1].height, 3);
}

#[test]
fn empty_cells_do_not_inherit_an_adjacent_cells_value_or_height() {
    let table = Table::detect(
        vec![
            "NAME  VALUE  STATE".into(),
            "one   data   ready".into(),
            "two".into(),
        ],
        cells,
    )
    .unwrap();
    let layout = table.wrap(15, cells).unwrap();
    assert!(layout.rows[2].cells[1].fragments.is_empty());
    assert!(layout.rows[2].cells[2].fragments.is_empty());
    assert!(layout.rows[2].cells[1].source_bytes.is_empty());
    assert!(layout.rows[2].cells[1].source_cells.is_empty());
    assert_eq!(layout.rows[2].height, 1);
}

#[test]
fn impossible_geometry_is_explicit_and_does_not_discard_source() {
    use automexia_ui_model::tables::WrapError;
    let source = narrow_pod_rows();
    let table = Table::detect(source.clone(), cells).unwrap();
    for width in [0, 1, 2, 14] {
        assert_eq!(
            table.wrap(width, cells).unwrap_err(),
            WrapError::TooNarrow { minimum_width: 15 }
        );
    }
    assert!(table.wrap(usize::MAX, cells).unwrap().width <= 4096);
    assert_eq!(table.source(), source);
}

#[test]
fn wrapped_output_has_a_total_row_budget() {
    use automexia_ui_model::tables::WrapError;
    let mut source = vec!["NAME                                                                                                                            VALUE".into()];
    source.extend((0..255).map(|_| format!("{}  value", "x".repeat(126))));
    let table = Table::detect(source.clone(), cells).unwrap();
    assert_eq!(table.wrap(6, cells).unwrap_err(), WrapError::Capacity);
    assert_eq!(table.source(), source);
}

#[test]
fn wrapped_output_has_a_separate_fragment_budget() {
    use automexia_ui_model::tables::WrapError;
    let header = (0..64)
        .map(|i| format!("{:<5}", format!("H{i}")))
        .collect::<String>();
    let mut source = vec![header];
    source.extend((0..255).map(|_| "ab   ".repeat(64)));
    let table = Table::detect(source.clone(), cells).unwrap();
    // At one content cell per column, 32,822 fragments exceed the independent
    // budget even though the entire table is only 513 content lines high.
    assert_eq!(table.wrap(192, cells).unwrap_err(), WrapError::Capacity);
    let layout = table.wrap(193, cells).unwrap();
    assert_eq!(layout.rows.iter().map(|row| row.height).sum::<usize>(), 513);
    assert_eq!(table.source(), source);
}

#[test]
fn emoji_clusters_are_not_split_or_remeasured_as_unicode_scalars() {
    fn terminal_cells(value: &str) -> usize {
        match value {
            "\u{1f469}\u{200d}\u{1f4bb}" | "\u{1f1eb}\u{1f1f7}" => 2,
            other => cells(other),
        }
    }
    let table = Table::detect(
        vec![
            "NAME    VALUE".into(),
            "\u{1f469}\u{200d}\u{1f4bb}e\u{301}\u{1f1eb}\u{1f1f7}   x".into(),
        ],
        terminal_cells,
    )
    .unwrap();
    let layout = table.wrap(7, terminal_cells).unwrap();
    let parts = layout.rows[1].cells[0]
        .fragments
        .iter()
        .map(|f| &table.source()[1][f.bytes.clone()])
        .collect::<Vec<_>>();
    assert_eq!(
        parts,
        [
            "\u{1f469}\u{200d}\u{1f4bb}",
            "e\u{301}",
            "\u{1f1eb}\u{1f1f7}"
        ]
    );
}

#[test]
fn inconsistent_width_adapter_fails_without_overflow_or_source_mutation() {
    use automexia_ui_model::tables::WrapError;
    let source = pod_rows();
    let table = Table::detect(source.clone(), cells).unwrap();
    assert_eq!(
        table.wrap(40, |_| usize::MAX).unwrap_err(),
        WrapError::Capacity
    );
    assert_eq!(table.wrap(40, |_| 0).unwrap_err(), WrapError::Capacity);
    assert_eq!(table.source(), source);
}

#[test]
fn generic_header_case_units_and_punctuation_use_data_evidence() {
    for header in [
        "Name      Id    CPU(s)",
        "name      pid   memory.bytes",
        "Image     ID    CPU %",
    ] {
        let source = vec![
            header.into(),
            "alpha     17    1.25".into(),
            "beta      18    0.00".into(),
        ];
        let table = Table::detect(source.clone(), cells).unwrap();
        let wrapped = table.wrap(18, cells).unwrap();
        assert_eq!(wrapped.columns.len(), 3);
        assert_eq!(table.source(), source);
        assert_eq!(
            &source[1][wrapped.rows[1].cells[1].source_bytes.clone()],
            "17"
        );
    }
}

#[test]
fn headed_empty_data_columns_remain_present_and_empty() {
    let source = vec![
        "NAME      PORTS      COUNT".into(),
        "alpha                17".into(),
        "beta                 18".into(),
    ];
    let table = Table::detect(source.clone(), cells).unwrap();
    let wrapped = table.wrap(15, cells).unwrap();
    assert_eq!(wrapped.columns.len(), 3);
    for row in &wrapped.rows[1..] {
        assert!(row.cells[1].source_bytes.is_empty());
        assert!(row.cells[1].fragments.is_empty());
    }
    assert_eq!(table.source(), source);
}

#[test]
fn single_gutter_numeric_output_and_multiword_labels_remain_generic() {
    for source in [
        vec!["PID COMMAND".into(), "17  alpha".into(), "18  beta".into()],
        vec![
            "Process Name    Working Set (MB)".into(),
            "alpha worker    32.5".into(),
            "beta daemon     64.0".into(),
        ],
    ] {
        let table = Table::detect(source.clone(), cells).unwrap();
        let wrapped = table.wrap(14, cells).unwrap();
        assert_eq!(wrapped.columns.len(), 2);
        assert_eq!(table.source(), source);
    }
}

#[test]
fn whitespace_header_rulers_preserve_physical_rows_and_do_not_wrap_rule_text() {
    let source = vec![
        "Handles  NPM(K)  CPU(s)  Id  ProcessName".into(),
        "-------  ------  ------  --  -----------".into(),
        "    125      17    1.25  31  alpha".into(),
        "    230      18    0.00  32  beta".into(),
    ];
    let table = Table::detect(source.clone(), cells).unwrap();
    let wrapped = table.wrap(25, cells).unwrap();
    assert_eq!(wrapped.rows.len(), source.len());
    assert_eq!(wrapped.columns.len(), 5);
    assert!(wrapped.rows[1]
        .cells
        .iter()
        .all(|cell| cell.fragments.is_empty()));
    assert_eq!(
        &source[2][wrapped.rows[2].cells[4].source_bytes.clone()],
        "alpha"
    );
    assert_eq!(table.source(), source);
}

#[test]
fn pipe_markdown_and_box_tables_keep_original_row_and_byte_identity() {
    for source in [
        vec![
            "| name | state |".into(),
            "| :--- | ---: |".into(),
            "| alpha | ready |".into(),
            "| beta | stopped |".into(),
        ],
        vec![
            "name | count".into(),
            "--- | ---".into(),
            "alpha | 1".into(),
            "beta | 22".into(),
        ],
        vec![
            "+-------+-------+".into(),
            "| name  | state |".into(),
            "+-------+-------+".into(),
            "| alpha | ready |".into(),
            "+-------+-------+".into(),
        ],
        vec![
            "\u{250c}\u{2500}\u{2500}\u{2500}\u{252c}\u{2500}\u{2500}\u{2500}\u{2510}"
                .into(),
            "\u{2502}name\u{2502}state\u{2502}".into(),
            "\u{251c}\u{2500}\u{2500}\u{2500}\u{253c}\u{2500}\u{2500}\u{2500}\u{2524}"
                .into(),
            "\u{2502}alpha\u{2502}ready\u{2502}".into(),
            "\u{2514}\u{2500}\u{2500}\u{2500}\u{2534}\u{2500}\u{2500}\u{2500}\u{2518}"
                .into(),
        ],
    ] {
        let table = Table::detect(source.clone(), cells).unwrap();
        let wrapped = table.wrap(12, cells).unwrap();
        assert_eq!(wrapped.columns.len(), 2);
        assert_eq!(wrapped.rows.len(), source.len());
        assert_eq!(table.source(), source);
        let (row, index) = source
            .iter()
            .enumerate()
            .find(|(_, row)| row.contains("alpha"))
            .unwrap();
        assert_eq!(
            &index[wrapped.rows[row].cells[0].source_bytes.clone()],
            "alpha"
        );
    }
}

#[test]
fn explicitly_ruled_one_column_tables_are_not_confused_with_plain_lists() {
    for source in [
        vec!["| name |".into(), "| --- |".into(), "| alpha |".into()],
        vec!["name".into(), "----".into(), "alpha".into()],
    ] {
        let table = Table::detect(source.clone(), cells).unwrap();
        let wrapped = table.wrap(5, cells).unwrap();
        assert_eq!(wrapped.columns.len(), 1);
        assert_eq!(wrapped.rows.len(), 3);
        assert!(wrapped.rows[1].cells[0].fragments.is_empty());
        assert_eq!(table.source(), source);
    }
    assert!(Table::detect(vec!["name".into(), "alpha".into()], cells).is_err());
}

#[test]
fn framed_empty_columns_and_unicode_values_keep_exact_cell_ranges() {
    let source = vec![
        "| Name | Count | Notes |".into(),
        "| --- | --- | --- |".into(),
        "| \u{754c}e\u{301} | 1 | |".into(),
    ];
    let table = Table::detect(source.clone(), cells).unwrap();
    let wrapped = table.wrap(10, cells).unwrap();
    assert_eq!(
        &source[2][wrapped.rows[2].cells[0].source_bytes.clone()],
        "\u{754c}e\u{301}"
    );
    assert_eq!(wrapped.rows[2].cells[0].source_cells, 2..5);
    assert!(wrapped.rows[2].cells[2].fragments.is_empty());
    assert_eq!(table.source(), source);
}

#[test]
fn quoted_csv_shell_commands_and_ambiguous_string_matrices_are_not_automatic_tables() {
    use automexia_ui_model::tables::WrapError;
    for source in [
        vec![
            "name      state".into(),
            "alpha     running".into(),
            "beta      stopped".into(),
        ],
        vec!["Alpha     Running".into(), "Beta      Stopped".into()],
        vec![
            "kubectl get pods".into(),
            "NAME   READY".into(),
            "api    1/1".into(),
        ],
        vec!["name,count".into(), "\"alpha,beta\",1".into()],
    ] {
        if let Ok(table) = Table::detect(source, cells) {
            assert_eq!(
                table.wrap(40, cells).unwrap_err(),
                WrapError::UncertainHeader
            );
        }
    }
}

#[test]
fn row_roles_keep_header_and_rules_at_their_original_source_indices() {
    use automexia_ui_model::tables::TableRowKind::{Data, Header, Rule};
    let source = vec![
        "+------+-----+".into(),
        "| name | pid |".into(),
        "+------+-----+".into(),
        "| a    |  17 |".into(),
        "+------+-----+".into(),
    ];
    let table = Table::detect(source.clone(), cells).unwrap();
    let wrapped = table.wrap(10, cells).unwrap();
    for (index, kind) in [Rule, Header, Rule, Data, Rule].into_iter().enumerate() {
        assert_eq!(table.row_kind(index), Some(kind));
        assert_eq!(wrapped.rows[index].kind, kind);
        if kind == Rule {
            assert!(wrapped.rows[index]
                .cells
                .iter()
                .all(|c| c.fragments.is_empty()));
        }
    }
    assert_eq!(table.row_kind(5), None);
    assert_eq!(table.source(), source);
}

#[test]
fn explicit_rulers_can_label_numeric_year_columns_without_guessing_plain_numbers() {
    let source = vec![
        "| 2024 | 2025 |".into(),
        "| --- | --- |".into(),
        "| 17 | 18 |".into(),
    ];
    let table = Table::detect(source.clone(), cells).unwrap();
    let wrapped = table.wrap(10, cells).unwrap();
    assert_eq!(wrapped.columns.len(), 2);
    assert_eq!(table.source(), source);
}

#[test]
fn escaped_pipe_cells_keep_the_escape_and_exact_native_offsets() {
    let value = format!("a{}|b", char::from(92));
    let source = vec![
        "| name | count |".into(),
        "| --- | --- |".into(),
        format!("| {value} | 2 |"),
    ];
    let table = Table::detect(source.clone(), cells).unwrap();
    let wrapped = table.wrap(8, cells).unwrap();
    let cell = &wrapped.rows[2].cells[0];
    assert_eq!(&source[2][cell.source_bytes.clone()], value);
    assert_eq!(cell.source_cells, 2..6);
    assert_eq!(
        cell.fragments
            .iter()
            .map(|f| &source[2][f.bytes.clone()])
            .collect::<String>(),
        value
    );
    assert_eq!(table.source(), source);
}

#[test]
fn malformed_frames_and_duplicate_header_names_never_gain_automatic_authority() {
    for source in [
        vec![
            "| name | state |".into(),
            "| --- |".into(),
            "| alpha | ready |".into(),
        ],
        vec![
            "| name | count |".into(),
            "| --- | --- |".into(),
            "| alpha | 1 | extra |".into(),
        ],
        vec![
            "| Name | NAME |".into(),
            "| --- | --- |".into(),
            "| alpha | 1 |".into(),
        ],
        vec![
            "name | count".into(),
            "--- | ---".into(),
            "`a|b` | 1".into(),
        ],
    ] {
        if let Ok(table) = Table::detect(source, cells) {
            assert!(table.wrap(40, cells).is_err());
        }
    }
}

#[test]
fn candidate_predicates_keep_prose_boundaries_and_support_explicit_syntax() {
    use automexia_ui_model::tables::{is_candidate_line, is_rule_line};
    for line in [
        "NAME  PORTS",
        "PID COMMAND",
        "17  alpha",
        "Name Count",
        "|name|count|",
        "+----+----+",
        "name | count",
    ] {
        assert!(is_candidate_line(line), "{line}");
    }
    for line in [
        "query result",
        "kubectl get pods",
        "ordinary prose",
        "alpha",
        "",
    ] {
        assert!(!is_candidate_line(line), "{line}");
    }
    for line in [
        "----",
        "----  ----",
        "| :--- | ---: |",
        "+---+---+",
        "\u{251c}\u{2500}\u{253c}\u{2500}\u{2524}",
    ] {
        assert!(is_rule_line(line), "{line}");
    }
    assert!(!is_rule_line("not-a-rule"));
    assert!(!is_rule_line("|value|"));
}

#[test]
fn delimited_columns_and_stateful_width_callbacks_remain_bounded() {
    let source = vec![
        format!(
            "|{}|",
            (0..65)
                .map(|i| format!("H{i}"))
                .collect::<Vec<_>>()
                .join("|")
        ),
        format!("|{}|", vec!["---"; 65].join("|")),
        format!("|{}|", vec!["x"; 65].join("|")),
    ];
    assert_eq!(
        Table::detect(source, cells).unwrap_err(),
        TableError::Capacity
    );
    let source = pod_rows();
    let measurements: usize = source.iter().map(String::len).sum();
    let counter = std::cell::Cell::new(0usize);
    let result = Table::detect(source, |_| {
        counter.set(counter.get() + 1);
        if counter.get() > measurements {
            usize::MAX
        } else {
            1
        }
    });
    assert_eq!(result.unwrap_err(), TableError::Capacity);
}

#[test]
fn shell_pipeline_prefix_cannot_become_a_framed_table_header() {
    let source = [
        "cat fixture | sort",
        "+-------+-------+",
        "| Name  | Count |",
        "+-------+-------+",
        "| alpha | 2     |",
        "+-------+-------+",
    ];
    if let Ok(table) =
        Table::detect(source.iter().map(|s| (*s).to_owned()).collect(), cells)
    {
        assert!(table.wrap(40, cells).is_err());
    }
    let table =
        Table::detect(source[1..].iter().map(|s| (*s).to_owned()).collect(), cells)
            .unwrap();
    assert!(table.wrap(40, cells).is_ok());
}
