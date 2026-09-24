//! Modal settings routing; no terminal input or additional persistence owner.
use super::*;
use automexia_ui_model::settings::Catalog;
use rio_window::event::WindowEvent;

impl Screen<'_> {
    pub(crate) fn open_settings_view(&mut self, catalog: Catalog) {
        self.stop_hint_mode_if_active();
        self.dismiss_suggestions(
            crate::automexia::suggestions::SuggestionInvalidation::ModalOpened,
        );
        self.dismiss_image_preview();
        self.table_view.close();
        self.renderer.command_palette.set_enabled(false);
        self.renderer.scrollbar.end_drag();
        self.resize_state = None;
        self.touchpurpose = TouchPurpose::None;
        self.mouse.accumulated_scroll = Default::default();
        self.mouse.left_button_state = ElementState::Released;
        self.mouse.middle_button_state = ElementState::Released;
        self.mouse.right_button_state = ElementState::Released;
        self.mouse.hint_click_latched = None;
        self.mouse.image_preview_click_latched = false;
        self.clear_highlighted_hint();
        self.context_manager.current_mut().ime.set_preedit(None);
        self.last_ime_cursor_pos = None;
        self.settings_view.open(catalog);
        self.fit_settings_view();
        self.mark_dirty();
    }

    pub(super) fn close_settings_view(&mut self) {
        if self.settings_view.is_open() {
            self.settings_view.close();
            self.context_manager.current_mut().ime.set_preedit(None);
            self.last_ime_cursor_pos = None;
            self.mark_dirty();
        }
    }

    pub(crate) fn fit_settings_view(&mut self) {
        if !self.settings_view.is_open() {
            return;
        }
        let size = self.sugarloaf.window_size();
        let scale = self.sugarloaf.scale_factor().max(0.1);
        self.settings_view.fit(
            size.width / scale,
            size.height / scale,
            self.context_manager
                .current()
                .dimension
                .font_size
                .clamp(10.0, 32.0),
        );
    }

    pub(crate) fn handle_settings_window_event(
        &mut self,
        event: &WindowEvent,
        clipboard: &mut Clipboard,
    ) -> bool {
        if let WindowEvent::KeyboardInput {
            event: key,
            is_synthetic,
            ..
        } = event
        {
            return if *is_synthetic {
                self.settings_view.is_open()
            } else {
                self.handle_settings_key(key, clipboard)
            };
        }
        if !self.settings_view.is_open() {
            return false;
        }
        if let WindowEvent::ModifiersChanged(modifiers) = event {
            self.set_modifiers(*modifiers);
            return true;
        }
        if let WindowEvent::CursorMoved { position, .. } = event {
            let size = self.sugarloaf.window_size();
            self.mouse.x = position.x.clamp(0.0, f64::from(size.width.max(1.0) - 1.0));
            self.mouse.y = position.y.clamp(0.0, f64::from(size.height.max(1.0) - 1.0));
        }
        if let WindowEvent::MouseInput { button, state, .. } = event {
            // Reuse the existing host-modal latches. Releases remain owned if
            // Escape closes Settings while a pointer button is still held.
            self.mouse.palette_consumes_button(*button, *state, true);
        }
        self.fit_settings_view();
        let result = self.settings_view.event(
            event,
            self.modifiers.state(),
            f64::from(self.sugarloaf.scale_factor()),
        );
        if result.closed || matches!(event, WindowEvent::Focused(false)) {
            self.last_ime_cursor_pos = None;
            self.context_manager.current_mut().ime.set_preedit(None);
        }
        if result.redraw {
            self.mark_dirty();
        }
        result.consumed
    }

    pub(crate) fn handle_settings_key(
        &mut self,
        key: &rio_window::event::KeyEvent,
        clipboard: &mut Clipboard,
    ) -> bool {
        if self.consume_overlay_key_release(key) {
            return true;
        }
        self.dispatch_settings_key(key, clipboard, true)
    }

    pub(crate) fn handle_settings_menu_key(
        &mut self,
        key: &rio_window::event::KeyEvent,
        clipboard: &mut Clipboard,
    ) -> bool {
        // A native menu command has no matching physical key release.
        self.dispatch_settings_key(key, clipboard, false)
    }

    fn dispatch_settings_key(
        &mut self,
        key: &rio_window::event::KeyEvent,
        clipboard: &mut Clipboard,
        record_release: bool,
    ) -> bool {
        self.fit_settings_view();
        let consumed = route_settings_key(
            &mut self.settings_view,
            &key.logical_key,
            key.text.as_deref(),
            self.modifiers.state(),
            key.state,
            key.repeat,
            || clipboard.get(ClipboardType::Clipboard),
        );
        if !consumed {
            return false;
        }
        if record_release {
            self.remember_settings_key(key);
        }
        if !self.settings_view.is_open() {
            self.context_manager.current_mut().ime.set_preedit(None);
            self.last_ime_cursor_pos = None;
        }
        self.mark_dirty();
        true
    }

    pub(super) fn remember_settings_key(&mut self, key: &rio_window::event::KeyEvent) {
        // One release owner is shared by modal views, even after Escape closes one.
        if key.state == ElementState::Pressed
            && !self.overlay_key_releases.contains(&key.physical_key)
            && self.overlay_key_releases.len() < 512
        {
            self.overlay_key_releases.push(key.physical_key);
        }
        #[cfg(windows)]
        if key.state == ElementState::Pressed {
            self.consumed_win32_key_releases
                .record_press(key.physical_key);
        } else {
            self.consumed_win32_key_releases
                .take_release(&key.physical_key);
        }
    }

    pub(super) fn update_settings_ime_cursor(
        &mut self,
        window: &rio_window::window::Window,
    ) {
        let Some(area) = self.settings_view.ime_cursor_area() else {
            return;
        };
        let scale = self.sugarloaf.scale_factor();
        if !scale.is_finite()
            || scale <= 0.0
            || area.iter().any(|v| !v.is_finite() || *v < 0.0)
        {
            return;
        }
        let [x, y, width, height] = area.map(|value| value * scale);
        if [x, y, width, height].iter().any(|value| !value.is_finite()) {
            return;
        }
        self.last_ime_cursor_pos = Some((x, y));
        window.set_ime_cursor_area(
            rio_window::dpi::PhysicalPosition::new(f64::from(x), f64::from(y)),
            rio_window::dpi::PhysicalSize::new(f64::from(width), f64::from(height)),
        );
    }

    /// Publish a visual preference without reloading fonts, images, or PTYs.
    pub(crate) fn update_presentation(
        &mut self,
        presentation: rio_backend::config::presentation::Presentation,
    ) {
        self.renderer.presentation = presentation;
        for grid in self.context_manager.contexts_mut() {
            for item in grid.contexts_mut().values_mut() {
                invalidate_presentation_item(item);
            }
        }
        self.mark_dirty();
    }
}

