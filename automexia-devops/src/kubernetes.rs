//! Passive kubeconfig projection. Only context names and namespaces survive parsing.
//!
//! This differs deliberately from import review: native kubeconfig merging uses
//! the first source's complete named context, including an omitted namespace.
//! Credentials, exec configuration and referenced files are never activated.

use serde::Deserialize;

use crate::{sanitize_label, KubernetesContext};

pub const MAX_BYTES: usize = 1024 * 1024;
pub const MAX_FILES: usize = 16;
const MAX_CONTEXTS: usize = 256;

/// Guest identity comes from the integrated session, never its mutable title.
pub(crate) fn is_wsl_candidate(session: &automexia_extension_api::SessionFacts) -> bool {
    session.distro.as_ref().is_some_and(|name| !name.is_empty())
        && !session.shell_name.as_deref().is_some_and(|name| {
            name.eq_ignore_ascii_case("PowerShell")
                || name.eq_ignore_ascii_case("CMD")
                || name.eq_ignore_ascii_case("Command Prompt")
        })
}

pub fn is_wsl_session(session: &automexia_extension_api::SessionFacts) -> bool {
    session.shell_integration
        && session.distro.as_ref().is_some_and(|name| !name.is_empty())
        && session
            .cwd
            .as_ref()
            .is_some_and(|cwd| cwd.to_string_lossy().starts_with('/'))
        && session.shell_name.as_deref().is_some_and(|name| {
            matches!(name, "bash" | "zsh" | "fish" | "sh" | "dash" | "ksh")
        })
}

fn safe_component(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 96
        && value != "."
        && value != ".."
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._-".contains(&byte))
}

