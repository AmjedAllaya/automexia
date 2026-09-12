use automexia_command_productivity::actions::{
    expand_for_shell, merge_action_search_hits, ActionIndex, ActionLayer,
    ActionProvenance, ActionScope, ActionSearchHit, ActionTemplate, ArgumentToken,
    ExecutionMode, ExpansionError, LayerIdentity, Placeholder, PlaceholderBindings,
    PlaceholderSensitivity, QuickAction, RiskClass, SearchContext, ShellKind,
    WorkingDirectoryPolicy,
};
use std::sync::Arc;

mod search_allocations {
    use std::{
        alloc::{GlobalAlloc, Layout, System},
        cell::Cell,
    };
    pub struct Allocator;
    thread_local! {
        static ENABLED: Cell<bool> = const { Cell::new(false) };
        static LARGEST: Cell<usize> = const { Cell::new(0) };
    }
    fn record(bytes: usize) {
        if ENABLED.try_with(Cell::get).unwrap_or(false) {
            let _ = LARGEST.try_with(|size| size.set(size.get().max(bytes)));
        }
    }
    pub struct Window;
    impl Window {
        pub fn start() -> Self {
            LARGEST.set(0);
            ENABLED.set(true);
            Self
        }
    }
    impl Drop for Window {
        fn drop(&mut self) {
            ENABLED.set(false);
        }
    }
    pub fn largest() -> usize {
        LARGEST.get()
    }
    pub fn enabled() -> bool {
        ENABLED.get()
    }
    // SAFETY: Test-only accounting neither allocates nor changes System's
    // pointer/layout contract. Thread-local windows exclude parallel tests.
    unsafe impl GlobalAlloc for Allocator {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            record(layout.size());
            unsafe { System.alloc(layout) }
        }
        unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
            record(layout.size());
            unsafe { System.alloc_zeroed(layout) }
        }
        unsafe fn realloc(
            &self,
            pointer: *mut u8,
            layout: Layout,
            size: usize,
        ) -> *mut u8 {
            record(size);
            unsafe { System.realloc(pointer, layout, size) }
        }
        unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
            unsafe { System.dealloc(pointer, layout) }
        }
    }
}

#[global_allocator]
static SEARCH_ALLOCATOR: search_allocations::Allocator = search_allocations::Allocator;

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
        automexia_command_productivity::actions::IndexError::InvalidAction { .. }
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

fn score_fixture(label: &str) -> (ActionIndex, QuickAction) {
    let mut candidate =
        action("search.fixture", ActionScope::GlobalUser, ShellKind::Bash);
    candidate.display_name = label.into();
    candidate.description.clear();
    candidate.tags.clear();
    let index = ActionIndex::build(vec![ActionLayer {
        identity: LayerIdentity::GlobalUser,
        revision: 1,
        actions: vec![candidate.clone()],
    }])
    .unwrap();
    (index, candidate)
}

// Deliberately indexed scalar oracle, independent of a streaming scorer. Case
// folding remains string-level: Greek final sigma depends on its neighbours.
fn reference_search_score(query: &str, action: &QuickAction) -> Option<i32> {
    let query = query.trim().to_lowercase();
    if query.is_empty() {
        return Some(0);
    }
    [&action.display_name, &action.id, &action.description]
        .into_iter()
        .chain(action.tags.iter())
        .filter_map(|label| {
            let chars: Vec<_> = label.to_lowercase().chars().collect();
            let mut start = 0;
            let mut score = 0;
            for needle in query.chars() {
                let found = (start..chars.len()).find(|&index| chars[index] == needle)?;
                score += 100 - ((found - start).min(90) as i32);
                if found == 0
                    || chars[found - 1].is_whitespace()
                    || ['-', '.', '_', '/'].contains(&chars[found - 1])
                {
                    score += 30;
                }
                start = found + 1;
            }
            Some(score - chars.len().min(256) as i32)
        })
        .max()
}

