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

fn write(
    processor: &mut Processor,
    terminal: &mut Crosswords<VoidListener>,
    key: &str,
    value: &str,
) {
    let encoded = base64::engine::general_purpose::STANDARD.encode(value);
    processor.advance(
        terminal,
        format!("\x1b]1337;SetUserVar={key}={encoded}\x07").as_bytes(),
    );
}

fn fields(
    processor: &mut Processor,
    terminal: &mut Crosswords<VoidListener>,
    values: &[(&str, &str)],
) {
    for &(key, value) in values {
        write(processor, terminal, key, value);
    }
}

fn frame(
    processor: &mut Processor,
    terminal: &mut Crosswords<VoidListener>,
    shell: &str,
    values: &[(&str, &str)],
) {
    write(processor, terminal, "automexia_env_pending", "1");
    fields(
        processor,
        terminal,
        &[("automexia_shell", "1"), ("automexia_shell_name", shell)],
    );
    fields(processor, terminal, values);
    write(processor, terminal, "automexia_env_pending", "0");
}

fn base() -> [(&'static str, &'static str); 2] {
    [
        ("automexia_env_HOME", "/fixture/home"),
        ("automexia_env_KUBECONFIG", ""),
    ]
}

fn new_content() -> RenderableContent {
    RenderableContent::new(Cursor::default())
}

const CMD_PARENT: &str = "0123456789abcdef0123456789abcdef";
const CMD_CHILD: &str = "fedcba9876543210fedcba9876543210";

fn register_cmd(
    processor: &mut Processor,
    terminal: &mut Crosswords<VoidListener>,
    token: &str,
    user: &str,
    path: &str,
) {
    write(
        processor,
        terminal,
        &format!("automexia_cmd_user_v1_{token}"),
        user,
    );
    write(
        processor,
        terminal,
        &format!("automexia_cmd_path_v1_{token}"),
        path,
    );
}

fn cmd_reference(
    processor: &mut Processor,
    terminal: &mut Crosswords<VoidListener>,
    token: &str,
) {
    frame(
        processor,
        terminal,
        "CMD",
        &[("automexia_cmd_ref_v1", token)],
    );
}

#[test]
fn cmd_reference_nested_identity_restores_without_guest_or_provider_state() {
    let (mut terminal, mut processor, mut content) =
        (new_terminal(), Processor::default(), new_content());
    let user = "fixture-\u{00e9}\u{754c}".repeat(40);
    let path = format!("C:\\{}\\cmd.exe", "fixture-\u{00e9}\u{754c}".repeat(50));
    register_cmd(&mut processor, &mut terminal, CMD_PARENT, &user, &path);
    register_cmd(
        &mut processor,
        &mut terminal,
        CMD_CHILD,
        "child",
        "C:\\fixture-child\\cmd.exe",
    );
    for token in [CMD_PARENT, CMD_CHILD, CMD_PARENT] {
        frame(
            &mut processor,
            &mut terminal,
            "bash",
            &[
                base()[0],
                base()[1],
                ("automexia_distro", "FixtureLinux"),
                ("automexia_shell_user", "guest"),
                ("automexia_env_AWS_PROFILE", "fixture"),
            ],
        );
        sync_session_metadata(&mut content, &terminal);
        cmd_reference(&mut processor, &mut terminal, token);
        sync_session_metadata(&mut content, &terminal);
        assert_eq!(
            content.session_metadata.readiness(),
            MetadataReadiness::Complete
        );
        assert_eq!(content.shell_name.as_deref(), Some("CMD"));
        assert_eq!(
            content.shell_user.as_deref(),
            Some(if token == CMD_PARENT {
                user.as_str()
            } else {
                "child"
            })
        );
        assert_eq!(
            content.shell_path.as_deref(),
            Some(if token == CMD_PARENT {
                path.as_str()
            } else {
                "C:\\fixture-child\\cmd.exe"
            })
        );
        assert!(content.shell_distro.is_none());
        assert!(content.shell_environment.is_empty());
    }
    let count = terminal.user_vars.len();
    for _ in 0..256 {
        register_cmd(&mut processor, &mut terminal, CMD_PARENT, &user, &path);
        cmd_reference(&mut processor, &mut terminal, CMD_PARENT);
        sync_session_metadata(&mut content, &terminal);
    }
    assert_eq!(terminal.user_vars.len(), count);
    assert_eq!(content.shell_user.as_deref(), Some(user.as_str()));
}

#[test]
fn cmd_reference_reset_clear_and_independent_terminal_preserve_isolation() {
    let (mut terminal, mut processor, mut content) =
        (new_terminal(), Processor::default(), new_content());
    register_cmd(
        &mut processor,
        &mut terminal,
        CMD_PARENT,
        "parent",
        "C:\\fixture\\cmd.exe",
    );
    for reset in [b"\x1b[2J\x1b[H".as_slice(), b"\x1bc".as_slice()] {
        processor.advance(&mut terminal, reset);
        cmd_reference(&mut processor, &mut terminal, CMD_PARENT);
        sync_session_metadata(&mut content, &terminal);
        assert_eq!(
            content.session_metadata.readiness(),
            MetadataReadiness::Complete
        );
        assert_eq!(content.shell_user.as_deref(), Some("parent"));
    }
    let mut independent = new_terminal();
    let mut other_content = new_content();
    cmd_reference(&mut processor, &mut independent, CMD_PARENT);
    sync_session_metadata(&mut other_content, &independent);
    assert_eq!(
        other_content.session_metadata.readiness(),
        MetadataReadiness::Unavailable
    );
    assert!(other_content.shell_user.is_none());
}

