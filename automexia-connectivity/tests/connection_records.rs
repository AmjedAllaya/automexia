use automexia_connectivity::connections::{
    parse_connection_definition_json, parse_connection_intent_json,
    parse_connection_observation_json, parse_connection_receipt_json,
    parse_connection_review_json, ConnectionModelErrorCode,
};
use serde_json::{json, Value};

fn digest(character: char) -> String {
    character.to_string().repeat(64)
}

fn source() -> Value {
    json!({
        "kind": "user",
        "reference": "source-connection",
        "revision": "7"
    })
}

fn transport() -> Value {
    json!({
        "kind": "open-ssh-explicit",
        "host": "host.example.invalid",
        "port": 22,
        "user": "operator",
        "proxy_jump": []
    })
}

fn identity() -> Value {
    json!({
        "kind": "agent",
        "reference": "identity-agent",
        "public_label": "operator through external agent",
        "owner": "open-ssh"
    })
}

fn capsule() -> Value {
    json!({
        "revision": 1,
        "public_environment": [
            {"name": "AUTOMEXIA_ENVIRONMENT", "value": "development"}
        ],
        "context_references": ["context-development"]
    })
}

fn intent() -> Value {
    json!({
        "schema_version": 1,
        "connection_id": "connection-development",
        "profile_revision": 3,
        "source_revision": "7",
        "public_destination": "host.example.invalid",
        "transport": transport(),
        "jump_chain": [],
        "tunnels": [],
        "identity": identity(),
        "capsule": capsule(),
        "destination_surface": "pane-tab",
        "requested_capabilities": ["session.launch"],
        "recipe_fingerprints": [digest('a')]
    })
}

fn assert_future_version_rejected(
    mut document: Value,
    parser: impl Fn(&[u8]) -> Result<(), ConnectionModelErrorCode>,
) {
    document["schema_version"] = json!(2);
    assert_eq!(
        parser(&serde_json::to_vec(&document).unwrap()).unwrap_err(),
        ConnectionModelErrorCode::UnsupportedVersion
    );
}

