use automexia_ssh_integration::{bootstrap, *};
use std::ffi::OsString;
fn invocation(args: &[&str]) -> Invocation {
    Invocation::new(args.iter().map(OsString::from).collect()).unwrap()
}
fn key() -> GenerationKey {
    GenerationKey::new(3, 7).unwrap()
}

#[test]
fn scoped_remote_context_is_complete_display_only_and_redacted() {
    use session::{RemoteContext, RemoteContextField};
    let empty = RemoteContext::empty_value(key());
    let mut populated = empty.clone();
    for field in RemoteContextField::ALL {
        populated = populated.replace(
            &format!("{}=\n", field.name()),
            &format!("{}=fixture-{}\n", field.name(), field.name()),
        );
    }
    let parsed = RemoteContext::decode(key(), &populated).unwrap().unwrap();
    assert_eq!(parsed.key(), key());
    for field in RemoteContextField::ALL {
        assert_eq!(
            parsed.value(field),
            Some(format!("fixture-{}", field.name()).as_str())
        );
    }
    assert!(!format!("{parsed:?}").contains("fixture-"));
    assert_eq!(
        RemoteContext::decode(key(), populated.trim_end_matches('\n')).unwrap(),
        Some(parsed)
    );
    let cleared = RemoteContext::decode(key(), &empty).unwrap().unwrap();
    assert!(RemoteContextField::ALL
        .iter()
        .all(|field| cleared.value(*field).is_none()));
    assert_eq!(
        RemoteContext::decode(GenerationKey::new(3, 8).unwrap(), &populated).unwrap(),
        None
    );
}

#[test]
fn remote_context_rejects_partial_duplicate_unknown_and_hostile_values() {
    use session::{RemoteContext, MAX_REMOTE_CONTEXT_BYTES};
    let empty = RemoteContext::empty_value(key());
    for value in [
        empty.replace("git_branch=\n", ""),
        format!("{empty}git_branch=second\n"),
        empty.replace("git_branch=", "unrecognized="),
        empty.replace("AMXSSHCTX1", "AMXSSHCTX2"),
        empty.replace("|3|7|", "|03|7|"),
        format!("{empty}\n"),
        "x".repeat(MAX_REMOTE_CONTEXT_BYTES + 1),
    ] {
        assert!(RemoteContext::decode(key(), &value).is_err());
    }
    for text in [
        "bad\0value",
        "bad\nvalue",
        "bad\rvalue",
        "bad\tvalue",
        "bad\u{0085}value",
        "bad\u{202e}value",
        "bad\u{2069}value",
    ] {
        assert_eq!(
            RemoteContext::decode(
                key(),
                &empty.replace("git_branch=", &format!("git_branch={text}"))
            ),
            Err(Error::InvalidFrame)
        );
    }
}

