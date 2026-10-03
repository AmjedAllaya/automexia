//! Table-specific descriptors and typed edits; generic settings owns interaction.
use super::*;
use rio_backend::config::presentation::{
    TableAppearance, TableBanding, TableBorderStyle, TableBorderWeight,
};

const COLORS: [(&str, &str, bool); 7] = [
    ("tables.colors.header", "Header text", false),
    ("tables.colors.body", "Cell text", false),
    ("tables.colors.alternate", "Alternate cell text", false),
    ("tables.backgrounds.border", "Border color", true),
    ("tables.backgrounds.header", "Header background", true),
    ("tables.backgrounds.body", "Cell background", true),
    ("tables.backgrounds.alternate", "Alternate background", true),
];

fn raw_color(table: &TableAppearance, index: usize) -> Option<[u8; 4]> {
    match index {
        0 => table.header_foreground.map(Rgb::rgba_bytes),
        1 => table.body_foreground.map(Rgb::rgba_bytes),
        2 => table.alternate_foreground.map(Rgb::rgba_bytes),
        3 => table.border_color.map(Rgba::bytes),
        4 => table.header_background.map(Rgba::bytes),
        5 => table.body_background.map(Rgba::bytes),
        6 => table.alternate_background.map(Rgba::bytes),
        _ => None,
    }
}

