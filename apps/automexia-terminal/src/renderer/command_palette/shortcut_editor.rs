use super::*;
use crate::bindings::shortcut;
use automexia_keybindings::Trigger;
use rio_window::keyboard::{Key, ModifiersState, NamedKey};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ShortcutChange {
    pub action: PaletteAction,
    pub trigger: Option<Trigger>,
}

pub(super) struct ShortcutEditor {
    pub(super) action: PaletteAction,
    pub(super) title: &'static str,
    pub(super) current: String,
    candidate: Option<Trigger>,
    pub(super) error: Option<String>,
    pub(super) message: &'static str,
    pub(super) focus: usize,
    saving: Option<u64>,
    saved: bool,
    reset_requested: bool,
}

impl CommandPalette {
    pub fn is_editing_shortcut(&self) -> bool {
        self.shortcut_editor.is_some()
    }

    pub fn begin_shortcut_edit(&mut self) -> bool {
        let Some(action) = self.get_selected_action() else {
            return false;
        };
        let Some(command) = COMMANDS.iter().find(|command| command.action == action)
        else {
            return false;
        };
        self.shortcut_editor = Some(ShortcutEditor {
            action,
            title: command.title,
            current: self.command_shortcut(command).into(),
            candidate: None,
            error: None,
            message: "Press a new shortcut",
            focus: 0,
            saving: None,
            saved: false,
            reset_requested: false,
        });
        self.reset_scroll_gesture();
        self.shortcut_click = None;
        true
    }

    pub fn interrupt_shortcut_capture(&mut self) {
        if let Some(editor) = &mut self.shortcut_editor {
            if editor.saving.is_none() {
                editor.candidate = None;
                editor.error = None;
                editor.message = "Recording paused; press a new shortcut to resume";
                editor.saved = false;
                editor.reset_requested = false;
            }
        }
        self.shortcut_click = None;
    }

    pub(crate) fn has_shortcut_change(&self) -> bool {
        self.shortcut_change.is_some()
    }

    pub(super) fn refresh_shortcut_current(&mut self) {
        let Some(action) = self.shortcut_editor.as_ref().map(|editor| editor.action)
        else {
            return;
        };
        let Some(command) = COMMANDS.iter().find(|command| command.action == action)
        else {
            return;
        };
        let current = self.command_shortcut(command).to_owned();
        if let Some(editor) = &mut self.shortcut_editor {
            // An applied save uses Some(0) until it receives its writer receipt.
            // Another window/config update invalidates a previous receipt's UI.
            if editor.saving.is_some_and(|revision| revision > 0) {
                editor.saving = None;
                editor.candidate = None;
                editor.saved = false;
                editor.message =
                    "Bindings changed; press a new shortcut to edit the current value";
            }
            editor.current = current;
        }
    }

    pub(crate) fn take_shortcut_change(&mut self) -> Option<ShortcutChange> {
        self.shortcut_change.take()
    }

    pub(crate) fn shortcut_save_started(&mut self, revision: u64) {
        if let Some(editor) = &mut self.shortcut_editor {
            editor.saving = Some(revision);
            editor.error = None;
            editor.message = "Applied for this session; saving...";
        }
    }

    pub(crate) fn shortcut_save_failed(&mut self, message: &str) {
        if let Some(editor) = &mut self.shortcut_editor {
            editor.saving = None;
            editor.saved = false;
            editor.error = Some(message.into());
            editor.message = "Press Enter to retry, or Esc to return";
        }
    }

    pub(crate) fn shortcut_write_finished(
        &mut self,
        revision: u64,
        success: bool,
    ) -> bool {
        let Some(editor) = &mut self.shortcut_editor else {
            return false;
        };
        // Coalesced later snapshots include the current overlay, but an earlier
        // completion must never certify a newer edit as durable.
        if !editor
            .saving
            .is_some_and(|expected| expected != 0 && revision >= expected)
        {
            return false;
        }
        editor.saving = None;
        editor.saved = success;
        editor.message = if success {
            "Saved · applies to new sessions too"
        } else {
            "Active this session only; press Enter to retry"
        };
        editor.error = (!success)
            .then(|| "Could not save settings; check permissions or free space".into());
        true
    }

