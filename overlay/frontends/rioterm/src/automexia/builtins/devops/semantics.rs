use crate::automexia::api::SemanticSeverity;

/// Classify one visible terminal row without allocating and without mutating
/// terminal bytes. Explicit application ANSI colors still win in grid_emit.
pub fn classify_row_text(text: &str) -> Option<SemanticSeverity> {
    let text = text.trim_matches('\0').trim();
    if text.is_empty() {
        return None;
    }

    if let Some(level) = structured_log_level(text) {
        return Some(level);
    }

    let zero_failure_summary = [
        "0 failed",
        "failed: 0",
        "0 errors",
        "errors: 0",
        "0 error",
        "error: 0",
    ]
    .iter()
    .any(|term| contains_ascii_case_insensitive(text, term));

    const ERROR_TERMS: &[&str] = &[
        // Shell / process failures.
        "command not found",
        "is not recognized as an internal or external command",
        "is not recognized as the name of a cmdlet",
        "cannot find path",
        "cannot find the path",
        "the system cannot find the file specified",
        "no such file or directory",
        "permission denied",
        "access denied",
        "operation not permitted",
        "fatal:",
        "panic:",
        "exception",
        "traceback (most recent call last)",
        // Connectivity / auth / TLS.
        "error from server",
        "error response from daemon",
        "unable to connect",
        "connection refused",
        "connection reset",
        "could not resolve host",
        "no such host",
        "forbidden",
        "unauthorized",
        "authentication failed",
        "timed out",
        "timeout",
        "certificate error",
        "certificate has expired",
        "x509",
        // Kubernetes/container failure states carried over from Automexia kgp.
        "crashloopbackoff",
        "imagepullbackoff",
        "errimagepull",
        "errimageneverpull",
        "createcontainerconfigerror",
        "createcontainererror",
        "runcontainererror",
        "containercannotrun",
        "invalidimagename",
        "oomkilled",
        "outofmemory",
        "outofcpu",
        "deadlineexceeded",
        "unexpectedadmissionerror",
        "starterror",
        "prestart",
        "poststart",
        "unhealthy",
        "evicted",
        "nodelost",
        "back-off restarting failed container",
        "insufficient cpu",
        "insufficient memory",
        "no space left on device",
        "diskpressure",
        "networkunavailable",
        "dns lookup failed",
        // Git/build/general hard failure vocabulary.
        "merge conflict",
        "conflict (content)",
        "npm err!",
        "npm error",
        "fullyqualifiederrorid",
        "categoryinfo",
        "error[",
        "rejected]",
        "build failed",
        "test failed",
        "tests failed",
        " failed",
        "failed ",
        " failure",
    ];
    if !zero_failure_summary
        && ERROR_TERMS
            .iter()
            .any(|term| contains_ascii_case_insensitive(text, term))
    {
        return Some(SemanticSeverity::Error);
    }

    if let Some(severity) = docker_status(text) {
        return Some(severity);
    }

    const WARNING_TERMS: &[&str] = &[
        "warning:",
        "warn:",
        "deprecated",
        "deprecation",
        " pending",
        "pending ",
        "containercreating",
        "podinitializing",
        "terminating",
        " waiting",
        "waiting ",
        "notready",
        "not ready",
        "schedulinggated",
        "init:",
        "degraded",
        "backoff",
        "back-off",
        "retrying",
        "progressing",
        "rate limit",
        "throttl",
        "unknown",
    ];
    if WARNING_TERMS
        .iter()
        .any(|term| contains_ascii_case_insensitive(text, term))
    {
        return Some(SemanticSeverity::Warning);
    }

    // Kubernetes-style pod output: Running is healthy only when READY is full.
    if contains_ascii_case_insensitive(text, "running") {
        if let Some(ready) = readiness_ratio(text) {
            return Some(if ready {
                SemanticSeverity::Success
            } else {
                SemanticSeverity::Warning
            });
        }
    }

    const SUCCESS_TERMS: &[&str] = &[
        " succeeded",
        "succeeded ",
        " completed",
        "completed ",
        " status: healthy",
        "status=healthy",
        " successfully",
        "successfully ",
        " success",
        "ready true",
        "available true",
        "apply complete!",
        "build succeeded",
        "tests passed",
        "test result: ok",
        "finished `dev` profile",
        "finished `release` profile",
        "finished `test` profile",
    ];
    if SUCCESS_TERMS
        .iter()
        .any(|term| contains_ascii_case_insensitive(text, term))
    {
        return Some(SemanticSeverity::Success);
    }

    if is_operational_table_header(text) || is_progress_message(text) {
        return Some(SemanticSeverity::Info);
    }

    None
}

