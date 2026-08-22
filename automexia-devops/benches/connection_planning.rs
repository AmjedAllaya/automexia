use std::hint::black_box;

use automexia_devops::connections::*;
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
    let profile = profile();
    let recipe = recipe();
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

criterion_group!(
    benches,
    connection_plan_64_steps,
    direct_openssh_preparation
);
criterion_main!(benches);
