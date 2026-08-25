//! Narrow screen-side adapter for reviewed Quick Action interaction.
//!
//! This module receives validated search results and writes only through the
//! existing clipboard or bracketed-paste paths after explicit review. It never
//! reads terminal cells, launches a process, accesses the network, or presses
//! Enter.

use std::sync::Arc;

use automexia_devops::actions::{
    environment_risk_label, ActionSearchHit, ExecutionMode, ExpandedAction,
    PlaceholderBindings, PlaceholderSensitivity, ProviderActionBinding,
    ProviderActionDecision, ProviderActionReview, QuickAction, RiskClass, SearchContext,
    ShellKind, MAX_QUERY_BYTES, MAX_STRING_BYTES,
};
use automexia_ui_model::quick_actions::{
    QuickActionListItem, QuickActionMode, QuickActionReviewView, QuickActionRisk,
};
use rio_backend::clipboard::{Clipboard, ClipboardType};

use super::Screen;

pub(crate) struct Controller {
    runtime: crate::automexia::quick_actions::QuickActionRuntime,
    provider_publisher: crate::automexia::quick_actions::ProviderActionPublisher,
    state: State,
}

impl Controller {
    pub(crate) fn new(
        runtime: crate::automexia::quick_actions::QuickActionRuntime,
    ) -> Self {
        Self {
            provider_publisher:
                crate::automexia::quick_actions::ProviderActionPublisher::new(
                    runtime.clone(),
                ),
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
    selected_provider: Option<Arc<ProviderActionBinding>>,
    provider_review: Option<ProviderActionReview>,
    bindings: PlaceholderBindings,
    placeholder_index: usize,
    expanded: Option<ExpandedAction>,
    requires_second_confirmation: bool,
    confirmation_armed: bool,
    runtime_status: Option<crate::automexia::quick_actions::QuickActionRuntimeStatus>,
    provider_publication_notice: Option<String>,
}

impl Screen<'_> {
    pub fn open_action_center(&mut self) {
        self.dismiss_suggestions(
            crate::automexia::suggestions::SuggestionInvalidation::ModalOpened,
        );
        let provider_publication_notice = self.sync_provider_actions_for_current_route();
        self.action_surface.state = State {
            provider_publication_notice,
            ..State::default()
        };
        self.renderer.command_palette.set_enabled(true);
        self.renderer
            .command_palette
            .enter_action_search(Vec::new(), String::new());
        self.submit_action_search(String::new());
        self.mark_dirty();
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
        self.action_surface.state.selected_provider = None;
        self.action_surface.state.provider_review = None;
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
        let Some((action, provider)) = self
            .action_surface
            .state
            .hits
            .iter()
            .find(|hit| hit.action.id == action_id)
            .map(|hit| {
                (
                    Arc::clone(&hit.action),
                    hit.provider.as_ref().map(Arc::clone),
                )
            })
        else {
            return;
        };
        self.action_surface.state.selected = Some(action);
        self.action_surface.state.selected_provider = provider;
        self.action_surface.state.provider_review = None;
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
        if !self.ensure_selected_provider_action_authorized()
            || !self.ensure_selected_workspace_action_authorized()
        {
            return;
        }
        let Some(expanded) = self.action_surface.state.expanded.clone() else {
            return;
        };
        if self.action_surface.state.requires_second_confirmation
            && !self.action_surface.state.confirmation_armed
        {
            self.action_surface.state.confirmation_armed = true;
            let mut view = review_view(
                &expanded,
                self.action_surface.state.selected_provider.as_deref(),
                self.action_surface.state.provider_review.as_ref(),
            );
            view.title = format!("Confirm {}", view.title);
            view.accessibility_label = final_confirmation_accessibility_label(
                &expanded.display_name,
                &view.command_context_label,
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
        let notice = self
            .action_surface
            .state
            .provider_publication_notice
            .clone()
            .unwrap_or_else(|| {
                action_notice(
                    self.action_surface.state.runtime_status,
                    self.action_surface.state.hits.is_empty(),
                )
            });
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
        if !self.ensure_selected_provider_action_authorized()
            || !self.ensure_selected_workspace_action_authorized()
        {
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
                ) || self
                    .action_surface
                    .state
                    .provider_review
                    .as_ref()
                    .is_some_and(ProviderActionReview::requires_production_confirmation);
                let view = review_view(
                    &expanded,
                    self.action_surface.state.selected_provider.as_deref(),
                    self.action_surface.state.provider_review.as_ref(),
                );
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

    fn ensure_selected_provider_action_authorized(&mut self) -> bool {
        let Some(binding) = self.action_surface.state.selected_provider.clone() else {
            self.action_surface.state.provider_review = None;
            return true;
        };
        let Some(action) = self.action_surface.state.selected.clone() else {
            return false;
        };
        let provider_publication_notice = self.sync_provider_actions_for_current_route();
        self.action_surface.state.provider_publication_notice =
            provider_publication_notice.clone();
        if let Some(reason) = provider_publication_notice {
            self.action_surface.state.provider_review = None;
            self.action_surface.state.expanded = None;
            self.action_surface.state.confirmation_armed = false;
            self.renderer
                .command_palette
                .enter_action_review(provider_unavailable_view(
                    &action,
                    &binding,
                    ProviderActionDecision::Replaced,
                    &reason,
                    false,
                ));
            return false;
        }
        let route_id = self.context_manager.current().route_id;
        let context = self.current_action_context();
        let review = self.action_surface.runtime.revalidate_provider_binding(
            route_id,
            context.session_id,
            context.capsule_revision,
            &binding,
            current_time_ms(),
        );
        let Some(review) = review else {
            self.action_surface.state.provider_review = None;
            self.action_surface.state.expanded = None;
            self.action_surface.state.confirmation_armed = false;
            self.renderer
                .command_palette
                .enter_action_review(provider_unavailable_view(
                    &action,
                    &binding,
                    ProviderActionDecision::Replaced,
                    "Provider context was revoked; refresh and review again",
                    false,
                ));
            return false;
        };
        self.action_surface.state.provider_review = Some(review.clone());
        if review.decision().can_insert() {
            return true;
        }
        self.action_surface.state.expanded = None;
        self.action_surface.state.confirmation_armed = false;
        self.renderer
            .command_palette
            .enter_action_review(provider_unavailable_view(
                &action,
                &binding,
                review.decision(),
                provider_unavailable_reason(review.decision()),
                review.requires_production_confirmation(),
            ));
        false
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

    fn sync_provider_actions_for_current_route(&mut self) -> Option<String> {
        self.connection_hub.sync();
        let publication = self.connection_hub.provider_action_publication();
        let route_id = self.context_manager.current().route_id;
        let context = self.current_action_context();
        self.action_surface
            .provider_publisher
            .sync_route(
                route_id,
                context.session_id,
                context.capsule_revision,
                publication.as_ref(),
                current_time_ms(),
            )
            .err()
            .map(provider_publication_notice)
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
            let item = match &hit.provider {
                Some(binding) => item.with_provider_context(binding.presentation_label()),
                None => item,
            };
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

fn provider_publication_notice(
    error: crate::automexia::quick_actions::ProviderActionRouteError,
) -> String {
    provider_publication_notice_for_code(error.code()).into()
}

const fn provider_publication_notice_for_code(
    code: crate::automexia::quick_actions::ProviderActionRouteErrorCode,
) -> &'static str {
    use crate::automexia::quick_actions::ProviderActionRouteErrorCode;
    match code {
        ProviderActionRouteErrorCode::InvalidRoute => {
            "Provider actions are unavailable for this pane"
        }
        ProviderActionRouteErrorCode::BindingMismatch => {
            "Provider context changed; refresh it before using provider actions"
        }
        ProviderActionRouteErrorCode::CompositionRejected => {
            "Provider actions are unavailable; review the cached provider context"
        }
        ProviderActionRouteErrorCode::RuntimeUnavailable => {
            "Provider actions are unavailable; Quick Actions needs attention"
        }
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

fn review_view(
    expanded: &ExpandedAction,
    provider: Option<&ProviderActionBinding>,
    provider_review: Option<&ProviderActionReview>,
) -> QuickActionReviewView {
    let mode = match expanded.mode {
        ExecutionMode::Insert => QuickActionMode::Insert,
        ExecutionMode::Copy => QuickActionMode::Copy,
        ExecutionMode::ExactLaunch => QuickActionMode::Unavailable,
    };
    let view = QuickActionReviewView::new(
        expanded.action_id.clone(),
        expanded.display_name.clone(),
        expanded.command.clone(),
        risk(expanded.risk),
        mode,
    );
    match (provider, provider_review) {
        (Some(binding), Some(review)) => view.with_provider_context(
            provider_context_label(binding, review.decision()),
            review.requires_production_confirmation(),
        ),
        _ => view,
    }
}

fn final_confirmation_accessibility_label(display_name: &str, context: &str) -> String {
    format!(
        "Final confirmation for {display_name}. Context: {context}. The command remains unexecuted."
    )
}

fn provider_unavailable_view(
    action: &QuickAction,
    binding: &ProviderActionBinding,
    decision: ProviderActionDecision,
    reason: &str,
    requires_production_confirmation: bool,
) -> QuickActionReviewView {
    QuickActionReviewView::new(
        action.id.clone(),
        action.display_name.clone(),
        reason.into(),
        risk(action.risk),
        QuickActionMode::Unavailable,
    )
    .with_provider_context(
        provider_context_label(binding, decision),
        requires_production_confirmation,
    )
}

fn provider_context_label(
    binding: &ProviderActionBinding,
    decision: ProviderActionDecision,
) -> String {
    format!(
        "{} · {} {} · {} · {}",
        binding.provider_label(),
        binding.target_kind(),
        binding.exact_target(),
        decision.status_label(),
        environment_risk_label(binding.environment_risk()),
    )
}

const fn provider_unavailable_reason(decision: ProviderActionDecision) -> &'static str {
    match decision {
        ProviderActionDecision::BrokerRequired => {
            "Reviewed provider broker is required; ambient context is not allowed"
        }
        ProviderActionDecision::Refreshing => {
            "Provider context is refreshing; wait for a complete snapshot"
        }
        ProviderActionDecision::Stale => {
            "Provider context is stale; refresh and review again"
        }
        ProviderActionDecision::Expired => {
            "Provider context expired; refresh and review again"
        }
        ProviderActionDecision::Offline => {
            "Provider is offline; restore connectivity and refresh"
        }
        ProviderActionDecision::Unavailable => {
            "Provider context is unavailable; refresh and review again"
        }
        ProviderActionDecision::Error => {
            "Provider refresh failed; inspect the provider status and retry"
        }
        ProviderActionDecision::Replaced => {
            "Provider context changed; refresh and review again"
        }
        ProviderActionDecision::InsertWithoutEnter => "Provider action is ready",
    }
}

fn current_time_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| u64::try_from(duration.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or(0)
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
    fn final_confirmation_keeps_exact_provider_context_accessible() {
        let label = final_confirmation_accessibility_label(
            "Show AWS identity",
            "AWS · Account 123456789012 · Current · Production",
        );
        assert!(label.contains("AWS · Account 123456789012"));
        assert!(label.contains("Production"));
        assert!(label.ends_with("The command remains unexecuted."));
    }

    #[test]
    fn provider_failure_states_are_actionable_and_never_claim_execution() {
        for decision in [
            ProviderActionDecision::BrokerRequired,
            ProviderActionDecision::Refreshing,
            ProviderActionDecision::Stale,
            ProviderActionDecision::Expired,
            ProviderActionDecision::Offline,
            ProviderActionDecision::Unavailable,
            ProviderActionDecision::Error,
            ProviderActionDecision::Replaced,
        ] {
            let reason = provider_unavailable_reason(decision);
            assert!(!reason.is_empty());
            assert!(!reason.to_ascii_lowercase().contains("executed"));
        }
        assert!(
            provider_unavailable_reason(ProviderActionDecision::BrokerRequired)
                .contains("ambient context is not allowed")
        );
    }

    #[test]
    fn provider_publication_notices_are_accessible_and_actionable() {
        use crate::automexia::quick_actions::ProviderActionRouteErrorCode;
        assert_eq!(
            provider_publication_notice_for_code(
                ProviderActionRouteErrorCode::InvalidRoute,
            ),
            "Provider actions are unavailable for this pane"
        );
        assert_eq!(
            provider_publication_notice_for_code(
                ProviderActionRouteErrorCode::BindingMismatch,
            ),
            "Provider context changed; refresh it before using provider actions"
        );
        assert_eq!(
            provider_publication_notice_for_code(
                ProviderActionRouteErrorCode::CompositionRejected,
            ),
            "Provider actions are unavailable; review the cached provider context"
        );
        assert_eq!(
            provider_publication_notice_for_code(
                ProviderActionRouteErrorCode::RuntimeUnavailable,
            ),
            "Provider actions are unavailable; Quick Actions needs attention"
        );
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
