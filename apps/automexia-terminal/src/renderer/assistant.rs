// Copyright (c) 2023-present, Raphael Amorim.
//
// This source code is licensed under the MIT license found in the
// LICENSE file in the root directory of this source tree.

use crate::renderer::responsive::{elide_end, Viewport};
use rio_backend::error::{RioError, RioErrorLevel};
use rio_backend::sugarloaf::text::DrawOpts;
use rio_backend::sugarloaf::Sugarloaf;

/// Convert `[f32; 4]` colour to `[u8; 4]` for the `Text` API (the
/// vertex shader premultiplies, so pass non-premul RGBA).
#[inline]
fn color_u8(c: [f32; 4]) -> [u8; 4] {
    [
        (c[0].clamp(0.0, 1.0) * 255.0) as u8,
        (c[1].clamp(0.0, 1.0) * 255.0) as u8,
        (c[2].clamp(0.0, 1.0) * 255.0) as u8,
        (c[3].clamp(0.0, 1.0) * 255.0) as u8,
    ]
}

// Layout
const OVERLAY_WIDTH: f32 = 480.0;
const OVERLAY_CORNER_RADIUS: f32 = 10.0;
const OVERLAY_MARGIN_TOP: f32 = 8.0;
const OVERLAY_MARGIN_RIGHT: f32 = 8.0;
const OVERLAY_PADDING: f32 = 16.0;

const HEADING_FONT_SIZE: f32 = 16.0;
const BODY_FONT_SIZE: f32 = 12.0;
const BUTTON_FONT_SIZE: f32 = 14.0;
const LINK_FONT_SIZE: f32 = 12.0;

const BUTTON_SIZE: f32 = 24.0;
const BUTTON_CORNER_RADIUS: f32 = 4.0;

const LINE_HEIGHT: f32 = 18.0;
const HEADING_HEIGHT: f32 = 28.0;
const LINK_ROW_HEIGHT: f32 = 24.0;
const MAX_VISIBLE_LINES: usize = 16;

const DOCS_URL: &str = "github.com/AmjedAllaya/automexia-terminal/tree/main/docs";

// Colors
const BACKDROP_COLOR: [f32; 4] = [0.0, 0.0, 0.0, 0.35];
const BG_COLOR: [f32; 4] = [0.12, 0.12, 0.12, 0.98];
const HEADING_COLOR_ERROR: [f32; 4] = [1.0, 0.07, 0.38, 1.0];
const HEADING_COLOR_WARNING: [f32; 4] = [0.99, 0.73, 0.16, 1.0];
const TEXT_COLOR: [f32; 4] = [0.85, 0.85, 0.85, 1.0];
const LINK_COLOR: [f32; 4] = [0.40, 0.60, 1.0, 1.0];
const BUTTON_TEXT_COLOR: [f32; 4] = [0.70, 0.70, 0.70, 1.0];
const BUTTON_HOVER_BG: [f32; 4] = [0.25, 0.25, 0.28, 1.0];

// Depth / order
const DEPTH_BACKDROP: f32 = 0.0;
const DEPTH_BG: f32 = 0.1;
const DEPTH_ELEMENT: f32 = 0.2;
const ORDER: u8 = 20;

/// Actions triggered by clicking assistant overlay buttons.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssistantOverlayAction {
    Close,
    OpenDocs,
}

pub struct AssistantOverlay {
    error: Option<RioError>,
    hovered_button: Option<AssistantOverlayAction>,
    /// Latest rendered width of the docs-link button text — used by
    /// `docs_button_rect` to size the hit target. Updated by `render`.
    link_button_width: f32,
}

impl Default for AssistantOverlay {
    fn default() -> Self {
        Self {
            error: None,
            hovered_button: None,
            link_button_width: 0.0,
        }
    }
}

impl AssistantOverlay {
    #[inline]
    pub fn is_active(&self) -> bool {
        self.error.is_some()
    }

    #[inline]
    pub fn set_error(&mut self, error: RioError) {
        self.error = Some(error);
    }

    #[inline]
    pub fn clear(&mut self) {
        self.error = None;
    }

    /// Returns (overlay_x, overlay_y, overlay_width, overlay_height) in logical coords.
    fn overlay_rect(
        &self,
        window_width: f32,
        window_height: f32,
        scale_factor: f32,
    ) -> (f32, f32, f32, f32, usize) {
        let viewport = Viewport::from_physical(window_width, window_height, scale_factor);
        let width = viewport.fitted_surface(OVERLAY_WIDTH, OVERLAY_MARGIN_RIGHT);
        let x = (viewport.width - width - OVERLAY_MARGIN_RIGHT).max(0.0);
        let y = OVERLAY_MARGIN_TOP.min((viewport.height * 0.05).max(0.0));
        let fixed_height = OVERLAY_PADDING * 2.0 + HEADING_HEIGHT + LINK_ROW_HEIGHT;
        let available_height = (viewport.height - y - OVERLAY_MARGIN_TOP).max(0.0);
        let fitting_lines =
            (((available_height - fixed_height) / LINE_HEIGHT).floor() as usize).max(1);
        let line_count = self
            .body_line_count()
            .min(MAX_VISIBLE_LINES)
            .min(fitting_lines);
        let h = OVERLAY_PADDING
            + HEADING_HEIGHT
            + (line_count as f32 * LINE_HEIGHT)
            + LINK_ROW_HEIGHT
            + OVERLAY_PADDING;
        (x, y, width, h, line_count)
    }

