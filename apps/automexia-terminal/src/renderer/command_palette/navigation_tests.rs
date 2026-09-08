use super::*;
use rio_window::keyboard::{Key, ModifiersState, NamedKey};

#[test]
fn all_platform_palette_labels_match_real_defaults_not_platform_guesses() {
    use crate::bindings::{default_key_bindings, registry, test_platform_defaults};
    use automexia_keybindings::PlatformFamily::{LinuxBsd, Macos, Windows};
    use rio_backend::config::Config;
    let config = Config::default();
    // In addition to isolated platform tables, exercise the same table as Screen.
    assert_eq!(
        default_key_bindings(&config),
        test_platform_defaults(&config, registry::platform_family())
    );
    for platform in [Windows, LinuxBsd, Macos] {
        let bindings = test_platform_defaults(&config, platform);
        let mut palette = CommandPalette::new();
        palette.set_effective_bindings(&bindings, None);
        let shortcut = |action| {
            let command = COMMANDS.iter().find(|c| c.action == action).unwrap();
            palette.command_shortcut(command)
        };
        let (settings, fresh, clone, copy) = match platform {
            Windows => ("ctrl+,", "shift+alt+r", "alt+r", "ctrl+shift+c"),
            LinuxBsd => ("ctrl+shift+,", "shift+alt+r", "alt+r", "ctrl+shift+c"),
            Macos => ("super+,", "super+d", "shift+alt+super+r", "super+c"),
        };
        for (action, expected) in [
            (PaletteAction::ConfigEditor, settings),
            (PaletteAction::SplitRight, fresh),
            (PaletteAction::CloneSplitRight, clone),
            (PaletteAction::Copy, copy),
            (PaletteAction::ToggleViMode, "shift+alt+space"),
        ] {
            assert_eq!(
                shortcut(action),
                format!("{expected} · Legacy"),
                "{platform:?} {action:?}"
            );
        }
        // These were advertised as launch shortcuts despite being unbound or
        // performing a different action. Clearing history is not ClearScreen.
        assert_eq!(shortcut(PaletteAction::ClearScreen), "Unbound");
        if platform != Macos {
            assert_eq!(shortcut(PaletteAction::Quit), "Unbound");
            assert_eq!(shortcut(PaletteAction::SearchBackward), "Unbound");
        }
        if platform == LinuxBsd {
            assert_eq!(shortcut(PaletteAction::ToggleFullscreen), "Unbound");
        }
        assert_eq!(palette.registry_shortcuts.len(), COMMANDS.len());
        palette.set_effective_bindings(&bindings, None);
        assert_eq!(palette.registry_shortcuts.len(), COMMANDS.len());
        let capacity = palette.registry_shortcuts.capacity();
        for _ in 0..64 {
            palette.set_effective_bindings(&bindings, None);
        }
        assert_eq!(palette.registry_shortcuts.capacity(), capacity);
        assert!(capacity <= COMMANDS.len() * 2);
        assert_eq!(palette.query, "");
    }
}

#[test]
fn shortcut_labels_reject_wrong_modes_and_follow_typed_tombstones() {
    use crate::bindings::{default_key_bindings, registry};
    use rio_backend::config::Config;
    let settings = COMMANDS
        .iter()
        .find(|c| c.action == PaletteAction::ConfigEditor)
        .unwrap();
    let mut config = Config::default();
    let mut palette = CommandPalette::new();
    let mut bindings = default_key_bindings(&config);
    bindings.retain(|b| b.action == crate::bindings::Action::ConfigEditor);
    let trigger = registry::legacy_trigger(&bindings[0]).unwrap().to_string();
    for (directive, expected) in [
        (format!("{trigger}=unbind"), "Unbound".to_owned()),
        (format!("{trigger}=quit"), "Conditional binding".to_owned()),
        (
            format!("{trigger}>alt+x=quit"),
            "Conditional binding".to_owned(),
        ),
        ("alt+f8=open_config".to_owned(), "alt+f8 · User".to_owned()),
    ] {
        config.bindings.keybinds = vec![directive];
        let snapshot = registry::build(&config).unwrap();
        palette.set_binding_registry(
            snapshot.as_ref().map(|s| s.registry.as_ref()),
            config.keyboard.binding_profile,
            &[],
        );
        palette.set_effective_bindings(&bindings, snapshot.as_ref());
        assert_eq!(palette.command_shortcut(settings), expected);
    }
    bindings[0].mode = crate::bindings::BindingMode::SEARCH;
    palette.set_effective_bindings(&bindings, None);
    assert_eq!(palette.command_shortcut(settings), "Unbound");
    assert_eq!(palette.registry_shortcuts.len(), COMMANDS.len());
}

