//! Narrow screen-side adapter for reviewed Quick Action interaction.
//!
//! This module receives validated search results and writes only through the
//! existing clipboard or bracketed-paste paths after explicit review. It never
//! reads terminal cells, launches a process, accesses the network, or presses
//! Enter.

use std::sync::Arc;

use automexia_devops::actions::{
    ActionSearchHit, ExecutionMode, ExpandedAction, PlaceholderBindings,
    PlaceholderSensitivity, QuickAction, RiskClass, SearchContext, ShellKind,
    MAX_QUERY_BYTES, MAX_STRING_BYTES,
};
use automexia_ui_model::quick_actions::{
    QuickActionListItem, QuickActionMode, QuickActionReviewView, QuickActionRisk,
};
use rio_backend::clipboard::{Clipboard, ClipboardType};

use super::Screen;

pub(crate) struct Controller {
    runtime: crate::automexia::quick_actions::QuickActionRuntime,
    state: State,
}

impl Controller {
    pub(crate) fn new(
        runtime: crate::automexia::quick_actions::QuickActionRuntime,
    ) -> Self {
        Self {
            runtime,
            state: State::default(),
        }
    }
}

impl Drop for Screen<'_> {
    fn drop(&mut self) {
        for route_id in self.context_manager.route_ids() {
            self.action_surface.runtime.forget_route(route_id);
        }
    }
}

#[derive(Default)]
struct State {
    last_request: u64,
    search_query: String,
    hits: Vec<ActionSearchHit>,
    selected: Option<Arc<QuickAction>>,
    bindings: PlaceholderBindings,
    placeholder_index: usize,
    expanded: Option<ExpandedAction>,
    requires_second_confirmation: bool,
    confirmation_armed: bool,
    runtime_status: Option<crate::automexia::quick_actions::QuickActionRuntimeStatus>,
}

