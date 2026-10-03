//! Shared-edge inline table paint. Geometry and glyphs are clipped to the pane.
use super::ui_theme::{self, color_u8, UiTheme};
use crate::context::renderable::RenderableContent;
use automexia_ui_model::tables::TableRowKind;
use rio_backend::config::{
    colors::{AnsiColor, NamedColor},
    presentation::{
        CommandOutputAppearance, HighlightAppearance, HighlightStyle, TableAppearance,
    },
};
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

#[derive(Clone, Copy)]
pub(super) struct PaintOptions<'a> {
    pub colors: Colors,
    pub tables: TableAppearance,
    pub highlight: Option<HighlightAppearance>,
    pub kubernetes_highlight: Option<HighlightAppearance>,
    pub preserve_selection_foreground: bool,
    pub active: bool,
    pub command_output: Option<CommandOutputAppearance>,
    /// Normalized, ordered, pane-local completion anchors from command_info.
    pub command_results: &'a [crate::automexia::ui::CommandResultAnchor],
    pub pulse: Option<&'a super::command_results::CommandResults>,
}

impl Default for PaintOptions<'_> {
    fn default() -> Self {
        Self {
            colors: Colors::default(),
            tables: TableAppearance::default(),
            highlight: None,
            kubernetes_highlight: None,
            preserve_selection_foreground: false,
            active: true,
            command_output: None,
            command_results: &[],
            pulse: None,
        }
    }
}

