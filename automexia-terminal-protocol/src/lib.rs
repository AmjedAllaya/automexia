#![no_std]
#![forbid(unsafe_code)]

//! Capability-free terminal wire values; policy and session ownership stay in callers.

/// A malformed wire value, without retaining or reporting untrusted input.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DecodeError {
    InvalidFrame,
    InvalidGeneration,
}

impl core::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match self {
            Self::InvalidFrame => "invalid terminal revision frame",
            Self::InvalidGeneration => "invalid terminal scope generation",
        })
    }
}

impl core::error::Error for DecodeError {}

/// Validated version-one revision value, independent of a caller's active scope.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScopeRevision {
    pane: u64,
    generation: u64,
    revision: u32,
}

impl ScopeRevision {
    /// Decode one complete value. Empty input represents explicit revocation.
    ///
    /// This checks wire validity only; callers must separately enforce their
    /// active scope, monotonic revisions, and revocation ordering.
    pub fn decode(value: &str) -> Result<Option<Self>, DecodeError> {
        if value.is_empty() {
            return Ok(None);
        }
        if value.len() > 96 || value.contains('\n') {
            return Err(DecodeError::InvalidFrame);
        }
        let mut fields = value.split('|');
        let magic = fields.next();
        let pane = fields.next().ok_or(DecodeError::InvalidFrame)?;
        let generation = fields.next().ok_or(DecodeError::InvalidFrame)?;
        let revision = fields.next().ok_or(DecodeError::InvalidFrame)?;
        if magic != Some("AMXSSHREV1") || fields.next().is_some() {
            return Err(DecodeError::InvalidFrame);
        }
        let pane = decimal(pane)?;
        let generation = decimal(generation)?;
        if pane == 0 || generation == 0 {
            return Err(DecodeError::InvalidGeneration);
        }
        let revision =
            u32::try_from(decimal(revision)?).map_err(|_| DecodeError::InvalidFrame)?;
        if revision == 0 {
            return Err(DecodeError::InvalidFrame);
        }
        Ok(Some(Self {
            pane,
            generation,
            revision,
        }))
    }

    pub const fn pane(self) -> u64 {
        self.pane
    }

    pub const fn generation(self) -> u64 {
        self.generation
    }

    pub const fn revision(self) -> u32 {
        self.revision
    }
}

fn decimal(value: &str) -> Result<u64, DecodeError> {
    if value.is_empty()
        || !value.bytes().all(|byte| byte.is_ascii_digit())
        || (value.len() > 1 && value.starts_with('0'))
    {
        return Err(DecodeError::InvalidFrame);
    }
    value.parse().map_err(|_| DecodeError::InvalidFrame)
}
