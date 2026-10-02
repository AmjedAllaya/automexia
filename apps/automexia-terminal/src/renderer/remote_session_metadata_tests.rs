//! Real VT framing and renderer admission tests; remote strings stay display-only.
use super::sync_session_metadata;
use crate::context::renderable::{Cursor, RenderableContent};
use base64::Engine;
use rio_backend::crosswords::{Crosswords, CrosswordsSize};
use rio_backend::event::{VoidListener, WindowId};
use rio_backend::performer::handler::Processor;
use sha2::{Digest, Sha256};

fn terminal() -> Crosswords<VoidListener> {
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

fn begin(
    processor: &mut Processor,
    terminal: &mut Crosswords<VoidListener>,
    secret: u8,
    generation: u64,
    shell: &str,
) {
    let digest = Sha256::digest([secret; 32]);
    let digest = digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    write(
        processor,
        terminal,
        "terminal_scope_v1",
        &format!("AMXSCOPE1|begin|{digest}|1|{generation}|{shell}"),
    );
}

fn end(processor: &mut Processor, terminal: &mut Crosswords<VoidListener>, secret: u8) {
    write(
        processor,
        terminal,
        "terminal_scope_v1",
        &format!("AMXSCOPE1|end|{}", format!("{secret:02x}").repeat(32)),
    );
}

fn local(processor: &mut Processor, terminal: &mut Crosswords<VoidListener>) {
    terminal.current_directory = Some("/fixture/local".into());
    for (key, value) in [
        ("automexia_env_pending", "1"),
        ("automexia_shell", "1"),
        ("automexia_shell_name", "bash"),
        ("automexia_shell_user", "local-user"),
        ("automexia_distro", "Fixture-Linux"),
        ("automexia_env_HOME", "/fixture/home"),
        ("automexia_env_KUBECONFIG", ""),
        ("automexia_env_pending", "0"),
    ] {
        write(processor, terminal, key, value);
    }
}

#[test]
fn ssh_discovery_revision_clears_old_context_and_rejects_late_results() {
    use automexia_ssh_integration::{
        helper::{ContextField, ContextUpdate, Revision},
        session::RemoteContext,
        GenerationKey,
    };
    let key = GenerationKey::new(1, 2).unwrap();
    let (mut terminal, mut processor, mut content) = (
        terminal(),
        Processor::default(),
        RenderableContent::new(Cursor::default()),
    );
    begin(&mut processor, &mut terminal, 0, 2, "bash");
    ready(
        &mut processor,
        &mut terminal,
        2,
        "remote-user",
        "/fixture/one",
    );
    write(
        &mut processor,
        &mut terminal,
        "automexia_ssh_context",
        &RemoteContext::empty_value(key),
    );
    sync_session_metadata(&mut content, &terminal);
    assert!(content
        .remote_session
        .presentation()
        .unwrap()
        .context
        .is_some());
    write(
        &mut processor,
        &mut terminal,
        "automexia_ssh_revision",
        &Revision::new(key, 1).unwrap().value(),
    );
    sync_session_metadata(&mut content, &terminal);
    assert!(
        content
            .remote_session
            .presentation()
            .unwrap()
            .context
            .is_none(),
        "new discovery revision must invalidate previous context immediately"
    );
    let mut result = ContextUpdate::new(key, 1).unwrap();
    result
        .set(ContextField::GitBranch, "fixture-first")
        .unwrap();
    write(
        &mut processor,
        &mut terminal,
        "automexia_ssh_context_v2",
        &result.encode(),
    );
    sync_session_metadata(&mut content, &terminal);
    assert_eq!(
        content
            .remote_session
            .presentation()
            .unwrap()
            .context
            .as_ref()
            .unwrap()
            .value(automexia_ssh_integration::session::RemoteContextField::GitBranch),
        Some("fixture-first")
    );
    // An unchanged prompt replays the revision without hiding its valid result.
    write(
        &mut processor,
        &mut terminal,
        "automexia_ssh_revision",
        &Revision::new(key, 1).unwrap().value(),
    );
    sync_session_metadata(&mut content, &terminal);
    assert!(content
        .remote_session
        .presentation()
        .unwrap()
        .context
        .is_some());
    write(
        &mut processor,
        &mut terminal,
        "automexia_ssh_context_v2",
        "malformed",
    );
    sync_session_metadata(&mut content, &terminal);
    assert!(content
        .remote_session
        .presentation()
        .unwrap()
        .context
        .is_none());
    write(
        &mut processor,
        &mut terminal,
        "automexia_ssh_context_v2",
        &result.encode(),
    );
    sync_session_metadata(&mut content, &terminal);
    assert!(content
        .remote_session
        .presentation()
        .unwrap()
        .context
        .is_some());
    write(
        &mut processor,
        &mut terminal,
        "automexia_ssh_revision",
        &Revision::new(key, 2).unwrap().value(),
    );
    // Delayed old result after changing directory cannot resurrect old tags.
    write(
        &mut processor,
        &mut terminal,
        "automexia_ssh_context_v2",
        &result.encode(),
    );
    write(
        &mut processor,
        &mut terminal,
        "automexia_ssh_context",
        &RemoteContext::empty_value(key),
    );
    sync_session_metadata(&mut content, &terminal);
    assert!(content
        .remote_session
        .presentation()
        .unwrap()
        .context
        .is_none());
    let mut current = ContextUpdate::new(key, 2).unwrap();
    current
        .set(ContextField::GitBranch, "fixture-second")
        .unwrap();
    write(
        &mut processor,
        &mut terminal,
        "automexia_ssh_context_v2",
        &current.encode(),
    );
    sync_session_metadata(&mut content, &terminal);
    assert!(content
        .remote_session
        .presentation()
        .unwrap()
        .context
        .is_some());
    // Exhaustion/IPC failure revokes v2, without falling back to old v1 hints.
    write(&mut processor, &mut terminal, "automexia_ssh_revision", "");
    write(
        &mut processor,
        &mut terminal,
        "automexia_ssh_context_v2",
        &current.encode(),
    );
    sync_session_metadata(&mut content, &terminal);
    assert!(content
        .remote_session
        .presentation()
        .unwrap()
        .context
        .is_none());
}

#[test]
fn ssh_discovery_revocation_survives_batched_replays_until_fresh_context() {
    use automexia_ssh_integration::{
        helper::{ContextUpdate, Revision},
        GenerationKey,
    };
    let key = GenerationKey::new(1, 2).unwrap();
    for (batched, invalid) in [false, true].into_iter().flat_map(|batched| {
        [None, Some("malformed"), Some("AMXSSHREV1|1|99|1")]
            .map(|invalid| (batched, invalid))
    }) {
        let (mut terminal, mut processor, mut content) = (
            terminal(),
            Processor::default(),
            RenderableContent::new(Cursor::default()),
        );
        begin(&mut processor, &mut terminal, 0, 2, "bash");
        ready(&mut processor, &mut terminal, 2, "remote-user", "/fixture");
        let revision = Revision::new(key, 1).unwrap().value();
        let context = ContextUpdate::new(key, 1).unwrap().encode();
        write(
            &mut processor,
            &mut terminal,
            "automexia_ssh_revision",
            &revision,
        );
        write(
            &mut processor,
            &mut terminal,
            "automexia_ssh_context_v2",
            &context,
        );
        // A normal replay does not invalidate an already fresh result.
        write(
            &mut processor,
            &mut terminal,
            "automexia_ssh_revision",
            &revision,
        );
        sync_session_metadata(&mut content, &terminal);
        assert!(content
            .remote_session
            .presentation()
            .unwrap()
            .discovered
            .is_some());
        write(&mut processor, &mut terminal, "automexia_ssh_revision", "");
        if !batched {
            sync_session_metadata(&mut content, &terminal);
        }
        if let Some(invalid) = invalid {
            write(
                &mut processor,
                &mut terminal,
                "automexia_ssh_revision",
                invalid,
            );
        }
        // An already queued old result, then ordinary prompt replay, cannot
        // revive facts revoked while the renderer was between snapshots.
        write(
            &mut processor,
            &mut terminal,
            "automexia_ssh_context_v2",
            &context,
        );
        write(
            &mut processor,
            &mut terminal,
            "automexia_ssh_revision",
            &revision,
        );
        write(
            &mut processor,
            &mut terminal,
            "automexia_ssh_revision",
            &revision,
        );
        sync_session_metadata(&mut content, &terminal);
        assert!(
            content
                .remote_session
                .presentation()
                .unwrap()
                .discovered
                .is_none(),
            "revoked context resurrected; batched={batched}"
        );
        write(
            &mut processor,
            &mut terminal,
            "automexia_ssh_context_v2",
            &context,
        );
        // Fresh recovery and ordinary replay may share one renderer snapshot.
        write(
            &mut processor,
            &mut terminal,
            "automexia_ssh_revision",
            &revision,
        );
        sync_session_metadata(&mut content, &terminal);
        assert!(content
            .remote_session
            .presentation()
            .unwrap()
            .discovered
            .is_some());
        write(
            &mut processor,
            &mut terminal,
            "automexia_ssh_revision",
            &revision,
        );
        sync_session_metadata(&mut content, &terminal);
        assert!(content
            .remote_session
            .presentation()
            .unwrap()
            .discovered
            .is_some());
    }
}

#[test]
fn ssh_discovery_maximum_survives_coalescing_and_nested_return() {
    use automexia_ssh_integration::{
        helper::{ContextUpdate, Revision},
        GenerationKey,
    };
    let key = GenerationKey::new(1, 2).unwrap();
    for nested in [false, true] {
        let (mut terminal, mut processor, mut content) = (
            terminal(),
            Processor::default(),
            RenderableContent::new(Cursor::default()),
        );
        begin(&mut processor, &mut terminal, 0, 2, "bash");
        ready(&mut processor, &mut terminal, 2, "remote-user", "/fixture");
        for revision in [2, 1] {
            write(
                &mut processor,
                &mut terminal,
                "automexia_ssh_revision",
                &Revision::new(key, revision).unwrap().value(),
            );
            write(
                &mut processor,
                &mut terminal,
                "automexia_ssh_context_v2",
                &ContextUpdate::new(key, revision).unwrap().encode(),
            );
            if nested {
                sync_session_metadata(&mut content, &terminal);
            }
        }
        if nested {
            begin(&mut processor, &mut terminal, 1, 3, "bash");
            sync_session_metadata(&mut content, &terminal);
            end(&mut processor, &mut terminal, 1);
        }
        sync_session_metadata(&mut content, &terminal);
        assert!(
            content
                .remote_session
                .presentation()
                .unwrap()
                .discovered
                .is_none(),
            "regressed revision admitted; nested={nested}"
        );
        write(
            &mut processor,
            &mut terminal,
            "automexia_ssh_revision",
            &Revision::new(key, 3).unwrap().value(),
        );
        write(
            &mut processor,
            &mut terminal,
            "automexia_ssh_context_v2",
            &ContextUpdate::new(key, 3).unwrap().encode(),
        );
        sync_session_metadata(&mut content, &terminal);
        assert_eq!(
            content
                .remote_session
                .presentation()
                .unwrap()
                .discovered
                .as_ref()
                .unwrap()
                .revision(),
            3
        );
    }
}

fn ready(
    processor: &mut Processor,
    terminal: &mut Crosswords<VoidListener>,
    generation: u64,
    user: &str,
    directory: &str,
) {
    for (key, value) in [
        ("automexia_ssh_ready", format!("AMXSSH1|1|{generation}|1|7")),
        (
            "automexia_ssh_cwd",
            format!("AMXSSHCWD1|1|{generation}|{directory}"),
        ),
        (
            "automexia_ssh_user",
            format!("AMXSSHUSER1|1|{generation}|{user}"),
        ),
    ] {
        write(processor, terminal, key, &value);
    }
}

#[test]
fn ssh_scope_separates_remote_display_from_local_paths_environment_and_clone_seeds() {
    let (mut terminal, mut processor, mut content) = (
        terminal(),
        Processor::default(),
        RenderableContent::new(Cursor::default()),
    );
    local(&mut processor, &mut terminal);
    sync_session_metadata(&mut content, &terminal);
    assert_eq!(content.shell_user.as_deref(), Some("local-user"));
    begin(&mut processor, &mut terminal, 0, 2, "bash");
    ready(
        &mut processor,
        &mut terminal,
        2,
        "remote-user",
        "/fixture/remote",
    );
    // A remote process can emit local-looking integration frames. They must
    // never regain provider or launch authority while the scope is active.
    local(&mut processor, &mut terminal);
    processor.advance(
        &mut terminal,
        b"\x1b]7;file://localhost/fixture/injected\x07",
    );
    sync_session_metadata(&mut content, &terminal);
    assert!(content.remote_session.active());
    assert!(content.session_context_rejected);
    assert!(content.current_directory.is_none());
    assert!(content.shell_environment.is_empty());
    assert!(content.shell_distro.is_none());
    assert!(content.shell_name.is_none());
    let remote = content.remote_session.presentation().unwrap();
    assert!(remote.ready);
    assert_eq!(remote.user.as_deref(), Some("remote-user"));
    assert_eq!(
        remote.directory.as_ref().unwrap().remote_text(),
        "/fixture/remote"
    );
    let mut independent = RenderableContent::new(Cursor::default());
    independent.apply_session_metadata_seed(content.session_metadata_seed());
    assert!(independent.current_directory.is_none());
    assert!(independent.shell_environment.is_empty());
    assert!(independent.shell_user.is_none());
    end(&mut processor, &mut terminal, 0);
    sync_session_metadata(&mut content, &terminal);
    assert!(!content.remote_session.active());
    assert!(!content.session_context_rejected);
    assert_eq!(content.shell_user.as_deref(), Some("local-user"));
    assert_eq!(
        content.current_directory.as_deref(),
        Some(std::path::Path::new("/fixture/local"))
    );
    assert_eq!(
        content.shell_environment.get("HOME").map(String::as_str),
        Some("/fixture/home")
    );
}

#[test]
fn ssh_scope_nested_return_restores_each_owned_remote_snapshot_and_local_context() {
    let (mut terminal, mut processor, mut content) = (
        terminal(),
        Processor::default(),
        RenderableContent::new(Cursor::default()),
    );
    local(&mut processor, &mut terminal);
    begin(&mut processor, &mut terminal, 0, 2, "bash");
    ready(&mut processor, &mut terminal, 2, "outer", "/fixture/outer");
    sync_session_metadata(&mut content, &terminal);
    begin(&mut processor, &mut terminal, 1, 3, "powershell");
    ready(
        &mut processor,
        &mut terminal,
        3,
        "inner",
        "C:\\fixture\\inner",
    );
    sync_session_metadata(&mut content, &terminal);
    let inner = content.remote_session.presentation().unwrap();
    assert_eq!(inner.user.as_deref(), Some("inner"));
    assert_eq!(
        inner.directory.as_ref().unwrap().remote_text(),
        "C:\\fixture\\inner"
    );
    end(&mut processor, &mut terminal, 2); // Wrong local preimage cannot leave.
    sync_session_metadata(&mut content, &terminal);
    assert_eq!(
        content
            .remote_session
            .presentation()
            .unwrap()
            .user
            .as_deref(),
        Some("inner")
    );
    end(&mut processor, &mut terminal, 1);
    sync_session_metadata(&mut content, &terminal);
    let outer = content.remote_session.presentation().unwrap();
    assert_eq!(outer.user.as_deref(), Some("outer"));
    assert_eq!(
        outer.directory.as_ref().unwrap().remote_text(),
        "/fixture/outer"
    );
    end(&mut processor, &mut terminal, 0);
    sync_session_metadata(&mut content, &terminal);
    assert_eq!(content.shell_user.as_deref(), Some("local-user"));
    assert!(content.remote_session.presentation().is_none());
}

#[test]
fn ssh_scope_rejects_stale_or_invalid_display_values_and_preserves_pane_isolation() {
    let (mut terminal, mut processor, mut content) = (
        terminal(),
        Processor::default(),
        RenderableContent::new(Cursor::default()),
    );
    let (mut other, mut other_processor, mut other_content) = (
        self::terminal(),
        Processor::default(),
        RenderableContent::new(Cursor::default()),
    );
    local(&mut other_processor, &mut other);
    begin(&mut processor, &mut terminal, 0, 2, "fish");
    ready(&mut processor, &mut terminal, 1, "stale", "/fixture/stale");
    sync_session_metadata(&mut content, &terminal);
    assert!(!content.remote_session.presentation().unwrap().ready);
    ready(
        &mut processor,
        &mut terminal,
        2,
        "remote",
        "/fixture/current",
    );
    sync_session_metadata(&mut content, &terminal);
    assert!(content.remote_session.presentation().unwrap().ready);
    for user in ["   ".to_owned(), "unsafe\u{202e}".into(), "x".repeat(257)] {
        write(
            &mut processor,
            &mut terminal,
            "automexia_ssh_user",
            &format!("AMXSSHUSER1|1|2|{user}"),
        );
        sync_session_metadata(&mut content, &terminal);
        assert!(content
            .remote_session
            .presentation()
            .unwrap()
            .user
            .is_none());
    }
    write(
        &mut processor,
        &mut terminal,
        "automexia_ssh_cwd",
        "AMXSSHCWD1|1|1|/fixture/stale",
    );
    sync_session_metadata(&mut content, &terminal);
    assert!(content
        .remote_session
        .presentation()
        .unwrap()
        .directory
        .is_none());
    sync_session_metadata(&mut other_content, &other);
    assert!(!other_content.remote_session.active());
    assert_eq!(other_content.shell_user.as_deref(), Some("local-user"));
    assert_eq!(other_content.shell_distro.as_deref(), Some("Fixture-Linux"));
}

#[test]
fn ssh_scope_without_ready_or_after_reset_never_lends_local_metadata() {
    for control in ["malformed", "AMXSCOPE1|begin|invalid|1|2|bash"] {
        let (mut terminal, mut processor, mut content) = (
            terminal(),
            Processor::default(),
            RenderableContent::new(Cursor::default()),
        );
        local(&mut processor, &mut terminal);
        write(&mut processor, &mut terminal, "terminal_scope_v1", control);
        processor.advance(&mut terminal, b"\x1bc");
        local(&mut processor, &mut terminal);
        sync_session_metadata(&mut content, &terminal);
        assert!(content.remote_session.active());
        assert!(content.remote_session.presentation().is_none());
        assert!(content.shell_environment.is_empty());
        assert!(content.current_directory.is_none());
    }
}

#[test]
fn ssh_scope_clone_caller_requires_explicit_new_connection() {
    let mut manager = crate::context::ContextManager::start_with_capacity(
        4,
        VoidListener {},
        WindowId::from(0),
    )
    .unwrap();
    {
        let source = manager.current_mut();
        let mut terminal = source.terminal.lock();
        begin(&mut Processor::default(), &mut terminal, 0, 2, "zsh");
        // Clone actions can arrive before the first renderer synchronization.
    }
    let error = manager.test_create_cloned_context(0).err().unwrap();
    assert!(error.contains("connect explicitly"));
    assert_eq!(
        manager.tab_profile_identity(0).as_deref(),
        Some("SSH · zsh")
    );
    let source = manager.current();
    assert_eq!(
        crate::context::title::update_title("{{absolute_path}}", source),
        "SSH · zsh"
    );
    assert_eq!(
        crate::context::title::create_title_extra_from_context(source)
            .unwrap()
            .program,
        "SSH · zsh"
    );
}

#[test]
fn ssh_scope_unknown_native_shell_never_claims_adapter_or_local_metadata() {
    let (mut terminal, mut processor, mut content) = (
        terminal(),
        Processor::default(),
        RenderableContent::new(Cursor::default()),
    );
    local(&mut processor, &mut terminal);
    begin(&mut processor, &mut terminal, 0, 2, "unknown");
    ready(
        &mut processor,
        &mut terminal,
        2,
        "remote",
        "/fixture/remote",
    );
    // Even installed remote integration with plausible local frame fields
    // cannot promote native fallback metadata or imply adapter capabilities.
    local(&mut processor, &mut terminal);
    sync_session_metadata(&mut content, &terminal);
    assert!(content.remote_session.active());
    assert!(content.remote_session.presentation().is_none());
    assert!(!content.shell_integration);
    assert!(content.current_directory.is_none());
    assert!(content.shell_environment.is_empty());
    end(&mut processor, &mut terminal, 0);
    sync_session_metadata(&mut content, &terminal);
    assert_eq!(content.shell_user.as_deref(), Some("local-user"));

    let mut manager = crate::context::ContextManager::start_with_capacity(
        1,
        VoidListener {},
        WindowId::from(0),
    )
    .unwrap();
    begin(
        &mut Processor::default(),
        &mut manager.current_mut().terminal.lock(),
        0,
        2,
        "unknown",
    );
    assert_eq!(manager.tab_profile_identity(0).as_deref(), Some("SSH"));
    assert_eq!(
        crate::context::title::update_title("{{absolute_path}}", manager.current()),
        "SSH"
    );
}

#[test]
fn ssh_scope_context_snapshot_updates_clear_and_never_become_local_environment() {
    use automexia_ssh_integration::session::RemoteContextField;
    let (mut terminal, mut processor, mut content) = (
        terminal(),
        Processor::default(),
        RenderableContent::new(Cursor::default()),
    );
    begin(&mut processor, &mut terminal, 0, 2, "bash");
    ready(
        &mut processor,
        &mut terminal,
        2,
        "remote",
        "/fixture/remote",
    );
    let context = "AMXSSHCTX1|1|2|\ngit_branch=feature-remote\nkubernetes_context=fixture-cluster\nkubernetes_namespace=fixture-ns\ndocker_context=fixture-docker\nterraform_workspace=fixture-workspace\nenvironment=staging\naws_profile=fixture-aws\nazure_cloud=fixture-azure\ngcp_project=fixture-gcp\n";
    write(
        &mut processor,
        &mut terminal,
        "automexia_ssh_context",
        context,
    );
    sync_session_metadata(&mut content, &terminal);
    assert_eq!(
        content
            .remote_session
            .presentation()
            .unwrap()
            .context
            .as_ref()
            .unwrap()
            .value(RemoteContextField::GitBranch),
        Some("feature-remote")
    );
    assert!(content.shell_environment.is_empty());
    for replacement in [
        context.replace("1|2|", "1|1|"),
        context.replace("git_branch=feature-remote", "unknown=value"),
    ] {
        write(
            &mut processor,
            &mut terminal,
            "automexia_ssh_context",
            &replacement,
        );
        sync_session_metadata(&mut content, &terminal);
        assert!(content
            .remote_session
            .presentation()
            .unwrap()
            .context
            .is_none());
    }
}

#[test]
fn ssh_unscoped_receipts_cannot_replace_local_context_and_unchanged_remote_frames_are_cached(
) {
    let (mut terminal, mut processor, mut content) = (
        terminal(),
        Processor::default(),
        RenderableContent::new(Cursor::default()),
    );
    local(&mut processor, &mut terminal);
    ready(
        &mut processor,
        &mut terminal,
        2,
        "private-remote-user",
        "/fixture/private-remote",
    );
    sync_session_metadata(&mut content, &terminal);
    assert!(content.remote_session.presentation().is_none());
    assert_eq!(content.shell_user.as_deref(), Some("local-user"));
    begin(&mut processor, &mut terminal, 0, 2, "bash");
    ready(
        &mut processor,
        &mut terminal,
        2,
        "private-remote-user",
        "/fixture/private-remote",
    );
    sync_session_metadata(&mut content, &terminal);
    let remote = content.remote_session.presentation().unwrap();
    assert!(!format!("{remote:?}").contains("private-remote"));
    let allocation = remote.user.as_ref().unwrap().as_ptr();
    for _ in 0..32 {
        sync_session_metadata(&mut content, &terminal);
        assert_eq!(
            content
                .remote_session
                .presentation()
                .unwrap()
                .user
                .as_ref()
                .unwrap()
                .as_ptr(),
            allocation
        );
    }
}

#[test]
fn ssh_scope_vt_rejected_update_clears_stale_facts_until_fresh_bounded_updates() {
    let (mut terminal, mut processor, mut content) = (
        terminal(),
        Processor::default(),
        RenderableContent::new(Cursor::default()),
    );
    begin(&mut processor, &mut terminal, 0, 2, "bash");
    ready(&mut processor, &mut terminal, 2, "old-user", "/fixture/old");
    sync_session_metadata(&mut content, &terminal);
    assert!(content
        .remote_session
        .presentation()
        .unwrap()
        .user
        .is_some());
    write(
        &mut processor,
        &mut terminal,
        "automexia_ssh_user",
        &"x".repeat(8193),
    );
    assert!(terminal.last_user_var_rejection().is_some());
    sync_session_metadata(&mut content, &terminal);
    let remote = content.remote_session.presentation().unwrap();
    assert!(remote.ready);
    assert!(remote.user.is_none());
    assert!(remote.directory.is_none());
    write(
        &mut processor,
        &mut terminal,
        "automexia_ssh_user",
        "AMXSSHUSER1|1|2|fresh-user",
    );
    sync_session_metadata(&mut content, &terminal);
    let remote = content.remote_session.presentation().unwrap();
    assert_eq!(remote.user.as_deref(), Some("fresh-user"));
    assert!(remote.directory.is_none());
}
