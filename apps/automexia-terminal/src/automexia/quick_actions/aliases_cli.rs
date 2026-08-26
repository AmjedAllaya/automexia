//! Dry-run-first CP3.1 alias projection management.

use std::{
    fs,
    io::{self, Write as _},
    path::Path,
    process::{Command, Stdio},
};

use automexia_command_productivity::actions::{
    validate_quick_actions, AliasArgumentPolicy as ModelArgumentPolicy, AliasProjection,
    AliasProjectionMode, CompletionHealth, CompletionMode, ExactOverrideConsent,
    OverridePolicy, ProjectionArtifact, ProjectionDecision, RiskClass, ShellKind,
    ToolHealth,
};
use serde::Serialize;

use crate::cli::{
    AliasArgumentPolicy, AliasCompletion, AliasShell, AliasesAction, AliasesCommand,
};

use super::{
    collect_local_alias_observations, shell_label, AliasDoctorReport, AliasError,
    AliasErrorCode, AliasHealth, AliasObservationSet, AliasProjectionPlan,
    AliasProjectionStore, GenerationExpectation, QuickActionService, QuickActionStore,
    StoreError, StoreErrorCode,
};

#[derive(Serialize)]
struct AliasOperation<'a> {
    operation: &'a str,
    applied: bool,
    source_revision: u64,
    source_digest: &'a str,
    expected_revision: Option<u64>,
    expected_generation: Option<&'a str>,
    published_generation: Option<&'a str>,
    ready_bindings: usize,
    decisions: usize,
    reload: &'a str,
}

#[derive(Serialize)]
struct AliasListItem {
    action_id: String,
    name: String,
    shells: Vec<ShellKind>,
    enabled: bool,
    risk: RiskClass,
}

