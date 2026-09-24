//! One layout authority for optional context and core completion information.

use super::{command_results, devops_status, SemanticPaneRenderState};
use crate::automexia::ui::command_info::{pack, Band, CompletionLabel, Fragment, Label};
use crate::automexia::ui::{CommandResultAnchor, PromptAnchor};
use crate::context::renderable::RenderableContent;
use rio_backend::config::colors::Colors;
use rio_backend::sugarloaf::{text::DrawOpts, Sugarloaf};

struct Header {
    row: usize,
    prompt: Option<PromptAnchor>,
    result: Option<CommandResultAnchor>,
    completion: String,
    band: Band,
}

/// Shared text emission for context and completion fragments. Geometry values
/// are logical pixels: row height, font size, and tag height respectively.
pub(super) fn draw_fragment_text(
    engine: &mut rio_backend::sugarloaf::text::Text,
    value: &str,
    fragment: &Fragment,
    origin: [f32; 2],
    metrics: [f32; 3],
    color: [u8; 4],
) {
    let [row_height, base_font_size, tag_height] = metrics;
    let inset =
        automexia_ui_model::prompt_context_top_inset(row_height, tag_height, true)
            .unwrap_or(0.0);
    let font_size = base_font_size * fragment.text_scale;
    engine.draw(
        origin[0] + 2.0 + fragment.x + fragment.padding + fragment.leading,
        origin[1]
            + fragment.row as f32 * row_height
            + inset
            + (tag_height - font_size) * 0.5
            - 1.0,
        value,
        &DrawOpts {
            font_size,
            color,
            ..DrawOpts::default()
        },
    );
}

/// Slice native image quads only at display discontinuities. Texture coordinates
/// are cropped, not stretched, so images cannot cover inserted information rows.
pub(super) fn project_images(
    overlays: &mut Vec<rio_backend::sugarloaf::GraphicOverlay>,
    content: &RenderableContent,
    origin_y: f32,
    cell_height: f32,
) {
    if !content.command_rows.expanded() {
        return;
    }
    let mut runs = Vec::new();
    let mut first = 0;
    for row in 1..=content.visible_rows.len() {
        if row == content.visible_rows.len()
            || content.command_rows.origin(row)
                != content.command_rows.origin(row - 1) + 1
        {
            runs.push((first, row));
            first = row;
        }
    }
    let native = std::mem::take(overlays);
    for overlay in native {
        for &(start, end) in &runs {
            let top = origin_y + start as f32 * cell_height;
            let bottom = origin_y + end as f32 * cell_height;
            if overlay.y >= bottom || overlay.y + overlay.height <= top {
                continue;
            }
            let mut slice = overlay.clone();
            let right = slice.x + slice.width;
            let left = slice.x;
            if !rio_backend::ansi::graphics::clip_overlay_to_rect(
                &mut slice, left, top, right, bottom,
            ) {
                continue;
            }
            slice.y += (content.command_rows.visual_row(start) - start as isize) as f32
                * cell_height;
            if rio_backend::ansi::graphics::clip_overlay_to_rect(
                &mut slice,
                left,
                origin_y,
                right,
                origin_y + content.screen_lines as f32 * cell_height,
            ) {
                overlays.push(slice);
            }
        }
    }
}

pub(super) fn layout(
    pane: &mut SemanticPaneRenderState,
    status: Option<&devops_status::DevOpsStatus>,
    content: &mut RenderableContent,
    sugarloaf: &mut Sugarloaf,
    colors: Colors,
) -> bool {
    let (changed, prompts) = prepare(pane, status, content, sugarloaf.text_mut());
    if let Some(status) = status {
        for (anchor, fragment) in prompts {
            status.draw_prompt_fragment(
                sugarloaf,
                colors,
                &anchor,
                pane.session.session_id,
                &fragment,
            );
        }
    }
    changed
}

