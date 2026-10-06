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

#[path = "grid_visual_quality_tests.rs"]
mod visual_quality_tests;

#[test]
fn concealed_runs_preserve_cells_without_emitting_ink_through_selection_or_hints() {
    let fonts = crate::visual_quality::fonts();
    let renderer = fixture_renderer(true);
    let mut rasterizer = GridGlyphRasterizer::new();
    let mut grid = GridRenderer::Cpu(CpuGridRenderer::new(80, 12));
    for text in [
        "secret",
        "e\u{301}",
        "中文",
        "👩\u{200d}💻",
        "─━█",
        "\u{e0b0}",
    ] {
        for attributes in ["8", "8;4;9", "8;7;1;3;38;2;255;0;0"] {
            let mut term = terminal(&format!("L\x1b[{attributes}m{text}\x1b[0mR"));
            let last = term.cursor().pos.col.0 as u16 - 1;
            let (rows, styles, extras) = snapshot(&mut term);
            for selected in [false, true] {
                for tag in [
                    HintTag::Match,
                    HintTag::Focused,
                    HintTag::HyperlinkHover,
                    HintTag::Label,
                ] {
                    let mut glyphs = Vec::new();
                    build_row_fg_classified(
                        &rows[0],
                        80,
                        0,
                        &styles,
                        &extras,
                        &renderer,
                        &TermColors::default(),
                        &mut rasterizer,
                        &mut grid,
                        16.0,
                        10.0,
                        24.0,
                        selected.then_some(RowSelection { lo: 0, hi: last }),
                        &[RowHint {
                            lo: 0,
                            hi: last,
                            tag,
                        }],
                        &fonts,
                        0,
                        Some(1),
                        &mut glyphs,
                        None,
                        &[true; 80],
                    );
                    assert!(!glyphs.is_empty());
                    assert!(glyphs.iter().all(|glyph| glyph.grid_pos[0] == 0 || glyph.grid_pos[0] == last),
                            "concealed glyph/decorations leaked for {attributes}, selection={selected}, hint={tag:?}");
                    assert!(
                        glyphs.iter().any(|glyph| glyph.grid_pos[0] == last),
                        "the visible suffix must retain its original cell position"
                    );
                }
            }
        }
    }
}

#[test]
fn parsed_combining_marks_shape_without_moving_following_cells() {
    let mut data = FontLibraryData::default();
    data.insert(FontData::from_static_slice(constants::FONT_CASCADIA_CODE_NF).unwrap());
    let fonts = FontLibrary {
        inner: Arc::new(parking_lot::RwLock::new(data)),
    };
    let mut rasterizer = GridGlyphRasterizer::new();
    let mut grid = GridRenderer::Cpu(CpuGridRenderer::new(80, 12));
    let renderer = fixture_renderer(false);
    let mut emitted = Vec::new();
    for source in ["e\u{301}X", "éX"] {
        let (rows, styles, extras) = snapshot(&mut terminal(source));
        let mut glyphs = Vec::new();
        build_row_fg_classified(
            &rows[0],
            80,
            0,
            &styles,
            &extras,
            &renderer,
            &TermColors::default(),
            &mut rasterizer,
            &mut grid,
            24.0,
            14.0,
            32.0,
            None,
            &[],
            &fonts,
            0,
            None,
            &mut glyphs,
            None,
            &[],
        );
        emitted.push(
            glyphs
                .iter()
                .map(|g| (g.grid_pos, g.glyph_pos, g.glyph_size, g.bearings))
                .collect::<Vec<_>>(),
        );
    }
    assert_eq!(
        emitted[0], emitted[1],
        "combining input must paint the same glyphs and cells as its composed form"
    );
    assert_eq!(emitted[0].last().unwrap().0, [1, 0]);
}

#[cfg(any(windows, target_os = "macos"))]
#[test]
fn parsed_emoji_zwj_reaches_one_native_color_glyph() {
    let mut data = FontLibraryData::default();
    data.insert(FontData::from_static_slice(constants::FONT_CASCADIA_CODE_NF).unwrap());
    let fonts = FontLibrary {
        inner: Arc::new(parking_lot::RwLock::new(data)),
    };
    let mut term = terminal("👩\u{200d}💻X");
    assert_eq!(
        term.cursor().pos.col.0,
        5,
        "rendering must preserve legacy PTY cell widths"
    );
    let (rows, styles, extras) = snapshot(&mut term);
    let mut glyphs = Vec::new();
    build_row_fg_classified(
        &rows[0],
        80,
        0,
        &styles,
        &extras,
        &fixture_renderer(false),
        &TermColors::default(),
        &mut GridGlyphRasterizer::new(),
        &mut GridRenderer::Cpu(CpuGridRenderer::new(80, 12)),
        24.0,
        14.0,
        32.0,
        None,
        &[],
        &fonts,
        0,
        None,
        &mut glyphs,
        None,
        &[],
    );
    let emoji: Vec<_> = glyphs
        .iter()
        .filter(|g| g.atlas == CellText::ATLAS_COLOR)
        .collect();
    assert_eq!(
        emoji.len(),
        1,
        "a ZWJ cluster must reach the native shaper intact"
    );
    assert_eq!(emoji[0].grid_pos, [0, 0]);
    assert!(
        glyphs.iter().any(|g| g.grid_pos == [4, 0]),
        "following text must retain its PTY column"
    );
}