    fn body_line_count(&self) -> usize {
        if let Some(error) = &self.error {
            error.report.to_string().lines().count().max(1)
        } else {
            0
        }
    }

    /// Returns the close button rect.
    fn close_button_rect(
        &self,
        overlay_x: f32,
        overlay_y: f32,
        overlay_width: f32,
    ) -> (f32, f32, f32, f32) {
        let bx = overlay_x + overlay_width - OVERLAY_PADDING - BUTTON_SIZE;
        let by = overlay_y + OVERLAY_PADDING / 2.0;
        (bx, by, BUTTON_SIZE, BUTTON_SIZE)
    }

    /// Returns the docs link button rect (covers the link text area).
    fn docs_button_rect(
        &self,
        overlay_x: f32,
        overlay_y: f32,
        line_count: usize,
        overlay_width: f32,
    ) -> (f32, f32, f32, f32) {
        let by = overlay_y
            + OVERLAY_PADDING
            + HEADING_HEIGHT
            + (line_count as f32 * LINE_HEIGHT);
        let bx = overlay_x + OVERLAY_PADDING - 4.0;
        let bw = (self.link_button_width + 8.0)
            .min((overlay_width - OVERLAY_PADDING * 2.0).max(1.0));
        (bx, by, bw, LINK_ROW_HEIGHT)
    }

    #[inline]
    pub fn hovered_button(&self) -> Option<AssistantOverlayAction> {
        self.hovered_button
    }

    fn hit_test_button(
        mouse_x: f32,
        mouse_y: f32,
        bx: f32,
        by: f32,
        bw: f32,
        bh: f32,
    ) -> bool {
        mouse_x >= bx && mouse_x <= bx + bw && mouse_y >= by && mouse_y <= by + bh
    }

    /// Hit-test a mouse click. Returns Some(action) if a button was clicked.
    /// Returns Err(()) if clicked outside the overlay entirely.
    pub fn hit_test(
        &self,
        mouse_x: f32,
        mouse_y: f32,
        window_width: f32,
        window_height: f32,
        scale_factor: f32,
    ) -> Result<Option<AssistantOverlayAction>, ()> {
        if !self.is_active() {
            return Err(());
        }

        let (ox, oy, ow, oh, line_count) =
            self.overlay_rect(window_width, window_height, scale_factor);

        if mouse_x < ox || mouse_x > ox + ow || mouse_y < oy || mouse_y > oy + oh {
            return Err(());
        }

        let (bx, by, bw, bh) = self.close_button_rect(ox, oy, ow);
        if Self::hit_test_button(mouse_x, mouse_y, bx, by, bw, bh) {
            return Ok(Some(AssistantOverlayAction::Close));
        }

        let (bx, by, bw, bh) = self.docs_button_rect(ox, oy, line_count, ow);
        if Self::hit_test_button(mouse_x, mouse_y, bx, by, bw, bh) {
            return Ok(Some(AssistantOverlayAction::OpenDocs));
        }

        Ok(None)
    }

    /// Update hover state based on mouse position. Returns true if changed.
    pub fn hover(
        &mut self,
        mouse_x: f32,
        mouse_y: f32,
        window_width: f32,
        window_height: f32,
        scale_factor: f32,
    ) -> bool {
        if !self.is_active() {
            return false;
        }

        let (ox, oy, ow, _oh, line_count) =
            self.overlay_rect(window_width, window_height, scale_factor);

        let (bx, by, bw, bh) = self.close_button_rect(ox, oy, ow);
        let mut new_hover = if Self::hit_test_button(mouse_x, mouse_y, bx, by, bw, bh) {
            Some(AssistantOverlayAction::Close)
        } else {
            None
        };

        if new_hover.is_none() {
            let (bx, by, bw, bh) = self.docs_button_rect(ox, oy, line_count, ow);
            if Self::hit_test_button(mouse_x, mouse_y, bx, by, bw, bh) {
                new_hover = Some(AssistantOverlayAction::OpenDocs);
            }
        }

        if new_hover != self.hovered_button {
            self.hovered_button = new_hover;
            return true;
        }
        false
    }