pub(super) fn draw(
    canvas: &mut impl Canvas,
    content: &RenderableContent,
    [x, y, cell_w, cell_h]: [f32; 4],
    font: f32,
    scale: f32,
    options: PaintOptions<'_>,
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
    let now = std::time::Instant::now();
    let theme = UiTheme::from_colors(&colors);
    let clip = [
        x,
        y,
        x + content.columns as f32 * cell_w,
        y + content.screen_lines as f32 * cell_h,
    ];
    let table_style = super::table_style::TableStyle::new(options.tables, colors);
    let stroke = table_style.stroke(scale);
    let snap = |v: f32| (v * scale).round() / scale;
    for (si, surface) in content.inline_tables.surfaces.iter().enumerate() {
        let mut classifier = crate::automexia::output_semantics::RowClassifier::default();
        let width = surface.layout.width as f32 * cell_w;
        let mut extent: Option<(f32, f32)> = None;
        let mut data_rows = 0;
        for (ri, row) in surface.layout.rows.iter().enumerate() {
            let data_row = data_rows;
            if row.kind != TableRowKind::Header && row.kind != TableRowKind::Rule {
                data_rows += 1;
            }
            // Visit retained headers even when their painted rows are offscreen.
            // One pane-local classifier owns both raw and tabulated output.
            let classification = classifier.classify(&surface.table.source()[ri]);
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
            // One bounded classification per visible logical source row, never
            // a wrapped fragment. Explicit ANSI and selection still win below.
            let classification = (!is_rule).then_some(classification).flatten();
            let row_colors = classification.and_then(|classification| {
                use crate::automexia::output_semantics::OutputDomain;
                let appearance = match classification.domain {
                    OutputDomain::General => options.highlight,
                    OutputDomain::Kubernetes => options.kubernetes_highlight,
                    OutputDomain::Uncertain => None,
                }?;
                let severity = classification.severity?;
                let foreground =
                    (appearance.style != HighlightStyle::Background).then(|| {
                        crate::automexia::presentation::output_foreground(
                            &appearance,
                            &colors,
                            severity,
                        )
                        .map(|channel| f32::from(channel) / 255.0)
                    });
                let background = (appearance.style != HighlightStyle::Foreground)
                    .then(|| {
                        crate::automexia::presentation::output_background(
                            &appearance,
                            severity,
                        )
                    })
                    .flatten()
                    .map(|color| color.map(|channel| f32::from(channel) / 255.0));
                Some((foreground, background))
            });
            let (row_foreground, status_background) = row_colors.unwrap_or((None, None));
            let command_background = (!is_header
                && !is_rule
                && content.hint_matches.is_none()
                && content.hint_labels.is_none()
                && !crate::automexia::output_semantics::protects_command_background(
                    classification,
                    options.highlight.is_some(),
                ))
            .then(|| {
                let appearance = options.command_output?;
                // Command projection and row layout reach the same physical
                // edge through different floating-point sums. Compare painted
                // pixel boundaries so fractional scale cannot drop the last row.
                let visible_top = snap(top.max(clip[1]));
                let visible_bottom = snap(bottom.min(clip[3]));
                // Lookup in the owner's sorted anchors is logarithmic; never
                // scan terminal history or rebuild table recognition here.
                let index = options
                    .command_results
                    .partition_point(|anchor| snap(anchor.y) <= visible_top);
                let anchor = options.command_results.get(index)?;
                let start = anchor.output_top?;
                if !start.is_finite()
                    || snap(start) > visible_top
                    || visible_bottom > snap(anchor.y)
                {
                    return None;
                }
                Some(super::command_results::command_background_color(
                    anchor,
                    appearance,
                    &colors,
                    options
                        .pulse
                        .map_or(0.0, |pulse| pulse.pulse_alpha_for(anchor, now)),
                ))
            })
            .flatten();
            let edge = snap((top + bottom) / 2.0);
            let extent_top = if is_rule { edge } else { top };
            let extent_bottom = if is_rule { edge + stroke } else { bottom };
            extent = Some(
                extent.map_or((extent_top, extent_bottom), |(a, _)| (a, extent_bottom)),
            );
            if let Some(rect) = clipped_rect([x, top, width, bottom - top], clip) {
                canvas.rect(
                    rect,
                    if is_rule {
                        theme.background
                    } else if is_header && options.tables.header_background.is_some() {
                        table_style.background(true, data_row, 0)
                    } else if let Some(color) = status_background.or(command_background) {
                        ui_theme::over(colors.background.0, color)
                    } else {
                        table_style.background(is_header, data_row, 0)
                    },
                );
            }
            if is_rule {
                let after_header = ri
                    .checked_sub(1)
                    .and_then(|previous| surface.layout.rows.get(previous))
                    .is_some_and(|previous| previous.kind == TableRowKind::Header);
                if if ri == 0 || ri + 1 == surface.layout.rows.len() {
                    options.tables.outer_border.unwrap_or(true)
                } else if after_header {
                    options.tables.header_separator.unwrap_or(true)
                } else {
                    options.tables.row_lines.unwrap_or(true)
                } {
                    table_style.line(
                        [x, edge, width, stroke],
                        clip,
                        false,
                        after_header,
                        |rect, color| canvas.rect(rect, color),
                    );
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
            for (ci, cell) in row.cells.iter().enumerate() {
                let column = &surface.layout.columns[ci];
                let cell_left = x + column.content_x as f32 * cell_w;
                let cell_right = cell_left + column.content_width as f32 * cell_w;
                let decoration = table_style.background(is_header, data_row, ci);
                let cell_background =
                    if is_header && options.tables.header_background.is_some() {
                        decoration
                    } else if let Some(color) = status_background {
                        ui_theme::over(colors.background.0, color)
                    } else if let Some(color) = command_background {
                        ui_theme::over(decoration, color)
                    } else {
                        decoration
                    };
                if let Some(bounds) = clipped_rect(
                    [
                        x + column.x as f32 * cell_w,
                        top,
                        column.width as f32 * cell_w,
                        bottom - top,
                    ],
                    clip,
                ) {
                    canvas.rect(bounds, cell_background);
                }
                for (line, fragment) in cell.fragments.iter().enumerate() {
                    let line_top = top + line as f32 * cell_h;
                    if line_top + cell_h <= clip[1] || line_top >= clip[3] {
                        continue;
                    }
                    let Some(bounds) = clipped_rect(
                        [
                            cell_left,
                            line_top + 1.0 / scale,
                            cell_right - cell_left,
                            cell_h - 2.0 / scale,
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
                            } else {
                                // Keep source ANSI colors and inverse video.
                                // Selection above retains its configured
                                // foreground-preservation and background rules.
                                if !style.flags.contains(StyleFlags::INVERSE) {
                                    if matches!(
                                        style.fg,
                                        AnsiColor::Named(NamedColor::Foreground)
                                    ) {
                                        let custom = table_style
                                            .foreground(is_header, data_row, ci);
                                        if let Some(color) = if is_header {
                                            custom.or(row_foreground)
                                        } else {
                                            row_foreground.or(custom)
                                        } {
                                            fg = color;
                                        }
                                    }
                                    if matches!(
                                        style.bg,
                                        AnsiColor::Named(NamedColor::Background)
                                    ) {
                                        bg = cell_background;
                                    }
                                }
                            }
                            canvas.rect(run_clip, bg);
                            if style.flags.contains(StyleFlags::HIDDEN) {
                                return;
                            }
                            let opts = DrawOpts {
                                font_size: font,
                                color: color_u8(fg),
                                bold: style.flags.contains(StyleFlags::BOLD)
                                    || (is_header
                                        && options.tables.header_bold.unwrap_or(false)),
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
            let last = ri + 1 == surface.layout.rows.len();
            let boundary_enabled = if last {
                options.tables.outer_border.unwrap_or(true)
            } else if is_header {
                options.tables.header_separator.unwrap_or(true)
            } else {
                options.tables.row_lines.unwrap_or(true)
            };
            if !next_is_rule && boundary_enabled {
                table_style.line(
                    [x, snap(bottom - stroke), width, stroke],
                    clip,
                    false,
                    is_header,
                    |rect, color| canvas.rect(rect, color),
                );
            }
            if ri == 0 && options.tables.outer_border.unwrap_or(true) {
                table_style.line(
                    [x, snap(top), width, stroke],
                    clip,
                    false,
                    false,
                    |rect, color| canvas.rect(rect, color),
                );
            }
        }
        if let Some((top, bottom)) = extent {
            for boundary in std::iter::once(0)
                .chain(surface.layout.columns.iter().map(|c| c.x + c.width))
            {
                let outer = boundary == 0 || boundary == surface.layout.width;
                if !if outer {
                    options.tables.outer_border.unwrap_or(true)
                } else {
                    options.tables.column_lines.unwrap_or(true)
                } {
                    continue;
                }
                let border_x = snap(
                    x + boundary as f32 * cell_w
                        - (if boundary == surface.layout.width {
                            stroke
                        } else {
                            0.0
                        }),
                );
                table_style.line(
                    [border_x, top, stroke, bottom - top],
                    clip,
                    true,
                    false,
                    |rect, color| canvas.rect(rect, color),
                );
            }
        }
    }
}