impl Screen<'_> {
    pub fn open_action_center(&mut self) {
        self.action_surface.state = State::default();
        self.renderer
            .command_palette
            .enter_action_search(Vec::new(), String::new());
        self.submit_action_search(String::new());
    }

    pub fn set_action_query(&mut self, query: String) {
        if query.len() > MAX_QUERY_BYTES || query.chars().any(char::is_control) {
            return;
        }
        self.renderer.command_palette.set_query(query.clone());
        self.submit_action_search(query);
    }

    pub fn leave_action_detail(&mut self) -> bool {
        if !self.renderer.command_palette.is_action_placeholder()
            && !self.renderer.command_palette.is_action_review()
        {
            return false;
        }
        self.action_surface.state.selected = None;
        self.action_surface.state.expanded = None;
        self.action_surface.state.bindings = PlaceholderBindings::default();
        self.action_surface.state.placeholder_index = 0;
        self.action_surface.state.confirmation_armed = false;
        self.renderer.command_palette.enter_action_search(
            list_items(
                &self.action_surface.state.hits,
                self.action_surface.state.runtime_status,
            ),
            self.action_surface.state.search_query.clone(),
        );
        true
    }

    pub fn begin_action_review(&mut self, action_id: &str) {
        let Some(action) = self
            .action_surface
            .state
            .hits
            .iter()
            .find(|hit| hit.action.id == action_id)
            .map(|hit| Arc::clone(&hit.action))
        else {
            return;
        };
        self.action_surface.state.selected = Some(action);
        self.action_surface.state.bindings = PlaceholderBindings::default();
        self.action_surface.state.placeholder_index = 0;
        self.action_surface.state.expanded = None;
        self.action_surface.state.confirmation_armed = false;
        self.continue_action_review();
    }

    pub fn submit_action_placeholder(&mut self, value: String) {
        let Some(action) = self.action_surface.state.selected.as_ref() else {
            return;
        };
        let Some(placeholder) = action
            .placeholders
            .get(self.action_surface.state.placeholder_index)
        else {
            return;
        };
        if value.len() > MAX_STRING_BYTES || value.chars().any(char::is_control) {
            return;
        }
        if value.is_empty() && placeholder.required && placeholder.default.is_none() {
            return;
        }
        if !value.is_empty() {
            self.action_surface
                .state
                .bindings
                .insert(placeholder.name.clone(), value);
        }
        self.action_surface.state.placeholder_index += 1;
        self.continue_action_review();
    }

    pub fn apply_reviewed_action(
        &mut self,
        choice: crate::renderer::command_palette::QuickActionReviewChoice,
        clipboard: &mut Clipboard,
    ) {
        if !self.ensure_selected_workspace_action_authorized() {
            return;
        }
        let Some(expanded) = self.action_surface.state.expanded.clone() else {
            return;
        };
        if self.action_surface.state.requires_second_confirmation
            && !self.action_surface.state.confirmation_armed
        {
            self.action_surface.state.confirmation_armed = true;
            let mut view = review_view(&expanded);
            view.title = format!("Confirm {}", view.title);
            view.accessibility_label = format!(
                "Final confirmation for {}. The command remains unexecuted.",
                expanded.display_name
            );
            self.renderer.command_palette.enter_action_review(view);
            return;
        }
        match choice {
            crate::renderer::command_palette::QuickActionReviewChoice::Insert => {
                match expanded.mode {
                    ExecutionMode::Insert => self.paste(&expanded.command, true),
                    ExecutionMode::Copy => {
                        clipboard.set(ClipboardType::Clipboard, expanded.command)
                    }
                    ExecutionMode::ExactLaunch => return,
                }
            }
            crate::renderer::command_palette::QuickActionReviewChoice::Copy => {
                clipboard.set(ClipboardType::Clipboard, expanded.command);
            }
        }
        self.renderer.command_palette.set_enabled(false);
        self.action_surface.state = State::default();
    }

    pub(super) fn sync_action_surface(&mut self) {
        if !self.renderer.command_palette.is_action_search() {
            return;
        }
        let route_id = self.context_manager.current().route_id;
        let Some(result) = self
            .action_surface
            .runtime
            .take_result(route_id, self.action_surface.state.last_request)
        else {
            return;
        };
        if result.query != self.action_surface.state.search_query {
            return;
        }
        self.action_surface.state.runtime_status = Some(result.status);
        self.action_surface.state.hits = result.hits;
        let notice = action_notice(
            self.action_surface.state.runtime_status,
            self.action_surface.state.hits.is_empty(),
        );
        self.renderer.command_palette.update_action_items(
            list_items(
                &self.action_surface.state.hits,
                self.action_surface.state.runtime_status,
            ),
            notice,
        );
    }

    fn submit_action_search(&mut self, query: String) {
        self.action_surface.state.search_query = query.clone();
        let (route_id, workspace_path) = {
            let current = self.context_manager.current();
            (
                current.route_id,
                current.renderable_content.current_directory.clone(),
            )
        };
        let context = self.current_action_context();
        let wake = self.context_manager.devops_refresh_completion(route_id);
        match self.action_surface.runtime.submit_for_workspace(
            route_id,
            query,
            context,
            workspace_path,
            wake,
        ) {
            crate::automexia::quick_actions::SearchSubmission::Queued { request_id } => {
                self.action_surface.state.last_request = request_id
            }
            crate::automexia::quick_actions::SearchSubmission::Disabled { error } => {
                tracing::warn!("Quick Action search disabled: {error}");
                self.renderer.command_palette.update_action_items(
                    Vec::new(),
                    format!("Quick Actions unavailable ({error})"),
                );
            }
        }
    }

    fn continue_action_review(&mut self) {
        if !self.ensure_selected_workspace_action_authorized() {
            return;
        }
        let Some(action) = self.action_surface.state.selected.as_ref() else {
            return;
        };
        if let Some(reason) = unavailable_before_placeholder(action) {
            let view = QuickActionReviewView::new(
                action.id.clone(),
                action.display_name.clone(),
                reason.into(),
                risk(action.risk),
                QuickActionMode::Unavailable,
            );
            self.action_surface.state.expanded = None;
            self.renderer.command_palette.enter_action_review(view);
            return;
        }
        if let Some(placeholder) = action
            .placeholders
            .get(self.action_surface.state.placeholder_index)
        {
            let suffix = if placeholder.required {
                " (required)"
            } else {
                " (optional)"
            };
            self.renderer
                .command_palette
                .enter_action_placeholder(format!("{}{}", placeholder.prompt, suffix));
            return;
        }

        let shell = self.current_action_context().shell;
        match automexia_devops::actions::expand_for_shell(
            action,
            shell,
            &self.action_surface.state.bindings,
        ) {
            Ok(expanded) => {
                self.action_surface.state.requires_second_confirmation = matches!(
                    expanded.risk,
                    RiskClass::Destructive | RiskClass::Privileged
                );
                let view = review_view(&expanded);
                self.action_surface.state.expanded = Some(expanded);
                self.renderer.command_palette.enter_action_review(view);
            }
            Err(error) => {
                let view = QuickActionReviewView::new(
                    action.id.clone(),
                    action.display_name.clone(),
                    error.to_string(),
                    risk(action.risk),
                    QuickActionMode::Unavailable,
                );
                self.action_surface.state.expanded = None;
                self.renderer.command_palette.enter_action_review(view);
            }
        }
    }

    fn ensure_selected_workspace_action_authorized(&mut self) -> bool {
        let Some(action) = self.action_surface.state.selected.clone() else {
            return true;
        };
        let (route_id, workspace_path) = {
            let current = self.context_manager.current();
            (
                current.route_id,
                current.renderable_content.current_directory.clone(),
            )
        };
        if self.action_surface.runtime.workspace_action_is_authorized(
            route_id,
            workspace_path.as_deref(),
            &action,
        ) {
            return true;
        }
        self.action_surface.state.expanded = None;
        self.action_surface.state.confirmation_armed = false;
        self.renderer
            .command_palette
            .enter_action_review(QuickActionReviewView::new(
                action.id.clone(),
                action.display_name.clone(),
                "Workspace trust changed or expired; refresh and review again".into(),
                risk(action.risk),
                QuickActionMode::Unavailable,
            ));
        false
    }

    fn current_action_context(&self) -> SearchContext {
        let current = self.context_manager.current();
        let shell_name = current
            .renderable_content
            .shell_name
            .as_deref()
            .or_else(|| current.launch_descriptor.profile_identity())
            .or_else(|| current.launch_descriptor.program())
            .unwrap_or("");
        SearchContext {
            session_id: current.environment_capsule.session_id.get(),
            capsule_revision: current.environment_capsule.revision,
            workspace_identity: None,
            workspace_trusted: false,
            shell: shell_kind(shell_name, current.launch_descriptor.wsl_distro()),
        }
    }
}

