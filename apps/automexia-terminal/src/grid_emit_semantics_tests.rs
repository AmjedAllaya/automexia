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
fn resized_native_table_matches_independent_viewport_pixels() {
    use rio_backend::crosswords::ResizePolicy;
    let lines: Vec<_> = (1..=32).map(|index| format!(
        "ROW-{index:02}  -a---  2026-01-01 12:00:00  {index:04}  artifact-{index:02}-abcdefghijklmnopqrstuvwxyz0123456789.txt"
    )).collect();
    let create = |cols, rows| {
        Crosswords::new(
            CrosswordsSize::new(cols, rows),
            rio_backend::ansi::CursorShape::Block,
            VoidListener {},
            WindowId::from(0),
            0,
            2_000,
        )
    };
    let mut actual = create(100, 24);
    actual.set_resize_policy(ResizePolicy::Conpty);
    let mut parser = Processor::default();
    let source = format!(
        "{}\r\n\r\n/example\r\nlambda ",
        lines
            .iter()
            .map(|line| format!("{line:<100}"))
            .collect::<Vec<_>>()
            .join("\r\n")
    );
    for bytes in source.as_bytes().chunks(3) {
        parser.advance(&mut actual, bytes);
    }
    actual.resize(CrosswordsSize::new(16, 10));
    let repaint = format!("\x1b[H0123456789.txt  \r\n{}  \r\n\x1b[K\r\n/example        \r\nlambda\x1b[K\x1b[1C", lines[31]);
    for bytes in repaint.as_bytes().chunks(3) {
        parser.advance(&mut actual, bytes);
    }

    let mut data = FontLibraryData::default();
    data.insert(FontData::from_static_slice(constants::FONT_CASCADIA_CODE_NF).unwrap());
    let fonts = FontLibrary {
        inner: Arc::new(parking_lot::RwLock::new(data)),
    };
    let renderer = fixture_renderer(false);
    let pixels = |terminal: &mut Crosswords<VoidListener>, scale: f32| {
        let cols = terminal.columns();
        let height = terminal.screen_lines();
        let (rows, styles, extras) = snapshot(terminal);
        let mut grid =
            GridRenderer::Cpu(CpuGridRenderer::new(cols as u32, height as u32));
        let mut rasterizer = GridGlyphRasterizer::new();
        let mut glyphs = Vec::new();
        for (y, row) in rows.iter().enumerate() {
            build_row_fg(
                row,
                cols,
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
            grid.write_row(y as u32, &[], &glyphs);
        }
        let width = (cols as f32 * 10.0 * scale) as u32;
        let height = (height as f32 * 24.0 * scale) as u32;
        let uniforms = GridUniforms {
            projection: [0.0; 16],
            grid_padding: [0.0; 4],
            cursor_color: [0.0; 4],
            cursor_bg_color: [0.0; 4],
            cell_size: [10.0 * scale, 24.0 * scale],
            grid_size: [cols as u32, rows.len() as u32],
            cursor_pos: [u32::MAX; 2],
            _pad_cursor: [0; 2],
            min_contrast: 0.0,
            flags: 0,
            padding_extend: 0,
            input_colorspace: 0,
        };
        let mut output = vec![0x00081218; (width * height) as usize];
        grid.render_text_cpu(&mut output, width, height, &uniforms);
        (output, width, height)
    };
    for (cols, rows, history) in [(16, 10, false), (100, 24, false), (100, 24, true)] {
        actual.resize(CrosswordsSize::new(cols, rows));
        if history {
            actual.scroll_display(rio_backend::crosswords::grid::Scroll::Top);
        }
        // Expected native viewport is literal observed text in a fresh grid,
        // never a second call to the reflow algorithm under test. Font shaping
        // is shared; this oracle checks restored layout and glyph delivery.
        let mut expected = create(cols, rows);
        let reference_text = if history {
            lines[..rows].join("\r\n")
        } else {
            format!("0123456789.txt\r\n{}\r\n\r\n/example\r\nlambda ", lines[31])
        };
        Processor::default().advance(&mut expected, reference_text.as_bytes());
        for scale in [1.0, 1.25, 2.0] {
            let (rendered, width, height) = pixels(&mut actual, scale);
            let (reference, _, _) = pixels(&mut expected, scale);
            let changed = rendered
                .iter()
                .zip(&reference)
                .filter(|(a, b)| a != b)
                .count();
            assert_eq!(
                changed, 0,
                "exact table pixels at {cols} columns, scale {scale}, history {history}"
            );
            assert!(
                rendered.iter().any(|pixel| *pixel != 0x00081218),
                "nonempty glyph oracle"
            );
            if history && scale == 1.0 {
                if let Some(path) = std::env::var_os("AUTOMEXIA_RESIZE_PREVIEW") {
                    image_rs::RgbImage::from_fn(width, height, |x, y| {
                        let pixel = rendered[(y * width + x) as usize];
                        image_rs::Rgb([
                            (pixel >> 16) as u8,
                            (pixel >> 8) as u8,
                            pixel as u8,
                        ])
                    })
                    .save(path)
                    .expect("fictional resize preview");
                }
            }
        }
    }
    assert_native_seam_pixels(pixels);
    assert_extreme_resize_pixels(pixels);
}

fn assert_extreme_resize_pixels(
    pixels: impl Fn(&mut Crosswords<VoidListener>, f32) -> (Vec<u32>, u32, u32),
) {
    let create = || {
        Crosswords::new(
            CrosswordsSize::new(146, 16),
            rio_backend::ansi::CursorShape::Block,
            VoidListener {},
            WindowId::from(0),
            0,
            2_000,
        )
    };
    let lines: Vec<_> = (1..=14).map(|index| format!(
        "ROW-{index:02}  folder-{index:02}                          document-{index:02}.txt"
    )).collect();
    let mut actual = create();
    actual.set_resize_policy(rio_backend::crosswords::ResizePolicy::Conpty);
    let mut parser = Processor::default();
    parser.advance(
        &mut actual,
        format!("{}\r\n\r\n/example\r\nlambda ", lines.join("\r\n")).as_bytes(),
    );
    for (cols, rows) in [(2, 24), (16, 3), (146, 16)] {
        actual.resize(CrosswordsSize::new(cols, rows));
        if (cols, rows) == (16, 3) {
            for byte in b"\x1b[H\x1b[K\r\n/example        \r\nlambda\x1b[K\x1b[1C" {
                parser.advance(&mut actual, &[*byte]);
            }
        }
    }
    // Native history cannot be pulled back into its mutable viewport. Check
    // both the final live rows and the original ordered table in scrollback.
    for history in [false, true] {
        let mut reference = create();
        let text = if history {
            actual.scroll_display(rio_backend::crosswords::grid::Scroll::Top);
            format!("{}\r\n\r\n/example", lines.join("\r\n"))
        } else {
            "\r\n/example\r\nlambda ".into()
        };
        Processor::default().advance(&mut reference, text.as_bytes());
        for scale in [1.0, 1.25, 2.0] {
            let (rendered, width, height) = pixels(&mut actual, scale);
            let (expected, _, _) = pixels(&mut reference, scale);
            assert_eq!(rendered.len(), expected.len());
            assert_eq!(
                rendered
                    .iter()
                    .zip(&expected)
                    .filter(|(a, b)| a != b)
                    .count(),
                0,
                "extreme restore pixels, history={history}, scale={scale}"
            );
            assert!(rendered.iter().any(|pixel| *pixel != 0x00081218));
            if history && scale == 1.0 {
                if let Some(path) = std::env::var_os("AUTOMEXIA_EXTREME_PREVIEW") {
                    image_rs::RgbImage::from_fn(width, height, |x, y| {
                        let pixel = rendered[(y * width + x) as usize];
                        image_rs::Rgb([
                            (pixel >> 16) as u8,
                            (pixel >> 8) as u8,
                            pixel as u8,
                        ])
                    })
                    .save(path)
                    .expect("fictional extreme resize preview");
                }
            }
        }
    }
}

fn assert_native_seam_pixels(
    pixels: impl Fn(&mut Crosswords<VoidListener>, f32) -> (Vec<u32>, u32, u32),
) {
    let create = |columns| {
        Crosswords::new(
            CrosswordsSize::new(columns, 8),
            rio_backend::ansi::CursorShape::Block,
            VoidListener {},
            WindowId::from(0),
            0,
            2_000,
        )
    };
    let prefix = "xxxxxx   ".repeat(6);
    let mut actual = create(9);
    actual.set_resize_policy(rio_backend::crosswords::ResizePolicy::Conpty);
    let mut parser = Processor::default();
    for byte in format!("{prefix}tail\r\n{}", "\r\n".repeat(6)).as_bytes() {
        parser.advance(&mut actual, &[*byte]);
    }
    actual.resize(CrosswordsSize::new(100, 8));
    // Once the live suffix also enters history, growing rejoins it to the
    // retained prefix. Lost seam spaces visibly shift 'tail' to the left.
    parser.advance(&mut actual, "after\r\n".repeat(8).as_bytes());
    actual.resize(CrosswordsSize::new(101, 8));
    actual.scroll_display(rio_backend::crosswords::grid::Scroll::Top);
    let mut reference = create(101);
    Processor::default().advance(
        &mut reference,
        format!("{prefix}tail{}after", "\r\n".repeat(7)).as_bytes(),
    );
    for scale in [1.0, 1.25, 2.0] {
        let (rendered, width, height) = pixels(&mut actual, scale);
        let (expected, reference_width, reference_height) = pixels(&mut reference, scale);
        assert_eq!((width, height), (reference_width, reference_height));
        assert_eq!(rendered.len(), expected.len());
        assert_eq!(
            rendered
                .iter()
                .zip(&expected)
                .filter(|(a, b)| a != b)
                .count(),
            0,
            "exact seam-gap pixels at scale {scale}"
        );
        if scale == 1.0 {
            if let Some(path) = std::env::var_os("AUTOMEXIA_SEAM_PREVIEW") {
                image_rs::RgbImage::from_fn(width, height, |x, y| {
                    let pixel = rendered[(y * width + x) as usize];
                    image_rs::Rgb([(pixel >> 16) as u8, (pixel >> 8) as u8, pixel as u8])
                })
                .save(path)
                .expect("fictional seam preview");
            }
        }
    }
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

#[test]
fn presentation_highlighting_toggle_preserves_extension_and_native_output() {
    let mut terminal = terminal("[ERROR] fixture failed\r\n");
    let (rows, _, _) = snapshot(&mut terminal);
    let config: Config =
        toml::from_str("[presentation]\noutput-highlighting = false\n").unwrap();
    let mut renderer = Renderer::new(&config);
    renderer.devops_enabled = true;
    assert_eq!(
        semantic_row_severity(&rows[0], 80, &renderer, &mut String::new()),
        None
    );
    assert!(renderer.devops_enabled);
    renderer.update_config(&Config::default());
    renderer.devops_enabled = true;
    assert_eq!(
        semantic_row_severity(&rows[0], 80, &renderer, &mut String::new()),
        Some(crate::automexia::api::SemanticSeverity::Error)
    );
    renderer.devops_enabled = false;
    assert_eq!(
        semantic_row_severity(&rows[0], 80, &renderer, &mut String::new()),
        None
    );
}
