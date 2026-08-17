use std::fs;
use std::path::{Path, PathBuf};

use automexia_devops::actions::{
    parse_quick_actions, validate_quick_actions, ActionProvenance, ActionScope,
    ActionTemplate, AliasArgumentPolicy, AliasProjection, AliasProjectionMode,
    ArgumentToken, CompletionMode, ExecutionMode, OverridePolicy, Placeholder,
    PlaceholderSensitivity, QuickAction, QuickActionDocument, RiskClass, ShellKind,
    MAX_ACTIONS, MAX_ARGUMENTS_PER_ACTION, MAX_ENABLED_ALIASES,
    MAX_PLACEHOLDERS_PER_ACTION, MAX_SOURCE_BYTES, MAX_STRING_BYTES, MAX_TAGS_PER_ACTION,
};
use serde::Deserialize;

fn fixture_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/command-productivity")
}

fn read_fixture(relative: &str) -> String {
    fs::read_to_string(fixture_root().join(relative)).expect("fixture must be readable")
}

fn minimal_action(index: usize) -> QuickAction {
    QuickAction {
        id: format!("action.{index:04}"),
        display_name: "Bounded action".to_owned(),
        description: String::new(),
        tags: Vec::new(),
        scope: ActionScope::Session,
        shells: vec![ShellKind::Bash],
        template: ActionTemplate::TypedArgv {
            executable_id: "true".to_owned(),
            arguments: Vec::new(),
        },
        placeholders: Vec::new(),
        working_directory_policy: Default::default(),
        risk: RiskClass::ReadOnly,
        execution: ExecutionMode::Insert,
        provenance: ActionProvenance::User,
        enabled: true,
        alias_projection: None,
    }
}

fn assert_invalid(actions: Vec<QuickAction>, expected: &str) {
    let error = validate_quick_actions(QuickActionDocument {
        schema_version: 1,
        revision: 0,
        actions,
    })
    .expect_err("invalid model must fail");
    assert_eq!(error.code(), expected);
}

#[test]
fn valid_fixture_round_trips_without_losing_the_typed_contract() {
    let parsed = parse_quick_actions(&read_fixture("cp2-quick-actions/valid-basic.toml"))
        .expect("valid fixture");
    assert_eq!(parsed.document().schema_version, 1);
    assert_eq!(parsed.document().revision, 7);
    assert_eq!(parsed.document().actions.len(), 2);
    assert_eq!(parsed.document().actions[0].id, "kubernetes.get-resource");

    let encoded = parsed.to_toml().expect("validated model must serialize");
    let reparsed =
        parse_quick_actions(&encoded).expect("serialized model must remain valid");
    assert_eq!(reparsed, parsed);
}

#[derive(Deserialize)]
struct HostileManifest {
    schema: u32,
    phase: String,
    cases: Vec<HostileCase>,
}

#[derive(Deserialize)]
struct HostileCase {
    id: String,
    fixture: String,
    expected: String,
}

#[test]
fn versioned_hostile_corpus_fails_closed_with_stable_codes() {
    let manifest: HostileManifest =
        serde_json::from_str(&read_fixture("cp2-hostile-actions-v1.json"))
            .expect("hostile manifest");
    assert_eq!(manifest.schema, 1);
    assert_eq!(manifest.phase, "CP2.0");
    assert_eq!(manifest.cases.len(), 11);

    for case in manifest.cases {
        let source = read_fixture(&format!("cp2-quick-actions/{}", case.fixture));
        let error = parse_quick_actions(&source)
            .unwrap_err_or_else(|| panic!("hostile case {} was accepted", case.id));
        assert_eq!(error.code(), case.expected, "hostile case {}", case.id);
    }
}

trait ResultTestExt<T, E> {
    fn unwrap_err_or_else(self, on_ok: impl FnOnce() -> E) -> E;
}

impl<T, E> ResultTestExt<T, E> for Result<T, E> {
    fn unwrap_err_or_else(self, on_ok: impl FnOnce() -> E) -> E {
        match self {
            Ok(_) => on_ok(),
            Err(error) => error,
        }
    }
}

#[test]
fn source_limit_is_checked_before_toml_decode() {
    let source = "x".repeat(MAX_SOURCE_BYTES + 1);
    let error = parse_quick_actions(&source).expect_err("oversized source must fail");
    assert_eq!(error.code(), "source-too-large");
}