#[test]
fn cmd_reference_missing_partial_malformed_or_rewritten_registration_clears_guest() {
    for defect in 0..7 {
        let (mut terminal, mut processor, mut content) =
            (new_terminal(), Processor::default(), new_content());
        register_cmd(
            &mut processor,
            &mut terminal,
            CMD_PARENT,
            "parent",
            "C:\\fixture\\cmd.exe",
        );
        frame(
            &mut processor,
            &mut terminal,
            "bash",
            &[
                base()[0],
                base()[1],
                ("automexia_shell_user", "guest"),
                ("automexia_distro", "FixtureLinux"),
            ],
        );
        sync_session_metadata(&mut content, &terminal);
        let user_key = format!("automexia_cmd_user_v1_{CMD_PARENT}");
        let path_key = format!("automexia_cmd_path_v1_{CMD_PARENT}");
        match defect {
            0 => {
                terminal.user_vars.remove(&user_key);
            }
            1 => {
                terminal.user_vars.remove(&path_key);
            }
            2 => write(&mut processor, &mut terminal, &user_key, "rewritten"),
            3 => register_cmd(
                &mut processor,
                &mut terminal,
                CMD_PARENT,
                "bad\nuser",
                "C:\\fixture\\cmd.exe",
            ),
            4 => register_cmd(
                &mut processor,
                &mut terminal,
                CMD_PARENT,
                "user",
                &"p".repeat(4097),
            ),
            _ => {}
        }
        cmd_reference(
            &mut processor,
            &mut terminal,
            if defect == 5 {
                "../malformed"
            } else if defect == 6 {
                CMD_CHILD
            } else {
                CMD_PARENT
            },
        );
        sync_session_metadata(&mut content, &terminal);
        assert_eq!(
            content.session_metadata.readiness(),
            MetadataReadiness::Unavailable,
            "case {defect}"
        );
        assert_eq!(content.shell_name.as_deref(), Some("CMD"));
        assert!(content.shell_user.is_none(), "case {defect}");
        assert!(content.shell_path.is_none(), "case {defect}");
        assert!(content.shell_distro.is_none(), "case {defect}");
        assert!(content.shell_environment.is_empty(), "case {defect}");
    }
}

#[test]
fn cmd_reference_saturated_dictionary_and_late_registration_fail_without_eviction() {
    let (mut terminal, mut processor, mut content) =
        (new_terminal(), Processor::default(), new_content());
    register_cmd(
        &mut processor,
        &mut terminal,
        CMD_PARENT,
        "parent",
        "C:\\fixture\\cmd.exe",
    );
    cmd_reference(&mut processor, &mut terminal, CMD_PARENT);
    sync_session_metadata(&mut content, &terminal);
    let mut index = 0;
    while terminal.user_vars.len() < 128 {
        write(
            &mut processor,
            &mut terminal,
            &format!("fixture{index}"),
            "",
        );
        index += 1;
    }
    register_cmd(
        &mut processor,
        &mut terminal,
        CMD_CHILD,
        "child",
        "C:\\child\\cmd.exe",
    );
    cmd_reference(&mut processor, &mut terminal, CMD_CHILD);
    sync_session_metadata(&mut content, &terminal);
    assert_eq!(terminal.user_vars.len(), 128);
    assert_eq!(
        content.session_metadata.readiness(),
        MetadataReadiness::Unavailable
    );
    assert!(content.shell_user.is_none());
    cmd_reference(&mut processor, &mut terminal, CMD_PARENT);
    sync_session_metadata(&mut content, &terminal);
    assert_eq!(content.shell_user.as_deref(), Some("parent"));
    assert_eq!(
        content.session_metadata.readiness(),
        MetadataReadiness::Complete
    );
    register_cmd(
        &mut processor,
        &mut terminal,
        CMD_PARENT,
        "changed",
        "C:\\changed\\cmd.exe",
    );
    sync_session_metadata(&mut content, &terminal);
    assert_eq!(
        content.session_metadata.readiness(),
        MetadataReadiness::Unavailable
    );
    assert!(content.shell_user.is_none());
    cmd_reference(&mut processor, &mut terminal, CMD_PARENT);
    sync_session_metadata(&mut content, &terminal);
    assert_eq!(content.shell_user.as_deref(), Some("changed"));
}

#[test]
fn cmd_reference_late_or_malformed_reference_cannot_downgrade_to_legacy() {
    for token in [
        CMD_PARENT.to_owned(),
        "r".repeat(8192),
        "bad\nreference".to_owned(),
    ] {
        let (mut terminal, mut processor, mut content) =
            (new_terminal(), Processor::default(), new_content());
        register_cmd(
            &mut processor,
            &mut terminal,
            CMD_PARENT,
            "parent",
            "C:\\fixture\\cmd.exe",
        );
        frame(&mut processor, &mut terminal, "CMD", &[]);
        write(
            &mut processor,
            &mut terminal,
            "automexia_cmd_ref_v1",
            &token,
        );
        sync_session_metadata(&mut content, &terminal);
        assert_eq!(
            content.session_metadata.readiness(),
            MetadataReadiness::Unavailable
        );
        assert!(content.shell_user.is_none());
        cmd_reference(&mut processor, &mut terminal, &token);
        sync_session_metadata(&mut content, &terminal);
        assert_eq!(
            content.session_metadata.readiness(),
            if token == CMD_PARENT {
                MetadataReadiness::Complete
            } else {
                MetadataReadiness::Unavailable
            }
        );
    }
}

