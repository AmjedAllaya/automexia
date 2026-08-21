//! Screen-side input adapter for the read-only Connection Hub.

use std::path::PathBuf;

use automexia_devops_ssh::GrantKind;
use automexia_ui_model::connection_hub::{
    HubFocus, HubKey, HubVisualPreferences, Viewport,
};
use rio_window::{
    event::{ElementState, KeyEvent},
    keyboard::{Key, NamedKey},
};

use crate::renderer::connection_hub::ConnectionHubHit;

use super::Screen;

impl Screen<'_> {
    pub fn open_connection_hub(&mut self) {
        self.renderer.command_palette.set_enabled(false);
        let opener =
            format!("terminal-route-{}", self.context_manager.current().route_id);
        self.connection_hub.open(opener);
        self.sync_connection_hub();
        self.mark_dirty();
    }

    pub fn connection_hub_is_active(&self) -> bool {
        self.connection_hub.is_active()
    }

    pub(super) fn sync_connection_hub(&mut self) {
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

    pub fn handle_connection_hub_key(&mut self, key_event: &KeyEvent) -> bool {
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

        let catalog_controls_visible = self.connection_hub.catalog_controls_visible();
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
        let visible_start = self
            .connection_hub
            .presentation(
                Viewport::new(size.width / scale, size.height / scale, 1.0),
                HubVisualPreferences::default(),
            )
            .view
            .visible_range
            .start;
        let route_id = self.context_manager.current().route_id;
        match hit {
            ConnectionHubHit::Search => self.connection_hub.focus_search(),
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
