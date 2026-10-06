//! Parser -> retained grid -> actual glyph/background emitter -> CPU raster.
//! These pinned-font frames complement native fallback/IME/compositor captures.
use super::*;

const SCENE: &str = concat!(
    "Automexia visual quality | fixed public fixture\r\n",
    "ASCII: ABCDEFGHIJKLMNOPQRSTUVWXYZ abcdefghijklmnopqrstuvwxyz 0123456789\r\n",
    "Metrics: Il1O0 []{} () <> /\\ _-+ = == != <= >= -> => :: && ||\r\n",
    "Combining: e\u{301} A\u{30a} n\u{303} | composed: é Å ñ | marks: a\u{301}\u{323}\r\n",
    "\x1b[1mBold\x1b[0m  \x1b[3mItalic\x1b[0m  \x1b[1;3mBold italic\x1b[0m  \x1b[2mDim\x1b[0m  \x1b[7mInverse\x1b[0m\r\n",
    "\x1b[4mUnderline\x1b[0m  \x1b[9mStrike\x1b[0m  \x1b[4:3mUndercurl\x1b[0m\r\n",
    "\x1b[31mRed\x1b[32m Green\x1b[33m Yellow\x1b[34m Blue\x1b[35m Magenta\x1b[36m Cyan\x1b[0m\r\n",
    "\x1b[91mBright red\x1b[92m Green\x1b[93m Yellow\x1b[94m Blue\x1b[95m Magenta\x1b[96m Cyan\x1b[0m\r\n",
    "\x1b[38;2;89;210;180mTruecolor 59d2b4\x1b[0m | \x1b[38;5;208m256-color 208\x1b[0m | \x1b[48;2;45;60;85m background \x1b[0m\r\n",
    "┌──────────────────────────┬──────────────────────────┐\r\n",
    "│ Box drawing and seams    │ █▓▒░ ▀▄▌▐ ⠁⠃⠇⡇⣿          │\r\n",
    "├──────────────────────────┼──────────────────────────┤\r\n",
    "│ Literal cells stay fixed │ 100% 23ms 1.25x          │\r\n",
    "└──────────────────────────┴──────────────────────────┘\r\n",
    "Selection: selected text, normal text and trailing spaces.\r\n",
    "SUCCESS completed | WARNING retry | ERROR failed (not color alone)\r\n",
    "\x1b]8;;https://example.invalid/fixture\x1b\\Hyperlink label\x1b]8;;\x1b\\\r\n",
    "Powerline: \u{e0b0}\u{e0b1}\u{e0b2}\u{e0b3} | Nerd icons: \u{f07b} \u{f013} \u{f121}\r\n",
    "Prompt> fixed-input --option=value"
);

const DEFECTS: &[&str] = &[
    "baseline-one-pixel",
    "missing-glyph",
    "clipped-glyph",
    "wrong-foreground",
    "cell-shift",
    "cursor-one-pixel",
    "cursor-hidden",
    "cursor-low-contrast",
    "cell-padding",
    "line-height",
    "underline-one-pixel",
    "ansi-red-one-channel",
    "background-gap",
    "selection-shift",
    "box-seam",
    "powerline-shift",
];

#[test]
fn visual_quality_concealed_text_has_no_glyph_or_decoration_ink_even_when_selected() {
    let colors = crate::automexia::theme_gallery::builtins()
        .remove(0)
        .theme
        .unwrap()
        .colors;
    let hidden = SCENE.replace("selected text", "\x1b[8;4;9mselected text\x1b[28;24;29m");
    let blank = SCENE.replace("selected text", "             ");
    for selected in [false, true] {
        let concealed = render_scene(&hidden, 1.0, colors, selected, "correct").0;
        let expected = render_scene(&blank, 1.0, colors, selected, "correct").0;
        assert!(concealed == expected, "concealed text or decorations reached delivered pixels (selected={selected})");
        assert!(
            concealed != render_scene(SCENE, 1.0, colors, selected, "correct").0,
            "the same unconcealed text must remain visible"
        );
    }
}