#[test]
fn metadata_freshness_equal_pair_replaces_five_key_frame_without_inheriting_candidates() {
    let mut terminal = new_terminal();
    let mut processor = Processor::default();
    let mut content = new_content();
    let five = [
        base()[0],
        base()[1],
        ("automexia_env_HOMEDRIVE", "D:"),
        ("automexia_env_HOMEPATH", "/fixture/drive"),
        ("automexia_env_USERPROFILE", "C:/fixture/profile"),
    ];
    frame(&mut processor, &mut terminal, "PowerShell", &five);
    sync_session_metadata(&mut content, &terminal);
    frame(&mut processor, &mut terminal, "PowerShell", &base());
    sync_session_metadata(&mut content, &terminal);
    assert_eq!(
        content.session_metadata.readiness(),
        MetadataReadiness::Complete
    );
    assert_eq!(content.shell_environment.len(), 2);
    assert_eq!(content.shell_environment["HOME"], "/fixture/home");
    assert_eq!(content.shell_environment["KUBECONFIG"], "");
}

#[test]
fn shell_selector_frames_switch_and_clear_without_borrowing_previous_values() {
    let mut terminal = new_terminal();
    let mut processor = Processor::default();
    let mut content = new_content();
    let first = [
        base()[0],
        base()[1],
        ("automexia_env_DOCKER_CONTEXT", "team-one"),
        ("automexia_env_AWS_PROFILE", "staging"),
        ("automexia_env_TF_WORKSPACE", "review"),
        ("automexia_env_AUTOMEXIA_ENV", "development"),
    ];
    frame(&mut processor, &mut terminal, "bash", &first);
    sync_session_metadata(&mut content, &terminal);
    assert_eq!(
        content.session_metadata.readiness(),
        MetadataReadiness::Complete
    );
    assert_eq!(
        content
            .shell_environment
            .get("DOCKER_CONTEXT")
            .map(String::as_str),
        Some("team-one")
    );
    assert_eq!(
        content
            .shell_environment
            .get("AWS_PROFILE")
            .map(String::as_str),
        Some("staging")
    );
    assert_eq!(
        content
            .shell_environment
            .get("TF_WORKSPACE")
            .map(String::as_str),
        Some("review")
    );
    assert_eq!(
        content
            .shell_environment
            .get("AUTOMEXIA_ENV")
            .map(String::as_str),
        Some("development")
    );

    let second = [
        base()[0],
        base()[1],
        ("automexia_env_DOCKER_CONTEXT", "team-two"),
        ("automexia_env_AWS_PROFILE", ""),
        ("automexia_env_TF_WORKSPACE", ""),
        ("automexia_env_AUTOMEXIA_ENV", "production"),
    ];
    frame(&mut processor, &mut terminal, "bash", &second);
    sync_session_metadata(&mut content, &terminal);
    assert_eq!(
        content
            .shell_environment
            .get("DOCKER_CONTEXT")
            .map(String::as_str),
        Some("team-two")
    );
    assert_eq!(
        content
            .shell_environment
            .get("AWS_PROFILE")
            .map(String::as_str),
        Some("")
    );
    assert_eq!(
        content
            .shell_environment
            .get("TF_WORKSPACE")
            .map(String::as_str),
        Some("")
    );
    assert_eq!(
        content
            .shell_environment
            .get("AUTOMEXIA_ENV")
            .map(String::as_str),
        Some("production")
    );

    frame(&mut processor, &mut terminal, "PowerShell", &base());
    sync_session_metadata(&mut content, &terminal);
    assert!(!content.shell_environment.contains_key("DOCKER_CONTEXT"));
    assert!(!content.shell_environment.contains_key("AWS_PROFILE"));
    assert!(!content.shell_environment.contains_key("TF_WORKSPACE"));
    assert!(!content.shell_environment.contains_key("AUTOMEXIA_ENV"));
}

#[test]
fn late_selector_write_invalidates_frame_until_a_complete_prompt_republishes_it() {
    let mut terminal = new_terminal();
    let mut processor = Processor::default();
    let mut content = new_content();
    let first = [
        base()[0],
        base()[1],
        ("automexia_env_DOCKER_CONTEXT", "review"),
    ];
    frame(&mut processor, &mut terminal, "bash", &first);
    sync_session_metadata(&mut content, &terminal);
    assert_eq!(
        content.session_metadata.readiness(),
        MetadataReadiness::Complete
    );
    write(
        &mut processor,
        &mut terminal,
        "automexia_env_DOCKER_CONTEXT",
        "production",
    );
    sync_session_metadata(&mut content, &terminal);
    assert_eq!(
        content.session_metadata.readiness(),
        MetadataReadiness::Unavailable
    );
    frame(&mut processor, &mut terminal, "bash", &first);
    sync_session_metadata(&mut content, &terminal);
    assert_eq!(
        content.session_metadata.readiness(),
        MetadataReadiness::Complete
    );
    assert_eq!(
        content
            .shell_environment
            .get("DOCKER_CONTEXT")
            .map(String::as_str),
        Some("review")
    );
}