pub fn execute_aliases_command(
    command: &AliasesCommand,
) -> Result<(), Box<dyn std::error::Error>> {
    let config_root = rio_backend::config::config_dir_path();
    let actions_root = config_root.join("actions");
    let aliases_root = config_root.join("generated").join("aliases");
    match &command.action {
        AliasesAction::List { shell, json } => {
            let source = open_actions_existing(&actions_root)?;
            let items = source
                .as_ref()
                .map(|service| {
                    let snapshot = service.snapshot();
                    snapshot
                        .actions()
                        .document()
                        .actions
                        .iter()
                        .filter_map(|action| {
                            let alias = action.alias_projection.as_ref()?;
                            if shell.is_some_and(|shell| {
                                !alias.shells.contains(&ShellKind::from(shell))
                            }) {
                                return None;
                            }
                            Some(AliasListItem {
                                action_id: action.id.clone(),
                                name: alias.requested_name.clone(),
                                shells: alias.shells.clone(),
                                enabled: action.enabled && alias.enabled,
                                risk: action.risk,
                            })
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            let health = doctor_existing(&aliases_root, source.as_ref());
            if *json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "health": health,
                        "aliases": items,
                    }))?
                );
            } else {
                println!(
                    "health={:?} aliases={} generation={}",
                    health.health,
                    items.len(),
                    health.generation.as_deref().unwrap_or("none")
                );
                for item in items {
                    println!(
                        "{}\t{}\t{}\t{:?}",
                        item.action_id,
                        item.name,
                        if item.enabled { "enabled" } else { "disabled" },
                        item.shells
                    );
                }
            }
            Ok(())
        }
        AliasesAction::Preview {
            shell,
            show_source,
            json,
        } => {
            let source = require_actions(&actions_root)?;
            let snapshot = source.snapshot();
            let (temporary, store) = preview_store(&aliases_root)?;
            let mut observations =
                collect_local_alias_observations(snapshot.actions(), &config_root)?;
            reuse_authenticated_exact_overrides(
                &store,
                snapshot.actions(),
                &mut observations,
            )?;
            let plan = store.compile_snapshot(&snapshot, &observations)?;
            print_preview(&plan, *shell, *show_source, *json)?;
            drop(temporary);
            Ok(())
        }
        AliasesAction::Test { shell, json } => {
            let source = require_actions(&actions_root)?;
            let snapshot = source.snapshot();
            let (temporary, store) = preview_store(&aliases_root)?;
            let mut observations =
                collect_local_alias_observations(snapshot.actions(), &config_root)?;
            reuse_authenticated_exact_overrides(
                &store,
                snapshot.actions(),
                &mut observations,
            )?;
            let plan = store.compile_snapshot(&snapshot, &observations)?;
            let result = test_plan(&plan, *shell, *json);
            drop(temporary);
            result
        }
        AliasesAction::Regenerate {
            apply,
            expected_generation,
            json,
        } => {
            let source = require_actions(&actions_root)?;
            let snapshot = source.snapshot();
            let (temporary, store) = if *apply {
                (None, AliasProjectionStore::open_or_create(&aliases_root)?)
            } else {
                preview_store(&aliases_root)?
            };
            if *apply {
                store.recover_pending(snapshot.actions())?;
            }
            let current_generation =
                generation_token(&store.doctor(Some(snapshot.actions())));
            let mut observations =
                collect_local_alias_observations(snapshot.actions(), &config_root)?;
            reuse_authenticated_exact_overrides(
                &store,
                snapshot.actions(),
                &mut observations,
            )?;
            let plan = store.compile_snapshot(&snapshot, &observations)?;
            let generation = if *apply {
                let expected = expectation(expected_generation.as_deref())?;
                Some(store.publish(&plan, expected)?.generation)
            } else {
                None
            };
            let result = print_plan(
                "regenerate",
                *apply,
                &plan,
                None,
                Some(
                    expected_generation
                        .as_deref()
                        .unwrap_or(current_generation.as_str()),
                ),
                generation.as_deref(),
                *json,
            );
            drop(temporary);
            result
        }
        AliasesAction::Enable {
            id,
            name,
            shell,
            argument_policy,
            completion,
            mutating_acknowledged,
            override_owner_fingerprint,
            apply,
            expected_revision,
            expected_generation,
            json,
        } => mutate_and_publish(
            MutationRequest {
                operation: "enable",
                config_root: &config_root,
                actions_root: &actions_root,
                aliases_root: &aliases_root,
                action_id: id,
                apply: *apply,
                expected_revision: *expected_revision,
                expected_generation: expected_generation.as_deref(),
                json: *json,
                override_fingerprint: override_owner_fingerprint.as_deref(),
            },
            |action| {
                if matches!(action.risk, RiskClass::Mutating) && !*mutating_acknowledged {
                    return Err(input(
                        "mutating aliases require --mutating-acknowledged",
                    ));
                }
                let shells = shell
                    .iter()
                    .copied()
                    .map(ShellKind::from)
                    .collect::<Vec<_>>();
                if shells.is_empty()
                    || shells
                        .iter()
                        .any(|candidate| !action.shells.contains(candidate))
                {
                    return Err(input(
                        "selected alias shells must be supported by the action",
                    ));
                }
                if override_owner_fingerprint.is_some() && shells.len() != 1 {
                    return Err(input("an exact override requires exactly one --shell"));
                }
                action.alias_projection = Some(AliasProjection {
                    requested_name: name.clone(),
                    shells,
                    mode: AliasProjectionMode::Auto,
                    argument_policy: model_argument_policy(*argument_policy),
                    completion: model_completion(*completion),
                    override_policy: if override_owner_fingerprint.is_some() {
                        OverridePolicy::ExplicitExactOverride
                    } else {
                        OverridePolicy::NativeWins
                    },
                    mutating_acknowledged: *mutating_acknowledged,
                    enabled: true,
                });
                Ok(())
            },
        ),
        AliasesAction::Disable {
            id,
            shell,
            apply,
            expected_revision,
            expected_generation,
            json,
        } => mutate_and_publish(
            MutationRequest {
                operation: "disable",
                config_root: &config_root,
                actions_root: &actions_root,
                aliases_root: &aliases_root,
                action_id: id,
                apply: *apply,
                expected_revision: *expected_revision,
                expected_generation: expected_generation.as_deref(),
                json: *json,
                override_fingerprint: None,
            },
            |action| {
                let alias = action
                    .alias_projection
                    .as_mut()
                    .ok_or_else(|| input("action has no alias projection"))?;
                if let Some(shell) = shell {
                    let shell = ShellKind::from(*shell);
                    alias.shells.retain(|candidate| *candidate != shell);
                    if alias.shells.is_empty() {
                        alias.enabled = false;
                    }
                } else {
                    alias.enabled = false;
                }
                Ok(())
            },
        ),
        AliasesAction::Rename {
            id,
            name,
            apply,
            expected_revision,
            expected_generation,
            json,
        } => mutate_and_publish(
            MutationRequest {
                operation: "rename",
                config_root: &config_root,
                actions_root: &actions_root,
                aliases_root: &aliases_root,
                action_id: id,
                apply: *apply,
                expected_revision: *expected_revision,
                expected_generation: expected_generation.as_deref(),
                json: *json,
                override_fingerprint: None,
            },
            |action| {
                action
                    .alias_projection
                    .as_mut()
                    .ok_or_else(|| input("action has no alias projection"))?
                    .requested_name = name.clone();
                Ok(())
            },
        ),
        AliasesAction::DisableAll {
            apply,
            expected_generation,
        } => {
            if *apply {
                let store = AliasProjectionStore::open_or_create(&aliases_root)?;
                let source = require_actions(&actions_root)?;
                let snapshot = source.snapshot();
                store.recover_pending(snapshot.actions())?;
                let expected = expected_generation.as_deref().ok_or_else(|| {
                    input("--expected-generation is required with --apply")
                })?;
                let previous = store.disable(expectation(Some(expected))?)?;
                println!(
                    "operation=disable-all applied=true expected-generation={} published-generation=disabled previous-generation={} saved-actions=preserved reload=required",
                    expected,
                    previous.as_deref().unwrap_or("none")
                );
            } else {
                let source = open_actions_existing(&actions_root)?;
                let report = doctor_existing(&aliases_root, source.as_ref());
                println!(
                    "operation=disable-all applied=false expected-generation={} published-generation=none saved-actions=preserved reload=not-applied",
                    generation_token(&report)
                );
            }
            Ok(())
        }
        AliasesAction::Rollback {
            current_generation,
            apply,
            json,
        } => {
            if !*apply {
                println!(
                    "would roll back generation={current_generation} (dry-run; add --apply)"
                );
                return Ok(());
            }
            let store = AliasProjectionStore::open_or_create(&aliases_root)?;
            let source = require_actions(&actions_root)?;
            let snapshot = source.snapshot();
            store.recover_pending(snapshot.actions())?;
            let publication = store.rollback(current_generation)?;
            if *json {
                println!("{}", serde_json::to_string_pretty(&publication)?);
            } else {
                println!(
                    "rolled-back generation={} previous={} reload=required",
                    publication.generation,
                    publication.previous_generation.as_deref().unwrap_or("none")
                );
            }
            Ok(())
        }
        AliasesAction::Doctor { json } => {
            let report = match open_actions_existing(&actions_root) {
                Ok(source) => doctor_existing(&aliases_root, source.as_ref()),
                Err(error) => source_failure_report(&aliases_root, error.as_ref()),
            };
            if *json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                println!(
                    "health={:?} generation={} previous={} source-revision={} canonical-revision={} ready={} decisions={} error={}",
                    report.health,
                    report.generation.as_deref().unwrap_or("none"),
                    report.previous_generation.as_deref().unwrap_or("none"),
                    report
                        .source_revision
                        .map_or_else(|| "none".to_owned(), |value| value.to_string()),
                    report
                        .canonical_revision
                        .map_or_else(|| "none".to_owned(), |value| value.to_string()),
                    report.ready_bindings,
                    report.decisions,
                    report.error_code.unwrap_or("none")
                );
            }
            Ok(())
        }
        AliasesAction::Reload { shell } => {
            let command = match shell {
                AliasShell::Powershell => "Reload-AutomexiaAliases",
                AliasShell::Bash | AliasShell::Zsh | AliasShell::Fish => {
                    "automexia_aliases_reload"
                }
                AliasShell::Cmd => "automexia_aliases_reload",
            };
            println!(
                "Run `{command}` in the target shell. If unavailable, open a new shell; this command does not inject into another session."
            );
            Ok(())
        }
    }
}

