use super::session_metadata::MetadataReadiness;
use super::sync_session_metadata;
use crate::context::renderable::{Cursor, RenderableContent};
use base64::Engine;
use rio_backend::crosswords::{Crosswords, CrosswordsSize};
use rio_backend::event::{VoidListener, WindowId};
use rio_backend::performer::handler::Processor;

fn new_terminal() -> Crosswords<VoidListener> {
    Crosswords::new(
        CrosswordsSize::new(96, 10),
        rio_backend::ansi::CursorShape::Block,
        VoidListener {},
        WindowId::from(0),
        0,
        128,
    )
}

fn write(processor: &mut Processor, terminal: &mut Crosswords<VoidListener>, key: &str, value: &str) {
    let encoded = base64::engine::general_purpose::STANDARD.encode(value);
    processor.advance(terminal, format!("\x1b]1337;SetUserVar={key}={encoded}\x07").as_bytes());
}

fn fields(processor: &mut Processor, terminal: &mut Crosswords<VoidListener>, values: &[(&str, &str)]) {
    for &(key, value) in values {
        write(processor, terminal, key, value);
    }
}

fn frame(processor: &mut Processor, terminal: &mut Crosswords<VoidListener>, shell: &str, values: &[(&str, &str)]) {
    write(processor, terminal, "automexia_env_pending", "1");
    fields(processor, terminal, &[("automexia_shell", "1"), ("automexia_shell_name", shell)]);
    fields(processor, terminal, values);
    write(processor, terminal, "automexia_env_pending", "0");
}

fn base() -> [(&'static str, &'static str); 2] {
    [("automexia_env_HOME", "/fixture/home"), ("automexia_env_KUBECONFIG", "")]
}

fn new_content() -> RenderableContent {
    RenderableContent::new(Cursor::default())
}

#[test]
fn metadata_freshness_equal_pair_replaces_five_key_frame_without_inheriting_candidates() {
    let mut terminal = new_terminal();
    let mut processor = Processor::default();
    let mut content = new_content();
    let five = [
        base()[0], base()[1],
        ("automexia_env_HOMEDRIVE", "D:"),
        ("automexia_env_HOMEPATH", "/fixture/drive"),
        ("automexia_env_USERPROFILE", "C:/fixture/profile"),
    ];
    frame(&mut processor, &mut terminal, "PowerShell", &five);
    sync_session_metadata(&mut content, &terminal);
    frame(&mut processor, &mut terminal, "PowerShell", &base());
    sync_session_metadata(&mut content, &terminal);
    assert_eq!(content.session_metadata.readiness(), MetadataReadiness::Complete);
    assert_eq!(content.shell_environment.len(), 2);
    assert_eq!(content.shell_environment["HOME"], "/fixture/home");
    assert_eq!(content.shell_environment["KUBECONFIG"], "");
}

#[test]
fn metadata_freshness_coalesced_frames_clear_omitted_optional_identity() {
    let mut terminal = new_terminal();
    let mut processor = Processor::default();
    let mut content = new_content();
    let extended = [base()[0], base()[1], ("automexia_distro", "Fixture-Distro"),
        ("automexia_os_version", "1"), ("automexia_shell_user", "alice"), ("automexia_shell_path", "/fixture/shell")];
    frame(&mut processor, &mut terminal, "bash", &extended);
    // No paint between the two complete frames, and the required values are equal.
    frame(&mut processor, &mut terminal, "bash", &base());
    sync_session_metadata(&mut content, &terminal);
    assert_eq!(content.session_metadata.readiness(), MetadataReadiness::Complete);
    assert_eq!(content.shell_name.as_deref(), Some("bash"));
    assert!(content.shell_distro.is_none());
    assert!(content.shell_os_version.is_none());
    assert!(content.shell_user.is_none());
    assert!(content.shell_path.is_none());
}

