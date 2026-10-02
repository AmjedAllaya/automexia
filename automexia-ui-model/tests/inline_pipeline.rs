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

#[test]
fn inline_pipeline_schema_bootstrap_then_cached_rows_share_one_attempt_budget() {
    use automexia_ui_model::tables::{CandidateSchema, TableDetectionBudget};
    let mut schema = CandidateSchema::default();
    let mut budget = TableDetectionBudget::default();
    assert!(schema
        .admits_row(["NAME     VALUE"], "         beta", width, &mut budget)
        .unwrap());
    assert_eq!(budget.attempts(), 1);
    for index in 0..100 {
        let row = format!("         value-{index:03}");
        let unread_prefix = std::iter::once_with(|| {
            panic!("cached admission must not revisit growing source")
        });
        assert!(schema
            .admits_row(unread_prefix, &row, width, &mut budget)
            .unwrap());
    }
    assert_eq!(budget.attempts(), 1);
    assert!(!schema
        .admits_row(std::iter::empty(), "finished", width, &mut budget)
        .unwrap());
    assert!(!schema
        .admits_row(std::iter::empty(), "    misplaced", width, &mut budget)
        .unwrap());
    assert!(!schema
        .admits_row(
            std::iter::empty(),
            "         \u{202e}spoof",
            width,
            &mut budget
        )
        .unwrap());
}

#[test]
fn inline_pipeline_schema_admits_aligned_multi_cell_records_and_rejects_gutter_crossing()
{
    use automexia_ui_model::tables::{CandidateSchema, TableDetectionBudget};
    let mut schema = CandidateSchema::default();
    let mut budget = TableDetectionBudget::default();
    assert!(schema
        .admits_row(["NAME VALUE"], "aaaa 1", width, &mut budget)
        .unwrap());
    assert!(schema
        .admits_row(std::iter::empty(), "bbbb 2", width, &mut budget)
        .unwrap());
    assert!(!schema
        .admits_row(std::iter::empty(), "abcde2", width, &mut budget)
        .unwrap());
    assert!(!schema
        .admits_row(std::iter::empty(), "bbbb", width, &mut budget)
        .unwrap());
}

#[test]
fn inline_pipeline_schema_probes_reserve_complete_detection_and_fail_at_the_ceiling() {
    use automexia_ui_model::tables::{
        CandidateSchema, DetectionError, TableDetectionBudget, MAX_TABLE_MODEL_ATTEMPTS,
    };
    let mut budget = TableDetectionBudget::default();
    while budget.attempts() < MAX_TABLE_MODEL_ATTEMPTS / 2 {
        let mut schema = CandidateSchema::default();
        assert!(!schema
            .admits_row(
                ["UPPER  HEAD", "UPPER  HEAD"],
                "         UPPER",
                width,
                &mut budget
            )
            .unwrap());
    }
    let mut schema = CandidateSchema::default();
    assert_eq!(
        schema.admits_row(["NAME     VALUE"], "         beta", width, &mut budget),
        Err(DetectionError::Budget)
    );
    for _ in MAX_TABLE_MODEL_ATTEMPTS / 2..MAX_TABLE_MODEL_ATTEMPTS {
        let table = budget
            .detect(vec!["NAME  VALUE".into(), "alpha beta".into()], width)
            .unwrap();
        assert_eq!(table.column_starts(), [0, 6]);
    }
    assert!(matches!(
        budget.detect(vec!["NAME  VALUE".into(), "alpha beta".into()], width),
        Err(DetectionError::Budget)
    ));
    assert_eq!(budget.attempts(), MAX_TABLE_MODEL_ATTEMPTS);
}

#[test]
fn inline_pipeline_schema_rejects_oversized_prefix_before_model_attempts() {
    use automexia_ui_model::tables::{
        CandidateSchema, DetectionError, TableDetectionBudget, MAX_TABLE_ROWS,
    };
    let mut schema = CandidateSchema::default();
    let mut budget = TableDetectionBudget::default();
    assert_eq!(
        schema.admits_row(
            std::iter::repeat_n("NAME  VALUE", MAX_TABLE_ROWS + 1),
            "alpha beta",
            width,
            &mut budget
        ),
        Err(DetectionError::Table(TableError::Capacity))
    );
    assert_eq!(budget.attempts(), 0);
}

#[test]
fn inline_pipeline_exhausted_schema_budget_never_reads_or_clones_prefix() {
    use automexia_ui_model::tables::{
        CandidateSchema, DetectionError, TableDetectionBudget, MAX_TABLE_MODEL_ATTEMPTS,
    };
    let mut budget = TableDetectionBudget::default();
    for _ in 0..MAX_TABLE_MODEL_ATTEMPTS / 2 {
        budget
            .detect(vec!["NAME  VALUE".into(), "alpha beta".into()], width)
            .unwrap();
    }
    let mut schema = CandidateSchema::default();
    let unread_prefix = std::iter::once_with(|| {
        panic!("exhausted bootstrap must not read or allocate source")
    });
    assert_eq!(
        schema.admits_row(unread_prefix, "         beta", width, &mut budget),
        Err(DetectionError::Budget)
    );
    assert_eq!(budget.attempts(), MAX_TABLE_MODEL_ATTEMPTS / 2);
    assert_eq!(
        schema.admits_row(std::iter::empty(), "finished", width, &mut budget),
        Ok(false)
    );
}

#[test]
fn inline_pipeline_warm_schema_accepts_without_prefix_after_bootstrap_budget_exhausted() {
    use automexia_ui_model::tables::{
        CandidateSchema, TableDetectionBudget, MAX_TABLE_MODEL_ATTEMPTS,
    };
    let mut schema = CandidateSchema::default();
    let mut budget = TableDetectionBudget::default();
    assert!(schema
        .admits_row(["NAME     VALUE"], "         beta", width, &mut budget)
        .unwrap());
    while budget.attempts() < MAX_TABLE_MODEL_ATTEMPTS / 2 {
        budget
            .detect(vec!["NAME  VALUE".into(), "alpha beta".into()], width)
            .unwrap();
    }
    let unread_prefix = std::iter::once_with(|| {
        panic!("warm schema must retain zero-prefix-cost admission")
    });
    assert!(schema
        .admits_row(unread_prefix, "         gamma", width, &mut budget)
        .unwrap());
    assert_eq!(budget.attempts(), MAX_TABLE_MODEL_ATTEMPTS / 2);
}

#[test]
fn inline_pipeline_schema_right_aligned_numeric_values_keep_sparse_api_contract() {
    use automexia_ui_model::tables::{CandidateSchema, TableDetectionBudget};
    let table =
        Table::detect(vec!["Name     Count".into(), "            1".into()], width)
            .unwrap();
    assert_eq!(table.column_starts(), [0, 9]);
    // Public sparse API remains deliberately exact-start and single-cell.
    assert!(!table.accepts_aligned_sparse_row("            1", width));
    let mut schema = CandidateSchema::default();
    let mut budget = TableDetectionBudget::default();
    assert!(schema
        .admits_row(["Name     Count"], "            1", width, &mut budget)
        .unwrap());
    assert!(schema
        .admits_row(std::iter::empty(), "           23", width, &mut budget)
        .unwrap());
    assert!(!schema
        .admits_row(
            std::iter::empty(),
            "           finished",
            width,
            &mut budget
        )
        .unwrap());
    assert!(!schema
        .admits_row(std::iter::empty(), "     123", width, &mut budget)
        .unwrap());
}
