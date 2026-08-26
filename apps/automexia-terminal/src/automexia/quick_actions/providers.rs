//! Explicit-refresh CP4 composition for immutable provider action snapshots.
//!
//! Provider adapters retain their exact command grammars. This module performs
//! no discovery or I/O; callers pass one already validated public capsule after
//! refresh and publish the complete snapshot through `QuickActionRuntime`.

use std::fmt;

use automexia_command_productivity::actions::{
    build_provider_action_snapshot, build_ssh_provider_action, ProviderActionCandidate,
    ProviderActionSnapshot,
};
use automexia_connectivity::connections::{ProviderCapsule, ProviderKind};

use crate::automexia::connections::ProviderProductPublication;

use super::worker::{QuickActionRuntime, QuickActionRuntimeErrorCode};

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProviderActionRouteErrorCode {
    InvalidRoute,
    BindingMismatch,
    CompositionRejected,
    RuntimeUnavailable,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ProviderActionRouteError {
    code: ProviderActionRouteErrorCode,
}

impl ProviderActionRouteError {
    const fn new(code: ProviderActionRouteErrorCode) -> Self {
        Self { code }
    }

    pub const fn code(self) -> ProviderActionRouteErrorCode {
        self.code
    }
}

impl fmt::Debug for ProviderActionRouteError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ProviderActionRouteError")
            .field("code", &self.code)
            .finish()
    }
}

impl fmt::Display for ProviderActionRouteError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let code = match self.code {
            ProviderActionRouteErrorCode::InvalidRoute => "invalid-route",
            ProviderActionRouteErrorCode::BindingMismatch => "binding-mismatch",
            ProviderActionRouteErrorCode::CompositionRejected => "composition-rejected",
            ProviderActionRouteErrorCode::RuntimeUnavailable => "runtime-unavailable",
        };
        write!(formatter, "provider-action-route-{code}")
    }
}

impl std::error::Error for ProviderActionRouteError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProviderActionPublicationOutcome {
    Published {
        generation: u64,
        action_count: usize,
    },
    Unchanged {
        generation: u64,
        action_count: usize,
    },
    Cleared {
        removed: bool,
    },
}

#[derive(Clone, Debug)]
pub struct ProviderActionPublisher {
    runtime: QuickActionRuntime,
}

impl ProviderActionPublisher {
    pub fn new(runtime: QuickActionRuntime) -> Self {
        Self { runtime }
    }

    pub fn sync_route(
        &self,
        route_id: usize,
        current_session_id: u64,
        current_capsule_revision: u64,
        publication: Option<&ProviderProductPublication>,
        generated_at_ms: u64,
    ) -> Result<ProviderActionPublicationOutcome, ProviderActionRouteError> {
        if route_id == 0 {
            self.runtime.clear_provider_snapshot(route_id);
            return Err(ProviderActionRouteError::new(
                ProviderActionRouteErrorCode::InvalidRoute,
            ));
        }
        let Some(publication) = publication else {
            return Ok(ProviderActionPublicationOutcome::Cleared {
                removed: self.runtime.clear_provider_snapshot(route_id),
            });
        };
        let route_session = u64::try_from(route_id).map_err(|_| {
            ProviderActionRouteError::new(ProviderActionRouteErrorCode::InvalidRoute)
        })?;
        let capsule = publication.capsule();
        if route_session != current_session_id
            || capsule.session_id != current_session_id
            || capsule.revision != current_capsule_revision
        {
            self.runtime.clear_provider_snapshot(route_id);
            return Err(ProviderActionRouteError::new(
                ProviderActionRouteErrorCode::BindingMismatch,
            ));
        }
        let snapshot = compose_provider_action_snapshot(
            capsule,
            publication.generation(),
            generated_at_ms,
        )
        .map_err(|_| {
            ProviderActionRouteError::new(
                ProviderActionRouteErrorCode::CompositionRejected,
            )
        })?;
        let generation = snapshot.generation();
        let action_count = snapshot.actions().len();
        if self.runtime.provider_snapshot_matches(route_id, &snapshot) {
            return Ok(ProviderActionPublicationOutcome::Unchanged {
                generation,
                action_count,
            });
        }
        self.runtime
            .publish_provider_snapshot(route_id, snapshot)
            .map_err(|error| match error {
                QuickActionRuntimeErrorCode::StaleProviderSnapshot => {
                    ProviderActionRouteError::new(
                        ProviderActionRouteErrorCode::BindingMismatch,
                    )
                }
                _ => ProviderActionRouteError::new(
                    ProviderActionRouteErrorCode::RuntimeUnavailable,
                ),
            })?;
        Ok(ProviderActionPublicationOutcome::Published {
            generation,
            action_count,
        })
    }
}

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
    use automexia_connectivity::connections::{
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
                == automexia_command_productivity::actions::ExecutionMode::Insert
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
