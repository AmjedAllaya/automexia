use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
    fs::{self, OpenOptions},
    io::{Cursor, Read},
    path::Path,
};

use automexia_ecosystem::{
    decode_strict_json, portable_identifier, safe_relative_path, safe_text, valid_digest,
    EcosystemManifest, Limits, ProvenanceStatement, RevocationSnapshot, SignatureBundle,
    StrictJsonError, TrustedPublisher, VerificationReceipt, REQUIRED_PACKAGE_ENTRIES,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use ed25519_dalek::{Signature, VerifyingKey};
use serde::Deserialize;
use sha2::{Digest as _, Sha256};
use zip::ZipArchive;

const SIGNATURE_ENTRY: &str = "signature-bundle.json";
const PROVENANCE_ENTRY: &str = "provenance.json";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BundleErrorCode {
    SourceTooLarge,
    SourceChanged,
    LinkRejected,
    Archive,
    TooManyFiles,
    UnsafeEntry,
    DuplicateEntry,
    EntryTooLarge,
    ExpansionLimit,
    MissingEntry,
    StrictJson,
    Manifest,
    Component,
    Sbom,
    Licenses,
    ContentDigest,
    Provenance,
    UntrustedPublisher,
    ExpiredSignature,
    StaleRevocation,
    Revoked,
    Signature,
}

#[derive(Debug)]
pub struct BundleError {
    pub code: BundleErrorCode,
    pub detail: String,
}

impl BundleError {
    fn new(code: BundleErrorCode, detail: impl Into<String>) -> Self {
        Self {
            code,
            detail: detail.into(),
        }
    }
}

impl fmt::Display for BundleError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{:?}: {}", self.code, self.detail)
    }
}

impl std::error::Error for BundleError {}

impl From<StrictJsonError> for BundleError {
    fn from(error: StrictJsonError) -> Self {
        Self::new(BundleErrorCode::StrictJson, error.to_string())
    }
}

#[derive(Clone)]
pub struct VerifiedBundle {
    pub receipt: VerificationReceipt,
    entries: BTreeMap<String, Vec<u8>>,
    expanded_bytes: usize,
}

impl VerifiedBundle {
    pub fn entry(&self, name: &str) -> Option<&[u8]> {
        self.entries.get(name).map(Vec::as_slice)
    }

    pub fn entries(&self) -> impl ExactSizeIterator<Item = (&str, &[u8])> {
        self.entries
            .iter()
            .map(|(name, bytes)| (name.as_str(), bytes.as_slice()))
    }

    #[cfg(test)]
    pub(crate) fn from_verified_parts_for_test(
        receipt: VerificationReceipt,
        entries: BTreeMap<String, Vec<u8>>,
    ) -> Self {
        let expanded_bytes = entries.values().map(Vec::len).sum();
        Self {
            receipt,
            entries,
            expanded_bytes,
        }
    }

    pub const fn expanded_bytes(&self) -> usize {
        self.expanded_bytes
    }
}

impl fmt::Debug for VerifiedBundle {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("VerifiedBundle")
            .field("receipt", &self.receipt)
            .field("entry_count", &self.entries.len())
            .field("expanded_bytes", &self.expanded_bytes)
            .finish()
    }
}

#[derive(Clone, Copy, Debug)]
pub struct VerificationContext<'a> {
    pub now_unix: u64,
    pub minimum_revocation_sequence: u64,
    pub supported_sdk_minor: u16,
    pub trusted_publishers: &'a [TrustedPublisher],
    pub revocation: &'a RevocationSnapshot,
}

