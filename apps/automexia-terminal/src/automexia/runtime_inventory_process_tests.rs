// Native process boundary for cached extension inventory and feature settings.
use crate::automexia::runtime::{self, InventoryInitialization, InventoryStatus};
use std::{process::Command, sync::mpsc, time::Duration};

const CHILD_MODE: &str = "AUTOMEXIA_CONTEXT_SETTINGS_TEST_MODE";

#[test]
fn isolated_context_settings_child() {
    let Ok(mode) = std::env::var(CHILD_MODE) else {
        return;
    };
    let expected_member = mode != "uninstalled";
    let desired = mode != "feature-disabled";
    assert_eq!(runtime::inventory_status(), InventoryStatus::Uninitialized);
    assert!(!runtime::output_highlighting_available());
    assert!(!runtime::context_status_enabled());
    assert_eq!(
        runtime::inventory_status(),
        InventoryStatus::Uninitialized,
        "cached getters must not initialize marker reads"
    );
    runtime::set_context_status_enabled(desired).unwrap();
    let caller = std::thread::current().id();
    let (sender, receiver) = mpsc::sync_channel(1);
    assert_eq!(
        runtime::schedule_initialize(Box::new(move || {
            let published = runtime::inventory_status() == InventoryStatus::Ready
                && runtime::output_highlighting_available() == expected_member
                && runtime::context_status_enabled() == (expected_member && desired);
            sender
                .send((published, std::thread::current().id() != caller))
                .unwrap();
        })),
        InventoryInitialization::Queued
    );
    assert_eq!(
        receiver.recv_timeout(Duration::from_secs(10)).unwrap(),
        (true, true),
        "inventory must publish before its background completion wake"
    );
    assert_eq!(
        runtime::schedule_initialize(Box::new(|| panic!(
            "ready inventory must not schedule another load"
        ))),
        InventoryInitialization::Ready
    );
    assert_eq!(runtime::is_installed("automexia.devops"), expected_member);
    if mode == "membership-cycle" {
        membership_cycle_rejects_old_request();
    } else if mode == "membership-overflow" {
        runtime::write_runtime().context_revision = u64::MAX;
        assert!(!runtime::toggle("automexia.devops").unwrap());
        assert!(!runtime::is_installed("automexia.devops"));
        assert_eq!(
            runtime::set_context_status_enabled(true),
            Err(runtime::ContextStatusError::RevisionExhausted),
            "exhausted removal must fail context closed without undoing uninstall"
        );
        assert!(runtime::toggle("automexia.devops").unwrap());
        assert!(!runtime::context_status_enabled());
        runtime::shutdown_background_services();
        return;
    }
    runtime::set_context_status_enabled(false).unwrap();
    assert!(!runtime::context_status_enabled());
    assert_eq!(
        runtime::is_installed("automexia.devops"),
        expected_member,
        "feature disable must preserve install membership"
    );
    runtime::set_context_status_enabled(true).unwrap();
    assert_eq!(
        runtime::context_status_enabled(),
        expected_member,
        "an enabled preference cannot install an absent extension"
    );
    runtime::shutdown_background_services();
    assert_eq!(runtime::inventory_status(), InventoryStatus::Unavailable);
    assert!(!runtime::context_status_enabled());
}

fn membership_cycle_rejects_old_request() {
    let capture = || {
        let facts = super::session(51, "fixture");
        let mut state = runtime::write_runtime();
        runtime::RefreshRequest {
            context_revision: state.context_revision,
            operation_id: super::OperationId::new(99),
            source_revision: runtime::source_revision(&facts),
            capsule_revision: state.capsule_revision(&facts),
            session: facts,
            cancellation: super::CancellationToken::default(),
            completion: None,
        }
    };
    let old = capture();
    assert!(!runtime::toggle("automexia.devops").unwrap());
    assert!(runtime::toggle("automexia.devops").unwrap());
    let current = capture();
    let mut state = runtime::write_runtime();
    assert!(state.register_refresh(
        current.session.session_id,
        current.operation_id,
        current.capsule_revision,
        current.context_revision,
        current.cancellation.clone(),
    ));
    assert!(state.accepts_refresh(&current));
    assert!(
        !state.register_refresh(
            old.session.session_id,
            old.operation_id,
            old.capsule_revision,
            old.context_revision,
            old.cancellation.clone(),
        ),
        "uninstall/reinstall must revoke the previous installation's queued request"
    );
    assert!(old.cancellation.is_cancelled());
    assert!(state.accepts_refresh(&current));
}

#[test]
fn membership_cycle_rejects_refresh_from_previous_installation() {
    run_inventory_mode("membership-cycle");
}

#[test]
fn membership_cycle_exhaustion_preserves_uninstall_and_fails_context_closed() {
    run_inventory_mode("membership-overflow");
}

#[test]
fn background_inventory_preserves_preference_and_marker_state_across_real_processes() {
    for mode in ["default", "feature-disabled", "uninstalled"] {
        run_inventory_mode(mode);
    }
}

fn run_inventory_mode(mode: &str) {
    let fixture = tempfile::tempdir().unwrap();
    let current = fixture.path().join("current");
    let legacy = fixture.path().join("legacy");
    std::fs::create_dir_all(&current).unwrap();
    std::fs::create_dir_all(&legacy).unwrap();
    if mode == "uninstalled" {
        let marker = current.join("extensions").join("automexia.devops");
        std::fs::create_dir_all(&marker).unwrap();
        std::fs::write(marker.join("disabled"), b"enabled=false\n").unwrap();
    }
    let output = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "automexia::runtime::tests::inventory_process::isolated_context_settings_child"])
        .env(CHILD_MODE, mode)
        .env("AUTOMEXIA_CONFIG_HOME", &current)
        .env("RIO_CONFIG_HOME", &legacy)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "isolated context settings case {mode} failed: {}",
        String::from_utf8_lossy(&output.stdout)
    );
}
