//! Explicit-refresh CP4 composition for immutable provider action snapshots.
//!
//! Provider adapters retain their exact command grammars. This module performs
//! no discovery or I/O; callers pass one already validated public capsule after
//! refresh and publish the complete snapshot through `QuickActionRuntime`.

use std::fmt;

use automexia_devops::{
    actions::{
        build_provider_action_snapshot, build_ssh_provider_action,
        ProviderActionCandidate, ProviderActionSnapshot,
    },
    connections::{ProviderCapsule, ProviderKind},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProviderActionCompositionErrorCode {
    UnsupportedProvider,
    AdapterRejected,
    SnapshotRejected,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ProviderActionCompositionError {
    code: ProviderActionCompositionErrorCode,
    provider: ProviderKind,
}

impl ProviderActionCompositionError {
    const fn new(
        code: ProviderActionCompositionErrorCode,
        provider: ProviderKind,
    ) -> Self {
        Self { code, provider }
    }

    pub const fn code(&self) -> ProviderActionCompositionErrorCode {
        self.code
    }

    pub const fn provider(&self) -> ProviderKind {
        self.provider
    }
}

impl fmt::Debug for ProviderActionCompositionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ProviderActionCompositionError")
            .field("code", &self.code)
            .field("provider", &self.provider)
            .finish()
    }
}

impl fmt::Display for ProviderActionCompositionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "provider-action-{:?}-{:?}",
            self.provider, self.code
        )
    }
}

impl std::error::Error for ProviderActionCompositionError {}

pub fn compose_provider_action_snapshot(
    capsule: &ProviderCapsule,
    generation: u64,
    generated_at_ms: u64,
) -> Result<ProviderActionSnapshot, ProviderActionCompositionError> {
    let mut actions =
        Vec::<ProviderActionCandidate>::with_capacity(capsule.contexts.len());
    for context in &capsule.contexts {
        let action = match context.provider {
            ProviderKind::Ssh => {
                build_ssh_provider_action(capsule, generation, generated_at_ms)
                    .map_err(|_| adapter_error(context.provider))
            }
            ProviderKind::Aws => automexia_devops_aws::build_provider_quick_action(
                capsule,
                generation,
                generated_at_ms,
            )
            .map_err(|_| adapter_error(context.provider)),
            ProviderKind::Azure => automexia_devops_azure::build_provider_quick_action(
                capsule,
                generation,
                generated_at_ms,
            )
            .map_err(|_| adapter_error(context.provider)),
            ProviderKind::Gcp => automexia_devops_gcp::build_provider_quick_action(
                capsule,
                generation,
                generated_at_ms,
            )
            .map_err(|_| adapter_error(context.provider)),
            ProviderKind::Kubernetes => {
                automexia_devops_kubernetes::build_provider_quick_action(
                    capsule,
                    generation,
                    generated_at_ms,
                )
                .map_err(|_| adapter_error(context.provider))
            }
            ProviderKind::OpenShift => {
                automexia_devops_openshift::build_provider_quick_action(
                    capsule,
                    generation,
                    generated_at_ms,
                )
                .map_err(|_| adapter_error(context.provider))
            }
            ProviderKind::Teleport => {
                automexia_devops_teleport::build_provider_quick_action(
                    capsule,
                    generation,
                    generated_at_ms,
                )
                .map_err(|_| adapter_error(context.provider))
            }
            ProviderKind::None | ProviderKind::OpenBao | ProviderKind::LocalContainer => {
                return Err(ProviderActionCompositionError::new(
                    ProviderActionCompositionErrorCode::UnsupportedProvider,
                    context.provider,
                ));
            }
        }?;
        actions.push(action);
    }
    build_provider_action_snapshot(capsule, generation, generated_at_ms, actions).map_err(
        |_| {
            ProviderActionCompositionError::new(
                ProviderActionCompositionErrorCode::SnapshotRejected,
                ProviderKind::None,
            )
        },
    )
}

const fn adapter_error(provider: ProviderKind) -> ProviderActionCompositionError {
    ProviderActionCompositionError::new(
        ProviderActionCompositionErrorCode::AdapterRejected,
        provider,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use automexia_devops::connections::{
        EnvironmentRisk, OpaqueReference, ProviderContextFreshness,
        ProviderContextProvenance, ProviderContextTemplate, ProviderProvenanceKind,
        ProviderScopeBinding, CONNECTION_SCHEMA_VERSION,
    };

    fn context(
        provider: ProviderKind,
        public_identity: &str,
        scope: &[(&str, &str)],
    ) -> ProviderContextTemplate {
        ProviderContextTemplate {
            provider,
            configuration_reference: OpaqueReference::new("configuration.fixture"),
            public_identity: public_identity.into(),
            scope: scope
                .iter()
                .map(|(name, value)| ProviderScopeBinding {
                    name: (*name).into(),
                    public_value: (*value).into(),
                })
                .collect(),
            provenance: ProviderContextProvenance {
                kind: ProviderProvenanceKind::OfficialCliObservation,
                source_reference: OpaqueReference::new("source.fixture"),
                source_revision: "revision-1".into(),
                observed_at_ms: 100,
            },
            freshness: ProviderContextFreshness::Current,
            expires_at_ms: None,
            risk: EnvironmentRisk::Production,
        }
    }

    #[test]
    fn multi_provider_composition_is_complete_sorted_and_nonexecuting() {
        let capsule = ProviderCapsule {
            schema_version: CONNECTION_SCHEMA_VERSION,
            capsule_id: "capsule.multi-cloud".into(),
            session_id: 81,
            revision: 7,
            contexts: vec![
                context(
                    ProviderKind::Azure,
                    "person@example.invalid",
                    &[
                        ("subscription", "11111111-2222-3333-4444-555555555555"),
                        ("tenant", "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee"),
                    ],
                ),
                context(
                    ProviderKind::Aws,
                    "engineering@123456789012",
                    &[
                        ("profile", "engineering"),
                        ("account", "123456789012"),
                        ("region", "eu-west-3"),
                    ],
                ),
            ],
            created_at_ms: 100,
        };
        let snapshot = compose_provider_action_snapshot(&capsule, 4, 200).unwrap();
        assert_eq!(snapshot.actions().len(), 2);
        assert_eq!(
            snapshot.actions()[0].action().id,
            "provider.aws.caller-identity"
        );
        assert_eq!(snapshot.actions()[1].action().id, "provider.azure.account");
        assert!(snapshot.actions().iter().all(|candidate| {
            candidate.binding().execution()
                == automexia_devops::actions::ExecutionMode::Insert
        }));
    }

    #[test]
    fn unsupported_provider_prevents_partial_snapshot_publication() {
        let capsule = ProviderCapsule {
            schema_version: CONNECTION_SCHEMA_VERSION,
            capsule_id: "capsule.openbao".into(),
            session_id: 82,
            revision: 1,
            contexts: vec![context(
                ProviderKind::OpenBao,
                "engineering",
                &[("namespace", "engineering")],
            )],
            created_at_ms: 100,
        };
        let error = compose_provider_action_snapshot(&capsule, 1, 200).unwrap_err();
        assert_eq!(
            error.code(),
            ProviderActionCompositionErrorCode::UnsupportedProvider
        );
        assert_eq!(error.provider(), ProviderKind::OpenBao);
    }
}
