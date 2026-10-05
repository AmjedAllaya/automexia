//! Application-owned Settings transactions and inventory publication.
use super::*;
use crate::automexia::preferences::UserPreferences;
use crate::automexia::{package_customizations, runtime, settings_extensions};
use crate::settings_catalog;
use crate::settings_view::CustomizationIntent;
use automexia_ui_model::settings::SettingsError;

#[derive(Clone, Copy)]
pub(super) enum PreferenceWriteKind {
    Settings,
    Package,
}

fn preference_write_allowed(
    preview: Option<&UserPreferences>,
    kind: PreferenceWriteKind,
) -> bool {
    match kind {
        PreferenceWriteKind::Settings | PreferenceWriteKind::Package => preview.is_none(),
    }
}

fn desired_devops_feature_preferences(preferences: &UserPreferences) -> (bool, bool) {
    let context_enabled = preferences
        .extension_feature_enabled(settings_extensions::DEVOPS_CONTEXT_STATUS_ID)
        .unwrap_or(true);
    let git_enabled = preferences
        .extension_feature_enabled(settings_extensions::DEVOPS_GIT_STATUS_ID)
        .unwrap_or(true);
    (context_enabled, git_enabled)
}

fn publish_devops_feature_preferences(preferences: &UserPreferences) -> bool {
    let (context_enabled, git_enabled) = desired_devops_feature_preferences(preferences);
    runtime::set_context_status_enabled(context_enabled).is_ok()
        && runtime::set_git_status_enabled(git_enabled).is_ok()
}

fn prune_inventory_preferences(
    current: &mut UserPreferences,
    restore: Option<&mut UserPreferences>,
    market: &[crate::automexia::marketplace::MarketItem],
    inventory_ready: bool,
    packages: Option<&automexia_ecosystem_runtime::CommittedSettingsSnapshot>,
) -> (bool, bool) {
    let prune = |preferences: &mut UserPreferences| {
        let mut changed = settings_catalog::prune_removed_extension_features(
            preferences,
            market,
            inventory_ready,
        );
        let mut package_changed = false;
        // Avoid cloning retained package records for an uninitialized store;
        // the shared pruning owner also protects this boundary for Restore.
        if let Some(committed) = packages.filter(|snapshot| snapshot.revision != 0) {
            let retained = package_customizations::retain_installed_overrides(
                &preferences.package_overrides,
                committed,
            );
            if retained != preferences.package_overrides {
                preferences.package_overrides = retained;
                changed = true;
                package_changed = true;
            }
        }
        (changed, package_changed)
    };
    let changed = prune(current);
    if let Some(restore) = restore {
        // Confirmed uninstall applies to both live choices and the in-memory
        // restore point. The existing preview write gate still protects disk.
        prune(restore);
    }
    changed
}