pub(super) fn descriptors(
    base: &TableAppearance,
    current: &TableAppearance,
    user: &TableAppearance,
    palette: &Colors,
) -> Result<Vec<settings::SettingDescriptor>, SettingsError> {
    let mut rows = Vec::new();
    macro_rules! choice {
        ($field:ident, $id:literal, $label:literal, $ty:ty, $help:literal) => {{
            let values = <$ty>::ALL
                .iter()
                .map(|value| (value.id(), value.label()))
                .collect::<Vec<_>>();
            let mut row = visual_row(
                $id,
                $label.into(),
                choice_kind(&values),
                SettingValue::Choice(current.$field.unwrap_or_default().id().into()),
                SettingValue::Choice(base.$field.unwrap_or_default().id().into()),
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
        banding,
        "tables.banding",
        "Alternating backgrounds",
        TableBanding,
        "Shade rows, columns, or a checkerboard; wrapped cells stay together."
    );
    choice!(
        border_style,
        "tables.border-style",
        "Border style",
        TableBorderStyle,
        "Solid, dashed, dotted, double, or no borders."
    );
    choice!(
        border_weight,
        "tables.border-weight",
        "Border weight",
        TableBorderWeight,
        "Thickness of visible borders."
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
        outer_border,
        "tables.outer-border",
        "Outer border",
        true,
        "Frame the table edges."
    );
    boolean!(
        row_lines,
        "tables.row-lines",
        "Row separators",
        true,
        "Lines between data rows."
    );
    boolean!(
        column_lines,
        "tables.column-lines",
        "Column separators",
        true,
        "Lines between columns."
    );
    boolean!(
        header_separator,
        "tables.header-separator",
        "Header separator",
        true,
        "Line below the header."
    );
    boolean!(
        header_bold,
        "tables.header-bold",
        "Bold headers",
        false,
        "Emphasize header text."
    );
    let defaults = crate::renderer::table_style::TableStyle::color_defaults(*palette);
    for (index, (id, label, alpha)) in COLORS.iter().enumerate() {
        let fallback = defaults[index]
            .map(|channel| (channel.clamp(0.0, 1.0) * 255.0).round() as u8);
        let value = |table: &TableAppearance| {
            let mut color = raw_color(table, index)
                .or_else(|| {
                    (index == 2)
                        .then(|| table.body_foreground.map(Rgb::rgba_bytes))
                        .flatten()
                })
                .unwrap_or(fallback);
            if !alpha {
                color[3] = 255;
            }
            SettingValue::Color(color)
        };
        let mut row = visual_row(
            id,
            (*label).into(),
            settings::SettingKind::Color { alpha: *alpha },
            value(current),
            value(base),
            visual_origin(
                raw_color(user, index).is_some(),
                raw_color(base, index).is_some(),
                ValueOrigin::Default,
            ),
        )?;
        row.description = if index == 2 || index == 6 {
            "Used by the alternating background pattern."
        } else if index == 1 || index == 5 {
            "Status, selection and explicit terminal colors keep priority."
        } else {
            "Choose a color; reset follows the configuration and theme."
        }
        .into();
        row.keywords =
            vec!["table header row column stripe zebra color opacity border".into()];
        rows.push(row);
    }
    Ok(rows)
}

pub(super) fn apply(
    candidate: &mut UserPreferences,
    edit: &Edit,
) -> Result<bool, SettingsError> {
    let table = &mut candidate.visual.tables;
    macro_rules! choice {
        ($field:ident, $ty:ty) => {{
            table.$field = match &edit.change {
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
            table.$field = match &edit.change {
                Change::Reset => None,
                Change::Set(SettingValue::Boolean(value)) => Some(*value),
                _ => return Err(SettingsError::InvalidValue),
            };
            return Ok(true);
        }};
    }
    match edit.id.as_str() {
        "tables.border-style" => choice!(border_style, TableBorderStyle),
        "tables.border-weight" => choice!(border_weight, TableBorderWeight),
        "tables.banding" => choice!(banding, TableBanding),
        "tables.row-lines" => boolean!(row_lines),
        "tables.column-lines" => boolean!(column_lines),
        "tables.outer-border" => boolean!(outer_border),
        "tables.header-separator" => boolean!(header_separator),
        "tables.header-bold" => boolean!(header_bold),
        _ => {}
    }
    let Some(index) = COLORS.iter().position(|(id, _, _)| *id == edit.id.as_str()) else {
        return Ok(false);
    };
    let value = match &edit.change {
        Change::Reset => None,
        Change::Set(SettingValue::Color(value)) => Some(*value),
        _ => return Err(SettingsError::InvalidValue),
    };
    let rgb = value.map(|value| Rgb::from_bytes([value[0], value[1], value[2]]));
    let rgba = value.map(Rgba::from_bytes);
    match index {
        0 => table.header_foreground = rgb,
        1 => table.body_foreground = rgb,
        2 => table.alternate_foreground = rgb,
        3 => table.border_color = rgba,
        4 => table.header_background = rgba,
        5 => table.body_background = rgba,
        6 => table.alternate_background = rgba,
        _ => return Err(SettingsError::UnknownSetting),
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inline_table_controls_are_grouped_editable_resettable_and_isolated() {
        let base = Config::default();
        let original = UserPreferences::default();
        let full = catalog(1, &base, &original, &[]).unwrap();
        let groups = customization_groups(&full);
        let group = groups
            .iter()
            .find(|group| group.key.as_str() == settings::INLINE_TABLES)
            .unwrap();
        let detail = table_settings_catalog(1, &base, &original, &base.colors).unwrap();
        let table_rows: Vec<_> = detail
            .entries()
            .iter()
            .filter(|row| row.id.as_str().starts_with("tables."))
            .collect();
        assert_eq!(table_rows.len(), 19);
        let page = table_page_catalog(
            &full,
            &slot_page_snapshot_with_config(&original, &base, &base),
        )
        .unwrap();
        assert_eq!(page.entries().len(), 20);
        assert!(!full
            .entries()
            .iter()
            .any(|row| row.id.as_str().starts_with("tables.")));
        assert!(table_rows.iter().all(|row| page.get(&row.id).is_some()));
        let mut prefs = original.clone();
        for row in table_rows {
            let value = match row.id.as_str() {
                "tables.border-style" => SettingValue::Choice("dashed".into()),
                "tables.border-weight" => SettingValue::Choice("thick".into()),
                "tables.banding" => SettingValue::Choice("rows".into()),
                id if id.starts_with("tables.opacity.") => SettingValue::Number(25.0),
                id if id.starts_with("tables.colors.")
                    || id.starts_with("tables.backgrounds.") =>
                {
                    SettingValue::Color([9, 19, 29, 255])
                }
                _ => SettingValue::Boolean(false),
            };
            let edit = Edit {
                revision: 1,
                id: row.id.clone(),
                change: Change::Set(value.clone()),
            };
            prefs = apply_edit(1, &base, &prefs, &[], &edit).unwrap();
            assert_eq!(
                table_settings_catalog(1, &base, &prefs, &base.colors)
                    .unwrap()
                    .get(&row.id)
                    .unwrap()
                    .value,
                value
            );
        }
        assert_eq!(
            prefs.visual.tables.border_style,
            Some(TableBorderStyle::Dashed)
        );
        assert_eq!(prefs.visual.tables.banding, Some(TableBanding::Rows));
        assert_eq!(
            prefs.visual.tables.border_color.unwrap().bytes(),
            [9, 19, 29, 64]
        );
        assert_eq!(prefs.visual.kubernetes, original.visual.kubernetes);
        assert_eq!(prefs.visual.highlight, original.visual.highlight);
        assert_eq!(prefs.presentation, original.presentation);
        let mut expected = original;
        expected.presentation.inline_tables = Some(true);
        assert_eq!(
            reset_customizations(
                &prefs,
                &CustomizationResetScope::Group(group.key.clone()),
                None
            )
            .unwrap(),
            expected
        );
    }

    #[test]
    fn table_opacity_uses_configured_rgb_and_invalid_choices_do_not_apply() {
        let mut base = Config::default();
        base.presentation.tables.border_color = Some(Rgba::from_bytes([11, 22, 33, 255]));
        let original = UserPreferences::default();
        for percent in [0.0, 50.0, 100.0] {
            let edited = apply_edit(
                1,
                &base,
                &original,
                &[],
                &Edit {
                    revision: 1,
                    id: settings::SettingId::new("tables.opacity.border").unwrap(),
                    change: Change::Set(SettingValue::Number(percent)),
                },
            )
            .unwrap();
            assert_eq!(
                edited.visual.tables.border_color.unwrap().bytes(),
                [11, 22, 33, (percent * 255.0 / 100.0).round() as u8]
            );
            let reset = apply_edit(
                1,
                &base,
                &edited,
                &[],
                &Edit {
                    revision: 1,
                    id: settings::SettingId::new("tables.opacity.border").unwrap(),
                    change: Change::Reset,
                },
            )
            .unwrap();
            assert_eq!(reset, original);
        }
        for (id, value) in [
            (
                "tables.border-style",
                SettingValue::Choice("invalid".into()),
            ),
            ("tables.banding", SettingValue::Boolean(true)),
            ("tables.opacity.border", SettingValue::Number(101.0)),
            ("tables.opacity.header", SettingValue::Number(f64::NAN)),
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
