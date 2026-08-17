use std::fs;

use automexia_devops::actions::NativeAliasSource;
use automexia_terminal::automexia::quick_actions::{
    apply_native_alias_import, export_to_path, preview_native_alias_import_file,
    NativeAliasImportSelection, NativeImportError, QuickActionService, QuickActionStore,
};

fn service(root: &std::path::Path) -> QuickActionService {
    let store = QuickActionStore::open_or_create(root.join("actions")).unwrap();
    QuickActionService::open(store).unwrap()
}

#[test]
fn selected_native_alias_import_is_dry_run_cas_conflict_and_rename_safe() {
    let root = tempfile::tempdir().unwrap();
    let inventory = root.path().join("aliases.txt");
    let inventory_source = concat!(
        "alias gst='git status --short'\n",
        "alias gco='git checkout'\n",
        "alias unsafe='echo $(id)'\n"
    );
    fs::write(&inventory, inventory_source).unwrap();
    let target = service(root.path());
    let selections = vec![NativeAliasImportSelection {
        source_name: "gst".into(),
        action_id: Some("team.git-status".into()),
    }];

    let preview = preview_native_alias_import_file(
        &target.snapshot(),
        NativeAliasSource::Bash,
        &inventory,
        &selections,
    )
    .unwrap();
    assert_eq!(preview.selected_actions.len(), 1);
    assert_eq!(preview.selected_actions[0].id, "team.git-status");
    assert_eq!(preview.available_actions, 2);
    assert_eq!(preview.rejected_actions, 1);
    assert!(preview.conflicts.is_empty());
    assert_eq!(target.snapshot().revision(), 0);
    assert!(!root.path().join("actions").join("actions.toml").exists());

    let saved = apply_native_alias_import(
        &target,
        NativeAliasSource::Bash,
        &inventory,
        &selections,
        0,
        false,
    )
    .unwrap();
    assert_eq!(saved.snapshot.revision(), 1);
    assert_eq!(saved.preview.selected_actions[0].id, "team.git-status");
    assert_eq!(fs::read_to_string(&inventory).unwrap(), inventory_source);

    assert_eq!(
        apply_native_alias_import(
            &target,
            NativeAliasSource::Bash,
            &inventory,
            &selections,
            0,
            false,
        )
        .unwrap_err(),
        NativeImportError::StaleRevision
    );
    assert_eq!(
        apply_native_alias_import(
            &target,
            NativeAliasSource::Bash,
            &inventory,
            &selections,
            1,
            false,
        )
        .unwrap_err(),
        NativeImportError::Conflict
    );
    let replaced = apply_native_alias_import(
        &target,
        NativeAliasSource::Bash,
        &inventory,
        &selections,
        1,
        true,
    )
    .unwrap();
    assert_eq!(replaced.snapshot.revision(), 2);
    assert_eq!(replaced.snapshot.actions().document().actions.len(), 1);
}

#[test]
fn native_import_requires_explicit_supported_unique_names_and_portably_exports() {
    let root = tempfile::tempdir().unwrap();
    let inventory = root.path().join("aliases.txt");
    fs::write(
        &inventory,
        "alias gst='git status'\nalias unsafe='echo $TOKEN'\n",
    )
    .unwrap();
    let target = service(root.path());

    assert_eq!(
        preview_native_alias_import_file(
            &target.snapshot(),
            NativeAliasSource::Bash,
            &inventory,
            &[],
        )
        .unwrap_err(),
        NativeImportError::SelectionRequired
    );
    assert_eq!(
        preview_native_alias_import_file(
            &target.snapshot(),
            NativeAliasSource::Bash,
            &inventory,
            &[
                NativeAliasImportSelection {
                    source_name: "gst".into(),
                    action_id: None,
                },
                NativeAliasImportSelection {
                    source_name: "gst".into(),
                    action_id: None,
                },
            ],
        )
        .unwrap_err(),
        NativeImportError::DuplicateSelection
    );
    assert_eq!(
        preview_native_alias_import_file(
            &target.snapshot(),
            NativeAliasSource::Bash,
            &inventory,
            &[NativeAliasImportSelection {
                source_name: "unsafe".into(),
                action_id: None,
            }],
        )
        .unwrap_err(),
        NativeImportError::SelectedAliasRejected
    );

    let selections = [NativeAliasImportSelection {
        source_name: "gst".into(),
        action_id: None,
    }];
    apply_native_alias_import(
        &target,
        NativeAliasSource::Bash,
        &inventory,
        &selections,
        0,
        false,
    )
    .unwrap();
    let exported_path = root.path().join("portable.toml");
    let exported =
        export_to_path(&target.snapshot(), &exported_path, false, false).unwrap();
    assert_eq!(exported.exported_actions, 1);
    assert!(!fs::read_to_string(exported_path)
        .unwrap()
        .contains(root.path().to_string_lossy().as_ref()));

    let id = target.snapshot().actions().document().actions[0].id.clone();
    target.delete(1, &id).unwrap();
    assert!(target.snapshot().actions().document().actions.is_empty());
    assert!(inventory.exists());
}

#[cfg(unix)]
#[test]
fn linked_native_inventory_is_rejected_without_following_it() {
    use std::os::unix::fs::symlink;
    let root = tempfile::tempdir().unwrap();
    let target_file = root.path().join("real.txt");
    fs::write(&target_file, "alias gst='git status'\n").unwrap();
    let linked = root.path().join("linked.txt");
    symlink(&target_file, &linked).unwrap();
    let target = service(root.path());
    let error = preview_native_alias_import_file(
        &target.snapshot(),
        NativeAliasSource::Bash,
        &linked,
        &[NativeAliasImportSelection {
            source_name: "gst".into(),
            action_id: None,
        }],
    )
    .unwrap_err();
    assert_eq!(error, NativeImportError::LinkedOrSpecialFile);
}
