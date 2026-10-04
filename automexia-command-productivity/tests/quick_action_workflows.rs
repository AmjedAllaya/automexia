use automexia_command_productivity::actions::*;

fn step(command: &str) -> WorkflowStep {
    WorkflowStep {
        command: command.into(),
        shell: ShellKind::Bash,
        completion: WorkflowCompletion::Success,
        timeout_seconds: 30,
    }
}

fn prompt(id: u64, status: Option<(u64, i32)>) -> WorkflowObservation {
    WorkflowObservation {
        generation: 1,
        scope: 0,
        input_revision: 0,
        prompt: Some(id),
        completed: status,
        shell: Some(ShellKind::Bash),
    }
}

#[test]
fn workflow_advances_once_only_after_the_reviewed_command_succeeds() {
    let mut run =
        WorkflowRun::new(vec![step("echo one"), step("echo two")], prompt(1, None), 0)
            .unwrap();
    assert_eq!(run.poll(prompt(1, None), 0), WorkflowEffect::Submit(0));
    assert_eq!(run.poll(prompt(1, None), 1), WorkflowEffect::Wait);
    assert_eq!(
        run.poll(prompt(2, Some((1, 0))), 2),
        WorkflowEffect::Submit(1)
    );
    assert_eq!(run.poll(prompt(2, Some((1, 0))), 3), WorkflowEffect::Wait);
    assert_eq!(
        run.poll(prompt(3, Some((2, 0))), 4),
        WorkflowEffect::Finished
    );
}

#[test]
fn workflow_never_continues_after_error_user_input_reset_or_unexpected_scope() {
    for change in 0..4 {
        let mut run =
            WorkflowRun::new(vec![step("false"), step("echo next")], prompt(1, None), 0)
                .unwrap();
        assert_eq!(run.poll(prompt(1, None), 0), WorkflowEffect::Submit(0));
        let mut next = prompt(2, Some((1, 0)));
        match change {
            0 => next.completed = Some((1, 1)),
            1 => next.input_revision = 1,
            2 => next.generation = 2,
            _ => next.scope = 1,
        }
        assert!(matches!(run.poll(next, 1), WorkflowEffect::Paused(_)));
        assert!(matches!(
            run.poll(prompt(3, Some((2, 0))), 2),
            WorkflowEffect::Paused(_)
        ));
    }
}

#[test]
fn workflow_waits_for_known_remote_shell_and_has_bounded_timeout() {
    let mut ssh = step("ssh example.invalid");
    ssh.completion = WorkflowCompletion::NewShell;
    let mut run =
        WorkflowRun::new(vec![ssh, step("whoami")], prompt(1, None), 0).unwrap();
    assert_eq!(run.poll(prompt(1, None), 0), WorkflowEffect::Submit(0));
    let mut remote = prompt(2, None);
    remote.scope = 1;
    remote.shell = None;
    assert_eq!(run.poll(remote, 2), WorkflowEffect::Wait);
    remote.shell = Some(ShellKind::Bash);
    assert_eq!(run.poll(remote, 3), WorkflowEffect::Submit(1));
    assert!(matches!(
        run.poll(remote, 30_004),
        WorkflowEffect::Paused(_)
    ));
}

#[test]
fn workflow_validation_rejects_unsafe_unbounded_and_unsupported_steps() {
    for bad in [
        "",
        "echo a\nrm file",
        "echo \u{1b}[31m",
        "echo \u{202e}text",
    ] {
        assert!(validate_workflow_steps(&[step(bad)]).is_err());
    }
    assert!(validate_workflow_steps(&[]).is_err());
    assert!(
        validate_workflow_steps(&vec![step("echo ok"); MAX_WORKFLOW_STEPS + 1]).is_err()
    );
    let mut invalid = step("echo ok");
    invalid.timeout_seconds = 0;
    assert!(validate_workflow_steps(&[invalid]).is_err());
}

#[test]
fn workflow_remote_transition_accounts_for_scope_generation_and_interactive_login() {
    let mut ssh = step("ssh example.invalid");
    ssh.completion = WorkflowCompletion::NewShell;
    let mut run =
        WorkflowRun::new(vec![ssh, step("whoami")], prompt(1, None), 0).unwrap();
    assert_eq!(run.poll(prompt(1, None), 0), WorkflowEffect::Submit(0));
    let mut remote = prompt(2, None);
    remote.scope = 1;
    remote.generation = 2;
    remote.input_revision = 4;
    assert!(matches!(run.poll(remote, 1), WorkflowEffect::Paused(_)));
    run.resume(remote, 2).unwrap();
    assert_eq!(run.poll(remote, 3), WorkflowEffect::Submit(1));
}

#[test]
fn workflow_unknown_status_and_failed_ssh_never_submit_following_commands() {
    let mut run =
        WorkflowRun::new(vec![step("command"), step("echo next")], prompt(1, None), 0)
            .unwrap();
    assert_eq!(run.poll(prompt(1, None), 0), WorkflowEffect::Submit(0));
    assert!(matches!(
        run.poll(prompt(2, None), 1),
        WorkflowEffect::Paused(_)
    ));
    let mut ssh = step("ssh example.invalid");
    ssh.completion = WorkflowCompletion::NewShell;
    let mut run =
        WorkflowRun::new(vec![ssh, step("whoami")], prompt(1, None), 0).unwrap();
    run.poll(prompt(1, None), 0);
    assert!(matches!(
        run.poll(prompt(2, Some((1, 255))), 1),
        WorkflowEffect::Paused(_)
    ));
}

#[test]
fn workflow_roundtrip_and_legacy_actions_keep_distinct_execution_contracts() {
    let action = QuickAction {
        id: "workflow.test".into(),
        display_name: "Test workflow".into(),
        description: String::new(),
        tags: vec![],
        scope: ActionScope::GlobalUser,
        shells: vec![ShellKind::Bash],
        template: ActionTemplate::Workflow {
            version: WORKFLOW_VERSION,
            steps: vec![step("echo ready")],
        },
        placeholders: vec![],
        working_directory_policy: WorkingDirectoryPolicy::Inherit,
        risk: RiskClass::ReadOnly,
        execution: ExecutionMode::RunWorkflow,
        provenance: ActionProvenance::User,
        enabled: true,
        alias_projection: None,
    };
    let document = QuickActionDocument {
        schema_version: QUICK_ACTION_SCHEMA_VERSION,
        revision: 1,
        actions: vec![action.clone()],
    };
    let valid = validate_quick_actions(document.clone()).unwrap();
    assert_eq!(
        parse_quick_actions(&valid.to_toml().unwrap())
            .unwrap()
            .document(),
        &document
    );
    assert!(
        expand_for_shell(&action, ShellKind::Bash, &PlaceholderBindings::default())
            .is_err()
    );
    for mutation in 0..4 {
        let mut document = document.clone();
        let action = &mut document.actions[0];
        match mutation {
            0 => action.execution = ExecutionMode::Insert,
            1 => action.scope = ActionScope::TrustedWorkspace,
            2 => action.working_directory_policy = WorkingDirectoryPolicy::WorkspaceRoot,
            _ => {
                if let ActionTemplate::Workflow { version, .. } = &mut action.template {
                    *version = 99;
                }
            }
        }
        assert!(validate_quick_actions(document).is_err());
    }
}
