use super::*;
use proptest::prelude::*;

proptest! {
    #[test]
    fn arbitrary_snapshot_bytes_never_bypass_validation(bytes in proptest::collection::vec(any::<u8>(), 0..4096)) {
        if let Ok(snapshot) = Snapshot::decode(&bytes) {
            prop_assert!(snapshot.validate());
            let encoded = snapshot.encode().unwrap();
            prop_assert_eq!(Snapshot::decode(&encoded).unwrap(), snapshot);
        }
    }

    #[test]
    fn hostile_tree_indexes_are_rejected_or_roundtrip(root in 0usize..256, focused in 0usize..256) {
        let mut snapshot = fixture();
        snapshot.windows[0].tabs[0].root = root;
        snapshot.windows[0].tabs[0].focused = focused;
        prop_assert_eq!(snapshot.validate(), root == 0 && focused == 0);
    }
}
pub(super) fn fixture() -> Snapshot {
    Snapshot {
        version: 1,
        windows: vec![Window {
            width: 1000,
            height: 700,
            position: None,
            active: 0,
            tabs: vec![Tab {
                title: Some("Work".into()),
                color: None,
                root: 0,
                focused: 0,
                nodes: vec![Node::Pane {
                    sessions: vec![Session {
                        history: None,
                        source: None,
                        profile: Profile::Configured,
                        cwd: None,
                        disconnected: false,
                    }],
                    active: 0,
                }],
            }],
        }],
    }
}
#[test]
fn topology_roundtrip_has_no_process_or_content_fields() {
    let snapshot = fixture();
    let bytes = snapshot.encode().unwrap();
    assert_eq!(Snapshot::decode(&bytes).unwrap(), snapshot);
    let json = String::from_utf8(bytes).unwrap();
    for key in [
        "argv",
        "environment",
        "password",
        "scrollback",
        "pid",
        "command",
        "credential",
    ] {
        assert!(!json.contains(key));
    }
}
#[test]
fn hostile_fields_versions_cycles_and_extents_are_rejected() {
    assert_eq!(
        Snapshot::decode(br#"{"version":2,"windows":[]}"#),
        Err(StoreError::Version)
    );
    assert!(Snapshot::decode(br#"{"version":1,"windows":[],"command":"bad"}"#).is_err());
    assert!(Snapshot::decode(&vec![b' '; MAX_BYTES + 1]).is_err());
    let mut snapshot = fixture();
    snapshot.windows[0].width = u32::MAX;
    assert!(!snapshot.validate());
    let mut snapshot = fixture();
    snapshot.windows[0].tabs[0].nodes.push(Node::Split {
        vertical: false,
        children: vec![0, 1],
        weights: vec![1, 1],
    });
    snapshot.windows[0].tabs[0].root = 1;
    assert!(!snapshot.validate());
}

#[test]
fn legacy_plaintext_migration_rejects_history_fields() {
    let mut legacy = serde_json::to_value(fixture()).unwrap();
    legacy["windows"][0]["tabs"][0]["nodes"][0]["sessions"][0]["history"] =
        serde_json::Value::Null;
    let bytes = serde_json::to_vec(&legacy).unwrap();
    assert_eq!(Checkpoint::decode(&bytes), Err(StoreError::Invalid));
}
#[test]
fn profiles_and_directories_cannot_be_commands_or_network_paths() {
    assert!(!interactive_args(&["-Command".into(), "anything".into()]));
    assert!(!interactive_args(&["-c".into(), "anything".into()]));
    assert!(interactive_args(&["-NoLogo".into()]));
    assert!(!safe_cwd("\\\\server\\share", false));
    assert!(!safe_cwd("//server/share", false));
    assert!(!safe_cwd("/tmp/\x1b]0;title", true));
    assert!(!Profile::Wsl {
        distribution: Some("Example".into())
    }
    .allowed(&["wsl".into()]));
    assert!(!Profile::Shell { shell: Shell::Cmd }.allowed(&["CMD".into()]));
}

#[test]
fn changed_opt_out_policy_prunes_old_snapshots_without_orphan_indexes() {
    let mut snapshot = fixture();
    snapshot.windows[0].tabs[0].nodes.push(Node::Pane {
        sessions: vec![Session {
            history: None,
            source: None,
            profile: Profile::Shell { shell: Shell::Bash },
            cwd: None,
            disconnected: true,
        }],
        active: 0,
    });
    snapshot.windows[0].tabs[0].nodes.push(Node::Split {
        vertical: true,
        children: vec![0, 1],
        weights: vec![3, 1],
    });
    snapshot.windows[0].tabs[0].root = 2;
    let filtered = snapshot.excluding(&["configured".into()]);
    assert!(filtered.validate());
    assert_eq!(filtered.session_count(), 1);
    assert_eq!(filtered.windows[0].tabs[0].nodes.len(), 1);
    assert!(filtered.excluding(&["bash".into()]).windows.is_empty());
}

#[test]
fn smart_prompt_distinguishes_idle_time_from_real_activity() {
    use rio_backend::crosswords::SessionActivity as Activity;
    assert!(!Checkpoint::from(fixture()).noteworthy());
    let mut many = fixture();
    let tab = many.windows[0].tabs[0].clone();
    many.windows[0].tabs.push(tab);
    assert!(Checkpoint::from(many).noteworthy());
    assert!(!significant_activity(Activity {
        age_ms: 86_400_000,
        ..Activity::default()
    }));
    assert!(!significant_activity(Activity {
        longest_command_ms: 59_999,
        ..Activity::default()
    }));
    assert!(significant_activity(Activity {
        longest_command_ms: 60_000,
        ..Activity::default()
    }));
    assert!(significant_activity(Activity {
        running_ms: 60_000,
        ..Activity::default()
    }));
    assert!(significant_activity(Activity {
        remote_used: true,
        ..Activity::default()
    }));
    assert!(significant_activity(Activity {
        age_ms: 900_000,
        completed_commands: 10,
        execution_ms: 120_000,
        ..Activity::default()
    }));
    assert!(!significant_activity(Activity {
        age_ms: 900_000,
        completed_commands: 10,
        execution_ms: 119_999,
        ..Activity::default()
    }));
}

#[test]
fn checkpoint_migrates_topology_without_accepting_future_envelopes() {
    let legacy = fixture();
    let checkpoint = Checkpoint::decode(&legacy.encode().unwrap()).unwrap();
    assert_eq!(checkpoint.snapshot, legacy);
    assert_eq!(checkpoint.version, 2);
    assert_eq!(
        Checkpoint::decode(&checkpoint.encode().unwrap()).unwrap(),
        checkpoint
    );
    assert_eq!(
        Checkpoint::decode(br#"{"version":999}"#),
        Err(StoreError::Version)
    );
}