#[test]
fn non_pod_kubernetes_colors_survive_raw_soft_wrapping() {
    use crate::automexia::api::SemanticSeverity::{Error, Info, Success, Warning};
    use crate::automexia::output_semantics::{OutputClassification, OutputDomain};
    for (header, data, severity) in [
        (
            "NAME  READY  UP-TO-DATE  AVAILABLE  AGE",
            "api   1/3    3           1          1d",
            Warning,
        ),
        (
            "NAME  DESIRED  CURRENT  READY  AGE",
            "api   3        3        3      1d",
            Success,
        ),
        ("NAME  READY  AGE", "db    2/2    1d", Success),
        (
            "NAME  STATUS  COMPLETIONS  DURATION  AGE",
            "job   Failed  0/1          1m        1m",
            Error,
        ),
        (
            "NAME  STATUS  VOLUME  CAPACITY  ACCESS MODES  STORAGECLASS  AGE",
            "data  Lost    disk    1Gi       RWO           standard      1d",
            Error,
        ),
        ("NAME  STATUS       AGE", "demo  Terminating  1d", Warning),
        (
            "NAME  SERVICE       AVAILABLE                 AGE",
            "api   demo/metrics  False (MissingEndpoints)  1d",
            Error,
        ),
        (
            "NAME  TYPE       CLUSTER-IP  EXTERNAL-IP  PORT(S)  AGE",
            "api   ClusterIP  10.0.0.1    <none>       80/TCP   1d",
            Info,
        ),
    ] {
        for width in [12, 32, 120] {
            let mut terminal = Crosswords::new(
                CrosswordsSize::new(width, 40),
                rio_backend::ansi::CursorShape::Block,
                VoidListener {},
                WindowId::from(0),
                0,
                128,
            );
            Processor::default()
                .advance(&mut terminal, format!("{header}\r\n{data}\r\n").as_bytes());
            let (rows, _, _) = snapshot(&mut terminal);
            let mut classified = Vec::new();
            classify_visible_output(
                &rows,
                width,
                false,
                &mut classified,
                &mut String::new(),
            );
            let start = header.len().div_ceil(width);
            let end = start + data.len().div_ceil(width);
            assert!(
                classified[start..end].iter().all(|value| *value
                    == Some(OutputClassification {
                        domain: OutputDomain::Kubernetes,
                        severity: Some(severity),
                    })),
                "{header}: {data} at {width}: {:?}",
                &classified[start..end]
            );
        }
    }
}

#[test]
fn wrapped_kubernetes_rows_keep_domain_status_and_independent_gate() {
    use crate::automexia::output_semantics::{OutputClassification, OutputDomain};
    let source = "pod-a  0/1  Running  0  1m\r\npod-b  1/1  FutureState error\r\n";
    for width in [8, 12, 80] {
        let mut terminal = Crosswords::new(
            CrosswordsSize::new(width, 20),
            rio_backend::ansi::CursorShape::Block,
            VoidListener {},
            WindowId::from(0),
            0,
            128,
        );
        Processor::default().advance(&mut terminal, source.as_bytes());
        let (rows, _, _) = snapshot(&mut terminal);
        let mut classified = Vec::new();
        classify_visible_output(&rows, width, false, &mut classified, &mut String::new());
        let first_height = "pod-a  0/1  Running  0  1m".len().div_ceil(width);
        let second_height = "pod-b  1/1  FutureState error".len().div_ceil(width);
        for class in &classified[..first_height] {
            assert_eq!(
                *class,
                Some(OutputClassification {
                    domain: OutputDomain::Kubernetes,
                    severity: Some(crate::automexia::api::SemanticSeverity::Warning),
                })
            );
        }
        for class in &classified[first_height..first_height + second_height] {
            assert_eq!(
                *class,
                Some(OutputClassification {
                    domain: OutputDomain::Kubernetes,
                    severity: None,
                })
            );
        }
        let mut renderer = fixture_renderer(true);
        renderer.presentation.kubernetes_highlighting = false;
        assert_eq!(semantic_classification_fg(classified[0], &renderer), None);
        renderer.presentation.output_highlighting = false;
        renderer.presentation.kubernetes_highlighting = true;
        assert_eq!(
            semantic_classification_fg(classified[0], &renderer),
            Some([255, 255, 0, 255])
        );
    }
}

#[test]
fn visible_output_does_not_join_hard_lines_or_color_incomplete_wraps() {
    let mut terminal = terminal("api 0/1\r\nRunning\r\n[ERROR] fixture\r\n");
    let (rows, _, _) = snapshot(&mut terminal);
    let mut classified = Vec::new();
    classify_visible_output(&rows, 80, false, &mut classified, &mut String::new());
    assert_eq!(classified[0], None);
    assert_eq!(classified[1], None);
    assert!(classified[2].is_some());

    let mut narrow = Crosswords::new(
        CrosswordsSize::new(8, 12),
        rio_backend::ansi::CursorShape::Block,
        VoidListener {},
        WindowId::from(0),
        0,
        128,
    );
    Processor::default().advance(&mut narrow, b"pod-a 0/1 Running\r\n");
    let (rows, _, _) = snapshot(&mut narrow);
    classify_visible_output(&rows, 8, true, &mut classified, &mut String::new());
    assert!(classified[..3]
        .iter()
        .all(|entry| entry.is_some_and(|entry| entry.domain
            == crate::automexia::output_semantics::OutputDomain::Uncertain)));
    classify_visible_output(&rows[..1], 8, false, &mut classified, &mut String::new());
    assert_eq!(classified.len(), 1);
    assert_eq!(
        classified[0].unwrap().domain,
        crate::automexia::output_semantics::OutputDomain::Uncertain
    );
}