#[test]
fn metadata_freshness_coalesced_frames_clear_omitted_optional_identity() {
    let mut terminal = new_terminal();
    let mut processor = Processor::default();
    let mut content = new_content();
    let extended = [
        base()[0],
        base()[1],
        ("automexia_distro", "Fixture-Distro"),
        ("automexia_os_version", "1"),
        ("automexia_shell_user", "alice"),
        ("automexia_shell_path", "/fixture/shell"),
    ];
    frame(&mut processor, &mut terminal, "bash", &extended);
    // No paint between the two complete frames, and the required values are equal.
    frame(&mut processor, &mut terminal, "bash", &base());
    sync_session_metadata(&mut content, &terminal);
    assert_eq!(
        content.session_metadata.readiness(),
        MetadataReadiness::Complete
    );
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
        assert_eq!(
            content.session_metadata.readiness(),
            MetadataReadiness::Pending
        );
        assert_eq!(content.shell_name.as_deref(), Some("PowerShell"));
        assert_eq!(content.shell_environment, previous);
    }
    fields(
        &mut processor,
        &mut terminal,
        &[("automexia_shell", "1"), base()[0], base()[1]],
    );
    write(&mut processor, &mut terminal, "automexia_env_pending", "0");
    sync_session_metadata(&mut content, &terminal);
    assert_eq!(
        content.session_metadata.readiness(),
        MetadataReadiness::Complete
    );
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
        frame(
            &mut processor,
            &mut terminal,
            "zsh",
            &[(included, "/fixture/new")],
        );
        sync_session_metadata(&mut content, &terminal);
        assert_eq!(
            content.session_metadata.readiness(),
            MetadataReadiness::Unavailable
        );
        assert_eq!(content.shell_name.as_deref(), Some("bash"));
        assert_eq!(content.shell_environment, previous);
    }
}

#[test]
fn metadata_freshness_partial_windows_trio_is_unavailable_on_every_platform() {
    for extra in [
        vec![("automexia_env_HOMEDRIVE", "D:")],
        vec![
            ("automexia_env_HOMEDRIVE", "D:"),
            ("automexia_env_HOMEPATH", "/fixture/drive"),
        ],
    ] {
        let mut terminal = new_terminal();
        let mut processor = Processor::default();
        let mut content = new_content();
        let mut values = base().to_vec();
        values.extend(extra);
        frame(&mut processor, &mut terminal, "PowerShell", &values);
        sync_session_metadata(&mut content, &terminal);
        assert_eq!(
            content.session_metadata.readiness(),
            MetadataReadiness::Unavailable
        );
        assert!(content.shell_environment.is_empty());
    }
}

#[test]
fn metadata_freshness_duplicate_and_orphan_commits_cannot_reuse_old_fields() {
    let mut terminal = new_terminal();
    let mut processor = Processor::default();
    let mut content = new_content();
    fields(
        &mut processor,
        &mut terminal,
        &[
            ("automexia_shell", "1"),
            ("automexia_shell_name", "bash"),
            base()[0],
            base()[1],
        ],
    );
    write(&mut processor, &mut terminal, "automexia_env_pending", "0");
    sync_session_metadata(&mut content, &terminal);
    assert_eq!(
        content.session_metadata.readiness(),
        MetadataReadiness::Unavailable
    );
    frame(&mut processor, &mut terminal, "bash", &base());
    sync_session_metadata(&mut content, &terminal);
    assert_eq!(
        content.session_metadata.readiness(),
        MetadataReadiness::Complete
    );
    write(&mut processor, &mut terminal, "automexia_env_pending", "0");
    sync_session_metadata(&mut content, &terminal);
    assert_eq!(
        content.session_metadata.readiness(),
        MetadataReadiness::Unavailable
    );
}

#[test]
fn metadata_freshness_old_powershell_exact_activation_postlude_keeps_pair_compatibility()
{
    let mut terminal = new_terminal();
    let mut processor = Processor::default();
    let mut content = new_content();
    write(&mut processor, &mut terminal, "automexia_env_pending", "1");
    write(
        &mut processor,
        &mut terminal,
        "automexia_shell_name",
        "PowerShell",
    );
    fields(&mut processor, &mut terminal, &base());
    write(&mut processor, &mut terminal, "automexia_env_pending", "0");
    write(&mut processor, &mut terminal, "automexia_shell", "1");
    sync_session_metadata(&mut content, &terminal);
    assert_eq!(
        content.session_metadata.readiness(),
        MetadataReadiness::Complete
    );
    assert!(content.shell_integration);
    assert_eq!(content.shell_environment.len(), 2);
}

#[test]
fn metadata_freshness_nonadjacent_powershell_activation_is_not_old_hook_provenance() {
    let mut terminal = new_terminal();
    let mut processor = Processor::default();
    let mut content = new_content();
    write(&mut processor, &mut terminal, "automexia_env_pending", "1");
    write(
        &mut processor,
        &mut terminal,
        "automexia_shell_name",
        "PowerShell",
    );
    fields(&mut processor, &mut terminal, &base());
    write(&mut processor, &mut terminal, "automexia_env_pending", "0");
    write(&mut processor, &mut terminal, "fixture_other", "value");
    write(&mut processor, &mut terminal, "automexia_shell", "1");
    sync_session_metadata(&mut content, &terminal);
    assert_eq!(
        content.session_metadata.readiness(),
        MetadataReadiness::Unavailable
    );
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
    write(
        &mut processor,
        &mut terminal,
        "automexia_shell_name",
        "fish",
    );
    sync_session_metadata(&mut content, &terminal);
    assert_eq!(
        content.session_metadata.readiness(),
        MetadataReadiness::Unavailable
    );
    frame(&mut processor, &mut terminal, "fish", &base());
    sync_session_metadata(&mut content, &terminal);
    assert_eq!(
        content.session_metadata.readiness(),
        MetadataReadiness::Complete
    );
    assert_eq!(content.shell_name.as_deref(), Some("fish"));
}

