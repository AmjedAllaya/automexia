//! Screen-owned input and focus for WSL setup. Process authority stays in the
//! application, which revalidates the originating route before installing.

use super::Screen;
use crate::renderer::ui_theme::UiTheme;
use crate::renderer::wsl_setup::{Choice, Layout, WslSetupDialog};
use rio_window::event::{ElementState, KeyEvent, MouseButton, WindowEvent};
use rio_window::keyboard::{Key, KeyCode, ModifiersState, NamedKey, PhysicalKey};

pub(super) struct WslSetupPopup {
    route: usize,
    dialog: WslSetupDialog,
    focused: Choice,
    hovered: Option<Choice>,
    pressed_button: Option<Choice>,
    held_enter: Option<(PhysicalKey, Choice)>,
    responded: bool,
}

impl WslSetupPopup {
    fn new(route: usize, dialog: WslSetupDialog) -> Self {
        Self {
            route,
            dialog,
            focused: Choice::Dismiss,
            hovered: None,
            pressed_button: None,
            held_enter: None,
            responded: false,
        }
    }

    fn response(&mut self, route: usize, choice: Choice) -> Option<Choice> {
        if self.responded {
            return None;
        }
        self.responded = true;
        Some(if self.route == route && self.dialog.offers(choice) {
            choice
        } else {
            Choice::Dismiss
        })
    }

    fn release_button(&mut self, hovered: Option<Choice>) -> Option<Choice> {
        self.pressed_button
            .take()
            .filter(|choice| Some(*choice) == hovered)
    }

    fn release_enter(&mut self, key: PhysicalKey) -> Option<Choice> {
        if self.held_enter.is_some_and(|(pressed, _)| pressed == key) {
            self.held_enter.take().map(|(_, choice)| choice)
        } else {
            None
        }
    }

    fn focused_in(&self, layout: &Layout) -> Choice {
        if layout.is_enabled(self.focused) {
            self.focused
        } else {
            Choice::Dismiss
        }
    }
}

