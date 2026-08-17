use automexia_devops::connections::parse_connection_review_json;
use automexia_ui_model::connection_hub::{
    project_connection_review, AccessibilityRole, HubLayout, Viewport,
};
use serde_json::json;

fn digest(character: char) -> String {
    character.to_string().repeat(64)
}

#[test]
fn connection_review_exposes_every_decision_section_and_remains_non_executing() {
    let review = json!({
        "schema_version": 1,
        "normalized_intent": {
            "schema_version": 1,
            "connection_id": "connection-production",
            "profile_revision": 9,
            "source_revision": "12",
            "public_destination": "production.example.invalid",
            "transport": {
                "kind": "open-ssh-explicit",
                "host": "production.example.invalid",
                "port": 22,
                "user": "operator",
                "proxy_jump": ["jump-corporate"]
            },
            "jump_chain": ["jump-corporate"],
            "tunnels": [{
                "schema_version": 1,
                "id": "database",
                "kind": "local",
                "bind_address": "0.0.0.0",
                "listen_port": 15432,
                "destination_host": "database.internal",
                "destination_port": 5432,
                "lifetime": "session"
            }],
            "identity": {
                "kind": "agent",
                "reference": "identity-production",
                "public_label": "operator through external agent",
                "owner": "open-ssh"
            },
            "capsule": {
                "revision": 4,
                "public_environment": [
                    {"name": "AUTOMEXIA_ENVIRONMENT", "value": "production"}
                ],
                "context_references": ["context-production"]
            },
            "destination_surface": "workspace-tab",
            "requested_capabilities": ["session.launch", "listener.non-loopback"],
            "recipe_fingerprints": [digest('a')]
        },
        "policy_decisions": [{
            "code": "listener-denied",
            "outcome": "deny",
            "reason": "Non-loopback listeners require a later activation phase"
        }],
        "warnings": ["production-review-required"],
        "host_trust": {
            "state": "changed",
            "fingerprint_sha256": digest('b')
        },
        "changed_fields": ["source_revision", "host_trust"],
        "executable_preview": [{
            "executable_id": "openssh",
            "arguments": [{"label": "identity reference", "redacted": true}]
        }],
        "approval_fingerprint": digest('c')
    });
    let review =
        parse_connection_review_json(&serde_json::to_vec(&review).unwrap()).unwrap();

    for (viewport, layout) in [
        (Viewport::new(1_440.0, 900.0, 1.0), HubLayout::Wide),
        (Viewport::new(360.0, 640.0, 4.0), HubLayout::Narrow),
    ] {
        let view = project_connection_review(&review, viewport);
        assert_eq!(view.layout, layout);
        assert!(!view.execution_enabled);
        assert_eq!(view.sections.len(), 7);
        assert_eq!(
            view.sections
                .iter()
                .map(|section| section.id.as_str())
                .collect::<Vec<_>>(),
            vec![
                "identity",
                "target",
                "transport",
                "tunnels",
                "host-trust",
                "capabilities",
                "destination"
            ]
        );
        assert!(view
            .sections
            .iter()
            .filter(|section| matches!(
                section.id.as_str(),
                "tunnels" | "host-trust" | "capabilities"
            ))
            .all(|section| section.blocking));
        assert!(view.accessibility_tree.iter().any(|node| {
            node.id == "review-primary"
                && node.role == AccessibilityRole::Button
                && node.focusable
                && node.disabled
        }));
    }
}
