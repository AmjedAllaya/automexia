use super::*;
use crate::event::VoidListener;
use crate::performer::handler::Processor;
use command_actions::CommandActionError as E;
fn terminal() -> Crosswords<VoidListener> {
    Crosswords::new(
        CrosswordsSize::new(40, 8),
        CursorShape::Block,
        VoidListener {},
        WindowId::from(0),
        0,
        256,
    )
}
fn feed(t: &mut Crosswords<VoidListener>, text: &str) {
    Processor::default().advance(t, text.as_bytes());
}
fn completed(t: &mut Crosswords<VoidListener>) {
    feed(t, "\x1b]133;A;aid=1\x07prefix> \x1b]133;B\x07echo hello\r\n\x1b]133;C\x07hello\r\n\x1b]133;D;0\x07\x1b]133;A;aid=2\x07prefix> \x1b]133;B\x07");
}

fn cmd_edit(t: &mut Crosswords<VoidListener>, text: &str) {
    for (key, value) in [
        ("automexia_shell", "1"),
        ("automexia_shell_name", "CMD"),
        ("automexia_prompt_active", "1"),
    ] {
        t.user_vars.insert(key.into(), value.into());
    }
    feed(t, "\x1b[?9001h\x1b]133;A;aid=1\x07> \x1b]133;B\x07");
    feed(t, text);
    feed(t, "\r\x1b[2C");
}

fn wsl_prompt(t: &mut Crosswords<VoidListener>, shell: &str, draft: &str) {
    for (key, value) in [
        ("automexia_shell", "1"),
        ("automexia_shell_name", shell),
        ("automexia_prompt_active", "1"),
    ] {
        t.user_vars.insert(key.into(), value.into());
    }
    let prompt = if shell == "CMD" {
        "\x1b]133;A\x07> \x1b]133;B\x07"
    } else {
        "\x1b]133;A;aid=1\x07> \x1b]133;B\x07"
    };
    feed(t, prompt);
    feed(t, draft);
}

#[test]
fn wsl_setup_hint_only_matches_exact_bare_commands_at_owned_host_prompts() {
    for shell in ["PowerShell", "CMD"] {
        for (draft, expected) in [
            ("wsl", true),
            ("WSL.EXE", true),
            ("wsl\x1b[K", true),
            ("wsl --list", false),
            ("echo wsl", false),
            ("wsl & echo text", false),
            ("wsl --list\x1b[7D", false),
            ("wsl\x1b[8m\x1b[1C\x1b[0m", false),
            ("\x1b[8mwsl\x1b[0m", false),
            ("wsl\u{0301}", false),
            ("wsl\r\nother\x1b[1;6H", false),
        ] {
            let mut t = terminal();
            wsl_prompt(&mut t, shell, draft);
            assert_eq!(t.is_bare_wsl_prompt_input(), expected, "{shell}: {draft:?}");
        }
    }
}

#[test]
fn wsl_setup_hint_rejects_nonlocal_and_inactive_prompts() {
    let mut t = terminal();
    feed(&mut t, "> wsl");
    assert!(!t.is_bare_wsl_prompt_input());

    for mode in [Mode::ALT_SCREEN, Mode::VI, Mode::MOUSE_MODE] {
        let mut t = terminal();
        wsl_prompt(&mut t, "PowerShell", "wsl");
        t.mode.insert(mode);
        assert!(!t.is_bare_wsl_prompt_input());
    }
    for (name, value) in [
        ("automexia_distro", "Fixture-Distro"),
        ("automexia_env_pending", "1"),
        ("automexia_prompt_active", "0"),
        ("automexia_shell_name", "bash"),
    ] {
        let mut t = terminal();
        wsl_prompt(&mut t, "PowerShell", "wsl");
        t.user_vars.insert(name.into(), value.into());
        assert!(!t.is_bare_wsl_prompt_input());
    }
    let mut t = terminal();
    wsl_prompt(&mut t, "PowerShell", "wsl");
    t.apply_integration_scope("AMXSCOPE1|begin|66687aadf862bd776c8fc18b8e9f8e20089714856ee233b3902a591d0d5f2925|1|2|bash");
    assert!(!t.is_bare_wsl_prompt_input());
}

