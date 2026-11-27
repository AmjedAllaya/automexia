use super::*;
use proptest::prelude::*;

#[test]
fn package_progress_preserves_severity_and_source_ranges() {
    use OperationalPart::{Detail, Measurement, Phase, Subject, Version};
    use SemanticSeverity::{Error, Info, Success, Warning};

    type Case<'a> = (&'a str, SemanticSeverity, &'a [(OperationalPart, &'a str)]);
    let cases: &[Case<'_>] = &[
        (
            "  \u{2003}Get:1 https://example.invalid/dépot stable/main amd64 unknown amd64 1.2.3 [241 kB]  ",
            Info,
            &[
                (Phase, "Get:1"),
                (Detail, "https://example.invalid/dépot"),
                (Subject, "unknown"),
                (Version, "1.2.3"),
                (Measurement, "[241 kB]"),
            ],
        ),
        (
            "Hit:2 https://example.invalid/ubuntu stable InRelease",
            Info,
            &[(Phase, "Hit:2"), (Detail, "https://example.invalid/ubuntu")],
        ),
        (
            "Get:2 https://example.invalid/ubuntu stable InRelease [128 kB]",
            Info,
            &[
                (Phase, "Get:2"),
                (Detail, "https://example.invalid/ubuntu"),
                (Measurement, "[128 kB]"),
            ],
        ),
        (
            "Ign:3 https://example.invalid/ubuntu stable InRelease",
            Warning,
            &[(Phase, "Ign:3"), (Detail, "https://example.invalid/ubuntu")],
        ),
        (
            "Err:4 https://example.invalid/ubuntu stable InRelease",
            Error,
            &[(Phase, "Err:4"), (Detail, "https://example.invalid/ubuntu")],
        ),
        (
            "Fetched 25.3 MB in 3s (8565 kB/s)",
            Success,
            &[
                (Phase, "Fetched"),
                (Measurement, "25.3 MB"),
                (Measurement, "3s"),
                (Measurement, "8565 kB/s"),
            ],
        ),
        (
            "Preconfiguring packages ...",
            Info,
            &[(Phase, "Preconfiguring packages")],
        ),
        (
            "Need to get 1.2 MB/2.4 MB of archives.",
            Info,
            &[
                (Phase, "Need to get"),
                (Measurement, "1.2 MB"),
                (Measurement, "2.4 MB"),
            ],
        ),
        (
            "After this operation, 36 kB of additional disk space will be used.",
            Info,
            &[(Phase, "After this operation,"), (Measurement, "36 kB")],
        ),
        (
            "Selecting previously unselected package demo:amd64.",
            Info,
            &[(Phase, "Selecting previously unselected package"), (Subject, "demo:amd64")],
        ),
        (
            "(Reading database ... 74789 files and directories currently installed.)",
            Info,
            &[(Phase, "(Reading database ..."), (Measurement, "74789")],
        ),
        (
            "(Reading database ... 35%",
            Info,
            &[(Phase, "(Reading database ..."), (Measurement, "35%")],
        ),
        (
            "Preparing to unpack .../00-demo_1.2.3_amd64.deb ...",
            Info,
            &[(Phase, "Preparing to unpack"), (Subject, ".../00-demo_1.2.3_amd64.deb")],
        ),
        (
            "Unpacking demo:amd64 (5:29.10.1-1~stable+1) over (5:29.10.0-1~stable) ...",
            Info,
            &[
                (Phase, "Unpacking"),
                (Subject, "demo:amd64"),
                (Version, "5:29.10.1-1~stable+1"),
                (Detail, "5:29.10.0-1~stable"),
            ],
        ),
        (
            "Setting up demo (1.2.3) ...",
            Info,
            &[(Phase, "Setting up"), (Subject, "demo"), (Version, "1.2.3")],
        ),
        (
            "Processing triggers for demo (1.2.3) ...",
            Info,
            &[(Phase, "Processing triggers for"), (Subject, "demo"), (Version, "1.2.3")],
        ),
        (
            "Current default time zone: 'Etc/UTC'",
            Info,
            &[(Phase, "Current default time zone:"), (Subject, "Etc/UTC")],
        ),
        (
            "Local time is now:      Mon Jan  5 08:00:00 UTC 2026.",
            Info,
            &[(Phase, "Local time is now:"), (Measurement, "Mon Jan  5 08:00:00 UTC 2026")],
        ),
        (
            "Universal Time is now: Mon Jan  5 08:00:00 UTC 2026.",
            Info,
            &[(Phase, "Universal Time is now:"), (Measurement, "Mon Jan  5 08:00:00 UTC 2026")],
        ),
        (
            "Run 'dpkg-reconfigure tzdata' if you wish to change it.",
            Info,
            &[(Phase, "Run"), (Subject, "dpkg-reconfigure tzdata")],
        ),
        (
            "Reading package lists... Done",
            Success,
            &[(Phase, "Reading package lists... Done")],
        ),
        (
            "Continue? [Y/n] y",
            Warning,
            &[(Phase, "Continue?"), (Subject, "[Y/n]")],
        ),
        (
            "E: Failed to fetch https://example.invalid/pkg.deb  404 Not Found",
            Error,
            &[(Phase, "E:")],
        ),
        (
            "W: Failed to fetch https://example.invalid/archive stable InRelease",
            Warning,
            &[(Phase, "W:")],
        ),
    ];

    let mut classifier = RowClassifier::default();
    for &(row, severity, fields) in cases {
        let expected = Some(OutputClassification {
            domain: OutputDomain::General,
            severity: Some(severity),
        });
        let progress = operational_progress(row).expect(row);
        assert_eq!(progress.severity, severity, "{row}");
        assert_eq!(classify_row(row), expected, "{row}");
        assert_eq!(classifier.classify(row), expected, "{row}");
        let mut last_end = 0;
        let actual = progress
            .spans
            .into_iter()
            .flatten()
            .map(|span| {
                assert!(last_end <= span.start && span.start < span.end, "{row}");
                last_end = span.end;
                (span.role, &row[span.start..span.end])
            })
            .collect::<Vec<_>>();
        assert_eq!(actual, fields, "{row}");
    }
}

