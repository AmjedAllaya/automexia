use super::*;
use rio_backend::{
    ansi::CursorShape,
    config::hints::{HintAction, HintInternalAction, Hints},
    crosswords::{grid::Scroll, Crosswords, CrosswordsSize},
    event::{VoidListener, WindowId},
    performer::handler::Processor,
};

fn fixture(columns: usize, rows: usize, bytes: &[u8]) -> Crosswords<VoidListener> {
    let mut term = Crosswords::new(
        CrosswordsSize::new(columns, rows),
        CursorShape::Block,
        VoidListener {},
        WindowId::from(0),
        0,
        2000,
    );
    let mut parser = Processor::default();
    for byte in bytes {
        parser.advance(&mut term, &[*byte]);
    }
    term
}
fn start(term: &Crosswords<VoidListener>, alphabet: &str) -> HintState {
    let mut state = HintState::new(alphabet.into());
    state.start(Rc::new(Hints::default().rules.remove(0)));
    state.update_matches(term);
    state
}

#[test]
fn keyboard_links_preserve_explicit_destination_and_precedence() {
    // The visible URL deliberately differs from the OSC 8 destination.
    let term = fixture(80, 4, b"\x1b]8;;https://example.invalid/actual?\x1b\\https://example.invalid/label\x1b]8;;\x1b\\");
    let state = start(&term, "ab");
    assert_eq!(state.matches().len(), 1);
    assert_eq!(state.matches()[0].text, "https://example.invalid/actual?");
}

#[test]
fn keyboard_links_find_history_wrapped_urls_and_map_cells() {
    let mut term = fixture(
        16,
        3,
        "界 e\u{301} https://example.invalid/path\r\nnext\r\nlast\r\n".as_bytes(),
    );
    term.scroll_display(Scroll::Top);
    let state = start(&term, "ab");
    assert_eq!(state.matches().len(), 1);
    assert_eq!(state.matches()[0].text, "https://example.invalid/path");
    assert_eq!(state.matches()[0].start.col, Column(5));
    assert!(state.matches()[0].start.row < Line(0));
}

#[test]
fn keyboard_links_cover_both_cells_of_final_wide_grapheme() {
    let target = "https://example.invalid/界\u{301}";
    let term = fixture(80, 4, target.as_bytes());
    let state = start(&term, "ab");
    assert_eq!(state.matches().len(), 1);
    assert_eq!(state.matches()[0].text, target);
    assert_eq!(state.matches()[0].start, Pos::new(Line(0), Column(0)));
    // The final grapheme occupies cells 24 and 25 despite having five UTF-8 bytes.
    assert_eq!(state.matches()[0].end, Pos::new(Line(0), Column(25)));
}

#[test]
fn keyboard_links_label_navigation_resets_destination_pan() {
    let term = fixture(
        120,
        4,
        b"https://example.invalid/a https://example.invalid/b https://example.invalid/c",
    );
    let mut state = start(&term, "ab");
    state.pan(true);
    assert!(state.preview_offset > 0);
    state.keyboard_input(&term, 'b');
    assert_eq!(state.preview_offset, 0);
    state.pan(true);
    state.keyboard_input(&term, '\x08');
    assert_eq!(state.preview_offset, 0);
}

#[test]
fn keyboard_links_right_edge_labels_are_whole_and_do_not_overlap() {
    for columns in [1, 2, 16] {
        let mut bytes = String::new();
        for row in [2, 4, 6] {
            bytes.push_str(&format!("\x1b[{row};{columns}H\x1b]8;id={row};https://example.invalid/{row}\x1b\\x\x1b]8;;\x1b\\"));
        }
        let term = fixture(columns, 6, bytes.as_bytes());
        let state = start(&term, "ab");
        assert_eq!(state.matches().len(), 3);
        let cells = state.label_cells();
        assert!(cells
            .iter()
            .all(|(position, _, _)| position.col.0 < columns));
        assert_eq!(cells.len(), if columns == 1 { 0 } else { 6 });
        assert_eq!(state.focused_label(), "aa");
        assert!(preview::lines(&state)[0].contains("[aa]"));
    }
    // One-cell OSC anchors need multi-cell labels; never paint an ambiguous overlap.
    let term = fixture(16, 4, b"\x1b]8;id=a;https://example.invalid/a\x1b\\x\x1b]8;id=b;https://example.invalid/b\x1b\\y\x1b]8;id=c;https://example.invalid/c\x1b\\z\x1b]8;;\x1b\\");
    let state = start(&term, "ab");
    let cells = state.label_cells();
    let positions: std::collections::BTreeSet<_> =
        cells.iter().map(|(p, _, _)| (p.row.0, p.col.0)).collect();
    assert_eq!(positions.len(), cells.len());
    assert_eq!(cells.len() % 2, 0);
}