#[test]
fn remote_context_value_limits_count_utf8_bytes_and_allow_literal_delimiters() {
    use session::{RemoteContext, RemoteContextField};
    let empty = RemoteContext::empty_value(key());
    for value in [
        "x".repeat(256),
        "é".repeat(128),
        "fixture|value=literal".to_owned(),
    ] {
        let wire = empty.replace("git_branch=", &format!("git_branch={value}"));
        assert_eq!(
            RemoteContext::decode(key(), &wire)
                .unwrap()
                .unwrap()
                .value(RemoteContextField::GitBranch),
            Some(value.as_str())
        );
    }
    for value in ["x".repeat(257), "é".repeat(129)] {
        let wire = empty.replace("git_branch=", &format!("git_branch={value}"));
        assert_eq!(
            RemoteContext::decode(key(), &wire),
            Err(Error::InvalidFrame)
        );
    }
}
fn enhanced() -> Options {
    let mut options = Options::conservative(key());
    options.shell = RemoteShell::Bash;
    options.startup = Startup::Interactive;
    options.terminal_input = true;
    options.terminal_output = true;
    options.posix_account_shell = true;
    options.permit_session_files = true;
    options.configuration = ConfigEvidence::CompatibleForPreview;
    options
}
#[test]
fn eligible_invocation_table() {
    for args in [
        vec!["host"],
        vec!["-p", "2222", "alice@host"],
        vec!["-p2222", "host"],
        vec!["-J", "jump", "host"],
        vec!["-vv", "-i", "key with spaces", "host"],
        vec!["--", "host"],
        vec!["-t", "user@[::1]"],
        vec!["-F", "custom-config", "host"],
    ] {
        assert!(
            matches!(
                invocation(&args).classify(true, true),
                InvocationClass::Interactive { .. }
            ),
            "{args:?}"
        );
    }
}
#[test]
fn noninteractive_transports_remain_native() {
    for flag in ["-N", "-n", "-T", "-f", "-s", "-W", "-O", "-G", "-V"] {
        assert!(matches!(
            invocation(&[flag, "host"]).classify(true, true),
            InvocationClass::Passthrough(_)
        ));
    }
}
#[test]
fn explicit_command_is_never_bootstrapped() {
    for args in [
        vec!["host", "printf 'hello'"],
        vec!["host", ""],
        vec!["host", "scp", "-t", "file"],
    ] {
        assert_eq!(
            invocation(&args).classify(true, true),
            InvocationClass::Passthrough(PassReason::ExplicitCommand)
        );
    }
}
#[test]
fn unknown_options_are_not_guessed() {
    for arg in [
        "-oRemoteCommand=evil",
        "-o",
        "-A",
        "-X",
        "-L8080:a:80",
        "-nt",
        "-Z",
    ] {
        assert!(matches!(
            invocation(&[arg, "host"]).classify(true, true),
            InvocationClass::Passthrough(_)
        ));
    }
}
#[test]
fn wrapper_accepts_known_interactive_options_before_observed_config_review() {
    for args in [
        vec!["-A", "host"],
        vec!["-avv", "host"],
        vec!["-o", "ServerAliveInterval=30", "host"],
        vec!["-oIdentityFile=key with spaces", "host"],
        vec!["-L", "8080:localhost:80", "host"],
        vec!["-vp2222", "host"],
        vec!["-i/fixture/界 key", "host"],
    ] {
        assert!(
            matches!(
                invocation(&args).classify_for_wrapper(true, true),
                InvocationClass::Interactive { .. }
            ),
            "known interactive option was not admitted for observed review: {args:?}"
        );
    }
}

#[test]
fn wrapper_counts_option_values_not_option_like_contents() {
    for args in [
        vec!["-i", "-N", "host"],
        vec!["-vvi-N", "host"],
        vec!["-o", "IdentityFile=-N", "host"],
        vec!["-p", "2222", "-o", "ServerAliveInterval=30", "--", "host"],
    ] {
        let request = invocation(&args);
        assert_eq!(
            request.classify_for_wrapper(true, true),
            InvocationClass::Interactive {
                destination_index: args.len() - 1
            },
            "{args:?}"
        );
        assert_eq!(request.arguments(), invocation(&args).arguments());
    }
}

#[test]
fn wrapper_does_not_enhance_control_subsystem_query_or_background_modes() {
    for flag in [
        "-N",
        "-n",
        "-T",
        "-f",
        "-s",
        "-W",
        "-O",
        "-G",
        "-V",
        "-M",
        "-MM",
        "-Q",
        "-qN",
        "-vf",
        "-vvT",
        "-Ocheck",
        "-Qcipher",
        "-Wlocalhost:22",
    ] {
        assert_eq!(
            invocation(&[flag, "host"]).classify_for_wrapper(true, true),
            InvocationClass::Passthrough(PassReason::TransportOrControl),
            "{flag}"
        );
    }
}

#[test]
fn wrapper_leaves_commands_and_nonterminal_streams_native() {
    for args in [
        vec!["-A", "host", "uptime"],
        vec!["host", ""],
        vec!["--", "host", "-N"],
        vec!["host", "printf '%s' '$HOME'"],
    ] {
        assert_eq!(
            invocation(&args).classify_for_wrapper(true, true),
            InvocationClass::Passthrough(PassReason::ExplicitCommand)
        );
    }
    for terminals in [(false, false), (false, true), (true, false)] {
        assert_eq!(
            invocation(&["host"]).classify_for_wrapper(terminals.0, terminals.1),
            InvocationClass::Passthrough(PassReason::NotInteractive)
        );
    }
}

