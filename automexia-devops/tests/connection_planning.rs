use std::collections::BTreeMap;

use automexia_devops::connections::{
    apply_auth_event, apply_result_event, parse_profile_json, parse_recipe_json,
    resolve_connection_plan, AuthEvent, AuthState, ConnectionModelErrorCode,
    ConnectionProfileV1, OperationResultEvent, OperationResultState, PlanContext,
    ResolvedExecutable,
};
use serde_json::{json, Value};

fn digest(byte: char) -> String {
    byte.to_string().repeat(64)
}

fn profile_value() -> Value {
    json!({
        "schema_version": 1,
        "id": "profile-prod",
        "revision": 7,
        "display_name": "Production bastion",
        "description": "Synthetic public fixture",
        "tags": ["production", "payments"],
        "favorite": true,
        "environment": {
            "kind": "production",
            "label": "Production",
            "risk": "production"
        },
        "provider": "ssh",
        "transport": {
            "kind": "open-ssh-explicit",
            "host": "prod.example.invalid",
            "port": 22,
            "user": "operator",
            "proxy_jump": ["jump-corp"]
        },
        "public_target": "prod.example.invalid",
        "identity": {
            "kind": "agent",
            "reference": "identity-prod",
            "public_label": "operator via external agent",
            "owner": "open-ssh"
        },
        "capsule": {
            "revision": 3,
            "public_environment": [
                {"name": "AUTOMEXIA_ENVIRONMENT", "value": "production"}
            ],
            "context_references": ["context-prod"]
        },
        "recipe_references": [
            {"id": "recipe-base", "revision": 2, "fingerprint": digest('a')}
        ],
        "tunnels": [{
            "schema_version": 1,
            "id": "postgres",
            "kind": "local",
            "bind_address": "127.0.0.1",
            "listen_port": 15432,
            "destination_host": "database.internal",
            "destination_port": 5432,
            "lifetime": "session"
        }],
        "destination_preference": "pane-tab",
        "source": {
            "kind": "user",
            "reference": "source-profile-prod",
            "revision": "8"
        },
        "approval_fingerprint": null,
        "created_at_ms": 1_700_000_000_000_u64,
        "updated_at_ms": 1_700_000_100_000_u64,
        "last_used_at_ms": null
    })
}

fn recipe_value() -> Value {
    json!({
        "schema_version": 1,
        "id": "recipe-base",
        "revision": 2,
        "display_name": "Platform preflight",
        "description": "Synthetic capability-free plan",
        "compatible_providers": ["ssh"],
        "compatible_transports": ["open-ssh"],
        "variables": [],
        "steps": [
            {
                "schema_version": 1,
                "id": "require-ssh",
                "stage": "preflight",
                "action": {"kind": "require-executable", "executable_id": "openssh"},
                "depends_on": [],
                "preconditions": [],
                "timeout_ms": 1_000,
                "failure_policy": "stop-and-keep-diagnostic",
                "retry_policy": {
                    "kind": "automatic",
                    "max_attempts": 3,
                    "initial_backoff_ms": 50,
                    "max_backoff_ms": 200,
                    "total_deadline_ms": 1_000,
                    "jitter_percent": 10,
                    "idempotent": true,
                    "interaction_free": true,
                    "persistent_mutation_free": true,
                    "cancellation_safe": true
                },
                "risk": "observe",
                "confirmation_policy": "review-with-profile",
                "reconnect_policy": "once-per-connection"
            },
            {
                "schema_version": 1,
                "id": "verify-user",
                "stage": "verify",
                "action": {"kind": "verify-remote-user"},
                "depends_on": ["require-ssh"],
                "preconditions": [],
                "timeout_ms": 2_000,
                "failure_policy": "stop-and-keep-diagnostic",
                "retry_policy": {"kind": "never"},
                "risk": "remote-session",
                "confirmation_policy": "review-with-recipe",
                "reconnect_policy": "once-per-connection"
            }
        ],
        "approval_fingerprint": null,
        "created_at_ms": 1_700_000_000_000_u64,
        "updated_at_ms": 1_700_000_100_000_u64
    })
}

fn parse_profile(value: &Value) -> ConnectionProfileV1 {
    parse_profile_json(&serde_json::to_vec(value).unwrap())
        .unwrap()
        .into_inner()
}