#[test]
fn wsl_setup_hint_expires_when_the_windows_shell_returns() {
    for shell in ["PowerShell", "CMD"] {
        let mut t = terminal();
        wsl_prompt(&mut t, shell, "wsl");
        let receipt = t.wsl_setup_receipt().unwrap();
        assert!(t.wsl_setup_hint_still_current(receipt));

        // PowerShell can redraw the same logical prompt without returning.
        if shell == "PowerShell" {
            feed(&mut t, "\r\x1b]133;A;aid=1\x07> \x1b]133;B\x07wsl");
            assert!(t.wsl_setup_hint_still_current(receipt));
        }
        feed(&mut t, "\x1b]133;C\x07\r\n");
        assert!(t.wsl_setup_hint_still_current(receipt));
        t.user_vars
            .insert("automexia_shell_name".into(), "bash".into());
        feed(&mut t, "\x1b]133;A;aid=50\x07~ $ \x1b]133;B\x07");
        assert!(t.wsl_setup_hint_still_current(receipt));

        t.user_vars
            .insert("automexia_shell_name".into(), shell.into());
        let returned = if shell == "CMD" {
            "\x1b]133;A\x07> \x1b]133;B\x07"
        } else {
            "\x1b]133;A;aid=2\x07> \x1b]133;B\x07"
        };
        feed(&mut t, returned);
        assert!(!t.wsl_setup_hint_still_current(receipt), "{shell}");
    }
}

#[test]
fn wsl_setup_hint_expires_when_remote_scope_takes_over() {
    let mut t = terminal();
    wsl_prompt(&mut t, "PowerShell", "wsl");
    let receipt = t.wsl_setup_receipt().unwrap();
    t.apply_integration_scope("AMXSCOPE1|begin|66687aadf862bd776c8fc18b8e9f8e20089714856ee233b3902a591d0d5f2925|1|2|bash");
    assert!(!t.wsl_setup_hint_still_current(receipt));
}

#[test]
fn wsl_host_enter_receipt_matches_only_the_current_capable_prompt() {
    use std::time::{Duration, Instant};

    let now = Instant::now();
    for preceding_boundary in [
        "",
        "\r\x1b]133;A;aid=1\x07> \x1b]133;B;amx-wsl=1\x07",
        "\r\x1b]133;A;aid=2\x07> \x1b]133;B;amx-wsl=1\x07",
        "\x1b]133;D;0\x07",
        "\x1bc",
    ] {
        let mut t = terminal();
        wsl_prompt(&mut t, "PowerShell", "");
        feed(&mut t, "\x1b]133;B;amx-wsl=1\x07");
        let owner = t.wsl_setup_receipt().unwrap();
        assert!(!t.is_bare_wsl_prompt_input());
        assert_eq!(t.note_wsl_host_enter(now), None);
        feed(&mut t, preceding_boundary);
        t.note_interactive_input(); // Win32 Enter release is ordinary input.
        let expected = (preceding_boundary.is_empty()
            || preceding_boundary.contains("aid=1"))
        .then_some(owner);
        assert_eq!(
            t.accept_wsl_command_start(true, || now + Duration::from_millis(50)),
            expected,
            "{preceding_boundary:?}"
        );
        assert_eq!(t.accept_wsl_command_start(true, || now), None);
    }

    let mut t = terminal();
    wsl_prompt(&mut t, "PowerShell", "");
    feed(&mut t, "\x1b]133;B;amx-wsl=1\x07");
    assert_eq!(t.note_wsl_host_enter(now), None);
    assert_eq!(
        t.accept_wsl_command_start(true, || now + Duration::from_secs(2)),
        None
    );
}

#[test]
fn wsl_host_enter_receipt_does_not_survive_remote_scope_rollover() {
    let now = std::time::Instant::now();
    let mut t = terminal();
    wsl_prompt(&mut t, "PowerShell", "");
    feed(&mut t, "\x1b]133;B;amx-wsl=1\x07");
    assert_eq!(t.note_wsl_host_enter(now), None);
    t.apply_integration_scope("AMXSCOPE1|begin|66687aadf862bd776c8fc18b8e9f8e20089714856ee233b3902a591d0d5f2925|1|2|bash");
    assert!(t.integration_scope_active());
    t.apply_integration_scope(
        "AMXSCOPE1|end|0000000000000000000000000000000000000000000000000000000000000000",
    );
    assert!(!t.integration_scope_active());
    assert_eq!(t.accept_wsl_command_start(true, || now), None);
}