#[test]
fn back_arrow_has_a_left_pointing_head_and_no_tab_frame() {
    // Independent coordinates pin the intended arrow on the existing 22px grid.
    assert_eq!(
        BACK_ARROW_STROKES,
        [
            [4.0, 11.0, 18.0, 11.0],
            [4.0, 11.0, 10.0, 5.0],
            [4.0, 11.0, 10.0, 17.0],
        ]
    );
    for scale in [1.0_f32, 1.25, 1.5, 2.0, 3.0] {
        for [x1, y1, x2, y2] in BACK_ARROW_STROKES {
            for coordinate in [x1, y1, x2, y2] {
                assert!(coordinate * scale > 1.45 * scale / 2.0);
                assert!((coordinate + 1.45 / 2.0) * scale < RESULT_ICON_SIZE * scale);
            }
        }
    }
}

fn press(palette: &mut CommandPalette, key: NamedKey, mods: ModifiersState) -> bool {
    palette.handle_navigation_key(&Key::Named(key), mods, false)
}

#[test]
fn back_navigation_is_not_presented_as_previous_tab() {
    let mut palette = CommandPalette::new();
    palette.set_enabled(true);
    palette.selected_index = 1;
    assert!(palette.activate_navigation());
    let rows = palette.filtered_rows();
    let back = &rows[0].1;
    assert_eq!(back.title(), "Back to categories");
    assert_eq!(back.action(), None);
    assert_ne!(back.presentation().icon, CommandIcon::TabPrevious);
    assert_eq!(back.presentation().icon, CommandIcon::Back);
}

#[test]
fn typed_search_label_preserves_the_dispatchers_forward_pane_scope() {
    use crate::bindings::{default_key_bindings, registry};
    use rio_backend::config::Config;
    let mut config = Config::default();
    config.bindings.keybinds = vec!["alt+f8=start_search".into()];
    let snapshot = registry::build(&config).unwrap().unwrap();
    let mut palette = CommandPalette::new();
    palette.set_binding_registry(
        Some(&snapshot.registry),
        config.keyboard.binding_profile,
        &[],
    );
    palette.set_effective_bindings(&default_key_bindings(&config), Some(&snapshot));
    let label = |action| {
        palette.command_shortcut(COMMANDS.iter().find(|c| c.action == action).unwrap())
    };
    assert_eq!(label(PaletteAction::SearchForward), "alt+f8 · User");
    for action in [
        PaletteAction::SearchBackward,
        PaletteAction::SearchGlobalForward,
        PaletteAction::SearchGlobalBackward,
    ] {
        assert_ne!(
            label(action),
            "alt+f8 · User",
            "typed start_search starts forward in one pane"
        );
    }
}

#[test]
fn non_pane_shortcuts_follow_effective_bindings_and_removal() {
    use crate::bindings::{config_key_bindings, default_key_bindings};
    use rio_backend::config::{bindings::KeyBinding, Config};
    let mut palette = CommandPalette::new();
    let bindings = config_key_bindings(
        vec![KeyBinding {
            key: "F8".into(),
            action: "OpenConfigEditor".into(),
            with: "alt".into(),
            esc: String::new(),
            mode: String::new(),
        }],
        default_key_bindings(&Config::default())
            .into_iter()
            .filter(|binding| binding.action != crate::bindings::Action::ConfigEditor)
            .collect(),
    );
    let command = COMMANDS
        .iter()
        .find(|c| c.action == PaletteAction::ConfigEditor)
        .unwrap();
    palette.set_effective_bindings(&bindings, None);
    assert_eq!(palette.command_shortcut(command), "alt+f8 · Legacy");
    palette.set_effective_bindings(&[], None);
    assert_eq!(palette.command_shortcut(command), "Unbound");
}

