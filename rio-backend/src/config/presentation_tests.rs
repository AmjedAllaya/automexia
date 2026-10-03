use super::Config;

#[test]
fn inline_table_appearance_accepts_independent_borders_headers_and_banding() {
    let source = "[presentation.tables]\nborder-style = 'dashed'\nborder-weight = 'medium'\nrow-lines = false\ncolumn-lines = true\nbanding = 'rows'\nborder-color = '#abcdef80'\nheader-foreground = '#123456'\nheader-background = '#11223344'\nbody-background = '#22334455'\nalternate-background = '#33445566'\n";
    let config: Config =
        toml::from_str(source).expect("table appearance must be configurable");
    let value: toml::Value = toml::from_str(&presentation_document(&config)).unwrap();
    assert_eq!(
        value["presentation"]["tables"]["border-style"].as_str(),
        Some("dashed")
    );
    assert_eq!(
        value["presentation"]["tables"]["banding"].as_str(),
        Some("rows")
    );
    assert_eq!(
        value["presentation"]["tables"]["row-lines"].as_bool(),
        Some(false)
    );
}

fn presentation_document(config: &Config) -> String {
    #[derive(serde::Serialize)]
    struct Document<'a> {
        presentation: &'a super::presentation::Presentation,
    }
    // Keep real Config table paths while excluding unrelated legacy fields
    // (for example, margin's existing map-versus-sequence serialization).
    toml::to_string(&Document {
        presentation: &config.presentation,
    })
    .unwrap()
}

#[test]
fn presentation_defaults_keep_all_legacy_behaviors_enabled() {
    for config in [
        Config::default(),
        toml::from_str::<Config>("").unwrap(),
        toml::from_str::<Config>("[presentation]").unwrap(),
    ] {
        let value = toml::Value::try_from(config).unwrap();
        let presentation = value.get("presentation").expect("typed presentation table");
        for key in [
            "inline-tables",
            "output-highlighting",
            "command-output-highlighting",
            "kubernetes-highlighting",
            "command-timestamps",
        ] {
            assert_eq!(
                presentation.get(key).and_then(toml::Value::as_bool),
                Some(true)
            );
        }
    }
}

#[test]
fn output_domains_have_independent_config_switches_and_colors() {
    let config: Config = toml::from_str(
        "[presentation]\noutput-highlighting = false\ncommand-output-highlighting = true\nkubernetes-highlighting = true\n[presentation.command-output]\nsuccess = '#11223344'\nfailure = '#22334455'\nneutral = '#33445566'\npulse = false\n[presentation.highlight.colors]\nerror = '#abcdef'\n[presentation.kubernetes.colors]\nerror = '#123456'\n",
    ).expect("independent output domains must be configurable");
    let value: toml::Value = toml::from_str(&presentation_document(&config)).unwrap();
    let appearance = &value["presentation"];
    assert_eq!(appearance["output-highlighting"].as_bool(), Some(false));
    assert_eq!(
        appearance["command-output-highlighting"].as_bool(),
        Some(true)
    );
    assert_eq!(appearance["kubernetes-highlighting"].as_bool(), Some(true));
    assert_eq!(
        appearance["command-output"]["success"].as_str(),
        Some("#11223344")
    );
    assert_eq!(appearance["command-output"]["pulse"].as_bool(), Some(false));
    assert_eq!(
        appearance["highlight"]["colors"]["error"].as_str(),
        Some("#abcdef")
    );
    assert_eq!(
        appearance["kubernetes"]["colors"]["error"].as_str(),
        Some("#123456")
    );
    let restored: Config = toml::from_str(&presentation_document(&config)).unwrap();
    assert_eq!(restored.presentation, config.presentation);
}

