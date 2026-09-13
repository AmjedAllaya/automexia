use super::*;
use rio_backend::crosswords::grid::row::SemanticPrompt;
use rio_backend::crosswords::grid::Scroll;
use rio_backend::crosswords::pos::{Column, Line, Pos, Side};
use rio_backend::crosswords::{Crosswords, CrosswordsSize};
use rio_backend::event::{TerminalDamage, VoidListener, WindowId};
use rio_backend::performer::handler::Processor;
use rio_backend::selection::{Selection, SelectionType};

fn project(
    terminal: &mut Crosswords<VoidListener>,
    cell: f32,
) -> Vec<CommandResultAnchor> {
    let mut visible = Vec::new();
    let mut styles = Vec::new();
    let mut extras = rustc_hash::FxHashMap::default();
    terminal.snapshot_visible(
        &TerminalDamage::Full,
        terminal.columns(),
        &mut visible,
        &mut styles,
        &mut extras,
    );
    let prompts = visible
        .iter()
        .enumerate()
        .filter_map(|(index, row)| {
            (row.semantic_prompt == SemanticPrompt::Prompt)
                .then(|| crate::renderer::prompt_visual_anchor(&visible, index))
                .flatten()
                .map(|index| crate::automexia::ui::PromptAnchor {
                    generation: row.semantic_prompt_id,
                    key: index as u64,
                    x: 4.0,
                    y: index as f32 * cell,
                    width: terminal.columns() as f32 * 10.0,
                    height: cell,
                })
        })
        .collect::<Vec<_>>();
    crate::renderer::command_result_anchors(
        &visible,
        4.0,
        0.0,
        terminal.columns() as f32 * 10.0,
        cell,
        &prompts,
    )
}

#[test]
fn parser_tables_keep_copy_and_row_gaps_across_resize_and_scrollback() {
    let tables = [
        "NAME          READY   STATUS    RESTARTS   AGE\nexample-api   1/1     Running   0          2d\nexample-job   1/1     Running   1          1d",
        "CONTAINER ID   IMAGE       STATUS\n123abc         demo:v1     Up 2 hours\n456def         demo:v2     Up 3 hours",
        "ITEM          VALUE       NOTE\nexample       123         aligned columns\nother         456         Unicode 界 e\u{301}",
    ];
    for table in tables {
        let mut terminal = Crosswords::new(
            CrosswordsSize::new(84, 14),
            rio_backend::ansi::CursorShape::Block,
            VoidListener {},
            WindowId::from(0),
            0,
            1024,
        );
        let mut processor = Processor::default();
        let stream = format!(
            "\x1b]133;A;aid=41\x07 \r\n\x1b]133;P;k=c;aid=41\x07/example\r\n\x1b]133;P;k=c;aid=41\x07λ example\x1b]133;B\x07\r\n\x1b]133;C\x07{}\r\n\x1b]133;D;0\x07\x1b]133;A;aid=42\x07 \r\n\x1b]133;P;k=c;aid=42\x07/example\r\n\x1b]133;P;k=c;aid=42\x07λ \x1b]133;B\x07",
            table.replace('\n', "\r\n"),
        );
        // Fragment control sequences and Unicode as a PTY reader may do. The
        // fixture never injects the semantic anchors whose creation is tested.
        for chunk in stream.as_bytes().chunks(7) {
            processor.advance(&mut terminal, chunk);
        }
        let mut selection = Selection::new(
            SelectionType::Simple,
            Pos::new(Line(3), Column(0)),
            Side::Left,
        );
        let last_column =
            unicode_width::UnicodeWidthStr::width(table.lines().last().unwrap()) - 1;
        selection.update(Pos::new(Line(5), Column(last_column)), Side::Right);
        terminal.selection = Some(selection);
        assert_eq!(terminal.selection_to_string().as_deref(), Some(table));

        let anchors = project(&mut terminal, 20.0);
        let anchor = anchors
            .iter()
            .find(|anchor| anchor.generation == Some(41))
            .unwrap();
        assert_eq!(anchor.output_top, Some(60.0));
        assert_eq!(anchor.y, 120.0);
        let visual = command_result_visual(anchor).unwrap();
        let bands = rows::surfaces(visual.surface, 20.0).collect::<Vec<_>>();
        assert_eq!(bands.len(), 3);
        assert_eq!(
            bands.iter().map(|band| band[1]).collect::<Vec<_>>(),
            vec![61.0, 81.0, 101.0]
        );

        // Keep the original selection while output wraps and its owner moves
        // offscreen. No re-selection may hide changed or inserted copy bytes.
        for (columns, lines) in [(25, 9), (84, 14), (42, 7), (96, 16)] {
            terminal.resize(CrosswordsSize::new(columns, lines));
            for scroll in [Scroll::Top, Scroll::Bottom] {
                terminal.scroll_display(scroll);
                for cell in [12.0, 20.0, 31.25] {
                    let cursor = terminal.cursor();
                    for anchor in project(&mut terminal, cell) {
                        if let Some(visual) = command_result_visual(&anchor) {
                            let bands =
                                rows::surfaces(visual.surface, cell).collect::<Vec<_>>();
                            assert!(!bands.is_empty());
                            assert!(bands
                                .windows(2)
                                .all(|pair| pair[0][1] + pair[0][3] < pair[1][1]));
                            assert!(bands
                                .iter()
                                .all(|band| band[1] + band[3] < anchor.y));
                        }
                    }
                    assert_eq!(terminal.cursor(), cursor);
                    assert_eq!(terminal.selection_to_string().as_deref(), Some(table));
                }
            }
        }
    }
}