pub fn read_and_verify_local_bundle(
    path: &Path,
    context: VerificationContext<'_>,
) -> Result<VerifiedBundle, BundleError> {
    let before = fs::symlink_metadata(path)
        .map_err(|error| BundleError::new(BundleErrorCode::Archive, error.to_string()))?;
    if before.file_type().is_symlink() || !before.is_file() {
        return Err(BundleError::new(
            BundleErrorCode::LinkRejected,
            "bundle must be a regular non-link file",
        ));
    }
    if before.len() > Limits::BUNDLE_BYTES as u64 {
        return Err(BundleError::new(
            BundleErrorCode::SourceTooLarge,
            "bundle exceeds the accepted byte limit",
        ));
    }
    let mut options = OpenOptions::new();
    options.read(true);
    apply_no_follow(&mut options);
    let mut file = options
        .open(path)
        .map_err(|error| BundleError::new(BundleErrorCode::Archive, error.to_string()))?;
    let opened = file
        .metadata()
        .map_err(|error| BundleError::new(BundleErrorCode::Archive, error.to_string()))?;
    if opened.len() != before.len() || !same_identity(&before, &opened) {
        return Err(BundleError::new(
            BundleErrorCode::SourceChanged,
            "bundle changed before it was opened",
        ));
    }
    let mut bytes =
        Vec::with_capacity(usize::try_from(opened.len()).unwrap_or(Limits::BUNDLE_BYTES));
    file.by_ref()
        .take(Limits::BUNDLE_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| BundleError::new(BundleErrorCode::Archive, error.to_string()))?;
    let after = file
        .metadata()
        .map_err(|error| BundleError::new(BundleErrorCode::Archive, error.to_string()))?;
    let path_after = fs::symlink_metadata(path).map_err(|_| {
        BundleError::new(
            BundleErrorCode::SourceChanged,
            "bundle path changed while reading",
        )
    })?;
    if bytes.len() > Limits::BUNDLE_BYTES
        || !same_snapshot(&opened, &after)
        || !same_identity(&after, &path_after)
        || after.len() != bytes.len() as u64
    {
        return Err(BundleError::new(
            BundleErrorCode::SourceChanged,
            "bundle changed while reading",
        ));
    }
    verify_bundle_bytes(&bytes, context)
}