#[test]
fn metadata_freshness_legacy_downgrade_cannot_reuse_framed_locations() {
    let mut terminal = new_terminal();
    let mut processor = Processor::default();
    let mut content = new_content();
    fields(
        &mut processor,
        &mut terminal,
        &[("automexia_shell", "1"), ("automexia_shell_name", "bash")],
    );
    sync_session_metadata(&mut content, &terminal);
    assert_eq!(
        content.session_metadata.readiness(),
        MetadataReadiness::Complete
    );
    assert_eq!(content.shell_name.as_deref(), Some("bash"));
    frame(&mut processor, &mut terminal, "PowerShell", &base());
    sync_session_metadata(&mut content, &terminal);
    write(
        &mut processor,
        &mut terminal,
        "automexia_shell_name",
        "bash",
    );
    sync_session_metadata(&mut content, &terminal);
    assert_eq!(
        content.session_metadata.readiness(),
        MetadataReadiness::Unavailable
    );
    frame(&mut processor, &mut terminal, "bash", &base());
    sync_session_metadata(&mut content, &terminal);
    assert_eq!(
        content.session_metadata.readiness(),
        MetadataReadiness::Complete
    );
}

#[test]
fn metadata_freshness_rejected_required_value_does_not_borrow_previous_complete_value() {
    let mut terminal = new_terminal();
    let mut processor = Processor::default();
    let mut content = new_content();
    frame(&mut processor, &mut terminal, "bash", &base());
    sync_session_metadata(&mut content, &terminal);
    write(&mut processor, &mut terminal, "automexia_env_pending", "1");
    fields(
        &mut processor,
        &mut terminal,
        &[("automexia_shell", "1"), ("automexia_shell_name", "bash")],
    );
    write(
        &mut processor,
        &mut terminal,
        "automexia_env_HOME",
        &"x".repeat(8193),
    );
    assert_eq!(terminal.user_vars["automexia_env_HOME"], "/fixture/home");
    write(
        &mut processor,
        &mut terminal,
        "automexia_env_KUBECONFIG",
        "",
    );
    write(&mut processor, &mut terminal, "automexia_env_pending", "0");
    sync_session_metadata(&mut content, &terminal);
    assert_eq!(
        content.session_metadata.readiness(),
        MetadataReadiness::Unavailable
    );
    assert_eq!(content.shell_environment["HOME"], "/fixture/home");
    frame(&mut processor, &mut terminal, "bash", &base());
    sync_session_metadata(&mut content, &terminal);
    assert_eq!(
        content.session_metadata.readiness(),
        MetadataReadiness::Complete
    );
}

#[test]
fn metadata_freshness_rejected_first_begin_cannot_fall_back_to_legacy() {
    let mut terminal = new_terminal();
    let mut processor = Processor::default();
    let mut content = new_content();
    fields(
        &mut processor,
        &mut terminal,
        &[
            ("automexia_shell", "1"),
            ("automexia_shell_name", "bash"),
            base()[0],
            base()[1],
        ],
    );
    for index in 0..124 {
        write(
            &mut processor,
            &mut terminal,
            &format!("fixture_{index}"),
            "",
        );
    }
    assert_eq!(terminal.user_vars.len(), 128);
    sync_session_metadata(&mut content, &terminal);
    assert_eq!(
        content.session_metadata.readiness(),
        MetadataReadiness::Complete
    );
    frame(&mut processor, &mut terminal, "zsh", &base());
    assert!(!terminal.user_vars.contains_key("automexia_env_pending"));
    sync_session_metadata(&mut content, &terminal);
    assert_eq!(
        content.session_metadata.readiness(),
        MetadataReadiness::Unavailable
    );
    assert_eq!(content.shell_name.as_deref(), Some("bash"));
}

#[test]
fn metadata_freshness_invalid_utf8_write_taints_commit_and_later_frame_recovers() {
    let mut terminal = new_terminal();
    let mut processor = Processor::default();
    let mut content = new_content();
    write(&mut processor, &mut terminal, "automexia_env_pending", "1");
    fields(
        &mut processor,
        &mut terminal,
        &[
            ("automexia_shell", "1"),
            ("automexia_shell_name", "bash"),
            base()[0],
            base()[1],
        ],
    );
    processor.advance(
        &mut terminal,
        b"\x1b]1337;SetUserVar=automexia_distro=/w==\x07",
    );
    write(&mut processor, &mut terminal, "automexia_env_pending", "0");
    sync_session_metadata(&mut content, &terminal);
    assert_eq!(
        content.session_metadata.readiness(),
        MetadataReadiness::Unavailable
    );
    frame(&mut processor, &mut terminal, "bash", &base());
    sync_session_metadata(&mut content, &terminal);
    assert_eq!(
        content.session_metadata.readiness(),
        MetadataReadiness::Complete
    );
}