#[test]
fn header_back_is_fixed_visible_and_non_executing_after_scrolling_or_search() {
    for (width, height, scale) in [
        (300.0, 260.0, 1.0),
        (1280.0, 760.0, 1.0),
        (2560.0, 1520.0, 2.0),
    ] {
        let dimensions = (width, height, scale);
        let mut palette = CommandPalette::new();
        palette.set_enabled(true);
        assert!(palette.back_button_rect(dimensions).is_none());
        palette.selected_index = 1;
        assert!(palette.activate_navigation());
        let rect = palette.back_button_rect(dimensions).unwrap();
        palette.scroll_offset = 5;
        palette.selected_index = 8;
        assert_eq!(palette.back_button_rect(dimensions), Some(rect));
        palette.set_query("clone".into());
        assert_eq!(palette.back_button_rect(dimensions), Some(rect));
        let [x, y, w, h] = rect;
        let (px, py, pw, _, _) = palette.palette_rect(width, height, scale);
        let input_x = px + PALETTE_PADDING;
        let text_x = input_x + INPUT_PADDING_X + w + 8.0;
        let esc_x = px + pw - PALETTE_PADDING - ESC_BADGE_WIDTH - 10.0;
        assert!(
            x + w <= text_x && text_x < esc_x,
            "Back, input and Escape do not intersect"
        );
        assert!(
            y + h < py + PALETTE_PADDING + INPUT_HEIGHT,
            "Back is independent of scrolling rows"
        );
        assert!(x >= 0.0 && y >= 0.0 && x + w < width / scale && y + h < height / scale);
        assert!(!palette.try_back_click(x + w, y + h, dimensions));
        assert!(palette.try_back_click(x + w / 2.0, y + h / 2.0, dimensions));
        assert_eq!(palette.category, None);
        assert_eq!(palette.selected_index, 1);
        assert!(palette.query.is_empty());
        assert_eq!(palette.get_selected_action(), None);
        assert!(palette.is_enabled());
        palette.set_enabled(false);
        assert!(!palette.try_back_click(x, y, dimensions));
    }
}

#[test]
fn header_back_returns_from_font_and_extension_lists_to_the_parent_action() {
    for (mode, action) in [
        (
            PaletteMode::Fonts(vec!["Fixture Mono".into()]),
            PaletteAction::ListFonts,
        ),
        (PaletteMode::Market(Vec::new()), PaletteAction::OpenMarket),
    ] {
        let mut palette = CommandPalette::new();
        palette.set_enabled(true);
        palette.mode = mode;
        let dimensions = (1280.0, 760.0, 1.0);
        let [x, y, ..] = palette.back_button_rect(dimensions).unwrap();
        assert!(palette.try_back_click(x + 1.0, y + 1.0, dimensions));
        assert_eq!(palette.get_selected_action(), Some(action));
        assert!(palette.is_enabled());
    }
}

#[test]
fn allocation_reduced_matching_keeps_the_independent_legacy_score_oracle() {
    // Frozen pre-optimization semantics, deliberately not the streaming matcher.
    fn legacy(query: &str, target: &str) -> Option<i32> {
        let q: Vec<_> = query.to_lowercase().chars().collect();
        let t: Vec<_> = target.to_lowercase().chars().collect();
        if q.is_empty() {
            return Some(0);
        }
        let (mut qi, mut score, mut previous, mut first) = (0, 0, false, None);
        for (i, c) in t.iter().enumerate() {
            if qi < q.len() && *c == q[qi] {
                first.get_or_insert(i);
                if previous {
                    score += 5;
                }
                if i == 0 || !t[i - 1].is_alphanumeric() {
                    score += 10;
                }
                previous = true;
                qi += 1;
            } else {
                previous = false;
            }
        }
        (qi == q.len()).then(|| score + 20_i32.saturating_sub(first.unwrap_or(0) as i32))
    }
    let targets = [
        "",
        "Clone Active Session Right",
        "CLOSE  PANE",
        "İstanbul",
        "ΟΣ",
        "AΣ AΣA",
        "e\u{301}界🙂",
        "Long-name_under.score",
    ];
    let queries = [
        "", "cr", "CL", "xx", "i\u{307}", "ος", "σ", "ς", "界🙂", "e\u{301}", "lns",
    ];
    for target in targets {
        for query in queries {
            assert_eq!(
                fuzzy_score(query, target),
                legacy(query, target),
                "query={query:?}, target={target:?}"
            );
        }
    }
    for command in COMMANDS {
        for query in queries {
            assert_eq!(
                fuzzy_score(query, command.title),
                legacy(query, command.title)
            );
        }
    }
}

