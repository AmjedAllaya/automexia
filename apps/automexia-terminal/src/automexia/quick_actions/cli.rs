//! Dry-run-first CLI management for CP2.2 Quick Actions.

use std::{collections::BTreeMap, fs, io, path::Path};

use serde::Serialize;

use crate::cli::{ActionsAction, ActionsCommand};

use super::{
    apply_import, apply_native_alias_import, export_to_path, preview_import,
    preview_native_alias_import_file, secure_fs, NativeAliasImportSelection,
    QuickActionService, QuickActionStore, ServiceStatus, WorkspaceActionStore,
    WorkspaceTaskBridgeInput, WorkspaceTrustStore,
};

#[derive(Serialize)]
struct ActionSummary<'a> {
    id: &'a str,
    display_name: &'a str,
    description: &'a str,
    scope: automexia_devops::actions::ActionScope,
    shells: &'a [automexia_devops::actions::ShellKind],
    risk: automexia_devops::actions::RiskClass,
    execution: automexia_devops::actions::ExecutionMode,
    enabled: bool,
}

#[derive(Serialize)]
struct OperationSummary<'a> {
    operation: &'a str,
    applied: bool,
    revision: u64,
    actions: usize,
    conflicts: &'a [String],
    source_digest: Option<&'a str>,
}

