use std::collections::BTreeMap;

use automexia_devops::connections::{
    prepare_direct_openssh, resolve_connection_plan,
    review_direct_openssh as review_direct_openssh_at, AuthState, ConnectionModelError,
    ConnectionObservation, ConnectionProfileV1, ConnectionSource, DestinationSurface,
    DirectOpenSshDestinationKind, DirectOpenSshHostTrustPolicy,
    DirectOpenSshIdentityReadiness, DirectOpenSshReview, EnvironmentCapsuleTemplate,
    EnvironmentClassification, EnvironmentKind, EnvironmentRisk, HostTrustState,
    IdentityKind, IdentityReference, OpaqueReference, PlanContext, ProviderKind,
    ResolvedConnectionPlan, ResolvedExecutable, SourceKind, ToolState,
    TransportDescriptor, TransportState, CONNECTION_SCHEMA_VERSION,
    DIRECT_OPENSSH_MANAGED_OPTIONS,
};

const NOW_MS: u64 = 1_700_000_000_001;

fn review_direct_openssh(
    profile: &ConnectionProfileV1,
    plan: &ResolvedConnectionPlan,
    observation: &ConnectionObservation,
    host_trust: HostTrustState,
) -> Result<DirectOpenSshReview, ConnectionModelError> {
    review_direct_openssh_at(profile, plan, observation, host_trust, NOW_MS)
}

fn digest(byte: char) -> String {
    byte.to_string().repeat(64)
}

fn profile(
    transport: TransportDescriptor,
    source_kind: SourceKind,
) -> ConnectionProfileV1 {
    ConnectionProfileV1 {
        schema_version: CONNECTION_SCHEMA_VERSION,
        id: "profile-prod".into(),
        revision: 7,
        display_name: "Production bastion".into(),
        description: "Synthetic public fixture".into(),
        tags: vec!["production".into()],
        favorite: false,
        environment: EnvironmentClassification {
            kind: EnvironmentKind::Production,
            label: "Production".into(),
            risk: EnvironmentRisk::Production,
        },
        provider: ProviderKind::Ssh,
        transport,
        public_target: "production.example.invalid".into(),
        jump_profile_references: Vec::new(),
        identity: IdentityReference {
            kind: IdentityKind::Agent,
            reference: OpaqueReference::new("identity-canary-private"),
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
            kind: source_kind,
            reference: OpaqueReference::new("source-private"),
            revision: "source-8".into(),
        },
        approval_fingerprint: None,
        created_at_ms: 1,
        updated_at_ms: 2,
        last_used_at_ms: None,
    }
}

