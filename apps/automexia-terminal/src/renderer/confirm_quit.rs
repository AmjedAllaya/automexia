// Copyright (c) 2023-present, Raphael Amorim.
//
// This source code is licensed under the MIT license found in the
// LICENSE file in the root directory of this source tree.

use crate::renderer::responsive::Viewport;
use crate::renderer::ui_theme::{color_u8, UiTheme, MODAL_SHADOW as SHADOW};
use rio_backend::sugarloaf::text::DrawOpts;
use rio_backend::sugarloaf::Sugarloaf;

const ORDER: u8 = 20;

#[derive(Clone, Copy, Debug, PartialEq)]
struct Rect {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}

impl Rect {
    fn contains(self, x: f32, y: f32) -> bool {
        x >= self.x
            && x <= self.x + self.width
            && y >= self.y
            && y <= self.y + self.height
    }
}

#[derive(Clone, Copy, Debug)]
struct Layout {
    card: Rect,
    cancel: Rect,
    quit: Rect,
    compact: bool,
    tiny: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConfirmQuitAction {
    Cancel,
    Quit,
}

#[derive(Default)]
pub struct ConfirmQuit {
    active: bool,
    hovered: Option<ConfirmQuitAction>,
    recovery: bool,
    loading: bool,
    choice: Option<bool>,
    selected_restore: bool,
    held: Option<rio_window::keyboard::Key>,
    notice: Option<(&'static str, std::time::Instant)>,
}

impl ConfirmQuit {
    pub(crate) fn accessibility_surface(
        &self,
        dimensions: (f32, f32, f32),
    ) -> automexia_ui_model::accessibility::Surface {
        use accesskit::{Node, Role};
        use automexia_ui_model::accessibility::{physical_bounds, Surface};
        let viewport =
            accesskit::Rect::new(0.0, 0.0, dimensions.0 as f64, dimensions.1 as f64);
        let layout = Self::layout(dimensions);
        let rect = |r: Rect| {
            physical_bounds([r.x, r.y, r.width, r.height], dimensions.2, viewport)
        };
        let mut surface = Surface::dialog(
            u64::MAX - 2,
            if self.recovery {
                "Restore previous session"
            } else {
                "Quit Automexia?"
            },
            rect(layout.card).unwrap_or(viewport),
        );
        let mut explanation = Node::new(Role::Label);
        explanation.set_label(self.accessibility_summary());
        surface.push(1, explanation, self.loading);
        for (id, label, bounds, focused) in [
            (
                2,
                if self.recovery {
                    "Start clean"
                } else {
                    "Cancel"
                },
                layout.cancel,
                self.recovery && !self.selected_restore,
            ),
            (
                3,
                if self.recovery { "Restore" } else { "Quit" },
                layout.quit,
                !self.recovery || self.selected_restore,
            ),
        ] {
            let mut node = Node::new(Role::Button);
            node.set_label(label);
            if let Some(bounds) = rect(bounds) {
                node.set_bounds(bounds);
            }
            if self.loading {
                node.set_disabled();
            }
            surface.push(id, node, focused && !self.loading);
        }
        surface
    }

