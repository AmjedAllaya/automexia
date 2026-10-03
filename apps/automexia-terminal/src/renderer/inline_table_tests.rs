use super::inline_tables::{draw, Canvas, PaintOptions};
use crate::automexia::api::SemanticSeverity;
use crate::automexia::inline_tables::Snapshot;
use crate::context::renderable::RenderableContent;
use rio_backend::{
    ansi::CursorShape,
    config::colors::{AnsiColor, Colors, NamedColor},
    config::presentation::{
        HighlightAppearance, HighlightColors, HighlightStyle, Rgb, Rgba,
    },
    crosswords::{
        grid::Scroll,
        pos::{Column, Line, Pos},
        style::{Style, StyleFlags},
        Crosswords, CrosswordsSize,
    },
    event::{TerminalDamage, VoidListener, WindowId},
    performer::handler::Processor,
    selection::SelectionRange,
    sugarloaf::{
        font::{constants, FontData, FontLibrary, FontLibraryData},
        text::{DrawOpts, Text},
    },
};

const SENTINEL: u32 = 0x00112233;

#[path = "table_appearance_tests.rs"]
mod table_appearance_tests;

#[test]
fn non_pod_kubernetes_tables_keep_their_palette_after_tabulation() {
    for source in [
        "NAME  READY  UP-TO-DATE  AVAILABLE  AGE\r\napi   1/3    3           1          1d\r\n\r\nprompt ",
        "NAME  READY  AGE\r\ndb    1/2    1d\r\n\r\nprompt ",
        "NAME  STATUS       AGE\r\ndemo  Terminating  1d\r\n\r\nprompt ",
        "NAME  SERVICE       AVAILABLE               AGE\r\napi   demo/metrics  Unknown (ProbePending)  1d\r\n\r\nprompt ",
    ] {
        for width in [36, 80] {
        let content = content(&mut terminal(source, width, 20));
        assert_eq!(content.inline_tables.surfaces.len(), 1);
        let general = HighlightAppearance { style: HighlightStyle::Background,
            warning_background: Some(Rgba::from_bytes([240, 20, 30, 192])), ..Default::default() };
        let kube = HighlightAppearance { style: HighlightStyle::Background,
            warning_background: Some(Rgba::from_bytes([20, 30, 240, 192])), ..Default::default() };
        let baseline = render_with_options(&content, 1.0, 8.0, 14.0, PaintOptions::default());
        let (first, height) = content.inline_tables.row_geometry(0, 1, &content.command_rows).unwrap();
        for generic_on in [false, true] {
            for kube_on in [false, true] {
                let canvas = render_with_options(&content, 1.0, 8.0, 14.0, PaintOptions {
                    highlight: generic_on.then_some(general), kubernetes_highlight: kube_on.then_some(kube),
                    ..Default::default()
                });
                for line in 0..height {
                let y = 8.0 + (first + line as isize) as f32 * 20.0 + 8.0;
                let ordinary = painted_background(&baseline, 12.5, y);
                let painted = painted_background(&canvas, 12.5, y);
                if kube_on { assert!(painted[2] > painted[0] + 0.4, "{source}"); }
                else { assert_eq!(painted, ordinary, "generic colors must not take over: {source}"); }
                }
            }
        }
        }
    }
}

struct RasterCanvas {
    text: Text,
    rects: Vec<([f32; 4], [f32; 4])>,
    scale: f32,
}

impl RasterCanvas {
    fn new(scale: f32) -> Self {
        let mut data = FontLibraryData::default();
        data.insert(
            FontData::from_static_slice(constants::FONT_CASCADIA_CODE_NF).unwrap(),
        );
        for _ in 0..3 {
            data.insert_alias(0);
        }
        let fonts = FontLibrary {
            inner: std::sync::Arc::new(parking_lot::RwLock::new(data)),
        };
        let mut text = Text::new(&fonts);
        text.init_cpu();
        text.set_scale_factor(scale);
        Self {
            text,
            rects: Vec::new(),
            scale,
        }
    }

    fn pixels(&mut self, width: u32, height: u32, glyphs: bool) -> Vec<u32> {
        let mut pixels = vec![SENTINEL; width as usize * height as usize];
        // Independent pixel-centre coverage. Production clipping and rectangle
        // helpers must not compute the expected occupied pixels in this test.
        for ([left, top, w, h], color) in &self.rects {
            let pixel = ((color[0] * 255.0) as u32) << 16
                | ((color[1] * 255.0) as u32) << 8
                | (color[2] * 255.0) as u32;
            for y in 0..height {
                for x in 0..width {
                    let px = (x as f32 + 0.5) / self.scale;
                    let py = (y as f32 + 0.5) / self.scale;
                    if px >= *left && px < left + w && py >= *top && py < top + h {
                        pixels[(y * width + x) as usize] = pixel;
                    }
                }
            }
        }
        if glyphs {
            self.text.render_cpu_base(&mut pixels, width, height);
            self.text.render_cpu_modal(&mut pixels, width, height);
        }
        pixels
    }
}

impl Canvas for RasterCanvas {
    fn text(&mut self) -> &mut Text {
        &mut self.text
    }
    fn rect(&mut self, bounds: [f32; 4], color: [f32; 4]) {
        self.rects.push((bounds, color));
    }
}

fn terminal(output: &str, columns: usize, rows: usize) -> Crosswords<VoidListener> {
    let mut terminal = Crosswords::new(
        CrosswordsSize::new(100, rows),
        CursorShape::Block,
        VoidListener {},
        WindowId::from(0),
        0,
        2000,
    );
    Processor::default().advance(&mut terminal, output.as_bytes());
    terminal.resize(CrosswordsSize::new(columns, rows));
    terminal
}

fn content(terminal: &mut Crosswords<VoidListener>) -> RenderableContent {
    let mut content = RenderableContent {
        columns: terminal.columns(),
        screen_lines: terminal.screen_lines(),
        display_offset: terminal.display_offset(),
        ..RenderableContent::default()
    };
    refresh_content(&mut content, terminal);
    content
}

fn refresh_content(
    content: &mut RenderableContent,
    terminal: &mut Crosswords<VoidListener>,
) {
    content.columns = terminal.columns();
    content.screen_lines = terminal.screen_lines();
    content.display_offset = terminal.display_offset();
    terminal.snapshot_visible(
        &TerminalDamage::Full,
        terminal.columns(),
        &mut content.visible_rows,
        &mut content.style_table,
        &mut content.extras,
    );
    content.term_colors = terminal.colors;
    content.inline_tables.refresh(Snapshot::capture(terminal));
    content
        .command_rows
        .rebuild(content.screen_lines, &content.inline_tables.bands());
}

fn simple(columns: usize) -> RenderableContent {
    let mut terminal = terminal(
        "NAME   VALUE\r\nalpha  bravo\r\ngamma  omega\r\n\r\nprompt ",
        columns,
        12,
    );
    // Narrowing can push the beginning into history. This geometry fixture
    // deliberately renders the table from its first source row.
    terminal.scroll_display(Scroll::Top);
    content(&mut terminal)
}

fn source_colors(style: &Style) -> ([f32; 4], [f32; 4]) {
    let foreground = match style.fg {
        AnsiColor::Named(NamedColor::Red) => [1.0, 0.0, 0.0, 1.0],
        AnsiColor::Named(NamedColor::Green) => [0.0, 1.0, 0.0, 1.0],
        AnsiColor::Named(NamedColor::Cyan) => [0.0, 1.0, 1.0, 1.0],
        AnsiColor::Named(NamedColor::Yellow) => [1.0, 0.85, 0.0, 1.0],
        _ => [1.0; 4],
    };
    let background = match style.bg {
        AnsiColor::Spec(rgb) => rgb.to_arr(),
        AnsiColor::Indexed(1) | AnsiColor::Named(NamedColor::Red) => [1.0, 0.0, 0.0, 1.0],
        _ => [0.04, 0.04, 0.04, 1.0],
    };
    if style.flags.contains(StyleFlags::INVERSE) {
        (background, foreground)
    } else {
        (foreground, background)
    }
}

fn render(
    content: &RenderableContent,
    scale: f32,
    cell_width: f32,
    font: f32,
) -> RasterCanvas {
    render_with_options(content, scale, cell_width, font, PaintOptions::default())
}

fn render_with_options(
    content: &RenderableContent,
    scale: f32,
    cell_width: f32,
    font: f32,
    options: PaintOptions,
) -> RasterCanvas {
    let mut canvas = RasterCanvas::new(scale);
    draw(
        &mut canvas,
        content,
        [8.0, 8.0, cell_width, 20.0],
        font,
        scale,
        options,
        source_colors,
    );
    canvas
}

#[track_caller]
fn close(actual: f32, expected: f32) {
    assert!((actual - expected).abs() < 0.001, "{actual} != {expected}");
}