    pub(super) fn edit_shortcut_key(
        &mut self,
        key: &Key,
        mods: ModifiersState,
        repeat: bool,
    ) -> bool {
        if repeat {
            return true;
        }
        if *key == Key::Named(NamedKey::Escape) && mods.is_empty() {
            self.shortcut_editor = None;
            self.shortcut_change = None;
            return true;
        }
        let Some(editor) = &mut self.shortcut_editor else {
            return false;
        };
        if editor.saving.is_some() {
            return true;
        }
        if *key == Key::Named(NamedKey::Tab)
            && (mods.is_empty() || mods == ModifiersState::SHIFT)
        {
            editor.focus = (editor.focus + if mods.shift_key() { 3 } else { 1 }) % 4;
        } else if *key == Key::Named(NamedKey::Enter) && mods.is_empty() {
            let control = if editor.reset_requested && editor.focus == 0 {
                2
            } else {
                editor.focus
            };
            if editor.saved || control == 3 {
                self.shortcut_editor = None;
            } else {
                self.activate_shortcut_control(control);
            }
        } else {
            match shortcut::capture(key, mods) {
                Ok(Some(trigger)) => {
                    editor.error = shortcut::conflict(
                        editor.action,
                        &trigger,
                        &self.edit_bindings,
                        self.edit_registry.as_ref(),
                    );
                    editor.message = shortcut::system_warning(&trigger)
                        .unwrap_or("Enter saves · Esc cancels · Tab selects controls");
                    editor.candidate = Some(trigger);
                    editor.focus = 0;
                    editor.saved = false;
                    editor.reset_requested = false;
                }
                Ok(None) => {}
                Err(error) => {
                    editor.candidate = None;
                    editor.error = Some(error.into());
                    editor.saved = false;
                    editor.reset_requested = false;
                    editor.focus = 0;
                }
            }
        }
        true
    }

    fn activate_shortcut_control(&mut self, control: usize) {
        let Some(editor) = &mut self.shortcut_editor else {
            return;
        };
        if control == 3 {
            self.shortcut_editor = None;
            return;
        }
        if editor.saving.is_some() {
            return;
        }
        if control == 1 && editor.saved {
            self.shortcut_editor = None;
            return;
        }
        let reset = control == 2 || editor.reset_requested;
        let trigger = if reset {
            None
        } else {
            let Some(trigger) = &editor.candidate else {
                editor.message = "Press a shortcut before saving";
                return;
            };
            if let Some(error) = shortcut::conflict(
                editor.action,
                trigger,
                &self.edit_bindings,
                self.edit_registry.as_ref(),
            ) {
                editor.error = Some(error);
                return;
            }
            Some(trigger.clone())
        };
        editor.reset_requested = reset;
        editor.focus = control;
        if reset {
            editor.candidate = None;
        }
        self.shortcut_change = Some(ShortcutChange {
            action: editor.action,
            trigger,
        });
        editor.saving = Some(0);
        editor.message = "Checking shortcut...";
        editor.error = None;
    }

    pub(super) fn shortcut_editor_geometry(
        dimensions: (f32, f32, f32),
    ) -> ([f32; 4], [[f32; 4]; 3]) {
        let viewport = Viewport::from_physical(dimensions.0, dimensions.1, dimensions.2);
        let width = viewport.fitted_surface(520.0, 8.0).min(viewport.width);
        let height = (viewport.height - 16.0).clamp(0.0, 270.0);
        let x = ((viewport.width - width) / 2.0).max(0.0);
        let y = ((viewport.height - height) / 2.0).max(0.0);
        let padding = 12.0_f32.min(width / 8.0);
        let gap = 8.0_f32.min(width / 12.0);
        let button_width = ((width - padding * 2.0 - gap * 2.0) / 3.0).max(0.0);
        let button_height = 32.0_f32.min(height);
        let buttons = std::array::from_fn(|i| {
            [
                x + padding + i as f32 * (button_width + gap),
                y + height
                    - button_height
                    - padding.min((height - button_height).max(0.0)),
                button_width,
                button_height,
            ]
        });
        ([x, y, width, height], buttons)
    }

    pub fn shortcut_editor_click(
        &mut self,
        x: f32,
        y: f32,
        dimensions: (f32, f32, f32),
    ) -> bool {
        if !self.is_editing_shortcut() {
            return false;
        }
        let (_, buttons) = Self::shortcut_editor_geometry(dimensions);
        for (i, rect) in buttons.into_iter().enumerate() {
            if inside(x, y, rect) {
                self.activate_shortcut_control(i + 1);
                break;
            }
        }
        // Outside presses never fall through, dismiss an unsaved edit, or run a row.
        true
    }

