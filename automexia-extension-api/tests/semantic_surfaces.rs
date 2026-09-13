use automexia_extension_api::surface::*;

// Literal protocol data is the oracle; production serializers do not create it.
const EMPTY: &str = r#"{"schema":{"id":"inventory","columns":[{"id":"name","title":"Name","min_width":1,"preferred_width":16,"max_width":null,"priority":0,"alignment":"left","overflow":"ellipsis","responsive":"always","data_kind":"text"}],"row_identity":"stable-handle"},"rows":[]}"#;

fn table(rows: &str) -> String {
    EMPTY.replace("\"rows\":[]", &format!("\"rows\":[{rows}]"))
}

#[test]
fn literal_table_preserves_stable_identity_and_missing_vs_empty() {
    let wire = table(
        r#"{"id":7,"resource":99,"cells":[{"text":""}]},{"id":8,"resource":null,"cells":["missing"]}"#,
    );
    let decoded: SemanticTable = serde_json::from_str(&wire).unwrap();
    assert_eq!(decoded.schema().id().as_str(), "inventory");
    assert_eq!(decoded.rows().len(), 2);
    assert_eq!(decoded.rows()[0].id().get(), 7);
    assert_eq!(decoded.rows()[0].resource().unwrap().get(), 99);
    assert!(
        matches!(&decoded.rows()[0].cells()[0], CellValue::Text(text) if text.as_str().is_empty())
    );
    assert_eq!(decoded.rows()[1].cells()[0], CellValue::Missing);
    assert_eq!(
        serde_json::to_value(&decoded).unwrap(),
        serde_json::from_str::<serde_json::Value>(&wire).unwrap()
    );
}

#[test]
fn rejects_duplicate_rows_columns_fields_and_unknown_authority() {
    let row = r#"{"id":1,"resource":null,"cells":[{"text":"item"}]}"#;
    let duplicate_rows = table(&format!("{row},{row}"));
    let column: serde_json::Value = serde_json::from_str::<serde_json::Value>(EMPTY)
        .unwrap()["schema"]["columns"][0]
        .clone();
    let mut duplicate_columns: serde_json::Value = serde_json::from_str(EMPTY).unwrap();
    duplicate_columns["schema"]["columns"] = serde_json::json!([column, column]);
    for wire in [
        duplicate_rows,
        duplicate_columns.to_string(),
        EMPTY.replace("\"rows\":[]", "\"rows\":[],\"rows\":[]"),
        EMPTY.replace(
            "\"rows\":[]",
            "\"rows\":[],\"credential\":\"fictional-secret\"",
        ),
    ] {
        assert!(serde_json::from_str::<SemanticTable>(&wire).is_err());
    }
}

