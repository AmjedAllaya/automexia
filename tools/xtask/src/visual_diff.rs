use image_rs::{DynamicImage, ImageFormat, ImageReader, RgbaImage};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};

use super::TaskResult;

const MAX_CONFIG_BYTES: u64 = 64 * 1024;
const MAX_ENCODED_IMAGE_BYTES: u64 = 64 * 1024 * 1024;
const MAX_REPORT_BYTES: usize = 64 * 1024;
const MAX_DIMENSION: u32 = 8_192;
const MAX_PIXELS: u64 = 40_000_000;
const MAX_MASKS: usize = 32;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct VisualDiffConfig {
    schema: u32,
    max_channel_delta: u8,
    max_changed_pixel_ratio: f64,
    masks: Vec<Mask>,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Mask {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

#[derive(Debug, Serialize)]
struct VisualDiffReport {
    schema: u32,
    status: &'static str,
    width: u32,
    height: u32,
    compared_pixels: u64,
    masked_pixels: u64,
    changed_pixels: u64,
    changed_channels: u64,
    changed_pixel_ratio: f64,
    first_changed_pixel: Option<[u32; 2]>,
    changed_bounds: Option<[u32; 4]>,
    max_observed_channel_delta: u8,
    max_channel_delta: u8,
    max_changed_pixel_ratio: f64,
}

struct Comparison {
    report: VisualDiffReport,
    diff: RgbaImage,
}

pub(super) fn dispatch(args: &[String]) -> TaskResult {
    let arguments = parse_arguments(args)?;
    let expected = arguments
        .get("--expected")
        .ok_or_else(|| usage().to_string())?;
    let actual = arguments
        .get("--actual")
        .ok_or_else(|| usage().to_string())?;
    let config = arguments
        .get("--config")
        .ok_or_else(|| usage().to_string())?;
    let diff = arguments.get("--diff").ok_or_else(|| usage().to_string())?;
    let report = arguments
        .get("--report")
        .ok_or_else(|| usage().to_string())?;

    let expected = canonical_input(Path::new(expected))?;
    let actual = canonical_input(Path::new(actual))?;
    let config = canonical_input(Path::new(config))?;
    let diff = safe_output(Path::new(diff))?;
    let report = safe_output(Path::new(report))?;
    let distinct = [&expected, &actual, &config, &diff, &report];
    for left in 0..distinct.len() {
        for right in (left + 1)..distinct.len() {
            if distinct[left] == distinct[right] {
                return Err("visual-diff inputs and outputs must be distinct".into());
            }
        }
    }

    let config = load_config(&config)?;
    let comparison = compare(&expected, &actual, &config)?;
    write_diff_atomic(&diff, &comparison.diff)?;
    write_report_atomic(&report, &comparison.report)?;
    if comparison.report.status == "passed" {
        println!(
            "PASS: visual diff changed {:.6}% of compared pixels",
            comparison.report.changed_pixel_ratio * 100.0
        );
        Ok(())
    } else {
        Err(format!(
            "visual diff exceeded policy: {:.6}% changed (limit {:.6}%)",
            comparison.report.changed_pixel_ratio * 100.0,
            comparison.report.max_changed_pixel_ratio * 100.0
        ))
    }
}

fn usage() -> &'static str {
    "visual-diff requires --expected PATH --actual PATH --config PATH --diff PATH --report PATH"
}

fn parse_arguments(args: &[String]) -> TaskResult<BTreeMap<&str, &str>> {
    if args.len() != 10 {
        return Err(usage().into());
    }
    let allowed = ["--expected", "--actual", "--config", "--diff", "--report"];
    let mut parsed = BTreeMap::new();
    for pair in args.chunks_exact(2) {
        if !allowed.contains(&pair[0].as_str()) || pair[1].is_empty() {
            return Err(usage().into());
        }
        if parsed.insert(pair[0].as_str(), pair[1].as_str()).is_some() {
            return Err(format!("duplicate visual-diff option: {}", pair[0]));
        }
    }
    if parsed.len() != allowed.len() {
        return Err(usage().into());
    }
    Ok(parsed)
}

fn canonical_input(path: &Path) -> TaskResult<PathBuf> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("cannot inspect visual-diff input: {error}"))?;
    if metadata.file_type().is_symlink() || !metadata.file_type().is_file() {
        return Err("visual-diff inputs must be non-symlink regular files".into());
    }
    path.canonicalize()
        .map_err(|error| format!("cannot resolve visual-diff input: {error}"))
}