fn configured_highlight(style: HighlightStyle) -> HighlightAppearance {
    HighlightAppearance {
        style,
        colors: HighlightColors {
            error: Some(Rgb::from_bytes([20, 220, 240])),
            warning: Some(Rgb::from_bytes([220, 20, 220])),
            success: Some(Rgb::from_bytes([240, 150, 20])),
            ..HighlightColors::default()
        },
        error_background: Some(Rgba::from_bytes([170, 20, 30, 192])),
        warning_background: Some(Rgba::from_bytes([20, 140, 30, 192])),
        ..HighlightAppearance::default()
    }
}

fn painted_background(canvas: &RasterCanvas, x: f32, y: f32) -> [f32; 4] {
    canvas
        .rects
        .iter()
        .rev()
        .find(|([left, top, width, height], _)| {
            x >= *left && x < *left + *width && y >= *top && y < *top + *height
        })
        .map(|(_, color)| *color)
        .expect("projected cell has a painted background")
}

#[test]
fn inline_table_kubernetes_and_general_highlights_have_independent_switches() {
    for (source, kubernetes, known) in [
        (
            "NAME   READY  STATUS\r\napi    0/1    Unknown\r\n\r\nprompt ",
            true,
            true,
        ),
        (
            "NAME   READY  STATUS\r\napi    1/1    FutureState error\r\n\r\nprompt ",
            true,
            false,
        ),
        (
            "TASK   STATUS\r\nbuild  warning: retrying\r\n\r\nprompt ",
            false,
            true,
        ),
    ] {
        let content = content(&mut terminal(source, 80, 12));
        assert_eq!(content.inline_tables.surfaces.len(), 1);
        let uncolored =
            render_with_options(&content, 1.0, 8.0, 14.0, PaintOptions::default());
        let ordinary = painted_background(&uncolored, 12.5, 35.5);
        let general = HighlightAppearance {
            style: HighlightStyle::Background,
            warning_background: Some(Rgba::from_bytes([240, 20, 30, 192])),
            ..HighlightAppearance::default()
        };
        let kube = HighlightAppearance {
            style: HighlightStyle::Background,
            warning_background: Some(Rgba::from_bytes([20, 30, 240, 192])),
            ..HighlightAppearance::default()
        };
        for general_on in [false, true] {
            for kube_on in [false, true] {
                let canvas = render_with_options(
                    &content,
                    1.0,
                    8.0,
                    14.0,
                    PaintOptions {
                        highlight: general_on.then_some(general),
                        kubernetes_highlight: kube_on.then_some(kube),
                        ..PaintOptions::default()
                    },
                );
                let painted = painted_background(&canvas, 12.5, 35.5);
                if known && (if kubernetes { kube_on } else { general_on }) {
                    if kubernetes {
                        assert!(painted[2] > painted[0] + 0.4);
                    } else {
                        assert!(painted[0] > painted[2] + 0.4);
                    }
                } else {
                    assert_eq!(
                        painted, ordinary,
                        "disabled or unknown status stays neutral"
                    );
                }
            }
        }
    }
}

#[test]
fn inline_watch_uses_kubernetes_palette_with_general_highlighting_disabled() {
    let mut terminal = terminal(
        "NAME   READY  STATUS\r\napi    0/1    Unknown\r\n\r\nprompt ",
        80,
        12,
    );
    let mut content = content(&mut terminal);
    let appearance = HighlightAppearance {
        style: HighlightStyle::Background,
        warning_background: Some(Rgba::from_bytes([220, 20, 30, 192])),
        success_background: Some(Rgba::from_bytes([20, 220, 30, 192])),
        ..HighlightAppearance::default()
    };
    let options = PaintOptions {
        kubernetes_highlight: Some(appearance),
        ..PaintOptions::default()
    };
    let before = render_with_options(&content, 1.0, 8.0, 14.0, options);
    assert!(painted_background(&before, 12.5, 35.5)[0] > 0.4);
    let cursor = terminal.cursor().pos;
    Processor::default().advance(
        &mut terminal,
        b"\x1b[2A\r\x1b[2Kapi    1/1    Running\x1b[2B\r\x1b[7C",
    );
    assert_eq!(terminal.cursor().pos, cursor);
    refresh_content(&mut content, &mut terminal);
    let after = render_with_options(&content, 1.0, 8.0, 14.0, options);
    let healthy = painted_background(&after, 12.5, 35.5);
    assert!(healthy[1] > healthy[0] + 0.4);
}

#[test]
fn inline_table_capture_preserves_compact_ansi_background_in_a_status_cell() {
    use rio_backend::config::colors::ColorRgb;
    use rio_backend::crosswords::square::ContentTag;
    for (escape, expected) in [
        (
            "\x1b[48;2;17;29;43m",
            AnsiColor::Spec(ColorRgb {
                r: 17,
                g: 29,
                b: 43,
            }),
        ),
        ("\x1b[41m", AnsiColor::Indexed(1)),
    ] {
        let mut terminal = terminal(
            "NAME  STATUS\r\napi   warning: delayed\r\n\r\nprompt ",
            80,
            12,
        );
        // ECH stores an explicit background-only cell between status words.
        Processor::default().advance(
            &mut terminal,
            format!("\x1b7\x1b[2;15H{escape}\x1b[X\x1b[0m\x1b8").as_bytes(),
        );
        assert_ne!(
            terminal.grid[Line(1)][Column(14)].content_tag(),
            ContentTag::Codepoint
        );
        assert_eq!(
            terminal.bounds_to_display_string_bounded(
                Pos::new(Line(1), Column(0)), Pos::new(Line(1), Column(79)), 128,
            ).unwrap(),
            "api   warning: delayed",
            "compact background bytes must serialize as a space, never NUL or unrelated marks"
        );
        let content = content(&mut terminal);
        assert_eq!(content.inline_tables.surfaces.len(), 1);
        let (_, style) = content.inline_tables.source_cell(0, 1, 14).unwrap();
        assert_eq!(
            style.bg, expected,
            "snapshot retains explicit compact background provenance"
        );
        let options = PaintOptions {
            highlight: Some(configured_highlight(HighlightStyle::Both)),
            ..PaintOptions::default()
        };
        let canvas = render_with_options(&content, 1.0, 8.0, 14.0, options);
        let surface = &content.inline_tables.surfaces[0];
        let fragment = &surface.layout.rows[1].cells[1].fragments[0];
        let offset = 14 - fragment.source_cells.start;
        let x = 8.0 + (surface.layout.columns[1].content_x + offset) as f32 * 8.0 + 0.5;
        assert_eq!(
            painted_background(&canvas, x, 35.5),
            source_colors(&style).1,
            "status tint cannot overwrite compact ANSI fill"
        );
    }
}

#[test]
fn composed_command_bands_never_recolor_tables_statuses_ansi_or_selection() {
    use super::command_results::{
        command_background_paints, project_protected_rows, protect_source_rows,
    };
    use crate::automexia::ui::CommandResultAnchor;
    use rio_backend::config::presentation::CommandOutputAppearance;
    for source in [
        "NAME   READY  STATUS\r\napi    0/1    Unknown\r\nworker 1/1    Running\r\n\r\nprompt ",
        "NAME   READY  STATUS\r\n\x1b[31mapi    0/1    Unknown\x1b[0m\r\n\x1b[7mworker 1/1    Running\x1b[0m\r\n\r\nprompt ",
        "NAME   VALUE\r\nfirst  ordinary-text\r\nsecond another-value\r\n\r\nprompt ",
    ] {
        for columns in [20, 80] {
            let mut terminal = terminal(source, columns, 20);
            terminal.scroll_display(Scroll::Top);
            let mut content = content(&mut terminal);
            assert_eq!(content.inline_tables.surfaces.len(), 1);
            content.selection_range = Some(SelectionRange::new(
                Pos::new(Line(1), Column(0)), Pos::new(Line(1), Column(1)), false,
            ));
            crate::grid_emit::classify_visible_output(&content.visible_rows, columns, false,
                &mut content.output_classifications, &mut String::new());
            protect_source_rows(&mut content);
            let mut protected = Vec::new();
            project_protected_rows(&content, &mut protected, true);
            let last = content.inline_tables.surfaces[0].layout.rows.iter().enumerate()
                .filter_map(|(row, _)| content.inline_tables.row_geometry(0, row, &content.command_rows))
                .map(|(first, height)| first + height as isize).max().unwrap() as usize;
            let last = last.min(content.screen_lines);
            assert!(last > 1);
            assert!(protected[..last].iter().all(|value| *value));
            let anchor = CommandResultAnchor {
                generation: Some(7), key: 7, x: 8.0, y: 8.0 + last as f32 * 20.0,
                width: columns as f32 * 8.0, height: 20.0, output_top: Some(8.0),
                separates_next_prompt: true, exit_code: Some(0), elapsed_ms: Some(10), completed_at: None,
            };
            let background = CommandOutputAppearance {
                success: Some(Rgba::from_bytes([255, 0, 255, 255])), pulse: false,
                ..CommandOutputAppearance::default()
            };
            let options = PaintOptions {
                highlight: Some(configured_highlight(HighlightStyle::Both)),
                kubernetes_highlight: Some(configured_highlight(HighlightStyle::Both)),
                ..PaintOptions::default()
            };
            let bounds = [8.0, 8.0 + content.screen_lines as f32 * 20.0];
            let raster = |canvas: &mut RasterCanvas| canvas.pixels(columns as u32 * 8 + 16, 416, true);
            let mut before = render_with_options(&content, 1.0, 8.0, 14.0, options);
            let expected = raster(&mut before);
            for enabled in [false, true] {
                let mut after = render_with_options(&content, 1.0, 8.0, 14.0, options);
                for (rect, color) in command_background_paints(&anchor, enabled.then_some(background),
                    &Colors::default(), 0.0, bounds, &protected) {
                    after.rect(rect, color);
                }
                assert!(raster(&mut after) == expected,
                    "the real command-band paint output must preserve every table pixel");
            }
            // Positive control: this fixture really overlaps the command band.
            // Without source protection the comparison must detect recoloring.
            let mut unprotected = render_with_options(&content, 1.0, 8.0, 14.0, options);
            for (rect, color) in command_background_paints(&anchor, Some(background), &Colors::default(),
                0.0, bounds, &vec![false; content.screen_lines]) {
                unprotected.rect(rect, color);
            }
            assert!(raster(&mut unprotected) != expected, "unprotected positive control must recolor the table");
        }
    }
}

