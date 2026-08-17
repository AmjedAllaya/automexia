use automexia_devops::actions::{
    build_trusted_task_bridge, preview_native_alias_import, trusted_workspace_layer,
    ActionProvenance, ActionScope, ActionTemplate, ArgumentToken,
    NativeAliasRejectionCode, NativeAliasSource, RiskClass, ShellKind, TaskBridgeRequest,
    TaskRunner, WorkingDirectoryPolicy, WorkspaceTrustError, WorkspaceTrustReceipt,
};

fn imported_action(
    source: NativeAliasSource,
    input: &str,
    name: &str,
) -> automexia_devops::actions::QuickAction {
    let preview = preview_native_alias_import(source, input.as_bytes()).unwrap();
    preview
        .entries
        .iter()
        .find(|entry| entry.source_name == name)
        .unwrap()
        .action
        .clone()
        .unwrap()
}

#[test]
fn bash_and_zsh_import_only_simple_fixed_token_aliases() {
    let bash = preview_native_alias_import(
        NativeAliasSource::Bash,
        b"alias gst='git status --short'\nalias deploy='kubectl get pods | tee /tmp/pods'\nalias dynamic='echo $(id)'\n",
    )
    .unwrap();
    assert_eq!(bash.importable_count(), 1);
    assert_eq!(bash.rejected_count(), 2);

    let action = bash.entries[0].action.as_ref().unwrap();
    assert_eq!(action.id, "imported.bash.gst");
    assert_eq!(action.scope, ActionScope::ShellUser);
    assert_eq!(action.shells, vec![ShellKind::Bash]);
    assert_eq!(action.risk, RiskClass::Mutating);
    assert!(action.alias_projection.is_none());
    assert!(matches!(
        action.provenance,
        ActionProvenance::Imported { .. }
    ));
    assert_eq!(
        action.template,
        ActionTemplate::TypedArgv {
            executable_id: "git".into(),
            arguments: vec![
                ArgumentToken::Literal {
                    value: "status".into()
                },
                ArgumentToken::Literal {
                    value: "--short".into()
                },
            ],
        }
    );
    assert_eq!(
        bash.entries[1].rejection,
        Some(NativeAliasRejectionCode::UnsafeShellConstruct)
    );
    assert_eq!(
        bash.entries[2].rejection,
        Some(NativeAliasRejectionCode::UnsafeShellConstruct)
    );

    let zsh = preview_native_alias_import(
        NativeAliasSource::Zsh,
        b"alias -g G='| grep'\nalias gco='git checkout'\n",
    )
    .unwrap();
    assert_eq!(zsh.importable_count(), 1);
    assert_eq!(zsh.entries[0].source_name, "G");
    assert_eq!(
        zsh.entries[0].rejection,
        Some(NativeAliasRejectionCode::UnsupportedAliasKind)
    );
    assert_eq!(
        zsh.entries[1].action.as_ref().unwrap().id,
        "imported.zsh.gco"
    );
}

#[test]
fn powershell_fish_cmd_and_git_use_their_native_inventory_contracts() {
    let powershell = imported_action(
        NativeAliasSource::Powershell,
        "#TYPE System.Management.Automation.AliasInfo\r\n\"Name\",\"Definition\",\"Description\",\"Options\"\r\n\"gci\",\"Get-ChildItem\",\"\",\"None\"\r\n",
        "gci",
    );
    assert_eq!(powershell.shells, vec![ShellKind::Powershell]);
    assert_eq!(
        powershell.template,
        ActionTemplate::TypedArgv {
            executable_id: "Get-ChildItem".into(),
            arguments: vec![],
        }
    );

    let fish = preview_native_alias_import(
        NativeAliasSource::Fish,
        b"abbr -a -- gst 'git status --short'\nabbr -a --regex '^g.*' dynamic 'echo $argv'\n",
    )
    .unwrap();
    assert_eq!(fish.importable_count(), 1);
    assert_eq!(
        fish.entries[0].action.as_ref().unwrap().shells,
        vec![ShellKind::Fish]
    );
    assert_eq!(
        fish.entries[1].rejection,
        Some(NativeAliasRejectionCode::UnsupportedAliasKind)
    );

    let cmd = preview_native_alias_import(
        NativeAliasSource::Cmd,
        b"gst=git status --short $*\nbad=echo before$Tdel important.txt\nredirect=type file$Goutput.txt\n",
    )
    .unwrap();
    assert_eq!(cmd.importable_count(), 1);
    assert_eq!(
        cmd.entries[0].action.as_ref().unwrap().shells,
        vec![ShellKind::Cmd]
    );
    assert!(cmd.entries[1..]
        .iter()
        .all(|entry| entry.rejection
            == Some(NativeAliasRejectionCode::UnsafeShellConstruct)));

    let git = preview_native_alias_import(
        NativeAliasSource::Git,
        b"alias.co=checkout\nalias.last=log -1 HEAD\nalias.shell=!sh -c 'echo unsafe'\n",
    )
    .unwrap();
    assert_eq!(git.importable_count(), 2);
    let last = git
        .entries
        .iter()
        .find(|entry| entry.source_name == "last")
        .unwrap()
        .action
        .as_ref()
        .unwrap();
    assert_eq!(last.scope, ActionScope::GlobalUser);
    assert_eq!(last.shells.len(), 5);
    assert_eq!(
        last.template,
        ActionTemplate::TypedArgv {
            executable_id: "git".into(),
            arguments: vec![
                ArgumentToken::Literal {
                    value: "log".into()
                },
                ArgumentToken::Literal { value: "-1".into() },
                ArgumentToken::Literal {
                    value: "HEAD".into()
                },
            ],
        }
    );
    assert_eq!(
        git.entries[2].rejection,
        Some(NativeAliasRejectionCode::UnsafeShellConstruct)
    );
}

