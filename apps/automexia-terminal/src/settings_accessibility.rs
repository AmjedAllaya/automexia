//! Native semantics reuse the settings sheet's painted rectangles and focus.
use super::*;
use accesskit::{Node, Role, Toggled};
use automexia_ui_model::accessibility::{
    physical_bounds, sanitize_accessible_text, Surface,
};

impl SettingsView {
    pub(crate) fn accessibility_surface(
        &self,
        scale: f32,
        viewport: accesskit::Rect,
    ) -> Surface {
        let card = if self.confirmation.is_some() {
            self.confirmation_geometry.card
        } else if self.color_editor.is_some() {
            self.color_geometry.card
        } else {
            self.geometry.card
        };
        let mut surface = Surface::dialog(
            u64::MAX - 6,
            self.title(),
            physical_bounds(card.array(), scale, viewport).unwrap_or(viewport),
        );
        let mut add = |id: u64,
                       role,
                       label: &str,
                       value: &str,
                       description: &str,
                       rect: Rect,
                       focused: bool,
                       enabled: bool| {
            let Some(bounds) = physical_bounds(rect.array(), scale, viewport) else {
                return;
            };
            let mut node = Node::new(role);
            if let Some(row) = id
                .checked_sub(100)
                .and_then(|index| self.rows.get(index as usize))
            {
                node.set_author_id(row.id.as_str());
            }
            node.set_label(sanitize_accessible_text(label));
            if !value.is_empty() {
                node.set_value(sanitize_accessible_text(value));
            }
            if !description.is_empty() {
                node.set_description(sanitize_accessible_text(description));
            }
            node.set_bounds(bounds);
            if !enabled {
                node.set_disabled();
            }
            if matches!(role, Role::CheckBox | Role::RadioButton) {
                node.set_toggled(if value == "On" {
                    Toggled::True
                } else {
                    Toggled::False
                });
            }
            surface.push(id, node, focused);
        };
        if let Some(confirmation) = &self.confirmation {
            add(
                1,
                Role::Label,
                &confirmation.title,
                "",
                confirmation.description,
                self.confirmation_geometry.card,
                false,
                true,
            );
            add(
                2,
                Role::Button,
                "Cancel",
                "",
                "Escape",
                self.confirmation_geometry.cancel,
                !confirmation.accept_selected,
                true,
            );
            add(
                3,
                Role::Button,
                confirmation.accept_label,
                "",
                "Y",
                self.confirmation_geometry.accept,
                confirmation.accept_selected,
                true,
            );
            return surface;
        }
        if self.layout_dirty || self.requires_larger_window() {
            add(
                1,
                Role::Label,
                &self.accessibility_summary(),
                "",
                "",
                card,
                true,
                true,
            );
            return surface;
        }
        if let Some(editor) = &self.color_editor {
            let label = self
                .catalog
                .as_ref()
                .and_then(|catalog| catalog.get(&editor.id))
                .map_or("Value", |entry| entry.label.as_str());
            add(
                10,
                Role::TextInput,
                label,
                &editor.draft,
                "Enter applies; Escape cancels",
                self.color_geometry.input,
                editor.focus == ColorFocus::Hex,
                true,
            );
            for (id, label, focus, rect) in [
                (11, "Apply", ColorFocus::Apply, self.color_geometry.apply),
                (12, "Cancel", ColorFocus::Cancel, self.color_geometry.cancel),
                (
                    13,
                    "Reset to configuration",
                    ColorFocus::Reset,
                    self.color_geometry.reset,
                ),
            ] {
                add(
                    id,
                    Role::Button,
                    label,
                    "",
                    "",
                    rect,
                    editor.focus == focus,
                    focus != ColorFocus::Apply || editor_value(editor).is_some(),
                );
            }
            let colors = self.color_palette_colors();
            for (index, (focus, rect)) in
                self.color_palette_controls().into_iter().enumerate()
            {
                let (label, role, selected) = match focus {
                    ColorFocus::Suggested => ("Suggested colors".to_owned(), Role::RadioButton, !editor.show_favorites),
                    ColorFocus::Favorites => ("Favorite colors".to_owned(), Role::RadioButton, editor.show_favorites),
                    ColorFocus::FavoriteToggle => (
                        if editor_value(editor).is_some_and(|v| matches!(v, SettingValue::Color(c) if self.color_favorites.contains(&c))) { "Remove favorite" } else { "Save favorite" }.to_owned(), Role::Button, false),
                    ColorFocus::Swatch(i) => {
                        let Some([r,g,b,a]) = colors.get(i) else { continue; };
                        (format!("Color #{r:02X}{g:02X}{b:02X}{a:02X}"), Role::RadioButton,
                            parse_color(&editor.draft, editor.alpha) == colors.get(i).copied())
                    }
                    _ => continue,
                };
                add(
                    30 + index as u64,
                    role,
                    &label,
                    if selected { "On" } else { "Off" },
                    "Enter chooses a draft; Apply commits the color",
                    rect,
                    editor.focus == focus,
                    focus != ColorFocus::FavoriteToggle || editor_value(editor).is_some(),
                );
            }
            return surface;
        }
        // These pages have their own layout owners; never expose the underlying
        // settings rows while they cover the sheet.
        if let Some(gallery) = self.gallery_accessibility_surface(scale, viewport) {
            return gallery;
        }
        add(
            20,
            Role::SearchInput,
            "Search settings",
            self.query(),
            "Tab moves to settings",
            self.geometry.search,
            self.focus == Focus::Search,
            true,
        );
        if self.is_category_detail() {
            add(
                21,
                Role::Button,
                "Back",
                "",
                "Escape, Backspace or Alt+Left",
                self.geometry.back,
                false,
                true,
            );
        }
        for (index, row) in self.rows.iter().take(256).enumerate() {
            let Some(rect) = row.bounds.intersect(self.geometry.body) else {
                continue;
            };
            let Some(entry) = self
                .catalog
                .as_ref()
                .and_then(|catalog| catalog.get(&row.id))
            else {
                continue;
            };
            let selected =
                self.view.as_ref().and_then(ViewState::focused) == Some(&row.id);
            let role = if row.navigation {
                Role::Button
            } else {
                match entry.kind {
                    SettingKind::Boolean => Role::CheckBox,
                    SettingKind::Number { .. } | SettingKind::ContinuousNumber { .. } => {
                        Role::SpinButton
                    }
                    SettingKind::Action => Role::Button,
                    SettingKind::Choice { .. } => Role::ComboBox,
                    _ => Role::TextInput,
                }
            };
            let value = self
                .numeric_editor
                .as_ref()
                .filter(|editor| editor.id == row.id)
                .map_or_else(|| display_value(entry), |editor| editor.draft.clone());
            let description = format!(
                "{}. {}",
                entry.description,
                entry.availability.reason().unwrap_or("")
            );
            add(
                100 + index as u64,
                role,
                &entry.label,
                &value,
                &description,
                rect,
                self.focus == Focus::List && selected,
                entry.availability.reason().is_none(),
            );
        }
        for (id, label, focus, rect, enabled) in [
            (
                30,
                "Edit preview",
                Focus::PreviewButton,
                self.preview_button,
                true,
            ),
            (31, "Reset", Focus::Reset, self.geometry.reset, true),
            (
                32,
                "Restore saved",
                Focus::Restore,
                self.geometry.restore,
                self.temporary_customizations,
            ),
            (33, "Close", Focus::Close, self.geometry.close, true),
        ] {
            add(
                id,
                Role::Button,
                label,
                "",
                "",
                rect,
                self.focus == focus,
                enabled,
            );
        }
        if !self.status.is_empty() {
            add(
                40,
                Role::Status,
                &self.status,
                "",
                "",
                self.geometry.status,
                false,
                true,
            );
        }
        if self.is_window_controls_preview() {
            for (index, (id, rect)) in self.preview_targets.iter().enumerate() {
                let Some(style) = window_controls_preview::choice_style(id) else {
                    continue;
                };
                let Some(bounds) = physical_bounds(rect.array(), scale, viewport) else {
                    continue;
                };
                let mut node = Node::new(Role::RadioButton);
                node.set_author_id(id.as_str());
                node.set_label(format!("{} button style", style.label()));
                node.set_description(
                    "Choose a style. Live state examples below are not controls.",
                );
                node.set_toggled(if style == self.selected_window_control_style() {
                    Toggled::True
                } else {
                    Toggled::False
                });
                node.set_bounds(bounds);
                surface.push(
                    500 + index as u64,
                    node,
                    self.focus == Focus::Preview
                        && self.preview_selected.as_ref() == Some(id),
                );
            }
        }
        surface
    }
}