    pub fn render(&mut self, sugarloaf: &mut Sugarloaf, dimensions: (f32, f32, f32)) {
        if !self.is_active() {
            // Immediate mode: not drawing == not visible.
            return;
        }

        let (window_width, window_height, scale_factor) = dimensions;

        let (ox, oy, ow, oh, visible_count) =
            self.overlay_rect(window_width, window_height, scale_factor);

        // Backdrop
        sugarloaf.rect(
            None,
            0.0,
            0.0,
            window_width / scale_factor,
            window_height / scale_factor,
            BACKDROP_COLOR,
            DEPTH_BACKDROP,
            ORDER,
        );

        // Background
        sugarloaf.rounded_rect(
            None,
            ox,
            oy,
            ow,
            oh,
            BG_COLOR,
            DEPTH_BG,
            OVERLAY_CORNER_RADIUS,
            ORDER,
        );

        let error = self.error.clone().unwrap();
        let is_error = error.level == RioErrorLevel::Error;
        let heading_color = if is_error {
            HEADING_COLOR_ERROR
        } else {
            HEADING_COLOR_WARNING
        };

        // Heading
        let heading_text = if is_error { "Error" } else { "Warning" };
        let text_x = ox + OVERLAY_PADDING;
        let heading_y = oy + OVERLAY_PADDING;
        let heading_opts = DrawOpts {
            font_size: HEADING_FONT_SIZE,
            color: color_u8(heading_color),
            ..DrawOpts::default()
        };
        sugarloaf
            .text_mut()
            .draw(text_x, heading_y, heading_text, &heading_opts);

        // Body lines
        let body_y_start = heading_y + HEADING_HEIGHT;
        let report_text = error.report.to_string();
        let lines: Vec<&str> = report_text.lines().collect();
        let visible_count = lines.len().min(visible_count);
        let body_opts = DrawOpts {
            font_size: BODY_FONT_SIZE,
            color: color_u8(TEXT_COLOR),
            ..DrawOpts::default()
        };
        for (i, line_text) in lines.iter().take(visible_count).enumerate() {
            let line_y = body_y_start + (i as f32 * LINE_HEIGHT);
            let visible_line = elide_end(
                sugarloaf,
                line_text,
                (ow - OVERLAY_PADDING * 2.0).max(0.0),
                BODY_FONT_SIZE,
            );
            sugarloaf
                .text_mut()
                .draw(text_x, line_y, &visible_line, &body_opts);
        }

        // Docs link button
        let line_count = visible_count;
        let link_area_y =
            oy + OVERLAY_PADDING + HEADING_HEIGHT + (line_count as f32 * LINE_HEIGHT);
        let link_x = ox + OVERLAY_PADDING;
        let link_y = link_area_y + (LINK_ROW_HEIGHT - LINK_FONT_SIZE) / 2.0;
        let link_opts = DrawOpts {
            font_size: LINK_FONT_SIZE,
            color: color_u8(LINK_COLOR),
            ..DrawOpts::default()
        };
        let visible_link = elide_end(
            sugarloaf,
            DOCS_URL,
            (ow - OVERLAY_PADDING * 2.0).max(0.0),
            LINK_FONT_SIZE,
        );
        let rendered_width =
            sugarloaf
                .text_mut()
                .draw(link_x, link_y, &visible_link, &link_opts);
        self.link_button_width = rendered_width;

        let (dbx, dby, dbw, dbh) = self.docs_button_rect(ox, oy, line_count, ow);
        let docs_hovered = self.hovered_button == Some(AssistantOverlayAction::OpenDocs);

        if docs_hovered {
            sugarloaf.rounded_rect(
                None,
                dbx,
                dby,
                dbw,
                dbh,
                BUTTON_HOVER_BG,
                DEPTH_ELEMENT,
                BUTTON_CORNER_RADIUS,
                ORDER,
            );
        }

        // Close button
        let (bx, by, bw, bh) = self.close_button_rect(ox, oy, ow);
        let is_hovered = self.hovered_button == Some(AssistantOverlayAction::Close);

        if is_hovered {
            sugarloaf.rounded_rect(
                None,
                bx,
                by,
                bw,
                bh,
                BUTTON_HOVER_BG,
                DEPTH_ELEMENT,
                BUTTON_CORNER_RADIUS,
                ORDER,
            );
        }

        let close_opts = DrawOpts {
            font_size: BUTTON_FONT_SIZE,
            color: color_u8(BUTTON_TEXT_COLOR),
            ..DrawOpts::default()
        };
        let ui = sugarloaf.text_mut();
        let label_w = ui.measure("\u{2022}", &close_opts);
        let label_x = bx + (bw - label_w) / 2.0;
        let label_y = by + (bh - BUTTON_FONT_SIZE) / 2.0;
        ui.draw(label_x, label_y, "\u{2022}", &close_opts);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagnostic_surface_fits_minimum_window() {
        let overlay = AssistantOverlay::default();
        let (x, y, width, height, _) = overlay.overlay_rect(300.0, 200.0, 1.0);
        assert!(x >= 0.0 && y >= 0.0);
        assert!(x + width <= 300.0);
        assert!(y + height <= 200.0);
    }

    #[test]
    fn diagnostic_surface_is_dpi_invariant() {
        let overlay = AssistantOverlay::default();
        assert_eq!(
            overlay.overlay_rect(600.0, 400.0, 1.0),
            overlay.overlay_rect(1_200.0, 800.0, 2.0)
        );
    }
}
