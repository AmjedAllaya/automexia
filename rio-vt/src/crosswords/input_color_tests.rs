use super::*;
use crate::crosswords::grid::row::PromptInputShell;
use crate::event::VoidListener;
use crate::performer::handler::Processor;

fn terminal(shell: &str, columns: usize) -> Crosswords<VoidListener> {
    let mut terminal = Crosswords::new(
        CrosswordsSize::new(columns, 16),
        CursorShape::Block,
        VoidListener {},
        crate::event::WindowId::from(0),
        0,
        128,
    );
    terminal
        .user_vars
        .insert("automexia_shell".into(), "1".into());
    terminal
        .user_vars
        .insert("automexia_shell_name".into(), shell.into());
    terminal
}

#[test]
fn input_color_boundary_requires_owned_prompt_and_supported_ready_shell() {
    for (shell, expected) in [
        ("CMD", Some(PromptInputShell::Cmd)),
        ("bash", Some(PromptInputShell::Posix)),
        ("zsh", Some(PromptInputShell::Posix)),
        ("fish", None),
        ("PowerShell", None),
        ("unknown", None),
    ] {
        let mut term = terminal(shell, 80);
        let mut parser = Processor::default();
        parser.advance(&mut term, b"output> \x1b]133;B\x07docker -a\r\n");
        assert!(term.grid[Line(0)].semantic_input.is_none());
        parser.advance(&mut term, b"\x1b]133;A\x07prompt> \x1b]133;B\x07docker -a");
        let input = term.grid[Line(1)].semantic_input;
        assert_eq!(
            input
                .map(|input| input.shell)
                .filter(|shell| *shell != PromptInputShell::Native),
            expected,
            "{shell}"
        );
        if let Some(input) = input {
            assert_eq!(input.column, 8);
            assert!(!input.continuation);
        }
        parser.advance(
            &mut term,
            b"\r\n\x1b]133;C\x07docker -a output\r\n\x1b]133;D;0\x07",
        );
        assert!(term.grid[Line(2)].semantic_input.is_none());
        term.user_vars
            .insert("automexia_env_pending".into(), "1".into());
        parser.advance(&mut term, b"\x1b]133;A\x07> \x1b]133;B\x07docker");
        assert_eq!(
            term.grid[Line(3)].semantic_input.map(|input| input.shell),
            if shell == "fish" {
                None
            } else {
                Some(PromptInputShell::Native)
            }
        );
        term.user_vars.remove("automexia_env_pending");
        term.user_vars.remove("automexia_shell");
        parser.advance(&mut term, b"\r\n\x1b]133;A\x07> \x1b]133;B\x07docker");
        assert_eq!(
            term.grid[Line(4)].semantic_input.map(|input| input.shell),
            if shell == "fish" {
                None
            } else {
                Some(PromptInputShell::Native)
            }
        );
    }
}

#[test]
fn input_color_metadata_survives_wrapping_reflow_and_snapshot_without_changing_text() {
    for shell in ["CMD", "bash", "zsh"] {
        for owned_id in ["", ";aid=61"] {
            let mut term = terminal(shell, 80);
            let mut parser = Processor::default();
            let text = "docker ps --format '日本語' -a";
            parser.advance(
                &mut term,
                format!("\x1b]133;A{owned_id}\x07> \x1b]133;B\x07{text}").as_bytes(),
            );
            // Test retained rows; live prompt repair is exercised separately.
            parser.advance(&mut term, b"\r\n\x1b]133;C\x07");
            for width in [12, 7, 23, 80] {
                term.resize(CrosswordsSize::new(width, 16));
                let mut recovered = String::new();
                let mut starts = 0;
                for line in term.grid.topmost_line().0..16 {
                    let row = &term.grid[Line(line)];
                    if let Some(input) = row.semantic_input {
                        if input.column >= row.len() {
                            continue;
                        }
                        if !input.continuation {
                            starts += 1;
                            assert_eq!(input.column, 2);
                        }
                        for cell in row.inner.iter().skip(input.column) {
                            if !matches!(
                                cell.wide(),
                                crate::crosswords::square::Wide::Spacer
                                    | crate::crosswords::square::Wide::LeadingSpacer
                            ) {
                                recovered.push(if cell.c() == '\0' {
                                    ' '
                                } else {
                                    cell.c()
                                });
                            }
                        }
                    }
                }
                assert_eq!(starts, 1, "{shell} {owned_id} {width}");
                assert_eq!(recovered.trim_end(), text, "{shell} {owned_id} {width}");
            }
            let mut rows = Vec::new();
            term.snapshot_visible(
                &crate::event::TerminalDamage::Full,
                80,
                &mut rows,
                &mut Vec::new(),
                &mut Default::default(),
            );
            assert_eq!(rows[0].semantic_input, term.grid[Line(0)].semantic_input);
            parser.advance(&mut term, b"\x1b[1;1Hnew output");
            assert!(
                term.grid[Line(0)].semantic_input.is_none(),
                "overwritten rows cannot keep input ownership"
            );
        }
    }
}

#[test]
fn input_color_shell_changes_are_row_local_and_cmd_output_is_not_input() {
    let mut host = terminal("CMD", 10);
    let other = terminal("bash", 10);
    let mut parser = Processor::default();
    parser.advance(
        &mut host,
        b"\x1b]133;A\x07> \x1b]133;B\x07docker ps -a\r\nplain\r\n",
    );
    assert_eq!(
        host.grid[Line(0)].semantic_input.unwrap().shell,
        PromptInputShell::Cmd
    );
    assert!(host.grid[Line(1)].semantic_input.unwrap().continuation);
    assert!(host.grid[Line(2)].semantic_input.is_none());
    host.user_vars
        .insert("automexia_shell_name".into(), "bash".into());
    parser.advance(&mut host, b"\x1b]133;A;aid=2\x07> \x1b]133;B\x07ls -a");
    assert_eq!(
        host.grid[Line(3)].semantic_input.unwrap().shell,
        PromptInputShell::Posix
    );
    assert_eq!(
        host.grid[Line(0)].semantic_input.unwrap().shell,
        PromptInputShell::Cmd
    );
    assert!(other.grid[Line(0)].semantic_input.is_none());
    parser.advance(
        &mut host,
        b"\x1b[?1049h\x1b]133;A\x07> \x1b]133;B\x07docker",
    );
    assert!(host.grid[Line(0)].semantic_input.is_none());
}

#[test]
fn input_color_boundary_survives_active_context_repair_and_row_recycling() {
    let mut term = terminal("bash", 40);
    let mut parser = Processor::default();
    parser.advance(&mut term, b"\x1b]133;A;aid=7\x07 \r\n\x1b]133;P;k=c;aid=7\x07/srv/demo\r\n\x1b]133;P;k=c;aid=7\x07> \x1b]133;B\x07docker -a");
    let input = term.grid[Line(2)].semantic_input;
    assert!(input.is_some());
    // Lose only terminal-owned context, keeping the live shell editor row.
    term.grid[Line(0)].clear_semantic_metadata();
    term.grid[Line(1)].clear_semantic_metadata();
    term.repair_active_prompt_after_resize_if_missing();
    assert_eq!(term.grid[term.cursor().pos.row].semantic_input, input);
    let mut copy = crate::crosswords::grid::row::Row::new(40);
    copy.copy_from(&term.grid[term.cursor().pos.row]);
    assert_eq!(copy.semantic_input, input);
    copy.clear_semantic_metadata();
    assert!(copy.semantic_input.is_none());
    parser.advance(&mut term, b"\x1bc");
    assert!(term.grid[Line(0)].semantic_input.is_none());
}