#[test]
fn package_progress_rejects_ambiguous_prose_and_over_budget_rows() {
    for row in [
        "Get:1 packages in stock",
        "Get: https://example.invalid/archive stable InRelease",
        "Get:1 https://",
        "Get:x https://example.invalid/archive stable InRelease",
        "Prefetched 25.3 MB in 3s (8565 kB/s)",
        "Fetched some records in 3s",
        "Fetched 20 packages in 3s",
        "Need to get 20 packages of archives.",
        "After this operation, the installer will restart.",
        "Preparing to unpack the shipment ...",
        "Setting up a configuration (default) ...",
        "Continue? [Y/n] yes, please",
        "E: drive",
        "Reading package lists... can take a while",
        "Current default time zone: 'ask your administrator'",
        "Local time is now: Mon Jan 5 00:00 UTC 2026.",
        "Universal Time is now: available",
        "Unpacking demo (1.2.3) ...\nsecond line",
    ] {
        assert_eq!(operational_progress(row), None, "{row}");
    }
    let oversized = format!(
        "Get:1 https://example.invalid/{} stable InRelease",
        "a".repeat(MAX_ROW_BYTES)
    );
    assert_eq!(operational_progress(&oversized), None);
}

#[test]
fn plain_informational_output_uses_shared_log_palette() {
    for row in [
        "/workspace/project",
        "/workspace/équipe tools/error-reports",
        "/",
        "~/workspace/project",
        "./project",
        "../project",
        r"C:\workspace\project",
        r"D:\workspace\Project Files",
        r"\\example.invalid\share\project",
        "No resources found in demo namespace.",
        "No resources found.",
        "logout",
        "exit",
    ] {
        assert_eq!(
            classify_row(row),
            Some(OutputClassification {
                domain: OutputDomain::General,
                severity: Some(SemanticSeverity::Info),
            }),
            "{row}"
        );
    }
}

