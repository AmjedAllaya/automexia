// Implementation made by ayosec
// https://github.com/ayosec/alacritty/commit/661a64c2b35283c97bac71d29535393e909c7d19
// This module implements support for the [iTerm2 images protocol](https://iterm2.com/documentation-images.html).
//
// iTerm2 uses the OSC 1337 for a many non-standard commands, but we only support
// adding inline graphics.
//
// This implementation also supports `width` and `height` parameters to resize the image.

use rio_graphics::GraphicData;
#[cfg(feature = "graphics")]
use rio_graphics::{GraphicId, ResizeCommand, ResizeParameter, MAX_GRAPHIC_DIMENSIONS};

use rustc_hash::FxHashMap;
#[cfg(feature = "graphics")]
use std::io::Cursor;
use std::str;

use crate::simd_base64;
use crate::simd_utf8;

/// Cursor movement after an inline image: iTerm2 leaves the cursor on
/// the image's last row, in the first column after the image
/// (`insert_graphic` movement value 2). `doNotMoveCursor=1` is a
/// WezTerm extension used by TUIs drawing on the bottom row (value 1).
pub const CURSOR_RIGHT_OF_IMAGE: u8 = 2;
pub const CURSOR_DO_NOT_MOVE: u8 = 1;

/// OSC output is untrusted. This cap and the decoder limits prevent a large
/// base64 payload or a compressed image bomb from monopolizing memory.
#[cfg(feature = "graphics")]
const MAX_ENCODED_IMAGE_BYTES: usize = 64 * 1024 * 1024;
#[cfg(feature = "graphics")]
const MAX_BASE64_IMAGE_BYTES: usize = (MAX_ENCODED_IMAGE_BYTES / 3) * 4 + 4;
#[cfg(feature = "graphics")]
const MAX_IMAGE_ALLOCATION: u64 = 96 * 1024 * 1024;

/// Parse the OSC 1337 parameters to add a graphic to the grid.
/// Returns the graphic plus the cursor movement it requires.
pub fn parse(params: &[&[u8]]) -> Option<(GraphicData, u8)> {
    let (params, contents) = param_values(params)?;

    if params.get("inline") != Some(&"1") {
        return None;
    }

    #[cfg(feature = "graphics")]
    if contents.len() > MAX_BASE64_IMAGE_BYTES {
        tracing::warn!("Rejected oversized iTerm2 base64 payload");
        return None;
    }

    let buffer = match simd_base64::decode(contents) {
        Some(buffer) => buffer,
        None => {
            tracing::warn!("Can't decode iTerm2 base64 image payload");
            return None;
        }
    };

    #[cfg(not(feature = "graphics"))]
    {
        // Image decoding needs the `graphics` feature; without it iTerm2
        // image payloads are parsed but not displayed.
        let _ = &buffer;
        None
    }
    #[cfg(feature = "graphics")]
    {
        if buffer.is_empty() || buffer.len() > MAX_ENCODED_IMAGE_BYTES {
            tracing::warn!("Rejected oversized iTerm2 image payload");
            return None;
        }
        if let Some(declared) = params.get("size") {
            let Ok(declared) = declared.parse::<usize>() else {
                tracing::warn!("Rejected malformed iTerm2 image size");
                return None;
            };
            if declared != buffer.len() || declared > MAX_ENCODED_IMAGE_BYTES {
                tracing::warn!("Rejected mismatched iTerm2 image size");
                return None;
            }
        }

        let mut reader =
            match image_rs::ImageReader::new(Cursor::new(buffer)).with_guessed_format() {
                Ok(reader) => reader,
                Err(err) => {
                    tracing::warn!("Can't detect iTerm2 image format: {}", err);
                    return None;
                }
            };
        let mut limits = image_rs::Limits::default();
        limits.max_image_width = Some(MAX_GRAPHIC_DIMENSIONS[0] as u32);
        limits.max_image_height = Some(MAX_GRAPHIC_DIMENSIONS[1] as u32);
        limits.max_alloc = Some(MAX_IMAGE_ALLOCATION);
        reader.limits(limits);
        let image = match reader.decode() {
            Ok(image) => image,
            Err(err) => {
                tracing::warn!("Can't load image: {}", err);
                return None;
            }
        };

        let cursor_movement = if params.get("doNotMoveCursor") == Some(&"1") {
            CURSOR_DO_NOT_MOVE
        } else {
            CURSOR_RIGHT_OF_IMAGE
        };

        let mut graphics = GraphicData::from_dynamic_image(GraphicId::new(1), image);
        graphics.resize = resize_param(&params);
        Some((graphics, cursor_movement))
    }
}

/// Extract parameter values.
///
/// The format defined by iTerm2 starts with a `File=` string, and the file
/// contents are specified after a `:`.
///
/// ```notrust
/// ESC ] 1337 ; File = [arguments] : base-64 encoded file contents ^G
/// ```
///
/// This format is not expected by the parser in the `vte` crate.
///
/// The `File=` string is found in the first parameter, and the file contents are
/// appended in the last one. We have to split these parameter to get the expected
/// data.
fn param_values<'a>(
    params: &[&'a [u8]],
) -> Option<(FxHashMap<&'a str, &'a str>, &'a [u8])> {
    let mut map = FxHashMap::default();
    let mut contents = None;

    for (index, mut param) in params.iter().skip(1).copied().enumerate() {
        // First parameter should start with "File="
        if index == 0 {
            if !param.starts_with(&b"File="[..]) {
                return None;
            }

            param = &param[5..];
        }

        if let Some(separator) = param.iter().position(|&b| b == b'=') {
            let (key, mut value) = param.split_at(separator);
            value = &value[1..];

            // Last parameter has the file contents after the first ':'.
            // Add 2 because we are skipping the first param.
            if index + 2 == params.len() {
                if let Some(separator) = value.iter().position(|&b| b == b':') {
                    let (a, b) = value.split_at(separator);
                    value = a;
                    contents = Some(&b[1..]);
                }
            }

            if let (Ok(key), Ok(value)) = (
                simd_utf8::from_utf8_fast(key),
                simd_utf8::from_utf8_fast(value),
            ) {
                map.insert(key, value);
            }
        }
    }

    contents.map(|c| (map, c))
}

