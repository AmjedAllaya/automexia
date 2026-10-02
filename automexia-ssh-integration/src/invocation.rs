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
    /// Arguments for the explicitly requested, separate staging connection.
    /// First-value config overrides and removal of enabling short switches keep
    /// forwarding and local commands confined to the later native session.
    /// Authentication/destination options and option values retain OS strings.
    pub fn upload_stage_arguments(&self) -> Result<Vec<OsString>, Error> {
        let InvocationClass::Interactive { destination_index } =
            self.classify_for_wrapper(true, true)
        else {
            return Err(Error::InvalidArgument);
        };
        let mut result: Vec<OsString> = ["-T", "-a", "-x"].map(OsString::from).into();
        for setting in [
            "RequestTTY=no",
            "ClearAllForwardings=yes",
            "ForwardAgent=no",
            "ForwardX11=no",
            "PermitLocalCommand=no",
            "ControlMaster=no",
            "ControlPersist=no",
            "Tunnel=no",
            "GSSAPIDelegateCredentials=no",
        ] {
            result.push("-o".into());
            result.push(setting.into());
        }
        let mut index = 0;
        while index < destination_index {
            let raw = &self.arguments[index];
            let argument = raw.to_str().ok_or(Error::InvalidArgument)?;
            if argument == "--" {
                result.push(raw.clone());
                index += 1;
                continue;
            }
            let bytes = argument.as_bytes();
            let mut kept = String::from("-");
            let mut offset = 1;
            while offset < bytes.len() {
                let flag = bytes[offset];
                if b"tAXYgK".contains(&flag) {
                    offset += 1;
                    continue;
                }
                let takes_value = b"BbcDEeFIiJLlmoPpRSw".contains(&flag);
                let omit = b"DLRw".contains(&flag);
                if !omit {
                    kept.push(char::from(flag));
                }
                if takes_value {
                    if offset + 1 < bytes.len() {
                        if !omit {
                            kept.push_str(&argument[offset + 1..]);
                        }
                    } else {
                        index += 1;
                        let value =
                            self.arguments.get(index).ok_or(Error::InvalidArgument)?;
                        if !omit {
                            result.push(OsString::from(&kept));
                            kept.clear();
                            result.push(value.clone());
                        }
                    }
                    break;
                }
                offset += 1;
            }
            if kept.len() > 1 {
                result.push(kept.into());
            }
            index += 1;
        }
        result.push(self.arguments[destination_index].clone());
        // The generated stage is subject to the same native argument bounds.
        Self::new(result).map(|stage| stage.arguments)
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

    /// Recognize the native argument grammar for the explicit SSH wrapper.
    /// Interactive means eligible for a separate effective-configuration check,
    /// never permission to append a remote command. Values are neither split nor
    /// interpreted as shell text; the original OS strings remain authoritative.
    pub fn classify_for_wrapper(
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
            let Some(argument) = self.arguments[index].to_str() else {
                return Passthrough(PassReason::NonUnicode);
            };
            if options && argument == "--" {
                options = false;
                index += 1;
                continue;
            }
            if options && argument.starts_with('-') {
                if argument.len() == 1 {
                    return Passthrough(PassReason::UnsupportedOption);
                }
                // OpenSSH short options may be clustered. An option taking a
                // value owns the entire remainder, including option-like text.
                let flags = argument.as_bytes();
                let mut offset = 1;
                while offset < flags.len() {
                    let flag = flags[offset];
                    if b"TnNfsWOGVQM".contains(&flag) {
                        return Passthrough(PassReason::TransportOrControl);
                    }
                    if b"46AaCgKkqtvXxYy".contains(&flag) {
                        offset += 1;
                        continue;
                    }
                    if !b"BbcDEeFIiJLlmoPpRSw".contains(&flag) {
                        return Passthrough(PassReason::UnsupportedOption);
                    }
                    let value = if offset + 1 < flags.len() {
                        // All preceding option bytes are ASCII, so this boundary
                        // is valid even when the attached value contains Unicode.
                        &argument[offset + 1..]
                    } else {
                        index += 1;
                        let Some(value) = self.arguments.get(index) else {
                            return Passthrough(PassReason::UnsupportedOption);
                        };
                        let Some(value) = value.to_str() else {
                            return Passthrough(PassReason::NonUnicode);
                        };
                        value
                    };
                    if value.is_empty() || value.chars().any(char::is_control) {
                        return Passthrough(PassReason::UnsupportedOption);
                    }
                    if flag == b'p'
                        && (!value.bytes().all(|byte| byte.is_ascii_digit())
                            || value
                                .parse::<u16>()
                                .ok()
                                .filter(|port| *port > 0)
                                .is_none())
                    {
                        return Passthrough(PassReason::UnsupportedOption);
                    }
                    break;
                }
                index += 1;
                continue;
            }
            if index + 1 < self.arguments.len() {
                return Passthrough(PassReason::ExplicitCommand);
            }
            if !destination_is_unambiguous(argument) {
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