fn scene(
    scale: f32,
    colors: rio_backend::config::colors::Colors,
    selected: bool,
    defect: &str,
) -> (Vec<u32>, [u32; 2]) {
    let (pixels, size, _) = render_scene(SCENE, scale, colors, selected, defect);
    (pixels, size)
}

fn render_scene(
    text: &str,
    scale: f32,
    colors: rio_backend::config::colors::Colors,
    selected: bool,
    defect: &str,
) -> (Vec<u32>, [u32; 2], FontLibrary) {
    render_scene_with_fonts(
        text,
        scale,
        colors,
        selected,
        defect,
        crate::visual_quality::fonts(),
    )
}

fn render_scene_with_fonts(
    text: &str,
    scale: f32,
    colors: rio_backend::config::colors::Colors,
    selected: bool,
    defect: &str,
    fonts: FontLibrary,
) -> (Vec<u32>, [u32; 2], FontLibrary) {
    assert!(defect == "correct" || DEFECTS.contains(&defect));
    let mut terminal = Crosswords::new(
        CrosswordsSize::new(80, 20),
        rio_backend::ansi::CursorShape::Block,
        VoidListener {},
        WindowId::from(0),
        0,
        128,
    );
    let mut parser = Processor::default();
    for part in text.as_bytes().chunks(3) {
        parser.advance(&mut terminal, part);
    }
    let (rows, styles, extras) = snapshot(&mut terminal);
    let mut renderer = Renderer::new(&Config {
        colors,
        ..Default::default()
    });
    renderer.named_colors = colors;
    renderer.colors = rio_backend::config::colors::term::List::from(&colors);
    renderer.presentation.output_highlighting = false;
    renderer.presentation.kubernetes_highlighting = false;
    let mut grid = GridRenderer::Cpu(CpuGridRenderer::new(80, 20));
    let mut rasterizer = GridGlyphRasterizer::new();
    let mut glyphs = Vec::new();
    let mut backgrounds = Vec::new();
    for (y, row) in rows.iter().enumerate() {
        let shift = u16::from(defect == "selection-shift");
        let selection = (selected && y == 14).then_some(RowSelection {
            lo: 11 + shift,
            hi: 23 + shift,
        });
        build_row_bg(
            row,
            80,
            &styles,
            &renderer,
            &TermColors::default(),
            selection,
            &[],
            &mut rasterizer,
            &mut backgrounds,
        );
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
            16.0 * scale,
            10.0 * scale,
            24.0 * scale,
            selection,
            &[],
            &fonts,
            0,
            None,
            &mut glyphs,
        );
        if y < text.lines().count().min(20) {
            assert!(!glyphs.is_empty(), "fixture row {y} must contain ink");
        }
        assert!(glyphs
            .iter()
            .all(|g| g.grid_pos[0] < 80 && g.grid_pos[1] < 20));
        if y == 1 {
            match defect {
                "baseline-one-pixel" => {
                    for glyph in &mut glyphs {
                        glyph.bearings[1] += 1;
                    }
                }
                "missing-glyph" => {
                    glyphs.remove(0);
                }
                "clipped-glyph" => {
                    glyphs[0].glyph_size[1] /= 2;
                }
                "wrong-foreground" => {
                    glyphs[0].color = [255, 0, 0, 255];
                }
                "cell-shift" => {
                    glyphs[0].grid_pos[0] += 1;
                }
                _ => {}
            }
        }
        if y == 5 && defect == "underline-one-pixel" {
            glyphs[0].bearings[1] += 1;
        }
        if y == 6 && defect == "ansi-red-one-channel" {
            for glyph in &mut glyphs {
                glyph.color[0] = glyph.color[0].wrapping_sub(1);
            }
        }
        if y == 8 && defect == "background-gap" {
            let default = normalized_to_u8(colors.background.0);
            let background = backgrounds
                .iter_mut()
                .find(|cell| cell.rgba[3] != 0 && cell.rgba[..3] != default[..3])
                .expect("fixture must contain explicit background");
            // Skip implicit transparent cells; erase an actual explicit fill.
            background.rgba = default;
        }
        if y == 9 && defect == "box-seam" {
            glyphs.remove(1);
        }
        if y == 17 && defect == "powerline-shift" {
            for glyph in &mut glyphs {
                glyph.bearings[1] += 1;
            }
        }
        grid.write_row(y as u32, &backgrounds, &glyphs);
    }
    let cursor = terminal.cursor().pos;
    let (_, mut sprite) = cursor_sprite_cell(
        &mut grid,
        CursorRenderStyle::Bar,
        cursor.col.0 as u16,
        cursor.row.0 as u16,
        [90, 210, 180, 255],
        (10.0 * scale).round() as u32,
        (24.0 * scale).round() as u32,
    )
    .unwrap();
    if defect == "cursor-one-pixel" {
        sprite.bearings[0] += 1;
    }
    if defect == "cursor-low-contrast" {
        sprite.color = normalized_to_u8(colors.background.0);
    }
    if defect != "cursor-hidden" {
        grid.set_cursor(&[], &[sprite]);
    }
    let mut uniforms = GridUniforms {
        projection: [0.0; 16],
        grid_padding: [0.0; 4],
        cursor_color: [0.0; 4],
        cursor_bg_color: [0.0; 4],
        cell_size: [10.0 * scale, 24.0 * scale],
        grid_size: [80, 20],
        cursor_pos: [u32::MAX; 2],
        _pad_cursor: [0; 2],
        min_contrast: 0.0,
        flags: 0,
        padding_extend: 0,
        input_colorspace: 0,
    };
    if defect == "cell-padding" {
        uniforms.grid_padding[3] += 1.0;
    }
    if defect == "line-height" {
        uniforms.cell_size[1] += 1.0;
    }
    let size = [(800.0 * scale) as u32, (480.0 * scale) as u32];
    let bg = normalized_to_u8(colors.background.0);
    let packed = (u32::from(bg[0]) << 16) | (u32::from(bg[1]) << 8) | u32::from(bg[2]);
    let mut pixels = vec![packed; (size[0] * size[1]) as usize];
    grid.render_bg_cpu(&mut pixels, size[0], size[1], &uniforms);
    grid.render_text_cpu(&mut pixels, size[0], size[1], &uniforms);
    assert!(pixels.iter().filter(|p| **p != packed).count() > 5000);
    (pixels, size, fonts)
}