fn command_marker_terminal() -> Crosswords<VoidListener> {
    let mut terminal = Crosswords::new(
        CrosswordsSize::new(84, 20),
        rio_backend::ansi::CursorShape::Block,
        VoidListener {},
        WindowId::from(0),
        0,
        1024,
    );
    let mut processor = Processor::default();
    // Consecutive OSC lifecycles, not injected anchors. A silent command must
    // not borrow its predecessor's output surface.
    for (id, output, exit) in [
        (41, "first output\r\n", 0),
        (42, "failed output\r\n", 1),
        (43, "", 0),
    ] {
        let stream = format!(
            "\x1b]133;A;aid={id}\x07 \r\n\x1b]133;P;k=c;aid={id}\x07/example\r\n\x1b]133;P;k=c;aid={id}\x07λ example\x1b]133;B\x07\r\n\x1b]133;C\x07{output}\x1b]133;D;{exit}\x07"
        );
        for chunk in stream.as_bytes().chunks(3) {
            processor.advance(&mut terminal, chunk);
        }
    }
    processor.advance(
        &mut terminal,
        b"\x1b]133;A;aid=44\x07 \r\n\x1b]133;P;k=c;aid=44\x07lambda ",
    );
    terminal
}

#[test]
fn parser_command_markers_cannot_look_like_pane_dividers() {
    let mut terminal = command_marker_terminal();
    let anchors = project(&mut terminal, 20.0);
    assert!(anchors.iter().any(|a| a.generation == Some(41)));
    assert!(anchors.iter().any(|a| a.generation == Some(42)));
    for anchor in anchors {
        if let Some([x, y, width, height]) = command_result_divider(&anchor) {
            assert_eq!(
                [x, y, width, height],
                [anchor.x + 12.0, anchor.y + 1.5, 48.0, 1.0]
            );
            assert!(width <= anchor.width * 0.25, "command marker spans pane");
            let expected_top = match anchor.generation {
                Some(41) => 81.5,
                Some(42) => 161.5,
                Some(43) => 221.5,
                _ => panic!("unexpected command marker owner"),
            };
            assert_marker_pixels([x, y, width, height], expected_top);
        }
    }
    for (columns, lines) in [(24, 8), (84, 20), (40, 10), (84, 20)] {
        terminal.resize(CrosswordsSize::new(columns, lines));
        for forward in [false, false, true, true, true] {
            let before = terminal.grid.display_offset();
            let moved = terminal.scroll_to_prompt(forward);
            assert_eq!(moved, terminal.grid.display_offset() != before);
            let cursor = terminal.cursor();
            let anchors = project(&mut terminal, 20.0);
            let mut owners = std::collections::HashSet::new();
            for anchor in anchors {
                assert!(owners.insert((anchor.generation, anchor.key)));
                if let Some([x, y, w, h]) = command_result_divider(&anchor) {
                    assert!(x > anchor.x && x + w < anchor.x + anchor.width);
                    assert!(w <= 48.0 && w <= anchor.width * 0.25);
                    assert!(y >= anchor.y && y + h <= anchor.y + anchor.height);
                }
            }
            assert_eq!(terminal.cursor(), cursor);
        }
    }
}

