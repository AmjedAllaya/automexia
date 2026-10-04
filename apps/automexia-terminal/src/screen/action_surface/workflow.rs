//! Explicit user-reviewed workflows share the live route, shell metadata and
//! existing PTY writer. No process, credentials or terminal output buffer here.
use super::*;
use automexia_command_productivity::actions::{
    ActionTemplate, WorkflowEffect, WorkflowObservation, WorkflowRun,
};
use automexia_ui_model::quick_actions::QuickActionControl as Control;
use rio_backend::crosswords::command_actions::WorkflowPrompt;

pub(super) struct ActiveWorkflow {
    route: usize,
    rich_text: usize,
    name: String,
    run: WorkflowRun,
    clock: std::time::Instant,
    status: &'static str,
    finished: bool,
    pub(super) panel_open: bool,
}

impl Screen<'_> {
    pub(super) fn review_action_workflow(&mut self) {
        self.action_surface.state.inspecting_workflow = false;
        let Some(action) = self.action_surface.state.selected.as_ref() else {
            return;
        };
        let ActionTemplate::Workflow { steps, .. } = &action.template else {
            return;
        };
        let mut controls = vec![Control::new(
            "",
            "Review every step before running",
            "Runs in this terminal · No credentials stored",
        )];
        for (i, step) in steps.iter().enumerate() {
            controls.push(Control::new(
                format!("inspect-workflow:{i}"),
                format!("{}. {}", i + 1, step.command),
                format!(
                    "{:?} · {:?} · {}s · Enter: full command",
                    step.shell, step.completion, step.timeout_seconds
                ),
            ));
        }
        controls.push(Control::new(
            "run-workflow",
            if self.action_surface.state.confirmation_armed {
                "Confirm Run workflow"
            } else {
                "Run workflow"
            },
            format!("{:?} · Automatic steps", action.risk),
        ));
        controls.push(Control::new(
            "manage",
            "Edit / duplicate saved workflows",
            "",
        ));
        self.renderer
            .command_palette
            .enter_action_page(format!("Review: {}", action.display_name), controls);
    }

    fn workflow_observation(&self) -> (WorkflowObservation, Option<WorkflowPrompt>) {
        let context = self.context_manager.current();
        let shell = self.current_action_context().shell;
        let terminal = context.terminal.lock();
        let known = !terminal.integration_scope_active()
            || known_remote_action_shell(
                terminal.integration_scope().map(|s| s.shell.as_str()),
            );
        let receipt = terminal.workflow_prompt();
        (
            WorkflowObservation {
                generation: terminal.workflow_generation(),
                scope: terminal.integration_scope_revision(),
                input_revision: terminal.workflow_input_revision(),
                prompt: receipt.map(|r| r.prompt),
                completed: terminal.workflow_completed(),
                shell: known.then_some(shell),
            },
            receipt,
        )
    }

    pub(super) fn inspect_workflow_command(&mut self, index: usize) {
        let Some(action) = self.action_surface.state.selected.as_ref() else {
            return;
        };
        let ActionTemplate::Workflow { steps, .. } = &action.template else {
            return;
        };
        let Some(step) = steps.get(index) else {
            return;
        };
        let mut rows = vec![Control::new(
            "review-workflow",
            "Back to workflow review",
            "Esc",
        )];
        rows.extend(
            command_review_lines(&step.command)
                .into_iter()
                .enumerate()
                .map(|(i, line)| {
                    Control::new("", line, format!("Part {} · display wrap only", i + 1))
                }),
        );
        self.action_surface.state.inspecting_workflow = true;
        self.renderer
            .command_palette
            .enter_action_page(format!("Full command · Step {}", index + 1), rows);
    }

    pub(super) fn workflow_control(&mut self, control: &str) {
        match control {
            "run-workflow" => {
                if self
                    .action_surface
                    .workflow
                    .as_ref()
                    .is_some_and(|w| !w.finished)
                {
                    self.show_workflow_progress();
                    return;
                }
                if !self.ensure_action_scope_authorized() {
                    return;
                }
                let Some(action) = self.action_surface.state.selected.clone() else {
                    return;
                };
                let current = self.action_surface.runtime.service().map(|s| s.snapshot());
                let unchanged = current.as_ref().is_some_and(|snapshot| {
                    snapshot
                        .actions()
                        .document()
                        .actions
                        .iter()
                        .any(|a| a == action.as_ref() && a.enabled)
                });
                if !unchanged {
                    self.renderer.command_palette.enter_action_page(
                        "Workflow changed".into(),
                        vec![
                            Control::new(
                                "",
                                "Reopen and review the saved workflow again.",
                                "Nothing executed",
                            ),
                            Control::new("discard", "Back to Quick Actions", ""),
                        ],
                    );
                    return;
                }
                if matches!(action.risk, RiskClass::Destructive | RiskClass::Privileged)
                    && !self.action_surface.state.confirmation_armed
                {
                    self.action_surface.state.confirmation_armed = true;
                    self.review_action_workflow();
                    return;
                }
                let ActionTemplate::Workflow { steps, .. } = &action.template else {
                    return;
                };
                let (observed, _) = self.workflow_observation();
                if observed.shell == Some(ShellKind::Cmd) {
                    self.renderer.command_palette.enter_action_page(
                        "CMD cannot verify automatic steps".into(),
                        vec![
                            Control::new(
                                "",
                                "Use an insert-only Quick Action here",
                                "Or run this workflow in an integrated shell",
                            ),
                            Control::new(
                                "manage",
                                "Edit saved actions",
                                "Nothing executed",
                            ),
                        ],
                    );
                    return;
                }
                match WorkflowRun::new(steps.clone(), observed, 0) {
                    Ok(run) => {
                        let context = self.context_manager.current();
                        self.action_surface.workflow = Some(ActiveWorkflow {
                            route: context.route_id,
                            rich_text: context.rich_text_id,
                            name: action.display_name.clone(),
                            run,
                            clock: std::time::Instant::now(),
                            status: "Starting…",
                            finished: false,
                            panel_open: false,
                        });
                        self.renderer.command_palette.set_enabled(false);
                        self.sync_action_workflow();
                    }
                    Err(error) => self.renderer.command_palette.enter_action_page(
                        "Workflow needs an integrated prompt".into(),
                        vec![
                            Control::new("", error, "Nothing executed"),
                            Control::new("run-workflow", "Try again", "Explicit Run"),
                        ],
                    ),
                }
            }
            "pause-workflow" => {
                self.invalidate_workflow_submission();
                if let Some(workflow) = self.action_surface.workflow.as_mut() {
                    workflow.run.pause(
                        "Paused by you. The active command continues in the terminal.",
                    );
                }
                self.sync_action_workflow();
                self.show_workflow_progress();
            }
            "resume-workflow" => {
                let (observed, _) = self.workflow_observation();
                if let Some(workflow) = self.action_surface.workflow.as_mut() {
                    if workflow.route != self.context_manager.current().route_id
                        || workflow.rich_text
                            != self.context_manager.current().rich_text_id
                    {
                        workflow.status = "Return to the original terminal to resume.";
                    } else if let Err(error) =
                        workflow.run.resume(observed, elapsed(workflow.clock))
                    {
                        workflow.status = error;
                    } else {
                        workflow.status = "Resuming…";
                        workflow.panel_open = false;
                        self.renderer.command_palette.set_enabled(false);
                    }
                }
                self.sync_action_workflow();
            }
            "cancel-workflow" => {
                self.invalidate_workflow_submission();
                if let Some(workflow) = self.action_surface.workflow.as_mut() {
                    workflow.finished = true;
                    workflow.status = "Cancelled. No more steps will run; the active shell command is left under your control.";
                }
                self.show_workflow_progress();
            }
            _ => self.show_workflow_progress(),
        }
    }

    pub(super) fn sync_action_workflow(&mut self) {
        if self
            .action_surface
            .workflow
            .as_ref()
            .is_none_or(|w| w.finished)
        {
            return;
        }
        if self.action_surface.workflow.as_ref().is_some_and(|w| {
            w.route != self.context_manager.current().route_id
                || w.rich_text != self.context_manager.current().rich_text_id
        }) {
            self.invalidate_workflow_submission();
        }
        let (observed, receipt) = self.workflow_observation();
        let Some(workflow) = self.action_surface.workflow.as_mut() else {
            return;
        };
        let context = self.context_manager.current();
        if context.route_id != workflow.route
            || context.rich_text_id != workflow.rich_text
        {
            workflow.run.pause(
                "Terminal focus changed. Return to the original terminal to resume.",
            );
        }
        let effect = workflow.run.poll(observed, elapsed(workflow.clock));
        let previous = workflow.status;
        match effect {
            WorkflowEffect::Submit(index) => {
                let target = context.paste_target();
                let sent = receipt.zip(workflow.run.steps().get(index)).is_some_and(
                    |(receipt, step)| {
                        self.context_manager
                            .deliver_workflow_command(target, &step.command, receipt)
                            .is_ok()
                    },
                );
                if sent {
                    workflow.status = "Running. Passwords and MFA stay interactive; keyboard input pauses subsequent steps.";
                } else {
                    workflow.run.pause(
                        "Command could not be submitted. No further steps will run.",
                    );
                    workflow.status = "Command could not be submitted.";
                }
            }
            WorkflowEffect::Wait => {}
            WorkflowEffect::Paused(reason) => workflow.status = reason,
            WorkflowEffect::Finished => {
                workflow.status = "Workflow complete.";
                workflow.finished = true;
            }
        }
        let update_panel = workflow.panel_open
            && previous != workflow.status
            && self.renderer.command_palette.is_action_page();
        if matches!(effect, WorkflowEffect::Wait | WorkflowEffect::Submit(_))
            && !workflow.finished
        {
            self.context_manager.schedule_render_on_route(100);
        }
        if update_panel {
            self.show_workflow_progress();
        }
    }

    fn show_workflow_progress(&mut self) {
        let Some(workflow) = self.action_surface.workflow.as_mut() else {
            self.renderer.command_palette.enter_action_page(
                "Workflow progress".into(),
                vec![
                    Control::new(
                        "",
                        "No workflow is running",
                        "Choose a saved workflow to review and run",
                    ),
                    Control::new("manage", "Manage Quick Actions", ""),
                ],
            );
            return;
        };
        workflow.panel_open = true;
        let mut controls = vec![Control::new("", workflow.status, "")];
        for (i, step) in workflow.run.steps().iter().enumerate() {
            controls.push(Control::new(
                "",
                format!("{}. {}", i + 1, step.command),
                if i < workflow.run.position() {
                    "Completed"
                } else if i == workflow.run.position() && !workflow.finished {
                    "Current"
                } else {
                    "Pending"
                },
            ));
        }
        if !workflow.finished {
            controls.extend([
                Control::new(
                    "pause-workflow",
                    "Pause workflow",
                    "Current command continues",
                ),
                Control::new(
                    "resume-workflow",
                    "Resume workflow",
                    "From a safe integrated prompt",
                ),
                Control::new(
                    "cancel-workflow",
                    "Cancel remaining steps",
                    "Current command stays under your control",
                ),
            ]);
        }
        self.renderer
            .command_palette
            .enter_action_page(format!("Workflow: {}", workflow.name), controls);
    }

    fn invalidate_workflow_submission(&mut self) {
        if let Some(workflow) = &self.action_surface.workflow {
            if let Some(context) = self.context_manager.get_by_route_id(workflow.route) {
                if context.rich_text_id == workflow.rich_text {
                    context.terminal.lock().note_interactive_input();
                }
            }
        }
    }
}

fn command_review_lines(command: &str) -> Vec<String> {
    use unicode_segmentation::UnicodeSegmentation;
    use unicode_width::UnicodeWidthStr;
    let mut lines = Vec::new();
    let mut line = String::new();
    let mut width = 0;
    for grapheme in command.graphemes(true) {
        let next = grapheme.width();
        if !line.is_empty() && width + next > 28 {
            lines.push(std::mem::take(&mut line));
            width = 0;
        }
        line.push_str(grapheme);
        width += next;
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}

fn elapsed(clock: std::time::Instant) -> u64 {
    clock.elapsed().as_millis().min(u128::from(u64::MAX)) as u64
}

#[cfg(test)]
mod tests {
    #[test]
    fn full_workflow_review_preserves_every_grapheme_and_space() {
        let command =
            "echo '日本語 e\u{301} 👩‍💻' --long-option=abcdefghijklmnopqrstuvwxyz  tail";
        let lines = super::command_review_lines(command);
        assert!(lines.len() > 1);
        assert_eq!(lines.concat(), command);
        assert!(lines
            .iter()
            .all(|line| unicode_width::UnicodeWidthStr::width(line.as_str()) <= 28));
    }
}