fn structured_log_level(text: &str) -> Option<SemanticSeverity> {
    // Common human and structured logger formats: ERROR:, [WARN],
    // `2026-... INFO ...`, `level=error`, and JSON `"level":"debug"`.
    for token in text.split_whitespace().take(4) {
        let token = token.trim_matches(|c: char| {
            matches!(c, '[' | ']' | '(' | ')' | '{' | '}' | ':' | ',' | '"' | '\'')
        });
        if token.eq_ignore_ascii_case("error")
            || token.eq_ignore_ascii_case("err")
            || token.eq_ignore_ascii_case("err!")
            || token.eq_ignore_ascii_case("fatal")
            || token.eq_ignore_ascii_case("critical")
            || token.eq_ignore_ascii_case("crit")
            || starts_ascii_case_insensitive(token, "error[")
        {
            return Some(SemanticSeverity::Error);
        }
        if token.eq_ignore_ascii_case("warn")
            || token.eq_ignore_ascii_case("warning")
            || token.eq_ignore_ascii_case("wrn")
        {
            return Some(SemanticSeverity::Warning);
        }
        if token.eq_ignore_ascii_case("info")
            || token.eq_ignore_ascii_case("inf")
            || token.eq_ignore_ascii_case("notice")
            || token.eq_ignore_ascii_case("message")
        {
            return Some(SemanticSeverity::Info);
        }
        if token.eq_ignore_ascii_case("debug")
            || token.eq_ignore_ascii_case("dbg")
            || token.eq_ignore_ascii_case("trace")
            || token.eq_ignore_ascii_case("trc")
        {
            return Some(SemanticSeverity::Debug);
        }
    }

    for (needle, severity) in [
        ("level=error", SemanticSeverity::Error),
        ("level=err", SemanticSeverity::Error),
        ("lvl=error", SemanticSeverity::Error),
        ("level=fatal", SemanticSeverity::Error),
        ("severity=error", SemanticSeverity::Error),
        ("\"level\":\"error\"", SemanticSeverity::Error),
        ("\"level\": \"error\"", SemanticSeverity::Error),
        ("level=warn", SemanticSeverity::Warning),
        ("level=wrn", SemanticSeverity::Warning),
        ("lvl=warn", SemanticSeverity::Warning),
        ("level=warning", SemanticSeverity::Warning),
        ("severity=warning", SemanticSeverity::Warning),
        ("\"level\":\"warn\"", SemanticSeverity::Warning),
        ("\"level\": \"warn\"", SemanticSeverity::Warning),
        ("level=info", SemanticSeverity::Info),
        ("level=inf", SemanticSeverity::Info),
        ("lvl=info", SemanticSeverity::Info),
        ("severity=info", SemanticSeverity::Info),
        ("\"level\":\"info\"", SemanticSeverity::Info),
        ("\"level\": \"info\"", SemanticSeverity::Info),
        ("level=debug", SemanticSeverity::Debug),
        ("level=dbg", SemanticSeverity::Debug),
        ("lvl=debug", SemanticSeverity::Debug),
        ("level=trace", SemanticSeverity::Debug),
        ("\"level\":\"debug\"", SemanticSeverity::Debug),
        ("\"level\": \"debug\"", SemanticSeverity::Debug),
    ] {
        if contains_ascii_case_insensitive(text, needle) {
            return Some(severity);
        }
    }
    None
}

fn docker_status(text: &str) -> Option<SemanticSeverity> {
    if contains_ascii_case_insensitive(text, "restarting") {
        return Some(SemanticSeverity::Warning);
    }
    if contains_ascii_case_insensitive(text, "(unhealthy)") {
        return Some(SemanticSeverity::Error);
    }
    if let Some(code) = exited_code(text) {
        return Some(if code == 0 {
            SemanticSeverity::Success
        } else {
            SemanticSeverity::Error
        });
    }
    if contains_ascii_case_insensitive(text, " up ")
        && contains_ascii_case_insensitive(text, "(healthy)")
    {
        return Some(SemanticSeverity::Success);
    }
    None
}

fn exited_code(text: &str) -> Option<i32> {
    let start = find_ascii_case_insensitive(text, "exited (")? + "exited (".len();
    let rest = text.get(start..)?;
    let end = rest.find(')')?;
    rest[..end].trim().parse().ok()
}

fn is_operational_table_header(text: &str) -> bool {
    (contains_ascii_word(text, "NAME")
        && contains_ascii_word(text, "READY")
        && contains_ascii_word(text, "STATUS"))
        || (contains_ascii_case_insensitive(text, "CONTAINER ID")
            && contains_ascii_word(text, "IMAGE")
            && contains_ascii_word(text, "STATUS"))
}

fn is_progress_message(text: &str) -> bool {
    const PREFIXES: &[&str] = &[
        "Compiling ",
        "Checking ",
        "Building ",
        "Downloading ",
        "Downloaded ",
        "Updating ",
        "Pulling ",
        "Pushing ",
        "Creating ",
        "Planning ",
        "Plan: ",
    ];
    PREFIXES
        .iter()
        .any(|prefix| starts_ascii_case_insensitive(text, prefix))
}

