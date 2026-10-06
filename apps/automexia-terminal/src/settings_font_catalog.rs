//! Font controls reuse standard settings editors and the existing font contract.
use super::*;
use crate::automexia::font_preferences::{
    parse_features, valid_family, FontColor, FontPreferences, MAX_FAMILY_BYTES,
    MAX_FEATURE_BYTES,
};
use rio_backend::sugarloaf::font::SugarloafFonts;

#[derive(Clone)]
pub(super) struct Snapshot {
    pub fonts: SugarloafFonts,
    pub line_height: f32,
    pub palette: Colors,
    base_fonts: SugarloafFonts,
    base_height: f32,
    base_palette: Colors,
    user: FontPreferences,
}
impl Snapshot {
    pub fn new(base: &Config, current: &Config, prefs: &UserPreferences) -> Self {
        Self {
            fonts: current.fonts.clone(),
            line_height: current.line_height,
            palette: current.colors,
            base_fonts: base.fonts.clone(),
            base_height: base.line_height,
            base_palette: base.colors,
            user: prefs.fonts.clone(),
        }
    }
}
pub(super) fn descriptors(
    snapshot: &Snapshot,
) -> Result<Vec<settings::SettingDescriptor>, SettingsError> {
    let current = &snapshot.fonts;
    let base = &snapshot.base_fonts;
    let user = &snapshot.user;
    let mut rows = Vec::new();
    let family = |fonts: &SugarloafFonts| {
        fonts
            .family
            .as_deref()
            .unwrap_or(&fonts.regular.family)
            .to_owned()
    };
    let mut row = visual_row(
        "fonts.family",
        "Font family".into(),
        settings::SettingKind::Text {
            max_bytes: MAX_FAMILY_BYTES,
            allow_empty: false,
        },
        SettingValue::Text(family(current)),
        SettingValue::Text(family(base)),
        visual_origin(user.family.is_some(), true, ValueOrigin::Default),
    )?;
    row.description = "Browse installed fonts with a live terminal preview. Enter applies; Escape restores the current font.".into();
    unavailable_unless(
        &mut row,
        valid_family(&family(current)) && valid_family(&family(base)),
    );
    rows.push(row);
    macro_rules! number {
        ($id:literal, $label:literal, $value:expr, $default:expr, $over:expr, $min:literal, $max:literal, $step:literal, $help:literal) => {{
            let value = f64::from($value);
            let default = f64::from($default);
            let mut row = visual_row(
                $id,
                $label.into(),
                settings::SettingKind::ContinuousNumber {
                    min: $min,
                    max: $max,
                    step: $step,
                },
                SettingValue::Number(value),
                SettingValue::Number(default),
                visual_origin($over, true, ValueOrigin::Default),
            )?;
            row.description = $help.into();
            unavailable_unless(
                &mut row,
                value.is_finite()
                    && default.is_finite()
                    && ($min..=$max).contains(&value)
                    && ($min..=$max).contains(&default),
            );
            rows.push(row);
        }};
    }
    number!(
        "fonts.regular-weight",
        "Regular weight",
        current.regular.weight.unwrap_or(400),
        base.regular.weight.unwrap_or(400),
        user.regular_weight.is_some(),
        100.0,
        900.0,
        100.0,
        "Face weight for regular and italic text; supported weights depend on the font."
    );
    number!(
        "fonts.bold-weight",
        "Bold weight",
        current.bold.weight.unwrap_or(700),
        base.bold.weight.unwrap_or(700),
        user.bold_weight.is_some(),
        100.0,
        900.0,
        100.0,
        "Face weight for bold and bold italic text."
    );
    number!(
        "fonts.line-height",
        "Line spacing",
        snapshot.line_height,
        snapshot.base_height,
        user.line_height.is_some(),
        0.8,
        3.0,
        0.1,
        "Row height multiplier. Values below 1 can make tall glyphs overlap."
    );
    macro_rules! boolean {
        ($id:literal, $label:literal, $value:expr, $default:expr, $over:expr, $help:literal) => {{
            let mut row = visual_row(
                $id,
                $label.into(),
                settings::SettingKind::Boolean,
                SettingValue::Boolean($value),
                SettingValue::Boolean($default),
                visual_origin($over, true, ValueOrigin::Default),
            )?;
            row.description = $help.into();
            rows.push(row);
        }};
    }
    boolean!(
        "fonts.bold-enabled",
        "Bold faces",
        !current.bold.style.is_disabled(),
        !base.bold.style.is_disabled(),
        user.bold_enabled.is_some(),
        "Allow bold faces requested by applications; off reuses the regular face."
    );
    boolean!(
        "fonts.italic-enabled",
        "Italic faces",
        !current.italic.style.is_disabled(),
        !base.italic.style.is_disabled(),
        user.italic_enabled.is_some(),
        "Allow italic faces requested by applications; off reuses the regular face."
    );
    boolean!(
        "fonts.ligatures",
        "Ligatures",
        ligatures(current),
        ligatures(base),
        user.ligatures.is_some(),
        "Join programming symbols with standard and contextual ligatures when supported."
    );
    boolean!(
        "fonts.hinting",
        "Font hinting",
        current.hinting,
        base.hinting,
        user.hinting.is_some(),
        "Align small glyphs to pixels where the platform renderer supports hinting."
    );
    boolean!(
        "fonts.drawable-chars",
        "Draw box characters",
        current.use_drawable_chars,
        base.use_drawable_chars,
        user.drawable_chars.is_some(),
        "Draw terminal box and block characters precisely; off uses the font glyphs."
    );
    // Show the editable feature list before the separate ligature override;
    // appending liga/calt must not push a valid draft beyond the editor limit.
    let features =
        |value: Option<&Vec<String>>| value.map_or(String::new(), |v| v.join(","));
    let current_features = features(user.features.as_ref().or(base.features.as_ref()));
    let base_features = features(base.features.as_ref());
    let mut row = visual_row(
        "fonts.features",
        "OpenType features".into(),
        settings::SettingKind::Text {
            max_bytes: MAX_FEATURE_BYTES,
            allow_empty: true,
        },
        SettingValue::Text(current_features.clone()),
        SettingValue::Text(base_features.clone()),
        visual_origin(
            user.features.is_some(),
            base.features.is_some(),
            ValueOrigin::Default,
        ),
    )?;
    row.description = "Comma-separated features, e.g. ss01=1, zero=1. The Ligatures choice overrides liga/calt.".into();
    unavailable_unless(
        &mut row,
        parse_features(&current_features).is_some()
            && parse_features(&base_features).is_some(),
    );
    rows.push(row);
    for key in FontColor::ALL {
        let bytes = |palette: &Colors| {
            key.get(palette)
                .map(|c| (c.clamp(0.0, 1.0) * 255.0).round() as u8)
        };
        let mut row = visual_row(
            &format!("fonts.colors.{}", key.id()),
            key.label().into(),
            settings::SettingKind::Color { alpha: false },
            SettingValue::Color(bytes(&snapshot.palette)),
            SettingValue::Color(bytes(&snapshot.base_palette)),
            visual_origin(user.colors.contains_key(key), true, ValueOrigin::Default),
        )?;
        row.description = match key {
            FontColor::Foreground => "Default terminal text; explicit application and highlighting colors remain in control.",
            FontColor::Background => "Terminal background color. Window transparency remains an appearance setting.",
            FontColor::BrightText => "Default foreground for bold or bright text.",
            FontColor::DimText => "Default foreground for dim text.",
            FontColor::SelectionText | FontColor::SelectionBackground => "Colors for selected terminal text.",
            FontColor::Cursor => "Terminal cursor color.",
            _ => "Terminal ANSI palette entry. Explicit custom output and Kubernetes colors remain separate.",
        }.into();
        rows.push(row);
    }
    for row in &mut rows {
        row.keywords = vec![
            "fonts family text size typography spacing".into(),
            "color palette weight ligatures rendering".into(),
        ];
    }
    Ok(rows)
}
fn ligatures(fonts: &SugarloafFonts) -> bool {
    !fonts.features.as_ref().is_some_and(|features| {
        features
            .iter()
            .any(|v| matches!(v.as_str(), "liga=0" | "calt=0"))
    })
}
fn unavailable_unless(row: &mut settings::SettingDescriptor, supported: bool) {
    if !supported {
        row.kind = settings::SettingKind::Action;
        row.value = SettingValue::Action;
        row.default = SettingValue::Action;
        row.availability = settings::Availability::Unavailable { reason: "Configured value is outside this editor's supported range; edit configuration.".into() };
    }
}

