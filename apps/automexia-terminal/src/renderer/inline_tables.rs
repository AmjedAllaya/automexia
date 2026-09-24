//! Shared-edge inline table paint. Geometry and glyphs are clipped to the pane.
use super::ui_theme::{self, color_u8, UiTheme};
use crate::context::renderable::RenderableContent;
use automexia_ui_model::tables::TableRowKind;
use rio_backend::crosswords::style::{Style, StyleFlags};
use rio_backend::{
    config::colors::Colors,
    sugarloaf::{
        text::{DrawOpts, Text, TextCellAnchor, TextCellLayout},
        Sugarloaf,
    },
};
use smallvec::SmallVec;
use unicode_segmentation::UnicodeSegmentation;

pub(super) trait Canvas {
    fn text(&mut self) -> &mut Text;
    fn rect(&mut self, bounds: [f32; 4], color: [f32; 4]);
}
impl Canvas for Sugarloaf<'_> {
    fn text(&mut self) -> &mut Text {
        self.text_mut()
    }
    fn rect(&mut self, [x, y, w, h]: [f32; 4], color: [f32; 4]) {
        Sugarloaf::rect(self, None, x, y, w, h, color, 0.0, 4);
    }
}

fn clipped_rect(
    [x, y, w, h]: [f32; 4],
    [left, top, right, bottom]: [f32; 4],
) -> Option<[f32; 4]> {
    let x1 = x.max(left);
    let y1 = y.max(top);
    let x2 = (x + w).min(right);
    let y2 = (y + h).min(bottom);
    (x2 > x1 && y2 > y1).then_some([x1, y1, x2 - x1, y2 - y1])
}

pub(super) struct PaintOptions {
    pub colors: Colors,
    pub preserve_selection_foreground: bool,
    pub active: bool,
}

impl Default for PaintOptions {
    fn default() -> Self {
        Self {
            colors: Colors::default(),
            preserve_selection_foreground: false,
            active: true,
        }
    }
}