#[test]
fn categories_cover_every_enabled_command_once_and_back_restores_root() {
    for adaptive in [false, true] {
        let mut palette = CommandPalette::new();
        palette.has_adaptive_theme = adaptive;
        palette.set_enabled(true);
        let mut seen = Vec::new();
        for index in 0..6 {
            palette.selected_index = index;
            assert!(press(
                &mut palette,
                NamedKey::Enter,
                ModifiersState::empty()
            ));
            assert!(palette.is_enabled());
            assert_eq!(palette.filtered_rows()[0].1.title(), "Back to categories");
            for (_, row) in palette.filtered_rows() {
                if let Some(action) = row.action() {
                    assert!(!seen.contains(&action));
                    seen.push(action);
                }
            }
            assert!(press(
                &mut palette,
                NamedKey::ArrowLeft,
                ModifiersState::ALT
            ));
            assert_eq!(palette.selected_index, index);
        }
        assert_eq!(seen.len(), COMMANDS.len() - usize::from(!adaptive));
    }
}

#[test]
fn typing_searches_globally_without_losing_category_and_limits() {
    let mut palette = CommandPalette::new();
    palette.set_enabled(true);
    palette.selected_index = 1;
    assert!(palette.activate_navigation());
    palette.set_query("quit".into());
    assert_eq!(palette.get_selected_action(), Some(PaletteAction::Quit));
    assert!(!palette.activate_navigation());
    palette.set_query("no-matching-action-xyz".into());
    assert_eq!(palette.get_selected_action(), None);
    assert!(!palette.activate_navigation());
    assert!(palette.is_enabled());
    palette.set_query("x".repeat(MAX_PALETTE_QUERY_BYTES));
    let accepted = palette.query.clone();
    palette.set_query("x".repeat(MAX_PALETTE_QUERY_BYTES + 1));
    assert_eq!(palette.query, accepted);
    palette.set_query("bad\ninput".into());
    assert_eq!(palette.query, accepted);
    palette.set_query(String::new());
    assert_eq!(palette.category, Some(Category::Panes));
    assert!(press(
        &mut palette,
        NamedKey::Backspace,
        ModifiersState::empty()
    ));
    assert_eq!(palette.selected_index, 1);
}

#[test]
fn pointer_navigation_uses_live_geometry_after_resize_and_scroll() {
    for (width, height, scale) in [
        (1280.0, 800.0, 1.0),
        (640.0, 480.0, 1.0),
        (1920.0, 1080.0, 3.0),
    ] {
        let mut palette = CommandPalette::new();
        palette.set_enabled(true);
        let (x, y, w, h, visible) = palette.palette_rect(width, height, scale);
        assert!(x >= 0.0 && x + w <= width / scale);
        assert!(y >= 0.0 && y + h <= height / scale);
        palette.visible_results = visible;
        let row_y =
            y + PALETTE_PADDING + INPUT_HEIGHT + SEPARATOR_HEIGHT + RESULTS_MARGIN_TOP;
        let index = palette
            .hit_test(x + 40.0, row_y + 22.0, width, height, scale)
            .unwrap()
            .unwrap();
        palette.selected_index = index;
        assert!(palette.activate_navigation());
        let (_, y, _, _, visible) = palette.palette_rect(width, height, scale);
        palette.visible_results = visible;
        palette.scroll_line_delta(-100.0);
        assert!(palette.go_back());
        assert_eq!(palette.scroll_offset, 0);
        assert_eq!(palette.wheel_accumulated_y, 0.0);
        assert!(y >= 0.0);
    }
}

#[test]
fn keyboard_back_paging_and_reopen_never_dispatch_commands() {
    let mut palette = CommandPalette::new();
    palette.set_enabled(true);
    palette.selected_index = 1;
    assert!(press(
        &mut palette,
        NamedKey::ArrowRight,
        ModifiersState::empty()
    ));
    palette.visible_results = 3;
    assert!(press(&mut palette, NamedKey::End, ModifiersState::empty()));
    assert_eq!(palette.selected_index, palette.filtered_rows().len() - 1);
    assert!(press(
        &mut palette,
        NamedKey::PageUp,
        ModifiersState::empty()
    ));
    assert_eq!(palette.selected_index, palette.filtered_rows().len() - 4);
    assert!(press(&mut palette, NamedKey::Home, ModifiersState::empty()));
    assert_eq!(palette.get_selected_action(), None);
    assert!(press(
        &mut palette,
        NamedKey::Enter,
        ModifiersState::empty()
    ));
    assert_eq!(palette.selected_index, 1);
    assert!(press(&mut palette, NamedKey::Tab, ModifiersState::SHIFT));
    assert_eq!(palette.selected_index, 0);
    assert!(!press(
        &mut palette,
        NamedKey::Escape,
        ModifiersState::empty()
    ));
    palette.set_enabled(false);
    palette.set_enabled(true);
    assert_eq!(palette.category, None);
    assert_eq!(palette.selected_index, 0);
}

