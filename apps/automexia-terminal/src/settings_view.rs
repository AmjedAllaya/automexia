//! Application-owned settings sheet. Effects remain typed application intents.
use crate::renderer::ui_theme::{color_u8, UiTheme};
use automexia_ui_model::settings::{
    Catalog, Change, Edit, FocusMove, SettingDescriptor, SettingId, SettingKind,
    SettingValue, ValueOrigin, ViewState, MAX_QUERY_BYTES,
};
use rio_backend::sugarloaf::{
    text::{DrawOpts, Text},
    Sugarloaf,
};
use rio_window::{
    event::{ElementState, Ime, MouseButton, MouseScrollDelta, TouchPhase, WindowEvent},
    keyboard::{Key, ModifiersState, NamedKey},
};
use unicode_segmentation::UnicodeSegmentation;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Rect {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}
impl Rect {
    fn array(self) -> [f32; 4] {
        [self.x, self.y, self.width, self.height]
    }
    fn contains(self, x: f32, y: f32) -> bool {
        x >= self.x && x < self.x + self.width && y >= self.y && y < self.y + self.height
    }
    fn intersect(self, other: Self) -> Option<Self> {
        let x = self.x.max(other.x);
        let y = self.y.max(other.y);
        let width = (self.x + self.width).min(other.x + other.width) - x;
        let height = (self.y + self.height).min(other.y + other.height) - y;
        (width > 0.0 && height > 0.0).then_some(Self {
            x,
            y,
            width,
            height,
        })
    }
}
#[derive(Clone, Copy, Debug, Default)]
struct Geometry {
    viewport: Rect,
    card: Rect,
    search: Rect,
    body: Rect,
    reset: Rect,
    close: Rect,
    status: Rect,
}
#[derive(Clone, Debug)]
struct Row {
    id: SettingId,
    bounds: Rect,
    control: Rect,
    lines: Vec<String>,
    label_lines: usize,
    value_lines: Vec<String>,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Focus {
    #[default]
    Search,
    List,
    Reset,
    Close,
}
#[derive(Clone, Debug, PartialEq, Eq)]
enum Target {
    Search,
    Control(SettingId, i32),
    Reset,
    Close,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct EventResult {
    pub(crate) consumed: bool,
    pub(crate) redraw: bool,
    pub(crate) closed: bool,
}
trait Canvas {
    fn text(&mut self) -> &mut Text;
    fn rect(&mut self, bounds: [f32; 4], color: [f32; 4]);
}
impl Canvas for Sugarloaf<'_> {
    fn text(&mut self) -> &mut Text {
        self.text_mut()
    }
    fn rect(&mut self, [x, y, width, height]: [f32; 4], color: [f32; 4]) {
        Sugarloaf::rect(self, None, x, y, width, height, color, 0.0, 30);
    }
}

#[derive(Default)]
pub(crate) struct SettingsView {
    catalog: Option<Catalog>,
    view: Option<ViewState>,
    focus: Focus,
    pending: Option<Edit>,
    preedit: String,
    caret: usize,
    anchor: Option<usize>,
    status: String,
    saving: Option<u64>,
    width: f32,
    height: f32,
    font: f32,
    geometry: Geometry,
    caret_rect: Rect,
    rows: Vec<Row>,
    compact_lines: Vec<String>,
    layout_dirty: bool,
    reveal_focus: bool,
    scroll: f32,
    content_height: f32,
    pointer: Option<(f32, f32)>,
    pressed: Option<Target>,
    touch: Option<(u64, f32)>,
}
impl SettingsView {
    pub(crate) fn open(&mut self, catalog: Catalog) {
        self.close();
        self.view = Some(ViewState::new(&catalog, 6));
        self.catalog = Some(catalog);
        self.focus = Focus::Search;
        self.layout_dirty = true;
        self.reveal_focus = true;
    }
    pub(crate) fn close(&mut self) {
        self.catalog = None;
        self.view = None;
        self.pending = None;
        self.preedit.clear();
        self.status.clear();
        self.saving = None;
        self.rows.clear();
        self.compact_lines.clear();
        self.pressed = None;
        self.touch = None;
        self.pointer = None;
        self.scroll = 0.0;
        self.caret = 0;
        self.anchor = None;
    }
    pub(crate) fn is_open(&self) -> bool {
        self.catalog.is_some()
    }
    pub(crate) fn refresh(&mut self, catalog: Catalog) {
        if !self.is_open() {
            return;
        }
        if self
            .catalog
            .as_ref()
            .is_some_and(|previous| previous.revision() != catalog.revision())
        {
            self.pending = None;
            self.pressed = None;
        }
        if let Some(view) = &mut self.view {
            view.refresh(&catalog);
        }
        self.catalog = Some(catalog);
        self.layout_dirty = true;
        self.reveal_focus = true;
    }
    pub(crate) fn fit(&mut self, width: f32, height: f32, font: f32) {
        let clean = |value: f32, max: f32| {
            if value.is_finite() {
                value.clamp(0.0, max)
            } else {
                0.0
            }
        };
        let width = clean(width, 32768.0);
        let height = clean(height, 32768.0);
        let font = if font.is_finite() {
            font.clamp(10.0, 64.0)
        } else {
            14.0
        };
        if (width, height, font) != (self.width, self.height, self.font) {
            self.width = width;
            self.height = height;
            self.font = font;
            self.layout_dirty = true;
            self.reveal_focus = true;
            self.pressed = None;
            if self.requires_larger_window() {
                self.pending = None;
                self.preedit.clear();
                self.anchor = None;
            }
        }
    }
    pub(crate) fn event(
        &mut self,
        event: &WindowEvent,
        modifiers: ModifiersState,
        scale: f64,
    ) -> EventResult {
        if !self.is_open() {
            return EventResult::default();
        }
        let scale = if scale.is_finite() && scale > 0.0 {
            scale
        } else {
            1.0
        };
        let mut result = EventResult {
            consumed: true,
            redraw: true,
            closed: false,
        };
        match event {
            WindowEvent::KeyboardInput {
                event,
                is_synthetic,
                ..
            } => {
                if event.state == ElementState::Pressed && !is_synthetic {
                    self.key(
                        &event.logical_key,
                        event.text.as_deref(),
                        modifiers,
                        event.repeat,
                    );
                }
            }
            WindowEvent::Ime(Ime::Commit(text)) => {
                self.preedit.clear();
                self.paste(text);
            }
            WindowEvent::Ime(Ime::Preedit(text, _)) => {
                self.preedit.clear();
                if self.focus == Focus::Search
                    && !self.requires_larger_window()
                    && self.query().len().saturating_add(text.len()) <= MAX_QUERY_BYTES
                {
                    if let Some((mut probe, catalog)) =
                        self.view.clone().zip(self.catalog.as_ref())
                    {
                        if probe.set_query(text, catalog).is_ok() {
                            self.preedit = text.clone();
                        }
                    }
                }
            }
            WindowEvent::Ime(Ime::Disabled) => {
                self.preedit.clear();
            }
            WindowEvent::Ime(Ime::Enabled) => {}
            WindowEvent::CursorMoved { position, .. } => {
                let point = ((position.x / scale) as f32, (position.y / scale) as f32);
                self.pointer =
                    (point.0.is_finite() && point.1.is_finite()).then_some(point);
            }
            WindowEvent::CursorLeft { .. } => {
                self.pointer = None;
                self.pressed = None;
            }
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state,
                ..
            } => {
                let target = self.pointer.and_then(|(x, y)| self.target_at(x, y));
                if *state == ElementState::Pressed {
                    self.pressed = target;
                } else if let Some(pressed) = self.pressed.take() {
                    if target.as_ref() == Some(&pressed) {
                        self.activate_target(pressed);
                    }
                }
            }
            WindowEvent::MouseInput { .. } => {}
            WindowEvent::MouseWheel { delta, .. } => {
                let amount = match delta {
                    MouseScrollDelta::LineDelta(_, y) => *y * self.font.max(10.0) * 3.0,
                    MouseScrollDelta::PixelDelta(point) => (point.y / scale) as f32,
                };
                self.scroll_by(-amount);
            }
            WindowEvent::Touch(touch) => {
                let x = (touch.location.x / scale) as f32;
                let y = (touch.location.y / scale) as f32;
                match touch.phase {
                    TouchPhase::Started if self.touch.is_none() => {
                        self.touch = Some((touch.id, y));
                        self.pressed = self.target_at(x, y);
                    }
                    TouchPhase::Moved => {
                        if let Some((id, previous)) = self.touch {
                            if id == touch.id {
                                let delta = previous - y;
                                if delta.abs() > 2.0 {
                                    self.pressed = None;
                                }
                                self.scroll_by(delta);
                                self.touch = Some((id, y));
                            }
                        }
                    }
                    TouchPhase::Ended | TouchPhase::Cancelled
                        if self.touch.is_some_and(|(id, _)| id == touch.id) =>
                    {
                        self.touch = None;
                        if let Some(target) = self.pressed.take() {
                            if touch.phase == TouchPhase::Ended
                                && self.target_at(x, y).as_ref() == Some(&target)
                            {
                                self.activate_target(target);
                            }
                        }
                    }
                    _ => {}
                }
            }
            WindowEvent::DroppedFile(_)
            | WindowEvent::HoveredFile(_)
            | WindowEvent::HoveredFileCancelled => {}
            WindowEvent::Focused(false) => {
                self.preedit.clear();
                self.pressed = None;
                self.touch = None;
                result.consumed = false;
            }
            _ => {
                result.consumed = false;
                result.redraw = false;
            }
        }
        result.closed = !self.is_open();
        result
    }
    pub(crate) fn draw(&mut self, sugarloaf: &mut Sugarloaf, theme: UiTheme) {
        if !self.is_open() {
            return;
        }
        sugarloaf.begin_modal_layer();
        self.paint(sugarloaf, theme);
        sugarloaf.end_modal_layer();
    }
    pub(crate) fn take_edit(&mut self) -> Option<Edit> {
        self.pending.take()
    }
    pub(crate) fn save_started(&mut self, revision: u64) {
        if !self.is_open() {
            return;
        }
        self.saving = Some(revision);
        self.status = "Saving settings...".into();
    }
    pub(crate) fn save_completed(&mut self, revision: u64, success: bool) {
        if self.saving.is_none_or(|pending| revision < pending) {
            return;
        }
        self.saving = None;
        if success {
            self.status = "Saved".into();
        } else {
            self.save_failed();
        }
    }
    pub(crate) fn requires_larger_window(&self) -> bool {
        let font = self.font.max(10.0);
        let margin = if self.width < 360.0 || self.height < 300.0 {
            4.0
        } else {
            16.0
        };
        let width = (self.width - 2.0 * margin).max(0.0);
        let height = (self.height - 2.0 * margin).max(0.0);
        let pad = 8.0_f32.min(width * 0.1);
        // Room for full-height search/footer controls and a two-line label with
        // its value. Below this geometry hidden controls cannot safely be edited.
        width < font * 6.0 + pad * 4.0 || height < font * 1.45 * 7.0 + pad * 8.0 + 4.0
    }

