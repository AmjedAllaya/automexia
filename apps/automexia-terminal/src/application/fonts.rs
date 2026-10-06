//! Prepare font resources before publishing a settings transaction.
use super::*;
use crate::automexia::font_preferences::resource_identity;
use crate::font_loading::{Failure, Prepared};
use crate::settings_view::{CustomizationIntent, FontPickerIntent};
use automexia_extension_runtime::RefreshSubmission;
use automexia_ui_model::settings::Edit;
use rio_backend::event::WindowId;
use rio_backend::sugarloaf::font::{FontLibrary, SugarloafFonts};

#[derive(Clone)]
pub(super) enum FontAction {
    Edit(Edit),
    Intent(CustomizationIntent),
    Inventory(u64),
    Preview {
        generation: u64,
        family: String,
        apply: bool,
    },
}
pub(super) struct PendingFont {
    revision: u64,
    window: WindowId,
    route: usize,
    identity: SugarloafFonts,
    action: FontAction,
}

pub(super) struct FontPreview {
    target: PendingFont,
    library: FontLibrary,
}

impl Application<'_> {
    pub(super) fn cancel_font_edit_for_window(&mut self, window: WindowId) {
        if self
            .queued_font_picker
            .as_ref()
            .is_some_and(|p| p.window == window)
        {
            self.queued_font_picker = None;
        }
        if self
            .font_preview
            .as_ref()
            .is_some_and(|p| p.target.window == window)
        {
            self.restore_font_preview();
        }
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
            let view = &mut route.window.screen.settings_view;
            if view.font_picker_session().is_some() {
                view.font_picker_notice(message);
            } else {
                view.set_status(message);
            }
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

    fn font_request_current(&self, pending: &PendingFont, selection: bool) -> bool {
        let Some(route) = self.router.routes.get(&pending.window) else {
            return false;
        };
        let view = &route.window.screen.settings_view;
        if !font_target_is_current(
            pending.revision,
            self.settings_revision,
            Some(pending.route),
            view.is_open()
                .then(|| route.window.screen.ctx().current_route()),
        ) {
            return false;
        }
        match &pending.action {
            FontAction::Inventory(generation) => {
                view.font_picker_session() == Some((*generation, pending.revision))
            }
            FontAction::Preview {
                generation, family, ..
            } => {
                view.font_picker_session() == Some((*generation, pending.revision))
                    && (!selection
                        || view.font_picker_selection() == Some(family.as_str()))
            }
            _ => true,
        }
    }
    fn restore_font_preview(&mut self) {
        if let Some(preview) = self.font_preview.take() {
            if let Some(route) = self.router.routes.get_mut(&preview.target.window) {
                route.window.screen.update_runtime_preferences(
                    &self.config,
                    true,
                    Some(&self.router.font_library),
                );
                route.request_redraw();
            }
        }
    }
    pub(super) fn apply_font_picker_intent(
        &mut self,
        event_loop: &ActiveEventLoop,
        window: WindowId,
        intent: FontPickerIntent,
    ) {
        if matches!(intent, FontPickerIntent::Cancel) {
            self.cancel_font_edit_for_window(window);
            return;
        }
        if matches!(intent, FontPickerIntent::Load) {
            self.restore_font_preview();
        }
        let Some(route) = self.router.routes.get(&window) else {
            return;
        };
        let Some((generation, revision)) =
            route.window.screen.settings_view.font_picker_session()
        else {
            return;
        };
        let mut identity = resource_identity(&self.config.fonts);
        let apply = matches!(intent, FontPickerIntent::Apply(_));
        let action = match intent {
            FontPickerIntent::Load => FontAction::Inventory(generation),
            FontPickerIntent::Preview(family) | FontPickerIntent::Apply(family) => {
                if !crate::automexia::font_preferences::valid_family(&family) {
                    return;
                }
                identity.family = Some(family.clone());
                FontAction::Preview {
                    generation,
                    family,
                    apply,
                }
            }
            FontPickerIntent::Cancel => return,
        };
        let request = PendingFont {
            revision,
            window,
            route: route.window.screen.ctx().current_route(),
            identity,
            action,
        };
        if !self.font_request_current(&request, true) {
            return;
        }
        if let Some(preview) = &self.font_preview {
            if matches!(request.action, FontAction::Preview { .. })
                && preview.target.window == window
                && preview.target.identity == request.identity
            {
                if !apply {
                    return;
                }
                let library = preview.library.clone();
                self.publish_font_picker(event_loop, request, library);
                return;
            }
        }
        if let Some(pending) = &mut self.pending_font {
            if pending.window == window
                && pending.revision == revision
                && pending.identity == request.identity
                && matches!(pending.action, FontAction::Preview { generation: g, .. } if g == generation)
                && matches!(request.action, FontAction::Preview { .. })
            {
                pending.action = request.action;
                self.queued_font_picker = None;
                return;
            }
            self.font_preparation.cancel();
            self.pending_font = None;
        }
        self.queued_font_picker = Some(request);
        self.start_queued_font_picker();
    }
    fn start_queued_font_picker(&mut self) {
        if self.font_preparation.busy() {
            return;
        }
        let Some(request) = self.queued_font_picker.take() else {
            return;
        };
        if !self.font_request_current(&request, true) {
            return;
        }
        let submitted = if matches!(request.action, FontAction::Inventory(_)) {
            self.font_preparation.submit_inventory(
                (*self.router.font_library).clone(),
                request.window,
                Instant::now(),
            )
        } else {
            self.font_preparation.submit(
                request.identity.clone(),
                request.window,
                Instant::now(),
            )
        };
        if submitted == RefreshSubmission::Queued {
            self.font_status(
                request.window,
                if matches!(request.action, FontAction::Inventory(_)) {
                    "Finding installed fonts… Esc cancels."
                } else {
                    "Loading preview… Continue browsing or press Esc to cancel."
                },
            );
            self.pending_font = Some(request);
        } else {
            self.font_status(
                request.window,
                "Font loading is unavailable. Press F5 to retry.",
            );
        }
    }
    fn publish_font_picker(
        &mut self,
        event_loop: &ActiveEventLoop,
        request: PendingFont,
        library: FontLibrary,
    ) {
        let FontAction::Preview { family, apply, .. } = &request.action else {
            return;
        };
        if *apply {
            let Ok(id) = automexia_ui_model::settings::SettingId::new("fonts.family")
            else {
                return;
            };
            let edit = Edit {
                revision: request.revision,
                id,
                change: automexia_ui_model::settings::Change::Set(
                    automexia_ui_model::settings::SettingValue::Text(family.clone()),
                ),
            };
            self.prepared_font = Some((request.identity.clone(), library));
            self.apply_settings_value(event_loop, request.window, edit);
            self.prepared_font = None;
            if self.settings_revision != request.revision {
                if let Some(route) = self.router.routes.get_mut(&request.window) {
                    route.window.screen.settings_view.close_font_picker();
                    route.window.screen.settings_view.take_font_picker_intent();
                    route.request_redraw();
                }
                // Publishing preferences replaces the router's saved library.
                // Restore uses that current authority, never the old preview.
                self.restore_font_preview();
            }
        } else {
            if self
                .font_preview
                .as_ref()
                .is_some_and(|old| old.target.window != request.window)
            {
                self.restore_font_preview();
            }
            let mut config = self.config.clone();
            config.fonts.family = Some(family.clone());
            if let Some(route) = self.router.routes.get_mut(&request.window) {
                route.window.screen.update_runtime_preferences(
                    &config,
                    true,
                    Some(&library),
                );
                // The terminal previews the candidate; controls remain readable
                // even for a symbol font. Apply updates the shared UI font too.
                route
                    .window
                    .screen
                    .sugarloaf
                    .text_mut()
                    .update_font(&self.router.font_library);
                route
                    .window
                    .screen
                    .settings_view
                    .font_picker_notice("Preview only · Enter applies · Esc restores");
                route.request_redraw();
                self.font_preview = Some(FontPreview {
                    target: request,
                    library,
                });
            }
        }
    }
    pub(super) fn finish_font_preparation(&mut self, event_loop: &ActiveEventLoop) {
        if self
            .font_preview
            .as_ref()
            .is_some_and(|p| !self.font_request_current(&p.target, false))
        {
            self.restore_font_preview();
        }
        if self
            .pending_font
            .as_ref()
            .is_some_and(|p| !self.font_request_current(p, true))
        {
            self.font_preparation.cancel();
            self.pending_font = None;
        }
        if let Some(result) = self.font_preparation.take(Instant::now()) {
            if let Some(pending) = self.pending_font.take() {
                match result {
                    Ok(Prepared::Inventory(entries, limited)) => {
                        if matches!(pending.action, FontAction::Inventory(_)) {
                            if let Some(route) = self.router.routes.get_mut(&pending.window) {
                                route.window.screen.settings_view.font_picker_inventory(entries, limited);
                                route.request_overlay_redraw();
                            }
                        }
                    }
                    Ok(Prepared::Library(library)) => {
                        if matches!(pending.action, FontAction::Preview { .. }) {
                            self.publish_font_picker(event_loop, pending, library);
                        } else {
                            self.prepared_font = Some((pending.identity, library));
                            match pending.action {
                                FontAction::Edit(edit) => self.apply_settings_value(event_loop, pending.window, edit),
                                FontAction::Intent(intent) => self.apply_customization_intent(event_loop, pending.window, intent),
                                _ => {},
                            }
                            self.prepared_font = None;
                        }
                    }
                    Err(error) => self.font_status(pending.window, match error {
                        Failure::Missing => "This font or requested style is unavailable. Choose another font or adjust its weights.",
                        Failure::TimedOut => "Font loading timed out. Current choices remain active. Press F5 to retry.",
                        Failure::Worker | Failure::Cancelled => "Font could not be loaded. Current choices remain active. Press F5 to retry.",
                    }),
                }
            }
        }
        self.start_queued_font_picker();
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
