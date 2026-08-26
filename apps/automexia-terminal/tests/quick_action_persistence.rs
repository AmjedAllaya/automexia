use automexia_command_productivity::actions::{
    ActionProvenance, ActionScope, ActionTemplate, ExecutionMode, QuickAction, RiskClass,
    ShellKind, WorkingDirectoryPolicy,
};
use automexia_terminal::automexia::quick_actions::{
    QuickActionService, QuickActionStore, StoreErrorCode, ACTIONS_FILE_NAME,
    LOCK_FILE_NAME, PREVIOUS_ACTIONS_FILE_NAME,
};
use proptest::prelude::*;

fn action(id: &str) -> QuickAction {
    QuickAction {
        id: id.into(),
        display_name: format!("Action {id}"),
        description: "Unicode safe: résumé 東京".into(),
        tags: vec!["test".into()],
        scope: ActionScope::GlobalUser,
        shells: vec![ShellKind::Powershell, ShellKind::Bash],
        template: ActionTemplate::TypedArgv {
            executable_id: "git".into(),
            arguments: Vec::new(),
        },
        placeholders: Vec::new(),
        working_directory_policy: WorkingDirectoryPolicy::Inherit,
        risk: RiskClass::ReadOnly,
        execution: ExecutionMode::Insert,
        provenance: ActionProvenance::User,
        enabled: true,
        alias_projection: None,
    }
}

#[test]
fn unicode_and_spaced_roots_survive_restart_without_extra_state() {
    let root = tempfile::tempdir().unwrap();
    let actions_root = root.path().join("Automexia résumé 東京").join("actions");
    let store = QuickActionStore::open_or_create(&actions_root).unwrap();
    let service = QuickActionService::open(store).unwrap();
    service.create(0, action("unicode-action")).unwrap();
    drop(service);

    let reopened = QuickActionService::open(
        QuickActionStore::open_or_create(&actions_root).unwrap(),
    )
    .unwrap();
    assert_eq!(reopened.snapshot().revision(), 1);
    assert_eq!(
        reopened.snapshot().actions().document().actions[0].description,
        "Unicode safe: résumé 東京"
    );
    let names = std::fs::read_dir(&actions_root)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        names,
        [ACTIONS_FILE_NAME, LOCK_FILE_NAME]
            .into_iter()
            .map(str::to_owned)
            .collect()
    );
}

#[test]
fn one_thousand_loads_release_handles_and_keep_storage_bounded() {
    let root = tempfile::tempdir().unwrap();
    let store = QuickActionStore::open_or_create(root.path().join("actions")).unwrap();
    store.create(0, action("load-cycle")).unwrap();
    for _ in 0..1_000 {
        let loaded = store.load().unwrap();
        assert_eq!(loaded.snapshot.revision(), 1);
    }

    let source = store.source_path();
    let moved = store.root().join("actions.moved.toml");
    std::fs::rename(&source, &moved).unwrap();
    std::fs::rename(&moved, &source).unwrap();
    let total_bytes = std::fs::read_dir(store.root())
        .unwrap()
        .map(|entry| entry.unwrap().metadata().unwrap().len())
        .sum::<u64>();
    assert!(total_bytes < 2 * 1_048_576);
}

#[test]
fn public_errors_and_snapshots_never_echo_action_content_or_paths() {
    let root = tempfile::tempdir().unwrap();
    let store = QuickActionStore::open_or_create(root.path().join("actions")).unwrap();
    std::fs::write(
        store.source_path(),
        b"private_token = 'TOP-SECRET-AUTOMEXIA'",
    )
    .unwrap();
    let error = store.load().unwrap_err();
    assert_eq!(error.code(), StoreErrorCode::ModelRejected);
    let rendered = format!("{error:?} {error}");
    assert!(!rendered.contains("TOP-SECRET-AUTOMEXIA"));
    assert!(!rendered.contains(root.path().to_string_lossy().as_ref()));
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(32))]

    #[test]
    fn revisioned_create_delete_sequences_round_trip(
        decisions in prop::collection::vec(any::<bool>(), 1..24)
    ) {
        let root = tempfile::tempdir().unwrap();
        let store = QuickActionStore::open_or_create(root.path().join("actions"))
            .unwrap();
        let mut revision = 0u64;
        let mut live = Vec::<String>::new();
        for (index, create) in decisions.into_iter().enumerate() {
            if create || live.is_empty() {
                let id = format!("action-{index}");
                store.create(revision, action(&id)).unwrap();
                live.push(id);
            } else {
                let id = live.remove(0);
                store.delete(revision, &id).unwrap();
            }
            revision += 1;
            let loaded = store.load().unwrap().snapshot;
            prop_assert_eq!(loaded.revision(), revision);
            let actual = loaded
                .actions()
                .document()
                .actions
                .iter()
                .map(|action| action.id.clone())
                .collect::<Vec<_>>();
            prop_assert_eq!(actual, live.clone());
        }

        let names = std::fs::read_dir(store.root())
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect::<std::collections::BTreeSet<_>>();
        prop_assert!(names.contains(ACTIONS_FILE_NAME));
        prop_assert!(names.contains(PREVIOUS_ACTIONS_FILE_NAME) || revision == 1);
        prop_assert!(names.contains(LOCK_FILE_NAME));
        prop_assert_eq!(names.len(), if revision == 1 { 2 } else { 3 });
    }
}