#[test]
fn plain_output_detection_preserves_diagnostics_and_neutral_text() {
    for row in [
        "/bin/bash: fixture: command not found",
        "/workspace/tool: permission denied",
        r"C:\workspace\tool: error: permission denied",
        "No resources found: permission denied",
    ] {
        assert_eq!(
            classify_row_text(row),
            Some(SemanticSeverity::Error),
            "{row}"
        );
    }
    for row in [
        "ordinary text",
        "exit code: 0",
        "logout is a shell command",
        "/ this is not a path",
        "// ordinary comment",
        "C:relative",
        "No resources found is a sentence in this documentation",
    ] {
        assert_eq!(classify_row_text(row), None, "{row}");
    }
    assert_eq!(
        classify_row_text(&format!("/{}", "a".repeat(MAX_ROW_BYTES))),
        None
    );
}

#[test]
fn cli_diagnostics_use_log_colours_independently_of_resource_statuses() {
    use rio_backend::config::presentation::Presentation;
    for row in [
        "Error from server (NotFound): pods fixture not found",
        "Error from server (Forbidden): deployments.apps is forbidden",
        "error: the server doesn't have a resource type fixture",
        "fish: Unknown command: fixture",
        "fixture: The term 'fixture' is not recognized as a name of a cmdlet",
    ] {
        let actual = classify_row(row).expect("recognized CLI diagnostic");
        assert_eq!(actual.domain, OutputDomain::General, "{row}");
        assert_eq!(actual.severity, Some(SemanticSeverity::Error), "{row}");
        for logs in [false, true] {
            for resources in [false, true] {
                let presentation = Presentation {
                    output_highlighting: logs,
                    kubernetes_highlighting: resources,
                    ..Presentation::default()
                };
                assert_eq!(
                    appearance_for(&presentation, actual),
                    logs.then_some(&presentation.highlight)
                );
            }
        }
    }
}

#[test]
fn kubernetes_domain_survives_disabled_and_unknown_statuses() {
    use rio_backend::config::presentation::Presentation;
    for row in [
        "NAME READY STATUS RESTARTS AGE",
        "pod/api 0/1 Unknown 0 1m",
        "pod/api 1/1 FutureState error",
        "Ready FutureState",
        "node FutureState worker 1d v1.0",
        "| api | 0/1 | Unknown | 0 | 1m |",
    ] {
        let classification = classify_row(row).expect("recognized Kubernetes row");
        assert_eq!(classification.domain, OutputDomain::Kubernetes, "{row}");
        for generic in [false, true] {
            let presentation = Presentation {
                output_highlighting: generic,
                kubernetes_highlighting: false,
                ..Presentation::default()
            };
            assert_eq!(appearance_for(&presentation, classification), None, "{row}");
        }
    }
    assert_eq!(
        classify_row("pod/api 1/1 FutureState error")
            .unwrap()
            .severity,
        None
    );
    for row in ["[ERROR] compilation failed", "web Up 2 minutes (healthy)"] {
        assert_eq!(classify_row(row).unwrap().domain, OutputDomain::General);
    }
}

#[test]
fn semantic_domains_select_independent_palette_and_style() {
    use rio_backend::config::presentation::{HighlightStyle, Presentation, Rgb};
    let mut presentation = Presentation::default();
    presentation.highlight.colors.warning = Some(Rgb::from_bytes([11, 22, 33]));
    presentation.kubernetes.colors.warning = Some(Rgb::from_bytes([44, 55, 66]));
    presentation.kubernetes.style = HighlightStyle::Background;
    for generic in [false, true] {
        for kubernetes in [false, true] {
            presentation.output_highlighting = generic;
            presentation.kubernetes_highlighting = kubernetes;
            assert_eq!(
                appearance_for(&presentation, classify_row("[WARN] retrying").unwrap()),
                generic.then_some(&presentation.highlight),
            );
            assert_eq!(
                appearance_for(&presentation, classify_row("api 0/1 Running").unwrap()),
                kubernetes.then_some(&presentation.kubernetes),
            );
        }
    }
}

