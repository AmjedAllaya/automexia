//! Allocation-free recognition of plain operational table fields. This is a
//! presentation hint, not a Kubernetes/Docker API parser or a health probe.
use super::{
    contains_ascii_case_insensitive as contains, starts_ascii_case_insensitive as starts,
};
use automexia_extension_api::SemanticSeverity;

pub(super) fn classify(text: &str) -> Option<Option<SemanticSeverity>> {
    if let Some(status) = pod(text) {
        return Some(status);
    }
    condition(text).map(Some).or_else(|| container(text))
}

fn identifier(text: &str) -> bool {
    !text.is_empty()
        && text
            .bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || b"-_.".contains(&c))
}

fn ratio(text: &str) -> Option<(Option<u32>, Option<u32>)> {
    let (ready, total) = text.split_once('/')?;
    if ready.is_empty()
        || total.is_empty()
        || !ready
            .bytes()
            .chain(total.bytes())
            .all(|byte| byte.is_ascii_digit())
    {
        return None;
    }
    Some((ready.parse().ok(), total.parse().ok()))
}

fn pod(text: &str) -> Option<Option<SemanticSeverity>> {
    let mut fields = text.split_whitespace();
    // kubectl/oc default and --all-namespaces; also accept a bare READY STATUS.
    for _ in 0..3 {
        let token = fields.next()?;
        if let Some((ready, total)) = ratio(token) {
            let status = fields.next()?;
            let severity = if status.eq_ignore_ascii_case("Running") {
                Some(if ready.is_some() && ready == total && total != Some(0) {
                    SemanticSeverity::Success
                } else {
                    SemanticSeverity::Warning
                })
            } else if ["Completed", "Succeeded"]
                .iter()
                .any(|value| status.eq_ignore_ascii_case(value))
            {
                Some(SemanticSeverity::Info)
            } else if starts(status, "Init:") {
                let init = &status[5..];
                Some(
                    if failure(init)
                        || starts(init, "ExitCode:")
                            && init[9..].parse::<i32>().is_ok_and(|code| code != 0)
                    {
                        SemanticSeverity::Error
                    } else {
                        SemanticSeverity::Warning
                    },
                )
            } else if failure(status) {
                Some(SemanticSeverity::Error)
            } else if [
                "Pending",
                "ContainerCreating",
                "PodInitializing",
                "Terminating",
                "Waiting",
                "Unknown",
                "NotReady",
                "SchedulingGated",
            ]
            .iter()
            .any(|value| status.eq_ignore_ascii_case(value))
            {
                Some(SemanticSeverity::Warning)
            } else {
                None
            };
            return Some(severity);
        }
        if !identifier(token) && !token.strip_prefix("pod/").is_some_and(identifier) {
            return None;
        }
    }
    None
}

fn failure(status: &str) -> bool {
    [
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
        "DeadlineExceeded",
        "UnexpectedAdmissionError",
        "StartError",
        "PreStartHookError",
        "PostStartHookError",
        "Unhealthy",
        "Evicted",
        "NodeLost",
        "ContainerStatusUnknown",
    ]
    .iter()
    .any(|value| status.eq_ignore_ascii_case(value))
}

fn condition(text: &str) -> Option<SemanticSeverity> {
    let mut words = text.split_whitespace();
    let first = words.next()?;
    let (name, value) = first
        .split_once('=')
        .or_else(|| first.split_once(':'))
        .filter(|(_, value)| !value.is_empty())
        .unwrap_or((first.trim_end_matches(':'), words.next().unwrap_or("")));
    let positive = ["Ready", "Available", "Healthy"]
        .iter()
        .any(|token| name.eq_ignore_ascii_case(token));
    let negative = [
        "DiskPressure",
        "MemoryPressure",
        "PIDPressure",
        "NetworkUnavailable",
    ]
    .iter()
    .any(|token| name.eq_ignore_ascii_case(token));
    if positive || negative {
        if value.eq_ignore_ascii_case("Unknown") {
            return Some(SemanticSeverity::Warning);
        }
        if value.eq_ignore_ascii_case("True") || value.eq_ignore_ascii_case("False") {
            return Some(if value.eq_ignore_ascii_case("True") == positive {
                SemanticSeverity::Success
            } else if negative {
                SemanticSeverity::Error
            } else {
                SemanticSeverity::Warning
            });
        }
    }
    if identifier(first)
        && text.split_whitespace().nth(4).is_some_and(|version| {
            version.starts_with('v')
                && version.as_bytes().get(1).is_some_and(u8::is_ascii_digit)
        })
    {
        let status = text.split_whitespace().nth(1)?;
        if status.eq_ignore_ascii_case("Ready") {
            return Some(SemanticSeverity::Success);
        }
        if [
            "NotReady",
            "Ready,SchedulingDisabled",
            "NotReady,SchedulingDisabled",
        ]
        .iter()
        .any(|value| status.eq_ignore_ascii_case(value))
        {
            return Some(SemanticSeverity::Warning);
        }
    }
    None
}

fn container(text: &str) -> Option<Option<SemanticSeverity>> {
    if text.contains("  ") || text.contains('\t') {
        // Column delimiters keep container names/commands out of STATUS.
        let mut cells = text
            .split("  ")
            .flat_map(|cell| cell.split('\t'))
            .map(str::trim)
            .filter(|cell| !cell.is_empty());
        let first = cells.next()?;
        if (12..=64).contains(&first.len())
            && first.bytes().all(|byte| byte.is_ascii_hexdigit())
        {
            // Default Docker/Podman columns: ID IMAGE COMMAND CREATED STATUS.
            // Neither an image called `dead` nor a name called `unhealthy`
            // may override STATUS, including a future unknown status value.
            return cells.nth(3).map(container_cell);
        }
        return identifier(first)
            .then(|| cells.next().and_then(container_cell))
            .flatten()
            .map(Some);
    }
    if let Some(status) = container_cell(text) {
        return Some(Some(status));
    }
    let (name, rest) = text.split_once(' ')?;
    identifier(name)
        .then(|| container_cell(rest))
        .flatten()
        .map(Some)
}

fn container_cell(text: &str) -> Option<SemanticSeverity> {
    if starts(text, "Exited (") {
        let rest = &text[8..];
        let code = rest
            .split_once(')')
            .and_then(|(code, _)| code.parse::<i32>().ok());
        return Some(match code {
            Some(0) => SemanticSeverity::Info,
            Some(_) => SemanticSeverity::Error,
            None => SemanticSeverity::Warning,
        });
    }
    if starts(text, "Restarting (")
        || text.eq_ignore_ascii_case("Removing")
        || text.eq_ignore_ascii_case("Paused")
    {
        return Some(SemanticSeverity::Warning);
    }
    if text.eq_ignore_ascii_case("Dead") {
        return Some(SemanticSeverity::Error);
    }
    if text.eq_ignore_ascii_case("Created") {
        return Some(SemanticSeverity::Info);
    }
    if starts(text, "Up ") {
        let first = text[3..].split_whitespace().next()?;
        if !first.bytes().all(|byte| byte.is_ascii_digit())
            && !first.eq_ignore_ascii_case("About")
            && !first.eq_ignore_ascii_case("Less")
        {
            return None;
        }
        return Some(if contains(text, "(unhealthy)") {
            SemanticSeverity::Error
        } else if contains(text, "(Paused)") || contains(text, "(health: starting)") {
            SemanticSeverity::Warning
        } else if contains(text, "(healthy)") {
            SemanticSeverity::Success
        } else {
            SemanticSeverity::Info
        });
    }
    None
}
