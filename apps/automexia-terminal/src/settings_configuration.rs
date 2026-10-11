//! Bounded structured TOML exports. This module never opens or writes config.toml.
//! Exported fragments contain only the explicitly edited field. The existing
//! configuration loader validates candidates; UserPreferences owns runtime values.
use automexia_ui_model::settings::{
    Availability, Catalog, Change, ChangeScope, Edit, Section, SettingDescriptor,
    SettingId, SettingKind, SettingOwner, SettingValue, SettingsError,
};
use rio_backend::config::Config;

const LIMIT: usize = 2048;
struct Group {
    key: &'static str,
    label: &'static str,
    section: Section,
    editor_only: bool,
}
macro_rules! groups {
    ($( $key:literal, $label:literal, $section:ident, $editor:literal; )*) => {
        const GROUPS: &[Group] = &[$(Group { key: $key, label: $label,
            section: Section::$section, editor_only: $editor },)*];
    };
}
groups! {
    "navigation", "Advanced tab behavior", Window, false;
    "window", "Native window options", Window, false;
    "keyboard", "Keyboard profile and IME modifiers", Input, false;
    "title", "Window title template", Window, false;
    "renderer", "Renderer backend and filters", Advanced, false;
    "session-recovery", "Recovery prompt and exclusions", Terminal, false;
    "effects", "Cursor effects", Appearance, false;
    "developer", "Diagnostics and logging", Advanced, false;
    "hints", "Link hints", Input, true;
    "bindings", "Custom keyboard bindings", Input, true;
    "shell", "Default shell and argument array", Shell, true;
    "editor", "Configuration editor and arguments", Shell, true;
    "working-dir", "Default working directory", Shell, true;
    "use-fork", "Shell fork behavior", Shell, false;
    "env-vars", "Environment configuration", Shell, true;
    "option-as-alt", "macOS Option key behavior", Input, false;
    "platform", "Platform-specific overrides", Advanced, true;
    "margin", "Window margins", Window, false;
    "panel", "Panel geometry", Window, false;
    "fonts", "Advanced font faces and fallbacks", Appearance, true;
    "colors", "Complete theme palette", Appearance, true;
    "adaptive-theme", "Adaptive theme files", Appearance, true;
    "window.background-image", "Background image", Appearance, true;
    "config-file", "Open complete config.toml", Advanced, true;
}
fn configured_document(base: &Config) -> Result<toml::Value, SettingsError> {
    let mut fields = toml::map::Map::new();
    macro_rules! field {
        ($key:literal, $value:expr) => {
            fields.insert(
                $key.into(),
                toml::Value::try_from($value).map_err(|_| SettingsError::InvalidValue)?,
            );
        };
    }
    field!("cursor", &base.cursor);
    field!("navigation", &base.navigation);
    field!("window", &base.window);
    field!("keyboard", &base.keyboard);
    field!("title", &base.title);
    field!("renderer", &base.renderer);
    // The normal config serializer deliberately omits this startup selection.
    // Explicit export must retain it instead of silently hiding the control.
    if let Some(renderer) = fields
        .get_mut("renderer")
        .and_then(toml::Value::as_table_mut)
    {
        renderer.insert(
            "backend".into(),
            toml::Value::try_from(&base.renderer.backend)
                .map_err(|_| SettingsError::InvalidValue)?,
        );
    }
    field!("session-recovery", &base.session_recovery);
    field!("effects", &base.effects);
    field!("developer", &base.developer);
    field!("use-fork", base.use_fork);
    field!("option-as-alt", &base.option_as_alt);
    let edges = |margin: rio_backend::config::layout::Margin| {
        toml::Value::Array(
            [margin.top, margin.right, margin.bottom, margin.left]
                .into_iter()
                .map(|value| toml::Value::Float(f64::from(value)))
                .collect(),
        )
    };
    fields.insert("margin".into(), edges(base.margin));
    field!("panel", &base.panel);
    if let Some(panel) = fields.get_mut("panel").and_then(toml::Value::as_table_mut) {
        panel.insert("margin".into(), edges(base.panel.margin));
        panel.insert("padding".into(), edges(base.panel.padding));
    }
    Ok(toml::Value::Table(fields))
}
fn remove_path(document: &mut toml::Value, path: &str) {
    if let Some((head, rest)) = path.split_once('.') {
        if let Some(child) = document.get_mut(head) {
            remove_path(child, rest);
        }
    } else if let Some(table) = document.as_table_mut() {
        table.remove(path);
    }
}
fn residual(base: &Config) -> Result<toml::Value, SettingsError> {
    let mut document = configured_document(base)?;
    for path in crate::settings_general_catalog::paths() {
        remove_path(&mut document, path);
    }
    // These fields already have dedicated Appearance controls.
    for path in [
        "window.opacity",
        "window.opacity-cells",
        "window.blur",
        "window.decorations",
        "navigation.unfocused-split-opacity",
        "navigation.unfocused-split-fill",
        "navigation.max-tab-width",
        "window.background-image",
        // Legacy parser compatibility fields have no runtime consumers.
        "navigation.clickable",
        "navigation.use-terminal-title",
    ] {
        remove_path(&mut document, path);
    }
    Ok(document)
}
fn value_at(document: &toml::Value, key: &str) -> toml::Value {
    document
        .get(key)
        .cloned()
        .unwrap_or_else(|| toml::Value::String(String::new()))
}
fn editable_text(group: &Group, document: &toml::Value) -> Option<String> {
    if group.editor_only {
        return None;
    }
    let value = value_at(document, group.key).to_string();
    (value.len() <= LIMIT && !value.chars().any(char::is_control)).then_some(value)
}
pub(crate) fn descriptors(
    base: &Config,
) -> Result<Vec<SettingDescriptor>, SettingsError> {
    let document = residual(base)?;
    GROUPS
        .iter()
        .map(|group| {
            let text = editable_text(group, &document);
            let (kind, value, description) = if let Some(text) = text {
                (
                    SettingKind::Text {
                        max_bytes: LIMIT,
                        allow_empty: false,
                    },
                    SettingValue::Text(text),
                    "Edit TOML; export a patch without changing config.toml.",
                )
            } else {
                (
                    SettingKind::Action,
                    SettingValue::Action,
                    "Edit in config.toml; no values enter UI preferences.",
                )
            };
            Ok(SettingDescriptor {
                id: SettingId::new(format!("configuration.{}", group.key))?,
                owner: SettingOwner::Core,
                section: group.section,
                label: if group.editor_only {
                    group.label.into()
                } else {
                    format!("{} · TOML", group.label)
                },
                description: description.into(),
                keywords: vec![group.key.into(), "configuration".into(), "export".into()],
                default: value.clone(),
                value,
                kind,
                origin: crate::settings_catalog::setting_origin(
                    &base.setting_sources,
                    group.key,
                    false,
                ),
                availability: Availability::Available,
                scope: ChangeScope::ConfigOnly,
            })
        })
        .collect()
}
pub(crate) enum Operation {
    OpenEditor,
    Export(String),
}
fn merge(target: &mut toml::Value, patch: &toml::Value) {
    if let (Some(target), Some(patch)) = (target.as_table_mut(), patch.as_table()) {
        for (key, value) in patch {
            if let Some(current) = target.get_mut(key) {
                merge(current, value);
            } else {
                target.insert(key.clone(), value.clone());
            }
        }
    } else {
        *target = patch.clone();
    }
}
pub(crate) fn prepare(
    revision: u64,
    base: &Config,
    edit: &Edit,
) -> Result<Operation, SettingsError> {
    Catalog::new(revision, descriptors(base)?)?.validate_edit(edit)?;
    let key = edit
        .id
        .as_str()
        .strip_prefix("configuration.")
        .ok_or(SettingsError::UnknownSetting)?;
    let group = GROUPS
        .iter()
        .find(|g| g.key == key)
        .ok_or(SettingsError::UnknownSetting)?;
    if edit.change == Change::Activate {
        return Ok(Operation::OpenEditor);
    }
    if edit.change == Change::Reset {
        let document = residual(base)?;
        fn leaves(value: &toml::Value, prefix: &str, out: &mut Vec<String>) {
            if let Some(table) = value.as_table() {
                for (key, value) in table {
                    leaves(value, &format!("{prefix}.{key}"), out);
                }
            } else {
                out.push(prefix.into());
            }
        }
        let mut fields = Vec::new();
        leaves(&value_at(&document, key), key, &mut fields);
        return Ok(Operation::Export(format!("# To inherit, remove only these fields from config.toml:\n# {}\n# No settings were changed. Review platform overrides too.\n", fields.join(", "))));
    }
    let Change::Set(SettingValue::Text(input)) = &edit.change else {
        return Err(SettingsError::InvalidValue);
    };
    if group.editor_only || input.len() > LIMIT {
        return Err(SettingsError::InvalidValue);
    }
    let wrapper: toml::Value = toml::from_str(&format!("value = {input}"))
        .map_err(|_| SettingsError::InvalidValue)?;
    let table = wrapper.as_table().ok_or(SettingsError::InvalidValue)?;
    if table.len() != 1 {
        return Err(SettingsError::InvalidValue);
    }
    let value = table
        .get("value")
        .ok_or(SettingsError::InvalidValue)?
        .clone();
    // Do not allow a structural patch to overwrite a field with a dedicated UI owner.
    let mut only = toml::map::Map::new();
    only.insert(key.into(), value);
    let patch = toml::Value::Table(only);
    let allowed = residual(base)?;
    let shape = allowed.get(key).ok_or(SettingsError::InvalidValue)?;
    fn known_shape(value: &toml::Value, shape: &toml::Value) -> bool {
        if let (Some(value), Some(shape)) = (value.as_table(), shape.as_table()) {
            value.iter().all(|(key, value)| {
                shape
                    .get(key)
                    .is_some_and(|schema| known_shape(value, schema))
            })
        } else {
            std::mem::discriminant(value) == std::mem::discriminant(shape)
        }
    }
    if !known_shape(patch.get(key).ok_or(SettingsError::InvalidValue)?, shape) {
        return Err(SettingsError::InvalidValue);
    }
    let mut selected = toml::map::Map::new();
    selected.insert(key.into(), value_at(&configured_document(base)?, key));
    let mut candidate = toml::Value::Table(selected);
    merge(&mut candidate, &patch);
    let candidate =
        toml::to_string(&candidate).map_err(|_| SettingsError::InvalidValue)?;
    Config::parse(&candidate).map_err(|_| SettingsError::InvalidValue)?;
    let canonical =
        toml::to_string_pretty(&patch).map_err(|_| SettingsError::InvalidValue)?;
    Ok(Operation::Export(format!("# Automexia configuration patch; review and merge into config.toml.\n# Existing comments and unrelated settings have not been modified.\n{canonical}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn edit(value: &str) -> Edit {
        Edit {
            revision: 4,
            id: SettingId::new("configuration.renderer").unwrap(),
            change: Change::Set(SettingValue::Text(value.into())),
        }
    }
    #[test]
    fn export_is_structured_bounded_and_validated_without_changing_configuration() {
        let original = "# user comment\ncopy-on-select = true\n[cursor] # retain this order\nshape = 'block'\n";
        let base = Config::parse(original).unwrap();
        let Operation::Export(patch) =
            prepare(4, &base, &edit("{ strategy = 'events' }")).unwrap()
        else {
            panic!()
        };
        let parsed: toml::Value = toml::from_str(&patch).unwrap();
        assert_eq!(parsed["renderer"]["strategy"].as_str(), Some("events"));
        assert_eq!(parsed.as_table().unwrap().len(), 1);
        assert_eq!(base, Config::parse(original).unwrap());
        assert!(!patch.contains("copy-on-select"));
        for invalid in [
            "{ strategy = 'unknown' }",
            "{ use-cpu = true }",
            "{ typo = true }",
            "42",
            "{}\ninjected=true",
        ] {
            assert!(prepare(4, &base, &edit(invalid)).is_err(), "{invalid}");
        }
        assert!(prepare(5, &base, &edit("{ strategy = 'events' }")).is_err());
    }
    #[test]
    fn every_offered_export_round_trips_its_current_typed_value() {
        let base = Config::default();
        for row in descriptors(&base).unwrap() {
            if let SettingValue::Text(value) = row.value {
                assert!(
                    prepare(
                        4,
                        &base,
                        &Edit {
                            revision: 4,
                            id: row.id.clone(),
                            change: Change::Set(SettingValue::Text(value))
                        }
                    )
                    .is_ok(),
                    "{}",
                    row.id.as_str()
                );
            }
        }
    }
    #[test]
    fn serialized_root_fields_have_a_catalog_or_structural_owner() {
        let base = Config::default();
        let document = toml::Value::try_from(&base).unwrap();
        let dedicated = [
            "profiles",
            "theme",
            "line-height",
            "force-theme",
            "presentation",
            "scroll",
            "bell",
        ];
        for key in document.as_table().unwrap().keys() {
            let general = crate::settings_general_catalog::paths()
                .any(|path| path.split('.').next() == Some(key.as_str()));
            assert!(
                general
                    || dedicated.contains(&key.as_str())
                    || GROUPS.iter().any(|group| group.key == key),
                "new configuration field needs a Settings owner: {key}"
            );
        }
        // Optional/skipped serialized fields have explicit advanced entry points.
        for key in [
            "colors",
            "adaptive-theme",
            "working-dir",
            "window.background-image",
            "platform",
            "config-file",
        ] {
            assert!(GROUPS
                .iter()
                .any(|group| group.key == key && group.editor_only));
        }
    }
    #[test]
    fn legacy_navigation_noops_are_not_offered_as_active_controls() {
        let base = Config::default();
        let document = residual(&base).unwrap();
        assert!(document["navigation"].get("clickable").is_none());
        assert!(document["navigation"].get("use-terminal-title").is_none());
    }

    #[test]
    fn backend_and_structural_reset_exports_keep_other_owners_out() {
        let base = Config::default();
        let Operation::Export(patch) =
            prepare(4, &base, &edit("{ backend = 'wgpu' }")).unwrap()
        else {
            panic!()
        };
        assert_eq!(
            toml::from_str::<toml::Value>(&patch).unwrap()["renderer"]["backend"]
                .as_str(),
            Some("wgpu")
        );
        let Operation::Export(reset) = prepare(
            4,
            &base,
            &Edit {
                change: Change::Reset,
                ..edit("{}")
            },
        )
        .unwrap() else {
            panic!()
        };
        assert!(reset.contains("renderer.backend"));
        assert!(!reset.contains("use-cpu"));
        assert!(!reset.contains("disable-unfocused-render"));
        assert!(prepare(4, &base, &edit("{ backend = 'unknown' }")).is_err());
    }
    #[test]
    fn sensitive_launch_details_are_not_projected_or_exported() {
        let base = Config::parse("env-vars=['TOKEN=private-value']\n[shell]\nprogram='private-program'\nargs=['private-arg']").unwrap();
        let catalog = descriptors(&base).unwrap();
        assert!(!format!("{catalog:?}").contains("private-"));
        let mut attempt = edit("[]");
        attempt.id = SettingId::new("configuration.env-vars").unwrap();
        assert!(prepare(4, &base, &attempt).is_err());
        attempt.change = Change::Activate;
        assert!(matches!(
            prepare(4, &base, &attempt),
            Ok(Operation::OpenEditor)
        ));
    }
}
