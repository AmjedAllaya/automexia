//! Modal settings routing; no terminal input or additional persistence owner.
use super::*;
use automexia_ui_model::settings::Catalog;
use rio_window::event::WindowEvent;

impl Screen<'_> {
    /// The tab menu delegates to the same descriptor-driven editor as settings.
    pub(crate) fn open_requested_tab_color(&mut self) {
        use automexia_ui_model::settings::{
            Section, SettingDescriptor, SettingId, SettingKind, SettingValue,
        };
        let Some(index) = self
            .renderer
            .island
            .as_mut()
            .and_then(|island| island.take_color_request())
        else {
            return;
        };
        let Some(target) = self.context_manager.tab_color_target(index) else {
            return;
        };
        let Ok(id) = SettingId::new("tab.color") else {
            return;
        };
        let fallback =
            crate::renderer::ui_theme::UiTheme::from_colors(&self.renderer.named_colors)
                .blue;
        let current = self.context_manager.custom_color(index).unwrap_or(fallback);
        let mut entry = SettingDescriptor::boolean(
            id,
            Section::Appearance,
            "Tab color",
            "Choose a tab accent. Reset inherits the theme.",
            true,
            true,
        );
        entry.kind = SettingKind::Color { alpha: false };
        entry.value = SettingValue::Color(
            current.map(|value| (value.clamp(0.0, 1.0) * 255.0).round() as u8),
        );
        entry.default = SettingValue::Color(
            fallback.map(|value| (value.clamp(0.0, 1.0) * 255.0).round() as u8),
        );
        let Ok(catalog) = Catalog::new(1, vec![entry]) else {
            return;
        };
        self.open_settings_view(catalog);
        self.settings_view.open_tab_color_editor(target);
        self.fit_settings_view();
        self.mark_dirty();
    }

    pub(crate) fn open_settings_view(&mut self, catalog: Catalog) {
        self.open_settings_view_for_section(catalog, None, None, None);
    }

    pub(crate) fn open_customizations_view(
        &mut self,
        catalog: Catalog,
        packages: Option<
            crate::automexia::package_customizations::PackageCustomizationPages,
        >,
        slot_pages: Option<crate::settings_catalog::SlotPageSnapshot>,
    ) {
        self.open_settings_view_for_section(
            catalog,
            Some(automexia_ui_model::settings::Section::Customizations),
            packages,
            slot_pages,
        );
    }

    fn open_settings_view_for_section(
        &mut self,
        catalog: Catalog,
        section: Option<automexia_ui_model::settings::Section>,
        packages: Option<
            crate::automexia::package_customizations::PackageCustomizationPages,
        >,
        slot_pages: Option<crate::settings_catalog::SlotPageSnapshot>,
    ) {
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
        if let Some(section) = section {
            if section == automexia_ui_model::settings::Section::Customizations {
                self.settings_view
                    .open_customizations_with_slots(catalog, packages, slot_pages);
            } else {
                self.settings_view.open_with_section(catalog, Some(section));
            }
        } else {
            self.settings_view.open(catalog);
        }
        self.fit_settings_view();
        self.mark_dirty();
    }

    pub(crate) fn close_settings_view(&mut self) {
        if self.settings_view.is_open() {
            self.settings_view.close();
            self.renderer.command_palette.remember_menu_origin(None);
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
        // The quit confirmation is above Settings and owns its input until
        // dismissed. Keep the customization page intact when close is canceled.
        if self.renderer.confirm_quit.is_active() {
            return route_covered_settings_event(&mut self.settings_view, event);
        }
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
        if result.closed {
            if self.settings_view.take_back_to_menu() {
                self.renderer.command_palette.resume_menu_parent();
            } else {
                self.renderer.command_palette.remember_menu_origin(None);
            }
        }
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
        route_settings_menu_input(self.renderer.confirm_quit.is_active(), || {
            self.dispatch_settings_key(key, clipboard, false)
        })
    }

    fn dispatch_settings_key(
        &mut self,
        key: &rio_window::event::KeyEvent,
        clipboard: &mut Clipboard,
        record_release: bool,
    ) -> bool {
        if self.renderer.confirm_quit.is_active() {
            return false;
        }
        self.fit_settings_view();
        if key.state == ElementState::Pressed
            && self.settings_view.is_open()
            && (key.logical_key == Key::Named(rio_window::keyboard::NamedKey::Escape)
                || self
                    .settings_view
                    .back_alias(&key.logical_key, self.modifiers.state()))
        {
            self.remember_menu_back_key(key);
        }
        let consumed = route_settings_key(
            &mut self.settings_view,
            &key.logical_key,
            key.text.as_deref(),
            self.modifiers.state(),
            key.state,
            key.repeat,
            |operation| match operation {
                SettingsClipboard::Read => clipboard.try_get(ClipboardType::Clipboard),
                SettingsClipboard::Write(text) => clipboard
                    .try_set(ClipboardType::Clipboard, text)
                    .map(|()| String::new()),
            },
        );
        if !consumed {
            return false;
        }
        if record_release {
            self.remember_settings_key(key);
        }
        if !self.settings_view.is_open() {
            if self.settings_view.take_back_to_menu() {
                self.renderer.command_palette.resume_menu_parent();
            } else {
                self.renderer.command_palette.remember_menu_origin(None);
            }
            self.context_manager.current_mut().ime.set_preedit(None);
            self.last_ime_cursor_pos = None;
        }
        self.mark_dirty();
        true
    }

    pub(crate) fn remember_settings_key(&mut self, key: &rio_window::event::KeyEvent) {
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

    pub(crate) fn remember_menu_back_key(&mut self, key: &rio_window::event::KeyEvent) {
        if key.state == ElementState::Pressed && !key.repeat {
            self.overlay_back_key = Some(key.physical_key);
            self.remember_settings_key(key);
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

    /// Apply already-resolved runtime preferences using loaded resources.
    /// The application supplies already-prepared fonts; filters and images remain
    /// owned by configuration reload. No resource discovery runs here.
    pub(crate) fn update_runtime_preferences(
        &mut self,
        config: &rio_backend::config::Config,
        font_changed: bool,
        prepared: Option<&rio_backend::sugarloaf::font::FontLibrary>,
    ) {
        self.profile_base_config = config.clone();
        self.profile_theme_route = None;
        self.apply_profile_preferences(config, font_changed, prepared);
    }

    fn apply_profile_preferences(
        &mut self,
        config: &rio_backend::config::Config,
        font_changed: bool,
        prepared: Option<&rio_backend::sugarloaf::font::FontLibrary>,
    ) {
        if let Some(library) = prepared {
            self.sugarloaf.update_font(library);
        }
        if font_changed {
            self.sugarloaf.style_mut().font_size = config.fonts.size;
            self.sugarloaf.style_mut().line_height = config.line_height;
            self.grid_rasterizer.clear_font_caches();
            for grid in self.grids.values_mut() {
                grid.clear_atlas();
            }
            for grid in self.context_manager.contexts_mut() {
                grid.update_line_height(config.line_height);
                for item in grid.contexts_mut().values_mut() {
                    rebaseline_preference_item(
                        item,
                        config.fonts.size,
                        config.line_height,
                    );
                }
                grid.update_dimensions(&mut self.sugarloaf);
            }
            self.resize_all_contexts();
        }
        // Settings updates reuse the same pane geometry owner as config reload.
        // A color-only edit never resizes PTYs.
        let size = self.sugarloaf.window_size();
        let scale = self.sugarloaf.scale_factor();
        let top = super::padding_top_from_config(
            &config.navigation,
            config.presentation.interface.header,
            config.margin.top,
            config.window.macos_use_unified_titlebar,
            size.width,
            size.height,
            scale,
        );
        let margin = rio_backend::config::layout::Margin::new(
            top * scale,
            config.margin.right * scale,
            config.margin.bottom * scale,
            config.margin.left * scale,
        );
        self.context_manager.config.panel = config.panel;
        self.context_manager.config.footer_appearance =
            config.presentation.interface.footer;
        self.context_manager.config.split_color = config.colors.split;
        self.context_manager.config.split_active_color = config.colors.split_active;
        for grid in self.context_manager.contexts_mut() {
            let changed = grid.update_appearance(config);
            let margin_changed = grid.scaled_margin != margin;
            if margin_changed {
                grid.update_scaled_margin(margin);
            }
            if changed || margin_changed {
                grid.update_dimensions(&mut self.sugarloaf);
            }
        }
        self.sugarloaf
            .set_window_opaque(super::window_should_be_opaque(config));
        self.renderer.update_config(config);
        self.sugarloaf
            .set_background_color(Some(self.renderer.dynamic_background.1));
        self.update_presentation(config.presentation);
        self.fit_settings_view();
        self.last_ime_cursor_pos = None;
    }

    pub(super) fn sync_profile_theme(&mut self) {
        // Gallery previews temporarily own the window palette. Invalidate the
        // route cache so closing or cancelling the gallery restores the profile.
        if self.settings_view.theme_session().is_some() {
            self.profile_theme_route = None;
            return;
        }
        let current = self.context_manager.current();
        if self.profile_theme_route == Some(current.route_id) {
            return;
        }
        let route = current.route_id;
        let mut config = self.profile_base_config.clone();
        if let Some(colors) = current.profile_colors {
            config.colors = colors;
            config.adaptive_colors = None;
        }
        self.apply_profile_preferences(&config, false, None);
        self.profile_theme_route = Some(route);
    }

    pub(crate) fn apply_profile_presentation(
        &mut self,
        profile: &rio_backend::config::profiles::NamedProfile,
        config: &rio_backend::config::Config,
    ) {
        let current = self.context_manager.current_mut();
        current.profile_colors = profile.theme.as_ref().map(|_| config.colors);
        current.profile_icon = profile.icon.clone();
        let index = self.context_manager.current_index();
        self.context_manager
            .set_custom_title(index, Some(profile.name.clone()));
        let color = profile
            .color
            .as_deref()
            .and_then(|hex| u32::from_str_radix(hex.trim_start_matches('#'), 16).ok())
            .map(|v| {
                [
                    ((v >> 16) & 255) as f32 / 255.0,
                    ((v >> 8) & 255) as f32 / 255.0,
                    (v & 255) as f32 / 255.0,
                    1.0,
                ]
            });
        self.context_manager.set_custom_color(index, color);
        self.profile_theme_route = None;
        self.mark_dirty();
    }

    /// Restore ordinary new-tab defaults after creating the first profile terminal.
    pub(crate) fn reset_profile_window_defaults(
        &mut self,
        base: &rio_backend::config::Config,
        launch: &crate::context::ContextManagerConfig,
    ) {
        self.profile_base_config = base.clone();
        self.profile_theme_route = None;
        self.context_manager.restore_launch_defaults(launch);
    }

    pub(crate) fn create_profile_tab(
        &mut self,
        config: &rio_backend::config::Config,
    ) -> bool {
        let old = self.context_manager.current_index();
        self.resize_top_or_bottom_line();
        #[cfg(not(target_os = "macos"))]
        self.context_manager.contexts_mut()[old].update_dimensions(&mut self.sugarloaf);
        if !self
            .context_manager
            .add_profile_context(next_rich_text_id(), config)
        {
            return false;
        }
        self.context_manager.invalidate_topology_redo();
        let new = self.context_manager.current_index();
        self.context_manager
            .switch_context_visibility(&mut self.sugarloaf, old, new);
        self.resize_top_or_bottom_line();
        self.clear_selection();
        self.mark_dirty();
        true
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

fn route_covered_settings_event(
    view: &mut crate::settings_view::SettingsView,
    event: &WindowEvent,
) -> bool {
    view.suspend_input();
    // Keyboard, pointer and lifecycle events have existing confirmation owners.
    // These input paths do not; consume them before downstream shell handlers.
    matches!(
        event,
        WindowEvent::Ime(_)
            | WindowEvent::Touch(_)
            | WindowEvent::DroppedFile(_)
            | WindowEvent::HoveredFile(_)
            | WindowEvent::HoveredFileCancelled
    )
}

fn route_settings_menu_input(
    confirmation_active: bool,
    dispatch: impl FnOnce() -> bool,
) -> bool {
    // Native menus have no quit-dialog key equivalent. Consume their commands
    // while it is open, rather than letting the caller fall through to the PTY.
    confirmation_active || dispatch()
}

enum SettingsClipboard {
    Read,
    Write(String),
}

/// Shared physical-key/native-menu route. Clipboard access belongs only to the
/// focused editor; a missing selection never falls through to terminal copy.
fn route_settings_key(
    view: &mut crate::settings_view::SettingsView,
    logical_key: &Key,
    text: Option<&str>,
    modifiers: ModifiersState,
    state: ElementState,
    repeat: bool,
    mut clipboard: impl FnMut(
        SettingsClipboard,
    ) -> Result<String, rio_backend::clipboard::ClipboardError>,
) -> bool {
    if !view.is_open() {
        return false;
    }
    if state == ElementState::Pressed {
        let command =
            (modifiers.control_key() || modifiers.super_key()) && !modifiers.alt_key();
        let paste = command
            && matches!(logical_key, Key::Character(value) if value.eq_ignore_ascii_case("v"));
        let copy_or_cut = command
            && matches!(logical_key, Key::Character(value) if value.eq_ignore_ascii_case("c") || value.eq_ignore_ascii_case("x"));
        if paste {
            if !view.requires_larger_window() {
                match clipboard(SettingsClipboard::Read) {
                    Ok(text) => {
                        view.paste(&text);
                    }
                    Err(_) => view.set_status("Paste failed. Try again."),
                }
            }
        } else if copy_or_cut {
            let cut = matches!(logical_key, Key::Character(value) if value.eq_ignore_ascii_case("x"));
            if let Some(text) = view.clipboard_selection() {
                if clipboard(SettingsClipboard::Write(text)).is_ok() {
                    view.set_status("");
                    if cut {
                        view.paste("");
                    }
                } else {
                    view.set_status("Copy failed. Try again.");
                }
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
    fn settings_copy_and_cut_use_only_the_focused_editor_selection() {
        for modifiers in [ModifiersState::SUPER, ModifiersState::CONTROL] {
            let mut view = view();
            view.paste("table");
            view.key(&Key::Character("a".into()), None, modifiers, false);
            let mut clipboard_called = false;
            assert!(route_settings_key(
                &mut view,
                &Key::Character("c".into()),
                None,
                modifiers,
                ElementState::Pressed,
                false,
                |operation| {
                    assert!(
                        matches!(operation, SettingsClipboard::Write(text) if text == "table")
                    );
                    clipboard_called = true;
                    Ok(String::new())
                },
            ));
            assert!(
                clipboard_called,
                "copy must reach the existing clipboard owner"
            );
            assert!(view.accessibility_summary().contains("Search: table."));
            assert!(view.take_edit().is_none());
            assert!(route_settings_key(
                &mut view,
                &Key::Character("x".into()),
                None,
                modifiers,
                ElementState::Pressed,
                false,
                |operation| {
                    assert!(
                        matches!(operation, SettingsClipboard::Write(text) if text == "table")
                    );
                    Ok(String::new())
                }
            ));
            assert!(view.accessibility_summary().contains("Search: ."));
            assert!(route_settings_key(
                &mut view,
                &Key::Character("c".into()),
                None,
                modifiers,
                ElementState::Pressed,
                false,
                |_| panic!("empty selection must not overwrite clipboard")
            ));
        }
    }
    #[test]
    fn settings_failed_paste_preserves_the_selected_draft() {
        let mut view = view();
        view.paste("table");
        view.key(
            &Key::Character("a".into()),
            None,
            ModifiersState::CONTROL,
            false,
        );
        assert!(route_settings_key(
            &mut view,
            &Key::Character("v".into()),
            None,
            ModifiersState::CONTROL,
            ElementState::Pressed,
            false,
            |operation| {
                assert!(matches!(operation, SettingsClipboard::Read));
                Err(rio_backend::clipboard::ClipboardError::Failed)
            }
        ));
        assert_eq!(view.clipboard_selection().as_deref(), Some("table"));
        assert!(view.take_edit().is_none());
    }
    #[test]
    fn settings_failed_cut_preserves_draft_and_selection_for_retry() {
        let mut view = view();
        view.paste("界 text");
        view.key(
            &Key::Character("a".into()),
            None,
            ModifiersState::CONTROL,
            false,
        );
        for state in [ElementState::Released, ElementState::Pressed] {
            assert!(route_settings_key(
                &mut view,
                &Key::Character("x".into()),
                None,
                ModifiersState::CONTROL,
                state,
                false,
                |operation| {
                    assert!(
                        matches!(operation, SettingsClipboard::Write(text) if text == "界 text")
                    );
                    Err(rio_backend::clipboard::ClipboardError::Failed)
                }
            ));
            assert_eq!(view.clipboard_selection().as_deref(), Some("界 text"));
            assert!(view.take_edit().is_none());
        }
        assert!(route_settings_key(
            &mut view,
            &Key::Character("x".into()),
            None,
            ModifiersState::CONTROL,
            ElementState::Pressed,
            false,
            |_| Ok(String::new())
        ));
        assert!(view.clipboard_selection().is_none());
        assert!(view.accessibility_summary().contains("Search: ."));
    }
    #[test]
    fn confirmation_blocks_native_menu_commands_without_reading_clipboard_or_editing_settings(
    ) {
        let mut view = view();
        let before = view.accessibility_summary();
        for key in ["v", "c", "a"] {
            assert!(route_settings_menu_input(true, || route_settings_key(
                &mut view,
                &Key::Character(key.into()),
                None,
                ModifiersState::SUPER,
                ElementState::Pressed,
                false,
                |_| panic!("covered native menu must never read the clipboard"),
            )));
            assert_eq!(view.accessibility_summary(), before);
            assert!(view.take_edit().is_none());
        }
        assert!(route_settings_menu_input(false, || route_settings_key(
            &mut view,
            &Key::Character("v".into()),
            None,
            ModifiersState::SUPER,
            ElementState::Pressed,
            false,
            |_| Ok("table".into()),
        )));
        assert!(view.accessibility_summary().contains("Search: table."));
    }
    #[test]
    fn covered_settings_consume_composition_touch_and_drops_but_yield_confirmation_pointer(
    ) {
        use rio_window::event::{DeviceId, Ime, Touch, TouchPhase};
        let mut view = view();
        view.paste("unchanged draft");
        let before = view.accessibility_summary();
        // SAFETY: confined to pure window-event adapter tests.
        let device_id = unsafe { DeviceId::dummy() };
        for event in [
            WindowEvent::Ime(Ime::Commit("unwanted".into())),
            WindowEvent::Ime(Ime::Preedit("unwanted".into(), None)),
            WindowEvent::DroppedFile("fixture.txt".into()),
            WindowEvent::Touch(Touch {
                device_id,
                phase: TouchPhase::Started,
                location: rio_window::dpi::PhysicalPosition::new(20.0, 30.0),
                force: None,
                id: 1,
            }),
        ] {
            assert!(route_covered_settings_event(&mut view, &event));
            assert_eq!(view.accessibility_summary(), before);
            assert!(view.take_edit().is_none());
        }
        assert!(!route_covered_settings_event(
            &mut view,
            &WindowEvent::MouseInput {
                device_id,
                button: MouseButton::Left,
                state: ElementState::Pressed,
            }
        ));
        assert!(!route_covered_settings_event(
            &mut view,
            &WindowEvent::CloseRequested
        ));
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
                |_| Ok("table".into())
            ));
            assert!(view.accessibility_summary().contains("Search: table."));
            assert!(route_settings_key(
                &mut view,
                &Key::Character("a".into()),
                None,
                modifiers,
                ElementState::Pressed,
                false,
                |_| panic!("Select All must not read the clipboard")
            ));
            assert!(route_settings_key(
                &mut view,
                &Key::Character("v".into()),
                None,
                modifiers,
                ElementState::Pressed,
                false,
                |_| Ok("time".into())
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
                |_| panic!("Select All must not read the clipboard")
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
            |_| panic!("a hidden Settings search must not fetch the clipboard")
        ));
        assert!(view.take_edit().is_none());
        assert!(route_settings_key(
            &mut view,
            &Key::Named(NamedKey::Escape),
            None,
            ModifiersState::empty(),
            ElementState::Pressed,
            false,
            |_| panic!("Escape must not fetch the clipboard")
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
            |_| panic!("closed Settings must not read or deliver clipboard text"),
        );
        assert!(
            !consumed,
            "the existing terminal route regains ownership after close"
        );
        assert!(!view.is_open());
        assert!(view.take_edit().is_none());
    }
}

// Shared font-size owner for runtime preferences and configuration reload.
pub(super) fn rebaseline_preference_item<T: rio_backend::event::EventListener>(
    item: &mut crate::layout::ContextGridItem<T>,
    points: f32,
    line_height: f32,
) {
    for context in item.contexts_mut() {
        context.dimension.rebaseline_font_size(points);
        context.dimension.line_height = line_height;
    }
}

#[cfg(test)]
mod appearance_tests {
    use super::*;
    use crate::context::create_dead_context;
    use crate::event::VoidListener;
    #[test]
    fn appearance_font_rebaseline_updates_every_inactive_local_tab_without_replacing_contexts(
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
        let identities: Vec<_> = item.contexts().map(|c| c.route_id).collect();
        rebaseline_preference_item(&mut item, 21.5, 1.2);
        for context in item.contexts() {
            assert_eq!(
                context.dimension.font_size, 21.5,
                "tab {}",
                context.route_id
            );
            assert_eq!(context.dimension.line_height, 1.2);
        }
        assert_eq!(
            item.contexts().map(|c| c.route_id).collect::<Vec<_>>(),
            identities
        );
        assert_eq!(item.active_tab_index(), 2);
    }
}