    pub fn shortcut_badge_click(
        &mut self,
        sugarloaf: &mut Sugarloaf,
        x: f32,
        y: f32,
        dimensions: (f32, f32, f32),
        double: bool,
    ) -> bool {
        let Ok(Some(index)) =
            self.hit_test(x, y, dimensions.0, dimensions.1, dimensions.2)
        else {
            self.shortcut_click = None;
            return false;
        };
        let rows = self.filtered_rows();
        let Some((_, row @ PaletteRow::Command { action, .. })) = rows.get(index) else {
            self.shortcut_click = None;
            return false;
        };
        let action = *action;
        let (px, py, pw, _, visible) = self.palette_rect_for_count(
            dimensions.0,
            dimensions.1,
            dimensions.2,
            rows.len(),
        );
        let offset = bounded_scroll_offset(rows.len(), visible, self.scroll_offset);
        let item_y = py
            + PALETTE_PADDING
            + INPUT_HEIGHT
            + SEPARATOR_HEIGHT
            + RESULTS_MARGIN_TOP
            + index.saturating_sub(offset) as f32 * RESULT_ITEM_HEIGHT;
        let rect = shortcut_badge_rect(
            sugarloaf,
            row.shortcut(),
            px + PALETTE_PADDING,
            pw - PALETTE_PADDING * 2.0,
            item_y,
        );
        if !inside(x, y, rect) {
            self.shortcut_click = None;
            return false;
        }
        self.register_shortcut_badge_press(action, index, x, y, double);
        true
    }

    fn register_shortcut_badge_press(
        &mut self,
        action: PaletteAction,
        index: usize,
        x: f32,
        y: f32,
        double: bool,
    ) {
        self.selected_index = index;
        let matching = self.shortcut_click.is_some_and(|(previous, old_x, old_y)| {
            previous == action && (old_x - x).abs() <= 4.0 && (old_y - y).abs() <= 4.0
        });
        self.shortcut_click = Some((action, x, y));
        if double && matching {
            self.begin_shortcut_edit();
        }
    }

    pub(super) fn render_shortcut_editor(
        &self,
        sugarloaf: &mut Sugarloaf,
        dimensions: (f32, f32, f32),
    ) {
        let Some(editor) = &self.shortcut_editor else {
            return;
        };
        let viewport = Viewport::from_physical(dimensions.0, dimensions.1, dimensions.2);
        let ([x, y, w, h], buttons) = Self::shortcut_editor_geometry(dimensions);
        sugarloaf.begin_modal_layer();
        sugarloaf.rect(
            None,
            0.0,
            0.0,
            viewport.width,
            viewport.height,
            BACKDROP_COLOR,
            DEPTH_BACKDROP,
            ORDER,
        );
        sugarloaf.rounded_rect(None, x, y, w, h, BG_COLOR, DEPTH_BG, CARD_RADIUS, ORDER);
        let candidate = editor
            .candidate
            .as_ref()
            .map_or_else(|| "Press shortcut keys".into(), ToString::to_string);
        let current = format!("Current: {}", editor.current);
        let lines = [
            ("Edit shortcut", 18.0, TEXT_COLOR),
            (editor.title, 14.0, DIM_TEXT_COLOR),
            (current.as_str(), 11.0, DIM_TEXT_COLOR),
            (candidate.as_str(), 18.0, BRAND_CYAN),
            (
                editor.error.as_deref().unwrap_or(editor.message),
                11.0,
                if editor.error.is_some() {
                    BRAND_CORAL
                } else {
                    DIM_TEXT_COLOR
                },
            ),
            (
                "Enter saves · Esc returns · Tab moves · Reset restores config",
                10.0,
                DIM_TEXT_COLOR,
            ),
        ];
        let mut ly = y + 14.0;
        for (text, size, color) in lines {
            if ly + size + 5.0 > buttons[0][1] {
                break;
            }
            let opts = DrawOpts {
                font_size: size,
                color: color_u8(color),
                ..DrawOpts::default()
            };
            let text = elide_end(sugarloaf, text, (w - 28.0).max(0.0), &opts);
            sugarloaf.text_mut().draw(x + 14.0, ly, &text, &opts);
            ly += size + 14.0;
        }
        for (i, [bx, by, bw, bh]) in buttons.into_iter().enumerate() {
            let active = editor.focus == i + 1 || (editor.focus == 0 && i == 0);
            sugarloaf.rounded_rect(
                None,
                bx,
                by,
                bw,
                bh,
                if active {
                    SELECTED_BG_COLOR
                } else {
                    INPUT_BG_COLOR
                },
                DEPTH_ELEMENT,
                CONTROL_RADIUS,
                ORDER,
            );
            if bw < 20.0 || bh < 20.0 {
                continue;
            }
            let opts = DrawOpts {
                font_size: 12.0,
                color: color_u8(if active { BRAND_CYAN } else { TEXT_COLOR }),
                ..DrawOpts::default()
            };
            let label = if i == 0 {
                if editor.saving.is_some() {
                    "Saving..."
                } else if editor.saved {
                    "Done"
                } else if editor.reset_requested {
                    "Retry reset"
                } else {
                    "Save"
                }
            } else if i == 1 {
                "Reset"
            } else if editor.saving.is_some() || editor.saved {
                "Back"
            } else {
                "Cancel"
            };
            let label = elide_end(sugarloaf, label, (bw - 12.0).max(0.0), &opts);
            sugarloaf
                .text_mut()
                .draw(bx + 6.0, by + (bh - 12.0) / 2.0, &label, &opts);
        }
        sugarloaf.end_modal_layer();
    }
}

