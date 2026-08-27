#![no_main]

use automexia_connectivity::connections::{
    parse_profile_document_json, parse_profile_json, parse_provider_capsule_json,
    parse_recipe_document_json, parse_recipe_json, parse_workspace_document_json,
    parse_workspace_json, review_broadcast, BroadcastTargetV1, EnvironmentRisk,
};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = parse_profile_json(data);
    let _ = parse_provider_capsule_json(data);
    let _ = parse_recipe_json(data);
    let _ = parse_profile_document_json(data);
    let _ = parse_recipe_document_json(data);
    let _ = parse_workspace_json(data);
    let _ = parse_workspace_document_json(data);
    if let Ok(command) = std::str::from_utf8(data) {
        let targets = [BroadcastTargetV1 {
            id: "fuzz-target".into(),
            public_label: "Fuzz target".into(),
            profile_id: "fuzz-profile".into(),
            profile_revision: 1,
            environment_risk: EnvironmentRisk::Development,
        }];
        let _ = review_broadcast(command, &targets, 0, 1);
    }
});