#[test]
fn cmd_word_delete_uses_live_input_graphemes_and_never_submits() {
    for (text, count) in [
        ("alpha beta", 5),
        ("  alpha beta", 7),
        ("", 0),
        ("caf\u{301}é next", 4),
        ("界面 next", 2),
        ("👩\u{200d}💻 next", 1),
    ] {
        let mut t = terminal();
        cmd_edit(&mut t, text);
        let receipt = t.cmd_word_delete_receipt().unwrap();
        let bytes = t.accept_cmd_word_delete(receipt);
        if count == 0 {
            assert!(bytes.is_none());
        } else {
            assert_eq!(
                bytes.unwrap(),
                format!("\x1b[46;83;0;1;256;{count}_\x1b[46;83;0;0;256;1_").into_bytes(),
                "{text}"
            );
        }
        assert!(
            t.accept_cmd_word_delete(receipt).is_none(),
            "receipt is one use"
        );
    }
}

#[test]
fn cmd_word_delete_rejects_stale_and_unowned_edits() {
    for change in 0..9 {
        let mut t = terminal();
        cmd_edit(&mut t, "alpha beta");
        let receipt = t.cmd_word_delete_receipt().unwrap();
        match change {
            0 => t.note_interactive_input(),
            1 => feed(&mut t, "\x1b[C"),
            2 => t.clear_screen_and_history(),
            3 => feed(&mut t, "\x1b[?1049h"),
            4 => feed(&mut t, "\x1b]133;C\x07"),
            5 => {
                t.user_vars
                    .insert("automexia_env_pending".into(), "1".into());
            }
            6 => {
                t.user_vars
                    .insert("automexia_shell_name".into(), "bash".into());
            }
            7 => feed(&mut t, "\x1b[?9001l"),
            _ => feed(&mut t, "\x1b[8malpha"),
        }
        assert!(
            t.accept_cmd_word_delete(receipt).is_none(),
            "change {change}"
        );
    }
}

#[test]
fn cmd_word_delete_respects_prefix_middle_end_and_soft_wrap() {
    let mut t = terminal();
    cmd_edit(&mut t, "alpha beta");
    feed(&mut t, "\x1b[2C");
    let receipt = t.cmd_word_delete_receipt().unwrap();
    assert!(
        String::from_utf8(t.accept_cmd_word_delete(receipt).unwrap())
            .unwrap()
            .contains("256;3_")
    );
    feed(&mut t, "\r");
    assert!(
        t.cmd_word_delete_receipt().is_none(),
        "prefix belongs to shell"
    );
    feed(&mut t, "\x1b[12C");
    let receipt = t.cmd_word_delete_receipt().unwrap();
    assert!(
        t.accept_cmd_word_delete(receipt).is_none(),
        "end cannot recall history"
    );

    let mut t = terminal();
    cmd_edit(&mut t, &format!("{} tail", "x".repeat(45)));
    feed(&mut t, "\x1b[H\x1b[2C");
    let receipt = t.cmd_word_delete_receipt().unwrap();
    assert!(
        String::from_utf8(t.accept_cmd_word_delete(receipt).unwrap())
            .unwrap()
            .contains("256;45_")
    );
}

#[test]
fn shell_word_delete_preserves_foreground_and_enhanced_key_ownership() {
    for shell in ["bash", "zsh", "fish"] {
        let mut t = terminal();
        t.user_vars
            .insert("automexia_shell_name".into(), shell.into());
        assert_eq!(t.shell_word_delete_sequence(true), None);
        feed(&mut t, "\x1b]133;A;aid=1\x07> \x1b]133;B\x07");
        assert_eq!(t.shell_word_delete_sequence(true), Some("\x1bd"));
        assert_eq!(t.shell_word_delete_sequence(false), Some("\x17"));
        for mode in [
            Mode::ALT_SCREEN,
            Mode::VI,
            Mode::DISAMBIGUATE_ESC_CODES,
            Mode::REPORT_EVENT_TYPES,
        ] {
            t.mode.insert(mode);
            assert_eq!(t.shell_word_delete_sequence(true), None);
            t.mode.remove(mode);
        }
        feed(&mut t, "\x1b]133;C\x07");
        assert_eq!(t.shell_word_delete_sequence(true), None);
    }
}