fn inside(x: f32, y: f32, rect: [f32; 4]) -> bool {
    x.is_finite()
        && y.is_finite()
        && x >= rect[0]
        && x < rect[0] + rect[2]
        && y >= rect[1]
        && y < rect[1] + rect[3]
}

pub(super) fn shortcut_badge_rect(
    sugarloaf: &mut Sugarloaf,
    shortcut: &str,
    input_x: f32,
    input_width: f32,
    item_y: f32,
) -> [f32; 4] {
    let opts = DrawOpts {
        font_size: SHORTCUT_FONT_SIZE,
        ..DrawOpts::default()
    };
    let label = elide_end(
        sugarloaf,
        shortcut,
        trailing_label_max_width(input_width),
        &opts,
    );
    let width = sugarloaf.text_mut().measure(&label, &opts) + 18.0;
    [
        input_x + input_width - 10.0 - width,
        item_y + 10.0,
        width,
        24.0,
    ]
}
#[cfg(test)]
mod tests {
    use super::*;

    fn palette() -> CommandPalette {
        let config = rio_backend::config::Config::default();
        let bindings = crate::bindings::default_key_bindings(&config);
        let mut palette = CommandPalette::new();
        palette.set_effective_bindings(&bindings, None);
        palette.set_enabled(true);
        palette.set_query("Clone Active Session Right".into());
        assert!(palette.begin_shortcut_edit());
        palette
    }

    #[test]
    fn queued_save_is_invalidated_by_binding_reload_and_pointer_back_is_safe_while_saving(
    ) {
        let mut p = palette();
        key(
            &mut p,
            NamedKey::F9,
            ModifiersState::CONTROL | ModifiersState::SHIFT,
        );
        key_enter(&mut p);
        p.set_effective_bindings(
            &crate::bindings::default_key_bindings(
                &rio_backend::config::Config::default(),
            ),
            None,
        );
        assert!(p.take_shortcut_change().is_none());
        key_enter(&mut p);
        assert!(p.take_shortcut_change().is_none());
        p.shortcut_save_started(2);
        p.activate_shortcut_control(3);
        assert!(!p.is_editing_shortcut());
        assert!(!p.shortcut_write_finished(2, true));
        assert!(p.is_enabled());
    }

    #[test]
    fn badge_double_click_requires_same_action_position_and_unchanged_list() {
        let mut p = palette();
        key(&mut p, NamedKey::Escape, ModifiersState::empty());
        let action = PaletteAction::CloneSplitRight;
        p.register_shortcut_badge_press(action, 0, 40.0, 30.0, false);
        assert!(!p.is_editing_shortcut());
        assert!(!p.has_shortcut_change());
        p.register_shortcut_badge_press(action, 0, 70.0, 30.0, true);
        assert!(!p.is_editing_shortcut());
        p.set_query("Clone Active Session Right".into());
        p.register_shortcut_badge_press(action, 0, 70.0, 30.0, true);
        assert!(!p.is_editing_shortcut());
        p.register_shortcut_badge_press(action, 0, 70.0, 30.0, true);
        assert!(p.is_editing_shortcut());
        assert!(p.get_selected_action().is_none());
    }

