//! Visual editing composes the existing validated model and asynchronous store.
use super::*;
use crate::automexia::quick_actions::ActionMutation;
use automexia_command_productivity::actions::{
    validate_quick_actions, ActionProvenance, ActionScope, ActionTemplate,
    QuickActionDocument, WorkflowCompletion, WorkflowStep, WorkingDirectoryPolicy,
    MAX_WORKFLOW_STEPS, QUICK_ACTION_SCHEMA_VERSION, WORKFLOW_VERSION,
};
use automexia_ui_model::quick_actions::QuickActionControl as Control;

#[derive(Default)]
pub(super) struct Editor {
    draft: Option<QuickAction>,
    revision: u64,
    existing: bool,
    field: Option<Field>,
    step: Option<usize>,
    pending: Option<(usize, u64)>,
    notice: String,
    confirming: bool,
}

#[derive(Clone, Copy)]
enum Field {
    Name,
    Description,
    Command,
    Timeout,
    Executable,
    Argument(usize),
    AddArgument,
}

fn fresh_id() -> String {
    static SERIAL: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    format!(
        "action-{:x}-{:x}",
        current_time_ms(),
        SERIAL.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    )
}
fn shell_label(shell: ShellKind) -> &'static str {
    match shell {
        ShellKind::Powershell => "PowerShell",
        ShellKind::Cmd => "CMD",
        ShellKind::Bash => "Bash",
        ShellKind::Zsh => "Zsh",
        ShellKind::Fish => "Fish",
    }
}
fn next_shell(shell: ShellKind) -> ShellKind {
    match shell {
        ShellKind::Powershell => ShellKind::Cmd,
        ShellKind::Cmd => ShellKind::Bash,
        ShellKind::Bash => ShellKind::Zsh,
        ShellKind::Zsh => ShellKind::Fish,
        ShellKind::Fish => ShellKind::Powershell,
    }
}
fn completion_label(value: WorkflowCompletion) -> &'static str {
    match value {
        WorkflowCompletion::Success => "Wait for successful completion",
        WorkflowCompletion::NewShell => "Wait for a new integrated shell (SSH)",
        WorkflowCompletion::Pause => "Pause after successful completion",
    }
}