#[test]
fn legacy_output_config_preserves_kubernetes_choices_without_linking_new_fields() {
    let legacy: Config = toml::from_str("[presentation]\noutput-highlighting = false\n[presentation.highlight]\nstyle = 'background'\nwarning-background = '#01020304'\n[presentation.highlight.colors]\nsuccess = '#123456'\n").unwrap();
    let value: toml::Value = toml::from_str(&presentation_document(&legacy)).unwrap();
    let appearance = &value["presentation"];
    assert_eq!(appearance["kubernetes-highlighting"].as_bool(), Some(false));
    assert_eq!(
        appearance["command-output-highlighting"].as_bool(),
        Some(true)
    );
    assert_eq!(appearance["kubernetes"], appearance["highlight"]);

    let explicit: Config = toml::from_str("[presentation]\noutput-highlighting = false\nkubernetes-highlighting = true\n[presentation.highlight.colors]\nerror = '#123456'\n[presentation.kubernetes]\n").unwrap();
    let value: toml::Value = toml::from_str(&presentation_document(&explicit)).unwrap();
    assert_eq!(
        value["presentation"]["kubernetes-highlighting"].as_bool(),
        Some(true)
    );
    assert!(value["presentation"]["kubernetes"]["colors"]
        .as_table()
        .unwrap()
        .is_empty());
}

#[test]
fn output_domains_reject_invalid_new_controls() {
    for source in [
        "[presentation]\ncommand-output-highlighting = 1",
        "[presentation]\nkubernetes-highlighting = 'false'",
        "[presentation.command-output]\nsuccess = '#xyz'",
        "[presentation.command-output]\npulse = 1",
        "[presentation.command-output]\nunknown = true",
        "[presentation.kubernetes]\nstyle = 'arbitrary'",
        "[presentation.kubernetes.colors]\nunknown = '#123456'",
    ] {
        assert!(toml::from_str::<Config>(source).is_err());
    }
}

#[test]
fn partial_presentation_config_roundtrips_explicit_false() {
    let config: Config =
        toml::from_str("[presentation]\ninline-tables = false\n").unwrap();
    let value = toml::Value::try_from(&config).unwrap();
    assert_eq!(
        value["presentation"]["inline-tables"].as_bool(),
        Some(false)
    );
    assert_eq!(
        value["presentation"]["output-highlighting"].as_bool(),
        Some(true)
    );
    assert_eq!(
        value["presentation"]["command-timestamps"].as_bool(),
        Some(true)
    );
    // Parse the typed root document through the real Config codec and compare
    // every presentation field, preserving nested TOML table paths.
    let serialized = presentation_document(&config);
    let decoded: Config = toml::from_str(&serialized).unwrap();
    assert_eq!(decoded.presentation, config.presentation);
}

#[test]
fn presentation_rejects_unknown_keys_and_non_booleans() {
    for source in [
        "[presentation]\ninline-table = false",
        "[presentation]\ninline-tables = 'false'",
        "[presentation]\noutput-highlighting = 0",
        "[presentation]\ncommand-timestamps = []",
        "presentation = true",
    ] {
        assert!(
            toml::from_str::<Config>(source).is_err(),
            "invalid presentation input: {source}"
        );
    }
}

#[test]
fn visual_config_defaults_preserve_old_style_opacity_and_inherited_anchors() {
    let value = toml::Value::try_from(Config::default()).unwrap();
    let tags = value["presentation"]
        .get("tags")
        .expect("typed tag appearance");
    assert_eq!(tags["style"].as_str(), Some("tinted"));
    assert_eq!(tags["opacity"].as_integer(), Some(12));
    assert_eq!(
        tags["colors"].as_table().unwrap().len(),
        0,
        "None must inherit existing anchors"
    );
    let highlight = value["presentation"]
        .get("highlight")
        .expect("typed highlight appearance");
    assert_eq!(highlight["style"].as_str(), Some("both"));
    assert_eq!(highlight["colors"].as_table().unwrap().len(), 0);
    assert!(highlight.get("error-background").is_none());
}

#[test]
fn information_tags_visibility_defaults_on_and_roundtrips_explicit_off() {
    let default: Config = toml::from_str("").unwrap();
    let default_document = presentation_document(&default);
    let default_value: toml::Value = toml::from_str(&default_document).unwrap();
    assert_eq!(
        default_value["presentation"]["tags"]["enabled"].as_bool(),
        Some(true)
    );

    let disabled: Config =
        toml::from_str("[presentation.tags]\nenabled = false\n").unwrap();
    let disabled_document = presentation_document(&disabled);
    let disabled_value: toml::Value = toml::from_str(&disabled_document).unwrap();
    assert_eq!(
        disabled_value["presentation"]["tags"]["enabled"].as_bool(),
        Some(false)
    );
    let restored: Config = toml::from_str(&disabled_document).unwrap();
    assert_eq!(restored.presentation.tags, disabled.presentation.tags);

    for invalid in ["'false'", "0", "[]"] {
        assert!(toml::from_str::<Config>(&format!(
            "[presentation.tags]\nenabled = {invalid}\n"
        ))
        .is_err());
    }
}