#[test]
fn visual_quality_unicode_fallback_golden_matrix() {
    const UNICODE: &str = concat!(
        "Unicode fallback / native font files are bound in metadata\r\n",
        "CJK: 中文 日本語 한국어 | alignment X\r\n",
        "Combining: e\u{301} A\u{30a} a\u{301}\u{323} | precomposed: é Å\r\n",
        "Arabic: مرحبا بالعالم | Hebrew: שלום עולם | Latin 123\r\n",
        "Emoji: 😀 👩\u{200d}💻 👨\u{200d}👩\u{200d}👧\u{200d}👦 👍🏽 ❤️ X\r\n",
        "RTL mixed: abc אבג 123 مرحبا xyz (terminal logical cell order)\r\n",
        "Wide cells: A中B文C | combining: Ae\u{301}B | end X"
    );
    let colors = crate::automexia::theme_gallery::builtins()
        .remove(0)
        .theme
        .unwrap()
        .colors;
    // Keep the four pinned primary faces, but obtain the installed emoji chain
    // from the real loader. Otherwise a script fallback discovered earlier in
    // this scene can win emoji with monochrome glyphs, unlike the application.
    let fonts = crate::visual_quality::fonts();
    let (native, _) = FontLibrary::new(Default::default());
    {
        let native = native.inner.read();
        let mut fixture = fonts.inner.write();
        for id in 0..native.inner.len() {
            let face = native.get(&id);
            if face.is_emoji
                && !face.postscript_name().is_some_and(|name| {
                    fixture.font_id_for_postscript_name(name).is_some()
                })
            {
                fixture.insert(face.clone());
            }
        }
        assert!(
            fixture.inner.len() > 4,
            "native emoji prerequisite is missing"
        );
        let (_, is_emoji) = fixture
            .find_best_font_match_strict('😀', &Default::default(), None)
            .expect("the native emoji chain must cover the fixture");
        assert!(is_emoji, "emoji must use the installed color-emoji policy");
    }
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        let (pixels, size, fonts) = render_scene_with_fonts(
            UNICODE,
            scale,
            colors,
            false,
            "correct",
            fonts.clone(),
        );
        let library = fonts.inner.read();
        for ch in ['中', '文', '日', '本', '한', '국', 'م', 'ر', 'ש', 'ל', '😀']
        {
            assert!(
                library
                    .find_best_font_match_strict(ch, &Default::default(), None)
                    .is_some(),
                "native fallback missing for required fixture U+{:04X}",
                ch as u32
            );
        }
        drop(library);
        let hashes = crate::visual_quality::font_hashes(&fonts);
        crate::visual_quality::export_with_fonts(
            &format!("unicode-fallback-{}", (scale * 100.0) as u32),
            "correct",
            &pixels,
            size,
            scale,
            "aurora-night",
            vec![("viewport", [0.0, 0.0, size[0] as f32, size[1] as f32])],
            &hashes,
        );
    }
}

