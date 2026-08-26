use std::{collections::BTreeSet, fmt};

use serde::{Deserialize, Serialize};

use crate::{portable_identifier, safe_text, Limits};

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "kebab-case")]
pub enum Capability {
    PublicMetadataRead,
    SelectedInputReadOnce,
    SuggestionPublish,
    StructuredOperationPropose,
    DiagnosticPublishRedacted,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GrantBinding {
    pub publisher_id: String,
    pub extension_id: String,
    pub version: String,
    pub package_sha256: String,
    pub capability: Capability,
    pub exact_scope: String,
    pub profile_id: String,
    pub expires_at_unix: u64,
    pub generation: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InvocationBinding<'a> {
    pub publisher_id: &'a str,
    pub extension_id: &'a str,
    pub version: &'a str,
    pub package_sha256: &'a str,
    pub capability: Capability,
    pub exact_scope: &'a str,
    pub profile_id: &'a str,
    pub now_unix: u64,
    pub generation: u64,
    pub revoked: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GrantError {
    Malformed,
    Expired,
    Revoked,
    StaleGeneration,
    BindingMismatch,
}

impl fmt::Display for GrantError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Malformed => "grant is malformed",
            Self::Expired => "grant expired",
            Self::Revoked => "extension or package is revoked",
            Self::StaleGeneration => "grant generation is stale",
            Self::BindingMismatch => "grant does not match the exact invocation",
        })
    }
}

impl std::error::Error for GrantError {}

impl GrantBinding {
    pub fn validate(&self) -> Result<(), GrantError> {
        if !portable_identifier(&self.publisher_id)
            || !portable_identifier(&self.extension_id)
            || !valid_digest(&self.package_sha256)
            || !portable_identifier(&self.profile_id)
            || !safe_text(&self.exact_scope, false)
            || self.exact_scope.len() > 512
            || self.expires_at_unix == 0
            || self.generation == 0
        {
            return Err(GrantError::Malformed);
        }
        Ok(())
    }

    pub fn authorize(
        &self,
        invocation: &InvocationBinding<'_>,
    ) -> Result<(), GrantError> {
        self.validate()?;
        if invocation.revoked {
            return Err(GrantError::Revoked);
        }
        if invocation.now_unix >= self.expires_at_unix {
            return Err(GrantError::Expired);
        }
        if invocation.generation != self.generation {
            return Err(GrantError::StaleGeneration);
        }
        if self.publisher_id != invocation.publisher_id
            || self.extension_id != invocation.extension_id
            || self.version != invocation.version
            || self.package_sha256 != invocation.package_sha256
            || self.capability != invocation.capability
            || self.exact_scope != invocation.exact_scope
            || self.profile_id != invocation.profile_id
        {
            return Err(GrantError::BindingMismatch);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapabilityDiff {
    pub added: Vec<Capability>,
    pub removed: Vec<Capability>,
    pub unchanged: Vec<Capability>,
    pub fresh_review_required: bool,
}

pub fn capability_diff(previous: &[Capability], next: &[Capability]) -> CapabilityDiff {
    let previous = previous.iter().copied().collect::<BTreeSet<_>>();
    let next = next.iter().copied().collect::<BTreeSet<_>>();
    CapabilityDiff {
        added: next.difference(&previous).copied().collect(),
        removed: previous.difference(&next).copied().collect(),
        unchanged: previous.intersection(&next).copied().collect(),
        fresh_review_required: previous != next,
    }
}

pub fn validate_capabilities(capabilities: &[Capability]) -> bool {
    capabilities.len() <= Limits::CAPABILITY_REQUESTS
        && capabilities.iter().copied().collect::<BTreeSet<_>>().len()
            == capabilities.len()
}

pub fn valid_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grant() -> GrantBinding {
        GrantBinding {
            publisher_id: "example.publisher".into(),
            extension_id: "example.extension".into(),
            version: "1.0.0".into(),
            package_sha256: "a".repeat(64),
            capability: Capability::PublicMetadataRead,
            exact_scope: "profile/dev/pane/7".into(),
            profile_id: "dev".into(),
            expires_at_unix: 200,
            generation: 4,
        }
    }

    #[test]
    fn grants_deny_expiry_revocation_stale_generation_and_cross_scope_replay() {
        let grant = grant();
        fn invocation(
            scope: &str,
            now: u64,
            generation: u64,
            revoked: bool,
        ) -> InvocationBinding<'_> {
            InvocationBinding {
                publisher_id: "example.publisher",
                extension_id: "example.extension",
                version: "1.0.0",
                package_sha256:
                    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                capability: Capability::PublicMetadataRead,
                exact_scope: scope,
                profile_id: "dev",
                now_unix: now,
                generation,
                revoked,
            }
        }
        grant
            .authorize(&invocation("profile/dev/pane/7", 100, 4, false))
            .unwrap();
        assert_eq!(
            grant.authorize(&invocation("profile/dev/pane/8", 100, 4, false)),
            Err(GrantError::BindingMismatch)
        );
        assert_eq!(
            grant.authorize(&invocation("profile/dev/pane/7", 200, 4, false)),
            Err(GrantError::Expired)
        );
        assert_eq!(
            grant.authorize(&invocation("profile/dev/pane/7", 100, 5, false)),
            Err(GrantError::StaleGeneration)
        );
        assert_eq!(
            grant.authorize(&invocation("profile/dev/pane/7", 100, 4, true)),
            Err(GrantError::Revoked)
        );
    }
}