struct MutationRequest<'a> {
    operation: &'a str,
    config_root: &'a Path,
    actions_root: &'a Path,
    aliases_root: &'a Path,
    action_id: &'a str,
    apply: bool,
    expected_revision: Option<u64>,
    expected_generation: Option<&'a str>,
    json: bool,
    override_fingerprint: Option<&'a str>,
}

fn mutate_and_publish(
    request: MutationRequest<'_>,
    mutate: impl FnOnce(
        &mut automexia_command_productivity::actions::QuickAction,
    ) -> Result<(), io::Error>,
) -> Result<(), Box<dyn std::error::Error>> {
    let MutationRequest {
        operation,
        config_root,
        actions_root,
        aliases_root,
        action_id,
        apply,
        expected_revision,
        expected_generation,
        json,
        override_fingerprint,
    } = request;
    let service = require_actions(actions_root)?;
    let current = service.snapshot();
    let mut document = current.actions().document().clone();
    let action = document
        .actions
        .iter_mut()
        .find(|action| action.id == action_id)
        .ok_or_else(|| input("action was not found"))?;
    mutate(action)?;
    document.revision = current
        .revision()
        .checked_add(1)
        .ok_or_else(|| input("source revision overflow"))?;
    let validated = validate_quick_actions(document.clone())?;
    let (temporary, store) = if apply {
        (None, AliasProjectionStore::open_or_create(aliases_root)?)
    } else {
        preview_store(aliases_root)?
    };
    if apply {
        store.recover_pending(current.actions())?;
    }
    let current_generation = generation_token(&store.doctor(Some(current.actions())));
    let mut observations = collect_local_alias_observations(&validated, config_root)?;
    reuse_authenticated_exact_overrides(&store, &validated, &mut observations)?;
    if let Some(fingerprint) = override_fingerprint {
        add_exact_override(&validated, &mut observations, action_id, fingerprint)?;
    }
    let plan = store.compile(&validated, &observations)?;
    if !apply {
        let result = print_plan(
            operation,
            false,
            &plan,
            Some(current.revision()),
            Some(current_generation.as_str()),
            None,
            json,
        );
        drop(temporary);
        return result;
    }
    let expected_revision = expected_revision
        .ok_or_else(|| input("--expected-revision is required with --apply"))?;
    if expected_revision != current.revision() {
        return Err(input("stale source revision").into());
    }
    let expectation = expectation(expected_generation)?;
    let prepared = store.prepare_transition(&plan, expectation, current.actions())?;
    let saved = match service.replace(expected_revision, document) {
        Ok(saved) => saved,
        Err(error) => {
            store.abort_prepared(prepared)?;
            return Err(error.into());
        }
    };
    if saved.revision() != plan.source_revision {
        // The durable journal intentionally remains so the next mutating
        // command can finish the all-old/all-new recovery decision.
        return Err(input("saved source does not match compiled generation").into());
    }
    let publication = store.activate_prepared(prepared)?;
    print_plan(
        operation,
        true,
        &plan,
        Some(expected_revision),
        expected_generation,
        Some(&publication.generation),
        json,
    )
}

