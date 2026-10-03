use super::*;

fn stamp() -> SemanticCommandTimestamp {
    SemanticCommandTimestamp {
        unix_ms: 1_790_793_296_123,
        year: 2026,
        month: 9,
        day: 30,
        hour: 12,
        minute: 34,
        second: 56,
    }
}
fn label(appearance: TimestampAppearance) -> TimestampText {
    TimestampText::new(appearance, true, Some(stamp()), Some(0), Some(104))
}

#[test]
fn timestamp_defaults_preserve_existing_label_and_missing_metadata_is_not_invented() {
    assert_eq!(
        label(Default::default()).text,
        "✓  104ms  ·  2026-09-30 12:34:56"
    );
    assert_eq!(
        TimestampText::new(Default::default(), false, Some(stamp()), Some(0), Some(104))
            .text,
        "✓  104ms"
    );
    assert_eq!(
        TimestampText::new(Default::default(), true, None, None, None).text,
        "•  done"
    );
    let hidden = TimestampAppearance {
        date_format: Some(TimestampDateFormat::Hidden),
        time_format: Some(TimestampTimeFormat::Hidden),
        show_status: Some(false),
        show_duration: Some(false),
        ..Default::default()
    };
    let empty = label(hidden);
    assert!(empty.text.is_empty());
    assert!(empty
        .pack(&[], 100.0, 4.0, &[], None, 0.0, |_, _| 10.0)
        .unwrap()
        .fragments
        .is_empty());
}

#[test]
fn timestamp_formats_precision_order_and_separators_are_independent() {
    let appearance = TimestampAppearance {
        date_format: Some(TimestampDateFormat::DayMonthYear),
        date_separator: Some(TimestampDateSeparator::Slash),
        time_format: Some(TimestampTimeFormat::Hour12),
        precision: Some(TimestampPrecision::Milliseconds),
        order: Some(TimestampOrder::TimeDateResult),
        date_time_separator: Some(TimestampSeparator::Pipe),
        separator: Some(TimestampSeparator::Dash),
        show_exit_code: Some(true),
        duration_format: Some(TimestampDurationFormat::Seconds),
        ..Default::default()
    };
    assert_eq!(
        label(appearance).text,
        "12:34:56.123 PM | 30/09/2026 – ✓  exit 0  0.104s"
    );
    let mut midnight = stamp();
    midnight.hour = 0;
    let mut noon = midnight;
    noon.hour = 12;
    assert!(
        TimestampText::new(appearance, true, Some(midnight), None, None)
            .text
            .starts_with("12:34:56.123 AM")
    );
    assert!(TimestampText::new(appearance, true, Some(noon), None, None)
        .text
        .starts_with("12:34:56.123 PM"));
}

#[test]
fn timestamp_calendar_conversion_is_pure_and_handles_leap_day_and_year_boundary() {
    let appearance = TimestampAppearance {
        timezone: Some(TimestampZone::Utc),
        weekday: Some(true),
        zone_label: Some(true),
        ..Default::default()
    };
    let leap = SemanticCommandTimestamp {
        unix_ms: 1_709_164_800_000,
        year: 2024,
        month: 2,
        day: 29,
        hour: 2,
        minute: 0,
        second: 0,
    };
    assert_eq!(
        date_time(appearance, leap).unwrap(),
        ("Thursday 2024-02-29".into(), "00:00:00 UTC".into())
    );
    let local = TimestampAppearance {
        timezone: Some(TimestampZone::Recorded),
        ..appearance
    };
    assert_eq!(date_time(local, leap).unwrap().1, "02:00:00 UTC+02:00");
    let year = SemanticCommandTimestamp {
        unix_ms: 1_735_689_599_000,
        year: 2025,
        month: 1,
        day: 1,
        hour: 1,
        minute: 59,
        second: 59,
    };
    assert_eq!(date_time(appearance, year).unwrap().0, "Tuesday 2024-12-31");
    assert_eq!(date_time(local, year).unwrap().0, "Wednesday 2025-01-01");
    assert!(
        date_time(appearance, SemanticCommandTimestamp { month: 13, ..leap }).is_none()
    );
    assert!(date_time(
        appearance,
        SemanticCommandTimestamp {
            unix_ms: u64::MAX,
            ..leap
        }
    )
    .is_none());
}

