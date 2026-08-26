//! Product-owned, cached-only provider catalog composition.
//!
//! Provider extensions retain parsing and exact-operation ownership. This
//! module accepts only an already validated public capsule and publishes a
//! bounded immutable catalog for the Connection Hub. It owns no process,
//! network, browser, credential, PTY, or provider-filesystem authority.

use std::{fmt, sync::Arc};

use automexia_connectivity::connections::{
    validate_provider_capsule, AuthState, EnvironmentRisk, ProviderCapsule,
    ProviderContextFreshness, ProviderContextTemplate, ProviderKind,
    ProviderRecoveryAction,
};
use automexia_ui_model::connection_hub::ProviderCatalogItem;

pub const MAX_PRODUCT_PROVIDERS: usize = 6;
pub const MAX_PROVIDER_SCOPE_SUMMARY_BYTES: usize = 512;
pub const PROVIDER_ACTIVATION_BLOCKER: &str =
    "Protected activation, executable attestation, and native evidence are pending";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ProviderDescriptor {
    provider: ProviderKind,
    extension_id: &'static str,
    executable_id: &'static str,
}

const PROVIDERS: [ProviderDescriptor; MAX_PRODUCT_PROVIDERS] = [
    ProviderDescriptor {
        provider: ProviderKind::Aws,
        extension_id: automexia_devops_aws::ID,
        executable_id: automexia_devops_aws::AWS_EXECUTABLE_ID,
    },
    ProviderDescriptor {
        provider: ProviderKind::Azure,
        extension_id: automexia_devops_azure::ID,
        executable_id: automexia_devops_azure::AZ_EXECUTABLE_ID,
    },
    ProviderDescriptor {
        provider: ProviderKind::Gcp,
        extension_id: automexia_devops_gcp::ID,
        executable_id: automexia_devops_gcp::GCLOUD_EXECUTABLE_ID,
    },
    ProviderDescriptor {
        provider: ProviderKind::Kubernetes,
        extension_id: automexia_devops_kubernetes::ID,
        executable_id: automexia_devops_kubernetes::KUBECTL_EXECUTABLE_ID,
    },
    ProviderDescriptor {
        provider: ProviderKind::OpenShift,
        extension_id: automexia_devops_openshift::ID,
        executable_id: automexia_devops_openshift::OC_EXECUTABLE_ID,
    },
    ProviderDescriptor {
        provider: ProviderKind::Teleport,
        extension_id: automexia_devops_teleport::ID,
        executable_id: automexia_devops_teleport::TSH_EXECUTABLE_ID,
    },
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProviderProductErrorCode {
    InvalidCapsule,
    UnsupportedProvider,
    StalePublication,
    CrossSessionPublication,
    RuntimeUnavailable,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProviderProductError {
    code: ProviderProductErrorCode,
}

impl ProviderProductError {
    const fn new(code: ProviderProductErrorCode) -> Self {
        Self { code }
    }

    pub const fn code(&self) -> ProviderProductErrorCode {
        self.code
    }

    pub(crate) const fn runtime_unavailable() -> Self {
        Self::new(ProviderProductErrorCode::RuntimeUnavailable)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProviderProductSnapshot {
    pub revision: u64,
    pub capsule_id: Option<String>,
    pub session_id: Option<u64>,
    pub capsule_revision: Option<u64>,
    pub catalog: Arc<Vec<ProviderCatalogItem>>,
    publication: Option<ProviderProductPublication>,
}

#[derive(Clone, PartialEq, Eq)]
pub struct ProviderProductPublication {
    generation: u64,
    capsule: Arc<ProviderCapsule>,
}

impl ProviderProductPublication {
    pub const fn generation(&self) -> u64 {
        self.generation
    }

    pub fn capsule(&self) -> &ProviderCapsule {
        &self.capsule
    }
}

impl fmt::Debug for ProviderProductPublication {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ProviderProductPublication")
            .field("generation", &self.generation)
            .field("capsule", &"<redacted-public-provider-capsule>")
            .finish()
    }
}

impl Default for ProviderProductSnapshot {
    fn default() -> Self {
        Self {
            revision: 0,
            capsule_id: None,
            session_id: None,
            capsule_revision: None,
            catalog: Arc::new(compose_catalog(None)),
            publication: None,
        }
    }
}

impl ProviderProductSnapshot {
    pub fn publication(&self) -> Option<&ProviderProductPublication> {
        self.publication.as_ref()
    }

    pub fn publish(
        &self,
        capsule: &ProviderCapsule,
    ) -> Result<Self, ProviderProductError> {
        validate_provider_capsule(capsule).map_err(|_| {
            ProviderProductError::new(ProviderProductErrorCode::InvalidCapsule)
        })?;
        if capsule
            .contexts
            .iter()
            .any(|context| !supports_product_publication(context.provider))
        {
            return Err(ProviderProductError::new(
                ProviderProductErrorCode::UnsupportedProvider,
            ));
        }
        if let Some(active_capsule_id) = self.capsule_id.as_deref() {
            if active_capsule_id == capsule.capsule_id {
                return Err(ProviderProductError::new(
                    ProviderProductErrorCode::StalePublication,
                ));
            }
            if self.session_id == Some(capsule.session_id) {
                return Err(ProviderProductError::new(
                    ProviderProductErrorCode::CrossSessionPublication,
                ));
            }
            if self
                .capsule_revision
                .is_some_and(|revision| capsule.revision <= revision)
            {
                return Err(ProviderProductError::new(
                    ProviderProductErrorCode::StalePublication,
                ));
            }
        }
        let revision = self.revision.checked_add(1).ok_or_else(|| {
            ProviderProductError::new(ProviderProductErrorCode::StalePublication)
        })?;
        Ok(Self {
            revision,
            capsule_id: Some(capsule.capsule_id.clone()),
            session_id: Some(capsule.session_id),
            capsule_revision: Some(capsule.revision),
            catalog: Arc::new(compose_catalog(Some(capsule))),
            publication: Some(ProviderProductPublication {
                generation: revision,
                capsule: Arc::new(capsule.clone()),
            }),
        })
    }

    pub fn revoke(
        &self,
        capsule_id: &str,
        session_id: u64,
    ) -> Result<Self, ProviderProductError> {
        if self.capsule_id.as_deref() != Some(capsule_id)
            || self.session_id != Some(session_id)
        {
            return Err(ProviderProductError::new(
                ProviderProductErrorCode::CrossSessionPublication,
            ));
        }
        let revision = self.revision.checked_add(1).ok_or_else(|| {
            ProviderProductError::new(ProviderProductErrorCode::StalePublication)
        })?;
        Ok(Self {
            revision,
            ..Self::default()
        })
    }
}

fn supports_product_publication(provider: ProviderKind) -> bool {
    // SSH remains owned by the inventory surface rather than the cloud catalog,
    // but its validated public context shares the CP4 route publication.
    provider == ProviderKind::Ssh || descriptor(provider).is_some()
}

fn descriptor(provider: ProviderKind) -> Option<&'static ProviderDescriptor> {
    PROVIDERS
        .iter()
        .find(|descriptor| descriptor.provider == provider)
}

fn auth_state(context: &ProviderContextTemplate) -> AuthState {
    match context.freshness {
        ProviderContextFreshness::Current => AuthState::Available {
            evidence_id: "cached-public-provider-context".into(),
        },
        ProviderContextFreshness::Refreshing | ProviderContextFreshness::Stale => {
            AuthState::Stale {
                previous: automexia_connectivity::connections::StaleAuthState::Available,
            }
        }
        ProviderContextFreshness::Expired => AuthState::Expired {
            previous_evidence_id: Some("cached-public-provider-context".into()),
        },
        ProviderContextFreshness::Offline => AuthState::Offline {
            diagnostic_code: "provider-context-offline".into(),
        },
        ProviderContextFreshness::Unavailable => AuthState::Missing {
            diagnostic_code: "provider-context-unavailable".into(),
        },
        ProviderContextFreshness::Error => AuthState::Error {
            diagnostic_code: "provider-context-error".into(),
        },
    }
}

fn recovery(context: &ProviderContextTemplate) -> ProviderRecoveryAction {
    match context.freshness {
        ProviderContextFreshness::Current
        | ProviderContextFreshness::Refreshing
        | ProviderContextFreshness::Stale => ProviderRecoveryAction::Refresh,
        ProviderContextFreshness::Expired | ProviderContextFreshness::Unavailable => {
            ProviderRecoveryAction::Authenticate
        }
        ProviderContextFreshness::Offline => ProviderRecoveryAction::RetryWhenOnline,
        ProviderContextFreshness::Error => ProviderRecoveryAction::Retry,
    }
}

fn bounded_scope_summary(context: &ProviderContextTemplate) -> String {
    if context.scope.is_empty() {
        return "Provider default scope".into();
    }
    let mut summary = String::new();
    for binding in context.scope.iter().take(4) {
        let prefix = if summary.is_empty() { "" } else { " · " };
        for character in prefix
            .chars()
            .chain(binding.name.chars())
            .chain(" ".chars())
            .chain(binding.public_value.chars())
        {
            if summary.len().saturating_add(character.len_utf8())
                > MAX_PROVIDER_SCOPE_SUMMARY_BYTES.saturating_sub(3)
            {
                summary.push_str("...");
                return summary;
            }
            summary.push(character);
        }
    }
    summary
}

fn unconfigured(descriptor: &ProviderDescriptor) -> ProviderCatalogItem {
    ProviderCatalogItem {
        provider: descriptor.provider,
        extension_id: descriptor.extension_id.into(),
        executable_id: descriptor.executable_id.into(),
        public_identity: None,
        scope_summary: "Choose a public provider context".into(),
        source_revision: None,
        freshness: ProviderContextFreshness::Unavailable,
        auth_state: AuthState::Missing {
            diagnostic_code: "provider-context-not-selected".into(),
        },
        recovery_action: ProviderRecoveryAction::ChooseContext,
        risk: EnvironmentRisk::Local,
        configured: false,
        activation_blocker: PROVIDER_ACTIVATION_BLOCKER.into(),
    }
}

fn compose_catalog(capsule: Option<&ProviderCapsule>) -> Vec<ProviderCatalogItem> {
    PROVIDERS
        .iter()
        .map(|descriptor| {
            let Some(context) = capsule.and_then(|capsule| {
                capsule
                    .contexts
                    .iter()
                    .find(|context| context.provider == descriptor.provider)
            }) else {
                return unconfigured(descriptor);
            };
            ProviderCatalogItem {
                provider: descriptor.provider,
                extension_id: descriptor.extension_id.into(),
                executable_id: descriptor.executable_id.into(),
                public_identity: Some(context.public_identity.clone()),
                scope_summary: bounded_scope_summary(context),
                source_revision: Some(context.provenance.source_revision.clone()),
                freshness: context.freshness,
                auth_state: auth_state(context),
                recovery_action: recovery(context),
                risk: context.risk,
                configured: true,
                activation_blocker: PROVIDER_ACTIVATION_BLOCKER.into(),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use automexia_connectivity::connections::{
        OpaqueReference, ProviderContextProvenance, ProviderProvenanceKind,
        ProviderScopeBinding, CONNECTION_SCHEMA_VERSION,
    };

    fn context(provider: ProviderKind, identity: &str) -> ProviderContextTemplate {
        ProviderContextTemplate {
            provider,
            configuration_reference: OpaqueReference::new("configuration-one"),
            public_identity: identity.into(),
            scope: vec![ProviderScopeBinding {
                name: "region".into(),
                public_value: "eu-west-1".into(),
            }],
            provenance: ProviderContextProvenance {
                kind: ProviderProvenanceKind::OfficialCliObservation,
                source_reference: OpaqueReference::new("source-one"),
                source_revision: "revision-one".into(),
                observed_at_ms: 100,
            },
            freshness: ProviderContextFreshness::Current,
            expires_at_ms: None,
            risk: EnvironmentRisk::Production,
        }
    }

    fn capsule(id: &str, session_id: u64, revision: u64) -> ProviderCapsule {
        ProviderCapsule {
            schema_version: CONNECTION_SCHEMA_VERSION,
            capsule_id: id.into(),
            session_id,
            revision,
            contexts: vec![
                context(ProviderKind::Aws, "account 123456789012"),
                context(ProviderKind::Kubernetes, "cluster production"),
            ],
            created_at_ms: 100,
        }
    }

    #[test]
    fn default_catalog_exposes_reviewed_adapters_without_claiming_contexts() {
        let snapshot = ProviderProductSnapshot::default();
        assert_eq!(snapshot.catalog.len(), MAX_PRODUCT_PROVIDERS);
        assert!(snapshot.catalog.iter().all(|item| !item.configured));
        assert!(snapshot.catalog.iter().all(|item| !matches!(
            item.provider,
            ProviderKind::OpenBao | ProviderKind::Ssh | ProviderKind::LocalContainer
        )));
    }

    #[test]
    fn ssh_context_is_retained_without_claiming_provider_catalog() {
        let mut candidate = capsule("capsule-ssh", 7, 1);
        candidate.contexts =
            vec![context(ProviderKind::Ssh, "target bastion.example.com")];
        candidate.contexts[0].scope[0].name = "target".into();
        candidate.contexts[0].scope[0].public_value = "bastion.example.com".into();

        let snapshot = ProviderProductSnapshot::default()
            .publish(&candidate)
            .unwrap();
        assert_eq!(
            snapshot.publication().unwrap().capsule().contexts[0].provider,
            ProviderKind::Ssh
        );
        assert!(snapshot
            .catalog
            .iter()
            .all(|item| item.provider != ProviderKind::Ssh && !item.configured));
    }

    #[test]
    fn publication_is_public_bounded_and_stale_safe() {
        let snapshot = ProviderProductSnapshot::default()
            .publish(&capsule("capsule-one", 7, 1))
            .unwrap();
        let publication = snapshot.publication().unwrap();
        assert_eq!(publication.generation(), 1);
        let debug = format!("{publication:?}");
        assert!(debug.contains("<redacted-public-provider-capsule>"));
        for forbidden in [
            "123456789012",
            "production",
            "configuration-one",
            "source-one",
        ] {
            assert!(!debug.contains(forbidden));
        }
        let aws = snapshot
            .catalog
            .iter()
            .find(|item| item.provider == ProviderKind::Aws)
            .unwrap();
        assert_eq!(aws.public_identity.as_deref(), Some("account 123456789012"));
        assert!(aws.scope_summary.len() <= MAX_PROVIDER_SCOPE_SUMMARY_BYTES);
        assert_eq!(
            snapshot
                .publish(&capsule("capsule-one", 7, 1))
                .unwrap_err()
                .code(),
            ProviderProductErrorCode::StalePublication
        );
        assert_eq!(
            snapshot
                .publish(&capsule("capsule-one", 7, 2))
                .unwrap_err()
                .code(),
            ProviderProductErrorCode::StalePublication
        );
        assert_eq!(
            snapshot
                .publish(&capsule("capsule-two", 7, 2))
                .unwrap_err()
                .code(),
            ProviderProductErrorCode::CrossSessionPublication
        );

        let replacement = snapshot.publish(&capsule("capsule-two", 8, 2)).unwrap();
        assert_eq!(replacement.capsule_id.as_deref(), Some("capsule-two"));
        assert_eq!(replacement.session_id, Some(8));
    }

    #[test]
    fn unsupported_openbao_context_is_rejected_before_product_ui() {
        let mut candidate = capsule("capsule-one", 7, 1);
        candidate.contexts = vec![context(ProviderKind::OpenBao, "organization")];
        assert_eq!(
            ProviderProductSnapshot::default()
                .publish(&candidate)
                .unwrap_err()
                .code(),
            ProviderProductErrorCode::UnsupportedProvider
        );
    }

    #[test]
    fn revoke_is_session_bound_and_returns_to_nonconfigured_catalog() {
        let snapshot = ProviderProductSnapshot::default()
            .publish(&capsule("capsule-one", 7, 1))
            .unwrap();
        assert_eq!(
            snapshot.revoke("capsule-one", 8).unwrap_err().code(),
            ProviderProductErrorCode::CrossSessionPublication
        );
        let revoked = snapshot.revoke("capsule-one", 7).unwrap();
        assert!(revoked.catalog.iter().all(|item| !item.configured));
        assert_eq!(revoked.revision, 2);
        assert!(revoked.publication().is_none());
    }
}
