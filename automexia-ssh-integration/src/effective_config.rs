//! Bounded observations of successful system OpenSSH `-G` output.
//!
//! This is not an SSH configuration interpreter or a launch authorization. The
//! application owns the explicit, bounded subprocess and must reject unsuccessful
//! or truncated capture. Never run that probe passively: OpenSSH may evaluate
//! `Match exec` while reading the user's configuration.
use crate::{Error, PassReason, MAX_EFFECTIVE_CONFIG_BYTES};

const MAX_CONFIG_LINES: usize = 1024;
const MAX_CONFIG_LINE_BYTES: usize = 4096;

/// Only compatibility facts survive parsing. Hosts, identities, paths, commands,
/// environment values and the captured output are not retained or exposed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EffectiveConfig {
    passthrough: Option<PassReason>,
    native_interactive: bool,
}

impl EffectiveConfig {
    pub fn parse(output: &[u8]) -> Result<Self, Error> {
        if output.len() > MAX_EFFECTIVE_CONFIG_BYTES {
            return Err(Error::ConfigurationLimit);
        }
        let mut tty = None;
        let mut session = None;
        let mut stdin = None;
        let mut fork = None;
        let mut master = None;
        let mut automatic_master = false;
        let mut persist = None;
        let mut command = false;
        let lines = output.strip_suffix(b"\n").unwrap_or(output);
        for (index, raw) in lines.split(|byte| *byte == b'\n').enumerate() {
            if index >= MAX_CONFIG_LINES {
                return Err(Error::ConfigurationLimit);
            }
            if raw.len() > MAX_CONFIG_LINE_BYTES {
                return Err(Error::ConfigurationLimit);
            }
            let line = raw.strip_suffix(b"\r").unwrap_or(raw);
            if line.is_empty() {
                continue;
            }
            if line
                .iter()
                .any(|byte| byte.is_ascii_control() && *byte != b'\t')
            {
                return Err(Error::InvalidConfiguration);
            }
            let Some(split) = line.iter().position(u8::is_ascii_whitespace) else {
                return Err(Error::InvalidConfiguration);
            };
            let (name, value) = line.split_at(split);
            if name.is_empty() || !name.iter().all(u8::is_ascii_alphanumeric) {
                return Err(Error::InvalidConfiguration);
            }
            let value = value.trim_ascii();
            match name {
                b"requesttty" => {
                    set_once(
                        &mut tty,
                        match value {
                            b"auto" | b"yes" | b"force" => true,
                            b"no" => false,
                            _ => return Err(Error::InvalidConfiguration),
                        },
                    )?;
                }
                b"sessiontype" => {
                    set_once(
                        &mut session,
                        match value {
                            b"default" => true,
                            b"none" | b"subsystem" => false,
                            _ => return Err(Error::InvalidConfiguration),
                        },
                    )?;
                }
                b"stdinnull" => set_once(&mut stdin, disabled(value)?)?,
                b"forkafterauthentication" => set_once(&mut fork, disabled(value)?)?,
                b"controlmaster" => {
                    automatic_master = matches!(value, b"auto" | b"autoask");
                    set_once(
                        &mut master,
                        match value {
                            b"false" | b"no" | b"auto" | b"autoask" => true,
                            b"true" | b"yes" | b"ask" => false,
                            _ => return Err(Error::InvalidConfiguration),
                        },
                    )?;
                }
                b"controlpersist" => {
                    let enabled = match value {
                        b"no" => false,
                        b"yes" => true,
                        // OpenSSH -G normalizes a duration to decimal seconds.
                        value
                            if !value.is_empty()
                                && value.iter().all(u8::is_ascii_digit) =>
                        {
                            value.iter().any(|byte| *byte != b'0')
                        }
                        _ => return Err(Error::InvalidConfiguration),
                    };
                    set_once(&mut persist, enabled)?;
                }
                // An unset RemoteCommand is omitted by OpenSSH, not necessarily
                // represented by `none`. Any present command stays native, even
                // an apparent `none`: string values in -G output are not escaped.
                b"remotecommand" => {
                    if command {
                        return Err(Error::InvalidConfiguration);
                    }
                    command = true;
                }
                _ => {}
            }
        }
        let (Some(tty), Some(session), Some(stdin), Some(fork), Some(master)) =
            (tty, session, stdin, fork, master)
        else {
            return Err(Error::InvalidConfiguration);
        };
        let passthrough = if command {
            Some(PassReason::ExplicitCommand)
        // A persistent automatic master intentionally outlives this session.
        // Wrapper-owned process-tree cleanup must not adopt or retire it.
        } else if !session
            || !fork
            || !master
            || (automatic_master && persist == Some(true))
        {
            Some(PassReason::TransportOrControl)
        } else if !tty || !stdin {
            Some(PassReason::NotInteractive)
        } else {
            None
        };
        let native_interactive = !command && session && tty && stdin && fork && master;
        Ok(Self {
            passthrough,
            native_interactive,
        })
    }

    /// None means observed compatibility, never a grant to execute a candidate.
    pub const fn passthrough_reason(&self) -> Option<PassReason> {
        self.passthrough
    }

    /// Configuration permits an ordinary native interactive session, including
    /// an automatic persistent master whose descendants the wrapper must retain.
    /// This does not authorize execution or shell injection: the caller must
    /// independently require an interactive invocation and native leader-only
    /// ownership. Command, subsystem, detached and control-only modes are false.
    pub const fn native_interactive_eligible(&self) -> bool {
        self.native_interactive
    }
}

fn disabled(value: &[u8]) -> Result<bool, Error> {
    match value {
        b"no" => Ok(true),
        b"yes" => Ok(false),
        _ => Err(Error::InvalidConfiguration),
    }
}

fn set_once(target: &mut Option<bool>, value: bool) -> Result<(), Error> {
    if target.replace(value).is_some() {
        Err(Error::InvalidConfiguration)
    } else {
        Ok(())
    }
}
