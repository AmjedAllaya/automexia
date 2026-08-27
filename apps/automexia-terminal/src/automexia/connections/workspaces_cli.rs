//! Preview-first CLI for the private declarative workspace library.

use std::{
    collections::BTreeMap,
    fmt,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use automexia_connectivity::connections::{
    from_json_slice_without_duplicate_keys, parse_workspace_json, RecipeRunMode,
    ResolvedExecutable, MAX_BROADCAST_COMMAND_BYTES, MAX_DOCUMENT_BYTES,
};
use serde::Deserialize;

use crate::{
    automexia::private_fs,
    cli::{WorkspacesAction, WorkspacesCommand},
};

use super::{
    m6_activation_readiness, preview_library_edit, review_library_broadcast,
    review_library_recipe, review_library_workspace_restore, ConnectionLibraryStore,
    LibraryEdit, LibraryLoadOrigin, M6ActivationBlocker, RecipeReviewRequest,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WorkspaceCliErrorCode {
    InputRejected,
    LibraryUnavailable,
    StaleRevision,
    ReviewRejected,
    MigrationNotRequired,
    RecoveryRequired,
    RecoveryNotRequired,
    ClockUnavailable,
}

#[derive(Debug)]
struct WorkspaceCliError(WorkspaceCliErrorCode);

impl fmt::Display for WorkspaceCliError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "workspace command failed ({:?})", self.0)
    }
}

impl std::error::Error for WorkspaceCliError {}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RecipeContextDocument {
    #[serde(default)]
    executable_identities: Vec<ResolvedExecutable>,
    #[serde(default)]
    requested_capabilities: Vec<String>,
    #[serde(default)]
    public_variables: BTreeMap<String, String>,
}

pub fn execute_workspaces_command(
    command: &WorkspacesCommand,
) -> Result<(), Box<dyn std::error::Error>> {
    execute_workspaces_command_at(command, &rio_backend::config::config_dir_path())
}