#[test]
fn keyboard_links_labels_are_prefix_free_and_empty_alphabet_safe() {
    let term = fixture(
        120,
        4,
        b"https://example.invalid/a https://example.invalid/b https://example.invalid/c",
    );
    for alphabet in ["ab", "", "a", "aaa", "a\u{301}"] {
        let mut state = start(&term, alphabet);
        assert_eq!(state.matches().len(), 3);
        let labels = state.visible_labels();
        for (i, (_, a)) in labels.iter().enumerate() {
            for (j, (_, b)) in labels.iter().enumerate() {
                if i != j {
                    assert!(!a.starts_with(b));
                }
            }
        }
        // Typing selects; only an explicit Enter may activate the selected link.
        for c in labels[2].1.iter().copied() {
            assert!(state.keyboard_input(&term, c).is_none());
        }
        assert_eq!(state.focused().unwrap().text, "https://example.invalid/c");
        assert_eq!(state.activate().unwrap().text, "https://example.invalid/c");
    }
}

#[test]
fn keyboard_links_tab_wrap_backspace_and_cancel_have_no_terminal_effect() {
    let term = fixture(
        80,
        4,
        b"https://example.invalid/a https://example.invalid/b",
    );
    let before = term
        .bounds_to_string(Pos::new(Line(0), Column(0)), Pos::new(Line(3), Column(79)));
    let mut state = start(&term, "ab");
    state.cycle(true);
    assert!(state.focused().unwrap().text.ends_with("/b"));
    state.cycle(true);
    assert!(state.focused().unwrap().text.ends_with("/a"));
    state.cycle(false);
    assert!(state.focused().unwrap().text.ends_with("/b"));
    state.keyboard_input(&term, '\x08');
    state.keyboard_input(&term, '\x1b');
    assert!(!state.is_active());
    assert!(state.activate().is_none());
    assert_eq!(
        term.bounds_to_string(
            Pos::new(Line(0), Column(0)),
            Pos::new(Line(3), Column(79))
        ),
        before
    );
}

#[test]
fn keyboard_links_reject_stale_geometry_output_and_unsafe_open_targets() {
    let mut term = fixture(80, 4, b"https://example.invalid/a");
    let mut state = start(&term, "ab");
    assert!(state.is_current(&term));
    Processor::default().advance(&mut term, b"\rhttps://example.invalid/b");
    assert!(!state.is_current(&term));
    state.update_matches(&term);
    term.resize(CrosswordsSize::new(8, 2));
    assert!(!state.is_current(&term));
    for text in [
        "javascript:alert(1)",
        "data:text/plain,hello",
        "https://example.invalid/\nrun",
        "https://example.invalid/\u{202e}txt",
        "https://user:password@example.invalid",
        "file://remote.invalid/item",
        "--help",
    ] {
        assert!(!safe_open_target(text), "unsafe fixture accepted");
    }
    assert!(safe_open_target("https://example.invalid/?a=1&b=2"));
    assert!(safe_open_target("mailto:alice@example.invalid"));
}

#[test]
fn keyboard_links_bounds_and_custom_action_are_retained() {
    let term = fixture(120, 300, &b"https://example.invalid/a\r\n".repeat(300));
    let state = start(&term, "ab");
    assert_eq!(state.matches().len(), MAX_HINT_MATCHES);
    let mut custom = Hints::default().rules.remove(0);
    custom.action = HintAction::Action {
        action: HintInternalAction::Copy,
    };
    let mut state = HintState::new("ab".into());
    state.start(Rc::new(custom));
    state.update_matches(&term);
    assert!(matches!(
        state.activate().unwrap().hint.action,
        HintAction::Action {
            action: HintInternalAction::Copy
        }
    ));
}

#[test]
fn keyboard_links_match_and_byte_budget_boundaries_are_exact() {
    for count in [
        0,
        1,
        MAX_HINT_MATCHES - 1,
        MAX_HINT_MATCHES,
        MAX_HINT_MATCHES + 1,
    ] {
        let term = fixture(
            80,
            count + 1,
            &b"https://example.invalid/a\r\n".repeat(count),
        );
        assert_eq!(
            start(&term, "ab").matches().len(),
            count.min(MAX_HINT_MATCHES)
        );
    }
    let prefix = "https://example.invalid/";
    for bytes in [MAX_HINT_BYTES - 1, MAX_HINT_BYTES, MAX_HINT_BYTES + 1] {
        let target = format!("{prefix}{}", "x".repeat(bytes - prefix.len()));
        let term = fixture(
            80,
            4,
            format!("\x1b]8;;{target}\x1b\\label\x1b]8;;\x1b\\").as_bytes(),
        );
        let state = start(&term, "ab");
        assert_eq!(state.matches().len(), usize::from(bytes <= MAX_HINT_BYTES));
        if bytes <= MAX_HINT_BYTES {
            assert_eq!(state.matches()[0].text, target);
        }
    }
}

