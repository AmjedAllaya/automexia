//! Shared artifact exporter for real renderer tests, excluded from product builds.
//! The CI driver seals these capture facts with source/environment identity.

use rio_backend::sugarloaf::font::{constants, FontData, FontLibrary, FontLibraryData};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::io::Write;
use std::sync::Arc;

pub fn fonts() -> FontLibrary {
    let mut data = FontLibraryData::default();
    data.insert(FontData::from_static_slice(constants::FONT_CASCADIA_CODE_NF).unwrap());
    data.insert(
        FontData::from_static_slice(constants::FONT_CASCADIA_CODE_NF_ITALIC).unwrap(),
    );
    data.insert(
        FontData::from_static_slice_with_wght(
            constants::FONT_CASCADIA_CODE_NF,
            Some(constants::WGHT_BOLD),
        )
        .unwrap(),
    );
    data.insert(
        FontData::from_static_slice_with_wght(
            constants::FONT_CASCADIA_CODE_NF_ITALIC,
            Some(constants::WGHT_BOLD),
        )
        .unwrap(),
    );
    FontLibrary {
        inner: Arc::new(parking_lot::RwLock::new(data)),
    }
}

#[test]
fn visual_quality_fixture_faces_match_the_production_slot_contract() {
    use rio_backend::sugarloaf::font::{
        FONT_ID_BOLD, FONT_ID_BOLD_ITALIC, FONT_ID_ITALIC, FONT_ID_REGULAR,
    };
    let fonts = fonts();
    let data = fonts.inner.read();
    for (id, bold, italic) in [
        (FONT_ID_REGULAR, false, false),
        (FONT_ID_ITALIC, false, true),
        (FONT_ID_BOLD, true, false),
        (FONT_ID_BOLD_ITALIC, true, true),
    ] {
        assert_eq!(data.get(&id).is_bold(), bold, "fixture bold slot {id}");
        assert_eq!(
            data.get(&id).is_italic(),
            italic,
            "fixture italic slot {id}"
        );
    }
}

pub fn sha256(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut result = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        result.push(char::from(HEX[usize::from(byte >> 4)]));
        result.push(char::from(HEX[usize::from(byte & 15)]));
    }
    result
}

/// Geometry is measured by the production layout/emission owner in physical px.
pub fn export(
    name: &str,
    pixels: &[u32],
    size: [u32; 2],
    scale: f32,
    theme: &str,
    geometry: Vec<(&str, [f32; 4])>,
) {
    export_variant(name, "correct", pixels, size, scale, theme, geometry);
}

#[allow(clippy::too_many_arguments)]
pub fn export_variant(
    name: &str,
    variant: &str,
    pixels: &[u32],
    size: [u32; 2],
    scale: f32,
    theme: &str,
    geometry: Vec<(&str, [f32; 4])>,
) {
    let hashes = [
        constants::FONT_CASCADIA_CODE_NF,
        constants::FONT_CASCADIA_CODE_NF_ITALIC,
        constants::FONT_CASCADIA_CODE_NF,
        constants::FONT_CASCADIA_CODE_NF_ITALIC,
    ]
    .map(sha256);
    export_with_fonts(name, variant, pixels, size, scale, theme, geometry, &hashes);
}

#[allow(clippy::too_many_arguments)]
pub fn export_with_fonts(
    name: &str,
    variant: &str,
    pixels: &[u32],
    size: [u32; 2],
    scale: f32,
    theme: &str,
    geometry: Vec<(&str, [f32; 4])>,
    hashes: &[String],
) {
    let Some(directory) = std::env::var_os("AUTOMEXIA_VISUAL_CAPTURE_DIR") else {
        return;
    };
    assert!(
        name.len() <= 96
            && name
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
    );
    assert!(!geometry.is_empty() && geometry.len() <= 128);
    assert!(
        variant.len() <= 32
            && variant
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-')
    );
    let filename = if variant == "correct" {
        name.to_owned()
    } else {
        format!("{name}--{variant}")
    };
    let [width, height] = size;
    let count = u64::from(width) * u64::from(height);
    assert!(width > 0 && height > 0 && width <= 8192 && height <= 8192);
    assert!(count <= 40_000_000);
    assert_eq!(pixels.len() as u64, count);
    assert!(!hashes.is_empty() && hashes.len() <= 16);
    let directory = std::path::PathBuf::from(directory);
    assert!(
        directory.is_dir(),
        "driver must create a private capture directory"
    );
    let image = image_rs::RgbImage::from_fn(width, height, |x, y| {
        let pixel = pixels[(y * width + x) as usize];
        image_rs::Rgb([(pixel >> 16) as u8, (pixel >> 8) as u8, pixel as u8])
    });
    let mut staged = tempfile::NamedTempFile::new_in(&directory).unwrap();
    image
        .write_to(staged.as_file_mut(), image_rs::ImageFormat::Png)
        .unwrap();
    staged.flush().unwrap();
    let encoded = std::fs::read(staged.path()).unwrap();
    staged
        .persist(directory.join(format!("{filename}.png")))
        .unwrap();
    let facts = json!({
        "schema":1, "scenario":name, "variant":variant, "fixture":"visual-quality-v1",
        "evidence_kind":"controlled-raster", "renderer":"cpu", "width":width, "height":height,
        "scale_milli": (scale * 1000.0).round() as u32, "font_size_milli":16000,
        "theme":theme, "image_sha256":sha256(&encoded), "frame_generation":1,
        "font_hashes":hashes,
        "geometry": geometry.into_iter().map(|(id, rect)| json!({
            "id":id,"rect_milli":rect.map(|v| {assert!(v.is_finite()); (v * 1000.0).round() as i64})
        })).collect::<Vec<_>>()
    });
    let mut staged = tempfile::NamedTempFile::new_in(&directory).unwrap();
    staged
        .write_all(&serde_json::to_vec_pretty(&facts).unwrap())
        .unwrap();
    staged.flush().unwrap();
    staged
        .persist(directory.join(format!("{filename}.capture.json")))
        .unwrap();
}
