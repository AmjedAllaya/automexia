//! UI-only projection of effective settings. Raw catalogues still own validation
//! and preferences; hiding a control never rewrites its saved value or provenance.
use super::*;
use std::collections::BTreeMap;

pub(crate) fn visible_controls(
    page: Catalog,
    full: &Catalog,
) -> Result<Catalog, SettingsError> {
    let controls = Visibility::new(&page, full);
    let entries = page
        .entries()
        .iter()
        .filter(|row| controls.allows(row.id.as_str()))
        .cloned()
        .collect();
    Catalog::new(page.revision(), entries)
}

struct Visibility<'a> {
    rows: BTreeMap<&'a str, &'a settings::SettingDescriptor>,
    selected_slot_enabled: bool,
}

impl<'a> Visibility<'a> {
    fn new(page: &'a Catalog, full: &'a Catalog) -> Self {
        Self {
            rows: full
                .entries()
                .iter()
                .chain(page.entries())
                .map(|row| (row.id.as_str(), row))
                .collect(),
            selected_slot_enabled: page
                .entries()
                .iter()
                .find(|row| {
                    row.id.as_str().starts_with("tags.slot.")
                        && row.id.as_str().ends_with(".enabled")
                })
                .is_none_or(|row| row.value == SettingValue::Boolean(true)),
        }
    }

    // Missing parents are not invented for standalone or extension-owned pages.
    // Known application pages include their effective parent in page or full.
    fn enabled(&self, id: &str) -> bool {
        self.rows.get(id).is_none_or(|row| {
            row.availability.reason().is_none()
                && row.value != SettingValue::Boolean(false)
        })
    }

    fn boolean(&self, id: &str, default: bool) -> bool {
        self.rows.get(id).map_or(default, |row| {
            matches!(row.value, SettingValue::Boolean(true))
                && row.availability.reason().is_none()
        })
    }

