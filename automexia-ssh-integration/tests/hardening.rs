//! Independent boundary/property evidence for the non-executing SSH contracts.
use automexia_ssh_integration::{self as ssh, session::RemoteDirectoryUpdate, *};
use proptest::prelude::*;

fn key() -> GenerationKey {
    GenerationKey::new(3, 7).unwrap()
}

#[test]
fn unsupported_status_capability_is_rejected_without_publication() {
    let mut state = Negotiation::with_capabilities(
        key(),
        0,
        1000,
        Capabilities::from_bits(3).unwrap(),
    )
    .unwrap();
    assert!(state.receive(b"AMXSSH1|3|7|1|4", 1).is_err());
    assert_eq!(state.phase(), Phase::Pending);
    assert_eq!(state.capabilities().bits(), 0);
    assert!(state.receive(b"AMXSSH1|3|7|1|3", 2).unwrap());
}
#[test]
fn restricted_capability_ceiling_survives_reconnect() {
    let mut state =
        Negotiation::with_capabilities(key(), 0, 1000, Capabilities::PROMPT).unwrap();
    assert!(state.receive(b"AMXSSH1|3|7|1|3", 1).is_err());
    state
        .reconnect(GenerationKey::new(3, 8).unwrap(), 2, 1000)
        .unwrap();
    assert!(state.receive(b"AMXSSH1|3|8|1|2", 3).is_err());
    assert!(state.receive(b"AMXSSH1|3|8|1|1", 4).unwrap());
}
#[test]
fn directory_envelope_is_remote_and_generation_bound() {
    assert!(
        RemoteDirectoryUpdate::decode(key(), "AMXSSHCWD1|9|7|/same/path")
            .unwrap()
            .is_none()
    );
    assert!(
        RemoteDirectoryUpdate::decode(key(), "AMXSSHCWD1|3|6|/same/path")
            .unwrap()
            .is_none()
    );
    let event = RemoteDirectoryUpdate::decode(key(), "AMXSSHCWD1|3|7|/a b|c/界")
        .unwrap()
        .unwrap();
    assert_eq!(event.key, key());
    assert_eq!(event.path.unwrap().remote_text(), "/a b|c/界");
    let clear = RemoteDirectoryUpdate::decode(key(), "AMXSSHCWD1|3|7|")
        .unwrap()
        .unwrap();
    assert!(clear.path.is_none());
}
#[test]
fn directory_envelope_rejects_controls_bidi_relative_paths_and_oversize() {
    for value in [
        "AMXSSHCWD1|3|7|relative",
        "AMXSSHCWD1|3|7|/x\n",
        "AMXSSHCWD1|3|7|/\u{202e}x",
        "AMXSSHCWD1|03|7|/x",
        "AMXSSHCWD1|3|0|/x",
        "AMXSSHCWD1|3|7",
    ] {
        assert!(
            RemoteDirectoryUpdate::decode(key(), value).is_err(),
            "{value:?}"
        );
    }
    let path = format!("/{}", "a".repeat(ssh::bootstrap::MAX_CWD_PAYLOAD_PATH - 1));
    assert!(
        RemoteDirectoryUpdate::decode(key(), &format!("AMXSSHCWD1|3|7|{path}")).is_ok()
    );
    assert!(
        RemoteDirectoryUpdate::decode(key(), &format!("AMXSSHCWD1|3|7|{path}x")).is_err()
    );
}

#[test]
fn windows_directory_envelopes_remain_remote_scoped_text() {
    for path in [
        "C:\\workspace\\project",
        "D:/workspace/project",
        "\\\\server.example.invalid\\share\\project",
    ] {
        let value = format!("AMXSSHCWD1|3|7|{path}");
        let update = RemoteDirectoryUpdate::decode(key(), &value)
            .unwrap()
            .unwrap();
        assert_eq!(update.path.unwrap().remote_text(), path);
    }
    for path in ["C:relative", "1:\\invalid", "\\relative"] {
        assert!(
            RemoteDirectoryUpdate::decode(key(), &format!("AMXSSHCWD1|3|7|{path}"))
                .is_err()
        );
    }
}

