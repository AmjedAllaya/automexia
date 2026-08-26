#![no_main]

use automexia_ecosystem::RevocationSnapshot;
use automexia_ecosystem_runtime::{VerificationContext, verify_bundle_bytes};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|bytes: &[u8]| {
    let revocation = RevocationSnapshot {
        sequence: 1,
        observed_at_unix: 1,
        valid_until_unix: u64::MAX,
        revoked_publishers: Vec::new(),
        revoked_keys: Vec::new(),
        revoked_package_digests: Vec::new(),
    };
    let _ = verify_bundle_bytes(
        bytes,
        VerificationContext {
            now_unix: 2,
            minimum_revocation_sequence: 1,
            supported_sdk_minor: 0,
            trusted_publishers: &[],
            revocation: &revocation,
        },
    );
});
