use automexia_extension_api::SemanticSeverity;

mod workloads;

const MAX_ROW_BYTES: usize = 32 * 1024;

#[cfg(test)]
#[path = "semantics_tests.rs"]
mod status_tests;

/// Classify one visible terminal row without allocating and without mutating
/// terminal bytes. Explicit application ANSI colors still win in grid_emit.
pub fn classify_row_text(text: &str) -> Option<SemanticSeverity> {
    if text.len() > MAX_ROW_BYTES {
        return None;
    }
    let text = text.trim_matches('\0').trim();
    if text.is_empty() {
        return None;
    }

    // Read known table fields before scanning prose: resource names and image
    // tags are not health signals. A recognized but unknown status stays neutral.
    if let Some(status) = workloads::classify(text) {
        return status;
    }

    if let Some(level) = structured_log_level(text) {
        return Some(level);
    }

    if has_failure_count_or_word(text) {
        return Some(SemanticSeverity::Error);
    }

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
        " failure",
        "unsuccessful",
        "unsuccessfully",
        "not successful",
        "not completed",
    ];
    if ERROR_TERMS
        .iter()
        .any(|term| contains_phrase(text, term.trim()))
    {
        return Some(SemanticSeverity::Error);
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
        "throttle",
        "throttled",
        "throttling",
        "unknown",
    ];
    if WARNING_TERMS
        .iter()
        .any(|term| contains_phrase(text, term.trim()))
    {
        return Some(SemanticSeverity::Warning);
    }

    const SUCCESS_TERMS: &[&str] = &[
        "tests completed",
        "test completed",
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
        .any(|term| contains_phrase(text, term.trim()))
    {
        return Some(SemanticSeverity::Success);
    }

    // A wrapped status can lose its READY column. Completion alone still does
    // not prove readiness; explicit successful command summaries above do.
    if contains_phrase(text, "Completed")
        || contains_phrase(text, "Succeeded")
        || is_operational_table_header(text)
        || is_progress_message(text)
    {
        return Some(SemanticSeverity::Info);
    }

    None
}