#[test]
fn bundled_shell_adapters_are_bounded_and_leave_no_unexpanded_constants() {
    for shell in [
        RemoteShell::Bash,
        RemoteShell::Zsh,
        RemoteShell::Fish,
        RemoteShell::PowerShell,
        RemoteShell::Pwsh,
    ] {
        let source = bootstrap::interactive_candidate(
            shell,
            GenerationKey::new(u64::MAX, u64::MAX).unwrap(),
        )
        .unwrap_or_else(|error| panic!("{shell:?}: {error}"));
        assert!(!source.contains("@@"));
        assert!(source.len() <= MAX_BOOTSTRAP_BYTES);
        if matches!(shell, RemoteShell::PowerShell | RemoteShell::Pwsh) {
            assert!(source.len() <= bootstrap::MAX_WINDOWS_REMOTE_COMMAND_BYTES);
            assert!(!source.contains("-NoProfile"));
            assert!(!source.contains("ExecutionPolicy"));
        }
    }
    assert_eq!(
        bootstrap::interactive_candidate(RemoteShell::Unknown, key()),
        Err(Error::UnsupportedShell)
    );
}

#[test]
fn powershell_adapter_encodes_only_its_fixed_resource_as_utf16le() {
    use base64::{engine::general_purpose::STANDARD, Engine as _};
    for shell in [RemoteShell::PowerShell, RemoteShell::Pwsh] {
        let candidate = bootstrap::interactive_candidate(shell, key()).unwrap();
        let encoded = candidate.split_once(" -EncodedCommand ").unwrap().1;
        let bytes = STANDARD.decode(encoded).unwrap();
        let words: Vec<_> = bytes
            .chunks_exact(2)
            .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
            .collect();
        assert_eq!(
            String::from_utf16(&words).unwrap(),
            bootstrap::core_candidate(shell, key()).unwrap()
        );
    }
}

#[test]
fn app_owned_user_variable_frames_validate_names_and_budgets() {
    assert_eq!(
        bootstrap::user_var_frame("automexia_ssh_scope", "fixture").unwrap(),
        "\x1b]1337;SetUserVar=automexia_ssh_scope=Zml4dHVyZQ==\x07"
    );
    for name in ["", "0invalid", "bad=name", "bad;name", "bad\x07name", "界"] {
        assert_eq!(
            bootstrap::user_var_frame(name, "value"),
            Err(Error::InvalidFrame)
        );
    }
    assert!(bootstrap::user_var_frame(&"x".repeat(64), &"x".repeat(4096)).is_ok());
    assert!(bootstrap::user_var_frame(&"x".repeat(65), "value").is_err());
    assert!(bootstrap::user_var_frame("valid", &"x".repeat(4097)).is_err());
    assert!(bootstrap::user_var_frame("valid", "x\0y").is_err());
}
#[test]
fn reconnect_does_not_resurrect_old_capabilities_or_paths() {
    let mut state = Negotiation::new(key(), 0, 100).unwrap();
    state.receive(b"AMXSSH1|3|7|1|3", 1).unwrap();
    state.close();
    assert!(!state.receive(b"AMXSSH1|3|7|2|3", 2).unwrap());
    let next = GenerationKey::new(3, 8).unwrap();
    state.reconnect(next, 3, 100).unwrap();
    assert_eq!(state.capabilities().bits(), 0);
    assert!(!state.receive(b"AMXSSH1|3|7|99|3", 4).unwrap());
    assert!(RemoteDirectoryUpdate::decode(next, "AMXSSHCWD1|3|7|/old")
        .unwrap()
        .is_none());
}
#[test]
fn argument_limit_is_shared_with_the_existing_connectivity_owner() {
    assert_eq!(
        ssh::MAX_DESTINATION_BYTES,
        automexia_connectivity::connections::MAX_DIRECT_OPENSSH_DESTINATION_BYTES
    );
}
#[test]
fn negotiation_retains_fixed_inline_state_after_repeated_receipts() {
    assert!(std::mem::size_of::<Negotiation>() <= 128);
    let mut state = Negotiation::new(key(), 0, 1000).unwrap();
    for sequence in 1..=10_000 {
        let frame = format!("AMXSSH1|3|7|{sequence}|3");
        assert!(state.receive(frame.as_bytes(), 1).unwrap());
    }
    assert_eq!(state.phase(), Phase::Ready);
    assert_eq!(state.capabilities().bits(), 3);
}
#[test]
fn bootstrap_preserves_limits_at_maximum_generation() {
    let source = ssh::bootstrap::bash_interactive_candidate(
        GenerationKey::new(u64::MAX, u64::MAX).unwrap(),
    )
    .unwrap();
    assert!(source.len() <= ssh::MAX_BOOTSTRAP_BYTES);
    assert!(!source.contains("sh -c"));
    assert!(!source.contains("\numask 077\n"));
    assert!(!source.contains("OSC 7"));
    assert!(source.contains("AMXSSHCWD1|18446744073709551615|18446744073709551615|"));
}

