use super::*;
use crate::renderer::table_style::TableStyle;

pub(super) const TABLE_PREVIEW_ROWS: [[&str; 3]; 5] = [
    ["FILE", "TYPE", "SIZE"],
    ["deployment-notes.md", "Markdown", "12 KB"],
    ["application.toml", "Config", "4 KB"],
    ["resources.json", "JSON", "128 KB"],
    ["build.log", "Text", "36 KB"],
];

impl SettingsView {
    pub(super) fn paint_table_preview(
        &self,
        canvas: &mut impl Canvas,
        sample: Rect,
        _theme: UiTheme,
    ) {
        let (appearance, colors) = self
            .customizations
            .as_ref()
            .and_then(|navigation| navigation.slot_pages.as_ref())
            .map(SlotPageSnapshot::preview_tables)
            .unwrap_or_default();
        let style = TableStyle::new(appearance, colors);
        let enabled = self.preview_bool(automexia_ui_model::settings::INLINE_TABLES);
        rect(canvas, sample, colors.background.0, sample);
        let clip = [
            sample.x,
            sample.y,
            sample.x + sample.width,
            sample.y + sample.height,
        ];
        let font = (self.font * 0.68).clamp(9.0, 15.0);
        let line = font * 1.55;
        let stroke = style.stroke(1.0);
        let widths = [
            sample.width * 0.46,
            sample.width * 0.32,
            sample.width * 0.22,
        ];
        let opts = DrawOpts {
            font_size: font,
            ..Default::default()
        };
        let mut y = sample.y;
        let mut painted = 0;
        for (index, values) in TABLE_PREVIEW_ROWS.iter().enumerate() {
            let texts = std::array::from_fn::<_, 3, _>(|column| {
                wrapped(
                    values[column],
                    (widths[column] - 10.0).max(1.0),
                    canvas.text(),
                    &opts,
                )
            });
            let height =
                texts.iter().map(Vec::len).max().unwrap_or(1).max(1) as f32 * line + 4.0;
            if y + height > sample.y + sample.height {
                break;
            }
            let header = index == 0;
            let row = index.saturating_sub(1);
            let mut x = sample.x;
            for column in 0..3 {
                let cell = Rect {
                    x,
                    y,
                    width: widths[column],
                    height,
                };
                let (foreground, background) = if enabled {
                    (
                        style.foreground(header, row, column).unwrap_or_else(|| {
                            style.inherited_foreground(
                                colors.foreground,
                                style.background(header, row, column),
                            )
                        }),
                        style.background(header, row, column),
                    )
                } else {
                    (colors.foreground, colors.background.0)
                };
                rect(canvas, cell, background, sample);
                for (line_index, text) in texts[column].iter().enumerate() {
                    label(
                        canvas,
                        Rect {
                            y: y + line_index as f32 * line,
                            height: line,
                            ..cell
                        },
                        text,
                        font,
                        foreground,
                        enabled && header && appearance.header_bold.unwrap_or(false),
                        cell,
                    );
                }
                x += widths[column];
            }
            if enabled
                && index + 1 < TABLE_PREVIEW_ROWS.len()
                && if header {
                    appearance.header_separator.unwrap_or(true)
                } else {
                    appearance.row_lines.unwrap_or(true)
                }
            {
                style.line(
                    [sample.x, y + height - stroke, sample.width, stroke],
                    clip,
                    false,
                    header,
                    |bounds, color| canvas.rect(bounds, color),
                );
            }
            y += height;
            painted += 1;
        }
        if !enabled || painted == 0 {
            return;
        }
        if appearance.outer_border.unwrap_or(true) {
            for top in [sample.y, y - stroke] {
                style.line(
                    [sample.x, top, sample.width, stroke],
                    clip,
                    false,
                    false,
                    |bounds, color| canvas.rect(bounds, color),
                );
            }
        }
        let mut x = sample.x;
        for column in 0..=3 {
            let outer = column == 0 || column == 3;
            if if outer {
                appearance.outer_border.unwrap_or(true)
            } else {
                appearance.column_lines.unwrap_or(true)
            } {
                style.line(
                    [
                        x - if column == 3 { stroke } else { 0.0 },
                        sample.y,
                        stroke,
                        y - sample.y,
                    ],
                    clip,
                    true,
                    false,
                    |bounds, color| canvas.rect(bounds, color),
                );
            }
            if let Some(width) = widths.get(column) {
                x += width;
            }
        }
    }
}
