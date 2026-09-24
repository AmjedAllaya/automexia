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

#[test]
fn cell_clipping_contains_real_glyph_ink_at_fractional_scales() {
    for scale in [1.0, 1.25, 1.5, 2.0] {
        let fonts = fixture_fonts();
        let mut text = Text::new(&fonts);
        text.init_cpu();
        text.set_scale_factor(scale);
        let opts = DrawOpts {
            font_size: 18.0,
            italic: true,
            ..DrawOpts::default()
        };
        let clip = [8.0, 6.0, 17.0, 16.0];
        let measured = text.measure("Wide overflow", &opts);
        let advance = text.draw_clipped(7.0, 4.0, "Wide overflow", &opts, clip);
        assert_eq!(advance, measured);
        assert!(!text.instances.is_empty());
        for glyph in &text.instances {
            let left = glyph.pos[0] + glyph.bearings[0] as f32;
            let top = glyph.pos[1] + glyph.bearings[1] as f32;
            assert!(left >= 8.0 * scale && top >= 6.0 * scale);
            assert!(left + glyph.glyph_size[0] as f32 <= 25.0 * scale);
            assert!(top + glyph.glyph_size[1] as f32 <= 22.0 * scale);
        }
        text.clear();
        text.begin_modal_layer();
        text.draw_clipped(0.0, 0.0, "offscreen", &opts, [50.0, 50.0, 3.0, 3.0]);
        assert!(text.modal_instances.is_empty());
        text.draw_clipped(0.0, 0.0, "invalid", &opts, [f32::NAN, 0.0, 20.0, 20.0]);
        assert!(text.modal_instances.is_empty());
    }
}

#[test]
fn shaped_run_uses_final_font_color_metadata_for_overrides_and_cache() {
    // Synthetic metadata isolates selection/cache ownership on every platform;
    // the macOS-only test below owns actual color-font raster evidence.
    let fonts = fixture_fonts();
    let mut color =
        FontData::from_static_slice(constants::FONT_CASCADIA_CODE_NF).unwrap();
    color.is_emoji = true;
    fonts.inner.write().insert(color);
    let mut text = Text::new(&fonts);
    let normal = DrawOpts::default();
    let overridden = DrawOpts {
        font_id: Some(4),
        ..normal
    };
    let automatic = text.shape_for("A", &normal).unwrap();
    assert_eq!(automatic.font_id, 0);
    assert!(!automatic.is_color);
    let color = text.shape_for("A", &overridden).unwrap();
    assert_eq!(color.font_id, 4);
    assert!(color.is_color, "the explicit font owns its bitmap format");
    assert!(Arc::ptr_eq(
        &color,
        &text.shape_for("A", &overridden).unwrap()
    ));
    assert!(Arc::ptr_eq(
        &automatic,
        &text.shape_for("A", &normal).unwrap()
    ));
    assert!(!text.shape_for("A", &normal).unwrap().is_color);
}

#[test]
fn font_replacement_refreshes_color_metadata_for_reused_ids() {
    let mono = fixture_fonts();
    let mut color_font =
        FontData::from_static_slice(constants::FONT_CASCADIA_CODE_NF).unwrap();
    color_font.is_emoji = true;
    let color = fixture_library(color_font);
    let opts = DrawOpts {
        font_id: Some(0),
        ..DrawOpts::default()
    };
    let mut text = Text::new(&mono);
    assert!(!text.shape_for("A", &opts).unwrap().is_color);
    for fonts in [&color, &mono, &color] {
        text.update_font(fonts);
        let expected = fonts.inner.read().get(&0).is_emoji;
        assert_eq!(text.shape_for("A", &opts).unwrap().is_color, expected);
        assert_eq!(text.shape_for("A", &opts).unwrap().is_color, expected);
    }
}