#[test]
fn semantic_prompt_and_editable_input_are_never_output_status_rows() {
    use rio_backend::crosswords::grid::row::SemanticPrompt;
    let mut terminal =
        terminal("[ERROR] prompt\r\napi 0/1 Running\r\n[ERROR] actual output\r\n");
    let (mut rows, _, _) = snapshot(&mut terminal);
    rows[0].set_semantic_prompt(SemanticPrompt::Prompt, Some(7));
    rows[1].set_semantic_prompt(SemanticPrompt::PromptContinuation, Some(7));
    let mut classified = Vec::new();
    classify_visible_output(&rows, 80, false, &mut classified, &mut String::new());
    assert!(classified[..2]
        .iter()
        .all(|entry| entry.is_some_and(|entry| entry.domain
            == crate::automexia::output_semantics::OutputDomain::Uncertain)));
    assert!(classified[2].is_some());
    let renderer = fixture_renderer(true);
    for row in &rows[..2] {
        assert_eq!(
            semantic_row_fg(row, 80, &renderer, &mut String::new()),
            None
        );
    }
}

#[test]
fn wrapped_prompt_boundary_does_not_absorb_first_plain_output() {
    use crate::automexia::api::SemanticSeverity;
    use crate::automexia::output_semantics::OutputDomain;
    use rio_backend::crosswords::grid::row::SemanticPrompt;
    let mut term =
        terminal("command continuation\r\n/workspace/project\r\n[ERROR] fixture\r\n");
    let (mut rows, _, _) = snapshot(&mut term);
    // ConPTY may leave a wrap marker after repainting a submitted command.
    // The shell boundary already identifies the next row as output.
    rows[0].set_semantic_prompt(SemanticPrompt::PromptContinuation, Some(7));
    rows[0].inner[79].set_wrapline(true);
    for clipped_prefix in [false, true] {
        let mut classified = Vec::new();
        classify_visible_output(
            &rows,
            80,
            clipped_prefix,
            &mut classified,
            &mut String::new(),
        );
        assert_eq!(classified[0].unwrap().domain, OutputDomain::Uncertain);
        assert_eq!(
            classified[1].unwrap().severity,
            Some(SemanticSeverity::Info)
        );
        assert_eq!(
            classified[2].unwrap().severity,
            Some(SemanticSeverity::Error)
        );
    }
    // A prompt also retires a partial output prefix; it cannot lend status to
    // subsequent output or be interpreted as output itself.
    rows[0].set_semantic_prompt(SemanticPrompt::None, None);
    rows[1].set_semantic_prompt(SemanticPrompt::Prompt, Some(8));
    rows[1].inner[79].set_wrapline(true);
    let mut classified = Vec::new();
    classify_visible_output(&rows, 80, false, &mut classified, &mut String::new());
    assert_eq!(classified[0].unwrap().domain, OutputDomain::Uncertain);
    assert_eq!(classified[1].unwrap().domain, OutputDomain::Uncertain);
    assert_eq!(
        classified[2].unwrap().severity,
        Some(SemanticSeverity::Error)
    );
}

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
    renderer.presentation.output_highlighting = enabled;
    renderer.presentation.kubernetes_highlighting = enabled;
    renderer
}

#[test]
fn plain_output_colors_follow_logical_rows_without_coloring_prompts() {
    use crate::automexia::api::SemanticSeverity;
    use crate::automexia::output_semantics::{OutputClassification, OutputDomain};
    for text in [
        "/workspace/a-long-project-name/subdirectory",
        "No resources found in demo namespace.",
    ] {
        for width in [12, 80] {
            let mut terminal = Crosswords::new(
                CrosswordsSize::new(width, 12),
                rio_backend::ansi::CursorShape::Block,
                VoidListener {},
                WindowId::from(0),
                0,
                128,
            );
            Processor::default().advance(
                &mut terminal,
                format!("{text}\r\n\x1b]133;A\x07/workspace/prompt\x1b]133;B\x07")
                    .as_bytes(),
            );
            let (rows, _, _) = snapshot(&mut terminal);
            let mut classified = Vec::new();
            classify_visible_output(
                &rows,
                width,
                false,
                &mut classified,
                &mut String::new(),
            );
            let output_rows = text.len().div_ceil(width);
            assert!(classified[..output_rows].iter().all(|value| *value
                == Some(OutputClassification {
                    domain: OutputDomain::General,
                    severity: Some(SemanticSeverity::Info),
                })));
            assert_ne!(
                rows[output_rows].semantic_prompt,
                rio_backend::crosswords::grid::row::SemanticPrompt::None
            );
            assert!(classified[output_rows..]
                .iter()
                .all(|entry| entry
                    .is_none_or(|entry| entry.domain == OutputDomain::Uncertain)));
        }
    }
}

