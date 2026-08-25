//! Screen-side input adapter for the read-only Connection Hub.

use std::path::PathBuf;

use automexia_devops_ssh::GrantKind;
use automexia_extension_api::Decision;
use automexia_ui_model::connection_hub::{
    HubFocus, HubKey, HubRoute, HubVisualPreferences, Viewport,
};
use rio_backend::clipboard::{Clipboard, ClipboardType};
use rio_window::{
    event::{ElementState, KeyEvent},
    keyboard::{Key, ModifiersState, NamedKey},
};

use crate::renderer::connection_hub::ConnectionHubHit;

use super::Screen;

fn is_literal_destination_shortcut(logical_key: &Key, modifiers: ModifiersState) -> bool {
    !modifiers.control_key()
        && !modifiers.super_key()
        && !modifiers.alt_key()
        && matches!(
            logical_key,
            Key::Character(value) if value.eq_ignore_ascii_case("l")
        )
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum HubSectionShortcut {
    Connections,
    Workspaces,
    Providers,
}

fn hub_section_shortcut(
    logical_key: &Key,
    modifiers: ModifiersState,
    focus: &HubFocus,
    route: HubRoute,
) -> Option<HubSectionShortcut> {
    if matches!(focus, HubFocus::Search)
        || modifiers.control_key()
        || modifiers.super_key()
        || modifiers.alt_key()
        || !matches!(
            route,
            HubRoute::Results | HubRoute::Workspaces | HubRoute::Providers
        )
    {
        return None;
    }
    match logical_key {
        Key::Character(value) if value.eq_ignore_ascii_case("c") => {
            Some(HubSectionShortcut::Connections)
        }
        Key::Character(value) if value.eq_ignore_ascii_case("w") => {
            Some(HubSectionShortcut::Workspaces)
        }
        Key::Character(value) if value.eq_ignore_ascii_case("p") => {
            Some(HubSectionShortcut::Providers)
        }
        _ => None,
    }
}

enum ManagedApprovalAction {
    Allow(Decision),
    CopyReviewedCommand,
    Deny,
}

fn managed_approval_action(
    logical_key: &Key,
    modifiers: ModifiersState,
    focus: &HubFocus,
) -> Option<ManagedApprovalAction> {
    if modifiers.control_key() || modifiers.super_key() || modifiers.alt_key() {
        return None;
    }
    match logical_key {
        Key::Named(NamedKey::Enter)
            if matches!(focus, HubFocus::Review | HubFocus::PrimaryAction) =>
        {
            Some(ManagedApprovalAction::Allow(Decision::AllowOnce))
        }
        Key::Character(value) if value.eq_ignore_ascii_case("a") => {
            Some(ManagedApprovalAction::Allow(Decision::AllowOnce))
        }
        Key::Character(value) if value.eq_ignore_ascii_case("s") => {
            Some(ManagedApprovalAction::Allow(Decision::AllowSession))
        }
        Key::Character(value) if value.eq_ignore_ascii_case("c") => {
            Some(ManagedApprovalAction::CopyReviewedCommand)
        }
        Key::Character(value) if value.eq_ignore_ascii_case("d") => {
            Some(ManagedApprovalAction::Deny)
        }
        _ => None,
    }
}

fn managed_launch_diagnostic(
    error: &crate::context::external_tool_runner::RunnerError,
) -> &'static str {
    use crate::context::external_tool_runner::RunnerErrorCode;
    use crate::context::launch_broker::LaunchDenialCode;

    match error.code {
        RunnerErrorCode::LaunchDenied(LaunchDenialCode::PendingSecurityReview) => {
            "connection-launch-protected-review-pending"
        }
        RunnerErrorCode::LaunchDenied(LaunchDenialCode::InvalidPrincipal) => {
            "connection-launch-package-attestation-unavailable"
        }
        RunnerErrorCode::LaunchDenied(LaunchDenialCode::ExecutableUnavailable) => {
            "connection-launch-openssh-unavailable"
        }
        RunnerErrorCode::LaunchDenied(LaunchDenialCode::ExecutableIdentityChanged)
        | RunnerErrorCode::ExecutableIdentityChanged => {
            "connection-launch-executable-changed"
        }
        RunnerErrorCode::CapacityExceeded => "connection-launch-capacity-reached",
        RunnerErrorCode::SafeDefaultUnavailable => {
            "connection-launch-working-directory-unavailable"
        }
        RunnerErrorCode::InvalidRequest | RunnerErrorCode::InvalidRoute => {
            "connection-launch-review-stale"
        }
        RunnerErrorCode::NotPublished | RunnerErrorCode::StaleLease => {
            "connection-launch-publication-failed"
        }
        RunnerErrorCode::LaunchDenied(_) => "connection-launch-denied",
        RunnerErrorCode::ReviewBusy => "connection-launch-review-busy",
        RunnerErrorCode::ReviewUnavailable => "connection-launch-review-unavailable",
    }
}
fn managed_publish_diagnostic(
    error: crate::context::ManagedPublishError,
) -> &'static str {
    match error {
        crate::context::ManagedPublishError::CapacityExceeded => {
            "connection-launch-capacity-reached"
        }
        crate::context::ManagedPublishError::InvalidScope => {
            "connection-launch-review-stale"
        }
        crate::context::ManagedPublishError::PtyUnavailable => {
            "connection-launch-pty-unavailable"
        }
        crate::context::ManagedPublishError::PublicationFailed => {
            "connection-launch-publication-failed"
        }
    }
}

