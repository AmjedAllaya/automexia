//! Shared native color editor; hosts supply validated descriptors and own effects.
use super::*;

const PALETTE_COLUMNS: usize = 8;
const PALETTE_HEIGHT: f32 = 148.0;

fn hex_color([r, g, b, a]: [u8; 4], alpha: bool) -> String {
    if alpha {
        format!("#{r:02X}{g:02X}{b:02X}{a:02X}")
    } else {
        format!("#{r:02X}{g:02X}{b:02X}")
    }
}

impl SettingsView {
    pub(crate) fn open_tab_color_editor(
        &mut self,
        target: crate::context::TabColorTarget,
    ) {
        self.standalone_color_route = Some(target);
        self.focus = Focus::List;
        self.open_color();
    }

    pub(crate) fn take_tab_color_edit(
        &mut self,
    ) -> Option<(crate::context::TabColorTarget, Option<[u8; 4]>)> {
        let target = self.standalone_color_route.as_ref()?.clone();
        let edit = self.pending.take()?;
        let color = match edit.change {
            Change::Set(SettingValue::Color(color))
                if edit.id.as_str() == "tab.color" =>
            {
                Some(color)
            }
            Change::Reset if edit.id.as_str() == "tab.color" => None,
            _ => return None,
        };
        self.close();
        Some((target, color))
    }

    pub(crate) fn set_color_favorites(&mut self, favorites: &[[u8; 4]]) {
        if favorites == self.color_favorites {
            return;
        }
        let mut bounded = Vec::with_capacity(16);
        for color in favorites
            .iter()
            .take(crate::automexia::preferences::MAX_COLOR_FAVORITES)
        {
            if !bounded.contains(color) {
                bounded.push(*color);
            }
        }
        if bounded != self.color_favorites {
            let previous = self.focused_palette_color();
            self.color_favorites = bounded;
            self.pressed = None;
            self.layout_dirty = true;
            if self.color_editor.as_ref().is_some_and(|e| e.show_favorites) {
                self.reconcile_palette_focus(previous);
            }
        }
    }

    pub(crate) fn take_color_favorite_intent(&mut self) -> Option<ColorFavoriteIntent> {
        self.pending_color_favorite.take()
    }

    fn update_color_suggestions(&mut self, theme: UiTheme) {
        let mut colors = Vec::with_capacity(16);
        for color in [
            theme.background,
            theme.surface,
            theme.text,
            theme.muted_text,
            theme.accent,
            theme.blue,
            theme.purple,
            theme.success,
            theme.warning,
            theme.danger,
        ]
        .map(color_u8)
        .into_iter()
        .chain([
            [255, 255, 255, 255],
            [18, 24, 32, 255],
            [255, 153, 102, 255],
            [247, 128, 179, 255],
            [93, 211, 193, 255],
            [196, 213, 125, 255],
        ]) {
            if !colors.contains(&color) {
                colors.push(color);
            }
        }
        if colors != self.color_suggestions {
            let previous = self.focused_palette_color();
            self.color_suggestions = colors;
            self.pressed = None;
            if self
                .color_editor
                .as_ref()
                .is_some_and(|e| !e.show_favorites)
            {
                self.reconcile_palette_focus(previous);
            }
        }
    }

    fn focused_palette_color(&self) -> Option<[u8; 4]> {
        let ColorFocus::Swatch(index) = self.color_editor.as_ref()?.focus else {
            return None;
        };
        self.color_palette_colors().get(index).copied()
    }

    fn reconcile_palette_focus(&mut self, previous: Option<[u8; 4]>) {
        let colors = self.color_palette_colors();
        let Some(editor) = &mut self.color_editor else {
            return;
        };
        if !editor.palette_browsing {
            return;
        }
        let index = previous
            .and_then(|color| colors.iter().position(|c| *c == color))
            .unwrap_or_else(|| match editor.focus {
                ColorFocus::Swatch(index) => index.min(colors.len().saturating_sub(1)),
                _ => 0,
            });
        editor.focus = if colors.is_empty() {
            if editor.show_favorites {
                ColorFocus::Favorites
            } else {
                ColorFocus::Suggested
            }
        } else {
            ColorFocus::Swatch(index)
        };
    }

    pub(super) fn color_palette_colors(&self) -> Vec<[u8; 4]> {
        let Some(editor) = &self.color_editor else {
            return Vec::new();
        };
        if editor.text_limits.is_some() || self.color_geometry.palette.height < 120.0 {
            return Vec::new();
        }
        let colors = if editor.show_favorites {
            &self.color_favorites
        } else {
            &self.color_suggestions
        };
        colors
            .iter()
            .copied()
            .filter(|color| editor.alpha || color[3] == 255)
            .take(16)
            .collect()
    }