fn readiness_ratio(text: &str) -> Option<bool> {
    for token in text.split_whitespace() {
        let token = token.trim_matches(|c: char| !c.is_ascii_digit() && c != '/');
        let Some((ready, total)) = token.split_once('/') else {
            continue;
        };
        let ready: u32 = match ready.parse() {
            Ok(value) => value,
            Err(_) => continue,
        };
        let total: u32 = match total.parse() {
            Ok(value) if value > 0 => value,
            _ => continue,
        };
        return Some(ready == total);
    }
    None
}

fn starts_ascii_case_insensitive(haystack: &str, needle: &str) -> bool {
    haystack
        .get(..needle.len())
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case(needle))
}

fn contains_ascii_word(haystack: &str, needle: &str) -> bool {
    haystack.split_whitespace().any(|token| {
        token
            .trim_matches(|c: char| !c.is_ascii_alphanumeric() && c != '_')
            .eq_ignore_ascii_case(needle)
    })
}

fn find_ascii_case_insensitive(haystack: &str, needle: &str) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }
    let needle = needle.as_bytes();
    haystack
        .as_bytes()
        .windows(needle.len())
        .position(|window| window.eq_ignore_ascii_case(needle))
}

fn contains_ascii_case_insensitive(haystack: &str, needle: &str) -> bool {
    find_ascii_case_insensitive(haystack, needle).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn screenshot_command_not_found_is_error() {
        assert_eq!(
            classify_row_text("version: command not found"),
            Some(SemanticSeverity::Error)
        );
        assert_eq!(
            classify_row_text("'version' is not recognized as an internal or external command"),
            Some(SemanticSeverity::Error)
        );
    }

    #[test]
    fn structured_logs_use_their_declared_level() {
        assert_eq!(classify_row_text("[ERROR] request failed"), Some(SemanticSeverity::Error));
        assert_eq!(classify_row_text("2026-08-10 INFO service started"), Some(SemanticSeverity::Info));
        assert_eq!(classify_row_text("level=warn retry scheduled"), Some(SemanticSeverity::Warning));
        assert_eq!(classify_row_text(r#"{"level":"debug","msg":"poll"}"#), Some(SemanticSeverity::Debug));
        assert_eq!(classify_row_text("[INF] service started"), Some(SemanticSeverity::Info));
        assert_eq!(classify_row_text("[WRN] retry scheduled"), Some(SemanticSeverity::Warning));
        assert_eq!(classify_row_text("[DBG] cache miss"), Some(SemanticSeverity::Debug));
    }


    #[test]
    fn common_developer_failures_are_errors() {
        assert_eq!(
            classify_row_text("error[E0599]: no method named `foo` found"),
            Some(SemanticSeverity::Error)
        );
        assert_eq!(
            classify_row_text("npm ERR! code ELIFECYCLE"),
            Some(SemanticSeverity::Error)
        );
        assert_eq!(
            classify_row_text("FullyQualifiedErrorId : CommandNotFoundException"),
            Some(SemanticSeverity::Error)
        );
        assert_eq!(
            classify_row_text("Error response from daemon: pull access denied"),
            Some(SemanticSeverity::Error)
        );
    }

    #[test]
    fn semantic_classifier_preserves_zero_failure_summaries() {
        assert_ne!(
            classify_row_text("tests completed: 42 passed, 0 failed"),
            Some(SemanticSeverity::Error)
        );
        assert_eq!(classify_row_text("Error from server: Forbidden"), Some(SemanticSeverity::Error));
        assert_eq!(classify_row_text("deployment progressing"), Some(SemanticSeverity::Warning));
        assert_eq!(classify_row_text("apply completed successfully"), Some(SemanticSeverity::Success));
    }

    #[test]
    fn kubernetes_running_readiness_controls_severity() {
        assert_eq!(classify_row_text("api 2/2 Running"), Some(SemanticSeverity::Success));
        assert_eq!(classify_row_text("api 1/2 Running"), Some(SemanticSeverity::Warning));
        assert_eq!(
            classify_row_text("NAME READY STATUS RESTARTS AGE"),
            Some(SemanticSeverity::Info)
        );
    }

    #[test]
    fn docker_statuses_are_semantic() {
        assert_eq!(classify_row_text("web Up 2 minutes (healthy)"), Some(SemanticSeverity::Success));
        assert_eq!(classify_row_text("api Exited (1) 3 seconds ago"), Some(SemanticSeverity::Error));
        assert_eq!(classify_row_text("worker Restarting (1)"), Some(SemanticSeverity::Warning));
    }

    #[test]
    fn cargo_progress_and_finish_are_semantic() {
        assert_eq!(classify_row_text("Compiling sugarloaf v0.5.20"), Some(SemanticSeverity::Info));
        assert_eq!(
            classify_row_text("Finished `release` profile [optimized] target(s) in 2m 41s"),
            Some(SemanticSeverity::Success)
        );
    }

    #[test]
    fn readiness_ratio_ignores_invalid_tokens() {
        assert_eq!(readiness_ratio("x/y 3/3"), Some(true));
        assert_eq!(readiness_ratio("0/0"), None);
        assert_eq!(readiness_ratio("no ratio"), None);
    }
}