#[test]
fn metadata_freshness_cmd_complete_frame_recovers_interrupted_guest_without_paths() {
    let mut terminal = new_terminal();
    let mut processor = Processor::default();
    let mut content = new_content();
    frame(&mut processor, &mut terminal, "bash", &base());
    sync_session_metadata(&mut content, &terminal);
    write(&mut processor, &mut terminal, "automexia_env_pending", "1");
    write(
        &mut processor,
        &mut terminal,
        "automexia_shell_name",
        "fish",
    );
    sync_session_metadata(&mut content, &terminal);
    assert_eq!(
        content.session_metadata.readiness(),
        MetadataReadiness::Pending
    );
    frame(&mut processor, &mut terminal, "CMD", &[]);
    sync_session_metadata(&mut content, &terminal);
    assert_eq!(
        content.session_metadata.readiness(),
        MetadataReadiness::Complete
    );
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
    assert_eq!(
        content.session_metadata.readiness(),
        MetadataReadiness::Unavailable
    );

    let mut independent = new_terminal();
    let mut other_content = new_content();
    fields(
        &mut processor,
        &mut independent,
        &[("automexia_shell", "1"), ("automexia_shell_name", "zsh")],
    );
    sync_session_metadata(&mut other_content, &independent);
    assert_eq!(
        other_content.session_metadata.readiness(),
        MetadataReadiness::Complete
    );
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
        let pane = super::semantic_snapshot(
            &content,
            (10.0, 24.0),
            Default::default(),
            1.0,
            active,
            (416, 0),
        );
        assert_eq!(pane.metadata_readiness, MetadataReadiness::Unavailable);
        let mut status = super::devops_status::DevOpsStatus::default();
        status.set_metadata_readiness(416, pane.metadata_readiness);
        assert!(!status.refresh_visible_session(&pane.session, || panic!(
            "invalid parser metadata must not schedule discovery"
        )));
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
    content.current_directory = Some("/fixture/retained".into());
    content.terminal_title = "fixture retained title".into();
    let mut independent = new_content();
    independent.apply_session_metadata_seed(content.session_metadata_seed());
    assert!(
        independent.current_directory.is_none(),
        "incomplete directory reached an independent seed"
    );
    assert!(
        independent.terminal_title.is_empty(),
        "incomplete title reached an independent seed"
    );
    assert!(!independent.shell_integration);
    assert!(independent.shell_name.is_none());
    assert!(independent.shell_environment.is_empty());
    assert_eq!(
        independent.session_metadata.readiness(),
        MetadataReadiness::Complete
    );
}

#[test]
fn metadata_freshness_accepted_invalid_path_is_unavailable_and_valid_empty_pair_recovers()
{
    for invalid in ["x".repeat(4097), "/fixture/\ninvalid".into()] {
        let mut terminal = new_terminal();
        let mut processor = Processor::default();
        let mut content = new_content();
        frame(
            &mut processor,
            &mut terminal,
            "bash",
            &[("automexia_env_HOME", &invalid), base()[1]],
        );
        assert_eq!(terminal.user_vars["automexia_env_HOME"], invalid);
        sync_session_metadata(&mut content, &terminal);
        assert_eq!(
            content.session_metadata.readiness(),
            MetadataReadiness::Unavailable
        );
        frame(
            &mut processor,
            &mut terminal,
            "bash",
            &[("automexia_env_HOME", ""), base()[1]],
        );
        sync_session_metadata(&mut content, &terminal);
        assert_eq!(
            content.session_metadata.readiness(),
            MetadataReadiness::Complete
        );
        assert!(content.shell_environment.values().all(String::is_empty));
    }
}

#[test]
fn metadata_readiness_pending_source_does_not_seed_an_independent_terminal() {
    let mut terminal = new_terminal();
    let mut processor = Processor::default();
    let mut content = new_content();
    frame(&mut processor, &mut terminal, "bash", &base());
    sync_session_metadata(&mut content, &terminal);
    write(&mut processor, &mut terminal, "automexia_env_pending", "1");
    sync_session_metadata(&mut content, &terminal);
    assert_eq!(
        content.session_metadata.readiness(),
        MetadataReadiness::Pending
    );
    content.current_directory = Some("/fixture/retained".into());
    content.terminal_title = "fixture retained title".into();
    let mut independent = new_content();
    independent.apply_session_metadata_seed(content.session_metadata_seed());
    assert!(
        independent.current_directory.is_none(),
        "incomplete directory reached an independent seed"
    );
    assert!(
        independent.terminal_title.is_empty(),
        "incomplete title reached an independent seed"
    );
    assert!(!independent.shell_integration);
    assert!(independent.shell_name.is_none());
    assert!(independent.shell_environment.is_empty());
    assert_eq!(
        independent.session_metadata.readiness(),
        MetadataReadiness::Complete
    );
}

