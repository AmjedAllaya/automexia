//! Application-owned Settings transactions and inventory publication.
use super::*;
use crate::automexia::{runtime, settings_extensions};
use crate::settings_catalog;
use automexia_ui_model::settings::SettingsError;

impl Application<'_> {
    pub(super) fn initialize_extension_inventory(&mut self) {
        let enabled = self
            .user_preferences
            .extension_feature_enabled(settings_extensions::DEVOPS_CONTEXT_STATUS_ID)
            .unwrap_or(true);
        // Desired feature state must be published before the worker can expose
        // installed membership or start optional context collection.
        if runtime::set_context_status_enabled(enabled).is_err() {
            tracing::warn!("extension settings could not be initialized");
            return;
        }
        let proxy = self.event_proxy.clone();
        let result = runtime::schedule_initialize(Box::new(move || {
            proxy.send_event(
                RioEventType::Rio(RioEvent::ExtensionInventoryChanged),
                rio_backend::event::WindowId::from(0),
            );
        }));
        if matches!(result, runtime::InventoryInitialization::Ready) {
            self.extension_inventory_changed();
        }
    }

    pub(super) fn open_settings(&mut self, window_id: rio_backend::event::WindowId) {
        let market = runtime::market_items();
        let Ok(catalog) = settings_catalog::catalog(
            self.settings_revision,
            &self.base_config,
            &self.user_preferences,
            &market,
        ) else {
            tracing::warn!("settings catalogue could not be prepared");
            return;
        };
        let Some(route) = self.router.routes.get_mut(&window_id) else {
            return;
        };
        if route.path != RoutePath::Terminal {
            return;
        }
        self.scheduler.unschedule(TimerId::new(
            Topic::SelectionScrolling,
            route.window.screen.ctx().current_route(),
        ));
        route.window.screen.open_settings_view(catalog);
        route.window.winit_window.set_cursor(CursorIcon::Default);
        route.window.winit_window.set_cursor_visible(true);
        route.request_overlay_redraw();
    }

    /// Every effective preference/configuration/inventory change invalidates
    /// pending UI edits, including changes made in another window.
    pub(super) fn refresh_settings_catalogs(&mut self) {
        self.settings_revision = self.settings_revision.saturating_add(1);
        let market = runtime::market_items();
        let catalog = settings_catalog::catalog(
            self.settings_revision,
            &self.base_config,
            &self.user_preferences,
            &market,
        );
        for route in self.router.routes.values_mut() {
            if !route.window.screen.settings_view.is_open() {
                continue;
            }
            match &catalog {
                Ok(catalog) => route.window.screen.settings_view.refresh(catalog.clone()),
                Err(_) => route.window.screen.settings_view.set_status(
                    "Settings could not be refreshed. Close and reopen Settings.",
                ),
            }
            // A font face or theme may change without changing point size.
            // Refresh invalidates measured rows as well as current values.
            route.window.screen.fit_settings_view();
            route.window.winit_window.set_cursor(CursorIcon::Default);
            route.window.winit_window.set_cursor_visible(true);
            route.request_overlay_redraw();
        }
    }

    pub(super) fn apply_settings_edit(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: rio_backend::event::WindowId,
    ) {
        let Some(edit) = self
            .router
            .routes
            .get_mut(&window_id)
            .and_then(|route| route.window.screen.settings_view.take_edit())
        else {
            return;
        };
        let result = if self.settings_revision == u64::MAX {
            Err(SettingsError::Unavailable)
        } else {
            settings_catalog::apply_edit(
                self.settings_revision,
                &self.base_config,
                &self.user_preferences,
                &runtime::market_items(),
                &edit,
            )
        };
        let candidate = match result {
            Ok(candidate) => candidate,
            Err(_) => {
                self.refresh_settings_catalogs();
                if let Some(route) = self.router.routes.get_mut(&window_id) {
                    route.window.screen.settings_view.set_status(
                        "Settings changed or this option is unavailable. Review the current values and try again.",
                    );
                    route.request_overlay_redraw();
                }
                return;
            }
        };
        let enabled = candidate
            .extension_feature_enabled(settings_extensions::DEVOPS_CONTEXT_STATUS_ID)
            .unwrap_or(true);
        if runtime::set_context_status_enabled(enabled).is_err() {
            if let Some(route) = self.router.routes.get_mut(&window_id) {
                route.window.screen.settings_view.set_status(
                    "This extension option could not be changed. The previous setting remains active.",
                );
                route.request_overlay_redraw();
            }
            return;
        }
        if matches!(
            edit.id.as_str(),
            automexia_ui_model::settings::FONT_SIZE
                | automexia_ui_model::settings::APPEARANCE_THEME
        ) {
            let font_changed = candidate.apply_to(&self.base_config).fonts.size
                != self.config.fonts.size;
            self.user_preferences = candidate;
            let revision = self.publish_user_preferences(event_loop, font_changed);
            self.settings_save_started(revision);
            return;
        }
        self.config.presentation = candidate.apply_to(&self.base_config).presentation;
        self.user_preferences = candidate;
        for route in self.router.routes.values_mut() {
            route
                .window
                .screen
                .update_presentation(self.config.presentation);
            route.request_overlay_redraw();
        }
        self.refresh_settings_catalogs();
        self.save_settings_preferences();
    }

    pub(super) fn finish_settings_input(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: rio_backend::event::WindowId,
        typing_release: bool,
    ) {
        let Some(route) = self.router.routes.get_mut(&window_id) else {
            return;
        };
        let settings_open = route.window.screen.settings_view.is_open();
        route.window.winit_window.set_cursor(CursorIcon::Default);
        route.window.winit_window.set_cursor_visible(
            settings_open
                || !(route.window.screen.renderer.custom_mouse_cursor
                    || self.config.hide_cursor_when_typing && typing_release),
        );
        route.request_overlay_redraw();
        self.apply_settings_edit(event_loop, window_id);
    }

    fn save_settings_preferences(&mut self) {
        let revision = self.preference_writer.submit(self.user_preferences.clone());
        self.settings_save_started(revision);
    }

    fn settings_save_started(&mut self, revision: u64) {
        for route in self.router.routes.values_mut() {
            if !route.window.screen.settings_view.is_open() {
                continue;
            }
            if revision == 0 {
                route.window.screen.settings_view.save_failed();
            } else {
                route.window.screen.settings_view.save_started(revision);
            }
            route.request_overlay_redraw();
        }
    }

    pub(super) fn extension_inventory_changed(&mut self) {
        let changed = settings_catalog::prune_removed_extension_features(
            &mut self.user_preferences,
            &runtime::market_items(),
            runtime::inventory_status() == runtime::InventoryStatus::Ready,
        );
        if changed {
            let desired = self
                .user_preferences
                .extension_feature_enabled(settings_extensions::DEVOPS_CONTEXT_STATUS_ID)
                .unwrap_or(true);
            if runtime::set_context_status_enabled(desired).is_err() {
                tracing::warn!("extension feature reset could not be published");
            }
        }
        // Membership affects rendering even if there was no stored override.
        for route in self.router.routes.values_mut() {
            route
                .window
                .screen
                .update_presentation(self.config.presentation);
            route.request_overlay_redraw();
        }
        self.refresh_settings_catalogs();
        if changed {
            self.save_settings_preferences();
        }
    }
}