    fn choice(&self, id: &str, default: &'a str) -> &'a str {
        self.rows
            .get(id)
            .copied()
            .and_then(|row| match &row.value {
                SettingValue::Choice(value) => Some(value.as_str()),
                _ => None,
            })
            .unwrap_or(default)
    }

    fn allows(&self, id: &str) -> bool {
        if id.starts_with("timestamps.") {
            return self.timestamp(id);
        }
        if id.starts_with("tables.") {
            if !self.enabled(settings::INLINE_TABLES) {
                return false;
            }
            let borders = self.choice("tables.border-style", "solid") != "none";
            let lines = [
                "tables.outer-border",
                "tables.row-lines",
                "tables.column-lines",
                "tables.header-separator",
            ]
            .iter()
            .any(|id| self.boolean(id, true));
            return match id {
                "tables.border-weight"
                | "tables.backgrounds.border"
                | "tables.opacity.border" => borders && lines,
                "tables.outer-border"
                | "tables.row-lines"
                | "tables.column-lines"
                | "tables.header-separator" => borders,
                "tables.colors.alternate"
                | "tables.backgrounds.alternate"
                | "tables.opacity.alternate" => {
                    self.choice("tables.banding", "rows") != "none"
                }
                _ => true,
            };
        }
        for (domain, enabled, style) in [
            ("output.", settings::OUTPUT_HIGHLIGHTING, "output.style"),
            (
                "kubernetes.",
                settings::KUBERNETES_HIGHLIGHTING,
                "kubernetes.style",
            ),
        ] {
            if let Some(field) = id.strip_prefix(domain) {
                if !self.enabled(enabled) {
                    return false;
                }
                let style = self.choice(style, "both");
                return if field.starts_with("colors.") {
                    style != "background"
                } else if field.starts_with("backgrounds.")
                    || field.starts_with("opacity.")
                {
                    style != "foreground"
                } else {
                    true
                };
            }
        }
        if id.starts_with("command_output.") {
            return self.enabled(settings::COMMAND_OUTPUT_HIGHLIGHTING);
        }
        if id == "fonts.bold-weight" {
            return self.enabled("fonts.bold-enabled");
        }
        if id == settings_extensions::DEVOPS_CONTEXT_STATUS_ID {
            return self.enabled("tags.enabled");
        }
        if id.starts_with("tags.") && id != "tags.enabled" {
            if !self.enabled("tags.enabled") {
                return false;
            }
            if id == "tags.opacity" {
                return self.choice("tags.style", "tinted") != "plain";
            }
            if id.starts_with("tags.colors.") {
                if !self.selected_slot_enabled {
                    return false;
                }
                let availability = BarContextAvailability {
                    devops: self
                        .boolean(settings_extensions::DEVOPS_CONTEXT_STATUS_ID, false),
                    git: self.boolean(settings_extensions::DEVOPS_GIT_STATUS_ID, false),
                };
                if crate::automexia::presentation::TAG_COLOR_BINDINGS
                    .iter()
                    .any(|binding| {
                        binding.id == id && !availability.role_available(binding.role)
                    })
                {
                    return false;
                }
            }
            if let Some((slot, field)) = id
                .strip_prefix("tags.slot.")
                .and_then(|rest| rest.split_once('.'))
            {
                if matches!(field, "enabled" | "remove" | "page") {
                    return true;
                }
                if !self.enabled(&format!("tags.slot.{slot}.enabled")) {
                    return false;
                }
                return match field {
                    "literal" => {
                        self.choice(&format!("tags.slot.{slot}.text"), "literal")
                            == "literal"
                    }
                    "prefix" | "suffix" => {
                        self.choice(&format!("tags.slot.{slot}.text"), "") != "none"
                    }
                    "icon-context" => {
                        self.choice(&format!("tags.slot.{slot}.icon"), "context")
                            == "context"
                    }
                    "icon-fixed" => {
                        self.choice(&format!("tags.slot.{slot}.icon"), "fixed") == "fixed"
                    }
                    _ => true,
                };
            }
        }
        if id.starts_with("extension.") {
            if let Some((feature, _)) = id.split_once(".option.") {
                return self.enabled(&format!("{feature}.enabled"));
            }
        }
        true
    }

    fn timestamp(&self, id: &str) -> bool {
        let clock = self.enabled(settings::COMMAND_TIMESTAMPS);
        let date_format = self.choice("timestamps.date-format", "year-month-day");
        let date = clock && date_format != "hidden";
        let time = clock && self.choice("timestamps.time-format", "24-hour") != "hidden";
        let duration = self.boolean("timestamps.show-duration", true);
        let result = duration
            || self.boolean("timestamps.show-status", true)
            || self.boolean("timestamps.show-exit-code", false);
        let any = date || time || result;
        let date_position = self.choice("timestamps.date-position", "right");
        let time_position = self.choice("timestamps.time-position", "right");
        let result_position = self.choice("timestamps.result-position", "right");
        let shared = (date && time && date_position == time_position)
            || (result && date && result_position == date_position)
            || (result && time && result_position == time_position);
        let result_between = result
            && result_position == date_position
            && matches!(
                self.choice("timestamps.order", "result-date-time"),
                "date-result-time" | "time-result-date"
            );
        match id {
            "timestamps.date-format" | "timestamps.time-format" => clock,
            "timestamps.date-separator" => {
                date && !matches!(date_format, "day-month-name" | "month-name-day")
            }
            "timestamps.date-position"
            | "timestamps.colors.date"
            | "timestamps.weekday" => date,
            "timestamps.precision"
            | "timestamps.time-position"
            | "timestamps.colors.time" => time,
            "timestamps.timezone" | "timestamps.zone-label" => date || time,
            "timestamps.duration-format" => duration,
            "timestamps.result-position" | "timestamps.colors.result" => result,
            "timestamps.order" => shared,
            "timestamps.date-time-separator" => {
                date && time && date_position == time_position && !result_between
            }
            "timestamps.separator" => {
                result
                    && ((date && date_position == result_position)
                        || (time && time_position == result_position))
            }
            "timestamps.status-colors" => [
                (date, "timestamps.colors.date"),
                (time, "timestamps.colors.time"),
                (result, "timestamps.colors.result"),
            ]
            .into_iter()
            .any(|(shown, color)| {
                shown
                    && self
                        .rows
                        .get(color)
                        .is_none_or(|row| row.origin == ValueOrigin::Default)
            }),
            "timestamps.size"
            | "timestamps.bold"
            | "timestamps.backgrounds.label"
            | "timestamps.opacity.label" => any,
            _ => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rio_backend::config::presentation::*;

    fn present(page: &Catalog, id: &str) -> bool {
        page.entries().iter().any(|row| row.id.as_str() == id)
    }

    #[test]
    fn dependent_controls_bold_face_retains_its_weight_while_disabled() {
        let base = Config::default();
        let mut prefs = UserPreferences::default();
        prefs.fonts.bold_weight = Some(800);
        for enabled in [false, true, false, true] {
            prefs.fonts.bold_enabled = Some(enabled);
            let full = catalog(1, &base, &prefs, &[]).unwrap();
            let snapshot = slot_page_snapshot_with_config(
                &prefs,
                &prefs.apply_to(&base),
                &base,
                &crate::settings_catalog::test_installed_extensions(),
            );
            let page =
                visible_controls(font_page_catalog(&full, &snapshot).unwrap(), &full)
                    .unwrap();
            assert!(present(&page, "fonts.bold-enabled"));
            assert_eq!(present(&page, "fonts.bold-weight"), enabled);
            if enabled {
                let row = page
                    .entries()
                    .iter()
                    .find(|row| row.id.as_str() == "fonts.bold-weight")
                    .unwrap();
                assert_eq!(row.value, SettingValue::Number(800.0));
                assert_eq!(row.origin, ValueOrigin::User);
            }
            assert_eq!(prefs.fonts.bold_weight, Some(800));
        }
    }

    #[test]
    fn dependent_controls_unavailable_parent_retains_reason_and_hides_children() {
        let full = catalog(1, &Config::default(), &Default::default(), &[]).unwrap();
        let mut rows = full.entries().to_vec();
        let parent = rows
            .iter_mut()
            .find(|row| row.id.as_str() == settings::OUTPUT_HIGHLIGHTING)
            .unwrap();
        parent.availability = settings::Availability::Unavailable {
            reason: "Unavailable in this fixture".into(),
        };
        let full = Catalog::new(1, rows).unwrap();
        let before = full.clone();
        let page = visible_controls(full.clone(), &full).unwrap();
        assert!(page
            .entries()
            .iter()
            .find(|row| row.id.as_str() == settings::OUTPUT_HIGHLIGHTING)
            .unwrap()
            .availability
            .reason()
            .is_some());
        assert!(!present(&page, "output.style"));
        assert!(!present(&page, "output.colors.error"));
        assert_eq!(full, before);
        assert_eq!(visible_controls(page.clone(), &full).unwrap(), page);
    }

    #[test]
    fn dependent_controls_disabled_package_options_keep_their_saved_values() {
        let full = catalog(1, &Config::default(), &Default::default(), &[]).unwrap();
        let mut parent = full
            .entries()
            .iter()
            .find(|row| row.id.as_str() == settings::OUTPUT_HIGHLIGHTING)
            .unwrap()
            .clone();
        parent.id =
            settings::SettingId::new("extension.example.feature.enabled").unwrap();
        parent.owner = settings::SettingOwner::Extension("example".into());
        parent.value = SettingValue::Boolean(false);
        let mut child = parent.clone();
        child.id =
            settings::SettingId::new("extension.example.feature.option.badge").unwrap();
        child.value = SettingValue::Boolean(true);
        child.origin = ValueOrigin::User;
        let original = child.clone();
        let off = Catalog::new(2, vec![parent.clone(), child.clone()]).unwrap();
        let page = visible_controls(off.clone(), &full).unwrap();
        assert_eq!(page.entries().len(), 1);
        assert_eq!(off.entries()[1], original);
        parent.value = SettingValue::Boolean(true);
        let on = Catalog::new(3, vec![parent, child]).unwrap();
        let page = visible_controls(on, &full).unwrap();
        assert_eq!(page.entries().len(), 2);
        assert_eq!(page.entries()[1], original);
    }

    #[test]
    fn dependent_controls_table_matrix_keeps_reactivation_controls() {
        let base = Config::default();
        for enabled in [false, true] {
            for border in TableBorderStyle::ALL {
                for banding in TableBanding::ALL {
                    let mut prefs = UserPreferences::default();
                    prefs.presentation.inline_tables = Some(enabled);
                    prefs.visual.tables.border_style = Some(*border);
                    prefs.visual.tables.banding = Some(*banding);
                    let full = catalog(1, &base, &prefs, &[]).unwrap();
                    let snapshot = slot_page_snapshot_with_config(
                        &prefs,
                        &prefs.apply_to(&base),
                        &base,
                        &crate::settings_catalog::test_installed_extensions(),
                    );
                    let raw = table_page_catalog(&full, &snapshot).unwrap();
                    let page = visible_controls(raw, &full).unwrap();
                    assert!(present(&page, settings::INLINE_TABLES));
                    assert_eq!(present(&page, "tables.border-style"), enabled);
                    for id in [
                        "tables.border-weight",
                        "tables.outer-border",
                        "tables.row-lines",
                        "tables.column-lines",
                        "tables.header-separator",
                        "tables.backgrounds.border",
                        "tables.opacity.border",
                    ] {
                        assert_eq!(
                            present(&page, id),
                            enabled && *border != TableBorderStyle::None,
                            "{id}"
                        );
                    }
                    for id in [
                        "tables.colors.alternate",
                        "tables.backgrounds.alternate",
                        "tables.opacity.alternate",
                    ] {
                        assert_eq!(
                            present(&page, id),
                            enabled && *banding != TableBanding::None,
                            "{id}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn dependent_controls_clock_results_and_numeric_dates_are_independent() {
        let base = Config::default();
        for clock in [false, true] {
            for date in TimestampDateFormat::ALL {
                for time in TimestampTimeFormat::ALL {
                    for duration in [false, true] {
                        let mut prefs = UserPreferences::default();
                        prefs.presentation.command_timestamps = Some(clock);
                        prefs.visual.timestamps.date_format = Some(*date);
                        prefs.visual.timestamps.time_format = Some(*time);
                        prefs.visual.timestamps.show_duration = Some(duration);
                        prefs.visual.timestamps.show_status = Some(false);
                        prefs.visual.timestamps.show_exit_code = Some(false);
                        let full = catalog(1, &base, &prefs, &[]).unwrap();
                        let snapshot = slot_page_snapshot_with_config(
                            &prefs,
                            &prefs.apply_to(&base),
                            &base,
                            &crate::settings_catalog::test_installed_extensions(),
                        );
                        let raw = timestamp_page_catalog(&full, &snapshot).unwrap();
                        let page = visible_controls(raw, &full).unwrap();
                        let date_shown = clock && *date != TimestampDateFormat::Hidden;
                        let time_shown = clock && *time != TimestampTimeFormat::Hidden;
                        assert_eq!(present(&page, "timestamps.date-format"), clock);
                        assert_eq!(present(&page, "timestamps.time-format"), clock);
                        for id in [
                            "timestamps.date-position",
                            "timestamps.weekday",
                            "timestamps.colors.date",
                        ] {
                            assert_eq!(present(&page, id), date_shown, "{id}");
                        }
                        assert_eq!(
                            present(&page, "timestamps.date-separator"),
                            clock
                                && matches!(
                                    date,
                                    TimestampDateFormat::YearMonthDay
                                        | TimestampDateFormat::DayMonthYear
                                        | TimestampDateFormat::MonthDayYear
                                )
                        );
                        for id in [
                            "timestamps.precision",
                            "timestamps.time-position",
                            "timestamps.colors.time",
                        ] {
                            assert_eq!(present(&page, id), time_shown, "{id}");
                        }
                        for id in ["timestamps.timezone", "timestamps.zone-label"] {
                            assert_eq!(
                                present(&page, id),
                                date_shown || time_shown,
                                "{id}"
                            );
                        }
                        for id in [
                            "timestamps.duration-format",
                            "timestamps.result-position",
                            "timestamps.colors.result",
                        ] {
                            assert_eq!(present(&page, id), duration, "{id}");
                        }
                        for id in [
                            "timestamps.show-status",
                            "timestamps.show-duration",
                            "timestamps.show-exit-code",
                        ] {
                            assert!(present(&page, id));
                        }
                        assert_eq!(
                            present(&page, "timestamps.size"),
                            date_shown || time_shown || duration
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn dependent_controls_output_styles_match_effective_paint_domains() {
        let mut base = Config::default();
        for enabled in [false, true] {
            for style in [
                HighlightStyle::Foreground,
                HighlightStyle::Background,
                HighlightStyle::Both,
            ] {
                base.presentation.output_highlighting = enabled;
                base.presentation.kubernetes_highlighting = enabled;
                base.presentation.highlight.style = style;
                base.presentation.kubernetes.style = style;
                let full = catalog(1, &base, &Default::default(), &[]).unwrap();
                let page = visible_controls(full.clone(), &full).unwrap();
                for domain in ["output", "kubernetes"] {
                    assert_eq!(present(&page, &format!("{domain}.style")), enabled);
                    assert_eq!(
                        present(&page, &format!("{domain}.colors.error")),
                        enabled && style != HighlightStyle::Background
                    );
                    for field in ["backgrounds", "opacity"] {
                        assert_eq!(
                            present(&page, &format!("{domain}.{field}.error")),
                            enabled && style != HighlightStyle::Foreground
                        );
                    }
                }
                assert!(present(&page, settings::OUTPUT_HIGHLIGHTING));
                assert!(present(&page, settings::KUBERNETES_HIGHLIGHTING));
            }
        }
    }

    #[test]
    fn dependent_controls_slot_sources_and_disabled_features_hide_children() {
        let base = Config::default();
        let mut prefs = UserPreferences::default();
        for (id, value) in [
            (
                "tags.slot.windows.text",
                SettingValue::Choice("none".into()),
            ),
            (
                "tags.slot.windows.icon",
                SettingValue::Choice("fixed".into()),
            ),
        ] {
            prefs = apply_edit(
                1,
                &base,
                &prefs,
                &[],
                &Edit {
                    revision: 1,
                    id: settings::SettingId::new(id).unwrap(),
                    change: Change::Set(value),
                },
            )
            .unwrap();
        }
        let full = catalog(1, &base, &prefs, &[]).unwrap();
        let snapshot = slot_page_snapshot_with_config(
            &prefs,
            &prefs.apply_to(&base),
            &base,
            &crate::settings_catalog::test_installed_extensions(),
        );
        let raw = selected_tag_catalog(&full, &snapshot, "windows").unwrap();
        let page = visible_controls(raw, &full).unwrap();
        for suffix in ["literal", "prefix", "suffix", "icon-context"] {
            assert!(!present(&page, &format!("tags.slot.windows.{suffix}")));
        }
        for suffix in ["enabled", "text", "icon", "icon-fixed"] {
            assert!(present(&page, &format!("tags.slot.windows.{suffix}")));
        }
        prefs = apply_edit(
            1,
            &base,
            &prefs,
            &[],
            &Edit {
                revision: 1,
                id: settings::SettingId::new("tags.slot.windows.enabled").unwrap(),
                change: Change::Set(SettingValue::Boolean(false)),
            },
        )
        .unwrap();
        let full = catalog(2, &base, &prefs, &[]).unwrap();
        let snapshot = slot_page_snapshot_with_config(
            &prefs,
            &prefs.apply_to(&base),
            &base,
            &crate::settings_catalog::test_installed_extensions(),
        );
        let raw = selected_tag_catalog(&full, &snapshot, "windows").unwrap();
        let page = visible_controls(raw, &full).unwrap();
        assert!(present(&page, "tags.slot.windows.enabled"));
        assert!(!present(&page, "tags.slot.windows.icon-fixed"));
        assert!(!present(&page, "tags.slot.windows.color"));
    }

    #[test]
    fn dependent_controls_separators_need_adjacent_components_in_same_position() {
        let base = Config::default();
        for (date, time, result, order, date_time, result_separator, shared) in [
            (
                TimestampPosition::Left,
                TimestampPosition::Right,
                TimestampPosition::AboveLeft,
                TimestampOrder::ResultDateTime,
                false,
                false,
                false,
            ),
            (
                TimestampPosition::Left,
                TimestampPosition::Left,
                TimestampPosition::AboveLeft,
                TimestampOrder::DateResultTime,
                true,
                false,
                true,
            ),
            (
                TimestampPosition::Left,
                TimestampPosition::Left,
                TimestampPosition::Left,
                TimestampOrder::DateResultTime,
                false,
                true,
                true,
            ),
            (
                TimestampPosition::Left,
                TimestampPosition::Left,
                TimestampPosition::Left,
                TimestampOrder::ResultDateTime,
                true,
                true,
                true,
            ),
        ] {
            let mut prefs = UserPreferences::default();
            prefs.visual.timestamps.date_position = Some(date);
            prefs.visual.timestamps.time_position = Some(time);
            prefs.visual.timestamps.result_position = Some(result);
            prefs.visual.timestamps.order = Some(order);
            let full = catalog(1, &base, &prefs, &[]).unwrap();
            let snapshot = slot_page_snapshot_with_config(
                &prefs,
                &prefs.apply_to(&base),
                &base,
                &crate::settings_catalog::test_installed_extensions(),
            );
            let raw = timestamp_page_catalog(&full, &snapshot).unwrap();
            let page = visible_controls(raw, &full).unwrap();
            assert_eq!(present(&page, "timestamps.date-time-separator"), date_time);
            assert_eq!(present(&page, "timestamps.separator"), result_separator);
            assert_eq!(present(&page, "timestamps.order"), shared);
        }
    }

    #[test]
    #[ignore = "same-host settings projection microbenchmark; run explicitly with --ignored"]
    fn dependent_controls_projection_benchmark() {
        let base = Config::default();
        let full = catalog(1, &base, &Default::default(), &[]).unwrap();
        let mut samples = Vec::with_capacity(1000);
        for iteration in 0..1050 {
            let start = std::time::Instant::now();
            let projected =
                visible_controls(std::hint::black_box(full.clone()), &full).unwrap();
            std::hint::black_box(&projected);
            if iteration >= 50 {
                samples.push(start.elapsed().as_nanos());
            }
        }
        samples.sort_unstable();
        println!(
            "{}",
            serde_json::json!({
                "benchmark": "dependent_controls_projection",
                "rows": full.entries().len(), "samples": samples.len(),
                "p50_ns": samples[499], "p95_ns": samples[949],
                "excludes": ["rendering", "presentation"]
            })
        );
    }
}