#[test]
fn cmd_word_delete_rejects_hidden_remote_and_oversized_input() {
    let mut t = terminal();
    cmd_edit(&mut t, "\x1b[8mhidden\x1b[0m tail");
    assert!(t.cmd_word_delete_receipt().is_none());
    let mut t = terminal();
    cmd_edit(&mut t, "alpha beta");
    let receipt = t.cmd_word_delete_receipt().unwrap();
    t.apply_integration_scope("AMXSCOPE1|begin|66687aadf862bd776c8fc18b8e9f8e20089714856ee233b3902a591d0d5f2925|1|2|bash");
    assert!(t.accept_cmd_word_delete(receipt).is_none());
    let mut t = Crosswords::new(
        CrosswordsSize::new(100, 100),
        CursorShape::Block,
        VoidListener {},
        WindowId::from(0),
        0,
        256,
    );
    cmd_edit(&mut t, &"x".repeat(8200));
    feed(&mut t, "\x1b[H\x1b[2C");
    assert!(t.cmd_word_delete_receipt().is_none());
}

#[test]
fn quick_action_workflow_receipts_reject_replay_input_reset_and_alternate_screen() {
    for change in 0..5 {
        let mut t = terminal();
        completed(&mut t);
        let receipt = t.workflow_prompt().unwrap();
        match change {
            0 => {
                assert!(t.accept_workflow_submission(receipt));
            }
            1 => t.note_interactive_input(),
            2 => t.clear_screen_and_history(),
            3 => feed(&mut t, "\x1b[?1049h"),
            _ => feed(&mut t, "pending input"),
        }
        assert!(!t.accept_workflow_submission(receipt));
    }
}

