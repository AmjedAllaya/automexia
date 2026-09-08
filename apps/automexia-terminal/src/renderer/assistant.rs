// Copyright (c) 2023-present, Raphael Amorim.
//
// This source code is licensed under the MIT license found in the
// LICENSE file in the root directory of this source tree.

//! Branded, renderer-owned diagnostic dialog.
//!
//! The dialog is modal: application input routing must consume pointer,
//! keyboard, IME, wheel, and dropped-file events while it is active.

use crate::renderer::responsive::{elide_end, Viewport};
use crate::renderer::ui_theme::{
    color_u8, UiTheme, BRAND_AMBER, BRAND_CORAL, BRAND_CYAN, MODAL_SCRIM, MODAL_SHADOW,
};
use rio_backend::config::colors::Colors;
use rio_backend::error::{RioError, RioErrorLevel};
use rio_backend::sugarloaf::text::DrawOpts;
use rio_backend::sugarloaf::Sugarloaf;

const IDEAL_WIDTH: f32 = 520.0;
const IDEAL_HEIGHT: f32 = 300.0;
const MARGIN: f32 = 18.0;
const PADDING: f32 = 22.0;
const CLOSE_SIZE: f32 = 30.0;
const ACTION_HEIGHT: f32 = 36.0;
const HEADER_HEIGHT: f32 = 72.0;
const LINE_HEIGHT: f32 = 19.0;
const MAX_VISIBLE_LINES: usize = 8;
const ORDER: u8 = 29;

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
    docs: Rect,
    compact: bool,
    tiny: bool,
    visible_lines: usize,
}

/// Actions triggered by the diagnostic dialog controls.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssistantOverlayAction {
    Close,
    OpenDocs,
}

#[derive(Default)]
pub struct AssistantOverlay {
    error: Option<RioError>,
    hovered_button: Option<AssistantOverlayAction>,
}

impl AssistantOverlay {
    #[inline]
    pub fn is_active(&self) -> bool {
        self.error.is_some()
    }

    #[inline]
    pub fn set_error(&mut self, error: RioError) {
        self.error = Some(error);
        self.hovered_button = None;
    }

    #[inline]
    pub fn clear(&mut self) {
        self.error = None;
        self.hovered_button = None;
    }

    fn body_line_count(&self) -> usize {
        self.error
            .as_ref()
            .map(|error| error.report.to_string().lines().count().max(1))
            .unwrap_or_default()
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
        let compact = width < 360.0 || height < 230.0;
        let tiny = width < 220.0 || height < 125.0;
        let inset = if tiny {
            6.0
        } else if compact {
            12.0
        } else {
            PADDING
        };
        let control_height = ACTION_HEIGHT.min((height - inset * 2.0).max(24.0));
        let docs = Rect {
            x: card.x + inset,
            y: (card.y + height - inset - control_height).max(card.y),
            width: (width - inset * 2.0).max(1.0),
            height: control_height,
        };
        let close_size = CLOSE_SIZE.min((height - inset).max(24.0));
        let close = Rect {
            x: (card.x + width - inset - close_size).max(card.x),
            y: card.y + inset,
            width: close_size,
            height: close_size,
        };
        let body_top = card.y + if compact { 48.0 } else { HEADER_HEIGHT };
        let body_height = (docs.y - body_top - 8.0).max(0.0);
        let visible_lines = self
            .body_line_count()
            .min(MAX_VISIBLE_LINES)
            .min((body_height / LINE_HEIGHT).floor() as usize);
        Layout {
            card,
            close,
            docs,
            compact,
            tiny,
            visible_lines,
        }
    }

    #[inline]
    pub fn hovered_button(&self) -> Option<AssistantOverlayAction> {
        self.hovered_button
    }

    /// Returns Err outside the card so callers may dismiss the dialog.
    pub fn hit_test(
        &self,
        mouse_x: f32,
        mouse_y: f32,
        dimensions: (f32, f32, f32),
    ) -> Result<Option<AssistantOverlayAction>, ()> {
        if !self.is_active() {
            return Err(());
        }
        let layout = self.layout(dimensions);
        if !layout.card.contains(mouse_x, mouse_y) {
            return Err(());
        }
        if layout.close.contains(mouse_x, mouse_y) {
            Ok(Some(AssistantOverlayAction::Close))
        } else if layout.docs.contains(mouse_x, mouse_y) {
            Ok(Some(AssistantOverlayAction::OpenDocs))
        } else {
            Ok(None)
        }
    }

    /// Update hover state. Returns true when a redraw is required.
    pub fn hover(
        &mut self,
        mouse_x: f32,
        mouse_y: f32,
        dimensions: (f32, f32, f32),
    ) -> bool {
        let next = if self.is_active() {
            let layout = self.layout(dimensions);
            if layout.close.contains(mouse_x, mouse_y) {
                Some(AssistantOverlayAction::Close)
            } else if layout.docs.contains(mouse_x, mouse_y) {
                Some(AssistantOverlayAction::OpenDocs)
            } else {
                None
            }
        } else {
            None
        };
        if next == self.hovered_button {
            false
        } else {
            self.hovered_button = next;
            true
        }
    }

