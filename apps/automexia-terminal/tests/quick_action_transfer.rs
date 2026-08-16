use std::fs;

use automexia_devops::actions::{
    ActionProvenance, ActionScope, ActionTemplate, ExecutionMode, QuickAction,
    QuickActionDocument, RiskClass, ShellKind, WorkingDirectoryPolicy,
    QUICK_ACTION_SCHEMA_VERSION,
};
use automexia_terminal::automexia::quick_actions::{
    apply_import, export_to_path, preview_import, QuickActionService, QuickActionStore,
    TransferError,
};

fn action(id: &str, working_directory_policy: WorkingDirectoryPolicy) -> QuickAction {
    QuickAction {
        id: id.into(),
        display_name: format!("Action {id}"),
        description: "Portable test action".into(),
        tags: vec!["test".into()],
        scope: ActionScope::GlobalUser,
        shells: vec![ShellKind::Bash, ShellKind::Powershell],
        template: ActionTemplate::TypedArgv {
            executable_id: "git".into(),
            arguments: vec![],
        },
        placeholders: vec![],
        working_directory_policy,
        risk: RiskClass::ReadOnly,
        execution: ExecutionMode::Insert,
        provenance: ActionProvenance::User,
        enabled: true,
        alias_projection: None,
    }
}

fn service(root: &std::path::Path) -> QuickActionService {
    let store = QuickActionStore::open_or_create(root.join("actions")).unwrap();
    QuickActionService::open(store).unwrap()
}

#[test]
fn portable_export_import_round_trips_with_digest_and_cas() {
    let source_root = tempfile::tempdir().unwrap();
    let source = service(source_root.path());
    source
        .replace(
            0,
            QuickActionDocument {
                schema_version: QUICK_ACTION_SCHEMA_VERSION,
                revision: 1,
                actions: vec![action("portable.action", WorkingDirectoryPolicy::Inherit)],
            },
        )
        .unwrap();
    let transfer = source_root.path().join("portable-actions.toml");
    let exported = export_to_path(&source.snapshot(), &transfer, false, false).unwrap();
    assert_eq!(exported.exported_actions, 1);
    assert_eq!(exported.source_digest.len(), 64);

    let target_root = tempfile::tempdir().unwrap();
    let target = service(target_root.path());
    let (_, preview) = preview_import(&target.snapshot(), &transfer, false).unwrap();
    assert!(preview.conflicts.is_empty());
    let (saved, applied) = apply_import(&target, &transfer, 0, false, false).unwrap();
    assert_eq!(saved.revision(), 1);
    assert_eq!(applied.imported_actions, 1);
    let imported = &saved.actions().document().actions[0];
    assert!(matches!(
        imported.provenance,
        ActionProvenance::Imported { .. }
    ));
}

#[test]
fn import_is_dry_run_until_apply_and_conflicts_fail_closed() {
    let root = tempfile::tempdir().unwrap();
    let source = service(root.path());
    source
        .create(0, action("same.action", WorkingDirectoryPolicy::Inherit))
        .unwrap();
    let transfer = root.path().join("export.toml");
    export_to_path(&source.snapshot(), &transfer, false, false).unwrap();

    let target_root = tempfile::tempdir().unwrap();
    let target = service(target_root.path());
    target
        .create(0, action("same.action", WorkingDirectoryPolicy::Inherit))
        .unwrap();
    let (_, preview) = preview_import(&target.snapshot(), &transfer, false).unwrap();
    assert_eq!(preview.conflicts, vec!["same.action"]);
    assert_eq!(target.snapshot().revision(), 1);
    assert_eq!(
        apply_import(&target, &transfer, 1, false, false).unwrap_err(),
        TransferError::Conflict
    );
    assert_eq!(target.snapshot().revision(), 1);
    let (saved, _) = apply_import(&target, &transfer, 1, false, true).unwrap();
    assert_eq!(saved.revision(), 2);
    assert_eq!(saved.actions().document().actions.len(), 1);
}

#[test]
fn digest_tampering_and_machine_paths_are_rejected() {
    let root = tempfile::tempdir().unwrap();
    let source_service = service(root.path());
    source_service
        .create(
            0,
            action(
                "path.action",
                WorkingDirectoryPolicy::Fixed {
                    path: "/private/machine".into(),
                },
            ),
        )
        .unwrap();
    let transfer = root.path().join("machine.toml");
    let preview =
        export_to_path(&source_service.snapshot(), &transfer, false, false).unwrap();
    assert_eq!(preview.exported_actions, 0);
    assert_eq!(preview.excluded_machine_paths, 1);

    export_to_path(&source_service.snapshot(), &transfer, true, true).unwrap();
    let target_root = tempfile::tempdir().unwrap();
    let target = service(target_root.path());
    assert_eq!(
        preview_import(&target.snapshot(), &transfer, false).unwrap_err(),
        TransferError::MachinePathDenied
    );

    let text = fs::read_to_string(&transfer)
        .unwrap()
        .replace("Action path.action", "Tampered action");
    fs::write(&transfer, text).unwrap();
    assert_eq!(
        preview_import(&target.snapshot(), &transfer, true).unwrap_err(),
        TransferError::DigestMismatch
    );
}

#[test]
fn persistent_store_rejects_session_workspace_capsule_and_builtin_scopes() {
    for scope in [
        ActionScope::Session,
        ActionScope::Capsule,
        ActionScope::TrustedWorkspace,
        ActionScope::BuiltinDisabled,
    ] {
        let root = tempfile::tempdir().unwrap();
        let service = service(root.path());
        let mut candidate = action("wrong.scope", WorkingDirectoryPolicy::Inherit);
        candidate.scope = scope;
        let error = service.create(0, candidate).unwrap_err();
        assert_eq!(error.code().as_str(), "unsupported-persistent-scope");
    }
}

#[cfg(unix)]
#[test]
fn linked_import_file_is_rejected_without_following_it() {
    use std::os::unix::fs::symlink;
    let root = tempfile::tempdir().unwrap();
    let target = root.path().join("target.toml");
    fs::write(&target, "not a transfer").unwrap();
    let link = root.path().join("linked.toml");
    symlink(&target, &link).unwrap();
    let service_root = tempfile::tempdir().unwrap();
    let service = service(service_root.path());
    assert_eq!(
        preview_import(&service.snapshot(), &link, false).unwrap_err(),
        TransferError::LinkedOrSpecialFile
    );
}
