//! Native gallery view; filesystem and preference effects stay with Application.
use super::*;
use crate::automexia::theme_gallery::{
    self, ThemeDescriptor, ThemeSelection, ThemeSource, ValidationStatus,
};
use automexia_ui_model::settings::{Availability, ChangeScope, SettingOwner};
use rio_backend::config::theme::Theme;

#[derive(Clone)]
pub(crate) struct ThemeContext {
    pub configured: Theme,
    pub saved: Option<ThemeSelection>,
    pub font_colors: bool,
}
#[derive(Clone, Debug)]
pub(crate) enum ThemeIntent {
    Load,
    Preview(Option<ThemeSelection>),
    Apply(Option<ThemeSelection>),
    Import,
    Export(ThemeSelection),
    SaveCopy(ThemeSelection),
    Cancel,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum GalleryTarget {
    Row(usize),
    Apply,
    Customize,
    Import,
    Export,
    Configuration,
    Back,
    Refresh,
}
#[derive(Default)]
pub(super) struct Gallery {
    entries: Vec<ThemeDescriptor>,
    selected: usize,
    source_selected: usize,
    first: usize,
    visible: usize,
    focus: usize,
    draft: Option<ThemeSelection>,
    original: Option<ThemeSelection>,
    fields: Vec<(String, [u8; 4])>,
    targets: Vec<(GalleryTarget, Rect)>,
    list: Rect,
    preview: Rect,
    notice: String,
    busy: bool,
}
const ACTIONS: [GalleryTarget; 7] = [
    GalleryTarget::Apply,
    GalleryTarget::Customize,
    GalleryTarget::Import,
    GalleryTarget::Export,
    GalleryTarget::Configuration,
    GalleryTarget::Refresh,
    GalleryTarget::Back,
];
impl Gallery {
    fn actions(&self) -> &'static [GalleryTarget] {
        if self.draft.is_some() {
            &[
                GalleryTarget::Apply,
                GalleryTarget::Export,
                GalleryTarget::Back,
            ]
        } else {
            &ACTIONS
        }
    }
    fn unavailable(
        &self,
        target: GalleryTarget,
        temporary: bool,
    ) -> Option<&'static str> {
        if matches!(target, GalleryTarget::Back | GalleryTarget::Row(_)) {
            return None;
        }
        if self.busy {
            return Some("Please wait for the current file operation.");
        }
        if temporary
            && matches!(target, GalleryTarget::Apply | GalleryTarget::Configuration)
        {
            return Some("Restore saved customizations before applying a theme.");
        }
        if matches!(
            target,
            GalleryTarget::Apply | GalleryTarget::Customize | GalleryTarget::Export
        ) && self.draft.is_none()
            && self
                .entries
                .get(self.selected)
                .is_none_or(|entry| entry.theme.is_none())
        {
            return Some("This file is invalid. Choose a valid theme to continue.");
        }
        None
    }
    fn selection(&self) -> Option<ThemeSelection> {
        self.draft
            .clone()
            .or_else(|| self.entries.get(self.selected)?.selection())
    }
    fn count(&self) -> usize {
        if self.draft.is_some() {
            self.fields.len() + 1
        } else {
            self.entries.len()
        }
    }
}
impl SettingsView {
    #[cfg(feature = "native-gui-test-hooks")]
    pub(super) fn gallery_snapshot(&self) -> serde_json::Value {
        self.gallery.as_ref().map_or(serde_json::Value::Null,|g|serde_json::json!({
            "count":g.count(),"selected":g.selected,"busy":g.busy,"customizing":g.draft.is_some(),
            "builtin":g.entries.get(g.selected).filter(|e|e.source==ThemeSource::BuiltIn).map(|e|e.id.as_str()),
            "targets":g.targets.iter().map(|(target,bounds)|serde_json::json!({"id":format!("{target:?}"),"bounds":bounds.array()})).collect::<Vec<_>>(),
            "preview":g.preview.array(),
        }))
    }
    #[cfg(test)]
    pub(super) fn gallery_summary(&self) -> Option<String> {
        let g = self.gallery.as_ref()?;
        let entry = g.entries.get(g.selected);
        Some(format!(
            "Themes. {}. {}. Preview only. Enter: apply. Escape: restore. {}",
            entry.map_or("", |e| e.name.as_str()),
            entry.map_or("", |e| e.description.as_str()),
            g.notice
        ))
    }
    pub(crate) fn set_theme_context(&mut self, context: ThemeContext) {
        self.theme_context = Some(context);
    }
    pub(crate) fn theme_session(&self) -> Option<u64> {
        self.gallery.as_ref().map(|_| self.theme_generation)
    }
    pub(crate) fn take_theme_intent(&mut self) -> Option<ThemeIntent> {
        if self.gallery.is_some() {
            if let Some(edit) = self.pending.take() {
                self.edit_gallery_value(edit);
            }
        }
        self.pending_theme.take()
    }
    pub(crate) fn open_theme_gallery(&mut self) {
        if !self.is_open() {
            return;
        }
        let Some(context) = &self.theme_context else {
            self.set_status("Theme gallery is loading. Reopen Customizations to retry.");
            return;
        };
        self.theme_generation = self.theme_generation.wrapping_add(1);
        let mut gallery = Gallery::default();
        gallery.entries.push(ThemeDescriptor::parsed(
            "configuration".into(),
            "Use configuration".into(),
            "Follow the theme in your configuration".into(),
            ThemeSource::Configuration,
            context.configured.clone(),
        ));
        if let Some(saved) = &context.saved {
            gallery.entries.push(ThemeDescriptor::parsed(
                "saved".into(),
                saved.name.clone(),
                "Your applied palette".into(),
                ThemeSource::Saved,
                saved.theme.clone(),
            ));
            gallery.selected = 1;
        }
        gallery.notice = "Loading theme library…".into();
        gallery.busy = true;
        self.gallery = Some(gallery);
        self.pending_theme = Some(ThemeIntent::Load);
        self.preedit.clear();
        self.pressed = None;
        self.focus = Focus::List;
        self.layout_dirty = true;
        self.reveal_focus = true;
    }
    pub(crate) fn theme_inventory(
        &mut self,
        entries: Vec<ThemeDescriptor>,
        limited: bool,
    ) {
        let Some(gallery) = &mut self.gallery else {
            return;
        };
        let selected = gallery
            .entries
            .get(gallery.selected)
            .map(|entry| (entry.id.clone(), entry.selection()));
        gallery.entries.retain(|e| {
            matches!(e.source, ThemeSource::Configuration | ThemeSource::Saved)
        });
        gallery.entries.extend(entries);
        gallery.selected = selected
            .as_ref()
            .and_then(|(id, _)| gallery.entries.iter().position(|e| &e.id == id))
            .unwrap_or(0);
        let changed = gallery
            .entries
            .get(gallery.selected)
            .map(|entry| (entry.id.clone(), entry.selection()))
            != selected;
        gallery.notice = if limited {
            "Library limit reached; some files were omitted."
        } else {
            "Preview only · Enter applies · Esc restores"
        }
        .into();
        gallery.busy = false;
        self.layout_dirty = true;
        if changed {
            self.gallery_preview();
        }
    }
    pub(crate) fn theme_added(&mut self, entry: ThemeDescriptor) {
        let Some(gallery) = &mut self.gallery else {
            return;
        };
        gallery.entries.retain(|e| e.id != entry.id);
        gallery.entries.push(entry);
        gallery.selected = gallery.entries.len() - 1;
        gallery.draft = None;
        gallery.original = None;
        gallery.fields.clear();
        gallery.busy = false;
        gallery.focus = 0;
        gallery.notice = "Imported locally. Preview now; Apply to use it.".into();
        self.gallery_preview();
        self.layout_dirty = true;
        self.reveal_focus = true;
    }
    pub(crate) fn theme_notice(&mut self, message: &str, busy: bool) {
        if let Some(gallery) = &mut self.gallery {
            gallery.notice = message.into();
            gallery.busy = busy;
            self.layout_dirty = true;
        }
    }
    pub(super) fn gallery_target(&self, x: f32, y: f32) -> Option<Target> {
        if self.layout_dirty {
            return None;
        }
        self.gallery
            .as_ref()?
            .targets
            .iter()
            .find(|(_, r)| r.contains(x, y))
            .map(|(t, _)| Target::Theme(*t))
    }
    pub(super) fn gallery_scroll(&mut self, delta: f32) {
        if let Some(gallery) = &mut self.gallery {
            if delta > 0.0 {
                gallery.first = gallery.first.saturating_add(1);
            } else {
                gallery.first = gallery.first.saturating_sub(1);
            }
            self.layout_dirty = true;
            self.reveal_focus = false;
            self.pressed = None;
        }
    }
    fn gallery_preview(&mut self) {
        let Some(gallery) = &self.gallery else {
            return;
        };
        if gallery.draft.is_none()
            && gallery
                .entries
                .get(gallery.selected)
                .is_some_and(|e| e.source == ThemeSource::Configuration)
        {
            self.pending_theme = Some(ThemeIntent::Preview(None));
        } else if let Some(selection) = gallery.selection() {
            self.pending_theme = Some(ThemeIntent::Preview(Some(selection)));
        }
    }
    fn gallery_back(&mut self) {
        if let Some(gallery) = &mut self.gallery {
            if gallery.draft.take().is_some() {
                gallery.original = None;
                gallery.fields.clear();
                gallery.selected = gallery.source_selected;
                gallery.first = 0;
                gallery.focus = 0;
                self.gallery_preview();
                self.layout_dirty = true;
                return;
            }
        }
        self.gallery = None;
        self.pending_theme = Some(ThemeIntent::Cancel);
        self.layout_dirty = true;
        self.reveal_focus = true;
    }
    pub(super) fn gallery_key(
        &mut self,
        key: &Key,
        modifiers: ModifiersState,
        repeat: bool,
    ) {
        if matches!(key, Key::Named(NamedKey::Escape)) {
            self.gallery_back();
            return;
        }
        if self.width < self.font.max(10.0) * 22.0
            || self.height < self.font.max(10.0) * 17.0
        {
            return;
        }
        if modifiers.control_key() || modifiers.super_key() || modifiers.alt_key() {
            return;
        }
        let Some(gallery) = &mut self.gallery else {
            return;
        };
        match key {
            Key::Named(NamedKey::Tab) => {
                let count = gallery.actions().len() + 1;
                gallery.focus = if modifiers.shift_key() {
                    (gallery.focus + count - 1) % count
                } else {
                    (gallery.focus + 1) % count
                };
            }
            Key::Named(
                NamedKey::ArrowDown
                | NamedKey::ArrowRight
                | NamedKey::ArrowUp
                | NamedKey::ArrowLeft
                | NamedKey::Home
                | NamedKey::End
                | NamedKey::PageUp
                | NamedKey::PageDown,
            ) if gallery.focus == 0 => {
                let last = gallery.count().saturating_sub(1);
                let page = gallery.visible.max(1);
                gallery.selected = match key {
                    Key::Named(NamedKey::Home) => 0,
                    Key::Named(NamedKey::End) => last,
                    Key::Named(NamedKey::ArrowUp | NamedKey::ArrowLeft) => {
                        gallery.selected.saturating_sub(1)
                    }
                    Key::Named(NamedKey::PageUp) => gallery.selected.saturating_sub(page),
                    Key::Named(NamedKey::PageDown) => {
                        gallery.selected.saturating_add(page).min(last)
                    }
                    _ => (gallery.selected + 1).min(last),
                };
                if gallery.draft.is_none() {
                    self.gallery_preview();
                }
                self.reveal_focus = true;
            }
            Key::Named(NamedKey::Enter | NamedKey::Space) if !repeat => {
                let target = if gallery.focus == 0 {
                    if gallery.draft.is_some() {
                        GalleryTarget::Row(gallery.selected)
                    } else {
                        GalleryTarget::Apply
                    }
                } else {
                    gallery.actions()[gallery.focus - 1]
                };
                self.gallery_activate(target);
            }
            _ => {}
        }
        self.layout_dirty = true;
    }
    pub(super) fn gallery_activate(&mut self, target: GalleryTarget) {
        if target == GalleryTarget::Back {
            self.gallery_back();
            return;
        }
        let Some(gallery) = &mut self.gallery else {
            return;
        };
        if let GalleryTarget::Row(index) = target {
            if index >= gallery.count() {
                return;
            }
            gallery.selected = index;
            gallery.focus = 0;
            if gallery.draft.is_some() {
                self.open_gallery_color();
            } else {
                self.gallery_preview();
            }
            self.layout_dirty = true;
            return;
        }
        if let Some(reason) = gallery.unavailable(target, self.temporary_customizations) {
            gallery.notice = reason.into();
            self.layout_dirty = true;
            return;
        }
        match target {
            GalleryTarget::Apply => {
                if self.temporary_customizations {
                    gallery.notice =
                        "Restore saved customizations before applying a theme.".into();
                    return;
                }
                if let Some(draft) = &gallery.draft {
                    self.pending_theme = Some(ThemeIntent::SaveCopy(draft.clone()));
                } else if gallery
                    .entries
                    .get(gallery.selected)
                    .is_some_and(|e| e.source == ThemeSource::Configuration)
                {
                    self.pending_theme = Some(ThemeIntent::Apply(None));
                } else if let Some(selection) = gallery.selection() {
                    self.pending_theme = Some(ThemeIntent::Apply(Some(selection)));
                }
            }
            GalleryTarget::Customize if gallery.draft.is_none() => {
                if let Some(mut selection) = gallery.selection() {
                    while selection.name.len() > theme_gallery::MAX_NAME_BYTES - 5 {
                        selection.name.pop();
                    }
                    selection.name.push_str(" copy");
                    gallery.source_selected = gallery.selected;
                    gallery.original = Some(selection.clone());
                    gallery.fields = theme_gallery::editable_colors(&selection.theme)
                        .unwrap_or_default();
                    gallery.draft = Some(selection);
                    gallery.selected = 0;
                    gallery.first = 0;
                    gallery.focus = 0;
                    gallery.notice = "Arrows / wheel: browse colors · Enter: edit · Save copy: keep changes".into();
                }
            }
            GalleryTarget::Import if gallery.draft.is_none() => {
                self.pending_theme = Some(ThemeIntent::Import)
            }
            GalleryTarget::Export => {
                if let Some(selection) = gallery.selection() {
                    self.pending_theme = Some(ThemeIntent::Export(selection));
                }
            }
            GalleryTarget::Configuration
                if !self.temporary_customizations && gallery.draft.is_none() =>
            {
                self.pending_theme = Some(ThemeIntent::Apply(None))
            }
            GalleryTarget::Refresh if gallery.draft.is_none() => {
                self.pending_theme = Some(ThemeIntent::Load)
            }
            _ => {}
        }
        self.layout_dirty = true;
    }
    fn open_gallery_color(&mut self) {
        let Some(gallery) = &self.gallery else {
            return;
        };
        let Some(draft) = &gallery.draft else {
            return;
        };
        let Some(original) = &gallery.original else {
            return;
        };
        let (key, value, default, kind) = if gallery.selected == 0 {
            (
                "name".to_owned(),
                SettingValue::Text(draft.name.clone()),
                SettingValue::Text(original.name.clone()),
                SettingKind::Text {
                    max_bytes: theme_gallery::MAX_NAME_BYTES,
                    allow_empty: false,
                },
            )
        } else {
            let Some((key, color)) = gallery.fields.get(gallery.selected - 1) else {
                return;
            };
            let default = theme_gallery::editable_colors(&original.theme)
                .ok()
                .and_then(|fields| fields.into_iter().find(|(id, _)| id == key))
                .map_or(*color, |(_, color)| color);
            (
                key.clone(),
                SettingValue::Color(*color),
                SettingValue::Color(default),
                SettingKind::Color { alpha: true },
            )
        };
        let Ok(id) = SettingId::new(format!("themes.{key}")) else {
            return;
        };
        let entry = SettingDescriptor {
            id,
            owner: SettingOwner::Core,
            section: Section::Customizations,
            label: key.replace('-', " "),
            description: "Edit your theme copy".into(),
            keywords: vec![],
            kind,
            value,
            default,
            origin: ValueOrigin::User,
            availability: Availability::Available,
            scope: ChangeScope::Immediate,
        };
        let revision = self.catalog.as_ref().map_or(0, Catalog::revision);
        let Ok(catalog) = Catalog::new(revision, vec![entry]) else {
            return;
        };
        self.theme_editor_backup = self.catalog.take().zip(self.view.take());
        self.view = Some(ViewState::new(&catalog, 1));
        self.catalog = Some(catalog);
        self.open_color();
        if self.color_editor.is_none() {
            self.restore_gallery_catalog();
        }
    }
    pub(super) fn restore_gallery_catalog(&mut self) {
        if self.theme_editor_backup.is_none() {
            return;
        }
        // Resolve Reset while the single-field catalogue still supplies its default.
        if let Some(edit) = self.pending.take() {
            let edit = if matches!(edit.change, Change::Reset) {
                self.catalog
                    .as_ref()
                    .and_then(|catalog| catalog.get(&edit.id))
                    .map(|entry| Edit {
                        change: Change::Set(entry.default.clone()),
                        ..edit.clone()
                    })
                    .unwrap_or(edit)
            } else {
                edit
            };
            self.pending = Some(edit);
        }
        if let Some((catalog, view)) = self.theme_editor_backup.take() {
            self.catalog = Some(catalog);
            self.view = Some(view);
        }
    }
    fn edit_gallery_value(&mut self, edit: Edit) {
        let Some(key) = edit.id.as_str().strip_prefix("themes.") else {
            self.pending = Some(edit);
            return;
        };
        let Some(gallery) = &mut self.gallery else {
            return;
        };
        let Some(draft) = &mut gallery.draft else {
            return;
        };
        match edit.change {
            Change::Set(SettingValue::Text(name))
                if key == "name" && theme_gallery::valid_name(&name) =>
            {
                draft.name = name
            }
            Change::Set(SettingValue::Color(color)) => {
                if let Ok(theme) = theme_gallery::edit_color(&draft.theme, key, color) {
                    draft.theme = theme;
                }
            }
            _ => {
                gallery.notice = "Enter a valid theme name or color.".into();
                return;
            }
        }
        gallery.fields = theme_gallery::editable_colors(&draft.theme).unwrap_or_default();
        self.gallery_preview();
        self.layout_dirty = true;
    }
    pub(super) fn prepare_gallery(&mut self, viewport: Rect) {
        let Some(gallery) = &mut self.gallery else {
            return;
        };
        let f = self.font.max(10.0);
        let pad = 16.0;
        let card = Rect {
            x: pad,
            y: pad,
            width: (viewport.width - pad * 2.0).max(0.0),
            height: (viewport.height - pad * 2.0).max(0.0),
        };
        self.geometry = Geometry {
            viewport,
            card,
            status: Rect {
                x: card.x + pad,
                y: card.y + card.height - f * 3.0,
                width: (card.width - pad * 2.0).max(0.0),
                height: f * 2.7,
            },
            ..Geometry::default()
        };
        let wide = card.width >= f * 52.0;
        let columns = if wide { 4 } else { 2 };
        let action_rows = gallery.actions().len().div_ceil(columns);
        let body = Rect {
            x: card.x + pad,
            y: card.y + f * 3.5,
            width: (card.width - pad * 2.0).max(0.0),
            height: (card.height - f * (6.5 + action_rows as f32 * 1.9) - 8.0).max(0.0),
        };
        gallery.list = Rect {
            width: if wide { body.width * 0.39 } else { body.width },
            height: if wide { body.height } else { body.height * 0.6 },
            ..body
        };
        gallery.preview = if wide {
            Rect {
                x: body.x + gallery.list.width + pad,
                width: (body.width - gallery.list.width - pad).max(0.0),
                ..body
            }
        } else {
            Rect {
                y: body.y + gallery.list.height + 8.0,
                height: (body.height - gallery.list.height - 8.0).max(0.0),
                ..body
            }
        };
        let row_h = if gallery.draft.is_some() {
            f * 2.0
        } else {
            f * 5.0
        };
        gallery.visible = (gallery.list.height / row_h).floor().max(0.0) as usize;
        if self.reveal_focus {
            if gallery.selected < gallery.first {
                gallery.first = gallery.selected;
            }
            if gallery.selected >= gallery.first + gallery.visible {
                gallery.first = gallery
                    .selected
                    .saturating_sub(gallery.visible.saturating_sub(1));
            }
        }
        gallery.first = gallery
            .first
            .min(gallery.count().saturating_sub(gallery.visible.max(1)));
        gallery.targets.clear();
        for index in gallery.first..(gallery.first + gallery.visible).min(gallery.count())
        {
            gallery.targets.push((
                GalleryTarget::Row(index),
                Rect {
                    y: gallery.list.y + (index - gallery.first) as f32 * row_h,
                    height: row_h - 6.0,
                    ..gallery.list
                },
            ));
        }
        let width = (body.width - (columns - 1) as f32 * 8.0) / columns as f32;
        let top = body.y + body.height + f * 0.5;
        if card.width < f * 22.0 || card.height < f * 17.0 {
            // The compact fallback reserves its bottom edge for the Back button.
            self.geometry.status = Rect::default();
            gallery.targets.clear();
            gallery.targets.push((
                GalleryTarget::Back,
                Rect {
                    x: card.x + 8.0,
                    y: (card.y + card.height - f * 2.0).max(card.y),
                    width: (card.width - 16.0).max(0.0),
                    height: (f * 1.6).min(card.height),
                },
            ));
            gallery.notice = "Enlarge the window to browse themes. Esc returns.".into();
            gallery.preview = Rect::default();
            self.layout_dirty = false;
            self.reveal_focus = false;
            return;
        }
        for (index, target) in gallery.actions().iter().enumerate() {
            gallery.targets.push((
                *target,
                Rect {
                    x: body.x + (index % columns) as f32 * (width + 8.0),
                    y: top + (index / columns) as f32 * f * 1.9,
                    width: width.max(0.0),
                    height: f * 1.65,
                },
            ));
        }
        self.layout_dirty = false;
        self.reveal_focus = false;
    }
    pub(super) fn paint_gallery(&self, canvas: &mut impl Canvas, theme: UiTheme) {
        let Some(gallery) = &self.gallery else {
            return;
        };
        let g = self.geometry;
        let f = self.font.max(10.0);
        rect(canvas, g.viewport, theme.background, g.viewport);
        rounded_surface(canvas, g.card, theme.surface, g.viewport);
        shortcut_hint(canvas, g.status,
            "Arrows: preview | Tab: focus | Enter: select | Esc / Backspace / Alt+Left: back",
            f * 0.73, theme, g.card);
        label(
            canvas,
            Rect {
                x: g.card.x + 16.0,
                y: g.card.y + 12.0,
                width: g.card.width - 32.0,
                height: f * 1.5,
            },
            if gallery.draft.is_some() {
                "Customize theme copy"
            } else {
                "Themes"
            },
            f * 1.1,
            theme.text,
            true,
            g.card,
        );
        label(
            canvas,
            Rect {
                x: g.card.x + 16.0,
                y: g.card.y + f * 2.1,
                width: g.card.width - 32.0,
                height: f * 1.2,
            },
            &gallery.notice,
            f * 0.73,
            theme.muted_text,
            false,
            g.card,
        );
        for (target, bounds) in &gallery.targets {
            if let GalleryTarget::Row(index) = target {
                control(
                    canvas,
                    *bounds,
                    *index == gallery.selected && gallery.focus == 0,
                    theme,
                    gallery.list,
                );
                if let Some(draft) = &gallery.draft {
                    let (name, value) = if *index == 0 {
                        ("Name".into(), draft.name.clone())
                    } else {
                        let (key, color) = &gallery.fields[index - 1];
                        (
                            key.replace('-', " "),
                            format!(
                                "#{:02x}{:02x}{:02x}{:02x}",
                                color[0], color[1], color[2], color[3]
                            ),
                        )
                    };
                    label(
                        canvas,
                        *bounds,
                        &format!("{name}  {value}"),
                        f * 0.77,
                        theme.text,
                        false,
                        gallery.list,
                    );
                } else if let Some(entry) = gallery.entries.get(*index) {
                    label(
                        canvas,
                        Rect {
                            height: f * 1.5,
                            ..*bounds
                        },
                        &entry.name,
                        f * 0.95,
                        theme.text,
                        true,
                        gallery.list,
                    );
                    let mode = if entry.appearance
                        == rio_backend::config::theme::AppearanceTheme::Light
                    {
                        "Light"
                    } else {
                        "Dark"
                    };
                    let metadata = match &entry.validation {
                        ValidationStatus::Invalid(_) => "Invalid file".into(),
                        ValidationStatus::LowContrast => {
                            format!("{mode} · {} · Low contrast", entry.source.label())
                        }
                        ValidationStatus::Valid => {
                            format!("{mode} · {}", entry.source.label())
                        }
                    };
                    label(
                        canvas,
                        Rect {
                            y: bounds.y + f * 1.6,
                            height: f,
                            ..*bounds
                        },
                        &metadata,
                        f * 0.69,
                        theme.muted_text,
                        false,
                        gallery.list,
                    );
                    for (swatch, color) in entry.palette_preview.iter().enumerate() {
                        rounded_surface(
                            canvas,
                            Rect {
                                x: bounds.x + 8.0 + swatch as f32 * f * 1.6,
                                y: bounds.y + f * 3.0,
                                width: f * 1.2,
                                height: f * 0.7,
                            },
                            *color,
                            gallery.list,
                        );
                    }
                }
            } else {
                let index = gallery
                    .actions()
                    .iter()
                    .position(|a| a == target)
                    .unwrap_or(0);
                let enabled = gallery
                    .unavailable(*target, self.temporary_customizations)
                    .is_none();
                control(canvas, *bounds, gallery.focus == index + 1, theme, g.card);
                let caption = match target {
                    GalleryTarget::Apply => {
                        if gallery.draft.is_some() {
                            "Save copy"
                        } else {
                            "Apply · Enter"
                        }
                    }
                    GalleryTarget::Customize => "Customize",
                    GalleryTarget::Import => "Import…",
                    GalleryTarget::Export => "Export…",
                    GalleryTarget::Configuration => "Use configuration",
                    GalleryTarget::Refresh => "Refresh",
                    GalleryTarget::Back => {
                        if gallery.draft.is_some() {
                            "Cancel copy · Esc"
                        } else {
                            "Back · Esc"
                        }
                    }
                    _ => "",
                };
                label(
                    canvas,
                    *bounds,
                    caption,
                    f * 0.76,
                    if enabled {
                        theme.text
                    } else {
                        theme.muted_text
                    },
                    false,
                    g.card,
                );
            }
        }
        if gallery.count() > gallery.visible && gallery.visible > 0 {
            let track = gallery.list;
            let height = (track.height * gallery.visible as f32 / gallery.count() as f32)
                .max(8.0)
                .min(track.height);
            let travel = (track.height - height).max(0.0);
            let offset = travel * gallery.first as f32
                / (gallery.count() - gallery.visible).max(1) as f32;
            rounded_surface(
                canvas,
                Rect {
                    x: track.x + track.width - 4.0,
                    y: track.y + offset,
                    width: 3.0,
                    height,
                },
                theme.muted_text,
                track,
            );
        }
        if let Some(selection) = gallery.selection() {
            let c = selection.theme.colors;
            let p = gallery.preview;
            rounded_surface(canvas, p, c.background.0, g.card);
            let rows = [
                ("  Terminal preview", c.foreground, true),
                ("", c.foreground, false),
                ("  λ git status", c.cyan, false),
                ("  On branch main", c.foreground, false),
                ("  ✓ Working tree clean", c.green, false),
                ("  warning: example message", c.yellow, false),
                ("  error: example message", c.red, false),
                ("  const theme = 'your style'", c.magenta, false),
            ];
            for (index, (text, color, bold)) in rows.iter().enumerate() {
                label(
                    canvas,
                    Rect {
                        y: p.y + 12.0 + index as f32 * f * 1.7,
                        height: f * 1.6,
                        ..p
                    },
                    text,
                    f * 0.83,
                    *color,
                    *bold,
                    p,
                );
            }
            let contrast = theme_gallery::text_contrast(&selection.theme);
            let note = if contrast < 4.5 {
                "Low text/selection contrast: adjust your colors."
            } else if self.theme_context.as_ref().is_some_and(|c| c.font_colors) {
                "Your custom font colors still take precedence."
            } else {
                "Palette preview · your terminal updates instantly"
            };
            label(
                canvas,
                Rect {
                    x: p.x + 8.0,
                    y: (p.y + p.height - f * 2.6).max(p.y),
                    width: (p.width - 16.0).max(0.0),
                    height: f * 2.3,
                },
                note,
                f * 0.68,
                theme.muted_text,
                false,
                p,
            );
        } else if let Some(entry) = gallery.entries.get(gallery.selected) {
            if let ValidationStatus::Invalid(error) = &entry.validation {
                label(
                    canvas,
                    gallery.preview,
                    error,
                    f * 0.8,
                    theme.text,
                    false,
                    gallery.preview,
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn gallery() -> SettingsView {
        let mut view = SettingsView::default();
        view.fit(1000.0, 800.0, 16.0);
        let base = rio_backend::config::Config::default();
        let catalog =
            crate::settings_catalog::catalog(1, &base, &Default::default(), &[]).unwrap();
        view.open_with_section(catalog, Some(Section::Customizations));
        view.set_theme_context(ThemeContext {
            configured: Theme {
                colors: base.colors,
            },
            saved: None,
            font_colors: false,
        });
        view.open_theme_gallery();
        assert!(matches!(view.take_theme_intent(), Some(ThemeIntent::Load)));
        view.theme_inventory(theme_gallery::builtins(), false);
        view
    }
    fn key(view: &mut SettingsView, key: NamedKey) {
        view.key(&Key::Named(key), None, ModifiersState::empty(), false);
    }
    #[test]
    fn menu_back_leaves_gallery_and_cancels_preview_once() {
        for (key, modifiers) in [
            (NamedKey::ArrowLeft, ModifiersState::ALT),
            (NamedKey::Backspace, ModifiersState::empty()),
        ] {
            let mut view = gallery();
            view.key(&Key::Named(key), None, modifiers, false);
            assert!(view.gallery.is_none());
            assert!(view.is_category_root());
            assert!(matches!(
                view.take_theme_intent(),
                Some(ThemeIntent::Cancel)
            ));
            view.key(&Key::Named(key), None, modifiers, true);
            assert!(view.is_open());
            view.key(&Key::Named(key), None, modifiers, false);
            assert!(!view.is_open());
        }
    }
    #[test]
    fn gallery_cannot_open_after_settings_closes() {
        let mut view = gallery();
        view.close();
        view.open_theme_gallery();
        assert!(view.theme_session().is_none());
        assert!(!matches!(view.take_theme_intent(), Some(ThemeIntent::Load)));
    }
    #[test]
    fn gallery_arrows_preview_enter_applies_escape_restores_without_setting_edits() {
        let mut view = gallery();
        key(&mut view, NamedKey::ArrowDown);
        assert!(
            matches!(view.take_theme_intent(),Some(ThemeIntent::Preview(Some(s))) if s.name=="Aurora Night")
        );
        assert!(view.take_edit().is_none());
        for _ in 0..8 {
            key(&mut view, NamedKey::Tab);
        }
        assert_eq!(view.gallery.as_ref().unwrap().focus, 0);
        key(&mut view, NamedKey::Enter);
        assert!(
            matches!(view.take_theme_intent(),Some(ThemeIntent::Apply(Some(s))) if s.name=="Aurora Night")
        );
        key(&mut view, NamedKey::Escape);
        assert!(matches!(
            view.take_theme_intent(),
            Some(ThemeIntent::Cancel)
        ));
        assert!(view.theme_session().is_none());
        assert!(view.is_open());
    }
    #[test]
    fn gallery_copy_uses_existing_color_editor_and_never_mutates_builtin() {
        let mut view = gallery();
        key(&mut view, NamedKey::ArrowDown);
        view.take_theme_intent();
        let original = view.gallery.as_ref().unwrap().entries[1].theme.clone();
        view.gallery_activate(GalleryTarget::Customize);
        key(&mut view, NamedKey::Enter);
        assert!(view.color_editor.is_some());
        assert!(view.paste_color("My palette"));
        view.activate_color(ColorFocus::Apply);
        assert!(
            matches!(view.take_theme_intent(),Some(ThemeIntent::Preview(Some(s))) if s.name=="My palette")
        );
        key(&mut view, NamedKey::ArrowDown);
        key(&mut view, NamedKey::Enter);
        assert!(view.paste_color("#123456"));
        view.activate_color(ColorFocus::Apply);
        assert!(matches!(
            view.take_theme_intent(),
            Some(ThemeIntent::Preview(Some(_)))
        ));
        assert_eq!(view.gallery.as_ref().unwrap().entries[1].theme, original);
        assert!(view.theme_editor_backup.is_none());
        view.gallery_activate(GalleryTarget::Apply);
        assert!(
            matches!(view.take_theme_intent(),Some(ThemeIntent::SaveCopy(s)) if s.name=="My palette")
        );
        key(&mut view, NamedKey::Escape);
        assert!(view.gallery.as_ref().unwrap().draft.is_none());
        view.take_theme_intent();
        key(&mut view, NamedKey::Escape);
        assert!(matches!(
            view.take_theme_intent(),
            Some(ThemeIntent::Cancel)
        ));
    }
    #[test]
    fn gallery_refresh_previews_changed_or_removed_local_selection() {
        let mut view = gallery();
        let mut local = theme_gallery::builtins().remove(0);
        local.id = "local:example.toml".into();
        local.source = ThemeSource::Local;
        view.theme_added(local.clone());
        view.take_theme_intent();
        local.theme = theme_gallery::builtins().remove(1).theme;
        let expected = local.selection();
        view.theme_inventory(vec![local], false);
        assert!(
            matches!(view.take_theme_intent(), Some(ThemeIntent::Preview(selection)) if selection == expected)
        );
        view.theme_inventory(theme_gallery::builtins(), false);
        assert!(matches!(
            view.take_theme_intent(),
            Some(ThemeIntent::Preview(None))
        ));
    }
    #[test]
    fn gallery_invalid_files_and_temporary_resets_cannot_apply() {
        let mut view = gallery();
        view.theme_added(ThemeDescriptor::invalid(
            "bad".into(),
            "Broken".into(),
            "Invalid colors".into(),
        ));
        assert!(view.take_theme_intent().is_none());
        view.gallery_activate(GalleryTarget::Apply);
        assert!(view.take_theme_intent().is_none());
        view.gallery_activate(GalleryTarget::Row(1));
        view.take_theme_intent();
        view.set_temporary_customizations(true);
        view.gallery_activate(GalleryTarget::Apply);
        assert!(view.take_theme_intent().is_none());
        view.gallery_activate(GalleryTarget::Configuration);
        assert!(view.take_theme_intent().is_none());
    }
    #[test]
    fn gallery_layout_and_scrolling_keep_targets_within_viewport() {
        for (w, h, f) in [
            (1000.0, 800.0, 16.0),
            (500.0, 700.0, 16.0),
            (900.0, 640.0, 24.0),
        ] {
            let mut view = gallery();
            view.fit(w, h, f);
            view.prepare_gallery(Rect {
                x: 0.0,
                y: 0.0,
                width: w,
                height: h,
            });
            let gallery = view.gallery.as_ref().unwrap();
            assert!(gallery.visible > 0);
            for (_, r) in &gallery.targets {
                assert!(
                    r.x >= 0.0 && r.y >= 0.0 && r.x + r.width <= w && r.y + r.height <= h,
                    "{r:?}"
                );
                assert!(r.y + r.height <= view.geometry.status.y);
            }
            key(&mut view, NamedKey::End);
            view.prepare_gallery(Rect {
                x: 0.0,
                y: 0.0,
                width: w,
                height: h,
            });
            let gallery = view.gallery.as_ref().unwrap();
            assert!(gallery
                .targets
                .iter()
                .any(|(t, _)| *t == GalleryTarget::Row(5)));
        }
    }

    #[test]
    fn gallery_compact_back_target_does_not_overlap_footer_hints() {
        let mut view = gallery();
        view.fit(320.0, 240.0, 16.0);
        view.prepare_gallery(Rect {
            x: 0.0,
            y: 0.0,
            width: 320.0,
            height: 240.0,
        });
        let gallery = view.gallery.as_ref().unwrap();
        assert_eq!(gallery.targets.len(), 1);
        assert_eq!(gallery.targets[0].0, GalleryTarget::Back);
        assert_eq!(view.geometry.status.height, 0.0);
    }
}