fn safe_output(path: &Path) -> TaskResult<PathBuf> {
    if let Ok(metadata) = fs::symlink_metadata(path) {
        if metadata.file_type().is_symlink() || !metadata.file_type().is_file() {
            return Err("visual-diff outputs must be regular files when present".into());
        }
        return path
            .canonicalize()
            .map_err(|error| format!("cannot resolve visual-diff output: {error}"));
    }
    let file_name = path
        .file_name()
        .ok_or_else(|| "visual-diff output requires a file name".to_string())?;
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let parent = parent.canonicalize().map_err(|error| {
        format!("cannot resolve visual-diff output directory: {error}")
    })?;
    if !parent.is_dir() {
        return Err("visual-diff output parent must be a directory".into());
    }
    Ok(parent.join(file_name))
}

fn read_bounded(path: &Path, maximum: u64) -> TaskResult<Vec<u8>> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("cannot inspect bounded input: {error}"))?;
    if metadata.file_type().is_symlink()
        || !metadata.file_type().is_file()
        || metadata.len() > maximum
    {
        return Err("visual-diff input is not a bounded regular file".into());
    }
    let mut file = File::open(path)
        .map_err(|error| format!("cannot open bounded input: {error}"))?;
    let opened = file
        .metadata()
        .map_err(|error| format!("cannot inspect opened input: {error}"))?;
    if !opened.is_file() || opened.len() > maximum {
        return Err("visual-diff opened input exceeds its bound".into());
    }
    let capacity =
        usize::try_from(opened.len()).map_err(|_| "visual-diff input is too large")?;
    let mut bytes = Vec::with_capacity(capacity);
    Read::by_ref(&mut file)
        .take(maximum + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("cannot read bounded input: {error}"))?;
    if bytes.len() as u64 > maximum || bytes.len() as u64 != opened.len() {
        return Err("visual-diff input changed while it was read".into());
    }
    Ok(bytes)
}

fn load_config(path: &Path) -> TaskResult<VisualDiffConfig> {
    let bytes = read_bounded(path, MAX_CONFIG_BYTES)?;
    let config: VisualDiffConfig = serde_json::from_slice(&bytes)
        .map_err(|error| format!("invalid visual-diff policy: {error}"))?;
    if config.schema != 1
        || !config.max_changed_pixel_ratio.is_finite()
        || !(0.0..=1.0).contains(&config.max_changed_pixel_ratio)
        || config.masks.len() > MAX_MASKS
    {
        return Err("visual-diff policy is outside bounded schema v1".into());
    }
    Ok(config)
}

fn decode_bounded(path: &Path) -> TaskResult<RgbaImage> {
    let bytes = read_bounded(path, MAX_ENCODED_IMAGE_BYTES)?;
    if bytes.is_empty() {
        return Err("visual-diff image is empty".into());
    }
    let guessed = ImageReader::new(Cursor::new(&bytes))
        .with_guessed_format()
        .map_err(|error| format!("cannot detect visual-diff image: {error}"))?;
    let format = guessed
        .format()
        .ok_or_else(|| "visual-diff image format is unsupported".to_string())?;
    if format != ImageFormat::Png {
        return Err("visual-diff accepts PNG images only".into());
    }
    let (width, height) = ImageReader::with_format(Cursor::new(&bytes), format)
        .into_dimensions()
        .map_err(|error| format!("cannot read visual-diff dimensions: {error}"))?;
    let pixels = u64::from(width).saturating_mul(u64::from(height));
    if width == 0
        || height == 0
        || width > MAX_DIMENSION
        || height > MAX_DIMENSION
        || pixels > MAX_PIXELS
    {
        return Err("visual-diff decoded dimensions exceed policy".into());
    }

    let mut reader = ImageReader::with_format(Cursor::new(&bytes), format);
    let mut limits = image_rs::Limits::default();
    limits.max_image_width = Some(MAX_DIMENSION);
    limits.max_image_height = Some(MAX_DIMENSION);
    limits.max_alloc = Some(MAX_PIXELS * 4);
    reader.limits(limits);
    reader
        .decode()
        .map(DynamicImage::into_rgba8)
        .map_err(|error| format!("cannot decode bounded visual-diff image: {error}"))
}

fn validate_masks(config: &VisualDiffConfig, width: u32, height: u32) -> TaskResult<()> {
    for mask in &config.masks {
        let right = mask.x.checked_add(mask.width);
        let bottom = mask.y.checked_add(mask.height);
        if mask.width == 0
            || mask.height == 0
            || right.is_none_or(|right| right > width)
            || bottom.is_none_or(|bottom| bottom > height)
        {
            return Err("visual-diff mask is empty, overflowing, or out of bounds".into());
        }
    }
    Ok(())
}