#[test]
fn plain_shell_output_reaches_retained_grid_and_obeys_customization() {
    use rio_backend::config::presentation::{HighlightStyle, Rgb, Rgba};
    let mut terminal = terminal(
        "/workspace/project\r\nNo resources found in demo namespace.\r\nlogout\r\n",
    );
    let (rows, styles, _) = snapshot(&mut terminal);
    let mut renderer = fixture_renderer(true);
    renderer.presentation.kubernetes_highlighting = false;
    renderer.presentation.command_output_highlighting = false;
    renderer.presentation.highlight.colors.info = Some(Rgb::from_bytes([21, 132, 243]));
    renderer.presentation.highlight.info_background =
        Some(Rgba::from_bytes([12, 34, 56, 78]));
    for style in [
        HighlightStyle::Foreground,
        HighlightStyle::Background,
        HighlightStyle::Both,
    ] {
        renderer.presentation.highlight.style = style;
        for enabled in [false, true] {
            renderer.presentation.output_highlighting = enabled;
            for row in rows.iter().take(3) {
                assert_eq!(
                    semantic_row_fg(row, 80, &renderer, &mut String::new()),
                    (enabled && style != HighlightStyle::Background)
                        .then_some([21, 132, 243, 255])
                );
                let mut backgrounds = Vec::new();
                build_row_bg(
                    row,
                    80,
                    &styles,
                    &renderer,
                    &TermColors::default(),
                    None,
                    &[],
                    &mut GridGlyphRasterizer::new(),
                    &mut backgrounds,
                );
                let expected = if enabled && style != HighlightStyle::Foreground {
                    [12, 34, 56, 78]
                } else {
                    cell_bg(
                        row[Column(0)],
                        resolve_style(&styles, row[Column(0)]),
                        &renderer,
                        &TermColors::default(),
                    )
                };
                assert_eq!(backgrounds[0].rgba, expected);
            }
        }
    }
}

#[test]
fn plain_shell_errors_reach_glyphs_with_independent_domains() {
    use crate::automexia::output_semantics::OutputDomain;
    let mut data = FontLibraryData::default();
    data.insert(FontData::from_static_slice(constants::FONT_CASCADIA_CODE_NF).unwrap());
    let fonts = FontLibrary {
        inner: Arc::new(parking_lot::RwLock::new(data)),
    };
    let cases = [
        ("Error from server (NotFound): pods fixture not found", OutputDomain::General),
        ("pod/fixture 0/1 CrashLoopBackOff 0 1m", OutputDomain::Kubernetes),
        ("bash: missing-fixture: command not found", OutputDomain::General),
        ("zsh: command not found: missing-fixture", OutputDomain::General),
        ("fish: Unknown command: missing-fixture", OutputDomain::General),
        ("'missing-fixture' is not recognized as an internal or external command", OutputDomain::General),
        ("missing-fixture : The term 'missing-fixture' is not recognized as the name of a cmdlet", OutputDomain::General),
        ("missing-fixture : The term 'missing-fixture' is not recognized as a name of a cmdlet", OutputDomain::General),
    ];
    for (text, domain) in cases {
        let mut term = terminal(&format!("\x1b]133;A;aid=1\x07> \x1b]133;B\x07run -a\r\n\x1b]133;C\x07{text}\r\n\x1b]133;D;1\x07"));
        let (rows, styles, extras) = snapshot(&mut term);
        let mut classified = Vec::new();
        classify_visible_output(&rows, 80, false, &mut classified, &mut String::new());
        assert_eq!(classified[1].unwrap().domain, domain, "{text}");
        let mut renderer = fixture_renderer(true);
        // Only the owning switch is on. Command result bands stay disabled.
        renderer.presentation.output_highlighting = domain == OutputDomain::General;
        renderer.presentation.kubernetes_highlighting =
            domain == OutputDomain::Kubernetes;
        renderer.presentation.command_output_highlighting = false;
        let mut glyphs = Vec::new();
        build_row_fg_classified(
            &rows[1],
            80,
            1,
            &styles,
            &extras,
            &renderer,
            &TermColors::default(),
            &mut GridGlyphRasterizer::new(),
            &mut GridRenderer::Cpu(CpuGridRenderer::new(80, 12)),
            16.0,
            10.0,
            24.0,
            None,
            &[],
            &fonts,
            0,
            None,
            &mut glyphs,
            classified[1],
            &[],
        );
        assert!(!glyphs.is_empty());
        assert!(
            glyphs.iter().all(|glyph| glyph.color == [255, 0, 0, 255]),
            "{text}"
        );
        renderer.presentation.output_highlighting = domain != OutputDomain::General;
        renderer.presentation.kubernetes_highlighting =
            domain != OutputDomain::Kubernetes;
        assert_eq!(semantic_classification_fg(classified[1], &renderer), None);
    }
}

#[test]
fn typed_input_accents_reach_glyphs_preserve_native_styles_and_selection() {
    use rio_backend::crosswords::grid::row::{PromptInputShell, SemanticInput};
    let mut terminal = terminal("> docker ps -a\r\n> \x1b[38;2;7;19;31mdocker\x1b[0m ps -a\r\n> \x1b[2mdocker\x1b[0m ps -a\r\n> \x1b[7mdocker\x1b[0m ps -a\r\n> \x1b[8mdocker\x1b[0m ps -a\r\ndocker ps -a");
    let (mut rows, styles, extras) = snapshot(&mut terminal);
    for row in &mut rows[..5] {
        row.semantic_input = Some(SemanticInput {
            column: 2,
            shell: PromptInputShell::Cmd,
            command_complete: false,
            continuation: false,
        });
    }
    let mut data = FontLibraryData::default();
    data.insert(FontData::from_static_slice(constants::FONT_CASCADIA_CODE_NF).unwrap());
    let fonts = FontLibrary {
        inner: Arc::new(parking_lot::RwLock::new(data)),
    };
    let renderer = fixture_renderer(false);
    let mut grid = GridRenderer::Cpu(CpuGridRenderer::new(80, 12));
    let mut rasterizer = GridGlyphRasterizer::new();
    let mut glyphs = Vec::new();
    for selected in [false, true] {
        for (y, row) in rows.iter().take(6).enumerate() {
            let selection = selected.then_some(RowSelection { lo: 0, hi: 79 });
            build_row_fg(
                row,
                80,
                y as u16,
                &styles,
                &extras,
                &renderer,
                &TermColors::default(),
                &mut rasterizer,
                &mut grid,
                16.0,
                10.0,
                24.0,
                selection,
                &[],
                &fonts,
                0,
                None,
                &mut glyphs,
            );
            assert!(!glyphs.is_empty());
            let accent_count = glyphs
                .iter()
                .filter(|glyph| glyph.color == [181, 140, 255, 255])
                .count();
            if selected || y == 5 {
                assert_eq!(accent_count, 0);
            } else if y == 0 {
                assert_eq!(
                    accent_count, 8,
                    "command plus option, no positional argument"
                );
            } else {
                assert_eq!(
                    accent_count, 2,
                    "only the option, native command style wins"
                );
            }
            if !selected && y == 1 {
                assert_eq!(
                    glyphs
                        .iter()
                        .filter(|glyph| glyph.color == [7, 19, 31, 255])
                        .count(),
                    6
                );
            }
        }
    }
}