pub fn execute_actions_command(
    command: &ActionsCommand,
) -> Result<(), Box<dyn std::error::Error>> {
    let root = rio_backend::config::config_dir_path().join("actions");
    match &command.action {
        ActionsAction::List { json } => {
            let Some(service) = open_existing(&root)? else {
                return print_list(&[], 0, *json);
            };
            let snapshot = service.snapshot();
            print_list(
                &snapshot.actions().document().actions,
                snapshot.revision(),
                *json,
            )
        }
        ActionsAction::Show { id, json } => {
            let service = open_existing(&root)?.ok_or_else(|| not_found(id))?;
            let snapshot = service.snapshot();
            let action = snapshot
                .actions()
                .document()
                .actions
                .iter()
                .find(|action| action.id == *id)
                .ok_or_else(|| not_found(id))?;
            if *json {
                println!("{}", serde_json::to_string_pretty(action)?);
            } else {
                println!("{}", toml::to_string_pretty(action)?);
            }
            Ok(())
        }
        ActionsAction::Put {
            input,
            apply,
            expected_revision,
            replace,
        } => {
            let action = read_single_action(input)?;
            let existing = open_existing(&root)?;
            let current_revision = existing
                .as_ref()
                .map(|service| service.snapshot().revision())
                .unwrap_or(0);
            let already_exists = existing.as_ref().is_some_and(|service| {
                service
                    .snapshot()
                    .actions()
                    .document()
                    .actions
                    .iter()
                    .any(|candidate| candidate.id == action.id)
            });
            if *apply {
                let expected = expected_revision.ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "--expected-revision is required with --apply",
                    )
                })?;
                let service = match existing {
                    Some(service) => service,
                    None => open_for_write(&root)?,
                };
                let saved = if already_exists {
                    if !replace {
                        return Err(io::Error::new(
                            io::ErrorKind::AlreadyExists,
                            "action ID already exists; review and add --replace",
                        )
                        .into());
                    }
                    let action_id = action.id.clone();
                    service.update(expected, &action_id, action)?
                } else {
                    service.create(expected, action)?
                };
                println!("put revision={} (applied)", saved.revision());
            } else {
                println!(
                    "would {} id={} revision={} (dry-run; add --apply --expected-revision {}{})",
                    if already_exists { "update" } else { "create" },
                    action.id,
                    current_revision,
                    current_revision,
                    if already_exists { " --replace" } else { "" }
                );
            }
            Ok(())
        }
        ActionsAction::Import {
            input,
            apply,
            expected_revision,
            replace_conflicts,
            allow_machine_paths,
            json,
        } => {
            let existing = open_existing(&root)?;
            let current = match existing.as_ref() {
                Some(service) => service.snapshot(),
                None => std::sync::Arc::new(super::QuickActionSnapshot::empty()?),
            };
            let (_, preview) = preview_import(&current, input, *allow_machine_paths)?;
            let revision = if *apply {
                let expected = expected_revision.ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "--expected-revision is required with --apply",
                    )
                })?;
                let service = match existing {
                    Some(service) => service,
                    None => open_for_write(&root)?,
                };
                apply_import(
                    &service,
                    input,
                    expected,
                    *allow_machine_paths,
                    *replace_conflicts,
                )?
                .0
                .revision()
            } else {
                current.revision()
            };
            let summary = OperationSummary {
                operation: "import",
                applied: *apply,
                revision,
                actions: preview.imported_actions,
                conflicts: &preview.conflicts,
                source_digest: Some(&preview.source_digest),
            };
            print_summary(&summary, *json)
        }
        ActionsAction::ImportAliases {
            source,
            input,
            name,
            action_id,
            apply,
            expected_revision,
            replace_conflicts,
            json,
        } => {
            let selections = native_alias_selections(name, action_id)?;
            let existing = open_existing(&root)?;
            let current = match existing.as_ref() {
                Some(service) => service.snapshot(),
                None => std::sync::Arc::new(super::QuickActionSnapshot::empty()?),
            };
            let preview = preview_native_alias_import_file(
                &current,
                (*source).into(),
                input,
                &selections,
            )?;
            let revision = if *apply {
                let expected = required_revision(*expected_revision)?;
                let service = match existing {
                    Some(service) => service,
                    None => open_for_write(&root)?,
                };
                apply_native_alias_import(
                    &service,
                    (*source).into(),
                    input,
                    &selections,
                    expected,
                    *replace_conflicts,
                )?
                .snapshot
                .revision()
            } else {
                current.revision()
            };
            let selected_ids = preview
                .selected_actions
                .iter()
                .map(|action| action.id.as_str())
                .collect::<Vec<_>>();
            if *json {
                println!(
                    "{}",
                    serde_json::json!({
                        "operation": "import-aliases",
                        "applied": apply,
                        "revision": revision,
                        "source_digest": preview.source_digest,
                        "available_actions": preview.available_actions,
                        "rejected_actions": preview.rejected_actions,
                        "selected_action_ids": selected_ids,
                        "conflicts": preview.conflicts,
                    })
                );
            } else {
                println!(
                    "operation=import-aliases applied={} revision={} available={} rejected={} selected={} conflicts={} digest={}",
                    apply,
                    revision,
                    preview.available_actions,
                    preview.rejected_actions,
                    selected_ids.len(),
                    preview.conflicts.len(),
                    preview.source_digest
                );
                for id in selected_ids {
                    println!("selected={id}");
                }
            }
            Ok(())
        }
        ActionsAction::Export {
            output,
            overwrite,
            include_machine_paths,
            json,
        } => {
            let service = open_for_write(&root)?;
            let snapshot = service.snapshot();
            let preview =
                export_to_path(&snapshot, output, *include_machine_paths, *overwrite)?;
            let summary = OperationSummary {
                operation: "export",
                applied: true,
                revision: preview.source_revision,
                actions: preview.exported_actions,
                conflicts: &[],
                source_digest: Some(&preview.source_digest),
            };
            print_summary(&summary, *json)
        }
        ActionsAction::Remove {
            id,
            apply,
            expected_revision,
        } => {
            let service = open_existing(&root)?.ok_or_else(|| not_found(id))?;
            let current = service.snapshot();
            if !current
                .actions()
                .document()
                .actions
                .iter()
                .any(|action| action.id == *id)
            {
                return Err(not_found(id).into());
            }
            if *apply {
                let expected = expected_revision.ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "--expected-revision is required with --apply",
                    )
                })?;
                let saved = service.delete(expected, id)?;
                println!("removed id={id} revision={} (applied)", saved.revision());
            } else {
                println!(
                    "would remove id={id} revision={} (dry-run; add --apply --expected-revision {})",
                    current.revision(),
                    current.revision()
                );
            }
            Ok(())
        }
        ActionsAction::Recover {
            previous_revision,
            apply,
        } => {
            let service = open_existing(&root)?.ok_or_else(|| {
                io::Error::new(io::ErrorKind::NotFound, "Quick Action store is empty")
            })?;
            if *apply {
                let saved = service.recover_previous(*previous_revision)?;
                println!("recovered revision={} (applied)", saved.revision());
            } else {
                println!(
                    "would recover previous-revision={previous_revision} current-revision={} (dry-run; add --apply)",
                    service.snapshot().revision()
                );
            }
            Ok(())
        }
        action @ ActionsAction::TaskPut { .. }
        | action @ ActionsAction::TaskRemove { .. }
        | action @ ActionsAction::WorkspaceTrust { .. }
        | action @ ActionsAction::WorkspaceRevoke { .. }
        | action @ ActionsAction::WorkspaceDoctor { .. } => {
            execute_workspace_action(action, &root)
        }
        ActionsAction::Doctor { json } => {
            let Some(service) = open_existing(&root)? else {
                if *json {
                    println!(
                        "{}",
                        serde_json::json!({
                            "status": "uninitialized",
                            "revision": 0,
                            "actions": 0
                        })
                    );
                } else {
                    println!("status=uninitialized revision=0 actions=0");
                }
                return Ok(());
            };
            let snapshot = service.snapshot();
            let (status, error) = status_label(service.status());
            if *json {
                println!(
                    "{}",
                    serde_json::json!({
                        "status": status,
                        "error": error,
                        "revision": snapshot.revision(),
                        "actions": snapshot.actions().document().actions.len(),
                    })
                );
            } else if let Some(error) = error {
                println!(
                    "status={status} error={error} revision={} actions={}",
                    snapshot.revision(),
                    snapshot.actions().document().actions.len()
                );
            } else {
                println!(
                    "status={status} revision={} actions={}",
                    snapshot.revision(),
                    snapshot.actions().document().actions.len()
                );
            }
            Ok(())
        }
    }
}