#[doc(hidden)]
pub fn execute_workspaces_command_at(
    command: &WorkspacesCommand,
    config_root: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let store = ConnectionLibraryStore::open(config_root.join("connections"))
        .map_err(|_| WorkspaceCliError(WorkspaceCliErrorCode::LibraryUnavailable))?;
    let loaded = store
        .load()
        .map_err(|_| WorkspaceCliError(WorkspaceCliErrorCode::LibraryUnavailable))?;
    let document = &loaded.document;
    match &command.action {
        WorkspacesAction::List { json } => {
            if *json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "schema_version": document.schema_version,
                        "library_revision": document.revision,
                        "workspaces": document.workspaces.workspaces.iter().map(|workspace| serde_json::json!({
                            "id": workspace.id,
                            "revision": workspace.revision,
                            "display_name": workspace.display_name,
                            "environment": workspace.environment,
                            "window_count": workspace.windows.len(),
                            "connection_count": workspace.connections.len(),
                        })).collect::<Vec<_>>(),
                        "execution_enabled": false,
                    }))?
                );
            } else {
                println!(
                    "Workspace Library r{} · {} saved workspace(s) · review only",
                    document.revision,
                    document.workspaces.workspaces.len()
                );
                for workspace in &document.workspaces.workspaces {
                    println!(
                        "{}  r{}  {}  {} window(s) · {} connection(s)",
                        workspace.id,
                        workspace.revision,
                        workspace.display_name,
                        workspace.windows.len(),
                        workspace.connections.len()
                    );
                }
            }
        }
        WorkspacesAction::Show { id, json } => {
            let workspace = document
                .workspaces
                .workspaces
                .iter()
                .find(|workspace| workspace.id == *id)
                .ok_or(WorkspaceCliError(WorkspaceCliErrorCode::ReviewRejected))?;
            if *json {
                println!("{}", serde_json::to_string_pretty(workspace)?);
            } else {
                println!("{} · {}", workspace.display_name, workspace.id);
                println!(
                    "Revision {} · {:?} risk · {} window(s) · {} connection(s)",
                    workspace.revision,
                    workspace.environment.risk,
                    workspace.windows.len(),
                    workspace.connections.len()
                );
                println!("Declarative topology only; no live session state is saved.");
            }
        }
        WorkspacesAction::Put {
            input,
            apply,
            expected_revision,
            expected_entity_revision,
            json,
        } => {
            let bytes = read_bounded_regular(input, MAX_DOCUMENT_BYTES)?;
            let workspace = parse_workspace_json(&bytes)
                .map_err(|_| WorkspaceCliError(WorkspaceCliErrorCode::InputRejected))?;
            let current_entity_revision = document
                .workspaces
                .workspaces
                .iter()
                .find(|candidate| candidate.id == workspace.id)
                .map(|candidate| candidate.revision);
            let entity_revision = if *apply {
                match expected_entity_revision {
                    Some(0) => None,
                    Some(revision) => Some(*revision),
                    None => {
                        return Err(Box::new(WorkspaceCliError(
                            WorkspaceCliErrorCode::StaleRevision,
                        )));
                    }
                }
            } else {
                current_entity_revision
            };
            let preview = preview_library_edit(
                document,
                LibraryEdit::put_workspace(entity_revision, workspace),
            )
            .map_err(|_| WorkspaceCliError(WorkspaceCliErrorCode::ReviewRejected))?;
            let resulting_revision = if *apply {
                require_revision(*expected_revision, preview.base_revision)?;
                Some(
                    store
                        .commit_edit(&preview)
                        .map_err(|_| {
                            WorkspaceCliError(WorkspaceCliErrorCode::StaleRevision)
                        })?
                        .revision,
                )
            } else {
                None
            };
            print_edit_review(
                *json,
                &preview.changed_entity,
                preview.base_revision,
                preview.invalidated_approval_count,
                &preview.preview_fingerprint,
                resulting_revision,
            )?;
        }
        WorkspacesAction::Remove {
            id,
            entity_revision,
            apply,
            expected_revision,
            json,
        } => {
            let preview = preview_library_edit(
                document,
                LibraryEdit::RemoveWorkspace {
                    expected_entity_revision: *entity_revision,
                    workspace_id: id.clone(),
                },
            )
            .map_err(|_| WorkspaceCliError(WorkspaceCliErrorCode::ReviewRejected))?;
            let resulting_revision = if *apply {
                require_revision(*expected_revision, preview.base_revision)?;
                Some(
                    store
                        .commit_edit(&preview)
                        .map_err(|_| {
                            WorkspaceCliError(WorkspaceCliErrorCode::StaleRevision)
                        })?
                        .revision,
                )
            } else {
                None
            };
            print_edit_review(
                *json,
                &preview.changed_entity,
                preview.base_revision,
                preview.invalidated_approval_count,
                &preview.preview_fingerprint,
                resulting_revision,
            )?;
        }
        WorkspacesAction::Restore {
            id,
            generation,
            json,
        } => {
            let review = review_library_workspace_restore(document, id, *generation)
                .map_err(|_| WorkspaceCliError(WorkspaceCliErrorCode::ReviewRejected))?;
            if *json {
                println!("{}", serde_json::to_string_pretty(&review)?);
            } else {
                println!(
                    "Restore review: {} · r{} · generation {}",
                    review.workspace_id,
                    review.workspace_revision,
                    review.connection_generation
                );
                println!(
                    "{} target(s) · fresh sessions only · reconnect off · resume off",
                    review.targets.len()
                );
                println!(
                    "Execution unavailable; no process, PTY, or network was opened."
                );
            }
        }
        WorkspacesAction::RecipePlan {
            profile,
            generation,
            no_hooks,
            context,
            json,
        } => {
            let context = context
                .as_ref()
                .map(|path| read_recipe_context(path))
                .transpose()?
                .unwrap_or(RecipeContextDocument {
                    executable_identities: Vec::new(),
                    requested_capabilities: Vec::new(),
                    public_variables: BTreeMap::new(),
                });
            let review = review_library_recipe(
                document,
                RecipeReviewRequest {
                    profile_id: profile.clone(),
                    mode: if *no_hooks {
                        RecipeRunMode::NoHooks
                    } else {
                        RecipeRunMode::ReviewedHooks
                    },
                    connection_generation: *generation,
                    executable_identities: context.executable_identities,
                    requested_capabilities: context.requested_capabilities,
                    public_variables: context.public_variables,
                },
            )
            .map_err(|_| WorkspaceCliError(WorkspaceCliErrorCode::ReviewRejected))?;
            if *json {
                println!("{}", serde_json::to_string_pretty(&review)?);
            } else {
                println!(
                    "Recipe review: {} · r{} · {:?}",
                    review.profile_id, review.profile_revision, review.mode
                );
                for step in &review.steps {
                    println!(
                        "{:>2}. {:?} · {:?} · {}",
                        usize::from(step.sequence) + 1,
                        step.stage,
                        step.risk,
                        step.id
                    );
                }
                println!(
                    "{} hook(s) omitted · execution unavailable",
                    review.omitted_hook_count
                );
            }
        }
        WorkspacesAction::Broadcast {
            id,
            command_file,
            arm_duration_ms,
            json,
        } => {
            let exact_command = read_exact_command(command_file)?;
            let now_ms = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|_| WorkspaceCliError(WorkspaceCliErrorCode::ClockUnavailable))?
                .as_millis()
                .try_into()
                .map_err(|_| {
                    WorkspaceCliError(WorkspaceCliErrorCode::ClockUnavailable)
                })?;
            let review = review_library_broadcast(
                document,
                id,
                &exact_command,
                now_ms,
                *arm_duration_ms,
            )
            .map_err(|_| WorkspaceCliError(WorkspaceCliErrorCode::ReviewRejected))?;
            if *json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "schema_version": review.schema_version,
                        "exact_command": review.exact_command(),
                        "command_digest": review.command_digest,
                        "command_byte_count": review.command_byte_count,
                        "targets": review.targets,
                        "production_confirmation_required": review.production_confirmation_required,
                        "approval_fingerprint": review.approval_fingerprint,
                        "reviewed_at_ms": review.reviewed_at_ms,
                        "armed_until_ms": review.armed_until_ms,
                        "review_required": true,
                        "execution_enabled": false,
                        "enter_requested": false,
                    }))?
                );
            } else {
                println!("Exact command: {}", review.exact_command());
                println!(
                    "{} target(s) · production confirmation {} · execution unavailable",
                    review.targets.len(),
                    if review.production_confirmation_required {
                        "required"
                    } else {
                        "not required"
                    }
                );
                println!("No Enter key was sent and no session was opened.");
            }
        }
        WorkspacesAction::Migrate {
            apply,
            expected_revision,
            json,
        } => {
            match loaded.origin {
                LibraryLoadOrigin::PrimaryMigrationPreview => {}
                LibraryLoadOrigin::PreviousMigrationPreview => {
                    return Err(Box::new(WorkspaceCliError(
                        WorkspaceCliErrorCode::RecoveryRequired,
                    )));
                }
                _ => {
                    return Err(Box::new(WorkspaceCliError(
                        WorkspaceCliErrorCode::MigrationNotRequired,
                    )));
                }
            }
            let resulting_revision = if *apply {
                require_revision(*expected_revision, document.revision)?;
                Some(
                    store
                        .compare_and_swap(document.revision, document)
                        .map_err(|_| {
                            WorkspaceCliError(WorkspaceCliErrorCode::StaleRevision)
                        })?
                        .revision,
                )
            } else {
                None
            };
            print_state_review(
                *json,
                "schema-migration",
                document.revision,
                resulting_revision,
            )?;
        }
        WorkspacesAction::Recover {
            previous_revision,
            apply,
            json,
        } => {
            if !matches!(
                loaded.origin,
                LibraryLoadOrigin::PreviousRecovery
                    | LibraryLoadOrigin::PreviousMigrationPreview
            ) {
                return Err(Box::new(WorkspaceCliError(
                    WorkspaceCliErrorCode::RecoveryNotRequired,
                )));
            }
            require_revision(Some(*previous_revision), document.revision)?;
            let resulting_revision = if *apply {
                Some(
                    store
                        .recover_previous(*previous_revision)
                        .map_err(|_| {
                            WorkspaceCliError(WorkspaceCliErrorCode::StaleRevision)
                        })?
                        .revision,
                )
            } else {
                None
            };
            print_state_review(
                *json,
                "previous-generation-recovery",
                *previous_revision,
                resulting_revision,
            )?;
        }
        WorkspacesAction::Doctor { json } => {
            let blockers = match m6_activation_readiness() {
                super::M6ActivationReadiness::Blocked { blockers } => blockers,
            };
            let blocker_names = blockers
                .iter()
                .map(|blocker| match blocker {
                    M6ActivationBlocker::D3ProtectedActivation => {
                        "d3-protected-activation"
                    }
                    M6ActivationBlocker::M5NativeLifecycleEvidence => {
                        "m5-native-lifecycle-evidence"
                    }
                })
                .collect::<Vec<_>>();
            if *json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "schema_version": document.schema_version,
                        "library_revision": document.revision,
                        "load_origin": format!("{:?}", loaded.origin),
                        "rejected_primary": loaded.rejected_primary,
                        "workspace_count": document.workspaces.workspaces.len(),
                        "activation_blockers": blocker_names,
                        "execution_enabled": false,
                    }))?
                );
            } else {
                println!(
                    "Workspace Library r{} · {:?} · {} workspace(s)",
                    document.revision,
                    loaded.origin,
                    document.workspaces.workspaces.len()
                );
                println!("Execution unavailable: {}", blocker_names.join(", "));
            }
        }
    }
    Ok(())
}