#[test]
fn completion_fragments_never_claim_running_health() {
    for row in ["Completed", "Succeeded", "Completed 0 4h", "Succeeded 0 4h"] {
        assert_eq!(classify_row_text(row), Some(SemanticSeverity::Info));
    }
}

#[test]
fn kind_prefixed_pods_keep_readiness_and_status_precedence() {
    for prefix in ["pod/", "example pod/"] {
        for (fields, severity) in [
            ("1/1 Running", SemanticSeverity::Success),
            ("0/1 Running", SemanticSeverity::Warning),
            ("0/1 Completed", SemanticSeverity::Info),
            ("0/1 Init:Error", SemanticSeverity::Error),
        ] {
            let row = format!("{prefix}error {fields} 0 1d");
            assert_eq!(classify_row_text(&row), Some(severity), "{row}");
        }
    }
    assert_eq!(classify_row_text("pod/api 1/1 FutureState"), None);
    assert_eq!(classify_row_text("path/api 1/1 Running"), None);
}

#[test]
fn zero_counts_are_not_log_timestamp_prefixes() {
    for row in ["0 error", "0 errors", "0 failures", "2026-01-01 0 error"] {
        assert_eq!(classify_row_text(row), None, "{row}");
    }
    assert_eq!(classify_row_text("4 errors"), Some(SemanticSeverity::Error));
    assert_eq!(
        classify_row_text("[ERROR] 0 errors"),
        Some(SemanticSeverity::Error)
    );
}

#[test]
fn kubernetes_completion_is_not_readiness() {
    for status in ["Completed", "Succeeded"] {
        for ready in ["0/1", "1/1", "0/0"] {
            for prefix in ["", "batch ", "example batch ", "error ", "pending-api "] {
                let row = format!("{prefix}{ready} {status} 0 4h");
                assert_eq!(
                    classify_row_text(&row),
                    Some(SemanticSeverity::Info),
                    "{row}"
                );
            }
        }
    }
    // Successful commands remain success; a terminated workload is different.
    assert_eq!(
        classify_row_text("apply completed successfully"),
        Some(SemanticSeverity::Success)
    );
}

#[test]
fn kubernetes_status_fields_win_over_names_and_incidental_columns() {
    for name in [
        "error",
        "unknown-api",
        "unhealthy-check",
        "completed-job",
        "warning",
    ] {
        for (status, severity) in [
            ("Running", SemanticSeverity::Success),
            ("Completed", SemanticSeverity::Info),
            ("Pending", SemanticSeverity::Warning),
            ("CrashLoopBackOff", SemanticSeverity::Error),
        ] {
            let row = format!("example {name} 1/1 {status} 0 1d");
            assert_eq!(classify_row_text(&row), Some(severity), "{row}");
        }
    }
    for ratio in ["0/1", "1/2", "0/0", "2/1", "4294967296/1"] {
        let row = format!("web {ratio} Running 0 1h");
        assert_eq!(
            classify_row_text(&row),
            Some(SemanticSeverity::Warning),
            "{row}"
        );
    }
    for row in [
        "web x1/1 Running",
        "web 1/1x Running",
        "web 1/1 FutureState",
        "running-api 1/1 Mystery",
        "path/1/1 Running",
    ] {
        assert_eq!(classify_row_text(row), None, "{row}");
    }
}

