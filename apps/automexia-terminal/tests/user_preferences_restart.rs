use automexia_extension_api::{IconKind, SegmentRole};
use automexia_terminal::automexia::package_customizations::{
    PackageOverride, PackageValue,
};
use automexia_terminal::automexia::preferences::{
    load_from_root, write_package_to_root, write_to_root,
    CommandOutputAppearancePreferences, ExtensionFeaturePreference,
    HighlightAppearancePreferences, InformationBarPreferences, PreferenceSource,
    PresentationPreferences, TagAppearancePreferences, UserPreferences,
    VisualPreferences,
};
use automexia_terminal::automexia::settings_extensions::DEVOPS_CONTEXT_STATUS_ID;
use automexia_ui_model::information_bar::{
    preset_recipe, BarIconSource, BarSlotSourceDraft, BarTextSource, InformationBarPreset,
};
use rio_backend::config::{
    presentation::{
        HighlightStyle, Rgb, Rgba, TableAppearance, TableBanding, TableBorderStyle,
        TagStyle, TimestampAppearance, TimestampDateFormat, TimestampPosition,
    },
    theme::AppearanceTheme,
    Config,
};
use std::{collections::BTreeMap, path::Path, process::Command};

const CHILD_ROOT: &str = "AUTOMEXIA_PREFERENCE_TEST_CHILD_ROOT";
const CHILD_MODE: &str = "AUTOMEXIA_PREFERENCE_TEST_CHILD_MODE";

fn custom_information_bar() -> InformationBarPreferences {
    let preset = InformationBarPreset::FloatingCards;
    let mut recipe = preset_recipe(preset);
    recipe.spacing_percent = 145;
    let first = recipe.slots.get_mut(0).expect("preset has a first slot");
    let first_id = first.id.clone();
    first.icon = BarIconSource::Fixed(IconKind::Windows);
    first.text = BarTextSource::Role(SegmentRole::User);
    let mut source_drafts = BTreeMap::new();
    source_drafts.insert(
        first_id,
        BarSlotSourceDraft {
            literal: Some("Custom label".into()),
            text_role: Some(SegmentRole::Windows),
            icon_role: Some(SegmentRole::Docker),
            fixed_icon: Some(IconKind::Git),
            coerced_icon: None,
        },
    );
    InformationBarPreferences {
        preset: Some(preset),
        custom_recipe: Some(recipe),
        use_custom: true,
        source_drafts,
    }
}

#[test]
fn preference_child_write() {
    let Some(root) = std::env::var_os(CHILD_ROOT) else {
        return;
    };
    let preferences = match std::env::var(CHILD_MODE).as_deref() {
        Ok("set") => UserPreferences {
            font_size: Some(23.5),
            fonts: automexia_terminal::automexia::font_preferences::FontPreferences {
                family: Some("Example Mono".into()), line_height: Some(1.5), ligatures: Some(false),
                colors: [(automexia_terminal::automexia::font_preferences::FontColor::Foreground, Rgb::from_bytes([90, 120, 150]))].into(),
                ..Default::default()
            },
            appearance_theme: Some(AppearanceTheme::Light),
            shortcuts: vec![rio_backend::config::bindings::UiShortcut {
                action: "CloneSplitRight".into(),
                trigger: automexia_keybindings::Trigger::new(
                    automexia_keybindings::KeyAtom::Named(
                        automexia_keybindings::NamedKey::parse("f9").unwrap(),
                    ),
                    automexia_keybindings::Modifiers::CONTROL
                        .union(automexia_keybindings::Modifiers::SHIFT),
                )
                .unwrap(),
            }],
            presentation: PresentationPreferences {
                inline_tables: Some(false),
                output_highlighting: Some(false),
                command_output_highlighting: Some(false),
                kubernetes_highlighting: Some(true),
                command_timestamps: Some(false),
            },
            visual: VisualPreferences {
                timestamps: TimestampAppearance {
                    date_format: Some(TimestampDateFormat::DayMonthYear),
                    time_position: Some(TimestampPosition::BelowRight),
                    background: Some(Rgba::from_bytes([40, 50, 60, 128])),
                    ..Default::default()
                },
                tables: TableAppearance {
                    border_style: Some(TableBorderStyle::Dotted),
                    banding: Some(TableBanding::Rows),
                    header_foreground: Some(Rgb::from_bytes([10, 20, 30])),
                    alternate_background: Some(Rgba::from_bytes([20, 40, 60, 128])),
                    column_lines: Some(false),
                    ..Default::default()
                },
                tags: TagAppearancePreferences {
                    enabled: Some(false),
                    style: Some(TagStyle::Plain),
                    ..Default::default()
                },
                highlight: HighlightAppearancePreferences {
                    style: Some(HighlightStyle::Background),
                    error_background: Some(Rgba::from_bytes([1, 2, 3, 4])),
                    ..Default::default()
                },
                command_output: CommandOutputAppearancePreferences {
                    success: Some(Rgba::from_bytes([17, 34, 51, 68])),
                    failure: Some(Rgba::from_bytes([34, 51, 68, 85])),
                    neutral: Some(Rgba::from_bytes([51, 68, 85, 102])),
                    pulse: Some(false),
                },
                kubernetes: HighlightAppearancePreferences {
                    style: Some(HighlightStyle::Foreground),
                    error_background: Some(Rgba::from_bytes([5, 6, 7, 8])),
                    ..Default::default()
                },
                information_bar: custom_information_bar(),
            },
            extension_features: vec![ExtensionFeaturePreference {
                id: DEVOPS_CONTEXT_STATUS_ID.into(),
                enabled: false,
            }],
            package_overrides: vec![PackageOverride {
                publisher_id: "example.publisher".into(),
                extension_id: "example.inspect".into(),
                feature_id: "summary".into(),
                option_id: None,
                value: PackageValue::Boolean(false),
            }],
        },
        Ok("import-v4-and-edit-logs") => {
            let loaded = load_from_root(Path::new(&root));
            assert_eq!(loaded.source, PreferenceSource::Version4);
            let mut preferences = loaded.preferences;
            preferences.presentation.output_highlighting = Some(true);
            preferences.visual.highlight.colors.error = Some(Rgb::from_bytes([4, 5, 6]));
            preferences
        }
        Ok("reset") => UserPreferences::default(),
        _ => panic!("child mode must be set or reset"),
    };
    write_package_to_root(Path::new(&root), &preferences)
        .expect("child persists preferences");
}

