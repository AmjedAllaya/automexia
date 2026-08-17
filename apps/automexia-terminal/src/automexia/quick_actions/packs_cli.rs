//! Read-only-by-default CLI for reviewed CP3.2 DevOps action packs.

use std::io;

use automexia_devops::actions::{
    builtin_pack, builtin_packs, evaluate_pack_health, materialize_pack_action,
    pack_alias_eligibility, pack_registry_digest, validate_pack_registry,
    PackToolObservation,
};
use serde::Serialize;

use crate::cli::{PacksAction, PacksCommand};

use super::{QuickActionService, QuickActionStore};

#[derive(Serialize)]
struct PackSummary<'a> {
    id: &'a str,
    display_name: &'a str,
    version: &'a str,
    executable_id: &'a str,
    minimum_tool_version: &'a str,
    actions: usize,
    alias_eligible_actions: usize,
}

#[derive(Serialize)]
struct EnableSummary<'a> {
    pack_id: &'a str,
    action_id: &'a str,
    applied: bool,
    revision: u64,
    registry_digest: String,
    default_alias_enabled: bool,
    alias_eligible: bool,
    alias_reason: &'a str,
}

pub fn execute_packs_command(
    command: &PacksCommand,
) -> Result<(), Box<dyn std::error::Error>> {
    match &command.action {
        PacksAction::List { json } => print_list(*json),
        PacksAction::Show { id, json } => {
            let pack = builtin_pack(id).ok_or_else(|| not_found("pack", id))?;
            if *json {
                println!("{}", serde_json::to_string_pretty(pack)?);
            } else {
                println!("{}", toml::to_string_pretty(pack)?);
            }
            Ok(())
        }
        PacksAction::Doctor {
            id,
            tool_version,
            missing,
            completion_shell,
            json,
        } => {
            let Some(id) = id else {
                if tool_version.is_some() || *missing || !completion_shell.is_empty() {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "provider observations require a pack ID",
                    )
                    .into());
                }
                validate_pack_registry(builtin_packs())?;
                let result = serde_json::json!({
                    "state": "ready",
                    "packs": builtin_packs().len(),
                    "actions": builtin_packs().iter().map(|pack| pack.actions.len()).sum::<usize>(),
                    "registry_digest": pack_registry_digest(),
                    "provider_processes_started": 0,
                });
                return print_json_or_line(
                    result,
                    *json,
                    "state=ready packs=11 actions=33 provider-processes-started=0",
                );
            };
            let pack = builtin_pack(id).ok_or_else(|| not_found("pack", id))?;
            let observation = if *missing {
                PackToolObservation::Missing
            } else if let Some(version_output) = tool_version {
                PackToolObservation::Detected {
                    version_output: version_output.clone(),
                    completion_shells: completion_shell
                        .iter()
                        .copied()
                        .map(Into::into)
                        .collect(),
                }
            } else {
                PackToolObservation::Unobserved
            };
            let report = evaluate_pack_health(pack, &observation)?;
            if *json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                println!(
                    "pack={} state={:?} detected={} minimum={} missing-completions={}",
                    report.pack_id,
                    report.state,
                    report.detected_version.as_deref().unwrap_or("unobserved"),
                    report.minimum_version,
                    report.missing_completion_shells.len()
                );
            }
            Ok(())
        }
        PacksAction::Enable {
            pack,
            action,
            apply,
            expected_revision,
            json,
        } => enable(pack, action, *apply, *expected_revision, *json),
    }
}

fn print_list(json: bool) -> Result<(), Box<dyn std::error::Error>> {
    validate_pack_registry(builtin_packs())?;
    let summaries = builtin_packs()
        .iter()
        .map(|pack| PackSummary {
            id: &pack.id,
            display_name: &pack.display_name,
            version: &pack.version,
            executable_id: &pack.executable_id,
            minimum_tool_version: &pack.minimum_tool_version,
            actions: pack.actions.len(),
            alias_eligible_actions: pack
                .actions
                .iter()
                .filter(|entry| pack_alias_eligibility(entry).eligible)
                .count(),
        })
        .collect::<Vec<_>>();
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "registry_digest": pack_registry_digest(),
                "packs": summaries,
            }))?
        );
    } else {
        for item in summaries {
            println!(
                "{}\t{}\tversion={}\ttool>={}\tactions={}\talias-eligible={}",
                item.id,
                item.display_name,
                item.version,
                item.minimum_tool_version,
                item.actions,
                item.alias_eligible_actions
            );
        }
    }
    Ok(())
}

fn enable(
    pack_id: &str,
    action_id: &str,
    apply: bool,
    expected_revision: Option<u64>,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let pack = builtin_pack(pack_id).ok_or_else(|| not_found("pack", pack_id))?;
    let entry = pack
        .actions
        .iter()
        .find(|entry| entry.action.id == action_id)
        .ok_or_else(|| not_found("pack action", action_id))?;
    let action = materialize_pack_action(pack_id, action_id)?;
    let eligibility = pack_alias_eligibility(entry);
    let root = rio_backend::config::config_dir_path().join("actions");
    let existing = QuickActionStore::open_existing_read_only(&root)?
        .map(QuickActionService::open_read_only)
        .transpose()?;
    let revision = existing
        .as_ref()
        .map(|service| service.snapshot().revision())
        .unwrap_or(0);
    if existing.as_ref().is_some_and(|service| {
        service
            .snapshot()
            .actions()
            .document()
            .actions
            .iter()
            .any(|candidate| candidate.id == action_id)
    }) {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "action already exists; pack enable never overwrites user state",
        )
        .into());
    }

    let saved_revision = if apply {
        let expected = expected_revision.ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "--expected-revision is required with --apply",
            )
        })?;
        let service = QuickActionService::open(QuickActionStore::open_or_create(&root)?)?;
        service.create(expected, action)?.revision()
    } else {
        revision
    };
    let summary = EnableSummary {
        pack_id,
        action_id,
        applied: apply,
        revision: saved_revision,
        registry_digest: pack_registry_digest(),
        default_alias_enabled: false,
        alias_eligible: eligibility.eligible,
        alias_reason: &eligibility.reason,
    };
    if json {
        println!("{}", serde_json::to_string_pretty(&summary)?);
    } else if apply {
        println!(
            "enabled pack={pack_id} action={action_id} revision={saved_revision} alias=disabled (applied)"
        );
    } else {
        println!(
            "would enable pack={pack_id} action={action_id} revision={revision} alias=disabled (dry-run; add --apply --expected-revision {revision})"
        );
    }
    Ok(())
}

fn print_json_or_line(
    value: serde_json::Value,
    json: bool,
    line: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    if json {
        println!("{}", serde_json::to_string_pretty(&value)?);
    } else {
        println!("{line}");
    }
    Ok(())
}

fn not_found(kind: &str, id: &str) -> io::Error {
    io::Error::new(io::ErrorKind::NotFound, format!("{kind} {id:?} not found"))
}
