// Copyright (c) 2023-present, Raphael Amorim.
//
// This source code is licensed under the MIT license found in the
// LICENSE file in the root directory of this source tree.

use crate::renderer::responsive::Viewport;
use rio_backend::sugarloaf::text::DrawOpts;
use rio_backend::sugarloaf::Sugarloaf;

const SCRIM: [f32; 4] = [0.0, 0.012, 0.028, 0.82];
const SHADOW: [f32; 4] = [0.0, 0.0, 0.0, 0.52];
const OUTLINE: [f32; 4] = [0.0, 0.67, 0.94, 1.0];
const CARD: [f32; 4] = [0.012, 0.035, 0.062, 1.0];
const CANCEL: [f32; 4] = [0.025, 0.105, 0.155, 1.0];
const QUIT: [f32; 4] = [0.37, 0.025, 0.07, 1.0];
const QUIT_OUTLINE: [f32; 4] = [1.0, 0.30, 0.42, 1.0];
const ORDER: u8 = 20;

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
    cancel: Rect,
    quit: Rect,
    compact: bool,
    tiny: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConfirmQuitAction {
    Cancel,
    Quit,
}

#[derive(Default)]
pub struct ConfirmQuit {
    active: bool,
}

impl ConfirmQuit {
    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn set_active(&mut self, active: bool) {
        self.active = active;
    }

    fn layout(dimensions: (f32, f32, f32)) -> Layout {
        let viewport = Viewport::from_physical(dimensions.0, dimensions.1, dimensions.2);
        let win_w = viewport.width.max(1.0);
        let win_h = viewport.height.max(1.0);
        let margin = if win_w < 220.0 || win_h < 150.0 {
            6.0
        } else {
            18.0
        };
        let width = 432.0_f32.min((win_w - margin * 2.0).max(1.0));
        let height = 224.0_f32.min((win_h - margin * 2.0).max(1.0));
        let card = Rect {
            x: ((win_w - width) * 0.5).max(0.0),
            y: ((win_h - height) * 0.5).max(0.0),
            width,
            height,
        };
        let compact = width < 300.0 || height < 170.0;
        let tiny = width < 220.0 || height < 110.0;
        let inner = if tiny {
            6.0
        } else if compact {
            10.0
        } else {
            24.0
        };
        let gap = if tiny {
            4.0
        } else if compact {
            6.0
        } else {
            10.0
        };
        let button_height = (if tiny {
            28.0_f32
        } else if compact {
            32.0
        } else {
            38.0
        })
        .min((height - inner * 2.0).max(1.0));
        let button_width = ((width - inner * 2.0 - gap) * 0.5).max(1.0);
        let button_y = if tiny {
            card.y + ((height - button_height) * 0.5).max(0.0)
        } else {
            (card.y + height - inner - button_height).max(card.y)
        };
        let cancel = Rect {
            x: card.x + inner,
            y: button_y,
            width: button_width,
            height: button_height,
        };
        let quit = Rect {
            x: cancel.x + button_width + gap,
            ..cancel
        };
        Layout {
            card,
            cancel,
            quit,
            compact,
            tiny,
        }
    }

    pub fn hit_test(
        &self,
        mouse_x: f32,
        mouse_y: f32,
        dimensions: (f32, f32, f32),
    ) -> Option<ConfirmQuitAction> {
        if !self.active {
            return None;
        }
        let layout = Self::layout(dimensions);
        if layout.cancel.contains(mouse_x, mouse_y) {
            Some(ConfirmQuitAction::Cancel)
        } else if layout.quit.contains(mouse_x, mouse_y) {
            Some(ConfirmQuitAction::Quit)
        } else {
            None
        }
    }