#[test]
fn action_limit_accepts_the_boundary_and_rejects_one_more() {
    let action = minimal_action(0);
    let mut actions = (0..MAX_ACTIONS)
        .map(|index| QuickAction {
            id: format!("action.{index:04}"),
            ..action.clone()
        })
        .collect::<Vec<_>>();
    let source = toml::to_string(&QuickActionDocument {
        schema_version: 1,
        revision: 0,
        actions: actions.clone(),
    })
    .expect("bounded document serialization");
    parse_quick_actions(&source).expect("boundary-sized action set");

    actions.push(QuickAction {
        id: "action.extra".to_owned(),
        ..action
    });
    let source = toml::to_string(&QuickActionDocument {
        schema_version: 1,
        revision: 0,
        actions,
    })
    .expect("oversized document serialization");
    let error = parse_quick_actions(&source).expect_err("one extra action must fail");
    assert_eq!(error.code(), "too-many-actions");
}

#[test]
fn every_nested_collection_limit_fails_at_one_over_the_boundary() {
    let mut action = minimal_action(0);
    action.tags = (0..=MAX_TAGS_PER_ACTION)
        .map(|index| format!("tag-{index}"))
        .collect();
    assert_invalid(vec![action], "too-many-tags");

    let mut action = minimal_action(0);
    action.placeholders = (0..=MAX_PLACEHOLDERS_PER_ACTION)
        .map(|index| Placeholder {
            name: format!("p{index}"),
            prompt: format!("Value {index}"),
            sensitivity: PlaceholderSensitivity::Public,
            required: true,
            default: None,
        })
        .collect();
    assert_invalid(vec![action], "too-many-placeholders");

    let mut action = minimal_action(0);
    action.template = ActionTemplate::TypedArgv {
        executable_id: "true".to_owned(),
        arguments: (0..=MAX_ARGUMENTS_PER_ACTION)
            .map(|index| ArgumentToken::Literal {
                value: index.to_string(),
            })
            .collect(),
    };
    assert_invalid(vec![action], "too-many-arguments");
}

#[test]
fn enabled_alias_limit_counts_enabled_actions_and_projections() {
    let actions = (0..=MAX_ENABLED_ALIASES)
        .map(|index| {
            let mut action = minimal_action(index);
            action.scope = ActionScope::GlobalUser;
            action.alias_projection = Some(AliasProjection {
                requested_name: format!("a{index}"),
                shells: vec![ShellKind::Bash],
                mode: AliasProjectionMode::Auto,
                argument_policy: AliasArgumentPolicy::None,
                completion: CompletionMode::Required,
                override_policy: OverridePolicy::NativeWins,
                mutating_acknowledged: false,
                enabled: true,
            });
            action
        })
        .collect();
    assert_invalid(actions, "too-many-enabled-aliases");
}

#[test]
fn string_limit_is_measured_in_utf8_bytes() {
    let mut action = minimal_action(0);
    action.display_name = "é".repeat(MAX_STRING_BYTES / 2);
    validate_quick_actions(QuickActionDocument {
        schema_version: 1,
        revision: 0,
        actions: vec![action.clone()],
    })
    .expect("exact byte boundary");

    action.display_name.push('é');
    assert_invalid(vec![action], "string-too-long");
}

#[test]
fn repeated_parse_and_drop_has_no_persistent_runtime_state() {
    let source = read_fixture("cp2-quick-actions/valid-basic.toml");
    for _ in 0..1_000 {
        let parsed = parse_quick_actions(&source).expect("repeatable parse");
        assert_eq!(parsed.document().revision, 7);
    }
}

#[test]
fn aggregate_canonical_document_size_is_bounded() {
    let actions = (0..MAX_ACTIONS)
        .map(|index| {
            let mut action = minimal_action(index);
            action.template = ActionTemplate::TypedArgv {
                executable_id: "true".to_owned(),
                arguments: (0..MAX_ARGUMENTS_PER_ACTION)
                    .map(|argument| ArgumentToken::Literal {
                        value: format!("argument-{argument:02}-bounded"),
                    })
                    .collect(),
            };
            action
        })
        .collect();
    assert_invalid(actions, "source-too-large");
}

#[test]
fn unicode_labels_are_preserved_while_identifiers_remain_portable() {
    let source = read_fixture("cp2-quick-actions/valid-basic.toml").replace(
        "Get a Kubernetes resource",
        "Afficher les ressources 日本語 🛡️",
    );
    let parsed = parse_quick_actions(&source).expect("safe Unicode label");
    assert_eq!(
        parsed.document().actions[0].display_name,
        "Afficher les ressources 日本語 🛡️"
    );
}
