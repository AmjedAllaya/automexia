#[test]
fn candidate_capacity_growth_is_amortized_and_bounded() {
    use super::*;

    let source = "x".repeat(4096);
    for edge in [Edge::End, Edge::Start] {
        let mut view = BoundaryView::new(&source, edge);
        assert_eq!(view.ensure(2048), 2048);
        let mut scratch = String::new();
        let mut growths = 0;
        let mut previous = 0;
        for power in 0..=11 {
            let count = 1 << power;
            assert!(view.candidate(count, "…", &mut scratch).is_some());
            assert!(scratch.capacity() >= count + 3);
            assert!(scratch.capacity() <= 16_387);
            if scratch.capacity() != previous {
                growths += 1;
                previous = scratch.capacity();
            }
        }
        // Capacity transitions, not allocator addresses or RSS, are the oracle.
        // Small successive probes must not each request another larger buffer.
        assert!(growths <= 7, "capacity growth count: {growths}");
    }
}

#[test]
fn bounded_candidate_storage_is_reused_and_short_boundary_views_stay_inline() {
    use super::*;

    let source = "abcdef".repeat(400);
    for edge in [Edge::End, Edge::Start] {
        let mut view = BoundaryView::new(&source, edge);
        assert_eq!(view.ensure(63), 63);
        assert!(!view.offsets.spilled());
        assert_eq!(view.ensure(64), 64);
        assert!(view.offsets.spilled());
        assert_eq!(view.ensure(usize::MAX), 2048);
        assert!(view.offsets.capacity() <= 2049);

        let mut scratch = String::with_capacity(256);
        let allocation = scratch.as_ptr();
        for count in [1, 2, 16, 64, 3] {
            let candidate = view.candidate(count, "…", &mut scratch).unwrap();
            assert!(matches!(candidate.display, Cow::Borrowed(_)));
            assert!(std::ptr::eq(candidate.display.as_ptr(), allocation));
            assert_eq!(candidate.retained, Some(count));
            let expected = match edge {
                Edge::End => format!("{}…", &source[..count]),
                Edge::Start => format!("…{}", &source[source.len() - count..]),
            };
            assert_eq!(candidate.display, expected);
            assert_eq!(scratch.capacity(), 256);
        }
        // An overshoot followed by a smaller winner must not expose stale
        // bytes or perform another allocation when the buffer is sufficient.
        for count in [1024, 2048, 17] {
            let candidate = view.candidate(count, "...", &mut scratch).unwrap();
            assert_eq!(candidate.display.len(), count + 3);
            assert!(scratch.capacity() <= 16_387);
        }
    }
}

#[test]
fn candidate_order_and_retained_identity_remain_stable_during_refinement() {
    use super::*;

    // Literal probe traces characterize the existing measured solver, including
    // rejected full input and the last accepted prefix/suffix after overshoot.
    for (edge, expected, display) in [
        (
            Edge::End,
            ["abcdefghi", "…", "a…", "ab…", "abcd…", "abc…"],
            "abc…",
        ),
        (
            Edge::Start,
            ["abcdefghi", "…", "…i", "…hi", "…fghi", "…ghi"],
            "…ghi",
        ),
    ] {
        let mut calls = Vec::new();
        let result = fit("abcdefghi", 4.0, edge, "…", |candidate, retained| {
            calls.push((candidate.to_owned(), retained));
            candidate.chars().count() as f32
        });
        assert_eq!(
            calls
                .iter()
                .map(|(value, _)| value.as_str())
                .collect::<Vec<_>>(),
            expected
        );
        assert_eq!(
            calls.iter().map(|(_, count)| *count).collect::<Vec<_>>(),
            [None, Some(0), Some(1), Some(2), Some(4), Some(3)]
        );
        assert_eq!(result.display, display);
        assert_eq!(result.retained, Some(3));
    }
    let source = "x".repeat(65);
    let result = fit_end(&source, 100.0, "…", |value, _| value.len() as f32);
    assert!(matches!(result.display, Cow::Borrowed(_)));
    assert!(std::ptr::eq(result.display.as_ptr(), source.as_ptr()));
    assert_eq!(result.retained, None);
}