fn reuse_authenticated_exact_overrides(
    store: &AliasProjectionStore,
    actions: &automexia_command_productivity::actions::ValidatedQuickActions,
    observations: &mut AliasObservationSet,
) -> Result<(), Box<dyn std::error::Error>> {
    for consent in store.current_exact_overrides()? {
        let still_requested = actions.document().actions.iter().any(|action| {
            action.enabled
                && action.alias_projection.as_ref().is_some_and(|alias| {
                    alias.enabled
                        && alias.override_policy == OverridePolicy::ExplicitExactOverride
                        && alias.requested_name == consent.name
                        && alias.shells.contains(&consent.shell)
                })
        });
        if !still_requested {
            continue;
        }
        let Some(observation) = observations
            .shells
            .iter_mut()
            .find(|entry| entry.shell == consent.shell)
        else {
            continue;
        };
        let owner_is_unchanged = observation.collisions.entries.iter().any(|entry| {
            entry.name == consent.name
                && entry.owner_fingerprint == consent.owner_fingerprint
        });
        if owner_is_unchanged && !observation.exact_overrides.contains(&consent) {
            observation.exact_overrides.push(consent);
        }
    }
    Ok(())
}

fn add_exact_override(
    actions: &automexia_command_productivity::actions::ValidatedQuickActions,
    observations: &mut AliasObservationSet,
    action_id: &str,
    fingerprint: &str,
) -> Result<(), io::Error> {
    if fingerprint.len() != 64
        || !fingerprint
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(input(
            "override owner fingerprint must be 64 lowercase hex characters",
        ));
    }
    let action = actions
        .document()
        .actions
        .iter()
        .find(|action| action.id == action_id)
        .ok_or_else(|| input("action was not found"))?;
    let alias = action
        .alias_projection
        .as_ref()
        .ok_or_else(|| input("action has no alias projection"))?;
    if alias.shells.len() != 1 {
        return Err(input("an exact override requires exactly one alias shell"));
    }
    let shell = alias.shells[0];
    let observation = observations
        .shells
        .iter_mut()
        .find(|entry| entry.shell == shell)
        .ok_or_else(|| input("shell observations are unavailable"))?;
    let collision = observation
        .collisions
        .entries
        .iter()
        .find(|entry| {
            entry.name == alias.requested_name && entry.owner_fingerprint == fingerprint
        })
        .ok_or_else(|| {
            input("fingerprint does not match the currently observed native owner")
        })?;
    observation.exact_overrides.push(ExactOverrideConsent {
        shell,
        name: collision.name.clone(),
        owner_fingerprint: collision.owner_fingerprint.clone(),
    });
    Ok(())
}