    #[test]
    #[ignore = "explicit bounded benchmark, not a native interaction latency claim"]
    fn benchmark_shortcut_record_validate_and_cancel() {
        use criterion::Criterion;
        use std::{hint::black_box, time::Duration};
        let report = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/qa/shortcut-editor/criterion");
        let mut criterion = Criterion::default()
            .without_plots()
            .output_directory(&report)
            .sample_size(30)
            .warm_up_time(Duration::from_secs(1))
            .measurement_time(Duration::from_secs(3));
        let mut p = palette();
        key(&mut p, NamedKey::Escape, ModifiersState::empty());
        criterion.bench_function("shortcut_record_validate_cancel", |b| {
            b.iter(|| {
                assert!(p.begin_shortcut_edit());
                key(
                    &mut p,
                    NamedKey::F9,
                    black_box(ModifiersState::CONTROL | ModifiersState::SHIFT),
                );
                assert!(p.shortcut_editor.as_ref().unwrap().error.is_none());
                assert!(!p.has_shortcut_change());
                key(&mut p, NamedKey::Escape, ModifiersState::empty());
                assert!(!p.is_editing_shortcut());
            })
        });
        criterion.final_summary();
    }
    fn key(palette: &mut CommandPalette, key: NamedKey, mods: ModifiersState) {
        assert!(palette.handle_navigation_key(&Key::Named(key), mods, false));
    }

    #[test]
    fn capture_review_save_and_repeat_never_activate_a_command_or_destroy_the_query() {
        let mut p = palette();
        assert!(p.get_selected_action().is_none());
        key(
            &mut p,
            NamedKey::F9,
            ModifiersState::CONTROL | ModifiersState::SHIFT,
        );
        assert!(p.take_shortcut_change().is_none());
        assert!(p.handle_navigation_key(
            &Key::Named(NamedKey::Enter),
            ModifiersState::empty(),
            true
        ));
        assert!(p.take_shortcut_change().is_none());
        key(&mut p, NamedKey::Enter, ModifiersState::empty());
        let change = p.take_shortcut_change().unwrap();
        assert_eq!(change.action, PaletteAction::CloneSplitRight);
        assert_eq!(change.trigger.unwrap().to_string(), "ctrl+shift+f9");
        key(&mut p, NamedKey::Enter, ModifiersState::empty());
        assert!(p.take_shortcut_change().is_none());
        p.shortcut_save_started(5);
        assert!(!p.shortcut_write_finished(4, true));
        assert!(p.shortcut_write_finished(5, true));
        key(&mut p, NamedKey::Escape, ModifiersState::empty());
        assert!(p.is_enabled());
        assert!(!p.is_editing_shortcut());
        assert_eq!(p.query, "Clone Active Session Right");
    }

    #[test]
    fn recording_rejects_text_conflicts_and_modifier_only_without_saving() {
        let mut p = palette();
        for (key, mods) in [
            (Key::Character("r".into()), ModifiersState::CONTROL),
            (Key::Character("a".into()), ModifiersState::empty()),
            (
                Key::Character("p".into()),
                ModifiersState::CONTROL | ModifiersState::SHIFT,
            ),
            (Key::Dead(Some('^')), ModifiersState::empty()),
        ] {
            assert!(p.handle_navigation_key(&key, mods, false));
            key_enter(&mut p);
            assert!(p.take_shortcut_change().is_none());
        }
        let mut p = palette();
        key(&mut p, NamedKey::Control, ModifiersState::CONTROL);
        key_enter(&mut p);
        assert!(p.take_shortcut_change().is_none());
    }
    fn key_enter(p: &mut CommandPalette) {
        key(p, NamedKey::Enter, ModifiersState::empty());
    }

