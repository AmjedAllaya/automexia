use super::*;
use crate::font::{constants, FontData, FontLibraryData};
use std::sync::Arc;

fn fixture_fonts() -> FontLibrary {
    fixture_library(
        FontData::from_static_slice(constants::FONT_CASCADIA_CODE_NF).unwrap(),
    )
}

fn fixture_library(font: FontData) -> FontLibrary {
    let mut data = FontLibraryData::default();
    data.insert(font);
    for _ in 0..3 {
        data.insert_alias(0);
    }
    FontLibrary {
        inner: Arc::new(parking_lot::RwLock::new(data)),
    }
}

fn frame(text: &mut Text, label: &str, opts: &DrawOpts) -> (f32, Vec<u32>) {
    text.init_cpu();
    text.clear();
    let width = text.draw(4.0, 4.0, label, opts);
    text.finalize_modal_layer();
    let mut pixels = vec![0x00112233; 256 * 64];
    text.render_cpu_base(&mut pixels, 256, 64);
    assert!(pixels.iter().any(|pixel| *pixel != 0x00112233));
    (width, pixels)
}

#[test]
fn nearby_sizes_measure_like_fresh_instances_in_both_orders_and_scales() {
    let fonts = fixture_fonts();
    for scale in [1.0, 1.25, 2.5] {
        for (first, second) in [(12.4, 12.6), (12.6, 12.4)] {
            // Both requests used quarter bucket 50, but the native shaper
            // receives whole-pixel sizes 12 and 13. A fresh owner is the oracle.
            let mut cached = Text::new(&fonts);
            let mut fresh = Text::new(&fonts);
            cached.set_scale_factor(scale);
            fresh.set_scale_factor(scale);
            let mut opts = DrawOpts {
                font_size: first / scale,
                ..DrawOpts::default()
            };
            let before = cached.measure("Measure the label", &opts);
            opts.font_size = second / scale;
            let expected = fresh.measure("Measure the label", &opts);
            assert_ne!(before, expected, "fixture must distinguish raster sizes");
            assert_eq!(cached.measure("Measure the label", &opts), expected);
            assert_eq!(cached.measure("Measure the label", &opts), expected);
        }
    }
}

#[test]
fn nearby_sizes_draw_exact_fresh_pixels_after_each_transition() {
    let fonts = fixture_fonts();
    let mut cached = Text::new(&fonts);
    for scale in [1.0, 1.25, 2.5, 1.0] {
        cached.set_scale_factor(scale);
        for size in [12.4, 12.6, 12.4, 16.4, 16.6, 12.6] {
            let opts = DrawOpts {
                font_size: size,
                ..DrawOpts::default()
            };
            let mut fresh = Text::new(&fonts);
            fresh.set_scale_factor(scale);
            let expected = frame(&mut fresh, "Actual glyph pixels", &opts);
            let actual = frame(&mut cached, "Actual glyph pixels", &opts);
            assert_eq!(actual.0, expected.0);
            let first_difference = actual
                .1
                .iter()
                .zip(&expected.1)
                .position(|(actual, expected)| actual != expected);
            assert_eq!(
                first_difference, None,
                "first changed CPU pixel at size {size}, scale {scale}"
            );
        }
    }
}

#[test]
fn distinct_labels_do_not_accumulate_unbounded_shaped_runs() {
    let mut text = Text::new(&fixture_fonts());
    let opts = DrawOpts::default();
    for index in 0..600 {
        assert!(text.measure(&format!("Public fixture label {index}"), &opts) > 0.0);
    }
    assert!(text.shape_cache.len() <= 512);
}

#[test]
fn cached_measurement_and_draw_share_immutable_glyph_storage() {
    let mut text = Text::new(&fixture_fonts());
    let opts = DrawOpts::default();
    let first = text.shape_for("A long immutable label", &opts).unwrap();
    text.measure("A long immutable label", &opts);
    let second = text.shape_for("A long immutable label", &opts).unwrap();
    assert!(Arc::ptr_eq(&first, &second));
    assert!(std::ptr::eq(first.glyphs.as_ptr(), second.glyphs.as_ptr()));
}

#[test]
fn oversized_actual_shaped_run_is_measurable_without_retention() {
    let mut text = Text::new(&fixture_fonts());
    let opts = DrawOpts::default();
    text.measure("Keep this label", &opts);
    let input = "a".repeat(120_000);
    assert!(text.measure(&input, &opts) > 0.0);
    assert_eq!(text.shape_cache.len(), 1);
    assert_eq!(text.measure("", &opts), 0.0);
    assert_eq!(text.shape_cache.len(), 1);
}