    pub fn render(&self, sugarloaf: &mut Sugarloaf, dimensions: (f32, f32, f32)) {
        if !self.active {
            return;
        }
        let viewport = Viewport::from_physical(dimensions.0, dimensions.1, dimensions.2);
        let layout = Self::layout(dimensions);
        let card = layout.card;
        sugarloaf.begin_modal_layer();

        sugarloaf.rect(
            None,
            0.0,
            0.0,
            viewport.width,
            viewport.height,
            SCRIM,
            0.0,
            ORDER,
        );
        rounded(
            sugarloaf,
            card.x + 7.0,
            card.y + 9.0,
            card.width,
            card.height,
            SHADOW,
            14.0,
        );
        rounded(
            sugarloaf,
            card.x,
            card.y,
            card.width,
            card.height,
            OUTLINE,
            14.0,
        );
        rounded(
            sugarloaf,
            card.x + 1.0,
            card.y + 1.0,
            (card.width - 2.0).max(1.0),
            (card.height - 2.0).max(1.0),
            CARD,
            13.0,
        );
        rounded(
            sugarloaf,
            card.x + 20.0,
            card.y + 17.0,
            36.0_f32.min((card.width - 40.0).max(1.0)),
            3.0,
            OUTLINE,
            2.0,
        );

        let title = DrawOpts {
            font_size: if layout.compact { 15.0 } else { 18.0 },
            color: [235, 247, 255, 255],
            bold: true,
            ..DrawOpts::default()
        };
        let body = DrawOpts {
            font_size: 13.0,
            color: [164, 190, 207, 255],
            ..DrawOpts::default()
        };
        let label = DrawOpts {
            font_size: if layout.compact { 12.0 } else { 13.0 },
            color: [238, 249, 255, 255],
            bold: true,
            ..DrawOpts::default()
        };
        let key = DrawOpts {
            font_size: 11.0,
            color: [119, 157, 181, 255],
            bold: true,
            ..DrawOpts::default()
        };
        let text_x = card.x + if layout.compact { 12.0 } else { 24.0 };
        let title_y = card.y + if layout.compact { 17.0 } else { 31.0 };
        if !layout.tiny {
            sugarloaf
                .text_mut()
                .draw(text_x, title_y, "Close Automexia?", &title);
        }
        if !layout.compact {
            sugarloaf.text_mut().draw(
                text_x,
                title_y + 37.0,
                "All running sessions in this window will be closed.",
                &body,
            );
            sugarloaf.text_mut().draw(
                text_x,
                title_y + 62.0,
                "This action cannot be undone.",
                &body,
            );
        }

        button(
            sugarloaf,
            layout.cancel,
            CANCEL,
            OUTLINE,
            if layout.tiny { "N" } else { "Cancel" },
            if layout.tiny { "" } else { "Esc / N" },
            &label,
            &key,
        );
        button(
            sugarloaf,
            layout.quit,
            QUIT,
            QUIT_OUTLINE,
            if layout.tiny { "Y" } else { "Close" },
            if layout.tiny { "" } else { "Y" },
            &label,
            &key,
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

#[allow(clippy::too_many_arguments)]
fn button(
    sugarloaf: &mut Sugarloaf,
    rect: Rect,
    fill: [f32; 4],
    outline: [f32; 4],
    text: &str,
    key: &str,
    text_opts: &DrawOpts,
    key_opts: &DrawOpts,
) {
    rounded(
        sugarloaf,
        rect.x,
        rect.y,
        rect.width,
        rect.height,
        outline,
        8.0,
    );
    rounded(
        sugarloaf,
        rect.x + 1.0,
        rect.y + 1.0,
        (rect.width - 2.0).max(1.0),
        (rect.height - 2.0).max(1.0),
        fill,
        7.0,
    );
    let text_width = sugarloaf.text_mut().measure(text, text_opts);
    let key_width = sugarloaf.text_mut().measure(key, key_opts);
    let label_gap = if key.is_empty() { 0.0 } else { 9.0 };
    let x = rect.x + ((rect.width - text_width - key_width - label_gap) * 0.5).max(2.0);
    let y = rect.y + ((rect.height - text_opts.font_size) * 0.5).max(2.0) - 1.0;
    let drawn = sugarloaf.text_mut().draw(x, y, text, text_opts);
    if !key.is_empty() {
        sugarloaf
            .text_mut()
            .draw(x + drawn + label_gap, y + 1.0, key, key_opts);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_stays_inside_extreme_viewports() {
        for d in [
            (80.0, 60.0, 1.0),
            (320.0, 180.0, 1.0),
            (1920.0, 1080.0, 1.0),
            (7680.0, 4320.0, 2.0),
        ] {
            let viewport = Viewport::from_physical(d.0, d.1, d.2);
            let l = ConfirmQuit::layout(d);
            assert!(l.card.x >= 0.0 && l.card.y >= 0.0);
            assert!(l.card.x + l.card.width <= viewport.width + f32::EPSILON);
            assert!(l.card.y + l.card.height <= viewport.height + f32::EPSILON);
            assert!(l.cancel.x + l.cancel.width <= l.quit.x);
            assert!(l.quit.x + l.quit.width <= l.card.x + l.card.width);
            assert!(l.quit.y + l.quit.height <= l.card.y + l.card.height);
        }
    }

    #[test]
    fn hit_test_requires_an_active_explicit_button() {
        let d = (1280.0, 720.0, 1.0);
        let mut dialog = ConfirmQuit::default();
        assert_eq!(dialog.hit_test(0.0, 0.0, d), None);
        dialog.set_active(true);
        let l = ConfirmQuit::layout(d);
        assert_eq!(
            dialog.hit_test(l.cancel.x + 1.0, l.cancel.y + 1.0, d),
            Some(ConfirmQuitAction::Cancel)
        );
        assert_eq!(
            dialog.hit_test(l.quit.x + 1.0, l.quit.y + 1.0, d),
            Some(ConfirmQuitAction::Quit)
        );
        assert_eq!(dialog.hit_test(l.card.x, l.card.y, d), None);
    }

    #[test]
    fn modal_surfaces_are_opaque_and_scrim_is_strong() {
        for surface in [CARD, CANCEL, QUIT] {
            assert_eq!(surface[3], 1.0);
        }
        let [_, _, _, scrim_alpha] = SCRIM;
        assert!(scrim_alpha >= 0.75);
    }
}