impl Screen<'_> {
    pub(crate) fn open_wsl_setup_dialog(
        &mut self,
        route: usize,
        dialog: WslSetupDialog,
    ) -> bool {
        if self.wsl_setup.is_some()
            || route != self.context_manager.current_route()
            || !self.terminal_accepts_wsl_setup_dialog()
        {
            return false;
        }
        if self.mouse.left_button_state == ElementState::Pressed
            || self.renderer.scrollbar.is_dragging()
            || self.resize_state.is_some()
        {
            self.mouse.capture_left_for_overlay();
            self.renderer.scrollbar.end_drag();
            self.resize_state = None;
        }
        self.wsl_setup = Some(WslSetupPopup::new(route, dialog));
        self.cancel_terminal_composition();
        self.dismiss_suggestions(
            crate::automexia::suggestions::SuggestionInvalidation::ModalOpened,
        );
        self.mark_dirty();
        true
    }

    pub(crate) fn close_wsl_setup_dialog(&mut self) {
        if self.wsl_setup.take().is_some() {
            self.mark_dirty();
        }
    }

    pub(crate) fn wsl_setup_dialog_active(&self) -> bool {
        self.wsl_setup.is_some()
    }

    fn respond_to_wsl_setup(&mut self, choice: Choice) {
        let current_route = self.context_manager.current_route();
        if let Some(popup) = &mut self.wsl_setup {
            if let Some(choice) = popup.response(current_route, choice) {
                self.context_manager
                    .send_wsl_setup_response(popup.route, choice);
            }
        }
    }

    fn wsl_setup_dimensions(&self) -> (f32, f32, f32) {
        let size = self.sugarloaf.window_size();
        (size.width, size.height, self.sugarloaf.scale_factor())
    }

    pub(super) fn handle_wsl_setup_key(&mut self, key: &KeyEvent) -> bool {
        if self.wsl_setup.is_none() {
            return false;
        }
        if !self.terminal_accepts_wsl_setup_dialog() {
            self.respond_to_wsl_setup(Choice::Dismiss);
            return false;
        }
        self.remember_hint_key(key);
        let dimensions = self.wsl_setup_dimensions();
        let modifiers = self.modifiers.state();
        let Some(popup) = &mut self.wsl_setup else {
            return false;
        };
        let layout = Layout::new(dimensions, &popup.dialog);
        if key.state == ElementState::Released {
            let choice = popup
                .release_enter(key.physical_key)
                .filter(|choice| modifiers.is_empty() && layout.is_enabled(*choice));
            self.consume_overlay_key_release(key);
            if let Some(choice) = choice {
                self.respond_to_wsl_setup(choice);
            }
            return true;
        }
        if key.repeat || popup.responded {
            return true;
        }
        match &key.logical_key {
            Key::Named(NamedKey::Escape) => self.respond_to_wsl_setup(Choice::Dismiss),
            Key::Named(NamedKey::Enter)
                if modifiers.is_empty()
                    && matches!(
                        key.physical_key,
                        PhysicalKey::Code(KeyCode::Enter | KeyCode::NumpadEnter)
                    ) =>
            {
                popup.held_enter = Some((key.physical_key, popup.focused_in(&layout)));
            }
            Key::Named(NamedKey::Tab)
                if modifiers.is_empty() || modifiers == ModifiersState::SHIFT =>
            {
                popup.held_enter = None;
                popup.focused = layout.next_focus(
                    popup.focused_in(&layout),
                    modifiers == ModifiersState::SHIFT,
                );
                self.mark_dirty();
            }
            _ => {}
        }
        true
    }

    pub(crate) fn handle_wsl_setup_window_event(&mut self, event: &WindowEvent) -> bool {
        if self.wsl_setup.is_none() {
            return false;
        }
        if let WindowEvent::KeyboardInput {
            event,
            is_synthetic,
            ..
        } = event
        {
            return *is_synthetic || self.handle_wsl_setup_key(event);
        }
        let dimensions = self.wsl_setup_dimensions();
        let Some(popup) = &mut self.wsl_setup else {
            return false;
        };
        let layout = Layout::new(dimensions, &popup.dialog);
        match event {
            WindowEvent::Focused(false) | WindowEvent::CloseRequested => {
                self.respond_to_wsl_setup(Choice::Dismiss);
                return false;
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.mouse.x = position.x;
                self.mouse.y = position.y;
                self.mouse.raw_y = position.y;
                self.mouse.last_cell = None;
                let hovered = layout.hit((position.x as f32, position.y as f32));
                if popup.hovered != hovered {
                    popup.hovered = hovered;
                    self.mark_dirty();
                }
            }
            WindowEvent::CursorLeft { .. } => {
                popup.pressed_button = None;
                if popup.hovered.take().is_some() {
                    self.mark_dirty();
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                self.mouse.palette_consumes_button(*button, *state, true);
                if *button == MouseButton::Left {
                    let hit = layout.hit((self.mouse.x as f32, self.mouse.y as f32));
                    if *state == ElementState::Pressed {
                        popup.pressed_button = hit;
                    } else if let Some(choice) = popup.release_button(hit) {
                        self.respond_to_wsl_setup(choice);
                    }
                }
            }
            WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. } => {
                popup.pressed_button = None;
                popup.held_enter = None;
                popup.hovered = None;
                return false;
            }
            WindowEvent::ModifiersChanged(modifiers) => self.set_modifiers(*modifiers),
            WindowEvent::MouseWheel { .. }
            | WindowEvent::Ime(_)
            | WindowEvent::DroppedFile(_)
            | WindowEvent::Touch(_) => {}
            _ => return false,
        }
        true
    }

    pub(super) fn render_wsl_setup_dialog(&mut self) {
        let Some(popup) = &self.wsl_setup else { return };
        if popup.route != self.context_manager.current_route()
            || self.renderer.confirm_quit.is_active()
        {
            return;
        }
        Layout::new(self.wsl_setup_dimensions(), &popup.dialog).render(
            &mut self.sugarloaf,
            UiTheme::from_colors(&self.renderer.named_colors),
            &popup.dialog,
            popup.focused,
            popup.hovered,
        );
    }

    pub(super) fn wsl_setup_accessibility(
        &self,
    ) -> Option<automexia_ui_model::accessibility::Surface> {
        let popup = self.wsl_setup.as_ref()?;
        (popup.route == self.context_manager.current_route()).then(|| {
            Layout::new(self.wsl_setup_dimensions(), &popup.dialog)
                .accessibility_surface(&popup.dialog, popup.focused)
        })
    }

    #[cfg(feature = "native-gui-test-hooks")]
    pub(super) fn wsl_setup_snapshot(&self) -> serde_json::Value {
        let Some(popup) = &self.wsl_setup else {
            return serde_json::Value::Null;
        };
        let layout = Layout::new(self.wsl_setup_dimensions(), &popup.dialog);
        let actions: Vec<_> = [Choice::Dismiss, Choice::InstallBash, Choice::InstallEza]
            .into_iter()
            .filter_map(|choice| {
                let bounds = layout.bounds(choice)?;
                Some(serde_json::json!({
                    "label": popup.dialog.action_label(choice), "bounds": bounds
                }))
            })
            .collect();
        serde_json::json!({
            "title": popup.dialog.title,
            "summary": popup.dialog.summary(),
            "simulation": popup.dialog.note.as_deref().is_some_and(|note| {
                note.contains("APT simulation only. No packages will be installed.")
            }),
            "focused_action": popup.dialog.action_label(popup.focused_in(&layout)),
            "hovered_action": popup.hovered.map(|choice| popup.dialog.action_label(choice)),
            "actions": actions,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::renderer::wsl_setup::Tone;

    fn popup(can_install_bash: bool, can_install_eza: bool) -> WslSetupPopup {
        WslSetupPopup::new(
            7,
            WslSetupDialog {
                title: "WSL setup",
                subtitle: None,
                details: Vec::new(),
                note: None,
                tone: Tone::Warning,
                can_install_bash,
                can_install_eza,
            },
        )
    }

    #[test]
    fn wsl_setup_popup_approves_once_from_its_own_route() {
        for action in [Choice::InstallBash, Choice::InstallEza] {
            let mut offer = popup(true, true);
            assert_eq!(offer.focused, Choice::Dismiss);
            assert_eq!(offer.response(7, action), Some(action));
            assert_eq!(offer.response(7, action), None);
            assert_eq!(popup(true, true).response(8, action), Some(Choice::Dismiss));
            assert_eq!(
                popup(false, false).response(7, action),
                Some(Choice::Dismiss)
            );
        }
        assert_eq!(
            popup(false, true).response(7, Choice::InstallBash),
            Some(Choice::Dismiss)
        );
        assert_eq!(
            popup(true, false).response(7, Choice::InstallEza),
            Some(Choice::Dismiss)
        );
    }

    #[test]
    fn wsl_setup_popup_requires_a_matching_press_and_release() {
        let mut offer = popup(true, true);
        assert_eq!(offer.release_button(Some(Choice::InstallBash)), None);
        offer.pressed_button = Some(Choice::InstallEza);
        assert_eq!(offer.release_button(Some(Choice::InstallBash)), None);
        offer.pressed_button = Some(Choice::InstallBash);
        assert_eq!(
            offer.release_button(Some(Choice::InstallBash)),
            Some(Choice::InstallBash)
        );

        let enter = PhysicalKey::Code(KeyCode::Enter);
        let numpad = PhysicalKey::Code(KeyCode::NumpadEnter);
        assert_eq!(offer.release_enter(enter), None);
        offer.held_enter = Some((enter, Choice::Dismiss));
        assert_eq!(offer.release_enter(numpad), None);
        assert_eq!(offer.release_enter(enter), Some(Choice::Dismiss));
        assert_eq!(offer.release_enter(enter), None);
    }
}
