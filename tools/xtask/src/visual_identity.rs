//! Baseline compatibility and physical geometry for the existing pixel oracle.
//! Metadata is evidence supplied by a capture owner, not hardware attestation.

use super::{
    read_bounded, TaskResult, MAX_CONFIG_BYTES, MAX_DIMENSION, MAX_ENCODED_IMAGE_BYTES,
    MAX_PIXELS,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct Identity {
    platform: String,
    os_version: String,
    architecture: String,
    display_server: String,
    renderer: String,
    gpu: String,
    driver: String,
    shell: String,
    shell_version: String,
    font_hashes: Vec<String>,
    font_size_milli: u32,
    scale_milli: u32,
    width: u32,
    height: u32,
    theme: String,
    configuration_sha256: String,
    scenario: String,
    fixture: String,
    locale: String,
    evidence_kind: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Geometry {
    id: String,
    /// Physical pixels multiplied by 1000, never logical pixels or cells.
    rect_milli: [i64; 4],
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Metadata {
    schema: u32,
    identity: Identity,
    source_commit: String,
    source_dirty: bool,
    image_sha256: String,
    frame_generation: u64,
    geometry: Vec<Geometry>,
}

#[derive(Debug, Serialize)]
pub(super) struct BindingReport {
    pub width: u32,
    pub height: u32,
    pub changed_geometry: Vec<String>,
    evidence_kind: String,
    expected_commit: String,
    actual_commit: String,
    expected_dirty: bool,
    actual_dirty: bool,
    identity_verified: bool,
}

fn identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 96
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._+-".contains(&b))
}

fn hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn sha256_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut result = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        result.push(char::from(HEX[usize::from(byte >> 4)]));
        result.push(char::from(HEX[usize::from(byte & 15)]));
    }
    result
}

fn load_metadata(path: &Path) -> TaskResult<Metadata> {
    let bytes = read_bounded(path, MAX_CONFIG_BYTES)?;
    let metadata: Metadata = serde_json::from_slice(&bytes)
        .map_err(|_| "invalid visual baseline metadata schema".to_owned())?;
    let id = &metadata.identity;
    if metadata.schema != 1
        || !(hex(&metadata.source_commit, 40) || hex(&metadata.source_commit, 64))
        || !hex(&metadata.image_sha256, 64)
        || metadata.frame_generation == 0
        || !matches!(id.platform.as_str(), "windows" | "linux" | "macos")
        || !matches!(id.architecture.as_str(), "x86_64" | "aarch64")
        || !matches!(
            id.evidence_kind.as_str(),
            "controlled-raster" | "native-window" | "native-compositor"
        )
        || !matches!(
            id.display_server.as_str(),
            "controlled" | "win32" | "x11" | "wayland" | "quartz"
        )
        || !matches!(
            id.renderer.as_str(),
            "cpu"
                | "wgpu-dx12"
                | "wgpu-vulkan"
                | "wgpu-metal"
                | "wgpu-gl"
                | "vulkan"
                | "metal"
        )
        || !matches!(
            id.gpu.as_str(),
            "software" | "intel" | "amd" | "nvidia" | "apple" | "other"
        )
        || id.width == 0
        || id.height == 0
        || id.width > MAX_DIMENSION
        || id.height > MAX_DIMENSION
        || u64::from(id.width) * u64::from(id.height) > MAX_PIXELS
        || !(1000..=256_000).contains(&id.font_size_milli)
        || !(500..=8000).contains(&id.scale_milli)
        || id.font_hashes.is_empty()
        || id.font_hashes.len() > 16
        || id.font_hashes.iter().any(|v| !hex(v, 64))
        || !hex(&id.configuration_sha256, 64)
        || [
            &id.os_version,
            &id.driver,
            &id.shell,
            &id.shell_version,
            &id.theme,
            &id.scenario,
            &id.fixture,
            &id.locale,
        ]
        .into_iter()
        .any(|v| !identifier(v))
        || metadata.geometry.is_empty()
        || metadata.geometry.len() > 128
    {
        return Err("visual baseline metadata is outside bounded schema v1".into());
    }
    let mut names = BTreeSet::new();
    for geometry in &metadata.geometry {
        if !identifier(&geometry.id)
            || !names.insert(&geometry.id)
            || geometry
                .rect_milli
                .iter()
                .any(|v| !(-16_384_000..=16_384_000).contains(v))
            || geometry.rect_milli[2] < 0
            || geometry.rect_milli[3] < 0
        {
            return Err("visual baseline geometry is invalid or duplicated".into());
        }
    }
    Ok(metadata)
}

