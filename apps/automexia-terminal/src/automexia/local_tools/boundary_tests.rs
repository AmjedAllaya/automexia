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

#[test]
fn amx_local_structured_matches_cannot_inject_controls_or_fake_result_rows() {
    assert_eq!(
        render_matches(&match_row(
            "./file",
            42,
            "hello\u{1b}]52;\u{202e}\nforged\r\n"
        ))
        .unwrap(),
        "./file:42:hello\\u{1b}]52;\\u{202e}\\nforged\n"
    );
    for bytes in [
        match_row("../file", 1, "x"),
        match_row("./file", 0, "x"),
        b"{\n".to_vec(),
        b"{\"type\":\"context\"}\n".to_vec(),
    ] {
        assert!(render_matches(&bytes).is_err());
    }
    let row = match_row("./file", 1, "match\n");
    assert!(render_matches(&row.repeat(MAX_RESULTS)).is_ok());
    assert!(render_matches(&row.repeat(MAX_RESULTS + 1)).is_err());
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
    let rows = (1..=500)
        .flat_map(|n| match_row("./src/file.rs", n, "connection refused\n"))
        .collect::<Vec<_>>();
    let mut times = Vec::new();
    for _ in 0..100 {
        let start = std::time::Instant::now();
        let result =
            render_files(std::hint::black_box(files.as_bytes()), "file").unwrap();
        assert_eq!(result.lines().count(), 500);
        assert!(result.starts_with("./src/file-0000.rs\n"));
        let result = render_matches(std::hint::black_box(&rows)).unwrap();
        assert_eq!(result.lines().count(), 500);
        assert!(result.ends_with("./src/file.rs:500:connection refused\n"));
        times.push(start.elapsed().as_micros());
    }
    times.sort_unstable();
    println!("amx-local-parser 500 files + 500 matches: median={}us p95={}us; excludes native process and filesystem latency", times[50], times[95]);
}
