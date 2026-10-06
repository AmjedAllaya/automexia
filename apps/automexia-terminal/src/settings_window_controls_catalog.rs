//! Per-style caption controls use the shared settings editors and preferences.
use super::*;
use rio_backend::config::presentation::{
    WindowControlProfile, WindowControlSize, WindowControlSpacing, WindowControlStyle,
    WindowControlWeight, WindowControlsAppearance,
};

pub(super) fn style_row(
    base: WindowControlsAppearance,
    current: WindowControlsAppearance,
    user: WindowControlsAppearance,
) -> Result<settings::SettingDescriptor, SettingsError> {
    let choices: Vec<_> = WindowControlStyle::ALL
        .iter()
        .map(|s| (s.id(), s.label()))
        .collect();
    let mut row = visual_row(
        WINDOW_CONTROLS,
        "Button style".into(),
        choice_kind(&choices),
        SettingValue::Choice(current.style.unwrap_or_default().id().into()),
        SettingValue::Choice(base.style.unwrap_or_default().id().into()),
        visual_origin(
            user.style.is_some(),
            base.style.is_some(),
            ValueOrigin::Default,
        ),
    )?;
    row.description =
        "Customize Automexia's title-bar buttons; each style remembers its appearance."
            .into();
    row.keywords = vec![
        "window controls buttons caption".into(),
        "minimize maximize restore close".into(),
        "soft glass circles outline".into(),
    ];
    Ok(row)
}

fn color(profile: WindowControlProfile, index: usize) -> Option<[u8; 4]> {
    match index {
        0 => profile.minimize.map(Rgb::rgba_bytes),
        1 => profile.maximize.map(Rgb::rgba_bytes),
        2 => profile.close.map(Rgb::rgba_bytes),
        3 => profile.background.map(Rgba::bytes),
        4 => profile.border.map(Rgba::bytes),
        _ => None,
    }
}
const COLORS: [(&str, &str); 5] = [
    ("minimize", "Minimize icon"),
    ("maximize", "Maximize / restore icon"),
    ("close", "Close icon"),
    ("background", "Button background"),
    ("border", "Button border"),
];

