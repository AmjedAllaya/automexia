use automexia_connectivity::connections::*;
use automexia_terminal::automexia::connections::*;
use automexia_ui_model::connection_hub::HubKey;
use std::time::Duration;

fn settle(runtime: &ConnectionHubRuntime) {
    assert!(runtime.wait_for_settled(Duration::from_secs(5)));
}
fn enter(editor: &mut ProfileEditor, focus: ProfileFocus, value: &str) {
    editor.apply(ProfileAction::Focus(focus));
    editor.apply(ProfileAction::SelectAll);
    editor.apply(ProfileAction::Append(value.into()));
}
fn new_draft(editor: &mut ProfileEditor) {
    editor.apply(ProfileAction::Add);
    enter(editor, ProfileFocus::Name, "Team shell");
    enter(editor, ProfileFocus::Host, "shell.example.test");
    enter(editor, ProfileFocus::User, "operator");
    enter(editor, ProfileFocus::Port, "2222");
}
fn save(
    runtime: &ConnectionHubRuntime,
    editor: &mut ProfileEditor,
    action: ProfileAction,
) {
    let ProfileEffect::Save { revision, edit } = editor.apply(action) else {
        panic!("missing reviewed save intent: {:?}", editor.notice);
    };
    editor.pending = Some(
        runtime
            .apply_library_edit(revision, edit, Box::new(|| {}))
            .unwrap(),
    );
    settle(runtime);
    let snapshot = runtime.snapshot();
    editor.sync(&snapshot.library, &snapshot.library_change);
    assert!(editor.pending.is_none());
    assert!(matches!(
        snapshot.library_change,
        HubMetadataChangeState::Applied { .. }
    ));
}

#[test]
fn saved_connection_create_edit_reopen_and_confirmed_remove_use_one_library() {
    let temporary = tempfile::tempdir().unwrap();
    let runtime = ConnectionHubRuntime::open_at_root(temporary.path());
    settle(&runtime);
    let mut editor = ProfileEditor::new(&runtime.snapshot().library);
    new_draft(&mut editor);
    save(&runtime, &mut editor, ProfileAction::Save);
    assert_eq!(runtime.catalog().len(), 1);
    let id = runtime.catalog()[0].summary.id.clone();
    let prepared = runtime.prepare_direct_openssh(&id).unwrap();
    assert_eq!(
        prepared.profile().public_target,
        "operator@shell.example.test:2222"
    );
    editor.apply(ProfileAction::Edit);
    enter(&mut editor, ProfileFocus::Name, "Renamed shell");
    save(&runtime, &mut editor, ProfileAction::Save);
    assert_eq!(runtime.catalog()[0].summary.id, id);
    assert_eq!(runtime.catalog()[0].summary.display_name, "Renamed shell");
    drop(runtime);
    let runtime = ConnectionHubRuntime::open_at_root(temporary.path());
    settle(&runtime);
    let mut editor = ProfileEditor::new(&runtime.snapshot().library);
    assert_eq!(editor.library.profiles.profiles.len(), 1);
    editor.apply(ProfileAction::Remove);
    assert_eq!(editor.focus, ProfileFocus::Cancel);
    editor.apply(ProfileAction::Key(HubKey::Enter));
    assert!(!editor.confirming_remove);
    editor.apply(ProfileAction::Remove);
    editor.apply(ProfileAction::Key(HubKey::Tab));
    save(&runtime, &mut editor, ProfileAction::Key(HubKey::Enter));
    assert!(runtime.catalog().is_empty());
}

#[test]
fn saved_connection_binding_stays_external_and_never_falls_back_to_another_identity() {
    let temporary = tempfile::tempdir().unwrap();
    let runtime = ConnectionHubRuntime::open_at_root(temporary.path());
    settle(&runtime);
    runtime
        .apply_library_edit(
            0,
            LibraryEdit::PutCredentialSource {
                expected_entity_revision: None,
                source: CredentialSourceV1 {
                    schema_version: 1,
                    id: "team-agent".into(),
                    revision: 1,
                    display_name: "Team vault".into(),
                    provider: CredentialProvider::OnePassword,
                    endpoint: SshAgentEndpoint::System,
                },
            },
            Box::new(|| {}),
        )
        .unwrap();
    settle(&runtime);
    let mut editor = ProfileEditor::new(&runtime.snapshot().library);
    new_draft(&mut editor);
    editor.apply(ProfileAction::CycleSource(true));
    assert_eq!(editor.source_label(), "Team vault");
    save(&runtime, &mut editor, ProfileAction::Save);
    assert_eq!(
        editor.library.profiles.profiles[0]
            .identity
            .reference
            .as_str(),
        "team-agent"
    );
    assert!(editor.library.profiles.profiles[0]
        .approval_fingerprint
        .is_none());
    assert!(runtime
        .prepare_direct_openssh(&runtime.catalog()[0].summary.id)
        .is_err());
    runtime
        .apply_library_edit(
            editor.library.revision,
            LibraryEdit::RemoveCredentialSource {
                expected_entity_revision: 1,
                source_id: "team-agent".into(),
            },
            Box::new(|| {}),
        )
        .unwrap();
    settle(&runtime);
    assert!(matches!(
        runtime.snapshot().library_change,
        HubMetadataChangeState::Error { .. }
    ));
    assert_eq!(
        runtime
            .snapshot()
            .library
            .document
            .credential_sources
            .sources
            .len(),
        1
    );
}