fn assert_marker_pixels(marker: [f32; 4], expected_top: f32) {
    // Exact controlled geometry raster, not native GPU evidence. The literal
    // expected ranges are independent of boundary_marker and reject full width.
    for scale in [1.0, 1.25, 2.0, 3.0, 4.0] {
        let width = (900.0 * scale) as usize;
        let height = (400.0 * scale) as usize;
        let mut actual = vec![0x030d15ffu32; width * height];
        let [x, y, w, h] = marker;
        for py in (y * scale).round() as usize..((y + h) * scale).round() as usize {
            for px in (x * scale).round() as usize..((x + w) * scale).round() as usize {
                actual[py * width + px] = 0x28f597ff;
            }
        }
        let mut expected = vec![0x030d15ffu32; width * height];
        // Literal lifecycle rows include the silent command's status marker;
        // it has no output surface but still receives completion feedback.
        for py in (expected_top * scale).round() as usize
            ..((expected_top + 1.0) * scale).round() as usize
        {
            for px in (16.0 * scale).round() as usize..(64.0 * scale).round() as usize {
                expected[py * width + px] = 0x28f597ff;
            }
        }
        assert!(
            actual == expected,
            "marker {marker:?} pixel coverage at scale {scale}"
        );
        expected[0] ^= 1;
        assert!(
            actual != expected,
            "one-channel drift must fail exact comparison"
        );
    }
}

