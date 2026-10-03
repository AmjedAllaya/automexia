//! Timestamp choices use the standard settings editors, validation and recovery.
use super::*;
use rio_backend::config::presentation::*;

pub(super) fn descriptors(
    base: &TimestampAppearance,
    current: &TimestampAppearance,
    user: &TimestampAppearance,
    palette: &Colors,
) -> Result<Vec<settings::SettingDescriptor>, SettingsError> {
    let mut rows = Vec::new();
    macro_rules! choice {
        ($field:ident, $id:literal, $label:literal, $ty:ty, $help:literal) => {{
            let values = <$ty>::ALL
                .iter()
                .map(|value| (value.id(), value.label()))
                .collect::<Vec<_>>();
            let default = if $id == "timestamps.date-time-separator" {
                "space"
            } else {
                <$ty>::default().id()
            };
            let mut row = visual_row(
                $id,
                $label.into(),
                choice_kind(&values),
                SettingValue::Choice(current.$field.map_or(default, <$ty>::id).into()),
                SettingValue::Choice(base.$field.map_or(default, <$ty>::id).into()),
                visual_origin(
                    user.$field.is_some(),
                    base.$field.is_some(),
                    ValueOrigin::Default,
                ),
            )?;
            row.description = $help.into();
            rows.push(row);
        }};
    }
    choice!(
        date_format,
        "timestamps.date-format",
        "Date format",
        TimestampDateFormat,
        "Choose a date order, English month name, or hide the date."
    );
    choice!(
        date_separator,
        "timestamps.date-separator",
        "Date separator",
        TimestampDateSeparator,
        "Separator for numeric dates."
    );
    choice!(
        time_format,
        "timestamps.time-format",
        "Time format",
        TimestampTimeFormat,
        "24-hour, AM/PM, or hide the time."
    );
    choice!(
        precision,
        "timestamps.precision",
        "Time precision",
        TimestampPrecision,
        "Choose minutes, seconds or milliseconds."
    );
    choice!(
        timezone,
        "timestamps.timezone",
        "Time zone",
        TimestampZone,
        "Local time captured at completion, or UTC; saved command times stay unchanged."
    );
    choice!(
        date_position,
        "timestamps.date-position",
        "Date position",
        TimestampPosition,
        "Choose the date's row and side independently."
    );
    choice!(
        time_position,
        "timestamps.time-position",
        "Time position",
        TimestampPosition,
        "Choose the time's row and side independently."
    );
    choice!(
        result_position,
        "timestamps.result-position",
        "Result position",
        TimestampPosition,
        "Place status and duration independently of the clock."
    );
    choice!(
        order,
        "timestamps.order",
        "Order on the same side",
        TimestampOrder,
        "Order components that share a position; narrow layouts wrap."
    );
    choice!(
        separator,
        "timestamps.separator",
        "Result separator",
        TimestampSeparator,
        "Between the result and date or time in the same position."
    );
    choice!(
        date_time_separator,
        "timestamps.date-time-separator",
        "Date / time separator",
        TimestampSeparator,
        "Between adjacent date and time in the same position."
    );
    choice!(
        duration_format,
        "timestamps.duration-format",
        "Duration format",
        TimestampDurationFormat,
        "Automatic units, milliseconds, seconds, or a clock."
    );
    choice!(
        size,
        "timestamps.size",
        "Text size",
        TimestampSize,
        "Relative to information tags; narrow rows still wrap."
    );
    macro_rules! boolean {
        ($field:ident, $id:literal, $label:literal, $default:literal, $help:literal) => {{
            let mut row = visual_row(
                $id,
                $label.into(),
                settings::SettingKind::Boolean,
                SettingValue::Boolean(current.$field.unwrap_or($default)),
                SettingValue::Boolean(base.$field.unwrap_or($default)),
                visual_origin(
                    user.$field.is_some(),
                    base.$field.is_some(),
                    ValueOrigin::Default,
                ),
            )?;
            row.description = $help.into();
            rows.push(row);
        }};
    }
    boolean!(
        weekday,
        "timestamps.weekday",
        "Day of the week",
        false,
        "Add the English weekday to visible dates."
    );
    boolean!(
        zone_label,
        "timestamps.zone-label",
        "Time zone label",
        false,
        "Show UTC or the recorded local offset beside the time."
    );
    boolean!(
        show_status,
        "timestamps.show-status",
        "Status symbol",
        true,
        "Success, failure or unknown; independent of date/time visibility."
    );
    boolean!(
        show_exit_code,
        "timestamps.show-exit-code",
        "Exit code",
        false,
        "Show the reported exit code; ? means unavailable."
    );
    boolean!(
        show_duration,
        "timestamps.show-duration",
        "Duration",
        true,
        "Show elapsed time; done means duration is unavailable."
    );
    boolean!(
        bold,
        "timestamps.bold",
        "Bold text",
        false,
        "Emphasize the date, time and result."
    );
    boolean!(
        status_colors,
        "timestamps.status-colors",
        "Use status colors",
        true,
        "Use success/failure colors unless a text color below overrides them."
    );
    let defaults = [palette.green, palette.green, palette.green, [0.0; 4]];
    for (index, (id, label)) in COLORS.iter().enumerate() {
        let fallback = defaults[index].map(|c| (c.clamp(0.0, 1.0) * 255.0).round() as u8);
        let resolved = |value: &TimestampAppearance| {
            let fallback = if index < 3 && !value.status_colors.unwrap_or(true) {
                palette
                    .foreground
                    .map(|c| (c.clamp(0.0, 1.0) * 255.0).round() as u8)
            } else {
                fallback
            };
            SettingValue::Color(raw_color(value, index).unwrap_or(fallback))
        };
        let mut row = visual_row(
            id,
            (*label).into(),
            settings::SettingKind::Color { alpha: index == 3 },
            resolved(current),
            resolved(base),
            visual_origin(
                raw_color(user, index).is_some(),
                raw_color(base, index).is_some(),
                ValueOrigin::Default,
            ),
        )?;
        row.description = if index == 3 {
            "Completion label background only; opacity 0 makes it transparent."
        } else {
            "Override this text color; reset follows configuration and status colors."
        }
        .into();
        rows.push(row);
    }
    for row in &mut rows {
        row.keywords = vec![
            "timestamp date time clock completion format".into(),
            "position color opacity".into(),
        ];
    }
    Ok(rows)
}