pub(super) fn descriptors(
    base: WindowControlsAppearance,
    current: WindowControlsAppearance,
    user: WindowControlsAppearance,
    palette: &Colors,
    style: WindowControlStyle,
) -> Result<Vec<settings::SettingDescriptor>, SettingsError> {
    let mut rows = vec![style_row(base, current, user)?];
    let (base, current, user) = (
        *base.profile(style),
        *current.profile(style),
        *user.profile(style),
    );
    let prefix = format!("window-controls.{}", style.id());
    macro_rules! choice {
        ($field:ident,$id:literal,$label:literal,$ty:ty,$help:literal) => {{
            let choices: Vec<_> =
                <$ty>::ALL.iter().map(|v| (v.id(), v.label())).collect();
            let mut row = visual_row(
                &format!("{prefix}.{}", $id),
                $label.into(),
                choice_kind(&choices),
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
        size,
        "size",
        "Button size",
        WindowControlSize,
        "Visual size; the full caption click targets stay available."
    );
    choice!(
        spacing,
        "spacing",
        "Button spacing",
        WindowControlSpacing,
        "Space between the visible buttons."
    );
    choice!(
        icon_size,
        "icon-size",
        "Icon size",
        WindowControlSize,
        "Symbols automatically fit inside small buttons."
    );
    choice!(
        icon_weight,
        "icon-weight",
        "Icon weight",
        WindowControlWeight,
        "Line thickness for all three action symbols."
    );
    macro_rules! percent {
        ($field:ident,$id:literal,$label:literal,$default:literal,$help:literal) => {{
            let mut row = visual_row(
                &format!("{prefix}.{}", $id),
                $label.into(),
                settings::SettingKind::Number {
                    min: 0.0,
                    max: 100.0,
                    step: 1.0,
                },
                SettingValue::Number(f64::from(
                    current.$field.map_or($default, |v| v.get()),
                )),
                SettingValue::Number(f64::from(
                    base.$field.map_or($default, |v| v.get()),
                )),
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
    if style != WindowControlStyle::Circles {
        percent!(
            roundness,
            "roundness",
            "Corner roundness (%)",
            45,
            "0: square corners. 100: fully rounded."
        );
    }
    percent!(
        hover_strength,
        "hover-strength",
        "Hover tint (%)",
        16,
        "Strength of the icon color on hover; pressing adds clear feedback."
    );
    percent!(
        inactive_opacity,
        "inactive-opacity",
        "Inactive intensity (%)",
        65,
        "Dim unfocused window controls while keeping symbols readable."
    );
    let defaults = crate::renderer::island::window_control_color_defaults(
        crate::renderer::ui_theme::UiTheme::from_colors(palette),
        style,
    )
    .map(crate::renderer::ui_theme::color_u8);
    for (index, (id, label)) in COLORS.into_iter().enumerate() {
        let value = color(current, index).unwrap_or(defaults[index]);
        let default = color(base, index).unwrap_or(defaults[index]);
        let origin = visual_origin(
            color(user, index).is_some(),
            color(base, index).is_some(),
            ValueOrigin::Default,
        );
        let mut row = visual_row(
            &format!("{prefix}.{id}"),
            label.into(),
            settings::SettingKind::Color { alpha: index >= 3 },
            SettingValue::Color(value),
            SettingValue::Color(default),
            origin,
        )?;
        row.description = if index < 3 {
            "Follows the theme until edited; contrast stays readable on the button."
        } else {
            "Reset follows this style and the active theme."
        }
        .into();
        rows.push(row);
        if index >= 3 {
            rows.push(visual_row(
                &format!("{prefix}.{id}-opacity"),
                format!("{label} opacity (%)"),
                settings::SettingKind::Number {
                    min: 0.0,
                    max: 100.0,
                    step: 1.0,
                },
                SettingValue::Number((f64::from(value[3]) * 100.0 / 255.0).round()),
                SettingValue::Number((f64::from(default[3]) * 100.0 / 255.0).round()),
                origin,
            )?);
        }
    }
    Ok(rows)
}

pub(super) fn catalog(
    revision: u64,
    base: &Config,
    prefs: &UserPreferences,
    palette: &Colors,
    id: &str,
) -> Result<Catalog, SettingsError> {
    let mut current = base.presentation.window_controls;
    prefs.visual.window_controls.overlay(&mut current);
    let style = if id == WINDOW_CONTROLS {
        current.style.unwrap_or_default()
    } else {
        let rest = id
            .strip_prefix("window-controls.")
            .ok_or(SettingsError::UnknownSetting)?;
        let (style, _) = rest.split_once('.').ok_or(SettingsError::UnknownSetting)?;
        WindowControlStyle::from_id(style).ok_or(SettingsError::UnknownSetting)?
    };
    Catalog::new(
        revision,
        descriptors(
            base.presentation.window_controls,
            current,
            prefs.visual.window_controls,
            palette,
            style,
        )?,
    )
}

pub(super) fn apply(
    base: &Config,
    prefs: &UserPreferences,
    edit: &Edit,
    palette: &Colors,
) -> Result<UserPreferences, SettingsError> {
    let rows = catalog(edit.revision, base, prefs, palette, edit.id.as_str())?;
    rows.validate_edit(edit)?;
    let mut candidate = prefs.clone();
    if edit.id.as_str() == WINDOW_CONTROLS {
        candidate.visual.window_controls.style = match &edit.change {
            Change::Reset => None,
            Change::Set(SettingValue::Choice(value)) => Some(
                WindowControlStyle::from_id(value).ok_or(SettingsError::InvalidValue)?,
            ),
            _ => return Err(SettingsError::InvalidValue),
        };
        return Ok(candidate);
    }
    let rest = edit
        .id
        .as_str()
        .strip_prefix("window-controls.")
        .ok_or(SettingsError::UnknownSetting)?;
    let (style, field) = rest.split_once('.').ok_or(SettingsError::UnknownSetting)?;
    let style =
        WindowControlStyle::from_id(style).ok_or(SettingsError::UnknownSetting)?;
    let profile = candidate.visual.window_controls.profile_mut(style);
    macro_rules! choice {
        ($field:ident,$ty:ty) => {{
            profile.$field = match &edit.change {
                Change::Reset => None,
                Change::Set(SettingValue::Choice(value)) => {
                    Some(<$ty>::from_id(value).ok_or(SettingsError::InvalidValue)?)
                }
                _ => return Err(SettingsError::InvalidValue),
            };
        }};
    }
    macro_rules! percent {
        ($field:ident) => {{
            profile.$field = match &edit.change {
                Change::Reset => None,
                Change::Set(SettingValue::Number(value)) => Some(
                    OpacityPercent::new(*value as u8)
                        .map_err(|_| SettingsError::InvalidValue)?,
                ),
                _ => return Err(SettingsError::InvalidValue),
            };
        }};
    }
    match field {
        "size" => choice!(size, WindowControlSize),
        "spacing" => choice!(spacing, WindowControlSpacing),
        "icon-size" => choice!(icon_size, WindowControlSize),
        "icon-weight" => choice!(icon_weight, WindowControlWeight),
        "roundness" => percent!(roundness),
        "hover-strength" => percent!(hover_strength),
        "inactive-opacity" => percent!(inactive_opacity),
        "minimize" => profile.minimize = rgb_change(&edit.change)?,
        "maximize" => profile.maximize = rgb_change(&edit.change)?,
        "close" => profile.close = rgb_change(&edit.change)?,
        "background" | "border" => {
            let value = match &edit.change {
                Change::Reset => None,
                Change::Set(SettingValue::Color(value)) => Some(Rgba::from_bytes(*value)),
                _ => return Err(SettingsError::InvalidValue),
            };
            if field == "background" {
                profile.background = value;
            } else {
                profile.border = value;
            }
        }
        "background-opacity" | "border-opacity" => {
            let key = field
                .strip_suffix("-opacity")
                .ok_or(SettingsError::UnknownSetting)?;
            let row = rows
                .get(&settings::SettingId::new(format!(
                    "window-controls.{}.{key}",
                    style.id()
                ))?)
                .ok_or(SettingsError::UnknownSetting)?;
            let (SettingValue::Color(mut value), SettingValue::Color(default)) =
                (&row.value, &row.default)
            else {
                return Err(SettingsError::InvalidValue);
            };
            value[3] = match edit.change {
                Change::Reset => default[3],
                Change::Set(SettingValue::Number(v)) => (v * 255.0 / 100.0).round() as u8,
                _ => return Err(SettingsError::InvalidValue),
            };
            if key == "background" {
                profile.background = Some(Rgba::from_bytes(value));
            } else {
                profile.border = Some(Rgba::from_bytes(value));
            }
        }
        _ => return Err(SettingsError::UnknownSetting),
    }
    Ok(candidate)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn edit(id: &str, change: Change) -> Edit {
        Edit {
            revision: 17,
            id: settings::SettingId::new(id).unwrap(),
            change,
        }
    }
    #[test]
    fn window_controls_all_styles_are_editable_isolated_and_resettable() {
        let base = Config::default();
        let original = UserPreferences::default();
        let mut prefs = original.clone();
        for &style in WindowControlStyle::ALL {
            prefs = apply_edit(
                17,
                &base,
                &prefs,
                &[],
                &edit(
                    WINDOW_CONTROLS,
                    Change::Set(SettingValue::Choice(style.id().into())),
                ),
            )
            .unwrap();
            let full = crate::settings_catalog::catalog(17, &base, &prefs, &[]).unwrap();
            let snapshot = slot_page_snapshot_with_config(
                &prefs,
                &prefs.apply_to(&base),
                &base,
                &crate::settings_catalog::test_installed_extensions(),
            );
            let page = window_controls_page_catalog(&full, &snapshot).unwrap();
            assert!(page.entries().len() >= 14);
            for row in page
                .entries()
                .iter()
                .filter(|row| row.id.as_str() != WINDOW_CONTROLS)
            {
                let value = match &row.kind {
                    settings::SettingKind::Choice { options } => {
                        SettingValue::Choice(options.last().unwrap().value.clone())
                    }
                    settings::SettingKind::Number { .. } => SettingValue::Number(50.0),
                    settings::SettingKind::Color { alpha } => {
                        SettingValue::Color([19, 37, 53, if *alpha { 128 } else { 255 }])
                    }
                    _ => panic!("unexpected caption editor"),
                };
                prefs = apply_edit(
                    17,
                    &base,
                    &prefs,
                    &[],
                    &Edit {
                        revision: 17,
                        id: row.id.clone(),
                        change: Change::Set(value.clone()),
                    },
                )
                .unwrap();
                let rows =
                    catalog(17, &base, &prefs, &base.colors, row.id.as_str()).unwrap();
                assert_eq!(rows.get(&row.id).unwrap().value, value);
            }
        }
        for &style in WindowControlStyle::ALL {
            assert!(!prefs.visual.window_controls.profile(style).is_empty());
        }
        assert_eq!(prefs.visual.tables, original.visual.tables);
        assert_eq!(prefs.visual.kubernetes, original.visual.kubernetes);
        assert_eq!(prefs.presentation, original.presentation);
        let reset = reset_customizations(
            &prefs,
            &CustomizationResetScope::Group(
                settings::SettingId::new(WINDOW_CONTROLS).unwrap(),
            ),
            None,
        )
        .unwrap();
        assert_eq!(reset, original);
    }
    #[test]
    fn window_controls_opacity_keeps_configured_rgb_and_rejects_invalid_input() {
        let mut base = Config::default();
        base.presentation.window_controls.glass.background =
            Some(Rgba::from_bytes([11, 22, 33, 128]));
        let original = UserPreferences::default();
        for percent in [0.0, 50.0, 100.0] {
            let prefs = apply_edit(
                17,
                &base,
                &original,
                &[],
                &edit(
                    "window-controls.glass.background-opacity",
                    Change::Set(SettingValue::Number(percent)),
                ),
            )
            .unwrap();
            assert_eq!(
                prefs
                    .visual
                    .window_controls
                    .glass
                    .background
                    .unwrap()
                    .bytes(),
                [11, 22, 33, (percent * 255.0 / 100.0).round() as u8]
            );
            let reset = apply_edit(
                17,
                &base,
                &prefs,
                &[],
                &edit("window-controls.glass.background", Change::Reset),
            )
            .unwrap();
            assert!(reset.visual.window_controls.glass.background.is_none());
        }
        for value in [f64::NAN, f64::INFINITY, -1.0, 101.0, 0.5] {
            assert!(apply_edit(
                17,
                &base,
                &original,
                &[],
                &edit(
                    "window-controls.soft.hover-strength",
                    Change::Set(SettingValue::Number(value))
                )
            )
            .is_err());
        }
        assert!(apply_edit(
            18,
            &base,
            &original,
            &[],
            &edit(
                WINDOW_CONTROLS,
                Change::Set(SettingValue::Choice("glass".into()))
            )
        )
        .is_err());
        assert!(apply_edit(
            17,
            &base,
            &original,
            &[],
            &edit(
                WINDOW_CONTROLS,
                Change::Set(SettingValue::Choice("unknown".into()))
            )
        )
        .is_err());
    }
}