    pub(crate) fn ime_cursor_area(&self) -> Option<[f32; 4]> {
        if !self.is_open()
            || self.focus != Focus::Search
            || self.layout_dirty
            || self.requires_larger_window()
        {
            return None;
        }
        self.caret_rect
            .intersect(self.geometry.search)
            .map(Rect::array)
    }
    pub(crate) fn save_failed(&mut self) {
        self.saving = None;
        self.set_status("Active this session; settings could not be saved. Change again or Reset to retry.");
    }
    pub(crate) fn set_status(&mut self, message: &str) {
        if !self.is_open() {
            return;
        }
        if message.len() <= 256 && !message.chars().any(char::is_control) {
            self.status = message.into();
        } else {
            self.status = "Settings could not be applied.".into();
        }
    }
    pub(crate) fn paste(&mut self, text: &str) -> bool {
        if !self.is_open()
            || self.requires_larger_window()
            || self.focus != Focus::Search
            || text.len() > MAX_QUERY_BYTES
        {
            return false;
        }
        let query = self.query();
        let end = self.caret.min(query.len());
        let start = self.anchor.unwrap_or(end).min(end);
        let finish = self.anchor.unwrap_or(end).max(end).min(query.len());
        if !query.is_char_boundary(start) || !query.is_char_boundary(finish) {
            return false;
        }
        let mut candidate = query.to_owned();
        candidate.replace_range(start..finish, text);
        let Some((view, catalog)) = self.view.as_mut().zip(self.catalog.as_ref()) else {
            return false;
        };
        if view.set_query(&candidate, catalog).is_err() {
            self.status =
                "Search text is too long or contains unsupported characters.".into();
            return false;
        }
        self.caret = start + text.len();
        self.anchor = None;
        self.layout_dirty = true;
        self.scroll = 0.0;
        self.reveal_focus = false;
        self.pressed = None;
        true
    }
    #[cfg(test)]
    pub(crate) fn accessibility_summary(&self) -> String {
        if !self.is_open() {
            return String::new();
        }
        if self.requires_larger_window() {
            return "Settings. Enlarge the window to use these controls. Escape closes Settings.".into();
        }
        let mut text = format!("Settings. Search: {}. {}", self.query(), self.status);
        if let Some(entry) = self.focused_entry() {
            text.push_str(&format!(
                " {}. {}. {}. {}.",
                entry.section.label(),
                entry.label,
                display_value(entry),
                entry.description
            ));
            if let Some(reason) = entry.availability.reason() {
                text.push_str(reason);
            }
        }
        text.push_str(" Tab changes focus. Arrows navigate. Space changes the value. Reset restores configuration. Escape closes settings.");
        text
    }
    fn query(&self) -> &str {
        self.view.as_ref().map_or("", ViewState::query)
    }
    pub(crate) fn key(
        &mut self,
        key: &Key,
        text: Option<&str>,
        modifiers: ModifiersState,
        repeat: bool,
    ) {
        if !self.is_open() {
            return;
        }
        if matches!(key, Key::Named(NamedKey::Escape)) {
            if self.preedit.is_empty() {
                self.close();
            } else {
                self.preedit.clear();
            }
            return;
        }
        if self.requires_larger_window() || !self.preedit.is_empty() {
            return;
        }
        if matches!(key, Key::Named(NamedKey::Tab)) {
            let index = match self.focus {
                Focus::Search => 0,
                Focus::List => 1,
                Focus::Reset => 2,
                Focus::Close => 3,
            };
            let next = if modifiers.shift_key() {
                (index + 3) % 4
            } else {
                (index + 1) % 4
            };
            self.focus = match next {
                0 => Focus::Search,
                1 => Focus::List,
                2 => Focus::Reset,
                _ => Focus::Close,
            };
            self.reveal_focus = self.focus == Focus::List;
            return;
        }
        if self.focus == Focus::Search {
            if (modifiers.control_key() || modifiers.super_key())
                && matches!(key,Key::Character(value) if value.eq_ignore_ascii_case("a"))
            {
                self.anchor = Some(0);
                self.caret = self.query().len();
                return;
            }
            match key {
                Key::Named(NamedKey::ArrowDown) => {
                    self.focus = Focus::List;
                    self.reveal_focus = true;
                }
                Key::Named(NamedKey::ArrowLeft) => {
                    self.caret = self.query()[..self.caret]
                        .grapheme_indices(true)
                        .next_back()
                        .map_or(0, |(start, _)| start);
                    self.anchor = None;
                }
                Key::Named(NamedKey::ArrowRight) => {
                    self.caret = self.query()[self.caret..]
                        .graphemes(true)
                        .next()
                        .map_or(self.caret, |next| self.caret + next.len());
                    self.anchor = None;
                }
                Key::Named(NamedKey::Home) => {
                    self.caret = 0;
                    self.anchor = None;
                }
                Key::Named(NamedKey::End) => {
                    self.caret = self.query().len();
                    self.anchor = None;
                }
                Key::Named(NamedKey::Backspace) => {
                    if self.anchor.is_none() {
                        self.anchor = self.query()[..self.caret]
                            .grapheme_indices(true)
                            .next_back()
                            .map(|(start, _)| start);
                    }
                    self.paste("");
                }
                Key::Named(NamedKey::Delete) => {
                    if self.anchor.is_none() {
                        self.anchor = self.query()[self.caret..]
                            .graphemes(true)
                            .next()
                            .map(|next| self.caret + next.len());
                    }
                    self.paste("");
                }
                _ if !modifiers.control_key()
                    && !modifiers.super_key()
                    && !modifiers.alt_key() =>
                {
                    if let Some(text) = text {
                        self.paste(text);
                    }
                }
                _ => {}
            }
            return;
        }
        if self.focus == Focus::List {
            let movement = match key {
                Key::Named(NamedKey::ArrowDown) => Some(FocusMove::Next),
                Key::Named(NamedKey::ArrowUp) => Some(FocusMove::Previous),
                Key::Named(NamedKey::PageDown) => Some(FocusMove::PageNext),
                Key::Named(NamedKey::PageUp) => Some(FocusMove::PagePrevious),
                Key::Named(NamedKey::Home) => Some(FocusMove::First),
                Key::Named(NamedKey::End) => Some(FocusMove::Last),
                _ => None,
            };
            if let Some(movement) = movement {
                if let Some(view) = &mut self.view {
                    view.move_focus(movement);
                }
                self.reveal_focus = true;
                return;
            }
            if !repeat {
                match key {
                    Key::Named(NamedKey::ArrowLeft) => self.change_value(-1),
                    Key::Named(
                        NamedKey::ArrowRight | NamedKey::Enter | NamedKey::Space,
                    ) => self.change_value(1),
                    _ => {}
                }
            }
        } else if !repeat && matches!(key, Key::Named(NamedKey::Enter | NamedKey::Space))
        {
            if self.focus == Focus::Close {
                self.close();
            } else {
                self.queue(Change::Reset);
            }
        }
    }
    fn focused_entry(&self) -> Option<&SettingDescriptor> {
        self.catalog.as_ref()?.get(self.view.as_ref()?.focused()?)
    }
    fn queue(&mut self, change: Change) {
        if self.pending.is_some() {
            return;
        }
        let Some((catalog, id)) = self
            .catalog
            .as_ref()
            .zip(self.view.as_ref().and_then(ViewState::focused))
        else {
            return;
        };
        let edit = Edit {
            revision: catalog.revision(),
            id: id.clone(),
            change,
        };
        match catalog.validate_edit(&edit) {
            Ok(edit) => {
                self.pending = Some(edit);
                self.status.clear();
            }
            Err(_) => {
                self.status = self
                    .focused_entry()
                    .and_then(|entry| entry.availability.reason())
                    .unwrap_or("This setting cannot be changed.")
                    .into();
            }
        }
    }
    fn change_value(&mut self, direction: i32) {
        let Some(entry) = self.focused_entry() else {
            return;
        };
        let change = match (&entry.kind, &entry.value) {
            (SettingKind::Boolean, SettingValue::Boolean(value)) => {
                Change::Set(SettingValue::Boolean(!value))
            }
            (SettingKind::Choice { options }, SettingValue::Choice(value)) => {
                let Some(index) =
                    options.iter().position(|option| &option.value == value)
                else {
                    return;
                };
                let next = if direction < 0 {
                    index
                        .checked_sub(1)
                        .unwrap_or(options.len().saturating_sub(1))
                } else {
                    (index + 1) % options.len()
                };
                let Some(option) = options.get(next) else {
                    return;
                };
                Change::Set(SettingValue::Choice(option.value.clone()))
            }
            (SettingKind::Number { min, max, step }, SettingValue::Number(value)) => {
                let index = ((value - min) / step).round() + f64::from(direction);
                let next = (min + index * step).clamp(*min, *max);
                Change::Set(SettingValue::Number(next))
            }
            (SettingKind::Action, SettingValue::Action) => Change::Activate,
            _ => return,
        };
        self.queue(change);
    }
    fn target_at(&self, x: f32, y: f32) -> Option<Target> {
        if self.layout_dirty || !self.geometry.card.contains(x, y) {
            return None;
        }
        if self.geometry.close.contains(x, y) {
            return Some(Target::Close);
        }
        if self.geometry.reset.contains(x, y) {
            return Some(Target::Reset);
        }
        if self.geometry.search.contains(x, y) {
            return Some(Target::Search);
        }
        if self.geometry.body.contains(x, y) {
            return self
                .rows
                .iter()
                .find(|row| row.control.contains(x, y))
                .map(|row| {
                    let directional = self
                        .catalog
                        .as_ref()
                        .and_then(|catalog| catalog.get(&row.id))
                        .is_some_and(|entry| {
                            matches!(
                                entry.kind,
                                SettingKind::Choice { .. } | SettingKind::Number { .. }
                            )
                        });
                    let direction =
                        if directional && x < row.control.x + row.control.width * 0.5 {
                            -1
                        } else {
                            1
                        };
                    Target::Control(row.id.clone(), direction)
                });
        }
        None
    }
    fn activate_target(&mut self, target: Target) {
        match target {
            Target::Close => self.close(),
            Target::Search => {
                self.focus = Focus::Search;
                self.caret = self.query().len();
                self.anchor = None;
            }
            Target::Reset => {
                self.focus = Focus::Reset;
                self.queue(Change::Reset);
            }
            Target::Control(id, direction) => {
                if self.view.as_mut().is_some_and(|view| view.focus(&id)) {
                    self.focus = Focus::List;
                    self.change_value(direction);
                }
            }
        }
    }
    fn scroll_by(&mut self, delta: f32) {
        if !delta.is_finite() {
            return;
        }
        self.scroll = (self.scroll + delta).clamp(
            0.0,
            (self.content_height - self.geometry.body.height).max(0.0),
        );
        self.layout_dirty = true;
        self.reveal_focus = false;
        self.pressed = None;
    }
    fn prepare(&mut self, text: &mut Text) {
        if !self.layout_dirty && !self.reveal_focus {
            return;
        }
        let viewport = Rect {
            x: 0.0,
            y: 0.0,
            width: self.width,
            height: self.height,
        };
        let margin: f32 = if self.width < 360.0 || self.height < 300.0 {
            4.0
        } else {
            16.0
        };
        let card = Rect {
            x: margin.min(self.width * 0.5),
            y: margin.min(self.height * 0.5),
            width: (self.width - 2.0 * margin).clamp(0.0, 960.0),
            height: (self.height - 2.0 * margin).max(0.0),
        };
        let card = Rect {
            x: (self.width - card.width) * 0.5,
            ..card
        };
        let pad = 8.0_f32.min(card.width * 0.1);
        let line = self.font.max(10.0) * 1.45;
        if self.requires_larger_window() {
            let opts = DrawOpts {
                font_size: self.font.max(10.0),
                ..DrawOpts::default()
            };
            let width = (card.width - 2.0 * pad).max(0.0);
            self.compact_lines = if text.measure("Enlarge window", &opts) <= width {
                let mut lines = vec!["Enlarge window".into()];
                lines.extend(wrapped("to use Settings.", width, text, &opts));
                lines
            } else {
                wrapped("More room", width, text, &opts)
            };
            let mut content_height = self.compact_lines.len() as f32 * line;
            let control_height = line + 4.0;
            let show_close = content_height + control_height + pad * 3.0 <= card.height
                && text.measure("Close", &opts) + 8.0 <= width;
            let close = if show_close {
                Rect {
                    x: card.x + pad,
                    y: card.y + card.height - pad - control_height,
                    width,
                    height: control_height,
                }
            } else {
                Rect::default()
            };
            if !show_close && content_height + line <= card.height {
                self.compact_lines.push("Esc".into());
                content_height += line;
            }
            let available = if show_close {
                (close.y - card.y - pad).max(0.0)
            } else {
                card.height
            };
            let body = Rect {
                x: card.x + pad,
                y: card.y + ((available - content_height) * 0.5).max(0.0),
                width,
                height: content_height.min(available),
            };
            self.geometry = Geometry {
                viewport,
                card,
                body,
                close,
                ..Geometry::default()
            };
            self.rows.clear();
            self.scroll = 0.0;
            self.content_height = 0.0;
            self.caret_rect = Rect::default();
            self.layout_dirty = false;
            self.reveal_focus = false;
            return;
        }
        self.compact_lines.clear();
        let header = line * 2.0 + pad * 3.0;
        let footer = line * 2.0 + pad * 3.0;
        let search = Rect {
            x: card.x + pad,
            y: card.y + header * 0.5,
            width: (card.width - 2.0 * pad).max(0.0),
            height: (header * 0.5 - pad).max(0.0),
        };
        let body = Rect {
            x: card.x + pad,
            y: card.y + header,
            width: (card.width - 2.0 * pad).max(0.0),
            height: (card.height - header - footer).max(0.0),
        };
        let button_h = line + 4.0;
        let button_w = ((body.width - pad) * 0.5).max(0.0);
        let button_y = card.y + card.height - footer + pad.min(footer * 0.1);
        let reset = Rect {
            x: body.x,
            y: button_y,
            width: button_w,
            height: button_h,
        };
        let close = Rect {
            x: body.x + button_w + pad,
            y: button_y,
            width: button_w,
            height: button_h,
        };
        let status = Rect {
            x: body.x,
            y: button_y + button_h + pad.min(footer * 0.1),
            width: body.width,
            height: (card.y + card.height
                - pad
                - button_y
                - button_h
                - pad.min(footer * 0.1))
            .max(0.0),
        };
        self.geometry = Geometry {
            viewport,
            card,
            search,
            body,
            reset,
            close,
            status,
        };
        let opts = DrawOpts {
            font_size: self.font.max(10.0),
            ..DrawOpts::default()
        };
        let mut rows = Vec::new();
        let mut top = 0.0;
        if let Some((view, catalog)) = self.view.as_ref().zip(self.catalog.as_ref()) {
            for id in view.filtered_ids() {
                let Some(entry) = catalog.get(id) else {
                    continue;
                };
                let width = (body.width - 2.0 * pad - 4.0).max(1.0);
                let label_opts = DrawOpts { bold: true, ..opts };
                let mut lines = wrapped(&entry.label, width, text, &label_opts);
                let label_lines = lines.len();
                lines.extend(wrapped(&entry.description, width, text, &opts));
                if let Some(reason) = entry.availability.reason() {
                    lines.extend(wrapped(reason, width, text, &opts));
                }
                lines.extend(wrapped(
                    &format!(
                        "{} | {}",
                        entry.section.label(),
                        origin_label(entry.origin)
                    ),
                    width,
                    text,
                    &opts,
                ));
                let value_lines = wrapped(&display_value(entry), width, text, &opts);
                let control_h = value_lines.len().max(1) as f32 * line + pad;
                let height = lines.len() as f32 * line + control_h + pad * 4.0;
                rows.push(Row {
                    id: id.clone(),
                    bounds: Rect {
                        x: body.x,
                        y: top,
                        width: body.width,
                        height,
                    },
                    control: Rect {
                        x: body.x + pad,
                        y: top + label_lines as f32 * line + pad * 2.0,
                        width,
                        height: control_h,
                    },
                    lines,
                    label_lines,
                    value_lines,
                });
                top += height + pad;
            }
        }
        self.content_height = top;
        self.scroll = self.scroll.clamp(0.0, (top - body.height).max(0.0));
        if self.reveal_focus && self.focus == Focus::List {
            if let Some(row) = rows.iter().find(|row| {
                self.view.as_ref().and_then(ViewState::focused) == Some(&row.id)
            }) {
                let reveal_top =
                    if row.control.y + row.control.height - row.bounds.y <= body.height {
                        row.bounds.y
                    } else {
                        row.control.y
                    };
                if reveal_top < self.scroll {
                    self.scroll = reveal_top;
                }
                if row.control.y + row.control.height > self.scroll + body.height {
                    self.scroll =
                        (row.control.y + row.control.height - body.height).max(0.0);
                }
            }
        }
        for row in &mut rows {
            row.bounds.y += body.y - self.scroll;
            row.control.y += body.y - self.scroll;
        }
        self.rows = rows;
        self.layout_dirty = false;
        self.reveal_focus = false;
        if let Some(view) = &mut self.view {
            view.set_viewport_rows((body.height/(line*5.0)).floor().max(1.0)as usize);
        }
    }
    fn paint(&mut self, canvas: &mut impl Canvas, theme: UiTheme) {
        if !self.is_open() {
            return;
        }
        self.prepare(canvas.text());
        let g = self.geometry;
        rect(canvas, g.viewport, theme.background, g.viewport);
        rect(canvas, g.card, theme.surface, g.viewport);
        let font = self.font.max(10.0);
        let line = font * 1.45;
        let pad = 8.0_f32.min(g.card.width * 0.1);
        if self.requires_larger_window() {
            for (index, value) in self.compact_lines.iter().enumerate() {
                label(
                    canvas,
                    Rect {
                        y: g.body.y + index as f32 * line,
                        height: line,
                        ..g.body
                    },
                    value,
                    font,
                    theme.text,
                    false,
                    g.body,
                );
            }
            if g.close.height > 0.0 {
                control(canvas, g.close, true, theme, g.card);
                label(canvas, g.close, "Close", font, theme.text, false, g.card);
            }
            return;
        }

        let title = Rect {
            x: g.search.x,
            y: g.card.y + pad,
            width: g.search.width,
            height: (g.search.y - g.card.y - pad).max(0.0),
        };
        label(canvas, title, "Settings", font, theme.text, true, g.card);
        control(canvas, g.search, self.focus == Focus::Search, theme, g.card);
        let query = self.query();
        let display = if query.is_empty() && self.preedit.is_empty() {
            "Search settings".into()
        } else {
            format!(
                "{}{}{}",
                &query[..self.caret],
                self.preedit,
                &query[self.caret..]
            )
        };
        let opts = DrawOpts {
            font_size: font,
            color: color_u8(theme.text),
            ..DrawOpts::default()
        };
        let caret_width = canvas.text().measure(&query[..self.caret], &opts);
        let shift = (caret_width - (g.search.width - pad * 3.0)).max(0.0);
        let selection =
            self.anchor
                .filter(|anchor| *anchor != self.caret)
                .map(|anchor| {
                    let start = anchor.min(self.caret);
                    let end = anchor.max(self.caret);
                    let left = canvas.text().measure(&query[..start], &opts);
                    let right = canvas.text().measure(&query[..end], &opts);
                    Rect {
                        x: g.search.x + pad + left - shift,
                        y: g.search.y + 3.0,
                        width: (right - left).max(0.0),
                        height: (g.search.height - 6.0).max(0.0),
                    }
                });
        self.caret_rect = Rect {
            x: g.search.x + pad + caret_width - shift,
            y: g.search.y + 3.0,
            width: 1.0,
            height: (g.search.height - 6.0).max(0.0),
        };
        if let Some(selection) = selection {
            rect(canvas, selection, theme.raised, g.search);
        }

        if let Some(clip) = g.search.intersect(g.card) {
            canvas.text().draw_clipped(
                g.search.x + pad - shift,
                g.search.y + font.min(g.search.height) * 0.15,
                &display,
                &opts,
                clip.array(),
            );
        }
        if self.focus == Focus::Search && self.preedit.is_empty() {
            rect(canvas, self.caret_rect, theme.outline, g.search);
        }
        for row in &self.rows {
            if row.bounds.intersect(g.body).is_none() {
                continue;
            }
            let selected =
                self.view.as_ref().and_then(ViewState::focused) == Some(&row.id);
            rect(
                canvas,
                row.bounds,
                if selected {
                    theme.raised
                } else {
                    theme.background
                },
                g.body,
            );
            for (index, value) in row.lines.iter().enumerate() {
                let bounds = Rect {
                    x: row.bounds.x + pad,
                    y: row.bounds.y
                        + pad
                        + index as f32 * line
                        + if index >= row.label_lines {
                            row.control.height + pad * 2.0
                        } else {
                            0.0
                        },
                    width: (row.bounds.width - 2.0 * pad).max(0.0),
                    height: line,
                };
                label(
                    canvas,
                    bounds,
                    value,
                    font,
                    if index < row.label_lines {
                        theme.text
                    } else {
                        theme.muted_text
                    },
                    index < row.label_lines,
                    g.body,
                );
            }
            control(
                canvas,
                row.control,
                selected && self.focus == Focus::List,
                theme,
                g.body,
            );
            for (index, value) in row.value_lines.iter().enumerate() {
                label(
                    canvas,
                    Rect {
                        y: row.control.y + index as f32 * line,
                        height: line,
                        ..row.control
                    },
                    value,
                    font,
                    theme.text,
                    false,
                    g.body,
                );
            }
        }
        if self.rows.is_empty() {
            label(
                canvas,
                g.body,
                "No settings match this search.",
                font,
                theme.muted_text,
                false,
                g.body,
            );
        }
        control(canvas, g.reset, self.focus == Focus::Reset, theme, g.card);
        label(canvas, g.reset, "Reset", font, theme.text, false, g.card);
        control(canvas, g.close, self.focus == Focus::Close, theme, g.card);
        label(canvas, g.close, "Close", font, theme.text, false, g.card);
        let status = if self.status.is_empty() {
            "Tab: focus | Arrows: navigate | Esc: close"
        } else {
            &self.status
        };
        label(
            canvas,
            g.status,
            status,
            font * 0.85,
            theme.muted_text,
            false,
            g.card,
        );
    }
}
fn origin_label(origin: ValueOrigin) -> &'static str {
    match origin {
        ValueOrigin::Default => "Default",
        ValueOrigin::Configuration => "Configuration",
        ValueOrigin::User => "User override",
        ValueOrigin::Extension => "Extension",
    }
}
fn display_value(entry: &SettingDescriptor) -> String {
    match &entry.value {
        SettingValue::Boolean(value) => if *value { "On" } else { "Off" }.into(),
        SettingValue::Choice(value) => {
            if let SettingKind::Choice { options } = &entry.kind {
                options
                    .iter()
                    .find(|option| &option.value == value)
                    .map_or_else(
                        || value.clone(),
                        |option| format!("< {} >", option.label),
                    )
            } else {
                value.clone()
            }
        }
        SettingValue::Number(value) => format!("-   {value}   +"),
        SettingValue::Action => "Open".into(),
    }
}
fn wrapped(value: &str, width: f32, text: &mut Text, opts: &DrawOpts) -> Vec<String> {
    let mut result = Vec::new();
    let mut line = String::new();
    for word in value.split_inclusive(char::is_whitespace) {
        let mut next = line.clone();
        next.push_str(word);
        if text.measure(&next, opts) <= width {
            line = next;
            continue;
        }
        if !line.trim_end().is_empty() {
            result.push(line.trim_end().to_owned());
        }
        line.clear();
        let word = word.trim_start();
        if text.measure(word, opts) <= width {
            line = word.into();
            continue;
        }
        for grapheme in word.graphemes(true) {
            let mut next = line.clone();
            next.push_str(grapheme);
            if !line.is_empty() && text.measure(&next, opts) > width {
                result.push(line);
                line = grapheme.into();
            } else {
                line = next;
            }
        }
    }
    if !line.trim_end().is_empty() {
        result.push(line.trim_end().to_owned());
    }
    result
}

