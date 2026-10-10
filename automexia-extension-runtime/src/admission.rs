//! Pure, allocation-free checks at the built-in extension discovery boundary.
//! A manifest or installed marker is never a grant. The application supplies
//! an independent, reviewed capability list for each live built-in.

use std::collections::BTreeMap;
use std::path::Path;

use automexia_extension_api::{Capability, ExtensionManifest, SessionFacts};

pub const MAX_DISCOVERY_TEXT_BYTES: usize = 4096;
const MAX_PUBLIC_HINTS: usize = 20;

/// Exact admission for one reviewed, passive built-in. This does not confer
/// filesystem authority; the application still owns the bounded worker and
/// the provider's explicit local-file policy.
#[derive(Clone, Copy, Debug)]
pub struct PassiveManifestAdmission<'a> {
    extension_id: &'a str,
    reviewed_capabilities: &'a [Capability],
}

impl<'a> PassiveManifestAdmission<'a> {
    pub const fn new(
        extension_id: &'a str,
        reviewed_capabilities: &'a [Capability],
    ) -> Self {
        Self {
            extension_id,
            reviewed_capabilities,
        }
    }

    pub fn admits(&self, manifest: &ExtensionManifest) -> bool {
        let reviewed = self.reviewed_capabilities;
        !self.extension_id.is_empty()
            && manifest.id == self.extension_id
            && !reviewed.is_empty()
            && reviewed.len() <= 2
            && manifest.capabilities == reviewed
            && reviewed.iter().enumerate().all(|(index, capability)| {
                matches!(
                    capability,
                    Capability::FilesystemRead | Capability::EnvironmentRead
                ) && !reviewed[..index].contains(capability)
            })
    }
}

/// Check untrusted pane facts before cloning them into a discovery request.
/// The terminal parser can retain a much larger OSC title or OSC 7 path; those
/// values must never become repeated large renderer/worker allocations.
pub fn discovery_inputs_bounded(
    title: &str,
    cwd: Option<&Path>,
    metadata: [Option<&str>; 5],
    environment: &BTreeMap<String, String>,
) -> bool {
    title.len() <= MAX_DISCOVERY_TEXT_BYTES
        && cwd.is_none_or(|path| path.as_os_str().len() <= MAX_DISCOVERY_TEXT_BYTES)
        && metadata
            .into_iter()
            .flatten()
            .all(|value| value.len() <= MAX_DISCOVERY_TEXT_BYTES)
        && environment.len() <= MAX_PUBLIC_HINTS
        && environment.iter().all(|(name, value)| {
            let location = matches!(
                name.as_str(),
                "HOME" | "KUBECONFIG" | "HOMEDRIVE" | "HOMEPATH" | "USERPROFILE"
            );
            let selector = matches!(
                name.as_str(),
                "DOCKER_CONTEXT"
                    | "DOCKER_HOST_PRESENT"
                    | "AWS_PROFILE"
                    | "AWS_DEFAULT_PROFILE"
                    | "AWS_REGION"
                    | "AWS_DEFAULT_REGION"
                    | "AZURE_CLOUD_NAME"
                    | "CLOUDSDK_ACTIVE_CONFIG_NAME"
                    | "CLOUDSDK_CORE_PROJECT"
                    | "CLOUDSDK_COMPUTE_REGION"
                    | "TF_WORKSPACE"
                    | "AUTOMEXIA_ENV"
                    | "ENVIRONMENT"
                    | "APP_ENV"
                    | "NODE_ENV"
            );
            (location && value.len() <= MAX_DISCOVERY_TEXT_BYTES)
                || (selector
                    && value.len() <= 256
                    && !value.chars().any(char::is_control)
                    && (name != "DOCKER_HOST_PRESENT"
                        || matches!(value.as_str(), "0" | "1")))
        })
}

pub fn session_facts_bounded(session: &SessionFacts) -> bool {
    session
        .os_name
        .as_deref()
        .is_none_or(automexia_extension_api::valid_os_name)
        && discovery_inputs_bounded(
            &session.title,
            session.cwd.as_deref(),
            [
                session.distro.as_deref(),
                session.os_version.as_deref(),
                session.shell_name.as_deref(),
                session.shell_user.as_deref(),
                session.shell_path.as_deref(),
            ],
            &session.environment,
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn oversized_or_foreign_pane_facts_fail_before_worker_clone() {
        let hints = BTreeMap::from([("HOME".to_owned(), "/fixture/home".to_owned())]);
        let small = [
            Some("Linux"),
            Some("1"),
            Some("bash"),
            Some("alice"),
            Some("/bin/bash"),
        ];
        assert!(discovery_inputs_bounded(
            "title",
            Some(Path::new("/fixture")),
            small,
            &hints
        ));
        let huge = "x".repeat(MAX_DISCOVERY_TEXT_BYTES + 1);
        assert!(!discovery_inputs_bounded(&huge, None, small, &hints));
        assert!(!discovery_inputs_bounded(
            "title",
            Some(Path::new(&huge)),
            small,
            &hints
        ));
        assert!(!discovery_inputs_bounded(
            "title",
            None,
            [Some(&huge), None, None, None, None],
            &hints,
        ));
        let foreign = BTreeMap::from([("TOKEN".to_owned(), "secret".to_owned())]);
        assert!(!discovery_inputs_bounded("title", None, small, &foreign));
        let oversized = BTreeMap::from([("HOME".to_owned(), huge)]);
        assert!(!discovery_inputs_bounded("title", None, small, &oversized));
    }

    #[test]
    fn public_selectors_are_bounded_and_provider_paths_are_not_admitted() {
        let metadata = [Some("Ubuntu"), None, Some("bash"), Some("example"), None];
        let mut hints = BTreeMap::from([
            ("HOME".to_owned(), "/fixture/home".to_owned()),
            ("DOCKER_CONTEXT".to_owned(), "review".to_owned()),
            ("AWS_PROFILE".to_owned(), String::new()),
            (
                "CLOUDSDK_CORE_PROJECT".to_owned(),
                "fixture-project".to_owned(),
            ),
        ]);
        assert!(discovery_inputs_bounded("", None, metadata, &hints));
        hints.insert("AWS_PROFILE".into(), "x".repeat(257));
        assert!(!discovery_inputs_bounded("", None, metadata, &hints));
        hints.insert("AWS_PROFILE".into(), String::new());
        hints.insert("AWS_CONFIG_FILE".into(), "/fixture/private".into());
        assert!(!discovery_inputs_bounded("", None, metadata, &hints));
        hints.remove("AWS_CONFIG_FILE");
        hints.insert("DOCKER_HOST_PRESENT".into(), "secret-endpoint".into());
        assert!(!discovery_inputs_bounded("", None, metadata, &hints));
        hints.insert("DOCKER_HOST_PRESENT".into(), "1".into());
        assert!(discovery_inputs_bounded("", None, metadata, &hints));
    }
}
