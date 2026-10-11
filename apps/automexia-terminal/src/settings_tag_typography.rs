//! One descriptor/edit owner for shared and individual information-tag styles.
use super::*;
use automexia_ui_model::information_bar::TagTypography;

pub(super) fn descriptors(
    snapshot: &SlotPageSnapshot,
    slot: Option<&str>,
) -> Result<Vec<settings::SettingDescriptor>, SettingsError> {
    let styles = &snapshot.bar.styles;
    let empty = TagTypography::default();
    let chosen = slot.map_or(&styles.shared, |id| styles.tags.get(id).unwrap_or(&empty));
    let inherited = if slot.is_some() {
        &styles.shared
    } else {
        &empty
    };
    let current = chosen.over(inherited);
    let prefix = slot.map_or_else(
        || "tags.shared.".into(),
        |id| format!("tags.slot.{id}.style."),
    );
    let fallback_family = snapshot
        .font
        .fonts
        .family
        .as_ref()
        .unwrap_or(&snapshot.font.fonts.regular.family);
    let mut rows = Vec::new();
    if slot.is_some() {
        let mut inherit = visual_row(
            &format!("{prefix}inherit"),
            "Inherit shared styling".into(),
            settings::SettingKind::Action,
            SettingValue::Action,
            SettingValue::Action,
            ValueOrigin::Default,
        )?;
        inherit.description =
            "Keep content and placement; remove appearance overrides.".into();
        rows.push(inherit);
    }

    macro_rules! number {
        ($field:ident,$id:literal,$label:literal,$default:expr,$min:expr,$max:expr,$help:literal) => {{
            let mut row = visual_row(
                &format!("{prefix}{}", $id),
                $label.into(),
                settings::SettingKind::Number {
                    min: $min,
                    max: $max,
                    step: 1.0,
                },
                SettingValue::Number(f64::from(current.$field.unwrap_or($default))),
                SettingValue::Number(f64::from(inherited.$field.unwrap_or($default))),
                visual_origin(chosen.$field.is_some(), false, ValueOrigin::Default),
            )?;
            row.description = $help.into();
            rows.push(row);
        }};
    }
    number!(
        size,
        "size",
        "Text size (%)",
        100,
        50.0,
        150.0,
        "Percent of automatic sizing; fits prompt rows."
    );
    let mut family = visual_row(
        &format!("{prefix}family"),
        "Font family".into(),
        settings::SettingKind::Text {
            max_bytes: 128,
            allow_empty: false,
        },
        SettingValue::Text(current.family.as_ref().unwrap_or(fallback_family).clone()),
        SettingValue::Text(inherited.family.as_ref().unwrap_or(fallback_family).clone()),
        visual_origin(chosen.family.is_some(), false, ValueOrigin::Default),
    )?;
    family.description = "Browse installed fonts; preview before applying.".into();
    rows.push(family);
    macro_rules! boolean {
        ($field:ident,$label:literal) => {{
            let mut row = visual_row(
                &format!("{prefix}{}", stringify!($field)),
                $label.into(),
                settings::SettingKind::Boolean,
                SettingValue::Boolean(current.$field.unwrap_or(false)),
                SettingValue::Boolean(inherited.$field.unwrap_or(false)),
                visual_origin(chosen.$field.is_some(), false, ValueOrigin::Default),
            )?;
            row.description = "Off overrides shared On; Reset inherits.".into();
            rows.push(row);
        }};
    }
    boolean!(bold, "Bold text");
    boolean!(italic, "Italic text");
    boolean!(underline, "Underline text");
    boolean!(strike, "Strike through text");
    let role = slot.and_then(automexia_ui_model::information_bar::role_from_id);
    let anchor = role.map_or(snapshot.foreground, |role| {
        crate::automexia::presentation::tag_anchor(&snapshot.tags, role)
    });
    macro_rules! color {
        ($field:ident,$id:literal,$label:literal,$fallback:expr) => {{
            let rgba = |c: [u8; 3]| SettingValue::Color([c[0], c[1], c[2], 255]);
            let mut row = visual_row(
                &format!("{prefix}{}", $id),
                $label.into(),
                settings::SettingKind::Color { alpha: false },
                rgba(current.$field.unwrap_or($fallback)),
                rgba(inherited.$field.unwrap_or($fallback)),
                visual_origin(chosen.$field.is_some(), false, ValueOrigin::Default),
            )?;
            row.description = "Contrast-corrected color; Reset inherits.".into();
            rows.push(row);
        }};
    }
    color!(foreground, "foreground", "Text color", anchor);
    color!(background, "background", "Background color", anchor);
    color!(
        icon_color,
        "icon-color",
        "Icon color",
        current.foreground.unwrap_or(anchor)
    );
    color!(
        border_color,
        "border-color",
        "Border color",
        current.foreground.unwrap_or(anchor)
    );
    if slot.is_some() {
        number!(
            opacity,
            "opacity",
            "Background opacity (%)",
            snapshot.tags.opacity.get(),
            0.0,
            100.0,
            "Background only; text stays opaque."
        );
    }
    number!(
        border_width,
        "border-width",
        "Border width",
        1,
        0.0,
        3.0,
        "Logical pixels; zero removes the outline."
    );
    number!(
        padding_x,
        "padding-x",
        "Horizontal padding",
        5,
        0.0,
        16.0,
        "Interior pixels; connectors retain their clearance."
    );
    number!(
        padding_y,
        "padding-y",
        "Vertical padding",
        2,
        0.0,
        8.0,
        "Interior pixels, bounded by prompt row height."
    );
    number!(
        icon_size,
        "icon-size",
        "Icon size (%)",
        100,
        50.0,
        150.0,
        "Percent of tag text; fits prompt rows."
    );
    number!(
        icon_gap,
        "icon-gap",
        "Icon-to-text spacing",
        4,
        0.0,
        16.0,
        "Pixels between icon and label."
    );
    for row in &mut rows {
        row.keywords = vec![
            "information tags shared individual".into(),
            "font typography color size padding".into(),
        ];
        if slot.is_some() && row.origin == ValueOrigin::Default {
            row.description = format!("Inherits shared. {}", row.description);
        }
    }
    Ok(rows)
}