fn execute_workspace_action(
    action: &ActionsAction,
    actions_root: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    match action {
        ActionsAction::TaskPut {
            workspace,
            runner,
            task,
            id,
            display_name,
            description,
            shell,
            risk,
            apply,
            expected_revision,
            replace,
            json,
        } => {
            let store = WorkspaceActionStore::open(workspace)?;
            let current = store.load()?;
            let expected = expected_revision.unwrap_or(current.document().revision);
            let input = WorkspaceTaskBridgeInput {
                action_id: id.clone(),
                display_name: display_name.clone(),
                description: description.clone(),
                runner: (*runner).into(),
                task_name: task.clone(),
                shells: shell.iter().copied().map(Into::into).collect(),
                risk: (*risk).into(),
            };
            let saved = if *apply {
                store.put_task_bridge(
                    input,
                    required_revision(*expected_revision)?,
                    *replace,
                )?
            } else {
                store.preview_task_bridge(input, expected, *replace)?
            };
            if *json {
                println!(
                    "{}",
                    serde_json::json!({
                        "operation": "task-put",
                        "applied": apply,
                        "workspace_identity": saved.workspace_identity(),
                        "source_digest": saved.source_digest(),
                        "revision": saved.document().revision,
                        "action_id": id,
                        "trust_status": "review-required",
                    })
                );
            } else {
                println!(
                    "operation=task-put applied={} revision={} id={} workspace={} digest={} trust=review-required",
                    apply,
                    saved.document().revision,
                    id,
                    saved.workspace_identity(),
                    saved.source_digest()
                );
            }
            Ok(())
        }
        ActionsAction::TaskRemove {
            workspace,
            id,
            apply,
            expected_revision,
            json,
        } => {
            let store = WorkspaceActionStore::open(workspace)?;
            let current = store.load()?;
            if !current.document().actions.iter().any(|item| item.id == *id) {
                return Err(not_found(id).into());
            }
            let revision = if *apply {
                store
                    .remove_task_bridge(id, required_revision(*expected_revision)?)?
                    .document()
                    .revision
            } else {
                current.document().revision
            };
            if *json {
                println!(
                    "{}",
                    serde_json::json!({
                        "operation": "task-remove",
                        "applied": apply,
                        "revision": revision,
                        "action_id": id,
                        "workspace_identity": store.workspace_identity(),
                        "trust_status": "revoked-by-source-change",
                    })
                );
            } else {
                println!(
                    "operation=task-remove applied={} revision={} id={} workspace={} trust=revoked-by-source-change",
                    apply,
                    revision,
                    id,
                    store.workspace_identity()
                );
            }
            Ok(())
        }
        ActionsAction::WorkspaceTrust {
            workspace,
            apply,
            expected_trust_revision,
            json,
        } => {
            let workspace = WorkspaceActionStore::open(workspace)?.load()?;
            let existing = open_existing_trust(actions_root)?;
            let current_revision = existing
                .as_ref()
                .map(WorkspaceTrustStore::load)
                .transpose()?
                .map_or(0, |snapshot| snapshot.revision());
            let revision = if *apply {
                open_trust_for_write(actions_root)?
                    .trust(&workspace, required_revision(*expected_trust_revision)?)?
                    .revision()
            } else {
                current_revision
            };
            if *json {
                println!(
                    "{}",
                    serde_json::json!({
                        "operation": "workspace-trust",
                        "applied": apply,
                        "trust_revision": revision,
                        "workspace_identity": workspace.workspace_identity(),
                        "source_revision": workspace.document().revision,
                        "source_digest": workspace.source_digest(),
                    })
                );
            } else {
                println!(
                    "operation=workspace-trust applied={} trust-revision={} workspace={} source-revision={} digest={}",
                    apply,
                    revision,
                    workspace.workspace_identity(),
                    workspace.document().revision,
                    workspace.source_digest()
                );
            }
            Ok(())
        }
        ActionsAction::WorkspaceRevoke {
            workspace,
            apply,
            expected_trust_revision,
            json,
        } => {
            let workspace = WorkspaceActionStore::open(workspace)?;
            let existing = open_existing_trust(actions_root)?.ok_or_else(|| {
                io::Error::new(io::ErrorKind::NotFound, "workspace trust store is empty")
            })?;
            let current = existing.load()?;
            if !current.receipts().iter().any(|receipt| {
                receipt.workspace_identity == workspace.workspace_identity()
            }) {
                return Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    "workspace has no active trust receipt",
                )
                .into());
            }
            let revision = if *apply {
                open_trust_for_write(actions_root)?
                    .revoke(
                        workspace.workspace_identity(),
                        required_revision(*expected_trust_revision)?,
                    )?
                    .revision()
            } else {
                current.revision()
            };
            if *json {
                println!(
                    "{}",
                    serde_json::json!({
                        "operation": "workspace-revoke",
                        "applied": apply,
                        "trust_revision": revision,
                        "workspace_identity": workspace.workspace_identity(),
                    })
                );
            } else {
                println!(
                    "operation=workspace-revoke applied={} trust-revision={} workspace={}",
                    apply,
                    revision,
                    workspace.workspace_identity()
                );
            }
            Ok(())
        }
        ActionsAction::WorkspaceDoctor { workspace, json } => {
            let workspace = WorkspaceActionStore::open(workspace)?.load()?;
            let existing = open_existing_trust(actions_root)?;
            let (trust_revision, trust_status) = match existing {
                Some(store) => {
                    let revision = store.load()?.revision();
                    let status = store
                        .trusted_layer(&workspace)
                        .map(|_| "trusted")
                        .unwrap_or_else(trust_error_label);
                    (revision, status)
                }
                None => (0, "not-trusted"),
            };
            if *json {
                println!(
                    "{}",
                    serde_json::json!({
                        "workspace_identity": workspace.workspace_identity(),
                        "source_revision": workspace.document().revision,
                        "source_digest": workspace.source_digest(),
                        "actions": workspace.document().actions.len(),
                        "trust_revision": trust_revision,
                        "trust_status": trust_status,
                    })
                );
            } else {
                println!(
                    "workspace={} source-revision={} actions={} digest={} trust-revision={} trust={}",
                    workspace.workspace_identity(),
                    workspace.document().revision,
                    workspace.document().actions.len(),
                    workspace.source_digest(),
                    trust_revision,
                    trust_status
                );
            }
            Ok(())
        }
        _ => unreachable!("workspace dispatcher received a non-workspace command"),
    }
}