impl Screen<'_> {
    pub(super) fn reopen_action_editor(&mut self) -> bool {
        if self.action_surface.editor.pending.is_some() {
            self.renderer.command_palette.enter_action_page(
                "Saving Quick Actions…".into(),
                vec![Control::new("", "Saving…", "Please wait")],
            );
            true
        } else if self.action_surface.editor.draft.is_some() {
            self.action_surface.editor.field = None;
            self.show_quick_action_editor();
            true
        } else {
            false
        }
    }

    pub(crate) fn quick_action_control(&mut self, control: &str) {
        if self.action_surface.editor.pending.is_some() {
            return;
        }
        if let Some(workflow) = self.action_surface.workflow.as_mut() {
            workflow.panel_open = false;
        }
        match control {
            "new-action" => self.new_quick_action(false),
            "new-workflow" => self.new_quick_action(true),
            "manage" => self.manage_quick_actions(),
            "save" => self.save_quick_action(false),
            "delete" => {
                self.action_surface.editor.confirming = true;
                self.renderer.command_palette.enter_action_page(
                    "Delete this saved action?".into(),
                    vec![
                        Control::new("editor", "Cancel", "Esc"),
                        Control::new(
                            "confirm-delete",
                            "Delete action",
                            "Permanent after save",
                        ),
                    ],
                );
            }
            "confirm-delete" => self.save_quick_action(true),
            "discard" => {
                self.action_surface.editor = Editor::default();
                self.open_action_center();
            }
            "cancel-edit" => self.confirm_discard_action(),
            "editor" => {
                self.action_surface.editor.confirming = false;
                self.action_surface.editor.step = None;
                self.show_quick_action_editor();
            }
            "name" => self.edit_action_field(Field::Name),
            "description" => self.edit_action_field(Field::Description),
            "command" => self.edit_action_field(Field::Command),
            "timeout" => self.edit_action_field(Field::Timeout),
            "executable" => self.edit_action_field(Field::Executable),
            "add-argument" => self.edit_action_field(Field::AddArgument),
            "duplicate" => {
                // A copy is a new record. Refresh its revision so a conflicting
                // edit can be preserved without overwriting the other window.
                self.action_surface.editor.revision = self
                    .action_surface
                    .runtime
                    .service()
                    .map_or(0, |s| s.snapshot().revision());
                self.action_surface.editor.notice.clear();
                if let Some(draft) = self.action_surface.editor.draft.as_mut() {
                    draft.id = fresh_id();
                    draft.display_name = format!("{} copy", draft.display_name);
                    draft.provenance = ActionProvenance::User;
                    draft.scope = ActionScope::GlobalUser;
                    draft.alias_projection = None;
                    self.action_surface.editor.existing = false;
                    self.action_surface.editor.step = None;
                }
                self.show_quick_action_editor();
            }
            "enabled" => {
                if let Some(draft) = self.action_surface.editor.draft.as_mut() {
                    draft.enabled = !draft.enabled;
                }
                self.show_quick_action_editor();
            }
            "risk" => {
                if let Some(draft) = self.action_surface.editor.draft.as_mut() {
                    draft.risk = match draft.risk {
                        RiskClass::ReadOnly => RiskClass::Mutating,
                        RiskClass::Mutating => RiskClass::Destructive,
                        RiskClass::Destructive => RiskClass::Privileged,
                        RiskClass::Privileged => RiskClass::ReadOnly,
                    };
                }
                self.show_quick_action_editor();
            }
            "shell" => {
                let index = self.action_surface.editor.step;
                if let Some(draft) = self.action_surface.editor.draft.as_mut() {
                    match &mut draft.template {
                        ActionTemplate::Workflow { steps, .. } => {
                            if let Some(step) = index.and_then(|i| steps.get_mut(i)) {
                                step.shell = next_shell(step.shell);
                            }
                            if let Some(step) = steps.first() {
                                draft.shells = vec![step.shell];
                            }
                        }
                        ActionTemplate::RawInsertOnly { shell, .. } => {
                            *shell = next_shell(*shell);
                            draft.shells = vec![*shell];
                        }
                        ActionTemplate::TypedArgv { .. } => {
                            draft.shells = vec![next_shell(
                                draft.shells.first().copied().unwrap_or(ShellKind::Bash),
                            )];
                        }
                    }
                }
                self.show_quick_action_editor();
            }
            "completion" => {
                if let Some(step) = self.editing_workflow_step() {
                    step.completion = match step.completion {
                        WorkflowCompletion::Success => WorkflowCompletion::NewShell,
                        WorkflowCompletion::NewShell => WorkflowCompletion::Pause,
                        WorkflowCompletion::Pause => WorkflowCompletion::Success,
                    };
                }
                self.show_quick_action_editor();
            }
            "add-step" | "remove-step" | "move-up" | "move-down" => {
                let editor = &mut self.action_surface.editor;
                if let Some(QuickAction {
                    template: ActionTemplate::Workflow { steps, .. },
                    ..
                }) = editor.draft.as_mut()
                {
                    match control {
                        "add-step" if steps.len() < MAX_WORKFLOW_STEPS => {
                            let shell = steps.last().map_or(ShellKind::Bash, |s| s.shell);
                            steps.push(WorkflowStep {
                                command: String::new(),
                                shell,
                                completion: WorkflowCompletion::Success,
                                timeout_seconds: 300,
                            });
                            editor.step = Some(steps.len() - 1);
                        }
                        "remove-step" => {
                            if let Some(index) = editor.step.filter(|i| *i < steps.len())
                            {
                                steps.remove(index);
                                editor.step = None;
                            }
                        }
                        "move-up" => {
                            if let Some(index) =
                                editor.step.filter(|i| *i > 0 && *i < steps.len())
                            {
                                steps.swap(index, index - 1);
                                editor.step = Some(index - 1);
                            }
                        }
                        "move-down" => {
                            if let Some(index) =
                                editor.step.filter(|i| i.saturating_add(1) < steps.len())
                            {
                                steps.swap(index, index + 1);
                                editor.step = Some(index + 1);
                            }
                        }
                        _ => {}
                    }
                }
                self.show_quick_action_editor();
            }
            "review-workflow" => self.review_action_workflow(),
            "progress" | "run-workflow" | "pause-workflow" | "resume-workflow"
            | "cancel-workflow" => self.workflow_control(control),
            _ => {
                if let Some(index) = control
                    .strip_prefix("inspect-workflow:")
                    .and_then(|s| s.parse::<usize>().ok())
                {
                    self.inspect_workflow_command(index);
                } else if let Some(id) = control.strip_prefix("edit:") {
                    self.load_action_editor(id);
                } else if let Some(index) = control
                    .strip_prefix("step:")
                    .and_then(|s| s.parse::<usize>().ok())
                {
                    self.action_surface.editor.step = Some(index);
                    self.show_quick_action_editor();
                } else if let Some(index) = control
                    .strip_prefix("argument:")
                    .and_then(|s| s.parse::<usize>().ok())
                {
                    self.edit_action_field(Field::Argument(index));
                } else if let Some(index) = control
                    .strip_prefix("remove-argument:")
                    .and_then(|s| s.parse::<usize>().ok())
                {
                    if let Some(QuickAction {
                        template: ActionTemplate::TypedArgv { arguments, .. },
                        ..
                    }) = self.action_surface.editor.draft.as_mut()
                    {
                        if index < arguments.len() {
                            arguments.remove(index);
                        }
                    }
                    self.show_quick_action_editor();
                }
            }
        }
        self.mark_dirty();
    }

    pub(crate) fn action_palette_shortcut(
        &mut self,
        key: &rio_window::keyboard::Key,
        modifiers: rio_window::keyboard::ModifiersState,
        repeat: bool,
    ) -> bool {
        use rio_window::keyboard::{Key, ModifiersState};
        let command = modifiers
            .intersects(ModifiersState::CONTROL | ModifiersState::SUPER)
            && !modifiers.alt_key();
        let Key::Character(value) = key else {
            return false;
        };
        if command
            && value.eq_ignore_ascii_case("a")
            && self.renderer.command_palette.is_action_text()
        {
            self.renderer.command_palette.select_action_query();
            return true;
        }
        if command
            && value.eq_ignore_ascii_case("n")
            && self.renderer.command_palette.is_action_search()
        {
            if !repeat {
                self.new_quick_action(modifiers.shift_key());
            }
            return true;
        }
        if command
            && value.eq_ignore_ascii_case("s")
            && self.renderer.command_palette.is_action_page()
            && self.action_surface.editor.draft.is_some()
        {
            if !repeat && self.action_surface.editor.pending.is_none() {
                self.save_quick_action(false);
            }
            return true;
        }
        false
    }

    fn new_quick_action(&mut self, workflow: bool) {
        let shell = self.current_action_context().shell;
        let revision = self
            .action_surface
            .runtime
            .service()
            .map_or(0, |s| s.snapshot().revision());
        self.action_surface.editor = Editor {
            revision,
            draft: Some(QuickAction {
                id: fresh_id(),
                display_name: String::new(),
                description: String::new(),
                tags: vec![],
                scope: ActionScope::GlobalUser,
                shells: vec![shell],
                template: if workflow {
                    ActionTemplate::Workflow {
                        version: WORKFLOW_VERSION,
                        steps: vec![WorkflowStep {
                            command: String::new(),
                            shell,
                            completion: WorkflowCompletion::Success,
                            timeout_seconds: 300,
                        }],
                    }
                } else {
                    ActionTemplate::RawInsertOnly {
                        shell,
                        text: String::new(),
                    }
                },
                placeholders: vec![],
                working_directory_policy: WorkingDirectoryPolicy::Inherit,
                risk: RiskClass::Mutating,
                execution: if workflow {
                    ExecutionMode::RunWorkflow
                } else {
                    ExecutionMode::Insert
                },
                provenance: ActionProvenance::User,
                enabled: true,
                alias_projection: None,
            }),
            ..Editor::default()
        };
        self.show_quick_action_editor();
    }

    fn load_action_editor(&mut self, id: &str) {
        let Some(snapshot) = self.action_surface.runtime.service().map(|s| s.snapshot())
        else {
            return;
        };
        let Some(action) = snapshot
            .actions()
            .document()
            .actions
            .iter()
            .find(|a| a.id == id)
            .cloned()
        else {
            return;
        };
        let user_owned = matches!(
            action.provenance,
            ActionProvenance::User | ActionProvenance::Imported { .. }
        );
        self.action_surface.editor = Editor {
            revision: snapshot.revision(),
            draft: Some(action),
            existing: true,
            ..Editor::default()
        };
        if !user_owned {
            self.quick_action_control("duplicate");
        } else {
            self.show_quick_action_editor();
        }
    }

    fn manage_quick_actions(&mut self) {
        let mut controls = vec![
            Control::new("new-action", "New Quick Action", "Insert without execution"),
            Control::new("new-workflow", "New workflow", "Review then run"),
        ];
        if let Some(service) = self.action_surface.runtime.service() {
            controls.extend(service.snapshot().actions().document().actions.iter().map(
                |a| {
                    Control::new(
                        format!("edit:{}", a.id),
                        a.display_name.clone(),
                        if a.enabled {
                            "Edit / duplicate"
                        } else {
                            "Disabled · Edit / duplicate"
                        },
                    )
                },
            ));
        }
        self.renderer
            .command_palette
            .enter_action_page("Manage Quick Actions".into(), controls);
    }

    fn editing_workflow_step(&mut self) -> Option<&mut WorkflowStep> {
        let editor = &mut self.action_surface.editor;
        match &mut editor.draft.as_mut()?.template {
            ActionTemplate::Workflow { steps, .. } => steps.get_mut(editor.step?),
            _ => None,
        }
    }

    fn show_quick_action_editor(&mut self) {
        let editor = &mut self.action_surface.editor;
        let Some(action) = editor.draft.as_mut() else {
            self.manage_quick_actions();
            return;
        };
        if let ActionTemplate::Workflow { steps, .. } = &action.template {
            if let Some(step) = steps.first() {
                action.shells = vec![step.shell];
            }
        }
        let mut controls = Vec::new();
        if !editor.notice.is_empty() {
            controls.push(Control::new("", &editor.notice, ""));
        }
        let title;
        if let Some(step) = editor.step.and_then(|i| match &action.template {
            ActionTemplate::Workflow { steps, .. } => steps.get(i),
            _ => None,
        }) {
            title = format!("Workflow step {}", editor.step.unwrap_or(0) + 1);
            controls.extend([
                Control::new(
                    "command",
                    "Command",
                    if step.command.is_empty() {
                        "Enter a command"
                    } else {
                        &step.command
                    },
                ),
                Control::new("shell", "Shell", shell_label(step.shell)),
                Control::new(
                    "completion",
                    "After this command",
                    completion_label(step.completion),
                ),
                Control::new(
                    "timeout",
                    "Timeout",
                    format!("{} seconds", step.timeout_seconds),
                ),
                Control::new("move-up", "Move earlier", ""),
                Control::new("move-down", "Move later", ""),
                Control::new("remove-step", "Remove step", "Draft only"),
                Control::new("editor", "Back to workflow", "Esc"),
            ]);
        } else {
            editor.step = None;
            title = if matches!(action.template, ActionTemplate::Workflow { .. }) {
                "Edit workflow"
            } else {
                "Edit Quick Action"
            }
            .into();
            controls.extend([
                Control::new(
                    "name",
                    "Name",
                    if action.display_name.is_empty() {
                        "Required"
                    } else {
                        &action.display_name
                    },
                ),
                Control::new("description", "Description", &action.description),
            ]);
            match &action.template {
                ActionTemplate::RawInsertOnly { text, shell } => controls.extend([
                    Control::new(
                        "command",
                        "Command",
                        if text.is_empty() { "Required" } else { text },
                    ),
                    Control::new("shell", "Shell", shell_label(*shell)),
                ]),
                ActionTemplate::TypedArgv {
                    executable_id,
                    arguments,
                } => {
                    controls.push(Control::new(
                        "executable",
                        "Executable",
                        executable_id,
                    ));
                    for (i, argument) in arguments.iter().enumerate() {
                        match argument {
                            automexia_command_productivity::actions::ArgumentToken::Literal { value } => controls.push(Control::new(format!("argument:{i}"), format!("Argument {}", i + 1), value)),
                            automexia_command_productivity::actions::ArgumentToken::Placeholder { name } => controls.push(Control::new("", format!("Parameter {}", i + 1), name)),
                        }
                        controls.push(Control::new(
                            format!("remove-argument:{i}"),
                            format!("Remove argument {}", i + 1),
                            "Draft only",
                        ));
                    }
                    controls.push(Control::new(
                        "add-argument",
                        "Add argument",
                        "Exact value; no shell parsing",
                    ));
                }
                ActionTemplate::Workflow { steps, .. } => {
                    for (i, step) in steps.iter().enumerate() {
                        controls.push(Control::new(
                            format!("step:{i}"),
                            format!(
                                "Step {} · {}",
                                i + 1,
                                if step.command.is_empty() {
                                    "Add command"
                                } else {
                                    &step.command
                                }
                            ),
                            shell_label(step.shell),
                        ));
                    }
                    if steps.len() < MAX_WORKFLOW_STEPS {
                        controls.push(Control::new("add-step", "Add step", "Up to 32"));
                    }
                }
            }
            controls.extend([
                Control::new("risk", "Risk", format!("{:?}", action.risk)),
                Control::new(
                    "enabled",
                    "Enabled",
                    if action.enabled { "On" } else { "Off" },
                ),
                Control::new("save", "Save", "Ctrl+S"),
                Control::new("duplicate", "Duplicate", "Creates a separate draft"),
            ]);
            if editor.existing {
                controls.push(Control::new(
                    "delete",
                    "Delete saved action…",
                    "Confirmation required",
                ));
            }
            controls.push(Control::new("cancel-edit", "Cancel editing", "Esc"));
        }
        self.renderer
            .command_palette
            .enter_action_page(title, controls);
    }

    fn edit_action_field(&mut self, field: Field) {
        let editor = &mut self.action_surface.editor;
        let Some(action) = editor.draft.as_ref() else {
            return;
        };
        let (label, value) = match field {
            Field::Name => ("Action name", action.display_name.clone()),
            Field::Description => ("Description (optional)", action.description.clone()),
            Field::Executable => match &action.template {
                ActionTemplate::TypedArgv { executable_id, .. } => {
                    ("Executable name", executable_id.clone())
                }
                _ => return,
            },
            Field::Argument(i) => match &action.template {
                ActionTemplate::TypedArgv { arguments, .. } => match arguments.get(i) {
                    Some(
                        automexia_command_productivity::actions::ArgumentToken::Literal {
                            value,
                        },
                    ) => ("Argument value", value.clone()),
                    _ => return,
                },
                _ => return,
            },
            Field::AddArgument => ("New argument value", String::new()),
            Field::Command | Field::Timeout => match &action.template {
                ActionTemplate::RawInsertOnly { text, .. } => {
                    ("Command · no passwords or tokens", text.clone())
                }
                ActionTemplate::Workflow { steps, .. } => {
                    let Some(step) = editor.step.and_then(|i| steps.get(i)) else {
                        return;
                    };
                    if matches!(field, Field::Timeout) {
                        (
                            "Timeout in seconds (1–3600)",
                            step.timeout_seconds.to_string(),
                        )
                    } else {
                        ("Command · no passwords or tokens", step.command.clone())
                    }
                }
                _ => return,
            },
        };
        editor.field = Some(field);
        self.renderer
            .command_palette
            .enter_action_text(label.into(), value);
    }

    pub(crate) fn submit_action_field(&mut self, value: String) {
        let editor = &mut self.action_surface.editor;
        if value.len() > MAX_STRING_BYTES || value.chars().any(char::is_control) {
            return;
        }
        let Some(field) = editor.field else {
            return;
        };
        let Some(action) = editor.draft.as_mut() else {
            return;
        };
        editor.notice.clear();
        match field {
            Field::Name => action.display_name = value,
            Field::Description => action.description = value,
            Field::Executable => {
                if let ActionTemplate::TypedArgv { executable_id, .. } =
                    &mut action.template
                {
                    *executable_id = value;
                }
            }
            Field::Argument(i) => {
                if let ActionTemplate::TypedArgv { arguments, .. } = &mut action.template
                {
                    if let Some(argument) = arguments.get_mut(i) {
                        *argument = automexia_command_productivity::actions::ArgumentToken::Literal { value };
                    }
                }
            }
            Field::AddArgument => {
                if let ActionTemplate::TypedArgv { arguments, .. } = &mut action.template
                {
                    if arguments.len() < automexia_command_productivity::actions::MAX_ARGUMENTS_PER_ACTION { arguments.push(automexia_command_productivity::actions::ArgumentToken::Literal { value }); }
                }
            }
            Field::Command => match &mut action.template {
                ActionTemplate::RawInsertOnly { text, .. } => *text = value,
                ActionTemplate::Workflow { steps, .. } => {
                    if let Some(step) = editor.step.and_then(|i| steps.get_mut(i)) {
                        step.command = value;
                    }
                }
                _ => {}
            },
            Field::Timeout => {
                if let Some(timeout) =
                    value.parse::<u32>().ok().filter(|n| (1..=3600).contains(n))
                {
                    if let ActionTemplate::Workflow { steps, .. } = &mut action.template {
                        if let Some(step) = editor.step.and_then(|i| steps.get_mut(i)) {
                            step.timeout_seconds = timeout;
                        }
                    }
                } else {
                    editor.notice =
                        "Timeout must be 1–3600 seconds. Previous value kept.".into();
                }
            }
        }
        editor.field = None;
        self.show_quick_action_editor();
    }

    fn save_quick_action(&mut self, delete: bool) {
        let editor = &mut self.action_surface.editor;
        let Some(action) = editor.draft.clone() else {
            return;
        };
        if !delete {
            if let Err(error) = validate_quick_actions(QuickActionDocument {
                schema_version: QUICK_ACTION_SCHEMA_VERSION,
                revision: editor.revision,
                actions: vec![action.clone()],
            }) {
                editor.notice =
                    format!("Cannot save: {}. Check the fields above.", error.code());
                self.show_quick_action_editor();
                return;
            }
        }
        let operation = if delete {
            ActionMutation::Delete(action.id)
        } else if editor.existing {
            ActionMutation::Update(action)
        } else {
            ActionMutation::Create(action)
        };
        let route = self.context_manager.current().route_id;
        let wake = self.context_manager.devops_refresh_completion(route);
        match self.action_surface.runtime.submit_mutation(
            route,
            editor.revision,
            operation,
            wake,
        ) {
            Ok(id) => {
                editor.pending = Some((route, id));
                self.renderer.command_palette.enter_action_page(
                    "Saving Quick Actions…".into(),
                    vec![Control::new("", "Saving…", "Please wait")],
                );
            }
            Err(error) => {
                editor.notice = error.into();
                self.show_quick_action_editor();
            }
        }
    }

    pub(super) fn sync_action_editor(&mut self) {
        let Some((route, id)) = self.action_surface.editor.pending else {
            return;
        };
        let Some(result) = self.action_surface.runtime.take_mutation_result(route, id)
        else {
            return;
        };
        self.action_surface.editor.pending = None;
        match result.result {
            Ok(_) => {
                self.action_surface.editor = Editor::default();
                if self.renderer.command_palette.is_action_page() {
                    self.open_action_center();
                }
            }
            Err(_) => {
                self.action_surface.editor.notice = "Save failed or another window changed the actions. Draft retained: Duplicate to save a copy, or discard and reopen.".into();
                if self.renderer.command_palette.is_action_page() {
                    self.show_quick_action_editor();
                }
            }
        }
    }

    fn confirm_discard_action(&mut self) {
        self.action_surface.editor.confirming = true;
        self.renderer.command_palette.enter_action_page(
            "Discard this draft?".into(),
            vec![
                Control::new("editor", "Keep editing", "Esc"),
                Control::new(
                    "discard",
                    "Discard changes",
                    "Saved action stays unchanged",
                ),
            ],
        );
    }

    pub(super) fn leave_action_editor(&mut self) -> bool {
        if self.action_surface.state.inspecting_workflow {
            self.review_action_workflow();
            return true;
        }
        if self.renderer.command_palette.is_action_text() {
            self.action_surface.editor.field = None;
            self.show_quick_action_editor();
            return true;
        }
        if !self.renderer.command_palette.is_action_page() {
            return false;
        }
        if self.action_surface.editor.pending.is_some() {
            return true;
        }
        if self.action_surface.editor.confirming {
            self.action_surface.editor.confirming = false;
            self.show_quick_action_editor();
            return true;
        }
        if self.action_surface.editor.step.take().is_some() {
            self.show_quick_action_editor();
        } else if self.action_surface.editor.draft.is_some() {
            self.confirm_discard_action();
        } else {
            self.open_action_center();
        }
        true
    }
}
