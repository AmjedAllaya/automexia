use automexia_ui_model::tables::{HeaderConfidence, Table, TableError, WrapError};
mod fixture {
    include!("support/inline_pipeline_fixture.rs");
}
fn width(s: &str) -> usize {
    s.chars().count()
}

#[test]
fn inline_pipeline_wide_schema_preserves_seven_columns_and_empty_values() {
    let source = fixture::pipeline_rows();
    let table = Table::detect(source.clone(), width).unwrap();
    assert_eq!(table.column_starts(), fixture::PIPELINE_STARTS);
    assert_eq!(table.header_confidence(), HeaderConfidence::UppercaseLabels);
    for columns in [21, 31, 80, 120, 250, 400] {
        let layout = table.wrap(columns, width).unwrap();
        assert_eq!(layout.columns.len(), 7);
        assert!(layout.width <= columns);
        for (r, expected) in source.iter().enumerate() {
            for c in 0..7 {
                let end = fixture::PIPELINE_STARTS
                    .get(c + 1)
                    .copied()
                    .unwrap_or(expected.len());
                let expected = expected
                    .get(fixture::PIPELINE_STARTS[c]..end)
                    .unwrap_or("")
                    .trim();
                let actual: String = layout.rows[r].cells[c]
                    .fragments
                    .iter()
                    .map(|fragment| &table.source()[r][fragment.bytes.clone()])
                    .collect();
                assert_eq!(actual, expected, "width {columns}, row {r}, cell {c}");
            }
        }
        assert!(layout.rows[2].cells[5].fragments.is_empty());
    }
    assert_eq!(table.source(), source);
}

#[test]
fn inline_pipeline_sparse_membership_is_schema_based_not_a_line_join() {
    let table =
        Table::detect(vec!["NAME     VALUE".into(), "alpha    beta".into()], width)
            .unwrap();
    assert!(table.accepts_aligned_sparse_row("         gamma", width));
    assert!(!table.accepts_aligned_sparse_row("gamma", width));
    assert!(!table.accepts_aligned_sparse_row("    gamma", width));
    assert!(!table.accepts_aligned_sparse_row("         gamma\nmore", width));
    assert!(!table.accepts_aligned_sparse_row("         \u{202e}gamma", width));
    assert_eq!(table.source(), ["NAME     VALUE", "alpha    beta"]);
}

#[test]
fn inline_pipeline_single_data_row_and_empty_middle_column_remain_valid() {
    let table = Table::detect(
        vec![
            "NAME     VALUE     STATE".into(),
            "alpha             ready".into(),
        ],
        width,
    )
    .unwrap();
    assert_eq!(table.column_starts(), [0, 9, 18]);
    let layout = table.wrap(30, width).unwrap();
    assert!(layout.rows[1].cells[1].fragments.is_empty());
    assert_eq!(table.source()[1], "alpha             ready");
}

#[test]
fn inline_pipeline_insufficient_width_does_not_truncate_source() {
    let source = fixture::pipeline_rows();
    let table = Table::detect(source.clone(), width).unwrap();
    assert!(matches!(
        table.wrap(2, width),
        Err(WrapError::TooNarrow { .. })
    ));
    assert_eq!(table.source(), source);
}

#[test]
fn inline_pipeline_hard_split_header_is_not_repaired_by_command_knowledge() {
    let source = vec![
        "RESOURCE ID     IMAGE".into(),
        "NAMES".into(),
        "resource-a001   image-v1".into(),
    ];
    match Table::detect(source, width) {
        Ok(table) => assert_ne!(
            table.column_starts().len(),
            3,
            "never invent a missing third column"
        ),
        Err(TableError::NotTable) => {}
        Err(other) => panic!("unexpected {other:?}"),
    }
}