#[cfg(test)]
mod appearance_tests {
    use super::*;
    use rio_backend::config::theme::{AdaptiveColors, AppearanceTheme};
    #[test]
    fn appearance_publication_uses_loaded_palettes_and_reset_inherits_configured_or_system_theme(
    ) {
        let mut base = rio_backend::config::Config::default();
        base.fonts.size = 18.25;
        base.fonts.family = Some("Fixture font family".into());
        base.theme = "fixture-already-loaded".into();
        let mut light = base.colors;
        light.foreground = [0.11, 0.22, 0.33, 1.0];
        let mut dark = base.colors;
        dark.foreground = [0.77, 0.88, 0.99, 1.0];
        base.adaptive_colors = Some(AdaptiveColors {
            light: Some(light),
            dark: Some(dark),
        });
        base.force_theme = Some(AppearanceTheme::Dark);
        let prefs = UserPreferences {
            font_size: Some(21.5),
            appearance_theme: Some(AppearanceTheme::Light),
            ..UserPreferences::default()
        };
        let prepared = resolve_runtime_preference_config(
            &base,
            &prefs,
            Some(rio_window::window::Theme::Dark),
        );
        assert_eq!(prepared.fonts.size, 21.5);
        assert_eq!(prepared.colors.foreground, [0.11, 0.22, 0.33, 1.0]);
        assert_eq!(prepared.theme, "fixture-already-loaded");
        assert_eq!(prepared.fonts.family, base.fonts.family);
        let reset = resolve_runtime_preference_config(
            &base,
            &UserPreferences::default(),
            Some(rio_window::window::Theme::Light),
        );
        assert_eq!(reset.fonts.size, 18.25);
        assert_eq!(
            reset.colors.foreground,
            [0.77, 0.88, 0.99, 1.0],
            "Reset must inherit forced configuration before OS appearance"
        );
        base.force_theme = None;
        let system = resolve_runtime_preference_config(
            &base,
            &UserPreferences::default(),
            Some(rio_window::window::Theme::Light),
        );
        assert_eq!(system.colors.foreground, [0.11, 0.22, 0.33, 1.0]);
        assert_eq!(base.fonts.size, 18.25);
        assert_eq!(base.theme, "fixture-already-loaded");
    }
}