fn plan(profile: &ConnectionProfileV1) -> ResolvedConnectionPlan {
    resolve_connection_plan(
        profile,
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
    .unwrap()
}

fn observation(profile: &ConnectionProfileV1) -> ConnectionObservation {
    ConnectionObservation {
        schema_version: CONNECTION_SCHEMA_VERSION,
        connection_id: profile.id.clone(),
        generation: 11,
        auth_state: AuthState::Ready {
            evidence_id: "agent-ready".into(),
            expires_at_ms: None,
        },
        observed_at_ms: 1_700_000_000_000,
        expires_at_ms: None,
        stale_after_ms: 30_000,
        tool_state: ToolState::Ready,
        transport_state: TransportState::Available,
        public_identity_summary: "External agent ready".into(),
        diagnostic_code: None,
        recovery_action: None,
    }
}

fn alias_profile(alias: &str) -> ConnectionProfileV1 {
    profile(
        TransportDescriptor::OpenSshAlias {
            alias: alias.into(),
        },
        SourceKind::OpenSshInventory,
    )
}

fn literal_profile(destination: &str) -> ConnectionProfileV1 {
    let mut profile = profile(
        TransportDescriptor::OpenSshExplicit {
            host: destination.into(),
            port: None,
            user: None,
            proxy_jump: Vec::new(),
        },
        SourceKind::User,
    );
    profile.public_target = destination.into();
    profile
}

#[test]
fn inventory_preparation_is_canonical_redacted_and_nonactivated() {
    let profile = alias_profile("private-preparation-canary");
    let prepared = prepare_direct_openssh(&profile).unwrap();

    assert_eq!(prepared.profile(), &profile);
    assert_eq!(prepared.plan().profile_id, profile.id);
    assert_eq!(prepared.plan().requested_capabilities, ["session.launch"]);
    assert!(prepared.plan().executable_identities.is_empty());
    assert!(!prepared.plan().execution_enabled);
    assert!(prepared
        .plan()
        .authority_ceiling
        .iter()
        .all(|authority| !authority.enabled));
    let debug = format!("{prepared:?}");
    assert!(!debug.contains("private-preparation-canary"));
    assert!(!debug.contains("profile-prod"));
}

#[test]
fn inventory_preparation_rejects_indirect_and_stale_profiles() {
    let mut indirect = alias_profile("prod");
    indirect.jump_profile_references.push("jump-profile".into());
    assert!(prepare_direct_openssh(&indirect).is_err());

    let profile = alias_profile("prod");
    let prepared = prepare_direct_openssh(&profile).unwrap();
    let mut changed = profile;
    changed.revision += 1;
    assert!(prepared.validate_current(&changed).is_err());
}

#[test]
fn inventory_typed_alias_binds_the_safe_exact_argv_to_the_f2_plan() {
    let profile = alias_profile("prod-alias");
    let plan = plan(&profile);
    let reviewed = review_direct_openssh(
        &profile,
        &plan,
        &observation(&profile),
        HostTrustState::Unknown,
    )
    .unwrap();

    assert_eq!(
        reviewed.request.destination_kind(),
        DirectOpenSshDestinationKind::InventoryAlias
    );
    let arguments = reviewed.request.arguments();
    assert_eq!(arguments.last(), Some(&"prod-alias"));
    assert_eq!(
        &arguments[..arguments.len() - 1],
        DIRECT_OPENSSH_MANAGED_OPTIONS
    );
    assert!(arguments
        .iter()
        .take(arguments.len() - 1)
        .all(|argument| argument.starts_with("-o")));
    assert!(arguments.contains(&"-oClearAllForwardings=yes"));
    assert!(arguments.contains(&"-oEnableEscapeCommandline=no"));
    assert!(arguments.contains(&"-oForwardAgent=no"));
    assert!(arguments.contains(&"-oProxyCommand=none"));
    assert!(arguments.contains(&"-oProxyJump=none"));
    assert!(arguments.contains(&"-oStrictHostKeyChecking=ask"));
    assert_eq!(reviewed.executable_identity.executable_id, "ssh");
    assert_eq!(
        reviewed.identity_readiness,
        DirectOpenSshIdentityReadiness::Ready
    );
    assert_eq!(
        reviewed.host_trust_policy,
        DirectOpenSshHostTrustPolicy::AskOnFirstUseRejectChanged
    );
    assert_eq!(reviewed.environment_risk, EnvironmentRisk::Production);
    assert!(!reviewed.execution_enabled);
    let preview = &reviewed.review.executable_preview[0].arguments;
    assert_eq!(preview.len(), arguments.len());
    assert!(preview[..preview.len() - 1]
        .iter()
        .all(
            |argument| argument.label == "managed-security-option" && !argument.redacted
        ));
    assert_eq!(preview.last().unwrap().label, "destination");
    assert!(preview.last().unwrap().redacted);
    assert!(reviewed
        .review
        .policy_decisions
        .iter()
        .any(|decision| decision.code == "managed-launch-activation-pending"));

    let debug = format!("{reviewed:?}");
    assert!(!debug.contains("prod-alias"));
    assert!(!debug.contains("identity-canary-private"));
}

#[test]
fn managed_options_preserve_openssh_post_quantum_defaults_and_warnings() {
    assert!(DIRECT_OPENSSH_MANAGED_OPTIONS
        .iter()
        .all(|argument| !argument.starts_with("-oKexAlgorithms=")));
    assert!(DIRECT_OPENSSH_MANAGED_OPTIONS
        .iter()
        .all(|argument| !argument.starts_with("-oWarnWeakCrypto=")));
}

#[test]
fn typed_literal_keeps_one_destination_and_defers_user_port_and_routes_to_m4() {
    let profile = literal_profile("host.example.invalid");
    let reviewed = review_direct_openssh(
        &profile,
        &plan(&profile),
        &observation(&profile),
        HostTrustState::Known {
            fingerprint_sha256: digest('b'),
        },
    )
    .unwrap();
    assert_eq!(
        reviewed.request.destination_kind(),
        DirectOpenSshDestinationKind::Literal
    );
    let arguments = reviewed.request.arguments();
    assert_eq!(arguments.last(), Some(&"host.example.invalid"));
    assert_eq!(
        arguments
            .iter()
            .filter(|argument| !argument.starts_with("-o"))
            .copied()
            .collect::<Vec<_>>(),
        ["host.example.invalid"]
    );

    let mut user = literal_profile("host.example.invalid");
    if let TransportDescriptor::OpenSshExplicit { user, .. } = &mut user.transport {
        *user = Some("operator".into());
    }
    assert!(review_direct_openssh(
        &user,
        &plan(&user),
        &observation(&user),
        HostTrustState::Unknown
    )
    .is_err());

    let mut port = literal_profile("host.example.invalid");
    if let TransportDescriptor::OpenSshExplicit { port, .. } = &mut port.transport {
        *port = Some(22);
    }
    assert!(review_direct_openssh(
        &port,
        &plan(&port),
        &observation(&port),
        HostTrustState::Unknown
    )
    .is_err());

    let mut jump = literal_profile("host.example.invalid");
    if let TransportDescriptor::OpenSshExplicit { proxy_jump, .. } = &mut jump.transport {
        proxy_jump.push("jump".into());
    }
    assert!(review_direct_openssh(
        &jump,
        &plan(&jump),
        &observation(&jump),
        HostTrustState::Unknown
    )
    .is_err());
}

#[test]
fn hostile_option_like_bidi_whitespace_and_ambiguous_destinations_fail_closed() {
    let valid_profile = alias_profile("safe");
    let valid_plan = plan(&valid_profile);
    for destination in [
        "",
        "-oProxyCommand=bad",
        "two hosts",
        "host\nname",
        "host\u{2066}name",
        "user@host",
        "host:22",
        "ssh://host",
        "*.example.invalid",
        "!host",
        "host;whoami",
    ] {
        let profile = literal_profile(destination);
        let error = review_direct_openssh(
            &profile,
            &valid_plan,
            &observation(&profile),
            HostTrustState::Unknown,
        )
        .unwrap_err();
        if !destination.is_empty() {
            assert!(!error.to_string().contains(destination));
        }
    }

    let oversized = "h".repeat(513);
    let profile = literal_profile(&oversized);
    assert!(review_direct_openssh(
        &profile,
        &valid_plan,
        &observation(&profile),
        HostTrustState::Unknown
    )
    .is_err());
}

#[test]
fn review_rejects_wrong_source_executable_capability_and_active_authority() {
    let wrong_source = profile(
        TransportDescriptor::OpenSshAlias {
            alias: "prod".into(),
        },
        SourceKind::User,
    );
    assert!(review_direct_openssh(
        &wrong_source,
        &plan(&wrong_source),
        &observation(&wrong_source),
        HostTrustState::Unknown
    )
    .is_err());

    let profile = alias_profile("prod");
    let mut wrong_executable = plan(&profile);
    wrong_executable.executable_identities[0].executable_id = "openssh".into();
    assert!(review_direct_openssh(
        &profile,
        &wrong_executable,
        &observation(&profile),
        HostTrustState::Unknown
    )
    .is_err());

    let mut extra_capability = plan(&profile);
    extra_capability
        .requested_capabilities
        .push("listener.open".into());
    assert!(review_direct_openssh(
        &profile,
        &extra_capability,
        &observation(&profile),
        HostTrustState::Unknown
    )
    .is_err());

    let mut active = plan(&profile);
    active.execution_enabled = true;
    active.authority_ceiling[0].enabled = true;
    assert!(review_direct_openssh(
        &profile,
        &active,
        &observation(&profile),
        HostTrustState::Unknown
    )
    .is_err());
}

#[test]
fn source_profile_capsule_plan_and_executable_changes_invalidate_the_review() {
    let profile = alias_profile("prod");
    let plan = plan(&profile);
    let current_observation = observation(&profile);
    let reviewed = review_direct_openssh(
        &profile,
        &plan,
        &current_observation,
        HostTrustState::Unknown,
    )
    .unwrap();

    let mut changed_profile = profile.clone();
    changed_profile.revision += 1;
    assert!(reviewed
        .request
        .validate_current(
            &changed_profile,
            &plan,
            &current_observation,
            &HostTrustState::Unknown,
            NOW_MS,
        )
        .is_err());

    let mut changed_source = profile.clone();
    changed_source.source.revision = "source-9".into();
    assert!(reviewed
        .request
        .validate_current(
            &changed_source,
            &plan,
            &current_observation,
            &HostTrustState::Unknown,
            NOW_MS,
        )
        .is_err());

    let mut changed_capsule = profile.clone();
    changed_capsule.capsule.revision += 1;
    assert!(reviewed
        .request
        .validate_current(
            &changed_capsule,
            &plan,
            &current_observation,
            &HostTrustState::Unknown,
            NOW_MS,
        )
        .is_err());

    let mut changed_destination = profile.clone();
    changed_destination.transport = TransportDescriptor::OpenSshAlias {
        alias: "other".into(),
    };
    assert!(reviewed
        .request
        .validate_current(
            &changed_destination,
            &plan,
            &current_observation,
            &HostTrustState::Unknown,
            NOW_MS,
        )
        .is_err());

    let mut changed_plan = plan.clone();
    changed_plan.approval_fingerprint = digest('c');
    assert!(reviewed
        .request
        .validate_current(
            &profile,
            &changed_plan,
            &current_observation,
            &HostTrustState::Unknown,
            NOW_MS,
        )
        .is_err());

    let mut changed_identity = plan;
    changed_identity.executable_identities[0].identity_digest = digest('d');
    assert!(reviewed
        .request
        .validate_current(
            &profile,
            &changed_identity,
            &current_observation,
            &HostTrustState::Unknown,
            NOW_MS,
        )
        .is_err());
}

#[test]
fn mismatched_or_stale_identity_observations_are_not_treated_as_ready() {
    let profile = alias_profile("prod");
    let mut mismatched = observation(&profile);
    mismatched.connection_id = "other-profile".into();
    assert!(review_direct_openssh(
        &profile,
        &plan(&profile),
        &mismatched,
        HostTrustState::Unknown
    )
    .is_err());

    let current = observation(&profile);
    let stale_now = current.observed_at_ms + current.stale_after_ms;
    assert!(review_direct_openssh_at(
        &profile,
        &plan(&profile),
        &current,
        HostTrustState::Unknown,
        stale_now,
    )
    .is_err());

    let mut stale = observation(&profile);
    stale.auth_state = AuthState::Stale {
        previous: automexia_devops::connections::StaleAuthState::Ready,
    };
    let reviewed =
        review_direct_openssh(&profile, &plan(&profile), &stale, HostTrustState::Unknown)
            .unwrap();
    assert_eq!(
        reviewed.identity_readiness,
        DirectOpenSshIdentityReadiness::Stale
    );
    assert!(!reviewed.execution_enabled);
}
#[test]
fn trust_and_identity_observation_changes_require_a_fresh_review() {
    let profile = alias_profile("prod");
    let plan = plan(&profile);
    let current_observation = observation(&profile);
    let first = review_direct_openssh(
        &profile,
        &plan,
        &current_observation,
        HostTrustState::Unknown,
    )
    .unwrap();
    let stale_now =
        current_observation.observed_at_ms + current_observation.stale_after_ms;
    assert!(first
        .request
        .validate_current(
            &profile,
            &plan,
            &current_observation,
            &HostTrustState::Unknown,
            stale_now,
        )
        .is_err());

    let mut newer_observation = observation(&profile);
    newer_observation.generation += 1;
    let newer = review_direct_openssh(
        &profile,
        &plan,
        &newer_observation,
        HostTrustState::Unknown,
    )
    .unwrap();
    assert_ne!(
        first.request.review_fingerprint(),
        newer.request.review_fingerprint()
    );
    assert!(first
        .request
        .validate_current(
            &profile,
            &plan,
            &newer_observation,
            &HostTrustState::Unknown,
            NOW_MS,
        )
        .is_err());

    let mut changed_same_generation = observation(&profile);
    changed_same_generation.public_identity_summary = "Changed identity evidence".into();
    let changed = review_direct_openssh(
        &profile,
        &plan,
        &changed_same_generation,
        HostTrustState::Unknown,
    )
    .unwrap();
    assert_ne!(
        first.request.review_fingerprint(),
        changed.request.review_fingerprint()
    );
    assert!(first
        .request
        .validate_current(
            &profile,
            &plan,
            &changed_same_generation,
            &HostTrustState::Unknown,
            NOW_MS,
        )
        .is_err());

    let known_trust = HostTrustState::Known {
        fingerprint_sha256: digest('b'),
    };
    let known = review_direct_openssh(
        &profile,
        &plan,
        &observation(&profile),
        known_trust.clone(),
    )
    .unwrap();
    assert_ne!(
        first.request.review_fingerprint(),
        known.request.review_fingerprint()
    );
    assert!(first
        .request
        .validate_current(
            &profile,
            &plan,
            &observation(&profile),
            &known_trust,
            NOW_MS
        )
        .is_err());
}

#[test]
fn identity_freshness_rejects_future_overflow_and_expired_evidence() {
    let profile = alias_profile("prod");
    let plan = plan(&profile);

    let mut future = observation(&profile);
    future.observed_at_ms = NOW_MS + 1;
    assert!(review_direct_openssh_at(
        &profile,
        &plan,
        &future,
        HostTrustState::Unknown,
        NOW_MS,
    )
    .is_err());

    let mut overflowing = observation(&profile);
    overflowing.observed_at_ms = u64::MAX - 5;
    overflowing.stale_after_ms = 10;
    assert!(review_direct_openssh_at(
        &profile,
        &plan,
        &overflowing,
        HostTrustState::Unknown,
        overflowing.observed_at_ms,
    )
    .is_err());

    let mut expired_observation = observation(&profile);
    expired_observation.expires_at_ms = Some(NOW_MS);
    assert!(review_direct_openssh_at(
        &profile,
        &plan,
        &expired_observation,
        HostTrustState::Unknown,
        NOW_MS,
    )
    .is_err());

    let mut expired_auth = observation(&profile);
    expired_auth.auth_state = AuthState::Ready {
        evidence_id: "expired-agent".into(),
        expires_at_ms: Some(NOW_MS),
    };
    assert!(review_direct_openssh_at(
        &profile,
        &plan,
        &expired_auth,
        HostTrustState::Unknown,
        NOW_MS,
    )
    .is_err());
}

#[test]
fn forged_plan_non_applicable_trust_and_misleading_literal_target_fail_closed() {
    let profile = alias_profile("prod");
    let mut forged = plan(&profile);
    forged.steps.clear();
    assert!(review_direct_openssh(
        &profile,
        &forged,
        &observation(&profile),
        HostTrustState::Unknown
    )
    .is_err());

    let mut altered_metadata = plan(&profile);
    altered_metadata.steps[0].timeout_ms += 1;
    assert!(review_direct_openssh(
        &profile,
        &altered_metadata,
        &observation(&profile),
        HostTrustState::Unknown
    )
    .is_err());

    assert!(review_direct_openssh(
        &profile,
        &plan(&profile),
        &observation(&profile),
        HostTrustState::NotApplicable
    )
    .is_err());

    let mut misleading = literal_profile("actual.example.invalid");
    misleading.public_target = "reviewed.example.invalid".into();
    assert!(review_direct_openssh(
        &misleading,
        &plan(&misleading),
        &observation(&misleading),
        HostTrustState::Unknown
    )
    .is_err());
}

#[test]
fn only_a_current_review_can_create_a_redacted_launch_binding() {
    let profile = alias_profile("private-binding-canary");
    let plan = plan(&profile);
    let observation = observation(&profile);
    let trust = HostTrustState::Known {
        fingerprint_sha256: digest('b'),
    };
    let reviewed =
        review_direct_openssh(&profile, &plan, &observation, trust.clone()).unwrap();
    let binding = reviewed
        .bind_launch(&profile, &plan, &observation, &trust, NOW_MS)
        .unwrap();

    assert_eq!(binding.public_connection_id(), profile.id);
    assert_eq!(binding.source_revision(), profile.source.revision);
    assert_eq!(binding.capsule_revision(), profile.capsule.revision);
    assert_eq!(binding.arguments().last(), Some(&"private-binding-canary"));
    assert_eq!(
        binding.review_fingerprint(),
        reviewed.request.review_fingerprint()
    );
    assert!(!format!("{binding:?}").contains("private-binding-canary"));

    let mut changed_observation = observation.clone();
    changed_observation.generation += 1;
    assert!(reviewed
        .bind_launch(&profile, &plan, &changed_observation, &trust, NOW_MS,)
        .is_err());

    let expired_at = observation.observed_at_ms + observation.stale_after_ms;
    assert!(reviewed
        .bind_launch(&profile, &plan, &observation, &trust, expired_at)
        .is_err());
}