pub(super) fn draw(
    canvas: &mut impl Canvas,
    content: &RenderableContent,
    [x, y, cell_w, cell_h]: [f32; 4],
    font: f32,
    scale: f32,
    options: PaintOptions,
    source_colors: impl Fn(&Style) -> ([f32; 4], [f32; 4]),
) {
    if ![x, y, cell_w, cell_h, font, scale]
        .iter()
        .all(|v| v.is_finite())
        || cell_w <= 0.0
        || cell_h <= 0.0
        || scale <= 0.0
    {
        return;
    }
    let colors = options.colors;
    let theme = UiTheme::resolve(colors.background.0, colors.foreground, colors.tabs);
    let clip = [
        x,
        y,
        x + content.columns as f32 * cell_w,
        y + content.screen_lines as f32 * cell_h,
    ];
    let stroke = 1.0 / scale;
    let snap = |v: f32| (v * scale).round() / scale;
    let rule = ui_theme::over(
        theme.background,
        [theme.outline[0], theme.outline[1], theme.outline[2], 0.4],
    );
    for (si, surface) in content.inline_tables.surfaces.iter().enumerate() {
        let width = surface.layout.width as f32 * cell_w;
        let mut extent: Option<(f32, f32)> = None;
        for (ri, row) in surface.layout.rows.iter().enumerate() {
            let Some((first, height)) =
                content
                    .inline_tables
                    .row_geometry(si, ri, &content.command_rows)
            else {
                continue;
            };
            let top = y + first as f32 * cell_h;
            let bottom = top + height as f32 * cell_h;
            if bottom <= clip[1] || top >= clip[3] {
                continue;
            }
            let is_header = row.kind == TableRowKind::Header;
            let is_rule = row.kind == TableRowKind::Rule;
            let edge = snap((top + bottom) / 2.0);
            let extent_top = if is_rule { edge } else { top };
            let extent_bottom = if is_rule { edge + stroke } else { bottom };
            extent = Some(
                extent.map_or((extent_top, extent_bottom), |(a, _)| (a, extent_bottom)),
            );
            if let Some(rect) = clipped_rect([x, top, width, bottom - top], clip) {
                canvas.rect(
                    rect,
                    if is_header {
                        theme.raised
                    } else {
                        theme.background
                    },
                );
            }
            if is_rule {
                let after_header = ri
                    .checked_sub(1)
                    .and_then(|previous| surface.layout.rows.get(previous))
                    .is_some_and(|previous| previous.kind == TableRowKind::Header);
                if let Some(rect) = clipped_rect([x, edge, width, stroke], clip) {
                    canvas.rect(rect, if after_header { theme.outline } else { rule });
                }
                continue;
            }
            // Structural ruler rows own their shared horizontal edge. Ordinary
            // rows draw one boundary only when no explicit ruler follows them.
            let next_is_rule = surface
                .layout
                .rows
                .get(ri + 1)
                .is_some_and(|next| next.kind == TableRowKind::Rule);
            let border = if is_header { theme.outline } else { rule };
            if !next_is_rule {
                if let Some(rect) =
                    clipped_rect([x, snap(bottom - stroke), width, stroke], clip)
                {
                    canvas.rect(rect, border);
                }
            }
            if is_header && ri == 0 {
                if let Some(rect) = clipped_rect([x, snap(top), width, stroke], clip) {
                    canvas.rect(rect, rule);
                }
            }
            for (ci, cell) in row.cells.iter().enumerate() {
                let column = &surface.layout.columns[ci];
                let cell_left = x + column.content_x as f32 * cell_w;
                let cell_right = cell_left + column.content_width as f32 * cell_w;
                for (line, fragment) in cell.fragments.iter().enumerate() {
                    let line_top = top + line as f32 * cell_h;
                    if line_top + cell_h <= clip[1] || line_top >= clip[3] {
                        continue;
                    }
                    let Some(bounds) = clipped_rect(
                        [
                            cell_left,
                            line_top + stroke,
                            cell_right - cell_left,
                            cell_h - 2.0 * stroke,
                        ],
                        clip,
                    ) else {
                        continue;
                    };
                    let value = &surface.table.source()[ri][fragment.bytes.clone()];
                    let mut paint_run =
                        |bytes: std::ops::Range<usize>,
                         start: usize,
                         width: usize,
                         style: Style,
                         selected: bool,
                         hovered: bool| {
                            let left = cell_left + start as f32 * cell_w;
                            let Some(run_clip) = clipped_rect(
                                [left, bounds[1], width as f32 * cell_w, bounds[3]],
                                [
                                    bounds[0],
                                    bounds[1],
                                    bounds[0] + bounds[2],
                                    bounds[1] + bounds[3],
                                ],
                            ) else {
                                return;
                            };
                            let (mut fg, mut bg) = source_colors(&style);
                            if selected {
                                if !options.preserve_selection_foreground {
                                    fg = colors.selection_foreground;
                                }
                                bg = colors.selection_background;
                            } else if is_header && bg == colors.background.0 {
                                bg = theme.raised;
                            }
                            canvas.rect(run_clip, bg);
                            if style.flags.contains(StyleFlags::HIDDEN) {
                                return;
                            }
                            let opts = DrawOpts {
                                font_size: font,
                                color: color_u8(fg),
                                bold: style.flags.contains(StyleFlags::BOLD),
                                italic: style.flags.contains(StyleFlags::ITALIC),
                                ..DrawOpts::default()
                            };
                            // Keep the VT stride while shaping adjacent text together:
                            // contextual scripts and ligatures need the whole font run.
                            let run_text = &value[bytes];
                            let mut anchors: SmallVec<[TextCellAnchor; 64]> =
                                SmallVec::new();
                            let mut glyph_column = 0usize;
                            for (byte_offset, grapheme) in run_text.grapheme_indices(true)
                            {
                                anchors.push(TextCellAnchor {
                                    byte_offset,
                                    column: glyph_column,
                                });
                                glyph_column +=
                                    crate::automexia::inline_tables::cell_width(grapheme);
                            }
                            canvas.text().draw_cells_clipped(
                                left,
                                line_top + (cell_h - font).max(0.0) / 2.0,
                                run_text,
                                &opts,
                                TextCellLayout {
                                    cell_width: cell_w,
                                    anchors: &anchors,
                                },
                                run_clip,
                            );
                            if hovered {
                                if let Some(underline) = clipped_rect(
                                    [
                                        left,
                                        line_top + cell_h - 2.0 * stroke,
                                        width as f32 * cell_w,
                                        stroke,
                                    ],
                                    [
                                        run_clip[0],
                                        run_clip[1],
                                        run_clip[0] + run_clip[2],
                                        run_clip[1] + run_clip[3],
                                    ],
                                ) {
                                    canvas.rect(underline, fg);
                                }
                            }
                        };
                    let mut cells = 0usize;
                    let mut run: Option<(usize, usize, Style, bool, bool)> = None;
                    for (byte, grapheme) in value.grapheme_indices(true) {
                        let Some((mut pos, style)) = content.inline_tables.source_cell(
                            si,
                            ri,
                            fragment.source_cells.start + cells,
                        ) else {
                            continue;
                        };
                        pos.row.0 -= content.display_offset as i32;
                        let width = crate::automexia::inline_tables::cell_width(grapheme);
                        let selected = content.selection_range.is_some_and(|selection| {
                            (0..width.max(1)).any(|offset| {
                                content
                                    .inline_tables
                                    .source_cell(
                                        si,
                                        ri,
                                        fragment.source_cells.start + cells + offset,
                                    )
                                    .is_some_and(|(mut cell, _)| {
                                        cell.row.0 -= content.display_offset as i32;
                                        selection.contains(cell)
                                    })
                            })
                        });
                        let hovered = options.active
                            && content
                                .highlighted_hint
                                .as_ref()
                                .is_some_and(|hint| hint.start <= pos && pos <= hint.end);
                        if let Some((start, at, previous, was_selected, was_hovered)) =
                            run
                        {
                            if previous != style
                                || was_selected != selected
                                || was_hovered != hovered
                            {
                                paint_run(
                                    start..byte,
                                    at,
                                    cells - at,
                                    previous,
                                    was_selected,
                                    was_hovered,
                                );
                                run = Some((byte, cells, style, selected, hovered));
                            }
                        } else {
                            run = Some((byte, cells, style, selected, hovered));
                        }
                        cells += width;
                    }
                    if let Some((start, at, style, selected, hovered)) = run {
                        paint_run(
                            start..value.len(),
                            at,
                            cells - at,
                            style,
                            selected,
                            hovered,
                        );
                    }
                }
            }
        }
        if let Some((top, bottom)) = extent {
            for boundary in std::iter::once(0)
                .chain(surface.layout.columns.iter().map(|c| c.x + c.width))
            {
                let border_x = snap(
                    x + boundary as f32 * cell_w
                        - (if boundary == surface.layout.width {
                            stroke
                        } else {
                            0.0
                        }),
                );
                if let Some(rect) =
                    clipped_rect([border_x, top, stroke, bottom - top], clip)
                {
                    canvas.rect(rect, rule);
                }
            }
        }
    }
}