fn doctor_existing(
    aliases_root: &Path,
    source: Option<&QuickActionService>,
) -> AliasDoctorReport {
    let canonical = source.map(QuickActionService::snapshot);
    let canonical_revision = canonical.as_ref().map(|snapshot| snapshot.revision());
    match AliasProjectionStore::open_existing(aliases_root) {
        Ok(None) => AliasDoctorReport {
            health: AliasHealth::Uninitialized,
            generation: None,
            previous_generation: None,
            source_revision: None,
            canonical_revision,
            ready_bindings: 0,
            decisions: 0,
            error_code: None,
        },
        Ok(Some(store)) => {
            store.doctor(canonical.as_ref().map(|snapshot| snapshot.actions()))
        }
        Err(error) => alias_failure_report(error, canonical_revision),
    }
}

fn alias_failure_report(
    error: AliasError,
    canonical_revision: Option<u64>,
) -> AliasDoctorReport {
    let health = match error.code() {
        AliasErrorCode::InvalidRoot
        | AliasErrorCode::UnsafePermissions
        | AliasErrorCode::LinkRejected
        | AliasErrorCode::NotRegularFile => AliasHealth::UnsafePermissions,
        AliasErrorCode::ArtifactTampered
        | AliasErrorCode::InvalidManifest
        | AliasErrorCode::InvalidPointer => AliasHealth::TamperedArtifact,
        _ => AliasHealth::GenerationFailed,
    };
    AliasDoctorReport {
        health,
        generation: None,
        previous_generation: None,
        source_revision: None,
        canonical_revision,
        ready_bindings: 0,
        decisions: 0,
        error_code: Some(error.code().as_str()),
    }
}

