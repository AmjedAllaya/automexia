use std::hint::black_box;

use automexia_connectivity::connections::*;
use criterion::{criterion_group, criterion_main, Criterion};

fn digest(character: char) -> String {
    character.to_string().repeat(64)
}

fn profile() -> ConnectionProfileV1 {
    ConnectionProfileV1 {
        schema_version: 1,
        id: "benchmark-profile".into(),
        revision: 1,
        display_name: "Benchmark profile".into(),
        description: String::new(),
        tags: vec!["benchmark".into()],
        favorite: false,
        environment: EnvironmentClassification {
            kind: EnvironmentKind::Development,
            label: "Development".into(),
            risk: EnvironmentRisk::Development,
        },
        provider: ProviderKind::Ssh,
        transport: TransportDescriptor::OpenSshAlias {
            alias: "benchmark-host".into(),
            host: None,
            port: None,
            user: None,
            proxy_jump: Vec::new(),
        },
        public_target: "benchmark-host".into(),
        jump_profile_references: Vec::new(),
        identity: IdentityReference {
            kind: IdentityKind::Agent,
            reference: OpaqueReference::new("identity-benchmark"),
            public_label: "External benchmark agent".into(),
            owner: "open-ssh".into(),
        },
        capsule: EnvironmentCapsuleTemplate {
            revision: 1,
            public_environment: Vec::new(),
            context_references: Vec::new(),
            provider_contexts: Vec::new(),
        },
        recipe_references: vec![RecipeReference {
            id: "benchmark-recipe".into(),
            revision: 1,
            fingerprint: digest('a'),
        }],
        tunnels: Vec::new(),
        destination_preference: DestinationSurface::Pane,
        source: ConnectionSource {
            kind: SourceKind::User,
            reference: OpaqueReference::new("source-benchmark"),
            revision: "1".into(),
        },
        approval_fingerprint: None,
        created_at_ms: 1,
        updated_at_ms: 1,
        last_used_at_ms: None,
    }
}

fn recipe() -> AutomationRecipeV1 {
    AutomationRecipeV1 {
        schema_version: 1,
        id: "benchmark-recipe".into(),
        revision: 1,
        display_name: "Benchmark recipe".into(),
        description: String::new(),
        compatible_providers: vec![ProviderKind::Ssh],
        compatible_transports: vec![TransportKind::OpenSsh],
        variables: Vec::new(),
        steps: (0..64)
            .map(|index| AutomationStepV1 {
                schema_version: 1,
                id: format!("step-{index:02}"),
                stage: ExecutionStage::Preflight,
                action: AutomationAction::CheckAgentState {
                    agent_kind: "openssh-agent".into(),
                },
                depends_on: Vec::new(),
                preconditions: Vec::new(),
                timeout_ms: 1_000,
                failure_policy: FailurePolicy::StopAndKeepDiagnostic,
                retry_policy: RetryPolicy::Never,
                risk: ActionRisk::Observe,
                confirmation_policy: ConfirmationPolicy::ReviewWithRecipe,
                reconnect_policy: ReconnectPolicy::OncePerConnection,
            })
            .collect(),
        approval_fingerprint: None,
        created_at_ms: 1,
        updated_at_ms: 1,
    }
}

fn connection_plan_64_steps(criterion: &mut Criterion) {
    let recipe = recipe();
    let mut profile = profile();
    profile.recipe_references[0].fingerprint = fingerprint_recipe(&recipe).unwrap();
    let context = PlanContext {
        executable_identities: vec![ResolvedExecutable {
            executable_id: "openssh".into(),
            identity_digest: digest('e'),
        }],
        requested_capabilities: vec!["session.launch.openssh".into()],
        public_variables: Default::default(),
    };
    criterion.bench_function("connection_plan_validate_64_steps", |bencher| {
        bencher.iter(|| validate_recipe(black_box(&recipe)).unwrap())
    });
    criterion.bench_function("connection_plan_resolve_64_steps", |bencher| {
        bencher.iter(|| {
            black_box(resolve_connection_plan(
                black_box(&profile),
                black_box(std::slice::from_ref(&recipe)),
                black_box(&context),
            ))
            .unwrap()
        })
    });
}

fn direct_openssh_preparation(criterion: &mut Criterion) {
    let mut profile = profile();
    profile.recipe_references.clear();
    profile.source.kind = SourceKind::OpenSshInventory;
    profile.destination_preference = DestinationSurface::PaneTab;
    criterion.bench_function("direct_openssh_prepare_selected", |bencher| {
        bencher.iter(|| black_box(prepare_direct_openssh(black_box(&profile))).unwrap())
    });
}