#[test]
fn wrapper_keeps_incomplete_unknown_and_ambiguous_arguments_native() {
    for args in [
        vec!["-"],
        vec!["--unknown", "host"],
        vec!["-vZ", "host"],
        vec!["-p"],
        vec!["-p", "0", "host"],
        vec!["-p65536", "host"],
        vec!["-p+22", "host"],
        vec!["-o", "", "host"],
        vec!["-i", "path\nother", "host"],
        vec!["-界", "host"],
    ] {
        assert_eq!(
            invocation(&args).classify_for_wrapper(true, true),
            InvocationClass::Passthrough(PassReason::UnsupportedOption),
            "{args:?}"
        );
    }
    assert_eq!(
        invocation(&["-v", "--"]).classify_for_wrapper(true, true),
        InvocationClass::Passthrough(PassReason::NoDestination)
    );
    assert_eq!(
        invocation(&["--", "-host"]).classify_for_wrapper(true, true),
        InvocationClass::Passthrough(PassReason::AmbiguousDestination)
    );
}

const INTERACTIVE_CONFIG: &str = "requesttty auto\nsessiontype default\nstdinnull no\nforkafterauthentication no\ncontrolmaster false\n";

#[test]
fn effective_configuration_observes_interactive_settings_without_remotecommand() {
    assert_eq!(
        EffectiveConfig::parse(INTERACTIVE_CONFIG.as_bytes())
            .unwrap()
            .passthrough_reason(),
        None
    );
    for value in ["yes", "force"] {
        let text =
            INTERACTIVE_CONFIG.replace("requesttty auto", &format!("requesttty {value}"));
        assert_eq!(
            EffectiveConfig::parse(text.as_bytes())
                .unwrap()
                .passthrough_reason(),
            None
        );
    }
    for value in ["no", "auto", "autoask"] {
        let text = INTERACTIVE_CONFIG
            .replace("controlmaster false", &format!("controlmaster {value}"));
        assert_eq!(
            EffectiveConfig::parse(text.as_bytes())
                .unwrap()
                .passthrough_reason(),
            None
        );
    }
}

#[test]
fn effective_configuration_keeps_commands_and_noninteractive_modes_native() {
    for command in ["uptime", "none", "", "none\nidentityfile /fixture/key"] {
        let text = format!("{INTERACTIVE_CONFIG}remotecommand {command}\n");
        assert_eq!(
            EffectiveConfig::parse(text.as_bytes())
                .unwrap()
                .passthrough_reason(),
            Some(PassReason::ExplicitCommand)
        );
    }
    for (before, after, reason) in [
        (
            "requesttty auto",
            "requesttty no",
            PassReason::NotInteractive,
        ),
        ("stdinnull no", "stdinnull yes", PassReason::NotInteractive),
        (
            "sessiontype default",
            "sessiontype none",
            PassReason::TransportOrControl,
        ),
        (
            "sessiontype default",
            "sessiontype subsystem",
            PassReason::TransportOrControl,
        ),
        (
            "forkafterauthentication no",
            "forkafterauthentication yes",
            PassReason::TransportOrControl,
        ),
        (
            "controlmaster false",
            "controlmaster true",
            PassReason::TransportOrControl,
        ),
        (
            "controlmaster false",
            "controlmaster ask",
            PassReason::TransportOrControl,
        ),
    ] {
        let text = INTERACTIVE_CONFIG.replace(before, after);
        assert_eq!(
            EffectiveConfig::parse(text.as_bytes())
                .unwrap()
                .passthrough_reason(),
            Some(reason)
        );
    }
}