#[test]
fn inline_table_status_background_covers_whole_row_without_flattening_statuses() {
    let content = content(&mut terminal(
        "NAME      READY  STATUS\r\nwarning   0/1    Unknown\r\nhealthy   1/1    Running\r\n\r\nprompt ",
        80,
        12,
    ));
    assert_eq!(content.inline_tables.surfaces.len(), 1);
    let mut appearance = HighlightAppearance {
        style: HighlightStyle::Both,
        warning_background: Some(Rgba::from_bytes([220, 30, 0, 192])),
        success_background: Some(Rgba::from_bytes([0, 180, 40, 192])),
        ..HighlightAppearance::default()
    };
    appearance.colors.warning = Some(Rgb::from_bytes([255, 210, 0]));
    appearance.colors.success = Some(Rgb::from_bytes([0, 240, 90]));
    let canvas = render_with_options(
        &content,
        1.0,
        8.0,
        14.0,
        PaintOptions {
            highlight: Some(appearance),
            kubernetes_highlight: Some(appearance),
            ..PaintOptions::default()
        },
    );
    // x=12 lies in the first cell's padding, outside every text run. A row
    // status is a row property, not a tint that disappears between glyphs.
    let warning = painted_background(&canvas, 12.5, 35.5);
    let healthy = painted_background(&canvas, 12.5, 55.5);
    assert!(
        warning[0] > warning[1] + 0.2,
        "warning padding: {warning:?}"
    );
    assert!(
        healthy[1] > healthy[0] + 0.2,
        "healthy padding: {healthy:?}"
    );
}

#[test]
fn inline_table_same_cursor_watch_rewrite_replaces_status_and_source_style() {
    let mut terminal = terminal(
        "\x1b[36mNAME   READY  STATUS\x1b[0m\r\npod-a  0/1    Unknown\r\n\r\nprompt ",
        80,
        12,
    );
    let mut content = content(&mut terminal);
    assert_eq!(content.inline_tables.surfaces.len(), 1);
    assert_eq!(
        crate::automexia::output_semantics::classify_row_text(
            &content.inline_tables.surfaces[0].table.source()[1]
        ),
        Some(SemanticSeverity::Warning)
    );
    let original_cursor = terminal.cursor().pos;
    terminal.reset_damage();
    Processor::default().advance(
        &mut terminal,
        b"\x1b[2A\r\x1b[2Kpod-a  1/1    Running\x1b[2B\r\x1b[7C",
    );
    assert_eq!(terminal.cursor().pos, original_cursor);
    assert!(matches!(
        terminal.peek_damage_event(),
        Some(TerminalDamage::Partial | TerminalDamage::Full)
    ));
    refresh_content(&mut content, &mut terminal);
    assert_eq!(content.inline_tables.surfaces.len(), 1);
    assert_eq!(
        crate::automexia::output_semantics::classify_row_text(
            &content.inline_tables.surfaces[0].table.source()[1]
        ),
        Some(SemanticSeverity::Success)
    );
    let appearance = HighlightAppearance {
        style: HighlightStyle::Both,
        success_background: Some(Rgba::from_bytes([0, 180, 40, 192])),
        ..HighlightAppearance::default()
    };
    let canvas = render_with_options(
        &content,
        1.0,
        8.0,
        14.0,
        PaintOptions {
            highlight: Some(appearance),
            kubernetes_highlight: Some(appearance),
            ..PaintOptions::default()
        },
    );
    let healthy = painted_background(&canvas, 12.5, 35.5);
    assert!(healthy[1] > healthy[0] + 0.2, "watch repaint: {healthy:?}");
}

#[test]
fn inline_table_highlighting_uses_configured_severity_and_style_on_real_raster() {
    let content = content(&mut terminal(
        "NAME   STATUS\r\nalpha  [ERROR] failed\r\ngamma  [WARN] retrying\r\npod    1/1 Running\r\n\r\nprompt ",
        32,
        12,
    ));
    assert_eq!(content.inline_tables.surfaces.len(), 1);
    let rows = content.inline_tables.surfaces[0].table.source();
    for (row, expected) in [
        (1, SemanticSeverity::Error),
        (2, SemanticSeverity::Warning),
        (3, SemanticSeverity::Success),
    ] {
        assert_eq!(
            crate::automexia::output_semantics::classify_row_text(&rows[row]),
            Some(expected),
            "real projected row {row} must be classifiable"
        );
    }
    let unhighlighted =
        render_with_options(&content, 1.0, 8.0, 14.0, PaintOptions::default());
    for style in [
        HighlightStyle::Foreground,
        HighlightStyle::Background,
        HighlightStyle::Both,
    ] {
        let mut canvas = render_with_options(
            &content,
            1.0,
            8.0,
            14.0,
            PaintOptions {
                highlight: Some(configured_highlight(style)),
                kubernetes_highlight: Some(configured_highlight(style)),
                ..PaintOptions::default()
            },
        );
        let error_background = painted_background(&canvas, 80.5, 35.5);
        let warning_background = painted_background(&canvas, 80.5, 55.5);
        let success_background = painted_background(&canvas, 80.5, 75.5);
        if style == HighlightStyle::Foreground {
            close(
                error_background[0],
                painted_background(&unhighlighted, 80.5, 35.5)[0],
            );
            close(
                warning_background[1],
                painted_background(&unhighlighted, 80.5, 55.5)[1],
            );
        } else {
            assert!(
                error_background[0] > 0.3 && error_background[1] < 0.12,
                "configured translucent error background must tint its cell: {error_background:?}"
            );
            assert!(
                warning_background[1] > 0.3 && warning_background[0] < 0.12,
                "configured translucent warning background must tint its cell: {warning_background:?}"
            );
        }
        let unhighlighted_success = painted_background(&unhighlighted, 80.5, 75.5);
        if style == HighlightStyle::Background {
            assert!(
                success_background[1] > unhighlighted_success[1] + 0.06,
                "background style must tint success cells: {success_background:?}"
            );
        } else {
            close(success_background[0], unhighlighted_success[0]);
        }
        let pixels = canvas.pixels(280, 180, true);
        let pixels = &pixels;
        for (top, bottom, colored) in
            [(29, 47, 0usize), (49, 67, 1usize), (69, 87, 2usize)]
        {
            let tinted = (top..bottom)
                .flat_map(|y| (73..205).map(move |x| pixels[y * 280 + x]))
                .filter(|pixel| {
                    let (red, green, blue) =
                        (pixel >> 16 & 255, pixel >> 8 & 255, pixel & 255);
                    match colored {
                        0 => green > red + 35 && blue > red + 35 && blue > 100,
                        1 => red > green + 35 && blue > green + 35 && red > 100,
                        _ => red > blue + 50 && green > blue + 20 && red > 100,
                    }
                })
                .count();
            if style == HighlightStyle::Background {
                assert_eq!(
                    tinted, 0,
                    "background style must retain source glyph colors"
                );
            } else {
                assert!(
                    tinted > 0,
                    "configured severity glyphs must reach the raster"
                );
            }
        }
    }
}

