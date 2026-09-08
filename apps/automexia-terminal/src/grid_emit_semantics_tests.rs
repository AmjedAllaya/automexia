use super::*;
use rio_backend::config::Config;
use rio_backend::crosswords::pos::Side;
use rio_backend::crosswords::{Crosswords, CrosswordsSize};
use rio_backend::event::{TerminalDamage, VoidListener, WindowId};
use rio_backend::performer::handler::Processor;
use rio_backend::selection::{Selection, SelectionType};
use rio_backend::sugarloaf::font::{constants, FontData, FontLibraryData};
use rio_backend::sugarloaf::grid::{cpu::CpuGridRenderer, GridUniforms};
use std::sync::Arc;

fn terminal(text: &str) -> Crosswords<VoidListener> {
    let mut terminal = Crosswords::new(
        CrosswordsSize::new(80, 12),
        rio_backend::ansi::CursorShape::Block,
        VoidListener {},
        WindowId::from(0),
        0,
        128,
    );
    let mut processor = Processor::default();
    for bytes in text.as_bytes().chunks(3) {
        processor.advance(&mut terminal, bytes);
    }
    terminal
}

fn snapshot(
    terminal: &mut Crosswords<VoidListener>,
) -> (Vec<Row<Square>>, Vec<Style>, ExtrasMap) {
    let mut rows = Vec::new();
    let mut styles = Vec::new();
    let mut extras = ExtrasMap::default();
    terminal.snapshot_visible(
        &TerminalDamage::Full,
        terminal.columns(),
        &mut rows,
        &mut styles,
        &mut extras,
    );
    (rows, styles, extras)
}

fn fixture_renderer(enabled: bool) -> Renderer {
    let mut config = Config::default();
    config.colors.cyan = [0.0, 1.0, 1.0, 1.0];
    config.colors.green = [0.0, 1.0, 0.0, 1.0];
    config.colors.yellow = [1.0, 1.0, 0.0, 1.0];
    config.colors.red = [1.0, 0.0, 0.0, 1.0];
    config.colors.foreground = [1.0; 4];
    config.colors.white = [0.4, 0.4, 0.4, 1.0];
    config.colors.light_white = [0.8, 0.8, 0.8, 1.0];
    let mut renderer = Renderer::new(&config);
    // Freeze the already-resolved palette without changing process-wide theme
    // environment. Theme selection itself is outside this colour-delivery test.
    renderer.named_colors = config.colors;
    renderer.colors = rio_backend::config::colors::term::List::from(&config.colors);
    renderer.devops_enabled = enabled;
    renderer
}

#[test]
fn parsed_statuses_reach_grid_glyphs_and_exact_cpu_pixels() {
    let cases = [
        ("NAME READY STATUS", [0, 255, 255, 255]),
        ("batch 0/1 Completed", [0, 255, 255, 255]),
        ("pod/api 1/1 Running", [0, 255, 0, 255]),
        ("api 0/1 Running", [255, 255, 0, 255]),
        ("api 0/1 CrashLoopBackOff", [255, 0, 0, 255]),
        ("worker Exited (0) 1 minute ago", [0, 255, 255, 255]),
        ("web Up 2 minutes (Paused)", [255, 255, 0, 255]),
        ("web Up 2 minutes (healthy)", [0, 255, 0, 255]),
        ("api 0/1 Init:Error", [255, 0, 0, 255]),
    ];
    let source = cases
        .iter()
        .map(|(text, _)| *text)
        .collect::<Vec<_>>()
        .join("\n");
    let mut terminal = terminal(&source.replace('\n', "\r\n"));
    let mut selection = Selection::new(
        SelectionType::Simple,
        Pos::new(Line(0), Column(0)),
        Side::Left,
    );
    selection.update(Pos::new(Line(8), Column(cases[8].0.len() - 1)), Side::Right);
    terminal.selection = Some(selection);
    let cursor = terminal.cursor();
    assert_eq!(
        terminal.selection_to_string().as_deref(),
        Some(source.as_str())
    );
    let (rows, styles, extras) = snapshot(&mut terminal);
    let mut data = FontLibraryData::default();
    data.insert(FontData::from_static_slice(constants::FONT_CASCADIA_CODE_NF).unwrap());
    let fonts = FontLibrary {
        inner: Arc::new(parking_lot::RwLock::new(data)),
    };
    let renderer = fixture_renderer(true);
    for scale in [1.0, 1.25, 2.0] {
        let mut grid = GridRenderer::Cpu(CpuGridRenderer::new(80, 12));
        let mut rasterizer = GridGlyphRasterizer::new();
        let mut issued = Vec::new();
        let mut glyphs = Vec::new();
        for (y, (_, expected)) in cases.iter().enumerate() {
            build_row_fg(
                &rows[y],
                80,
                y as u16,
                &styles,
                &extras,
                &renderer,
                &TermColors::default(),
                &mut rasterizer,
                &mut grid,
                16.0 * scale,
                10.0 * scale,
                24.0 * scale,
                None,
                &[],
                &fonts,
                0,
                None,
                &mut glyphs,
            );
            assert!(!glyphs.is_empty(), "real shaped row must emit glyphs");
            assert!(
                glyphs.iter().all(|glyph| glyph.color == *expected),
                "wrong colour on row {y}"
            );
            grid.write_row(y as u32, &[], &glyphs);
            issued.push(glyphs.clone());
        }
        let width = (800.0 * scale) as u32;
        let height = (288.0 * scale) as u32;
        let uniforms = GridUniforms {
            projection: [0.0; 16],
            grid_padding: [0.0; 4],
            cursor_color: [0.0; 4],
            cursor_bg_color: [0.0; 4],
            cell_size: [10.0 * scale, 24.0 * scale],
            grid_size: [80, 12],
            cursor_pos: [u32::MAX; 2],
            _pad_cursor: [0; 2],
            min_contrast: 0.0,
            flags: 0,
            padding_extend: 0,
            input_colorspace: 0,
        };
        let mut actual = vec![0x00081218; (width * height) as usize];
        grid.render_text_cpu(&mut actual, width, height, &uniforms);
        // Literal colour oracle, independent of severity mapping. Font geometry
        // is shared intentionally; this checks colour delivery, not font shaping.
        for (y, glyphs) in issued.iter_mut().enumerate() {
            for glyph in glyphs.iter_mut() {
                glyph.color = cases[y].1;
            }
            grid.write_row(y as u32, &[], glyphs);
        }
        let mut expected = vec![0x00081218; actual.len()];
        grid.render_text_cpu(&mut expected, width, height, &uniforms);
        assert_eq!(actual, expected, "zero-tolerance status pixels");
        for color in [0x0000ffff, 0x0000ff00, 0x00ffff00, 0x00ff0000] {
            assert!(actual.contains(&color));
        }
        if scale == 1.0 {
            if let Some(path) = std::env::var_os("AUTOMEXIA_STATUS_PREVIEW") {
                image_rs::RgbImage::from_fn(width, height, |x, y| {
                    let pixel = actual[(y * width + x) as usize];
                    image_rs::Rgb([(pixel >> 16) as u8, (pixel >> 8) as u8, pixel as u8])
                })
                .save(path)
                .expect("fictional status preview");
            }
        }
    }
    assert_eq!(terminal.cursor(), cursor);
    assert_eq!(
        terminal.selection_to_string().as_deref(),
        Some(source.as_str())
    );
}