#[test]
fn command_marker_controlled_preview() {
    use rio_backend::sugarloaf::{
        font::{constants, FontData, FontLibrary, FontLibraryData},
        text::Text,
    };
    use std::sync::Arc;

    let mut terminal = command_marker_terminal();
    let mut data = FontLibraryData::default();
    data.insert(FontData::from_static_slice(constants::FONT_CASCADIA_CODE_NF).unwrap());
    let fonts = FontLibrary {
        inner: Arc::new(parking_lot::RwLock::new(data)),
    };
    let mut text = Text::new(&fonts);
    text.init_cpu();
    let mut pixels = vec![0x00030d15u32; 900 * 320];
    let mut fill = |[x, y, w, h]: [f32; 4], color| {
        for py in (y.round() as usize).min(320)..((y + h).round() as usize).min(320) {
            for px in (x.round() as usize).min(900)..((x + w).round() as usize).min(900) {
                pixels[py * 900 + px] = color;
            }
        }
    };
    // Draw-only specimen from parsed lifecycles. Freeze label time and resting
    // animation, use fictional output, and never capture desktop/private data.
    for anchor in project(&mut terminal, 20.0) {
        let failure = anchor.exit_code == Some(1);
        if let Some(visual) = command_result_visual(&anchor) {
            for band in rows::surfaces(visual.surface, 20.0) {
                fill(band, if failure { 0x0020111b } else { 0x00092521 });
            }
        }
        if let Some(marker) = command_result_divider(&anchor) {
            fill(marker, if failure { 0x00802e46 } else { 0x00007850 });
        }
        let presentation = command_result_presentation(
            anchor.exit_code,
            Some(72),
            Some("2026-01-01 12:00:00"),
        );
        let opts = DrawOpts {
            font_size: 12.4,
            color: if failure {
                [255, 95, 124, 255]
            } else {
                [40, 245, 151, 255]
            },
            ..DrawOpts::default()
        };
        let (label, width) = fitting_command_result_label(&presentation, |label| {
            let width = text.measure(label, &opts);
            (width <= result_label_maximum_width(anchor.width)).then_some(width)
        })
        .unwrap();
        text.draw(
            anchor.x + anchor.width - width - 10.0,
            anchor.y + 5.0,
            label,
            &opts,
        );
    }
    // Structural reference stays continuous and separate from the short marks.
    fill([4.0, 300.0, 840.0, 2.0], 0x001589ab);
    let mut visible = Vec::new();
    terminal.snapshot_visible(
        &TerminalDamage::Full,
        84,
        &mut visible,
        &mut Vec::new(),
        &mut rustc_hash::FxHashMap::default(),
    );
    for (row, cells) in visible.iter().enumerate() {
        let label = cells.inner.iter().map(|cell| cell.c()).collect::<String>();
        text.draw(
            4.0,
            row as f32 * 20.0,
            label.trim_end(),
            &DrawOpts {
                font_size: 14.0,
                color: [211, 228, 237, 255],
                ..DrawOpts::default()
            },
        );
    }
    text.render_cpu_base(&mut pixels, 900, 320);
    text.render_cpu_modal(&mut pixels, 900, 320);
    assert!(pixels.contains(&0x001589ab));
    if let Some(path) = std::env::var_os("AUTOMEXIA_COMMAND_MARKER_PREVIEW") {
        image_rs::RgbImage::from_fn(900, 320, |x, y| {
            let p = pixels[y as usize * 900 + x as usize];
            image_rs::Rgb([(p >> 16) as u8, (p >> 8) as u8, p as u8])
        })
        .save(path)
        .expect("controlled command marker preview");
    }
}

#[test]
fn command_marker_scales_without_becoming_a_structural_border() {
    for (width, expected_x, expected_width) in [
        (4.0, 4.4, 1.0),
        (40.0, 8.0, 10.0),
        (100.0, 14.0, 25.0),
        (191.0, 16.0, 47.75),
        (192.0, 16.0, 48.0),
        (193.0, 16.0, 48.0),
        (8192.0, 16.0, 48.0),
    ] {
        assert_eq!(
            rows::boundary_marker([4.0, 40.0, width, 20.0]),
            Some([expected_x, 41.5, expected_width, 1.0])
        );
    }
    for row in [2.0, 12.0, 19.5, 20.0, 31.25, 80.0, 128.0] {
        for x in [-120.5, 0.0, 360.25, 8192.0] {
            let [mx, my, mw, mh] = rows::boundary_marker([x, 7.25, 100.5, row]).unwrap();
            assert!(mx > x && mx + mw < x + 100.5);
            assert!(my >= 7.25 && my + mh <= 7.25 + row);
        }
    }
}

#[test]
fn command_marker_rejects_invalid_or_unpaintable_geometry() {
    let anchor = CommandResultAnchor {
        generation: Some(1),
        key: 1,
        x: 4.0,
        y: 80.0,
        width: 720.0,
        height: 20.0,
        output_top: Some(40.0),
        separates_next_prompt: true,
        exit_code: None,
        elapsed_ms: None,
        completed_at: None,
    };
    for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        for component in 0..4 {
            let mut invalid = anchor;
            match component {
                0 => invalid.x = bad,
                1 => invalid.y = bad,
                2 => invalid.width = bad,
                _ => invalid.height = bad,
            }
            assert_eq!(command_result_divider(&invalid), None);
        }
    }
    for small in [-1.0, 0.0, 0.5, 1.0] {
        assert_eq!(
            command_result_divider(&CommandResultAnchor {
                width: small,
                ..anchor
            }),
            None
        );
        assert_eq!(
            command_result_divider(&CommandResultAnchor {
                height: small,
                ..anchor
            }),
            None
        );
    }
    assert_eq!(
        command_result_divider(&CommandResultAnchor {
            x: f32::MAX,
            width: f32::MAX,
            ..anchor
        }),
        None
    );
}