/// Pure WSL path translation; no provider lookup and no host-home fallback.
pub fn wsl_paths(
    session: &automexia_extension_api::SessionFacts,
) -> Option<Vec<std::path::PathBuf>> {
    if !is_wsl_session(session) {
        return None;
    }
    let distro = session.distro.as_deref()?;
    if !safe_component(distro) {
        return None;
    }
    let configured = session
        .environment
        .get("KUBECONFIG")
        .filter(|value| !value.is_empty());
    // An explicit override does not depend on HOME or a conventional username.
    let default;
    let raw = if let Some(configured) = configured {
        configured.as_str()
    } else {
        let home = match session.environment.get("HOME") {
            Some(home) => home.clone(),
            None => {
                let user = session.shell_user.as_deref()?;
                if !safe_component(user) {
                    return None;
                }
                if user == "root" {
                    "/root".to_owned()
                } else {
                    format!("/home/{user}")
                }
            }
        };
        if !home.starts_with('/') || home.len() > 4096 {
            return None;
        }
        default = format!("{home}/.kube/config");
        &default
    };
    if raw.len() > 4096 {
        return None;
    }
    let mut paths = Vec::new();
    for value in raw.split(':').filter(|value| !value.is_empty()) {
        let absolute = if value.starts_with('/') {
            value.to_owned()
        } else {
            format!("{}/{}", session.cwd.as_ref()?.to_str()?, value)
        };
        if !absolute.starts_with('/')
            || absolute.starts_with("//")
            || absolute.contains('\\')
            || absolute.chars().any(char::is_control)
        {
            return None;
        }
        let components = absolute
            .split('/')
            .filter(|part| !part.is_empty() && *part != ".")
            .collect::<Vec<_>>();
        if components
            .iter()
            .any(|part| *part == ".." || part.contains(':'))
        {
            return None;
        }
        let path = std::path::PathBuf::from(format!(
            "\\\\wsl.localhost\\{distro}\\{}",
            components.join("\\")
        ));
        if !paths.contains(&path) {
            paths.push(path);
        }
        if paths.len() > MAX_FILES {
            return None;
        }
    }
    (!paths.is_empty()).then_some(paths)
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn local_paths(
    session: &automexia_extension_api::SessionFacts,
    host_home: Option<&std::path::Path>,
) -> Vec<std::path::PathBuf> {
    let configured = session
        .environment
        .get("KUBECONFIG")
        .map(std::ffi::OsString::from)
        .or_else(|| std::env::var_os("KUBECONFIG"));
    if let Some(configured) = configured.filter(|value| !value.is_empty()) {
        if configured.len() > 4096 {
            return Vec::new();
        }
        let mut paths = Vec::new();
        for path in
            std::env::split_paths(&configured).filter(|path| !path.as_os_str().is_empty())
        {
            let path = if path.is_relative() {
                let Some(cwd) = session.cwd.as_ref() else {
                    return Vec::new();
                };
                cwd.join(path)
            } else {
                path
            };
            if !bounded_local_path(&path) {
                return Vec::new();
            }
            // Native client-go de-duplicates the configured list before loading.
            // Repeated files must not consume the unique-source budget.
            if !paths.contains(&path) {
                paths.push(path);
                if paths.len() > MAX_FILES {
                    return Vec::new();
                }
            }
        }
        return paths;
    }
    #[cfg(windows)]
    if windows_location_hints(session) || !session.environment.contains_key("HOME") {
        return windows_default_paths(session, host_home);
    }
    let home = session
        .environment
        .get("HOME")
        .map(std::path::Path::new)
        .or(host_home);
    home.filter(|path| bounded_local_path(path))
        .map(|home| vec![home.join(".kube/config")])
        .unwrap_or_default()
}

#[cfg(windows)]
pub(crate) fn windows_location_hints(
    session: &automexia_extension_api::SessionFacts,
) -> bool {
    ["HOMEDRIVE", "HOMEPATH", "USERPROFILE"]
        .iter()
        .all(|name| session.environment.contains_key(*name))
}

#[cfg(windows)]
fn windows_default_paths(
    session: &automexia_extension_api::SessionFacts,
    host_home: Option<&std::path::Path>,
) -> Vec<std::path::PathBuf> {
    use std::ffi::OsString;
    let published = windows_location_hints(session);
    let value = |name: &str| {
        if published {
            session.environment.get(name).map(OsString::from)
        } else {
            std::env::var_os(name)
        }
        .filter(|value| !value.is_empty())
    };
    let drive_path =
        value("HOMEDRIVE")
            .zip(value("HOMEPATH"))
            .map(|(mut drive, path)| {
                drive.push(path);
                drive
            });
    let profile = value("USERPROFILE").or_else(|| {
        (!published)
            .then(|| host_home.map(|home| home.as_os_str().to_owned()))
            .flatten()
    });
    // Match the client-go existing-config order, without shell hot-path I/O.
    // Do not merge default homes; the first existing config is authoritative.
    let homes = [value("HOME"), drive_path, profile]
        .into_iter()
        .flatten()
        .map(std::path::PathBuf::from)
        .collect::<Vec<_>>();
    if homes.iter().any(|home| !bounded_local_path(home)) {
        return Vec::new();
    }
    homes
        .into_iter()
        .map(|home| home.join(".kube/config"))
        .find(|path| path.exists())
        .map(|path| vec![path])
        .unwrap_or_default()
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn bounded_local_path(path: &std::path::Path) -> bool {
    path.is_absolute()
        && path.as_os_str().len() <= 4096
        && !path.to_string_lossy().chars().any(char::is_control)
        && local_config_path(path)
}

#[cfg(not(target_arch = "wasm32"))]
fn local_config_path(path: &std::path::Path) -> bool {
    #[cfg(windows)]
    {
        use std::path::{Component, Prefix};
        // Terminal output cannot turn passive local discovery into an implicit
        // SMB authentication attempt or a device-namespace open.
        !matches!(path.components().next(), Some(Component::Prefix(prefix))
            if !matches!(prefix.kind(), Prefix::Disk(_) | Prefix::VerbatimDisk(_)))
    }
    #[cfg(not(windows))]
    {
        !path.to_string_lossy().starts_with("//")
    }
}

#[cfg(all(test, windows))]
#[test]
fn passive_native_paths_reject_network_and_device_authority() {
    for value in [
        r"\\example.invalid\share\config",
        r"\\?\UNC\example.invalid\share\config",
        r"\\.\pipe\fixture",
        r"\\?\GLOBALROOT\Device\fixture",
    ] {
        assert!(!local_config_path(std::path::Path::new(value)));
    }
    for value in [
        r"D:\fixture\config",
        r"\\?\D:\fixture\config",
        "fixture/config",
    ] {
        assert!(local_config_path(std::path::Path::new(value)));
    }
}

#[derive(Default, Deserialize)]
struct Document {
    #[serde(default, rename = "current-context")]
    current: Option<String>,
    #[serde(default)]
    contexts: Option<Vec<NamedContext>>,
}

#[derive(Deserialize)]
struct NamedContext {
    name: String,
    #[serde(default)]
    context: Context,
}

#[derive(Default, Deserialize)]
struct Context {
    #[serde(default)]
    namespace: Option<String>,
}

fn parse(content: &str) -> Option<Document> {
    if content.len() > MAX_BYTES {
        return None;
    }
    let options = serde_saphyr::options! {
        strict_booleans: true,
        merge_keys: serde_saphyr::MergeKeyPolicy::Error,
        budget: serde_saphyr::budget! {
            max_reader_input_bytes: Some(MAX_BYTES),
            max_events: 20_000,
            max_aliases: 64,
            max_anchors: 64,
            max_depth: 32,
            max_documents: 1,
            max_nodes: 8_192,
            max_total_scalar_bytes: MAX_BYTES,
            max_total_comment_bytes: MAX_BYTES,
            max_merge_keys: 0,
        },
    };
    // Discard parser diagnostics: they can quote a credential-bearing source line.
    let document: Document =
        serde_saphyr::from_str_with_options(content, options).ok()?;
    if let Some(contexts) = &document.contexts {
        if contexts.len() > MAX_CONTEXTS {
            return None;
        }
        let mut names = std::collections::HashSet::new();
        if contexts.iter().any(|context| !names.insert(&context.name)) {
            return None;
        }
    }
    Some(document)
}

fn namespace(document: &Document, wanted: &str) -> Option<String> {
    let context = document
        .contexts
        .as_ref()?
        .iter()
        .find(|item| item.name == wanted)?;
    Some(
        context
            .context
            .namespace
            .as_deref()
            .filter(|value| !value.is_empty())
            .unwrap_or("default")
            .to_owned(),
    )
}

/// Resolve already bounded local documents in native first-source-wins order.
/// Invalid sources fail closed rather than silently selecting a different cluster.
pub fn from_documents(documents: &[&str]) -> Option<KubernetesContext> {
    if documents.len() > MAX_FILES {
        return None;
    }
    let parsed = documents
        .iter()
        .map(|content| parse(content))
        .collect::<Option<Vec<_>>>()?;
    let current = parsed
        .iter()
        .filter_map(|doc| doc.current.as_deref())
        .find(|value| !value.is_empty())?;
    let namespace = parsed.iter().find_map(|doc| namespace(doc, current))?;
    let context = sanitize_label(current);
    let namespace = sanitize_label(&namespace);
    if context.is_empty() || namespace.is_empty() {
        return None;
    }
    Some(KubernetesContext { context, namespace })
}

/// Read at most sixteen bounded local files. Call only on a discovery worker;
/// foreign/provider-backed paths require application-owned process isolation.
#[cfg(not(target_arch = "wasm32"))]
pub fn from_files(paths: &[std::path::PathBuf]) -> Option<KubernetesContext> {
    use std::io::Read;
    if paths.len() > MAX_FILES {
        return None;
    }
    let mut documents = Vec::with_capacity(paths.len());
    for path in paths {
        let file = match std::fs::File::open(path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(_) => return None,
        };
        let metadata = file.metadata().ok()?;
        if !metadata.is_file() || metadata.len() > MAX_BYTES as u64 {
            return None;
        }
        let mut content = String::new();
        file.take((MAX_BYTES + 1) as u64)
            .read_to_string(&mut content)
            .ok()?;
        if content.len() > MAX_BYTES {
            return None;
        }
        documents.push(content);
    }
    from_documents(&documents.iter().map(String::as_str).collect::<Vec<_>>())
}

#[cfg(test)]
pub(crate) fn namespace_for_context(content: &str, wanted: &str) -> Option<String> {
    namespace(&parse(content)?, wanted)
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(128))]
        #[test]
        fn hostile_bounded_documents_never_publish_controls_or_unbounded_labels(input in ".{0,4096}") {
            if let Some(context) = from_documents(&[&input]) {
                for label in [&context.context, &context.namespace] {
                    prop_assert!(!label.is_empty());
                    prop_assert!(label.chars().count() <= 96);
                    prop_assert_eq!(sanitize_label(label), label.as_str());
                }
            }
        }

        #[test]
        fn namespace_projection_preserves_literal_safe_values(namespace in "[a-z][a-z0-9-]{0,62}") {
            let text = format!(r#"{{"current-context":"fixture","contexts":[{{"context":{{"namespace":"{namespace}"}},"name":"fixture"}}]}}"#);
            prop_assert_eq!(from_documents(&[&text]).unwrap().namespace, namespace);
        }
    }

    #[test]
    fn first_named_context_wins_even_without_namespace() {
        let first = "current-context: fixture\ncontexts: [{name: fixture, context: {}}]";
        let second = "contexts: [{name: fixture, context: {namespace: wrong}}]";
        assert_eq!(
            from_documents(&[first, second]).unwrap().namespace,
            "default"
        );
    }

    #[test]
    fn context_and_current_selection_can_be_in_different_files() {
        let current = "current-context: fixture";
        let context = "contexts: [{context: {namespace: sandbox}, name: fixture}]";
        assert_eq!(
            from_documents(&[current, context]),
            Some(KubernetesContext {
                context: "fixture".into(),
                namespace: "sandbox".into(),
            })
        );
        assert_eq!(from_documents(&[current]), None);
        assert_eq!(from_documents(&[]), None);
    }

    #[test]
    fn malformed_complex_and_oversized_documents_fail_closed() {
        for content in [
            "current-context: [",
            "current-context: one\ncurrent-context: two",
            "current-context: fixture\n---\ncurrent-context: other",
            "contexts: [{name: fixture, context: {namespace: one, namespace: two}}]",
            "contexts: [{name: fixture, context: {}}, {name: fixture, context: {namespace: other}}]",
        ] {
            assert!(parse(content).is_none());
        }
        assert!(parse(&" ".repeat(MAX_BYTES + 1)).is_none());
        assert!(from_documents(&vec!["{}"; MAX_FILES + 1]).is_none());
        // Distinct names isolate the count ceiling from duplicate rejection.
        for count in [255, 256, 257] {
            let entries = (0..count)
                .map(|index| format!("{{name: fixture-{index}}}"))
                .collect::<Vec<_>>()
                .join(",");
            assert_eq!(
                parse(&format!("contexts: [{entries}]")).is_some(),
                count <= 256
            );
        }
        let valid = "current-context: fixture\ncontexts: [{name: fixture}]";
        for count in [15, 16, 17] {
            let mut documents = vec!["{}"; count];
            documents[0] = valid;
            assert_eq!(from_documents(&documents).is_some(), count <= 16);
        }
    }

    #[test]
    fn credentials_are_ignored_not_projected_or_activated() {
        let content = "current-context: fixture\ncontexts: [{name: fixture, context: {namespace: sandbox}}]\nusers:\n- name: fixture\n  user:\n    token: fixture-secret-canary\n    exec: {command: never-run-this-fixture}\n    client-key: /fictional/do-not-open\n";
        assert!(content.contains("fixture-secret-canary"));
        let projected = from_documents(&[content]).unwrap();
        assert_eq!(
            projected,
            KubernetesContext {
                context: "fixture".into(),
                namespace: "sandbox".into()
            }
        );
        assert!(!format!("{projected:?}").contains("fixture-secret-canary"));
    }
}
