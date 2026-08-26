//! Deterministic visual fixtures compiled only for controlled GUI assurance.
//!
//! Product builds do not contain this module. The hook accepts one exact
//! fixture identifier and exposes only fixed public test data; it never reads a
//! fixture file, terminal contents, user paths, credentials, or provider state.

use super::builtins::devops::{CloudContext, DevOpsSnapshot, KubernetesContext};

pub const FIXTURE_ENV: &str = "AUTOMEXIA_VISUAL_TEST_FIXTURE";
pub const FIXTURE_ID: &str = "s1-standard-v1";
pub const FROZEN_CLOCK: &str = "12:34";
pub const FROZEN_COMMAND_DATETIME: &str = "2026-08-26 12:34:56";

#[inline]
pub fn fixture_active() -> bool {
    std::env::var_os(FIXTURE_ENV).is_some_and(|value| value == FIXTURE_ID)
}

#[inline]
pub fn frozen_clock_label() -> Option<&'static str> {
    fixture_active().then_some(FROZEN_CLOCK)
}

#[inline]
pub fn frozen_command_datetime_label() -> Option<&'static str> {
    fixture_active().then_some(FROZEN_COMMAND_DATETIME)
}

#[inline]
pub fn animations_enabled() -> bool {
    !fixture_active()
}

/// Fixed, non-secret status facts used to stabilize controlled frame captures.
pub fn visual_test_snapshot() -> Option<DevOpsSnapshot> {
    fixture_active().then(|| DevOpsSnapshot {
        kubernetes: Some(KubernetesContext {
            context: "automexia-lab".to_owned(),
            namespace: "platform".to_owned(),
        }),
        docker: Some("local".to_owned()),
        clouds: vec![CloudContext {
            provider: "aws",
            profile: "development".to_owned(),
            region: "eu-west-1".to_owned(),
        }],
        terraform: Some("workspace".to_owned()),
        git_branch: Some("main".to_owned()),
        user: Some("automexia".to_owned()),
        environment: Some("demo".to_owned()),
        production: false,
        ..DevOpsSnapshot::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, OnceLock};

    fn environment_lock() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    #[test]
    fn exact_fixture_freezes_volatile_state_and_uses_public_facts() {
        let _guard = environment_lock();
        let previous = std::env::var_os(FIXTURE_ENV);
        // SAFETY: this test serializes access to the process variable and
        // restores its prior value before releasing the lock.
        unsafe { std::env::set_var(FIXTURE_ENV, FIXTURE_ID) };
        assert_eq!(frozen_clock_label(), Some("12:34"));
        assert_eq!(frozen_command_datetime_label(), Some("2026-08-26 12:34:56"));
        assert!(!animations_enabled());
        let snapshot = visual_test_snapshot().unwrap();
        assert_eq!(snapshot.environment.as_deref(), Some("demo"));
        assert_eq!(snapshot.clouds[0].provider, "aws");

        match previous {
            Some(value) => {
                // SAFETY: guarded and restored as described above.
                unsafe { std::env::set_var(FIXTURE_ENV, value) };
            }
            None => {
                // SAFETY: guarded and restored as described above.
                unsafe { std::env::remove_var(FIXTURE_ENV) };
            }
        }
    }

    #[test]
    fn unknown_fixture_cannot_inject_arbitrary_state() {
        let _guard = environment_lock();
        let previous = std::env::var_os(FIXTURE_ENV);
        // SAFETY: this test serializes and restores the process variable.
        unsafe { std::env::set_var(FIXTURE_ENV, "unreviewed") };
        assert_eq!(frozen_clock_label(), None);
        assert_eq!(frozen_command_datetime_label(), None);
        assert!(animations_enabled());
        assert_eq!(visual_test_snapshot(), None);
        match previous {
            Some(value) => {
                // SAFETY: guarded and restored as described above.
                unsafe { std::env::set_var(FIXTURE_ENV, value) };
            }
            None => {
                // SAFETY: guarded and restored as described above.
                unsafe { std::env::remove_var(FIXTURE_ENV) };
            }
        }
    }
}