#[test]
fn metadata_readiness_clone_caller_uses_owned_launch_while_metadata_is_incomplete() {
    use crate::context::launch::SessionLaunchDescriptor;
    for (shell, values, expected_program) in [
        ("CMD", vec![("automexia_shell_path", "cmd.exe")], "cmd.exe"),
        (
            "bash",
            vec![
                base()[0],
                base()[1],
                ("automexia_distro", "Fixture-Distro"),
                ("automexia_shell_user", "fixture"),
                ("automexia_shell_path", "/bin/bash"),
            ],
            // On Windows the live distro identifies a nested WSL session.
            // On Unix it is display metadata: retain the native launch owner.
            if cfg!(target_os = "windows") {
                "wsl.exe"
            } else {
                "fixture-shell"
            },
        ),
    ] {
        let mut manager = crate::context::ContextManager::start_with_capacity(
            4,
            VoidListener {},
            WindowId::from(0),
        )
        .unwrap();
        manager.current_mut().launch_descriptor = SessionLaunchDescriptor::new(
            Some("fixture-shell".into()),
            vec!["--fixture".into()],
            Vec::new(),
            Some("fixture-profile".into()),
            Some("/fixture/owned".into()),
        );
        let mut processor = Processor::default();
        {
            let source = manager.current_mut();
            let mut terminal = source.terminal.lock();
            terminal.current_directory = Some("/fixture/stale".into());
            frame(&mut processor, &mut terminal, shell, &values);
            sync_session_metadata(&mut source.renderable_content, &terminal);
        }
        let complete = manager.test_create_cloned_context(0).unwrap();
        assert_eq!(complete.launch_descriptor.program(), Some(expected_program));
        if expected_program == "fixture-shell" {
            assert_eq!(complete.launch_descriptor.args(), ["--fixture"]);
            assert_eq!(
                complete.launch_descriptor.profile_identity(),
                Some("fixture-profile")
            );
        }
        drop(complete);
        {
            let source = manager.current_mut();
            let mut terminal = source.terminal.lock();
            write(&mut processor, &mut terminal, "automexia_env_pending", "1");
            sync_session_metadata(&mut source.renderable_content, &terminal);
        }
        let pending = manager.test_create_cloned_context(0).unwrap();
        assert_eq!(pending.launch_descriptor.program(), Some("fixture-shell"));
        assert_eq!(pending.launch_descriptor.args(), ["--fixture"]);
        assert_eq!(
            pending.launch_descriptor.profile_identity(),
            Some("fixture-profile")
        );
        assert_eq!(
            pending.launch_descriptor.starting_directory(),
            Some("/fixture/owned")
        );
        drop(pending);
        {
            let source = manager.current_mut();
            let mut terminal = source.terminal.lock();
            write(&mut processor, &mut terminal, "automexia_env_pending", "0");
            sync_session_metadata(&mut source.renderable_content, &terminal);
        }
        let unavailable = manager.test_create_cloned_context(0).unwrap();
        assert_eq!(
            unavailable.launch_descriptor.program(),
            Some("fixture-shell")
        );
        assert_eq!(unavailable.launch_descriptor.args(), ["--fixture"]);
        assert_eq!(
            unavailable.launch_descriptor.profile_identity(),
            Some("fixture-profile")
        );
        assert_eq!(
            unavailable.launch_descriptor.starting_directory(),
            Some("/fixture/owned")
        );
    }
}

#[test]
fn powershell_wsl_roundtrip_keeps_fresh_sessions_and_panes_independent() {
    let mut processor = Processor::default();
    let mut changing_terminal = new_terminal();
    let mut changing = new_content();
    let mut sibling_terminal = new_terminal();
    let mut sibling = new_content();
    let host = [
        ("automexia_env_HOME", "/fixture/host"),
        ("automexia_env_KUBECONFIG", "/fixture/host/kube"),
        ("automexia_env_HOMEDRIVE", "C:"),
        ("automexia_env_HOMEPATH", "/fixture/host"),
        ("automexia_env_USERPROFILE", "C:/fixture/host"),
        ("automexia_shell_user", "host"),
        ("automexia_shell_path", "powershell.exe"),
        ("automexia_distro", ""),
        ("automexia_os_version", ""),
    ];
    let guest = [
        ("automexia_env_HOME", "/fixture/guest"),
        ("automexia_env_KUBECONFIG", ""),
        ("automexia_shell_user", "guest"),
        ("automexia_shell_path", "/bin/bash"),
        ("automexia_distro", "Fixture-Distro"),
        ("automexia_os_version", "24.04"),
    ];
    frame(&mut processor, &mut changing_terminal, "PowerShell", &host);
    sync_session_metadata(&mut changing, &changing_terminal);
    let mut sibling_host = host;
    sibling_host[5].1 = "sibling";
    frame(
        &mut processor,
        &mut sibling_terminal,
        "PowerShell",
        &sibling_host,
    );
    sync_session_metadata(&mut sibling, &sibling_terminal);
    assert_eq!(changing.shell_user.as_deref(), Some("host"));
    assert_eq!(sibling.shell_user.as_deref(), Some("sibling"));

    write(
        &mut processor,
        &mut changing_terminal,
        "automexia_env_pending",
        "1",
    );
    sync_session_metadata(&mut changing, &changing_terminal);
    assert_eq!(
        changing.session_metadata.readiness(),
        MetadataReadiness::Pending
    );
    frame(&mut processor, &mut changing_terminal, "bash", &guest);
    sync_session_metadata(&mut changing, &changing_terminal);
    assert_eq!(
        changing.session_metadata.readiness(),
        MetadataReadiness::Complete
    );
    assert_eq!(changing.shell_name.as_deref(), Some("bash"));
    assert_eq!(changing.shell_distro.as_deref(), Some("Fixture-Distro"));
    assert_eq!(changing.shell_user.as_deref(), Some("guest"));
    assert_eq!(changing.shell_environment["HOME"], "/fixture/guest");
    assert!(!changing.shell_environment.contains_key("USERPROFILE"));
    assert_eq!(sibling.shell_name.as_deref(), Some("PowerShell"));
    assert!(sibling.shell_distro.is_none());
    assert_eq!(sibling.shell_user.as_deref(), Some("sibling"));

    let mut fresh_terminal = new_terminal();
    let mut fresh = new_content();
    frame(&mut processor, &mut fresh_terminal, "bash", &guest);
    sync_session_metadata(&mut fresh, &fresh_terminal);
    assert_eq!(
        fresh.session_metadata.readiness(),
        MetadataReadiness::Complete
    );
    assert_eq!(fresh.shell_distro.as_deref(), Some("Fixture-Distro"));
    assert_eq!(fresh.shell_user.as_deref(), Some("guest"));
    assert!(!fresh.shell_environment.contains_key("USERPROFILE"));

    frame(&mut processor, &mut changing_terminal, "PowerShell", &host);
    sync_session_metadata(&mut changing, &changing_terminal);
    assert_eq!(
        changing.session_metadata.readiness(),
        MetadataReadiness::Complete
    );
    assert_eq!(changing.shell_name.as_deref(), Some("PowerShell"));
    assert!(changing.shell_distro.is_none());
    assert!(changing.shell_os_version.is_none());
    assert_eq!(changing.shell_user.as_deref(), Some("host"));
    assert_eq!(changing.shell_environment["HOME"], "/fixture/host");
    // Only a native Windows frame admits Windows default-home candidates.
    // Unix still exercises the whole roundtrip, but must discard those hints.
    assert_eq!(
        changing
            .shell_environment
            .get("USERPROFILE")
            .map(String::as_str),
        cfg!(windows).then_some("C:/fixture/host")
    );
    for name in ["HOMEDRIVE", "HOMEPATH"] {
        assert_eq!(changing.shell_environment.contains_key(name), cfg!(windows));
    }
    assert_eq!(sibling.shell_user.as_deref(), Some("sibling"));
    assert_eq!(fresh.shell_distro.as_deref(), Some("Fixture-Distro"));
}