#[test]
fn literal_cluster_fixtures_preserve_both_edges_and_whitespace() {
    use super::*;
    let scalar_measure = |value: &str, _: Option<usize>| value.chars().count() as f32;

    // Expected cuts are literal, independently reviewed cluster boundaries.
    for (value, maximum, end, start) in [
        ("", 1.0, "", ""),
        ("abc", 0.5, "", ""),
        ("abc", 1.0, "…", "…"),
        ("abc", 2.0, "a…", "…c"),
        ("abc", 3.0, "abc", "abc"),
        (" a b ", 4.0, " a …", "… b "),
        ("e\u{301}ab", 3.0, "e\u{301}…", "…ab"),
        ("👨\u{200d}💻ab", 4.0, "👨\u{200d}💻…", "…ab"),
        ("🇫🇷ab", 3.0, "🇫🇷…", "…ab"),
        ("क्षab", 4.0, "क्ष…", "…ab"),
        ("qa\u{301}", 2.0, "q…", "…"),
        ("界ab", 2.0, "界…", "…b"),
        ("aאב", 2.0, "a…", "…ב"),
    ] {
        let original = value.to_string();
        assert_eq!(
            fit_end(&original, maximum, "…", scalar_measure).display,
            end
        );
        assert_eq!(
            fit_start(&original, maximum, "…", scalar_measure).display,
            start
        );
        assert_eq!(original, value);
    }
}

#[test]
fn marker_identity_does_not_confuse_original_dots_with_generated_dots() {
    use super::*;
    let scalar_measure = |value: &str, _: Option<usize>| value.chars().count() as f32;

    let original = String::from("literal...");
    let exact = fit_end(&original, 10.0, "...", scalar_measure);
    assert!(matches!(exact.display, Cow::Borrowed(_)));
    assert!(std::ptr::eq(exact.display.as_ptr(), original.as_ptr()));
    assert_eq!(exact.retained, None);
    for (result, expected) in [
        (fit_end("abcdef", 4.0, "...", scalar_measure), "a..."),
        (fit_start("abcdef", 4.0, "...", scalar_measure), "...f"),
    ] {
        assert_eq!(result.display, expected);
        assert_eq!(result.retained, Some(1));
    }
    assert_eq!(fit_end("abcdef", 2.0, "...", scalar_measure).display, "");
    assert_eq!(
        fit_end("abcdef", 4.0, "unbounded marker", scalar_measure).display,
        ""
    );
}

#[test]
fn nonfinite_or_negative_measurements_never_certify_a_label() {
    use super::*;

    for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, -0.1] {
        assert_eq!(fit_end("label", 2.0, "…", |_, _| bad).display, "");
        assert_eq!(fit_start("label", 2.0, "…", |_, _| bad).display, "");
    }
}

#[test]
fn nonmonotonic_measurement_returns_only_an_independently_confirmed_fit() {
    use super::*;

    let oracle = |text: &str, _: Option<usize>| match text {
        "…" => 1.0,
        "a…" => 9.0,
        "ab…" => 2.0,
        _ => 20.0,
    };
    let result = fit_end("abcdef", 3.0, "…", oracle);
    assert_eq!(result.display, "…");
    assert!(oracle(&result.display, result.retained) <= 3.0);
    // A conservative result is intentional; do not binary-search under an
    // unsupported assumption that every font's shaped widths are monotonic.
}

#[test]
fn bounded_views_probes_and_offsets_do_not_scale_with_whole_input() {
    use super::*;
    let scalar_measure = |value: &str, _: Option<usize>| value.chars().count() as f32;

    assert_eq!(MAX_VIEW_BYTES, 16_384);
    assert_eq!(MAX_RETAINED_GRAPHEMES, 2_048);
    assert_eq!(MAX_MEASUREMENTS, 26);
    let cases = [
        "a".repeat(16_383),
        "a".repeat(16_384),
        "a".repeat(16_385),
        "a".repeat(MAX_VIEW_BYTES * 8),
        "e\u{301}👩\u{200d}💻 界".repeat(MAX_VIEW_BYTES),
        format!("a{}", "\u{301}".repeat(MAX_VIEW_BYTES * 2)),
        "🇫".repeat(MAX_VIEW_BYTES),
    ];
    for value in &cases {
        for edge in [Edge::End, Edge::Start] {
            for maximum in [0.5, 1.0, 16.0, 2048.0, 100_000.0] {
                let mut calls = 0;
                let result = fit(value, maximum, edge, "…", |candidate, retained| {
                    calls += 1;
                    assert!(candidate.len() <= MAX_VIEW_BYTES + 3);
                    assert!(retained.is_some_and(|count| count <= MAX_RETAINED_GRAPHEMES));
                    scalar_measure(candidate, retained)
                });
                assert!(calls <= MAX_MEASUREMENTS);
                assert!(result.display.len() <= MAX_VIEW_BYTES + 3);
                assert!(scalar_measure(&result.display, result.retained) <= maximum);
            }
            let mut view = BoundaryView::new(value, edge);
            view.ensure(usize::MAX);
            assert!(view.chunk.len() <= MAX_VIEW_BYTES);
            assert!(view.offsets.len() <= MAX_RETAINED_GRAPHEMES + 1);
            assert!(view.offsets.capacity() <= MAX_RETAINED_GRAPHEMES + 1);
        }
    }
}