#[cfg(target_os = "macos")]
#[test]
fn macos_color_font_draw_clipped_uses_color_atlas() {
    let fonts = fixture_fonts();
    let mut text = Text::new(&fonts);
    text.init_cpu();
    let opts = DrawOpts {
        font_size: 32.0,
        ..DrawOpts::default()
    };
    // CoreText resolves the native color face. A missing native fixture must
    // fail this test rather than silently turn into unexercised evidence.
    let run = text.shape_for("😀", &opts).unwrap();
    assert!(fonts.inner.read().get(&(run.font_id as usize)).is_emoji);
    let advance = text.draw_clipped(8.0, 8.0, "😀", &opts, [8.0, 8.0, 32.0, 40.0]);
    assert!(advance > 0.0);
    assert!(
        text.instances.iter().any(|instance| instance.atlas == 1),
        "native color glyphs must use the RGBA atlas"
    );
    text.finalize_modal_layer();
    let mut pixels = vec![0x00112233; 64 * 64];
    text.render_cpu_base(&mut pixels, 64, 64);
    let mut colored = false;
    for y in 0..64 {
        for x in 0..64 {
            let pixel = pixels[y * 64 + x];
            if pixel == 0x00112233 {
                continue;
            }
            assert!((8..40).contains(&x) && (8..48).contains(&y));
            let r = (pixel >> 16) & 255;
            let g = (pixel >> 8) & 255;
            let b = pixel & 255;
            colored |= r.abs_diff(g) > 24 || g.abs_diff(b) > 24;
        }
    }
    assert!(colored, "the native fixture must render visible color ink");
}

#[test]
fn fixed_cells_preserve_bundled_font_contextual_glyphs() {
    let mut text = Text::new(&fixture_fonts());
    text.init_cpu();
    let opts = DrawOpts::default();
    let sample = "==";
    let shaped = text.shape_for(sample, &opts).unwrap();
    let alone = text.shape_for("=", &opts).unwrap();
    assert_ne!(
        shaped.glyphs.iter().map(|g| g.id).collect::<Vec<_>>(),
        alone
            .glyphs
            .iter()
            .chain(alone.glyphs.iter())
            .map(|g| g.id)
            .collect::<Vec<_>>(),
        "the fixture must prove a real contextual shaping difference"
    );
    let expected: Vec<_> = shaped
        .glyphs
        .iter()
        .filter_map(|g| {
            text.rasterize_slot(&shaped, g.id)
                .filter(|(s, _)| s.w > 0 && s.h > 0)
                .map(|(s, color)| (s.x as u32, s.y as u32, u8::from(color)))
        })
        .collect();
    assert!(!expected.is_empty());
    let anchors = [
        TextCellAnchor {
            byte_offset: 0,
            column: 0,
        },
        TextCellAnchor {
            byte_offset: 1,
            column: 1,
        },
    ];
    text.draw_cells_clipped(
        8.0,
        8.0,
        sample,
        &opts,
        TextCellLayout {
            cell_width: 10.0,
            anchors: &anchors,
        },
        [0.0, 0.0, 64.0, 64.0],
    );
    let actual: Vec<_> = text
        .instances
        .iter()
        .map(|g| (g.glyph_pos[0], g.glyph_pos[1], g.atlas))
        .collect();
    assert_eq!(
        actual, expected,
        "fixed-cell output must use the whole-run glyphs"
    );
}

#[test]
fn fixed_cells_keep_ascii_stride_and_clipping_at_fractional_scales() {
    let anchors: Vec<_> = (0..24)
        .map(|column| TextCellAnchor {
            byte_offset: column,
            column,
        })
        .collect();
    let sample = "iiiiiiiiiiiiiiiiiiiiiiii";
    for scale in [1.0, 1.25, 1.5, 2.0] {
        for modal in [false, true] {
            let mut mapped = Text::new(&fixture_fonts());
            let mut expected = Text::new(&fixture_fonts());
            let opts = DrawOpts {
                font_size: 18.0,
                italic: true,
                ..DrawOpts::default()
            };
            let clip = [8.0, 6.0, 163.0, 20.0];
            for text in [&mut mapped, &mut expected] {
                text.init_cpu();
                text.set_scale_factor(scale);
                // Cropping a table run must not change an earlier draw.
                text.draw(200.0, 4.0, "X", &opts);
                if modal {
                    text.begin_modal_layer();
                }
            }
            mapped.draw_cells_clipped(
                7.0,
                4.0,
                sample,
                &opts,
                TextCellLayout {
                    cell_width: 7.0,
                    anchors: &anchors,
                },
                clip,
            );
            for anchor in &anchors {
                expected.draw_clipped(
                    7.0 + anchor.column as f32 * 7.0,
                    4.0,
                    "i",
                    &opts,
                    clip,
                );
            }
            assert_same_instances(&mapped.instances, &expected.instances);
            assert_same_instances(&mapped.modal_instances, &expected.modal_instances);
        }
    }
}