#[test]
fn font_replacement_refreshes_pixels_and_releases_stale_state_without_new_atlases() {
    let regular = fixture_fonts();
    let italic = fixture_library(
        FontData::from_static_slice(constants::FONT_CASCADIA_CODE_NF_ITALIC).unwrap(),
    );
    let bold = fixture_library(
        FontData::from_static_slice_with_wght(
            constants::FONT_CASCADIA_CODE_NF,
            Some(700.0),
        )
        .unwrap(),
    );
    let opts = DrawOpts::default();
    let mut cached = Text::new(&regular);
    let regular_frame = frame(&mut cached, "Actual glyph pixels", &opts);
    let italic_frame = frame(&mut Text::new(&italic), "Actual glyph pixels", &opts);
    assert!(
        regular_frame.1 != italic_frame.1,
        "font fixture must visibly change"
    );

    for scale in [1.0, 2.0, 1.0] {
        cached.set_scale_factor(scale);
        for fonts in [&italic, &bold, &regular] {
            frame(&mut cached, "Actual glyph pixels", &opts);
            let old_run = cached.shape_for("Actual glyph pixels", &opts).unwrap();
            let old_key = crate::grid::GlyphKey {
                font_id: old_run.font_id,
                glyph_id: old_run.glyphs[0].id as u32,
                size_bucket: old_run.size_u16,
            };
            let old_run_weak = Arc::downgrade(&old_run);
            drop(old_run);
            cached.begin_modal_layer();
            cached.draw(4.0, 4.0, "Pending old label", &opts);
            let atlas = &cached.cpu.as_ref().unwrap().atlas_grayscale;
            let old_pixels = atlas.pixels().as_ptr();
            assert!(atlas.lookup(old_key).is_some());

            // The bundled faces are monochrome. Seed the separate color atlas
            // explicitly to check its invalidation, not color-font shaping.
            let color_atlas = &mut cached.cpu.as_mut().unwrap().atlas_color;
            let old_color_pixels = color_atlas.pixels().as_ptr();
            assert!(color_atlas
                .insert(
                    old_key,
                    crate::grid::RasterizedGlyph {
                        width: 1,
                        height: 1,
                        bearing_x: 0,
                        bearing_y: 0,
                        bytes: &[64, 32, 16, 128],
                    }
                )
                .is_some());

            cached.update_font(fonts);
            assert_eq!(cached.scale_factor, scale);
            assert!(Arc::ptr_eq(&cached.font_library.inner, &fonts.inner));
            assert!(old_run_weak.upgrade().is_none());
            assert_eq!(cached.shape_cache.len(), 0);
            assert!(cached.font_resolve.is_empty());
            assert!(cached.ascent_cache.is_empty());
            assert!(cached.synthesis_cache.is_empty());
            #[cfg(not(target_os = "macos"))]
            {
                assert!(cached.wght_variation_cache.is_empty());
                assert!(cached.font_data_cache.is_empty());
            }
            #[cfg(target_os = "macos")]
            assert!(cached.handle_cache.is_empty());
            assert!(cached.instances.is_empty() && cached.modal_instances.is_empty());
            assert!(!cached.recording_modal);
            let atlas = &cached.cpu.as_ref().unwrap().atlas_grayscale;
            assert!(
                std::ptr::eq(atlas.pixels().as_ptr(), old_pixels),
                "retain CPU atlas allocation"
            );
            assert!(
                atlas.lookup(old_key).is_none(),
                "discard old font-slot identity"
            );
            let color_atlas = &cached.cpu.as_ref().unwrap().atlas_color;
            assert!(std::ptr::eq(
                color_atlas.pixels().as_ptr(),
                old_color_pixels
            ));
            assert!(color_atlas.lookup(old_key).is_none());

            let mut fresh = Text::new(fonts);
            fresh.set_scale_factor(scale);
            let expected = frame(&mut fresh, "Actual glyph pixels", &opts);
            let actual = frame(&mut cached, "Actual glyph pixels", &opts);
            assert_eq!(actual.0, expected.0);
            let first_difference = actual
                .1
                .iter()
                .zip(&expected.1)
                .position(|(actual, expected)| actual != expected);
            assert_eq!(first_difference, None, "font replacement pixel mismatch");
        }
    }
}

#[test]
fn font_replacement_does_not_initialize_an_unused_backend() {
    let fonts = fixture_fonts();
    let mut text = Text::new(&fonts);
    assert!(text.measure("Measured only", &DrawOpts::default()) > 0.0);
    assert!(text.cpu.is_none());
    text.update_font(&fonts);
    assert!(text.cpu.is_none());
    assert!(text.font_resolve.is_empty());
    assert_eq!(text.shape_cache.len(), 0);
    #[cfg(all(feature = "wgpu", not(target_os = "macos")))]
    assert!(text.wgpu.is_none());
    #[cfg(target_os = "linux")]
    assert!(text.vulkan.is_none());
    #[cfg(target_os = "macos")]
    assert!(text.metal.is_none());
}