fn require_revision(
    supplied: Option<u64>,
    current: u64,
) -> Result<(), WorkspaceCliError> {
    if supplied == Some(current) {
        Ok(())
    } else {
        Err(WorkspaceCliError(WorkspaceCliErrorCode::StaleRevision))
    }
}

fn read_bounded_regular(
    path: &Path,
    maximum: usize,
) -> Result<Vec<u8>, WorkspaceCliError> {
    private_fs::read_bounded_untrusted_regular(path, maximum)
        .map_err(|_| WorkspaceCliError(WorkspaceCliErrorCode::InputRejected))?
        .ok_or(WorkspaceCliError(WorkspaceCliErrorCode::InputRejected))
}

fn read_recipe_context(path: &Path) -> Result<RecipeContextDocument, WorkspaceCliError> {
    let bytes = read_bounded_regular(path, MAX_DOCUMENT_BYTES)?;
    from_json_slice_without_duplicate_keys(&bytes)
        .map_err(|_| WorkspaceCliError(WorkspaceCliErrorCode::InputRejected))
}

fn read_exact_command(path: &Path) -> Result<String, WorkspaceCliError> {
    let bytes = read_bounded_regular(path, MAX_BROADCAST_COMMAND_BYTES + 2)?;
    let mut command = String::from_utf8(bytes)
        .map_err(|_| WorkspaceCliError(WorkspaceCliErrorCode::InputRejected))?;
    if command.ends_with("\r\n") {
        command.truncate(command.len() - 2);
    } else if command.ends_with('\n') {
        command.pop();
    }
    if command.len() > MAX_BROADCAST_COMMAND_BYTES {
        return Err(WorkspaceCliError(WorkspaceCliErrorCode::InputRejected));
    }
    Ok(command)
}