fn plan_context() -> PlanContext {
    PlanContext {
        executable_identities: vec![ResolvedExecutable {
            executable_id: "openssh".into(),
            identity_digest: digest('e'),
        }],
        requested_capabilities: vec!["session.launch.openssh".into()],
        public_variables: BTreeMap::new(),
    }
}

#[test]
fn strict_profiles_and_recipes_compile_to_a_non_executing_plan() {
    let profile = parse_profile(&profile_value());
    let recipe = parse_recipe_json(&serde_json::to_vec(&recipe_value()).unwrap())
        .unwrap()
        .into_inner();
    let plan = resolve_connection_plan(&profile, &[recipe], &plan_context()).unwrap();

    assert_eq!(plan.schema_version, 1);
    assert_eq!(plan.profile_id, "profile-prod");
    assert_eq!(plan.steps.len(), 4, "resolve and connect are synthesized");
    assert_eq!(plan.approval_fingerprint.len(), 64);
    assert!(!plan.execution_enabled);
    assert!(plan
        .authority_ceiling
        .iter()
        .all(|authority| !authority.enabled));
}

#[test]
fn hostile_unknown_secret_command_and_bidi_fields_fail_closed() {
    let mut unknown = profile_value();
    unknown["password"] = json!("canary-secret");
    let error = parse_profile_json(&serde_json::to_vec(&unknown).unwrap()).unwrap_err();
    assert_eq!(error.code, ConnectionModelErrorCode::MalformedSchema);
    assert!(!error.to_string().contains("canary-secret"));

    let mut command = recipe_value();
    command["steps"][0]["action"] = json!({"kind": "command", "command": "ssh prod"});
    assert_eq!(
        parse_recipe_json(&serde_json::to_vec(&command).unwrap())
            .unwrap_err()
            .code,
        ConnectionModelErrorCode::MalformedSchema
    );

    let mut bidi = profile_value();
    bidi["display_name"] = json!("prod\u{202e}ved");
    assert_eq!(
        parse_profile_json(&serde_json::to_vec(&bidi).unwrap())
            .unwrap_err()
            .code,
        ConnectionModelErrorCode::UnsafeText
    );
}

#[test]
fn duplicates_cycles_limits_and_policy_mismatches_are_rejected() {
    let mut duplicate = recipe_value();
    duplicate["steps"][1]["id"] = json!("require-ssh");
    assert_eq!(
        parse_recipe_json(&serde_json::to_vec(&duplicate).unwrap())
            .unwrap_err()
            .code,
        ConnectionModelErrorCode::DuplicateId
    );

    let mut cycle = recipe_value();
    cycle["steps"][0]["depends_on"] = json!(["verify-user"]);
    assert_eq!(
        parse_recipe_json(&serde_json::to_vec(&cycle).unwrap())
            .unwrap_err()
            .code,
        ConnectionModelErrorCode::DependencyCycle
    );

    let mut unsafe_retry = recipe_value();
    unsafe_retry["steps"][0]["risk"] = json!("authenticate");
    assert_eq!(
        parse_recipe_json(&serde_json::to_vec(&unsafe_retry).unwrap())
            .unwrap_err()
            .code,
        ConnectionModelErrorCode::InvalidPolicy
    );

    let mut oversized = profile_value();
    oversized["display_name"] = json!("x".repeat(4_097));
    assert_eq!(
        parse_profile_json(&serde_json::to_vec(&oversized).unwrap())
            .unwrap_err()
            .code,
        ConnectionModelErrorCode::LimitExceeded
    );
}