#[test]
fn quick_action_workflow_receipts_wait_for_complete_empty_prompt_and_own_result() {
    let mut t = terminal();
    assert!(t.workflow_prompt().is_none());
    feed(&mut t, "\x1b]133;A;aid=1\x07> ");
    assert!(t.workflow_prompt().is_none());
    feed(&mut t, "\x1b]133;B\x07");
    let receipt = t.workflow_prompt().unwrap();
    assert!(t.accept_workflow_submission(receipt));
    feed(&mut t, "false\r\n\x1b]133;C\x07\x1b]133;D;1\x07");
    assert!(t.workflow_prompt().is_none());
    assert_eq!(t.workflow_completed(), Some((1, 1)));
    feed(&mut t, "\x1b]133;A;aid=2\x07> \x1b]133;B\x07");
    assert_eq!(t.workflow_prompt().unwrap().prompt, 2);
    assert_eq!(t.workflow_completed(), Some((1, 1)));
}
#[test]
fn quick_action_workflow_requires_identity_but_legacy_reinsert_does_not() {
    let mut t = terminal();
    run_command(&mut t, "echo hello", "hello\r\n", "");
    assert!(t.workflow_prompt().is_none());
    let handle = t.last_command_handle().unwrap();
    assert_eq!(t.command_text_for_reinsert(handle).unwrap(), "echo hello");
}
fn run_command(t: &mut Crosswords<VoidListener>, command: &str, output: &str, aid: &str) {
    feed(t, &format!("\x1b]133;A{aid}\x07> \x1b]133;B\x07{command}\r\n\x1b]133;C\x07{output}\x1b]133;D;0\x07\x1b]133;A\x07> \x1b]133;B\x07"));
}
#[test]
fn command_actions_read_live_integrated_grid_without_prompt_or_output_buffer() {
    let mut t = terminal();
    completed(&mut t);
    let handle = t.last_command_handle().unwrap();
    assert_eq!(t.command_text(handle).unwrap(), "echo hello");
    assert_eq!(t.command_output(handle).unwrap(), "hello");
    assert_eq!(t.command_text_for_reinsert(handle).unwrap(), "echo hello");
    assert!(t.selection.is_none());
}
#[test]
fn command_actions_reject_unmarked_output_and_cleared_generations() {
    let mut t = terminal();
    feed(&mut t, "prefix> echo hello\r\nhello\r\n");
    assert!(t.last_command_handle().is_none());
    completed(&mut t);
    let handle = t.last_command_handle().unwrap();
    t.clear_screen_and_history();
    assert!(t.command_output(handle).is_err());
}
#[test]
fn command_actions_preserve_unicode_and_soft_wraps_across_reflow() {
    for shell in ["bash", "zsh", "CMD", "PowerShell"] {
        for aid in ["", ";aid=5"] {
            let mut t = terminal();
            t.user_vars.insert("automexia_shell".into(), "1".into());
            t.user_vars
                .insert("automexia_shell_name".into(), shell.into());
            let command = "echo '\u{65e5}\u{672c}\u{8a9e} e\u{301} \u{1f680}' --long-option=abcdefghijklmnop";
            let output = "\u{65e5}\u{672c}\u{8a9e} e\u{301} \u{1f680} abcdefghijklmnopqrstuvwxyz0123456789\r\nsecond line\r\n";
            run_command(&mut t, command, output, aid);
            let handle = t.last_command_handle().unwrap();
            for width in [40, 12, 7, 23, 80] {
                t.resize(CrosswordsSize::new(width, 16));
                assert_eq!(
                    t.command_text(handle).unwrap(),
                    command,
                    "{shell} {aid} width={width}"
                );
                assert_eq!(
                    t.command_output(handle).unwrap(),
                    output.replace("\r", "").trim_end_matches('\n'),
                    "{shell} {aid} width={width}"
                );
            }
        }
    }
}
#[test]
fn command_actions_empty_output_missing_input_and_missing_next_prompt() {
    let mut t = terminal();
    run_command(&mut t, "true", "", ";aid=5");
    let h = t.last_command_handle().unwrap();
    assert_eq!(t.command_text(h).unwrap(), "true");
    assert_eq!(t.command_output(h).unwrap(), "");
    assert!(t.command_ranges(h).unwrap().output.is_none());
    let mut t = terminal();
    feed(
        &mut t,
        "\x1b]133;A;aid=5\x07> true\r\n\x1b]133;C\x07done\r\n\x1b]133;D;0\x07",
    );
    let h = t.last_command_handle().unwrap();
    assert_eq!(t.command_output(h), Err(E::Incomplete));
    feed(&mut t, "\x1b]133;A;aid=6\x07> \x1b]133;B\x07");
    assert_eq!(t.command_output(h).unwrap(), "done");
    assert_eq!(t.command_text(h), Err(E::Unavailable));
}
#[test]
fn command_actions_do_not_guess_fish_native_prompt_or_multiline_text() {
    let mut t = terminal();
    t.user_vars
        .insert("automexia_shell_name".into(), "fish".into());
    run_command(&mut t, "native> echo hello", "hello\r\n", ";aid=5");
    let h = t.last_command_handle().unwrap();
    assert_eq!(t.command_text(h), Err(E::Unavailable));
    assert_eq!(t.command_output(h).unwrap(), "hello");
    let mut t = terminal();
    run_command(
        &mut t,
        "echo 'one\r\ncontinuation> two'",
        "one\r\ntwo\r\n",
        ";aid=5",
    );
    assert_eq!(
        t.command_text(t.last_command_handle().unwrap()),
        Err(E::UnsafeInput)
    );
}
#[test]
fn command_actions_reject_hidden_and_unsafe_reinsertion_without_guessing() {
    for (command, output, hidden_command) in [
        ("echo \x1b[8mconcealed\x1b[0m", "ok\r\n", true),
        ("echo output", "\x1b[8mconcealed\x1b[0m\r\n", false),
    ] {
        let mut t = terminal();
        run_command(&mut t, command, output, ";aid=5");
        let h = t.last_command_handle().unwrap();
        assert_eq!(
            if hidden_command {
                t.command_text(h)
            } else {
                t.command_output(h)
            },
            Err(E::Hidden)
        );
    }
    for c in ['\u{2028}', '\u{2029}', '\u{202e}', '\u{2066}'] {
        let mut t = terminal();
        run_command(&mut t, &format!("echo a{c}b"), "ok\r\n", ";aid=5");
        assert_eq!(
            t.command_text_for_reinsert(t.last_command_handle().unwrap()),
            Err(E::UnsafeInput)
        );
    }
}
#[test]
fn command_actions_reinsert_requires_empty_input_including_continuation_rows() {
    for pending in [
        "pending",
        "pending\x1b[7D",
        "\r\nmore\x1b[1A\x1b[9G",
        "\x1b]133;C\x07",
    ] {
        let mut t = terminal();
        completed(&mut t);
        let h = t.last_command_handle().unwrap();
        feed(&mut t, pending);
        assert_eq!(t.command_text_for_reinsert(h), Err(E::Busy), "{pending:?}");
    }
}
#[test]
fn command_actions_scope_clear_reset_alt_screen_expire_handles_and_latest_choice() {
    for control in ["\x1b[2J", "\x1b[3J", "\x1bc", "\x1b[?1049h\x1b[?1049l"] {
        let mut t = terminal();
        completed(&mut t);
        let h = t.last_command_handle().unwrap();
        feed(&mut t, control);
        assert_eq!(t.command_output(h), Err(E::Stale), "{control:?}");
        assert!(t.last_command_handle().is_none(), "{control:?}");
    }
    let mut t = terminal();
    completed(&mut t);
    let h = t.last_command_handle().unwrap();
    t.apply_integration_scope("AMXSCOPE1|begin|66687aadf862bd776c8fc18b8e9f8e20089714856ee233b3902a591d0d5f2925|1|2|bash");
    assert_eq!(t.command_output(h), Err(E::Stale));
    assert!(t.last_command_handle().is_none());
}
#[test]
fn command_actions_expired_history_is_not_silently_truncated() {
    let mut t = terminal();
    run_command(&mut t, "echo many", &"line\r\n".repeat(300), ";aid=5");
    assert!(t.command_output(t.last_command_handle().unwrap()).is_err());
}
#[test]
fn command_actions_size_limit_and_fragmented_markers_are_bounded() {
    let mut t = Crosswords::new(
        CrosswordsSize::new(100, 8),
        CursorShape::Block,
        VoidListener {},
        WindowId::from(0),
        0,
        1024,
    );
    run_command(&mut t, &"x".repeat(16 * 1024 + 1), "ok\r\n", ";aid=5");
    assert_eq!(
        t.command_text(t.last_command_handle().unwrap()),
        Err(E::Limit)
    );
    let mut t = terminal();
    let mut parser = Processor::default();
    for byte in b"\x1b]133;A;aid=1\x07> \x1b]133;B\x07echo ok\r\n\x1b]133;C\x07ok\r\n\x1b]133;D;0\x07\x1b]133;A;aid=2\x07> \x1b]133;B\x07" { parser.advance(&mut t, &[*byte]); }
    let h = t.last_command_handle().unwrap();
    assert_eq!(t.command_text(h).unwrap(), "echo ok");
    assert_eq!(t.command_output(h).unwrap(), "ok");
}