fn print_edit_review(
    json: bool,
    changed_entity: &str,
    base_revision: u64,
    invalidated_approval_count: usize,
    preview_fingerprint: &str,
    resulting_revision: Option<u64>,
) -> Result<(), serde_json::Error> {
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "changed_entity": changed_entity,
                "base_revision": base_revision,
                "invalidated_approval_count": invalidated_approval_count,
                "preview_fingerprint": preview_fingerprint,
                "review_required": true,
                "execution_enabled": false,
                "applied": resulting_revision.is_some(),
                "resulting_revision": resulting_revision,
            }))?
        );
    } else {
        println!(
            "Review {} at library r{} · {} approval(s) invalidated",
            changed_entity, base_revision, invalidated_approval_count
        );
        match resulting_revision {
            Some(revision) => println!("Applied atomically as library r{revision}."),
            None => println!("Preview only; no files were changed."),
        }
    }
    Ok(())
}

fn print_state_review(
    json: bool,
    operation: &str,
    source_revision: u64,
    resulting_revision: Option<u64>,
) -> Result<(), serde_json::Error> {
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "operation": operation,
                "source_revision": source_revision,
                "review_required": true,
                "applied": resulting_revision.is_some(),
                "resulting_revision": resulting_revision,
                "execution_enabled": false,
            }))?
        );
    } else if let Some(revision) = resulting_revision {
        println!("{operation} applied atomically as library r{revision}.");
    } else {
        println!("{operation} preview at r{source_revision}; no files were changed.");
    }
    Ok(())
}