fn prepare(
    pane: &mut SemanticPaneRenderState,
    status: Option<&devops_status::DevOpsStatus>,
    content: &mut RenderableContent,
    text_engine: &mut rio_backend::sugarloaf::text::Text,
) -> (bool, Vec<(PromptAnchor, Fragment)>) {
    let mut prompts = Vec::new();
    let height = pane.cell_height;
    if !height.is_finite() || height <= 0.0 {
        return (false, prompts);
    }
    let native_row = |y: f32| ((y - pane.origin_y) / height).round().max(0.0) as usize;
    let mut headers = std::collections::BTreeMap::<usize, Header>::new();
    let blank = |row: usize| {
        content
            .visible_rows
            .get(row)
            .is_some_and(super::terminal_row_is_blank)
    };
    if status.is_some() {
        for anchor in pane
            .historical_anchors
            .iter()
            .chain(pane.live_anchor.iter())
        {
            let row = native_row(anchor.y);
            if blank(row) {
                headers
                    .entry(row)
                    .or_insert_with(|| Header {
                        row,
                        prompt: None,
                        result: None,
                        completion: String::new(),
                        band: Band::default(),
                    })
                    .prompt = Some(*anchor);
            }
        }
    }
    for anchor in &pane.command_results {
        let row = native_row(anchor.y);
        if blank(row) {
            let header = headers.entry(row).or_insert_with(|| Header {
                row,
                prompt: None,
                result: None,
                completion: String::new(),
                band: Band::default(),
            });
            header.result = Some(*anchor);
            header.completion = command_results::complete_result_label(anchor);
        }
    }
    let metrics = devops_status::prompt_tag_metrics(height);
    let mut spans = Vec::with_capacity(headers.len());
    for header in headers.values_mut() {
        let segments =
            header
                .prompt
                .as_ref()
                .zip(status)
                .map_or(&[][..], |(anchor, status)| {
                    status.segments_for_prompt(pane.session.session_id, anchor)
                });
        let mut labels: Vec<_> = segments
            .iter()
            .map(|segment| Label {
                text: &segment.value,
                leading: metrics.icon_slot + metrics.icon_gap,
                padding: metrics.padding_x,
                align_end: false,
            })
            .collect();
        if header.result.is_some() {
            labels.push(Label {
                text: &header.completion,
                leading: 0.0,
                padding: 0.0,
                align_end: true,
            });
        }
        let width = header
            .prompt
            .map(|a| a.width)
            .or_else(|| header.result.map(|a| a.width))
            .unwrap_or(0.0);
        let options = DrawOpts {
            font_size: metrics.font_size,
            ..DrawOpts::default()
        };
        if let Some(band) = pack(
            &labels,
            (width - 12.0).max(1.0),
            metrics.tag_gap,
            |_, text| text_engine.measure(text, &options),
        ) {
            spans.push((header.row, band.rows));
            header.band = band;
        }
    }
    spans.extend(content.inline_tables.bands());
    spans.sort_unstable();
    let previous_top = content.command_rows.top();
    let changed = content
        .command_rows
        .rebuild(content.visible_rows.len(), &spans);
    let cursor = (content.display_offset == 0)
        .then_some(content.cursor.state.pos.row.0.max(0) as usize);
    if let Some(cursor) = cursor {
        let last = content
            .visible_rows
            .iter()
            .rposition(|row| !super::terminal_row_is_blank(row))
            .unwrap_or(0)
            .max(cursor);
        content.command_rows.limit_to_native_end(
            (last + 1).min(content.visible_rows.len()),
            content.screen_lines,
        );
    }
    content.command_rows.settle(cursor, content.screen_lines);
    let projection = &content.command_rows;
    let bottom = pane.origin_y + content.screen_lines as f32 * height;
    let project_y = |y: f32| {
        let row = native_row(y);
        pane.origin_y
            + projection.visual_row(row) as f32 * height
            + (y - pane.origin_y - row as f32 * height)
    };
    pane.completion_labels.clear();
    for header in headers.into_values() {
        let first = projection.visual_row(header.row);
        let mut completion = header.result.map(|mut anchor| {
            anchor.y = project_y(anchor.y);
            CompletionLabel {
                anchor,
                text: header.completion,
                fragments: Vec::new(),
            }
        });
        for fragment in header.band.fragments {
            let visual = first + fragment.row as isize;
            if visual < 0 || visual >= content.screen_lines as isize {
                continue;
            }
            let is_completion =
                completion.is_some()
                    && header.prompt.as_ref().zip(status).map_or(
                        0,
                        |(anchor, status)| {
                            status
                                .segments_for_prompt(pane.session.session_id, anchor)
                                .len()
                        },
                    ) == fragment.item;
            if is_completion {
                completion.as_mut().unwrap().fragments.push(fragment);
            } else if let Some(mut anchor) = header.prompt {
                anchor.y = project_y(anchor.y);
                prompts.push((anchor, fragment));
            }
        }
        if let Some(completion) = completion {
            pane.completion_labels.push(completion);
        }
    }
    for anchor in &mut pane.command_results {
        anchor.y = project_y(anchor.y).clamp(pane.origin_y, bottom);
        anchor.output_top = anchor
            .output_top
            .map(|y| project_y(y).clamp(pane.origin_y, bottom));
    }
    (
        changed || previous_top != content.command_rows.top(),
        prompts,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use rio_backend::crosswords::{Crosswords, CrosswordsSize};
    use rio_backend::event::{TerminalDamage, VoidListener, WindowId};
    use rio_backend::performer::handler::Processor;
    use rio_backend::sugarloaf::font::{
        constants, FontData, FontLibrary, FontLibraryData,
    };
    use std::sync::Arc;

    fn fonts() -> FontLibrary {
        let mut data = FontLibraryData::default();
        data.insert(
            FontData::from_static_slice(constants::FONT_CASCADIA_CODE_NF).unwrap(),
        );
        FontLibrary {
            inner: Arc::new(parking_lot::RwLock::new(data)),
        }
    }

    fn projected_grid_pixels(
        content: &RenderableContent,
        width: u32,
        height: u32,
    ) -> Vec<u32> {
        use rio_backend::sugarloaf::grid::{
            cpu::CpuGridRenderer, GridRenderer, GridUniforms,
        };
        let config = rio_backend::config::Config::default();
        let mut renderer = super::super::Renderer::new(&config);
        renderer.named_colors = config.colors;
        renderer.colors = rio_backend::config::colors::term::List::from(&config.colors);
        let mut grid = GridRenderer::Cpu(CpuGridRenderer::new(
            content.columns as u32,
            content.screen_lines as u32,
        ));
        let mut rasterizer = crate::grid_emit::GridGlyphRasterizer::new();
        let mut glyphs = Vec::new();
        let fonts = fonts();
        for visual_row in 0..content.screen_lines {
            let Some(native_row) = content.command_rows.source_row(visual_row) else {
                grid.write_row(visual_row as u32, &[], &[]);
                continue;
            };
            crate::grid_emit::build_row_fg(
                &content.visible_rows[native_row],
                content.columns,
                visual_row as u16,
                &content.style_table,
                &content.extras,
                &renderer,
                &content.term_colors,
                &mut rasterizer,
                &mut grid,
                16.0,
                10.0,
                24.0,
                None,
                &[],
                &fonts,
                1,
                None,
                &mut glyphs,
            );
            grid.write_row(visual_row as u32, &[], &glyphs);
        }
        let uniforms = GridUniforms {
            projection: [0.0; 16],
            grid_padding: [4.0, 8.0, 0.0, 0.0],
            cursor_color: [0.0; 4],
            cursor_bg_color: [0.0; 4],
            cell_size: [10.0, 24.0],
            grid_size: [content.columns as u32, content.screen_lines as u32],
            cursor_pos: [u32::MAX; 2],
            _pad_cursor: [0; 2],
            min_contrast: 0.0,
            flags: 0,
            padding_extend: 0,
            input_colorspace: 0,
        };
        let mut pixels = vec![0x00081218; (width * height) as usize];
        grid.render_text_cpu(&mut pixels, width, height, &uniforms);
        assert!(
            pixels.iter().any(|pixel| *pixel != 0x00081218),
            "native glyphs reach pixels"
        );
        pixels
    }

    #[test]
    fn wrapped_real_font_ink_stays_inside_narrow_panes_at_fractional_scales() {
        let values = [
            "Ubuntu",
            "topic/a\u{301}-example",
            "example-space",
            "alice",
            "ok 21ms 2026-01-01 12:00:00",
        ];
        let labels: Vec<_> = values
            .iter()
            .map(|value| Label {
                text: value,
                leading: 18.0,
                padding: 4.0,
                align_end: false,
            })
            .collect();
        for scale in [1.0, 1.25, 1.5, 2.0, 3.0] {
            let mut text = rio_backend::sugarloaf::text::Text::new(&fonts());
            text.init_cpu();
            text.set_scale_factor(scale);
            for width in [24.0, 64.0, 120.0, 240.0, 720.0] {
                text.clear();
                let options = DrawOpts {
                    font_size: 14.0,
                    ..DrawOpts::default()
                };
                let band = pack(&labels, width - 12.0, 4.0, |_, value| {
                    text.measure(value, &options)
                })
                .unwrap();
                for fragment in &band.fragments {
                    draw_fragment_text(
                        &mut text,
                        &values[fragment.item][fragment.bytes.clone()],
                        fragment,
                        [4.0, 8.0],
                        [24.0, 14.0, 18.0],
                        [255; 4],
                    );
                }
                assert!(text.instance_count() > 0);
                for glyph in text.instances() {
                    let x = glyph.pos[0] + glyph.bearings[0] as f32;
                    assert!(
                        x >= 4.0 * scale
                            && x + glyph.glyph_size[0] as f32 <= (4.0 + width) * scale,
                        "real ink fits {width} logical pixels at scale {scale}"
                    );
                }
            }
        }
    }

    #[test]
    fn short_pane_keeps_all_context_reachable_without_native_history() {
        let mut terminal = Crosswords::new(
            CrosswordsSize::new(12, 3),
            rio_backend::ansi::CursorShape::Block,
            VoidListener {},
            WindowId::from(0),
            0,
            128,
        );
        Processor::default().advance(&mut terminal, b"\x1b]133;A;aid=1\x07 \r\n\x1b]133;P;k=c;aid=1\x07/work\r\n\x1b]133;P;k=c;aid=1\x07lambda \x1b]133;B\x07");
        let mut content = RenderableContent::default();
        let mut pane = snapshot(&mut terminal, &mut content);
        let mut status = devops_status::DevOpsStatus::default();
        status.prepare_prompt_rows(
            &pane.session,
            true,
            &pane.historical_anchors,
            pane.live_anchor,
        );
        let mut text = rio_backend::sugarloaf::text::Text::new(&fonts());
        let (_, initial) = prepare(&mut pane, Some(&status), &mut content, &mut text);
        assert_eq!(terminal.history_size(), 0);
        assert!(content.command_rows.top() > 0);
        assert_eq!(content.command_rows.visual_row(2), 2);
        let mut seen = std::collections::BTreeSet::new();
        for (_, fragment) in initial {
            seen.insert(fragment.item);
        }
        for _ in 0..8 {
            content.scroll_lines(&mut terminal, 1);
            pane = snapshot(&mut terminal, &mut content);
            let (_, prompts) = prepare(&mut pane, Some(&status), &mut content, &mut text);
            for (_, fragment) in prompts {
                seen.insert(fragment.item);
            }
        }
        let anchor = pane.live_anchor.unwrap();
        assert_eq!(seen.len(), status.segments_for_prompt(1, &anchor).len());
        assert_eq!(content.command_rows.top(), 0);
        assert_eq!(terminal.history_size(), 0);
        let before = content.visible_rows.clone();
        pane = snapshot(&mut terminal, &mut content);
        prepare(&mut pane, None, &mut content, &mut text);
        assert!(!content.command_rows.expanded());
        assert_eq!(content.visible_rows, before);
    }

    #[test]
    fn image_slices_leave_wrapped_rows_clear_and_preserve_texture_coordinates() {
        use rio_backend::crosswords::{grid::row::Row, square::Square};
        let mut content = RenderableContent {
            screen_lines: 6,
            visible_rows: vec![Row::<Square>::new(8); 6],
            ..Default::default()
        };
        content.command_rows.rebuild(6, &[(1, 3)]);
        let original = rio_backend::sugarloaf::GraphicOverlay {
            image_id: 1,
            x: 10.0,
            y: 100.0,
            width: 80.0,
            height: 60.0,
            z_index: 0,
            source_rect: [0.0, 0.0, 1.0, 1.0],
        };
        let mut images = vec![original.clone()];
        project_images(&mut images, &content, 100.0, 10.0);
        assert_eq!(images.len(), 2);
        assert_eq!((images[0].y, images[0].height), (100.0, 20.0));
        assert_eq!((images[1].y, images[1].height), (140.0, 20.0));
        assert!((images[0].source_rect[3] - 1.0 / 3.0).abs() < 0.00001);
        assert!((images[1].source_rect[1] - 1.0 / 3.0).abs() < 0.00001);
        assert!((images[1].source_rect[3] - 2.0 / 3.0).abs() < 0.00001);
        assert_eq!(content.command_rows.scroll(-2, 0, 0), 0);
        content.command_rows.rebuild(6, &[(1, 3)]);
        content.command_rows.settle(None, 6);
        let mut images = vec![original];
        project_images(&mut images, &content, 100.0, 10.0);
        assert_eq!(images.len(), 1);
        assert_eq!((images[0].y, images[0].height), (120.0, 40.0));
        assert_eq!(images[0].source_rect[3], 1.0);
    }

    fn snapshot(
        terminal: &mut Crosswords<VoidListener>,
        content: &mut RenderableContent,
    ) -> SemanticPaneRenderState {
        terminal.snapshot_visible(
            &TerminalDamage::Full,
            terminal.columns(),
            &mut content.visible_rows,
            &mut content.style_table,
            &mut content.extras,
        );
        content.columns = terminal.columns();
        content.screen_lines = terminal.screen_lines();
        content.display_offset = terminal.display_offset();
        content.history_size = terminal.history_size();
        content.lines_evicted = terminal.lines_evicted();
        content.cursor.state = terminal.cursor();
        content.shell_integration = true;
        content.shell_prompt_active = true;
        content.shell_name = Some("bash".into());
        content.shell_distro = Some("Ubuntu".into());
        content.shell_user = Some("alice".into());
        let mut pane = super::super::semantic_snapshot(
            content,
            (10.0, 24.0),
            rio_backend::config::layout::Margin {
                left: 4.0,
                top: 8.0,
                right: 0.0,
                bottom: 0.0,
            },
            1.0,
            true,
            (1, 0),
        );
        // Freeze only the clock fields; prompt identities, blank rows, and
        // result ownership must come from the parser, never fixture injection.
        for result in &mut pane.command_results {
            result.elapsed_ms = Some(21);
            result.completed_at = Some(
                rio_backend::crosswords::grid::row::SemanticCommandTimestamp {
                    unix_ms: 1767268800000,
                    year: 2026,
                    month: 1,
                    day: 1,
                    hour: 12,
                    minute: 0,
                    second: 0,
                },
            );
        }
        pane
    }

    #[test]
    fn narrow_prompt_band_cannot_give_all_its_width_to_the_timestamp() {
        let mut terminal = Crosswords::new(
            CrosswordsSize::new(80, 16),
            rio_backend::ansi::CursorShape::Block,
            VoidListener {},
            WindowId::from(0),
            0,
            256,
        );
        let stream = b"\x1b]133;A;aid=1\x07 \r\n\x1b]133;P;k=c;aid=1\x07/work\r\n\x1b]133;P;k=c;aid=1\x07lambda one\x1b]133;B\x07\r\n\x1b]133;C\x07output\r\n\x1b]133;D;0\x07\x1b]133;A;aid=2\x07 \r\n\x1b]133;P;k=c;aid=2\x07/work\r\n\x1b]133;P;k=c;aid=2\x07lambda \x1b]133;B\x07";
        let mut parser = Processor::default();
        for chunk in stream.chunks(3) {
            parser.advance(&mut terminal, chunk);
        }
        let mut content = RenderableContent::default();
        let mut status = devops_status::DevOpsStatus::default();
        let mut text = rio_backend::sugarloaf::text::Text::new(&fonts());
        text.init_cpu();
        let mut original_pixels: Option<Vec<u32>> = None;
        for cols in [80, 24, 12, 24, 80] {
            terminal.resize(CrosswordsSize::new(cols, 16));
            let mut pane = snapshot(&mut terminal, &mut content);
            assert_eq!(pane.command_results.len(), 1);
            status.prepare_prompt_rows(
                &pane.session,
                true,
                &pane.historical_anchors,
                pane.live_anchor,
            );
            let source = content.visible_rows.clone();
            let cursor = terminal.grid.cursor.pos;
            let history = terminal.history_size();
            let (_, paints) = prepare(&mut pane, Some(&status), &mut content, &mut text);
            assert_eq!(terminal.grid.cursor.pos, cursor);
            assert_eq!(terminal.history_size(), history);
            assert_eq!(content.visible_rows, source);
            let anchor = pane.live_anchor.unwrap();
            let expected = status.segments_for_prompt(1, &anchor);
            assert!(!expected.is_empty());
            for (item, segment) in expected.iter().enumerate() {
                let restored: String = paints
                    .iter()
                    .filter(|(a, f)| a.generation == anchor.generation && f.item == item)
                    .map(|(_, f)| &segment.value[f.bytes.clone()])
                    .collect();
                assert_eq!(
                    restored, segment.value,
                    "all context survives at {cols} columns"
                );
            }
            let completion = &pane.completion_labels[0];
            let restored: String = completion
                .fragments
                .iter()
                .map(|f| &completion.text[f.bytes.clone()])
                .collect();
            assert_eq!(restored, completion.text);
            assert!(restored.contains("2026-01-01 12:00:00"));
            let mut rectangles = Vec::new();
            for (anchor, fragment) in &paints {
                rectangles.push((
                    anchor.y + fragment.row as f32 * 24.0,
                    fragment.x,
                    fragment.width,
                ));
            }
            for fragment in &completion.fragments {
                rectangles.push((
                    completion.anchor.y + fragment.row as f32 * 24.0,
                    fragment.x,
                    fragment.width,
                ));
            }
            for (index, &(y, x, width)) in rectangles.iter().enumerate() {
                assert!(x >= 0.0 && x + width <= cols as f32 * 10.0 - 12.0 + 0.001);
                for &(other_y, other_x, other_width) in &rectangles[index + 1..] {
                    assert!(
                        y != other_y
                            || x + width <= other_x
                            || other_x + other_width <= x
                    );
                }
            }
            assert_eq!(content.command_rows.expanded(), cols < 80);
            if cols == 24 {
                let origins: Vec<_> =
                    (0..8).map(|row| content.command_rows.origin(row)).collect();
                // Two context chips fit the first line; the complete status,
                // date and time require two further lines at this font size.
                assert_eq!(origins, [0, 1, 2, 3, 4, 7, 8, 9]);
            }

            text.clear();
            let metrics = devops_status::prompt_tag_metrics(24.0);
            for (anchor, fragment) in &paints {
                let value = &status.segments_for_prompt(1, anchor)[fragment.item].value
                    [fragment.bytes.clone()];
                draw_fragment_text(
                    &mut text,
                    value,
                    fragment,
                    [anchor.x, anchor.y],
                    [24.0, metrics.font_size, metrics.height],
                    [80, 210, 240, 255],
                );
            }
            for fragment in &completion.fragments {
                draw_fragment_text(
                    &mut text,
                    &completion.text[fragment.bytes.clone()],
                    fragment,
                    [completion.anchor.x, completion.anchor.y],
                    [24.0, metrics.font_size, metrics.height],
                    [90, 240, 150, 255],
                );
            }
            assert!(text.instance_count() > 0);
            let width = cols as u32 * 10 + 8;
            let height = 16 * 24 + 16;
            for glyph in text.instances() {
                let left = glyph.pos[0] + glyph.bearings[0] as f32;
                let top = glyph.pos[1] + glyph.bearings[1] as f32;
                assert!(
                    left >= 4.0
                        && left + glyph.glyph_size[0] as f32 <= width as f32 - 4.0,
                    "metadata ink fits pane at {cols} columns"
                );
                assert!(
                    top >= 8.0 && top + glyph.glyph_size[1] as f32 <= height as f32 - 8.0
                );
            }
            let mut pixels = projected_grid_pixels(&content, width, height);
            let grid_pixels = pixels.clone();
            text.render_cpu_base(&mut pixels, width, height);
            // Standalone Text has no window frame finalizer. Paint both
            // partitions so a nonempty draw queue must produce real pixels.
            text.render_cpu_modal(&mut pixels, width, height);
            assert!(
                pixels.iter().zip(&grid_pixels).any(|(a, b)| a != b),
                "metadata reaches the actual grid frame"
            );
            if cols == 80 {
                if let Some(original) = &original_pixels {
                    assert_eq!(
                        pixels.iter().zip(original).filter(|(a, b)| a != b).count(),
                        0,
                        "exact metadata pixels after restore"
                    );
                } else {
                    original_pixels = Some(pixels.clone());
                }
            }
            if cols == 24 {
                if let Some(path) = std::env::var_os("AUTOMEXIA_COMMAND_INFO_PREVIEW") {
                    image_rs::RgbImage::from_fn(width, height, |x, y| {
                        let pixel = pixels[(y * width + x) as usize];
                        image_rs::Rgb([
                            (pixel >> 16) as u8,
                            (pixel >> 8) as u8,
                            pixel as u8,
                        ])
                    })
                    .save(path)
                    .expect("fictional command information preview");
                }
            }
        }
    }
}