// Kept below the platform adapter so the real local-tab owner can be exercised
// without creating a window, GPU, PTY or optional extension.
fn invalidate_presentation_item<T: rio_backend::event::EventListener>(
    item: &mut crate::layout::ContextGridItem<T>,
) {
    for context in item.contexts_mut() {
        context
            .renderable_content
            .pending_update
            .set_terminal_damage(rio_backend::event::TerminalDamage::Full);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::create_dead_context;
    use crate::event::VoidListener;

    #[test]
    fn settings_presentation_invalidates_inactive_local_tabs_without_changing_source_maps(
    ) {
        let dead = |id| {
            create_dead_context(
                VoidListener {},
                rio_backend::event::WindowId::from(7),
                id,
                id,
                ContextDimension::default(),
            )
        };
        let mut item = crate::layout::ContextGridItem::new(dead(11));
        item.push_tab_core(dead(22));
        item.push_tab_core(dead(33));
        for context in item.contexts_mut() {
            context
                .renderable_content
                .pending_update
                .take_terminal_damage();
            context.renderable_content.pending_update.reset();
        }
        let before: Vec<_> = item
            .contexts()
            .map(|context| {
                (
                    context.route_id,
                    context.renderable_content.command_rows.visual_row(7),
                )
            })
            .collect();
        invalidate_presentation_item(&mut item);
        for context in item.contexts_mut() {
            assert_eq!(
                context
                    .renderable_content
                    .pending_update
                    .take_terminal_damage(),
                Some(rio_backend::event::TerminalDamage::Full),
                "inactive local tab {} was omitted",
                context.route_id
            );
        }
        let after: Vec<_> = item
            .contexts()
            .map(|context| {
                (
                    context.route_id,
                    context.renderable_content.command_rows.visual_row(7),
                )
            })
            .collect();
        assert_eq!(
            before, after,
            "current pixel/source maps stay paired until the next render"
        );
        assert_eq!(item.active_tab_index(), 2);
    }
}

/// Shared physical-key/native-menu route, with clipboard access deferred until
/// an open Settings search actually receives a paste command.
fn route_settings_key(
    view: &mut crate::settings_view::SettingsView,
    logical_key: &Key,
    text: Option<&str>,
    modifiers: ModifiersState,
    state: ElementState,
    repeat: bool,
    read_clipboard: impl FnOnce() -> String,
) -> bool {
    if !view.is_open() {
        return false;
    }
    if state == ElementState::Pressed {
        let paste = (modifiers.control_key() || modifiers.super_key())
            && !modifiers.alt_key()
            && matches!(logical_key, Key::Character(value) if value.eq_ignore_ascii_case("v"));
        if paste {
            if !view.requires_larger_window() {
                view.paste(&read_clipboard());
            }
        } else {
            view.key(logical_key, text, modifiers, repeat);
        }
    }
    true
}

#[cfg(test)]
mod menu_tests {
    use super::*;
    use automexia_ui_model::settings::{self, CoreOrigins, CoreValues};

    fn view() -> crate::settings_view::SettingsView {
        let mut view = crate::settings_view::SettingsView::default();
        view.fit(720.0, 560.0, 14.0);
        view.open(
            Catalog::new(
                1,
                settings::core_descriptors(
                    CoreValues::default(),
                    CoreValues::default(),
                    CoreOrigins::default(),
                ),
            )
            .unwrap(),
        );
        view
    }
    #[test]
    fn settings_menu_paste_and_select_all_share_the_physical_key_search_owner() {
        let mut view = view();
        for modifiers in [ModifiersState::SUPER, ModifiersState::CONTROL] {
            assert!(route_settings_key(
                &mut view,
                &Key::Character("v".into()),
                None,
                modifiers,
                ElementState::Pressed,
                false,
                || "table".into()
            ));
            assert!(view.accessibility_summary().contains("Search: table."));
            assert!(route_settings_key(
                &mut view,
                &Key::Character("a".into()),
                None,
                modifiers,
                ElementState::Pressed,
                false,
                || panic!("Select All must not read the clipboard")
            ));
            assert!(route_settings_key(
                &mut view,
                &Key::Character("v".into()),
                None,
                modifiers,
                ElementState::Pressed,
                false,
                || "time".into()
            ));
            assert!(view.accessibility_summary().contains("Search: time."));
            assert!(
                view.take_edit().is_none(),
                "search never changes a preference"
            );
            assert!(route_settings_key(
                &mut view,
                &Key::Character("a".into()),
                None,
                modifiers,
                ElementState::Pressed,
                false,
                || panic!("Select All must not read the clipboard")
            ));
        }
    }
    #[test]
    fn settings_compact_menu_paste_never_reads_clipboard_and_escape_still_closes() {
        let mut view = view();
        view.fit(80.0, 100.0, 20.0);
        assert!(route_settings_key(
            &mut view,
            &Key::Character("v".into()),
            None,
            ModifiersState::SUPER,
            ElementState::Pressed,
            false,
            || panic!("a hidden Settings search must not fetch the clipboard")
        ));
        assert!(view.take_edit().is_none());
        assert!(route_settings_key(
            &mut view,
            &Key::Named(NamedKey::Escape),
            None,
            ModifiersState::empty(),
            ElementState::Pressed,
            false,
            || panic!("Escape must not fetch the clipboard")
        ));
        assert!(!view.is_open());
    }

    #[test]
    fn settings_closed_menu_route_restores_terminal_dispatch_without_reading_clipboard() {
        let mut view = view();
        view.close();
        let consumed = route_settings_key(
            &mut view,
            &Key::Character("v".into()),
            None,
            ModifiersState::SUPER,
            ElementState::Pressed,
            false,
            || panic!("closed Settings must not read or deliver clipboard text"),
        );
        assert!(
            !consumed,
            "the existing terminal route regains ownership after close"
        );
        assert!(!view.is_open());
        assert!(view.take_edit().is_none());
    }
}