#[test]
fn command_actions_legacy_multiline_and_absent_execution_marker_do_not_reinsert_partial_text(
) {
    let mut t = terminal();
    run_command(&mut t, "echo 'first\r\nsecondary> second'", "done\r\n", "");
    assert!(t.command_text(t.last_command_handle().unwrap()).is_err());
    let mut t = terminal();
    feed(&mut t, "\x1b]133;A;aid=1\x07> \x1b]133;B\x07echo data\r\noutput\r\n\x1b]133;D;0\x07\x1b]133;A;aid=2\x07> \x1b]133;B\x07");
    assert!(t.command_text(t.last_command_handle().unwrap()).is_err());
}

#[test]
fn command_actions_ranges_and_copy_follow_live_history_without_retargeting() {
    let mut t = terminal();
    completed(&mut t);
    let h = t.last_command_handle().unwrap();
    let ranges = t.command_ranges(h).unwrap();
    assert_eq!(ranges.prompt, Pos::new(Line(0), Column(0)));
    assert_eq!(
        ranges.output,
        Some((Pos::new(Line(1), Column(0)), Pos::new(Line(1), Column(39))))
    );
    // A real cursor-addressed output repaint is read at activation time.
    feed(&mut t, "\x1b[2;1HHELLO\x1b[3;9Hsecond\r\n\x1b]133;C\x07second-output\r\n\x1b]133;D;0\x07\x1b]133;A;aid=3\x07> \x1b]133;B\x07");
    let next = t.last_command_handle().unwrap();
    assert_ne!(h, next);
    assert_eq!(t.command_output(h).unwrap(), "HELLO");
    assert_eq!(t.command_text(h).unwrap(), "echo hello");
    assert_eq!(t.command_output(next).unwrap(), "second-output");
    let (start, end) = t.command_ranges(h).unwrap().output.unwrap();
    let mut selection = crate::selection::Selection::new(
        crate::selection::SelectionType::Simple,
        start,
        Side::Left,
    );
    selection.update(end, Side::Right);
    t.selection = Some(selection);
    assert_eq!(t.selection_to_string().unwrap(), "HELLO");
}

#[test]
fn command_actions_result_id_reuse_retires_old_generation() {
    let mut t = terminal();
    completed(&mut t);
    let h = t.last_command_handle().unwrap();
    t.semantic_command_result_sequence = u64::MAX;
    feed(&mut t, "echo second\r\n\x1b]133;C\x07second\r\n\x1b]133;D;0\x07\x1b]133;A;aid=3\x07> \x1b]133;B\x07");
    assert_eq!(t.command_output(h), Err(E::Stale));
    assert_eq!(
        t.command_output(t.last_command_handle().unwrap()).unwrap(),
        "second"
    );
}