#[test]
fn keyboard_links_real_platform_bindings_and_modal_intents() {
    use automexia_keybindings::PlatformFamily;
    use rio_window::keyboard::{Key, ModifiersState as M, NamedKey};
    let config = rio_backend::config::Config::default();
    for platform in [
        PlatformFamily::Windows,
        PlatformFamily::LinuxBsd,
        PlatformFamily::Macos,
    ] {
        let bindings = crate::bindings::test_platform_defaults(&config, platform);
        let hints: Vec<_> = bindings
            .iter()
            .filter(|b| matches!(b.action, crate::bindings::Action::Hint(_)))
            .collect();
        assert_eq!(hints.len(), 1);
        assert_eq!(hints[0].mods, M::CONTROL | M::ALT);
        assert!(
            matches!(&hints[0].trigger, crate::bindings::BindingKey::Keycode { key: Key::Character(c), .. } if c == "o")
        );
    }
    for (key, mods, expected) in [
        (Key::Named(NamedKey::Enter), M::empty(), KeyIntent::Activate),
        (Key::Named(NamedKey::Tab), M::SHIFT, KeyIntent::Next(false)),
        (
            Key::Named(NamedKey::ArrowDown),
            M::empty(),
            KeyIntent::Next(true),
        ),
        (Key::Named(NamedKey::Escape), M::empty(), KeyIntent::Close),
        (
            Key::Character("c".into()),
            M::CONTROL | M::SHIFT,
            KeyIntent::Copy,
        ),
        (Key::Character("c".into()), M::SUPER, KeyIntent::Copy),
        (Key::Character("v".into()), M::CONTROL, KeyIntent::Consume),
    ] {
        assert_eq!(key_intent(&key, mods, true, false), expected);
        assert_eq!(key_intent(&key, mods, true, true), KeyIntent::Consume);
        assert_eq!(key_intent(&key, mods, false, false), KeyIntent::Consume);
    }
}

#[test]
fn keyboard_links_empty_oversize_controls_and_invalid_regex_fail_safely() {
    let term = fixture(80, 4, b"ordinary output");
    let state = start(&term, "ab");
    assert!(state.is_active());
    assert!(state.matches().is_empty());
    assert_eq!(preview::lines(&state)[1], "No links in this view");
    let mut rule = Hints::default().rules.remove(0);
    rule.regex = Some("(".into());
    let mut state = HintState::new("ab".into());
    state.start(Rc::new(rule));
    state.update_matches(&term);
    assert!(state.matches().is_empty());
    for uri in [
        format!("https://example.invalid/{}", "x".repeat(MAX_HINT_BYTES)),
        "https://example.invalid/\u{202e}hidden".into(),
    ] {
        let term = fixture(
            80,
            4,
            format!("\x1b]8;;{uri}\x1b\\label\x1b]8;;\x1b\\").as_bytes(),
        );
        assert!(start(&term, "ab").matches().is_empty());
    }
}

#[test]
fn keyboard_links_snapshots_reject_reused_extras_and_preserve_selection() {
    let mut term = fixture(
        80,
        4,
        b"\x1b]8;id=one;https://example.invalid/a\x1b\\label\x1b]8;;\x1b\\",
    );
    let state = start(&term, "ab");
    let cursor = term.cursor();
    assert!(state.is_current(&term));
    assert!(!format!("{:?}", state.matches()[0]).contains("example.invalid"));
    Processor::default().advance(
        &mut term,
        b"\r\x1b]8;id=one;https://example.invalid/b\x1b\\label\x1b]8;;\x1b\\",
    );
    assert!(!state.is_current(&term));
    assert_eq!(term.cursor(), cursor);
    assert!(term.selection.is_none());
}

#[test]
fn keyboard_links_one_osc_anchor_survives_wrapping_and_combining_extras() {
    let term = fixture(8, 5, "\x1b]8;id=one;https://example.invalid/exact?\x1b\\hello e\u{301} long label\x1b]8;;\x1b\\".as_bytes());
    let state = start(&term, "12;");
    assert_eq!(state.matches().len(), 1);
    assert_eq!(state.matches()[0].text, "https://example.invalid/exact?");
    assert_eq!(state.matches()[0].start, Pos::new(Line(0), Column(0)));
    assert!(state.matches()[0].end.row > Line(0));
}