#[test]
fn effective_configuration_rejects_incomplete_duplicate_unknown_or_malformed_facts() {
    for line in INTERACTIVE_CONFIG.lines() {
        let missing = INTERACTIVE_CONFIG.replace(&format!("{line}\n"), "");
        assert_eq!(
            EffectiveConfig::parse(missing.as_bytes()),
            Err(Error::InvalidConfiguration)
        );
        let duplicate = format!("{INTERACTIVE_CONFIG}{line}\n");
        assert_eq!(
            EffectiveConfig::parse(duplicate.as_bytes()),
            Err(Error::InvalidConfiguration)
        );
        let name = line.split_once(' ').unwrap().0;
        let unknown = INTERACTIVE_CONFIG.replace(line, &format!("{name} unsupported"));
        assert_eq!(
            EffectiveConfig::parse(unknown.as_bytes()),
            Err(Error::InvalidConfiguration)
        );
    }
    for extra in [
        "invalid",
        " key value",
        "requesttty auto\0",
        "remotecommand x\nremotecommand y",
        "hostname x\x1b",
    ] {
        assert_eq!(
            EffectiveConfig::parse(format!("{INTERACTIVE_CONFIG}{extra}\n").as_bytes()),
            Err(Error::InvalidConfiguration)
        );
    }
    assert_eq!(
        EffectiveConfig::parse(b""),
        Err(Error::InvalidConfiguration)
    );
}

#[test]
fn effective_configuration_discards_private_values_and_accepts_crlf() {
    let text = format!("host private-canary\nhostname private-canary\nidentityfile /fixture/private-canary\n{INTERACTIVE_CONFIG}").replace('\n', "\r\n");
    let observed = EffectiveConfig::parse(text.as_bytes()).unwrap();
    assert_eq!(observed.passthrough_reason(), None);
    assert!(!format!("{observed:?}").contains("private-canary"));
    assert!(std::mem::size_of::<EffectiveConfig>() <= 16);
}

#[test]
fn effective_configuration_accepts_native_openssh_mixed_case_keywords() {
    // OpenSSH's real -G dump uses this spelling (including the capital P),
    // although configuration keywords themselves are case-insensitive.
    let text = format!("{INTERACTIVE_CONFIG}canonicalizePermittedcnames none\n");
    assert_eq!(
        EffectiveConfig::parse(text.as_bytes())
            .unwrap()
            .passthrough_reason(),
        None
    );
}

#[test]
fn effective_configuration_preserves_persistent_automatic_masters() {
    for master in ["auto", "autoask"] {
        for persist in ["yes", "30", "3600"] {
            let text = format!(
                "{}controlpersist {persist}\n",
                INTERACTIVE_CONFIG
                    .replace("controlmaster false", &format!("controlmaster {master}"))
            );
            assert_eq!(
                EffectiveConfig::parse(text.as_bytes())
                    .unwrap()
                    .passthrough_reason(),
                Some(PassReason::TransportOrControl)
            );
        }
        for persist in ["no", "0"] {
            let text = format!(
                "{}controlpersist {persist}\n",
                INTERACTIVE_CONFIG
                    .replace("controlmaster false", &format!("controlmaster {master}"))
            );
            assert_eq!(
                EffectiveConfig::parse(text.as_bytes())
                    .unwrap()
                    .passthrough_reason(),
                None
            );
        }
    }
    // ControlMaster=no cannot start a new master. Older -G output may omit
    // ControlPersist entirely; do not make that optional evidence mandatory.
    assert_eq!(
        EffectiveConfig::parse(
            format!("{INTERACTIVE_CONFIG}controlpersist yes\n").as_bytes()
        )
        .unwrap()
        .passthrough_reason(),
        None
    );
    assert_eq!(
        EffectiveConfig::parse(INTERACTIVE_CONFIG.as_bytes())
            .unwrap()
            .passthrough_reason(),
        None
    );
}

#[test]
fn native_interactive_configuration_is_independent_of_bootstrap_eligibility() {
    for master in ["false", "no", "auto", "autoask"] {
        for persist in ["no", "0", "yes", "30"] {
            let text = format!(
                "{}controlpersist {persist}\n",
                INTERACTIVE_CONFIG
                    .replace("controlmaster false", &format!("controlmaster {master}"))
            );
            let config = EffectiveConfig::parse(text.as_bytes()).unwrap();
            assert!(config.native_interactive_eligible(), "{master}/{persist}");
            if matches!(master, "auto" | "autoask") && matches!(persist, "yes" | "30") {
                assert_eq!(
                    config.passthrough_reason(),
                    Some(PassReason::TransportOrControl)
                );
            }
        }
    }
    assert!(EffectiveConfig::parse(INTERACTIVE_CONFIG.as_bytes())
        .unwrap()
        .native_interactive_eligible());
    for tty in ["yes", "force"] {
        let text =
            INTERACTIVE_CONFIG.replace("requesttty auto", &format!("requesttty {tty}"));
        assert!(EffectiveConfig::parse(text.as_bytes())
            .unwrap()
            .native_interactive_eligible());
    }
}