pub(super) fn apply(
    prefs: &mut UserPreferences,
    edit: &Edit,
) -> Result<bool, SettingsError> {
    if !edit.id.as_str().starts_with("fonts.") {
        return Ok(false);
    }
    let fonts = &mut prefs.fonts;
    macro_rules! boolean {
        ($field:ident) => {
            fonts.$field = match &edit.change {
                Change::Reset => None,
                Change::Set(SettingValue::Boolean(v)) => Some(*v),
                _ => return Err(SettingsError::InvalidValue),
            };
        };
    }
    match edit.id.as_str() {
        "fonts.family" => {
            fonts.family = match &edit.change {
                Change::Reset => None,
                Change::Set(SettingValue::Text(v)) if valid_family(v) => Some(v.clone()),
                _ => return Err(SettingsError::InvalidValue),
            }
        }
        "fonts.features" => {
            fonts.features = match &edit.change {
                Change::Reset => None,
                Change::Set(SettingValue::Text(v)) => {
                    Some(parse_features(v).ok_or(SettingsError::InvalidValue)?)
                }
                _ => return Err(SettingsError::InvalidValue),
            }
        }
        "fonts.regular-weight" | "fonts.bold-weight" => {
            let value = match &edit.change {
                Change::Reset => None,
                Change::Set(SettingValue::Number(v))
                    if v.is_finite()
                        && (100.0..=900.0).contains(v)
                        && v.fract() == 0.0 =>
                {
                    Some(*v as u16)
                }
                _ => return Err(SettingsError::InvalidValue),
            };
            if edit.id.as_str() == "fonts.regular-weight" {
                fonts.regular_weight = value;
            } else {
                fonts.bold_weight = value;
            }
        }
        "fonts.line-height" => {
            fonts.line_height = match &edit.change {
                Change::Reset => None,
                Change::Set(SettingValue::Number(v))
                    if v.is_finite() && (0.8..=3.0).contains(v) =>
                {
                    Some(*v as f32)
                }
                _ => return Err(SettingsError::InvalidValue),
            }
        }
        "fonts.bold-enabled" => {
            boolean!(bold_enabled);
        }
        "fonts.italic-enabled" => {
            boolean!(italic_enabled);
        }
        "fonts.ligatures" => {
            boolean!(ligatures);
        }
        "fonts.hinting" => {
            boolean!(hinting);
        }
        "fonts.drawable-chars" => {
            boolean!(drawable_chars);
        }
        id => {
            let key = id
                .strip_prefix("fonts.colors.")
                .and_then(FontColor::from_id)
                .ok_or(SettingsError::UnknownSetting)?;
            match &edit.change {
                Change::Reset => {
                    fonts.colors.remove(&key);
                }
                Change::Set(SettingValue::Color([r, g, b, 255])) => {
                    fonts.colors.insert(key, Rgb::from_bytes([*r, *g, *b]));
                }
                _ => return Err(SettingsError::InvalidValue),
            }
        }
    }
    if !fonts.is_valid() {
        return Err(SettingsError::InvalidValue);
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use automexia_ui_model::settings::SettingId;
    #[test]
    fn fonts_controls_edit_reset_and_preserve_unrelated_preferences() {
        let base = Config::default();
        let mut prefs = UserPreferences::default();
        prefs.presentation.kubernetes_highlighting = Some(true);
        let original = prefs.clone();
        let root = catalog(7, &base, &prefs, &[]).unwrap();
        let page = font_page_catalog(
            &root,
            &slot_page_snapshot_with_config(
                &prefs,
                &base,
                &base,
                &crate::settings_catalog::test_installed_extensions(),
            ),
        )
        .unwrap();
        assert_eq!(page.entries().len(), 34);
        for row in page.entries() {
            let value = match &row.kind {
                settings::SettingKind::Boolean => SettingValue::Boolean(false),
                settings::SettingKind::Color { .. } => {
                    SettingValue::Color([10, 20, 30, 255])
                }
                settings::SettingKind::ContinuousNumber { min, .. } => {
                    SettingValue::Number(*min)
                }
                settings::SettingKind::Number { min, .. } => SettingValue::Number(*min),
                settings::SettingKind::Text { .. } => SettingValue::Text(
                    if row.id.as_str() == "fonts.family" {
                        "Example Mono"
                    } else {
                        "ss01=1, zero=1"
                    }
                    .into(),
                ),
                kind => panic!("unexpected font control: {kind:?}"),
            };
            prefs = apply_edit(
                7,
                &base,
                &prefs,
                &[],
                &Edit {
                    revision: 7,
                    id: row.id.clone(),
                    change: Change::Set(value),
                },
            )
            .unwrap();
        }
        assert_eq!(prefs.fonts.family.as_deref(), Some("Example Mono"));
        assert_eq!(prefs.fonts.line_height, Some(0.8));
        assert_eq!(prefs.fonts.colors.len(), 23);
        assert_eq!(prefs.presentation, original.presentation);
        let root = tempfile::tempdir().unwrap();
        crate::automexia::preferences::write_to_root(root.path(), &prefs).unwrap();
        assert_eq!(
            crate::automexia::preferences::load_from_root(root.path()).preferences,
            prefs
        );
        for row in page.entries() {
            prefs = apply_edit(
                7,
                &base,
                &prefs,
                &[],
                &Edit {
                    revision: 7,
                    id: row.id.clone(),
                    change: Change::Reset,
                },
            )
            .unwrap();
        }
        assert_eq!(prefs, original);
    }

    #[test]
    fn fonts_invalid_edits_stale_revisions_and_transparency_are_rejected() {
        let base = Config::default();
        for (id, value) in [
            ("fonts.family", SettingValue::Text("../font.ttf".into())),
            ("fonts.family", SettingValue::Text("x".repeat(129))),
            ("fonts.features", SettingValue::Text("liga=1,liga=0".into())),
            ("fonts.regular-weight", SettingValue::Number(450.5)),
            ("fonts.line-height", SettingValue::Number(0.1)),
            (
                "fonts.colors.foreground",
                SettingValue::Color([1, 2, 3, 10]),
            ),
        ] {
            let edit = Edit {
                revision: 2,
                id: SettingId::new(id).unwrap(),
                change: Change::Set(value),
            };
            assert!(
                apply_edit(2, &base, &UserPreferences::default(), &[], &edit).is_err(),
                "{id}"
            );
        }
        let edit = Edit {
            revision: 1,
            id: SettingId::new("fonts.family").unwrap(),
            change: Change::Set(SettingValue::Text("Example Mono".into())),
        };
        assert_eq!(
            apply_edit(2, &base, &UserPreferences::default(), &[], &edit),
            Err(SettingsError::StaleRevision)
        );
    }

    #[test]
    fn fonts_full_feature_drafts_remain_editable_with_ligatures_and_group_reset_is_scoped(
    ) {
        let base = Config::default();
        let mut prefs = UserPreferences::default();
        prefs.visual.timestamps.bold = Some(false);
        prefs.fonts.features = Some((0..24).map(|n| format!("a{n:03}")).collect());
        prefs.fonts.ligatures = Some(false);
        prefs.font_size = Some(22.5);
        let page = font_settings_catalog(1, &base, &prefs, &base.colors).unwrap();
        let row = page
            .get(&SettingId::new("fonts.features").unwrap())
            .unwrap();
        assert!(matches!(row.kind, settings::SettingKind::Text { .. }));
        assert_eq!(
            row.value,
            SettingValue::Text(prefs.fonts.features.as_ref().unwrap().join(","))
        );
        let reset = reset_customizations(
            &prefs,
            &CustomizationResetScope::Group(SettingId::new(settings::FONT_SIZE).unwrap()),
            None,
        )
        .unwrap();
        assert!(reset.fonts.is_empty());
        assert!(reset.font_size.is_none());
        assert_eq!(reset.visual, prefs.visual);
    }
}