fn mask_intervals(config: &VisualDiffConfig, height: u32) -> Vec<Vec<(u32, u32)>> {
    let mut rows = vec![Vec::new(); height as usize];
    for mask in &config.masks {
        for row in mask.y..(mask.y + mask.height) {
            rows[row as usize].push((mask.x, mask.x + mask.width));
        }
    }
    for intervals in &mut rows {
        intervals.sort_unstable();
        let mut merged: Vec<(u32, u32)> = Vec::with_capacity(intervals.len());
        for &(start, end) in intervals.iter() {
            match merged.last_mut() {
                Some((_, previous_end)) if start <= *previous_end => {
                    *previous_end = (*previous_end).max(end);
                }
                _ => merged.push((start, end)),
            }
        }
        *intervals = merged;
    }
    rows
}

fn compare(
    expected_path: &Path,
    actual_path: &Path,
    config: &VisualDiffConfig,
) -> TaskResult<Comparison> {
    let expected = decode_bounded(expected_path)?;
    let actual = decode_bounded(actual_path)?;
    if expected.dimensions() != actual.dimensions() {
        return Err("visual-diff images must have identical dimensions".into());
    }
    let (width, height) = expected.dimensions();
    validate_masks(config, width, height)?;
    let masks = mask_intervals(config, height);
    let expected_bytes = expected.as_raw();
    let actual_bytes = actual.as_raw();
    let mut diff_bytes = vec![0_u8; expected_bytes.len()];
    let mut compared_pixels = 0_u64;
    let mut masked_pixels = 0_u64;
    let mut changed_pixels = 0_u64;
    let mut changed_channels = 0_u64;
    let mut first_changed_pixel = None;
    let mut minimum_changed_x = width;
    let mut minimum_changed_y = height;
    let mut maximum_changed_x = 0_u32;
    let mut maximum_changed_y = 0_u32;
    let mut max_observed_channel_delta = 0_u8;

    for y in 0..height {
        let intervals = &masks[y as usize];
        let mut interval = 0_usize;
        for x in 0..width {
            while interval < intervals.len() && x >= intervals[interval].1 {
                interval += 1;
            }
            let offset = ((u64::from(y) * u64::from(width) + u64::from(x)) * 4) as usize;
            let expected_pixel = &expected_bytes[offset..offset + 4];
            let actual_pixel = &actual_bytes[offset..offset + 4];
            if interval < intervals.len() && x >= intervals[interval].0 {
                masked_pixels += 1;
                diff_bytes[offset..offset + 4].copy_from_slice(&[48, 96, 160, 160]);
                continue;
            }

            compared_pixels += 1;
            let delta = expected_pixel
                .iter()
                .zip(actual_pixel)
                .map(|(left, right)| left.abs_diff(*right))
                .max()
                .unwrap_or_default();
            max_observed_channel_delta = max_observed_channel_delta.max(delta);
            if delta > config.max_channel_delta {
                changed_pixels += 1;
                changed_channels += expected_pixel
                    .iter()
                    .zip(actual_pixel)
                    .filter(|(left, right)| {
                        (**left).abs_diff(**right) > config.max_channel_delta
                    })
                    .count() as u64;
                first_changed_pixel.get_or_insert([x, y]);
                minimum_changed_x = minimum_changed_x.min(x);
                minimum_changed_y = minimum_changed_y.min(y);
                maximum_changed_x = maximum_changed_x.max(x);
                maximum_changed_y = maximum_changed_y.max(y);
                diff_bytes[offset..offset + 4].copy_from_slice(&[255, 32, 64, 255]);
            } else {
                let gray = ((u16::from(expected_pixel[0])
                    + u16::from(expected_pixel[1])
                    + u16::from(expected_pixel[2]))
                    / 3) as u8;
                diff_bytes[offset..offset + 4].copy_from_slice(&[gray, gray, gray, 96]);
            }
        }
    }
    if compared_pixels == 0 {
        return Err("visual-diff masks cannot cover every pixel".into());
    }
    let changed_pixel_ratio = changed_pixels as f64 / compared_pixels as f64;
    let changed_bounds = if changed_pixels > 0 {
        Some([
            minimum_changed_x,
            minimum_changed_y,
            maximum_changed_x - minimum_changed_x + 1,
            maximum_changed_y - minimum_changed_y + 1,
        ])
    } else {
        None
    };
    let status = if changed_pixel_ratio <= config.max_changed_pixel_ratio {
        "passed"
    } else {
        "failed"
    };
    let diff = RgbaImage::from_raw(width, height, diff_bytes)
        .ok_or_else(|| "visual-diff output buffer has an invalid size".to_string())?;
    Ok(Comparison {
        report: VisualDiffReport {
            schema: 1,
            status,
            width,
            height,
            compared_pixels,
            masked_pixels,
            changed_pixels,
            changed_channels,
            changed_pixel_ratio,
            first_changed_pixel,
            changed_bounds,
            max_observed_channel_delta,
            max_channel_delta: config.max_channel_delta,
            max_changed_pixel_ratio: config.max_changed_pixel_ratio,
        },
        diff,
    })
}