#[test]
fn semantic_colours_preserve_explicit_ansi_and_disabled_mode() {
    for enabled in [false, true] {
        let renderer = fixture_renderer(enabled);
        let mut terminal = terminal(
            "batch 0/1 Completed\r\n\x1b[38;2;180;120;240mbatch 0/1 Completed\x1b[0m\r\n\x1b[37mbatch 0/1 Completed\x1b[0m\r\n\x1b[97mbatch 0/1 Completed\x1b[0m",
        );
        let (rows, styles, _) = snapshot(&mut terminal);
        for (y, row) in rows.iter().take(4).enumerate() {
            let semantic = semantic_row_fg(row, 80, &renderer, &mut String::new());
            assert_eq!(semantic, enabled.then_some([0, 255, 255, 255]));
            let square = row[Column(0)];
            let actual = semantic_or_cell_fg(
                semantic,
                square,
                resolve_style(&styles, square),
                &renderer,
                &TermColors::default(),
            );
            let expected = if y == 1 {
                [180, 120, 240, 255]
            } else if y == 2 {
                [102, 102, 102, 255]
            } else if y == 3 {
                [204, 204, 204, 255]
            } else if enabled {
                [0, 255, 255, 255]
            } else {
                [255; 4]
            };
            assert_eq!(actual, expected);
        }
    }
}

#[test]
fn wrapped_completion_status_never_turns_green() {
    let source = "batch 0/1 Completed 0 4h";
    // Completed command output ends before the next prompt. Keep that real
    // lifecycle distinct from an unfinished line edited by a live shell.
    let mut terminal = terminal(&format!("{source}\r\nlambda "));
    let mut selection = Selection::new(
        SelectionType::Simple,
        Pos::new(Line(0), Column(0)),
        Side::Left,
    );
    selection.update(Pos::new(Line(0), Column(source.len() - 1)), Side::Right);
    terminal.selection = Some(selection);
    let renderer = fixture_renderer(true);
    for columns in [10, 12, 20, 80] {
        terminal.resize(CrosswordsSize::new(columns, 12));
        let (rows, _, _) = snapshot(&mut terminal);
        let mut saw_info = false;
        for row in &rows {
            let color = semantic_row_fg(row, columns, &renderer, &mut String::new());
            assert_ne!(
                color,
                Some([0, 255, 0, 255]),
                "a completion fragment cannot mean healthy"
            );
            saw_info |= color == Some([0, 255, 255, 255]);
        }
        if columns == 80 {
            assert!(
                saw_info,
                "restored output must retain its completion colour"
            );
        }
        assert_eq!(terminal.selection_to_string().as_deref(), Some(source));
    }
}