#[test]
fn kubernetes_failure_and_transition_matrix() {
    for status in [
        "Error",
        "Failed",
        "CrashLoopBackOff",
        "ImagePullBackOff",
        "ErrImagePull",
        "ErrImageNeverPull",
        "CreateContainerConfigError",
        "CreateContainerError",
        "RunContainerError",
        "ContainerCannotRun",
        "InvalidImageName",
        "OOMKilled",
        "OutOfMemory",
        "OutOfCpu",
        "UnexpectedAdmissionError",
        "StartError",
        "PreStartHookError",
        "PostStartHookError",
        "Unhealthy",
        "ContainerStatusUnknown",
        "Evicted",
        "NodeLost",
        "DeadlineExceeded",
        "Init:Error",
        "Init:CrashLoopBackOff",
        "Init:ExitCode:2",
    ] {
        let row = format!("batch 0/1 {status} 0 1m");
        assert_eq!(
            classify_row_text(&row),
            Some(SemanticSeverity::Error),
            "{row}"
        );
    }
    for status in [
        "Pending",
        "ContainerCreating",
        "PodInitializing",
        "Terminating",
        "Waiting",
        "Unknown",
        "NotReady",
        "SchedulingGated",
        "Init:0/2",
        "Init:1/1",
    ] {
        let row = format!("web 1/1 {status} 0 1m");
        assert_eq!(
            classify_row_text(&row),
            Some(SemanticSeverity::Warning),
            "{row}"
        );
    }
}

#[test]
fn conditions_respect_polarity_and_unknown_state() {
    for row in [
        "Ready True",
        "Available=True",
        "node Ready worker 1d v1.0",
        "DiskPressure False",
        "MemoryPressure=False",
        "PIDPressure False",
        "Healthy=True",
        "NetworkUnavailable False",
    ] {
        assert_eq!(
            classify_row_text(row),
            Some(SemanticSeverity::Success),
            "{row}"
        );
    }
    for row in [
        "Ready False",
        "Available=False",
        "Healthy False",
        "Available Unknown",
        "Ready Unknown",
        "node NotReady worker 1d v1.0",
        "node Ready,SchedulingDisabled worker 1d v1.0",
        "DiskPressure Unknown",
    ] {
        assert_eq!(
            classify_row_text(row),
            Some(SemanticSeverity::Warning),
            "{row}"
        );
    }
    for row in [
        "DiskPressure True",
        "MemoryPressure=True",
        "PIDPressure True",
        "NetworkUnavailable True",
    ] {
        assert_eq!(
            classify_row_text(row),
            Some(SemanticSeverity::Error),
            "{row}"
        );
    }
}

#[test]
fn container_lifecycle_is_not_health() {
    for (row, severity) in [
        ("worker Exited (0) 3 seconds ago", SemanticSeverity::Info),
        ("worker Exited (137) 3 seconds ago", SemanticSeverity::Error),
        ("worker Exited (-1) 3 seconds ago", SemanticSeverity::Error),
        (
            "worker Exited (oops) 3 seconds ago",
            SemanticSeverity::Warning,
        ),
        (
            "worker Restarting (1) 3 seconds ago",
            SemanticSeverity::Warning,
        ),
        ("web Up 2 minutes (healthy)", SemanticSeverity::Success),
        ("web Up 2 minutes (unhealthy)", SemanticSeverity::Error),
        (
            "web Up 2 minutes (health: starting)",
            SemanticSeverity::Warning,
        ),
        ("web Up 2 minutes (Paused)", SemanticSeverity::Warning),
        (
            "web Up 2 minutes (healthy) (Paused)",
            SemanticSeverity::Warning,
        ),
        ("web Up 2 minutes", SemanticSeverity::Info),
        ("web Up About an hour", SemanticSeverity::Info),
        ("web  Created", SemanticSeverity::Info),
        ("web  Dead", SemanticSeverity::Error),
        ("web  Removing", SemanticSeverity::Warning),
        (
            "dead-check  Up 2 minutes (healthy)",
            SemanticSeverity::Success,
        ),
        ("web  Up 2 minutes  unhealthy", SemanticSeverity::Info),
    ] {
        assert_eq!(classify_row_text(row), Some(severity), "{row}");
    }
}

