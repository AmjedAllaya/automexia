#[test]
fn matched_runs_preserve_combining_and_joined_graphemes() {
    use super::*;
    assert_eq!(
        matched_grapheme_runs("e\u{301}cho 👨\u{200d}💻", &[0, 5]).collect::<Vec<_>>(),
        vec![(true, "e\u{301}"), (false, "cho "), (true, "👨\u{200d}💻"),]
    );
}

#[test]
fn legacy_elision_preserves_literal_cuts_and_whitespace() {
    use super::*;
    use rio_backend::sugarloaf::font::{
        constants, FontData, FontLibrary, FontLibraryData,
    };
    use std::sync::Arc;

    let mut data = FontLibraryData::default();
    data.insert(FontData::from_static_slice(constants::FONT_CASCADIA_CODE_NF).unwrap());
    let fonts = FontLibrary {
        inner: Arc::new(parking_lot::RwLock::new(data)),
    };
    let options = DrawOpts::default();
    let mut text = Text::new(&fonts);
    for (source, budget_label, expected) in [
        ("", "x", ""),
        ("abc", "abc", "abc"),
        ("abcdefgh", "abc...", "abc..."),
        ("abcdefgh", "...", "..."),
        ("abcdefgh", ".", ""),
        (" abcd efgh ", " ab...", " ab..."),
        (" ab ", " ab ", " ab "),
    ] {
        let original = source.to_owned();
        let budget = text.measure(budget_label, &options);
        assert_eq!(
            fit_plain(&mut text, &original, budget, &options).display,
            expected
        );
        assert_eq!(original, source);
    }
    for width in [0.0, -1.0] {
        assert_eq!(fit_plain(&mut text, "abc", width, &options).display, "");
    }
}

#[test]
fn private_draw_adapters_preserve_independent_literal_cpu_frames() {
    use super::*;
    use rio_backend::sugarloaf::font::{
        constants, FontData, FontLibrary, FontLibraryData,
    };
    use std::sync::Arc;

    fn fixture_fonts() -> FontLibrary {
        let mut data = FontLibraryData::default();
        data.insert(
            FontData::from_static_slice(constants::FONT_CASCADIA_CODE_NF).unwrap(),
        );
        for _ in 0..3 {
            data.insert_alias(0);
        }
        FontLibrary {
            inner: Arc::new(parking_lot::RwLock::new(data)),
        }
    }

    fn prepared_text(fonts: &FontLibrary, scale: f32) -> Text {
        let mut text = Text::new(fonts);
        text.set_scale_factor(scale);
        text.init_cpu();
        text
    }

    fn pixels(text: &mut Text) -> Vec<u32> {
        let mut pixels = vec![0x00112233; 512 * 96];
        text.render_cpu_base(&mut pixels, 512, 96);
        text.render_cpu_modal(&mut pixels, 512, 96);
        pixels
    }

    let fonts = fixture_fonts();
    for scale in [1.0, 1.25, 2.5] {
        for size in [12.4, 12.6, 19.5] {
            let normal = DrawOpts {
                font_size: size,
                color: [230, 240, 250, 255],
                ..DrawOpts::default()
            };
            let highlighted = DrawOpts {
                color: [10, 220, 240, 255],
                ..normal
            };
            for matched in [false, true] {
                let mut actual = prepared_text(&fonts, scale);
                let mut reference = prepared_text(&fonts, scale);
                let original = String::from("abcdefghijklmnop");
                let budget = reference.measure("abc...", &normal);
                // The oracle issues literal known runs, independently of the
                // migrated partitioner and elision algorithm.
                if matched {
                    draw_matched_text(
                        &mut actual,
                        (4.0, 4.0),
                        &original,
                        &[0, 1, 7],
                        budget,
                        &normal,
                        &highlighted,
                    );
                    let advance = reference.draw(4.0, 4.0, "ab", &highlighted);
                    reference.draw(4.0 + advance, 4.0, "c...", &normal);
                } else {
                    draw_text(&mut actual, (4.0, 4.0), &original, budget, &normal);
                    reference.draw(4.0, 4.0, "abc...", &normal);
                }
                let actual = pixels(&mut actual);
                let expected = pixels(&mut reference);
                assert!(actual.iter().any(|pixel| *pixel != 0x00112233));
                assert_eq!(
                    actual.iter().zip(&expected).position(|(a, b)| a != b),
                    None,
                    "first changed CPU pixel: scale={scale}, size={size}, matched={matched}"
                );
                assert_eq!(original, "abcdefghijklmnop");
            }
        }
    }
}

