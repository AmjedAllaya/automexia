#![no_main]

use automexia_ecosystem::{
    decode_settings_metadata, Compatibility, EcosystemManifest, ExtensionKind,
    RevocationSnapshot, WIT_WORLD,
};
use automexia_ecosystem_runtime::{verify_bundle_bytes, VerificationContext};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|bytes: &[u8]| {
    // Reach the pure metadata decoder directly. The hostile verifier below has
    // no trusted publishers; signed projection integrity belongs to package tests.
    let manifest = EcosystemManifest {
        schema_version: 1,
        publisher_id: "example.publisher".into(),
        extension_id: "example.inspect".into(),
        version: "1.0.0".into(),
        display_name: "Example".into(),
        description: String::new(),
        kind: ExtensionKind::Component,
        compatibility: Compatibility {
            sdk_major: 1,
            sdk_minor_minimum: 0,
            sdk_minor_maximum: 0,
        },
        world: WIT_WORLD.into(),
        imports: Vec::new(),
        capabilities: Vec::new(),
        action_pack_entry: None,
    };
    assert!(
        manifest.validate(0).is_ok(),
        "fixed fuzz manifest must remain valid"
    );
    let _ = decode_settings_metadata(bytes, &manifest);
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
