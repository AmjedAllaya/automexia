//! Dry-run-first CLI management for CP2.2 Quick Actions.

use std::{fs, io, path::Path};

use serde::Serialize;

use crate::cli::{ActionsAction, ActionsCommand};

use super::{
    apply_import, export_to_path, preview_import, secure_fs, QuickActionService,
    QuickActionStore, ServiceStatus,
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