#[test]
fn output_rows_leave_literal_two_pixel_gaps_without_moving_cells() {
    // Three 20 px terminal rows; the existing prompt gutter clips the last one.
    assert_eq!(
        rows::surfaces([6.0, 40.0, 716.0, 52.0], 20.0).collect::<Vec<_>>(),
        vec![
            [6.0, 41.0, 716.0, 18.0],
            [6.0, 61.0, 716.0, 18.0],
            [6.0, 81.0, 716.0, 11.0],
        ]
    );
}

#[test]
fn output_rows_reject_invalid_geometry_and_bound_work() {
    for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        for component in 0..4 {
            let mut surface = [0.0, 0.0, 100.0, 60.0];
            surface[component] = bad;
            assert_eq!(rows::surfaces(surface, 20.0).count(), 0);
        }
        assert_eq!(rows::surfaces([0.0, 0.0, 100.0, 60.0], bad).count(), 0);
    }
    for height in [-1.0, 0.0, 0.5] {
        assert_eq!(rows::surfaces([0.0, 0.0, 100.0, 60.0], height).count(), 0);
    }
    for height in [-1.0, 0.0, 8193.0, f32::MAX] {
        assert_eq!(rows::surfaces([0.0, 0.0, 100.0, height], 1.0).count(), 0);
    }
    assert_eq!(rows::surfaces([0.0, 0.0, 0.0, 20.0], 20.0).count(), 0);
    assert_eq!(rows::surfaces([0.0, 0.0, 100.0, 8192.0], 1.0).count(), 8192);
    assert_eq!(
        rows::surfaces([f32::MAX, 0.0, f32::MAX, 20.0], 20.0).count(),
        0
    );
}

#[test]
fn output_rows_keep_fractional_grid_phase_and_clip_both_edges() {
    for cell in [1.0, 4.0, 12.0, 19.5, 20.0, 31.25, 80.0, 128.0] {
        for top in [-20.5, 0.0, 7.25] {
            for count in [1, 2, 40, 257] {
                let surface = [4.25, top, 100.5, cell * count as f32 - cell * 0.4];
                let bands = rows::surfaces(surface, cell).collect::<Vec<_>>();
                assert_eq!(bands.len(), count);
                let mut previous_bottom = top;
                for (index, [x, y, width, height]) in bands.into_iter().enumerate() {
                    assert_eq!((x, width), (4.25, 100.5));
                    assert!(y > previous_bottom);
                    assert!(y >= top + index as f32 * cell);
                    assert!(y + height <= top + (index + 1) as f32 * cell);
                    assert!(y + height <= top + surface[3]);
                    previous_bottom = y + height;
                }
            }
        }
    }
}

#[test]
fn output_rows_match_independent_exact_pixel_coverage() {
    // Pixel-centre coverage is an independent, renderer-neutral geometry oracle,
    // not a claim about native GPU antialiasing or desktop presentation.
    let bands = rows::surfaces([4.0, 4.0, 88.0, 60.0], 20.0).collect::<Vec<_>>();
    for y in 0..70 {
        for x in 0..100 {
            let px = x as f32 + 0.5;
            let py = y as f32 + 0.5;
            let hits = bands
                .iter()
                .filter(|band| {
                    px >= band[0]
                        && px < band[0] + band[2]
                        && py >= band[1]
                        && py < band[1] + band[3]
                })
                .count();
            let expected = (4..92).contains(&x)
                && ((5..23).contains(&y)
                    || (25..43).contains(&y)
                    || (45..63).contains(&y));
            assert_eq!(hits, usize::from(expected), "coverage at ({x}, {y})");
        }
    }
}

