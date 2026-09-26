//! Admission of shell metadata from the existing VT write chronology.
//!
//! This is not another OSC parser or an authentication boundary. Accepted
//! writes are still untrusted shell input. Framing prevents old and new local
//! discovery facts from being combined; it does not authorize remote paths.

use rio_backend::crosswords::Crosswords;
use rio_backend::event::EventListener;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum MetadataReadiness {
    Pending,
    #[default]
    Complete,
    Unavailable,
}

const MARKER: &str = "automexia_env_pending";
const SHELL: &str = "automexia_shell";
const SHELL_NAME: &str = "automexia_shell_name";
const BASE: [&str; 2] = ["automexia_env_HOME", "automexia_env_KUBECONFIG"];
const WINDOWS: [&str; 3] = [
    "automexia_env_HOMEDRIVE",
    "automexia_env_HOMEPATH",
    "automexia_env_USERPROFILE",
];
const FIELDS: [&str; 10] = [
    SHELL_NAME,
    "automexia_distro",
    "automexia_os_version",
    "automexia_shell_user",
    "automexia_shell_path",
    BASE[0],
    BASE[1],
    WINDOWS[0],
    WINDOWS[1],
    WINDOWS[2],
];

/// None is the legacy contract. Framed values must belong to this exact
/// interval, including equal and explicit-empty writes. No strings are copied
/// until the whole candidate has passed admission.
#[derive(Clone, Copy)]
pub(super) struct MetadataFrame {
    interval: Option<(u64, u64)>,
}

impl MetadataFrame {
    pub(super) fn value<'a, T: EventListener>(
        self,
        terminal: &'a Crosswords<T>,
        name: &str,
    ) -> Option<&'a String> {
        let value = terminal.user_vars.get(name)?;
        let Some((begin, end)) = self.interval else {
            return Some(value);
        };
        // Activation may precede an ordinary replay frame. The sole supported
        // postlude is validated in observe(), never guessed from map contents.
        if name == SHELL || name == MARKER {
            return Some(value);
        }
        let serial = terminal.user_var_write_stamp(name)?.latest.get();
        (begin < serial && serial < end).then_some(value)
    }
}

#[derive(Default)]
pub(crate) struct ShellMetadataState {
    readiness: MetadataReadiness,
    framed_seen: bool,
}

impl ShellMetadataState {
    pub(crate) fn readiness(&self) -> MetadataReadiness {
        self.readiness
    }

    pub(super) fn observe<T: EventListener>(
        &mut self,
        terminal: &Crosswords<T>,
    ) -> Option<MetadataFrame> {
        self.readiness = MetadataReadiness::Unavailable;
        if !terminal.user_var_chronology_valid() {
            self.framed_seen = true;
            return None;
        }
        let rejection = terminal.last_user_var_rejection().map(|value| value.get());
        let Some(marker) = terminal.user_vars.get(MARKER) else {
            // Rejected first framing attempts must not look like a legacy
            // session merely because the saturated map has no marker entry.
            if rejection.is_some() {
                self.framed_seen = true;
            }
            if self.framed_seen {
                return None;
            }
            self.readiness = MetadataReadiness::Complete;
            return Some(MetadataFrame { interval: None });
        };
        self.framed_seen = true;
        let stamp = terminal.user_var_write_stamp(MARKER)?;
        let end = stamp.latest.get();
        if marker == "1" {
            if rejection.is_none_or(|serial| serial < end) {
                self.readiness = MetadataReadiness::Pending;
            }
            return None;
        }
        if marker != "0" || !terminal.user_var_previous_was_one(MARKER) {
            return None;
        }
        let begin = stamp.previous?.get();
        if begin >= end || rejection.is_some_and(|serial| serial > begin) {
            return None;
        }
        let frame = MetadataFrame {
            interval: Some((begin, end)),
        };
        let shell_name = frame.value(terminal, SHELL_NAME)?;
        if shell_name.trim().is_empty() {
            return None;
        }
        let activation = terminal.user_var_write_stamp(SHELL)?.latest.get();
        if terminal.user_vars.get(SHELL).map(String::as_str) != Some("1") {
            return None;
        }
        // Older PowerShell publishes its activation immediately after commit.
        // Missing/reordered writes, another shell, or intervening events cannot
        // borrow that compatibility exception.
        if activation > end
            && !(shell_name.eq_ignore_ascii_case("PowerShell")
                && end.checked_add(1) == Some(activation))
        {
            return None;
        }
        for name in FIELDS {
            if terminal
                .user_var_write_stamp(name)
                .is_some_and(|value| value.latest.get() > end)
            {
                return None;
            }
            if frame.value(terminal, name).is_some_and(|value| {
                value.len() > 4096 || value.chars().any(char::is_control)
            }) {
                return None;
            }
        }
        if !shell_name.eq_ignore_ascii_case("CMD")
            && BASE
                .iter()
                .any(|name| frame.value(terminal, name).is_none())
        {
            return None;
        }
        let windows_count = WINDOWS
            .iter()
            .filter(|name| frame.value(terminal, name).is_some())
            .count();
        if windows_count != 0 && windows_count != WINDOWS.len() {
            return None;
        }
        self.readiness = MetadataReadiness::Complete;
        Some(frame)
    }
}
