use super::*;

// Independent f64 sRGB oracle, including the byte conversion used by Text.
fn luminance(bytes: [u8; 4]) -> f64 {
    let linear = bytes.map(|value| {
        let value = f64::from(value) / 255.0;
        if value <= 0.04045 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    });
    linear[0] * 0.2126 + linear[1] * 0.7152 + linear[2] * 0.0722
}

fn contrast(a: [u8; 4], b: [u8; 4]) -> f64 {
    let (a, b) = (luminance(a), luminance(b));
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}

#[test]
fn chrome_text_contrast_survives_quantization_and_custom_foregrounds() {
    for background in [[0.0; 4], [1.0; 4], [0.9, 0.2, 0.8, 0.3]] {
        for red in [0.0, 0.2, 0.47, 0.7, 1.0] {
            for green in [0.0, 0.2, 0.47, 0.7, 1.0] {
                for blue in [0.0, 0.2, 0.47, 0.7, 1.0] {
                    let theme = UiTheme::resolve(
                        background,
                        [red, green, blue, 0.2],
                        [blue, red, green, 0.0],
                    );
                    for surface in [theme.background, theme.surface, theme.raised] {
                        for label in [theme.text, theme.muted_text] {
                            assert_eq!(color_u8(label)[3], 255);
                            assert!(contrast(color_u8(label), color_u8(surface)) >= 4.5);
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn chrome_decoration_focus_and_text_have_distinct_roles() {
    for surface in [CARD, SURFACE, SURFACE_RAISED] {
        for text in [TEXT, MUTED_TEXT] {
            assert!(contrast(color_u8(text), color_u8(surface)) >= 4.5);
        }
        for focus in [OUTLINE, BRAND_CYAN] {
            assert!(contrast(color_u8(focus), color_u8(surface)) >= 3.0);
        }
    }
    assert!(luminance(color_u8(CARD)) < luminance(color_u8(SURFACE)));
    assert!(luminance(color_u8(SURFACE)) < luminance(color_u8(SURFACE_RAISED)));
    assert!(luminance(color_u8(BORDER)) < luminance(color_u8(OUTLINE)));
    assert_eq!(
        (CARD_RADIUS, CONTROL_RADIUS, KEYCAP_RADIUS),
        (14.0, 8.0, 5.0)
    );
}

#[test]
fn chrome_preserves_readable_configured_foreground() {
    let foreground = [0.93, 0.85, 0.75, 1.0];
    let theme = UiTheme::resolve([0.0; 4], foreground, MUTED_TEXT);
    assert_eq!(theme.text, foreground);
    assert_eq!(theme.muted_text, MUTED_TEXT);
}

#[test]
fn chrome_token_strip_matches_literal_rgba_and_rejects_one_channel_drift() {
    let actual = [
        CARD,
        SURFACE,
        SURFACE_RAISED,
        BORDER,
        OUTLINE,
        TEXT,
        MUTED_TEXT,
    ]
    .map(color_u8);
    let expected = [
        [6, 11, 17, 255],
        [14, 23, 32, 255],
        [20, 44, 58, 255],
        [41, 65, 79, 255],
        [51, 173, 204, 255],
        [224, 233, 238, 255],
        [154, 172, 184, 255],
    ];
    assert_eq!(actual, expected);
    let mut changed = actual;
    changed[3][1] += 1;
    assert_ne!(changed, expected);
}

#[test]
fn chrome_controlled_style_specimen() {
    use rio_backend::sugarloaf::{
        font::{constants, FontData, FontLibrary, FontLibraryData},
        text::{DrawOpts, Text},
    };
    use std::sync::Arc;
    let mut data = FontLibraryData::default();
    data.insert(FontData::from_static_slice(constants::FONT_CASCADIA_CODE_NF).unwrap());
    let fonts = FontLibrary {
        inner: Arc::new(parking_lot::RwLock::new(data)),
    };
    let mut text = Text::new(&fonts);
    text.init_cpu();
    let width = 600usize;
    let height = 310usize;
    let pack = |color| {
        let [r, g, b, _] = color_u8(color);
        (u32::from(r) << 16) | (u32::from(g) << 8) | u32::from(b)
    };
    let mut pixels = vec![pack(CARD); width * height];
    // A style specimen, not a native app screenshot: literal layout, production
    // colour tokens and real bundled-font rasterization; no private shell data.
    for y in 0..height {
        for x in 0..width {
            let color = if (16..584).contains(&x) && (48..94).contains(&y) {
                if x == 16 || x == 583 || y == 48 || y == 93 {
                    OUTLINE
                } else {
                    SURFACE
                }
            } else if (16..584).contains(&x) && (152..194).contains(&y) {
                if (20..22).contains(&x) && (163..183).contains(&y) {
                    BRAND_CYAN
                } else {
                    SURFACE_RAISED
                }
            } else if x == 0 || x == width - 1 || y == 0 || y == height - 1 {
                BORDER
            } else {
                CARD
            };
            pixels[y * width + x] = pack(color);
        }
    }
    for (x, y, label, color, size) in [
        (20.0, 16.0, "AUTOMEXIA", TEXT, 15.0),
        (32.0, 63.0, "Search all commands…", MUTED_TEXT, 15.0),
        (46.0, 120.0, "Tabs & windows", TEXT, 14.5),
        (46.0, 164.0, "Panes & layout", TEXT, 14.5),
        (470.0, 166.0, "Enter", MUTED_TEXT, 11.0),
        (46.0, 208.0, "Search & navigation", TEXT, 14.5),
        (46.0, 252.0, "Appearance", TEXT, 14.5),
        (
            20.0,
            288.0,
            "Controlled style specimen · not a native frame",
            MUTED_TEXT,
            10.0,
        ),
    ] {
        let options = DrawOpts {
            font_size: size,
            color: color_u8(color),
            ..Default::default()
        };
        assert!(x + text.measure(label, &options) < width as f32);
        text.draw(x, y, label, &options);
    }
    let before = pixels.clone();
    text.render_cpu_base(&mut pixels, width as u32, height as u32);
    text.render_cpu_modal(&mut pixels, width as u32, height as u32);
    assert_ne!(pixels, before, "real font pixels must be present");
    if let Some(path) = std::env::var_os("AUTOMEXIA_CHROME_PREVIEW") {
        image_rs::RgbImage::from_fn(width as u32, height as u32, |x, y| {
            let pixel = pixels[y as usize * width + x as usize];
            image_rs::Rgb([(pixel >> 16) as u8, (pixel >> 8) as u8, pixel as u8])
        })
        .save(path)
        .expect("controlled style specimen writes successfully");
    }
}