pub fn verify_bundle_bytes(
    bytes: &[u8],
    context: VerificationContext<'_>,
) -> Result<VerifiedBundle, BundleError> {
    if bytes.len() > Limits::BUNDLE_BYTES {
        return Err(BundleError::new(
            BundleErrorCode::SourceTooLarge,
            "bundle exceeds the accepted byte limit",
        ));
    }
    let package_sha256 = sha256(bytes);
    if !context
        .revocation
        .is_current(context.now_unix, context.minimum_revocation_sequence)
    {
        return Err(BundleError::new(
            BundleErrorCode::StaleRevocation,
            "revocation state is expired, future-dated, or below the rollback floor",
        ));
    }

    let mut archive = ZipArchive::new(Cursor::new(bytes))
        .map_err(|error| BundleError::new(BundleErrorCode::Archive, error.to_string()))?;
    if archive.is_empty() || archive.len() > Limits::PACKAGE_FILES {
        return Err(BundleError::new(
            BundleErrorCode::TooManyFiles,
            "package file count is empty or exceeds the accepted limit",
        ));
    }
    let mut entries = BTreeMap::new();
    let mut folded = BTreeSet::new();
    let mut expanded = 0usize;
    for index in 0..archive.len() {
        let mut file = archive.by_index(index).map_err(|error| {
            BundleError::new(BundleErrorCode::Archive, error.to_string())
        })?;
        let raw_name = std::str::from_utf8(file.name_raw())
            .map_err(|_| {
                BundleError::new(BundleErrorCode::UnsafeEntry, "entry name is not UTF-8")
            })?
            .to_owned();
        if raw_name != file.name()
            || !safe_relative_path(&raw_name)
            || file.encrypted()
            || file.is_dir()
            || !file.is_file()
            || !regular_unix_mode(file.unix_mode())
        {
            return Err(BundleError::new(
                BundleErrorCode::UnsafeEntry,
                format!(
                    "entry {raw_name:?} is encrypted, linked, special, directory, or unsafe"
                ),
            ));
        }
        let folded_name = raw_name.to_lowercase();
        if entries.contains_key(&raw_name) || !folded.insert(folded_name) {
            return Err(BundleError::new(
                BundleErrorCode::DuplicateEntry,
                format!("entry {raw_name:?} collides with another portable path"),
            ));
        }
        let declared = usize::try_from(file.size()).map_err(|_| {
            BundleError::new(
                BundleErrorCode::EntryTooLarge,
                "entry size does not fit this platform",
            )
        })?;
        expanded = expanded.checked_add(declared).ok_or_else(|| {
            BundleError::new(BundleErrorCode::ExpansionLimit, "expanded size overflow")
        })?;
        if expanded > Limits::EXPANDED_BYTES {
            return Err(BundleError::new(
                BundleErrorCode::ExpansionLimit,
                "expanded package exceeds the accepted limit",
            ));
        }
        if raw_name == "manifest.json" && declared > Limits::MANIFEST_BYTES {
            return Err(BundleError::new(
                BundleErrorCode::EntryTooLarge,
                "manifest exceeds its accepted limit",
            ));
        }
        if raw_name == "component.wasm" && declared > Limits::COMPONENT_BYTES {
            return Err(BundleError::new(
                BundleErrorCode::EntryTooLarge,
                "component exceeds its accepted limit",
            ));
        }
        let remaining = Limits::EXPANDED_BYTES.saturating_sub(expanded - declared);
        let mut content = Vec::with_capacity(declared.min(remaining));
        file.by_ref()
            .take(remaining as u64 + 1)
            .read_to_end(&mut content)
            .map_err(|error| {
                BundleError::new(BundleErrorCode::Archive, error.to_string())
            })?;
        if content.len() != declared || content.len() > remaining {
            return Err(BundleError::new(
                BundleErrorCode::ExpansionLimit,
                "entry output differs from its bounded declared size",
            ));
        }
        entries.insert(raw_name, content);
    }
    for required in REQUIRED_PACKAGE_ENTRIES {
        if !entries.contains_key(required) {
            return Err(BundleError::new(
                BundleErrorCode::MissingEntry,
                format!("required entry {required:?} is missing"),
            ));
        }
    }

    let manifest: EcosystemManifest =
        decode_strict_json(entry(&entries, "manifest.json")?, Limits::MANIFEST_BYTES)?;
    manifest
        .validate(context.supported_sdk_minor)
        .map_err(|error| {
            BundleError::new(BundleErrorCode::Manifest, error.to_string())
        })?;
    let component = entry(&entries, "component.wasm")?;
    if component.is_empty()
        || component.len() > Limits::COMPONENT_BYTES
        || &component[..4.min(component.len())] != b"\0asm"
    {
        return Err(BundleError::new(
            BundleErrorCode::Component,
            "component.wasm is empty, oversized, or lacks the WebAssembly magic",
        ));
    }
    if let Some(action_pack_entry) = &manifest.action_pack_entry {
        if !entries.contains_key(action_pack_entry) {
            return Err(BundleError::new(
                BundleErrorCode::MissingEntry,
                "manifest action pack entry is absent",
            ));
        }
    }
    let sbom: SpdxDocument =
        decode_strict_json(entry(&entries, "sbom.spdx.json")?, Limits::MANIFEST_BYTES)?;
    if sbom.spdx_version != "SPDX-2.3"
        || sbom.data_license != "CC0-1.0"
        || !safe_text(&sbom.name, false)
    {
        return Err(BundleError::new(
            BundleErrorCode::Sbom,
            "SBOM must be bounded SPDX-2.3 JSON with the CC0-1.0 data license",
        ));
    }
    if entry(&entries, "LICENSES.txt")?.is_empty() {
        return Err(BundleError::new(
            BundleErrorCode::Licenses,
            "license inventory is empty",
        ));
    }

    let content_sha256 = content_digest(&entries);
    let provenance_bytes = entry(&entries, PROVENANCE_ENTRY)?;
    let provenance_sha256 = sha256(provenance_bytes);
    let provenance: ProvenanceStatement =
        decode_strict_json(provenance_bytes, Limits::MANIFEST_BYTES)?;
    let signature: SignatureBundle =
        decode_strict_json(entry(&entries, SIGNATURE_ENTRY)?, Limits::MANIFEST_BYTES)?;
    verify_provenance(&manifest, &provenance, &content_sha256, context.now_unix)?;
    verify_signature(
        &manifest,
        &signature,
        &package_sha256,
        &content_sha256,
        &provenance_sha256,
        context,
    )?;

    Ok(VerifiedBundle {
        receipt: VerificationReceipt {
            schema_version: 1,
            extension_id: manifest.extension_id.clone(),
            version: manifest.version.clone(),
            publisher_id: manifest.publisher_id.clone(),
            package_sha256,
            content_sha256,
            provenance_sha256,
            sbom_sha256: sha256(entry(&entries, "sbom.spdx.json")?),
            licenses_sha256: sha256(entry(&entries, "LICENSES.txt")?),
            key_id: signature.key_id,
            source_uri: provenance.source_uri,
            source_revision: provenance.source_revision,
            builder_id: provenance.builder_id,
            built_at_unix: provenance.built_at_unix,
            signature_expires_at_unix: signature.expires_at_unix,
            verified_at_unix: context.now_unix,
            revocation_sequence: context.revocation.sequence,
            manifest,
        },
        entries,
        expanded_bytes: expanded,
    })
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SpdxDocument {
    #[serde(rename = "spdxVersion")]
    spdx_version: String,
    #[serde(rename = "dataLicense")]
    data_license: String,
    name: String,
}

fn verify_provenance(
    manifest: &EcosystemManifest,
    provenance: &ProvenanceStatement,
    content_sha256: &str,
    now_unix: u64,
) -> Result<(), BundleError> {
    if provenance.schema_version != 1
        || provenance.content_sha256 != content_sha256
        || provenance.publisher_id != manifest.publisher_id
        || provenance.extension_id != manifest.extension_id
        || provenance.version != manifest.version
        || !provenance.source_uri.starts_with("https://")
        || !safe_text(&provenance.source_uri, false)
        || !safe_text(&provenance.source_revision, false)
        || !portable_identifier(&provenance.builder_id)
        || provenance.built_at_unix == 0
        || provenance.built_at_unix > now_unix
    {
        return Err(BundleError::new(
            BundleErrorCode::Provenance,
            "provenance does not bind the exact content, identity, source, builder, version, and time",
        ));
    }
    Ok(())
}

fn verify_signature(
    manifest: &EcosystemManifest,
    signature_bundle: &SignatureBundle,
    package_sha256: &str,
    content_sha256: &str,
    provenance_sha256: &str,
    context: VerificationContext<'_>,
) -> Result<(), BundleError> {
    if signature_bundle.schema_version != 1
        || signature_bundle.content_sha256 != content_sha256
        || signature_bundle.provenance_sha256 != provenance_sha256
        || signature_bundle.publisher_id != manifest.publisher_id
        || !portable_identifier(&signature_bundle.key_id)
        || !valid_digest(content_sha256)
        || !valid_digest(provenance_sha256)
    {
        return Err(BundleError::new(
            BundleErrorCode::ContentDigest,
            "signature bundle does not bind the verified content and provenance digests",
        ));
    }
    if signature_bundle.issued_at_unix > context.now_unix
        || context.now_unix >= signature_bundle.expires_at_unix
    {
        return Err(BundleError::new(
            BundleErrorCode::ExpiredSignature,
            "signature is future-dated or expired",
        ));
    }
    if context.revocation.is_revoked(
        &manifest.publisher_id,
        &signature_bundle.key_id,
        package_sha256,
    ) {
        return Err(BundleError::new(
            BundleErrorCode::Revoked,
            "publisher, key, or exact package is revoked",
        ));
    }
    let trusted = context
        .trusted_publishers
        .iter()
        .find(|trusted| {
            trusted.publisher_id == manifest.publisher_id
                && trusted.key_id == signature_bundle.key_id
                && trusted.public_key_base64 == signature_bundle.public_key_base64
                && trusted.valid_from_unix <= context.now_unix
                && context.now_unix < trusted.valid_until_unix
        })
        .ok_or_else(|| {
            BundleError::new(
                BundleErrorCode::UntrustedPublisher,
                "signer is not present and current in the trusted root",
            )
        })?;
    let public_key = decode_fixed::<32>(&trusted.public_key_base64, "public key")?;
    let signature = decode_fixed::<64>(&signature_bundle.signature_base64, "signature")?;
    let verifying_key = VerifyingKey::from_bytes(&public_key).map_err(|_| {
        BundleError::new(BundleErrorCode::Signature, "public key is invalid")
    })?;
    let signature = Signature::from_bytes(&signature);
    verifying_key
        .verify_strict(
            &signature_message(manifest, content_sha256, provenance_sha256),
            &signature,
        )
        .map_err(|_| {
            BundleError::new(
                BundleErrorCode::Signature,
                "detached signature verification failed",
            )
        })
}

pub fn signature_message(
    manifest: &EcosystemManifest,
    content_sha256: &str,
    provenance_sha256: &str,
) -> Vec<u8> {
    [
        "automexia-ecosystem-signature-v1",
        &manifest.publisher_id,
        &manifest.extension_id,
        &manifest.version,
        content_sha256,
        provenance_sha256,
    ]
    .join("\0")
    .into_bytes()
}

pub fn content_digest(entries: &BTreeMap<String, Vec<u8>>) -> String {
    let mut digest = Sha256::new();
    digest.update(b"automexia-ecosystem-content-v1\0");
    for (name, bytes) in entries {
        if name == SIGNATURE_ENTRY || name == PROVENANCE_ENTRY {
            continue;
        }
        digest.update(name.as_bytes());
        digest.update(b"\0");
        digest.update(sha256(bytes).as_bytes());
        digest.update(b"\n");
    }
    hex(&digest.finalize())
}

fn entry<'a>(
    entries: &'a BTreeMap<String, Vec<u8>>,
    name: &str,
) -> Result<&'a [u8], BundleError> {
    entries.get(name).map(Vec::as_slice).ok_or_else(|| {
        BundleError::new(
            BundleErrorCode::MissingEntry,
            format!("required entry {name:?} is missing"),
        )
    })
}

