//! One strict parser for configured startup and child-session environments.

use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EnvironmentError {
    entry: usize,
    reason: &'static str,
}

impl fmt::Display for EnvironmentError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "env-vars entry {} {}", self.entry, self.reason)
    }
}

impl std::error::Error for EnvironmentError {}

/// Validate the entire batch before returning anything a caller can apply.
/// Names retain the launch adapter's whitespace trimming; values are exact.
pub fn parse_environment(
    entries: &[String],
) -> Result<Vec<(String, String)>, EnvironmentError> {
    let mut parsed = Vec::new();
    let mut bytes = 0usize;
    for (index, entry) in entries.iter().enumerate() {
        let reject = |reason| EnvironmentError {
            entry: index + 1,
            reason,
        };
        bytes = bytes
            .checked_add(entry.len())
            .filter(|bytes| *bytes as u64 <= super::product::MAX_CONFIG_FILE_BYTES)
            .ok_or_else(|| reject("exceeds the configuration byte limit"))?;
        let (name, value) = entry
            .split_once('=')
            .ok_or_else(|| reject("must use NAME=VALUE"))?;
        let name = name.trim();
        if name.is_empty() {
            return Err(reject("has an empty name"));
        }
        if name.contains('\0') || value.contains('\0') {
            return Err(reject("contains a NUL character"));
        }
        parsed.push((name.to_owned(), value.to_owned()));
    }
    Ok(parsed)
}
