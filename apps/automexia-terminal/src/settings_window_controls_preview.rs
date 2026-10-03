//! Nonexecuting samples adapt the real caption painter to the settings canvas.
use super::*;
use crate::renderer::island::{
    draw_window_controls, ChromeAction, WindowControlCanvas, WindowControlRenderContext,
};
use rio_backend::config::presentation::WindowControlStyle;

struct PreviewCanvas<'a, C: Canvas> {
    canvas: &'a mut C,
    origin: (f32, f32),
}
impl<C: Canvas> WindowControlCanvas for PreviewCanvas<'_, C> {
    fn rounded_rect(
        &mut self,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        color: [f32; 4],
        _depth: f32,
        radius: f32,
        _order: u8,
    ) {
        self.canvas.rounded_rect(
            [x + self.origin.0, y + self.origin.1, width, height],
            radius,
            color,
        );
    }
    fn line(
        &mut self,
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        width: f32,
        _depth: f32,
        color: [f32; 4],
        _order: u8,
    ) {
        self.canvas.line(
            (x1 + self.origin.0, y1 + self.origin.1),
            (x2 + self.origin.0, y2 + self.origin.1),
            width,
            color,
        );
    }
    fn close_glyph(
        &mut self,
        x: f32,
        color: [f32; 4],
        size: f32,
        weight: f32,
        y: f32,
        _order: u8,
    ) {
        let h = size * 0.5;
        self.canvas.line(
            (self.origin.0 + x - h, self.origin.1 + y - h),
            (self.origin.0 + x + h, self.origin.1 + y + h),
            weight,
            color,
        );
        self.canvas.line(
            (self.origin.0 + x - h, self.origin.1 + y + h),
            (self.origin.0 + x + h, self.origin.1 + y - h),
            weight,
            color,
        );
    }
}

impl SettingsView {
    pub(super) fn paint_window_controls_preview(
        &self,
        canvas: &mut impl Canvas,
        sample: Rect,
        _theme: UiTheme,
    ) {
        let (appearance, colors) = self
            .customizations
            .as_ref()
            .and_then(|n| n.slot_pages.as_ref())
            .map(SlotPageSnapshot::preview_window_controls)
            .unwrap_or_default();
        let theme = UiTheme::from_colors(&colors);
        let selected = appearance.style.unwrap_or_default();
        let font = (self.font * 0.62).clamp(10.0, 15.0);
        let title_height = font * 1.6;
        let header = 38.0;
        let row_height = title_height + header + 8.0;
        if sample.width < 42.0 || sample.height < title_height + header {
            return;
        }
        rect(canvas, sample, theme.surface, sample);
        let compare = sample.height >= row_height * 4.0;
        let mut y = sample.y;
        for &style in WindowControlStyle::ALL {
            if !compare && style != selected {
                continue;
            }
            let text = if style == selected {
                format!("{} (selected)", style.label())
            } else {
                style.label().into()
            };
            label(
                canvas,
                Rect {
                    y,
                    height: title_height,
                    ..sample
                },
                &text,
                font,
                if style == selected {
                    theme.accent
                } else {
                    theme.text
                },
                style == selected,
                sample,
            );
            let width = (sample.width / 3.0).min(46.0);
            draw_window_controls(
                &mut PreviewCanvas {
                    canvas,
                    origin: (sample.x, y + title_height),
                },
                WindowControlRenderContext {
                    appearance:
                        rio_backend::config::presentation::WindowControlsAppearance {
                            style: Some(style),
                            ..appearance
                        },
                    theme,
                    hover: None,
                    pressed: None,
                    maximized: false,
                    focused: true,
                    header_height: header,
                    controls_x: 0.0,
                    button_width: width,
                },
            );
            y += row_height;
        }
        for (text, hover, pressed, focused, maximized) in [
            ("Hover", Some(ChromeAction::Maximize), None, true, false),
            (
                "Pressed / restore",
                Some(ChromeAction::Maximize),
                Some(ChromeAction::Maximize),
                true,
                true,
            ),
            ("Inactive window", None, None, false, false),
        ] {
            if y + title_height + header > sample.y + sample.height {
                break;
            }
            label(
                canvas,
                Rect {
                    y,
                    height: title_height,
                    ..sample
                },
                text,
                font,
                theme.muted_text,
                false,
                sample,
            );
            draw_window_controls(
                &mut PreviewCanvas {
                    canvas,
                    origin: (sample.x, y + title_height),
                },
                WindowControlRenderContext {
                    appearance,
                    theme,
                    hover,
                    pressed,
                    maximized,
                    focused,
                    header_height: header,
                    controls_x: 0.0,
                    button_width: (sample.width / 3.0).min(46.0),
                },
            );
            y += row_height;
        }
    }
}
