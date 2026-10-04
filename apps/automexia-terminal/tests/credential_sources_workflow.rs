use automexia_connectivity::connections::*;
use automexia_terminal::automexia::connections::*;
use automexia_ui_model::connection_hub::HubKey;
use std::time::Duration;

fn save(
    runtime: &ConnectionHubRuntime,
    editor: &mut CredentialEditor,
    action: CredentialAction,
) {
    let CredentialEffect::Save { revision, edit } = editor.apply(action) else {
        panic!("expected save intent");
    };
    let request = runtime
        .apply_library_edit(revision, edit, Box::new(|| {}))
        .unwrap();
    editor.pending = Some(request);
    assert!(runtime.wait_for_settled(Duration::from_secs(5)));
    let snapshot = runtime.snapshot();
    editor.sync(&snapshot.library, &snapshot.library_change);
    assert!(editor.pending.is_none());
    assert!(matches!(
        snapshot.library_change,
        HubMetadataChangeState::Applied { .. }
    ));
}

#[test]
fn keyboard_and_pointer_share_add_edit_remove_confirmation_and_persistence() {
    let temporary = tempfile::tempdir().unwrap();
    let runtime = ConnectionHubRuntime::open_at_root(temporary.path());
    assert!(runtime.wait_for_settled(Duration::from_secs(5)));
    let mut editor = CredentialEditor::new(&runtime.snapshot().library);
    editor.apply(CredentialAction::Add);
    editor.apply(CredentialAction::Focus(CredentialFocus::Provider));
    editor.apply(CredentialAction::Key(HubKey::Enter));
    assert_eq!(
        editor.draft.as_ref().unwrap().provider,
        CredentialProvider::OnePassword
    );
    save(&runtime, &mut editor, CredentialAction::Save);
    assert_eq!(editor.sources.len(), 1);
    editor.apply(CredentialAction::Key(HubKey::Enter));
    assert!(editor.draft.is_some());
    editor.apply(CredentialAction::Append(" example".into()));
    editor.apply(CredentialAction::Key(HubKey::Escape));
    assert_eq!(editor.sources[0].display_name, "My vault");
    editor.apply(CredentialAction::Remove);
    assert_eq!(editor.focus, CredentialFocus::Cancel);
    editor.apply(CredentialAction::Key(HubKey::Enter));
    assert!(!editor.confirming_remove);
    assert_eq!(editor.sources.len(), 1);
    editor.apply(CredentialAction::Remove);
    editor.apply(CredentialAction::Key(HubKey::Tab));
    save(&runtime, &mut editor, CredentialAction::Key(HubKey::Enter));
    assert!(editor.sources.is_empty());
    drop(runtime);
    let reopened = ConnectionHubRuntime::open_at_root(temporary.path());
    assert!(reopened.wait_for_settled(Duration::from_secs(5)));
    assert!(reopened
        .snapshot()
        .library
        .document
        .credential_sources
        .sources
        .is_empty());
}

#[test]
fn tab_stays_in_editor_and_text_does_not_trigger_actions() {
    let mut editor = CredentialEditor::new(&HubLibrarySnapshot::default());
    editor.apply(CredentialAction::Add);
    for _ in 0..(if cfg!(windows) { 20 } else { 25 }) {
        assert!(matches!(
            editor.apply(CredentialAction::Key(HubKey::Tab)),
            CredentialEffect::None
        ));
    }
    assert_eq!(editor.focus, CredentialFocus::Name);
    editor.apply(CredentialAction::Append("nvs".into()));
    assert_eq!(editor.draft.as_ref().unwrap().display_name, "My vaultnvs");
    editor.apply(CredentialAction::SelectAll);
    editor.apply(CredentialAction::Append("Team vault".into()));
    assert_eq!(editor.draft.as_ref().unwrap().display_name, "Team vault");
    for hostile in ["\u{061c}", "\u{202e}", "\n", "\u{2066}"] {
        editor.apply(CredentialAction::Append(hostile.into()));
        assert_eq!(editor.draft.as_ref().unwrap().display_name, "Team vault");
    }
    editor.apply(CredentialAction::Append("e\u{301}".into()));
    editor.apply(CredentialAction::Backspace);
    assert_eq!(editor.draft.as_ref().unwrap().display_name, "Team vault");
    let nodes = editor.accessibility_tree();
    assert!(nodes
        .iter()
        .any(|node| node.id == "credential-name" && node.name.contains("Team vault")));
}

#[test]
fn pointer_provider_keeps_keyboard_focus_and_cancel_clears_text_selection() {
    let mut editor = CredentialEditor::new(&HubLibrarySnapshot::default());
    editor.apply(CredentialAction::Add);
    editor.apply(CredentialAction::NextProvider);
    assert_eq!(editor.focus, CredentialFocus::Provider);
    editor.apply(CredentialAction::Focus(CredentialFocus::Name));
    editor.apply(CredentialAction::SelectAll);
    editor.apply(CredentialAction::Cancel);
    editor.apply(CredentialAction::Add);
    editor.apply(CredentialAction::Append(" 2".into()));
    assert_eq!(editor.draft.as_ref().unwrap().display_name, "My vault 2");
}