fn load(path: &Path, image: &Path) -> TaskResult<Metadata> {
    let metadata = load_metadata(path)?;
    let image_bytes = read_bounded(image, MAX_ENCODED_IMAGE_BYTES)?;
    let digest = sha256_hex(&image_bytes);
    if digest != metadata.image_sha256 {
        return Err("visual baseline image does not match its metadata digest".into());
    }
    Ok(metadata)
}

pub(super) fn validate(metadata: &Path, image: &Path) -> TaskResult {
    let metadata = load(metadata, image)?;
    let decoded = super::decode_bounded(image)?;
    if decoded.width() != metadata.identity.width
        || decoded.height() != metadata.identity.height
    {
        return Err("visual capture image dimensions disagree with metadata".into());
    }
    Ok(())
}

pub(super) fn compare(
    expected_metadata: &Path,
    actual_metadata: &Path,
    expected_image: &Path,
    actual_image: &Path,
) -> TaskResult<BindingReport> {
    let expected = load(expected_metadata, expected_image)?;
    let actual = load(actual_metadata, actual_image)?;
    compare_binding(expected, actual)
}

#[derive(Debug, Serialize)]
pub(super) struct ReceiptReport {
    schema: u32,
    pub status: &'static str,
    image_digest_matches: bool,
    expected_image_available: bool,
    binding: BindingReport,
}

pub(super) fn compare_receipts(
    expected_metadata: &Path,
    actual_metadata: &Path,
    actual_image: &Path,
) -> TaskResult<ReceiptReport> {
    let expected = load_metadata(expected_metadata)?;
    let actual = load(actual_metadata, actual_image)?;
    let decoded = super::decode_bounded(actual_image)?;
    if decoded.width() != actual.identity.width
        || decoded.height() != actual.identity.height
    {
        return Err("visual capture image dimensions disagree with metadata".into());
    }
    let image_digest_matches = expected.image_sha256 == actual.image_sha256;
    let binding = compare_binding(expected, actual)?;
    let passed = image_digest_matches && binding.changed_geometry.is_empty();
    Ok(ReceiptReport {
        schema: 3,
        status: if passed { "passed" } else { "failed" },
        image_digest_matches,
        expected_image_available: false,
        binding,
    })
}

