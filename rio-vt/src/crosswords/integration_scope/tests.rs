use super::*;
use crate::event::VoidListener;
use crate::performer::handler::Processor;
use base64::{engine::general_purpose::STANDARD, Engine as _};

fn terminal() -> Crosswords<VoidListener> {
    Crosswords::new(
        CrosswordsSize::new(80, 12),
        CursorShape::Block,
        VoidListener {},
        WindowId::from(0),
        0,
        128,
    )
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn wire(name: &str, value: &str) -> Vec<u8> {
    format!("\x1b]1337;SetUserVar={name}={}\x07", STANDARD.encode(value)).into_bytes()
}

fn begin(secret: u8, generation: u64) -> Vec<u8> {
    begin_shell(secret, generation, "bash")
}

fn begin_shell(secret: u8, generation: u64, shell: &str) -> Vec<u8> {
    wire(
        "terminal_scope_v1",
        &format!(
            "AMXSCOPE1|begin|{}|1|{generation}|{shell}",
            hex(&Sha256::digest([secret; 32]))
        ),
    )
}

fn end(secret: u8) -> Vec<u8> {
    wire(
        "terminal_scope_v1",
        &format!("AMXSCOPE1|end|{}", hex(&[secret; 32])),
    )
}

#[test]
fn local_nested_shell_counters_have_distinct_prompt_identities() {
    for fragment in [1, 7, 4096] {
        let mut t = terminal();
        let mut p = Processor::default();
        let mut ids = Vec::new();
        // Parent, new guest, another guest prompt, return to parent, and a
        // second guest with its counter restarted. No SSH scope is involved.
        for wire_id in [1, 1, 2, 2, 1] {
            let stream = format!("\x1b]133;A;aid={wire_id}\x07 \r\n\x1b]133;P;k=c;aid={wire_id}\x07> \x1b]133;B\x07");
            for bytes in stream.as_bytes().chunks(fragment) {
                p.advance(&mut t, bytes);
            }
            let id = t.grid[t.cursor().pos.row].semantic_prompt_id.unwrap();
            assert!(
                !ids.contains(&id),
                "a nested shell reused a historical identity"
            );
            ids.push(id);
            assert_eq!(
                t.grid[t.cursor().pos.row - 1i32].semantic_prompt_id,
                Some(id)
            );
            // A repeated A while editing is a redraw, not a new command.
            for bytes in stream.as_bytes().chunks(fragment) {
                p.advance(&mut t, bytes);
            }
            assert_eq!(t.grid[t.cursor().pos.row].semantic_prompt_id, Some(id));
            p.advance(&mut t, b"run\r\n\x1b]133;C\x07done\r\n\x1b]133;D;0\x07");
        }
    }
}

#[test]
fn discovery_revocation_barrier_survives_replays_and_nested_scopes() {
    let mut t = terminal();
    let mut p = Processor::default();
    p.advance(&mut t, &wire("automexia_ssh_revision", ""));
    assert_eq!(t.integration_scope_discovery_barrier(), None);
    p.advance(&mut t, &begin(1, 1));
    p.advance(&mut t, &wire("automexia_ssh_revision", "AMXSSHREV1|1|1|1"));
    assert!(t.integration_scope_discovery_barrier().is_some());
    assert_eq!(t.integration_scope_discovery_revision(), Some(1));
    p.advance(&mut t, &wire("automexia_ssh_revision", ""));
    let revoked = t.integration_scope_discovery_barrier().unwrap();
    p.advance(&mut t, &wire("automexia_ssh_context_v2", "late"));
    p.advance(&mut t, &wire("automexia_ssh_revision", "AMXSSHREV1|1|1|1"));
    let resumed = t.integration_scope_discovery_barrier().unwrap();
    assert!(resumed > revoked);
    p.advance(&mut t, &wire("automexia_ssh_context_v2", "fresh"));
    p.advance(&mut t, &wire("automexia_ssh_revision", "AMXSSHREV1|1|1|1"));
    assert_eq!(t.integration_scope_discovery_barrier(), Some(resumed));
    p.advance(&mut t, &begin(2, 2));
    assert_eq!(t.integration_scope_discovery_barrier(), None);
    p.advance(&mut t, &wire("automexia_ssh_revision", ""));
    assert!(t.integration_scope_discovery_barrier().unwrap() > resumed);
    p.advance(&mut t, &end(2));
    assert_eq!(t.integration_scope_discovery_barrier(), Some(resumed));
    p.advance(&mut t, &end(1));
    assert_eq!(t.integration_scope_discovery_barrier(), None);
}

#[test]
fn discovery_revision_maximum_is_scope_owned_and_invalid_writes_revoke() {
    let mut t = terminal();
    let mut p = Processor::default();
    p.advance(&mut t, &begin(1, 1));
    p.advance(&mut t, &wire("automexia_ssh_revision", "AMXSSHREV1|1|1|2"));
    assert_eq!(t.integration_scope_discovery_revision(), Some(2));
    for invalid in ["", "malformed", "AMXSSHREV1|1|9|8", "AMXSSHREV1|1|1|1"] {
        p.advance(&mut t, &wire("automexia_ssh_revision", invalid));
        assert_eq!(t.integration_scope_discovery_revision(), None);
        let revoked = t.integration_scope_discovery_barrier().unwrap();
        p.advance(&mut t, &wire("automexia_ssh_revision", "AMXSSHREV1|1|1|2"));
        assert_eq!(t.integration_scope_discovery_revision(), Some(2));
        assert!(t.integration_scope_discovery_barrier().unwrap() > revoked);
    }
    p.advance(&mut t, &begin(2, 2));
    p.advance(&mut t, &wire("automexia_ssh_revision", "AMXSSHREV1|1|2|1"));
    assert_eq!(t.integration_scope_discovery_revision(), Some(1));
    p.advance(&mut t, &end(2));
    assert_eq!(t.integration_scope_discovery_revision(), Some(2));
    p.advance(&mut t, &wire("automexia_ssh_revision", "AMXSSHREV1|1|1|1"));
    assert_eq!(t.integration_scope_discovery_revision(), None);
    p.advance(&mut t, &wire("automexia_ssh_revision", "AMXSSHREV1|1|1|3"));
    assert_eq!(t.integration_scope_discovery_revision(), Some(3));
}

#[test]
fn interrupted_metadata_prefix_cannot_consume_scope_return_or_local_text() {
    for length in [1, 1024, 6144] {
        let mut t = terminal();
        let mut p = Processor::default();
        p.advance(&mut t, &begin(1, 1));
        // Retirement can interrupt a blocked metadata writer only after its
        // remote shell stops. The wrapper then emits its authentic scope end.
        let mut partial = b"\x1b]1337;SetUserVar=automexia_ssh_context_v2=".to_vec();
        partial.extend(std::iter::repeat_n(b'A', length));
        p.advance(&mut t, &partial);
        p.advance(&mut t, &end(1));
        p.advance(&mut t, b"LOCAL-OUTPUT");
        assert!(!t.integration_scope_active(), "partial length {length}");
        let visible: String = t.grid[Line(0)].inner.iter().map(|cell| cell.c()).collect();
        assert!(visible.contains("LOCAL-OUTPUT"), "partial length {length}");
    }
}

#[test]
fn native_unknown_scope_isolates_metadata_without_guessing_input_shell() {
    let mut t = terminal();
    let mut p = Processor::default();
    t.current_directory = Some("/local/work".into());
    p.advance(&mut t, &wire("automexia_shell_name", "bash"));
    p.advance(&mut t, &begin_shell(1, 1, "unknown"));
    assert_eq!(
        t.integration_scope().map(|scope| scope.shell.as_str()),
        Some("unknown")
    );
    p.advance(&mut t, b"\x1b]7;file://localhost/remote/work\x07");
    p.advance(&mut t, &wire("automexia_shell_name", "bash"));
    p.advance(&mut t, &wire("automexia_shell", "1"));
    p.advance(&mut t, b"\x1b]133;A;aid=1\x07remote> \x1b]133;B\x07");
    assert_eq!(t.current_directory, None);
    assert!(t.grid[t.grid.cursor.pos.row].semantic_input.is_none());
    p.advance(&mut t, &end(1));
    assert!(!t.integration_scope_active());
    assert_eq!(t.current_directory, Some("/local/work".into()));
    assert!(!t.user_vars.contains_key("automexia_shell"));
    assert_eq!(
        t.user_vars.get("automexia_shell_name").map(String::as_str),
        Some("bash")
    );
}

#[test]
fn remote_scopes_cannot_authorize_cmd_host_clear() {
    for shell in ["unknown", "bash", "zsh", "fish", "powershell", "pwsh"] {
        let mut t = terminal();
        let mut p = Processor::default();
        p.advance(&mut t, &wire("automexia_prompt_active", "1"));
        p.advance(&mut t, b"\x1b]133;A;aid=1\x07local> \x1b]133;B\x07");
        assert!(t.host_clear_input_prompt_active());
        p.advance(&mut t, &begin_shell(1, 1, shell));
        p.advance(&mut t, &wire("automexia_prompt_active", "1"));
        p.advance(&mut t, b"\x1b]133;A;aid=1\x07remote> \x1b]133;B\x07");
        assert!(!t.host_clear_input_prompt_active(), "remote shell {shell}");
        p.advance(&mut t, &end(1));
        p.advance(&mut t, b"\r\n\x1b]133;A;aid=2\x07local> \x1b]133;B\x07");
        assert!(t.host_clear_input_prompt_active());
    }
}

#[test]
fn scope_rejected_metadata_wakes_display_without_output_or_resize() {
    for frame in [
        b"\x1b]1337;SetUserVar=automexia_ssh_user=!!!\x07".to_vec(),
        wire(
            "automexia_ssh_user",
            &"x".repeat(MAX_USER_VAR_VALUE_BYTES + 1),
        ),
    ] {
        let mut t = terminal();
        let mut p = Processor::default();
        p.advance(&mut t, &begin(1, 1));
        p.advance(
            &mut t,
            &wire("automexia_ssh_user", "AMXSSHUSER1|1|1|remote-user"),
        );
        p.advance(&mut t, b"\x1b]133;A;aid=1\x07\r\n\x1b]133;B\x07");
        let cursor = t.grid.cursor.pos;
        t.reset_damage();
        assert!(t.peek_damage_event().is_none());
        p.advance(&mut t, &frame);
        assert!(t.last_user_var_rejection().is_some());
        assert!(
            t.peek_damage_event().is_some(),
            "rejected replacement must wake metadata admission"
        );
        assert_eq!(t.grid.cursor.pos, cursor);
        assert!(t.integration_scope_prompt_active());
    }
}

#[test]
fn scope_restores_metadata_and_chronology_after_coalesced_batch() {
    let mut t = terminal();
    let mut p = Processor::default();
    p.advance(&mut t, &wire("automexia_env_home", "/local/home"));
    t.current_directory = Some("/local/work".into());
    let stamp = t.user_var_write_stamp("automexia_env_home");
    let mut bytes = begin(1, 1);
    bytes.extend(wire("automexia_env_home", "/remote/home"));
    bytes.extend(b"\x1b]7;file://server/remote/work\x07");
    bytes.extend(end(1));
    p.advance(&mut t, &bytes);
    assert!(!t.integration_scope_active());
    assert_eq!(
        t.user_vars.get("automexia_env_home").map(String::as_str),
        Some("/local/home")
    );
    assert_eq!(t.current_directory, Some("/local/work".into()));
    assert_eq!(t.user_var_write_stamp("automexia_env_home"), stamp);
    assert_eq!(t.integration_scope_revision(), 2);
    assert!(!t.user_vars.contains_key("terminal_scope_v1"));
    let clock = t.user_var_clock;
    p.advance(&mut t, &wire("after", "local"));
    assert!(t.user_var_clock > clock);
}

#[test]
fn scope_forged_end_and_reset_cannot_restore_local_authority() {
    let mut t = terminal();
    let mut p = Processor::default();
    p.advance(&mut t, &begin(1, 1));
    p.advance(&mut t, &end(2));
    p.advance(&mut t, b"\x1bc\x1b]7;file://server/remote/work\x07");
    assert!(t.integration_scope_active());
    assert_eq!(t.current_directory, None);
    p.advance(&mut t, &end(1));
    assert!(!t.integration_scope_active());
    p.advance(&mut t, &begin(2, 2));
    p.advance(&mut t, &end(1));
    assert_eq!(
        t.integration_scope().unwrap().generation,
        2,
        "ended scope cannot be replayed"
    );
}

#[test]
fn scope_nested_return_and_outer_cleanup_are_ordered() {
    let mut t = terminal();
    let mut p = Processor::default();
    p.advance(&mut t, &begin(1, 1));
    p.advance(&mut t, &wire("automexia_ssh_cwd", "outer"));
    p.advance(&mut t, &begin(2, 2));
    p.advance(&mut t, &wire("automexia_ssh_cwd", "inner"));
    p.advance(&mut t, &end(2));
    assert_eq!(
        t.user_vars.get("automexia_ssh_cwd").map(String::as_str),
        Some("outer")
    );
    assert_eq!(t.integration_scope().unwrap().generation, 1);
    for generation in 2..=10 {
        p.advance(&mut t, &begin(generation as u8, generation));
    }
    assert_eq!(t.integration_scopes.frames.len(), MAX_DEPTH);
    assert!(t.integration_scope_active());
    assert!(t.integration_scope().is_none());
    p.advance(&mut t, &end(1));
    assert!(!t.integration_scope_active());
    assert!(t.integration_scopes.frames.is_empty());
}

#[test]
fn scope_malformed_begin_stays_quarantined_without_known_outer_end() {
    let mut t = terminal();
    let mut p = Processor::default();
    for invalid in ["AMXSCOPE2|begin", "AMXSCOPE1|begin|bad|0|2|bash"] {
        p.advance(&mut t, &wire("terminal_scope_v1", invalid));
        p.advance(&mut t, &begin(1, 1));
        p.advance(&mut t, &end(1));
        assert!(t.integration_scope_active());
        assert!(t.integration_scope().is_none());
    }
}

#[test]
fn scope_controls_survive_full_metadata_dictionary() {
    let mut t = terminal();
    let mut p = Processor::default();
    for index in 0..MAX_USER_VARS {
        p.advance(&mut t, &wire(&format!("k{index}"), "v"));
    }
    p.advance(&mut t, &begin(1, 1));
    assert!(t.integration_scope_active());
    assert!(t.user_vars.is_empty());
    for index in 0..MAX_USER_VARS {
        p.advance(&mut t, &wire(&format!("r{index}"), "v"));
    }
    p.advance(&mut t, &end(1));
    assert_eq!(t.user_vars.len(), MAX_USER_VARS);
    assert!(t.user_vars.keys().all(|name| name.starts_with('k')));
}

#[test]
fn scope_prompt_and_command_status_do_not_consume_outer_command() {
    let mut t = terminal();
    let mut p = Processor::default();
    p.advance(
        &mut t,
        b"\x1b]133;A;aid=501\x07local> \x1b]133;B\x07ssh\r\n\x1b]133;C\x07",
    );
    let outer = t.semantic_command_started;
    assert!(outer.is_some());
    p.advance(&mut t, &begin(1, 1));
    p.advance(
        &mut t,
        b"\x1b]133;A;aid=1\x07\r\n\x1b]133;P;k=c;aid=1\x07remote> \x1b]133;B\x07",
    );
    assert!(t.integration_scope_prompt_active());
    assert_eq!(
        t.grid[t.grid.cursor.pos.row].semantic_input.unwrap().shell,
        crate::crosswords::grid::row::PromptInputShell::Posix
    );
    p.advance(
        &mut t,
        b"false\r\n\x1b]133;C\x07failure\r\n\x1b]133;D;1\x07",
    );
    assert!(!t.integration_scope_prompt_active());
    assert!(t.semantic_command_started.is_none());
    p.advance(&mut t, &end(1));
    assert_eq!(t.semantic_command_started, outer);
    assert_eq!(t.semantic_prompt_id, Some(501));
}

#[test]
fn scope_is_pane_local_and_unfinished_child_fails_closed() {
    let mut first = terminal();
    let mut second = terminal();
    let mut p = Processor::default();
    p.advance(&mut first, &begin(1, 1));
    p.advance(&mut second, &end(1));
    assert!(first.integration_scope_active());
    assert!(!second.integration_scope_active());
    p.advance(&mut first, b"\x1b]7;file://server/remote/work\x07");
    assert_eq!(first.current_directory, None);
}

#[test]
fn scope_reused_prompt_ids_cannot_overwrite_other_shell_results() {
    let mut t = terminal();
    let mut p = Processor::default();
    p.advance(
        &mut t,
        b"\x1b]133;A;aid=1\x07local> \x1b]133;B\x07ssh\r\n\x1b]133;C\x07",
    );
    p.advance(&mut t, &begin(1, 1));
    p.advance(&mut t, b"\x1b]133;A;aid=1\x07\r\n\x1b]133;P;k=c;aid=1\x07remote> \x1b]133;B\x07false\r\n\x1b]133;C\x07failure\r\n\x1b]133;D;9\x07");
    p.advance(&mut t, &end(1));
    p.advance(
        &mut t,
        b"\x1b]133;D;0\x07\x1b]133;A;aid=2\x07local> \x1b]133;B\x07",
    );
    let prompts: Vec<_> = (0..12)
        .map(Line)
        .filter(|line| {
            t.grid[*line].semantic_prompt
                == crate::crosswords::grid::row::SemanticPrompt::Prompt
        })
        .map(|line| {
            (
                t.grid[line].semantic_prompt_id,
                t.grid[line]
                    .semantic_command_result
                    .map(|result| result.exit_code),
            )
        })
        .collect();
    assert_eq!(prompts.len(), 3);
    assert_eq!(prompts[0].1, Some(Some(0)));
    assert_eq!(prompts[1].1, Some(Some(9)));
    assert_ne!(prompts[0].0, prompts[1].0);
    assert_ne!(prompts[1].0, prompts[2].0);
}

#[test]
fn scope_actual_bash_ctrl_l_keeps_context_row_above_the_native_editor() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../tests/fixtures/shell/ssh-bash-ctrl-l-v1.json"
    ))
    .unwrap();
    for chunk in [1, 7, usize::MAX] {
        let mut t = Crosswords::new(
            CrosswordsSize::new(80, 24),
            CursorShape::Block,
            VoidListener {},
            WindowId::from(0),
            0,
            128,
        );
        let mut p = Processor::default();
        for name in ["scope_begin", "initial", "typed", "ctrl_l"] {
            let bytes = fixture[name].as_str().unwrap().as_bytes();
            for bytes in bytes.chunks(chunk.min(bytes.len())) {
                p.advance(&mut t, bytes);
                t.finish_pty_batch();
            }
        }
        assert!(t.integration_scope_prompt_active());
        let start = -(t.grid.display_offset() as i32);
        assert_eq!(
            t.grid[Line(start)].semantic_prompt,
            crate::crosswords::grid::row::SemanticPrompt::Prompt,
            "context row must be first in the visible viewport, chunk={chunk}"
        );
        let native: String = t.grid[Line(start + 1)]
            .inner
            .iter()
            .map(|cell| cell.c())
            .collect();
        assert!(
            native.starts_with("AMX_AUDIT_PROMPT> printf demo"),
            "{native:?}"
        );
        assert_eq!(
            (start..start + 24)
                .filter(|row| t.grid[Line(*row)].semantic_prompt
                    == crate::crosswords::grid::row::SemanticPrompt::Prompt)
                .count(),
            1
        );
    }
}
