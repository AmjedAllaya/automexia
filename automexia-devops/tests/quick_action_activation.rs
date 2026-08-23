use automexia_devops::actions::{
    expand_for_shell, merge_action_search_hits, ActionIndex, ActionLayer,
    ActionProvenance, ActionScope, ActionSearchHit, ActionTemplate, ArgumentToken,
    ExecutionMode, ExpansionError, LayerIdentity, Placeholder, PlaceholderBindings,
    PlaceholderSensitivity, QuickAction, RiskClass, SearchContext, ShellKind,
    WorkingDirectoryPolicy,
};
use std::sync::Arc;

fn action(id: &str, scope: ActionScope, shell: ShellKind) -> QuickAction {
    QuickAction {
        id: id.into(),
        display_name: format!("Action {id}"),
        description: "inspect a target safely".into(),
        tags: vec!["devops".into()],
        scope,
        shells: vec![shell],
        template: ActionTemplate::TypedArgv {
            executable_id: "printf".into(),
            arguments: vec![ArgumentToken::Placeholder {
                name: "target".into(),
            }],
        },
        placeholders: vec![Placeholder {
            name: "target".into(),
            prompt: "Target".into(),
            sensitivity: PlaceholderSensitivity::Public,
            required: true,
            default: None,
        }],
        working_directory_policy: WorkingDirectoryPolicy::Inherit,
        risk: RiskClass::ReadOnly,
        execution: ExecutionMode::Insert,
        provenance: ActionProvenance::User,
        enabled: true,
        alias_projection: None,
    }
}

fn context(shell: ShellKind) -> SearchContext {
    SearchContext {
        session_id: 7,
        capsule_revision: 3,
        workspace_identity: Some("workspace-a".into()),
        workspace_trusted: true,
        shell,
    }
}

#[test]
fn layered_search_is_deterministic_and_higher_scope_shadows_lower_scope() {
    let index = ActionIndex::build(vec![
        ActionLayer {
            identity: LayerIdentity::GlobalUser,
            revision: 4,
            actions: vec![action(
                "shared.action",
                ActionScope::GlobalUser,
                ShellKind::Bash,
            )],
        },
        ActionLayer {
            identity: LayerIdentity::Session { session_id: 7 },
            revision: 1,
            actions: vec![action(
                "shared.action",
                ActionScope::Session,
                ShellKind::Bash,
            )],
        },
        ActionLayer {
            identity: LayerIdentity::TrustedWorkspace {
                identity: "workspace-a".into(),
            },
            revision: 2,
            actions: vec![action(
                "workspace.action",
                ActionScope::TrustedWorkspace,
                ShellKind::Bash,
            )],
        },
    ])
    .unwrap();

    let hits = index.search("action", &context(ShellKind::Bash)).unwrap();
    assert_eq!(hits.len(), 2);
    let shared = hits
        .iter()
        .find(|hit| hit.action.id == "shared.action")
        .unwrap();
    assert_eq!(shared.source, "Session");
    assert_eq!(shared.shadowed_count, 1);
    assert_eq!(index.conflicts(&context(ShellKind::Bash)).len(), 1);

    let mut untrusted = context(ShellKind::Bash);
    untrusted.workspace_trusted = false;
    assert!(index.search("workspace", &untrusted).unwrap().is_empty());
}

#[test]
fn global_user_layer_rejects_session_or_workspace_scopes() {
    let error = ActionIndex::build(vec![ActionLayer {
        identity: LayerIdentity::GlobalUser,
        revision: 1,
        actions: vec![action("wrong.scope", ActionScope::Session, ShellKind::Bash)],
    }])
    .unwrap_err();
    assert!(error.to_string().contains("incompatible"));
}

#[test]
fn typed_arguments_use_separate_shell_quoting_and_never_append_enter() {
    let value = "space ' quote/\u{9879}\u{76ee}";
    let mut bindings = PlaceholderBindings::default();
    bindings.insert("target", value);

    let bash = expand_for_shell(
        &action("quote.bash", ActionScope::GlobalUser, ShellKind::Bash),
        ShellKind::Bash,
        &bindings,
    )
    .unwrap();
    assert_eq!(bash.command, "printf 'space '\\'' quote/\u{9879}\u{76ee}'");
    assert!(!bash.command.contains(['\r', '\n']));

    let powershell = expand_for_shell(
        &action("quote.ps", ActionScope::GlobalUser, ShellKind::Powershell),
        ShellKind::Powershell,
        &bindings,
    )
    .unwrap();
    assert_eq!(
        powershell.command,
        "printf 'space '' quote/\u{9879}\u{76ee}'"
    );

    let fish = expand_for_shell(
        &action("quote.fish", ActionScope::GlobalUser, ShellKind::Fish),
        ShellKind::Fish,
        &bindings,
    )
    .unwrap();
    assert_eq!(fish.command, "printf 'space \\' quote/\u{9879}\u{76ee}'");
}

#[test]
fn cmd_accepts_unicode_and_spaces_but_rejects_expansion_metacharacters() {
    let action = action("quote.cmd", ActionScope::GlobalUser, ShellKind::Cmd);
    let mut bindings = PlaceholderBindings::default();
    bindings.insert("target", "folder name \u{9879}\u{76ee}");
    assert_eq!(
        expand_for_shell(&action, ShellKind::Cmd, &bindings)
            .unwrap()
            .command,
        "printf \"folder name \u{9879}\u{76ee}\""
    );
    bindings.insert("target", "%PATH% & whoami");
    assert_eq!(
        expand_for_shell(&action, ShellKind::Cmd, &bindings).unwrap_err(),
        ExpansionError::UnsafeValue
    );
}