#[test]
fn container_columns_never_treat_image_or_name_as_status() {
    for row in [
        "012345abcdef  dead  \"/entrypoint\"  2 minutes ago  Up 1 minute (healthy)  80/tcp  unhealthy",
        "012345abcdef  created  \"/entrypoint\"  2 minutes ago  Up 1 minute (healthy)  dead",
        "dead  Up 1 minute (healthy)",
    ] {
        assert_eq!(classify_row_text(row), Some(SemanticSeverity::Success), "{row}");
    }
    assert_eq!(
        classify_row_text(
            "012345abcdef  dead  \"/entrypoint\"  2 minutes ago  FutureState  unhealthy"
        ),
        None
    );
}

#[test]
fn logs_and_summaries_do_not_hide_failures_or_invent_success() {
    for row in [
        "tests completed: 10 failed, 0 errors",
        "tests completed: 0 failed, 2 errors",
        "permission denied; 0 failed",
        "operation unsuccessful",
        "apply completed unsuccessfully",
        "build not successful",
        "tests completed: 0 failed,errors:2",
    ] {
        assert_eq!(
            classify_row_text(row),
            Some(SemanticSeverity::Error),
            "{row}"
        );
    }
    for row in [
        "tests completed: 42 passed, 0 failed",
        "tests completed: 42 passed, failed: 0",
        "tests completed: 42 passed, 0 errors",
    ] {
        assert_eq!(
            classify_row_text(row),
            Some(SemanticSeverity::Success),
            "{row}"
        );
    }
    for row in [
        "image=example.invalid/exceptional:v1",
        "level=errorish",
        "successful-api",
        "completed-backup",
        "notes say info is missing",
        "界exception",
    ] {
        assert_eq!(classify_row_text(row), None, "{row}");
    }
    for (row, severity) in [
        ("[INFO] request failed", SemanticSeverity::Info),
        ("[ERROR] 0 failed", SemanticSeverity::Error),
        (
            "2026-01-01 12:00:00 WARN retrying",
            SemanticSeverity::Warning,
        ),
        ("level=debug operation failed", SemanticSeverity::Debug),
        (
            "Terraform will perform these actions",
            SemanticSeverity::Info,
        ),
        (
            "Apply complete! Resources: 1 added",
            SemanticSeverity::Success,
        ),
        ("Warning: deprecated API", SemanticSeverity::Warning),
        ("merge conflict", SemanticSeverity::Error),
        ("not ready", SemanticSeverity::Warning),
        ("request throttled", SemanticSeverity::Warning),
        (
            "test result: ok. 42 passed; 0 failed",
            SemanticSeverity::Success,
        ),
    ] {
        assert_eq!(classify_row_text(row), Some(severity), "{row}");
    }
}

#[test]
fn semantic_rows_have_a_fail_safe_byte_budget() {
    let suffix = "0/1 Completed";
    for length in [32768, 32769, 65536] {
        let text = format!("{}{suffix}", " ".repeat(length - suffix.len()));
        let expected = if length == 32768 {
            Some(SemanticSeverity::Info)
        } else {
            None
        };
        // Size is checked before trimming; padding cannot bypass it.
        assert_eq!(classify_row_text(&text), expected);
    }
    assert_eq!(classify_row_text(""), None);
    assert_eq!(classify_row_text("\0\0  \t"), None);
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(128))]
    #[test]
    fn readiness_has_exact_numeric_meaning(ready in 0u32..200, total in 0u32..200) {
        let row = format!("sample {ready}/{total} Running");
        let expected = if total > 0 && ready == total { SemanticSeverity::Success } else { SemanticSeverity::Warning };
        prop_assert_eq!(classify_row_text(&row), Some(expected));
    }
    #[test]
    fn classification_preserves_input(text in ".{0,2048}") {
        let original = text.clone();
        let first = classify_row_text(&text);
        prop_assert_eq!(classify_row_text(&text), first);
        prop_assert_eq!(text, original);
    }
}