fn list_items(
    hits: &[ActionSearchHit],
    status: Option<crate::automexia::quick_actions::QuickActionRuntimeStatus>,
) -> Vec<QuickActionListItem> {
    let health = action_health(status);
    hits.iter()
        .map(|hit| {
            let item = QuickActionListItem::new(
                hit.action.id.clone(),
                hit.action.display_name.clone(),
                hit.action.description.clone(),
                hit.source.to_owned(),
                risk(hit.action.risk),
                hit.shadowed_count,
            );
            match health {
                Some(health) => item.with_health(health),
                None => item,
            }
        })
        .collect()
}

fn unavailable_before_placeholder(action: &QuickAction) -> Option<&'static str> {
    if action.execution == ExecutionMode::ExactLaunch {
        return Some("Exact launch remains disabled until the D3 broker is accepted");
    }
    action
        .placeholders
        .iter()
        .any(|placeholder| {
            placeholder.sensitivity == PlaceholderSensitivity::SecretReference
        })
        .then_some(
            "Secret references remain disabled until the secret broker is accepted",
        )
}

fn action_health(
    status: Option<crate::automexia::quick_actions::QuickActionRuntimeStatus>,
) -> Option<&'static str> {
    use crate::automexia::quick_actions::QuickActionRuntimeStatus;
    match status {
        Some(QuickActionRuntimeStatus::Recovered { .. }) => Some("Recovered"),
        Some(QuickActionRuntimeStatus::Stale { .. }) => Some("Last-known-good"),
        Some(QuickActionRuntimeStatus::Ready { .. })
        | Some(QuickActionRuntimeStatus::Disabled { .. })
        | None => None,
    }
}

fn action_notice(
    status: Option<crate::automexia::quick_actions::QuickActionRuntimeStatus>,
    empty: bool,
) -> String {
    use crate::automexia::quick_actions::QuickActionRuntimeStatus;
    match status {
        Some(QuickActionRuntimeStatus::Recovered { .. }) => {
            "Using recovered Quick Actions".into()
        }
        Some(QuickActionRuntimeStatus::Stale { error, .. }) => {
            format!("Using last-known-good Quick Actions ({})", error.as_str())
        }
        Some(QuickActionRuntimeStatus::Disabled { error }) => {
            format!("Quick Actions unavailable ({error})")
        }
        Some(QuickActionRuntimeStatus::Ready { .. }) | None if empty => {
            "No matching Quick Actions".into()
        }
        Some(QuickActionRuntimeStatus::Ready { .. }) | None => String::new(),
    }
}