#[test]
fn fixed_cells_reject_malformed_maps_before_emitting() {
    let mut text = Text::new(&fixture_fonts());
    text.init_cpu();
    let opts = DrawOpts::default();
    text.draw(0.0, 0.0, "keep", &opts);
    let before = text.instances.clone();
    let cases: &[&[(usize, usize)]] = &[
        &[],
        &[(1, 0)],
        &[(0, 0), (1, 1)],
        &[(0, 0), (3, 1)],
        &[(0, 0), (0, 1)],
        &[(0, 1), (2, 0)],
        &[(0, 0), (usize::MAX, 1)],
    ];
    for case in cases {
        let anchors: Vec<_> = case
            .iter()
            .map(|&(byte_offset, column)| TextCellAnchor {
                byte_offset,
                column,
            })
            .collect();
        text.draw_cells_clipped(
            0.0,
            0.0,
            "éa",
            &opts,
            TextCellLayout {
                cell_width: 8.0,
                anchors: &anchors,
            },
            [0.0, 0.0, 64.0, 64.0],
        );
        assert_same_instances(&text.instances, &before);
    }
    let anchors = [TextCellAnchor {
        byte_offset: 0,
        column: 0,
    }];
    for cell_width in [0.0, -1.0, f32::NAN, f32::INFINITY] {
        text.draw_cells_clipped(
            0.0,
            0.0,
            "éa",
            &opts,
            TextCellLayout {
                cell_width,
                anchors: &anchors,
            },
            [0.0, 0.0, 64.0, 64.0],
        );
    }
    for font_size in [0.0, -1.0, f32::NAN, f32::INFINITY] {
        text.draw_cells_clipped(
            0.0,
            0.0,
            "éa",
            &DrawOpts { font_size, ..opts },
            TextCellLayout {
                cell_width: 8.0,
                anchors: &anchors,
            },
            [0.0, 0.0, 64.0, 64.0],
        );
    }
    text.draw_cells_clipped(
        f32::NAN,
        0.0,
        "éa",
        &opts,
        TextCellLayout {
            cell_width: 8.0,
            anchors: &anchors,
        },
        [0.0, 0.0, 64.0, 64.0],
    );
    assert_same_instances(&text.instances, &before);
}

#[test]
fn fixed_cells_group_fallback_fonts_and_respect_explicit_overrides() {
    let fonts = fixture_fonts();
    let mut fallback =
        FontData::from_static_slice(constants::FONT_CASCADIA_CODE_NF_ITALIC).unwrap();
    fallback.is_emoji = true; // Metadata ownership only; the fixture has monochrome glyphs.
    fonts.inner.write().insert(fallback);
    let mut text = Text::new(&fonts);
    text.init_cpu();
    // Make font selection deterministic without relying on installed system fonts.
    text.font_resolve.insert(('A', 0), (0, false));
    text.font_resolve.insert(('B', 0), (4, true));
    let anchors: Vec<_> = (0..4)
        .map(|column| TextCellAnchor {
            byte_offset: column,
            column,
        })
        .collect();
    let opts = DrawOpts::default();
    text.draw_cells_clipped(
        0.0,
        0.0,
        "ABBA",
        &opts,
        TextCellLayout {
            cell_width: 10.0,
            anchors: &anchors,
        },
        [0.0, 0.0, 64.0, 64.0],
    );
    let key = ShapeKey {
        font_id: 4,
        size: 14,
        style_flags: 0,
    };
    let fallback_run = text
        .shape_cache
        .get(key, "BB")
        .expect("adjacent fallback cells share one run");
    assert!(fallback_run.is_color);
    assert!(text
        .shape_cache
        .get(ShapeKey { font_id: 0, ..key }, "ABBA")
        .is_none());
    text.clear();
    text.draw_cells_clipped(
        0.0,
        0.0,
        "ABBA",
        &DrawOpts {
            font_id: Some(0),
            ..opts
        },
        TextCellLayout {
            cell_width: 10.0,
            anchors: &anchors,
        },
        [0.0, 0.0, 64.0, 64.0],
    );
    let overridden = text
        .shape_cache
        .get(ShapeKey { font_id: 0, ..key }, "ABBA")
        .expect("override owns the complete run");
    assert!(!overridden.is_color);
}