#[test]
fn semantics_follow_scope_without_exporting_query_or_external_item_text() {
    let mut palette = CommandPalette::new();
    assert_eq!(palette.accessibility_summary(), None);
    palette.set_enabled(true);
    assert!(palette
        .accessibility_summary()
        .unwrap()
        .starts_with("Command categories; 6 results; selected 1"));
    palette.selected_index = 1;
    palette.activate_navigation();
    assert!(palette
        .accessibility_summary()
        .unwrap()
        .starts_with("Panes & Sessions"));
    palette.set_query("fictional-private-query".into());
    let summary = palette.accessibility_summary().unwrap();
    assert!(summary.starts_with("All commands; 0 results; selected 0"));
    assert!(!summary.contains("fictional-private-query"));
    palette.enter_fonts_mode(vec!["fictional-private-font".into()]);
    assert!(!palette
        .accessibility_summary()
        .unwrap()
        .contains("fictional-private-font"));
}

#[test]
fn category_names_search_actions_and_navigation_has_no_executable_authority() {
    let mut palette = CommandPalette::new();
    palette.set_enabled(true);
    palette.set_query("appearance".into());
    let rows = palette.filtered_rows();
    assert!(rows
        .iter()
        .any(|(_, row)| row.action() == Some(PaletteAction::IncreaseFontSize)));
    for index in 0..6 {
        palette.set_enabled(true);
        palette.selected_index = index;
        assert_eq!(palette.get_selected_action(), None);
        assert_eq!(palette.get_selected_font(), None);
        assert_eq!(palette.get_selected_market_id(), None);
        assert_eq!(palette.get_selected_action_item_id(), None);
        assert_eq!(palette.get_review_choice(), None);
    }
}

#[test]
fn actual_default_clone_labels_follow_unbind_and_profile_replacement() {
    use crate::bindings::{default_key_bindings, registry};
    use rio_backend::config::Config;
    let mut palette = CommandPalette::new();
    let command = COMMANDS
        .iter()
        .find(|command| command.action == PaletteAction::CloneSplitRight)
        .unwrap();
    let trigger = if cfg!(target_os = "macos") {
        "super+alt+shift+r"
    } else {
        "alt+r"
    };
    let mut config = Config::default();
    let snapshot = registry::build(&config).unwrap();
    palette.set_effective_bindings(&default_key_bindings(&config), snapshot.as_ref());
    assert_ne!(palette.command_shortcut(command), "Unbound");
    config.bindings.keybinds = vec![format!("{trigger}=unbind")];
    let snapshot = registry::build(&config).unwrap();
    palette.set_effective_bindings(&default_key_bindings(&config), snapshot.as_ref());
    assert_eq!(palette.command_shortcut(command), "Unbound");
    config.keyboard.binding_profile = automexia_keybindings::ProfileId::Ghostty13;
    config.bindings.keybinds.clear();
    if cfg!(target_os = "macos") {
        assert!(matches!(
            registry::build(&config),
            Err(registry::RegistryBuildError::ProfileUnavailable(_))
        ));
        return;
    }
    let snapshot = registry::build(&config).unwrap();
    palette.set_effective_bindings(&default_key_bindings(&config), snapshot.as_ref());
    assert_eq!(palette.command_shortcut(command), "Unbound");
}

#[test]
fn held_enter_is_consumed_and_child_lists_restore_their_parent_command() {
    let mut palette = CommandPalette::new();
    palette.set_enabled(true);
    assert!(press(
        &mut palette,
        NamedKey::Enter,
        ModifiersState::empty()
    ));
    let selected = palette.selected_index;
    for _ in 0..10 {
        assert!(palette.handle_navigation_key(
            &Key::Named(NamedKey::Enter),
            ModifiersState::empty(),
            true
        ));
        assert_eq!(palette.selected_index, selected);
        assert_eq!(palette.category, Some(Category::Tabs));
        assert!(palette.is_enabled());
    }
    palette.enter_fonts_mode(vec!["Example Mono".into()]);
    assert!(palette.go_back());
    assert_eq!(palette.category, Some(Category::Appearance));
    assert_eq!(
        palette.get_selected_action(),
        Some(PaletteAction::ListFonts)
    );
    assert!(palette.go_back());
    assert_eq!(palette.selected_index, 4);
    palette.enter_market_mode(Vec::new());
    assert!(palette.go_back());
    assert_eq!(palette.category, Some(Category::Tools));
    assert_eq!(
        palette.get_selected_action(),
        Some(PaletteAction::OpenMarket)
    );
}

