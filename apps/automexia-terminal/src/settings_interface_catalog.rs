//! Terminal appearance pages use the existing catalog, picker and preference writer.
use super::*;
use rio_backend::config::presentation::{FooterAppearance, HeaderAppearance, UiPixels};

pub(super) const PAGES: [(&str, &str, &str); 4] = [
    (
        "interface.header.background",
        "Header & Tabs",
        "Style the header, tab colors, spacing and titles.",
    ),
    (
        "interface.footer.visible",
        "Footer",
        "Show or hide the footer; choose its style and status items.",
    ),
    (
        "interface.panes.padding",
        "Panes & Borders",
        "Adjust pane padding, gaps, dividers and inactive intensity.",
    ),
    (
        "interface.background.opacity",
        "Background & Spacing",
        "Adjust transparency, blur and space around terminal content.",
    ),
];

fn color(v: Rgba) -> [u8; 4] {
    v.bytes()
}
fn rgb(v: Rgb) -> [u8; 4] {
    v.rgba_bytes()
}

pub(super) fn descriptors(
    base: &Config,
    current: &Config,
    prefs: &UserPreferences,
    palette: &Colors,
) -> Result<Vec<settings::SettingDescriptor>, SettingsError> {
    let mut rows = Vec::new();
    let ui = crate::renderer::ui_theme::UiTheme::from_colors(palette);
    let bytes = crate::renderer::ui_theme::color_u8;
    let (active_tab, inactive_tab) =
        crate::renderer::island::tab_background_defaults(palette.background.0);
    let user = &prefs.visual.interface;
    macro_rules! row {
        ($id:expr,$label:expr,$kind:expr,$value:expr,$default:expr,$over:expr,$configured:expr,$help:expr) => {{
            let mut row = visual_row(
                $id,
                $label.into(),
                $kind,
                $value,
                $default,
                visual_origin($over, $configured, ValueOrigin::Default),
            )?;
            row.description = $help.into();
            rows.push(row);
        }};
    }
    macro_rules! pixels {
        ($prefix:literal,$b:expr,$c:expr,$u:expr,$field:ident,$id:literal,$label:literal,$fallback:expr,$min:expr,$max:expr,$help:literal) => {
            row!(
                concat!($prefix, $id),
                $label,
                settings::SettingKind::ContinuousNumber {
                    min: $min,
                    max: $max,
                    step: 1.0
                },
                SettingValue::Number(f64::from(
                    $c.$field.map_or($fallback, UiPixels::get)
                )),
                SettingValue::Number(f64::from(
                    $b.$field.map_or($fallback, UiPixels::get)
                )),
                $u.$field.is_some(),
                $b.$field.is_some(),
                $help
            );
        };
    }
    macro_rules! boolean {
        ($prefix:literal,$b:expr,$c:expr,$u:expr,$field:ident,$id:literal,$label:literal,$fallback:expr,$help:literal) => {
            row!(
                concat!($prefix, $id),
                $label,
                settings::SettingKind::Boolean,
                SettingValue::Boolean($c.$field.unwrap_or($fallback)),
                SettingValue::Boolean($b.$field.unwrap_or($fallback)),
                $u.$field.is_some(),
                $b.$field.is_some(),
                $help
            );
        };
    }
    macro_rules! colors {
        ($prefix:literal,$b:expr,$c:expr,$u:expr,$field:ident,$id:literal,$label:literal,$fallback:expr,$alpha:expr,$convert:ident) => {
            row!(
                concat!($prefix, $id),
                $label,
                settings::SettingKind::Color { alpha: $alpha },
                SettingValue::Color($c.$field.map_or(bytes($fallback), $convert)),
                SettingValue::Color($b.$field.map_or(bytes($fallback), $convert)),
                $u.$field.is_some(),
                $b.$field.is_some(),
                "Inherits theme colors; text contrast remains readable."
            );
        };
    }
    let (b, c, u) = (
        base.presentation.interface.footer,
        current.presentation.interface.footer,
        user.appearance.footer,
    );
    boolean!(
        "interface.footer.",
        b,
        c,
        u,
        visible,
        "visible",
        "Show Footer",
        true,
        "Hide the footer and reclaim its terminal space."
    );
    pixels!(
        "interface.footer.",
        b,
        c,
        u,
        height,
        "height",
        "Height (px)",
        32.0,
        24.0,
        72.0,
        "Logical pixels; grows when needed to fit the text."
    );
    pixels!(
        "interface.footer.",
        b,
        c,
        u,
        padding,
        "padding",
        "Horizontal padding (px)",
        14.0,
        0.0,
        32.0,
        "Space inside the footer, automatically limited in narrow panes."
    );
    pixels!(
        "interface.footer.",
        b,
        c,
        u,
        font_size,
        "font-size",
        "Text size (px)",
        12.0,
        8.0,
        24.0,
        "Uses the installed font selected in Fonts."
    );
    boolean!(
        "interface.footer.",
        b,
        c,
        u,
        bold,
        "bold",
        "Bold text",
        false,
        "Emphasize footer status text."
    );
    colors!(
        "interface.footer.",
        b,
        c,
        u,
        background,
        "background",
        "Background",
        ui.surface,
        true,
        color
    );
    colors!(
        "interface.footer.",
        b,
        c,
        u,
        text,
        "text",
        "Text color",
        ui.text,
        false,
        rgb
    );
    colors!(
        "interface.footer.",
        b,
        c,
        u,
        muted_text,
        "muted-text",
        "Secondary text color",
        ui.muted_text,
        false,
        rgb
    );
    colors!(
        "interface.footer.",
        b,
        c,
        u,
        border,
        "border",
        "Border color",
        ui.border,
        true,
        color
    );
    pixels!(
        "interface.footer.",
        b,
        c,
        u,
        border_width,
        "border-width",
        "Border width (px)",
        1.0,
        0.0,
        4.0,
        "Zero hides the footer border."
    );
    boolean!(
        "interface.footer.",
        b,
        c,
        u,
        show_clock,
        "show-clock",
        "Clock",
        true,
        "Local time."
    );
    boolean!(
        "interface.footer.",
        b,
        c,
        u,
        show_dimensions,
        "show-dimensions",
        "Grid dimensions",
        true,
        "Current columns and rows for this pane."
    );
    boolean!(
        "interface.footer.",
        b,
        c,
        u,
        show_encoding,
        "show-encoding",
        "Encoding",
        true,
        "UTF-8 indicator."
    );
    boolean!(
        "interface.footer.",
        b,
        c,
        u,
        show_line_ending,
        "show-line-ending",
        "Line ending",
        true,
        "Session-aware LF or CRLF indicator."
    );
    boolean!(
        "interface.footer.",
        b,
        c,
        u,
        show_pane,
        "show-pane",
        "Pane position",
        true,
        "Appears when the workspace has multiple panes."
    );
    boolean!(
        "interface.footer.",
        b,
        c,
        u,
        show_tab,
        "show-tab",
        "Local tab position",
        true,
        "Appears when the pane contains multiple tabs."
    );
    boolean!(
        "interface.footer.",
        b,
        c,
        u,
        show_context,
        "show-context",
        "Session indicators",
        true,
        "Profile, key chord, key table, zoom and shortcut diagnostics."
    );
    boolean!(
        "interface.footer.",
        b,
        c,
        u,
        show_selection,
        "show-selection",
        "Selection and history",
        true,
        "Indicates selection or a scrolled viewport."
    );
    let (b, c, u) = (
        base.presentation.interface.header,
        current.presentation.interface.header,
        user.appearance.header,
    );
    colors!(
        "interface.header.",
        b,
        c,
        u,
        background,
        "background",
        "Header background",
        ui.surface,
        true,
        color
    );
    colors!(
        "interface.header.",
        b,
        c,
        u,
        border,
        "border",
        "Header border color",
        ui.border,
        true,
        color
    );
    pixels!(
        "interface.header.",
        b,
        c,
        u,
        border_width,
        "border-width",
        "Header border width (px)",
        1.0,
        0.0,
        4.0,
        "Zero hides the lower header border."
    );
    colors!(
        "interface.header.",
        b,
        c,
        u,
        text,
        "text",
        "Active tab text",
        ui.text,
        false,
        rgb
    );
    colors!(
        "interface.header.",
        b,
        c,
        u,
        inactive_text,
        "inactive-text",
        "Inactive tab text",
        ui.muted_text,
        false,
        rgb
    );
    colors!(
        "interface.header.",
        b,
        c,
        u,
        active_tab,
        "active-tab",
        "Active tab background",
        active_tab,
        true,
        color
    );
    colors!(
        "interface.header.",
        b,
        c,
        u,
        inactive_tab,
        "inactive-tab",
        "Inactive tab background",
        inactive_tab,
        true,
        color
    );
    pixels!(
        "interface.header.",
        b,
        c,
        u,
        tab_radius,
        "tab-radius",
        "Tab corner radius (px)",
        9.0,
        0.0,
        24.0,
        "Zero gives tabs square corners."
    );
    pixels!(
        "interface.header.",
        b,
        c,
        u,
        tab_gap,
        "tab-gap",
        "Tab spacing (px)",
        7.0,
        0.0,
        20.0,
        "Space between tab surfaces; close targets remain usable."
    );
    pixels!(
        "interface.header.",
        b,
        c,
        u,
        font_size,
        "font-size",
        "Tab text size (px)",
        14.0,
        10.0,
        20.0,
        "Titles fit the available width and use the font chosen in Fonts."
    );
    macro_rules! native_number {
        ($field:ident,$id:literal,$label:literal,$base:expr,$value:expr,$min:expr,$max:expr,$help:literal) => {{
            let value = f64::from($value);
            let default = f64::from($base);
            if value.is_finite() && default.is_finite()
                && ($min..=$max).contains(&value) && ($min..=$max).contains(&default) {
                let kind = if matches!(stringify!($field), "opacity" | "inactive_opacity") {
                    settings::SettingKind::Number { min:$min,max:$max,step:1.0 }
                } else { settings::SettingKind::ContinuousNumber { min:$min,max:$max,step:1.0 } };
                row!($id,$label,kind,
                    SettingValue::Number(value),SettingValue::Number(default),user.$field.is_some(),true,$help);
            } else {
                // Preserve valid legacy configuration outside the bounded UI range.
                // One unusual field must never prevent opening other settings.
                row!($id,$label,settings::SettingKind::Text { max_bytes:64,allow_empty:false },
                    SettingValue::Text(format!("{value}")),SettingValue::Text(format!("{default}")),user.$field.is_some(),true,$help);
                if let Some(row) = rows.last_mut() { row.availability = settings::Availability::Unavailable {
                    reason: "Configured value is outside this control's range; edit config.toml.".into() }; }
            }
        }};
    }
    native_number!(
        tab_width,
        "interface.header.tab-width",
        "Maximum tab width (px)",
        base.navigation.max_tab_width,
        current.navigation.max_tab_width,
        80.0,
        280.0,
        "Tabs shrink to fit the window."
    );
    native_number!(
        pane_padding,
        "interface.panes.padding",
        "Pane padding (px)",
        base.panel.padding.left,
        current.panel.padding.left,
        0.0,
        32.0,
        "Space inside panes. Reset restores configured edges."
    );
    native_number!(
        pane_margin,
        "interface.panes.margin",
        "Pane margin (px)",
        base.panel.margin.left,
        current.panel.margin.left,
        0.0,
        32.0,
        "Space outside each pane; sets all four edges."
    );
    native_number!(
        row_gap,
        "interface.panes.row-gap",
        "Row gap (px)",
        base.panel.row_gap,
        current.panel.row_gap,
        0.0,
        32.0,
        "Vertical space between split panes."
    );
    native_number!(
        column_gap,
        "interface.panes.column-gap",
        "Column gap (px)",
        base.panel.column_gap,
        current.panel.column_gap,
        0.0,
        32.0,
        "Horizontal space between split panes."
    );
    native_number!(
        border_width,
        "interface.panes.border-width",
        "Divider width (px)",
        base.panel.border_width,
        current.panel.border_width,
        0.0,
        8.0,
        "Zero hides dividers; keyboard pane focus remains indicated."
    );
    row!(
        "interface.panes.border-color",
        "Divider color",
        settings::SettingKind::Color { alpha: false },
        SettingValue::Color(bytes(current.colors.split)),
        SettingValue::Color(bytes(base.colors.split)),
        user.border_color.is_some(),
        true,
        "Color of separators between panes."
    );
    native_number!(
        inactive_opacity,
        "interface.panes.inactive-opacity",
        "Inactive pane intensity (%)",
        (base.navigation.unfocused_split_opacity * 100.0).round(),
        (current.navigation.unfocused_split_opacity * 100.0).round(),
        15.0,
        100.0,
        "Keep inactive panes readable while making focus clear."
    );
    native_number!(
        opacity,
        "interface.background.opacity",
        "Window opacity (%)",
        (base.window.opacity * 100.0).round(),
        (current.window.opacity * 100.0).round(),
        20.0,
        100.0,
        "Transparency depends on the system compositor."
    );
    row!(
        "interface.background.opacity-cells",
        "Transparent cell backgrounds",
        settings::SettingKind::Boolean,
        SettingValue::Boolean(current.window.opacity_cells),
        SettingValue::Boolean(base.window.opacity_cells),
        user.opacity_cells.is_some(),
        true,
        "Also apply window opacity to explicit application background colors."
    );
    row!(
        "interface.background.blur",
        "Background blur",
        settings::SettingKind::Boolean,
        SettingValue::Boolean(current.window.blur.is_enabled()),
        SettingValue::Boolean(base.window.blur.is_enabled()),
        user.blur.is_some(),
        true,
        "Uses the system blur effect where supported; requires transparency."
    );
    native_number!(
        padding_top,
        "interface.background.padding-top",
        "Top padding (px)",
        base.margin.top,
        current.margin.top,
        0.0,
        64.0,
        "Space between header and terminal content."
    );
    native_number!(
        padding_bottom,
        "interface.background.padding-bottom",
        "Bottom padding (px)",
        base.margin.bottom,
        current.margin.bottom,
        0.0,
        64.0,
        "Space below the pane layout."
    );
    native_number!(
        padding_left,
        "interface.background.padding-left",
        "Left padding (px)",
        base.margin.left,
        current.margin.left,
        0.0,
        64.0,
        "Space to the left of terminal content."
    );
    native_number!(
        padding_right,
        "interface.background.padding-right",
        "Right padding (px)",
        base.margin.right,
        current.margin.right,
        0.0,
        64.0,
        "Space to the right of terminal content."
    );
    for prefix in ["interface.footer", "interface.header"] {
        let id = format!("{prefix}.background");
        if let Some(color) = rows.iter().find(|r| r.id.as_str() == id).cloned() {
            if let (SettingValue::Color(value), SettingValue::Color(default)) =
                (color.value, color.default)
            {
                row!(
                    &format!("{prefix}.background-opacity"),
                    "Background opacity (%)",
                    settings::SettingKind::Number {
                        min: 0.0,
                        max: 100.0,
                        step: 1.0
                    },
                    SettingValue::Number((f64::from(value[3]) * 100.0 / 255.0).round()),
                    SettingValue::Number((f64::from(default[3]) * 100.0 / 255.0).round()),
                    color.origin == ValueOrigin::User,
                    color.origin == ValueOrigin::Configuration,
                    "Opacity of this surface; text remains opaque."
                );
            }
        }
    }
    Ok(rows)
}