#[test]
fn all_top_level_connection_records_are_strict_versioned_and_bounded() {
    let definition = json!({
        "schema_version": 1,
        "id": "connection-development",
        "display_name": "Development host",
        "description": "Synthetic public-only connection definition",
        "tags": ["development"],
        "favorite": true,
        "last_used_at_ms": null,
        "source": source(),
        "provider": "ssh",
        "transport": transport(),
        "public_destination": "host.example.invalid",
        "environment_template_reference": "capsule-development",
        "identity": identity(),
        "jump_references": [],
        "tunnel_templates": [],
        "risk": "development"
    });
    assert_eq!(
        parse_connection_definition_json(&serde_json::to_vec(&definition).unwrap())
            .unwrap()
            .id,
        "connection-development"
    );
    assert_future_version_rejected(definition.clone(), |bytes| {
        parse_connection_definition_json(bytes)
            .map(|_| ())
            .map_err(|error| error.code)
    });

    let observation = json!({
        "schema_version": 1,
        "connection_id": "connection-development",
        "generation": 4,
        "auth_state": {
            "state": "ready",
            "evidence_id": "evidence-ready",
            "expires_at_ms": 3_000
        },
        "observed_at_ms": 1_000,
        "expires_at_ms": 3_000,
        "stale_after_ms": 500,
        "tool_state": "ready",
        "transport_state": "available",
        "public_identity_summary": "operator through external agent",
        "diagnostic_code": null,
        "recovery_action": null
    });
    assert_eq!(
        parse_connection_observation_json(&serde_json::to_vec(&observation).unwrap())
            .unwrap()
            .generation,
        4
    );
    assert_future_version_rejected(observation.clone(), |bytes| {
        parse_connection_observation_json(bytes)
            .map(|_| ())
            .map_err(|error| error.code)
    });

    let intent = intent();
    assert_eq!(
        parse_connection_intent_json(&serde_json::to_vec(&intent).unwrap())
            .unwrap()
            .profile_revision,
        3
    );
    assert_future_version_rejected(intent.clone(), |bytes| {
        parse_connection_intent_json(bytes)
            .map(|_| ())
            .map_err(|error| error.code)
    });

    let review = json!({
        "schema_version": 1,
        "normalized_intent": intent,
        "policy_decisions": [{
            "code": "review-production",
            "outcome": "review",
            "reason": "Explicit review remains required"
        }],
        "warnings": ["host-trust-review-required"],
        "host_trust": {
            "state": "first-use",
            "fingerprint_sha256": digest('b')
        },
        "changed_fields": ["source_revision"],
        "executable_preview": [{
            "executable_id": "openssh",
            "arguments": [
                {"label": "destination", "redacted": false},
                {"label": "identity reference", "redacted": true}
            ]
        }],
        "approval_fingerprint": digest('c')
    });
    assert_eq!(
        parse_connection_review_json(&serde_json::to_vec(&review).unwrap())
            .unwrap()
            .warnings,
        vec!["host-trust-review-required"]
    );
    assert_future_version_rejected(review.clone(), |bytes| {
        parse_connection_review_json(bytes)
            .map(|_| ())
            .map_err(|error| error.code)
    });

    let receipt = json!({
        "schema_version": 1,
        "operation_id": "operation-connect",
        "session_id": "session-development",
        "capsule_id": "capsule-development",
        "approved_intent_digest": digest('d'),
        "source_revision": "7",
        "process_ownership_references": ["process-owner"],
        "route_ownership_references": ["route-owner"],
        "tunnel_ownership_references": [],
        "started_at_ms": 2_000,
        "outcome": {"state": "succeeded"}
    });
    assert_eq!(
        parse_connection_receipt_json(&serde_json::to_vec(&receipt).unwrap())
            .unwrap()
            .operation_id,
        "operation-connect"
    );
    assert_future_version_rejected(receipt.clone(), |bytes| {
        parse_connection_receipt_json(bytes)
            .map(|_| ())
            .map_err(|error| error.code)
    });

    let mut unknown = receipt;
    unknown["credential"] = json!("secret-canary");
    let error = parse_connection_receipt_json(&serde_json::to_vec(&unknown).unwrap())
        .unwrap_err();
    assert_eq!(error.code, ConnectionModelErrorCode::MalformedSchema);
    assert!(!error.to_string().contains("secret-canary"));
}

#[test]
fn review_records_reject_duplicate_policy_and_executable_entries() {
    let review = || {
        json!({
            "schema_version": 1,
            "normalized_intent": intent(),
            "policy_decisions": [{
                "code": "review-production",
                "outcome": "review",
                "reason": "Explicit review remains required"
            }],
            "warnings": ["host-trust-review-required"],
            "host_trust": {"state": "unknown"},
            "changed_fields": ["source_revision"],
            "executable_preview": [{
                "executable_id": "openssh",
                "arguments": [{"label": "destination", "redacted": false}]
            }],
            "approval_fingerprint": digest('c')
        })
    };

    let mut duplicate_policy = review();
    let decision = duplicate_policy["policy_decisions"][0].clone();
    duplicate_policy["policy_decisions"]
        .as_array_mut()
        .unwrap()
        .push(decision);
    assert_eq!(
        parse_connection_review_json(&serde_json::to_vec(&duplicate_policy).unwrap())
            .unwrap_err()
            .code,
        ConnectionModelErrorCode::DuplicateId,
    );

    let mut duplicate_executable = review();
    let executable = duplicate_executable["executable_preview"][0].clone();
    duplicate_executable["executable_preview"]
        .as_array_mut()
        .unwrap()
        .push(executable);
    assert_eq!(
        parse_connection_review_json(&serde_json::to_vec(&duplicate_executable).unwrap())
            .unwrap_err()
            .code,
        ConnectionModelErrorCode::DuplicateId,
    );
}
