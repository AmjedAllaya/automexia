use super::*;

#[test]
fn amx_local_arguments_bound_flags_and_never_request_an_example_update() {
    for text in [false, true] {
        for invalid in ["", " ", "x\ny", "x\u{7f}", &"x".repeat(4097)] {
            assert!(find_arguments(text, invalid).is_err());
        }
        assert!(find_arguments(text, &"é".repeat(2048)).is_ok());
    }
    assert_eq!(
        explain_arguments(&["git".into(), "log".into()]).unwrap(),
        [
            "--no-auto-update",
            "--raw",
            "--color",
            "never",
            "--",
            "git",
            "log"
        ]
    );
    for words in [
        vec![],
        vec!["--update".into()],
        vec!["tar;id".into()],
        vec!["../tar".into()],
        vec!["a".repeat(257)],
        vec!["tar".into(); 9],
    ] {
        assert!(explain_arguments(&words).is_err());
    }
}

#[test]
fn amx_local_files_are_literal_sorted_relative_and_escape_terminal_controls() {
    let bytes =
        b"./src/Dockerfile\0./Dockerfile\0./file\nDockerfile\0./\x1b]52;Dockerfile\0";
    assert_eq!(
        render_files(bytes, "Dockerfile").unwrap(),
        "./Dockerfile\n./\\u{1b}]52;Dockerfile\n./file\\nDockerfile\n./src/Dockerfile\n"
    );
    assert_eq!(
        render_files(b"./Dockerfile\0", "*.rs").unwrap(),
        "No matching files.\n"
    );
    assert_eq!(render_files(b"", "x").unwrap(), "No matching files.\n");
    for bytes in [b"./x".as_slice(), b"../x\0", b"/x\0", b"\xff\0", b"\0"] {
        assert!(render_files(bytes, "x").is_err());
    }
    assert!(render_files(&b"./x\0".repeat(MAX_RESULTS), "x").is_ok());
    assert!(render_files(&b"./x\0".repeat(MAX_RESULTS + 1), "x").is_err());
    assert!(render_files(&b"./y\0".repeat(MAX_FILES), "x").is_ok());
    assert!(render_files(&b"./y\0".repeat(MAX_FILES + 1), "x").is_err());
}

fn match_row(path: &str, line: u64, text: &str) -> Vec<u8> {
    let mut bytes = serde_json::to_vec(&serde_json::json!({"type":"match","data":{"path":{"text":path},"line_number":line,"lines":{"text":text}}})).unwrap();
    bytes.push(b'\n');
    bytes
}

fn file_begin(path: &str) -> Vec<u8> {
    format!(
        "{}\n",
        serde_json::json!({"type":"begin","data":{"path":{"text":path}}})
    )
    .into_bytes()
}

fn file_end(path: &str, binary_offset: serde_json::Value) -> Vec<u8> {
    format!("{}\n", serde_json::json!({"type":"end","data":{"path":{"text":path},"binary_offset":binary_offset}})).into_bytes()
}

const SUMMARY: &[u8] = b"{\"type\":\"summary\",\"data\":{}}\n";

fn text_stream(path: &str, rows: &[u8]) -> Vec<u8> {
    [
        file_begin(path),
        rows.to_vec(),
        file_end(path, serde_json::Value::Null),
        SUMMARY.to_vec(),
    ]
    .concat()
}

#[test]
fn amx_local_late_binary_end_retracts_only_that_files_matches() {
    // Interleave files deliberately: publication cannot depend on rg's worker
    // scheduling, and the first file's late binary signal must not erase its neighbor.
    let rows = [
        file_begin("./binary"),
        match_row("./binary", 1, "omit"),
        file_begin("./text"),
        match_row("./text", 2, "keep"),
        file_end("./binary", 200019.into()),
        match_row("./text", 3, "also keep"),
        file_end("./text", serde_json::Value::Null),
        SUMMARY.to_vec(),
    ]
    .concat();
    assert_eq!(
        render_matches(&rows).unwrap(),
        "./text:2:keep\n./text:3:also keep\n"
    );
    for offset in [0, 200019, u64::MAX] {
        let rows = [
            file_begin("./binary"),
            match_row("./binary", 1, "omit"),
            file_end("./binary", offset.into()),
            SUMMARY.to_vec(),
        ]
        .concat();
        assert_eq!(render_matches(&rows).unwrap(), "No matching text.\n");
    }
}