fn run_child(root: &Path, mode: &str) {
    let status = Command::new(std::env::current_exe().expect("current test executable"))
        .args(["--exact", "preference_child_write", "--nocapture"])
        .env(CHILD_ROOT, root)
        .env(CHILD_MODE, mode)
        .status()
        .expect("launch typed preference test child");
    assert!(status.success(), "preference child failed with {status}");
}

#[test]
fn saved_preferences_survive_real_process_restart_and_reset_without_config_mutation() {
    let root = tempfile::tempdir().unwrap();
    let config_path = root.path().join("config.toml");
    let config_bytes = b"force-theme = \"dark\"\n[fonts]\nsize = 15.25\n";
    std::fs::write(&config_path, config_bytes).unwrap();

    run_child(root.path(), "set");
    let restarted = load_from_root(root.path());
    assert_eq!(restarted.source, PreferenceSource::Primary);
    let mut base = Config::default();
    base.fonts.size = 15.25;
    base.force_theme = Some(AppearanceTheme::Dark);
    let effective = restarted.preferences.apply_to(&base);
    assert_eq!(effective.fonts.size, 23.5);
    assert_eq!(effective.fonts.family.as_deref(), Some("Example Mono"));
    assert_eq!(effective.line_height, 1.5);
    assert_eq!(
        effective.fonts.features.as_deref(),
        Some(["liga=0".to_string(), "calt=0".to_string()].as_slice())
    );
    assert_eq!(
        effective.colors.foreground,
        [90.0 / 255.0, 120.0 / 255.0, 150.0 / 255.0, 1.0]
    );
    assert_eq!(effective.force_theme, Some(AppearanceTheme::Light));
    assert!(!effective.presentation.inline_tables);
    assert_eq!(
        effective.presentation.tables.border_style,
        Some(TableBorderStyle::Dotted)
    );
    assert_eq!(
        effective.presentation.tables.banding,
        Some(TableBanding::Rows)
    );
    assert_eq!(
        effective.presentation.tables.header_foreground,
        Some(Rgb::from_bytes([10, 20, 30]))
    );
    assert_eq!(
        effective.presentation.tables.alternate_background,
        Some(Rgba::from_bytes([20, 40, 60, 128]))
    );
    assert_eq!(effective.presentation.tables.column_lines, Some(false));
    assert!(!effective.presentation.output_highlighting);
    assert!(!effective.presentation.command_output_highlighting);
    assert!(effective.presentation.kubernetes_highlighting);
    assert!(!effective.presentation.command_timestamps);
    assert_eq!(
        effective.presentation.timestamps.date_format,
        Some(TimestampDateFormat::DayMonthYear)
    );
    assert_eq!(
        effective.presentation.timestamps.time_position,
        Some(TimestampPosition::BelowRight)
    );
    assert_eq!(
        effective.presentation.timestamps.background,
        Some(Rgba::from_bytes([40, 50, 60, 128]))
    );
    assert!(!effective.presentation.tags.enabled);
    assert_eq!(effective.presentation.tags.style, TagStyle::Plain);
    assert_eq!(
        effective.presentation.highlight.style,
        HighlightStyle::Background
    );
    assert_eq!(
        effective
            .presentation
            .highlight
            .error_background
            .unwrap()
            .bytes(),
        [1, 2, 3, 4]
    );
    assert_eq!(
        effective.presentation.kubernetes.style,
        HighlightStyle::Foreground
    );
    assert_eq!(
        effective
            .presentation
            .kubernetes
            .error_background
            .unwrap()
            .bytes(),
        [5, 6, 7, 8]
    );
    assert_eq!(
        effective
            .presentation
            .command_output
            .success
            .unwrap()
            .bytes(),
        [17, 34, 51, 68]
    );
    assert_eq!(
        effective
            .presentation
            .command_output
            .failure
            .unwrap()
            .bytes(),
        [34, 51, 68, 85]
    );
    assert_eq!(
        effective
            .presentation
            .command_output
            .neutral
            .unwrap()
            .bytes(),
        [51, 68, 85, 102]
    );
    assert!(!effective.presentation.command_output.pulse);
    assert_eq!(
        restarted.preferences.visual.information_bar,
        custom_information_bar()
    );
    assert_eq!(
        restarted.preferences.visual.information_bar.recipe(),
        custom_information_bar().recipe()
    );
    assert_eq!(
        restarted
            .preferences
            .extension_feature_enabled(DEVOPS_CONTEXT_STATUS_ID),
        Some(false)
    );
    assert_eq!(effective.bindings.ui_shortcuts.len(), 1);
    assert_eq!(restarted.preferences.package_overrides.len(), 1);
    assert_eq!(
        restarted.preferences.package_overrides[0].value,
        PackageValue::Boolean(false)
    );
    assert_eq!(effective.bindings.ui_shortcuts[0].action, "CloneSplitRight");
    assert_eq!(
        effective.bindings.ui_shortcuts[0].trigger.to_string(),
        "ctrl+shift+f9"
    );
    assert_eq!(std::fs::read(&config_path).unwrap(), config_bytes);

    run_child(root.path(), "reset");
    let reset = load_from_root(root.path());
    let effective = reset.preferences.apply_to(&base);
    assert_eq!(effective.fonts.size, 15.25);
    assert_eq!(effective.force_theme, base.force_theme);
    assert!(effective.bindings.ui_shortcuts.is_empty());
    assert_eq!(effective.presentation, base.presentation);
    assert!(effective.presentation.tags.enabled);
    assert_eq!(
        reset.preferences.visual.information_bar,
        InformationBarPreferences::default()
    );
    assert!(reset.preferences.extension_features.is_empty());
    assert!(reset.preferences.package_overrides.is_empty());
    assert_eq!(std::fs::read(&config_path).unwrap(), config_bytes);
}

