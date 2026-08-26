use std::fmt;

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DistributionGate {
    #[default]
    DisabledPendingNetworkAdr,
}

impl DistributionGate {
    pub const fn downloads_enabled(self) -> bool {
        false
    }

    pub const fn startup_refresh_enabled(self) -> bool {
        false
    }

    pub const fn typing_refresh_enabled(self) -> bool {
        false
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MetadataVersions {
    pub root: u64,
    pub targets: u64,
    pub snapshot: u64,
    pub timestamp: u64,
    pub timestamp_expires_at_unix: u64,
    pub consistent_snapshot: bool,
    pub threshold_role_separation: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MetadataError {
    Disabled,
    Rollback,
    Expired,
    UnsafeRoles,
}

impl fmt::Display for MetadataError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Disabled => "distribution network is disabled pending a separate ADR",
            Self::Rollback => "repository metadata version regressed",
            Self::Expired => "repository timestamp is expired",
            Self::UnsafeRoles => {
                "consistent snapshots and threshold role separation are required"
            }
        })
    }
}

impl std::error::Error for MetadataError {}

pub fn validate_offline_metadata(
    previous: Option<&MetadataVersions>,
    candidate: &MetadataVersions,
    now_unix: u64,
) -> Result<(), MetadataError> {
    if !candidate.consistent_snapshot || !candidate.threshold_role_separation {
        return Err(MetadataError::UnsafeRoles);
    }
    if now_unix >= candidate.timestamp_expires_at_unix {
        return Err(MetadataError::Expired);
    }
    if previous.is_some_and(|previous| {
        candidate.root < previous.root
            || candidate.targets < previous.targets
            || candidate.snapshot < previous.snapshot
            || candidate.timestamp < previous.timestamp
    }) {
        return Err(MetadataError::Rollback);
    }
    Ok(())
}

pub fn request_distribution_refresh(
    _gate: DistributionGate,
) -> Result<(), MetadataError> {
    Err(MetadataError::Disabled)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distribution_remains_nonactivating_and_rejects_rollback_freeze_and_unsafe_roles() {
        assert!(!DistributionGate::default().downloads_enabled());
        assert_eq!(
            request_distribution_refresh(DistributionGate::default()),
            Err(MetadataError::Disabled)
        );
        let current = MetadataVersions {
            root: 4,
            targets: 8,
            snapshot: 9,
            timestamp: 10,
            timestamp_expires_at_unix: 200,
            consistent_snapshot: true,
            threshold_role_separation: true,
        };
        validate_offline_metadata(None, &current, 100).unwrap();
        let mut rollback = current.clone();
        rollback.targets -= 1;
        assert_eq!(
            validate_offline_metadata(Some(&current), &rollback, 100),
            Err(MetadataError::Rollback)
        );
        assert_eq!(
            validate_offline_metadata(None, &current, 200),
            Err(MetadataError::Expired)
        );
    }
}