#[test]
fn visual_config_strict_fixed_colors_and_styles_roundtrip_through_config() {
    let config: Config = toml::from_str("[presentation.tags]\nstyle = 'plain'\nopacity = 100\n[presentation.tags.colors]\nproduction = '#010203'\nubuntu-wsl = '#112233'\nwindows = '#223344'\ngit = '#334455'\nkubernetes = '#445566'\ndocker = '#556677'\nazure = '#667788'\naws = '#778899'\ngcp = '#8899aa'\nunknown-cloud = '#99aabb'\nterraform = '#aabbcc'\nenvironment = '#bbccdd'\nuser = '#ccddee'\n[presentation.highlight]\nstyle = 'background'\nerror-background = '#01234500'\nwarning-background = '#fedcba80'\n[presentation.highlight.colors]\nerror = '#010203'\nwarning = '#112233'\nsuccess = '#223344'\ninfo = '#334455'\ndebug = '#445566'\n").expect("typed fixed appearance controls must be accepted");
    let serialized = presentation_document(&config);
    let roundtrip: Config = toml::from_str(&serialized).unwrap();
    assert_eq!(roundtrip.presentation, config.presentation);
    let value = toml::Value::try_from(config).unwrap();
    assert_eq!(
        value["presentation"]["tags"]["colors"]
            .as_table()
            .unwrap()
            .len(),
        13
    );
    assert_eq!(
        value["presentation"]["highlight"]["colors"]
            .as_table()
            .unwrap()
            .len(),
        5
    );
    assert_eq!(
        value["presentation"]["highlight"]["error-background"].as_str(),
        Some("#01234500")
    );
}

#[test]
fn visual_config_all_five_backgrounds_roundtrip_without_changing_absent_defaults() {
    let defaults = Config::default().presentation.highlight;
    assert!(defaults.error_background.is_none());
    assert!(defaults.warning_background.is_none());
    assert!(defaults.success_background.is_none());
    assert!(defaults.info_background.is_none());
    assert!(defaults.debug_background.is_none());

    let config: Config = toml::from_str(
        "[presentation.highlight]\nerror-background = '#01020304'\nwarning-background = '#05060708'\nsuccess-background = '#09101112'\ninfo-background = '#13141516'\ndebug-background = '#1718191a'\n",
    )
    .unwrap();
    let restored: Config = toml::from_str(&presentation_document(&config)).unwrap();
    assert_eq!(
        restored.presentation.highlight,
        config.presentation.highlight
    );
    for malformed in [
        "success-background = '#123'",
        "info-background = '#12gg00'",
        "debug-background = []",
    ] {
        assert!(toml::from_str::<Config>(&format!(
            "[presentation.highlight]\n{malformed}\n"
        ))
        .is_err());
    }
}

#[test]
fn visual_config_opacity_has_integer_percent_units_and_checked_boundaries() {
    use super::presentation::{AppearanceValueError, OpacityPercent};
    assert_eq!(OpacityPercent::default().as_alpha(), 0.12);
    for percent in [0, 1, 12, 99, 100] {
        let value = OpacityPercent::new(percent).unwrap();
        assert_eq!(value.get(), percent);
        let source = format!("[presentation.tags]\nopacity = {percent}");
        let config: Config = toml::from_str(&source).unwrap();
        assert_eq!(config.presentation.tags.opacity, value);
        let table = toml::Value::try_from(config).unwrap();
        assert_eq!(
            table["presentation"]["tags"]["opacity"].as_integer(),
            Some(i64::from(percent))
        );
    }
    assert_eq!(
        OpacityPercent::new(101),
        Err(AppearanceValueError::OpacityOutOfRange)
    );
    for value in ["-1", "101", "255", "256", "12.0", "nan", "'12'", "true"] {
        assert!(toml::from_str::<Config>(&format!(
            "[presentation.tags]\nopacity = {value}"
        ))
        .is_err());
    }
}