fn structured_log_level(text: &str) -> Option<SemanticSeverity> {
    // Common human and structured logger formats: ERROR:, [WARN],
    // `2026-... INFO ...`, `level=error`, and JSON `"level":"debug"`.
    for token in text.split_whitespace().take(4) {
        let token = token.trim_matches(|c: char| {
            matches!(
                c,
                '[' | ']' | '(' | ')' | '{' | '}' | ':' | ',' | '"' | '\''
            )
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
        // Only a leading level or a level following timestamp fields is a
        // declaration. Ordinary prose containing "info" is not a log prefix.
        if !token.bytes().any(|byte| byte.is_ascii_digit())
            || !token.bytes().any(|byte| b"-:./+TZtz".contains(&byte))
            || !token
                .bytes()
                .all(|byte| byte.is_ascii_digit() || b"-:./+TZtz".contains(&byte))
        {
            break;
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
        if contains_phrase(text, needle) {
            return Some(severity);
        }
    }
    None
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
        "Terraform will perform ",
    ];
    PREFIXES
        .iter()
        .any(|prefix| starts_ascii_case_insensitive(text, prefix))
}

fn has_failure_count_or_word(text: &str) -> bool {
    let mut tokens = text
        .split(|c: char| {
            c.is_whitespace() || matches!(c, ',' | ';' | ':' | '=' | '(' | ')')
        })
        .filter(|token| !token.is_empty())
        .peekable();
    let mut previous = "";
    while let Some(token) = tokens.next() {
        if ["failed", "error", "errors", "failures"]
            .iter()
            .any(|word| token.eq_ignore_ascii_case(word))
        {
            // In `10 failed, 0 errors`, the following zero belongs to errors.
            // Prefer an immediately preceding count; overflowing counts cannot
            // turn into zero through a failed integer conversion.
            let numeric_zero = |value: &str| {
                (!value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit()))
                    .then(|| value.bytes().all(|byte| byte == b'0'))
            };
            let is_zero = numeric_zero(previous)
                .or_else(|| tokens.peek().and_then(|next| numeric_zero(next)));
            if is_zero != Some(true) {
                return true;
            }
        }
        previous = token;
    }
    false
}

fn contains_phrase(text: &str, phrase: &str) -> bool {
    let word = |byte: u8| {
        !byte.is_ascii() || byte.is_ascii_alphanumeric() || b"_-/".contains(&byte)
    };
    let bytes = text.as_bytes();
    bytes
        .windows(phrase.len())
        .enumerate()
        .any(|(start, part)| {
            // Reject interior bytes before comparing the phrase. Long ordinary
            // words otherwise pay for every vocabulary comparison at every byte.
            (start == 0 || !word(bytes[start - 1]))
                && part.eq_ignore_ascii_case(phrase.as_bytes())
                && (start + part.len() == bytes.len() || !word(bytes[start + part.len()]))
        })
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
            classify_row_text(
                "'version' is not recognized as an internal or external command"
            ),
            Some(SemanticSeverity::Error)
        );
    }

    #[test]
    fn structured_logs_use_their_declared_level() {
        assert_eq!(
            classify_row_text("[ERROR] request failed"),
            Some(SemanticSeverity::Error)
        );
        assert_eq!(
            classify_row_text("2026-08-10 INFO service started"),
            Some(SemanticSeverity::Info)
        );
        assert_eq!(
            classify_row_text("level=warn retry scheduled"),
            Some(SemanticSeverity::Warning)
        );
        assert_eq!(
            classify_row_text(r#"{"level":"debug","msg":"poll"}"#),
            Some(SemanticSeverity::Debug)
        );
        assert_eq!(
            classify_row_text("[INF] service started"),
            Some(SemanticSeverity::Info)
        );
        assert_eq!(
            classify_row_text("[WRN] retry scheduled"),
            Some(SemanticSeverity::Warning)
        );
        assert_eq!(
            classify_row_text("[DBG] cache miss"),
            Some(SemanticSeverity::Debug)
        );
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
        assert_eq!(
            classify_row_text("Error from server: Forbidden"),
            Some(SemanticSeverity::Error)
        );
        assert_eq!(
            classify_row_text("deployment progressing"),
            Some(SemanticSeverity::Warning)
        );
        assert_eq!(
            classify_row_text("apply completed successfully"),
            Some(SemanticSeverity::Success)
        );
    }

    #[test]
    fn kubernetes_running_readiness_controls_severity() {
        assert_eq!(
            classify_row_text("api 2/2 Running"),
            Some(SemanticSeverity::Success)
        );
        assert_eq!(
            classify_row_text("api 1/2 Running"),
            Some(SemanticSeverity::Warning)
        );
        assert_eq!(
            classify_row_text("NAME READY STATUS RESTARTS AGE"),
            Some(SemanticSeverity::Info)
        );
    }

    #[test]
    fn docker_statuses_are_semantic() {
        assert_eq!(
            classify_row_text("web Up 2 minutes (healthy)"),
            Some(SemanticSeverity::Success)
        );
        assert_eq!(
            classify_row_text("api Exited (1) 3 seconds ago"),
            Some(SemanticSeverity::Error)
        );
        assert_eq!(
            classify_row_text("worker Restarting (1)"),
            Some(SemanticSeverity::Warning)
        );
    }

    #[test]
    fn cargo_progress_and_finish_are_semantic() {
        assert_eq!(
            classify_row_text("Compiling sugarloaf v0.5.20"),
            Some(SemanticSeverity::Info)
        );
        assert_eq!(
            classify_row_text(
                "Finished `release` profile [optimized] target(s) in 2m 41s"
            ),
            Some(SemanticSeverity::Success)
        );
    }

    #[test]
    fn readiness_ratio_ignores_invalid_tokens() {
        assert_eq!(classify_row_text("x/y 3/3"), None);
        assert_eq!(classify_row_text("0/0"), None);
        assert_eq!(classify_row_text("no ratio"), None);
    }
}