#[test]
fn inline_table_highlighting_keeps_ansi_inverse_and_selected_source_cells() {
    let mut content = content(&mut terminal(
        "NAME   STATUS\r\nalpha  \x1b[31merror failed\x1b[0m\r\ngamma  \x1b[7mwarning: retrying\x1b[0m\r\ndelta  error failed\r\n\r\nprompt ",
        32,
        12,
    ));
    assert_eq!(content.inline_tables.surfaces.len(), 1);
    let options = PaintOptions {
        highlight: Some(configured_highlight(HighlightStyle::Both)),
        kubernetes_highlight: Some(configured_highlight(HighlightStyle::Both)),
        ..PaintOptions::default()
    };
    let mut canvas = render_with_options(&content, 1.0, 8.0, 14.0, options);
    let ansi_pixels = canvas.pixels(280, 180, true);
    let ansi_pixels = &ansi_pixels;
    assert!(
        (29..47)
            .flat_map(|y| (73..165).map(move |x| ansi_pixels[y * 280 + x]))
            .any(|pixel| {
                (pixel >> 16 & 255) > 50 && (pixel >> 8 & 255) < 30 && (pixel & 255) < 30
            }),
        "explicit source ANSI red must survive the semantic error color"
    );
    assert_eq!(
        painted_background(&canvas, 80.5, 55.5),
        [1.0; 4],
        "inverse source background must survive warning tint"
    );
    content.selection_range = Some(SelectionRange::new(
        Pos::new(Line(3), Column(7)),
        Pos::new(Line(3), Column(8)),
        false,
    ));
    let selected = render_with_options(&content, 1.0, 8.0, 14.0, options);
    let selection_background = Colors::default().selection_background;
    assert_eq!(
        painted_background(&selected, 80.5, 75.5),
        selection_background,
        "selection takes precedence over the semantic error background"
    );
    assert!(selected.rects.iter().any(|(bounds, color)| {
        *color == selection_background
            && bounds[0] >= 72.0
            && bounds[0] + bounds[2] <= 88.0
            && bounds[1] >= 68.0
            && bounds[1] + bounds[3] <= 88.0
    }));
    content.selection_range = Some(SelectionRange::new(
        Pos::new(Line(1), Column(7)),
        Pos::new(Line(1), Column(8)),
        false,
    ));
    let mut preserved = render_with_options(
        &content,
        1.0,
        8.0,
        14.0,
        PaintOptions {
            preserve_selection_foreground: true,
            ..options
        },
    );
    let preserved_pixels = preserved.pixels(280, 180, true);
    let preserved_pixels = &preserved_pixels;
    assert!(
        (29..47)
            .flat_map(|y| (73..88).map(move |x| preserved_pixels[y * 280 + x]))
            .any(|pixel| {
                (pixel >> 16 & 255) > 50 && (pixel >> 8 & 255) < 30 && (pixel & 255) < 30
            }),
        "selected source ANSI red must survive preserve-selection-foreground"
    );
}

#[test]
fn inline_table_pixels_have_single_shared_edges_at_wide_narrow_and_fractional_sizes() {
    for scale in [1.0, 1.25, 2.0] {
        for (columns, expected_width, cell_edges, row_edges, total_height) in [
            (20, 112.0, [8.0, 64.0, 120.0], [8.0, 28.0, 48.0, 68.0], 60.0),
            (10, 80.0, [8.0, 48.0, 88.0], [8.0, 48.0, 88.0, 128.0], 120.0),
        ] {
            let content = simple(columns);
            assert_eq!(content.inline_tables.surfaces.len(), 1);
            let mut canvas = render(&content, scale, 8.0, 14.0);
            let stroke = 1.0 / scale;
            let vertical: Vec<_> = canvas
                .rects
                .iter()
                .filter(|(r, _)| {
                    (r[2] - stroke).abs() < 0.001 && (r[3] - total_height).abs() < 0.001
                })
                .collect();
            assert_eq!(
                vertical.len(),
                3,
                "each shared border is emitted exactly once"
            );
            for (index, (rect, _)) in vertical.iter().enumerate() {
                close(
                    rect[0],
                    cell_edges[index] - if index == 2 { stroke } else { 0.0 },
                );
                close(rect[1], 8.0);
            }
            let mut horizontal: Vec<_> = canvas
                .rects
                .iter()
                .filter(|(r, _)| {
                    (r[3] - stroke).abs() < 0.001 && (r[2] - expected_width).abs() < 0.001
                })
                .map(|(r, _)| r[1])
                .collect();
            horizontal.sort_by(f32::total_cmp);
            assert_eq!(horizontal.len(), 4);
            for (index, actual) in horizontal.iter().enumerate() {
                close(
                    *actual,
                    row_edges[index] - if index == 0 { 0.0 } else { stroke },
                );
            }
            let width = ((8.0 + columns as f32 * 8.0 + 12.0) * scale).ceil() as u32;
            let height = (260.0 * scale).ceil() as u32;
            let pixels = canvas.pixels(width, height, true);
            assert!(pixels.iter().any(|pixel| *pixel != SENTINEL));
            for py in 0..height {
                for px in 0..width {
                    if pixels[(py * width + px) as usize] == SENTINEL {
                        continue;
                    }
                    let x = (px as f32 + 0.5) / scale;
                    let y = (py as f32 + 0.5) / scale;
                    assert!(x >= 8.0 && x < 8.0 + columns as f32 * 8.0);
                    assert!((8.0..248.0).contains(&y));
                }
            }
        }
    }
}

#[test]
fn inline_table_real_glyphs_cannot_escape_their_cells_or_pane() {
    let content = simple(10);
    assert_eq!(content.inline_tables.surfaces.len(), 1);
    for scale in [1.0, 1.25, 2.0] {
        // Deliberately oversized glyphs expose clipping errors; they must never
        // spill into a neighbouring cell, border, or pane despite their ink.
        let mut canvas = render(&content, scale, 3.0, 24.0);
        let width = (60.0 * scale).ceil() as u32;
        let height = (260.0 * scale).ceil() as u32;
        let backgrounds = canvas.pixels(width, height, false);
        let pixels = canvas.pixels(width, height, true);
        let mut glyph_pixels = 0;
        for py in 0..height {
            for px in 0..width {
                let index = (py * width + px) as usize;
                if pixels[index] == backgrounds[index] {
                    continue;
                }
                glyph_pixels += 1;
                let x = (px as f32 + 0.5) / scale;
                let y = (py as f32 + 0.5) / scale;
                assert!(
                    (11.0..20.0).contains(&x) || (26.0..35.0).contains(&x),
                    "glyph escaped its column at {x}"
                );
                assert!(
                    (0..6).any(|line| {
                        y >= 8.0 + line as f32 * 20.0 + 1.0 / scale
                            && y < 8.0 + (line + 1) as f32 * 20.0 - 1.0 / scale
                    }),
                    "glyph escaped its row at {y}"
                );
            }
        }
        assert!(glyph_pixels > 0, "clipping must not hide every glyph");
        if scale == 1.0 {
            if let Some(path) = std::env::var_os("AUTOMEXIA_INLINE_TABLE_PREVIEW") {
                image_rs::RgbImage::from_fn(width, height, |x, y| {
                    let pixel = pixels[(y * width + x) as usize];
                    image_rs::Rgb([(pixel >> 16) as u8, (pixel >> 8) as u8, pixel as u8])
                })
                .save(path)
                .expect("write controlled inline-table preview");
            }
        }
    }
}

#[test]
fn inline_table_preserves_ansi_status_colors_and_partial_cell_selection() {
    let mut content = content(&mut terminal(
        "NAME   VALUE\r\n\x1b[31malpha\x1b[0m  \x1b[32mbravo\x1b[0m\r\ngamma  omega\r\n\r\nprompt ",
        20, 12,
    ));
    assert_eq!(content.inline_tables.surfaces.len(), 1);
    let mut canvas = render(&content, 1.0, 8.0, 14.0);
    let pixels = canvas.pixels(180, 260, true);
    let mut red = false;
    let mut green = false;
    for y in 29..47 {
        for x in 16..56 {
            let pixel = pixels[y * 180 + x];
            red |=
                (pixel >> 16 & 255) > 50 && (pixel >> 8 & 255) < 30 && (pixel & 255) < 30;
        }
        for x in 72..112 {
            let pixel = pixels[y * 180 + x];
            green |=
                (pixel >> 8 & 255) > 50 && (pixel >> 16 & 255) < 30 && (pixel & 255) < 30;
        }
    }
    assert!(
        red && green,
        "source ANSI red and green must survive the inline presentation"
    );
    // Source text is alpha; only its final h and a are selected. No production
    // inverse mapping computes this expected selection span.
    content.selection_range = Some(SelectionRange::new(
        Pos::new(Line(1), Column(3)),
        Pos::new(Line(1), Column(4)),
        false,
    ));
    let selected = render(&content, 1.0, 8.0, 14.0);
    let selection_color = Colors::default().selection_background;
    let selection: Vec<_> = selected
        .rects
        .iter()
        .filter(|(_, color)| *color == selection_color)
        .map(|(rect, _)| rect)
        .collect();
    assert!(!selection.is_empty());
    for rect in &selection {
        assert!(
            rect[0] >= 40.0 && rect[0] + rect[2] <= 56.0,
            "selection escaped the two chosen source cells: {rect:?}"
        );
        assert!(rect[1] >= 28.0 && rect[1] + rect[3] <= 48.0);
    }
    for x in [41.0, 49.0] {
        assert!(selection.iter().any(|r| x >= r[0] && x < r[0] + r[2]));
    }
}

