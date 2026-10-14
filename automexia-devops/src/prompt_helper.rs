//! Restricted Windows prompt protocol shared by the desktop and console entrypoints.
//! No GUI, provider initialization, child process or network client is involved.
use std::io::Write;

use automexia_extension_api::SessionFacts;

use crate::{kubernetes, KubernetesContext};

pub const HELPER_FLAG: &str = "--internal-prompt-context-v2";
pub const MAX_REQUEST: usize = 8192;
pub const MAX_RESPONSE: usize = 4096;

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    session: SessionFacts,
    git: bool,
    kubernetes: bool,
}

#[derive(Default, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Context {
    pub kubernetes: Option<KubernetesContext>,
    pub git_branch: Option<String>,
}

fn discover(request: &Request) -> Context {
    Context {
        git_branch: request
            .git
            .then(|| crate::wsl_git::from_wsl_session(&request.session))
            .flatten(),
        kubernetes: request
            .kubernetes
            .then(|| kubernetes::from_wsl_session(&request.session))
            .flatten(),
    }
}

fn response(request: &str) -> Option<Vec<u8>> {
    if request.len() > MAX_REQUEST {
        return None;
    }
    let request: Request = serde_json::from_str(request).ok()?;
    let bytes = serde_json::to_vec(&discover(&request)).ok()?;
    (bytes.len() <= MAX_RESPONSE).then_some(bytes)
}

/// Dispatch before either entrypoint initializes unrelated application state.
/// Returning None leaves ordinary console commands on their existing path.
pub fn dispatch_helper() -> Option<i32> {
    let mut arguments = std::env::args_os().skip(1);
    if arguments.next().as_deref() != Some(std::ffi::OsStr::new(HELPER_FLAG)) {
        return None;
    }
    let result = (|| {
        let request = arguments.next()?;
        if arguments.next().is_some() {
            return None;
        }
        let bytes = response(request.to_str()?)?;
        std::io::stdout().lock().write_all(&bytes).ok()
    })();
    // No path, parser excerpt or error text escapes the restricted protocol.
    Some(if result.is_some() { 0 } else { 2 })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prompt_helper_rejects_oversized_and_malformed_requests() {
        for value in [
            "x".repeat(MAX_REQUEST + 1),
            "{}".into(),
            r#"{"session":{},"git":true}"#.into(),
        ] {
            assert!(response(&value).is_none());
        }
    }

    #[test]
    fn prompt_helper_disabled_scopes_do_not_read_guest_paths() {
        let session: SessionFacts = serde_json::from_value(serde_json::json!({
            "session_id": 1, "cwd": "/nonexistent", "title": "", "distro": "Fixture-Distro",
            "shell_name": "bash", "shell_user": "fixture", "shell_path": "/bin/bash",
            "shell_integration": true, "shell_pid": 1,
        })).unwrap();
        let result = discover(&Request {
            session,
            git: false,
            kubernetes: false,
        });
        assert!(result.git_branch.is_none() && result.kubernetes.is_none());
        assert!(serde_json::from_str::<Context>(
            r#"{"git_branch":null,"kubernetes":null,"extra":1}"#
        )
        .is_err());
    }
}