    pub(super) fn color_palette_controls(&self) -> Vec<(ColorFocus, Rect)> {
        if self
            .color_editor
            .as_ref()
            .is_none_or(|e| e.text_limits.is_some())
        {
            return Vec::new();
        }
        let area = self.color_geometry.palette;
        if area.height < 120.0 {
            return Vec::new();
        }
        let group_width = (area.width - 16.0) / 3.0;
        let mut controls = vec![
            (
                ColorFocus::Suggested,
                Rect {
                    width: group_width,
                    height: 26.0,
                    ..area
                },
            ),
            (
                ColorFocus::Favorites,
                Rect {
                    x: area.x + group_width + 8.0,
                    width: group_width,
                    height: 26.0,
                    ..area
                },
            ),
        ];
        controls.push((
            ColorFocus::FavoriteToggle,
            Rect {
                x: area.x + (group_width + 8.0) * 2.0,
                width: group_width,
                height: 26.0,
                ..area
            },
        ));
        let cell = (area.width - 6.0 * 7.0) / PALETTE_COLUMNS as f32;
        for index in 0..self.color_palette_colors().len() {
            controls.push((
                ColorFocus::Swatch(index),
                Rect {
                    x: area.x + (index % PALETTE_COLUMNS) as f32 * (cell + 6.0),
                    y: area.y + 34.0 + (index / PALETTE_COLUMNS) as f32 * 34.0,
                    width: cell,
                    height: 28.0,
                },
            ));
        }
        controls
    }

    pub(super) fn color_palette_target(&self, x: f32, y: f32) -> Option<ColorFocus> {
        self.color_palette_controls()
            .into_iter()
            .find(|(focus, rect)| {
                rect.contains(x, y)
                    && (*focus != ColorFocus::FavoriteToggle
                        || self.color_editor.as_ref().and_then(editor_value).is_some())
            })
            .map(|(focus, _)| focus)
    }

    pub(super) fn activate_color_palette(&mut self, focus: ColorFocus) {
        let colors = self.color_palette_colors();
        let Some(editor) = &mut self.color_editor else {
            return;
        };
        if editor.text_limits.is_some() || editor.composing {
            return;
        }
        match focus {
            ColorFocus::FavoriteToggle => {
                let Some(SettingValue::Color(color)) = editor_value(editor) else {
                    return;
                };
                self.pending_color_favorite =
                    Some(if self.color_favorites.contains(&color) {
                        ColorFavoriteIntent::Forget(color)
                    } else {
                        ColorFavoriteIntent::Remember(color)
                    });
                if !editor.palette_browsing {
                    editor.focus = focus;
                }
            }
            ColorFocus::Suggested | ColorFocus::Favorites => {
                editor.show_favorites = focus == ColorFocus::Favorites;
                editor.focus = focus;
                editor.palette_browsing = true;
            }
            ColorFocus::Swatch(index) => {
                let Some(color) = colors.get(index).copied() else {
                    return;
                };
                editor.draft = hex_color(color, editor.alpha);
                editor.caret = editor.draft.len();
                editor.anchor = Some(0);
                editor.last_valid_color = Some(color);
                editor.custom_input = false;
                editor.feedback = None;
                editor.focus = ColorFocus::Hex;
                editor.palette_browsing = false;
            }
            _ => return,
        }
        self.reconcile_palette_focus(None);
        self.preedit.clear();
        self.pressed = None;
        self.layout_dirty = true;
    }

    fn color_palette_key(
        &mut self,
        key: &Key,
        modifiers: ModifiersState,
        repeat: bool,
    ) -> bool {
        let Some(editor) = &self.color_editor else {
            return false;
        };
        if editor.text_limits.is_some() || self.color_geometry.palette.height < 120.0 {
            return false;
        }
        if modifiers.is_empty() {
            let shortcut = match key {
                Key::Named(NamedKey::F1) => Some(ColorFocus::Suggested),
                Key::Named(NamedKey::F2) => Some(ColorFocus::Favorites),
                Key::Named(NamedKey::F3) => Some(ColorFocus::FavoriteToggle),
                _ => None,
            };
            if let Some(focus) = shortcut {
                if !repeat {
                    self.activate_color_palette(focus);
                }
                return true;
            }
        }
        let colors = self.color_palette_colors();
        if key == &Key::Named(NamedKey::ArrowDown)
            && modifiers.is_empty()
            && editor.focus == ColorFocus::Hex
            && !colors.is_empty()
        {
            if let Some(editor) = &mut self.color_editor {
                editor.focus = ColorFocus::Swatch(0);
                editor.palette_browsing = true;
            }
            self.pressed = None;
            return true;
        }
        if editor.palette_browsing {
            // Palette navigation is its own focus scope, including an empty
            // favorites list. Only Escape or choosing a swatch ends keyboard
            // browsing; Apply/Reset mnemonics cannot leak through from here.
            let index = match editor.focus {
                ColorFocus::Swatch(i) => i,
                _ => 0,
            };
            if !modifiers.is_empty() && modifiers != ModifiersState::SHIFT {
                return true;
            }
            if modifiers.is_empty()
                && matches!(key, Key::Named(NamedKey::Enter | NamedKey::Space))
            {
                if !repeat && !colors.is_empty() {
                    self.activate_color_palette(ColorFocus::Swatch(index));
                }
                return true;
            }
            if modifiers.is_empty()
                && key == &Key::Named(NamedKey::Delete)
                && editor.show_favorites
            {
                if !repeat {
                    if let Some(color) = colors.get(index) {
                        self.pending_color_favorite =
                            Some(ColorFavoriteIntent::Forget(*color));
                    }
                }
                return true;
            }
            let delta = match key {
                Key::Named(NamedKey::Tab) => {
                    if modifiers.shift_key() {
                        -1
                    } else {
                        1
                    }
                }
                Key::Named(NamedKey::ArrowLeft) => -1,
                Key::Named(NamedKey::ArrowRight) => 1,
                Key::Named(NamedKey::ArrowUp) => -(PALETTE_COLUMNS as isize),
                Key::Named(NamedKey::ArrowDown) => PALETTE_COLUMNS as isize,
                Key::Named(NamedKey::Home) => -(index as isize),
                Key::Named(NamedKey::End) => {
                    colors.len().saturating_sub(1) as isize - index as isize
                }
                _ => return true,
            };
            if !colors.is_empty() {
                if let Some(editor) = &mut self.color_editor {
                    editor.focus = ColorFocus::Swatch(
                        (index as isize + delta).rem_euclid(colors.len() as isize)
                            as usize,
                    );
                }
            }
            self.pressed = None;
            return true;
        }
        false
    }

