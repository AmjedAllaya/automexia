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