#[test]
fn generated_marker_does_not_inherit_removed_source_highlights() {
    use super::*;
    use rio_backend::sugarloaf::font::{
        constants, FontData, FontLibrary, FontLibraryData,
    };
    use std::sync::Arc;

    let mut data = FontLibraryData::default();
    data.insert(FontData::from_static_slice(constants::FONT_CASCADIA_CODE_NF).unwrap());
    let fonts = FontLibrary {
        inner: Arc::new(parking_lot::RwLock::new(data)),
    };
    for scale in [1.0, 1.25, 2.5] {
        for size in [12.4, 12.6, 19.5] {
            let normal = DrawOpts {
                font_size: size,
                color: [230, 240, 250, 255],
                ..DrawOpts::default()
            };
            let highlighted = DrawOpts {
                color: [10, 220, 240, 255],
                ..normal
            };
            let mut actual = Text::new(&fonts);
            let mut reference = Text::new(&fonts);
            for text in [&mut actual, &mut reference] {
                text.set_scale_factor(scale);
                text.init_cpu();
            }
            let original = String::from("abcdefghijklmnop");
            let budget = reference.measure("abc...", &normal);
            // These indices belong to removed d/e/f, not to the three
            // generated dots. The oracle never calls the fitting code.
            draw_matched_text(
                &mut actual,
                (4.0, 4.0),
                &original,
                &[3, 4, 5],
                budget,
                &normal,
                &highlighted,
            );
            reference.draw(4.0, 4.0, "abc...", &normal);
            let mut pixels = vec![0x00112233; 512 * 96];
            let mut expected = pixels.clone();
            actual.render_cpu_base(&mut pixels, 512, 96);
            actual.render_cpu_modal(&mut pixels, 512, 96);
            reference.render_cpu_base(&mut expected, 512, 96);
            reference.render_cpu_modal(&mut expected, 512, 96);
            assert!(pixels.iter().any(|pixel| *pixel != 0x00112233));
            assert_eq!(
                pixels.iter().zip(&expected).position(|(a, b)| a != b),
                None,
                "generated-marker color changed: scale={scale}, size={size}"
            );
            assert_eq!(original, "abcdefghijklmnop");
        }
    }
}

#[test]
fn invalid_label_budget_preserves_previously_queued_pixels() {
    use super::*;
    use rio_backend::sugarloaf::font::{
        constants, FontData, FontLibrary, FontLibraryData,
    };
    use std::sync::Arc;

    let mut data = FontLibraryData::default();
    data.insert(FontData::from_static_slice(constants::FONT_CASCADIA_CODE_NF).unwrap());
    let fonts = FontLibrary {
        inner: Arc::new(parking_lot::RwLock::new(data)),
    };
    let mut text = Text::new(&fonts);
    text.init_cpu();
    let normal = DrawOpts::default();
    let highlighted = DrawOpts {
        color: [0, 220, 240, 255],
        ..normal
    };
    for budget in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, 0.0, -1.0] {
        for matched in [false, true] {
            text.clear();
            text.draw(4.0, 32.0, "OK", &normal);
            let mut expected = vec![0x00112233; 512 * 96];
            text.render_cpu_base(&mut expected, 512, 96);
            text.render_cpu_modal(&mut expected, 512, 96);
            let count = text.instances().len();
            assert!(count > 0);
            if matched {
                draw_matched_text(
                    &mut text,
                    (4.0, 4.0),
                    "label",
                    &[0, 1],
                    budget,
                    &normal,
                    &highlighted,
                );
            } else {
                draw_text(&mut text, (4.0, 4.0), "label", budget, &normal);
            }
            assert_eq!(
                text.instances().len(),
                count,
                "invalid budget queued a label"
            );
            let mut actual = vec![0x00112233; 512 * 96];
            text.render_cpu_base(&mut actual, 512, 96);
            text.render_cpu_modal(&mut actual, 512, 96);
            assert_eq!(
                actual.iter().zip(&expected).position(|(a, b)| a != b),
                None,
                "invalid budget changed existing pixels"
            );
        }
    }
}

#[test]
fn marker_provenance_preserves_prepend_boundaries_and_literal_source_dots() {
    use super::*;

    // The original Prepend cluster ends before a control. Appending dots
    // changes display segmentation, but must not change source ownership.
    let source = String::from("ab\u{600}\nz");
    let fitted = fit_end(&source, 6.0, "...", |_, retained| {
        retained.map_or(100.0, |count| (count + 3) as f32)
    });
    assert_eq!(fitted.display, "ab\u{600}...");
    assert_eq!(fitted.retained, Some(3));
    assert_eq!(
        fitted_runs(&fitted.display, fitted.retained, &[2]).collect::<Vec<_>>(),
        [(false, "ab"), (true, "\u{600}"), (false, "...")]
    );
    assert_eq!(source, "ab\u{600}\nz");

    let literal = fit_end("a...", 10.0, "...", |value, _| value.len() as f32);
    assert_eq!(literal.retained, None);
    assert_eq!(
        fitted_runs(&literal.display, literal.retained, &[1, 2, 3]).collect::<Vec<_>>(),
        [(false, "a"), (true, "...")]
    );
    assert_eq!(
        fitted_runs("abc...", Some(3), &[0, 1]).collect::<Vec<_>>(),
        [(true, "ab"), (false, "c...")]
    );
    assert!(fitted_runs("", Some(0), &[0]).next().is_none());
}