#[test]
fn inline_table_scrolled_sources_do_not_overlap_or_abort_visible_hit_testing() {
    let mut output = String::from("NAME   VALUE\r\n");
    for value in ["alpha", "bravo", "gamma", "delta", "omega", "sigma"] {
        output.push_str(&format!("{value}  value\r\n"));
    }
    output.push_str("\r\nprompt ");
    let mut terminal = terminal(&output, 10, 4);
    let content = content(&mut terminal);
    assert_eq!(content.inline_tables.surfaces.len(), 1);
    assert!(
        content
            .inline_tables
            .row_geometry(0, 0, &content.command_rows)
            .is_none(),
        "fully offscreen expanded rows must not paint into live rows"
    );
    let surface = &content.inline_tables.surfaces[0];
    let mut last_bottom = isize::MIN;
    let mut checked = 0;
    for (index, row) in surface.layout.rows.iter().enumerate() {
        let Some((top, height)) =
            content
                .inline_tables
                .row_geometry(0, index, &content.command_rows)
        else {
            continue;
        };
        assert!(top >= last_bottom, "projected logical table rows overlap");
        last_bottom = top + height as isize;
        for (line, fragment) in row.cells[0].fragments.iter().enumerate() {
            let visual = top + line as isize;
            if !(0..4).contains(&visual) {
                continue;
            }
            let (native, col) = content
                .inline_tables
                .source_position(
                    &content.command_rows,
                    visual as usize,
                    surface.layout.columns[0].content_x,
                )
                .expect("offscreen earlier rows cannot abort visible hit testing");
            let actual = terminal.grid[Line(native - terminal.display_offset() as i32)]
                [Column(col)]
            .c();
            assert_eq!(
                Some(actual),
                surface.table.source()[index][fragment.bytes.clone()]
                    .chars()
                    .next()
            );
            checked += 1;
        }
    }
    assert!(checked > 0);
}

#[test]
fn inline_table_wide_wrap_spacer_maps_to_the_actual_source_character() {
    let mut terminal = terminal("NAME         STATE\r\nabcdef\u{754c}     ready\r\nfoo          done\r\n\r\nprompt ", 7, 24);
    terminal.scroll_display(Scroll::Top);
    let content = content(&mut terminal);
    assert_eq!(content.inline_tables.surfaces.len(), 1);
    let surface = &content.inline_tables.surfaces[0];
    let (top, _) = content
        .inline_tables
        .row_geometry(0, 1, &content.command_rows)
        .unwrap();
    let (line, _) = surface.layout.rows[1].cells[0]
        .fragments
        .iter()
        .enumerate()
        .find(|(_, f)| surface.table.source()[1][f.bytes.clone()].contains('\u{754c}'))
        .unwrap();
    let (native, col) = content
        .inline_tables
        .source_position(
            &content.command_rows,
            (top + line as isize) as usize,
            surface.layout.columns[0].content_x,
        )
        .unwrap();
    assert_eq!(
        terminal.grid[Line(native - terminal.display_offset() as i32)][Column(col)].c(),
        '\u{754c}'
    );
    assert_eq!(col, 0, "wide character follows the one-cell wrap spacer");
}

fn source_character(
    content: &RenderableContent,
    terminal: &Crosswords<VoidListener>,
    visual_row: usize,
    visual_column: usize,
) -> char {
    let (native, column) = content
        .inline_tables
        .source_position(&content.command_rows, visual_row, visual_column)
        .expect("visible table cell has an authoritative source position");
    terminal.grid[Line(native - terminal.display_offset() as i32)][Column(column)].c()
}

#[test]
fn inline_table_visual_scroll_round_trip_preserves_partial_soft_wrapped_rows() {
    // Each ten-column logical row spans two six-column VT rows, but its two
    // single-column value cells need four presentation lines. Advancing three
    // visual lines must consume one native row and the already-inserted space.
    let mut terminal = terminal(
        "NAME  TYPE\r\nabcd  efgh\r\nijkl  mnop\r\n\r\nprompt ",
        6,
        4,
    );
    terminal.scroll_display(Scroll::Top);
    let original_offset = terminal.display_offset();
    assert!(original_offset > 1);
    let mut content = content(&mut terminal);
    content.command_rows.settle(None, 4);
    assert_eq!(content.inline_tables.surfaces.len(), 1);
    assert_eq!(content.inline_tables.surfaces[0].layout.rows[0].height, 4);
    assert_eq!(source_character(&content, &terminal, 0, 1), 'N');
    let original_pixels = render(&content, 1.0, 8.0, 14.0).pixels(80, 100, true);
    for _ in 0..3 {
        content.scroll_lines(&mut terminal, -3);
        assert_eq!(terminal.display_offset(), original_offset - 1);
        refresh_content(&mut content, &mut terminal);
        content.command_rows.settle(None, 4);
        assert_eq!(
            source_character(&content, &terminal, 0, 1),
            'E',
            "scrolling must not reinsert the expanded prefix and repeat A"
        );
        assert_eq!(source_character(&content, &terminal, 1, 1), 'a');
        content.scroll_lines(&mut terminal, 3);
        assert_eq!(terminal.display_offset(), original_offset);
        refresh_content(&mut content, &mut terminal);
        content.command_rows.settle(None, 4);
        assert_eq!(source_character(&content, &terminal, 0, 1), 'N');
        assert_eq!(
            render(&content, 1.0, 8.0, 14.0).pixels(80, 100, true),
            original_pixels,
            "round-trip scrolling must restore the original table raster"
        );
    }
}

#[test]
fn inline_table_mixed_unicode_keeps_vt_source_offsets_and_separate_glyph_runs() {
    // The VT stores this ZWJ sequence in four cells and ticket+VS16 in one;
    // grapheme display-width libraries commonly report two cells for each.
    let mut terminal = terminal(
        "NAME            STATE\r\nab界👩\u{200d}💻🎟\u{fe0f}       ready\r\nsecond          done\r\n\r\nprompt ",
        40, 12,
    );
    terminal.scroll_display(Scroll::Top);
    let content = content(&mut terminal);
    assert_eq!(content.inline_tables.surfaces.len(), 1);
    let surface = &content.inline_tables.surfaces[0];
    assert_eq!(surface.layout.columns[0].content_width, 9);
    assert_eq!(surface.layout.columns[1].content_x, 12);
    for (offset, expected) in [(0, 'a'), (2, '界'), (4, '👩'), (8, '🎟')] {
        assert_eq!(
            source_character(&content, &terminal, 1, offset + 1),
            expected
        );
    }
    assert_eq!(source_character(&content, &terminal, 1, 12), 'r');
    assert_eq!(
        surface.table.source()[1],
        "ab界👩\u{200d}💻🎟\u{fe0f}       ready"
    );
    for scale in [1.0, 1.25, 2.0] {
        let mut actual = render(&content, scale, 8.0, 14.0);
        let mut expected = RasterCanvas::new(scale);
        expected.rects.clone_from(&actual.rects);
        let opts = DrawOpts {
            font_size: 14.0,
            color: [255; 4],
            ..DrawOpts::default()
        };
        // Independently specified VT coordinates keep ASCII glyphs on the cell
        // stride and start non-ASCII clusters with their own font fallback.
        for (text, x, width) in [
            ("a", 16.0, 8.0),
            ("b", 24.0, 8.0),
            ("界", 32.0, 16.0),
            ("👩\u{200d}💻", 48.0, 32.0),
            ("🎟\u{fe0f}", 80.0, 8.0),
        ] {
            // Glyph bearings can overhang a native character slot within the
            // same ASCII run; only table-cell/run clipping bounds the ink.
            let (clip_x, clip_width) = if x < 32.0 { (16.0, 16.0) } else { (x, width) };
            expected.text.draw_clipped(
                x,
                31.0,
                text,
                &opts,
                [clip_x, 28.0 + 1.0 / scale, clip_width, 20.0 - 2.0 / scale],
            );
        }
        let width = (340.0 * scale) as u32;
        let height = (260.0 * scale) as u32;
        let actual = actual.pixels(width, height, true);
        let expected = expected.pixels(width, height, true);
        for py in (29.0 * scale) as u32..(47.0 * scale) as u32 {
            for px in (16.0 * scale) as u32..(88.0 * scale) as u32 {
                let at = (py * width + px) as usize;
                assert_eq!(
                    actual[at], expected[at],
                    "Unicode run placement at {px},{py}, scale {scale}"
                );
            }
        }
    }
}