#[test]
fn timestamps_all_component_positions_and_orders_pack_without_overlap_or_lost_bytes() {
    let tags = (0..16)
        .map(|_| Label {
            text: "tag",
            leading: 2.0,
            padding: 2.0,
            align_end: false,
        })
        .collect::<Vec<_>>();
    for date in TimestampPosition::ALL {
        for clock in TimestampPosition::ALL {
            for result in TimestampPosition::ALL {
                for order in TimestampOrder::ALL {
                    let appearance = TimestampAppearance {
                        date_position: Some(*date),
                        time_position: Some(*clock),
                        result_position: Some(*result),
                        order: Some(*order),
                        ..Default::default()
                    };
                    let text = label(appearance);
                    for width in [8.0, 48.0, 240.0, 1200.0] {
                        let band = text
                            .pack(&tags, width, 4.0, &[8], Some(8), 0.0, |_, value| {
                                value.chars().count() as f32 * 7.0
                            })
                            .unwrap();
                        let restored: String = band
                            .fragments
                            .iter()
                            .filter(|f| f.item == tags.len())
                            .map(|f| &text.text[f.bytes.clone()])
                            .collect();
                        assert_eq!(restored, text.text);
                        assert!(band.rows <= 256);
                        for (i, a) in band.fragments.iter().enumerate() {
                            assert!(a.x >= 0.0 && a.x + a.width <= width + 0.001);
                            for b in &band.fragments[i + 1..] {
                                assert!(
                                    a.row != b.row
                                        || a.x + a.width <= b.x + 0.001
                                        || b.x + b.width <= a.x + 0.001
                                );
                            }
                        }
                        for (index, tag) in tags.iter().enumerate() {
                            assert_eq!(
                                band.fragments
                                    .iter()
                                    .filter(|f| f.item == index)
                                    .map(|f| &tag.text[f.bytes.clone()])
                                    .collect::<String>(),
                                "tag"
                            );
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn timestamp_colors_are_independent_of_output_table_and_kubernetes_styles() {
    let colors = Colors::default();
    let appearance = TimestampAppearance {
        date_color: Some(Rgb::from_bytes([255, 0, 128])),
        status_colors: Some(false),
        ..Default::default()
    };
    assert_eq!(
        color(appearance, colors, Some(1), CompletionPart::Date),
        [1.0, 0.0, 128.0 / 255.0, 1.0]
    );
    assert_eq!(
        color(appearance, colors, Some(1), CompletionPart::Time),
        colors.foreground
    );
    assert_eq!(
        color(Default::default(), colors, Some(1), CompletionPart::Result),
        colors.red
    );
}

#[test]
fn timestamp_placement_keeps_date_above_tags_and_time_below_on_the_chosen_side() {
    let tags = [Label {
        text: "tag",
        leading: 0.0,
        padding: 5.0,
        align_end: false,
    }];
    let text = label(TimestampAppearance {
        date_position: Some(TimestampPosition::AboveLeft),
        time_position: Some(TimestampPosition::BelowRight),
        result_position: Some(TimestampPosition::Left),
        ..Default::default()
    });
    let band = text
        .pack(&tags, 600.0, 4.0, &[], None, 3.0, |_, value| {
            value.chars().count() as f32 * 7.0
        })
        .unwrap();
    let tag = band.fragments.iter().find(|f| f.item == 0).unwrap();
    let part = |kind| {
        let range = &text
            .spans
            .iter()
            .find(|span| span.part == kind)
            .unwrap()
            .bytes;
        band.fragments
            .iter()
            .find(|f| f.item == 1 && f.bytes.start == range.start)
            .unwrap()
    };
    let date = part(CompletionPart::Date);
    let clock = part(CompletionPart::Time);
    let result = part(CompletionPart::Result);
    assert!(date.row < tag.row && tag.row < clock.row);
    assert_eq!(date.x, 0.0);
    assert_eq!(result.row, tag.row);
    assert_eq!(result.x, 0.0);
    assert!(result.x + result.width + 4.0 <= tag.x);
    assert!((clock.x + clock.width - 600.0).abs() < 0.001);
    assert!(tag.shape_position.first && tag.shape_position.last);
}

#[test]
fn timestamp_date_and_duration_presets_have_literal_oracles() {
    for (format, expected) in [
        (TimestampDateFormat::YearMonthDay, "2026-09-30"),
        (TimestampDateFormat::DayMonthYear, "30-09-2026"),
        (TimestampDateFormat::MonthDayYear, "09-30-2026"),
        (TimestampDateFormat::DayMonthName, "30 Sep 2026"),
        (TimestampDateFormat::MonthNameDay, "Sep 30, 2026"),
        (TimestampDateFormat::Hidden, ""),
    ] {
        assert_eq!(
            date_time(
                TimestampAppearance {
                    date_format: Some(format),
                    ..Default::default()
                },
                stamp()
            )
            .unwrap()
            .0,
            expected
        );
    }
    for (format, expected) in [
        (TimestampDurationFormat::Auto, "✓  1m 02s"),
        (TimestampDurationFormat::Milliseconds, "✓  62004ms"),
        (TimestampDurationFormat::Seconds, "✓  62.004s"),
        (TimestampDurationFormat::Clock, "✓  00:01:02.004"),
    ] {
        assert_eq!(
            result_text(
                TimestampAppearance {
                    duration_format: Some(format),
                    ..Default::default()
                },
                Some(0),
                Some(62004)
            ),
            expected
        );
    }
}