fn source_failure_report(
    aliases_root: &Path,
    error: &(dyn std::error::Error + 'static),
) -> AliasDoctorReport {
    let mut report = doctor_existing(aliases_root, None);
    let (health, error_code) = error.downcast_ref::<StoreError>().map_or(
        (AliasHealth::GenerationFailed, "source-read-failed"),
        |error| {
            let health = match error.code() {
                StoreErrorCode::InvalidRoot
                | StoreErrorCode::PrivatePermissions
                | StoreErrorCode::LinkRejected
                | StoreErrorCode::NotDirectory
                | StoreErrorCode::NotRegularFile => AliasHealth::UnsafePermissions,
                StoreErrorCode::InvalidUtf8
                | StoreErrorCode::ModelRejected
                | StoreErrorCode::SourceTooLarge
                | StoreErrorCode::MemoryLimit
                | StoreErrorCode::RecoveryRequired => AliasHealth::MalformedSource,
                _ => AliasHealth::GenerationFailed,
            };
            (health, error.code().as_str())
        },
    );
    report.health = health;
    report.canonical_revision = None;
    report.error_code = Some(error_code);
    report
}
fn preview_store(
    aliases_root: &Path,
) -> Result<(Option<tempfile::TempDir>, AliasProjectionStore), Box<dyn std::error::Error>>
{
    if let Some(store) = AliasProjectionStore::open_existing(aliases_root)? {
        return Ok((None, store));
    }
    let temporary = tempfile::tempdir()?;
    let store = AliasProjectionStore::open_or_create(
        temporary.path().join("config/generated/aliases"),
    )?;
    Ok((Some(temporary), store))
}

fn open_actions_existing(
    root: &Path,
) -> Result<Option<QuickActionService>, Box<dyn std::error::Error>> {
    match fs::symlink_metadata(root) {
        Ok(_) => {
            let store = QuickActionStore::open_existing_read_only(root)?
                .ok_or_else(|| input("Quick Action source disappeared during read"))?;
            Ok(Some(QuickActionService::open_read_only(store)?))
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

fn require_actions(
    root: &Path,
) -> Result<QuickActionService, Box<dyn std::error::Error>> {
    open_actions_existing(root)?.ok_or_else(|| {
        input("Quick Action source is empty; save an action before managing aliases")
            .into()
    })
}

fn expectation(value: Option<&str>) -> Result<GenerationExpectation, io::Error> {
    match value {
        Some("empty") => Ok(GenerationExpectation::Empty),
        Some("disabled") => Ok(GenerationExpectation::Exact("disabled".to_owned())),
        Some(value)
            if value.len() == 64
                && value.bytes().all(|byte| {
                    byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)
                }) =>
        {
            Ok(GenerationExpectation::Exact(value.to_owned()))
        }
        Some(_) => Err(input(
            "expected generation must be `empty`, `disabled`, or 64 lowercase hex",
        )),
        None => Err(input("--expected-generation is required with --apply")),
    }
}

fn completion_json(health: &CompletionHealth) -> serde_json::Value {
    match health {
        CompletionHealth::Linked {
            provider,
            artifact_digest,
        } => serde_json::json!({
            "state": "linked",
            "provider": provider,
            "artifact_digest": artifact_digest,
        }),
        CompletionHealth::NativeAfterExpansion => {
            serde_json::json!({ "state": "native-after-expansion" })
        }
        CompletionHealth::Unavailable => serde_json::json!({ "state": "unavailable" }),
        CompletionHealth::Disabled => serde_json::json!({ "state": "disabled" }),
        CompletionHealth::Blocked { reason } => serde_json::json!({
            "state": "blocked",
            "reason": format!("{reason:?}"),
        }),
    }
}

fn tool_json(health: &ToolHealth) -> serde_json::Value {
    match health {
        ToolHealth::Ready {
            version,
            file_digest,
        } => serde_json::json!({
            "state": "ready",
            "version": version,
            "file_digest": file_digest,
        }),
        ToolHealth::Missing => serde_json::json!({ "state": "missing" }),
        ToolHealth::Unsupported { version } => serde_json::json!({
            "state": "unsupported",
            "version": version,
        }),
    }
}

fn decision_json(decision: &ProjectionDecision) -> serde_json::Value {
    serde_json::json!({
        "action_id": decision.action_id,
        "name": decision.requested_name,
        "state": format!("{:?}", decision.state),
        "reason": format!("{:?}", decision.reason),
        "collision": decision.collision.as_ref().map(|collision| serde_json::json!({
            "kind": format!("{:?}", collision.kind),
            "owner": collision.owner_label,
            "owner_fingerprint": collision.owner_fingerprint,
        })),
        "completion": completion_json(&decision.completion),
        "tool": decision.tool.as_ref().map(tool_json),
    })
}

fn artifact_json(artifact: &ProjectionArtifact, show_source: bool) -> serde_json::Value {
    serde_json::json!({
        "shell": shell_label(artifact.shell),
        "file": artifact.file_name,
        "schema_version": artifact.schema_version,
        "generator": artifact.generator,
        "source_revision": artifact.source_revision,
        "source_digest": artifact.source_digest,
        "artifact_digest": artifact.artifact_digest,
        "previous_artifact_digest": artifact.previous_artifact_digest,
        "activation_enabled": artifact.activation_enabled,
        "ready_bindings": artifact.bindings.iter().map(|binding| serde_json::json!({
            "action_id": binding.action_id,
            "name": binding.public_name,
            "internal_name": binding.internal_name,
            "mode": format!("{:?}", binding.mode),
            "argument_policy": format!("{:?}", binding.argument_policy),
            "completion": completion_json(&binding.completion),
            "owner_fingerprint": binding.owner_fingerprint,
            "tool": {
                "executable_id": binding.tool.executable_id,
                "version": binding.tool.version,
                "file_digest": binding.tool.file_digest,
            },
        })).collect::<Vec<_>>(),
        "decisions": artifact.decisions.iter().map(decision_json).collect::<Vec<_>>(),
        "source": show_source.then_some(artifact.content.as_str()),
    })
}

fn print_decision(decision: &ProjectionDecision) {
    println!(
        "  action={} name={} state={:?} reason={:?} completion={:?} tool={:?}",
        decision.action_id,
        decision.requested_name,
        decision.state,
        decision.reason,
        decision.completion,
        decision.tool,
    );
    if let Some(collision) = &decision.collision {
        println!(
            "    collision-kind={:?} owner={} owner-fingerprint={}",
            collision.kind, collision.owner_label, collision.owner_fingerprint
        );
    }
}

fn print_artifact(artifact: &ProjectionArtifact, show_source: bool) {
    println!(
        "shell={} file={} generator={} digest={} ready={} decisions={}",
        shell_label(artifact.shell),
        artifact.file_name,
        artifact.generator,
        artifact.artifact_digest,
        artifact.bindings.len(),
        artifact.decisions.len()
    );
    for decision in &artifact.decisions {
        print_decision(decision);
    }
    if show_source {
        println!("--- {} source ---", shell_label(artifact.shell));
        print!("{}", artifact.content);
        if !artifact.content.ends_with('\n') {
            println!();
        }
        println!("--- end {} source ---", shell_label(artifact.shell));
    }
}

fn print_preview(
    plan: &AliasProjectionPlan,
    selected_shell: Option<AliasShell>,
    show_source: bool,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let selected_shell = selected_shell.map(ShellKind::from);
    let artifacts = plan
        .artifacts
        .iter()
        .filter(|artifact| selected_shell.is_none_or(|shell| artifact.shell == shell))
        .collect::<Vec<_>>();
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "operation": "preview",
                "applied": false,
                "source_revision": plan.source_revision,
                "source_digest": plan.source_digest,
                "ready_bindings": plan.ready_bindings(),
                "decisions": plan.decisions(),
                "artifacts": artifacts
                    .iter()
                    .map(|artifact| artifact_json(artifact, show_source))
                    .collect::<Vec<_>>(),
            }))?
        );
        return Ok(());
    }

    println!(
        "operation=preview applied=false source-revision={} source-digest={} ready={} decisions={}",
        plan.source_revision,
        plan.source_digest,
        plan.ready_bindings(),
        plan.decisions()
    );
    for artifact in artifacts {
        print_artifact(artifact, show_source);
    }
    Ok(())
}
#[derive(Serialize)]
struct AliasValidation {
    shell: &'static str,
    compiler: &'static str,
    native_parser: &'static str,
}

