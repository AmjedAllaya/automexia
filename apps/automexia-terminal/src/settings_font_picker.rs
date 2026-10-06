//! Installed-family chooser. The application owns discovery, preview and saving.
use super::*;

#[derive(Clone, Debug)]
pub(crate) enum FontPickerIntent {
    Load,
    Preview(String),
    Apply(String),
    Cancel,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum FontPickerTarget {
    Row(usize),
    Search,
    Apply,
    Refresh,
    Back,
}
pub(super) struct FontPicker {
    revision: u64,
    query: String,
    entries: Vec<String>,
    filtered: Vec<usize>,
    pub(super) selected: usize,
    first: usize,
    visible: usize,
    original: String,
    notice: String,
    pub(super) ready: bool,
    pub(super) targets: Vec<(FontPickerTarget, Rect)>,
}
impl FontPicker {
    fn selected(&self) -> Option<&str> {
        self.entries
            .get(*self.filtered.get(self.selected)?)
            .map(String::as_str)
    }
    fn filter(&mut self) {
        let query = self.query.to_lowercase();
        self.filtered = self
            .entries
            .iter()
            .enumerate()
            .filter(|(_, name)| name.to_lowercase().contains(&query))
            .map(|(index, _)| index)
            .collect();
        self.selected = 0;
        self.first = 0;
    }
}
impl SettingsView {
    pub(super) fn open_font_picker(&mut self) {
        let Some(entry) = self.focused_entry() else {
            return;
        };
        if entry.id.as_str() != "fonts.family" || entry.availability.reason().is_some() {
            return;
        }
        let SettingValue::Text(original) = &entry.value else {
            return;
        };
        let Some(generation) = self.font_picker_generation.checked_add(1) else {
            return;
        };
        self.font_picker = Some(FontPicker {
            revision: self.catalog.as_ref().map_or(0, Catalog::revision),
            query: String::new(),
            entries: Vec::new(),
            filtered: Vec::new(),
            selected: 0,
            first: 0,
            visible: 0,
            original: original.clone(),
            notice: "Finding installed fonts… Esc cancels.".into(),
            ready: false,
            targets: Vec::new(),
        });
        self.font_picker_generation = generation;
        self.pending_font_picker = Some(FontPickerIntent::Load);
        self.focus = Focus::Search;
        self.caret = 0;
        self.anchor = None;
        self.preedit.clear();
        self.pressed = None;
        self.layout_dirty = true;
    }
    pub(crate) fn font_picker_session(&self) -> Option<(u64, u64)> {
        self.font_picker
            .as_ref()
            .map(|p| (self.font_picker_generation, p.revision))
    }
    pub(crate) fn font_picker_selection(&self) -> Option<&str> {
        self.font_picker.as_ref()?.selected()
    }
    pub(crate) fn take_font_picker_intent(&mut self) -> Option<FontPickerIntent> {
        self.pending_font_picker.take()
    }
    pub(crate) fn font_picker_inventory(
        &mut self,
        mut entries: Vec<String>,
        limited: bool,
    ) {
        let Some(picker) = &mut self.font_picker else {
            return;
        };
        // The current declarative/additional-directory family remains reachable
        // even when the OS inventory does not include it.
        if !entries
            .iter()
            .any(|s| s.eq_ignore_ascii_case(&picker.original))
        {
            entries.insert(0, picker.original.clone());
        }
        picker.entries = entries;
        picker.filter();
        picker.selected = picker
            .filtered
            .iter()
            .position(|index| {
                picker.entries[*index].eq_ignore_ascii_case(&picker.original)
            })
            .unwrap_or(0);
        picker.notice = if limited {
            "Some fonts were omitted. Search the listed families."
        } else {
            "Browse to preview · Enter applies · Esc restores"
        }
        .into();
        picker.ready = true;
        self.layout_dirty = true;
        self.reveal_focus = true;
    }
    pub(crate) fn font_picker_notice(&mut self, message: &str) {
        if let Some(picker) = &mut self.font_picker {
            picker.notice = message.into();
            self.layout_dirty = true;
        }
    }
    pub(crate) fn close_font_picker(&mut self) {
        if self.font_picker.take().is_none() {
            return;
        }
        self.pending_font_picker = Some(FontPickerIntent::Cancel);
        self.focus = Focus::List;
        self.caret = self.query().len();
        self.anchor = None;
        self.preedit.clear();
        self.pressed = None;
        self.layout_dirty = true;
        self.reveal_focus = true;
    }
    pub(super) fn invalidate_font_picker(&mut self, revision: u64) {
        if self
            .font_picker
            .as_ref()
            .is_some_and(|p| p.revision != revision)
        {
            self.close_font_picker();
        }
    }
    pub(super) fn font_picker_query(&self) -> Option<&str> {
        self.font_picker.as_ref().map(|p| p.query.as_str())
    }
    pub(super) fn set_font_picker_query(&mut self, query: String) {
        if let Some(picker) = &mut self.font_picker {
            picker.query = query;
            picker.filter();
            if picker.ready {
                self.font_picker_preview();
            }
        }
    }
    fn font_picker_preview(&mut self) {
        if self.font_picker.as_ref().is_none_or(|p| !p.ready) {
            return;
        }
        if let Some(family) = self.font_picker_selection() {
            self.pending_font_picker = Some(FontPickerIntent::Preview(family.into()));
        } else {
            // An empty search also invalidates any in-flight preview selection.
            self.pending_font_picker = Some(FontPickerIntent::Cancel);
        }
    }
    pub(super) fn font_picker_key(
        &mut self,
        key: &Key,
        modifiers: ModifiersState,
        repeat: bool,
    ) -> bool {
        if matches!(key, Key::Named(NamedKey::Escape)) {
            if self.preedit.is_empty() {
                self.close_font_picker();
            } else {
                self.preedit.clear();
            }
            return true;
        }
        if self.requires_larger_window() || !self.preedit.is_empty() {
            return true;
        }
        if modifiers.alt_key() {
            return true;
        }
        if matches!(key, Key::Named(NamedKey::F5)) && !repeat && modifiers.is_empty() {
            self.font_picker_activate(FontPickerTarget::Refresh);
            return true;
        }
        if modifiers.control_key() || modifiers.super_key() {
            return self.focus != Focus::Search;
        }
        if matches!(key, Key::Named(NamedKey::Tab)) {
            let stops = [
                Focus::Search,
                Focus::List,
                Focus::PreviewButton,
                Focus::Reset,
                Focus::Close,
            ];
            let index = stops.iter().position(|f| *f == self.focus).unwrap_or(0);
            self.focus = stops[(index
                + if modifiers.shift_key() {
                    stops.len() - 1
                } else {
                    1
                })
                % stops.len()];
            self.reveal_focus = true;
            self.layout_dirty = true;
            return true;
        }
        if (matches!(key, Key::Named(NamedKey::Enter))
            || (self.focus != Focus::Search
                && matches!(key, Key::Named(NamedKey::Space))))
            && !repeat
        {
            self.font_picker_activate(match self.focus {
                Focus::Close => FontPickerTarget::Back,
                Focus::Reset => FontPickerTarget::Refresh,
                _ => FontPickerTarget::Apply,
            });
            return true;
        }
        let browse = matches!(key, Key::Named(NamedKey::ArrowDown | NamedKey::ArrowUp))
            || (self.focus == Focus::List
                && matches!(
                    key,
                    Key::Named(
                        NamedKey::Home
                            | NamedKey::End
                            | NamedKey::PageDown
                            | NamedKey::PageUp
                    )
                ));
        if browse && matches!(self.focus, Focus::Search | Focus::List) {
            let Some(picker) = &mut self.font_picker else {
                return true;
            };
            let count = picker.filtered.len();
            if count > 0 {
                picker.selected = match key {
                    Key::Named(NamedKey::Home) => 0,
                    Key::Named(NamedKey::End) => count - 1,
                    Key::Named(NamedKey::PageUp) => {
                        picker.selected.saturating_sub(picker.visible.max(1))
                    }
                    Key::Named(NamedKey::PageDown) => {
                        (picker.selected + picker.visible.max(1)).min(count - 1)
                    }
                    Key::Named(NamedKey::ArrowUp) => {
                        (picker.selected + count - 1) % count
                    }
                    _ if self.focus == Focus::Search => picker.selected,
                    _ => (picker.selected + 1) % count,
                };
                self.font_picker_preview();
            }
            self.focus = Focus::List;
            self.reveal_focus = true;
            self.layout_dirty = true;
            return true;
        }
        // Reuse the shared Unicode search, selection, paste and IME editor.
        self.focus != Focus::Search
    }
    pub(super) fn font_picker_activate(&mut self, target: FontPickerTarget) {
        self.preedit.clear();
        match target {
            FontPickerTarget::Back => self.close_font_picker(),
            FontPickerTarget::Search => {
                self.focus = Focus::Search;
                self.caret = self.query().len();
                self.anchor = None;
            }
            FontPickerTarget::Refresh => {
                self.pending_font_picker = Some(FontPickerIntent::Load);
                self.font_picker_notice("Refreshing installed fonts…");
                if let Some(picker) = &mut self.font_picker {
                    picker.ready = false;
                }
            }
            FontPickerTarget::Apply => {
                if let Some(picker) = &self.font_picker {
                    if picker.ready {
                        if let Some(family) = picker.selected() {
                            self.pending_font_picker =
                                Some(FontPickerIntent::Apply(family.into()));
                        }
                    }
                }
            }
            FontPickerTarget::Row(index) => {
                if let Some(picker) = &mut self.font_picker {
                    if index < picker.filtered.len() {
                        picker.selected = index;
                        self.focus = Focus::List;
                        self.font_picker_preview();
                    }
                }
            }
        }
        self.pressed = None;
        self.layout_dirty = true;
    }
    pub(super) fn font_picker_target(&self, x: f32, y: f32) -> Option<Target> {
        self.font_picker
            .as_ref()?
            .targets
            .iter()
            .find(|(_, r)| r.contains(x, y))
            .map(|(t, _)| Target::FontPicker(*t))
    }
    pub(super) fn font_picker_scroll(&mut self, delta: f32) {
        if let Some(picker) = &mut self.font_picker {
            picker.first = if delta > 0.0 {
                picker.first.saturating_add(1)
            } else {
                picker.first.saturating_sub(1)
            };
            self.layout_dirty = true;
            self.reveal_focus = false;
            self.pressed = None;
        }
    }
    pub(super) fn prepare_font_picker(&mut self, viewport: Rect) {
        let f = self.font.max(10.0);
        let width = (viewport.width - 32.0).clamp(0.0, (f * 38.0).max(640.0));
        let height = (viewport.height - 32.0).clamp(0.0, (f * 34.0).max(680.0));
        let card = Rect {
            x: (viewport.width - width) * 0.5,
            y: (viewport.height - height) * 0.5,
            width,
            height,
        };
        let inner = Rect {
            x: card.x + 12.0,
            width: (width - 24.0).max(0.0),
            ..card
        };
        self.geometry = Geometry {
            viewport,
            card,
            search: Rect {
                y: card.y + f * 4.0,
                height: f * 1.9,
                ..inner
            },
            body: Rect {
                y: card.y + f * 6.5,
                height: (height - f * 14.0).max(0.0),
                ..inner
            },
            status: Rect {
                y: card.y + height - f * 6.5,
                height: f * 1.8,
                ..inner
            },
            ..Geometry::default()
        };
        let Some(picker) = &mut self.font_picker else {
            return;
        };
        picker.visible = (self.geometry.body.height / (f * 2.2)).floor() as usize;
        if self.reveal_focus {
            if picker.selected < picker.first {
                picker.first = picker.selected;
            }
            if picker.selected >= picker.first + picker.visible {
                picker.first = picker
                    .selected
                    .saturating_sub(picker.visible.saturating_sub(1));
            }
        }
        picker.first = picker
            .first
            .min(picker.filtered.len().saturating_sub(picker.visible.max(1)));
        picker.targets.clear();
        if width >= f * 22.0 && height >= f * 18.0 {
            picker
                .targets
                .push((FontPickerTarget::Search, self.geometry.search));
            for index in
                picker.first..(picker.first + picker.visible).min(picker.filtered.len())
            {
                picker.targets.push((
                    FontPickerTarget::Row(index),
                    Rect {
                        y: self.geometry.body.y + (index - picker.first) as f32 * f * 2.2,
                        height: f * 2.0,
                        ..self.geometry.body
                    },
                ));
            }
            for (index, target) in [
                FontPickerTarget::Apply,
                FontPickerTarget::Refresh,
                FontPickerTarget::Back,
            ]
            .into_iter()
            .enumerate()
            {
                picker.targets.push((
                    target,
                    Rect {
                        x: inner.x + index as f32 * (inner.width + 8.0) / 3.0,
                        y: card.y + height - f * 4.2,
                        width: (inner.width - 16.0) / 3.0,
                        height: f * 1.9,
                    },
                ));
            }
        } else {
            picker.targets.push((
                FontPickerTarget::Back,
                Rect {
                    y: (card.y + height - f * 2.0).max(card.y),
                    height: (f * 1.8).min(height),
                    ..inner
                },
            ));
        }
        self.layout_dirty = false;
        self.reveal_focus = false;
    }
    pub(super) fn paint_font_picker(&mut self, canvas: &mut impl Canvas, theme: UiTheme) {
        let g = self.geometry;
        let f = self.font.max(10.0);
        rounded_surface(canvas, g.card, theme.surface, g.viewport);
        let title = Rect {
            x: g.card.x + 12.0,
            y: g.card.y + 10.0,
            width: (g.card.width - 24.0).max(0.0),
            height: f * 1.6,
        };
        label(
            canvas,
            title,
            "Choose font",
            f * 1.1,
            theme.text,
            true,
            g.card,
        );
        let Some(picker) = &self.font_picker else {
            return;
        };
        if picker.targets.len() == 1 {
            label(
                canvas,
                Rect {
                    y: title.y + f * 2.0,
                    ..title
                },
                "Enlarge window to browse fonts",
                f * 0.8,
                theme.text,
                false,
                g.card,
            );
        } else {
            label(
                canvas,
                Rect {
                    y: title.y + f * 1.8,
                    ..title
                },
                "Live terminal preview · changes are not saved yet",
                f * 0.72,
                theme.muted_text,
                false,
                g.card,
            );
            let notice = picker.notice.clone();
            label(
                canvas,
                g.status,
                &notice,
                f * 0.75,
                theme.muted_text,
                false,
                g.card,
            );
            let hint = Rect {
                y: g.card.y + g.card.height - f * 1.7,
                height: f * 1.4,
                ..title
            };
            shortcut_hint(
                canvas,
                hint,
                "Arrows: preview | Tab: focus | Enter: apply | Esc: cancel",
                f * 0.7,
                theme,
                g.card,
            );
            self.paint_search_field(canvas, theme, "Search installed fonts");
        }
        let Some(picker) = &self.font_picker else {
            return;
        };
        for (target, bounds) in &picker.targets {
            match *target {
                FontPickerTarget::Search => {}
                FontPickerTarget::Row(index) => {
                    let Some(name) = picker
                        .filtered
                        .get(index)
                        .and_then(|i| picker.entries.get(*i))
                    else {
                        continue;
                    };
                    if index == picker.selected {
                        rounded_surface(canvas, *bounds, theme.raised, g.card);
                        rect(
                            canvas,
                            Rect {
                                width: 3.0,
                                ..*bounds
                            },
                            theme.outline,
                            g.card,
                        );
                    }
                    let suffix = if name.eq_ignore_ascii_case(&picker.original) {
                        " · Current"
                    } else if name
                        == rio_backend::sugarloaf::font::constants::DEFAULT_FONT_FAMILY
                    {
                        " · Bundled"
                    } else {
                        ""
                    };
                    label(
                        canvas,
                        Rect {
                            x: bounds.x + 10.0,
                            width: (bounds.width - 16.0).max(0.0),
                            ..*bounds
                        },
                        &format!("{name}{suffix}"),
                        f * 0.92,
                        theme.text,
                        index == picker.selected,
                        g.card,
                    );
                }
                action => {
                    let (text, key, focus) = match action {
                        FontPickerTarget::Apply => {
                            ("Apply", "Enter", Focus::PreviewButton)
                        }
                        FontPickerTarget::Refresh => ("Refresh", "F5", Focus::Reset),
                        _ => ("Cancel", "Esc", Focus::Close),
                    };
                    action_button(
                        canvas,
                        *bounds,
                        (text, key),
                        f,
                        (
                            self.focus == focus,
                            action != FontPickerTarget::Apply
                                || (picker.ready && picker.selected().is_some()),
                        ),
                        theme,
                        g.card,
                    );
                }
            }
        }
        if picker.ready && picker.filtered.is_empty() {
            label(
                canvas,
                g.body,
                "No matching fonts. Change the search or press F5 to refresh.",
                f * 0.8,
                theme.muted_text,
                false,
                g.card,
            );
        }
    }
    pub(super) fn font_picker_summary(&self) -> Option<String> {
        let picker = self.font_picker.as_ref()?;
        Some(format!("Choose font. {} matching fonts. {}. {}. Arrows preview, Enter applies, Escape restores.",picker.filtered.len(),picker.selected().unwrap_or("No selection"),picker.notice))
    }
    pub(super) fn font_picker_accessibility_surface(
        &self,
        scale: f32,
        viewport: accesskit::Rect,
    ) -> Option<automexia_ui_model::accessibility::Surface> {
        use accesskit::{Node, Role};
        use automexia_ui_model::accessibility::{
            physical_bounds, sanitize_accessible_text, Surface,
        };
        let picker = self.font_picker.as_ref()?;
        let mut surface = Surface::dialog(
            u64::MAX - 20,
            "Choose font",
            physical_bounds(self.geometry.card.array(), scale, viewport)
                .unwrap_or(viewport),
        );
        for (target, rect) in &picker.targets {
            let Some(bounds) = physical_bounds(rect.array(), scale, viewport) else {
                continue;
            };
            let (id, role, name, focused) = match *target {
                FontPickerTarget::Search => (
                    1,
                    Role::SearchInput,
                    "Search installed fonts".to_owned(),
                    self.focus == Focus::Search,
                ),
                FontPickerTarget::Row(index) => (
                    100 + index as u64,
                    Role::ListBoxOption,
                    picker.entries.get(*picker.filtered.get(index)?).cloned()?,
                    self.focus == Focus::List && index == picker.selected,
                ),
                FontPickerTarget::Apply => (
                    2,
                    Role::Button,
                    "Apply font".into(),
                    self.focus == Focus::PreviewButton,
                ),
                FontPickerTarget::Refresh => (
                    3,
                    Role::Button,
                    "Refresh installed fonts".into(),
                    self.focus == Focus::Reset,
                ),
                FontPickerTarget::Back => (
                    4,
                    Role::Button,
                    "Cancel font preview".into(),
                    self.focus == Focus::Close,
                ),
            };
            let mut node = Node::new(role);
            node.set_label(sanitize_accessible_text(&name));
            node.set_bounds(bounds);
            node.set_description("Arrows preview in the terminal. Enter applies. Escape restores the current font.");
            if *target == FontPickerTarget::Search {
                node.set_value(picker.query.clone());
            }
            if let FontPickerTarget::Row(index) = target {
                node.set_selected(*index == picker.selected);
            }
            if *target == FontPickerTarget::Apply
                && (!picker.ready || picker.selected().is_none())
            {
                node.set_disabled();
            }
            surface.push(id, node, focused);
        }
        Some(surface)
    }
    #[cfg(feature = "native-gui-test-hooks")]
    pub(super) fn font_picker_snapshot(&self) -> serde_json::Value {
        self.font_picker.as_ref().map_or(serde_json::Value::Null,|p|serde_json::json!({
            "ready":p.ready,"count":p.entries.len(),"matches":p.filtered.len(),"selected":p.selected(),"current":p.original,"notice":p.notice,
            "targets":p.targets.iter().map(|(target,bounds)|serde_json::json!({"id":format!("{target:?}"),"bounds":bounds.array()})).collect::<Vec<_>>()
        }))
    }
}