#[test]
fn exact_launch_controls_and_secret_references_fail_closed() {
    let mut exact = action("exact.action", ActionScope::GlobalUser, ShellKind::Bash);
    exact.execution = ExecutionMode::ExactLaunch;
    assert_eq!(
        expand_for_shell(&exact, ShellKind::Bash, &PlaceholderBindings::default())
            .unwrap_err(),
        ExpansionError::ExactLaunchDisabled
    );

    let mut secret = action("secret.action", ActionScope::GlobalUser, ShellKind::Bash);
    secret.placeholders[0].sensitivity = PlaceholderSensitivity::SecretReference;
    let error =
        expand_for_shell(&secret, ShellKind::Bash, &PlaceholderBindings::default())
            .unwrap_err();
    assert!(matches!(
        error,
        ExpansionError::SecretReferenceUnavailable(_)
    ));

    let mut bindings = PlaceholderBindings::default();
    bindings.insert("target", "hello\nworld");
    assert_eq!(
        expand_for_shell(
            &action("control.action", ActionScope::GlobalUser, ShellKind::Bash),
            ShellKind::Bash,
            &bindings
        )
        .unwrap_err(),
        ExpansionError::UnsafeValue
    );
}

#[test]
fn search_rejects_controls_and_obeys_shell_and_enabled_filters() {
    let mut disabled =
        action("disabled.action", ActionScope::GlobalUser, ShellKind::Bash);
    disabled.enabled = false;
    let index = ActionIndex::build(vec![ActionLayer {
        identity: LayerIdentity::GlobalUser,
        revision: 1,
        actions: vec![
            disabled,
            action("visible.action", ActionScope::GlobalUser, ShellKind::Bash),
        ],
    }])
    .unwrap();
    assert_eq!(
        index
            .search("visible", &context(ShellKind::Bash))
            .unwrap()
            .len(),
        1
    );
    assert!(index
        .search("visible", &context(ShellKind::Zsh))
        .unwrap()
        .is_empty());
    assert!(index
        .search("bad\u{1b}", &context(ShellKind::Bash))
        .is_err());
}

#[test]
fn shell_user_precedence_is_distinct_from_global_user() {
    let index = ActionIndex::build(vec![
        ActionLayer {
            identity: LayerIdentity::GlobalUser,
            revision: 4,
            actions: vec![action(
                "shared.user-action",
                ActionScope::GlobalUser,
                ShellKind::Bash,
            )],
        },
        ActionLayer {
            identity: LayerIdentity::ShellUser,
            revision: 7,
            actions: vec![action(
                "shared.user-action",
                ActionScope::ShellUser,
                ShellKind::Bash,
            )],
        },
    ])
    .unwrap();

    let hits = index.search("shared", &context(ShellKind::Bash)).unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].source, "Shell user");
    assert_eq!(hits[0].source_revision, 7);
    assert_eq!(hits[0].shadowed_count, 1);

    let zsh_hits = index.search("shared", &context(ShellKind::Zsh)).unwrap();
    assert!(zsh_hits.is_empty(), "shell filters remain authoritative");
}

#[test]
fn activation_index_rejects_unvalidated_action_text() {
    let mut invalid = action("invalid.action", ActionScope::GlobalUser, ShellKind::Bash);
    invalid.display_name = "hidden\u{202e}name".into();
    let error = ActionIndex::build(vec![ActionLayer {
        identity: LayerIdentity::GlobalUser,
        revision: 1,
        actions: vec![invalid],
    }])
    .unwrap_err();
    assert!(matches!(
        error,
        automexia_devops::actions::IndexError::InvalidAction { .. }
    ));
}

#[test]
fn optional_empty_values_are_quoted_and_required_empty_values_fail_closed() {
    let mut optional =
        action("optional.action", ActionScope::GlobalUser, ShellKind::Bash);
    optional.placeholders[0].required = false;
    let expanded =
        expand_for_shell(&optional, ShellKind::Bash, &PlaceholderBindings::default())
            .unwrap();
    assert_eq!(expanded.command, "printf ''");

    let required = action("required.action", ActionScope::GlobalUser, ShellKind::Bash);
    let mut bindings = PlaceholderBindings::default();
    bindings.insert("target", "");
    assert_eq!(
        expand_for_shell(&required, ShellKind::Bash, &bindings).unwrap_err(),
        ExpansionError::MissingPlaceholder("target".into())
    );
}

#[test]
fn cached_provider_hits_take_precedence_without_duplicate_rows() {
    let mut provider_action = action(
        "shared.provider-action",
        ActionScope::Session,
        ShellKind::Bash,
    );
    provider_action.description = "provider-bound".into();
    let mut persisted_action = provider_action.clone();
    persisted_action.description = "persisted".into();

    let merged = merge_action_search_hits(
        vec![ActionSearchHit {
            action: Arc::new(provider_action),
            provider: None,
            source: "Provider capsule",
            source_revision: 9,
            score: 200,
            shadowed_count: 1,
        }],
        vec![ActionSearchHit {
            action: Arc::new(persisted_action),
            provider: None,
            source: "Global user",
            source_revision: 4,
            score: 900,
            shadowed_count: 2,
        }],
    );

    assert_eq!(merged.len(), 1);
    assert_eq!(merged[0].action.description, "provider-bound");
    assert_eq!(merged[0].source, "Provider capsule");
    assert_eq!(merged[0].source_revision, 9);
    assert_eq!(merged[0].shadowed_count, 4);
}