#[cfg(unix)]
#[test]
fn wrapper_preserves_non_utf8_option_values_without_enhancement() {
    use std::os::unix::ffi::OsStringExt;
    let value = std::ffi::OsString::from_vec(vec![0xff, 0xfe]);
    let args = vec!["-i".into(), value, "host".into()];
    let request = Invocation::new(args.clone()).unwrap();
    assert_eq!(
        request.classify_for_wrapper(true, true),
        InvocationClass::Passthrough(PassReason::NonUnicode)
    );
    assert_eq!(request.arguments(), args);
}

#[cfg(windows)]
#[test]
fn wrapper_preserves_non_unicode_windows_option_values_without_enhancement() {
    use std::os::windows::ffi::OsStringExt;
    let value = std::ffi::OsString::from_wide(&[0xd800]);
    let args = vec!["-i".into(), value, "host".into()];
    let request = Invocation::new(args.clone()).unwrap();
    assert_eq!(
        request.classify_for_wrapper(true, true),
        InvocationClass::Passthrough(PassReason::NonUnicode)
    );
    assert_eq!(request.arguments(), args);
}
proptest! {
    #![proptest_config(ProptestConfig::with_cases(128))]
    #[test]
    fn arbitrary_receipts_cannot_escape_the_capability_ceiling(data in prop::collection::vec(any::<u8>(), 0..256)) {
        let mut state = Negotiation::with_capabilities(key(), 0, 1000, Capabilities::PROMPT).unwrap();
        let _ = state.receive(&data, 1);
        prop_assert_eq!(state.capabilities().bits() & !1, 0);
    }
    #[test]
    fn generations_cannot_cross_panes(pane in 4u64..u64::MAX, generation in 1u64..u64::MAX) {
        let mut state = Negotiation::new(key(), 0, 1000).unwrap();
        let value = format!("AMXSSH1|{pane}|{generation}|1|3");
        prop_assert!(!state.receive(value.as_bytes(), 1).unwrap());
        prop_assert_eq!(state.phase(), Phase::Pending);
    }
    #[test]
    fn denied_plans_never_become_passthrough(host in "[a-z]{1,24}") {
        let invocation = Invocation::new(vec![host.into()]).unwrap();
        let mut options = Options::conservative(key());
        options.policy_denied = true;
        options.mode = Mode::Off;
        prop_assert!(matches!(plan(invocation, options).unwrap(), Decision::Denied));
    }
    #[test]
    fn disabled_invocations_preserve_exact_arguments(values in prop::collection::vec("[^\\x00]{0,32}", 0..32)) {
        let args: Vec<std::ffi::OsString> = values.into_iter().map(Into::into).collect();
        let invocation = Invocation::new(args.clone()).unwrap();
        let mut options = Options::conservative(key());
        options.mode = Mode::Off;
        match plan(invocation, options).unwrap() {
            Decision::Passthrough { original, .. } => prop_assert_eq!(original.arguments(), args.as_slice()),
            _ => prop_assert!(false, "disabled integration changed execution class"),
        }
    }
    #[test]
    fn wrapper_classification_never_rewrites_arguments(values in prop::collection::vec("[^\\x00]{0,32}", 0..32)) {
        let args: Vec<std::ffi::OsString> = values.into_iter().map(Into::into).collect();
        let invocation = Invocation::new(args.clone()).unwrap();
        let _ = invocation.classify_for_wrapper(true, true);
        prop_assert_eq!(invocation.arguments(), args.as_slice());
    }
    #[test]
    fn arbitrary_effective_configuration_is_bounded_and_does_not_panic(data in prop::collection::vec(any::<u8>(), 0..2048)) {
        let result = EffectiveConfig::parse(&data);
        if let Ok(config) = result {
            prop_assert!(std::mem::size_of_val(&config) <= 16);
        }
    }
    #[test]
    fn arbitrary_remote_context_cannot_escape_its_scope(value in ".{0,4096}") {
        use ssh::session::{RemoteContext, RemoteContextField, MAX_REMOTE_CONTEXT_VALUE_BYTES};
        if let Ok(Some(context)) = RemoteContext::decode(key(), &value) {
            prop_assert_eq!(context.key(), key());
            for field in RemoteContextField::ALL {
                if let Some(text) = context.value(field) {
                    prop_assert!(text.len() <= MAX_REMOTE_CONTEXT_VALUE_BYTES);
                    prop_assert!(!text.chars().any(char::is_control));
                }
            }
        }
    }
}