#[test]
fn source_removal_cannot_be_confirmed_by_typing_an_unrelated_shortcut() {
    let mut library = HubLibrarySnapshot::default();
    std::sync::Arc::make_mut(&mut library.document)
        .credential_sources
        .sources
        .push(CredentialSourceV1 {
            schema_version: 1,
            id: "agent".into(),
            revision: 1,
            display_name: "Agent".into(),
            provider: CredentialProvider::SystemSshAgent,
            endpoint: SshAgentEndpoint::System,
        });
    let mut editor = CredentialEditor::new(&library);
    editor.apply(CredentialAction::Remove);
    editor.apply(CredentialAction::Add);
    assert!(editor.confirming_remove);
    assert!(editor.draft.is_none());
    assert!(matches!(
        editor.apply(CredentialAction::Key(HubKey::Escape)),
        CredentialEffect::None
    ));
}

#[test]
fn concurrent_library_writers_publish_conflict_without_losing_saved_sources() {
    let temporary = tempfile::tempdir().unwrap();
    let first = ConnectionHubRuntime::open_at_root(temporary.path());
    assert!(first.wait_for_settled(Duration::from_secs(5)));
    let second = ConnectionHubRuntime::open_at_root(temporary.path());
    assert!(second.wait_for_settled(Duration::from_secs(5)));
    let mut editor = CredentialEditor::new(&first.snapshot().library);
    editor.apply(CredentialAction::Add);
    save(&first, &mut editor, CredentialAction::Save);
    let mut stale = CredentialEditor::new(&second.snapshot().library);
    stale.apply(CredentialAction::Add);
    let CredentialEffect::Save { revision, edit } = stale.apply(CredentialAction::Save)
    else {
        panic!("save missing")
    };
    let (sender, receiver) = std::sync::mpsc::channel();
    let publication = second.clone();
    second
        .apply_library_edit(
            revision,
            edit,
            Box::new(move || {
                let snapshot = publication.snapshot();
                let _ = sender.send((snapshot.library.revision, snapshot.library_change));
            }),
        )
        .unwrap();
    let (revision, state) = receiver.recv_timeout(Duration::from_secs(5)).unwrap();
    assert_eq!(revision, first.snapshot().library.revision);
    assert!(matches!(state, HubMetadataChangeState::Conflict { .. }));
    assert_eq!(
        second.snapshot().library.document.credential_sources,
        first.snapshot().library.document.credential_sources
    );
}

#[test]
fn vault_overlay_blocks_outer_file_shortcuts_and_owns_ime_composition() {
    let temporary = tempfile::tempdir().unwrap();
    let runtime = ConnectionHubRuntime::open_at_root(temporary.path());
    assert!(runtime.wait_for_settled(Duration::from_secs(5)));
    let mut controller = ConnectionHubController::new(runtime);
    controller.open("fixture");
    assert!(controller.open_credentials());
    assert!(
        !controller.can_begin_literal_destination_entry(),
        "outer file-picker mnemonics must be inert inside Vaults"
    );
    controller.credential_action(CredentialAction::Add, Box::new(|| {}));
    assert!(controller.set_ime_preedit(Some("漢")));
    assert!(controller.commit_ime("漢"));
    assert!(controller
        .credential_editor()
        .unwrap()
        .draft
        .as_ref()
        .unwrap()
        .display_name
        .ends_with('漢'));
    assert!(!controller.commit_ime(&"a".repeat(1025)));
    controller.credential_action(CredentialAction::Key(HubKey::Escape), Box::new(|| {}));
    assert!(controller.credential_editor().is_some());
    controller.credential_action(CredentialAction::Key(HubKey::Escape), Box::new(|| {}));
    assert!(controller.credential_editor().is_none());
    assert!(controller.can_begin_literal_destination_entry());
}

#[test]
fn source_save_ignores_old_completion_and_retains_draft_if_completion_was_superseded() {
    let library = HubLibrarySnapshot::default();
    let mut editor = CredentialEditor::new(&library);
    editor.apply(CredentialAction::Add);
    editor.pending = Some(3);
    editor.sync(
        &library,
        &HubMetadataChangeState::Applied {
            request: 2,
            revision: 1,
        },
    );
    assert_eq!(editor.pending, Some(3));
    assert!(editor.draft.is_some());
    editor.apply(CredentialAction::Key(HubKey::Tab));
    assert_eq!(editor.focus, CredentialFocus::Close);
    assert!(editor
        .accessibility_tree()
        .iter()
        .any(|node| node.id == "credential-back" && node.focusable && !node.disabled));
    assert!(matches!(
        editor.apply(CredentialAction::Key(HubKey::Escape)),
        CredentialEffect::Close
    ));
    assert_eq!(
        editor.pending,
        Some(3),
        "leaving the editor does not cancel an accepted disk write"
    );
    editor.sync(
        &library,
        &HubMetadataChangeState::Applied {
            request: 4,
            revision: 2,
        },
    );
    assert!(editor.pending.is_none());
    assert!(editor.draft.is_some());
    assert!(editor.notice.unwrap().contains("verify"));
}
