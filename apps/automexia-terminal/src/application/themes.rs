//! Window-scoped preview transactions and bounded theme-library effects.
use super::*;
use crate::automexia::theme_gallery::ThemeSelection;
use crate::automexia::theme_gallery_io::{ThemeOperation, ThemeOutcome};
use crate::settings_view::{ThemeContext, ThemeIntent};
use automexia_extension_runtime::RefreshSubmission;
use rio_backend::event::WindowId;

pub(super) struct ThemeTarget {
    window: WindowId,
    route: usize,
    revision: u64,
    generation: u64,
}
pub(super) struct PendingTheme {
    target: ThemeTarget,
    apply_copy: bool,
}
impl Application<'_> {
    pub(super) fn theme_context(&self, window: WindowId) -> ThemeContext {
        let mut preferences = self.user_preferences.clone();
        preferences.theme_selection = None;
        preferences.appearance_theme = None;
        let system_theme = self
            .router
            .routes
            .get(&window)
            .and_then(|route| route.window.winit_window.theme());
        let config = resolve_runtime_preference_config(
            &self.base_config,
            &preferences,
            system_theme,
        );
        ThemeContext {
            configured: rio_backend::config::theme::Theme {
                colors: config.colors,
            },
            saved: self.user_preferences.theme_selection.clone(),
            font_colors: !self.user_preferences.fonts.colors.is_empty(),
        }
    }
    fn theme_target(&self, window: WindowId) -> Option<ThemeTarget> {
        let route = self.router.routes.get(&window)?;
        Some(ThemeTarget {
            window,
            route: route.window.screen.ctx().current_route(),
            revision: self.settings_revision,
            generation: route.window.screen.settings_view.theme_session()?,
        })
    }
    fn theme_target_current(&self, target: &ThemeTarget) -> bool {
        self.router.routes.get(&target.window).is_some_and(|route| {
            route.window.screen.ctx().current_route() == target.route
                && route.window.screen.settings_view.theme_session()
                    == Some(target.generation)
        })
    }
    pub(super) fn restore_theme_preview(&mut self) {
        if let Some(target) = self.theme_preview.take() {
            if let Some(route) = self.router.routes.get_mut(&target.window) {
                route
                    .window
                    .screen
                    .update_runtime_preferences(&self.config, false, None);
                route.window.configure_window(&self.config);
                route.request_redraw();
            }
        }
    }
    fn theme_notice(&mut self, window: WindowId, message: &str, busy: bool) {
        if let Some(route) = self.router.routes.get_mut(&window) {
            route
                .window
                .screen
                .settings_view
                .theme_notice(message, busy);
            route.request_overlay_redraw();
        }
    }
    fn queue_theme(
        &mut self,
        window: WindowId,
        operation: ThemeOperation,
        apply_copy: bool,
    ) {
        let Some(target) = self.theme_target(window) else {
            return;
        };
        match self.theme_library.submit(operation, window, Instant::now()) {
            RefreshSubmission::Queued => {
                self.pending_theme = Some(PendingTheme { target, apply_copy });
                self.theme_notice(window, "Loading… You can leave with Esc.", true);
            }
            RefreshSubmission::Busy => self.theme_notice(
                window,
                "A file operation is finishing. Try again shortly.",
                false,
            ),
            _ => self.theme_notice(
                window,
                "Theme files are unavailable. Reopen Themes to retry.",
                false,
            ),
        }
    }
    fn apply_theme(
        &mut self,
        event_loop: &ActiveEventLoop,
        window: WindowId,
        selection: Option<ThemeSelection>,
    ) {
        if selection.as_ref().is_some_and(|s| !s.is_valid()) {
            self.theme_notice(window, "This theme could not be applied.", false);
            return;
        }
        self.theme_preview = None;
        if selection.is_none() {
            self.user_preferences.appearance_theme = None;
        }
        self.user_preferences.theme_selection = selection;
        self.publish_user_preferences(event_loop, false);
        if let Some(route) = self.router.routes.get_mut(&window) {
            route.window.screen.close_settings_view();
            route.request_redraw();
        }
    }
    pub(super) fn apply_theme_intent(
        &mut self,
        event_loop: &ActiveEventLoop,
        window: WindowId,
        intent: ThemeIntent,
    ) {
        match intent {
            ThemeIntent::Load => self.queue_theme(window, ThemeOperation::Scan, false),
            ThemeIntent::Cancel => {
                if self
                    .pending_theme
                    .as_ref()
                    .is_some_and(|p| p.target.window == window)
                {
                    self.theme_library.cancel();
                    self.pending_theme = None;
                }
                if self
                    .theme_preview
                    .as_ref()
                    .is_some_and(|p| p.window == window)
                {
                    self.restore_theme_preview();
                }
            }
            ThemeIntent::Preview(selection) => {
                let Some(target) = self.theme_target(window) else {
                    return;
                };
                if self
                    .theme_preview
                    .as_ref()
                    .is_some_and(|old| old.window != window)
                {
                    self.restore_theme_preview();
                }
                let mut preferences = self.user_preferences.clone();
                if selection.is_none() {
                    preferences.appearance_theme = None;
                }
                preferences.theme_selection = selection;
                let config = resolve_runtime_preference_config(
                    &self.base_config,
                    &preferences,
                    event_loop.system_theme(),
                );
                if let Some(route) = self.router.routes.get_mut(&window) {
                    route
                        .window
                        .screen
                        .update_runtime_preferences(&config, false, None);
                    route.window.configure_window(&config);
                    route.request_redraw();
                    self.theme_preview = Some(target);
                }
            }
            ThemeIntent::Apply(selection) => {
                self.apply_theme(event_loop, window, selection)
            }
            ThemeIntent::SaveCopy(selection) => {
                self.queue_theme(window, ThemeOperation::SaveCopy(selection), true)
            }
            ThemeIntent::Import => {
                let Some(route) = self.router.routes.get(&window) else {
                    return;
                };
                let path = rfd::FileDialog::new()
                    .set_parent(&route.window.winit_window)
                    .set_title("Import a theme")
                    .add_filter("Theme TOML", &["toml"])
                    .pick_file();
                if let Some(path) = path {
                    self.queue_theme(window, ThemeOperation::Import(path), false);
                }
            }
            ThemeIntent::Export(selection) => {
                let Some(route) = self.router.routes.get(&window) else {
                    return;
                };
                let path = rfd::FileDialog::new()
                    .set_parent(&route.window.winit_window)
                    .set_title("Export theme")
                    .set_file_name("automexia-theme.toml")
                    .add_filter("Theme TOML", &["toml"])
                    .save_file();
                if let Some(destination) = path {
                    self.queue_theme(
                        window,
                        ThemeOperation::Export {
                            selection,
                            destination,
                        },
                        false,
                    );
                }
            }
        }
    }
    pub(super) fn finish_theme_work(&mut self, event_loop: &ActiveEventLoop) {
        if self.theme_preview.as_ref().is_some_and(|target| {
            !self.theme_target_current(target)
                || target.revision != self.settings_revision
        }) {
            self.restore_theme_preview();
        }
        if self.pending_theme.as_ref().is_some_and(|pending| {
            !self.theme_target_current(&pending.target)
                || (pending.apply_copy
                    && pending.target.revision != self.settings_revision)
        }) {
            self.theme_library.cancel();
            if let Some(pending) = self.pending_theme.take() {
                self.theme_notice(pending.target.window,"Theme operation cancelled because the editor or settings changed. Try again.",false);
            }
        }
        let Some(result) = self.theme_library.take(Instant::now()) else {
            return;
        };
        let Some(pending) = self.pending_theme.take() else {
            return;
        };
        let window = pending.target.window;
        match result {
            Ok(ThemeOutcome::Inventory { entries, limited }) => {
                if let Some(route) = self.router.routes.get_mut(&window) {
                    route
                        .window
                        .screen
                        .settings_view
                        .theme_inventory(entries, limited);
                    route.request_overlay_redraw();
                }
                self.apply_settings_edit(event_loop, window);
            }
            Ok(ThemeOutcome::Saved(entry)) if pending.apply_copy => {
                self.apply_theme(event_loop, window, entry.selection())
            }
            Ok(ThemeOutcome::Imported(entry) | ThemeOutcome::Saved(entry)) => {
                if let Some(route) = self.router.routes.get_mut(&window) {
                    route.window.screen.settings_view.theme_added(entry);
                    route.request_overlay_redraw();
                }
                self.apply_settings_edit(event_loop, window);
            }
            Ok(ThemeOutcome::Exported) => {
                self.theme_notice(window, "Theme exported.", false)
            }
            Err(error) => self.theme_notice(window, error.message(), false),
        }
    }
}
