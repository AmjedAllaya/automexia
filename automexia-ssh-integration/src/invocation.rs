//! Conservative enhancement eligibility. This is NOT an SSH authorization parser.
//! Unrecognized invocations retain their exact OsString arguments for a caller
//! that is separately authorized to launch native SSH.
use crate::{Error, MAX_ARGUMENTS, MAX_ARGUMENT_BYTES, MAX_DESTINATION_BYTES};
use std::ffi::{OsStr, OsString};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PassReason {
    Disabled,
    NotInteractive,
    ExplicitCommand,
    NoDestination,
    TransportOrControl,
    UnsupportedOption,
    NonUnicode,
    UnknownConfiguration,
    UnsupportedShell,
    UnsupportedStartup,
    RemoteWritesDenied,
    AmbiguousDestination,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InvocationClass {
    Interactive { destination_index: usize },
    Passthrough(PassReason),
}
#[derive(Clone, PartialEq, Eq)]
pub struct Invocation {
    arguments: Vec<OsString>,
}
impl std::fmt::Debug for Invocation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Invocation")
            .field("argument_count", &self.arguments.len())
            .field("arguments", &"<redacted>")
            .finish()
    }
}
impl Invocation {
    pub fn new(arguments: Vec<OsString>) -> Result<Self, Error> {
        if arguments.len() > MAX_ARGUMENTS {
            return Err(Error::ArgumentLimit);
        }
        let mut bytes = 0usize;
        for value in &arguments {
            let value = value.as_encoded_bytes();
            if value.contains(&0) {
                return Err(Error::InvalidArgument);
            }
            bytes = bytes.checked_add(value.len()).ok_or(Error::ArgumentLimit)?;
            if bytes > MAX_ARGUMENT_BYTES {
                return Err(Error::ArgumentLimit);
            }
        }
        Ok(Self { arguments })
    }
    pub fn arguments(&self) -> &[OsString] {
        &self.arguments
    }
    /// No copies, filesystem access, ssh -G, or user config evaluation.
    pub fn classify(
        &self,
        terminal_input: bool,
        terminal_output: bool,
    ) -> InvocationClass {
        use InvocationClass::{Interactive, Passthrough};
        if !terminal_input || !terminal_output {
            return Passthrough(PassReason::NotInteractive);
        }
        let mut index = 0usize;
        let mut options = true;
        while index < self.arguments.len() {
            let Some(arg) = self.arguments[index].to_str() else {
                return Passthrough(PassReason::NonUnicode);
            };
            if options && arg == "--" {
                options = false;
                index += 1;
                continue;
            }
            if options && arg.starts_with('-') {
                match arg {
                    "-T" | "-n" | "-N" | "-f" | "-s" | "-W" | "-O" | "-G" | "-V" => {
                        return Passthrough(PassReason::TransportOrControl);
                    }
                    "-4" | "-6" | "-C" | "-v" | "-vv" | "-vvv" | "-q" | "-t" | "-tt" => {
                        index += 1;
                        continue;
                    }
                    _ => {}
                }
                // A deliberately small known grammar. In particular -o is not
                // interpreted: RemoteCommand, Match and side effects need host review.
                let separated = matches!(arg, "-p" | "-l" | "-J" | "-i" | "-F");
                let attached = arg.len() > 2
                    && ["-p", "-l", "-J", "-i", "-F"]
                        .iter()
                        .any(|prefix| arg.starts_with(prefix));
                if separated || attached {
                    let value = if separated {
                        index += 1;
                        match self.arguments.get(index).and_then(|value| value.to_str()) {
                            Some(value) => value,
                            None => return Passthrough(PassReason::UnsupportedOption),
                        }
                    } else {
                        &arg[2..]
                    };
                    if value.is_empty() || value.chars().any(char::is_control) {
                        return Passthrough(PassReason::UnsupportedOption);
                    }
                    if arg.starts_with("-p")
                        && value.parse::<u16>().ok().filter(|port| *port > 0).is_none()
                    {
                        return Passthrough(PassReason::UnsupportedOption);
                    }
                    index += 1;
                    continue;
                }
                return Passthrough(PassReason::UnsupportedOption);
            }
            if index + 1 < self.arguments.len() {
                return Passthrough(PassReason::ExplicitCommand);
            }
            if !destination_is_unambiguous(arg) {
                return Passthrough(PassReason::AmbiguousDestination);
            }
            return Interactive {
                destination_index: index,
            };
        }
        Passthrough(PassReason::NoDestination)
    }
}
fn destination_is_unambiguous(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_DESTINATION_BYTES
        && !value.starts_with('-')
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._-@[]:%/".contains(&byte))
}

/// Preserve OS-native strings even when they are not UTF-8.
pub fn argument_bytes(value: &OsStr) -> usize {
    value.as_encoded_bytes().len()
}