fn native_alias_selections(
    names: &[String],
    mappings: &[String],
) -> Result<Vec<NativeAliasImportSelection>, Box<dyn std::error::Error>> {
    let mut overrides = BTreeMap::new();
    for mapping in mappings {
        let Some((name, action_id)) = mapping.split_once('=') else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "--action-id must be native-name=stable-action-id",
            )
            .into());
        };
        if name.is_empty()
            || action_id.is_empty()
            || overrides.insert(name, action_id).is_some()
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "--action-id mappings must be nonempty and unique",
            )
            .into());
        }
    }
    let mut selected = BTreeMap::new();
    for name in names {
        if selected
            .insert(name.as_str(), overrides.get(name.as_str()).copied())
            .is_some()
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "--name selections must be unique",
            )
            .into());
        }
    }
    if overrides.keys().any(|name| !selected.contains_key(name)) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "every --action-id mapping must refer to a selected --name",
        )
        .into());
    }
    Ok(selected
        .into_iter()
        .map(|(source_name, action_id)| NativeAliasImportSelection {
            source_name: source_name.to_owned(),
            action_id: action_id.map(str::to_owned),
        })
        .collect())
}

fn required_revision(revision: Option<u64>) -> Result<u64, io::Error> {
    revision.ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "the matching expected revision is required with --apply",
        )
    })
}

