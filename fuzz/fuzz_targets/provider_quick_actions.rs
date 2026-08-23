#![no_main]

use automexia_devops::{
    actions::{
        build_provider_action_candidate, build_provider_action_snapshot,
        revalidate_provider_action, ExecutionMode, ProviderActionSpec, RiskClass,
    },
    connections::{parse_provider_capsule_json, ProviderKind},
};
use libfuzzer_sys::fuzz_target;

const MAX_FUZZ_INPUT_BYTES: usize = 64 * 1024;

fuzz_target!(|data: &[u8]| {
    if data.len() > MAX_FUZZ_INPUT_BYTES {
        return;
    }
    let Ok(capsule) = parse_provider_capsule_json(data) else {
        return;
    };
    let generated_at_ms = capsule
        .contexts
        .iter()
        .map(|context| context.provenance.observed_at_ms)
        .fold(capsule.created_at_ms, u64::max);
    let mut candidates = Vec::with_capacity(capsule.contexts.len());
    for (index, context) in capsule.contexts.iter().enumerate() {
        if matches!(
            context.provider,
            ProviderKind::None | ProviderKind::OpenBao | ProviderKind::LocalContainer
        ) {
            return;
        }
        let Some(target) = context.scope.first() else {
            return;
        };
        let execution = if data.first().is_some_and(|byte| byte & 1 == 0) {
            ExecutionMode::Insert
        } else {
            ExecutionMode::ExactLaunch
        };
        let spec = ProviderActionSpec {
            action_id: format!("provider.fuzz.action-{index}"),
            display_name: format!("Provider fuzz action {index}"),
            description: "Fuzzed cached provider action".into(),
            executable_id: "provider-tool".into(),
            arguments: vec!["inspect".into(), target.public_value.clone()],
            target_kind: target.name.clone(),
            exact_target: target.public_value.clone(),
            command_risk: RiskClass::ReadOnly,
            execution,
        };
        let Ok(candidate) = build_provider_action_candidate(
            &capsule,
            context,
            1,
            generated_at_ms,
            spec,
        ) else {
            return;
        };
        candidates.push(candidate);
    }
    let Ok(snapshot) = build_provider_action_snapshot(
        &capsule,
        1,
        generated_at_ms,
        candidates,
    ) else {
        return;
    };
    assert_eq!(snapshot.actions().len(), capsule.contexts.len());
    for candidate in snapshot.actions() {
        let review = revalidate_provider_action(
            &snapshot,
            candidate.binding(),
            generated_at_ms,
        );
        assert_eq!(review.audit().generation, snapshot.generation());
        assert!(!review.audit().snapshot_digest().is_empty());
        assert!(!review.audit().binding_digest().is_empty());
        let debug = format!("{:?}", review.audit());
        assert!(!debug.contains(candidate.binding().binding_digest()));
    }
});