#[test]
fn inline_table_cursor_row_and_mouse_mode_invalidate_cached_presentation() {
    let mut terminal = terminal(
        "NAME   VALUE\r\nalpha  bravo\r\ngamma  omega\r\n\r\nprompt ",
        20,
        12,
    );
    let mut content = content(&mut terminal);
    assert_eq!(content.inline_tables.surfaces.len(), 1);
    assert!(!content.inline_tables.needs_snapshot(&terminal));
    terminal.reset_damage();
    Processor::default().advance(&mut terminal, b"\x1b[1;1H");
    assert!(
        matches!(terminal.peek_damage_event(), Some(TerminalDamage::Partial)),
        "the parser damages both old and new cursor rows for CUP"
    );
    assert!(
        content.inline_tables.needs_snapshot(&terminal),
        "moving the cursor into captured output changes its eligibility"
    );
    assert!(content.inline_tables.refresh(Snapshot::capture(&terminal)));
    assert!(content.inline_tables.surfaces.is_empty());
    assert!(!content.inline_tables.hides_native(0));
    Processor::default().advance(&mut terminal, b"\x1b[5;8H");
    content.inline_tables.refresh(Snapshot::capture(&terminal));
    assert_eq!(content.inline_tables.surfaces.len(), 1);
    terminal.reset_damage();
    Processor::default().advance(&mut terminal, b"\x1b[?1000h");
    assert!(
        matches!(terminal.peek_damage_event(), Some(TerminalDamage::Full)),
        "mouse eligibility changes must invalidate existing optional presentation"
    );
    assert!(content.inline_tables.needs_snapshot(&terminal));
    content.inline_tables.refresh(Snapshot::capture(&terminal));
    assert!(content.inline_tables.surfaces.is_empty());
    assert!(!content.inline_tables.hides_native(0));
    assert!(!content.inline_tables.needs_snapshot(&terminal));
    terminal.reset_damage();
    Processor::default().advance(&mut terminal, b"\x1b[?1000l");
    assert!(matches!(
        terminal.peek_damage_event(),
        Some(TerminalDamage::Full)
    ));
    assert!(content.inline_tables.needs_snapshot(&terminal));
    content.inline_tables.refresh(Snapshot::capture(&terminal));
    assert_eq!(content.inline_tables.surfaces.len(), 1);
    assert!(content.inline_tables.hides_native(0));
    assert!(!content.inline_tables.needs_snapshot(&terminal));
}

#[test]
fn inline_table_finishes_a_visible_soft_wrapped_row_below_the_viewport() {
    let mut terminal = terminal(
        "NAME   VALUE\r\nalpha  bravo\r\ngamma  omega\r\n\r\nprompt ",
        10,
        5,
    );
    terminal.scroll_display(Scroll::Top);
    let content = content(&mut terminal);
    assert_eq!(content.inline_tables.surfaces.len(), 1);
    assert_eq!(
        content.inline_tables.surfaces[0].table.source(),
        ["NAME   VALUE", "alpha  bravo", "gamma  omega",],
        "a logical row starting in the viewport must retain its below-pane tail"
    );
    assert!(content.inline_tables.hides_native(4));
    assert_eq!(source_character(&content, &terminal, 4, 1), 'g');
    let mut canvas = render(&content, 1.0, 8.0, 14.0);
    let pixels = canvas.pixels(100, 140, true);
    for y in 108..140 {
        assert!(
            pixels[y * 100..(y + 1) * 100]
                .iter()
                .all(|p| *p == SENTINEL),
            "lookahead source capture must not paint past the pane"
        );
    }
}

#[test]
fn inline_table_graphics_only_placement_and_deletion_invalidate_cached_presentation() {
    let mut terminal = terminal(
        "NAME   VALUE\r\nalpha  bravo\r\ngamma  omega\r\n\r\nprompt ",
        20,
        12,
    );
    terminal.resize(CrosswordsSize::new_with_dimensions(20, 12, 160, 240, 8, 20));
    let mut content = content(&mut terminal);
    assert_eq!(content.inline_tables.surfaces.len(), 1);
    assert!(!content.inline_tables.needs_snapshot(&terminal));
    let source_text = content.inline_tables.surfaces[0].table.source().to_vec();
    let source_cursor = terminal.cursor().pos;
    terminal.reset_damage();
    // One opaque white RGBA pixel, placed without changing cursor or cells.
    Processor::default().advance(
        &mut terminal,
        b"\x1b_Ga=T,f=32,s=1,v=1,i=7,C=1,q=2;/////w==\x1b\\",
    );
    assert_eq!(terminal.graphics.kitty_placements.len(), 1);
    assert_eq!(terminal.cursor().pos, source_cursor);
    assert!(terminal.graphics.kitty_graphics_dirty);
    assert!(
        terminal.peek_damage_event().is_none(),
        "the image placement changes graphics only"
    );
    assert!(
        content.inline_tables.needs_snapshot(&terminal),
        "image-only damage must invalidate the cached table eligibility"
    );
    content.inline_tables.refresh(Snapshot::capture(&terminal));
    assert!(content.inline_tables.surfaces.is_empty());
    assert!(!content.inline_tables.hides_native(0));
    assert!(!content.inline_tables.needs_snapshot(&terminal));
    terminal.reset_damage();
    Processor::default().advance(&mut terminal, b"\x1b_Ga=d,d=A,q=2\x1b\\");
    assert!(terminal.graphics.kitty_placements.is_empty());
    assert_eq!(terminal.cursor().pos, source_cursor);
    assert!(
        matches!(terminal.peek_damage_event(), Some(TerminalDamage::Full)),
        "image deletion explicitly damages the frame to remove its old overlay"
    );
    assert!(
        content.inline_tables.needs_snapshot(&terminal),
        "deleting the last image must restore eligible table presentation"
    );
    content.inline_tables.refresh(Snapshot::capture(&terminal));
    assert_eq!(content.inline_tables.surfaces.len(), 1);
    assert_eq!(
        content.inline_tables.surfaces[0].table.source(),
        source_text
    );
    assert!(content.inline_tables.hides_native(0));
    assert!(!content.inline_tables.needs_snapshot(&terminal));
}

#[test]
fn inline_table_selection_can_preserve_source_ansi_foreground() {
    let mut content = content(&mut terminal(
        "NAME   VALUE\r\n\x1b[31malpha\x1b[0m  bravo\r\ngamma  omega\r\n\r\nprompt ",
        20,
        12,
    ));
    content.selection_range = Some(SelectionRange::new(
        Pos::new(Line(1), Column(0)),
        Pos::new(Line(1), Column(4)),
        false,
    ));
    for preserve in [false, true] {
        let options = PaintOptions {
            colors: Colors {
                selection_foreground: [0.0, 0.0, 1.0, 1.0],
                ..Colors::default()
            },
            highlight: None,
            kubernetes_highlight: None,
            preserve_selection_foreground: preserve,
            active: true,
            ..PaintOptions::default()
        };
        let mut canvas = render_with_options(&content, 1.0, 8.0, 14.0, options);
        let pixels = canvas.pixels(180, 260, true);
        let (mut red, mut blue) = (0, 0);
        for y in 29..47 {
            for x in 16..56 {
                let pixel = pixels[y * 180 + x];
                red += usize::from(
                    (pixel >> 16 & 255) > 50
                        && (pixel >> 8 & 255) < 30
                        && (pixel & 255) < 30,
                );
                blue += usize::from(
                    (pixel & 255) > 50
                        && (pixel >> 8 & 255) < 30
                        && (pixel >> 16 & 255) < 30,
                );
            }
        }
        if preserve {
            assert!(
                red > 0 && blue == 0,
                "selected source red must survive the configured preservation setting"
            );
        } else {
            assert!(
                blue > 0 && red == 0,
                "default selection must use its configured blue foreground"
            );
        }
    }
}

#[test]
fn inline_table_hover_underline_follows_wrapped_source_only_in_active_pane() {
    use rio_backend::config::hints::{Hint, HintAction, HintInternalAction};
    let mut content = simple(10);
    // The source h/a live at columns 3/4 of the first native alpha row;
    // presentation places them at columns 1/2 on its second wrapped line.
    content.highlighted_hint = Some(crate::hints::HintMatch {
        text: "ha".to_owned(),
        start: Pos::new(Line(2 - content.display_offset as i32), Column(3)),
        end: Pos::new(Line(2 - content.display_offset as i32), Column(4)),
        hint: std::rc::Rc::new(Hint {
            regex: None,
            hyperlinks: true,
            post_processing: false,
            persist: false,
            action: HintAction::Action {
                action: HintInternalAction::Open,
            },
            mouse: Default::default(),
            binding: None,
        }),
    });
    for scale in [1.0, 1.25, 2.0] {
        for active in [false, true] {
            let canvas = render_with_options(
                &content,
                scale,
                8.0,
                14.0,
                PaintOptions {
                    active,
                    ..PaintOptions::default()
                },
            );
            let underlines: Vec<_> = canvas
                .rects
                .iter()
                .filter(|(_, color)| *color == [1.0; 4])
                .map(|(bounds, _)| bounds)
                .collect();
            if !active {
                assert!(
                    underlines.is_empty(),
                    "inactive pane must not paint a hover affordance"
                );
                continue;
            }
            assert!(!underlines.is_empty());
            let mut width = 0.0;
            for bounds in underlines {
                assert!(
                    bounds[0] >= 16.0 && bounds[0] + bounds[2] <= 32.0,
                    "hover underline must follow only source h/a: {bounds:?}"
                );
                close(bounds[1], 88.0 - 2.0 / scale);
                close(bounds[3], 1.0 / scale);
                width += bounds[2];
            }
            close(width, 16.0);
        }
    }
}