fn rect(canvas: &mut impl Canvas, bounds: Rect, color: [f32; 4], clip: Rect) {
    if let Some(bounds) = bounds.intersect(clip) {
        canvas.rect(bounds.array(), color);
    }
}
fn control(
    canvas: &mut impl Canvas,
    bounds: Rect,
    focused: bool,
    theme: UiTheme,
    clip: Rect,
) {
    rect(
        canvas,
        bounds,
        if focused { theme.outline } else { theme.raised },
        clip,
    );
    rect(
        canvas,
        Rect {
            x: bounds.x + 2.0,
            y: bounds.y + 2.0,
            width: (bounds.width - 4.0).max(0.0),
            height: (bounds.height - 4.0).max(0.0),
        },
        theme.background,
        clip,
    );
}
fn label(
    canvas: &mut impl Canvas,
    bounds: Rect,
    value: &str,
    font: f32,
    color: [f32; 4],
    bold: bool,
    clip: Rect,
) {
    if let Some(clip) = bounds.intersect(clip) {
        let opts = DrawOpts {
            font_size: font,
            color: color_u8(color),
            bold,
            ..DrawOpts::default()
        };
        canvas.text().draw_clipped(
            bounds.x + 4.0,
            bounds.y + 2.0,
            value,
            &opts,
            clip.array(),
        );
    }
}

#[cfg(test)]
#[path = "settings_view_tests.rs"]
mod tests;