#[test]
fn partial_context_is_not_treated_as_a_grapheme_boundary() {
    use super::*;
    let scalar_measure = |value: &str, _: Option<usize>| value.chars().count() as f32;

    let one_cluster = format!("a{}", "\u{301}".repeat(MAX_VIEW_BYTES * 2));
    for edge in [Edge::Start, Edge::End] {
        assert_eq!(
            fit(&one_cluster, 100_000.0, edge, "…", scalar_measure).display,
            "…"
        );
    }
    let prefix = format!("ok{one_cluster}");
    assert_eq!(fit_end(&prefix, 4.0, "…", scalar_measure).display, "ok…");
    let suffix = format!("{one_cluster}ok");
    assert_eq!(fit_start(&suffix, 4.0, "…", scalar_measure).display, "…ok");
}

#[test]
fn actual_font_styles_scales_and_cpu_rasters_use_the_measured_candidate() {
    use super::*;
    use rio_backend::sugarloaf::{
        font::{constants, FontData, FontLibrary, FontLibraryData},
        text::{DrawOpts, Text},
    };
    use std::sync::Arc;
    fn fixture_fonts() -> FontLibrary {
        let mut data = FontLibraryData::default();
        data.insert(
            FontData::from_static_slice(constants::FONT_CASCADIA_CODE_NF).unwrap(),
        );
        data.insert(
            FontData::from_static_slice_with_wght(
                constants::FONT_CASCADIA_CODE_NF,
                Some(700.0),
            )
            .unwrap(),
        );
        data.insert(
            FontData::from_static_slice(constants::FONT_CASCADIA_CODE_NF_ITALIC).unwrap(),
        );
        data.insert(
            FontData::from_static_slice_with_wght(
                constants::FONT_CASCADIA_CODE_NF_ITALIC,
                Some(700.0),
            )
            .unwrap(),
        );
        FontLibrary {
            inner: Arc::new(parking_lot::RwLock::new(data)),
        }
    }

    let fonts = fixture_fonts();
    let original = String::from("A wider public label with e\u{301} and punctuation...");
    for scale in [1.0, 1.25, 2.5] {
        for (index, bold, italic) in [
            (0, false, false),
            (1, true, false),
            (2, false, true),
            (3, true, true),
        ] {
            for size in [12.4, 12.6, 19.5] {
                let options = DrawOpts {
                    font_size: size,
                    bold,
                    italic,
                    font_id: Some(index),
                    ..DrawOpts::default()
                };
                let mut text = Text::new(&fonts);
                text.set_scale_factor(scale);
                text.init_cpu();
                for maximum in [1.0, 36.0, 80.0, 180.0] {
                    for edge in [Edge::Start, Edge::End] {
                        let result =
                            fit(&original, maximum, edge, "…", |candidate, _| {
                                text.measure(candidate, &options)
                            });
                        let mut reference = Text::new(&fonts);
                        reference.set_scale_factor(scale);
                        reference.init_cpu();
                        let measured = reference.measure(&result.display, &options);
                        assert!(measured.is_finite() && measured <= maximum);
                        text.clear();
                        assert_eq!(
                            text.draw(4.0, 4.0, &result.display, &options),
                            measured
                        );
                        assert_eq!(
                            reference.draw(4.0, 4.0, &result.display, &options),
                            measured
                        );
                        let mut actual = vec![0x00112233; 512 * 96];
                        let mut expected = actual.clone();
                        // Both phases cover all queued instances outside Sugarloaf's
                        // private frame finalizer; no synthetic glyph state is injected.
                        text.render_cpu_base(&mut actual, 512, 96);
                        text.render_cpu_modal(&mut actual, 512, 96);
                        reference.render_cpu_base(&mut expected, 512, 96);
                        reference.render_cpu_modal(&mut expected, 512, 96);
                        let difference =
                            actual.iter().zip(&expected).position(|(a, b)| a != b);
                        assert_eq!(difference, None, "first changed CPU pixel");
                        if !result.display.is_empty() {
                            assert!(actual.iter().any(|pixel| *pixel != 0x00112233));
                        }
                    }
                }
            }
        }
    }
    assert_eq!(
        original,
        "A wider public label with e\u{301} and punctuation..."
    );
}