#[test]
fn inline_table_kubernetes_columns_wrap_multiword_values_and_preserve_source_copy() {
    let plain = [
        format!(
            "{:<32}{:<8}{:<12}{:<18}{}",
            "NAME", "READY", "STATUS", "RESTARTS", "AGE"
        ),
        format!(
            "{:<32}{:<8}{:<12}{:<18}{}",
            "edge-demo-7cc9f6d4b4-a1b2c", "1/1", "Running", "4 (17s ago)", "21d"
        ),
        format!(
            "{:<32}{:<8}{:<12}{:<18}{}",
            "jobs-demo-58fb794d9c-c3d4e", "0/1", "Unknown", "28 (27m ago)", "16d"
        ),
    ];
    let output = format!(
        "\x1b[36m{}\x1b[0m\r\n\x1b[32m{}\x1b[0m\r\n\x1b[33m{}\x1b[0m\r\n\r\nprompt ",
        plain[0], plain[1], plain[2]
    );
    for (label, columns) in [("wide", 100), ("narrow", 36)] {
        let mut terminal = terminal(&output, columns, 32);
        terminal.scroll_display(Scroll::Top);
        let content = content(&mut terminal);
        assert_eq!(content.inline_tables.surfaces.len(), 1);
        let surface = &content.inline_tables.surfaces[0];
        assert_eq!(surface.table.column_starts(), [0, 32, 40, 52, 70]);
        assert_eq!(surface.table.source(), plain);
        let restarts = &surface.layout.rows[1].cells[3];
        let reconstructed: String = restarts
            .fragments
            .iter()
            .map(|fragment| &surface.table.source()[1][fragment.bytes.clone()])
            .collect();
        assert_eq!(reconstructed, "4 (17s ago)");
        if columns == 36 {
            assert!(
                restarts.fragments.len() > 1,
                "multiword restart values wrap inside one cell"
            );
            assert!(surface.layout.rows[1].cells[0].fragments.len() > 1);
        }
        let native_rows: usize =
            plain.iter().map(|line| line.len().div_ceil(columns)).sum();
        let copied = terminal.bounds_to_string(
            Pos::new(Line(-(terminal.display_offset() as i32)), Column(0)),
            Pos::new(
                Line(native_rows as i32 - 1 - terminal.display_offset() as i32),
                Column((plain[2].len() - 1) % columns),
            ),
        );
        assert_eq!(
            copied,
            plain.join("\n"),
            "source copy contains original output, without generated borders"
        );
        let mut canvas = render(&content, 1.0, 8.0, 14.0);
        let width = (columns * 8 + 16) as u32;
        let height = (surface
            .layout
            .rows
            .iter()
            .zip(&plain)
            .map(|(row, source)| row.height.max(source.len().div_ceil(columns)))
            .sum::<usize>()
            * 20
            + 16) as u32;
        let pixels = canvas.pixels(width, height, true);
        assert!(pixels.iter().any(|pixel| *pixel != SENTINEL));
        if let Some(directory) =
            std::env::var_os("AUTOMEXIA_INLINE_KUBERNETES_PREVIEW_DIR")
        {
            let directory = std::path::PathBuf::from(directory);
            std::fs::create_dir_all(&directory)
                .expect("create controlled preview output directory");
            image_rs::RgbImage::from_fn(width, height, |x, y| {
                let pixel = pixels[(y * width + x) as usize];
                image_rs::Rgb([(pixel >> 16) as u8, (pixel >> 8) as u8, pixel as u8])
            })
            .save(directory.join(format!("inline-kubernetes-{label}.png")))
            .expect("write fictional Kubernetes table CPU raster diagnostic");
        }
    }
}

#[test]
fn inline_table_initially_narrow_wide_ports_stay_inside_cells_and_pane() {
    mod fixture {
        include!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../automexia-ui-model/tests/support/inline_pipeline_fixture.rs"
        ));
    }
    let columns = 31;
    let source = fixture::pipeline_rows();
    let mut terminal = Crosswords::new(
        CrosswordsSize::new(columns, 20),
        CursorShape::Block,
        VoidListener {},
        WindowId::from(0),
        0,
        2000,
    );
    Processor::default().advance(
        &mut terminal,
        format!("{}\r\n\r\nprompt ", source.join("\r\n")).as_bytes(),
    );
    terminal.scroll_display(Scroll::Top);
    let content = content(&mut terminal);
    assert_eq!(content.inline_tables.surfaces.len(), 1);
    let surface = &content.inline_tables.surfaces[0];
    assert_eq!(surface.table.source(), source);
    assert_eq!(surface.table.column_starts(), fixture::PIPELINE_STARTS);
    assert!(surface.layout.width <= columns);
    for (row_index, row) in surface.layout.rows.iter().enumerate() {
        for (column_index, cell) in row.cells.iter().enumerate() {
            let column = &surface.layout.columns[column_index];
            assert!(column.x + column.width <= columns);
            let reconstructed: String = cell
                .fragments
                .iter()
                .map(|fragment| {
                    &surface.table.source()[row_index][fragment.bytes.clone()]
                })
                .collect();
            assert_eq!(
                reconstructed,
                surface.table.source()[row_index][cell.source_bytes.clone()],
                "all long cell text stays in its own wrapped column"
            );
        }
    }
    let mut canvas = render(&content, 1.0, 8.0, 14.0);
    let width = (8 + columns * 8 + 8) as u32;
    let pixels = canvas.pixels(width, 1280, true);
    let pane_right = (8 + columns * 8) as u32;
    for y in 0..1280 {
        for x in pane_right..width {
            assert_eq!(
                pixels[(y * width + x) as usize],
                SENTINEL,
                "initial-narrow glyph or border escaped pane at {x},{y}"
            );
        }
    }
}

#[test]
fn inline_table_ascii_glyphs_keep_native_cell_positions_at_rounded_font_metrics() {
    let value = "abcdefghijklmnopqrstuvwxyZ";
    let output = format!(
        "{:<32}STATE\r\n{:<32}ready\r\n{:<32}done\r\n\r\nprompt ",
        "NAME", value, "short"
    );
    let mut terminal = terminal(&output, 80, 12);
    terminal.scroll_display(Scroll::Top);
    let content = content(&mut terminal);
    assert_eq!(content.inline_tables.surfaces.len(), 1);
    assert_eq!(
        content.inline_tables.surfaces[0].layout.columns[0].content_width,
        26
    );
    let mut actual = render(&content, 1.0, 8.0, 14.0);
    let mut expected = RasterCanvas::new(1.0);
    expected.rects.clone_from(&actual.rects);
    let opts = DrawOpts {
        font_size: 14.0,
        color: [255; 4],
        ..DrawOpts::default()
    };
    // Integer VT cells stay eight pixels apart even though this font's shaped
    // advance is fractional. The final Z owns cell 25, not natural run x=25*w.
    assert!(
        expected.text.measure("M", &opts) > 8.0,
        "fixture must exercise a shaped advance wider than the native cell stride"
    );
    for (cell, character) in value.char_indices() {
        let left = 16.0 + cell as f32 * 8.0;
        expected.text.draw_clipped(
            left,
            31.0,
            &character.to_string(),
            &opts,
            [left, 29.0, 8.0, 18.0],
        );
    }
    let backgrounds = expected.pixels(660, 260, false);
    let expected = expected.pixels(660, 260, true);
    let actual = actual.pixels(660, 260, true);
    let mut final_glyph_pixels = 0;
    for y in 29..47 {
        for x in 216..224 {
            let at = y * 660 + x;
            final_glyph_pixels += usize::from(expected[at] != backgrounds[at]);
            assert_eq!(
                actual[at], expected[at],
                "the final ASCII glyph must remain in its own VT cell at {x},{y}"
            );
        }
    }
    assert!(
        final_glyph_pixels > 0,
        "the reference final glyph must have visible ink"
    );
    for y in 29..47 {
        for x in 16..224 {
            let at = y * 660 + x;
            assert_eq!(
                actual[at], expected[at],
                "every ASCII glyph must use the native cell stride at {x},{y}"
            );
        }
    }
}