fn compare_binding(expected: Metadata, actual: Metadata) -> TaskResult<BindingReport> {
    if expected.identity != actual.identity {
        return Err("incompatible visual baseline: platform, renderer, font, scale, configuration and scenario must match".into());
    }
    let left: BTreeMap<_, _> = expected
        .geometry
        .iter()
        .map(|v| (&v.id, v.rect_milli))
        .collect();
    let right: BTreeMap<_, _> = actual
        .geometry
        .iter()
        .map(|v| (&v.id, v.rect_milli))
        .collect();
    let names: BTreeSet<_> = left.keys().chain(right.keys()).copied().collect();
    let changed_geometry = names
        .into_iter()
        .filter(|name| left.get(*name) != right.get(*name))
        .cloned()
        .collect();
    Ok(BindingReport {
        width: expected.identity.width,
        height: expected.identity.height,
        evidence_kind: expected.identity.evidence_kind,
        changed_geometry,
        expected_commit: expected.source_commit,
        actual_commit: actual.source_commit,
        expected_dirty: expected.source_dirty,
        actual_dirty: actual.source_dirty,
        identity_verified: true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};

    fn metadata(image: &[u8]) -> Value {
        json!({
            "schema": 1,
            "source_commit": "1".repeat(40), "source_dirty": false,
            "image_sha256": sha256_hex(image), "frame_generation": 1,
            "identity": {
                "platform": "windows", "os_version": "test-v1", "architecture": "x86_64",
                "display_server": "controlled", "renderer": "cpu", "gpu": "software",
                "driver": "cpu-v1", "shell": "fixture", "shell_version": "1",
                "font_hashes": ["a".repeat(64)], "font_size_milli": 16000,
                "scale_milli": 1250, "width": 2, "height": 2, "theme": "dark",
                "configuration_sha256": "b".repeat(64), "scenario": "cursor",
                "fixture": "terminal-v1", "locale": "C.UTF-8", "evidence_kind": "controlled-raster"
            },
            "geometry": [{"id":"cursor", "rect_milli":[0,0,1000,2000]}]
        })
    }

    #[test]
    fn receipt_comparison_proves_exact_bytes_and_geometry_without_claiming_a_pixel_diff()
    {
        let dir = tempfile::tempdir().unwrap();
        let image = dir.path().join("actual.png");
        let left = dir.path().join("expected.json");
        let right = dir.path().join("actual.json");
        let report = dir.path().join("report.json");
        image_rs::RgbaImage::from_pixel(2, 2, image_rs::Rgba([20, 30, 40, 255]))
            .save(&image)
            .unwrap();
        let expected = metadata(&std::fs::read(&image).unwrap());
        std::fs::write(&left, serde_json::to_vec(&expected).unwrap()).unwrap();
        std::fs::write(&right, serde_json::to_vec(&expected).unwrap()).unwrap();
        let args = vec![
            "--compare-receipts".to_owned(),
            "--expected-metadata".into(),
            left.to_string_lossy().into_owned(),
            "--actual-metadata".into(),
            right.to_string_lossy().into_owned(),
            "--actual".into(),
            image.to_string_lossy().into_owned(),
            "--report".into(),
            report.to_string_lossy().into_owned(),
        ];
        super::super::dispatch(&args).unwrap();
        let value: Value =
            serde_json::from_slice(&std::fs::read(&report).unwrap()).unwrap();
        assert_eq!(value["schema"], 3);
        assert_eq!(value["status"], "passed");
        assert_eq!(value["image_digest_matches"], true);
        assert_eq!(value["expected_image_available"], false);
        assert!(value.get("changed_pixels").is_none());
        let mut moved = expected.clone();
        moved["geometry"][0]["rect_milli"][0] = json!(1000);
        std::fs::write(&right, serde_json::to_vec(&moved).unwrap()).unwrap();
        assert!(super::super::dispatch(&args).is_err());
        let value: Value =
            serde_json::from_slice(&std::fs::read(&report).unwrap()).unwrap();
        assert_eq!(value["image_digest_matches"], true);
        assert_eq!(value["binding"]["changed_geometry"], json!(["cursor"]));
        let mut incompatible = expected.clone();
        incompatible["identity"]["scale_milli"] = json!(1500);
        std::fs::write(&right, serde_json::to_vec(&incompatible).unwrap()).unwrap();
        let published = std::fs::read(&report).unwrap();
        assert!(super::super::dispatch(&args).is_err());
        assert_eq!(std::fs::read(&report).unwrap(), published);
        let mut alias = args.clone();
        alias[8] = alias[2].clone();
        let original = std::fs::read(&left).unwrap();
        assert!(super::super::dispatch(&alias).is_err());
        assert_eq!(std::fs::read(&left).unwrap(), original);
        image_rs::RgbaImage::from_pixel(2, 2, image_rs::Rgba([21, 30, 40, 255]))
            .save(&image)
            .unwrap();
        let changed = metadata(&std::fs::read(&image).unwrap());
        std::fs::write(&right, serde_json::to_vec(&changed).unwrap()).unwrap();
        assert!(super::super::dispatch(&args).is_err());
        let value: Value =
            serde_json::from_slice(&std::fs::read(&report).unwrap()).unwrap();
        assert_eq!(value["status"], "failed");
        assert_eq!(value["image_digest_matches"], false);
        let published = std::fs::read(&report).unwrap();
        std::fs::write(&left, b"{}").unwrap();
        assert!(super::super::dispatch(&args).is_err());
        assert_eq!(std::fs::read(&report).unwrap(), published);
    }

    #[test]
    fn individual_capture_validation_uses_the_same_schema_digest_and_decoder() {
        let dir = tempfile::tempdir().unwrap();
        let image = dir.path().join("image.png");
        let facts = dir.path().join("facts.json");
        image_rs::RgbaImage::new(2, 2).save(&image).unwrap();
        let mut data = metadata(&std::fs::read(&image).unwrap());
        std::fs::write(&facts, serde_json::to_vec(&data).unwrap()).unwrap();
        validate(&facts, &image).unwrap();
        data["identity"]["width"] = json!(3);
        std::fs::write(&facts, serde_json::to_vec(&data).unwrap()).unwrap();
        assert!(validate(&facts, &image).unwrap_err().contains("dimensions"));
        data["identity"]["width"] = json!("bad");
        std::fs::write(&facts, serde_json::to_vec(&data).unwrap()).unwrap();
        assert!(validate(&facts, &image).unwrap_err().contains("schema"));
    }

    #[test]
    fn incompatible_environments_are_not_visual_regressions_or_passes() {
        let dir = tempfile::tempdir().unwrap();
        let image = dir.path().join("fixture.png");
        let left = dir.path().join("left.json");
        let right = dir.path().join("right.json");
        std::fs::write(&image, b"digest fixture").unwrap();
        let baseline = metadata(b"digest fixture");
        std::fs::write(&left, serde_json::to_vec(&baseline).unwrap()).unwrap();
        for (key, value) in [
            ("platform", json!("macos")),
            ("os_version", json!("test-v2")),
            ("architecture", json!("aarch64")),
            ("display_server", json!("quartz")),
            ("renderer", json!("metal")),
            ("gpu", json!("apple")),
            ("driver", json!("cpu-v2")),
            ("shell", json!("bash")),
            ("shell_version", json!("2")),
            ("font_hashes", json!(["c".repeat(64)])),
            ("font_size_milli", json!(17000)),
            ("scale_milli", json!(1500)),
            ("width", json!(3)),
            ("height", json!(3)),
            ("theme", json!("light")),
            ("configuration_sha256", json!("c".repeat(64))),
            ("scenario", json!("selection")),
            ("fixture", json!("terminal-v2")),
            ("locale", json!("en-US")),
            ("evidence_kind", json!("native-window")),
        ] {
            let mut changed = baseline.clone();
            changed["identity"][key] = value;
            std::fs::write(&right, serde_json::to_vec(&changed).unwrap()).unwrap();
            assert!(
                compare(&left, &right, &image, &image)
                    .unwrap_err()
                    .contains("incompatible"),
                "{key}"
            );
        }
    }

    #[test]
    fn one_physical_pixel_geometry_drift_is_detected_even_with_identical_images() {
        let dir = tempfile::tempdir().unwrap();
        let image = dir.path().join("fixture.png");
        let left = dir.path().join("left.json");
        let right = dir.path().join("right.json");
        std::fs::write(&image, b"digest fixture").unwrap();
        let mut value = metadata(b"digest fixture");
        std::fs::write(&left, serde_json::to_vec(&value).unwrap()).unwrap();
        value["source_commit"] = json!("2".repeat(40));
        value["frame_generation"] = json!(7);
        std::fs::write(&right, serde_json::to_vec(&value).unwrap()).unwrap();
        assert!(compare(&left, &right, &image, &image)
            .unwrap()
            .changed_geometry
            .is_empty());
        value["geometry"][0]["rect_milli"][0] = json!(1000);
        std::fs::write(&right, serde_json::to_vec(&value).unwrap()).unwrap();
        assert_eq!(
            compare(&left, &right, &image, &image)
                .unwrap()
                .changed_geometry,
            ["cursor"]
        );
    }

    #[test]
    fn bound_dispatch_publishes_geometry_failure_and_never_overwrites_on_bad_identity() {
        let dir = tempfile::tempdir().unwrap();
        let expected = dir.path().join("expected.png");
        let actual = dir.path().join("actual.png");
        let left = dir.path().join("expected.json");
        let right = dir.path().join("actual.json");
        let policy = dir.path().join("policy.json");
        let diff = dir.path().join("diff.png");
        let report = dir.path().join("report.json");
        image_rs::RgbaImage::from_pixel(2, 2, image_rs::Rgba([20, 30, 40, 255]))
            .save(&expected)
            .unwrap();
        std::fs::copy(&expected, &actual).unwrap();
        let baseline = metadata(&std::fs::read(&expected).unwrap());
        let mut changed = baseline.clone();
        changed["geometry"][0]["rect_milli"][0] = json!(1000);
        std::fs::write(&left, serde_json::to_vec(&baseline).unwrap()).unwrap();
        std::fs::write(&right, serde_json::to_vec(&changed).unwrap()).unwrap();
        std::fs::write(&policy, br#"{"schema":1,"max_channel_delta":0,"max_changed_pixel_ratio":0.0,"masks":[]}"#).unwrap();
        let args: Vec<_> = [
            ("--expected", &expected),
            ("--actual", &actual),
            ("--config", &policy),
            ("--diff", &diff),
            ("--report", &report),
            ("--expected-metadata", &left),
            ("--actual-metadata", &right),
        ]
        .into_iter()
        .flat_map(|(key, path)| [key.to_owned(), path.to_string_lossy().into_owned()])
        .collect();
        assert!(super::super::dispatch(&args)
            .unwrap_err()
            .contains("pixel/geometry"));
        let published = std::fs::read(&report).unwrap();
        let result: Value = serde_json::from_slice(&published).unwrap();
        assert_eq!(result["schema"], 2);
        assert_eq!(result["status"], "failed");
        assert_eq!(result["changed_pixels"], 0);
        assert_eq!(result["binding"]["changed_geometry"], json!(["cursor"]));
        assert!(diff.is_file());
        changed["identity"]["scale_milli"] = json!(2000);
        std::fs::write(&right, serde_json::to_vec(&changed).unwrap()).unwrap();
        assert!(super::super::dispatch(&args)
            .unwrap_err()
            .contains("incompatible"));
        assert_eq!(std::fs::read(&report).unwrap(), published);
        std::fs::write(&right, serde_json::to_vec(&baseline).unwrap()).unwrap();
        super::super::dispatch(&args).unwrap();
        let result: Value =
            serde_json::from_slice(&std::fs::read(report).unwrap()).unwrap();
        assert_eq!(result["status"], "passed");
    }

    #[test]
    fn stale_images_duplicate_geometry_unknown_fields_and_invalid_bounds_fail_closed() {
        let dir = tempfile::tempdir().unwrap();
        let image = dir.path().join("fixture.png");
        let sidecar = dir.path().join("fixture.json");
        std::fs::write(&image, b"digest fixture").unwrap();
        let baseline = metadata(b"digest fixture");
        let mut cases = Vec::new();
        for (field, value) in [
            ("schema", json!(2)),
            ("frame_generation", json!(0)),
            ("image_sha256", json!("0".repeat(64))),
            ("secret", json!("unrecognized")),
            ("source_commit", json!("not-a-commit")),
            ("geometry", json!([])),
        ] {
            let mut changed = baseline.clone();
            changed[field] = value;
            cases.push(changed);
        }
        for (field, value) in [
            ("scale_milli", json!(0)),
            ("width", json!(9000)),
            ("font_hashes", json!([])),
            ("driver", json!("/private/path")),
        ] {
            let mut changed = baseline.clone();
            changed["identity"][field] = value;
            cases.push(changed);
        }
        let mut duplicate = baseline.clone();
        duplicate["geometry"]
            .as_array_mut()
            .unwrap()
            .push(baseline["geometry"][0].clone());
        cases.push(duplicate);
        for value in cases {
            std::fs::write(&sidecar, serde_json::to_vec(&value).unwrap()).unwrap();
            assert!(load(&sidecar, &image).is_err());
        }
    }
}
