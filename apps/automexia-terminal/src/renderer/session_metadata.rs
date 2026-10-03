use rio_backend::crosswords::Crosswords;
use rio_backend::event::EventListener;

const FRAME_MARKER: &str = "automexia_env_pending";
const ACTIVATION: &str = "automexia_shell";
const SHELL_NAME: &str = "automexia_shell_name";
const CMD_REFERENCE: &str = "automexia_cmd_ref_v1";
const BASE_LOCATIONS: [&str; 2] = ["automexia_env_HOME", "automexia_env_KUBECONFIG"];
const WINDOWS_LOCATIONS: [&str; 3] = [
    "automexia_env_HOMEDRIVE",
    "automexia_env_HOMEPATH",
    "automexia_env_USERPROFILE",
];
const IDENTITY_FIELDS: [&str; 4] = [
    "automexia_distro",
    "automexia_os_version",
    "automexia_shell_user",
    "automexia_shell_path",
];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum MetadataReadiness {
    Pending,
    #[default]
    Complete,
    Unavailable,
}

/// Per-terminal admission state. It is never copied into a cloned terminal.
#[derive(Clone, Default)]
pub(crate) struct ShellMetadataState {
    readiness: MetadataReadiness,
    framing_seen: bool,
}

#[derive(Clone, Copy)]
pub(super) enum Admission {
    Legacy,
    Frame {
        begin: u64,
        end: u64,
        old_powershell_activation: bool,
        cmd_reference: Option<[u8; 32]>,
    },
    Retain,
}

impl Admission {
    pub(super) fn value<'a, T: EventListener>(
        self,
        terminal: &'a Crosswords<T>,
        name: &str,
    ) -> Option<&'a String> {
        match self {
            Self::Legacy => terminal.user_vars.get(name),
            Self::Retain => None,
            Self::Frame {
                begin,
                end,
                old_powershell_activation,
                cmd_reference,
            } => {
                if let Some(reference) = cmd_reference {
                    let field = match name {
                        "automexia_shell_user" => Some("user"),
                        "automexia_shell_path" => Some("path"),
                        _ => None,
                    };
                    if let Some(field) = field {
                        let reference = std::str::from_utf8(&reference).ok()?;
                        return terminal
                            .user_vars
                            .get(&format!("automexia_cmd_{field}_v1_{reference}"));
                    }
                }
                let serial = terminal.user_var_write_stamp(name)?.latest.get();
                let in_frame = serial > begin && serial < end;
                let postlude = old_powershell_activation
                    && name == ACTIVATION
                    && end.checked_add(1) == Some(serial);
                (in_frame || postlude)
                    .then(|| terminal.user_vars.get(name))
                    .flatten()
            }
        }
    }
}

impl ShellMetadataState {
    pub(crate) fn readiness(&self) -> MetadataReadiness {
        self.readiness
    }