#[test]
fn fresh_split_labels_follow_actual_configuration_and_preserve_typed_bindings() {
    use crate::bindings::{default_key_bindings, registry};
    use rio_backend::config::{bindings::KeyBinding, Config};
    fn refresh(palette: &mut CommandPalette, config: &Config) {
        let snapshot = registry::build(config).unwrap();
        palette.set_binding_registry(
            snapshot.as_ref().map(|value| value.registry.as_ref()),
            config.keyboard.binding_profile,
            &snapshot
                .as_ref()
                .map_or_else(Vec::new, |value| value.legacy_unbind_labels()),
        );
        palette.set_effective_bindings(&default_key_bindings(config), snapshot.as_ref());
    }
    let right = COMMANDS
        .iter()
        .find(|command| command.action == PaletteAction::SplitRight)
        .unwrap();
    let down = COMMANDS
        .iter()
        .find(|command| command.action == PaletteAction::SplitDown)
        .unwrap();
    let mut palette = CommandPalette::new();
    let mut config = Config::default();
    config.navigation.use_split = false;
    refresh(&mut palette, &config);
    assert_eq!(palette.command_shortcut(right), "Unbound");
    assert_eq!(palette.command_shortcut(down), "Unbound");

    config.bindings.keys.push(KeyBinding {
        key: "y".into(),
        with: "control|shift".into(),
        action: "SplitRight".into(),
        esc: String::new(),
        mode: "~Search|~Vi".into(),
    });
    refresh(&mut palette, &config);
    assert_eq!(palette.command_shortcut(right), "ctrl+shift+y · Legacy");
    config.bindings.keybinds = vec!["alt+q=new_split:right".into()];
    refresh(&mut palette, &config);
    assert!(palette.command_shortcut(right).starts_with("alt+q · "));

    config.bindings.keys.clear();
    config.bindings.keybinds = vec![if cfg!(target_os = "macos") {
        "super+d=unbind".into()
    } else {
        "alt+shift+r=unbind".into()
    }];
    config.navigation.use_split = true;
    refresh(&mut palette, &config);
    assert_eq!(palette.command_shortcut(right), "Unbound");
    config.keyboard.binding_profile = automexia_keybindings::ProfileId::Ghostty13;
    config.bindings.keybinds.clear();
    if cfg!(target_os = "macos") {
        assert!(matches!(
            registry::build(&config),
            Err(registry::RegistryBuildError::ProfileUnavailable(_))
        ));
        return;
    }
    refresh(&mut palette, &config);
    assert_ne!(palette.command_shortcut(right), "Unbound");
    assert!(!palette.command_shortcut(right).contains("Legacy"));
}

#[test]
#[ignore = "manual Criterion model benchmark; not a native UI latency claim"]
fn benchmark_palette_browse_search_and_back() {
    use criterion::Criterion;
    use std::{hint::black_box, time::Duration};
    let report_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/qa/palette-navigation-01/criterion");
    let mut criterion = Criterion::default()
        .without_plots()
        .output_directory(&report_root)
        .sample_size(30)
        .warm_up_time(Duration::from_secs(1))
        .measurement_time(Duration::from_secs(3));
    let mut palette = CommandPalette::new();
    let config = rio_backend::config::Config::default();
    let bindings = crate::bindings::default_key_bindings(&config);
    palette.set_effective_bindings(&bindings, None);
    criterion.bench_function("palette_browse_search_back", |b| {
        b.iter(|| {
            palette.set_enabled(true);
            palette.selected_index = black_box(1);
            black_box(palette.activate_navigation());
            palette.set_query(black_box("clone".to_owned()));
            black_box(palette.get_selected_action());
            palette.set_query(String::new());
            black_box(palette.go_back());
            black_box(palette.filtered_rows().len());
        })
    });
    criterion.bench_function("palette_effective_shortcut_reload", |b| {
        b.iter(|| palette.set_effective_bindings(black_box(&bindings), None))
    });
    criterion.final_summary();
}