#[test]
fn independent_kubernetes_grid_palette_preserves_ansi_inverse_and_selection() {
    use rio_backend::config::presentation::{HighlightStyle, Rgb, Rgba};
    let mut terminal = terminal("\x1b[38;2;7;19;31;48;2;41;53;67mpod\x1b[0m-a 0/1 Running\r\n\x1b[7mpod-b 0/1 Running\x1b[0m\r\n");
    let (rows, styles, _) = snapshot(&mut terminal);
    let mut renderer = fixture_renderer(true);
    renderer.presentation.output_highlighting = false;
    renderer.presentation.highlight.colors.warning = Some(Rgb::from_bytes([190, 1, 2]));
    renderer.presentation.kubernetes.style = HighlightStyle::Both;
    renderer.presentation.kubernetes.colors.warning = Some(Rgb::from_bytes([3, 191, 5]));
    renderer.presentation.kubernetes.warning_background =
        Some(Rgba::from_bytes([11, 23, 37, 128]));
    let mut classified = Vec::new();
    classify_visible_output(&rows, 80, false, &mut classified, &mut String::new());
    let semantic = semantic_classification_fg(classified[0], &renderer);
    assert_eq!(semantic, Some([3, 191, 5, 255]));
    let square = rows[0][Column(0)];
    assert_eq!(
        semantic_or_cell_fg(
            semantic,
            square,
            resolve_style(&styles, square),
            &renderer,
            &TermColors::default()
        ),
        [7, 19, 31, 255]
    );
    let inverted = rows[1][Column(0)];
    assert_eq!(
        semantic_or_cell_fg(
            semantic,
            inverted,
            resolve_style(&styles, inverted),
            &renderer,
            &TermColors::default()
        ),
        cell_fg(
            inverted,
            resolve_style(&styles, inverted),
            &renderer,
            &TermColors::default()
        )
    );
    let mut background = Vec::new();
    build_row_bg_classified(
        &rows[0],
        80,
        &styles,
        &renderer,
        &TermColors::default(),
        Some(RowSelection { lo: 4, hi: 5 }),
        &[],
        &mut GridGlyphRasterizer::new(),
        &mut background,
        classified[0],
    );
    assert_eq!(background[0].rgba, [41, 53, 67, 255]);
    assert_eq!(background[3].rgba, [11, 23, 37, 128]);
    assert_eq!(
        background[4].rgba,
        normalized_to_u8(renderer.named_colors.selection_background)
    );
    renderer.presentation.kubernetes_highlighting = false;
    assert_eq!(semantic_classification_fg(classified[0], &renderer), None);
    assert!(
        classified[0].is_some(),
        "disabling paint must not erase row ownership"
    );
}

#[test]
fn semantic_background_never_overwrites_compact_explicit_ansi_fill_cells() {
    let renderer = fixture_renderer(true);
    let mut rgb = Square::default();
    rgb.set_bg_rgb(17, 29, 43);
    let mut indexed = Square::default();
    indexed.set_bg_palette(1);
    for square in [rgb, indexed] {
        let source = cell_bg(square, Style::default(), &renderer, &TermColors::default());
        assert_eq!(
            semantic_or_cell_bg(
                Some([200, 210, 220, 128]),
                square,
                Style::default(),
                &renderer,
                &TermColors::default()
            ),
            source
        );
    }
}

#[test]
fn compact_background_gap_does_not_join_kubernetes_ready_and_status_fields() {
    use crate::automexia::api::SemanticSeverity;
    use crate::automexia::output_semantics::OutputDomain;
    let mut terminal = terminal("api 0/1 Running\r\nprompt ");
    Processor::default().advance(
        &mut terminal,
        b"\x1b7\x1b[1;8H\x1b[48;2;17;29;43m\x1b[X\x1b[0m\x1b8",
    );
    assert!(terminal.grid[Line(0)][Column(7)].is_bg_only());
    let (rows, _, _) = snapshot(&mut terminal);
    let mut classified = Vec::new();
    classify_visible_output(&rows, 80, false, &mut classified, &mut String::new());
    let classification =
        classified[0].expect("erased colored space retains column separation");
    assert_eq!(classification.domain, OutputDomain::Kubernetes);
    assert_eq!(classification.severity, Some(SemanticSeverity::Warning));
}