#[test]
fn matched_runs_borrow_contiguous_source_storage() {
    use super::*;

    let source = String::from("abcde...");
    let matches = [0, 2, usize::MAX];
    let runs = matched_grapheme_runs(&source, &matches).collect::<Vec<_>>();
    assert_eq!(
        runs,
        [(true, "a"), (false, "b"), (true, "c"), (false, "de...")]
    );
    for ((_, run), offset) in runs.iter().zip([0, 1, 2, 3]) {
        assert_eq!(run.as_ptr(), source[offset..].as_ptr());
    }
    assert_eq!(source, "abcde...");
}

#[test]
fn plain_fitting_bounds_the_real_font_path_without_changing_source() {
    use super::*;
    use rio_backend::sugarloaf::font::{
        constants, FontData, FontLibrary, FontLibraryData,
    };
    use std::sync::Arc;

    let mut data = FontLibraryData::default();
    data.insert(FontData::from_static_slice(constants::FONT_CASCADIA_CODE_NF).unwrap());
    let fonts = FontLibrary {
        inner: Arc::new(parking_lot::RwLock::new(data)),
    };
    let mut text = Text::new(&fonts);
    let source = "x".repeat(32_768);
    let options = DrawOpts::default();
    let fit = fit_plain(&mut text, &source, 1_000_000.0, &options);
    assert_eq!(fit.retained, Some(2_048));
    assert_eq!(fit.display, format!("{}...", "x".repeat(2_048)));
    assert_eq!(source.len(), 32_768);

    let exact = String::from(" abc ");
    let fit = fit_plain(&mut text, &exact, 100.0, &options);
    assert_eq!(fit.retained, None);
    assert_eq!(fit.display.as_ptr(), exact.as_ptr());
    assert_eq!(fit.display, " abc ");
}

#[test]
fn fitting_measures_the_exact_styled_runs_that_are_drawn() {
    use super::*;
    use rio_backend::sugarloaf::font::{
        constants, FontData, FontLibrary, FontLibraryData,
    };
    use std::sync::Arc;

    let mut data = FontLibraryData::default();
    data.insert(FontData::from_static_slice(constants::FONT_CASCADIA_CODE_NF).unwrap());
    let fonts = FontLibrary {
        inner: Arc::new(parking_lot::RwLock::new(data)),
    };
    for scale in [1.0, 1.25, 2.5] {
        for size in [12.4, 12.6, 19.5] {
            let normal = DrawOpts {
                font_size: size,
                color: [230, 240, 250, 255],
                ..DrawOpts::default()
            };
            // Different metrics make a whole-label/normal-style measurement
            // an invalid oracle, even when all characters are monospaced.
            let highlighted = DrawOpts {
                font_size: size * 1.6,
                color: [10, 220, 240, 255],
                ..normal
            };
            let mut actual = Text::new(&fonts);
            let mut reference = Text::new(&fonts);
            for text in [&mut actual, &mut reference] {
                text.set_scale_factor(scale);
                text.init_cpu();
            }
            let budget = reference.measure("ab", &highlighted)
                + reference.measure("c...", &normal);
            draw_matched_text(
                &mut actual,
                (4.0, 4.0),
                "abcdefghijklmnop",
                &[0, 1],
                budget,
                &normal,
                &highlighted,
            );
            let advance = reference.draw(4.0, 4.0, "ab", &highlighted);
            reference.draw(4.0 + advance, 4.0, "c...", &normal);
            let mut pixels = vec![0x00112233; 512 * 96];
            let mut expected = pixels.clone();
            actual.render_cpu_base(&mut pixels, 512, 96);
            actual.render_cpu_modal(&mut pixels, 512, 96);
            reference.render_cpu_base(&mut expected, 512, 96);
            reference.render_cpu_modal(&mut expected, 512, 96);
            assert!(pixels.iter().any(|pixel| *pixel != 0x00112233));
            assert_eq!(
                pixels.iter().zip(&expected).position(|(a, b)| a != b),
                None,
                "styled-run fitting: scale={scale}, size={size}"
            );
        }
    }
}
