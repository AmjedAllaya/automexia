use automexia_ssh_integration::{
    helper::{
        ContextField, ContextUpdate, DiscoveryRequest, Revision, UploadManifest,
        UploadReceipt,
    },
    GenerationKey, Invocation,
};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use std::{collections::BTreeMap, ffi::OsString};

fn key() -> GenerationKey {
    GenerationKey::new(3, 7).unwrap()
}

#[test]
fn discovery_snapshot_roundtrips_and_rejects_partial_or_unknown_fields() {
    let request =
        DiscoveryRequest::new(key(), 1, "/fixture/work", BTreeMap::new()).unwrap();
    let frame = request.encode();
    assert_eq!(frame.first(), Some(&0));
    assert_eq!(frame.last(), Some(&0));
    assert_eq!(
        DiscoveryRequest::decode(key(), &frame).unwrap(),
        Some(request)
    );
    let text = String::from_utf8(frame).unwrap();
    for invalid in [
        text.replace("HOME=\n", ""),
        text.replace("HOME=\n", "SECRET=secret\n"),
        text.replace("HOME=\n", "HOME=a\nHOME=b\n"),
    ] {
        assert!(DiscoveryRequest::decode(key(), invalid.as_bytes()).is_err());
    }
}

#[test]
fn discovery_scope_revision_and_budget_are_strict() {
    assert!(DiscoveryRequest::new(key(), 0, "/fixture", BTreeMap::new()).is_err());
    assert!(
        DiscoveryRequest::new(key(), 1, "/fixture\nforged", BTreeMap::new()).is_err()
    );
    let mut environment = BTreeMap::new();
    environment.insert("DOCKER_HOST_PRESENT".into(), "tcp://endpoint".into());
    assert!(DiscoveryRequest::new(key(), 1, "/fixture", environment).is_err());
    let request =
        DiscoveryRequest::new(key(), u32::MAX, "/fixture", BTreeMap::new()).unwrap();
    assert_eq!(
        DiscoveryRequest::decode(GenerationKey::new(3, 8).unwrap(), &request.encode())
            .unwrap(),
        None
    );
    assert!(DiscoveryRequest::decode(key(), &vec![b'a'; 16385]).is_err());
    assert_eq!(
        Revision::decode(key(), &Revision::new(key(), 9).unwrap().value())
            .unwrap()
            .unwrap()
            .revision(),
        9
    );
}

#[test]
fn discovery_revocation_and_explicit_empty_snapshot_do_not_reuse_inputs() {
    assert_eq!(Revision::decode(key(), "").unwrap(), None);
    let clear = DiscoveryRequest::new(key(), 12, "", BTreeMap::new()).unwrap();
    assert!(clear.environment().values().all(String::is_empty));
    assert_eq!(
        DiscoveryRequest::decode(key(), &clear.encode()).unwrap(),
        Some(clear)
    );
    assert!(Revision::decode(key(), "AMXSSHREV1|3|7|4294967296").is_err());
    assert!(Revision::decode(key(), "AMXSSHREV1|3|7|01").is_err());
}

#[test]
fn discovery_records_reject_binary_injection_and_total_budget_exhaustion() {
    let request = DiscoveryRequest::new(key(), 1, "/fixture", BTreeMap::new()).unwrap();
    let mut binary = request.encode();
    binary[20] = 0xff;
    assert!(DiscoveryRequest::decode(key(), &binary).is_err());
    let mut environment = BTreeMap::new();
    for name in ["HOME", "KUBECONFIG", "HOMEDRIVE", "HOMEPATH", "USERPROFILE"] {
        environment.insert(name.into(), "p".repeat(4096));
    }
    assert!(DiscoveryRequest::new(key(), 1, "/fixture", environment).is_err());
    let mut context =
        ContextUpdate::new(GenerationKey::new(u64::MAX, u64::MAX).unwrap(), u32::MAX)
            .unwrap();
    for field in ContextField::ALL {
        if field != ContextField::Production {
            context.set(field, &"x".repeat(256)).unwrap();
        }
    }
    assert!(
        context.encode().len()
            <= automexia_ssh_integration::helper::MAX_DISCOVERY_CONTEXT_BYTES
    );
}

#[test]
fn helper_startup_uses_owned_filenames_and_core_before_the_bundled_hook() {
    use automexia_ssh_integration::{bootstrap, RemoteShell};
    for shell in [
        RemoteShell::Bash,
        RemoteShell::Zsh,
        RemoteShell::PowerShell,
        RemoteShell::Pwsh,
    ] {
        let files = bootstrap::helper_shell_files(
            shell,
            key(),
            "# fixture-hook @@PANE@@ @@GENERATION@@",
        )
        .unwrap();
        assert!(!files.is_empty());
        for (name, source) in files {
            assert!(!name.contains('/') && !name.contains('\\'));
            assert!(!source.contains("@@PANE@@") && !source.contains("@@GENERATION@@"));
            if name != ".zshenv" {
                assert!(
                    source.find("AMXSSH").unwrap_or(0)
                        < source.find("# fixture-hook 3 7").unwrap()
                );
                assert!(!source.contains("rmdir --"));
            }
        }
    }
    assert!(bootstrap::helper_shell_files(RemoteShell::Unknown, key(), "").is_err());
    assert!(bootstrap::helper_shell_files(RemoteShell::Fish, key(), "").is_err());
    assert!(bootstrap::interactive_candidate(RemoteShell::Fish, key()).is_ok());
    assert!(
        bootstrap::helper_shell_files(RemoteShell::Bash, key(), "bad\0source").is_err()
    );
    assert!(bootstrap::helper_shell_files(
        RemoteShell::Bash,
        key(),
        &"x".repeat(16 * 1024 + 1)
    )
    .is_err());
}

