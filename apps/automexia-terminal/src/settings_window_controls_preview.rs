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

pub(super) fn choice_style(id: &SettingId) -> Option<WindowControlStyle> {
    let value = id.as_str().strip_prefix("window-controls.choose.")?;
    WindowControlStyle::ALL
        .iter()
        .copied()
        .find(|style| style.id() == value)
}

impl SettingsView {
    pub(super) fn is_window_controls_preview(&self) -> bool {
        self.customizations
            .as_ref()
            .and_then(|n| n.active_key.as_ref())
            .is_some_and(|id| id.as_str() == crate::settings_catalog::WINDOW_CONTROLS)
    }

    pub(super) fn selected_window_control_style(&self) -> WindowControlStyle {
        self.customizations
            .as_ref()
            .and_then(|n| n.slot_pages.as_ref())
            .map(SlotPageSnapshot::preview_window_controls)
            .unwrap_or_default()
            .0
            .style
            .unwrap_or_default()
    }

    pub(super) fn paint_window_controls_preview(
        &mut self,
        canvas: &mut impl Canvas,
        sample: Rect,
        _theme: UiTheme,
    ) -> Vec<(SettingId, Rect)> {
        let (appearance, colors) = self
            .customizations
            .as_ref()
            .and_then(|n| n.slot_pages.as_ref())
            .map(SlotPageSnapshot::preview_window_controls)
            .unwrap_or_default();
        let theme = UiTheme::from_colors(&colors);
        let selected = appearance.style.unwrap_or_default();
        let font = (self.font * 0.72).clamp(10.0, 16.0);
        let title_height = font * 1.65;
        let gap = 8.0;
        let columns = if sample.width >= 240.0 { 2 } else { 1 };
        let rows = WindowControlStyle::ALL.len().div_ceil(columns);
        let rich_height = title_height + 46.0;
        let compact_height = title_height + 12.0;
        let preview_reserve = title_height * 3.0 + 65.0;
        let rich = sample.height
            >= title_height + rows as f32 * (rich_height + gap) + 23.0 + preview_reserve;
        let card_height = if rich { rich_height } else { compact_height };
        let choices_height = title_height + rows as f32 * (card_height + gap);
        let mut targets = Vec::with_capacity(4);
        if sample.width < 96.0 || sample.height < title_height + 38.0 {
            return targets;
        }
        let mut y = sample.y;
        if choices_height + 23.0 + preview_reserve <= sample.height {
            label(
                canvas,
                Rect {
                    y,
                    height: title_height,
                    ..sample
                },
                "Choose a style",
                font,
                theme.text,
                true,
                sample,
            );
            y += title_height + 4.0;
            let width = (sample.width - gap * (columns - 1) as f32) / columns as f32;
            for (index, &style) in WindowControlStyle::ALL.iter().enumerate() {
                let card = Rect {
                    x: sample.x + (index % columns) as f32 * (width + gap),
                    y: y + (index / columns) as f32 * (card_height + gap),
                    width,
                    height: card_height,
                };
                let Ok(id) =
                    SettingId::new(format!("window-controls.choose.{}", style.id()))
                else {
                    continue;
                };
                let active = style == selected;
                let focused = self.focus == Focus::Preview
                    && self.preview_selected.as_ref() == Some(&id);
                let fill = if active {
                    std::array::from_fn(|i| {
                        if i == 3 {
                            1.0
                        } else {
                            theme.surface[i] * 0.76 + theme.accent[i] * 0.24
                        }
                    })
                } else {
                    theme.surface
                };
                rounded_surface(canvas, card, fill, sample);
                let border = automexia_ui_model::ensure_contrast(theme.accent, fill, 3.1);
                preview_rounded_outline(
                    canvas,
                    Rect {
                        x: card.x + 1.0,
                        y: card.y + 1.0,
                        width: card.width - 2.0,
                        height: card.height - 2.0,
                    },
                    8.0,
                    if active { 2.0 } else { 1.0 },
                    if active { border } else { theme.border },
                );
                if focused {
                    preview_rounded_outline(
                        canvas,
                        Rect {
                            x: card.x + 4.0,
                            y: card.y + 4.0,
                            width: card.width - 8.0,
                            height: card.height - 8.0,
                        },
                        5.0,
                        1.5,
                        automexia_ui_model::ensure_contrast(theme.text, fill, 3.1),
                    );
                }
                let text_color = automexia_ui_model::ensure_contrast(
                    theme.text,
                    fill,
                    automexia_ui_model::MIN_TEXT_CONTRAST,
                );
                label(
                    canvas,
                    Rect {
                        x: card.x + 10.0,
                        y: card.y + 6.0,
                        width: card.width - 38.0,
                        height: title_height,
                    },
                    style.label(),
                    font,
                    text_color,
                    active,
                    card,
                );
                if active {
                    // Geometric checkmark remains meaningful without color or font support.
                    let cx = card.x + card.width - 17.0;
                    let cy = card.y + 6.0 + title_height * 0.5;
                    canvas.line((cx - 4.0, cy), (cx - 1.0, cy + 3.0), 2.0, text_color);
                    canvas.line(
                        (cx - 1.0, cy + 3.0),
                        (cx + 5.0, cy - 4.0),
                        2.0,
                        text_color,
                    );
                }
                if rich {
                    draw_sample(
                        canvas,
                        Rect {
                            x: card.x + 8.0,
                            y: card.y + title_height + 6.0,
                            width: card.width - 16.0,
                            height: 36.0,
                        },
                        appearance,
                        style,
                        theme,
                        PreviewState {
                            hover: None,
                            pressed: None,
                            focused: true,
                            maximized: false,
                        },
                    );
                }
                self.preview_order.push(id.clone());
                self.preview_item_rows.push((id.clone(), index));
                targets.push((id, card));
            }
            self.preview_total_rows = 4;
            self.preview_visible_rows = 4;
            y += rows as f32 * (card_height + gap) + 6.0;
            rect(
                canvas,
                Rect {
                    x: sample.x,
                    y,
                    width: sample.width,
                    height: 1.0,
                },
                theme.border,
                sample,
            );
            y += 12.0;
        }
        // State examples are separate from choices and never receive hit targets.
        let remaining = sample.y + sample.height - y;
        if remaining < title_height + 38.0 {
            return targets;
        }
        label(
            canvas,
            Rect {
                y,
                height: title_height,
                ..sample
            },
            "Live preview",
            font,
            theme.text,
            true,
            sample,
        );
        y += title_height;
        if sample.y + sample.height - y < title_height + 38.0 {
            draw_sample(
                canvas,
                Rect {
                    y,
                    height: 36.0,
                    ..sample
                },
                appearance,
                selected,
                theme,
                PreviewState {
                    hover: None,
                    pressed: None,
                    focused: true,
                    maximized: false,
                },
            );
            return targets;
        }
        if sample.y + sample.height - y >= title_height * 2.0 + 40.0 {
            let detail = if targets.is_empty() {
                format!("{} · use Button style", selected.label())
            } else {
                format!("{} · state examples", selected.label())
            };
            label(
                canvas,
                Rect {
                    y,
                    height: title_height,
                    ..sample
                },
                &detail,
                font * 0.9,
                theme.muted_text,
                false,
                sample,
            );
            y += title_height + 6.0;
        }
        for (text, hover, pressed, focused, maximized) in [
            ("Normal", None, None, true, false),
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
            if y + title_height + 38.0 > sample.y + sample.height {
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
                font * 0.9,
                theme.muted_text,
                false,
                sample,
            );
            draw_sample(
                canvas,
                Rect {
                    y: y + title_height,
                    height: 36.0,
                    ..sample
                },
                appearance,
                selected,
                theme,
                PreviewState {
                    hover,
                    pressed,
                    focused,
                    maximized,
                },
            );
            y += title_height + 44.0;
        }
        targets
    }
}

struct PreviewState {
    hover: Option<ChromeAction>,
    pressed: Option<ChromeAction>,
    focused: bool,
    maximized: bool,
}

fn draw_sample(
    canvas: &mut impl Canvas,
    bounds: Rect,
    appearance: rio_backend::config::presentation::WindowControlsAppearance,
    style: WindowControlStyle,
    theme: UiTheme,
    state: PreviewState,
) {
    draw_window_controls(
        &mut PreviewCanvas {
            canvas,
            origin: (bounds.x, bounds.y),
        },
        WindowControlRenderContext {
            appearance: rio_backend::config::presentation::WindowControlsAppearance {
                style: Some(style),
                ..appearance
            },
            theme,
            hover: state.hover,
            pressed: state.pressed,
            maximized: state.maximized,
            focused: state.focused,
            header_height: bounds.height,
            controls_x: 0.0,
            button_width: (bounds.width / 3.0).min(46.0),
        },
    );
}
