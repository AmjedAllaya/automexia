use super::Config;

#[test]
fn presentation_defaults_keep_all_legacy_behaviors_enabled() {
    for config in [
        Config::default(),
        toml::from_str::<Config>("").unwrap(),
        toml::from_str::<Config>("[presentation]").unwrap(),
    ] {
        let value = toml::Value::try_from(config).unwrap();
        let presentation = value.get("presentation").expect("typed presentation table");
        for key in ["inline-tables", "output-highlighting", "command-timestamps"] {
            assert_eq!(
                presentation.get(key).and_then(toml::Value::as_bool),
                Some(true)
            );
        }
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
    // Config has unrelated legacy serialization asymmetries (for example,
    // margin). Roundtrip this newly added typed section through Config itself.
    let section = toml::to_string(&config.presentation).unwrap();
    let decoded: Config = toml::from_str(&format!("[presentation]\n{section}")).unwrap();
    assert_eq!(decoded, config);
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
