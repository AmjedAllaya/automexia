use automexia_devops::connections::{
    resolve_connection_plan, validate_profile, ConnectionModelErrorCode,
    ConnectionProfileV1, ConnectionSource, DestinationSurface,
    EnvironmentCapsuleTemplate, EnvironmentClassification, EnvironmentKind,
    EnvironmentRisk, IdentityKind, IdentityReference, OpaqueReference, PlanContext,
    ProviderKind, ResolvedExecutable, SourceKind, TransportDescriptor,
};
use proptest::prelude::*;

fn digest(character: char) -> String {
    character.to_string().repeat(64)
}

fn profile(display_name: String, target: String) -> ConnectionProfileV1 {
    ConnectionProfileV1 {
        schema_version: 1,
        id: "property-profile".into(),
        revision: 1,
        display_name,
        description: String::new(),
        tags: vec!["property".into()],
        favorite: false,
        environment: EnvironmentClassification {
            kind: EnvironmentKind::Development,
            label: "Development".into(),
            risk: EnvironmentRisk::Development,
        },
        provider: ProviderKind::Ssh,
        transport: TransportDescriptor::OpenSshAlias {
            alias: target.clone(),
        },
        public_target: target,
        jump_profile_references: Vec::new(),
        identity: IdentityReference {
            kind: IdentityKind::Agent,
            reference: OpaqueReference::new("identity-property"),
            public_label: "External agent".into(),
            owner: "open-ssh".into(),
        },
        capsule: EnvironmentCapsuleTemplate {
            revision: 1,
            public_environment: Vec::new(),
            context_references: Vec::new(),
        },
        recipe_references: Vec::new(),
        tunnels: Vec::new(),
        destination_preference: DestinationSurface::Pane,
        source: ConnectionSource {
            kind: SourceKind::User,
            reference: OpaqueReference::new("source-property"),
            revision: "1".into(),
        },
        approval_fingerprint: None,
        created_at_ms: 1,
        updated_at_ms: 1,
        last_used_at_ms: None,
    }
}

proptest! {
    #[test]
    fn bounded_printable_unicode_labels_and_targets_validate(
        label in "[A-Za-z0-9][A-Za-z0-9 _.-]{0,127}",
        target in "[a-z][a-z0-9.-]{0,126}"
    ) {
        prop_assert!(validate_profile(&profile(label, target)).is_ok());
        prop_assert!(validate_profile(&profile("Équipe 東京".into(), "unicode.example.invalid".into())).is_ok());
    }

    #[test]
    fn every_ascii_control_character_is_rejected(character in 0_u8..=31) {
        let hostile = format!("safe{}unsafe", char::from(character));
        let error = validate_profile(&profile(hostile, "safe.example.invalid".into()))
            .unwrap_err();
        prop_assert_eq!(error.code, ConnectionModelErrorCode::UnsafeText);
    }

    #[test]
    fn executable_and_capability_input_order_does_not_change_the_plan_fingerprint(
        reverse_executables in any::<bool>(),
        reverse_capabilities in any::<bool>()
    ) {
        let profile = profile("Stable".into(), "stable.example.invalid".into());
        let mut executables = vec![
            ResolvedExecutable { executable_id: "openssh".into(), identity_digest: digest('a') },
            ResolvedExecutable { executable_id: "ssh-add".into(), identity_digest: digest('b') },
        ];
        let mut capabilities = vec!["session.launch.openssh".into(), "status.agent".into()];
        if reverse_executables { executables.reverse(); }
        if reverse_capabilities { capabilities.reverse(); }
        let plan = resolve_connection_plan(
            &profile,
            &[],
            &PlanContext {
                executable_identities: executables,
                requested_capabilities: capabilities,
                public_variables: Default::default(),
            },
        ).unwrap();
        let canonical = resolve_connection_plan(
            &profile,
            &[],
            &PlanContext {
                executable_identities: vec![
                    ResolvedExecutable { executable_id: "openssh".into(), identity_digest: digest('a') },
                    ResolvedExecutable { executable_id: "ssh-add".into(), identity_digest: digest('b') },
                ],
                requested_capabilities: vec!["session.launch.openssh".into(), "status.agent".into()],
                public_variables: Default::default(),
            },
        ).unwrap();
        prop_assert_eq!(plan.approval_fingerprint, canonical.approval_fingerprint);
    }
}