    pub fn render(
        &mut self,
        sugarloaf: &mut Sugarloaf,
        dimensions: (f32, f32, f32),
        colors: &Colors,
    ) {
        let Some(error) = self.error.clone() else {
            return;
        };
        let viewport = Viewport::from_physical(dimensions.0, dimensions.1, dimensions.2);
        let layout = self.layout(dimensions);
        let card = layout.card;
        let theme =
            UiTheme::resolve(colors.background.0, colors.foreground, colors.black);
        let severity = if error.level == RioErrorLevel::Error {
            BRAND_CORAL
        } else {
            BRAND_AMBER
        };
        let severity_label = if error.level == RioErrorLevel::Error {
            "ERROR"
        } else {
            "WARNING"
        };

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
            severity,
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
            let badge_width = if layout.compact { 68.0 } else { 78.0 };
            rounded(
                sugarloaf,
                card.x + inset,
                card.y + inset,
                badge_width,
                24.0,
                severity,
                7.0,
            );
            let badge = DrawOpts {
                font_size: 10.5,
                color: [3, 12, 21, 255],
                bold: true,
                ..DrawOpts::default()
            };
            sugarloaf.text_mut().draw(
                card.x + inset + 10.0,
                card.y + inset + 5.0,
                severity_label,
                &badge,
            );
            if !layout.compact {
                let title = DrawOpts {
                    font_size: 16.0,
                    color: color_u8(theme.text),
                    bold: true,
                    ..DrawOpts::default()
                };
                sugarloaf.text_mut().draw(
                    card.x + inset + badge_width + 12.0,
                    card.y + inset + 3.0,
                    "Automexia needs attention",
                    &title,
                );
            }

            let body = DrawOpts {
                font_size: 12.5,
                color: color_u8(theme.text),
                ..DrawOpts::default()
            };
            let body_x = card.x + inset;
            let body_y = card.y + if layout.compact { 51.0 } else { HEADER_HEIGHT };
            let available_width = (card.width - inset * 2.0).max(1.0);
            let report = error.report.to_string();
            for (index, line) in report.lines().take(layout.visible_lines).enumerate() {
                let line = elide_end(sugarloaf, line, available_width, &body);
                sugarloaf.text_mut().draw(
                    body_x,
                    body_y + index as f32 * LINE_HEIGHT,
                    &line,
                    &body,
                );
            }
        }

        let close_fill = if self.hovered_button == Some(AssistantOverlayAction::Close) {
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
            color: color_u8(theme.text),
            bold: true,
            ..DrawOpts::default()
        };
        sugarloaf.text_mut().draw(
            layout.close.x + 9.0,
            layout.close.y + 5.0,
            "×",
            &close,
        );

        let docs_fill = if self.hovered_button == Some(AssistantOverlayAction::OpenDocs) {
            theme.raised
        } else {
            theme.surface
        };
        rounded(
            sugarloaf,
            layout.docs.x,
            layout.docs.y,
            layout.docs.width,
            layout.docs.height,
            docs_fill,
            8.0,
        );
        let docs = DrawOpts {
            font_size: if layout.tiny { 11.0 } else { 12.5 },
            color: color_u8(BRAND_CYAN),
            bold: true,
            ..DrawOpts::default()
        };
        let docs_label = if layout.tiny {
            "Help"
        } else {
            "↗  Open troubleshooting guide   D"
        };
        sugarloaf.text_mut().draw(
            layout.docs.x + 12.0,
            layout.docs.y + (layout.docs.height - docs.font_size) * 0.5,
            docs_label,
            &docs,
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

    fn active_overlay() -> AssistantOverlay {
        let mut overlay = AssistantOverlay::default();
        overlay.set_error(RioError {
            level: RioErrorLevel::Warning,
            report: rio_backend::error::RioErrorType::ConfigurationNotFound,
        });
        overlay
    }

    #[test]
    fn diagnostic_surface_fits_minimum_window() {
        let overlay = active_overlay();
        for dimensions in [(300.0, 200.0, 1.0), (180.0, 90.0, 1.0), (600.0, 400.0, 2.0)] {
            let layout = overlay.layout(dimensions);
            let viewport =
                Viewport::from_physical(dimensions.0, dimensions.1, dimensions.2);
            assert!(layout.card.x >= 0.0 && layout.card.y >= 0.0);
            assert!(layout.card.x + layout.card.width <= viewport.width);
            assert!(layout.card.y + layout.card.height <= viewport.height);
        }
    }

    #[test]
    fn diagnostic_surface_is_dpi_invariant() {
        let overlay = active_overlay();
        let a = overlay.layout((600.0, 400.0, 1.0));
        let b = overlay.layout((1_200.0, 800.0, 2.0));
        assert_eq!(a.card, b.card);
        assert_eq!(a.close, b.close);
        assert_eq!(a.docs, b.docs);
    }

    #[test]
    fn normal_controls_meet_minimum_pointer_target() {
        let overlay = active_overlay();
        let layout = overlay.layout((800.0, 600.0, 1.0));
        assert!(layout.close.width >= 24.0 && layout.close.height >= 24.0);
        assert!(layout.docs.width >= 24.0 && layout.docs.height >= 24.0);
    }

    #[test]
    fn hit_test_distinguishes_controls_card_and_scrim() {
        let overlay = active_overlay();
        let dimensions = (800.0, 600.0, 1.0);
        let layout = overlay.layout(dimensions);
        assert_eq!(
            overlay.hit_test(layout.close.x + 1.0, layout.close.y + 1.0, dimensions),
            Ok(Some(AssistantOverlayAction::Close))
        );
        assert_eq!(
            overlay.hit_test(layout.docs.x + 1.0, layout.docs.y + 1.0, dimensions),
            Ok(Some(AssistantOverlayAction::OpenDocs))
        );
        assert_eq!(
            overlay.hit_test(layout.card.x + 2.0, layout.card.y + 2.0, dimensions),
            Ok(None)
        );
        assert_eq!(overlay.hit_test(0.0, 0.0, dimensions), Err(()));
    }
}
