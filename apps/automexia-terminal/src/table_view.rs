//! Explicit, route-owned, read-only terminal table presentation.
use crate::automexia::table_output::cell_width;
use crate::renderer::ui_theme::{self, color_u8, UiTheme};
use automexia_ui_model::tables::{Table, TableError, TableViewport};
use rio_backend::sugarloaf::{text::DrawOpts, Sugarloaf};
use rio_window::{
    event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent},
    keyboard::{Key, ModifiersState, NamedKey},
};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Rect {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}
impl Rect {
    fn hit(self, x: f32, y: f32) -> bool {
        x >= self.x && y >= self.y && x < self.x + self.w && y < self.y + self.h
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct Layout {
    width: f32,
    height: f32,
    body: Rect,
    track: Rect,
    back: Rect,
    copy: Rect,
    cell: f32,
    line: f32,
    columns: usize,
    rows: usize,
    font: f32,
}
impl Layout {
    fn new(width: f32, height: f32, font: f32, cell: f32) -> Self {
        let finite = |value: f32| {
            if value.is_finite() {
                value.max(0.0)
            } else {
                0.0
            }
        };
        let (width, height) = (finite(width), finite(height));
        let font = finite(font).clamp(10.0, 24.0);
        let cell = finite(cell).max(1.0);
        let line = (font * 1.55).ceil();
        let padding = 16.0_f32.min(width / 4.0);
        let toolbar = 56.0_f32.min(height);
        let footer = 48.0_f32.min((height - toolbar).max(0.0));
        let body = Rect {
            x: padding,
            y: toolbar,
            w: (width - padding * 2.0).max(0.0),
            h: (height - toolbar - footer).max(0.0),
        };
        let track = Rect {
            x: body.x,
            y: body.y + body.h + 6.0_f32.min(footer),
            w: body.w,
            h: 8.0_f32.min((footer - 6.0).max(0.0)),
        };
        let back = Rect {
            x: padding,
            y: 10.0_f32.min(height),
            w: 100.0_f32.min(body.w),
            h: 40.0_f32.min((height - 10.0).max(0.0)),
        };
        let copy = if width >= 240.0 {
            Rect {
                x: width - padding - 100.0,
                ..back
            }
        } else {
            Rect::default()
        };
        Self {
            width,
            height,
            body,
            track,
            back,
            copy,
            cell,
            line,
            columns: (body.w / cell) as usize,
            rows: (body.h / line) as usize,
            font,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Effect {
    Pass,
    Consumed,
    Copy,
    Close,
}

#[derive(Default)]
pub(crate) struct TableView {
    content: Option<(usize, Result<Table, TableError>)>,
    viewport: TableViewport,
    layout: Layout,
    pointer: (f32, f32),
    wheel: [f32; 2],
    dragging: bool,
    pressed: Option<bool>,
    copy_focused: bool,
}

impl TableView {
    pub(crate) fn open(&mut self, route: usize, table: Result<Table, TableError>) {
        *self = Self {
            content: Some((route, table)),
            ..Self::default()
        };
    }
    pub(crate) fn is_open(&self) -> bool {
        self.content.is_some()
    }
    fn can_copy(&self) -> bool {
        self.content
            .as_ref()
            .is_some_and(|(_, result)| result.is_ok())
    }
    pub(crate) fn close(&mut self) {
        *self = Self::default();
    }
    pub(crate) fn retain_route(&mut self, route: usize) {
        if self
            .content
            .as_ref()
            .is_some_and(|(owner, _)| *owner != route)
        {
            self.close();
        }
    }
    pub(crate) fn copy_text(&self) -> Option<String> {
        let table = self.content.as_ref()?.1.as_ref().ok()?;
        Some(table.source().join("\n"))
    }
    pub(crate) fn fit(&mut self, width: f32, height: f32, font: f32, cell: f32) {
        self.layout = Layout::new(width, height, font, cell);
        if let Some((_, Ok(table))) = &self.content {
            self.viewport.fit(
                table.width(),
                table.source().len(),
                self.layout.columns,
                self.layout.rows,
            );
        }
    }
    pub(crate) fn key(
        &mut self,
        key: &Key,
        mods: ModifiersState,
        pressed: bool,
    ) -> Effect {
        if !self.is_open() {
            return Effect::Pass;
        }
        if !pressed {
            return Effect::Consumed;
        }
        let control = mods.control_key() || mods.super_key();
        match key {
            Key::Named(NamedKey::Escape) => return Effect::Close,
            Key::Character(c)
                if control && c.eq_ignore_ascii_case("c") && self.can_copy() =>
            {
                return Effect::Copy
            }
            Key::Named(NamedKey::Tab) if self.can_copy() => {
                self.copy_focused = !self.copy_focused
            }
            Key::Named(NamedKey::Enter | NamedKey::Space) => {
                return if self.copy_focused {
                    Effect::Copy
                } else {
                    Effect::Close
                }
            }
            Key::Named(NamedKey::ArrowLeft) => {
                self.viewport.scroll(-1, 0);
            }
            Key::Named(NamedKey::ArrowRight) => {
                self.viewport.scroll(1, 0);
            }
            Key::Named(NamedKey::ArrowUp) => {
                self.viewport.scroll(0, -1);
            }
            Key::Named(NamedKey::ArrowDown) => {
                self.viewport.scroll(0, 1);
            }
            Key::Named(NamedKey::PageUp) => {
                self.viewport.scroll(0, -(self.layout.rows.max(1) as isize));
            }
            Key::Named(NamedKey::PageDown) => {
                self.viewport.scroll(0, self.layout.rows.max(1) as isize);
            }
            Key::Named(NamedKey::Home) => {
                self.viewport
                    .scroll(isize::MIN, if control { isize::MIN } else { 0 });
            }
            Key::Named(NamedKey::End) => {
                self.viewport
                    .scroll(isize::MAX, if control { isize::MAX } else { 0 });
            }
            _ => (), // Every other key stays local; no implicit shell input or paste.
        }
        Effect::Consumed
    }
    fn pan_track(&mut self) {
        let Some((_, thumb)) = self.viewport.horizontal_thumb(self.layout.track.w) else {
            return;
        };
        let travel = self.layout.track.w - thumb;
        if travel <= 0.0 {
            return;
        }
        let fraction = ((self.pointer.0 - self.layout.track.x - thumb / 2.0) / travel)
            .clamp(0.0, 1.0);
        let Some((_, Ok(table))) = &self.content else {
            return;
        };
        let target = (fraction * table.width().saturating_sub(self.layout.columns) as f32)
            .round() as isize;
        self.viewport
            .scroll(target - self.viewport.column() as isize, 0);
    }
    fn scroll(&mut self, x: f32, y: f32) {
        for (index, delta) in [x, y].into_iter().enumerate() {
            if delta.is_finite() {
                self.wheel[index] += delta.clamp(-64.0, 64.0);
            }
        }
        let steps = [
            self.wheel[0].trunc() as isize,
            self.wheel[1].trunc() as isize,
        ];
        self.wheel[0] -= steps[0] as f32;
        self.wheel[1] -= steps[1] as f32;
        self.viewport.scroll(steps[0], steps[1]);
    }
    pub(crate) fn event(
        &mut self,
        event: &WindowEvent,
        mods: ModifiersState,
        scale: f32,
    ) -> Effect {
        if !self.is_open() {
            return Effect::Pass;
        }
        let scale = if scale.is_finite() {
            scale.max(0.1)
        } else {
            1.0
        };
        match event {
            WindowEvent::KeyboardInput { event, .. } => self.key(
                &event.logical_key,
                mods,
                event.state == ElementState::Pressed,
            ),
            WindowEvent::CursorMoved { position, .. } => {
                self.pointer = (position.x as f32 / scale, position.y as f32 / scale);
                if self.dragging {
                    self.pan_track();
                }
                Effect::Consumed
            }
            WindowEvent::MouseInput {
                state,
                button: MouseButton::Left,
                ..
            } => {
                let (x, y) = self.pointer;
                if *state == ElementState::Pressed {
                    self.dragging = self.layout.track.hit(x, y);
                    self.pressed = if self.layout.back.hit(x, y) {
                        Some(false)
                    } else if self.can_copy() && self.layout.copy.hit(x, y) {
                        Some(true)
                    } else {
                        None
                    };
                    if self.dragging {
                        self.pan_track();
                    }
                } else {
                    self.dragging = false;
                    match self.pressed.take() {
                        Some(false) if self.layout.back.hit(x, y) => {
                            return Effect::Close
                        }
                        Some(true) if self.layout.copy.hit(x, y) => return Effect::Copy,
                        _ => (),
                    }
                }
                Effect::Consumed
            }
            WindowEvent::MouseWheel { delta, phase, .. } => {
                if matches!(
                    phase,
                    rio_window::event::TouchPhase::Started
                        | rio_window::event::TouchPhase::Cancelled
                ) {
                    self.wheel = [0.0; 2];
                }
                if *phase != rio_window::event::TouchPhase::Cancelled {
                    let (x, y) = match delta {
                        MouseScrollDelta::LineDelta(x, y) => (-*x * 3.0, -*y * 3.0),
                        MouseScrollDelta::PixelDelta(p) => (
                            -p.x as f32 / scale / self.layout.cell,
                            -p.y as f32 / scale / self.layout.line,
                        ),
                    };
                    if mods.shift_key() {
                        self.scroll(x + y, 0.0);
                    } else {
                        self.scroll(x, y);
                    }
                }
                Effect::Consumed
            }
            WindowEvent::Focused(false) | WindowEvent::CursorLeft { .. } => {
                self.dragging = false;
                self.pressed = None;
                self.wheel = [0.0; 2];
                Effect::Pass
            }
            WindowEvent::MouseInput { .. }
            | WindowEvent::Ime(_)
            | WindowEvent::Touch(_)
            | WindowEvent::DroppedFile(_)
            | WindowEvent::HoveredFile(_)
            | WindowEvent::HoveredFileCancelled => Effect::Consumed,
            _ => Effect::Pass,
        }
    }

    pub(crate) fn draw(&self, sugarloaf: &mut impl TableCanvas, theme: UiTheme) {
        if !self.is_open() {
            return;
        }
        let l = self.layout;
        sugarloaf.begin_modal_layer();
        let paint = |s: &mut _, r: Rect, color| {
            if r.w > 0.0 && r.h > 0.0 {
                TableCanvas::paint_rect(s, r, color);
            }
        };
        paint(
            sugarloaf,
            Rect {
                w: l.width,
                h: l.height,
                ..Rect::default()
            },
            theme.background,
        );
        for (rect, label, focused) in [
            (l.back, "← Back", !self.copy_focused),
            (
                if self.can_copy() {
                    l.copy
                } else {
                    Rect::default()
                },
                "Copy all",
                self.copy_focused,
            ),
        ] {
            paint(
                sugarloaf,
                rect,
                if focused { theme.raised } else { theme.surface },
            );
            if focused && rect.h >= 2.0 && rect.w > 0.0 {
                paint(
                    sugarloaf,
                    Rect {
                        y: rect.y + rect.h - 2.0,
                        h: 2.0,
                        ..rect
                    },
                    theme.outline,
                );
            }
            draw_label(sugarloaf, rect, label, l.font.min(14.0), theme.text);
        }
        if l.width >= 450.0 {
            draw_label(
                sugarloaf,
                Rect {
                    x: 132.0,
                    y: 10.0,
                    w: l.width - 260.0,
                    h: 32.0,
                },
                "Table · read-only snapshot",
                14.0,
                theme.text,
            );
        }
        if let Some((_, Ok(table))) = &self.content {
            let col = self.viewport.column();
            let right = col.saturating_add(l.columns);
            let mut opts = DrawOpts {
                font_size: l.font,
                color: color_u8(theme.text),
                ..DrawOpts::default()
            };
            for (visible, row) in (self.viewport.row()..table.source().len())
                .take(l.rows)
                .enumerate()
            {
                let y = l.body.y + visible as f32 * l.line;
                for (i, start) in table.column_starts().iter().copied().enumerate() {
                    let end = table
                        .column_starts()
                        .get(i + 1)
                        .copied()
                        .unwrap_or(table.width())
                        .min(right);
                    let start = start.max(col);
                    if start >= end {
                        continue;
                    }
                    if let Some(range) =
                        table.visible_range(row, start, end - start, cell_width)
                    {
                        let text = &table.source()[row][range.bytes];
                        let available =
                            (end - start - range.leading_cells) as f32 * l.cell;
                        let measured = sugarloaf.text_mut().measure(text, &opts);
                        opts.font_size =
                            l.font * (available / measured.max(1.0)).min(1.0);
                        sugarloaf.text_mut().draw(
                            l.body.x
                                + (start - col + range.leading_cells) as f32 * l.cell,
                            y + (l.line - l.font) / 2.0,
                            text,
                            &opts,
                        );
                        opts.font_size = l.font;
                    }
                }
                paint(
                    sugarloaf,
                    Rect {
                        x: l.body.x,
                        y: y + l.line - 1.0,
                        w: l.body.w,
                        h: 1.0,
                    },
                    ui_theme::over(
                        theme.background,
                        [theme.outline[0], theme.outline[1], theme.outline[2], 0.16],
                    ),
                );
            }
            for separator in table
                .separators()
                .iter()
                .copied()
                .filter(|s| *s >= col && *s < right)
            {
                paint(
                    sugarloaf,
                    Rect {
                        x: l.body.x + (separator - col) as f32 * l.cell,
                        y: l.body.y,
                        w: 1.0,
                        h: (table
                            .source()
                            .len()
                            .saturating_sub(self.viewport.row())
                            .min(l.rows) as f32
                            * l.line),
                    },
                    ui_theme::over(
                        theme.background,
                        [theme.outline[0], theme.outline[1], theme.outline[2], 0.2],
                    ),
                );
            }
            paint(sugarloaf, l.track, theme.surface);
            if let Some((start, width)) = self.viewport.horizontal_thumb(l.track.w) {
                paint(
                    sugarloaf,
                    Rect {
                        x: l.track.x + start,
                        w: width,
                        ..l.track
                    },
                    theme.outline,
                );
            }
            if l.width >= 620.0 && l.height >= 140.0 {
                draw_label(
                    sugarloaf,
                    Rect {
                        x: l.body.x,
                        y: l.height - 27.0,
                        w: l.body.w,
                        h: 24.0,
                    },
                    "← → pan · ↑ ↓ rows · Shift+wheel pan · Ctrl+C copy · Esc back",
                    12.0,
                    theme.muted_text,
                );
            }
        } else {
            let message = match self.content.as_ref().map(|(_, result)| result) {
                Some(Err(TableError::Capacity)) => {
                    "Table exceeds view limits. Select a smaller table."
                }
                Some(Err(TableError::InvalidText)) => {
                    "Unsupported control text. Return to terminal to inspect."
                }
                _ => "No aligned table found. Select table rows, then open again.",
            };
            draw_message(sugarloaf, l.body, message, theme.muted_text);
        }
        sugarloaf.end_modal_layer();
    }
}

fn draw_label(
    sugarloaf: &mut impl TableCanvas,
    rect: Rect,
    label: &str,
    font: f32,
    color: [f32; 4],
) {
    let opts = DrawOpts {
        font_size: font,
        color: color_u8(color),
        ..DrawOpts::default()
    };
    // Tiny viewports never put partial controls or text over the terminal.
    if rect.h >= font + 4.0 && sugarloaf.text_mut().measure(label, &opts) + 12.0 <= rect.w
    {
        sugarloaf
            .text_mut()
            .draw(rect.x + 6.0, rect.y + 5.0, label, &opts);
    }
}

fn draw_message(
    surface: &mut impl TableCanvas,
    rect: Rect,
    message: &str,
    color: [f32; 4],
) {
    use crate::automexia::ui::command_info::{pack, Label};
    let mut opts = DrawOpts {
        font_size: 14.0,
        color: color_u8(color),
        ..DrawOpts::default()
    };
    let labels = [Label {
        text: message,
        leading: 0.0,
        padding: 0.0,
        align_end: false,
    }];
    if let Some(band) = pack(&labels, rect.w, 0.0, |_, text| {
        surface.text_mut().measure(text, &opts)
    }) {
        for fragment in band
            .fragments
            .iter()
            .filter(|f| (f.row + 1) as f32 * 22.0 <= rect.h)
        {
            opts.font_size = 14.0 * fragment.text_scale;
            surface.text_mut().draw(
                rect.x + fragment.x,
                rect.y + fragment.row as f32 * 22.0,
                &message[fragment.bytes.clone()],
                &opts,
            );
        }
    }
}

#[cfg(test)]
#[path = "table_view_tests.rs"]
mod tests;

/// Same text shaper and drawing stream for live rendering and controlled CPU
/// evidence. The adapter owns graphics only, never terminal data or authority.
pub(crate) trait TableCanvas {
    fn text_mut(&mut self) -> &mut rio_backend::sugarloaf::text::Text;
    fn begin_modal_layer(&mut self);
    fn end_modal_layer(&mut self);
    fn paint_rect(&mut self, rect: Rect, color: [f32; 4]);
}

impl TableCanvas for Sugarloaf<'_> {
    fn text_mut(&mut self) -> &mut rio_backend::sugarloaf::text::Text {
        Sugarloaf::text_mut(self)
    }
    fn begin_modal_layer(&mut self) {
        Sugarloaf::begin_modal_layer(self);
    }
    fn end_modal_layer(&mut self) {
        Sugarloaf::end_modal_layer(self);
    }
    fn paint_rect(&mut self, r: Rect, color: [f32; 4]) {
        self.rect(None, r.x, r.y, r.w, r.h, color, 0.0, 48);
    }
}
