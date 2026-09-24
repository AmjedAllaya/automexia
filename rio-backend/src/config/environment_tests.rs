use super::environment::parse_environment;
use super::{Config, PlatformConfig};

#[test]
fn environment_batch_preserves_equals_unicode_empty_values_and_order() {
    let entries = [
        " FIRST =left=right",
        "GREETING=hello 世界",
        "EMPTY=",
        "FIRST=last",
    ]
    .map(String::from);
    assert_eq!(
        parse_environment(&entries).unwrap(),
        [
            ("FIRST", "left=right"),
            ("GREETING", "hello 世界"),
            ("EMPTY", ""),
            ("FIRST", "last")
        ]
        .map(|(name, value)| (name.to_owned(), value.to_owned()))
    );
    assert!(parse_environment(&[]).unwrap().is_empty());
}

#[test]
fn environment_batch_rejects_every_invalid_entry_without_returning_partial_state() {
    for invalid in [
        "missing-separator",
        "=value",
        "  =value",
        "BAD\0KEY=value",
        "KEY=value\0suffix",
    ] {
        let entries = [
            "FIRST=valid".to_owned(),
            invalid.to_owned(),
            "LAST=valid".to_owned(),
        ];
        let mut applied = Vec::new();
        let result = parse_environment(&entries).map(|batch| applied.extend(batch));
        let error = result.unwrap_err().to_string();
        assert!(applied.is_empty());
        assert!(error.contains("entry 2"));
        assert!(!error.contains(invalid));
        assert!(!error.contains("valid"));
    }
}

#[test]
fn configuration_environment_validation_includes_each_platform_override() {
    for platform in ["linux", "windows", "macos"] {
        let mut config = Config::default();
        config.env_vars = vec!["GLOBAL=value".into()];
        let overrides = PlatformConfig {
            env_vars: Some(vec!["GOOD=value".into(), "BAD=value\0suffix".into()]),
            ..PlatformConfig::default()
        };
        match platform {
            "linux" => config.platform.linux = Some(overrides),
            "windows" => config.platform.windows = Some(overrides),
            "macos" => config.platform.macos = Some(overrides),
            _ => unreachable!(),
        }
        assert!(config.validate_environment().is_err());
        assert_eq!(config.env_vars, ["GLOBAL=value"]);
    }
}