#[test]
fn duplicate_secret_control_and_oversized_inventories_fail_closed() {
    let duplicates = preview_native_alias_import(
        NativeAliasSource::Bash,
        b"alias gst='git status'\nalias gst='git status --short'\n",
    )
    .unwrap();
    assert_eq!(duplicates.importable_count(), 1);
    assert_eq!(
        duplicates.entries[1].rejection,
        Some(NativeAliasRejectionCode::DuplicateName)
    );

    let secret = preview_native_alias_import(
        NativeAliasSource::Bash,
        b"alias login='provider --token=TOP-SECRET'\n",
    )
    .unwrap();
    assert_eq!(
        secret.entries[0].rejection,
        Some(NativeAliasRejectionCode::PossibleSecret)
    );

    assert!(preview_native_alias_import(
        NativeAliasSource::Bash,
        b"alias bad='echo \x1b[31m'\n"
    )
    .is_err());
    assert!(preview_native_alias_import(
        NativeAliasSource::Bash,
        &vec![b'x'; automexia_devops::actions::MAX_SOURCE_BYTES + 1],
    )
    .is_err());
}

fn bridge(runner: TaskRunner, task_name: &str) -> automexia_devops::actions::QuickAction {
    build_trusted_task_bridge(TaskBridgeRequest {
        action_id: format!("workspace.{runner:?}").to_lowercase(),
        display_name: format!("Run {task_name}"),
        description: "Reviewed workspace task".into(),
        runner,
        task_name: task_name.into(),
        workspace_identity: "a".repeat(64),
        shells: vec![
            ShellKind::Powershell,
            ShellKind::Bash,
            ShellKind::Zsh,
            ShellKind::Fish,
            ShellKind::Cmd,
        ],
        risk: RiskClass::Mutating,
    })
    .unwrap()
}

#[test]
fn trusted_task_bridges_store_exact_named_argv_without_recipe_discovery() {
    let just = bridge(TaskRunner::Just, "test-all");
    let task = bridge(TaskRunner::Task, "build:release");
    let mise = bridge(TaskRunner::Mise, "lint");

    assert_eq!(
        just.template,
        ActionTemplate::TypedArgv {
            executable_id: "just".into(),
            arguments: vec![ArgumentToken::Literal {
                value: "test-all".into()
            }],
        }
    );
    assert_eq!(
        task.template,
        ActionTemplate::TypedArgv {
            executable_id: "task".into(),
            arguments: vec![ArgumentToken::Literal {
                value: "build:release".into()
            }],
        }
    );
    assert_eq!(
        mise.template,
        ActionTemplate::TypedArgv {
            executable_id: "mise".into(),
            arguments: vec![
                ArgumentToken::Literal {
                    value: "run".into()
                },
                ArgumentToken::Literal {
                    value: "lint".into()
                },
            ],
        }
    );
    for action in [&just, &task, &mise] {
        assert_eq!(action.scope, ActionScope::TrustedWorkspace);
        assert_eq!(
            action.working_directory_policy,
            WorkingDirectoryPolicy::WorkspaceRoot
        );
        assert!(action.alias_projection.is_none());
        assert!(matches!(
            action.provenance,
            ActionProvenance::WorkspaceTask { .. }
        ));
    }
}

#[test]
fn task_bridge_names_risk_and_exact_trust_are_fail_closed() {
    let mut request = TaskBridgeRequest {
        action_id: "workspace.test".into(),
        display_name: "Test".into(),
        description: String::new(),
        runner: TaskRunner::Just,
        task_name: "--list".into(),
        workspace_identity: "a".repeat(64),
        shells: vec![ShellKind::Bash],
        risk: RiskClass::Mutating,
    };
    assert!(build_trusted_task_bridge(request.clone()).is_err());
    request.task_name = "test;rm".into();
    assert!(build_trusted_task_bridge(request.clone()).is_err());
    request.task_name = "test".into();
    request.risk = RiskClass::ReadOnly;
    assert!(build_trusted_task_bridge(request).is_err());

    let action = bridge(TaskRunner::Just, "test");
    let document = automexia_devops::actions::QuickActionDocument {
        schema_version: 1,
        revision: 7,
        actions: vec![action],
    };
    let receipt = WorkspaceTrustReceipt::for_document("a".repeat(64), &document).unwrap();
    let layer = trusted_workspace_layer(&document, Some(&receipt)).unwrap();
    assert_eq!(layer.revision, 7);

    let mut stale = receipt.clone();
    stale.source_digest = "b".repeat(64);
    assert_eq!(
        trusted_workspace_layer(&document, Some(&stale)).unwrap_err(),
        WorkspaceTrustError::DigestMismatch
    );
    assert_eq!(
        trusted_workspace_layer(&document, None).unwrap_err(),
        WorkspaceTrustError::NotTrusted
    );
}