#[test]
fn every_material_plan_change_invalidates_the_fingerprint() {
    let base_profile = parse_profile(&profile_value());
    let recipe = parse_recipe_json(&serde_json::to_vec(&recipe_value()).unwrap())
        .unwrap()
        .into_inner();
    let baseline = resolve_connection_plan(
        &base_profile,
        std::slice::from_ref(&recipe),
        &plan_context(),
    )
    .unwrap()
    .approval_fingerprint;

    type ProfileMutation = Box<dyn Fn(&mut Value)>;
    let mutations: Vec<ProfileMutation> = vec![
        Box::new(|value| value["public_target"] = json!("other.example.invalid")),
        Box::new(|value| value["identity"]["reference"] = json!("identity-other")),
        Box::new(|value| value["transport"]["proxy_jump"] = json!(["jump-other"])),
        Box::new(|value| value["tunnels"][0]["listen_port"] = json!(25432)),
        Box::new(|value| {
            value["recipe_references"][0]["fingerprint"] = json!(digest('b'))
        }),
        Box::new(|value| value["source"]["revision"] = json!("9")),
    ];
    for mutate in mutations {
        let mut value = profile_value();
        mutate(&mut value);
        let changed = resolve_connection_plan(
            &parse_profile(&value),
            std::slice::from_ref(&recipe),
            &plan_context(),
        )
        .unwrap()
        .approval_fingerprint;
        assert_ne!(baseline, changed);
    }

    let mut executable = plan_context();
    executable.executable_identities[0].identity_digest = digest('f');
    assert_ne!(
        baseline,
        resolve_connection_plan(
            &base_profile,
            std::slice::from_ref(&recipe),
            &executable
        )
        .unwrap()
        .approval_fingerprint
    );

    let mut capability = plan_context();
    capability
        .requested_capabilities
        .push("listener.loopback".into());
    assert_ne!(
        baseline,
        resolve_connection_plan(&base_profile, &[recipe], &capability)
            .unwrap()
            .approval_fingerprint
    );
}

#[test]
fn authentication_and_result_reducers_cover_truthful_terminal_states() {
    let checking = apply_auth_event(
        AuthState::Unknown,
        AuthEvent::BeginCheck {
            operation_id: "operation-check".into(),
        },
    )
    .unwrap();
    let ready = apply_auth_event(
        checking,
        AuthEvent::ObservedReady {
            operation_id: "operation-check".into(),
            evidence_id: "evidence-ready".into(),
            expires_at_ms: Some(2_000),
        },
    )
    .unwrap();
    assert!(matches!(ready, AuthState::Ready { .. }));
    assert!(matches!(
        apply_auth_event(ready.clone(), AuthEvent::MarkStale).unwrap(),
        AuthState::Stale { .. }
    ));
    assert!(matches!(
        apply_auth_event(ready, AuthEvent::ExpiryReached { now_ms: 2_000 }).unwrap(),
        AuthState::Expired { .. }
    ));
    assert!(apply_auth_event(
        AuthState::Denied {
            diagnostic_code: "policy-denied".into()
        },
        AuthEvent::BeginAuthentication {
            operation_id: "operation-login".into()
        }
    )
    .is_err());

    let running =
        apply_result_event(OperationResultState::Pending, OperationResultEvent::Start)
            .unwrap();
    assert!(matches!(running, OperationResultState::Running));
    assert!(matches!(
        apply_result_event(
            running,
            OperationResultEvent::Offline {
                diagnostic_code: "network-offline".into()
            }
        )
        .unwrap(),
        OperationResultState::Offline { .. }
    ));
}

#[test]
fn the_64_step_architecture_limit_is_accepted_but_limit_plus_one_is_not() {
    let template = recipe_value()["steps"][0].clone();
    let mut exact = recipe_value();
    exact["steps"] = Value::Array(
        (0..64)
            .map(|index| {
                let mut step = template.clone();
                step["id"] = json!(format!("step-{index:02}"));
                step
            })
            .collect(),
    );
    parse_recipe_json(&serde_json::to_vec(&exact).unwrap()).unwrap();

    exact["steps"].as_array_mut().unwrap().push({
        let mut step = template;
        step["id"] = json!("step-64");
        step
    });
    assert_eq!(
        parse_recipe_json(&serde_json::to_vec(&exact).unwrap())
            .unwrap_err()
            .code,
        ConnectionModelErrorCode::LimitExceeded
    );
}