#[test]
fn visual_quality_terminal_golden_matrix() {
    for row in SCENE.lines().skip(9).take(5) {
        assert_eq!(
            row.chars().count(),
            55,
            "fixture box must close in one column"
        );
        assert!(matches!(row.chars().nth(27), Some('┬' | '│' | '┼' | '┴')));
    }
    for entry in crate::automexia::theme_gallery::builtins() {
        let colors = entry.theme.unwrap().colors;
        let theme = entry.name.to_ascii_lowercase().replace(' ', "-");
        for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
            let (pixels, size) = scene(scale, colors, true, "correct");
            let (repeat, repeat_size) = scene(scale, colors, true, "correct");
            assert_eq!(size, repeat_size);
            assert_eq!(
                pixels, repeat,
                "frame must be deterministic with fresh atlases"
            );
            let (unselected, _) = scene(scale, colors, false, "correct");
            assert_ne!(
                pixels, unselected,
                "selection must change the delivered pixels"
            );
            crate::visual_quality::export(
                &format!("terminal-{theme}-{}", (scale * 100.0) as u32),
                &pixels,
                size,
                scale,
                &theme,
                vec![
                    ("viewport", [0.0, 0.0, size[0] as f32, size[1] as f32]),
                    ("cell", [0.0, 0.0, 10.0 * scale, 24.0 * scale]),
                    (
                        "selection",
                        [110.0 * scale, 336.0 * scale, 130.0 * scale, 24.0 * scale],
                    ),
                ],
            );
        }
    }
}

#[test]
fn visual_quality_production_glyph_mutation_campaign() {
    let colors = crate::automexia::theme_gallery::builtins()
        .remove(0)
        .theme
        .unwrap()
        .colors;
    let (correct, size) = scene(1.0, colors, true, "correct");
    let geometry = vec![("viewport", [0.0, 0.0, size[0] as f32, size[1] as f32])];
    crate::visual_quality::export(
        "glyph-mutations",
        &correct,
        size,
        1.0,
        "aurora-night",
        geometry.clone(),
    );
    for defect in DEFECTS {
        let (broken, _) = scene(1.0, colors, true, defect);
        assert!(
            correct != broken,
            "undetected production glyph mutation: {defect}"
        );
        crate::visual_quality::export_variant(
            "glyph-mutations",
            defect,
            &broken,
            size,
            1.0,
            "aurora-night",
            geometry.clone(),
        );
    }
    let (restored, _) = scene(1.0, colors, true, "correct");
    assert_eq!(
        correct, restored,
        "fresh unmutated renderer restores all pixels"
    );
    crate::visual_quality::export_variant(
        "glyph-mutations",
        "restored",
        &restored,
        size,
        1.0,
        "aurora-night",
        geometry,
    );
}