#[test]
fn saved_connection_editor_traps_focus_rejects_commands_and_preserves_cancelled_draft_data(
) {
    let mut editor = ProfileEditor::new(&HubLibrarySnapshot::default());
    new_draft(&mut editor);
    editor.apply(ProfileAction::Focus(ProfileFocus::Name));
    for _ in 0..14 {
        editor.apply(ProfileAction::Key(HubKey::Tab));
    }
    assert_eq!(editor.focus, ProfileFocus::Name);
    enter(&mut editor, ProfileFocus::Host, "server;touch marker");
    assert!(matches!(
        editor.apply(ProfileAction::Save),
        ProfileEffect::None
    ));
    assert!(editor.notice.is_some());
    enter(&mut editor, ProfileFocus::Host, "server\nwhoami");
    assert_eq!(editor.draft.as_ref().unwrap().host, "server;touch marker");
    editor.apply(ProfileAction::Key(HubKey::Escape));
    assert!(editor.draft.is_none());
    assert!(editor.library.profiles.profiles.is_empty());
    assert!(matches!(
        editor.apply(ProfileAction::Key(HubKey::Escape)),
        ProfileEffect::Close
    ));
}

#[test]
fn saved_connection_stale_draft_and_unrelated_completion_cannot_overwrite_current_library(
) {
    let temporary = tempfile::tempdir().unwrap();
    let runtime = ConnectionHubRuntime::open_at_root(temporary.path());
    settle(&runtime);
    let mut stale = ProfileEditor::new(&runtime.snapshot().library);
    new_draft(&mut stale);
    let mut first = ProfileEditor::new(&runtime.snapshot().library);
    new_draft(&mut first);
    save(&runtime, &mut first, ProfileAction::Save);
    stale.sync(
        &runtime.snapshot().library,
        &runtime.snapshot().library_change,
    );
    let ProfileEffect::Save { revision, edit } = stale.apply(ProfileAction::Save) else {
        panic!("missing save");
    };
    assert_eq!(
        runtime.apply_library_edit(revision, edit, Box::new(|| {})),
        Err(HubRuntimeErrorCode::StaleReview)
    );
    stale.pending = Some(999);
    stale.sync(
        &runtime.snapshot().library,
        &runtime.snapshot().library_change,
    );
    assert_eq!(stale.pending, Some(999));
    assert!(stale.draft.is_some());
    assert!(matches!(
        stale.apply(ProfileAction::Key(HubKey::Escape)),
        ProfileEffect::Close
    ));
    assert_eq!(stale.pending, Some(999));
    stale.sync(
        &runtime.snapshot().library,
        &HubMetadataChangeState::Applied {
            request: 1000,
            revision: 2,
        },
    );
    assert!(stale.pending.is_none());
    assert!(stale.draft.is_some());
    assert!(stale.notice.unwrap().contains("verify"));
    assert_eq!(runtime.catalog().len(), 1);
}

#[test]
fn saved_connections_overlay_owns_text_input_and_blocks_outer_sections() {
    let temporary = tempfile::tempdir().unwrap();
    let runtime = ConnectionHubRuntime::open_at_root(temporary.path());
    settle(&runtime);
    let mut controller = ConnectionHubController::new(runtime);
    controller.open("fixture");
    assert!(controller.open_profiles());
    controller.profile_action(ProfileAction::Add, Box::new(|| {}));
    assert!(!controller.open_connections());
    assert!(!controller.open_credentials());
    assert!(!controller.can_begin_literal_destination_entry());
    assert!(controller.text_input_active());
    assert!(controller.commit_ime("漢字"));
    assert_eq!(
        controller
            .profile_editor()
            .unwrap()
            .draft
            .as_ref()
            .unwrap()
            .name,
        "漢字"
    );
    controller.profile_action(ProfileAction::Key(HubKey::Escape), Box::new(|| {}));
    assert!(controller.profile_editor().is_some());
    controller.profile_action(ProfileAction::Key(HubKey::Escape), Box::new(|| {}));
    assert!(controller.profile_editor().is_none());
    assert!(controller.open_credentials());
}