pub(super) fn apply(
    candidate: &mut UserPreferences,
    edit: &Edit,
) -> Result<bool, SettingsError> {
    let id = edit.id.as_str();
    let (slot, field) = if let Some(field) = id.strip_prefix("tags.shared.") {
        (None, field)
    } else if let Some((slot, field)) = id
        .strip_prefix("tags.slot.")
        .and_then(|s| s.split_once(".style."))
    {
        (Some(slot), field)
    } else {
        return Ok(false);
    };
    if field == "inherit" {
        let Some(slot) = slot else {
            return Err(SettingsError::InvalidValue);
        };
        if !matches!(edit.change, Change::Activate) {
            return Err(SettingsError::InvalidValue);
        };
        candidate.visual.information_bar.styles.tags.remove(slot);
        if let Some(recipe) = candidate.visual.information_bar.custom_recipe.as_mut() {
            if let Some(tag) = recipe.slots.iter_mut().find(|s| s.id == slot) {
                tag.color = None;
            }
        }
        return Ok(true);
    }
    let styles = &mut candidate.visual.information_bar.styles;
    let style = if let Some(slot) = slot {
        styles.tags.entry(slot.into()).or_default()
    } else {
        &mut styles.shared
    };
    macro_rules! number {
        ($field:ident,$ty:ty) => {
            style.$field = match edit.change {
                Change::Reset => None,
                Change::Set(SettingValue::Number(v))
                    if v.is_finite() && v.fract() == 0.0 =>
                {
                    Some(v as $ty)
                }
                _ => return Err(SettingsError::InvalidValue),
            }
        };
    }
    macro_rules! boolean {
        ($field:ident) => {
            style.$field = match edit.change {
                Change::Reset => None,
                Change::Set(SettingValue::Boolean(v)) => Some(v),
                _ => return Err(SettingsError::InvalidValue),
            }
        };
    }
    macro_rules! color {
        ($field:ident) => {
            style.$field = match edit.change {
                Change::Reset => None,
                Change::Set(SettingValue::Color([r, g, b, 255])) => Some([r, g, b]),
                _ => return Err(SettingsError::InvalidValue),
            }
        };
    }
    match field {
        "size" => number!(size, u16),
        "icon-size" => number!(icon_size, u16),
        "opacity" => number!(opacity, u8),
        "border-width" => number!(border_width, u8),
        "padding-x" => number!(padding_x, u8),
        "padding-y" => number!(padding_y, u8),
        "icon-gap" => number!(icon_gap, u8),
        "bold" => boolean!(bold),
        "italic" => boolean!(italic),
        "underline" => boolean!(underline),
        "strike" => boolean!(strike),
        "foreground" => color!(foreground),
        "background" => color!(background),
        "icon-color" => color!(icon_color),
        "border-color" => color!(border_color),
        "family" => {
            style.family = match &edit.change {
                Change::Reset => None,
                Change::Set(SettingValue::Text(v)) => Some(v.clone()),
                _ => return Err(SettingsError::InvalidValue),
            }
        }
        _ => return Err(SettingsError::UnknownSetting),
    }
    if !style.valid() {
        return Err(SettingsError::InvalidValue);
    }
    if let Some(slot) = slot {
        if style.is_empty() {
            styles.tags.remove(slot);
        }
    }
    if !styles.valid() {
        return Err(SettingsError::InvalidValue);
    }
    Ok(true)
}
