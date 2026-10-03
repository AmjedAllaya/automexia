use super::*;
use crate::automexia::ui::command_info::Label;
use crate::renderer::timestamps::{TimestampPaint, TimestampText};
use rio_backend::config::presentation::TimestampAppearance;

pub(super) fn sample_text(
    appearance: TimestampAppearance,
    enabled: bool,
    exit: Option<i32>,
    elapsed: Option<u64>,
) -> TimestampText {
    TimestampText::new(
        appearance,
        enabled,
        Some(
            rio_backend::crosswords::grid::row::SemanticCommandTimestamp {
                unix_ms: 1_790_793_296_123,
                year: 2026,
                month: 9,
                day: 30,
                hour: 12,
                minute: 34,
                second: 56,
            },
        ),
        exit,
        elapsed,
    )
}

impl SettingsView {
    pub(super) fn paint_timestamp_preview(
        &self,
        canvas: &mut impl Canvas,
        sample: Rect,
        _theme: UiTheme,
    ) {
        let (appearance, colors) = self
            .customizations
            .as_ref()
            .and_then(|navigation| navigation.slot_pages.as_ref())
            .map(SlotPageSnapshot::preview_timestamps)
            .unwrap_or_default();
        let shown = self.preview_bool(automexia_ui_model::settings::COMMAND_TIMESTAMPS);
        rect(canvas, sample, colors.background.0, sample);
        let font = (self.font * 0.72).clamp(9.0, 16.0);
        let row_height = font * 1.6;
        let tag_height = font * 1.35;
        let tag_opts = DrawOpts {
            font_size: font,
            ..Default::default()
        };
        let text_opts = DrawOpts {
            font_size: font * appearance.size.unwrap_or_default().scale(),
            bold: appearance.bold.unwrap_or(false),
            ..Default::default()
        };
        let tags = [
            Label {
                text: "main",
                leading: 0.0,
                padding: 5.0,
                align_end: false,
            },
            Label {
                text: "user",
                leading: 0.0,
                padding: 5.0,
                align_end: false,
            },
        ];
        let mut y = sample.y + 2.0;
        for (exit, duration, command) in [
            (Some(0), Some(104), "λ build"),
            (Some(1), Some(1250), "λ test"),
            (None, None, "λ command"),
        ] {
            let text = sample_text(appearance, shown, exit, duration);
            let Some(band) = text.pack(
                &tags,
                (sample.width - 12.0).max(1.0),
                4.0,
                &[],
                None,
                0.0,
                |completion, value| {
                    canvas
                        .text()
                        .measure(value, if completion { &text_opts } else { &tag_opts })
                },
            ) else {
                continue;
            };
            if y > sample.y + 2.0
                && y + (band.rows + 1) as f32 * row_height > sample.y + sample.height
            {
                break;
            }
            let paint = TimestampPaint {
                appearance,
                colors,
                exit_code: exit,
                origin: [sample.x, y],
                metrics: [row_height, font, tag_height],
                clip: sample.array(),
            };
            for fragment in &band.fragments {
                let [x, top, width, height] = paint.bounds(fragment);
                if top >= sample.y + sample.height {
                    break;
                }
                let bounds = Rect {
                    x,
                    y: top,
                    width,
                    height,
                };
                if fragment.item < tags.len() {
                    let color = if fragment.item == 0 {
                        colors.magenta
                    } else {
                        colors.green
                    };
                    rect(canvas, bounds, color, sample);
                    let value = tags[fragment.item]
                        .text
                        .get(fragment.bytes.clone())
                        .unwrap_or("");
                    if let Some(clip) = bounds.intersect(sample) {
                        canvas.text().draw_clipped(
                            x + fragment.padding,
                            top + (height - font) * 0.5 - 1.0,
                            value,
                            &DrawOpts {
                                font_size: font * fragment.text_scale,
                                color: color_u8(colors.background.0),
                                ..Default::default()
                            },
                            clip.array(),
                        );
                    }
                } else {
                    rect(canvas, bounds, paint.background(), sample);
                    paint.draw(canvas.text(), &text.text, &text.spans, fragment);
                }
            }
            y += band.rows as f32 * row_height;
            label(
                canvas,
                Rect {
                    x: sample.x + 2.0,
                    y,
                    width: sample.width - 4.0,
                    height: row_height,
                },
                command,
                font,
                colors.foreground,
                false,
                sample,
            );
            y += row_height * 2.0;
            if y >= sample.y + sample.height {
                break;
            }
        }
    }
}