#[test]
fn visual_config_color_adapter_preserves_every_alpha_byte_and_opaque_channels() {
    use super::presentation::{Rgb, Rgba};
    for alpha in 0..=255 {
        let mut config = Config::default();
        let color = Rgba::from_bytes([1, 127, 254, alpha]);
        config.presentation.highlight.error_background = Some(color);
        let serialized = presentation_document(&config);
        let restored: Config = toml::from_str(&serialized).unwrap();
        assert_eq!(
            restored.presentation.highlight.error_background,
            Some(color)
        );
        assert_eq!(color.to_color_array()[3], f32::from(alpha) / 255.0);
    }
    let color = Rgb::from_bytes([0, 128, 255]);
    assert_eq!(color.bytes(), [0, 128, 255]);
    assert_eq!(color.rgba_bytes(), [0, 128, 255, 255]);
    assert_eq!(color.to_color_array(), [0.0, 128.0 / 255.0, 1.0, 1.0]);
    let config: Config = toml::from_str("[presentation.tags.colors]\naws = 'aBcDeF'\n[presentation.highlight]\nwarning-background = '#00FF80'\n").unwrap();
    assert_eq!(
        config.presentation.tags.colors.aws.unwrap().bytes(),
        [171, 205, 239]
    );
    assert_eq!(
        config
            .presentation
            .highlight
            .warning_background
            .unwrap()
            .bytes(),
        [0, 255, 128, 255]
    );
    let serialized = toml::Value::try_from(config).unwrap();
    assert_eq!(
        serialized["presentation"]["tags"]["colors"]["aws"].as_str(),
        Some("#abcdef")
    );
    assert_eq!(
        serialized["presentation"]["highlight"]["warning-background"].as_str(),
        Some("#00ff80ff")
    );
}

#[test]
fn visual_config_rejects_unknown_nested_fields_and_untrusted_color_values() {
    for source in [
        "[presentation.tags]\nstyle = 'arbitrary'",
        "[presentation.highlight]\nstyle = 'arbitrary'",
        "[presentation.tags]\nunknown = true",
        "[presentation.tags.colors]\nhelm = '#123456'",
        "[presentation.highlight]\nunknown = true",
        "[presentation.highlight.colors]\ncritical = '#123456'",
        "[presentation.tags.colors]\naws = '#12345680'",
        "[presentation.tags.colors]\naws = '#123456ff'",
        "[presentation.tags.colors]\naws = '#123'",
        "[presentation.tags.colors]\naws = '#12gg00'",
        "[presentation.tags.colors]\naws = '#0000٠٠'",
        "[presentation.tags.colors]\naws = [1, 2, 3]",
        "[presentation.tags.colors]\naws = 123456",
        "[presentation.highlight]\nerror-background = '#1234567890'",
    ] {
        assert!(toml::from_str::<Config>(source).is_err());
    }
    assert!(toml::from_str::<Config>(&format!(
        "[presentation.tags.colors]\naws = '{}'",
        "f".repeat(10_000)
    ))
    .is_err());
    use super::presentation::Rgb;
    use serde::Deserialize;
    let error = Rgb::deserialize(serde::de::value::StrDeserializer::<
        serde::de::value::Error,
    >::new("private-placeholder"))
    .unwrap_err();
    assert_eq!(error.to_string(), "invalid appearance hex color");
}
#[test]
fn inline_table_appearance_rejects_invalid_fields_types_and_colors() {
    for value in [
        "border-style = 'zigzag'",
        "border-weight = 9",
        "banding = 'random'",
        "row-lines = 'yes'",
        "header-foreground = '#12345678'",
        "header-background = 'transparent'",
        "unknown = true",
    ] {
        let source = format!("[presentation.tables]\n{value}\n");
        assert!(toml::from_str::<super::Config>(&source).is_err(), "{value}");
    }
}