    fn paint_color_palette(&self, canvas: &mut impl Canvas, theme: UiTheme) {
        if self.color_geometry.palette.height < 120.0 {
            return;
        }
        let Some(editor) = &self.color_editor else {
            return;
        };
        let colors = self.color_palette_colors();
        let font = (self.font * 0.72).clamp(10.0, 16.0);
        let clip = self.color_geometry.card;
        for (focus, bounds) in self.color_palette_controls() {
            match focus {
                ColorFocus::Suggested
                | ColorFocus::Favorites
                | ColorFocus::FavoriteToggle => {
                    let selected = focus != ColorFocus::FavoriteToggle
                        && editor.show_favorites == (focus == ColorFocus::Favorites);
                    let mut button_theme = theme;
                    if selected {
                        button_theme.background = theme.raised;
                    }
                    let favorite = editor_value(editor).is_some_and(|v| matches!(v, SettingValue::Color(color) if self.color_favorites.contains(&color)));
                    let caption = if focus == ColorFocus::FavoriteToggle {
                        if bounds.width < 110.0 {
                            if favorite {
                                "Remove"
                            } else {
                                "Save"
                            }
                        } else if bounds.width < 190.0 {
                            if favorite {
                                "★ Remove"
                            } else {
                                "☆ Save"
                            }
                        } else if favorite {
                            "Remove favorite"
                        } else {
                            "Save favorite"
                        }
                    } else if focus == ColorFocus::Suggested {
                        if bounds.width < 110.0 {
                            "Suggest"
                        } else {
                            "Suggested"
                        }
                    } else if bounds.width < 110.0 {
                        "Saved"
                    } else {
                        "Favorites"
                    };
                    action_button(
                        canvas,
                        bounds,
                        (
                            caption,
                            match focus {
                                ColorFocus::Suggested => "F1",
                                ColorFocus::Favorites => "F2",
                                _ => "F3",
                            },
                        ),
                        font,
                        (
                            editor.focus == focus,
                            focus != ColorFocus::FavoriteToggle
                                || editor_value(editor).is_some(),
                        ),
                        button_theme,
                        clip,
                    );
                }
                ColorFocus::Swatch(index) => {
                    let Some(color) = colors.get(index).copied() else {
                        continue;
                    };
                    color_swatch(canvas, bounds, color, theme, clip);
                    if editor.focus == focus
                        || parse_color(&editor.draft, editor.alpha) == Some(color)
                    {
                        preview_rounded_outline(
                            canvas,
                            Rect {
                                x: bounds.x + 1.0,
                                y: bounds.y + 1.0,
                                width: bounds.width - 2.0,
                                height: bounds.height - 2.0,
                            },
                            5.0,
                            2.0,
                            theme.outline,
                        );
                        let badge = Rect {
                            x: bounds.x + (bounds.width - 16.0) * 0.5,
                            y: bounds.y + 5.0,
                            width: 16.0,
                            height: 18.0,
                        };
                        rounded_fill(canvas, badge, 3.0, theme.background, bounds);
                        label(
                            canvas,
                            badge,
                            if editor.focus == focus { ">" } else { "✓" },
                            font,
                            theme.text,
                            true,
                            bounds,
                        );
                    }
                }
                _ => {}
            }
        }
        let message = if let ColorFocus::Swatch(index) = editor.focus {
            colors
                .get(index)
                .map(|color| {
                    format!(
                        "{}{}",
                        hex_color(*color, editor.alpha),
                        if editor.show_favorites {
                            " | Del: remove favorite"
                        } else {
                            ""
                        }
                    )
                })
                .unwrap_or_default()
        } else if editor.show_favorites && colors.is_empty() {
            if editor.palette_browsing {
                "No favorites. F3: save the draft".into()
            } else {
                "No favorites. F3: save the draft color".into()
            }
        } else {
            "F1: suggested | F2: favorites | F3: favorite".into()
        };
        let area = self.color_geometry.palette;
        let compact = area.width < 400.0;
        let navigation = if editor.palette_browsing {
            if compact {
                "Tab/Arrows | Enter: pick | Esc: back"
            } else {
                "Tab / Arrows: colors | Enter: choose | Esc: back"
            }
        } else if compact {
            "Tab: controls | Enter: open | Esc: exit"
        } else {
            "Tab: controls | Enter: browse | Esc: cancel"
        };
        for (line, hint) in [&message, navigation].into_iter().enumerate() {
            shortcut_hint(
                canvas,
                Rect {
                    y: area.y + 102.0 + line as f32 * 22.0,
                    height: 22.0,
                    ..area
                },
                hint,
                if compact {
                    (font * 0.8).max(10.0)
                } else {
                    font
                },
                theme,
                clip,
            );
        }
    }