#[test]
fn metadata_freshness_bytewise_pending_retains_one_complete_snapshot() {
    let mut terminal = new_terminal();
    let mut processor = Processor::default();
    let mut content = new_content();
    frame(&mut processor, &mut terminal, "PowerShell", &base());
    sync_session_metadata(&mut content, &terminal);
    let previous = content.shell_environment.clone();
    write(&mut processor, &mut terminal, "automexia_env_pending", "1");
    let encoded = base64::engine::general_purpose::STANDARD.encode("bash");
    let partial = format!("\x1b]1337;SetUserVar=automexia_shell_name={encoded}\x07");
    for byte in partial.bytes() {
        processor.advance(&mut terminal, &[byte]);
        sync_session_metadata(&mut content, &terminal);
        assert_eq!(content.session_metadata.readiness(), MetadataReadiness::Pending);
        assert_eq!(content.shell_name.as_deref(), Some("PowerShell"));
        assert_eq!(content.shell_environment, previous);
    }
    fields(&mut processor, &mut terminal, &[("automexia_shell", "1"), base()[0], base()[1]]);
    write(&mut processor, &mut terminal, "automexia_env_pending", "0");
    sync_session_metadata(&mut content, &terminal);
    assert_eq!(content.session_metadata.readiness(), MetadataReadiness::Complete);
    assert_eq!(content.shell_name.as_deref(), Some("bash"));
    assert_eq!(terminal.grid.cursor.pos.row.0, 0);
    assert_eq!(terminal.grid.cursor.pos.col.0, 0);
}

#[test]
fn metadata_freshness_partial_base_commit_is_unavailable_without_mixing_values() {
    for included in ["automexia_env_HOME", "automexia_env_KUBECONFIG"] {
        let mut terminal = new_terminal();
        let mut processor = Processor::default();
        let mut content = new_content();
        frame(&mut processor, &mut terminal, "bash", &base());
        sync_session_metadata(&mut content, &terminal);
        let previous = content.shell_environment.clone();
        frame(&mut processor, &mut terminal, "zsh", &[(included, "/fixture/new")]);
        sync_session_metadata(&mut content, &terminal);
        assert_eq!(content.session_metadata.readiness(), MetadataReadiness::Unavailable);
        assert_eq!(content.shell_name.as_deref(), Some("bash"));
        assert_eq!(content.shell_environment, previous);
    }
}

#[test]
fn metadata_freshness_partial_windows_trio_is_unavailable_on_every_platform() {
    for extra in [
        vec![("automexia_env_HOMEDRIVE", "D:")],
        vec![("automexia_env_HOMEDRIVE", "D:"), ("automexia_env_HOMEPATH", "/fixture/drive")],
    ] {
        let mut terminal = new_terminal();
        let mut processor = Processor::default();
        let mut content = new_content();
        let mut values = base().to_vec();
        values.extend(extra);
        frame(&mut processor, &mut terminal, "PowerShell", &values);
        sync_session_metadata(&mut content, &terminal);
        assert_eq!(content.session_metadata.readiness(), MetadataReadiness::Unavailable);
        assert!(content.shell_environment.is_empty());
    }
}

#[test]
fn metadata_freshness_duplicate_and_orphan_commits_cannot_reuse_old_fields() {
    let mut terminal = new_terminal();
    let mut processor = Processor::default();
    let mut content = new_content();
    fields(&mut processor, &mut terminal, &[("automexia_shell", "1"), ("automexia_shell_name", "bash"), base()[0], base()[1]]);
    write(&mut processor, &mut terminal, "automexia_env_pending", "0");
    sync_session_metadata(&mut content, &terminal);
    assert_eq!(content.session_metadata.readiness(), MetadataReadiness::Unavailable);
    frame(&mut processor, &mut terminal, "bash", &base());
    sync_session_metadata(&mut content, &terminal);
    assert_eq!(content.session_metadata.readiness(), MetadataReadiness::Complete);
    write(&mut processor, &mut terminal, "automexia_env_pending", "0");
    sync_session_metadata(&mut content, &terminal);
    assert_eq!(content.session_metadata.readiness(), MetadataReadiness::Unavailable);
}

