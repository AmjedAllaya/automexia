//! Prepare font resources before publishing a settings transaction.
use super::*;
use crate::automexia::font_preferences::resource_identity;
use crate::font_loading::Failure;
use crate::settings_view::CustomizationIntent;
use automexia_extension_runtime::RefreshSubmission;
use automexia_ui_model::settings::Edit;
use rio_backend::event::WindowId;
use rio_backend::sugarloaf::font::{FontLibrary, SugarloafFonts};

#[derive(Clone)]
pub(super) enum FontAction {
    Edit(Edit),
    Intent(CustomizationIntent),
}
pub(super) struct PendingFont {
    revision: u64,
    window: WindowId,
    route: usize,
    identity: SugarloafFonts,
    action: FontAction,
}

impl Application<'_> {
    pub(super) fn cancel_font_edit_for_window(&mut self, window: WindowId) {
        // Reopening Settings must not revive a request from a closed editor,
        // even when close/open events arrive before about_to_wait drains it.
        if self
            .pending_font
            .as_ref()
            .is_some_and(|pending| pending.window == window)
        {
            self.font_preparation.cancel();
            self.pending_font = None;
        }
    }

    fn font_status(&mut self, window: WindowId, message: &str) {
        if let Some(route) = self.router.routes.get_mut(&window) {
            route.window.screen.settings_view.set_status(message);
            route.request_overlay_redraw();
        }
    }

    /// Returns true while the transaction must be deferred or rejected. No
    /// preference, extension, disk or renderer state is changed in that case.
    pub(super) fn defer_font_change(
        &mut self,
        window: WindowId,
        candidate: &UserPreferences,
        action: FontAction,
    ) -> bool {
        let config = candidate.apply_to(&self.base_config);
        let identity = resource_identity(&config.fonts);
        if identity == resource_identity(&self.config.fonts)
            || self
                .prepared_font
                .as_ref()
                .is_some_and(|(key, _)| *key == identity)
        {
            return false;
        }
        let Some(route) = self.router.routes.get(&window) else {
            return true;
        };
        let route_id = route.window.screen.ctx().current_route();
        match self
            .font_preparation
            .submit(config.fonts, window, Instant::now())
        {
            RefreshSubmission::Queued => {
                self.pending_font = Some(PendingFont {
                    revision: self.settings_revision,
                    window,
                    route: route_id,
                    identity,
                    action,
                });
                self.font_status(
                    window,
                    "Loading font… Keep this editor open. Current text stays active.",
                );
            }
            RefreshSubmission::Busy => self.font_status(
                window,
                "A font is still loading. Wait a moment and try again.",
            ),
            RefreshSubmission::Unavailable | RefreshSubmission::Rejected => self
                .font_status(
                    window,
                    "Font loading is unavailable. Current choices remain active.",
                ),
        }
        true
    }

    pub(super) fn finish_font_preparation(&mut self, event_loop: &ActiveEventLoop) {
        let stale = self.pending_font.as_ref().is_some_and(|pending| {
            !font_target_is_current(
                pending.revision,
                self.settings_revision,
                Some(pending.route),
                self.router.routes.get(&pending.window).and_then(|route| {
                    route
                        .window
                        .screen
                        .settings_view
                        .is_open()
                        .then(|| route.window.screen.ctx().current_route())
                }),
            )
        });
        if stale {
            self.font_preparation.cancel();
            if let Some(pending) = self.pending_font.take() {
                self.font_status(pending.window, "Font change cancelled because the editor or settings changed. Try again.");
            }
        }
        let Some(result) = self.font_preparation.take(Instant::now()) else {
            return;
        };
        let Some(pending) = self.pending_font.take() else {
            return;
        };
        match result {
            Ok(library) => {
                self.prepared_font = Some((pending.identity, library));
                match pending.action {
                    FontAction::Edit(edit) => self.apply_settings_value(event_loop, pending.window, edit),
                    FontAction::Intent(intent) => self.apply_customization_intent(event_loop, pending.window, intent),
                }
                // A failed replay must not leave resources for a later edit.
                self.prepared_font = None;
            }
            Err(error) => self.font_status(pending.window, match error {
                Failure::Missing => "That font or style is not installed. Current font remains active; choose another family or weight.",
                Failure::TimedOut => "Font loading timed out. Current choices remain active.",
                Failure::Worker | Failure::Cancelled => "Font could not be loaded. Current choices remain active.",
            }),
        }
    }

    pub(super) fn take_prepared_font(&mut self) -> Option<FontLibrary> {
        self.prepared_font.take().and_then(|(key, library)| {
            (key == resource_identity(&self.config.fonts)).then_some(library)
        })
    }
}

fn font_target_is_current(
    request: u64,
    current: u64,
    target: Option<usize>,
    active: Option<usize>,
) -> bool {
    request == current && request != u64::MAX && target.is_some() && target == active
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fonts_never_publish_to_changed_settings_closed_editor_or_another_pane() {
        assert!(font_target_is_current(5, 5, Some(2), Some(2)));
        assert!(!font_target_is_current(5, 6, Some(2), Some(2)));
        assert!(!font_target_is_current(5, 5, Some(2), None));
        assert!(!font_target_is_current(5, 5, Some(2), Some(3)));
        assert!(!font_target_is_current(
            u64::MAX,
            u64::MAX,
            Some(2),
            Some(2)
        ));
    }
}