#[test]
fn legacy_v4_import_survives_restart_and_keeps_kubernetes_independent() {
    let root = tempfile::tempdir().unwrap();
    let mut legacy = UserPreferences::default();
    legacy.presentation.output_highlighting = Some(false);
    legacy.visual.highlight.colors.error = Some(Rgb::from_bytes([1, 2, 3]));
    // Create fixture permissions through the real private writer, then retain
    // only fields supported by the predecessor. This is test data, not migration.
    write_to_root(root.path(), &legacy).unwrap();
    let current = root.path().join("state/user-preferences-v9.toml");
    let previous = root.path().join("state/user-preferences-v4.toml");
    let original = std::fs::read_to_string(&current)
        .unwrap()
        .replace("schema-version = 9", "schema-version = 4");
    std::fs::write(&current, &original).unwrap();
    std::fs::rename(&current, &previous).unwrap();
    assert!(!current.exists());

    run_child(root.path(), "import-v4-and-edit-logs");
    let restored = load_from_root(root.path());
    assert_eq!(restored.source, PreferenceSource::Primary);
    let effective = restored.preferences.apply_to(&Config::default());
    assert!(effective.presentation.output_highlighting);
    assert!(effective.presentation.command_output_highlighting);
    assert!(!effective.presentation.kubernetes_highlighting);
    assert_eq!(
        effective
            .presentation
            .highlight
            .colors
            .error
            .unwrap()
            .bytes(),
        [4, 5, 6]
    );
    assert_eq!(
        effective
            .presentation
            .kubernetes
            .colors
            .error
            .unwrap()
            .bytes(),
        [1, 2, 3]
    );
    assert_eq!(std::fs::read_to_string(&previous).unwrap(), original);
}