#[test]
fn metadata_freshness_old_powershell_exact_activation_postlude_keeps_pair_compatibility() {
    let mut terminal = new_terminal();
    let mut processor = Processor::default();
    let mut content = new_content();
    write(&mut processor, &mut terminal, "automexia_env_pending", "1");
    write(&mut processor, &mut terminal, "automexia_shell_name", "PowerShell");
    fields(&mut processor, &mut terminal, &base());
    write(&mut processor, &mut terminal, "automexia_env_pending", "0");
    write(&mut processor, &mut terminal, "automexia_shell", "1");
    sync_session_metadata(&mut content, &terminal);
    assert_eq!(content.session_metadata.readiness(), MetadataReadiness::Complete);
    assert!(content.shell_integration);
    assert_eq!(content.shell_environment.len(), 2);
}

#[test]
fn metadata_freshness_nonadjacent_powershell_activation_is_not_old_hook_provenance() {
    let mut terminal = new_terminal();
    let mut processor = Processor::default();
    let mut content = new_content();
    write(&mut processor, &mut terminal, "automexia_env_pending", "1");
    write(&mut processor, &mut terminal, "automexia_shell_name", "PowerShell");
    fields(&mut processor, &mut terminal, &base());
    write(&mut processor, &mut terminal, "automexia_env_pending", "0");
    write(&mut processor, &mut terminal, "fixture_other", "value");
    write(&mut processor, &mut terminal, "automexia_shell", "1");
    sync_session_metadata(&mut content, &terminal);
    assert_eq!(content.session_metadata.readiness(), MetadataReadiness::Unavailable);
}

#[test]
fn metadata_freshness_old_fish_postlude_is_unavailable_until_complete_new_frame() {
    let mut terminal = new_terminal();
    let mut processor = Processor::default();
    let mut content = new_content();
    write(&mut processor, &mut terminal, "automexia_env_pending", "1");
    fields(&mut processor, &mut terminal, &base());
    write(&mut processor, &mut terminal, "automexia_env_pending", "0");
    write(&mut processor, &mut terminal, "automexia_shell", "1");
    write(&mut processor, &mut terminal, "automexia_shell_name", "fish");
    sync_session_metadata(&mut content, &terminal);
    assert_eq!(content.session_metadata.readiness(), MetadataReadiness::Unavailable);
    frame(&mut processor, &mut terminal, "fish", &base());
    sync_session_metadata(&mut content, &terminal);
    assert_eq!(content.session_metadata.readiness(), MetadataReadiness::Complete);
    assert_eq!(content.shell_name.as_deref(), Some("fish"));
}

#[test]
fn metadata_freshness_legacy_downgrade_cannot_reuse_framed_locations() {
    let mut terminal = new_terminal();
    let mut processor = Processor::default();
    let mut content = new_content();
    fields(&mut processor, &mut terminal, &[("automexia_shell", "1"), ("automexia_shell_name", "bash")]);
    sync_session_metadata(&mut content, &terminal);
    assert_eq!(content.session_metadata.readiness(), MetadataReadiness::Complete);
    assert_eq!(content.shell_name.as_deref(), Some("bash"));
    frame(&mut processor, &mut terminal, "PowerShell", &base());
    sync_session_metadata(&mut content, &terminal);
    write(&mut processor, &mut terminal, "automexia_shell_name", "bash");
    sync_session_metadata(&mut content, &terminal);
    assert_eq!(content.session_metadata.readiness(), MetadataReadiness::Unavailable);
    frame(&mut processor, &mut terminal, "bash", &base());
    sync_session_metadata(&mut content, &terminal);
    assert_eq!(content.session_metadata.readiness(), MetadataReadiness::Complete);
}