pub(super) fn apply(
    base: &Config,
    prefs: &UserPreferences,
    edit: &Edit,
    palette: &Colors,
) -> Result<UserPreferences, SettingsError> {
    let mut next = prefs.clone();
    let target = &mut next.visual.interface;
    macro_rules! field {
        ($id:literal,$slot:expr,$pattern:pat => $value:expr) => {
            if edit.id.as_str() == $id {
                $slot = match &edit.change {
                    Change::Reset => None,
                    Change::Set($pattern) => Some($value),
                    _ => return Err(SettingsError::InvalidValue),
                };
                return Ok(next);
            }
        };
    }
    macro_rules! pixels { ($id:literal,$slot:expr) => { field!($id,$slot,SettingValue::Number(v) => UiPixels::from_decimal(*v as f32).ok_or(SettingsError::InvalidValue)?); }; }
    macro_rules! boolean { ($id:literal,$slot:expr) => { field!($id,$slot,SettingValue::Boolean(v) => *v); }; }
    macro_rules! rgb { ($id:literal,$slot:expr) => { field!($id,$slot,SettingValue::Color(v) => Rgb::from_bytes([v[0],v[1],v[2]])); }; }
    macro_rules! rgba { ($id:literal,$slot:expr) => { field!($id,$slot,SettingValue::Color(v) => Rgba::from_bytes(*v)); }; }
    // Fields below are the fixed catalog contract; no arbitrary config mutation.
    boolean!("interface.footer.visible", target.appearance.footer.visible);
    boolean!("interface.footer.bold", target.appearance.footer.bold);
    boolean!(
        "interface.footer.show-clock",
        target.appearance.footer.show_clock
    );
    boolean!(
        "interface.footer.show-dimensions",
        target.appearance.footer.show_dimensions
    );
    boolean!(
        "interface.footer.show-encoding",
        target.appearance.footer.show_encoding
    );
    boolean!(
        "interface.footer.show-line-ending",
        target.appearance.footer.show_line_ending
    );
    boolean!(
        "interface.footer.show-pane",
        target.appearance.footer.show_pane
    );
    boolean!(
        "interface.footer.show-tab",
        target.appearance.footer.show_tab
    );
    boolean!(
        "interface.footer.show-context",
        target.appearance.footer.show_context
    );
    boolean!(
        "interface.footer.show-selection",
        target.appearance.footer.show_selection
    );
    pixels!("interface.footer.height", target.appearance.footer.height);
    pixels!("interface.footer.padding", target.appearance.footer.padding);
    pixels!(
        "interface.footer.font-size",
        target.appearance.footer.font_size
    );
    pixels!(
        "interface.footer.border-width",
        target.appearance.footer.border_width
    );
    rgb!("interface.footer.text", target.appearance.footer.text);
    rgb!(
        "interface.footer.muted-text",
        target.appearance.footer.muted_text
    );
    rgba!(
        "interface.footer.background",
        target.appearance.footer.background
    );
    rgba!("interface.footer.border", target.appearance.footer.border);
    pixels!(
        "interface.header.border-width",
        target.appearance.header.border_width
    );
    pixels!(
        "interface.header.tab-radius",
        target.appearance.header.tab_radius
    );
    pixels!("interface.header.tab-gap", target.appearance.header.tab_gap);
    pixels!(
        "interface.header.font-size",
        target.appearance.header.font_size
    );
    rgb!("interface.header.text", target.appearance.header.text);
    rgb!(
        "interface.header.inactive-text",
        target.appearance.header.inactive_text
    );
    rgba!(
        "interface.header.background",
        target.appearance.header.background
    );
    rgba!("interface.header.border", target.appearance.header.border);
    rgba!(
        "interface.header.active-tab",
        target.appearance.header.active_tab
    );
    rgba!(
        "interface.header.inactive-tab",
        target.appearance.header.inactive_tab
    );
    pixels!("interface.header.tab-width", target.tab_width);
    pixels!("interface.panes.padding", target.pane_padding);
    pixels!("interface.panes.margin", target.pane_margin);
    pixels!("interface.panes.row-gap", target.row_gap);
    pixels!("interface.panes.column-gap", target.column_gap);
    pixels!("interface.panes.border-width", target.border_width);
    pixels!("interface.background.padding-top", target.padding_top);
    pixels!("interface.background.padding-bottom", target.padding_bottom);
    pixels!("interface.background.padding-left", target.padding_left);
    pixels!("interface.background.padding-right", target.padding_right);
    boolean!("interface.background.blur", target.blur);
    boolean!("interface.background.opacity-cells", target.opacity_cells);
    rgb!("interface.panes.border-color", target.border_color);
    field!("interface.background.opacity",target.opacity,SettingValue::Number(v) => OpacityPercent::new(*v as u8).map_err(|_| SettingsError::InvalidValue)?);
    field!("interface.panes.inactive-opacity",target.inactive_opacity,SettingValue::Number(v) => OpacityPercent::new(*v as u8).map_err(|_| SettingsError::InvalidValue)?);
    if edit.id.as_str().ends_with(".background-opacity") {
        let prefix = edit.id.as_str().trim_end_matches("-opacity");
        let rows = descriptors(base, &prefs.apply_to(base), prefs, palette)?;
        let entry = rows
            .iter()
            .find(|r| r.id.as_str() == prefix)
            .ok_or(SettingsError::UnknownSetting)?;
        let value = match &edit.change {
            Change::Reset => None,
            Change::Set(SettingValue::Number(value)) => {
                let SettingValue::Color(mut color) = entry.value else {
                    return Err(SettingsError::InvalidValue);
                };
                color[3] = (*value * 255.0 / 100.0).round() as u8;
                Some(Rgba::from_bytes(color))
            }
            _ => return Err(SettingsError::InvalidValue),
        };
        match prefix {
            "interface.footer.background" => target.appearance.footer.background = value,
            "interface.header.background" => target.appearance.header.background = value,
            _ => return Err(SettingsError::UnknownSetting),
        }
        return Ok(next);
    }
    Err(SettingsError::UnknownSetting)
}