#[test]
fn native_interactive_configuration_rejects_command_subsystem_detached_and_control_modes()
{
    for command in ["uptime", "none", ""] {
        let text = format!("{INTERACTIVE_CONFIG}remotecommand {command}\n");
        assert!(!EffectiveConfig::parse(text.as_bytes())
            .unwrap()
            .native_interactive_eligible());
    }
    for (before, after) in [
        ("requesttty auto", "requesttty no"),
        ("sessiontype default", "sessiontype none"),
        ("sessiontype default", "sessiontype subsystem"),
        ("stdinnull no", "stdinnull yes"),
        ("forkafterauthentication no", "forkafterauthentication yes"),
        ("controlmaster false", "controlmaster true"),
        ("controlmaster false", "controlmaster yes"),
        ("controlmaster false", "controlmaster ask"),
    ] {
        let text = format!(
            "{}controlpersist yes\n",
            INTERACTIVE_CONFIG.replace(before, after)
        );
        assert!(
            !EffectiveConfig::parse(text.as_bytes())
                .unwrap()
                .native_interactive_eligible(),
            "{after}"
        );
    }
}

#[test]
fn effective_configuration_rejects_ambiguous_persistence_evidence() {
    for extra in [
        "controlpersist unsupported\n",
        "controlpersist -1\n",
        "controlpersist \n",
        "controlpersist no\ncontrolpersist yes\n",
    ] {
        assert_eq!(
            EffectiveConfig::parse(format!("{INTERACTIVE_CONFIG}{extra}").as_bytes()),
            Err(Error::InvalidConfiguration)
        );
    }
}

