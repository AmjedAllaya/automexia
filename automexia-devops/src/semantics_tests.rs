use super::*;
use proptest::prelude::*;

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
