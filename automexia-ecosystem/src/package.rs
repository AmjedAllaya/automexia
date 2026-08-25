use serde::{Deserialize, Serialize};

use crate::{valid_digest, EcosystemManifest};

pub const REQUIRED_PACKAGE_ENTRIES: [&str; 6] = [
    "manifest.json",
    "component.wasm",
    "signature-bundle.json",
    "provenance.json",
    "sbom.spdx.json",
    "LICENSES.txt",
];

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignatureBundle {
    pub schema_version: u32,
    pub content_sha256: String,
    pub provenance_sha256: String,
    pub publisher_id: String,
    pub key_id: String,
    pub public_key_base64: String,
    pub signature_base64: String,
    pub issued_at_unix: u64,
    pub expires_at_unix: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProvenanceStatement {
    pub schema_version: u32,
    pub content_sha256: String,
    pub publisher_id: String,
    pub extension_id: String,
    pub version: String,
    pub source_uri: String,
    pub source_revision: String,
    pub builder_id: String,
    pub built_at_unix: u64,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrustedPublisher {
    pub publisher_id: String,
    pub key_id: String,
    pub public_key_base64: String,
    pub valid_from_unix: u64,
    pub valid_until_unix: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RevocationSnapshot {
    pub sequence: u64,
    pub observed_at_unix: u64,
    pub valid_until_unix: u64,
    pub revoked_publishers: Vec<String>,
    pub revoked_keys: Vec<String>,
    pub revoked_package_digests: Vec<String>,
}

impl RevocationSnapshot {
    pub fn is_current(&self, now_unix: u64, minimum_sequence: u64) -> bool {
        self.sequence >= minimum_sequence
            && self.observed_at_unix <= now_unix
            && now_unix < self.valid_until_unix
            && self
                .revoked_package_digests
                .iter()
                .all(|digest| valid_digest(digest))
    }

    pub fn is_revoked(
        &self,
        publisher_id: &str,
        key_id: &str,
        package_sha256: &str,
    ) -> bool {
        self.revoked_publishers
            .iter()
            .any(|value| value == publisher_id)
            || self.revoked_keys.iter().any(|value| value == key_id)
            || self
                .revoked_package_digests
                .iter()
                .any(|value| value == package_sha256)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VerificationReceipt {
    pub schema_version: u32,
    pub extension_id: String,
    pub version: String,
    pub publisher_id: String,
    pub package_sha256: String,
    pub content_sha256: String,
    pub provenance_sha256: String,
    pub sbom_sha256: String,
    pub licenses_sha256: String,
    pub key_id: String,
    pub source_uri: String,
    pub source_revision: String,
    pub builder_id: String,
    pub built_at_unix: u64,
    pub signature_expires_at_unix: u64,
    pub verified_at_unix: u64,
    pub revocation_sequence: u64,
    pub manifest: EcosystemManifest,
}

impl std::fmt::Debug for TrustedPublisher {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("TrustedPublisher")
            .field("publisher_id", &self.publisher_id)
            .field("key_id", &self.key_id)
            .field("public_key_base64", &"[redacted]")
            .field("valid_from_unix", &self.valid_from_unix)
            .field("valid_until_unix", &self.valid_until_unix)
            .finish()
    }
}