fn write_diff_atomic(path: &Path, diff: &RgbaImage) -> TaskResult {
    let parent = path
        .parent()
        .ok_or_else(|| "visual-diff output has no parent".to_string())?;
    let mut staged = tempfile::NamedTempFile::new_in(parent)
        .map_err(|error| format!("cannot stage visual-diff image: {error}"))?;
    DynamicImage::ImageRgba8(diff.clone())
        .write_to(staged.as_file_mut(), ImageFormat::Png)
        .map_err(|error| format!("cannot encode visual-diff image: {error}"))?;
    staged
        .as_file_mut()
        .flush()
        .map_err(|error| format!("cannot flush visual-diff image: {error}"))?;
    staged
        .persist(path)
        .map(|_| ())
        .map_err(|error| format!("cannot publish visual-diff image: {}", error.error))
}

fn write_report_atomic(path: &Path, report: &VisualDiffReport) -> TaskResult {
    let bytes = serde_json::to_vec_pretty(report)
        .map_err(|error| format!("cannot encode visual-diff report: {error}"))?;
    if bytes.len() > MAX_REPORT_BYTES {
        return Err("visual-diff report exceeds its bound".into());
    }
    let parent = path
        .parent()
        .ok_or_else(|| "visual-diff report has no parent".to_string())?;
    let mut staged = tempfile::NamedTempFile::new_in(parent)
        .map_err(|error| format!("cannot stage visual-diff report: {error}"))?;
    staged
        .write_all(&bytes)
        .and_then(|()| staged.flush())
        .map_err(|error| format!("cannot write visual-diff report: {error}"))?;
    staged
        .persist(path)
        .map(|_| ())
        .map_err(|error| format!("cannot publish visual-diff report: {}", error.error))
}

#[cfg(test)]
mod tests {
    use super::*;
    use image_rs::Rgba;

    fn image(path: &Path, pixels: &[[u8; 4]]) {
        let image = RgbaImage::from_fn(2, 2, |x, y| Rgba(pixels[(y * 2 + x) as usize]));
        image.save_with_format(path, ImageFormat::Png).unwrap();
    }

    fn config(delta: u8, ratio: f64, masks: Vec<Mask>) -> VisualDiffConfig {
        VisualDiffConfig {
            schema: 1,
            max_channel_delta: delta,
            max_changed_pixel_ratio: ratio,
            masks,
        }
    }

    #[test]
    fn exact_and_bounded_delta_comparisons_pass() {
        let directory = tempfile::tempdir().unwrap();
        let expected = directory.path().join("expected.png");
        let actual = directory.path().join("actual.png");
        let pixels = [[10, 20, 30, 255]; 4];
        image(&expected, &pixels);
        let mut changed = pixels;
        changed[3] = [12, 18, 31, 255];
        image(&actual, &changed);

        let comparison =
            compare(&expected, &actual, &config(2, 0.0, Vec::new())).unwrap();
        assert_eq!(comparison.report.status, "passed");
        assert_eq!(comparison.report.changed_pixels, 0);
        assert_eq!(comparison.report.max_observed_channel_delta, 2);
    }

    #[test]
    fn mask_excludes_only_the_reviewed_rectangle() {
        let directory = tempfile::tempdir().unwrap();
        let expected = directory.path().join("expected.png");
        let actual = directory.path().join("actual.png");
        let pixels = [[0, 0, 0, 255]; 4];
        image(&expected, &pixels);
        let mut changed = pixels;
        changed[0] = [255, 255, 255, 255];
        image(&actual, &changed);

        let comparison = compare(
            &expected,
            &actual,
            &config(
                0,
                0.0,
                vec![Mask {
                    x: 0,
                    y: 0,
                    width: 1,
                    height: 1,
                }],
            ),
        )
        .unwrap();
        assert_eq!(comparison.report.status, "passed");
        assert_eq!(comparison.report.masked_pixels, 1);
        assert_eq!(comparison.report.compared_pixels, 3);
    }