#[test]
fn metadata_freshness_rejected_required_value_does_not_borrow_previous_complete_value() {
    let mut terminal = new_terminal();
    let mut processor = Processor::default();
    let mut content = new_content();
    frame(&mut processor, &mut terminal, "bash", &base());
    sync_session_metadata(&mut content, &terminal);
    write(&mut processor, &mut terminal, "automexia_env_pending", "1");
    fields(&mut processor, &mut terminal, &[("automexia_shell", "1"), ("automexia_shell_name", "bash")]);
    write(&mut processor, &mut terminal, "automexia_env_HOME", &"x".repeat(8193));
    assert_eq!(terminal.user_vars["automexia_env_HOME"], "/fixture/home");
    write(&mut processor, &mut terminal, "automexia_env_KUBECONFIG", "");
    write(&mut processor, &mut terminal, "automexia_env_pending", "0");
    sync_session_metadata(&mut content, &terminal);
    assert_eq!(content.session_metadata.readiness(), MetadataReadiness::Unavailable);
    assert_eq!(content.shell_environment["HOME"], "/fixture/home");
    frame(&mut processor, &mut terminal, "bash", &base());
    sync_session_metadata(&mut content, &terminal);
    assert_eq!(content.session_metadata.readiness(), MetadataReadiness::Complete);
}

#[test]
fn metadata_freshness_rejected_first_begin_cannot_fall_back_to_legacy() {
    let mut terminal = new_terminal();
    let mut processor = Processor::default();
    let mut content = new_content();
    fields(&mut processor, &mut terminal, &[("automexia_shell", "1"), ("automexia_shell_name", "bash"), base()[0], base()[1]]);
    for index in 0..124 {
        write(&mut processor, &mut terminal, &format!("fixture_{index}"), "");
    }
    assert_eq!(terminal.user_vars.len(), 128);
    sync_session_metadata(&mut content, &terminal);
    assert_eq!(content.session_metadata.readiness(), MetadataReadiness::Complete);
    frame(&mut processor, &mut terminal, "zsh", &base());
    assert!(!terminal.user_vars.contains_key("automexia_env_pending"));
    sync_session_metadata(&mut content, &terminal);
    assert_eq!(content.session_metadata.readiness(), MetadataReadiness::Unavailable);
    assert_eq!(content.shell_name.as_deref(), Some("bash"));
}

#[test]
fn metadata_freshness_invalid_utf8_write_taints_commit_and_later_frame_recovers() {
    let mut terminal = new_terminal();
    let mut processor = Processor::default();
    let mut content = new_content();
    write(&mut processor, &mut terminal, "automexia_env_pending", "1");
    fields(&mut processor, &mut terminal, &[("automexia_shell", "1"), ("automexia_shell_name", "bash"), base()[0], base()[1]]);
    processor.advance(&mut terminal, b"\x1b]1337;SetUserVar=automexia_distro=/w==\x07");
    write(&mut processor, &mut terminal, "automexia_env_pending", "0");
    sync_session_metadata(&mut content, &terminal);
    assert_eq!(content.session_metadata.readiness(), MetadataReadiness::Unavailable);
    frame(&mut processor, &mut terminal, "bash", &base());
    sync_session_metadata(&mut content, &terminal);
    assert_eq!(content.session_metadata.readiness(), MetadataReadiness::Complete);
}

#[test]
fn metadata_freshness_cmd_complete_frame_recovers_interrupted_guest_without_paths() {
    let mut terminal = new_terminal();
    let mut processor = Processor::default();
    let mut content = new_content();
    frame(&mut processor, &mut terminal, "bash", &base());
    sync_session_metadata(&mut content, &terminal);
    write(&mut processor, &mut terminal, "automexia_env_pending", "1");
    write(&mut processor, &mut terminal, "automexia_shell_name", "fish");
    sync_session_metadata(&mut content, &terminal);
    assert_eq!(content.session_metadata.readiness(), MetadataReadiness::Pending);
    frame(&mut processor, &mut terminal, "CMD", &[]);
    sync_session_metadata(&mut content, &terminal);
    assert_eq!(content.session_metadata.readiness(), MetadataReadiness::Complete);
    assert_eq!(content.shell_name.as_deref(), Some("CMD"));
    assert!(content.shell_environment.is_empty());
}