fn decode_fixed<const N: usize>(
    source: &str,
    role: &str,
) -> Result<[u8; N], BundleError> {
    let decoded = BASE64.decode(source).map_err(|_| {
        BundleError::new(
            BundleErrorCode::Signature,
            format!("{role} is not canonical base64"),
        )
    })?;
    decoded.try_into().map_err(|_| {
        BundleError::new(
            BundleErrorCode::Signature,
            format!("{role} has the wrong length"),
        )
    })
}

fn regular_unix_mode(mode: Option<u32>) -> bool {
    mode.is_none_or(|mode| matches!(mode & 0o170_000, 0 | 0o100_000))
}

fn sha256(bytes: &[u8]) -> String {
    hex(&Sha256::digest(bytes))
}

fn hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output
}

fn apply_no_follow(options: &mut OpenOptions) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        const O_NOFOLLOW: i32 = 0x20000;
        options.custom_flags(O_NOFOLLOW);
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt as _;
        const FILE_FLAG_OPEN_REPARSE_POINT: u32 = 0x0020_0000;
        options.custom_flags(FILE_FLAG_OPEN_REPARSE_POINT);
    }
}

#[cfg(unix)]
fn same_identity(left: &fs::Metadata, right: &fs::Metadata) -> bool {
    use std::os::unix::fs::MetadataExt as _;
    left.dev() == right.dev() && left.ino() == right.ino()
}