fn apply_openssh_review_completion(
    controller: &mut crate::automexia::connections::ConnectionHubController,
    result: Result<
        crate::automexia::connections::CurrentDirectOpenSshReview,
        crate::context::external_tool_runner::RunnerError,
    >,
) {
    match result {
        Ok(review) => {
            if !controller.install_direct_openssh_review(review) && controller.is_active()
            {
                controller
                    .report_direct_openssh_diagnostic("connection-launch-review-stale");
            }
        }
        Err(error) => {
            controller.invalidate_direct_openssh_review();
            controller
                .report_direct_openssh_diagnostic(managed_launch_diagnostic(&error));
        }
    }
}

impl Screen<'_> {
    pub fn open_connection_hub(&mut self) {
        self.renderer.command_palette.set_enabled(false);
        let opener =
            format!("terminal-route-{}", self.context_manager.current().route_id);
        self.connection_hub.open(opener);
        self.sync_connection_hub();
        self.mark_dirty();
    }
    fn attempt_managed_openssh(&mut self, decision: Decision) {
        let Some(preparation) = self.connection_hub.direct_openssh_preparation().cloned()
        else {
            self.connection_hub
                .report_direct_openssh_diagnostic("connection-launch-review-stale");
            return;
        };
        if decision == Decision::AllowSession
            && preparation.tunnel_plan().requires_strong_confirmation()
        {
            self.connection_hub.report_direct_openssh_diagnostic(
                "connection-launch-tunnel-allow-once-required",
            );
            return;
        }
        if preparation.reviewed_destination().is_err() {
            self.connection_hub
                .report_direct_openssh_diagnostic("connection-launch-review-stale");
            return;
        }
        if let Some(error) = self.external_tool_runner.managed_openssh_activation_error()
        {
            self.connection_hub
                .report_direct_openssh_diagnostic(managed_launch_diagnostic(&error));
            return;
        }

        let now_ms = crate::context::external_tool_runner::current_time_ms();
        let binding = match self.connection_hub.direct_openssh_binding(now_ms) {
            Ok(binding) => binding,
            Err(_) => {
                self.connection_hub.invalidate_direct_openssh_review();
                if self.connection_hub_review_request.is_none() {
                    let route_id = self.context_manager.current_route();
                    let wake = self.context_manager.devops_refresh_completion(route_id);
                    match self.external_tool_runner.request_openssh_review(
                        preparation,
                        now_ms,
                        wake,
                    ) {
                        Ok(request) => {
                            self.connection_hub_review_request = Some(request);
                            self.connection_hub.report_direct_openssh_diagnostic(
                                "connection-launch-review-checking",
                            );
                        }
                        Err(error) => {
                            self.connection_hub.report_direct_openssh_diagnostic(
                                managed_launch_diagnostic(&error),
                            )
                        }
                    }
                } else {
                    self.connection_hub.report_direct_openssh_diagnostic(
                        "connection-launch-review-checking",
                    );
                }
                return;
            }
        };

        let reservation = match self.context_manager.reserve_managed_route() {
            Ok(reservation) => reservation,
            Err(error) => {
                self.connection_hub
                    .report_direct_openssh_diagnostic(managed_publish_diagnostic(error));
                return;
            }
        };
        let intent = match crate::context::external_tool_runner::OpenSshLaunchIntent::new(
            reservation,
            binding,
            decision,
            now_ms,
        ) {
            Ok(intent) => intent,
            Err(error) => {
                self.connection_hub
                    .report_direct_openssh_diagnostic(managed_launch_diagnostic(&error));
                return;
            }
        };
        let guarded = match self
            .external_tool_runner
            .authorize_openssh_candidate(intent)
        {
            Ok(guarded) => guarded,
            Err(error) => {
                self.connection_hub
                    .report_direct_openssh_diagnostic(managed_launch_diagnostic(&error));
                return;
            }
        };

        let old_index = self.context_manager.current_index();
        self.resize_top_or_bottom_line();
        #[cfg(not(target_os = "macos"))]
        self.context_manager.contexts_mut()[old_index]
            .update_dimensions(&mut self.sugarloaf);
        let rich_text_id = crate::context::next_rich_text_id();
        match self.context_manager.publish_managed_context(
            reservation,
            guarded,
            self.external_tool_runner.clone(),
            rich_text_id,
        ) {
            Ok(_) => {
                self.context_manager.invalidate_topology_redo();
                let new_index = self.context_manager.current_index();
                self.context_manager.switch_context_visibility(
                    &mut self.sugarloaf,
                    old_index,
                    new_index,
                );
                self.resize_top_or_bottom_line();
                let _ = self.connection_hub.close();
                self.renderer.connection_hub.set_presentation(None);
                self.mark_dirty();
            }
            Err(error) => self
                .connection_hub
                .report_direct_openssh_diagnostic(managed_publish_diagnostic(error)),
        }
    }
    pub fn connection_hub_is_active(&self) -> bool {
        self.connection_hub.is_active()
    }

    pub(super) fn sync_connection_hub(&mut self) {
        if let Some(request) = self.connection_hub_review_request {
            if !self.connection_hub.is_active() {
                self.external_tool_runner.cancel_openssh_review(request);
                self.connection_hub_review_request = None;
            } else if let Some(result) =
                self.external_tool_runner.take_openssh_review(request)
            {
                self.connection_hub_review_request = None;
                apply_openssh_review_completion(&mut self.connection_hub, result);
            }
        }
        if !self.connection_hub.is_active() {
            self.renderer.connection_hub.set_presentation(None);
            return;
        }
        self.connection_hub.sync();
        let size = self.sugarloaf.window_size();
        let scale = self.sugarloaf.scale_factor().max(f32::EPSILON);
        let presentation = self.connection_hub.presentation(
            Viewport::new(size.width / scale, size.height / scale, 1.0),
            HubVisualPreferences {
                high_contrast: false,
                reduced_motion: true,
                reduced_transparency: false,
            },
        );
        self.renderer
            .connection_hub
            .set_presentation(Some(presentation));
    }

    pub fn handle_connection_hub_key(
        &mut self,
        key_event: &KeyEvent,
        clipboard: &mut Clipboard,
    ) -> bool {
        if !self.connection_hub.is_active() {
            return false;
        }
        if key_event.state != ElementState::Pressed {
            return true;
        }
        let modifiers = self.modifiers.state();

        if let Some(review) = self.connection_hub.pending_grant_review_request() {
            let handled = match &key_event.logical_key {
                Key::Named(NamedKey::ArrowUp) => {
                    self.connection_hub.scroll_grant_review(-1);
                    true
                }
                Key::Named(NamedKey::ArrowDown) => {
                    self.connection_hub.scroll_grant_review(1);
                    true
                }
                Key::Named(NamedKey::PageUp) => {
                    self.connection_hub.scroll_grant_review(-8);
                    true
                }
                Key::Named(NamedKey::PageDown) => {
                    self.connection_hub.scroll_grant_review(8);
                    true
                }
                Key::Named(NamedKey::Home) => {
                    self.connection_hub.jump_grant_review(false);
                    true
                }
                Key::Named(NamedKey::End) => {
                    self.connection_hub.jump_grant_review(true);
                    true
                }
                Key::Named(NamedKey::Enter) => {
                    let route_id = self.context_manager.current().route_id;
                    let wake = self.context_manager.devops_refresh_completion(route_id);
                    let _ = self.connection_hub.confirm_reviewed_scan(review, wake);
                    true
                }
                Key::Named(NamedKey::Escape) => {
                    self.connection_hub.cancel_grant_review();
                    true
                }
                _ => true,
            };
            if handled {
                self.sync_connection_hub();
                self.mark_dirty();
                return true;
            }
        }

        if self.connection_hub.metadata_review_is_pending() {
            let key = match &key_event.logical_key {
                Key::Named(NamedKey::Enter) => Some(HubKey::Enter),
                Key::Named(NamedKey::Escape) => Some(HubKey::Escape),
                _ => None,
            };
            if let Some(key) = key {
                let route_id = self.context_manager.current().route_id;
                let wake = self.context_manager.devops_refresh_completion(route_id);
                let _ = self.connection_hub.handle_key(key, wake);
            }
            self.sync_connection_hub();
            self.mark_dirty();
            return true;
        }

        if self.connection_hub.tag_editor_is_active() {
            match &key_event.logical_key {
                Key::Named(NamedKey::Escape) => {
                    self.connection_hub.cancel_tag_editor();
                }
                Key::Named(NamedKey::Enter) => {
                    let _ = self.connection_hub.finish_tag_editor();
                }
                Key::Named(NamedKey::Backspace) => {
                    self.connection_hub.backspace_tag_editor();
                }
                Key::Character(value)
                    if (!modifiers.control_key() && !modifiers.super_key())
                        || (modifiers.control_key() && modifiers.alt_key()) =>
                {
                    let _ = self.connection_hub.append_tag_editor(value);
                }
                _ => {}
            }
            self.sync_connection_hub();
            self.mark_dirty();
            return true;
        }

        if self.connection_hub.literal_destination_entry_is_active() {
            match &key_event.logical_key {
                Key::Named(NamedKey::Escape) => {
                    self.connection_hub.cancel_literal_destination_entry();
                }
                Key::Named(NamedKey::Enter) => {
                    let _ = self.connection_hub.activate_literal_destination_focus();
                }
                Key::Named(NamedKey::Tab) => {
                    self.connection_hub
                        .cycle_literal_destination_focus(modifiers.shift_key());
                }
                Key::Named(NamedKey::Backspace)
                    if matches!(
                        self.connection_hub.focus(),
                        HubFocus::LiteralDestination
                            | HubFocus::LiteralUser
                            | HubFocus::LiteralPort
                    ) =>
                {
                    self.connection_hub.backspace_literal_field();
                }
                Key::Character(value)
                    if matches!(
                        self.connection_hub.focus(),
                        HubFocus::LiteralDestination
                            | HubFocus::LiteralUser
                            | HubFocus::LiteralPort
                    ) && ((!modifiers.control_key() && !modifiers.super_key())
                        || (modifiers.control_key() && modifiers.alt_key())) =>
                {
                    let _ = self.connection_hub.append_literal_field(value);
                }
                _ => {}
            }
            self.sync_connection_hub();
            self.mark_dirty();
            return true;
        }

        let catalog_controls_visible = self.connection_hub.catalog_controls_visible();
        if self.connection_hub.direct_openssh_preparation().is_some() {
            match managed_approval_action(
                &key_event.logical_key,
                modifiers,
                &self.connection_hub.focus(),
            ) {
                Some(ManagedApprovalAction::Allow(decision)) => {
                    self.attempt_managed_openssh(decision);
                    self.sync_connection_hub();
                    self.mark_dirty();
                    return true;
                }
                Some(ManagedApprovalAction::CopyReviewedCommand) => {
                    if let Some(preparation) =
                        self.connection_hub.direct_openssh_preparation()
                    {
                        clipboard.set(
                            ClipboardType::Clipboard,
                            preparation.user_owned_command(),
                        );
                        self.connection_hub.report_direct_openssh_diagnostic(
                            "connection-trust-command-copied",
                        );
                    }
                    self.sync_connection_hub();
                    self.mark_dirty();
                    return true;
                }
                Some(ManagedApprovalAction::Deny) => {
                    let route_id = self.context_manager.current().route_id;
                    let wake = self.context_manager.devops_refresh_completion(route_id);
                    let _ = self.connection_hub.handle_key(HubKey::Escape, wake);
                    self.sync_connection_hub();
                    self.mark_dirty();
                    return true;
                }
                None => {}
            }
        }

        let focus = self.connection_hub.focus();
        if catalog_controls_visible && matches!(focus, HubFocus::Search) {
            match &key_event.logical_key {
                Key::Named(NamedKey::Backspace) => {
                    self.connection_hub.backspace_search();
                    self.sync_connection_hub();
                    self.mark_dirty();
                    return true;
                }
                Key::Character(value)
                    if (!modifiers.control_key() && !modifiers.super_key())
                        || (modifiers.control_key() && modifiers.alt_key()) =>
                {
                    let _ = self.connection_hub.append_search_text(value);
                    self.sync_connection_hub();
                    self.mark_dirty();
                    return true;
                }
                _ => {}
            }
        }
        if let Some(shortcut) = hub_section_shortcut(
            &key_event.logical_key,
            modifiers,
            &self.connection_hub.focus(),
            self.connection_hub.route(),
        ) {
            let switched = match shortcut {
                HubSectionShortcut::Connections => self.connection_hub.open_connections(),
                HubSectionShortcut::Workspaces => self.connection_hub.open_workspaces(),
                HubSectionShortcut::Providers => self.connection_hub.open_providers(),
            };
            if switched {
                self.sync_connection_hub();
                self.mark_dirty();
                return true;
            }
        }
        if self.connection_hub.can_begin_literal_destination_entry()
            && is_literal_destination_shortcut(&key_event.logical_key, modifiers)
        {
            let _ = self.connection_hub.begin_literal_destination_entry();
            self.sync_connection_hub();
            self.mark_dirty();
            return true;
        }

        if catalog_controls_visible
            && !matches!(self.connection_hub.focus(), HubFocus::Search)
            && !modifiers.control_key()
            && !modifiers.super_key()
            && !modifiers.alt_key()
        {
            let handled = match &key_event.logical_key {
                Key::Character(value) if value.eq_ignore_ascii_case("t") => {
                    let _ = self.connection_hub.begin_selected_tag_editor();
                    true
                }
                Key::Character(value) if value.eq_ignore_ascii_case("g") => {
                    self.connection_hub.cycle_grouping();
                    true
                }
                Key::Character(value) if value.eq_ignore_ascii_case("v") => {
                    self.connection_hub.toggle_favorites_filter();
                    true
                }
                Key::Character(value) if value.eq_ignore_ascii_case("r") => {
                    self.connection_hub.toggle_recent_filter();
                    true
                }
                Key::Character(value) if value.eq_ignore_ascii_case("s") => {
                    self.connection_hub.cycle_source_filter();
                    true
                }
                Key::Character(value) if value.eq_ignore_ascii_case("x") => {
                    self.connection_hub.clear_filters();
                    true
                }
                _ => false,
            };
            if handled {
                self.sync_connection_hub();
                self.mark_dirty();
                return true;
            }
        }

        let key = match &key_event.logical_key {
            Key::Named(NamedKey::Escape) => Some(HubKey::Escape),
            Key::Named(NamedKey::ArrowUp) => Some(HubKey::Up),
            Key::Named(NamedKey::ArrowDown) => Some(HubKey::Down),
            Key::Named(NamedKey::Home) => Some(HubKey::Home),
            Key::Named(NamedKey::End) => Some(HubKey::End),
            Key::Named(NamedKey::PageUp) => Some(HubKey::PageUp),
            Key::Named(NamedKey::PageDown) => Some(HubKey::PageDown),
            Key::Named(NamedKey::Enter) => Some(HubKey::Enter),
            Key::Named(NamedKey::Tab) if modifiers.shift_key() => Some(HubKey::ShiftTab),
            Key::Named(NamedKey::Tab) => Some(HubKey::Tab),
            Key::Character(value)
                if catalog_controls_visible && value.as_str() == "/" =>
            {
                Some(HubKey::Slash)
            }
            Key::Character(value)
                if catalog_controls_visible
                    && value.eq_ignore_ascii_case("f")
                    && (modifiers.control_key() || modifiers.super_key()) =>
            {
                Some(HubKey::Find)
            }
            Key::Character(value) if value.as_str() == " " => Some(HubKey::Space),
            _ => None,
        };
        if let Some(key) = key {
            let route_id = self.context_manager.current().route_id;
            let wake = self.context_manager.devops_refresh_completion(route_id);
            let _ = self.connection_hub.handle_key(key, wake);
        }
        self.sync_connection_hub();
        self.mark_dirty();
        true
    }

    pub fn handle_connection_hub_hit(&mut self, hit: ConnectionHubHit) {
        if !self.connection_hub.is_active() {
            return;
        }
        let size = self.sugarloaf.window_size();
        let scale = self.sugarloaf.scale_factor().max(f32::EPSILON);
        let presentation = self.connection_hub.presentation(
            Viewport::new(size.width / scale, size.height / scale, 1.0),
            HubVisualPreferences::default(),
        );
        let visible_start = presentation.view.visible_range.start;
        let workspace_visible_start = presentation
            .workspace_catalog
            .as_ref()
            .map_or(0, |catalog| catalog.visible_range.start);
        let provider_visible_start = presentation
            .provider_catalog
            .as_ref()
            .map_or(0, |catalog| catalog.visible_range.start);
        let route_id = self.context_manager.current().route_id;
        match hit {
            ConnectionHubHit::Search => self.connection_hub.focus_search(),
            ConnectionHubHit::OpenConnections => {
                let _ = self.connection_hub.open_connections();
            }
            ConnectionHubHit::OpenWorkspaces => {
                let _ = self.connection_hub.open_workspaces();
            }
            ConnectionHubHit::OpenProviders => {
                let _ = self.connection_hub.open_providers();
            }
            ConnectionHubHit::SelectProvider { visible_index } => {
                let _ = self.connection_hub.select_provider_index(
                    provider_visible_start.saturating_add(visible_index),
                );
            }
            ConnectionHubHit::ReviewProvider => {
                let _ = self.connection_hub.review_selected_provider();
            }
            ConnectionHubHit::BackToProviders => {
                let _ = self.connection_hub.back_to_providers();
            }
            ConnectionHubHit::SelectWorkspace { visible_index } => {
                let _ = self.connection_hub.select_workspace_index(
                    workspace_visible_start.saturating_add(visible_index),
                );
            }
            ConnectionHubHit::ReviewWorkspace => {
                let _ = self.connection_hub.review_selected_workspace();
            }
            ConnectionHubHit::BackToWorkspaces => {
                let _ = self.connection_hub.back_to_workspaces();
            }
            ConnectionHubHit::BeginLiteralDestination => {
                let _ = self.connection_hub.begin_literal_destination_entry();
            }
            ConnectionHubHit::LiteralDestinationField => {
                self.connection_hub.focus_literal_destination();
            }
            ConnectionHubHit::LiteralUserField => {
                self.connection_hub.focus_literal_user();
            }
            ConnectionHubHit::LiteralPortField => {
                self.connection_hub.focus_literal_port();
            }
            ConnectionHubHit::ConfirmLiteralDestination => {
                let _ = self.connection_hub.confirm_literal_destination();
            }
            ConnectionHubHit::CancelLiteralDestination => {
                self.connection_hub.cancel_literal_destination_entry();
            }
            ConnectionHubHit::CycleGrouping => self.connection_hub.cycle_grouping(),
            ConnectionHubHit::ToggleFavoritesFilter => {
                self.connection_hub.toggle_favorites_filter();
            }
            ConnectionHubHit::ToggleRecentFilter => {
                self.connection_hub.toggle_recent_filter();
            }
            ConnectionHubHit::CycleSourceFilter => {
                self.connection_hub.cycle_source_filter();
            }
            ConnectionHubHit::ClearFilters => self.connection_hub.clear_filters(),
            ConnectionHubHit::BeginTagEditor => {
                let _ = self.connection_hub.begin_selected_tag_editor();
            }
            ConnectionHubHit::ConfirmMetadataChange => {
                if self.connection_hub.tag_editor_is_active() {
                    let _ = self.connection_hub.finish_tag_editor();
                } else {
                    let wake = self.context_manager.devops_refresh_completion(route_id);
                    let _ = self.connection_hub.confirm_metadata_review(wake);
                }
            }
            ConnectionHubHit::CancelOverlay => {
                if self.connection_hub.tag_editor_is_active() {
                    self.connection_hub.cancel_tag_editor();
                } else {
                    let wake = self.context_manager.devops_refresh_completion(route_id);
                    let _ = self.connection_hub.handle_key(HubKey::Escape, wake);
                }
            }
            ConnectionHubHit::SelectRow { visible_index } => self
                .connection_hub
                .select_projected_index(visible_start.saturating_add(visible_index)),
            ConnectionHubHit::ToggleFavorite { visible_index } => {
                let _ = self
                    .connection_hub
                    .toggle_favorite_at(visible_start.saturating_add(visible_index));
            }
            ConnectionHubHit::ConfirmReviewedScan { request } => {
                let wake = self.context_manager.devops_refresh_completion(route_id);
                let _ = self.connection_hub.confirm_reviewed_scan(request, wake);
            }
            ConnectionHubHit::CancelReviewedScan => {
                self.connection_hub.cancel_grant_review();
            }
            ConnectionHubHit::ApproveOnce => {
                self.attempt_managed_openssh(Decision::AllowOnce);
            }
            ConnectionHubHit::ApproveSession => {
                self.attempt_managed_openssh(Decision::AllowSession);
            }
            ConnectionHubHit::DenyManagedLaunch | ConnectionHubHit::BackToResults => {
                let wake = self.context_manager.devops_refresh_completion(route_id);
                let _ = self.connection_hub.handle_key(HubKey::Escape, wake);
            }
            ConnectionHubHit::Close => {
                self.connection_hub.close();
            }
            ConnectionHubHit::ReviewFiles | ConnectionHubHit::Inert => {}
        }
        self.sync_connection_hub();
        self.mark_dirty();
    }

    pub fn review_connection_files(&mut self, paths: Vec<PathBuf>) {
        if paths.is_empty() || !self.connection_hub.is_active() {
            return;
        }
        let route_id = self.context_manager.current().route_id;
        let wake = self.context_manager.devops_refresh_completion(route_id);
        let _ = self
            .connection_hub
            .review_exact_files(paths, GrantKind::User, wake);
        self.sync_connection_hub();
        self.mark_dirty();
    }

    pub fn connection_hub_ime_preedit(&mut self, value: Option<&str>) {
        if self.connection_hub.is_active() {
            let _ = self.connection_hub.set_ime_preedit(value);
            self.sync_connection_hub();
            self.mark_dirty();
        }
    }

    pub fn connection_hub_ime_commit(&mut self, value: &str) {
        if self.connection_hub.is_active() {
            let _ = self.connection_hub.commit_ime(value);
            self.sync_connection_hub();
            self.mark_dirty();
        }
    }

    pub fn shutdown_connection_hub(&self) {
        self.connection_hub.shutdown();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn managed_approval_mnemonics_respect_focus_and_modified_keys() {
        let none = ModifiersState::empty();
        assert!(matches!(
            managed_approval_action(
                &Key::Named(NamedKey::Enter),
                none,
                &HubFocus::Review,
            ),
            Some(ManagedApprovalAction::Allow(Decision::AllowOnce))
        ));
        assert!(matches!(
            managed_approval_action(
                &Key::Named(NamedKey::Enter),
                none,
                &HubFocus::PrimaryAction,
            ),
            Some(ManagedApprovalAction::Allow(Decision::AllowOnce))
        ));
        assert!(managed_approval_action(
            &Key::Named(NamedKey::Enter),
            none,
            &HubFocus::Back,
        )
        .is_none());
        assert!(matches!(
            managed_approval_action(&Key::Character("a".into()), none, &HubFocus::Back,),
            Some(ManagedApprovalAction::Allow(Decision::AllowOnce))
        ));
        assert!(matches!(
            managed_approval_action(&Key::Character("S".into()), none, &HubFocus::Review,),
            Some(ManagedApprovalAction::Allow(Decision::AllowSession))
        ));
        assert!(matches!(
            managed_approval_action(&Key::Character("c".into()), none, &HubFocus::Review,),
            Some(ManagedApprovalAction::CopyReviewedCommand)
        ));
        assert!(matches!(
            managed_approval_action(&Key::Character("d".into()), none, &HubFocus::Review,),
            Some(ManagedApprovalAction::Deny)
        ));
        assert!(managed_approval_action(
            &Key::Character("a".into()),
            ModifiersState::CONTROL,
            &HubFocus::Review,
        )
        .is_none());
    }

    #[test]
    fn literal_destination_shortcut_is_mnemonic_and_never_steals_modified_keys() {
        for value in ["l", "L"] {
            assert!(is_literal_destination_shortcut(
                &Key::Character(value.into()),
                ModifiersState::empty(),
            ));
        }
        assert!(is_literal_destination_shortcut(
            &Key::Character("L".into()),
            ModifiersState::SHIFT,
        ));
        for modifiers in [
            ModifiersState::CONTROL,
            ModifiersState::ALT,
            ModifiersState::SUPER,
            ModifiersState::CONTROL | ModifiersState::ALT,
        ] {
            assert!(!is_literal_destination_shortcut(
                &Key::Character("l".into()),
                modifiers,
            ));
        }
        assert!(!is_literal_destination_shortcut(
            &Key::Character("ll".into()),
            ModifiersState::empty(),
        ));
        assert!(!is_literal_destination_shortcut(
            &Key::Named(NamedKey::Enter),
            ModifiersState::empty(),
        ));
    }

    #[test]
    fn workspace_section_shortcuts_are_mnemonic_scope_safe_and_search_safe() {
        let none = ModifiersState::empty();
        for value in ["w", "W"] {
            assert_eq!(
                hub_section_shortcut(
                    &Key::Character(value.into()),
                    none,
                    &HubFocus::Results,
                    HubRoute::Results,
                ),
                Some(HubSectionShortcut::Workspaces)
            );
        }
        assert_eq!(
            hub_section_shortcut(
                &Key::Character("c".into()),
                none,
                &HubFocus::WorkspaceList,
                HubRoute::Workspaces,
            ),
            Some(HubSectionShortcut::Connections)
        );
        assert_eq!(
            hub_section_shortcut(
                &Key::Character("p".into()),
                none,
                &HubFocus::WorkspaceList,
                HubRoute::Workspaces,
            ),
            Some(HubSectionShortcut::Providers)
        );
        assert_eq!(
            hub_section_shortcut(
                &Key::Character("w".into()),
                none,
                &HubFocus::ProviderList,
                HubRoute::Providers,
            ),
            Some(HubSectionShortcut::Workspaces)
        );
        assert!(hub_section_shortcut(
            &Key::Character("w".into()),
            none,
            &HubFocus::Search,
            HubRoute::Results,
        )
        .is_none());
        assert!(hub_section_shortcut(
            &Key::Character("w".into()),
            none,
            &HubFocus::Results,
            HubRoute::WorkspaceReview,
        )
        .is_none());
        for modifiers in [
            ModifiersState::CONTROL,
            ModifiersState::ALT,
            ModifiersState::SUPER,
            ModifiersState::CONTROL | ModifiersState::SHIFT,
        ] {
            assert!(hub_section_shortcut(
                &Key::Character("w".into()),
                modifiers,
                &HubFocus::Results,
                HubRoute::Results,
            )
            .is_none());
        }
        assert!(hub_section_shortcut(
            &Key::Named(NamedKey::Enter),
            none,
            &HubFocus::Results,
            HubRoute::Results,
        )
        .is_none());
    }
    #[test]
    fn completed_executable_review_is_installed_for_the_second_decision() {
        use crate::automexia::connections::{
            ConnectionHubController, ConnectionHubRuntime, CurrentDirectOpenSshReview,
        };
        use automexia_devops::connections::ResolvedExecutable;

        const NOW_MS: u64 = 1_700_000_000_000;
        let temporary = tempfile::tempdir().unwrap();
        let runtime = ConnectionHubRuntime::open_at_root(temporary.path());
        assert!(runtime.wait_for_settled(std::time::Duration::from_secs(5)));
        let mut controller = ConnectionHubController::new(runtime);
        controller.open("terminal-grid");
        assert!(controller.begin_literal_destination_entry());
        assert!(controller.append_literal_destination("host.example.invalid"));
        assert_eq!(
            controller.confirm_literal_destination(),
            crate::automexia::connections::HubControllerEffect::LiteralReviewReady
        );
        let preparation = controller.direct_openssh_preparation().unwrap().clone();
        let review = CurrentDirectOpenSshReview::new(
            preparation,
            ResolvedExecutable {
                executable_id: "ssh".into(),
                identity_digest: "e".repeat(64),
            },
            9,
            NOW_MS,
        )
        .unwrap();

        apply_openssh_review_completion(&mut controller, Ok(review));

        assert!(controller.direct_openssh_binding(NOW_MS + 1).is_ok());
    }
}