    pub(crate) fn accessibility_summary(&self) -> String {
        if self.recovery {
            format!("Restore previous session. {}. Tab chooses; Enter confirms; Escape starts clean.",
                if self.loading { "Loading recovery" } else if self.selected_restore { "Restore selected" } else { "Start clean selected" })
        } else {
            "Quit Automexia? This closes the terminal window. Escape cancels; Enter confirms.".into()
        }
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn set_active(&mut self, active: bool) {
        self.active = active;
        if !active {
            self.hovered = None;
        }
    }

    pub(crate) fn show_recovery(&mut self, loading: bool) {
        self.active = true;
        self.recovery = true;
        self.loading = loading;
        self.choice = None;
        self.held = None;
        self.selected_restore = false;
        self.hovered = if loading {
            None
        } else {
            Some(ConfirmQuitAction::Cancel)
        };
    }
    pub(crate) fn is_recovery(&self) -> bool {
        self.active && self.recovery
    }
    #[cfg(feature = "native-gui-test-hooks")]
    pub(crate) fn recovery_ready(&self) -> bool {
        self.is_recovery() && !self.loading
    }
    pub(crate) fn choose_recovery(&mut self, restore: bool) {
        if self.is_recovery() && !self.loading {
            self.choice = Some(restore);
            self.loading = true;
        }
    }
    pub(crate) fn take_recovery_choice(&mut self) -> Option<bool> {
        self.choice.take()
    }
    pub(crate) fn finish_recovery(&mut self) {
        if !self.recovery {
            return;
        }
        self.active = false;
        self.recovery = false;
        self.loading = false;
        self.choice = None;
        self.hovered = None;
    }
    pub(crate) fn notice(&mut self, text: &'static str) {
        self.notice = Some((
            text,
            std::time::Instant::now() + std::time::Duration::from_secs(15),
        ));
    }
    pub(crate) fn has_notice(&self) -> bool {
        self.notice
            .is_some_and(|(_, until)| std::time::Instant::now() < until)
    }
    pub(crate) fn recovery_key(&mut self, event: &rio_window::event::KeyEvent) -> bool {
        self.recovery_key_event(&event.logical_key, event.state, event.repeat)
    }
    fn recovery_key_event(
        &mut self,
        key: &rio_window::keyboard::Key,
        state: rio_window::event::ElementState,
        repeat: bool,
    ) -> bool {
        use rio_window::{
            event::ElementState,
            keyboard::{Key, NamedKey},
        };
        if state == ElementState::Released && self.held.as_ref() == Some(key) {
            self.held = None;
            if self.is_recovery() && !self.loading {
                match key {
                    Key::Named(NamedKey::Escape) => self.choose_recovery(false),
                    Key::Named(NamedKey::Enter) => {
                        self.choose_recovery(self.selected_restore)
                    }
                    Key::Character(c) if c.eq_ignore_ascii_case("r") => {
                        self.choose_recovery(true)
                    }
                    Key::Character(c) if c.eq_ignore_ascii_case("s") => {
                        self.choose_recovery(false)
                    }
                    _ => {}
                }
            }
            return true;
        }
        if !self.is_recovery() {
            return false;
        }
        if state == ElementState::Pressed && !repeat {
            match key {
                Key::Named(
                    NamedKey::Tab | NamedKey::ArrowLeft | NamedKey::ArrowRight,
                ) => {
                    self.selected_restore = !self.selected_restore;
                    self.hovered = Some(if self.selected_restore {
                        ConfirmQuitAction::Quit
                    } else {
                        ConfirmQuitAction::Cancel
                    });
                }
                _ => self.held = Some(key.clone()),
            }
        }
        true
    }

    fn layout(dimensions: (f32, f32, f32)) -> Layout {
        let viewport = Viewport::from_physical(dimensions.0, dimensions.1, dimensions.2);
        let win_w = viewport.width.max(1.0);
        let win_h = viewport.height.max(1.0);
        let margin = if win_w < 220.0 || win_h < 150.0 {
            6.0
        } else {
            18.0
        };
        let width = 432.0_f32.min((win_w - margin * 2.0).max(1.0));
        let height = 224.0_f32.min((win_h - margin * 2.0).max(1.0));
        let card = Rect {
            x: ((win_w - width) * 0.5).max(0.0),
            y: ((win_h - height) * 0.5).max(0.0),
            width,
            height,
        };
        let compact = width < 300.0 || height < 170.0;
        let tiny = width < 220.0 || height < 110.0;
        let inner = if tiny {
            6.0
        } else if compact {
            10.0
        } else {
            24.0
        };
        let gap = if tiny {
            4.0
        } else if compact {
            6.0
        } else {
            10.0
        };
        let button_height = (if tiny {
            28.0_f32
        } else if compact {
            32.0
        } else {
            38.0
        })
        .min((height - inner * 2.0).max(1.0));
        let button_width = ((width - inner * 2.0 - gap) * 0.5).max(1.0);
        let button_y = if tiny {
            card.y + ((height - button_height) * 0.5).max(0.0)
        } else {
            (card.y + height - inner - button_height).max(card.y)
        };
        let cancel = Rect {
            x: card.x + inner,
            y: button_y,
            width: button_width,
            height: button_height,
        };
        let quit = Rect {
            x: cancel.x + button_width + gap,
            ..cancel
        };
        Layout {
            card,
            cancel,
            quit,
            compact,
            tiny,
        }
    }

    pub fn hit_test(
        &self,
        mouse_x: f32,
        mouse_y: f32,
        dimensions: (f32, f32, f32),
    ) -> Option<ConfirmQuitAction> {
        if !self.active || self.loading {
            return None;
        }
        let layout = Self::layout(dimensions);
        if layout.cancel.contains(mouse_x, mouse_y) {
            Some(ConfirmQuitAction::Cancel)
        } else if layout.quit.contains(mouse_x, mouse_y) {
            Some(ConfirmQuitAction::Quit)
        } else {
            None
        }
    }

    pub fn hover(
        &mut self,
        mouse_x: f32,
        mouse_y: f32,
        dimensions: (f32, f32, f32),
    ) -> bool {
        let next = self.hit_test(mouse_x, mouse_y, dimensions);
        if next == self.hovered {
            false
        } else {
            self.hovered = next;
            true
        }
    }

    pub fn hovered_action(&self) -> Option<ConfirmQuitAction> {
        self.hovered
    }

    pub(crate) fn render(
        &self,
        sugarloaf: &mut Sugarloaf,
        dimensions: (f32, f32, f32),
        theme: &UiTheme,
    ) {
        if !self.active {
            if let Some((message, _)) = self
                .notice
                .filter(|(_, until)| std::time::Instant::now() < *until)
            {
                let viewport =
                    Viewport::from_physical(dimensions.0, dimensions.1, dimensions.2);
                let width = (viewport.width - 24.0).clamp(1.0, 720.0);
                rounded(
                    sugarloaf,
                    12.0,
                    (viewport.height - 62.0).max(0.0),
                    width,
                    32.0,
                    theme.surface,
                    8.0,
                );
                let options = DrawOpts {
                    font_size: 12.0,
                    color: color_u8(theme.text),
                    ..DrawOpts::default()
                };
                let message = crate::renderer::responsive::elide_end(
                    sugarloaf,
                    message,
                    (width - 16.0).max(0.0),
                    &options,
                );
                sugarloaf.text_mut().draw(
                    20.0,
                    (viewport.height - 53.0).max(0.0),
                    &message,
                    &options,
                );
            }
            return;
        }
        let layout = Self::layout(dimensions);
        let card = layout.card;
        sugarloaf.replace_modal_layer();

        rounded(
            sugarloaf,
            card.x + 7.0,
            card.y + 9.0,
            card.width,
            card.height,
            SHADOW,
            14.0,
        );
        rounded(
            sugarloaf,
            card.x,
            card.y,
            card.width,
            card.height,
            theme.outline,
            14.0,
        );
        rounded(
            sugarloaf,
            card.x + 1.0,
            card.y + 1.0,
            (card.width - 2.0).max(1.0),
            (card.height - 2.0).max(1.0),
            theme.background,
            13.0,
        );
        rounded(
            sugarloaf,
            card.x + 20.0,
            card.y + 17.0,
            36.0_f32.min((card.width - 40.0).max(1.0)),
            3.0,
            theme.accent,
            2.0,
        );

        let title = DrawOpts {
            font_size: if layout.compact { 15.0 } else { 18.0 },
            color: color_u8(theme.text),
            bold: true,
            ..DrawOpts::default()
        };
        let body = DrawOpts {
            font_size: 13.0,
            color: color_u8(theme.muted_text),
            ..DrawOpts::default()
        };
        let label = DrawOpts {
            font_size: if layout.compact { 12.0 } else { 13.0 },
            color: color_u8(theme.text),
            bold: true,
            ..DrawOpts::default()
        };
        let key = DrawOpts {
            font_size: 11.0,
            color: color_u8(theme.muted_text),
            bold: true,
            ..DrawOpts::default()
        };
        let text_x = card.x + if layout.compact { 12.0 } else { 24.0 };
        let title_y = card.y + if layout.compact { 17.0 } else { 31.0 };
        if !layout.tiny {
            sugarloaf.text_mut().draw(
                text_x,
                title_y,
                if self.recovery {
                    if self.loading {
                        "Preparing workspace…"
                    } else {
                        "Restore previous workspace?"
                    }
                } else {
                    "Close Automexia?"
                },
                &title,
            );
        }
        if !layout.compact {
            sugarloaf.text_mut().draw(
                text_x,
                title_y + 37.0,
                if self.recovery {
                    "Reopen your workspace and saved history."
                } else {
                    "All running sessions in this window will be closed."
                },
                &body,
            );
            sugarloaf.text_mut().draw(
                text_x,
                title_y + 62.0,
                if self.recovery {
                    "Commands and SSH connections will not resume."
                } else {
                    "This action cannot be undone."
                },
                &body,
            );
        }

        if self.loading {
            sugarloaf.end_modal_layer();
            return;
        }
        button(
            sugarloaf,
            layout.cancel,
            if self.hovered == Some(ConfirmQuitAction::Cancel) {
                theme.raised
            } else {
                theme.surface
            },
            theme.accent,
            if self.recovery {
                if layout.tiny {
                    "S"
                } else {
                    "Start clean"
                }
            } else if layout.tiny {
                "N"
            } else {
                "Cancel"
            },
            if layout.tiny {
                ""
            } else if self.recovery {
                "Esc / S"
            } else {
                "Esc / N"
            },
            &label,
            &key,
        );
        button(
            sugarloaf,
            layout.quit,
            if self.hovered == Some(ConfirmQuitAction::Quit) {
                theme.raised
            } else {
                theme.surface
            },
            if self.recovery {
                theme.accent
            } else {
                theme.danger
            },
            if self.recovery {
                if layout.tiny {
                    "R"
                } else {
                    "Restore"
                }
            } else if layout.tiny {
                "Y"
            } else {
                "Close"
            },
            if layout.tiny {
                ""
            } else if self.recovery {
                "R"
            } else {
                "Y"
            },
            &label,
            &key,
        );
        sugarloaf.end_modal_layer();
    }
}

fn rounded(
    sugarloaf: &mut Sugarloaf,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    color: [f32; 4],
    radius: f32,
) {
    sugarloaf.rounded_rect(None, x, y, width, height, color, 0.1, radius, ORDER);
}

#[allow(clippy::too_many_arguments)]
fn button(
    sugarloaf: &mut Sugarloaf,
    rect: Rect,
    fill: [f32; 4],
    outline: [f32; 4],
    text: &str,
    key: &str,
    text_opts: &DrawOpts,
    key_opts: &DrawOpts,
) {
    rounded(
        sugarloaf,
        rect.x,
        rect.y,
        rect.width,
        rect.height,
        outline,
        8.0,
    );
    rounded(
        sugarloaf,
        rect.x + 1.0,
        rect.y + 1.0,
        (rect.width - 2.0).max(1.0),
        (rect.height - 2.0).max(1.0),
        fill,
        7.0,
    );
    let text_width = sugarloaf.text_mut().measure(text, text_opts);
    let key_width = sugarloaf.text_mut().measure(key, key_opts);
    let label_gap = if key.is_empty() { 0.0 } else { 9.0 };
    let x = rect.x + ((rect.width - text_width - key_width - label_gap) * 0.5).max(2.0);
    let y = rect.y + ((rect.height - text_opts.font_size) * 0.5).max(2.0) - 1.0;
    let drawn = sugarloaf.text_mut().draw(x, y, text, text_opts);
    if !key.is_empty() {
        sugarloaf
            .text_mut()
            .draw(x + drawn + label_gap, y + 1.0, key, key_opts);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rio_window::{
        event::ElementState,
        keyboard::{Key, NamedKey},
    };

    #[test]
    fn finishing_manual_recovery_does_not_dismiss_an_unrelated_quit_dialog() {
        let mut dialog = ConfirmQuit::default();
        dialog.set_active(true);
        dialog.finish_recovery();
        assert!(dialog.active);
        assert!(!dialog.is_recovery());
    }
    #[test]
    fn recovery_completion_restores_normal_quit_buttons() {
        let mut dialog = ConfirmQuit::default();
        dialog.show_recovery(false);
        dialog.choose_recovery(true);
        dialog.finish_recovery();
        dialog.set_active(true);
        let dimensions = (1280.0, 720.0, 1.0);
        let layout = ConfirmQuit::layout(dimensions);
        assert_eq!(
            dialog.hit_test(layout.quit.x + 1.0, layout.quit.y + 1.0, dimensions),
            Some(ConfirmQuitAction::Quit)
        );
        assert_eq!(dialog.take_recovery_choice(), None);
    }

    #[test]
    fn recovery_loading_keys_cannot_activate_a_later_choice() {
        let mut dialog = ConfirmQuit::default();
        dialog.show_recovery(true);
        let key = Key::Character("r".into());
        assert!(dialog.recovery_key_event(&key, ElementState::Pressed, false));
        dialog.show_recovery(false);
        assert!(dialog.recovery_key_event(&key, ElementState::Released, false));
        assert_eq!(dialog.take_recovery_choice(), None);
    }

    #[test]
    fn recovery_keyboard_defaults_to_clean_and_requires_deliberate_release() {
        let mut dialog = ConfirmQuit::default();
        dialog.show_recovery(false);
        let enter = Key::Named(NamedKey::Enter);
        dialog.recovery_key_event(&enter, ElementState::Pressed, true);
        dialog.recovery_key_event(&enter, ElementState::Released, false);
        assert_eq!(dialog.take_recovery_choice(), None);
        dialog.recovery_key_event(&enter, ElementState::Pressed, false);
        assert_eq!(dialog.take_recovery_choice(), None);
        dialog.recovery_key_event(&enter, ElementState::Released, false);
        assert_eq!(dialog.take_recovery_choice(), Some(false));
        dialog.show_recovery(false);
        dialog.recovery_key_event(
            &Key::Named(NamedKey::Tab),
            ElementState::Pressed,
            false,
        );
        dialog.recovery_key_event(&enter, ElementState::Pressed, false);
        dialog.recovery_key_event(&enter, ElementState::Released, false);
        assert_eq!(dialog.take_recovery_choice(), Some(true));
    }

    #[test]
    fn layout_stays_inside_extreme_viewports() {
        for d in [
            (80.0, 60.0, 1.0),
            (320.0, 180.0, 1.0),
            (1920.0, 1080.0, 1.0),
            (7680.0, 4320.0, 2.0),
        ] {
            let viewport = Viewport::from_physical(d.0, d.1, d.2);
            let l = ConfirmQuit::layout(d);
            assert!(l.card.x >= 0.0 && l.card.y >= 0.0);
            assert!(l.card.x + l.card.width <= viewport.width + f32::EPSILON);
            assert!(l.card.y + l.card.height <= viewport.height + f32::EPSILON);
            assert!(l.cancel.x + l.cancel.width <= l.quit.x);
            assert!(l.quit.x + l.quit.width <= l.card.x + l.card.width);
            assert!(l.quit.y + l.quit.height <= l.card.y + l.card.height);
        }
    }

    #[test]
    fn hit_test_requires_an_active_explicit_button() {
        let d = (1280.0, 720.0, 1.0);
        let mut dialog = ConfirmQuit::default();
        assert_eq!(dialog.hit_test(0.0, 0.0, d), None);
        dialog.set_active(true);
        let l = ConfirmQuit::layout(d);
        assert_eq!(
            dialog.hit_test(l.cancel.x + 1.0, l.cancel.y + 1.0, d),
            Some(ConfirmQuitAction::Cancel)
        );
        assert_eq!(
            dialog.hit_test(l.quit.x + 1.0, l.quit.y + 1.0, d),
            Some(ConfirmQuitAction::Quit)
        );
        assert_eq!(dialog.hit_test(l.card.x, l.card.y, d), None);
    }

    #[test]
    fn hover_tracks_buttons_and_clears_when_hidden() {
        let dimensions = (1_280.0, 720.0, 1.0);
        let mut dialog = ConfirmQuit::default();
        dialog.set_active(true);
        let layout = ConfirmQuit::layout(dimensions);
        assert!(dialog.hover(layout.cancel.x + 1.0, layout.cancel.y + 1.0, dimensions));
        assert_eq!(dialog.hovered_action(), Some(ConfirmQuitAction::Cancel));
        assert!(dialog.hover(layout.quit.x + 1.0, layout.quit.y + 1.0, dimensions));
        assert_eq!(dialog.hovered_action(), Some(ConfirmQuitAction::Quit));
        dialog.set_active(false);
        assert_eq!(dialog.hovered_action(), None);
    }

    #[test]
    fn modal_surfaces_remain_opaque_over_live_terminal() {
        for entry in crate::automexia::theme_gallery::builtins() {
            let theme = UiTheme::from_colors(&entry.theme.unwrap().colors);
            for surface in [theme.background, theme.surface, theme.raised] {
                assert_eq!(surface[3], 1.0);
                assert!(automexia_ui_model::contrast_ratio(theme.text, surface) >= 4.5);
            }
        }
    }
}