#[test]
fn discovery_context_is_versioned_stale_safe_and_keeps_existing_projection() {
    let mut context = ContextUpdate::new(key(), 2).unwrap();
    context
        .set(ContextField::GitBranch, "fixture-branch")
        .unwrap();
    context
        .set(ContextField::AwsRegion, "fixture-region")
        .unwrap();
    context.set(ContextField::Production, "1").unwrap();
    assert_eq!(
        ContextUpdate::decode(key(), 2, &context.encode()).unwrap(),
        Some(context.clone())
    );
    assert_eq!(
        ContextUpdate::decode(key(), 3, &context.encode()).unwrap(),
        None
    );
    assert_eq!(
        context
            .base_context()
            .value(automexia_ssh_integration::session::RemoteContextField::GitBranch),
        Some("fixture-branch")
    );
    assert!(context.set(ContextField::Production, "yes").is_err());
    assert!(context.set(ContextField::AwsRegion, "x\u{202e}y").is_err());
    assert!(!format!("{context:?}").contains("fixture-branch"));
}

#[test]
fn upload_manifest_and_receipt_bind_every_transfer_field() {
    let nonce = "a".repeat(64);
    let digest = "b".repeat(64);
    let manifest = UploadManifest::new(key(), &nonce, 12, &digest).unwrap();
    let path = format!("/tmp/automexia-ssh.{nonce}");
    let encoded = STANDARD.encode(&path);
    let receipt = format!("AMXSSHUPLOAD1|3|7|{nonce}|12|{digest}|{encoded}\n");
    let lease = UploadReceipt::decode(&manifest, receipt.as_bytes()).unwrap();
    assert_eq!(lease.directory(), path);
    for invalid in [
        receipt.replace("|12|", "|13|"),
        receipt.replace("|3|7|", "|3|8|"),
        format!("banner\n{receipt}"),
        receipt.replace(&encoded, "Li4vZXNjYXBl"),
    ] {
        assert!(UploadReceipt::decode(&manifest, invalid.as_bytes()).is_err());
    }
    assert!(UploadManifest::new(key(), &nonce, 0, &digest).is_err());
    assert!(UploadManifest::new(key(), &nonce, 64 * 1024 * 1024 + 1, &digest).is_err());
    assert!(!format!("{lease:?}").contains("fixture"));
}

#[test]
fn upload_stage_disables_side_effects_without_consuming_option_like_values() {
    let args = [
        "-vAXt",
        "-i",
        "-A",
        "-L",
        "127.0.0.1:8080:example:80",
        "-oPermitLocalCommand=yes",
        "--",
        "fixture-host",
    ];
    let invocation =
        Invocation::new(args.into_iter().map(OsString::from).collect()).unwrap();
    let staged = invocation.upload_stage_arguments().unwrap();
    assert_eq!(invocation.arguments(), args.map(OsString::from));
    assert!(staged
        .windows(2)
        .any(|pair| pair == [OsString::from("-i"), OsString::from("-A")]));
    assert!(!staged.iter().any(|arg| arg == "-L" || arg == "-vAXt"));
    assert_eq!(staged.last().unwrap(), "fixture-host");
    assert!(staged.iter().any(|arg| arg == "PermitLocalCommand=no"));
}

#[test]
fn helper_transfer_commands_fit_native_budgets_at_maximum_scope_and_payload() {
    use automexia_ssh_integration::{bootstrap, RemoteShell};
    let key = GenerationKey::new(u64::MAX, u64::MAX).unwrap();
    let manifest = UploadManifest::new(
        key,
        &"a".repeat(64),
        automexia_ssh_integration::helper::MAX_UPLOAD_BYTES,
        &"b".repeat(64),
    )
    .unwrap();
    for shell in [
        RemoteShell::Bash,
        RemoteShell::Zsh,
        RemoteShell::PowerShell,
        RemoteShell::Pwsh,
    ] {
        let command = bootstrap::upload_stage_candidate(shell, &manifest).unwrap();
        let windows = matches!(shell, RemoteShell::PowerShell | RemoteShell::Pwsh);
        assert!(command.len() <= if windows { 8191 } else { 12 * 1024 });
        assert!(!command.contains("@@"));
        let directory = if windows {
            format!("C:\\Fixture\\{}", manifest.directory_name())
        } else {
            format!("/tmp/{}", manifest.directory_name())
        };
        let receipt = format!(
            "AMXSSHUPLOAD1|{}|{}|{}|{}|{}|{}\n",
            key.pane(),
            key.generation(),
            manifest.nonce(),
            manifest.size(),
            manifest.sha256(),
            STANDARD.encode(directory)
        );
        let receipt = UploadReceipt::decode(&manifest, receipt.as_bytes()).unwrap();
        assert!(
            bootstrap::uploaded_session_candidate(RemoteShell::Fish, &receipt).is_err()
        );
        assert!(
            bootstrap::uploaded_session_candidate(shell, &receipt)
                .unwrap()
                .len()
                <= if windows { 8191 } else { 12 * 1024 }
        );
    }
    assert!(bootstrap::upload_stage_candidate(RemoteShell::Unknown, &manifest).is_err());
    assert!(bootstrap::upload_stage_candidate(RemoteShell::Fish, &manifest).is_err());
}