    #[cfg(any(test, feature = "native-gui-test-hooks"))]
    pub(super) fn color_palette_snapshot(&self) -> serde_json::Value {
        let colors = self.color_palette_colors();
        serde_json::json!({
            "favorites": self.color_editor.as_ref().is_some_and(|e| e.show_favorites),
            "browsing": self.color_editor.as_ref().is_some_and(|e| e.palette_browsing),
            "focus": self.color_editor.as_ref().map(|e| format!("{:?}", e.focus)),
            "controls": self.color_palette_controls().into_iter().map(|(focus, rect)| serde_json::json!({
                "focus": format!("{focus:?}"), "bounds": rect.array(),
                "value": if let ColorFocus::Swatch(index) = focus { colors.get(index).copied() } else { None },
            })).collect::<Vec<_>>()
        })
    }
}

impl SettingsView {
    pub(super) fn color_requires_larger_window(&self) -> bool {
        let font = self.font.max(10.0);
        self.width < font * 13.0 + 32.0 || self.height < font * 1.45 * 10.0 + 72.0
    }
    pub(super) fn open_color(&mut self) {
        if self.pending.is_some() {
            return;
        }
        let Some(entry) = self.focused_entry() else {
            return;
        };
        let (alpha, text_limits) = match (&entry.kind, &entry.value) {
            (SettingKind::Color { alpha }, SettingValue::Color(_)) => (*alpha, None),
            (
                SettingKind::Text {
                    max_bytes,
                    allow_empty,
                },
                SettingValue::Text(_),
            ) => (false, Some((*max_bytes, *allow_empty))),
            _ => return,
        };
        if let Some(reason) = entry.availability.reason() {
            self.status = reason.into();
            return;
        }
        if self.color_requires_larger_window() {
            self.set_status("Enlarge the window to edit this color.");
            return;
        }
        let draft = match &entry.value {
            SettingValue::Text(value) => value.clone(),
            _ => display_value(entry),
        };
        self.color_editor = Some(ColorEditor {
            id: entry.id.clone(),
            revision: self.catalog.as_ref().map_or(0, Catalog::revision),
            alpha,
            text_limits,
            caret: draft.len(),
            anchor: Some(0),
            draft,
            focus: ColorFocus::Hex,
            composing: false,
            show_favorites: false,
            palette_browsing: false,
            custom_input: false,
            last_valid_color: match &entry.value {
                SettingValue::Color(color) => Some(*color),
                _ => None,
            },
            feedback: None,
        });
        self.preedit.clear();
        self.pressed = None;
        self.touch = None;
        self.layout_dirty = true;
    }
    pub(super) fn cancel_color(&mut self) {
        if self.color_editor.take().is_none() {
            return;
        }
        if self.standalone_color_route.is_some() && self.pending.is_none() {
            self.close();
            return;
        }
        self.restore_gallery_catalog();
        self.preedit.clear();
        self.pressed = None;
        self.touch = None;
        self.caret_rect = Rect::default();
        self.focus = Focus::List;
        self.layout_dirty = true;
        self.reveal_focus = true;
    }
    pub(super) fn paste_color(&mut self, text: &str) -> bool {
        if self.color_requires_larger_window() {
            return false;
        }
        let Some(editor) = &mut self.color_editor else {
            return false;
        };
        let max = editor.text_limits.map_or(MAX_COLOR_BYTES, |(max, _)| max);
        if text.len() > max
            || if editor.text_limits.is_some() {
                !safe_editor_text(text, max, true)
            } else {
                !text.bytes().all(|byte| byte.is_ascii_graphic())
            }
        {
            return false;
        }
        if editor.focus != ColorFocus::Hex || editor.composing {
            return false;
        }
        let start = editor.anchor.unwrap_or(editor.caret).min(editor.caret);
        let end = editor.anchor.unwrap_or(editor.caret).max(editor.caret);
        if editor.draft.len() - (end - start) + text.len() > max {
            return false;
        }
        // Keyboard and mouse paths maintain UTF-8 boundaries for this draft.
        if !editor.draft.is_char_boundary(start) || !editor.draft.is_char_boundary(end) {
            return false;
        }
        editor.draft.replace_range(start..end, text);
        if editor.text_limits.is_none() {
            if let Some(color) = parse_color(&editor.draft, editor.alpha) {
                editor.last_valid_color = Some(color);
            }
        }
        editor.caret = start + text.len();
        editor.anchor = None;
        editor.feedback = None;
        editor.custom_input = editor.text_limits.is_none();
        self.pressed = None;
        self.layout_dirty = true;
        true
    }
    pub(super) fn activate_color(&mut self, focus: ColorFocus) {
        if matches!(
            focus,
            ColorFocus::Suggested
                | ColorFocus::Favorites
                | ColorFocus::FavoriteToggle
                | ColorFocus::Swatch(_)
        ) {
            self.activate_color_palette(focus);
            return;
        }
        if focus == ColorFocus::Cancel {
            self.cancel_color();
            return;
        }
        let Some(editor) = &self.color_editor else {
            return;
        };
        let change = match focus {
            ColorFocus::Apply => {
                if editor.composing {
                    return;
                }
                let Some(value) = editor_value(editor) else {
                    return;
                };
                Change::Set(value)
            }
            ColorFocus::Reset => Change::Reset,
            _ => return,
        };
        let edit = Edit {
            revision: editor.revision,
            id: editor.id.clone(),
            change,
        };
        if self.pending.is_none()
            && self
                .catalog
                .as_ref()
                .is_some_and(|catalog| catalog.validate_edit(&edit).is_ok())
        {
            if focus == ColorFocus::Reset {
                let title = self
                    .catalog
                    .as_ref()
                    .and_then(|catalog| catalog.get(&edit.id))
                    .map_or_else(
                        || "Reset this value?".into(),
                        |entry| format!("Reset {}?", entry.label),
                    );
                self.ask_confirmation(
                    ConfirmedSettingsAction::Setting(edit),
                    title,
                    "Restore this value to its configured default.",
                    "Reset",
                );
                return;
            }
            if editor.custom_input {
                if let Change::Set(SettingValue::Color(color)) = &edit.change {
                    self.pending_color_favorite =
                        Some(ColorFavoriteIntent::Remember(*color));
                }
            }
            self.pending = Some(edit);
            self.status.clear();
            self.cancel_color();
        }
    }
    pub(super) fn color_key(
        &mut self,
        key: &Key,
        text: Option<&str>,
        modifiers: ModifiersState,
        repeat: bool,
    ) {
        if matches!(key, Key::Named(NamedKey::Escape)) {
            if self
                .color_editor
                .as_ref()
                .is_some_and(|editor| editor.composing)
            {
                self.preedit.clear();
                if let Some(editor) = &mut self.color_editor {
                    editor.composing = false;
                }
            } else if let Some(editor) = &mut self.color_editor {
                if editor.palette_browsing {
                    editor.palette_browsing = false;
                    editor.focus = if editor.show_favorites {
                        ColorFocus::Favorites
                    } else {
                        ColorFocus::Suggested
                    };
                    self.pressed = None;
                } else {
                    self.cancel_color();
                }
            } else {
                self.cancel_color();
            }
            return;
        }
        if self
            .color_editor
            .as_ref()
            .is_some_and(|editor| editor.composing)
            || self.color_requires_larger_window()
        {
            return;
        }
        if self.color_palette_key(key, modifiers, repeat) {
            return;
        }
        let Some(editor) = &mut self.color_editor else {
            return;
        };
        let command = modifiers.control_key() || modifiers.super_key();
        if command
            && matches!(key, Key::Character(value) if value.eq_ignore_ascii_case("a"))
            && editor.focus == ColorFocus::Hex
        {
            editor.anchor = Some(0);
            editor.caret = editor.draft.len();
            return;
        }
        if command || modifiers.alt_key() {
            return;
        }
        let cycle = match key {
            Key::Named(NamedKey::Tab) => Some(if modifiers.shift_key() { -1 } else { 1 }),
            Key::Named(NamedKey::ArrowLeft | NamedKey::ArrowUp)
                if editor.focus != ColorFocus::Hex =>
            {
                Some(-1)
            }
            Key::Named(NamedKey::ArrowRight | NamedKey::ArrowDown)
                if editor.focus != ColorFocus::Hex =>
            {
                Some(1)
            }
            _ => None,
        };
        if let Some(direction) = cycle {
            let mut order = vec![ColorFocus::Hex];
            if editor.text_limits.is_none() && self.color_geometry.palette.height >= 120.0
            {
                order.extend([
                    ColorFocus::Suggested,
                    ColorFocus::Favorites,
                    ColorFocus::FavoriteToggle,
                ]);
            }
            order.extend([ColorFocus::Apply, ColorFocus::Cancel, ColorFocus::Reset]);
            let index = order
                .iter()
                .position(|focus| *focus == editor.focus)
                .unwrap_or(0) as i32;
            editor.focus =
                order[(index + direction).rem_euclid(order.len() as i32) as usize];
            self.preedit.clear();
            self.pressed = None;
            return;
        }
        if !repeat && matches!(key, Key::Named(NamedKey::Enter)) {
            let focus = if editor.focus == ColorFocus::Hex {
                ColorFocus::Apply
            } else {
                editor.focus
            };
            self.activate_color(focus);
            return;
        }
        if editor.focus != ColorFocus::Hex {
            if !repeat
                && matches!(key, Key::Character(value) if value.eq_ignore_ascii_case("r"))
            {
                self.activate_color(ColorFocus::Reset);
                return;
            }
            if !repeat
                && matches!(key, Key::Character(value) if value.eq_ignore_ascii_case("a"))
            {
                self.activate_color(ColorFocus::Apply);
                return;
            }
            if !repeat && matches!(key, Key::Named(NamedKey::Space)) {
                let focus = editor.focus;
                self.activate_color(focus);
            }
            return;
        }
        let old = editor.caret;
        match key {
            Key::Named(NamedKey::ArrowLeft) => {
                editor.caret = editor.draft[..editor.caret]
                    .grapheme_indices(true)
                    .next_back()
                    .map_or(0, |(index, _)| index);
            }
            Key::Named(NamedKey::ArrowRight) => {
                editor.caret = editor.draft[editor.caret..]
                    .graphemes(true)
                    .next()
                    .map_or(editor.caret, |next| editor.caret + next.len());
            }
            Key::Named(NamedKey::Home) => editor.caret = 0,
            Key::Named(NamedKey::End) => editor.caret = editor.draft.len(),
            Key::Named(NamedKey::Backspace | NamedKey::Delete) => {
                if editor.anchor == Some(editor.caret) {
                    editor.anchor = None;
                }
                if editor.anchor.is_none() {
                    editor.anchor =
                        Some(if matches!(key, Key::Named(NamedKey::Backspace)) {
                            editor.draft[..editor.caret]
                                .grapheme_indices(true)
                                .next_back()
                                .map_or(0, |(index, _)| index)
                        } else {
                            editor.draft[editor.caret..]
                                .graphemes(true)
                                .next()
                                .map_or(editor.caret, |next| editor.caret + next.len())
                        });
                }
                self.paste_color("");
                return;
            }
            _ => {
                if let Some(text) = text {
                    self.paste_color(text);
                }
                return;
            }
        }
        if modifiers.shift_key() {
            editor.anchor.get_or_insert(old);
        } else {
            editor.anchor = None;
        }
        self.pressed = None;
    }
    pub(super) fn prepare_color(&mut self, viewport: Rect) {
        let line = self.font.max(10.0) * 1.45;
        let width = (self.font.max(10.0) * 13.0 + 16.0)
            .max(520.0)
            .min(self.width - 32.0);
        let palette_height = if self
            .color_editor
            .as_ref()
            .is_some_and(|e| e.text_limits.is_none())
            && self.height >= line * 10.0 + 72.0 + PALETTE_HEIGHT
            && self.width >= 320.0
        {
            PALETTE_HEIGHT
        } else {
            0.0
        };
        if palette_height == 0.0 {
            if let Some(editor) = &mut self.color_editor {
                if matches!(
                    editor.focus,
                    ColorFocus::Suggested
                        | ColorFocus::Favorites
                        | ColorFocus::FavoriteToggle
                        | ColorFocus::Swatch(_)
                ) {
                    editor.focus = ColorFocus::Hex;
                }
                editor.palette_browsing = false;
            }
        }
        let height = (line * 10.0 + 40.0 + palette_height).min(self.height - 32.0);
        let card = Rect {
            x: (self.width - width) * 0.5,
            y: (self.height - height) * 0.5,
            width,
            height,
        };
        let area = Rect {
            x: card.x + 8.0,
            y: card.y + 8.0,
            width: width - 16.0,
            height: line,
        };
        let split = width >= 480.0;
        let form = if split {
            Rect {
                width: (area.width - 12.0) * 0.5,
                ..area
            }
        } else {
            area
        };
        let input = Rect {
            y: form.y + line * 3.0 + 8.0,
            height: line + 8.0,
            ..form
        };
        let preview = if split {
            Rect {
                x: form.x + form.width + 12.0,
                y: area.y + line + 4.0,
                width: form.width,
                height: line * 6.0 + 12.0,
            }
        } else {
            Rect {
                y: input.y + input.height + 8.0,
                height: line + 8.0,
                ..area
            }
        };
        let button_width = (area.width - 16.0) / 3.0;
        let apply = Rect {
            y: card.y + height - line - 16.0,
            height: line + 8.0,
            width: button_width,
            ..area
        };
        self.color_geometry = ColorGeometry {
            palette: Rect {
                x: area.x,
                y: apply.y - palette_height - 4.0,
                width: area.width,
                height: palette_height,
            },
            card,
            title: area,
            name: Rect {
                y: form.y + line + 4.0,
                height: line * 2.0,
                ..form
            },
            input,
            preview,
            help: Rect {
                y: if split {
                    input.y + input.height + 8.0
                } else {
                    preview.y + preview.height + 8.0
                },
                height: line * 2.0,
                ..form
            },
            apply,
            cancel: Rect {
                x: apply.x + button_width + 8.0,
                ..apply
            },
            reset: Rect {
                x: apply.x + (button_width + 8.0) * 2.0,
                ..apply
            },
        };
        self.geometry.viewport = viewport;
        self.rows.clear();
        self.caret_rect = Rect::default();
    }
    pub(super) fn paint_color(&mut self, canvas: &mut impl Canvas, theme: UiTheme) {
        self.update_color_suggestions(theme);
        let Some(editor) = &self.color_editor else {
            return;
        };
        let Some(entry) = self
            .catalog
            .as_ref()
            .and_then(|catalog| catalog.get(&editor.id))
        else {
            return;
        };
        let g = self.color_geometry;
        let viewport = self.geometry.viewport;
        let font = self.font.max(10.0);
        let line = font * 1.45;
        rounded_surface(canvas, g.card, theme.surface, viewport);
        label(
            canvas,
            g.title,
            if editor.text_limits.is_some() {
                "Edit text"
            } else {
                "Choose a color"
            },
            font,
            theme.text,
            true,
            g.card,
        );
        let opts = DrawOpts {
            font_size: font,
            color: color_u8(theme.text),
            ..DrawOpts::default()
        };
        let mut names = wrapped(&entry.label, g.name.width - 8.0, canvas.text(), &opts);
        if names.len() > 2 {
            names.truncate(2);
            if let Some(last) = names.last_mut() {
                while !last.is_empty()
                    && canvas.text().measure(&format!("{last}…"), &opts)
                        > g.name.width - 8.0
                {
                    let end = last
                        .grapheme_indices(true)
                        .next_back()
                        .map_or(0, |(start, _)| start);
                    last.truncate(end);
                }
                last.push('…');
            }
        }
        for (index, name) in names.iter().enumerate() {
            label(
                canvas,
                Rect {
                    y: g.name.y + index as f32 * line,
                    height: line,
                    ..g.name
                },
                name,
                font,
                theme.text,
                false,
                g.name,
            );
        }
        control(
            canvas,
            g.input,
            editor.focus == ColorFocus::Hex,
            theme,
            g.card,
        );
        let display = format!(
            "{}{}{}",
            &editor.draft[..editor.caret],
            self.preedit,
            &editor.draft[editor.caret..]
        );
        let caret_width = canvas.text().measure(&editor.draft[..editor.caret], &opts);
        let shift = (caret_width - (g.input.width - 16.0)).max(0.0);
        self.caret_rect = Rect {
            x: g.input.x + 8.0 + caret_width - shift,
            y: g.input.y + 3.0,
            width: 1.0,
            height: g.input.height - 6.0,
        };
        if let Some(anchor) = editor.anchor.filter(|anchor| *anchor != editor.caret) {
            let left = canvas
                .text()
                .measure(&editor.draft[..anchor.min(editor.caret)], &opts);
            let right = canvas
                .text()
                .measure(&editor.draft[..anchor.max(editor.caret)], &opts);
            rect(
                canvas,
                Rect {
                    x: g.input.x + 8.0 + left - shift,
                    y: g.input.y + 3.0,
                    width: right - left,
                    height: g.input.height - 6.0,
                },
                theme.raised,
                g.input,
            );
        }
        canvas.text().draw_clipped(
            g.input.x + 8.0 - shift,
            g.input.y + 2.0,
            &display,
            &opts,
            g.input.array(),
        );
        if editor.focus == ColorFocus::Hex && self.preedit.is_empty() {
            rect(canvas, self.caret_rect, theme.outline, g.input);
        }
        let draft = editor_value(editor);
        if editor.text_limits.is_some() {
            label(
                canvas,
                g.preview,
                if self.profiles.is_some() {
                    "Saved text · no shell expansion"
                } else {
                    "Display text only · never evaluated"
                },
                font * 0.85,
                theme.muted_text,
                false,
                g.preview,
            );
        } else {
            let color = match &entry.value {
                SettingValue::Color(color) => *color,
                _ => return,
            };
            let draft_color = draft
                .as_ref()
                .and_then(|value| match value {
                    SettingValue::Color(color) => Some(*color),
                    _ => None,
                })
                .or(editor.last_valid_color);
            for (index, (value, caption)) in [
                (Some(color), "Current"),
                (
                    draft_color,
                    if draft.is_some() {
                        "Draft"
                    } else {
                        "Last valid draft"
                    },
                ),
            ]
            .into_iter()
            .enumerate()
            {
                let split = g.preview.x > g.input.x + g.input.width;
                let section = if split {
                    Rect {
                        y: g.preview.y + index as f32 * g.preview.height * 0.5,
                        height: g.preview.height * 0.5,
                        ..g.preview
                    }
                } else {
                    Rect {
                        x: g.preview.x + index as f32 * g.preview.width * 0.5,
                        width: g.preview.width * 0.5,
                        ..g.preview
                    }
                };
                if let Some(value) = value {
                    self.paint_color_graphic(
                        canvas,
                        section,
                        editor.id.as_str(),
                        value,
                        theme,
                    );
                }
                label(
                    canvas,
                    Rect {
                        x: section.x + 4.0,
                        width: (section.width - 8.0).max(0.0),
                        ..section
                    },
                    caption,
                    (font * 0.58).max(8.0),
                    theme.muted_text,
                    false,
                    g.preview,
                );
            }
        }
        let text_help = editor.text_limits.map(|(max, allow_empty)| {
            let prompt = if allow_empty {
                "Optional text."
            } else {
                "Enter text."
            };
            format!("{prompt} Up to {max} UTF-8 bytes.")
        });
        let help = if let Some(feedback) = editor.feedback.as_deref() {
            feedback
        } else if let Some(text_help) = text_help.as_deref() {
            if draft.is_none() {
                "Text is too long or contains a control character. Apply unavailable."
            } else {
                text_help
            }
        } else if draft.is_none() {
            if editor.alpha {
                "Use #RRGGBB or #RRGGBBAA. Apply unavailable."
            } else {
                "Use #RRGGBB. Apply unavailable."
            }
        } else if g.palette.height == 0.0 {
            "Enlarge the window for suggested colors and favorites."
        } else if editor.alpha {
            "#RRGGBBAA includes opacity (00–FF)."
        } else {
            "#RRGGBB uses an opaque color."
        };
        for (index, value) in wrapped(help, g.help.width - 8.0, canvas.text(), &opts)
            .iter()
            .take(2)
            .enumerate()
        {
            label(
                canvas,
                Rect {
                    y: g.help.y + index as f32 * line,
                    height: line,
                    ..g.help
                },
                value,
                font * 0.85,
                theme.muted_text,
                false,
                g.help,
            );
        }
        if editor.text_limits.is_none() {
            self.paint_color_palette(canvas, theme);
        }
        for (bounds, focus, caption, key) in [
            (
                g.apply,
                ColorFocus::Apply,
                "Apply",
                if editor.palette_browsing {
                    "Esc→A"
                } else {
                    "A"
                },
            ),
            (
                g.cancel,
                ColorFocus::Cancel,
                "Cancel",
                if editor.palette_browsing {
                    "Esc×2"
                } else {
                    "Esc"
                },
            ),
            (
                g.reset,
                ColorFocus::Reset,
                "Reset",
                if editor.palette_browsing {
                    "Esc→R"
                } else {
                    "R"
                },
            ),
        ] {
            action_button(
                canvas,
                bounds,
                (caption, key),
                font,
                (
                    editor.focus == focus,
                    focus != ColorFocus::Apply || draft.is_some(),
                ),
                theme,
                g.card,
            );
        }
    }
    pub(super) fn paint_color_graphic(
        &self,
        canvas: &mut impl Canvas,
        bounds: Rect,
        id: &str,
        color: [u8; 4],
        theme: UiTheme,
    ) {
        let (terminal_background, terminal_foreground) = self
            .customizations
            .as_ref()
            .and_then(|navigation| navigation.slot_pages.as_ref())
            .map_or(
                (theme.background, theme.text),
                SlotPageSnapshot::preview_terminal_colors,
            );
        let graphic = Rect {
            x: bounds.x + 4.0,
            y: bounds.y + 11.0,
            width: (bounds.width - 8.0).max(0.0),
            height: (bounds.height - 13.0).clamp(0.0, 24.0),
        };
        if graphic.width <= 0.0 || graphic.height <= 0.0 {
            return;
        }
        rect(canvas, graphic, terminal_background, bounds);
        if id.starts_with("tags.colors.") || id.starts_with("tags.slot.") {
            let name = tag_color_graphic_name(id);
            let style = self
                .customizations
                .as_ref()
                .and_then(|navigation| navigation.slot_pages.as_ref())
                .map_or(BarVisualStyle::Capsule, |snapshot| {
                    snapshot.preview_recipe().visual
                });
            let opacity = self
                .customizations
                .as_ref()
                .and_then(|navigation| navigation.slot_pages.as_ref())
                .map_or(75, |snapshot| {
                    let appearance = snapshot.preview_appearance();
                    if appearance.style
                        == rio_backend::config::presentation::TagStyle::Plain
                        || style == BarVisualStyle::Underline
                    {
                        0
                    } else {
                        appearance.opacity.get()
                    }
                });
            let tag = automexia_ui_model::context_tag_colors(
                terminal_background,
                [color[0], color[1], color[2]],
                opacity,
            );
            paint_sample_tag_surface(canvas, graphic, style, tag);
            label(
                canvas,
                graphic,
                &format!("{name} tag"),
                (self.font * 0.64).max(9.0),
                tag.foreground,
                false,
                graphic,
            );
        } else if id.starts_with("output.backgrounds.") {
            rect(
                canvas,
                graphic,
                color.map(|channel| f32::from(channel) / 255.0),
                graphic,
            );
            let severity = id.strip_prefix("output.backgrounds.").unwrap_or("status");
            label(
                canvas,
                graphic,
                &format!("{severity}: sample"),
                (self.font * 0.64).max(9.0),
                terminal_foreground,
                false,
                graphic,
            );
        } else if id.starts_with("output.colors.") {
            let severity = id.strip_prefix("output.colors.").unwrap_or("status");
            label(
                canvas,
                graphic,
                &format!("{severity}: sample"),
                (self.font * 0.64).max(9.0),
                color.map(|channel| f32::from(channel) / 255.0),
                false,
                graphic,
            );
        } else {
            color_swatch(canvas, graphic, color, theme, graphic);
        }
    }
}