#[test]
fn metadata_freshness_coalesced_invalid_begin_never_becomes_complete() {
    for begin in ["", "00", "invalid", "0"] {
        let mut terminal = new_terminal();
        let mut processor = Processor::default();
        let mut content = new_content();
        // No renderer observation between writes: a previous serial alone
        // cannot establish that the previous marker's value was exactly 1.
        write(
            &mut processor,
            &mut terminal,
            "automexia_env_pending",
            begin,
        );
        fields(
            &mut processor,
            &mut terminal,
            &[
                ("automexia_shell", "1"),
                ("automexia_shell_name", "bash"),
                base()[0],
                base()[1],
            ],
        );
        write(&mut processor, &mut terminal, "automexia_env_pending", "0");
        sync_session_metadata(&mut content, &terminal);
        assert_eq!(
            content.session_metadata.readiness(),
            MetadataReadiness::Unavailable
        );
        assert!(content.shell_name.is_none());
        frame(&mut processor, &mut terminal, "bash", &base());
        sync_session_metadata(&mut content, &terminal);
        assert_eq!(
            content.session_metadata.readiness(),
            MetadataReadiness::Complete
        );
    }
}
#[test]
fn metadata_freshness_nested_begin_requires_fresh_fields_after_latest_begin() {
    let mut terminal = new_terminal();
    let mut processor = Processor::default();
    let mut content = new_content();
    write(&mut processor, &mut terminal, "automexia_env_pending", "1");
    fields(
        &mut processor,
        &mut terminal,
        &[
            ("automexia_shell", "1"),
            ("automexia_shell_name", "bash"),
            base()[0],
            base()[1],
        ],
    );
    write(&mut processor, &mut terminal, "automexia_env_pending", "1");
    write(&mut processor, &mut terminal, "automexia_env_pending", "0");
    sync_session_metadata(&mut content, &terminal);
    assert_eq!(
        content.session_metadata.readiness(),
        MetadataReadiness::Unavailable
    );
    assert!(content.shell_environment.is_empty());
}

#[test]
fn metadata_freshness_unchanged_frame_keeps_value_allocations() {
    let mut terminal = new_terminal();
    let mut processor = Processor::default();
    let mut content = new_content();
    frame(&mut processor, &mut terminal, "bash", &base());
    sync_session_metadata(&mut content, &terminal);
    let shell = content.shell_name.as_ref().unwrap().as_ptr();
    let home = content.shell_environment["HOME"].as_ptr();
    for _ in 0..128 {
        sync_session_metadata(&mut content, &terminal);
        assert_eq!(content.shell_name.as_ref().unwrap().as_ptr(), shell);
        assert_eq!(content.shell_environment["HOME"].as_ptr(), home);
    }
}

#[test]
fn metadata_readiness_pending_seed_is_empty_without_rewriting_source_snapshot() {
    let mut terminal = new_terminal();
    let mut processor = Processor::default();
    let mut content = new_content();
    frame(&mut processor, &mut terminal, "bash", &base());
    sync_session_metadata(&mut content, &terminal);
    write(&mut processor, &mut terminal, "automexia_env_pending", "1");
    sync_session_metadata(&mut content, &terminal);
    assert_eq!(content.shell_name.as_deref(), Some("bash"));
    let mut independent = new_content();
    independent.apply_session_metadata_seed(content.session_metadata_seed());
    assert!(independent.shell_name.is_none());
    assert!(!independent.shell_integration);
    assert!(independent.shell_environment.is_empty());
}
