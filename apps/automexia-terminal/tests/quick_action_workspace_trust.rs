use std::fs;

use automexia_devops::actions::{RiskClass, ShellKind, TaskRunner, WorkspaceTrustError};
use automexia_terminal::automexia::quick_actions::{
    WorkspaceActionStore, WorkspaceErrorCode, WorkspaceTaskBridgeInput,
    WorkspaceTrustStore,
};

fn task(id: &str, name: &str) -> WorkspaceTaskBridgeInput {
    WorkspaceTaskBridgeInput {
        action_id: id.into(),
        display_name: format!("Run {name}"),
        description: "Reviewed workspace task bridge".into(),
        runner: TaskRunner::Just,
        task_name: name.into(),
        shells: vec![ShellKind::Bash, ShellKind::Powershell],
        risk: RiskClass::Mutating,
    }
}

#[test]
fn workspace_task_source_is_dry_run_cas_trusted_and_revocable() {
    let root = tempfile::tempdir().unwrap();
    let workspace = root.path().join("project");
    fs::create_dir(&workspace).unwrap();
    let source = WorkspaceActionStore::open(&workspace).unwrap();
    let trust =
        WorkspaceTrustStore::open_or_create(root.path().join("private-trust")).unwrap();

    let empty = source.load().unwrap();
    assert_eq!(empty.document().revision, 0);
    assert!(!source.source_path().exists());
    let preview = source
        .preview_task_bridge(task("workspace.build", "build"), 0, false)
        .unwrap();
    assert_eq!(preview.document().revision, 1);
    assert!(!source.source_path().exists());

    let saved = source
        .put_task_bridge(task("workspace.build", "build"), 0, false)
        .unwrap();
    assert_eq!(saved.document().revision, 1);
    assert_eq!(
        source
            .put_task_bridge(task("workspace.test", "test"), 0, false)
            .unwrap_err()
            .code(),
        WorkspaceErrorCode::StaleRevision
    );
    assert_eq!(
        trust.trusted_layer(&saved).unwrap_err(),
        WorkspaceTrustError::NotTrusted
    );

    let trusted = trust.trust(&saved, 0).unwrap();
    assert_eq!(trusted.revision(), 1);
    let layer = trust.trusted_layer(&saved).unwrap();
    assert_eq!(layer.actions.len(), 1);

    let changed = source
        .put_task_bridge(task("workspace.test", "test"), 1, false)
        .unwrap();
    assert_eq!(
        trust.trusted_layer(&changed).unwrap_err(),
        WorkspaceTrustError::RevisionMismatch
    );
    trust.trust(&changed, 1).unwrap();
    assert_eq!(trust.trusted_layer(&changed).unwrap().actions.len(), 2);

    let revoked = trust.revoke(source.workspace_identity(), 2).unwrap();
    assert_eq!(revoked.revision(), 3);
    assert_eq!(
        trust.trusted_layer(&changed).unwrap_err(),
        WorkspaceTrustError::NotTrusted
    );
}

#[test]
fn conflicts_rename_removal_and_private_receipts_fail_closed() {
    let root = tempfile::tempdir().unwrap();
    let workspace = root.path().join("project");
    fs::create_dir(&workspace).unwrap();
    let native_task_file = workspace.join("justfile");
    fs::write(&native_task_file, "build:\n  cargo build\n").unwrap();
    let source = WorkspaceActionStore::open(&workspace).unwrap();
    let trust_root = root.path().join("private-trust");
    let trust = WorkspaceTrustStore::open_or_create(&trust_root).unwrap();

    source
        .put_task_bridge(task("workspace.build", "build"), 0, false)
        .unwrap();
    assert_eq!(
        source
            .put_task_bridge(task("workspace.build", "renamed"), 1, false)
            .unwrap_err()
            .code(),
        WorkspaceErrorCode::Conflict
    );
    let renamed = source
        .put_task_bridge(task("workspace.build", "renamed"), 1, true)
        .unwrap();
    assert_eq!(renamed.document().actions.len(), 1);
    trust.trust(&renamed, 0).unwrap();

    let receipt_text = fs::read_to_string(trust.source_path()).unwrap();
    assert!(!receipt_text.contains(workspace.to_string_lossy().as_ref()));
    assert!(!receipt_text.contains("renamed"));

    let removed = source.remove_task_bridge("workspace.build", 2).unwrap();
    assert!(removed.document().actions.is_empty());
    assert_eq!(
        fs::read_to_string(native_task_file).unwrap(),
        "build:\n  cargo build\n"
    );
    assert_eq!(
        trust.trusted_layer(&removed).unwrap_err(),
        WorkspaceTrustError::RevisionMismatch
    );
}

#[cfg(unix)]
#[test]
fn linked_workspace_sources_and_trust_files_are_rejected() {
    use std::os::unix::fs::symlink;

    let root = tempfile::tempdir().unwrap();
    let workspace = root.path().join("project");
    fs::create_dir(&workspace).unwrap();
    let source = WorkspaceActionStore::open(&workspace).unwrap();
    fs::create_dir(source.source_path().parent().unwrap()).unwrap();
    let outside = root.path().join("outside.toml");
    fs::write(&outside, "schema_version = 1\nrevision = 0\nactions = []\n").unwrap();
    symlink(&outside, source.source_path()).unwrap();
    assert_eq!(
        source.load().unwrap_err().code(),
        WorkspaceErrorCode::LinkRejected
    );

    let trust_root = root.path().join("trust");
    let trust = WorkspaceTrustStore::open_or_create(&trust_root).unwrap();
    let receipt = trust.source_path().to_path_buf();
    symlink(&outside, &receipt).unwrap();
    assert_eq!(
        trust.load().unwrap_err().code(),
        WorkspaceErrorCode::LinkRejected
    );
}