#[test]
fn profile_documents_reject_duplicate_ids_missing_jumps_and_jump_cycles() {
    let mut first = profile_value();
    first["jump_profile_references"] = json!(["profile-second"]);
    let mut second = profile_value();
    second["id"] = json!("profile-second");
    second["source"]["reference"] = json!("source-profile-second");
    second["jump_profile_references"] = json!([]);
    let document = json!({
        "schema_version": 1,
        "revision": 1,
        "profiles": [first.clone(), second.clone()]
    });
    automexia_devops::connections::parse_profile_document_json(
        &serde_json::to_vec(&document).unwrap(),
    )
    .unwrap();

    let duplicate = json!({
        "schema_version": 1,
        "revision": 1,
        "profiles": [profile_value(), profile_value()]
    });
    assert_eq!(
        automexia_devops::connections::parse_profile_document_json(
            &serde_json::to_vec(&duplicate).unwrap(),
        )
        .unwrap_err()
        .code,
        ConnectionModelErrorCode::DuplicateId
    );

    second["jump_profile_references"] = json!(["profile-prod"]);
    let cycle = json!({
        "schema_version": 1,
        "revision": 1,
        "profiles": [first.clone(), second]
    });
    assert_eq!(
        automexia_devops::connections::parse_profile_document_json(
            &serde_json::to_vec(&cycle).unwrap(),
        )
        .unwrap_err()
        .code,
        ConnectionModelErrorCode::DependencyCycle
    );

    first["jump_profile_references"] = json!(["profile-missing"]);
    let missing = json!({"schema_version": 1, "revision": 1, "profiles": [first]});
    assert_eq!(
        automexia_devops::connections::parse_profile_document_json(
            &serde_json::to_vec(&missing).unwrap(),
        )
        .unwrap_err()
        .code,
        ConnectionModelErrorCode::MissingDependency
    );
}

#[test]
fn resolved_plans_reject_cross_recipe_variable_collisions_and_preallocate_step_overflow()
{
    let mut oversized_profile = profile_value();
    let mut oversized_references = Vec::new();
    let mut oversized_recipes = Vec::new();
    for recipe_index in 0..17 {
        let recipe_id = format!("recipe-{recipe_index:02}");
        let mut value = recipe_value();
        value["id"] = json!(recipe_id);
        value["steps"] = Value::Array(
            (0..64)
                .map(|step_index| {
                    let mut step = recipe_value()["steps"][0].clone();
                    step["id"] = json!(format!("step-{step_index:02}"));
                    step
                })
                .collect(),
        );
        oversized_references.push(json!({
            "id": value["id"],
            "revision": 2,
            "fingerprint": digest('a')
        }));
        oversized_recipes.push(
            parse_recipe_json(&serde_json::to_vec(&value).unwrap())
                .unwrap()
                .into_inner(),
        );
    }
    oversized_profile["recipe_references"] = Value::Array(oversized_references);
    let error = resolve_connection_plan(
        &parse_profile(&oversized_profile),
        &oversized_recipes,
        &plan_context(),
    )
    .unwrap_err();
    assert_eq!(error.code, ConnectionModelErrorCode::LimitExceeded);

    let mut collision_profile = profile_value();
    let mut collision_references = Vec::new();
    let mut collision_recipes = Vec::new();
    for recipe_index in 0..2 {
        let mut value = recipe_value();
        value["id"] = json!(format!("collision-{recipe_index}"));
        value["variables"] = json!([{
            "id": "region",
            "prompt": "Public region",
            "required": false,
            "public_default": null
        }]);
        collision_references.push(json!({
            "id": value["id"],
            "revision": 2,
            "fingerprint": digest('a')
        }));
        collision_recipes.push(
            parse_recipe_json(&serde_json::to_vec(&value).unwrap())
                .unwrap()
                .into_inner(),
        );
    }
    collision_profile["recipe_references"] = Value::Array(collision_references);
    let error = resolve_connection_plan(
        &parse_profile(&collision_profile),
        &collision_recipes,
        &plan_context(),
    )
    .unwrap_err();
    assert_eq!(error.code, ConnectionModelErrorCode::DuplicateId);
}

#[test]
fn plan_context_rejects_hostile_bidi_variable_overrides() {
    let profile = parse_profile(&profile_value());
    let mut recipe = recipe_value();
    recipe["variables"] = json!([{
        "id": "public-label",
        "prompt": "Public deployment label",
        "required": true,
        "public_default": null
    }]);
    let recipe = parse_recipe_json(&serde_json::to_vec(&recipe).unwrap())
        .unwrap()
        .into_inner();
    let mut context = plan_context();
    context
        .public_variables
        .insert("public-label".into(), "safe\u{202e}unsafe".into());

    let error = resolve_connection_plan(&profile, &[recipe], &context).unwrap_err();
    assert_eq!(error.code, ConnectionModelErrorCode::UnsafeText);
}