#[test]
fn row_shape_and_type_must_match_the_schema() {
    for cells in [
        "[]",
        "[true]",
        "[{\"boolean\":true}]",
        "[\"missing\",\"missing\"]",
    ] {
        let wire = table(&format!(r#"{{"id":1,"resource":null,"cells":{cells}}}"#));
        assert!(serde_json::from_str::<SemanticTable>(&wire).is_err());
    }
}

#[test]
fn text_is_bounded_by_utf8_bytes_and_rejects_terminal_controls() {
    assert!(CellText::new("界".repeat(341)).is_ok());
    assert!(CellText::new("界".repeat(342)).is_err());
    assert!(CellText::new("x".repeat(1024)).is_ok());
    assert!(CellText::new("x".repeat(1025)).is_err());
    for text in [
        "\u{1b}[31m",
        "a\nb",
        "a\u{7f}",
        "a\u{202e}b",
        "a\u{2066}b",
        "a\u{200f}b",
    ] {
        assert!(CellText::new(text).is_err());
    }
    assert!(CellText::new("e\u{301} 界 👩‍💻").is_ok());
}

#[test]
fn invalid_widths_handles_and_reserved_versions_are_rejected() {
    for (from, to) in [
        ("\"min_width\":1", "\"min_width\":0"),
        ("\"min_width\":1", "\"min_width\":17"),
        ("\"max_width\":null", "\"max_width\":15"),
        ("\"priority\":0", "\"priority\":256"),
        ("\"id\":\"name\"", "\"id\":\"bad id\""),
    ] {
        assert!(serde_json::from_str::<SemanticTable>(&EMPTY.replace(from, to)).is_err());
    }
    assert!(RowId::new(0).is_err());
    assert!(ResourceHandle::new(0).is_err());
    assert!(SemanticSurfaceId::new(0).is_err());
    for version in [0, 2, 65536] {
        let wire = format!(
            r#"{{"version":{version},"binding":{{"extension":"example.inventory","session":4,"capsule_revision":2,"operation":3}},"surface":1,"generation":1,"revision":1,"event":"loading"}}"#
        );
        assert!(serde_json::from_str::<SurfaceUpdate>(&wire).is_err());
    }
}

#[test]
fn all_typed_values_keep_exact_units_and_large_integer_precision() {
    let cases = [
        ("text", r#"{"text":"plain"}"#),
        ("identifier", r#"{"identifier":"opaque-item"}"#),
        (
            "status",
            r#"{"status":{"label":"Waiting","severity":"warning"}}"#,
        ),
        ("percentage", r#"{"percentage":10000}"#),
        ("bytes", r#"{"bytes":18446744073709551615}"#),
        ("duration", r#"{"duration":18446744073709551615}"#),
        ("timestamp", r#"{"timestamp":-9223372036854775808}"#),
        ("boolean", r#"{"boolean":false}"#),
        ("number", r#"{"number":{"unsigned":9007199254740993}}"#),
        ("number", r#"{"number":{"signed":-9223372036854775808}}"#),
        (
            "number",
            r#"{"number":{"decimal":{"coefficient":123456789,"scale":9}}}"#,
        ),
    ];
    for (kind, cell) in cases {
        let wire = table(&format!(r#"{{"id":1,"resource":null,"cells":[{cell}]}}"#))
            .replace(
                "\"data_kind\":\"text\"",
                &format!("\"data_kind\":\"{kind}\""),
            );
        let decoded: SemanticTable = serde_json::from_str(&wire).unwrap();
        assert_eq!(
            serde_json::to_value(&decoded).unwrap(),
            serde_json::from_str::<serde_json::Value>(&wire).unwrap()
        );
    }
    for invalid in [
        r#"{"percentage":10001}"#,
        r#"{"percentage":-1}"#,
        r#"{"bytes":-1}"#,
        r#"{"number":{"decimal":{"coefficient":1,"scale":10}}}"#,
        r#"{"number":{"unsigned":18446744073709551616}}"#,
        r#"{"number":{"signed":1.5}}"#,
        r#"{"text":"x","boolean":true}"#,
        r#"{"status":{"label":"x","severity":"good"}}"#,
        r#"{"status":{"label":"x","severity":"info","private":true}}"#,
    ] {
        assert!(serde_json::from_str::<CellValue>(invalid).is_err());
    }
}

fn sized_table(rows: usize, columns: usize, text: Option<&str>) -> serde_json::Value {
    let mut value: serde_json::Value = serde_json::from_str(EMPTY).unwrap();
    let template = value["schema"]["columns"][0].clone();
    value["schema"]["columns"] = (0..columns)
        .map(|i| {
            let mut c = template.clone();
            c["id"] = format!("c{i}").into();
            c
        })
        .collect();
    let cell = text
        .map(|text| serde_json::json!({"text":text}))
        .unwrap_or(serde_json::json!("missing"));
    value["rows"] = (1..=rows).map(|id| serde_json::json!({"id":id,"resource":null,"cells":vec![cell.clone();columns]})).collect();
    value
}

#[test]
fn every_table_collection_budget_is_enforced_at_its_boundary() {
    for count in [0, 1, 100, 2_000, 19_999, 20_000] {
        let wire = sized_table(count, 1, None).to_string();
        let decoded: SemanticTable = serde_json::from_str(&wire).unwrap();
        assert_eq!(decoded.rows().len(), count);
        assert_eq!(decoded.cell_count(), count);
    }
    assert!(
        serde_json::from_value::<SemanticTable>(sized_table(20_001, 1, None)).is_err()
    );
    for count in [1, 63, 64] {
        assert!(
            serde_json::from_value::<SemanticTable>(sized_table(1, count, None)).is_ok()
        );
    }
    for count in [0, 65] {
        assert!(
            serde_json::from_value::<SemanticTable>(sized_table(1, count, None)).is_err()
        );
    }
    assert_eq!(
        serde_json::from_value::<SemanticTable>(sized_table(20_000, 16, None))
            .unwrap()
            .cell_count(),
        320_000
    );
    assert!(
        serde_json::from_value::<SemanticTable>(sized_table(20_000, 17, None)).is_err()
    );
    for bytes in [1023, 1024] {
        assert_eq!(
            serde_json::from_value::<SemanticTable>(sized_table(
                4096,
                1,
                Some(&"x".repeat(bytes))
            ))
            .unwrap()
            .text_bytes(),
            bytes * 4096
        );
    }
    assert!(serde_json::from_value::<SemanticTable>(sized_table(
        4097,
        1,
        Some(&"x".repeat(1024))
    ))
    .is_err());
}

#[test]
fn schema_id_title_and_cell_sequence_boundaries_are_independent() {
    assert!(ColumnId::new("x".repeat(64)).is_ok());
    assert!(ColumnId::new("x".repeat(65)).is_err());
    assert!(ColumnId::new("").is_err());
    for count in [127, 128] {
        assert!(SurfaceTitle::new("x".repeat(count)).is_ok());
    }
    assert!(SurfaceTitle::new("x".repeat(129)).is_err());
    let cells = vec![CellValue::Missing; 64];
    assert!(TableRow::new(RowId::new(1).unwrap(), None, cells).is_ok());
    assert!(
        TableRow::new(RowId::new(1).unwrap(), None, vec![CellValue::Missing; 65])
            .is_err()
    );
    assert!(serde_json::from_str::<SemanticTable>(
        &EMPTY.replace("\"title\":\"Name\"", "\"title\":\" \"")
    )
    .is_err());
}

#[test]
fn truncation_and_deterministic_hostile_mutations_never_accept_partial_frames() {
    let wire = table(r#"{"id":9,"resource":null,"cells":[{"text":"界 é"}]}"#);
    for end in 0..wire.len() {
        assert!(
            serde_json::from_slice::<SemanticTable>(&wire.as_bytes()[..end]).is_err()
        );
    }
    let mut seed = 0x5eed_u64;
    for _ in 0..2048 {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let mut mutated = wire.as_bytes().to_vec();
        let index = (seed as usize) % mutated.len();
        mutated[index] = (seed >> 32) as u8;
        if let Ok(decoded) = serde_json::from_slice::<SemanticTable>(&mutated) {
            assert_eq!(decoded.rows().len(), 1);
            assert_eq!(decoded.cell_count(), 1);
            assert!(decoded.text_bytes() <= MAX_CELL_TEXT_BYTES);
            let encoded = serde_json::to_vec(&decoded).unwrap();
            assert_eq!(
                serde_json::from_slice::<SemanticTable>(&encoded).unwrap(),
                decoded
            );
        }
    }
}

#[test]
fn diagnostic_summary_is_bounded_and_does_not_disclose_display_identifiers() {
    let mut value = sized_table(20_000, 1, Some("private-display-canary"));
    value["schema"]["id"] = "private-schema-canary".into();
    value["schema"]["columns"][0]["id"] = "private-column-canary".into();
    let table: SemanticTable = serde_json::from_value(value).unwrap();
    let summary = format!("{table:?}");
    for marker in [
        "private-display-canary",
        "private-schema-canary",
        "private-column-canary",
    ] {
        assert!(
            !summary.contains(marker),
            "table diagnostics must not contain display data"
        );
        assert!(!format!("{:?}", table.schema()).contains(marker));
    }
    assert!(summary.len() < 128, "diagnostics must not expand every row");
    assert!(summary.contains("20000"));
    // Redacted diagnostics must not corrupt the authorized user's displayed data.
    assert_eq!(table.schema().id().as_str(), "private-schema-canary");
    assert!(
        matches!(&table.rows()[0].cells()[0], CellValue::Text(text) if text.as_str() == "private-display-canary")
    );
}