fn open_existing_trust(
    actions_root: &Path,
) -> Result<Option<WorkspaceTrustStore>, Box<dyn std::error::Error>> {
    Ok(WorkspaceTrustStore::open_existing_read_only(
        actions_root.join("workspace-trust"),
    )?)
}

fn open_trust_for_write(
    actions_root: &Path,
) -> Result<WorkspaceTrustStore, Box<dyn std::error::Error>> {
    // Reuse the existing guarded application-owned `actions/` root contract.
    let _actions = QuickActionStore::open_or_create(actions_root)?;
    Ok(WorkspaceTrustStore::open_or_create(
        actions_root.join("workspace-trust"),
    )?)
}

fn trust_error_label(
    error: automexia_devops::actions::WorkspaceTrustError,
) -> &'static str {
    use automexia_devops::actions::WorkspaceTrustError;
    match error {
        WorkspaceTrustError::NotTrusted => "not-trusted",
        WorkspaceTrustError::InvalidIdentity | WorkspaceTrustError::IdentityMismatch => {
            "identity-mismatch"
        }
        WorkspaceTrustError::RevisionMismatch => "source-revision-changed",
        WorkspaceTrustError::DigestMismatch => "source-digest-changed",
        WorkspaceTrustError::InvalidDocument => "invalid-source",
    }
}

fn open_existing(
    root: &Path,
) -> Result<Option<QuickActionService>, Box<dyn std::error::Error>> {
    match fs::symlink_metadata(root) {
        Ok(_) => Ok(Some(open_for_write(root)?)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

fn open_for_write(root: &Path) -> Result<QuickActionService, Box<dyn std::error::Error>> {
    let store = QuickActionStore::open_or_create(root)?;
    Ok(QuickActionService::open(store)?)
}

fn read_single_action(
    source: &Path,
) -> Result<automexia_devops::actions::QuickAction, Box<dyn std::error::Error>> {
    let bytes = secure_fs::read_bounded_regular(
        source,
        automexia_devops::actions::MAX_SOURCE_BYTES,
    )?
    .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "action file not found"))?;
    let text = std::str::from_utf8(&bytes).map_err(|_| {
        io::Error::new(io::ErrorKind::InvalidData, "action file is not UTF-8")
    })?;
    let document = automexia_devops::actions::parse_quick_actions(text)?.into_document();
    if document.actions.len() != 1 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "put requires a document containing exactly one action",
        )
        .into());
    }
    let action = document.actions.into_iter().next().expect("length checked");
    if !matches!(
        action.scope,
        automexia_devops::actions::ActionScope::GlobalUser
            | automexia_devops::actions::ActionScope::ShellUser
    ) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "put accepts only global-user or shell-user actions",
        )
        .into());
    }
    Ok(action)
}