#[test]
fn visible_output_classification_is_bounded_and_fresh_after_same_cursor_updates() {
    use crate::automexia::api::SemanticSeverity;
    let mut terminal = terminal("api 0/1 Unknown\r\nprompt ");
    let (rows, _, _) = snapshot(&mut terminal);
    let mut classified = Vec::new();
    let mut scratch = String::new();
    classify_visible_output(&rows, 80, false, &mut classified, &mut scratch);
    assert_eq!(
        classified[0].unwrap().severity,
        Some(SemanticSeverity::Warning)
    );
    let cursor = terminal.cursor().pos;
    Processor::default().advance(
        &mut terminal,
        b"\x1b[A\r\x1b[2Kapi 1/1 Running\x1b[B\r\x1b[7C",
    );
    assert_eq!(terminal.cursor().pos, cursor);
    let (rows, _, _) = snapshot(&mut terminal);
    classify_visible_output(&rows, 80, false, &mut classified, &mut scratch);
    assert_eq!(
        classified[0].unwrap().severity,
        Some(SemanticSeverity::Success)
    );
    let many = vec![rows[0].clone(); 1025];
    classify_visible_output(&many, 80, false, &mut classified, &mut scratch);
    assert_eq!(classified.len(), 1024);
    assert!(classified[819..]
        .iter()
        .all(|entry| entry.is_some_and(|entry| entry.domain
            == crate::automexia::output_semantics::OutputDomain::Uncertain)));
    assert!(scratch.len() <= crate::automexia::output_semantics::MAX_ROW_BYTES);
}