fn test_plan(
    plan: &AliasProjectionPlan,
    selected_shell: Option<AliasShell>,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let selected_shell = selected_shell.map(ShellKind::from);
    let mut rejected = false;
    let mut validations = Vec::new();
    for artifact in plan
        .artifacts
        .iter()
        .filter(|artifact| selected_shell.is_none_or(|shell| artifact.shell == shell))
    {
        let native_parser = validate_native_syntax(artifact)?;
        rejected |= native_parser == "rejected";
        validations.push(AliasValidation {
            shell: shell_label(artifact.shell),
            compiler: "verified",
            native_parser,
        });
    }
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "valid": !rejected,
                "source_revision": plan.source_revision,
                "results": validations,
            }))?
        );
    } else {
        for validation in &validations {
            println!(
                "shell={} compiler={} native-parser={}",
                validation.shell, validation.compiler, validation.native_parser
            );
        }
    }
    if rejected {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "one or more installed native shell parsers rejected generated aliases",
        )
        .into());
    }
    Ok(())
}

fn validate_native_syntax(
    artifact: &automexia_command_productivity::actions::ProjectionArtifact,
) -> Result<&'static str, io::Error> {
    let mut file = tempfile::Builder::new()
        .prefix("automexia-alias-test-")
        .suffix(&format!("-{}", artifact.file_name))
        .tempfile()?;
    file.write_all(artifact.content.as_bytes())?;
    file.flush()?;
    let path = file.path().as_os_str();

    #[cfg(windows)]
    if matches!(
        artifact.shell,
        ShellKind::Bash | ShellKind::Zsh | ShellKind::Fish
    ) {
        return Ok("not-installed");
    }
    #[cfg(not(windows))]
    if artifact.shell == ShellKind::Cmd {
        return Ok("not-installed");
    }

    let status = match artifact.shell {
        ShellKind::Powershell => {
            let script = "$tokens=$null; $errors=$null; [System.Management.Automation.Language.Parser]::ParseFile($args[0],[ref]$tokens,[ref]$errors)>$null; if($errors.Count -ne 0){exit 1}";
            run_parser("pwsh", &["-NoLogo", "-NoProfile", "-Command", script], path)
                .or_else(|error| {
                    if error.kind() == io::ErrorKind::NotFound {
                        run_parser(
                            "powershell",
                            &["-NoLogo", "-NoProfile", "-Command", script],
                            path,
                        )
                    } else {
                        Err(error)
                    }
                })
        }
        ShellKind::Bash => run_parser("bash", &["-n"], path),
        ShellKind::Zsh => run_parser("zsh", &["-n"], path),
        ShellKind::Fish => run_parser("fish", &["-n"], path),
        ShellKind::Cmd => {
            let command = format!("doskey /macrofile=\"{}\"", file.path().display());
            Command::new("cmd")
                .args(["/d", "/q", "/c", &command])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
        }
    };
    match status {
        Ok(status) if status.success() => Ok("verified"),
        Ok(_) if artifact.shell == ShellKind::Cmd => Ok("unavailable"),
        Ok(_) => Ok("rejected"),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok("not-installed"),
        Err(error) => Err(error),
    }
}