#[cfg(windows)]
fn same_identity(left: &fs::Metadata, right: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt as _;
    left.creation_time() == right.creation_time()
        && left.file_size() == right.file_size()
        && left.file_attributes() == right.file_attributes()
}

#[cfg(not(any(unix, windows)))]
fn same_identity(left: &fs::Metadata, right: &fs::Metadata) -> bool {
    left.len() == right.len() && left.modified().ok() == right.modified().ok()
}

#[cfg(unix)]
fn same_snapshot(left: &fs::Metadata, right: &fs::Metadata) -> bool {
    use std::os::unix::fs::MetadataExt as _;
    same_identity(left, right)
        && left.size() == right.size()
        && left.mtime() == right.mtime()
        && left.mtime_nsec() == right.mtime_nsec()
}

#[cfg(windows)]
fn same_snapshot(left: &fs::Metadata, right: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt as _;
    same_identity(left, right)
        && left.file_size() == right.file_size()
        && left.last_write_time() == right.last_write_time()
}

#[cfg(not(any(unix, windows)))]
fn same_snapshot(left: &fs::Metadata, right: &fs::Metadata) -> bool {
    same_identity(left, right)
}

#[cfg(test)]
mod tests {
    use std::io::{Cursor, Write as _};

    use automexia_ecosystem::{Compatibility, ExtensionKind, ModelRisk, WIT_WORLD};
    use ed25519_dalek::{Signer as _, SigningKey};
    use zip::{write::SimpleFileOptions, CompressionMethod, ZipWriter};

    use super::*;

    struct Fixture {
        bundle: Vec<u8>,
        trusted: TrustedPublisher,
        revocation: RevocationSnapshot,
    }

    fn manifest() -> EcosystemManifest {
        EcosystemManifest {
            schema_version: 1,
            extension_id: "example.inspect".into(),
            display_name: "Example Inspect".into(),
            description: "Bounded example".into(),
            publisher_id: "example.publisher".into(),
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
        }
    }