    #[test]
    fn keyboard_and_pointer_reset_match_and_failed_reset_retries_reset_not_an_old_chord()
    {
        let mut keyboard = palette();
        key(
            &mut keyboard,
            NamedKey::F9,
            ModifiersState::CONTROL | ModifiersState::SHIFT,
        );
        key(&mut keyboard, NamedKey::Tab, ModifiersState::empty());
        key(&mut keyboard, NamedKey::Tab, ModifiersState::empty());
        key_enter(&mut keyboard);
        let expected = keyboard.take_shortcut_change().unwrap();
        assert!(expected.trigger.is_none());
        let mut pointer = palette();
        let (_, buttons) = CommandPalette::shortcut_editor_geometry((800.0, 600.0, 1.0));
        assert!(pointer.shortcut_editor_click(
            buttons[1][0] + 1.0,
            buttons[1][1] + 1.0,
            (800.0, 600.0, 1.0)
        ));
        assert_eq!(pointer.take_shortcut_change(), Some(expected.clone()));
        keyboard.shortcut_save_started(10);
        assert!(keyboard.shortcut_write_finished(10, false));
        key_enter(&mut keyboard);
        assert_eq!(keyboard.take_shortcut_change(), Some(expected));
    }

    #[test]
    fn pointer_reset_retry_preserves_intent_from_every_keyboard_focus() {
        let dimensions = (800.0, 600.0, 1.0);
        let (_, buttons) = CommandPalette::shortcut_editor_geometry(dimensions);
        for focus in 0..4 {
            for retry in 0..3 {
                let mut p = palette();
                key(
                    &mut p,
                    NamedKey::F9,
                    ModifiersState::CONTROL | ModifiersState::SHIFT,
                );
                for _ in 0..focus {
                    key(&mut p, NamedKey::Tab, ModifiersState::empty());
                }
                p.shortcut_editor_click(
                    buttons[1][0] + 1.0,
                    buttons[1][1] + 1.0,
                    dimensions,
                );
                let expected = p.take_shortcut_change().unwrap();
                assert!(expected.trigger.is_none());
                p.shortcut_save_started(10);
                assert!(p.shortcut_write_finished(10, false));
                // A failed reset must retain its intent even when retried via
                // the primary button instead of the originally clicked control.
                if retry == 0 {
                    key_enter(&mut p);
                } else {
                    let button = buttons[retry - 1];
                    p.shortcut_editor_click(button[0] + 1.0, button[1] + 1.0, dimensions);
                }
                assert_eq!(
                    p.take_shortcut_change(),
                    Some(expected),
                    "focus {focus}, retry {retry}"
                );
                assert!(p.take_shortcut_change().is_none());
            }
        }
    }

    #[test]
    fn capture_focus_loss_ime_and_config_reload_discard_draft_and_wheel_cannot_navigate()
    {
        let mut p = palette();
        key(
            &mut p,
            NamedKey::F9,
            ModifiersState::CONTROL | ModifiersState::SHIFT,
        );
        assert!(!p.scroll_line_delta(-100.0));
        assert!(!p.scroll_pixel_delta(-500.0));
        p.interrupt_shortcut_capture();
        key_enter(&mut p);
        assert!(p.take_shortcut_change().is_none());
        key(
            &mut p,
            NamedKey::F9,
            ModifiersState::CONTROL | ModifiersState::SHIFT,
        );
        p.set_effective_bindings(
            &crate::bindings::default_key_bindings(
                &rio_backend::config::Config::default(),
            ),
            None,
        );
        key_enter(&mut p);
        assert!(p.take_shortcut_change().is_none());
        assert_eq!(p.query, "Clone Active Session Right");
    }

    #[test]
    fn editor_geometry_and_button_targets_stay_within_tiny_to_8k_viewports() {
        for (w, h) in [
            (1.0, 1.0),
            (40.0, 30.0),
            (240.0, 180.0),
            (800.0, 600.0),
            (7680.0, 4320.0),
        ] {
            for scale in [1.0, 1.25, 2.0, 4.0] {
                let dims = (w, h, scale);
                let viewport = Viewport::from_physical(w, h, scale);
                let (rect, buttons) = CommandPalette::shortcut_editor_geometry(dims);
                assert!(rect.into_iter().all(f32::is_finite));
                assert!(
                    rect[0] >= 0.0
                        && rect[1] >= 0.0
                        && rect[0] + rect[2] <= viewport.width + 0.001
                        && rect[1] + rect[3] <= viewport.height + 0.001
                );
                for button in buttons {
                    assert!(
                        button[0] >= rect[0]
                            && button[0] + button[2] <= rect[0] + rect[2] + 0.001
                    );
                    assert!(
                        button[1] >= rect[1]
                            && button[1] + button[3] <= rect[1] + rect[3] + 0.001
                    );
                }
            }
        }
    }
}