#[test]
fn search_preserves_literal_scores_and_contextual_unicode_folding() {
    for (label, query, expected) in [
        ("ab", "ab", Some(228)),
        ("a b", "ab", Some(256)),
        ("a-b", "ab", Some(256)),
        ("cab", "ab", Some(196)),
        ("a🐈b", "ab", Some(226)),
        ("aaaa", "aa", Some(226)),
        ("ab", "ba", None),
        ("ΟΣ", "ς", Some(97)),
        ("ΟΣ", "σ", None),
        ("ab", "  ", Some(0)),
    ] {
        let (index, _) = score_fixture(label);
        let hits = index.search(query, &context(ShellKind::Bash)).unwrap();
        assert_eq!(hits.first().map(|hit| hit.score), expected);
    }
}

#[test]
fn maximum_length_search_avoids_character_tables() {
    use automexia_command_productivity::actions::MAX_STRING_BYTES;
    for label in [
        "a".repeat(MAX_STRING_BYTES),
        "猫".repeat(MAX_STRING_BYTES / 3),
    ] {
        let (index, _) = score_fixture(&label);
        let query: String = label.chars().take(2).collect();
        let context = context(ShellKind::Bash);
        let measured = search_allocations::Window::start();
        for _ in 0..32 {
            for query in [&query, &label] {
                let hits = index.search(std::hint::black_box(query), &context).unwrap();
                assert_eq!(hits.len(), 1);
                assert_eq!(
                    hits[0].score,
                    (query.chars().count() as i32) * 100 + 30 - 256
                );
                std::hint::black_box(hits);
            }
        }
        drop(measured);
        // Lowercase strings and small result structures fit this ceiling;
        // materializing 4,096 byte-offset/character pairs does not.
        assert!(
            search_allocations::largest() <= MAX_STRING_BYTES * 2,
            "largest search allocation: {}",
            search_allocations::largest()
        );
        println!(
            "maximum-input search: largest_allocation_bytes={}",
            search_allocations::largest()
        );
    }
}

#[test]
fn search_allocation_guard_detects_temporary_tables_and_resets() {
    let measured = search_allocations::Window::start();
    std::hint::black_box(vec![0_u8; std::hint::black_box(16_384)]);
    drop(measured);
    assert!(search_allocations::largest() >= 16_384);
    let failure = std::panic::catch_unwind(|| {
        let _measured = search_allocations::Window::start();
        panic!("fictional measurement failure");
    });
    assert!(failure.is_err());
    assert!(!search_allocations::enabled());
    let measured = search_allocations::Window::start();
    drop(measured);
    assert_eq!(search_allocations::largest(), 0);
}

#[test]
fn empty_search_preserves_deterministic_ties_and_result_ceiling() {
    let actions = (0..256)
        .rev()
        .map(|number| {
            let mut value = action(
                &format!("sort.{number:03}"),
                ActionScope::GlobalUser,
                ShellKind::Bash,
            );
            value.display_name = "Same label".into();
            value
        })
        .collect();
    let index = ActionIndex::build(vec![ActionLayer {
        identity: LayerIdentity::GlobalUser,
        revision: 1,
        actions,
    }])
    .unwrap();
    let hits = index.search("", &context(ShellKind::Bash)).unwrap();
    assert_eq!(hits.len(), 128);
    for (number, hit) in hits.iter().enumerate() {
        assert_eq!(hit.action.id, format!("sort.{number:03}"));
        assert_eq!(hit.score, 0);
    }
}

proptest::proptest! {
    #![proptest_config(proptest::test_runner::Config {
        cases: 128,
        rng_seed: proptest::test_runner::RngSeed::Fixed(0x0053_434f_5245),
        failure_persistence: Some(Box::new(proptest::test_runner::FileFailurePersistence::Direct(
            "tests/proptest-regressions/search_scores.txt",
        ))),
        ..proptest::test_runner::Config::default()
    })]
    #[test]
    fn public_search_matches_independent_scalar_oracle(
        label in "[abAB ./_éΣΟςσ猫́-]{0,64}",
        query in "[abAB ./_éΣΟςσ猫́-]{0,8}",
    ) {
        let (index, action) = score_fixture(&format!("Name {label}"));
        let expected = reference_search_score(&query, &action);
        let hits = index.search(&query, &context(ShellKind::Bash)).unwrap();
        proptest::prop_assert_eq!(hits.first().map(|hit| hit.score), expected);
        proptest::prop_assert_eq!(hits.len(), usize::from(expected.is_some()));
    }
}