impl Application<'_> {
    fn package_pages(
        &self,
    ) -> Result<Option<package_customizations::PackageCustomizationPages>, SettingsError>
    {
        let Some(committed) = self.package_customizations.snapshot() else {
            return Ok(None);
        };
        package_customizations::PackageCustomizationPages::from_committed(
            self.settings_revision,
            &committed,
            &self.user_preferences.package_overrides,
        )
        .map(Some)
    }

    pub(super) fn initialize_extension_inventory(&mut self) {
        // Desired feature state must be published before the worker can expose
        // installed membership or start optional context collection.
        if !publish_devops_feature_preferences(&self.user_preferences) {
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

    pub(super) fn open_settings(
        &mut self,
        window_id: rio_backend::event::WindowId,
        customizations: bool,
    ) {
        self.cancel_font_edit_for_window(window_id);
        self.restore_theme_preview();
        let theme_context = self.theme_context(window_id);
        if customizations {
            self.package_customizations.request_refresh();
        }
        let market = runtime::market_items();
        let Ok(catalog) = settings_catalog::catalog_with_palette(
            self.settings_revision,
            &self.base_config,
            &self.user_preferences,
            &market,
            &self.config.colors,
        ) else {
            tracing::warn!("settings catalogue could not be prepared");
            return;
        };
        let package_pages = if customizations {
            self.package_pages()
        } else {
            Ok(None)
        };
        let package_projection_failed = package_pages.is_err();
        let package_pages = package_pages.unwrap_or(None);
        let slot_pages = customizations.then(|| {
            settings_catalog::slot_page_snapshot_with_config(
                &self.user_preferences,
                &self.config,
                &self.base_config,
            )
        });
        let package_status = self.package_customizations.status();
        let save_status = self.preference_writer.save_status();
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
        if customizations {
            route.window.screen.open_customizations_view(
                catalog,
                package_pages,
                slot_pages,
            );
            route
                .window
                .screen
                .settings_view
                .set_theme_context(theme_context.clone());
            if self.temporary_customizations.is_some() {
                route
                    .window
                    .screen
                    .settings_view
                    .set_temporary_customizations(true);
                route
                    .window
                    .screen
                    .settings_view
                    .set_status("Temporary defaults. Restore saved to return.");
            }
        } else {
            route.window.screen.open_settings_view(catalog);
            route
                .window
                .screen
                .settings_view
                .set_theme_context(theme_context);
            if self.temporary_customizations.is_some() {
                route
                    .window
                    .screen
                    .settings_view
                    .set_temporary_customizations(true);
                route.window.screen.settings_view.set_status(
                    "Temporary preview active. Restore saved in Customizations.",
                );
            }
        }
        route
            .window
            .screen
            .settings_view
            .set_package_inventory_status(package_status, package_projection_failed);
        route
            .window
            .screen
            .settings_view
            .restore_save_status(save_status);
        route.window.winit_window.set_cursor(CursorIcon::Default);
        route.window.winit_window.set_cursor_visible(true);
        route.request_overlay_redraw();
    }

    /// Every effective preference/configuration/inventory change invalidates
    /// pending UI edits, including changes made in another window.
    pub(super) fn refresh_settings_catalogs(&mut self) {
        self.settings_revision = self.settings_revision.saturating_add(1);
        let package_pages = self.package_pages();
        let package_projection_failed = package_pages.is_err();
        let package_pages = package_pages.unwrap_or(None);
        let package_status = self.package_customizations.status();
        let market = runtime::market_items();
        let catalog = settings_catalog::catalog_with_palette(
            self.settings_revision,
            &self.base_config,
            &self.user_preferences,
            &market,
            &self.config.colors,
        );
        let slot_pages = settings_catalog::slot_page_snapshot_with_config(
            &self.user_preferences,
            &self.config,
            &self.base_config,
        );
        for route in self.router.routes.values_mut() {
            if !route.window.screen.settings_view.is_open() {
                continue;
            }
            match &catalog {
                Ok(catalog) => route.window.screen.settings_view.refresh_with_resources(
                    catalog.clone(),
                    package_pages.clone(),
                    Some(slot_pages.clone()),
                ),
                Err(_) => route.window.screen.settings_view.set_status(
                    "Settings could not be refreshed. Close and reopen Settings.",
                ),
            }
            route
                .window
                .screen
                .settings_view
                .set_temporary_customizations(self.temporary_customizations.is_some());
            route
                .window
                .screen
                .settings_view
                .set_package_inventory_status(package_status, package_projection_failed);
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
        if let Some(intent) = self.router.routes.get_mut(&window_id).and_then(|route| {
            route
                .window
                .screen
                .settings_view
                .take_color_favorite_intent()
        }) {
            match intent {
                crate::settings_view::ColorFavoriteIntent::Remember(color) => {
                    self.user_preferences.remember_color(color)
                }
                crate::settings_view::ColorFavoriteIntent::Forget(color) => {
                    self.user_preferences.forget_color(color)
                }
            }
            for route in self.router.routes.values_mut() {
                route
                    .window
                    .screen
                    .settings_view
                    .set_color_favorites(&self.user_preferences.color_favorites);
                if route.window.screen.settings_view.is_open() {
                    route.request_overlay_redraw();
                }
            }
            self.save_settings_preferences();
        }
        if let Some(route) = self.router.routes.get_mut(&window_id) {
            if let Some((target, color)) =
                route.window.screen.settings_view.take_tab_color_edit()
            {
                route
                    .window
                    .screen
                    .context_manager
                    .apply_tab_color(&target, color);
                route.request_overlay_redraw();
                return;
            }
        }
        if let Some(intent) = self
            .router
            .routes
            .get_mut(&window_id)
            .and_then(|route| route.window.screen.settings_view.take_profile_intent())
        {
            self.handle_profile_intent(window_id, intent);
            return;
        }
        if let Some(intent) = self
            .router
            .routes
            .get_mut(&window_id)
            .and_then(|route| route.window.screen.settings_view.take_theme_intent())
        {
            self.apply_theme_intent(event_loop, window_id, intent);
            return;
        }
        if let Some(intent) = self.router.routes.get_mut(&window_id).and_then(|route| {
            route
                .window
                .screen
                .settings_view
                .take_customization_intent()
        }) {
            self.apply_customization_intent(event_loop, window_id, intent);
            return;
        }
        let Some(edit) = self
            .router
            .routes
            .get_mut(&window_id)
            .and_then(|route| route.window.screen.settings_view.take_edit())
        else {
            return;
        };
        self.apply_settings_value(event_loop, window_id, edit);
    }

    pub(super) fn apply_settings_value(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: rio_backend::event::WindowId,
        edit: automexia_ui_model::settings::Edit,
    ) {
        if edit.id.as_str() == "profiles.open"
            && edit.change == automexia_ui_model::settings::Change::Activate
        {
            self.open_profiles(window_id);
            return;
        }
        if edit.id.as_str().starts_with("extension.pkg_") {
            let result = self
                .package_pages()
                .and_then(|pages| pages.ok_or(SettingsError::Unavailable))
                .and_then(|pages| {
                    pages.apply_edit(&self.user_preferences.package_overrides, &edit)
                });
            match result {
                Ok(overrides) => {
                    self.user_preferences.package_overrides = overrides;
                    self.refresh_settings_catalogs();
                    self.save_package_preferences();
                }
                Err(_) => {
                    self.refresh_settings_catalogs();
                    if let Some(route) = self.router.routes.get_mut(&window_id) {
                        route.window.screen.settings_view.set_status(
                            "This package feature changed or is unavailable. Review it and try again.",
                        );
                        route.request_overlay_redraw();
                    }
                }
            }
            return;
        }
        let result = if self.settings_revision == u64::MAX {
            Err(SettingsError::Unavailable)
        } else {
            settings_catalog::apply_edit_with_palette(
                self.settings_revision,
                &self.base_config,
                &self.user_preferences,
                &runtime::market_items(),
                &edit,
                &self.config.colors,
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
        if self.defer_font_change(
            window_id,
            &candidate,
            fonts::FontAction::Edit(edit.clone()),
        ) {
            return;
        }
        if !publish_devops_feature_preferences(&candidate) {
            if let Some(route) = self.router.routes.get_mut(&window_id) {
                route.window.screen.settings_view.set_status(
                    "This extension option could not be changed. The previous setting remains active.",
                );
                route.request_overlay_redraw();
            }
            return;
        }
        if edit.id.as_str().starts_with("fonts.")
            || matches!(
                edit.id.as_str(),
                automexia_ui_model::settings::FONT_SIZE
                    | automexia_ui_model::settings::APPEARANCE_THEME
            )
        {
            let font_changed = candidate.apply_to(&self.base_config).fonts.size
                != self.config.fonts.size;
            self.user_preferences = candidate;
            let revision = self.publish_user_preferences(event_loop, font_changed);
            if let Some(revision) = revision {
                self.settings_save_started(revision);
            } else {
                self.mark_temporary_customizations();
            }
            return;
        }
        self.config.presentation = candidate.apply_to(&self.base_config).presentation;
        self.user_preferences = candidate;
        self.router.set_information_bar_recipe(
            self.user_preferences.visual.information_bar.recipe(),
        );
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
        if let Some(revision) = self.queue_preference_write(PreferenceWriteKind::Settings)
        {
            self.settings_save_started(revision);
        } else {
            self.mark_temporary_customizations();
        }
    }

    fn save_package_preferences(&mut self) {
        if let Some(revision) = self.queue_preference_write(PreferenceWriteKind::Package)
        {
            self.settings_save_started(revision);
        } else {
            self.mark_temporary_customizations();
        }
    }

    /// The single application-owned gate for every persisted preference write.
    /// Temporary resets and subsequent edits stay live only until Restore saved
    /// or restart; neither file family is rewritten by that preview.
    pub(super) fn queue_preference_write(
        &mut self,
        kind: PreferenceWriteKind,
    ) -> Option<u64> {
        if !preference_write_allowed(self.temporary_customizations.as_ref(), kind) {
            return None;
        }
        let preferences = self.user_preferences.clone();
        Some(match kind {
            PreferenceWriteKind::Settings => self.preference_writer.submit(preferences),
            PreferenceWriteKind::Package => {
                self.preference_writer.submit_package(preferences)
            }
        })
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

    fn mark_temporary_customizations(&mut self) {
        for route in self.router.routes.values_mut() {
            if route.window.screen.settings_view.is_open() {
                route
                    .window
                    .screen
                    .settings_view
                    .set_temporary_customizations(true);
                route
                    .window
                    .screen
                    .settings_view
                    .set_status("Temporary preview only. Restore saved to return.");
                route.request_overlay_redraw();
            }
        }
    }

    pub(super) fn apply_customization_intent(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: rio_backend::event::WindowId,
        intent: CustomizationIntent,
    ) {
        let font_action = fonts::FontAction::Intent(intent.clone());
        match intent {
            CustomizationIntent::Reset { revision, scope } => {
                let candidate =
                    if revision != self.settings_revision || revision == u64::MAX {
                        Err(SettingsError::StaleRevision)
                    } else {
                        let packages = self.package_pages().ok().flatten();
                        settings_catalog::reset_customizations(
                            &self.user_preferences,
                            &scope,
                            packages.as_ref(),
                        )
                    };
                let Ok(candidate) = candidate else {
                    self.refresh_settings_catalogs();
                    if let Some(route) = self.router.routes.get_mut(&window_id) {
                        route.window.screen.settings_view.set_status(
                            "This part changed or is unavailable. Review it and try again.",
                        );
                        route.request_overlay_redraw();
                    }
                    return;
                };
                if self.defer_font_change(window_id, &candidate, font_action) {
                    return;
                }
                if !publish_devops_feature_preferences(&candidate) {
                    if let Some(route) = self.router.routes.get_mut(&window_id) {
                        route
                            .window
                            .screen
                            .settings_view
                            .set_status("The previous choices remain active. Try again.");
                        route.request_overlay_redraw();
                    }
                    return;
                }
                if self.temporary_customizations.is_none() {
                    self.temporary_customizations = Some(self.user_preferences.clone());
                }
                let font_changed = candidate.apply_to(&self.base_config).fonts.size
                    != self.config.fonts.size;
                self.user_preferences = candidate;
                self.apply_live_user_preferences(event_loop, font_changed);
                self.mark_temporary_customizations();
            }
            CustomizationIntent::RestoreSaved => {
                let Some(mut saved) = self.temporary_customizations.as_ref().cloned()
                else {
                    return;
                };
                // An extension removed during a preview must not regain live
                // state from the old snapshot. Its on-disk preferences remain
                // untouched by this temporary workflow.
                settings_catalog::prune_removed_extension_features(
                    &mut saved,
                    &runtime::market_items(),
                    runtime::inventory_status() == runtime::InventoryStatus::Ready,
                );
                if let Some(committed) = self.package_customizations.snapshot() {
                    saved.package_overrides =
                        package_customizations::retain_installed_overrides(
                            &saved.package_overrides,
                            &committed,
                        );
                }
                let saved_config = saved.apply_to(&self.base_config);
                let bindings_changed = saved_config.bindings != self.config.bindings;
                let prepared_bindings = if bindings_changed {
                    match crate::bindings::registry::build(&saved_config) {
                        Ok(snapshot) => Some(snapshot),
                        Err(_) => {
                            if let Some(route) = self.router.routes.get_mut(&window_id) {
                                route.window.screen.settings_view.set_status(
                                    "Saved shortcuts conflict with current configuration. Preview remains active.",
                                );
                                route.request_overlay_redraw();
                            }
                            return;
                        }
                    }
                } else {
                    None
                };
                if self.defer_font_change(window_id, &saved, font_action) {
                    return;
                }
                if !publish_devops_feature_preferences(&saved) {
                    if let Some(route) = self.router.routes.get_mut(&window_id) {
                        route.window.screen.settings_view.set_status(
                            "Saved choices could not be restored. Preview remains active.",
                        );
                        route.request_overlay_redraw();
                    }
                    return;
                }
                let font_changed = saved.apply_to(&self.base_config).fonts.size
                    != self.config.fonts.size;
                self.user_preferences = saved;
                self.temporary_customizations = None;
                self.apply_live_user_preferences(event_loop, font_changed);
                if let Some(bindings) = prepared_bindings {
                    for route in self.router.routes.values_mut() {
                        route
                            .window
                            .screen
                            .update_bindings(&self.config, bindings.clone());
                        route.request_redraw();
                    }
                }
                for route in self.router.routes.values_mut() {
                    if route.window.screen.settings_view.is_open() {
                        route.window.screen.settings_view.set_status(
                            "Previous choices restored. Saved files were unchanged.",
                        );
                        route.request_overlay_redraw();
                    }
                }
            }
        }
    }

    pub(super) fn extension_inventory_changed(&mut self) {
        let packages = self.package_customizations.snapshot();
        let (changed, package_changed) = prune_inventory_preferences(
            &mut self.user_preferences,
            self.temporary_customizations.as_mut(),
            &runtime::market_items(),
            runtime::inventory_status() == runtime::InventoryStatus::Ready,
            packages.as_deref(),
        );
        if changed && !publish_devops_feature_preferences(&self.user_preferences) {
            tracing::warn!("extension feature reset could not be published");
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
            if package_changed {
                self.save_package_preferences();
            } else {
                self.save_settings_preferences();
            }
        }
    }
}

#[cfg(test)]
mod extension_feature_preference_tests {
    use super::*;

    fn saved_extension_choices() -> UserPreferences {
        let mut saved = UserPreferences {
            font_size: Some(21.0),
            ..UserPreferences::default()
        };
        saved
            .set_extension_feature_enabled(
                settings_extensions::DEVOPS_CONTEXT_STATUS_ID,
                false,
            )
            .unwrap();
        saved
            .package_overrides
            .push(package_customizations::PackageOverride {
                publisher_id: "example.publisher".into(),
                extension_id: "example.inspect".into(),
                feature_id: "summary".into(),
                option_id: None,
                value: package_customizations::PackageValue::Boolean(false),
            });
        saved
    }

    #[test]
    fn confirmed_uninstall_during_preview_cannot_be_undone_by_reinstall_and_restore() {
        use automexia_ecosystem_runtime::{
            CommittedSettingsSnapshot, InstalledPackageSettings,
        };

        for reset_all in [false, true] {
            let saved = saved_extension_choices();
            let mut current = if reset_all {
                UserPreferences::default()
            } else {
                saved.clone()
            };
            let mut restore = Some(saved);
            let removed = CommittedSettingsSnapshot {
                revision: 2,
                packages: Vec::new(),
            };
            assert_eq!(
                prune_inventory_preferences(
                    &mut current,
                    restore.as_mut(),
                    &[],
                    true,
                    Some(&removed),
                ),
                (!reset_all, !reset_all),
            );
            assert!(current.extension_features.is_empty());
            assert!(current.package_overrides.is_empty());
            let reinstalled = CommittedSettingsSnapshot {
                revision: 3,
                packages: vec![InstalledPackageSettings {
                    publisher_id: "example.publisher".into(),
                    extension_id: "example.inspect".into(),
                    version: "1.0.0".into(),
                    package_sha256: "a".repeat(64),
                    metadata: Ok(None),
                }],
            };
            let market = vec![crate::automexia::marketplace::MarketItem {
                id: "automexia.devops".into(),
                name: "Example".into(),
                description: String::new(),
                installed: true,
            }];
            assert_eq!(
                prune_inventory_preferences(
                    &mut current,
                    restore.as_mut(),
                    &market,
                    true,
                    Some(&reinstalled),
                ),
                (false, false),
            );
            for kind in [PreferenceWriteKind::Settings, PreferenceWriteKind::Package] {
                assert!(!preference_write_allowed(restore.as_ref(), kind));
            }
            let restored = restore.unwrap();
            assert!(restored.extension_features.is_empty());
            assert!(restored.package_overrides.is_empty());
            assert_eq!(restored.font_size, Some(21.0));
        }
    }

    #[test]
    fn unavailable_and_uninitialized_inventory_preserve_both_preview_snapshots() {
        let uninitialized = automexia_ecosystem_runtime::CommittedSettingsSnapshot {
            revision: 0,
            packages: Vec::new(),
        };
        for packages in [None, Some(&uninitialized)] {
            let expected = saved_extension_choices();
            let mut current = expected.clone();
            let mut restore = Some(expected.clone());
            assert_eq!(
                prune_inventory_preferences(
                    &mut current,
                    restore.as_mut(),
                    &[],
                    false,
                    packages,
                ),
                (false, false),
            );
            assert_eq!(current, expected);
            assert_eq!(restore.as_ref(), Some(&expected));
        }
    }

    #[test]
    fn temporary_preview_blocks_both_preference_file_writes_until_restore() {
        let previous = UserPreferences::default();
        let mut preview = Some(previous);
        for kind in [PreferenceWriteKind::Settings, PreferenceWriteKind::Package] {
            assert!(!preference_write_allowed(preview.as_ref(), kind));
        }
        preview = None;
        for kind in [PreferenceWriteKind::Settings, PreferenceWriteKind::Package] {
            assert!(preference_write_allowed(preview.as_ref(), kind));
        }
    }

    #[test]
    fn saved_devops_context_choice_does_not_disable_git_branch_tags() {
        let mut preferences = UserPreferences::default();
        preferences
            .set_extension_feature_enabled(
                settings_extensions::DEVOPS_CONTEXT_STATUS_ID,
                false,
            )
            .unwrap();
        assert_eq!(
            desired_devops_feature_preferences(&preferences),
            (false, true)
        );

        preferences
            .set_extension_feature_enabled(
                settings_extensions::DEVOPS_GIT_STATUS_ID,
                false,
            )
            .unwrap();
        assert_eq!(
            desired_devops_feature_preferences(&preferences),
            (false, false)
        );
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