#[test]
fn amx_local_structured_lifecycle_fails_closed_before_publication() {
    let begin = file_begin("./file");
    let row = match_row("./file", 1, "match");
    let end = file_end("./file", serde_json::Value::Null);
    for records in [
        vec![row.clone(), end.clone(), SUMMARY.to_vec()],
        vec![begin.clone(), row.clone(), SUMMARY.to_vec()],
        vec![begin.clone(), row.clone(), end.clone()],
        vec![begin.clone(), begin.clone(), row.clone(), end.clone(), SUMMARY.to_vec()],
        vec![begin.clone(), row.clone(), end.clone(), end.clone(), SUMMARY.to_vec()],
        vec![begin.clone(), row.clone(), file_end("./other", serde_json::Value::Null), SUMMARY.to_vec()],
        vec![begin.clone(), end.clone(), row.clone(), SUMMARY.to_vec()],
        vec![SUMMARY.to_vec(), SUMMARY.to_vec()],
        vec![SUMMARY.to_vec(), begin.clone(), end.clone()],
        vec![begin.clone(), row.clone(), b"{\"type\":\"end\",\"data\":{\"path\":{\"text\":\"./file\"}}}\n".to_vec(), SUMMARY.to_vec()],
        vec![begin.clone(), row.clone(), b"{\"type\":\"end\",\"data\":{\"path\":{\"text\":\"./file\"},\"binary_offset\":1,\"binary_offset\":null}}\n".to_vec(), SUMMARY.to_vec()],
    ] {
        assert!(render_matches(&records.concat()).is_err(), "incomplete or ambiguous stream accepted");
    }
    for offset in [
        serde_json::json!(-1),
        serde_json::json!(1.5),
        serde_json::json!("0"),
        serde_json::json!(false),
    ] {
        assert!(render_matches(
            &[
                begin.clone(),
                row.clone(),
                file_end("./file", offset),
                SUMMARY.to_vec()
            ]
            .concat()
        )
        .is_err());
    }
}

#[test]
fn amx_local_structured_matches_cannot_inject_controls_or_fake_result_rows() {
    assert_eq!(
        render_matches(&text_stream(
            "./file",
            &match_row("./file", 42, "hello\u{1b}]52;\u{202e}\nforged\r\n")
        ))
        .unwrap(),
        "./file:42:hello\\u{1b}]52;\\u{202e}\\nforged\n"
    );
    for bytes in [
        text_stream("../file", &match_row("../file", 1, "x")),
        text_stream("./file", &match_row("./file", 0, "x")),
        b"{\n".to_vec(),
        b"{\"type\":\"context\"}\n".to_vec(),
    ] {
        assert!(render_matches(&bytes).is_err());
    }
    let row = match_row("./file", 1, "match\n");
    assert!(render_matches(&text_stream("./file", &row.repeat(MAX_RESULTS))).is_ok());
    assert!(
        render_matches(&text_stream("./file", &row.repeat(MAX_RESULTS + 1))).is_err()
    );
    assert_eq!(
        render_matches(b"{\"type\":\"summary\",\"data\":{}}\n").unwrap(),
        "No matching text.\n"
    );
}

#[test]
fn amx_local_parsers_reject_over_budget_input_before_decoding() {
    let bytes = vec![b' '; 4 * 1024 * 1024 + 1];
    assert!(render_files(&bytes, "x").is_err());
    assert!(render_matches(&bytes).is_err());
    let row = match_row("./file", 1, "x\n");
    assert!(
        render_matches(&row[..row.len() - 1]).is_err(),
        "truncated record stream accepted"
    );
}