fn print_list(
    actions: &[automexia_devops::actions::QuickAction],
    revision: u64,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let summaries = actions
        .iter()
        .map(|action| ActionSummary {
            id: &action.id,
            display_name: &action.display_name,
            description: &action.description,
            scope: action.scope,
            shells: &action.shells,
            risk: action.risk,
            execution: action.execution,
            enabled: action.enabled,
        })
        .collect::<Vec<_>>();
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "revision": revision,
                "actions": summaries,
            }))?
        );
    } else {
        println!("revision={revision} actions={}", summaries.len());
        for action in summaries {
            println!(
                "{}\t{}\t{:?}\t{:?}\t{}",
                action.id,
                action.display_name,
                action.scope,
                action.risk,
                if action.enabled {
                    "enabled"
                } else {
                    "disabled"
                }
            );
        }
    }
    Ok(())
}

fn print_summary(
    summary: &OperationSummary<'_>,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    if json {
        println!("{}", serde_json::to_string_pretty(summary)?);
    } else {
        println!(
            "operation={} applied={} revision={} actions={} conflicts={} digest={}",
            summary.operation,
            summary.applied,
            summary.revision,
            summary.actions,
            summary.conflicts.len(),
            summary.source_digest.unwrap_or("none")
        );
        for conflict in summary.conflicts {
            println!("conflict={conflict}");
        }
    }
    Ok(())
}

fn status_label(status: ServiceStatus) -> (&'static str, Option<&'static str>) {
    match status {
        ServiceStatus::Empty => ("empty", None),
        ServiceStatus::Fresh { .. } => ("ready", None),
        ServiceStatus::Recovered {
            rejected_primary, ..
        } => (
            "recovered",
            rejected_primary.map(super::StoreErrorCode::as_str),
        ),
        ServiceStatus::Stale { error, .. } => ("stale", Some(error.as_str())),
    }
}

fn not_found(id: &str) -> io::Error {
    io::Error::new(
        io::ErrorKind::NotFound,
        format!("Quick Action {id:?} was not found"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use automexia_devops::actions::{
        ActionProvenance, ActionScope, ActionTemplate, ExecutionMode, QuickAction,
        QuickActionDocument, RiskClass, ShellKind, WorkingDirectoryPolicy,
    };

    fn document(actions: usize) -> String {
        automexia_devops::actions::validate_quick_actions(QuickActionDocument {
            schema_version: 1,
            revision: 0,
            actions: (0..actions)
                .map(|index| QuickAction {
                    id: format!("test.action-{index}"),
                    display_name: format!("Action {index}"),
                    description: String::new(),
                    tags: Vec::new(),
                    scope: ActionScope::GlobalUser,
                    shells: vec![ShellKind::Bash],
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
                })
                .collect(),
        })
        .unwrap()
        .to_toml()
        .unwrap()
    }

    #[test]
    fn put_reader_accepts_exactly_one_bounded_validated_action() {
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("one.toml");
        fs::write(&source, document(1)).unwrap();
        assert_eq!(read_single_action(&source).unwrap().id, "test.action-0");

        fs::write(&source, document(2)).unwrap();
        assert!(read_single_action(&source).is_err());

        fs::write(&source, document(1).replace("global-user", "session")).unwrap();
        assert!(read_single_action(&source).is_err());
    }

    #[test]
    fn put_reader_rejects_oversized_input_before_parsing() {
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("oversized.toml");
        fs::write(
            &source,
            vec![b'x'; automexia_devops::actions::MAX_SOURCE_BYTES + 1],
        )
        .unwrap();
        assert!(read_single_action(&source).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn put_reader_rejects_symbolic_links() {
        use std::os::unix::fs::symlink;
        let root = tempfile::tempdir().unwrap();
        let target = root.path().join("target.toml");
        let linked = root.path().join("linked.toml");
        fs::write(&target, document(1)).unwrap();
        symlink(&target, &linked).unwrap();
        assert!(read_single_action(&linked).is_err());
    }
}