    #[test]
    fn changed_ratio_failure_produces_atomic_artifacts() {
        let directory = tempfile::tempdir().unwrap();
        let expected = directory.path().join("expected.png");
        let actual = directory.path().join("actual.png");
        let diff = directory.path().join("diff.png");
        let report = directory.path().join("report.json");
        image(&expected, &[[0, 0, 0, 255]; 4]);
        image(&actual, &[[255, 255, 255, 255]; 4]);
        let comparison =
            compare(&expected, &actual, &config(0, 0.5, Vec::new())).unwrap();
        assert_eq!(comparison.report.status, "failed");
        assert_eq!(comparison.report.changed_pixel_ratio, 1.0);

        write_diff_atomic(&diff, &comparison.diff).unwrap();
        write_report_atomic(&report, &comparison.report).unwrap();
        assert_eq!(decode_bounded(&diff).unwrap().dimensions(), (2, 2));
        let report: serde_json::Value =
            serde_json::from_slice(&fs::read(report).unwrap()).unwrap();
        assert_eq!(report["status"], "failed");
        assert!(report.get("expected").is_none());
    }

    #[test]
    fn repository_visual_policy_is_reviewed_and_strict() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/assurance/visual-diff-policy-v1.json");
        let config = load_config(&path.canonicalize().unwrap()).unwrap();
        assert_eq!(config.schema, 1);
        assert_eq!(config.max_channel_delta, 0);
        assert_eq!(config.max_changed_pixel_ratio, 0.0);
        assert!(config.masks.is_empty());
    }

    #[test]
    fn repository_policy_rejects_one_changed_channel_in_one_pixel() {
        let directory = tempfile::tempdir().unwrap();
        let expected = directory.path().join("expected.png");
        let actual = directory.path().join("actual.png");
        let pixels = [[10, 20, 30, 255]; 4];
        image(&expected, &pixels);
        let mut changed = pixels;
        changed[2] = [10, 21, 30, 255];
        image(&actual, &changed);
        let policy = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/assurance/visual-diff-policy-v1.json")
            .canonicalize()
            .unwrap();

        let comparison =
            compare(&expected, &actual, &load_config(&policy).unwrap()).unwrap();

        assert_eq!(comparison.report.status, "failed");
        assert_eq!(comparison.report.changed_pixels, 1);
        assert_eq!(comparison.report.changed_channels, 1);
        assert_eq!(comparison.report.first_changed_pixel, Some([0, 1]));
        assert_eq!(comparison.report.changed_bounds, Some([0, 1, 1, 1]));
    }

    #[test]
    fn dimensions_masks_and_fully_masked_images_fail_closed() {
        let directory = tempfile::tempdir().unwrap();
        let expected = directory.path().join("expected.png");
        let actual = directory.path().join("actual.png");
        image(&expected, &[[0, 0, 0, 255]; 4]);
        RgbaImage::new(1, 1)
            .save_with_format(&actual, ImageFormat::Png)
            .unwrap();
        assert!(compare(&expected, &actual, &config(0, 0.0, Vec::new())).is_err());

        image(&actual, &[[0, 0, 0, 255]; 4]);
        let all = Mask {
            x: 0,
            y: 0,
            width: 2,
            height: 2,
        };
        assert!(compare(&expected, &actual, &config(0, 0.0, vec![all])).is_err());
        let outside = Mask {
            x: 2,
            y: 0,
            width: 1,
            height: 1,
        };
        assert!(compare(&expected, &actual, &config(0, 0.0, vec![outside])).is_err());

        let jpeg = directory.path().join("actual.jpg");
        image_rs::RgbImage::new(2, 2)
            .save_with_format(&jpeg, ImageFormat::Jpeg)
            .unwrap();
        assert!(decode_bounded(&jpeg).is_err());
    }

    #[test]
    fn existing_output_alias_is_resolved_before_distinctness_checks() {
        let directory = tempfile::tempdir().unwrap();
        let expected = directory.path().join("expected.png");
        image(&expected, &[[0, 0, 0, 255]; 4]);

        assert_eq!(
            safe_output(&expected).unwrap(),
            expected.canonicalize().unwrap()
        );
    }
}