#[test]
fn amx_local_structured_fields_and_discarded_results_remain_bounded() {
    for row in [
        b"{\"type\":\"match\",\"data\":{\"path\":{\"bytes\":\"Zg==\"},\"line_number\":1,\"lines\":{\"text\":\"x\"}}}\n".as_slice(),
        b"{\"type\":\"match\",\"data\":{\"path\":{\"text\":\"./file\"},\"line_number\":1,\"lines\":{\"bytes\":\"/w==\"}}}\n",
        b"{\"type\":\"match\",\"data\":{\"path\":{\"text\":\"./file\"},\"line_number\":null,\"lines\":{\"text\":\"x\"}}}\n",
        b"{\"type\":\"match\",\"data\":{\"path\":{\"text\":\"./file\"},\"line_number\":1,\"line_number\":2,\"lines\":{\"text\":\"x\"}}}\n",
        b"{\"type\":\"match\",\"data\":{\"path\":{\"text\":\"./file\",\"text\":\"./other\"},\"line_number\":1,\"lines\":{\"text\":\"x\"}}}\n",
    ] {
        assert!(render_matches(&text_stream("./file", row)).is_err());
    }
    for (count, accepted) in [(MAX_RESULTS, true), (MAX_RESULTS + 1, false)] {
        let rows = [
            file_begin("./file"),
            match_row("./file", 1, "omit").repeat(count),
            file_end("./file", 1.into()),
            SUMMARY.to_vec(),
        ]
        .concat();
        assert_eq!(render_matches(&rows).is_ok(), accepted);
    }
    // Escaping expands Unicode controls beyond the captured byte count. The
    // formatted-output ceiling applies even to subsequently discarded files.
    let rows = text_stream(
        "./file",
        &match_row("./file", 1, &"\u{202e}".repeat(600000)),
    );
    assert!(rows.len() < 4 * 1024 * 1024);
    assert!(render_matches(&rows).is_err());
}

#[test]
fn amx_local_file_order_and_repeated_calls_keep_independent_state() {
    for seed in 0..64 {
        let mut rows = Vec::new();
        let mut expected = String::new();
        for file in 0..8 {
            rows.extend(file_begin(&format!("./file-{file}")));
        }
        for step in 0..8 {
            let file = (step * 3 + seed) % 8;
            let path = format!("./file-{file}");
            rows.extend(match_row(&path, 1, "fixture"));
            if (file + seed) % 3 != 0 {
                expected.push_str(&format!("./file-{file}:1:fixture\n"));
            }
        }
        for file in (0..8).rev() {
            rows.extend(file_end(
                &format!("./file-{file}"),
                if (file + seed) % 3 == 0 {
                    100.into()
                } else {
                    serde_json::Value::Null
                },
            ));
        }
        rows.extend_from_slice(SUMMARY);
        assert_eq!(render_matches(&rows).unwrap(), expected);
        assert_eq!(render_matches(SUMMARY).unwrap(), "No matching text.\n");
    }
}

#[test]
fn amx_local_cli_preserves_literal_query_and_redacts_debug() {
    use clap::Parser;
    let args = crate::cli::Cli::try_parse_from([
        "automexia",
        "find",
        "text",
        "connection refused",
    ])
    .unwrap();
    assert!(!format!("{args:?}").contains("connection refused"));
    let Some(crate::cli::CliCommand::Find(find)) = args.command else {
        panic!("find command missing")
    };
    assert_eq!(find.query, "connection refused");
    assert!(
        crate::cli::Cli::try_parse_from(["automexia", "find", "unknown", "x"]).is_err()
    );
}

#[test]
#[ignore = "explicit correctness-checked local search parsing benchmark"]
fn amx_local_benchmark_checked_parsing() {
    let files = (0..500)
        .map(|n| format!("./src/file-{n:04}.rs\0"))
        .collect::<String>();
    let rows = text_stream(
        "./src/file.rs",
        &(1..=500)
            .flat_map(|n| match_row("./src/file.rs", n, "connection refused\n"))
            .collect::<Vec<_>>(),
    );
    let expected_files = (0..500)
        .map(|n| format!("./src/file-{n:04}.rs\n"))
        .collect::<String>();
    let expected_matches = (1..=500)
        .map(|n| format!("./src/file.rs:{n}:connection refused\n"))
        .collect::<String>();
    let mut times = Vec::new();
    for _ in 0..100 {
        let start = std::time::Instant::now();
        let result =
            render_files(std::hint::black_box(files.as_bytes()), "file").unwrap();
        assert_eq!(result, expected_files);
        let result = render_matches(std::hint::black_box(&rows)).unwrap();
        assert_eq!(result, expected_matches);
        times.push(start.elapsed().as_micros());
    }
    times.sort_unstable();
    println!("amx-local-parser 500 files + 500 matches: median={}us p95={}us; excludes native process and filesystem latency", times[50], times[95]);
}