#[test]
fn core_output_highlighting_survives_disabled_optional_context() {
    let mut renderer = fixture_renderer(true);
    renderer.devops_context_enabled = false;
    let mut terminal = terminal("[ERROR] fixture failed\r\n");
    let (rows, _, _) = snapshot(&mut terminal);
    assert_eq!(
        semantic_row_severity(&rows[0], 80, &renderer, &mut String::new()),
        Some(crate::automexia::api::SemanticSeverity::Error),
    );
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
fn presentation_highlighting_toggle_preserves_optional_context_and_native_output() {
    let mut terminal = terminal("[ERROR] fixture failed\r\n");
    let (rows, _, _) = snapshot(&mut terminal);
    let config: Config =
        toml::from_str("[presentation]\noutput-highlighting = false\n").unwrap();
    let mut renderer = Renderer::new(&config);
    renderer.devops_context_enabled = true;
    assert_eq!(
        semantic_row_severity(&rows[0], 80, &renderer, &mut String::new()),
        None
    );
    assert!(renderer.devops_context_enabled);
    renderer.update_config(&Config::default());
    renderer.devops_context_enabled = true;
    assert_eq!(
        semantic_row_severity(&rows[0], 80, &renderer, &mut String::new()),
        Some(crate::automexia::api::SemanticSeverity::Error)
    );
    renderer.presentation.output_highlighting = false;
    assert_eq!(
        semantic_row_severity(&rows[0], 80, &renderer, &mut String::new()),
        None
    );
}

fn visual_renderer(style: &str) -> Renderer {
    let source = format!("[presentation.highlight]\nstyle = '{style}'\nerror-background = '#17293b4d'\nwarning-background = '#51637587'\n[presentation.highlight.colors]\nerror = '#102030'\nwarning = '#405060'\nsuccess = '#708090'\ninfo = '#a0b0c0'\ndebug = '#d0e0f0'\n");
    let config: Config = toml::from_str(&source).unwrap();
    let mut renderer = Renderer::new(&config);
    // Existing raster fixtures exercise one common palette for both domains.
    // Independent-domain tests below use distinct palettes and switches.
    renderer.presentation.kubernetes = renderer.presentation.highlight;
    renderer
}

#[test]
fn visual_render_custom_palette_reaches_real_glyph_and_background_emission() {
    let cases = [
        (
            "[ERROR] fixture failed",
            [16, 32, 48, 255],
            Some([23, 41, 59, 77]),
        ),
        (
            "api 0/1 Running",
            [64, 80, 96, 255],
            Some([81, 99, 117, 135]),
        ),
        ("pod/api 1/1 Running", [112, 128, 144, 255], None),
        ("batch 0/1 Completed", [160, 176, 192, 255], None),
        ("level=debug fixture", [208, 224, 240, 255], None),
        ("/workspace/project", [160, 176, 192, 255], None),
        (
            "No resources found in demo namespace.",
            [160, 176, 192, 255],
            None,
        ),
    ];
    let source = cases
        .iter()
        .map(|case| case.0)
        .collect::<Vec<_>>()
        .join("\r\n");
    let mut terminal = terminal(&source);
    let cursor = terminal.cursor();
    let (rows, styles, extras) = snapshot(&mut terminal);
    let mut data = FontLibraryData::default();
    data.insert(FontData::from_static_slice(constants::FONT_CASCADIA_CODE_NF).unwrap());
    let fonts = FontLibrary {
        inner: Arc::new(parking_lot::RwLock::new(data)),
    };
    let renderer = visual_renderer("both");
    for scale in [1.0, 1.25, 2.0] {
        let mut grid = GridRenderer::Cpu(CpuGridRenderer::new(80, 12));
        let mut rasterizer = GridGlyphRasterizer::new();
        let mut glyphs = Vec::new();
        let mut backgrounds = Vec::new();
        for (y, (_, expected, background)) in cases.iter().enumerate() {
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
            assert!(!glyphs.is_empty(), "real shaped row emits glyphs");
            assert!(
                glyphs.iter().all(|glyph| glyph.color == *expected),
                "custom palette on row {y}, scale {scale}"
            );
            build_row_bg(
                &rows[y],
                80,
                &styles,
                &renderer,
                &TermColors::default(),
                None,
                &[],
                &mut rasterizer,
                &mut backgrounds,
            );
            if let Some(expected) = background {
                assert!(backgrounds.iter().all(|cell| cell.rgba == *expected));
            } else {
                assert_eq!(
                    backgrounds[0].rgba,
                    cell_bg(
                        rows[y][Column(0)],
                        resolve_style(&styles, rows[y][Column(0)]),
                        &renderer,
                        &TermColors::default()
                    )
                );
            }
        }
    }
    assert_eq!(terminal.cursor(), cursor);
}

#[test]
fn visual_render_style_and_disable_controls_gate_consumed_colors() {
    let mut terminal = terminal("[ERROR] fixture failed\r\n");
    let (rows, styles, _) = snapshot(&mut terminal);
    for style in ["foreground", "background", "both"] {
        for enabled in [false, true] {
            let mut renderer = visual_renderer(style);
            renderer.presentation.output_highlighting = enabled;
            renderer.presentation.kubernetes_highlighting = enabled;
            let foreground = semantic_row_fg(&rows[0], 80, &renderer, &mut String::new());
            assert_eq!(
                foreground,
                (enabled && style != "background").then_some([16, 32, 48, 255])
            );
            let mut background = Vec::new();
            build_row_bg(
                &rows[0],
                80,
                &styles,
                &renderer,
                &TermColors::default(),
                None,
                &[],
                &mut GridGlyphRasterizer::new(),
                &mut background,
            );
            let expected = if enabled && style != "foreground" {
                [23, 41, 59, 77]
            } else {
                cell_bg(
                    rows[0][Column(0)],
                    resolve_style(&styles, rows[0][Column(0)]),
                    &renderer,
                    &TermColors::default(),
                )
            };
            assert!(
                background.iter().all(|cell| cell.rgba == expected),
                "style {style}, enabled {enabled}"
            );
        }
    }
}

#[test]
fn visual_render_background_style_highlights_each_recognized_severity() {
    let cases = [
        "[ERROR] fixture failed",
        "api 0/1 Running",
        "pod/api 1/1 Running",
        "batch 0/1 Completed",
        "level=debug fixture",
    ];
    let mut terminal = terminal(&cases.join("\r\n"));
    let (rows, styles, _) = snapshot(&mut terminal);
    let renderer = visual_renderer("background");
    for (y, label) in cases.iter().enumerate() {
        assert_eq!(
            semantic_row_fg(&rows[y], 80, &renderer, &mut String::new()),
            None
        );
        let mut backgrounds = Vec::new();
        build_row_bg(
            &rows[y],
            80,
            &styles,
            &renderer,
            &TermColors::default(),
            None,
            &[],
            &mut GridGlyphRasterizer::new(),
            &mut backgrounds,
        );
        let regular = cell_bg(
            rows[y][Column(0)],
            resolve_style(&styles, rows[y][Column(0)]),
            &renderer,
            &TermColors::default(),
        );
        assert_ne!(
            backgrounds[0].rgba, regular,
            "background style lost {label}"
        );
    }
}

#[test]
fn visual_render_independent_status_backgrounds_apply_in_combined_style() {
    let config: Config = toml::from_str(
        "[presentation.highlight]\nstyle = 'both'\nsuccess-background = '#11223344'\ninfo-background = '#55667788'\ndebug-background = '#99aabbcc'\n",
    )
    .unwrap();
    let renderer = Renderer::new(&config);
    let cases = [
        ("pod/api 1/1 Running", [17, 34, 51, 68]),
        ("batch 0/1 Completed", [85, 102, 119, 136]),
        ("level=debug fixture", [153, 170, 187, 204]),
    ];
    let mut terminal = terminal(
        &cases
            .iter()
            .map(|(text, _)| *text)
            .collect::<Vec<_>>()
            .join("\r\n"),
    );
    let (rows, styles, _) = snapshot(&mut terminal);
    for (y, (_, expected)) in cases.iter().enumerate() {
        let mut backgrounds = Vec::new();
        build_row_bg(
            &rows[y],
            80,
            &styles,
            &renderer,
            &TermColors::default(),
            None,
            &[],
            &mut GridGlyphRasterizer::new(),
            &mut backgrounds,
        );
        assert!(backgrounds.iter().all(|cell| cell.rgba == *expected));
    }
}

#[test]
fn visual_render_custom_palette_preserves_explicit_ansi_inverse_and_selection() {
    let renderer = visual_renderer("both");
    let mut terminal = terminal("\x1b[38;2;9;19;29m[ERROR] fixture failed\x1b[0m\r\n\x1b[48;2;39;49;59m[ERROR] fixture failed\x1b[0m\r\n\x1b[7m[ERROR] fixture failed\x1b[0m");
    let (rows, styles, _) = snapshot(&mut terminal);
    let semantic = semantic_row_fg(&rows[0], 80, &renderer, &mut String::new());
    let square = rows[0][Column(0)];
    assert_eq!(
        semantic_or_cell_fg(
            semantic,
            square,
            resolve_style(&styles, square),
            &renderer,
            &TermColors::default()
        ),
        [9, 19, 29, 255]
    );
    for y in [1, 2] {
        let mut background = Vec::new();
        build_row_bg(
            &rows[y],
            80,
            &styles,
            &renderer,
            &TermColors::default(),
            None,
            &[],
            &mut GridGlyphRasterizer::new(),
            &mut background,
        );
        let square = rows[y][Column(0)];
        assert_eq!(
            background[0].rgba,
            cell_bg(
                square,
                resolve_style(&styles, square),
                &renderer,
                &TermColors::default()
            )
        );
    }
    let mut background = Vec::new();
    build_row_bg(
        &rows[0],
        80,
        &styles,
        &renderer,
        &TermColors::default(),
        Some(RowSelection { lo: 0, hi: 4 }),
        &[],
        &mut GridGlyphRasterizer::new(),
        &mut background,
    );
    assert_eq!(
        background[0].rgba,
        normalized_to_u8(renderer.named_colors.selection_background)
    );
    assert_eq!(background[5].rgba, [23, 41, 59, 77]);
}

#[test]
fn settings_output_highlighting_switch_gates_live_error_and_success_rendering() {
    use automexia_ui_model::settings::{Change, Edit, SettingId, SettingValue};
    let base = Config::default();
    let preferences = crate::automexia::preferences::UserPreferences::default();
    let market = [crate::automexia::marketplace::MarketItem {
        id: crate::automexia::builtins::devops::ID.into(),
        name: "DevOps".into(),
        description: "Status coloring".into(),
        installed: true,
    }];
    let mut terminal =
        terminal("[ERROR] fixture failed\r\nweb Up 2 minutes (healthy)\r\n");
    let (rows, styles, _) = snapshot(&mut terminal);
    let mut renderer = Renderer::new(&base);
    renderer.devops_context_enabled = true;
    for row in rows.iter().take(2) {
        assert!(semantic_row_fg(row, 80, &renderer, &mut String::new()).is_some());
    }
    let edit = Edit {
        revision: 1,
        id: SettingId::new(automexia_ui_model::settings::OUTPUT_HIGHLIGHTING).unwrap(),
        change: Change::Set(SettingValue::Boolean(false)),
    };
    let disabled =
        crate::settings_catalog::apply_edit(1, &base, &preferences, &market, &edit)
            .unwrap();
    renderer.presentation = disabled.apply_to(&base).presentation;
    assert!(
        renderer.devops_context_enabled,
        "optional context remains enabled"
    );
    for row in rows.iter().take(2) {
        assert_eq!(
            semantic_row_fg(row, 80, &renderer, &mut String::new()),
            None
        );
        assert_eq!(
            semantic_row_severity(row, 80, &renderer, &mut String::new()),
            None
        );
    }
    let mut background = Vec::new();
    build_row_bg(
        &rows[0],
        80,
        &styles,
        &renderer,
        &TermColors::default(),
        None,
        &[],
        &mut GridGlyphRasterizer::new(),
        &mut background,
    );
    assert_eq!(
        background[0].rgba,
        cell_bg(
            rows[0][Column(0)],
            resolve_style(&styles, rows[0][Column(0)]),
            &renderer,
            &TermColors::default(),
        )
    );
    let restored = crate::settings_catalog::apply_edit(
        1,
        &base,
        &disabled,
        &market,
        &Edit {
            change: Change::Reset,
            ..edit
        },
    )
    .unwrap();
    assert_eq!(restored.presentation.output_highlighting, None);
    renderer.presentation = restored.apply_to(&base).presentation;
    for row in rows.iter().take(2) {
        assert!(semantic_row_fg(row, 80, &renderer, &mut String::new()).is_some());
    }
}

#[test]
fn parsed_stacked_marks_keep_the_same_ink_placement_as_ui_text() {
    use rio_backend::sugarloaf::text::{DrawOpts, Text};
    let mut data = FontLibraryData::default();
    data.insert(FontData::from_static_slice(constants::FONT_CASCADIA_CODE_NF).unwrap());
    let fonts = FontLibrary {
        inner: Arc::new(parking_lot::RwLock::new(data)),
    };
    let sample = "a\u{301}\u{301}";
    let (rows, styles, extras) = snapshot(&mut terminal(sample));
    let mut glyphs = Vec::new();
    build_row_fg_classified(
        &rows[0],
        80,
        0,
        &styles,
        &extras,
        &fixture_renderer(false),
        &TermColors::default(),
        &mut GridGlyphRasterizer::new(),
        &mut GridRenderer::Cpu(CpuGridRenderer::new(80, 12)),
        32.0,
        20.0,
        48.0,
        None,
        &[],
        &fonts,
        0,
        None,
        &mut glyphs,
        None,
        &[],
    );
    let mut text = Text::new(&fonts);
    text.init_cpu();
    text.draw(
        0.0,
        0.0,
        sample,
        &DrawOpts {
            font_size: 32.0,
            ..DrawOpts::default()
        },
    );
    let ui = text.instances();
    assert!(
        ui.len() >= 2,
        "fixture must keep a separately positioned mark"
    );
    assert_eq!(glyphs.len(), ui.len());
    for (grid, ui) in glyphs.iter().zip(ui) {
        assert_eq!(
            grid.grid_pos,
            [0, 0],
            "marks stay anchored to the base cell"
        );
        assert_eq!(grid.glyph_size, ui.glyph_size);
        let grid_ink = [grid.bearings[0] as f32, 48.0 - grid.bearings[1] as f32];
        let ui_ink = [
            ui.pos[0] + ui.bearings[0] as f32,
            ui.pos[1] + ui.bearings[1] as f32,
        ];
        for axis in 0..2 {
            assert!(
                (grid_ink[axis] - ui_ink[axis]).abs() <= 0.51,
                "grid and UI disagree about mark placement: {grid_ink:?} vs {ui_ink:?}"
            );
        }
    }
}
