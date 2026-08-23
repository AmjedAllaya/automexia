//! Renderer-owned, redacted compatibility inspector.
//!
//! The snapshot intentionally excludes environment values, clipboard data,
//! command history, paths, and terminal output. It is passive and never takes
//! PTY, filesystem, process, or configuration authority.

use crate::renderer::responsive::{elide_end, Viewport};
use rio_backend::sugarloaf::text::DrawOpts;
use rio_backend::sugarloaf::Sugarloaf;
use serde::Serialize;

const WIDTH: f32 = 520.0;
const MARGIN: f32 = 12.0;
const PADDING: f32 = 18.0;
const HEADER_HEIGHT: f32 = 34.0;
const LINE_HEIGHT: f32 = 20.0;
const MAX_DIAGNOSTICS: usize = 8;
const MAX_LINES: usize = 18;
const ORDER: u8 = 28;

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
        changed
    }

    pub fn replace_snapshot(&mut self, snapshot: InspectorSnapshot) {
        self.snapshot = snapshot;
    }

    fn panel_rect(
        &self,
        width: f32,
        height: f32,
        scale: f32,
    ) -> (f32, f32, f32, f32, usize) {
        let viewport = Viewport::from_physical(width, height, scale);
        let panel_width = viewport.fitted_surface(WIDTH, MARGIN);
        let available = (viewport.height - MARGIN * 2.0 - PADDING * 2.0 - HEADER_HEIGHT)
            .max(LINE_HEIGHT);
        let visible = self
            .snapshot
            .lines()
            .len()
            .min(MAX_LINES)
            .min((available / LINE_HEIGHT).floor().max(1.0) as usize);
        let panel_height = PADDING * 2.0 + HEADER_HEIGHT + visible as f32 * LINE_HEIGHT;
        (
            (viewport.width - panel_width - MARGIN).max(0.0),
            MARGIN.min((viewport.height - panel_height).max(0.0)),
            panel_width,
            panel_height.min(viewport.height),
            visible,
        )
    }

    pub fn render(&self, sugarloaf: &mut Sugarloaf, dimensions: (f32, f32, f32)) {
        if !self.active {
            return;
        }
        let (width, height, scale) = dimensions;
        let (x, y, panel_width, panel_height, visible) =
            self.panel_rect(width, height, scale);
        sugarloaf.begin_modal_layer();
        sugarloaf.rect(
            None,
            0.0,
            0.0,
            width / scale,
            height / scale,
            [0.01, 0.03, 0.05, 0.48],
            0.0,
            ORDER,
        );
        sugarloaf.rounded_rect(
            None,
            x,
            y,
            panel_width,
            panel_height,
            [0.035, 0.075, 0.11, 0.99],
            0.1,
            12.0,
            ORDER,
        );
        let heading = DrawOpts {
            font_size: 17.0,
            color: [71, 211, 255, 255],
            ..DrawOpts::default()
        };
        sugarloaf.text_mut().draw(
            x + PADDING,
            y + PADDING,
            "◈ Compatibility inspector",
            &heading,
        );
        let body = DrawOpts {
            font_size: 12.5,
            color: [218, 231, 240, 255],
            ..DrawOpts::default()
        };
        let available_width = (panel_width - PADDING * 2.0).max(1.0);
        for (index, line) in self.snapshot.lines().into_iter().take(visible).enumerate() {
            let line = elide_end(sugarloaf, &line, available_width, body.font_size);
            sugarloaf.text_mut().draw(
                x + PADDING,
                y + PADDING + HEADER_HEIGHT + index as f32 * LINE_HEIGHT,
                &line,
                &body,
            );
        }
        sugarloaf.end_modal_layer();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let inspector = CompatibilityInspector::default();
        assert_eq!(
            inspector.panel_rect(640.0, 480.0, 1.0),
            inspector.panel_rect(1_280.0, 960.0, 2.0)
        );
        let (x, y, width, height, _) = inspector.panel_rect(300.0, 200.0, 1.0);
        assert!(x >= 0.0 && y >= 0.0);
        assert!(x + width <= 300.0);
        assert!(y + height <= 200.0);
    }
}