fn workspace() -> WorkspaceIntentV1 {
    let windows = (0..16)
        .map(|window_index| WorkspaceWindowIntentV1 {
            id: format!("window-{window_index}"),
            panes: (0..4)
                .map(|pane_index| WorkspacePaneIntentV1 {
                    id: format!("window-{window_index}-pane-{pane_index}"),
                    parent_pane_id: (pane_index > 0)
                        .then(|| format!("window-{window_index}-pane-0")),
                    split: (pane_index > 0).then_some(WorkspaceSplitIntent {
                        axis: if pane_index % 2 == 0 {
                            WorkspaceSplitAxis::Horizontal
                        } else {
                            WorkspaceSplitAxis::Vertical
                        },
                        ratio_basis_points: 5_000,
                    }),
                })
                .collect(),
        })
        .collect();
    let connections = (0..128)
        .map(|index| {
            let window_index = index % 16;
            let pane_index = index % 4;
            WorkspaceConnectionIntentV1 {
                id: format!("connection-{index}"),
                window_id: format!("window-{window_index}"),
                pane_id: format!("window-{window_index}-pane-{pane_index}"),
                profile_id: format!("profile-{index}"),
                profile_revision: 1,
                profile_fingerprint: digest('b'),
                recipe_fingerprints: Vec::new(),
                destination_surface: DestinationSurface::PaneTab,
            }
        })
        .collect();
    WorkspaceIntentV1 {
        schema_version: 1,
        id: "benchmark-workspace".into(),
        revision: 1,
        display_name: "Benchmark workspace".into(),
        description: String::new(),
        environment: EnvironmentClassification {
            kind: EnvironmentKind::Development,
            label: "Development".into(),
            risk: EnvironmentRisk::Development,
        },
        windows,
        connections,
        approval_fingerprint: None,
        created_at_ms: 1,
        updated_at_ms: 1,
    }
}

fn m6_workspace_and_broadcast_planning(criterion: &mut Criterion) {
    let workspace = workspace();
    let workspace_json = serde_json::to_vec(&workspace).unwrap();
    let bindings = (0..MAX_WORKSPACE_CONNECTIONS)
        .map(|index| WorkspaceProfileBinding {
            profile_id: format!("profile-{index}"),
            profile_revision: 1,
            profile_fingerprint: digest('b'),
        })
        .collect::<Vec<_>>();
    let targets = (0..MAX_BROADCAST_TARGETS)
        .map(|index| BroadcastTargetV1 {
            id: format!("target-{index}"),
            public_label: format!("Target {index}"),
            profile_id: format!("profile-{index}"),
            profile_revision: 1,
            environment_risk: EnvironmentRisk::Development,
        })
        .collect::<Vec<_>>();
    criterion.bench_function(
        "workspace_validate_16_windows_64_panes_128_connections",
        |bencher| bencher.iter(|| validate_workspace(black_box(&workspace)).unwrap()),
    );
    criterion.bench_function(
        "workspace_parse_strict_json_16_windows_64_panes_128_connections",
        |bencher| {
            bencher.iter(|| {
                black_box(parse_workspace_json(black_box(&workspace_json))).unwrap()
            })
        },
    );
    criterion.bench_function(
        "connection_plan_workspace_restore_128_connections",
        |bencher| {
            bencher.iter(|| {
                black_box(resolve_workspace_restore(
                    black_box(&workspace),
                    black_box(&bindings),
                    1,
                ))
                .unwrap()
            })
        },
    );
    criterion.bench_function("broadcast_review_50_targets", |bencher| {
        bencher.iter(|| {
            black_box(review_broadcast(
                black_box("uptime"),
                black_box(&targets),
                1,
                5_000,
            ))
            .unwrap()
        })
    });
}
fn provider_context(index: usize) -> ProviderContextTemplate {
    ProviderContextTemplate {
        provider: ProviderKind::Aws,
        configuration_reference: OpaqueReference::new(format!("config-{index}")),
        public_identity: format!("account-{index}"),
        scope: vec![ProviderScopeBinding {
            name: "region".into(),
            public_value: "eu-west-3".into(),
        }],
        provenance: ProviderContextProvenance {
            kind: ProviderProvenanceKind::UserSelected,
            source_reference: OpaqueReference::new(format!("source-{index}")),
            source_revision: "revision-1".into(),
            observed_at_ms: 1,
        },
        freshness: ProviderContextFreshness::Current,
        expires_at_ms: Some(60_000),
        risk: EnvironmentRisk::Development,
    }
}

fn m7_provider_capsule_isolation(criterion: &mut Criterion) {
    let capsules = (0..MAX_PROVIDER_CAPSULES)
        .map(|index| ProviderCapsule {
            schema_version: CONNECTION_SCHEMA_VERSION,
            capsule_id: format!("capsule-{index}"),
            session_id: index as u64 + 1,
            revision: 1,
            contexts: vec![provider_context(index)],
            created_at_ms: 1,
        })
        .collect::<Vec<_>>();
    criterion.bench_function("provider_auth_bind_and_read_64_capsules", |bencher| {
        bencher.iter(|| {
            let mut store = ProviderAuthCapsuleStore::new(MAX_PROVIDER_CAPSULES);
            for capsule in black_box(&capsules) {
                store.bind(capsule.clone()).unwrap();
            }
            for index in 0..MAX_PROVIDER_CAPSULES {
                black_box(
                    store
                        .cached(
                            black_box(&format!("capsule-{index}")),
                            index as u64 + 1,
                            ProviderKind::Aws,
                        )
                        .unwrap(),
                );
            }
        })
    });
}
criterion_group!(
    benches,
    connection_plan_64_steps,
    direct_openssh_preparation,
    m6_workspace_and_broadcast_planning,
    m7_provider_capsule_isolation
);
criterion_main!(benches);