/// Compute the resize operation from the OSC parameters.
///
/// Accepted formats:
///
/// - N: N character cells.
/// - Npx: N pixels.
/// - N%: N percent of the window's width or height.
/// - auto: Computed from the original graphic size.
#[cfg(feature = "graphics")]
fn resize_param(params: &FxHashMap<&str, &str>) -> Option<ResizeCommand> {
    fn parse(value: Option<&str>) -> Option<ResizeParameter> {
        let value = match value {
            None | Some("auto") => return Some(ResizeParameter::Auto),
            Some(value) => value,
        };

        // Split the value after the first non-digit byte.
        // If there is no unit, parse as number of cells.
        let first_nondigit = value
            .as_bytes()
            .iter()
            .position(|b: &u8| !b.is_ascii_digit());
        // .position(|b| !(b'0'..=b'9').contains(&b));
        let (number, unit) = match first_nondigit {
            Some(position) => value.split_at(position),
            None => return Some(ResizeParameter::Cells(str::parse(value).ok()?)),
        };

        match (str::parse(number), unit) {
            (Ok(number), "%") => Some(ResizeParameter::WindowPercent(number)),
            (Ok(number), "px") => Some(ResizeParameter::Pixels(number)),
            _ => None,
        }
    }

    let width = parse(params.get(&"width").copied())?;
    let height = parse(params.get(&"height").copied())?;

    let preserve_aspect_ratio = params.get(&"preserveAspectRatio") != Some(&"0");

    Some(ResizeCommand {
        width,
        height,
        preserve_aspect_ratio,
    })
}

#[test]
fn parse_osc1337_parameters() {
    let params = [
        b"1337".as_ref(),
        b"File=name=ABCD".as_ref(),
        b"size=3".as_ref(),
        b"inline=1:AAAA".as_ref(),
    ];

    let (params, contents) = param_values(&params).unwrap();

    assert_eq!(params["name"], "ABCD");
    assert_eq!(params["size"], "3");
    assert_eq!(params["inline"], "1");

    assert_eq!(contents, b"AAAA".as_ref())
}

#[test]
fn parse_osc1337_single_parameter() {
    let params = [b"1337".as_ref(), b"File=inline=1:AAAA".as_ref()];

    let (params, contents) = param_values(&params).unwrap();

    assert_eq!(params["inline"], "1");
    assert_eq!(contents, b"AAAA".as_ref())
}

#[cfg(feature = "graphics")]
#[test]
fn resize_params() {
    use ResizeParameter::{Auto, Cells, Pixels, WindowPercent};

    macro_rules! assert_resize {
        ($param_width:expr, $param_height:expr, $width:expr, $height:expr) => {
            let mut params = FxHashMap::default();
            params.insert("width", $param_width);
            params.insert("height", $param_height);

            let resize = resize_param(&params).unwrap();
            assert_eq!(resize.width, $width);
            assert_eq!(resize.height, $height);
        };
    }

    assert_resize!("auto", "50%", Auto, WindowPercent(50));
    assert_resize!("10", "20", Cells(10), Cells(20));
    assert_resize!("10%", "50px", WindowPercent(10), Pixels(50));
}

#[cfg(all(test, feature = "graphics"))]
mod bounded_decode_tests {
    use super::*;
    use base64::engine::general_purpose::STANDARD as BASE64;
    use base64::Engine as _;
    use image_rs::ImageEncoder as _;

    fn png(width: u32, height: u32) -> Vec<u8> {
        let pixels = vec![0x80; width as usize * height as usize * 4];
        let mut encoded = Vec::new();
        image_rs::codecs::png::PngEncoder::new(&mut encoded)
            .write_image(&pixels, width, height, image_rs::ExtendedColorType::Rgba8)
            .unwrap();
        encoded
    }

    fn parse_png(bytes: &[u8], declared_size: usize) -> Option<(GraphicData, u8)> {
        let size = format!("size={declared_size}");
        let payload = format!("inline=1:{}", BASE64.encode(bytes));
        let params = [
            b"1337".as_ref(),
            b"File=name=cHJldmlldy5wbmc=".as_ref(),
            size.as_bytes(),
            payload.as_bytes(),
        ];
        parse(&params)
    }

    #[test]
    fn bounded_decoder_accepts_a_small_declared_png() {
        let bytes = png(2, 3);
        let (graphic, _) = parse_png(&bytes, bytes.len()).expect("valid bounded image");
        assert_eq!((graphic.width, graphic.height), (2, 3));
    }

    #[test]
    fn bounded_decoder_rejects_declared_size_mismatch() {
        let bytes = png(2, 3);
        assert!(parse_png(&bytes, bytes.len() + 1).is_none());
    }

    #[test]
    fn bounded_decoder_rejects_dimensions_above_graphics_limit() {
        let bytes = png(MAX_GRAPHIC_DIMENSIONS[0] as u32 + 1, 1);
        assert!(parse_png(&bytes, bytes.len()).is_none());
    }
}