#[test]
fn output_rows_preserve_gaps_for_every_result_tone_and_reduced_motion() {
    for exit_code in [None, Some(0), Some(1)] {
        let anchor = CommandResultAnchor {
            generation: Some(9),
            key: 1,
            x: 0.0,
            y: 80.0,
            width: 500.0,
            height: 20.0,
            output_top: Some(20.0),
            separates_next_prompt: true,
            exit_code,
            elapsed_ms: None,
            completed_at: None,
        };
        for animated in [false, true] {
            let now = Instant::now();
            let mut pulse = CommandResultPulse::default();
            pulse.observe(&[], animated, false, now);
            pulse.observe(&[anchor], animated, false, now);
            assert_eq!(pulse.alpha_for(&anchor, now) > 0.0, animated);
            let visual = command_result_visual(&anchor).unwrap();
            let bands = rows::surfaces(visual.surface, anchor.height).collect::<Vec<_>>();
            assert_eq!(bands.len(), 3);
            assert!(bands
                .windows(2)
                .all(|pair| pair[0][1] + pair[0][3] < pair[1][1]));
        }
    }
}

#[test]
fn output_rows_controlled_preview() {
    use rio_backend::sugarloaf::{
        font::{constants, FontData, FontLibrary, FontLibraryData},
        text::Text,
    };
    use std::sync::Arc;

    let mut data = FontLibraryData::default();
    data.insert(FontData::from_static_slice(constants::FONT_CASCADIA_CODE_NF).unwrap());
    let fonts = FontLibrary {
        inner: Arc::new(parking_lot::RwLock::new(data)),
    };
    let mut text = Text::new(&fonts);
    text.init_cpu();
    let width = 704usize;
    let height = 164usize;
    let mut pixels = vec![0x00030d15u32; width * height];
    let bands = rows::surfaces([8.0, 28.0, 688.0, 120.0], 24.0).collect::<Vec<_>>();
    for y in 0..height {
        for x in 0..width {
            if bands.iter().any(|band| {
                let px = x as f32 + 0.5;
                let py = y as f32 + 0.5;
                px >= band[0]
                    && px < band[0] + band[2]
                    && py >= band[1]
                    && py < band[1] + band[3]
            }) {
                pixels[y * width + x] = 0x00082520;
            }
        }
    }
    let labels = [
        "NAME                 READY    STATUS     RESTARTS    AGE",
        "example-api          1/1      Running    0           2d",
        "example-worker       1/1      Running    0           2d",
        "example-cache        1/1      Running    1           1d",
        "example-gateway      1/1      Running    0           1d",
    ];
    for (row, label) in labels.iter().enumerate() {
        let options = DrawOpts {
            font_size: 15.0,
            color: if row == 0 {
                [0, 224, 240, 255]
            } else {
                [40, 245, 151, 255]
            },
            ..DrawOpts::default()
        };
        text.draw(12.0, 30.0 + row as f32 * 24.0, label, &options);
    }
    text.render_cpu_base(&mut pixels, width as u32, height as u32);
    text.render_cpu_modal(&mut pixels, width as u32, height as u32);
    assert!(pixels
        .iter()
        .any(|pixel| *pixel != 0x00030d15 && *pixel != 0x00082520));
    // Opt-in fictional-data artifact: normal test runs do not write screenshots.
    if let Some(path) = std::env::var_os("AUTOMEXIA_TABLE_PREVIEW") {
        let image = image_rs::RgbImage::from_fn(width as u32, height as u32, |x, y| {
            let pixel = pixels[y as usize * width + x as usize];
            image_rs::Rgb([(pixel >> 16) as u8, (pixel >> 8) as u8, pixel as u8])
        });
        image
            .save(path)
            .expect("controlled table preview writes successfully");
    }
}
