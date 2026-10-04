use rio_backend::config::profiles::{ProfileDocument, ProfileError, ProfilePlatform};

#[test]
fn profile_document_roundtrips_exact_arguments_and_rejects_future_schema() {
    let source = r#"version = 1
[[profiles]]
id = "work"
name = "Work"
program = "/bin/bash"
args = ["--login", "a b", "$(literal)"]
"#;
    let document = ProfileDocument::parse(source).unwrap();
    assert_eq!(document.profiles[0].args, ["--login", "a b", "$(literal)"]);
    assert_eq!(
        ProfileDocument::parse(&document.to_toml().unwrap()).unwrap(),
        document
    );
    assert_eq!(
        ProfileDocument::parse(&source.replace("version = 1", "version = 2")),
        Err(ProfileError::Version)
    );
}

#[test]
fn profiles_reject_duplicate_ids_controls_and_environment_values() {
    let source =
        "version = 1\n[[profiles]]\nid = 'work'\nname = 'Work'\nprogram = 'bash'\n";
    assert!(ProfileDocument::parse(
        &(source.to_owned()
            + "[[profiles]]\nid = 'work'\nname = 'Other'\nprogram = 'zsh'\n")
    )
    .is_err());
    assert!(ProfileDocument::parse(
        &(source.to_owned()
            + "[profiles.environment]\nTOKEN = { value = 'must-not-save' }\n")
    )
    .is_err());
    assert!(ProfileDocument::parse(
        &source.replace("name = 'Work'", "name = \"bad\\u001b\"")
    )
    .is_err());
}

#[test]
fn platform_matching_is_explicit() {
    assert!(ProfilePlatform::Any.supports(ProfilePlatform::Linux));
    assert!(!ProfilePlatform::Windows.supports(ProfilePlatform::Macos));
}

#[test]
fn profile_resolution_preserves_exact_argv_and_does_not_mutate_base() {
    use rio_backend::config::{profiles::*, Config};
    let base = Config {
        env_vars: vec!["PUBLIC_MODE=base".into()],
        use_fork: true,
        ..Config::default()
    };
    let mut p = NamedProfile::new("work".into(), "Work".into());
    p.program = Some("bash".into());
    p.args = vec!["--login".into(), "a b".into(), "$literal".into()];
    p.directory = ProfileDirectory::Inherit;
    p.environment = vec![EnvironmentReference {
        name: "PUBLIC_MODE".into(),
        from: "SOURCE_MODE".into(),
    }];
    let resolved = p
        .resolve(
            &base,
            ProfilePlatform::Linux,
            Some("/workspace/project"),
            Some("/workspace/home"),
            |name| (name == "SOURCE_MODE").then(|| "session".into()),
        )
        .unwrap();
    assert_eq!(resolved.shell.args, ["--login", "a b", "$literal"]);
    assert_eq!(resolved.working_dir.as_deref(), Some("/workspace/project"));
    assert_eq!(resolved.env_vars, ["PUBLIC_MODE=session"]);
    assert_eq!(resolved.named_profile_identity.as_deref(), Some("work"));
    assert!(!resolved.use_fork);
    assert_eq!(base.env_vars, ["PUBLIC_MODE=base"]);
    assert!(base.use_fork);
    assert!(base.named_profile_identity.is_none());
    assert!(!ProfileDocument {
        version: 1,
        profiles: vec![p]
    }
    .to_toml()
    .unwrap()
    .contains("session"));
}
#[test]
fn environment_resolution_is_case_sensitive_on_unix_and_missing_references_fail() {
    use rio_backend::config::{profiles::*, Config};
    let base = Config {
        env_vars: vec!["MODE=base".into()],
        ..Config::default()
    };
    let mut p = NamedProfile::new("work".into(), "Work".into());
    p.environment = vec![EnvironmentReference {
        name: "mode".into(),
        from: "SOURCE".into(),
    }];
    assert_eq!(
        p.resolve(&base, ProfilePlatform::Linux, None, None, |_| None)
            .unwrap_err()
            .to_string(),
        ProfileError::Environment.to_string()
    );
    let unix = p
        .resolve(&base, ProfilePlatform::Linux, None, None, |_| {
            Some("value".into())
        })
        .unwrap();
    assert_eq!(unix.env_vars, ["MODE=base", "mode=value"]);
    let windows = p
        .resolve(&base, ProfilePlatform::Windows, None, None, |_| {
            Some("value".into())
        })
        .unwrap();
    assert_eq!(windows.env_vars, ["mode=value"]);
    p.environment[0].name = "AUTOMEXIA_SHELL_INTEGRATION".into();
    assert_eq!(p.validate(), Err(ProfileError::Environment));
}
#[test]
fn user_profiles_override_matching_config_id_without_modifying_config() {
    use rio_backend::config::profiles::*;
    let base = ProfileDocument {
        version: 1,
        profiles: vec![NamedProfile::new("work".into(), "Configured".into())],
    };
    let user = ProfileDocument {
        version: 1,
        profiles: vec![NamedProfile::new("work".into(), "Saved".into())],
    };
    assert_eq!(base.merged(&user).unwrap().profiles[0].name, "Saved");
    assert_eq!(base.profiles[0].name, "Configured");
}

#[test]
fn configured_directory_behavior_and_profile_limits_are_preserved() {
    use rio_backend::config::{profiles::*, Config};
    let mut base = Config {
        working_dir: Some("/workspace/configured".into()),
        ..Config::default()
    };
    let mut profile = NamedProfile::new("work".into(), "Work".into());
    base.navigation.current_working_directory = true;
    assert_eq!(
        profile
            .resolve(
                &base,
                ProfilePlatform::Linux,
                Some("/workspace/current"),
                None,
                |_| None
            )
            .unwrap()
            .working_dir
            .as_deref(),
        Some("/workspace/current")
    );
    base.navigation.current_working_directory = false;
    assert_eq!(
        profile
            .resolve(
                &base,
                ProfilePlatform::Linux,
                Some("/workspace/current"),
                None,
                |_| None
            )
            .unwrap()
            .working_dir,
        base.working_dir
    );
    profile.directory = ProfileDirectory::Home;
    assert!(profile
        .resolve(&base, ProfilePlatform::Linux, None, None, |_| None)
        .is_err());
    profile.program = Some("bash".into());
    profile.args = vec!["x".into(); MAX_ARGUMENTS + 1];
    assert_eq!(profile.validate(), Err(ProfileError::Limit));
    profile.args.clear();
    profile.icon = Some("long".into());
    assert_eq!(profile.validate(), Err(ProfileError::Invalid));
}
