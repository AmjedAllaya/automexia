//! Renderer-owned, redacted compatibility inspector.
//!
//! The snapshot intentionally excludes environment values, clipboard data,
//! command history, paths, and terminal output. It is modal while visible and
//! never takes PTY, filesystem, process, or configuration authority.

use crate::renderer::responsive::{elide_end, Viewport};
use crate::renderer::ui_theme::{
    color_u8, UiTheme, BRAND_BLUE, BRAND_CYAN, BRAND_LIME, MODAL_SCRIM, MODAL_SHADOW,
};
use rio_backend::config::colors::Colors;
use rio_backend::sugarloaf::text::DrawOpts;
use rio_backend::sugarloaf::Sugarloaf;
use serde::Serialize;

const IDEAL_WIDTH: f32 = 560.0;
const IDEAL_HEIGHT: f32 = 430.0;
const MARGIN: f32 = 18.0;
const PADDING: f32 = 22.0;
const HEADER_HEIGHT: f32 = 66.0;
const FOOTER_HEIGHT: f32 = 42.0;
const LINE_HEIGHT: f32 = 20.0;
const CLOSE_SIZE: f32 = 30.0;
const MAX_DIAGNOSTICS: usize = 8;
const MAX_LINES: usize = 18;
const ORDER: u8 = 28;

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
    close: Rect,
    compact: bool,
    tiny: bool,
    visible_lines: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CompatibilityInspectorAction {
    Close,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
pub struct InspectorSnapshot {
    pub route_id: usize,
    pub session_id: u64,
    pub columns: usize,
    pub rows: usize,
    pub viewport_offset: usize,
    pub history_lines: usize,
    pub terminal_modes: String,
    pub keyboard_modes: String,
    pub profile: String,
    pub binding_origin: String,
    pub last_binding: String,
    pub pending_bytes: usize,
    pub active_table: String,
    pub parser_diagnostics: Vec<String>,
}

impl InspectorSnapshot {
    fn lines(&self) -> Vec<String> {
        let mut lines = vec![
            format!("Grid          {} × {}", self.columns, self.rows),
            format!(
                "Viewport      offset {} · history {} lines",
                self.viewport_offset, self.history_lines
            ),
            format!("Terminal      {}", self.terminal_modes),
            format!("Keyboard      {}", self.keyboard_modes),
            format!("Profile       {}", self.profile),
            format!(
                "Binding       {} · {}",
                self.binding_origin, self.last_binding
            ),
            format!(
                "Sequence      {} buffered bytes · table {}",
                self.pending_bytes, self.active_table
            ),
            format!("PTY identity  session-{}", self.session_id),
            format!("Route         {}", self.route_id),
        ];
        lines.push("Diagnostics   recent redacted codes".into());
        if self.parser_diagnostics.is_empty() {
            lines.push("  ✓ none".into());
        } else {
            lines.extend(
                self.parser_diagnostics
                    .iter()
                    .take(MAX_DIAGNOSTICS)
                    .map(|diagnostic| format!("  • {diagnostic}")),
            );
        }
        lines.truncate(MAX_LINES);
        lines
    }
}

#[derive(Default)]
pub struct CompatibilityInspector {
    active: bool,
    snapshot: InspectorSnapshot,
    hovered_action: Option<CompatibilityInspectorAction>,
}

impl CompatibilityInspector {
    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn set_visibility(&mut self, value: &str) -> bool {
        let next = match value {
            "show" | "on" => true,
            "hide" | "off" => false,
            _ => !self.active,
        };
        let changed = self.active != next;
        self.active = next;
        if !next {
            self.hovered_action = None;
        }
        changed
    }

    pub fn replace_snapshot(&mut self, snapshot: InspectorSnapshot) {
        self.snapshot = snapshot;
    }

    fn layout(&self, dimensions: (f32, f32, f32)) -> Layout {
        let viewport = Viewport::from_physical(dimensions.0, dimensions.1, dimensions.2);
        let margin = if viewport.width < 260.0 || viewport.height < 180.0 {
            6.0
        } else {
            MARGIN
        };
        let width = IDEAL_WIDTH.min((viewport.width - margin * 2.0).max(1.0));
        let height = IDEAL_HEIGHT.min((viewport.height - margin * 2.0).max(1.0));
        let card = Rect {
            x: ((viewport.width - width) * 0.5).max(0.0),
            y: ((viewport.height - height) * 0.5).max(0.0),
            width,
            height,
        };
        let compact = width < 380.0 || height < 280.0;
        let tiny = width < 220.0 || height < 125.0;
        let inset = if tiny {
            6.0
        } else if compact {
            12.0
        } else {
            PADDING
        };
        let close_size = CLOSE_SIZE.min((height - inset).max(24.0));
        let close = Rect {
            x: (card.x + width - inset - close_size).max(card.x),
            y: card.y + inset,
            width: close_size,
            height: close_size,
        };
        let body_top = card.y + if compact { 52.0 } else { HEADER_HEIGHT };
        let footer = if compact { 8.0 } else { FOOTER_HEIGHT };
        let available = (card.y + height - inset - footer - body_top).max(0.0);
        let visible_lines = self
            .snapshot
            .lines()
            .len()
            .min(MAX_LINES)
            .min((available / LINE_HEIGHT).floor() as usize);
        Layout {
            card,
            close,
            compact,
            tiny,
            visible_lines,
        }
    }

    pub fn hit_test(
        &self,
        mouse_x: f32,
        mouse_y: f32,
        dimensions: (f32, f32, f32),
    ) -> Result<Option<CompatibilityInspectorAction>, ()> {
        if !self.active {
            return Err(());
        }
        let layout = self.layout(dimensions);
        if !layout.card.contains(mouse_x, mouse_y) {
            Err(())
        } else if layout.close.contains(mouse_x, mouse_y) {
            Ok(Some(CompatibilityInspectorAction::Close))
        } else {
            Ok(None)
        }
    }

    pub fn hover(
        &mut self,
        mouse_x: f32,
        mouse_y: f32,
        dimensions: (f32, f32, f32),
    ) -> bool {
        let next = self
            .active
            .then(|| self.layout(dimensions))
            .and_then(|layout| {
                layout
                    .close
                    .contains(mouse_x, mouse_y)
                    .then_some(CompatibilityInspectorAction::Close)
            });
        if next == self.hovered_action {
            false
        } else {
            self.hovered_action = next;
            true
        }
    }

    pub fn hovered_action(&self) -> Option<CompatibilityInspectorAction> {
        self.hovered_action
    }

    pub fn render(
        &mut self,
        sugarloaf: &mut Sugarloaf,
        dimensions: (f32, f32, f32),
        colors: &Colors,
    ) {
        if !self.active {
            return;
        }
        let viewport = Viewport::from_physical(dimensions.0, dimensions.1, dimensions.2);
        let layout = self.layout(dimensions);
        let card = layout.card;
        let theme =
            UiTheme::resolve(colors.background.0, colors.foreground, colors.black);

        sugarloaf.begin_modal_layer();
        sugarloaf.rect(
            None,
            0.0,
            0.0,
            viewport.width,
            viewport.height,
            MODAL_SCRIM,
            0.0,
            ORDER,
        );
        rounded(
            sugarloaf,
            card.x + 7.0,
            card.y + 9.0,
            card.width,
            card.height,
            MODAL_SHADOW,
            14.0,
        );
        rounded(
            sugarloaf,
            card.x,
            card.y,
            card.width,
            card.height,
            BRAND_BLUE,
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

        let inset = if layout.tiny {
            6.0
        } else if layout.compact {
            12.0
        } else {
            PADDING
        };
        if !layout.tiny {
            rounded(
                sugarloaf,
                card.x + inset,
                card.y + inset,
                74.0,
                24.0,
                BRAND_BLUE,
                7.0,
            );
            let badge = DrawOpts {
                font_size: 10.0,
                color: [3, 12, 21, 255],
                bold: true,
                ..DrawOpts::default()
            };
            sugarloaf.text_mut().draw(
                card.x + inset + 10.0,
                card.y + inset + 5.0,
                "REDACTED",
                &badge,
            );
            if !layout.compact {
                let title = DrawOpts {
                    font_size: 17.0,
                    color: color_u8(theme.text),
                    bold: true,
                    ..DrawOpts::default()
                };
                sugarloaf.text_mut().draw(
                    card.x + inset + 88.0,
                    card.y + inset + 3.0,
                    "Compatibility inspector",
                    &title,
                );
            }

            let body = DrawOpts {
                font_size: 12.5,
                color: color_u8(theme.text),
                ..DrawOpts::default()
            };
            let body_x = card.x + inset;
            let body_y = card.y + if layout.compact { 52.0 } else { HEADER_HEIGHT };
            let available_width = (card.width - inset * 2.0).max(1.0);
            for (index, line) in self
                .snapshot
                .lines()
                .into_iter()
                .take(layout.visible_lines)
                .enumerate()
            {
                let color = if line == "  ✓ none" {
                    color_u8(BRAND_LIME)
                } else {
                    color_u8(theme.text)
                };
                let body = DrawOpts { color, ..body };
                let line = elide_end(sugarloaf, &line, available_width, body.font_size);
                sugarloaf.text_mut().draw(
                    body_x,
                    body_y + index as f32 * LINE_HEIGHT,
                    &line,
                    &body,
                );
            }
            if !layout.compact {
                let footer = DrawOpts {
                    font_size: 11.0,
                    color: color_u8(theme.muted_text),
                    ..DrawOpts::default()
                };
                sugarloaf.text_mut().draw(
                    card.x + inset,
                    card.y + card.height - inset - footer.font_size,
                    "Esc close  ·  sensitive values never collected",
                    &footer,
                );
            }
        }

        let close_fill =
            if self.hovered_action == Some(CompatibilityInspectorAction::Close) {
                theme.raised
            } else {
                theme.surface
            };
        rounded(
            sugarloaf,
            layout.close.x,
            layout.close.y,
            layout.close.width,
            layout.close.height,
            close_fill,
            8.0,
        );
        let close = DrawOpts {
            font_size: 16.0,
            color: color_u8(BRAND_CYAN),
            bold: true,
            ..DrawOpts::default()
        };
        sugarloaf.text_mut().draw(
            layout.close.x + 9.0,
            layout.close.y + 5.0,
            "×",
            &close,
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

#[cfg(test)]
mod tests {
    use super::*;

    fn active_inspector() -> CompatibilityInspector {
        let mut inspector = CompatibilityInspector::default();
        assert!(inspector.set_visibility("show"));
        inspector
    }

    #[test]
    fn snapshot_schema_contains_only_explicit_redacted_fields() {
        let value = serde_json::to_value(InspectorSnapshot::default()).unwrap();
        let keys = value
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        for forbidden in [
            "environment",
            "clipboard",
            "output",
            "cwd",
            "path",
            "command",
        ] {
            assert!(!keys.iter().any(|key| key.contains(forbidden)));
        }
    }

    #[test]
    fn geometry_is_dpi_invariant_and_bounded() {
        let inspector = active_inspector();
        let a = inspector.layout((640.0, 480.0, 1.0));
        let b = inspector.layout((1_280.0, 960.0, 2.0));
        assert_eq!(a.card, b.card);
        assert_eq!(a.close, b.close);
        for dimensions in [(300.0, 200.0, 1.0), (180.0, 90.0, 1.0)] {
            let layout = inspector.layout(dimensions);
            let viewport =
                Viewport::from_physical(dimensions.0, dimensions.1, dimensions.2);
            assert!(layout.card.x >= 0.0 && layout.card.y >= 0.0);
            assert!(layout.card.x + layout.card.width <= viewport.width);
            assert!(layout.card.y + layout.card.height <= viewport.height);
        }
    }

    #[test]
    fn close_control_meets_target_and_hit_test_contract() {
        let inspector = active_inspector();
        let dimensions = (800.0, 600.0, 1.0);
        let layout = inspector.layout(dimensions);
        assert!(layout.close.width >= 24.0 && layout.close.height >= 24.0);
        assert_eq!(
            inspector.hit_test(layout.close.x + 1.0, layout.close.y + 1.0, dimensions),
            Ok(Some(CompatibilityInspectorAction::Close))
        );
        assert_eq!(
            inspector.hit_test(layout.card.x + 2.0, layout.card.y + 2.0, dimensions),
            Ok(None)
        );
        assert_eq!(inspector.hit_test(0.0, 0.0, dimensions), Err(()));
    }

    #[test]
    fn hiding_clears_hover_state() {
        let mut inspector = active_inspector();
        let dimensions = (800.0, 600.0, 1.0);
        let close = inspector.layout(dimensions).close;
        assert!(inspector.hover(close.x + 1.0, close.y + 1.0, dimensions));
        assert_eq!(
            inspector.hovered_action(),
            Some(CompatibilityInspectorAction::Close)
        );
        assert!(inspector.set_visibility("hide"));
        assert_eq!(inspector.hovered_action(), None);
    }
}
