use std::{
    collections::BTreeMap,
    hint::black_box,
    io::{Cursor, Write as _},
};

use automexia_ecosystem::{
    Compatibility, EcosystemManifest, ExtensionKind, ProvenanceStatement,
    RevocationSnapshot, SignatureBundle, TrustedPublisher, WIT_WORLD,
};
use automexia_ecosystem_runtime::{
    content_digest, signature_message, verify_bundle_bytes, VerificationContext,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use criterion::{criterion_group, criterion_main, Criterion};
use ed25519_dalek::{Signer as _, SigningKey};
use sha2::{Digest as _, Sha256};
use zip::{write::SimpleFileOptions, CompressionMethod, ZipWriter};

struct Fixture {
    bundle: Vec<u8>,
    trusted: TrustedPublisher,
    revocation: RevocationSnapshot,
}

fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn fixture() -> Fixture {
    let manifest = EcosystemManifest {
        schema_version: 1,
        extension_id: "benchmark.inspect".into(),
        display_name: "Benchmark Inspect".into(),
        description: "Bounded verification benchmark".into(),
        publisher_id: "benchmark.publisher".into(),
        version: "1.0.0".into(),
        kind: ExtensionKind::Component,
        compatibility: Compatibility {
            sdk_major: 1,
            sdk_minor_minimum: 0,
            sdk_minor_maximum: 0,
        },
        world: WIT_WORLD.into(),
        imports: vec![],
        capabilities: vec![],
        action_pack_entry: None,
    };
    let mut entries = BTreeMap::from([
        (
            "manifest.json".into(),
            serde_json::to_vec(&manifest).expect("benchmark manifest serializes"),
        ),
        ("component.wasm".into(), b"\0asm\r\0\x01\0".to_vec()),
        (
            "sbom.spdx.json".into(),
            br#"{"spdxVersion":"SPDX-2.3","dataLicense":"CC0-1.0","name":"benchmark"}"#
                .to_vec(),
        ),
        ("LICENSES.txt".into(), b"MIT\n".to_vec()),
    ]);
    let content_sha256 = content_digest(&entries);
    let provenance = ProvenanceStatement {
        schema_version: 1,
        content_sha256: content_sha256.clone(),
        publisher_id: manifest.publisher_id.clone(),
        extension_id: manifest.extension_id.clone(),
        version: manifest.version.clone(),
        source_uri: "https://example.invalid/benchmark".into(),
        source_revision: "0123456789abcdef".into(),
        builder_id: "benchmark.builder".into(),
        built_at_unix: 50,
    };
    let provenance_bytes =
        serde_json::to_vec(&provenance).expect("benchmark provenance serializes");
    let provenance_sha256 = sha256(&provenance_bytes);
    let signing = SigningKey::from_bytes(&[7; 32]);
    let signature = signing.sign(&signature_message(
        &manifest,
        &content_sha256,
        &provenance_sha256,
    ));
    let public_key_base64 = BASE64.encode(signing.verifying_key().to_bytes());
    entries.insert("provenance.json".into(), provenance_bytes);
    entries.insert(
        "signature-bundle.json".into(),
        serde_json::to_vec(&SignatureBundle {
            schema_version: 1,
            content_sha256,
            provenance_sha256,
            publisher_id: manifest.publisher_id.clone(),
            key_id: "benchmark.key".into(),
            public_key_base64: public_key_base64.clone(),
            signature_base64: BASE64.encode(signature.to_bytes()),
            issued_at_unix: 50,
            expires_at_unix: 200,
        })
        .expect("benchmark signature bundle serializes"),
    );

    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .unix_permissions(0o100600);
    for (name, bytes) in entries {
        writer
            .start_file(name, options)
            .expect("benchmark ZIP entry starts");
        writer
            .write_all(&bytes)
            .expect("benchmark ZIP entry writes");
    }

    Fixture {
        bundle: writer
            .finish()
            .expect("benchmark ZIP finishes")
            .into_inner(),
        trusted: TrustedPublisher {
            publisher_id: manifest.publisher_id,
            key_id: "benchmark.key".into(),
            public_key_base64,
            valid_from_unix: 1,
            valid_until_unix: 300,
        },
        revocation: RevocationSnapshot {
            sequence: 4,
            observed_at_unix: 90,
            valid_until_unix: 150,
            revoked_publishers: vec![],
            revoked_keys: vec![],
            revoked_package_digests: vec![],
        },
    }
}

fn verification(c: &mut Criterion) {
    let fixture = fixture();
    c.bench_function("ecosystem_signed_bundle_verification", |bench| {
        bench.iter(|| {
            black_box(
                verify_bundle_bytes(
                    black_box(&fixture.bundle),
                    VerificationContext {
                        now_unix: 100,
                        minimum_revocation_sequence: 4,
                        supported_sdk_minor: 0,
                        trusted_publishers: std::slice::from_ref(&fixture.trusted),
                        revocation: &fixture.revocation,
                    },
                )
                .expect("the fixed benchmark bundle remains valid"),
            )
        });
    });
}

criterion_group!(benches, verification);
criterion_main!(benches);