#[test]
fn fixed_cells_map_utf8_clusters_to_original_nonuniform_columns() {
    let fonts = fixture_fonts();
    let mut text = Text::new(&fonts);
    text.init_cpu();
    let opts = DrawOpts {
        font_id: Some(0),
        ..DrawOpts::default()
    };
    // Combining marks stay at their base anchor; the following glyph uses the
    // caller's original VT column, not the number of bytes or shaped glyphs.
    let anchors = [
        TextCellAnchor {
            byte_offset: 0,
            column: 0,
        },
        TextCellAnchor {
            byte_offset: 3,
            column: 4,
        },
    ];
    text.draw_cells_clipped(
        8.0,
        8.0,
        "éi",
        &opts,
        TextCellLayout {
            cell_width: 10.0,
            anchors: &anchors,
        },
        [0.0, 0.0, 80.0, 64.0],
    );
    let last = text.instances.last().expect("visible final glyph");
    let origin = last.pos[0] + last.bearings[0] as f32;
    assert!(
        (48.0..58.0).contains(&origin),
        "the final glyph must use VT column four: {origin}"
    );
    assert!(text
        .instances
        .iter()
        .any(|g| g.pos[0] + (g.bearings[0] as f32) < 28.0));
}

#[cfg(target_os = "macos")]
#[test]
fn fixed_cells_preserve_native_coretext_arabic_context() {
    let mut text = Text::new(&fixture_fonts());
    text.init_cpu();
    let opts = DrawOpts::default();
    let sample = "سلام";
    let font = text.resolve_font_id('س', &opts);
    assert!(
        sample
            .chars()
            .all(|ch| text.resolve_font_id(ch, &opts) == font),
        "native Arabic fixture must share a font"
    );
    let together = text.shape_for(sample, &opts).unwrap();
    let separated: Vec<_> = sample
        .chars()
        .flat_map(|ch| {
            text.shape_for(&ch.to_string(), &opts)
                .unwrap()
                .glyphs
                .iter()
                .map(|g| g.id)
                .collect::<Vec<_>>()
        })
        .collect();
    assert_ne!(
        together.glyphs.iter().map(|g| g.id).collect::<Vec<_>>(),
        separated,
        "native fixture must exercise contextual forms"
    );
    let expected: Vec<_> = together
        .glyphs
        .iter()
        .filter_map(|g| {
            text.rasterize_slot(&together, g.id)
                .filter(|(s, _)| s.w > 0 && s.h > 0)
                .map(|(s, color)| (s.x as u32, s.y as u32, u8::from(color)))
        })
        .collect();
    assert!(!expected.is_empty());
    let anchors: Vec<_> = sample
        .char_indices()
        .enumerate()
        .map(|(column, (byte_offset, _))| TextCellAnchor {
            byte_offset,
            column,
        })
        .collect();
    text.draw_cells_clipped(
        16.0,
        8.0,
        sample,
        &opts,
        TextCellLayout {
            cell_width: 14.0,
            anchors: &anchors,
        },
        [0.0, 0.0, 256.0, 64.0],
    );
    let actual: Vec<_> = text
        .instances
        .iter()
        .map(|g| (g.glyph_pos[0], g.glyph_pos[1], g.atlas))
        .collect();
    assert_eq!(actual, expected);
}

fn assert_same_instances(actual: &[TextInstance], expected: &[TextInstance]) {
    assert_eq!(actual.len(), expected.len());
    for (index, (a, b)) in actual.iter().zip(expected).enumerate() {
        assert_eq!(
            (
                a.pos,
                a.glyph_pos,
                a.glyph_size,
                a.bearings,
                a.color,
                a.atlas,
                a.page
            ),
            (
                b.pos,
                b.glyph_pos,
                b.glyph_size,
                b.bearings,
                b.color,
                b.atlas,
                b.page
            ),
            "instance {index}"
        );
    }
}