#[test]
fn effective_configuration_has_output_line_and_line_count_budgets() {
    assert_eq!(
        EffectiveConfig::parse(&vec![b'x'; MAX_EFFECTIVE_CONFIG_BYTES + 1]),
        Err(Error::ConfigurationLimit)
    );
    let valid_line = format!("ignored {}", "x".repeat(4096 - 8));
    assert!(EffectiveConfig::parse(
        format!("{INTERACTIVE_CONFIG}{valid_line}\n").as_bytes()
    )
    .is_ok());
    assert_eq!(
        EffectiveConfig::parse(format!("{INTERACTIVE_CONFIG}{valid_line}x\n").as_bytes()),
        Err(Error::ConfigurationLimit)
    );
    let exactly_full =
        format!("{INTERACTIVE_CONFIG}{}", "ignored value\n".repeat(1024 - 5));
    assert!(EffectiveConfig::parse(exactly_full.as_bytes()).is_ok());
    assert_eq!(
        EffectiveConfig::parse(format!("{exactly_full}ignored value\n").as_bytes()),
        Err(Error::ConfigurationLimit)
    );
}
#[test]
fn missing_and_invalid_option_values_are_native() {
    for args in [
        vec!["-p"],
        vec!["-p", "0", "host"],
        vec!["-p65536", "host"],
        vec!["-l", "", "host"],
    ] {
        assert!(matches!(
            invocation(&args).classify(true, true),
            InvocationClass::Passthrough(_)
        ));
    }
}
#[test]
fn stdin_and_stdout_must_both_be_terminals() {
    for (input, output) in [(false, false), (true, false), (false, true)] {
        assert_eq!(
            invocation(&["host"]).classify(input, output),
            InvocationClass::Passthrough(PassReason::NotInteractive)
        );
    }
}
#[test]
fn arguments_are_preserved_exactly() {
    let original = vec![
        OsString::from("host"),
        OsString::from("printf '%s' \"$HOME\""),
        OsString::from(""),
    ];
    let request = Invocation::new(original.clone()).unwrap();
    match plan(request, enhanced()).unwrap() {
        Decision::Passthrough {
            original: actual, ..
        } => assert_eq!(actual.arguments(), original),
        other => panic!("unexpected {other:?}"),
    }
}
#[test]
fn no_destination_is_not_an_enhanced_session() {
    assert_eq!(
        invocation(&[]).classify(true, true),
        InvocationClass::Passthrough(PassReason::NoDestination)
    );
}
#[test]
fn hostile_destination_stays_uninterpreted() {
    for host in [
        "bad host",
        "host;touch /tmp/marker",
        "$(id)",
        "host\nextra",
        "--bad",
    ] {
        assert!(matches!(
            invocation(&["--", host]).classify(true, true),
            InvocationClass::Passthrough(_)
        ));
    }
}
#[test]
fn oversize_and_nul_are_rejected_before_planning() {
    assert!(Invocation::new(vec![OsString::from("x"); MAX_ARGUMENTS + 1]).is_err());
    assert!(
        Invocation::new(vec![OsString::from("x".repeat(MAX_ARGUMENT_BYTES + 1))])
            .is_err()
    );
    assert!(Invocation::new(vec![OsString::from("a\0b")]).is_err());
}
#[cfg(unix)]
#[test]
fn non_utf8_os_arguments_are_not_lossily_rewritten() {
    use std::os::unix::ffi::OsStringExt;
    let argument = OsString::from_vec(vec![0xff, 0xfe]);
    let request = Invocation::new(vec![argument.clone()]).unwrap();
    assert_eq!(
        request.classify(true, true),
        InvocationClass::Passthrough(PassReason::NonUnicode)
    );
    assert_eq!(request.arguments()[0], argument);
}
#[test]
fn explicit_policy_denial_never_becomes_passthrough() {
    for mode in [Mode::Off, Mode::Auto, Mode::Required] {
        let mut options = enhanced();
        options.mode = mode;
        options.policy_denied = true;
        assert_eq!(
            plan(invocation(&["host"]), options).unwrap(),
            Decision::Denied
        );
    }
}
#[test]
fn disabled_mode_keeps_native_arguments() {
    let mut options = enhanced();
    options.mode = Mode::Off;
    assert!(matches!(
        plan(invocation(&["host"]), options).unwrap(),
        Decision::Passthrough {
            reason: PassReason::Disabled,
            ..
        }
    ));
}
#[test]
fn unknown_configuration_is_not_probed() {
    let mut options = enhanced();
    options.configuration = ConfigEvidence::Unknown;
    assert!(matches!(
        plan(invocation(&["host"]), options).unwrap(),
        Decision::Passthrough {
            reason: PassReason::UnknownConfiguration,
            ..
        }
    ));
}
#[test]
fn required_mode_does_not_silently_downgrade() {
    let mut options = enhanced();
    options.mode = Mode::Required;
    options.shell = RemoteShell::Unknown;
    assert!(matches!(
        plan(invocation(&["host"]), options).unwrap(),
        Decision::Unavailable {
            reason: PassReason::UnsupportedShell
        }
    ));
}
#[test]
fn unimplemented_shells_are_explicit_fallbacks() {
    for shell in [
        RemoteShell::Unknown,
        RemoteShell::Zsh,
        RemoteShell::Fish,
        RemoteShell::PowerShell,
        RemoteShell::Pwsh,
    ] {
        let mut options = enhanced();
        options.shell = shell;
        assert!(matches!(
            plan(invocation(&["host"]), options).unwrap(),
            Decision::Passthrough {
                reason: PassReason::UnsupportedShell,
                ..
            }
        ));
    }
}
#[test]
fn login_semantics_are_not_faked() {
    let mut options = enhanced();
    options.startup = Startup::Login;
    assert!(matches!(
        plan(invocation(&["host"]), options).unwrap(),
        Decision::Passthrough {
            reason: PassReason::UnsupportedStartup,
            ..
        }
    ));
}
#[test]
fn unknown_account_shell_dialect_refuses_bootstrap() {
    let mut options = enhanced();
    options.posix_account_shell = false;
    assert!(matches!(
        plan(invocation(&["host"]), options).unwrap(),
        Decision::Passthrough {
            reason: PassReason::UnsupportedShell,
            ..
        }
    ));
}
#[test]
fn remote_writes_require_explicit_policy() {
    let mut options = enhanced();
    options.permit_session_files = false;
    assert!(matches!(
        plan(invocation(&["host"]), options).unwrap(),
        Decision::Passthrough {
            reason: PassReason::RemoteWritesDenied,
            ..
        }
    ));
}
#[test]
fn candidate_is_a_new_complete_argument_vector() {
    let args = ["-p", "22", "--", "host"];
    let Decision::Candidate(candidate) = plan(invocation(&args), enhanced()).unwrap()
    else {
        panic!("candidate");
    };
    let proposed = candidate.proposed_arguments_for_new_review();
    assert_eq!(proposed[0], "-t");
    assert_eq!(&proposed[1..1 + args.len()], invocation(&args).arguments());
    assert!(proposed
        .last()
        .unwrap()
        .to_str()
        .unwrap()
        .starts_with("amx_plain()"));
    assert_eq!(
        candidate.original().arguments(),
        invocation(&args).arguments()
    );
}
#[test]
fn debug_does_not_leak_hosts_or_command_payloads() {
    let request = invocation(&["secret-host"]);
    assert!(!format!("{request:?}").contains("secret-host"));
    let decision = plan(request, enhanced()).unwrap();
    assert!(!format!("{decision:?}").contains("secret-host"));
    assert!(!format!("{decision:?}").contains("PROMPT_COMMAND"));
}
#[test]
fn bootstrap_size_and_receipt_identity_are_bounded() {
    for pane in [1, u64::MAX] {
        for generation in [1, u64::MAX] {
            let key = GenerationKey::new(pane, generation).unwrap();
            assert!(
                bootstrap::bash_interactive_candidate(key).unwrap().len()
                    <= MAX_BOOTSTRAP_BYTES
            );
            assert!(bootstrap::readiness_value(key).len() <= MAX_FRAME_BYTES);
        }
    }
}
#[test]
fn invalid_keys_are_rejected() {
    assert!(GenerationKey::new(0, 1).is_err());
    assert!(GenerationKey::new(1, 0).is_err());
}
#[test]
fn deadlines_have_no_overflow_or_unbounded_duration() {
    assert!(Negotiation::new(key(), u64::MAX, 1).is_err());
    assert!(Negotiation::new(key(), 0, 0).is_err());
    assert!(Negotiation::new(key(), 0, 30_001).is_err());
}
#[test]
fn readiness_is_scoped_and_idempotent() {
    let mut state = Negotiation::new(key(), 0, 100).unwrap();
    assert!(!state.receive(b"AMXSSH1|4|7|1|3", 1).unwrap());
    assert!(!state.receive(b"AMXSSH1|3|6|1|3", 2).unwrap());
    assert!(state.receive(b"AMXSSH1|3|7|1|3", 3).unwrap());
    assert_eq!(state.phase(), Phase::Ready);
    assert!(state.capabilities().contains(Capabilities::PROMPT));
    assert!(!state.capabilities().contains(Capabilities::COMMAND_STATUS));
    assert!(!state.receive(b"AMXSSH1|3|7|1|3", 4).unwrap());
}
#[test]
fn malformed_receipts_never_grant_capabilities() {
    for frame in [
        "AMXSSH2|3|7|1|3",
        "AMXSSH1|3|7|0|3",
        "AMXSSH1|03|7|1|3",
        "AMXSSH1|3|7|1|255",
        "AMXSSH1|3|7|1|3|extra",
        "AMXSSH1|3|7|1|3\n",
        "AMXSSH1|3|7|1|+3",
        "AMXSSH1|3|7|1|256",
    ] {
        let mut state = Negotiation::new(key(), 0, 100).unwrap();
        assert!(state.receive(frame.as_bytes(), 1).is_err(), "{frame}");
        assert_eq!(state.capabilities().bits(), 0);
        assert_eq!(state.phase(), Phase::Pending);
    }
}
#[test]
fn oversized_and_non_ascii_receipts_reject() {
    let mut state = Negotiation::new(key(), 0, 100).unwrap();
    assert!(state.receive(&[b'x'; MAX_FRAME_BYTES + 1], 1).is_err());
    assert!(state.receive(&[0xff], 2).is_err());
}
#[test]
fn timeout_is_integration_fallback_not_connection_failure() {
    let mut state = Negotiation::new(key(), 0, 100).unwrap();
    assert!(!state.tick(99).unwrap());
    assert!(state.tick(100).unwrap());
    assert_eq!(state.phase(), Phase::NativeFallback);
    assert!(!state.receive(b"AMXSSH1|3|7|1|3", 101).unwrap());
}
#[test]
fn ready_session_does_not_need_a_polling_heartbeat() {
    let mut state = Negotiation::new(key(), 0, 100).unwrap();
    state.receive(b"AMXSSH1|3|7|1|3", 1).unwrap();
    assert!(!state.tick(1_000_000).unwrap());
    assert_eq!(state.phase(), Phase::Ready);
}
#[test]
fn monotonic_clock_regression_is_reported() {
    let mut state = Negotiation::new(key(), 50, 100).unwrap();
    assert_eq!(state.tick(49), Err(Error::ClockRegression));
}
#[test]
fn close_and_reconnect_revoke_old_receipts() {
    let mut state = Negotiation::new(key(), 0, 100).unwrap();
    state.receive(b"AMXSSH1|3|7|1|3", 1).unwrap();
    state.close();
    assert_eq!(state.capabilities().bits(), 0);
    assert!(!state.receive(b"AMXSSH1|3|7|2|3", 2).unwrap());
    state
        .reconnect(GenerationKey::new(3, 8).unwrap(), 3, 100)
        .unwrap();
    assert!(!state.receive(b"AMXSSH1|3|7|3|3", 4).unwrap());
    assert!(state.receive(b"AMXSSH1|3|8|1|3", 5).unwrap());
}
#[test]
fn reconnect_cannot_reuse_or_change_pane_identity() {
    let mut state = Negotiation::new(key(), 0, 100).unwrap();
    assert!(state.reconnect(key(), 1, 100).is_err());
    assert!(state
        .reconnect(GenerationKey::new(4, 8).unwrap(), 1, 100)
        .is_err());
}
#[test]
fn capability_downgrade_is_local_to_one_session() {
    let mut left = Negotiation::new(key(), 0, 100).unwrap();
    let mut right = Negotiation::new(GenerationKey::new(4, 7).unwrap(), 0, 100).unwrap();
    left.receive(b"AMXSSH1|3|7|1|3", 1).unwrap();
    right.receive(b"AMXSSH1|4|7|1|3", 1).unwrap();
    left.receive(b"AMXSSH1|3|7|2|0", 2).unwrap();
    assert_eq!(left.phase(), Phase::NativeFallback);
    assert_eq!(right.phase(), Phase::Ready);
}
#[test]
fn remote_paths_remain_scoped_and_redacted() {
    for path in ["/home/secret/project", "C:\\Users\\secret", "/tmp/界"] {
        let remote = RemotePath::new(key(), path.into()).unwrap();
        assert_eq!(remote.remote_text(), path);
        assert_eq!(remote.key(), key());
        assert!(!format!("{remote:?}").contains("secret"));
    }
}
#[test]
fn hostile_or_oversize_remote_path_is_rejected() {
    for value in [
        String::new(),
        "x".repeat(MAX_REMOTE_PATH_BYTES + 1),
        "x\0y".into(),
        "x\n".into(),
        "x\u{202e}y".into(),
    ] {
        assert!(RemotePath::new(key(), value).is_err());
    }
}

#[cfg(unix)]
#[test]
fn posix_quote_is_checked_by_an_independent_shell() {
    for value in ["", "a'b", "$(printf BAD)", "x; printf BAD", "a\nb", "界"] {
        let script = format!("printf '%s' {}", bootstrap::quote_posix(value).unwrap());
        let output = std::process::Command::new("/bin/sh")
            .args(["-c", &script])
            .env_clear()
            .output()
            .unwrap();
        assert!(output.status.success());
        assert_eq!(output.stdout, value.as_bytes());
    }
}
#[cfg(unix)]
#[test]
fn complete_generated_bootstrap_is_valid_posix_syntax() {
    let script = bootstrap::bash_interactive_candidate(key()).unwrap();
    let output = std::process::Command::new("/bin/sh")
        .args(["-n", "-c", &script])
        .env_clear()
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