const COLORS: [(&str, &str); 4] = [
    ("timestamps.colors.date", "Date color"),
    ("timestamps.colors.time", "Time color"),
    ("timestamps.colors.result", "Result color"),
    ("timestamps.backgrounds.label", "Label background"),
];
fn raw_color(value: &TimestampAppearance, index: usize) -> Option<[u8; 4]> {
    match index {
        0 => value.date_color.map(Rgb::rgba_bytes),
        1 => value.time_color.map(Rgb::rgba_bytes),
        2 => value.result_color.map(Rgb::rgba_bytes),
        3 => value.background.map(Rgba::bytes),
        _ => None,
    }
}

pub(super) fn apply(
    candidate: &mut UserPreferences,
    edit: &Edit,
) -> Result<bool, SettingsError> {
    let appearance = &mut candidate.visual.timestamps;
    macro_rules! choice {
        ($field:ident, $ty:ty) => {{
            appearance.$field = match &edit.change {
                Change::Reset => None,
                Change::Set(SettingValue::Choice(value)) => {
                    Some(<$ty>::from_id(value).ok_or(SettingsError::InvalidValue)?)
                }
                _ => return Err(SettingsError::InvalidValue),
            };
            return Ok(true);
        }};
    }
    macro_rules! boolean {
        ($field:ident) => {{
            appearance.$field = match &edit.change {
                Change::Reset => None,
                Change::Set(SettingValue::Boolean(value)) => Some(*value),
                _ => return Err(SettingsError::InvalidValue),
            };
            return Ok(true);
        }};
    }
    match edit.id.as_str() {
        "timestamps.date-format" => choice!(date_format, TimestampDateFormat),
        "timestamps.date-separator" => choice!(date_separator, TimestampDateSeparator),
        "timestamps.time-format" => choice!(time_format, TimestampTimeFormat),
        "timestamps.precision" => choice!(precision, TimestampPrecision),
        "timestamps.timezone" => choice!(timezone, TimestampZone),
        "timestamps.date-position" => choice!(date_position, TimestampPosition),
        "timestamps.time-position" => choice!(time_position, TimestampPosition),
        "timestamps.result-position" => choice!(result_position, TimestampPosition),
        "timestamps.order" => choice!(order, TimestampOrder),
        "timestamps.separator" => choice!(separator, TimestampSeparator),
        "timestamps.date-time-separator" => {
            choice!(date_time_separator, TimestampSeparator)
        }
        "timestamps.duration-format" => choice!(duration_format, TimestampDurationFormat),
        "timestamps.size" => choice!(size, TimestampSize),
        "timestamps.weekday" => boolean!(weekday),
        "timestamps.zone-label" => boolean!(zone_label),
        "timestamps.show-status" => boolean!(show_status),
        "timestamps.show-duration" => boolean!(show_duration),
        "timestamps.show-exit-code" => boolean!(show_exit_code),
        "timestamps.bold" => boolean!(bold),
        "timestamps.status-colors" => boolean!(status_colors),
        _ => {}
    }
    let Some(index) = COLORS.iter().position(|(id, _)| *id == edit.id.as_str()) else {
        return Ok(false);
    };
    let value = match &edit.change {
        Change::Reset => None,
        Change::Set(SettingValue::Color(value)) => Some(*value),
        _ => return Err(SettingsError::InvalidValue),
    };
    let rgb = value.map(|v| Rgb::from_bytes([v[0], v[1], v[2]]));
    match index {
        0 => appearance.date_color = rgb,
        1 => appearance.time_color = rgb,
        2 => appearance.result_color = rgb,
        3 => appearance.background = value.map(Rgba::from_bytes),
        _ => return Err(SettingsError::UnknownSetting),
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timestamp_controls_are_reachable_editable_resettable_and_persisted() {
        let base = Config::default();
        let original = UserPreferences::default();
        let full = catalog(1, &base, &original, &[]).unwrap();
        assert!(!full
            .entries()
            .iter()
            .any(|row| row.id.as_str().starts_with("timestamps.")));
        let page = timestamp_page_catalog(
            &full,
            &slot_page_snapshot_with_config(&original, &base, &base),
        )
        .unwrap();
        assert_eq!(page.entries().len(), 26);
        let mut prefs = original.clone();
        for row in page
            .entries()
            .iter()
            .filter(|row| row.id.as_str().starts_with("timestamps."))
        {
            let value = match (&row.kind, &row.value) {
                (
                    settings::SettingKind::Choice { options },
                    SettingValue::Choice(current),
                ) => SettingValue::Choice(
                    options
                        .iter()
                        .find(|option| option.value != *current)
                        .unwrap()
                        .value
                        .clone(),
                ),
                (settings::SettingKind::Boolean, SettingValue::Boolean(current)) => {
                    SettingValue::Boolean(!current)
                }
                (settings::SettingKind::Color { .. }, _) => {
                    SettingValue::Color([19, 29, 39, 255])
                }
                (settings::SettingKind::Number { .. }, _) => SettingValue::Number(25.0),
                _ => panic!("unhandled timestamp control"),
            };
            prefs = apply_edit(
                1,
                &base,
                &prefs,
                &[],
                &Edit {
                    revision: 1,
                    id: row.id.clone(),
                    change: Change::Set(value.clone()),
                },
            )
            .unwrap();
            let changed =
                timestamp_settings_catalog(1, &base, &prefs, &base.colors).unwrap();
            assert_eq!(changed.get(&row.id).unwrap().value, value);
        }
        assert_eq!(
            prefs.visual.timestamps.background.unwrap().bytes(),
            [19, 29, 39, 64]
        );
        assert_eq!(prefs.visual.tables, original.visual.tables);
        assert_eq!(prefs.visual.kubernetes, original.visual.kubernetes);
        assert_eq!(prefs.presentation, original.presentation);
        let root = tempfile::tempdir().unwrap();
        crate::automexia::preferences::write_to_root(root.path(), &prefs).unwrap();
        assert_eq!(
            crate::automexia::preferences::load_from_root(root.path()).preferences,
            prefs
        );
        let mut expected = original;
        expected.presentation.command_timestamps = Some(true);
        assert_eq!(
            reset_customizations(
                &prefs,
                &CustomizationResetScope::Group(
                    settings::SettingId::new(settings::COMMAND_TIMESTAMPS).unwrap()
                ),
                None
            )
            .unwrap(),
            expected
        );
    }

    #[test]
    fn timestamp_opacity_preserves_configured_rgb_and_invalid_edits_are_rejected() {
        let mut base = Config::default();
        base.presentation.timestamps.background =
            Some(Rgba::from_bytes([11, 22, 33, 255]));
        let original = UserPreferences::default();
        for percent in [0.0, 50.0, 100.0] {
            let id = settings::SettingId::new("timestamps.opacity.label").unwrap();
            let edited = apply_edit(
                1,
                &base,
                &original,
                &[],
                &Edit {
                    revision: 1,
                    id: id.clone(),
                    change: Change::Set(SettingValue::Number(percent)),
                },
            )
            .unwrap();
            assert_eq!(
                edited.visual.timestamps.background.unwrap().bytes(),
                [11, 22, 33, (percent * 255.0 / 100.0).round() as u8]
            );
            assert_eq!(
                apply_edit(
                    1,
                    &base,
                    &edited,
                    &[],
                    &Edit {
                        revision: 1,
                        id,
                        change: Change::Reset
                    }
                )
                .unwrap(),
                original
            );
        }
        for (id, value) in [
            (
                "timestamps.date-format",
                SettingValue::Choice("unknown".into()),
            ),
            ("timestamps.date-position", SettingValue::Boolean(true)),
            ("timestamps.opacity.label", SettingValue::Number(101.0)),
            ("timestamps.opacity.label", SettingValue::Number(f64::NAN)),
        ] {
            assert!(apply_edit(
                1,
                &base,
                &original,
                &[],
                &Edit {
                    revision: 1,
                    id: settings::SettingId::new(id).unwrap(),
                    change: Change::Set(value)
                }
            )
            .is_err());
        }
    }
}
