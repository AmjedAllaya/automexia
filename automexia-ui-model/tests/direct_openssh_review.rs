use std::collections::BTreeMap;

use automexia_devops::connections::{
    prepare_direct_openssh, resolve_connection_plan, review_direct_openssh, AuthState,
    ConnectionObservation, ConnectionProfileV1, ConnectionSource, DestinationSurface,
    EnvironmentCapsuleTemplate, EnvironmentClassification, EnvironmentKind,
    EnvironmentRisk, HostTrustState, IdentityKind, IdentityReference, OpaqueReference,
    PlanContext, ProviderKind, ResolvedExecutable, SourceKind, ToolState,
    TransportDescriptor, TransportState, CONNECTION_SCHEMA_VERSION,
};
use automexia_ui_model::connection_hub::{
    project_direct_openssh_preparation, project_direct_openssh_review, HubLayout,
    Viewport,
};

fn digest(byte: char) -> String {
    byte.to_string().repeat(64)
}

fn fixture_profile() -> ConnectionProfileV1 {
    ConnectionProfileV1 {
        schema_version: CONNECTION_SCHEMA_VERSION,
        id: "profile-prod".into(),
        revision: 7,
        display_name: "Production".into(),
        description: String::new(),
        tags: Vec::new(),
        favorite: false,
        environment: EnvironmentClassification {
            kind: EnvironmentKind::Production,
            label: "Production".into(),
            risk: EnvironmentRisk::Production,
        },
        provider: ProviderKind::Ssh,
        transport: TransportDescriptor::OpenSshAlias {
            alias: "private-destination-canary".into(),
        },
        public_target: "production.example.invalid".into(),
        jump_profile_references: Vec::new(),
        identity: IdentityReference {
            kind: IdentityKind::Agent,
            reference: OpaqueReference::new("identity-private-canary"),
            public_label: "External agent".into(),
            owner: "open-ssh".into(),
        },
        capsule: EnvironmentCapsuleTemplate {
            revision: 3,
            public_environment: Vec::new(),
            context_references: Vec::new(),
        },
        recipe_references: Vec::new(),
        tunnels: Vec::new(),
        destination_preference: DestinationSurface::PaneTab,
        source: ConnectionSource {
            kind: SourceKind::OpenSshInventory,
            reference: OpaqueReference::new("source-private"),
            revision: "source-8".into(),
        },
        approval_fingerprint: None,
        created_at_ms: 1,
        updated_at_ms: 2,
        last_used_at_ms: None,
    }
}

fn fixture() -> automexia_devops::connections::DirectOpenSshReview {
    let profile = fixture_profile();
    let plan = resolve_connection_plan(
        &profile,
        &[],
        &PlanContext {
            executable_identities: vec![ResolvedExecutable {
                executable_id: "ssh".into(),
                identity_digest: digest('e'),
            }],
            requested_capabilities: vec!["session.launch".into()],
            public_variables: BTreeMap::new(),
        },
    )
    .unwrap();
    let observation = ConnectionObservation {
        schema_version: CONNECTION_SCHEMA_VERSION,
        connection_id: profile.id.clone(),
        generation: 9,
        auth_state: AuthState::Ready {
            evidence_id: "agent-ready".into(),
            expires_at_ms: None,
        },
        observed_at_ms: 1,
        expires_at_ms: None,
        stale_after_ms: 1_000,
        tool_state: ToolState::Ready,
        transport_state: TransportState::Available,
        public_identity_summary: "External agent ready".into(),
        diagnostic_code: None,
        recovery_action: None,
    };
    review_direct_openssh(&profile, &plan, &observation, HostTrustState::Unknown, 2)
        .unwrap()
}

#[test]
fn pending_preparation_is_complete_redacted_disabled_and_responsive() {
    let prepared = prepare_direct_openssh(&fixture_profile()).unwrap();
    for (viewport, expected_layout) in [
        (Viewport::new(1_600.0, 900.0, 1.0), HubLayout::Wide),
        (Viewport::new(900.0, 700.0, 1.0), HubLayout::Medium),
        (Viewport::new(480.0, 800.0, 1.0), HubLayout::Narrow),
    ] {
        let view = project_direct_openssh_preparation(&prepared, viewport);
        assert_eq!(view.layout, expected_layout);
        assert_eq!(view.sections.len(), 9);
        assert!(view.sections.iter().any(|section| {
            section.id == "identity" && section.summary.contains("verification pending")
        }));
        assert!(view.sections.iter().any(|section| {
            section.id == "host-trust" && section.summary.contains("changed keys blocked")
        }));
        assert!(view.sections.iter().any(|section| {
            section.id == "argv" && section.summary == "ssh <destination>"
        }));
        assert!(!view.execution_enabled);
        assert_eq!(
            view.primary_label,
            "Connection unavailable—verification pending"
        );
        let rendered = format!("{view:?}");
        assert!(!rendered.contains("private-destination-canary"));
        assert!(!rendered.contains("identity-private-canary"));
    }
}

#[test]
fn m3_projection_is_complete_redacted_disabled_and_accessible() {
    let reviewed = fixture();
    for (viewport, expected_layout) in [
        (Viewport::new(1_600.0, 900.0, 1.0), HubLayout::Wide),
        (Viewport::new(900.0, 700.0, 1.0), HubLayout::Medium),
        (Viewport::new(480.0, 800.0, 1.0), HubLayout::Narrow),
    ] {
        let view = project_direct_openssh_review(&reviewed, viewport);
        assert_eq!(view.layout, expected_layout);
        assert_eq!(
            view.sections
                .iter()
                .map(|section| section.id.as_str())
                .collect::<Vec<_>>(),
            [
                "identity",
                "target",
                "transport",
                "executable",
                "host-trust",
                "capabilities",
                "risk",
                "destination",
                "argv",
            ]
        );
        assert!(!view.execution_enabled);
        assert_eq!(
            view.primary_label,
            "Connection unavailable—M2 approval pending"
        );
        assert!(view.sections.iter().any(|section| {
            section.id == "identity" && section.summary.contains("Ready")
        }));
        assert!(view.sections.iter().any(|section| {
            section.id == "executable" && section.summary.contains("ssh")
        }));
        assert!(view.sections.iter().any(|section| {
            section.id == "host-trust" && section.summary.contains("changed keys blocked")
        }));
        assert!(view.sections.iter().any(|section| {
            section.id == "capabilities" && section.summary == "session.launch"
        }));
        assert!(view
            .sections
            .iter()
            .any(|section| { section.id == "risk" && section.summary == "Production" }));
        assert!(view.sections.iter().any(|section| {
            section.id == "argv" && section.summary == "ssh <destination>"
        }));
        let rendered = format!("{view:?}");
        assert!(!rendered.contains("private-destination-canary"));
        assert!(!rendered.contains("identity-private-canary"));
        assert!(view
            .accessibility_tree
            .iter()
            .any(|node| node.name == "Reviewed OpenSSH argument shape"));
    }
}