    fn fixture_with(extra: Option<(&str, &[u8], u32)>) -> Fixture {
        let manifest = manifest();
        let mut entries = BTreeMap::from([
            (
                "manifest.json".into(),
                serde_json::to_vec(&manifest).unwrap(),
            ),
            ("component.wasm".into(), b"\0asm\x0d\0\x01\0".to_vec()),
            (
                "sbom.spdx.json".into(),
                br#"{"spdxVersion":"SPDX-2.3","dataLicense":"CC0-1.0","name":"example"}"#
                    .to_vec(),
            ),
            ("LICENSES.txt".into(), b"MIT\n".to_vec()),
        ]);
        if let Some((name, bytes, _)) = extra {
            entries.insert(name.into(), bytes.to_vec());
        }
        let content_sha256 = content_digest(&entries);
        let provenance = ProvenanceStatement {
            schema_version: 1,
            content_sha256: content_sha256.clone(),
            publisher_id: manifest.publisher_id.clone(),
            extension_id: manifest.extension_id.clone(),
            version: manifest.version.clone(),
            source_uri: "https://example.invalid/source".into(),
            source_revision: "0123456789abcdef".into(),
            builder_id: "example.builder".into(),
            built_at_unix: 50,
        };
        let provenance_bytes = serde_json::to_vec(&provenance).unwrap();
        let provenance_sha256 = sha256(&provenance_bytes);
        let signing = SigningKey::from_bytes(&[7; 32]);
        let signature = signing.sign(&signature_message(
            &manifest,
            &content_sha256,
            &provenance_sha256,
        ));
        let public_key_base64 = BASE64.encode(signing.verifying_key().to_bytes());
        let signature_bundle = SignatureBundle {
            schema_version: 1,
            content_sha256,
            provenance_sha256,
            publisher_id: manifest.publisher_id.clone(),
            key_id: "example.key".into(),
            public_key_base64: public_key_base64.clone(),
            signature_base64: BASE64.encode(signature.to_bytes()),
            issued_at_unix: 50,
            expires_at_unix: 200,
        };
        entries.insert(PROVENANCE_ENTRY.into(), provenance_bytes);
        entries.insert(
            SIGNATURE_ENTRY.into(),
            serde_json::to_vec(&signature_bundle).unwrap(),
        );
        let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
        for (name, bytes) in entries {
            let mode = extra
                .filter(|(extra_name, _, _)| *extra_name == name)
                .map_or(0o100600, |(_, _, mode)| mode);
            let options = SimpleFileOptions::default()
                .compression_method(CompressionMethod::Deflated)
                .unix_permissions(mode);
            if mode & 0o170_000 == 0o120_000 {
                writer
                    .add_symlink(name, String::from_utf8_lossy(&bytes), options)
                    .unwrap();
            } else {
                writer.start_file(name, options).unwrap();
                writer.write_all(&bytes).unwrap();
            }
        }
        let bundle = writer.finish().unwrap().into_inner();
        Fixture {
            bundle,
            trusted: TrustedPublisher {
                publisher_id: manifest.publisher_id,
                key_id: "example.key".into(),
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

    fn verify(fixture: &Fixture) -> Result<VerifiedBundle, BundleError> {
        verify_bundle_bytes(
            &fixture.bundle,
            VerificationContext {
                now_unix: 100,
                minimum_revocation_sequence: 4,
                supported_sdk_minor: 0,
                trusted_publishers: std::slice::from_ref(&fixture.trusted),
                revocation: &fixture.revocation,
            },
        )
    }

    #[test]
    fn signed_bundle_verifies_before_bytes_are_available_for_install() {
        let fixture = fixture_with(None);
        let verified = verify(&fixture).unwrap();
        assert_eq!(verified.receipt.extension_id, "example.inspect");
        assert!(valid_digest(&verified.receipt.package_sha256));
        assert_eq!(verified.entry("LICENSES.txt"), Some(b"MIT\n".as_slice()));
        assert!(!format!("{verified:?}").contains("LICENSES"));
        let _risk_contract = ModelRisk::ReadOnly;
    }

    #[test]
    fn tamper_expiry_revocation_and_stale_metadata_fail_closed() {
        let mut fixture = fixture_with(None);
        let last = fixture.bundle.len() - 1;
        fixture.bundle[last] ^= 1;
        assert!(verify(&fixture).is_err());

        let mut fixture = fixture_with(None);
        fixture.revocation.valid_until_unix = 100;
        assert_eq!(
            verify(&fixture).unwrap_err().code,
            BundleErrorCode::StaleRevocation
        );

        let mut fixture = fixture_with(None);
        fixture.revocation.revoked_keys.push("example.key".into());
        assert_eq!(verify(&fixture).unwrap_err().code, BundleErrorCode::Revoked);
    }

    #[test]
    fn linked_and_traversal_entries_are_rejected_without_extraction() {
        let linked = fixture_with(Some(("payload/link", b"target", 0o120777)));
        assert_eq!(
            verify(&linked).unwrap_err().code,
            BundleErrorCode::UnsafeEntry
        );
        let traversal = fixture_with(Some(("../escape", b"bad", 0o100600)));
        assert_eq!(
            verify(&traversal).unwrap_err().code,
            BundleErrorCode::UnsafeEntry
        );
    }
}