#[test]
fn metadata_freshness_removed_marker_does_not_reset_framing_history() {
    let mut terminal = new_terminal();
    let mut processor = Processor::default();
    let mut content = new_content();
    frame(&mut processor, &mut terminal, "bash", &base());
    sync_session_metadata(&mut content, &terminal);
    terminal.user_vars.remove("automexia_env_pending");
    write(&mut processor, &mut terminal, "automexia_shell_name", "zsh");
    sync_session_metadata(&mut content, &terminal);
    assert_eq!(content.session_metadata.readiness(), MetadataReadiness::Unavailable);

    let mut independent = new_terminal();
    let mut other_content = new_content();
    fields(&mut processor, &mut independent, &[("automexia_shell", "1"), ("automexia_shell_name", "zsh")]);
    sync_session_metadata(&mut other_content, &independent);
    assert_eq!(other_content.session_metadata.readiness(), MetadataReadiness::Complete);
    assert_eq!(other_content.shell_name.as_deref(), Some("zsh"));
}

#[test]
fn metadata_readiness_real_parser_snapshot_suppresses_live_discovery() {
    let mut terminal = new_terminal();
    let mut processor = Processor::default();
    let mut content = new_content();
    frame(&mut processor, &mut terminal, "bash", &base());
    sync_session_metadata(&mut content, &terminal);
    frame(&mut processor, &mut terminal, "zsh", &[base()[0]]);
    sync_session_metadata(&mut content, &terminal);
    for active in [true, false] {
        let pane = super::semantic_snapshot(&content, (10.0, 24.0), Default::default(), 1.0, active, (416, 0));
        assert_eq!(pane.metadata_readiness, MetadataReadiness::Unavailable);
        let mut status = super::devops_status::DevOpsStatus::default();
        status.set_metadata_readiness(416, pane.metadata_readiness);
        assert!(!status.refresh_visible_session(&pane.session, || panic!("invalid parser metadata must not schedule discovery")));
    }
}

#[test]
fn metadata_readiness_unavailable_source_does_not_seed_an_independent_terminal() {
    let mut terminal = new_terminal();
    let mut processor = Processor::default();
    let mut content = new_content();
    frame(&mut processor, &mut terminal, "bash", &base());
    sync_session_metadata(&mut content, &terminal);
    frame(&mut processor, &mut terminal, "zsh", &[base()[0]]);
    sync_session_metadata(&mut content, &terminal);
    let mut independent = new_content();
    independent.apply_session_metadata_seed(content.session_metadata_seed());
    assert!(!independent.shell_integration);
    assert!(independent.shell_name.is_none());
    assert!(independent.shell_environment.is_empty());
    assert_eq!(independent.session_metadata.readiness(), MetadataReadiness::Complete);
}

#[test]
fn metadata_freshness_accepted_invalid_path_is_unavailable_and_valid_empty_pair_recovers() {
    for invalid in ["x".repeat(4097), "/fixture/\ninvalid".into()] {
        let mut terminal = new_terminal();
        let mut processor = Processor::default();
        let mut content = new_content();
        frame(&mut processor, &mut terminal, "bash", &[("automexia_env_HOME", &invalid), base()[1]]);
        assert_eq!(terminal.user_vars["automexia_env_HOME"], invalid);
        sync_session_metadata(&mut content, &terminal);
        assert_eq!(content.session_metadata.readiness(), MetadataReadiness::Unavailable);
        frame(&mut processor, &mut terminal, "bash", &[("automexia_env_HOME", ""), base()[1]]);
        sync_session_metadata(&mut content, &terminal);
        assert_eq!(content.session_metadata.readiness(), MetadataReadiness::Complete);
        assert!(content.shell_environment.values().all(String::is_empty));
    }
}