    /// Read the current admitted identity even when this terminal has not been
    /// painted. Reuse frame validation without mutating the renderer's state.
    pub(crate) fn current_identity<'a, T: EventListener>(
        &self,
        terminal: &'a Crosswords<T>,
    ) -> Option<(Option<&'a str>, Option<&'a str>)> {
        let mut observation = self.clone();
        let admitted = observation.observe(terminal);
        if observation.readiness != MetadataReadiness::Complete {
            return None;
        }
        Some((
            admitted
                .value(terminal, SHELL_NAME)
                .filter(|v| valid_value(v) && !v.trim().is_empty())
                .map(String::as_str),
            admitted
                .value(terminal, "automexia_distro")
                .filter(|v| valid_value(v) && !v.trim().is_empty())
                .map(String::as_str),
        ))
    }

    fn retain(&mut self, readiness: MetadataReadiness) -> Admission {
        self.readiness = readiness;
        Admission::Retain
    }

    /// Assemble a fixed-size metadata frame from generic accepted-write stamps.
    /// Retained map entries are not evidence that a field belongs to this frame.
    pub(super) fn observe<T: EventListener>(
        &mut self,
        terminal: &Crosswords<T>,
    ) -> Admission {
        if !terminal.user_var_chronology_valid() {
            return self.retain(MetadataReadiness::Unavailable);
        }
        let Some(marker) = terminal.user_vars.get(FRAME_MARKER) else {
            if self.framing_seen || terminal.last_user_var_rejection().is_some() {
                return self.retain(MetadataReadiness::Unavailable);
            }
            self.readiness = MetadataReadiness::Complete;
            return Admission::Legacy;
        };
        self.framing_seen = true;
        if marker != "0" {
            return self.retain(MetadataReadiness::Pending);
        }
        if !terminal.user_var_previous_was_one(FRAME_MARKER) {
            return self.retain(MetadataReadiness::Unavailable);
        }
        let Some(stamp) = terminal.user_var_write_stamp(FRAME_MARKER) else {
            return self.retain(MetadataReadiness::Unavailable);
        };
        let Some(begin) = stamp.previous.map(|serial| serial.get()) else {
            return self.retain(MetadataReadiness::Unavailable);
        };
        let end = stamp.latest.get();
        if begin >= end
            || terminal
                .last_user_var_rejection()
                .is_some_and(|serial| serial.get() > begin)
        {
            return self.retain(MetadataReadiness::Unavailable);
        }
        let mut admission = Admission::Frame {
            begin,
            end,
            old_powershell_activation: false,
            cmd_reference: None,
        };
        let Some(shell_name) = admission
            .value(terminal, SHELL_NAME)
            .filter(|name| !name.trim().is_empty() && valid_value(name))
        else {
            return self.retain(MetadataReadiness::Unavailable);
        };
        let old_powershell_activation = shell_name.eq_ignore_ascii_case("PowerShell")
            && terminal
                .user_var_write_stamp(ACTIVATION)
                .is_some_and(|stamp| end.checked_add(1) == Some(stamp.latest.get()));
        admission = Admission::Frame {
            begin,
            end,
            old_powershell_activation,
            cmd_reference: None,
        };
        if admission.value(terminal, ACTIVATION).map(String::as_str) != Some("1") {
            return self.retain(MetadataReadiness::Unavailable);
        }
        if shell_name.eq_ignore_ascii_case("CMD")
            && terminal
                .user_var_write_stamp(CMD_REFERENCE)
                .is_some_and(|stamp| stamp.latest.get() > end)
        {
            // A reference arriving after commit is an incomplete reference
            // frame, not a legacy CMD frame with no reference field.
            self.readiness = MetadataReadiness::Unavailable;
            return admission;
        }
        // Known post-commit PowerShell activation is the sole legacy postlude.
        // Other late identity/location writes are a downgrade or incomplete frame.
        let known_fields = [SHELL_NAME, ACTIVATION]
            .into_iter()
            .chain(BASE_LOCATIONS)
            .chain(WINDOWS_LOCATIONS)
            .chain(IDENTITY_FIELDS)
            .chain(
                automexia_devops::SHELL_SELECTOR_HINTS
                    .into_iter()
                    .map(|(_, source)| source),
            );
        for name in known_fields {
            if terminal
                .user_var_write_stamp(name)
                .is_some_and(|stamp| stamp.latest.get() > end)
                && !(name == ACTIVATION && old_powershell_activation)
            {
                return self.retain(MetadataReadiness::Unavailable);
            }
            if admission
                .value(terminal, name)
                .is_some_and(|value| !valid_value(value))
            {
                return self.retain(MetadataReadiness::Unavailable);
            }
        }
        if !shell_name.eq_ignore_ascii_case("CMD") {
            if BASE_LOCATIONS
                .iter()
                .any(|name| admission.value(terminal, name).is_none())
            {
                return self.retain(MetadataReadiness::Unavailable);
            }
            let windows_count = WINDOWS_LOCATIONS
                .iter()
                .filter(|name| admission.value(terminal, name).is_some())
                .count();
            if windows_count != 0 && windows_count != WINDOWS_LOCATIONS.len() {
                return self.retain(MetadataReadiness::Unavailable);
            }
        } else if let Some(reference) = admission.value(terminal, CMD_REFERENCE) {
            // The existing terminal dictionary owns these bounded registrations.
            // Both fields must be consecutive, complete writes before this frame;
            // a partially replaced registration cannot lend stale identity.
            let token: Option<[u8; 32]> = reference.as_bytes().try_into().ok();
            let valid_token = token.filter(|token| {
                token
                    .iter()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(byte))
            });
            let registration = valid_token.and_then(|token| {
                let user_key = format!("automexia_cmd_user_v1_{reference}");
                let path_key = format!("automexia_cmd_path_v1_{reference}");
                let user_stamp = terminal.user_var_write_stamp(&user_key)?.latest.get();
                let path_stamp = terminal.user_var_write_stamp(&path_key)?.latest.get();
                let user = terminal.user_vars.get(&user_key)?;
                let path = terminal.user_vars.get(&path_key)?;
                (user_stamp.checked_add(1) == Some(path_stamp)
                    && path_stamp < begin
                    && valid_value(user)
                    && valid_value(path)
                    && !user.trim().is_empty()
                    && !path.trim().is_empty())
                .then_some(token)
            });
            // A recognized bad reference clears guest/old identity through this
            // complete CMD frame while blocking discovery. It must not Retain.
            self.readiness = if registration.is_some() {
                MetadataReadiness::Complete
            } else {
                MetadataReadiness::Unavailable
            };
            return Admission::Frame {
                begin,
                end,
                old_powershell_activation: false,
                cmd_reference: registration,
            };
        }
        self.readiness = MetadataReadiness::Complete;
        admission
    }
}

fn valid_value(value: &str) -> bool {
    value.len() <= 4096 && !value.chars().any(char::is_control)
}