fn run_parser(
    program: &str,
    arguments: &[&str],
    path: &std::ffi::OsStr,
) -> io::Result<std::process::ExitStatus> {
    Command::new(program)
        .args(arguments)
        .arg(path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
}

fn print_plan(
    operation: &str,
    applied: bool,
    plan: &AliasProjectionPlan,
    expected_revision: Option<u64>,
    expected_generation: Option<&str>,
    published_generation: Option<&str>,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let summary = AliasOperation {
        operation,
        applied,
        source_revision: plan.source_revision,
        source_digest: &plan.source_digest,
        expected_revision,
        expected_generation,
        published_generation,
        ready_bindings: plan.ready_bindings(),
        decisions: plan.decisions(),
        reload: if applied { "required" } else { "not-applied" },
    };
    if json {
        let mut output = serde_json::to_value(&summary)?;
        output
            .as_object_mut()
            .expect("serialized operation is an object")
            .insert(
                "artifacts".to_owned(),
                serde_json::Value::Array(
                    plan.artifacts
                        .iter()
                        .map(|artifact| artifact_json(artifact, false))
                        .collect(),
                ),
            );
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!(
            "operation={} applied={} source-revision={} source-digest={} expected-revision={} expected-generation={} published-generation={} ready={} decisions={} reload={}",
            summary.operation,
            summary.applied,
            summary.source_revision,
            summary.source_digest,
            summary
                .expected_revision
                .map_or_else(|| "none".to_owned(), |value| value.to_string()),
            summary.expected_generation.unwrap_or("none"),
            summary.published_generation.unwrap_or("none"),
            summary.ready_bindings,
            summary.decisions,
            summary.reload
        );
        for artifact in &plan.artifacts {
            print_artifact(artifact, false);
        }
    }
    Ok(())
}

fn generation_token(report: &AliasDoctorReport) -> String {
    match report.health {
        AliasHealth::Uninitialized => "empty".to_owned(),
        AliasHealth::Disabled => "disabled".to_owned(),
        _ => report
            .generation
            .clone()
            .unwrap_or_else(|| "empty".to_owned()),
    }
}
const fn model_argument_policy(value: AliasArgumentPolicy) -> ModelArgumentPolicy {
    match value {
        AliasArgumentPolicy::None => ModelArgumentPolicy::None,
        AliasArgumentPolicy::ForwardAll => ModelArgumentPolicy::ForwardAll,
        AliasArgumentPolicy::TypedBindings => ModelArgumentPolicy::TypedBindings,
    }
}

const fn model_completion(value: AliasCompletion) -> CompletionMode {
    match value {
        AliasCompletion::Required => CompletionMode::Required,
        AliasCompletion::BestEffort => CompletionMode::BestEffort,
        AliasCompletion::Disabled => CompletionMode::Disabled,
    }
}

fn input(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message)
}
#[cfg(test)]
mod tests {
    use super::*;
    use automexia_command_productivity::actions::{
        CollisionDetail, NativeNameKind, ProjectionDecisionState, ProjectionReason,
    };

    fn report(health: AliasHealth, generation: Option<&str>) -> AliasDoctorReport {
        AliasDoctorReport {
            health,
            generation: generation.map(str::to_owned),
            previous_generation: None,
            source_revision: None,
            canonical_revision: None,
            ready_bindings: 0,
            decisions: 0,
            error_code: None,
        }
    }

    #[test]
    fn dry_run_decision_json_exposes_exact_override_and_health_details() {
        let fingerprint = "a".repeat(64);
        let decision = ProjectionDecision {
            action_id: "user.status".to_owned(),
            requested_name: "gst".to_owned(),
            state: ProjectionDecisionState::Collision,
            reason: ProjectionReason::ExactOverrideMissing,
            collision: Some(CollisionDetail {
                kind: NativeNameKind::Application,
                owner_label: "path-command".to_owned(),
                owner_fingerprint: fingerprint.clone(),
            }),
            completion: CompletionHealth::Unavailable,
            tool: Some(ToolHealth::Missing),
        };

        let value = decision_json(&decision);
        assert_eq!(value["collision"]["owner_fingerprint"], fingerprint);
        assert_eq!(value["collision"]["owner"], "path-command");
        assert_eq!(value["completion"]["state"], "unavailable");
        assert_eq!(value["tool"]["state"], "missing");
    }

    #[test]
    fn generation_token_is_directly_reusable_for_compare_and_swap() {
        assert_eq!(
            generation_token(&report(AliasHealth::Uninitialized, None)),
            "empty"
        );
        assert_eq!(
            generation_token(&report(AliasHealth::Disabled, None)),
            "disabled"
        );
        let generation = "b".repeat(64);
        assert_eq!(
            generation_token(&report(AliasHealth::Ready, Some(&generation))),
            generation
        );
    }

    #[test]
    fn operation_json_distinguishes_expected_and_published_generations() {
        let summary = AliasOperation {
            operation: "enable",
            applied: false,
            source_revision: 8,
            source_digest: &"c".repeat(64),
            expected_revision: Some(7),
            expected_generation: Some("empty"),
            published_generation: None,
            ready_bindings: 5,
            decisions: 5,
            reload: "not-applied",
        };
        let value = serde_json::to_value(summary).unwrap();
        assert_eq!(value["expected_revision"], 7);
        assert_eq!(value["expected_generation"], "empty");
        assert!(value["published_generation"].is_null());
        assert_eq!(value["source_digest"].as_str().unwrap().len(), 64);
    }
}