fn preview_text() -> rio_backend::sugarloaf::text::Text {
    use rio_backend::sugarloaf::font::{
        constants, FontData, FontLibrary, FontLibraryData,
    };
    let mut data = FontLibraryData::default();
    data.insert(FontData::from_static_slice(constants::FONT_CASCADIA_CODE_NF).unwrap());
    let fonts = FontLibrary {
        inner: std::sync::Arc::new(parking_lot::RwLock::new(data)),
    };
    let mut text = rio_backend::sugarloaf::text::Text::new(&fonts);
    text.init_cpu();
    text
}

fn preview_pixels(state: &HintState, width: u32, height: u32) -> Vec<u32> {
    use crate::renderer::ui_theme::{self, color_u8, UiTheme};
    let mut text = preview_text();
    let theme = UiTheme::resolve(ui_theme::CARD, ui_theme::TEXT, ui_theme::MUTED_TEXT);
    let plan =
        preview::layout(&mut text, state, width as f32, height as f32, theme).unwrap();
    let mut pixels = vec![0; width as usize * height as usize];
    let [left, top, w, h] = plan.rect;
    // Independent pixel-centre coverage; the same production font/draw plan
    // paints labels. This does not stand in for native compositor evidence.
    for y in 0..height {
        for x in 0..width {
            let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
            if px >= left && px < left + w && py >= top && py < top + h {
                let c = color_u8(if py < top + 2.0 {
                    theme.outline
                } else {
                    theme.background
                });
                pixels[(y * width + x) as usize] =
                    (u32::from(c[0]) << 16) | (u32::from(c[1]) << 8) | u32::from(c[2]);
            }
        }
    }
    for (x, y, label, opts) in plan.labels {
        text.draw(x, y, &label, &opts);
    }
    text.render_cpu_base(&mut pixels, width, height);
    text.render_cpu_modal(&mut pixels, width, height);
    pixels
}

#[test]
fn keyboard_links_parser_to_preview_pixels_and_resize_limits() {
    use crate::renderer::ui_theme::{self, UiTheme};
    let term = fixture(
        80,
        4,
        b"https://example.invalid/docs https://example.invalid/status",
    );
    let mut state = start(&term, "ab");
    let original = preview_pixels(&state, 760, 180);
    state.cycle(true);
    assert_ne!(original, preview_pixels(&state, 760, 180));
    state.cycle(false);
    assert_eq!(original, preview_pixels(&state, 760, 180));
    let mut damaged = original.clone();
    damaged[68 * 760] ^= 1;
    assert_ne!(original, damaged, "one-channel pixel mutation must fail");
    if let Some(path) = std::env::var_os("AUTOMEXIA_HYPERLINK_PREVIEW") {
        image_rs::RgbImage::from_fn(760, 180, |x, y| {
            let pixel = original[(y * 760 + x) as usize];
            image_rs::Rgb([(pixel >> 16) as u8, (pixel >> 8) as u8, pixel as u8])
        })
        .save(path)
        .expect("write controlled hyperlink preview");
    }
    let theme = UiTheme::resolve(ui_theme::CARD, ui_theme::TEXT, ui_theme::MUTED_TEXT);
    let mut text = preview_text();
    for scale in [1.0, 1.25, 2.0, 3.0, 4.0] {
        for (w, h) in [(1.0, 1.0), (80.0, 40.0), (240.0, 160.0), (7680.0, 4320.0)] {
            let (w, h) = (w / scale, h / scale);
            if let Some(plan) = preview::layout(&mut text, &state, w, h, theme) {
                let [x, y, pw, ph] = plan.rect;
                assert!(x >= 0.0 && y >= 0.0 && x + pw <= w && y + ph <= h);
                for (x, y, label, opts) in plan.labels {
                    assert!(x + text.measure(&label, &opts) <= w && y + 16.0 <= h);
                }
            }
        }
    }
}

#[test]
#[ignore = "explicit correctness-checked hyperlink microbenchmark"]
fn keyboard_links_benchmark_checked_capture_navigation() {
    let term = fixture(
        120,
        60,
        &b"https://example.invalid/a https://example.invalid/b\r\n".repeat(50),
    );
    let mut timings = Vec::with_capacity(80);
    for _ in 0..80 {
        let began = std::time::Instant::now();
        let mut state = start(&term, "jfkdlsah");
        assert_eq!(state.matches().len(), 100);
        assert_eq!(state.label_cells().len(), 300);
        for _ in 0..100 {
            state.cycle(true);
            assert!(state.is_current(&term));
        }
        assert_eq!(state.focused_index(), 0);
        timings.push(began.elapsed().as_micros());
        state.stop();
        assert!(state.snapshot.is_none());
        assert!(state.matches().is_empty());
    }
    timings.sort_unstable();
    println!("hint capture + 100 exact-snapshot/navigation operations: median={}us p95={}us; 100 links; no native opener/compositor timing", timings[40], timings[76]);
}