const fn risk(value: RiskClass) -> QuickActionRisk {
    match value {
        RiskClass::ReadOnly => QuickActionRisk::ReadOnly,
        RiskClass::Mutating => QuickActionRisk::Mutating,
        RiskClass::Destructive => QuickActionRisk::Destructive,
        RiskClass::Privileged => QuickActionRisk::Privileged,
    }
}

fn review_view(expanded: &ExpandedAction) -> QuickActionReviewView {
    let mode = match expanded.mode {
        ExecutionMode::Insert => QuickActionMode::Insert,
        ExecutionMode::Copy => QuickActionMode::Copy,
        ExecutionMode::ExactLaunch => QuickActionMode::Unavailable,
    };
    QuickActionReviewView::new(
        expanded.action_id.clone(),
        expanded.display_name.clone(),
        expanded.command.clone(),
        risk(expanded.risk),
        mode,
    )
}

fn shell_kind(identity: &str, wsl_distro: Option<&str>) -> ShellKind {
    let normalized = identity
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(identity)
        .to_ascii_lowercase();
    let normalized = normalized.trim_end_matches(".exe");
    if normalized.contains("powershell") || normalized == "pwsh" {
        ShellKind::Powershell
    } else if normalized == "cmd" {
        ShellKind::Cmd
    } else if normalized == "zsh" {
        ShellKind::Zsh
    } else if normalized == "fish" {
        ShellKind::Fish
    } else if normalized == "bash" || wsl_distro.is_some() {
        ShellKind::Bash
    } else if cfg!(windows) {
        ShellKind::Powershell
    } else {
        ShellKind::Bash
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use automexia_devops::actions::{
        ActionProvenance, ActionScope, ActionTemplate, Placeholder, RiskClass,
        WorkingDirectoryPolicy,
    };

    fn action_for_preflight() -> QuickAction {
        QuickAction {
            id: "test.action".into(),
            display_name: "Test action".into(),
            description: String::new(),
            tags: Vec::new(),
            scope: ActionScope::GlobalUser,
            shells: vec![ShellKind::Bash],
            template: ActionTemplate::TypedArgv {
                executable_id: "printf".into(),
                arguments: Vec::new(),
            },
            placeholders: vec![Placeholder {
                name: "target".into(),
                prompt: "Target".into(),
                sensitivity: PlaceholderSensitivity::Public,
                required: true,
                default: None,
            }],
            working_directory_policy: WorkingDirectoryPolicy::Inherit,
            risk: RiskClass::ReadOnly,
            execution: ExecutionMode::Insert,
            provenance: ActionProvenance::User,
            enabled: true,
            alias_projection: None,
        }
    }

    #[test]
    fn shell_detection_covers_every_supported_editor() {
        assert_eq!(
            shell_kind(r"C:\\Program Files\\PowerShell\\7\\pwsh.exe", None),
            ShellKind::Powershell
        );
        assert_eq!(shell_kind("CMD.EXE", None), ShellKind::Cmd);
        assert_eq!(shell_kind("/usr/bin/bash", None), ShellKind::Bash);
        assert_eq!(shell_kind("/bin/zsh", None), ShellKind::Zsh);
        assert_eq!(shell_kind("fish", None), ShellKind::Fish);
        assert_eq!(shell_kind("wsl.exe", Some("Ubuntu-24.04")), ShellKind::Bash);
    }

    #[test]
    fn secret_and_exact_actions_are_rejected_before_placeholder_collection() {
        let mut action = action_for_preflight();
        action.placeholders[0].sensitivity = PlaceholderSensitivity::SecretReference;
        assert!(unavailable_before_placeholder(&action)
            .unwrap()
            .contains("Secret references"));

        action.placeholders[0].sensitivity = PlaceholderSensitivity::Public;
        action.execution = ExecutionMode::ExactLaunch;
        assert!(unavailable_before_placeholder(&action)
            .unwrap()
            .contains("Exact launch"));
    }

    #[test]
    fn status_notices_are_redacted_and_health_is_visible() {
        use crate::automexia::quick_actions::{QuickActionRuntimeStatus, StoreErrorCode};
        let stale = QuickActionRuntimeStatus::Stale {
            revision: 4,
            error: StoreErrorCode::ModelRejected,
        };
        assert_eq!(action_health(Some(stale)), Some("Last-known-good"));
        assert_eq!(
            action_notice(Some(stale), true),
            "Using last-known-good Quick Actions (model-rejected)"
        );
        assert_eq!(
            action_notice(Some(QuickActionRuntimeStatus::Ready { revision: 5 }), true),
            "No matching Quick Actions"
        );
    }
}
