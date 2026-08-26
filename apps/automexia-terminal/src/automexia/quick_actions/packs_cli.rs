//! Read-only-by-default CLI for reviewed CP3.2 DevOps action packs.

use std::io;

use automexia_command_productivity::actions::{
    builtin_pack, builtin_packs, evaluate_pack_health, materialize_pack_action,
    pack_alias_eligibility, pack_registry_digest, validate_pack_registry, ActionTemplate,
    ArgumentToken, PackAction, PackActionEffect, PackHealthState, PackToolObservation,
    RiskClass, ShellKind,
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
    display_name: &'a str,
    description: &'a str,
    effect: PackActionEffect,
    risk: RiskClass,
    argv: Vec<String>,
    documentation_url: &'a str,
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
                return print_json_or_line(
                    registry_doctor_summary(),
                    *json,
                    "state=registry-ready packs=11 actions=33 provider-processes-started=0",
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
                let missing = report
                    .missing_completion_shells
                    .iter()
                    .copied()
                    .map(shell_label)
                    .collect::<Vec<_>>()
                    .join(",");
                println!(
                    "pack={} state={} detected={} minimum={} missing-completions=[{}] detail={}",
                    report.pack_id,
                    health_state_label(report.state),
                    report.detected_version.as_deref().unwrap_or("unobserved"),
                    report.minimum_version,
                    missing,
                    serde_json::to_string(&report.detail)?
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
                "{}\t{}\tversion={}\texecutable={}\ttool>={}\tactions={}\talias-eligible={}",
                item.id,
                item.display_name,
                item.version,
                item.executable_id,
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
    let argv = pack_action_argv(entry);
    let argv_text = serde_json::to_string(&argv)?;
    let display_name_text = serde_json::to_string(&action.display_name)?;
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
        ensure_expected_revision(expected, revision)?;
        let service = QuickActionService::open(QuickActionStore::open_or_create(&root)?)?;
        service.create(expected, action)?.revision()
    } else {
        revision
    };
    let summary = EnableSummary {
        pack_id,
        action_id,
        display_name: &entry.action.display_name,
        description: &entry.action.description,
        effect: entry.effect,
        risk: entry.action.risk,
        argv,
        documentation_url: &pack.documentation_url,
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
            "enabled pack={pack_id} action={action_id} name={display_name_text} effect={} risk={} argv={argv_text} docs={} revision={saved_revision} alias=disabled (applied)",
            effect_label(entry.effect),
            risk_label(entry.action.risk),
            pack.documentation_url
        );
    } else {
        println!(
            "would enable pack={pack_id} action={action_id} name={display_name_text} effect={} risk={} argv={argv_text} docs={} revision={revision} alias=disabled (dry-run; add --apply --expected-revision {revision})",
            effect_label(entry.effect),
            risk_label(entry.action.risk),
            pack.documentation_url
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

fn registry_doctor_summary() -> serde_json::Value {
    serde_json::json!({
        "state": "registry-ready",
        "packs": builtin_packs().len(),
        "actions": builtin_packs().iter().map(|pack| pack.actions.len()).sum::<usize>(),
        "registry_digest": pack_registry_digest(),
        "provider_processes_started": 0,
    })
}

fn pack_action_argv(entry: &PackAction) -> Vec<String> {
    let ActionTemplate::TypedArgv {
        executable_id,
        arguments,
    } = &entry.action.template
    else {
        unreachable!("validated built-in pack actions use typed argv")
    };
    std::iter::once(executable_id.clone())
        .chain(arguments.iter().map(|argument| match argument {
            ArgumentToken::Literal { value } => value.clone(),
            ArgumentToken::Placeholder { name } => format!("{{{name}}}"),
        }))
        .collect()
}

fn ensure_expected_revision(expected: u64, current: u64) -> io::Result<()> {
    if expected == current {
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "revision conflict: expected {expected}, current {current}; rerun the dry-run preview"
            ),
        ))
    }
}

const fn effect_label(effect: PackActionEffect) -> &'static str {
    match effect {
        PackActionEffect::Inspection => "inspection",
        PackActionEffect::BoundedMutation => "bounded-mutation",
        PackActionEffect::ContextChange => "context-change",
        PackActionEffect::Authentication => "authentication",
        PackActionEffect::Destructive => "destructive",
        PackActionEffect::Privileged => "privileged",
    }
}

const fn risk_label(risk: RiskClass) -> &'static str {
    match risk {
        RiskClass::ReadOnly => "read-only",
        RiskClass::Mutating => "mutating",
        RiskClass::Destructive => "destructive",
        RiskClass::Privileged => "privileged",
    }
}

const fn health_state_label(state: PackHealthState) -> &'static str {
    match state {
        PackHealthState::Unobserved => "unobserved",
        PackHealthState::Missing => "missing",
        PackHealthState::UnsupportedVersion => "unsupported-version",
        PackHealthState::CompletionUnavailable => "completion-unavailable",
        PackHealthState::Ready => "ready",
    }
}

const fn shell_label(shell: ShellKind) -> &'static str {
    match shell {
        ShellKind::Powershell => "powershell",
        ShellKind::Bash => "bash",
        ShellKind::Zsh => "zsh",
        ShellKind::Fish => "fish",
        ShellKind::Cmd => "cmd",
    }
}

fn not_found(kind: &str, id: &str) -> io::Error {
    io::Error::new(io::ErrorKind::NotFound, format!("{kind} {id:?} not found"))
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pack_enable_preview_exposes_exact_review_fields() {
        let pack = builtin_pack("kubernetes").unwrap();
        let entry = pack
            .actions
            .iter()
            .find(|entry| entry.action.id == "kubernetes.use-context")
            .unwrap();
        assert_eq!(
            pack_action_argv(entry),
            ["kubectl", "config", "use-context", "{context}"]
        );
        assert_eq!(effect_label(entry.effect), "context-change");
        assert_eq!(risk_label(entry.action.risk), "mutating");
    }

    #[test]
    fn registry_doctor_does_not_claim_provider_readiness() {
        let summary = registry_doctor_summary();
        assert_eq!(summary["state"], "registry-ready");
        assert_eq!(summary["provider_processes_started"], 0);
    }

    #[test]
    fn stale_pack_enable_revision_is_actionable() {
        let error = ensure_expected_revision(4, 5).unwrap_err();
        assert!(error.to_string().contains("expected 4, current 5"));
        assert!(error.to_string().contains("rerun the dry-run preview"));
    }
}