#[test]
fn inline_table_generic_header_formats_keep_every_source_row_and_cell() {
    let cases: &[(&str, &[&str])] = &[
        (
            "mixed-case ruled columns",
            &[
                "Handles  Id    ProcessName",
                "-------  --    -----------",
                "14       42    worker",
                "18       57    helper",
            ],
        ),
        (
            "lowercase typed columns",
            &[
                "pid   user   command",
                "42    guest  worker",
                "57    guest  helper",
            ],
        ),
        (
            "empty data column",
            &[
                "NAME       PORTS     STATE",
                "alpha                ready",
                "beta                 done",
            ],
        ),
        (
            "markdown with variable source positions",
            &[
                "| name | count | detail |",
                "|---|---:|---|",
                "|alpha|2|a longer multiword description|",
                "|beta|3|ready|",
            ],
        ),
        (
            "ascii frame",
            &[
                "+-------+-------+",
                "| Name  | Count |",
                "+-------+-------+",
                "| alpha | 2     |",
                "| beta  | 3     |",
                "+-------+-------+",
            ],
        ),
        (
            "unicode frame",
            &[
                "┌────────┬───────┐",
                "│ Name   │ Count │",
                "├────────┼───────┤",
                "│ alpha  │ 2     │",
                "└────────┴───────┘",
            ],
        ),
        (
            "explicit single column",
            &["| Name |", "|------|", "| alpha |", "| beta |"],
        ),
        (
            "bare ruled single column",
            &["Name", "----", "alpha", "beta"],
        ),
    ];
    for (name, lines) in cases {
        for (columns, prefix) in [
            (22, "query result"),
            (80, "query result"),
            (22, "cat fixture | sort"),
            (80, "cat fixture | sort"),
        ] {
            let output = format!("{prefix}\r\n{}\r\n\r\nprompt ", lines.join("\r\n"));
            let mut terminal = terminal(&output, columns, 40);
            terminal.scroll_display(Scroll::Top);
            let content = content(&mut terminal);
            assert_eq!(
                content.inline_tables.surfaces.len(),
                1,
                "{name}, width {columns}, prefix {prefix}"
            );
            let surface = &content.inline_tables.surfaces[0];
            assert_eq!(
                surface.table.source(),
                *lines,
                "{name}: source rows remain unchanged"
            );
            assert_eq!(surface.layout.rows.len(), lines.len());
            assert_eq!(
                surface
                    .layout
                    .rows
                    .iter()
                    .filter(|row| row.kind
                        == automexia_ui_model::tables::TableRowKind::Header)
                    .count(),
                1
            );
            for (row_index, row) in surface.layout.rows.iter().enumerate() {
                for cell in &row.cells {
                    for fragment in &cell.fragments {
                        let (position, _) = content
                            .inline_tables
                            .source_cell(0, row_index, fragment.source_cells.start)
                            .unwrap();
                        let actual = terminal.grid
                            [Line(position.row.0 - terminal.display_offset() as i32)]
                            [Column(position.col.0)]
                        .c();
                        assert_eq!(Some(actual), lines[row_index][fragment.bytes.clone()].chars().next(),
                            "{name}: displayed fragment must point to its original VT character");
                    }
                }
            }
            let mut canvas = render(&content, 1.0, 8.0, 14.0);
            assert!(canvas
                .pixels(columns as u32 * 8 + 16, 820, true)
                .iter()
                .any(|pixel| *pixel != SENTINEL));
        }
    }
}

#[test]
fn inline_table_selection_includes_wide_trailing_and_emoji_internal_cells() {
    for (value, selected_column, width) in [
        ("界          ready", 1, 2),
        ("👩\u{200d}💻        ready", 3, 4),
    ] {
        for columns in [16, 80] {
            let output = format!(
                "NAME        STATE\r\n{value}\r\nother       done\r\n\r\nprompt "
            );
            let mut terminal = terminal(&output, columns, 20);
            terminal.scroll_display(Scroll::Top);
            let mut content = content(&mut terminal);
            assert_eq!(content.inline_tables.surfaces.len(), 1);
            // Seventeen source cells put the data row after two native rows at
            // width16 and one at width80. Select only a trailing/internal cell.
            let data_row = if columns == 16 { 2 } else { 1 };
            let position = Pos::new(
                Line(data_row - terminal.display_offset() as i32),
                Column(selected_column),
            );
            content.selection_range = Some(SelectionRange::new(position, position, true));
            let canvas = render(&content, 1.0, 8.0, 14.0);
            let selected: Vec<_> = canvas
                .rects
                .iter()
                .filter(|(_, color)| *color == Colors::default().selection_background)
                .map(|(bounds, _)| bounds)
                .collect();
            assert_eq!(selected.len(), 1, "{value}, width {columns}: native trailing-cell selection must remain visible");
            close(selected[0][0], 16.0);
            close(selected[0][2], width as f32 * 8.0);
        }
    }
}

// AUTOMEXIA_INLINE_PIPELINE_V1
include!("inline_pipeline_render_tests.rs");

#[test]
fn ordinary_table_command_tints_obey_opacity_without_overriding_status_owners() {
    use crate::automexia::ui::CommandResultAnchor;
    use rio_backend::config::presentation::CommandOutputAppearance;
    for (source, semantic) in [
        ("NAME  VALUE\r\nalpha plain-text\r\nbeta  other-text\r\n\r\nprompt ", false),
        ("NAME    VALUE\r\nalpha   warning: delayed\r\nbeta    warning: delayed\r\n\r\nprompt ", true),
        ("NAME  READY  STATUS\r\nalpha 0/1    Unknown\r\nbeta  1/1    Running\r\n\r\nprompt ", true),
    ] {
        let mut terminal = terminal(source, 80, 16);
        let content = content(&mut terminal);
        assert_eq!(content.inline_tables.surfaces.len(), 1);
        let end = content.inline_tables.row_geometry(0, 2, &content.command_rows).unwrap();
        let anchors = [CommandResultAnchor { generation: Some(1), key: 1, x: 8.0,
            y: 8.0 + (end.0 as f32 + end.1 as f32) * 20.0, width: 640.0, height: 20.0,
            output_top: Some(8.0), separates_next_prompt: true, exit_code: Some(1),
            elapsed_ms: Some(10), completed_at: None }];
        let row = content.inline_tables.row_geometry(0, 1, &content.command_rows).unwrap();
        let point = (16.5, 8.0 + row.0 as f32 * 20.0 + 8.0);
        for logs in [false, true] {
            for kube in [false, true] {
                for style in [HighlightStyle::Foreground, HighlightStyle::Background, HighlightStyle::Both] {
                    let options = PaintOptions { highlight: logs.then_some(configured_highlight(style)),
                        kubernetes_highlight: kube.then_some(configured_highlight(style)), ..Default::default() };
                    let original = render_with_options(&content, 1.0, 8.0, 14.0, options);
                    let base = painted_background(&original, point.0, point.1);
                    for alpha in [0, 64, 255] {
                        let appearance = CommandOutputAppearance { failure: Some(Rgba::from_bytes([240, 10, 180, alpha])),
                            pulse: false, ..Default::default() };
                        let canvas = render_with_options(&content, 1.0, 8.0, 14.0, PaintOptions {
                            command_output: Some(appearance), command_results: &anchors, ..options });
                        let actual = painted_background(&canvas, point.0, point.1);
                        if alpha == 0 || (semantic && (logs || source.contains("READY"))) {
                            assert_eq!(actual, base, "semantic owner/transparent color wins");
                        } else {
                            let expected = super::ui_theme::over(base,
                                [240.0 / 255.0, 10.0 / 255.0, 180.0 / 255.0, f32::from(alpha) / 255.0]);
                            assert_eq!(actual, expected);
                            assert_ne!(actual, base, "completed ordinary table must be tinted");
                        }
                        assert_eq!(painted_background(&canvas, 16.5, 16.5),
                            painted_background(&original, 16.5, 16.5), "headers keep hierarchy");
                    }
                }
            }
        }
    }
}

#[test]
fn table_command_tints_cover_each_owned_row_at_fractional_display_scales() {
    use crate::automexia::ui::CommandResultAnchor;
    use rio_backend::config::presentation::CommandOutputAppearance;
    for prefix in [0, 12] {
        let source = format!("{}NAME  VALUE\r\nalpha ordinary\r\nbeta  another\r\nlast  outside\r\n\r\nprompt ", "\r\n".repeat(prefix));
        let mut terminal = terminal(&source, 80, 32);
        let content = content(&mut terminal);
        assert_eq!(content.inline_tables.surfaces.len(), 1);
        let rows = (0..4)
            .map(|row| {
                content
                    .inline_tables
                    .row_geometry(0, row, &content.command_rows)
                    .unwrap()
            })
            .collect::<Vec<_>>();
        for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
            let y = 77.0 / scale;
            let cell_height = 38.0 / scale;
            let anchors = [CommandResultAnchor {
                generation: Some(1),
                key: 1,
                x: 8.0,
                y: y + (rows[2].0 as f32 + rows[2].1 as f32) * cell_height,
                width: 640.0,
                height: cell_height,
                output_top: Some(y + rows[0].0 as f32 * cell_height),
                separates_next_prompt: true,
                exit_code: Some(0),
                elapsed_ms: None,
                completed_at: None,
            }];
            let mut canvas = RasterCanvas::new(scale);
            draw(
                &mut canvas,
                &content,
                [8.0, y, 8.0, cell_height],
                14.0,
                scale,
                PaintOptions {
                    command_output: Some(CommandOutputAppearance {
                        success: Some(Rgba::from_bytes([32, 64, 224, 255])),
                        pulse: false,
                        ..Default::default()
                    }),
                    command_results: &anchors,
                    ..Default::default()
                },
                source_colors,
            );
            let expected = [32.0 / 255.0, 64.0 / 255.0, 224.0 / 255.0, 1.0];
            for (index, (first, _)) in rows.iter().enumerate() {
                let actual = painted_background(
                    &canvas,
                    16.5,
                    y + (*first as f32 + 0.5) * cell_height,
                );
                if (1..=2).contains(&index) {
                    assert_eq!(
                        actual, expected,
                        "data row {index}, scale {scale}, prefix {prefix}"
                    );
                } else {
                    assert_ne!(
                        actual, expected,
                        "header and rows outside the command stay unchanged"
                    );
                }
            }
        }
    }
}