pub(super) fn reset(prefs: &mut UserPreferences, id: &str) -> bool {
    match id {
        "interface.footer.visible" => {
            prefs.visual.interface.appearance.footer = FooterAppearance::default()
        }
        "interface.header.background" => {
            prefs.visual.interface.appearance.header = HeaderAppearance::default();
            prefs.visual.interface.tab_width = None;
        }
        "interface.panes.padding" => {
            let v = &mut prefs.visual.interface;
            v.pane_padding = None;
            v.pane_margin = None;
            v.row_gap = None;
            v.column_gap = None;
            v.border_width = None;
            v.border_color = None;
            v.inactive_opacity = None;
        }
        "interface.background.opacity" => {
            let v = &mut prefs.visual.interface;
            v.opacity = None;
            v.opacity_cells = None;
            v.blur = None;
            v.padding_top = None;
            v.padding_bottom = None;
            v.padding_left = None;
            v.padding_right = None;
        }
        _ => return false,
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interface_every_control_round_trips_and_reset_inherits_without_cross_edits() {
        let mut base = Config::default();
        base.presentation.interface.footer.height = UiPixels::new(40);
        base.margin.left = 13.0;
        let prefs = UserPreferences::default();
        let rows = descriptors(&base, &base, &prefs, &base.colors).unwrap();
        assert!(rows.len() >= 40);
        for row in rows {
            let value = match row.kind {
                settings::SettingKind::Boolean => {
                    SettingValue::Boolean(row.value != SettingValue::Boolean(true))
                }
                settings::SettingKind::Number { min, max, .. }
                | settings::SettingKind::ContinuousNumber { min, max, .. } => {
                    SettingValue::Number(if row.value == SettingValue::Number(min) {
                        max
                    } else {
                        min
                    })
                }
                settings::SettingKind::Color { alpha } => {
                    SettingValue::Color([60, 90, 120, if alpha { 128 } else { 255 }])
                }
                _ => panic!("unexpected appearance control"),
            };
            let mut edit = Edit {
                revision: 7,
                id: row.id.clone(),
                change: Change::Set(value),
            };
            let changed = apply_edit(7, &base, &prefs, &[], &edit).unwrap();
            assert_ne!(
                changed,
                prefs,
                "{} must edit a real preference",
                row.id.as_str()
            );
            assert_eq!(changed.presentation, prefs.presentation);
            assert_eq!(changed.fonts, prefs.fonts);
            assert_eq!(changed.visual.tags, prefs.visual.tags);
            edit.change = Change::Reset;
            assert_eq!(
                apply_edit(7, &base, &changed, &[], &edit).unwrap(),
                prefs,
                "{} reset",
                row.id.as_str()
            );
            edit.revision = 6;
            assert!(
                apply_edit(7, &base, &prefs, &[], &edit).is_err(),
                "reject stale edits"
            );
        }
    }

    #[test]
    fn interface_pages_have_one_home_and_keep_detail_controls_out_of_root_budget() {
        let base = Config::default();
        let prefs = UserPreferences::default();
        let full = catalog(7, &base, &prefs, &[]).unwrap();
        let groups = customization_groups(&full);
        for group in &groups {
            assert_ne!(
                CustomizationArea::Terminal.includes(group),
                CustomizationArea::Workflow.includes(group)
            );
        }
        for id in [
            settings::FONT_SIZE,
            settings::APPEARANCE_THEME,
            WINDOW_CONTROLS,
        ] {
            let group = groups.iter().find(|g| g.key.as_str() == id).unwrap();
            assert!(CustomizationArea::Terminal.includes(group));
        }
        assert_eq!(
            full.entries()
                .iter()
                .filter(|r| r.id.as_str().starts_with("interface."))
                .count(),
            4
        );
        let snapshot = slot_page_snapshot(&prefs, &base);
        let footer =
            interface_page_catalog(&full, &snapshot, "interface.footer.visible").unwrap();
        assert!(footer.entries().len() >= 18);
        let mut hidden = prefs.clone();
        hidden.visual.interface.appearance.footer.visible = Some(false);
        let snapshot = slot_page_snapshot(&hidden, &hidden.apply_to(&base));
        let page =
            interface_page_catalog(&full, &snapshot, "interface.footer.visible").unwrap();
        let visible = visible_controls(page, &full).unwrap();
        assert_eq!(visible.entries().len(), 1);
        assert_eq!(visible.entries()[0].id.as_str(), "interface.footer.visible");
    }

    #[test]
    fn interface_fractional_and_large_config_values_do_not_break_settings() {
        let mut base = Config::default();
        base.panel.padding.left = 2.5;
        base.margin.right = 150.0;
        base.presentation.interface.header.tab_gap = UiPixels::from_decimal(3.5);
        let prefs = UserPreferences::default();
        let full = catalog(7, &base, &prefs, &[]).unwrap();
        let rows = descriptors(&base, &base, &prefs, &base.colors).unwrap();
        let all = Catalog::new(7, rows).unwrap();
        let padding = all
            .get(&settings::SettingId::new("interface.panes.padding").unwrap())
            .unwrap();
        assert_eq!(padding.value, SettingValue::Number(2.5));
        let edit = Edit {
            revision: 7,
            id: padding.id.clone(),
            change: Change::Set(SettingValue::Number(3.5)),
        };
        let changed = apply_edit(7, &base, &prefs, &[], &edit).unwrap();
        assert_eq!(changed.apply_to(&base).panel.padding.left, 3.5);
        assert_eq!(changed.apply_to(&base).margin.right, 150.0);
        assert!(all
            .get(&settings::SettingId::new("interface.background.padding-right").unwrap())
            .unwrap()
            .availability
            .reason()
            .is_some());
        assert!(!full.entries().is_empty());
    }

    #[test]
    fn interface_ineffective_border_and_transparency_controls_hide_without_erasing() {
        let mut base = Config::default();
        base.presentation.interface.footer.border_width = UiPixels::new(0);
        let prefs = UserPreferences::default();
        let full = catalog(1, &base, &prefs, &[]).unwrap();
        let snapshot = slot_page_snapshot(&prefs, &base);
        for (page, hidden) in [
            ("interface.footer.visible", "interface.footer.border"),
            ("interface.background.opacity", "interface.background.blur"),
        ] {
            let all = interface_page_catalog(&full, &snapshot, page).unwrap();
            assert!(all.entries().iter().any(|row| row.id.as_str() == hidden));
            let visible = visible_controls(all, &full).unwrap();
            assert!(visible
                .entries()
                .iter()
                .all(|row| row.id.as_str() != hidden));
        }
    }

    #[test]
    fn interface_area_reset_preserves_workflow_and_color_favorites() {
        let mut prefs = UserPreferences::default();
        prefs.presentation.inline_tables = Some(false);
        prefs.visual.interface.appearance.footer.visible = Some(false);
        prefs.font_size = Some(19.0);
        prefs.remember_color([1, 2, 3, 255]);
        let terminal = reset_customizations(
            &prefs,
            &CustomizationResetScope::Area(CustomizationArea::Terminal),
            None,
        )
        .unwrap();
        assert!(terminal.visual.interface.is_empty());
        assert_eq!(terminal.presentation, prefs.presentation);
        assert_eq!(terminal.color_favorites, prefs.color_favorites);
        let workflow = reset_customizations(
            &prefs,
            &CustomizationResetScope::Area(CustomizationArea::Workflow),
            None,
        )
        .unwrap();
        assert_eq!(workflow.visual.interface, prefs.visual.interface);
        assert_eq!(workflow.font_size, prefs.font_size);
        assert_eq!(workflow.color_favorites, prefs.color_favorites);
    }
}
