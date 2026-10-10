//! Application preferences projected into the shared, effect-free settings catalogue.
use crate::automexia::{
    marketplace::MarketItem, package_customizations::PackageCustomizationPages,
    preferences::UserPreferences, settings_extensions,
};
#[path = "settings_font_catalog.rs"]
mod fonts;
#[path = "settings_interface_catalog.rs"]
mod interface;
#[path = "settings_table_catalog.rs"]
mod tables;
#[path = "settings_timestamp_catalog.rs"]
mod timestamps;
#[path = "settings_visibility.rs"]
mod visibility;
#[path = "settings_window_controls_catalog.rs"]
mod window_controls;
pub(crate) use visibility::visible_controls;
pub(crate) const WINDOW_CONTROLS: &str = "window-controls.style";

use automexia_ui_model::information_bar::BarContextAvailability;
use automexia_ui_model::settings::{
    self, Catalog, Change, CoreOrigins, CoreValues, Edit, SettingValue, SettingsError,
    ValueOrigin,
};
use rio_backend::config::{
    colors::Colors,
    presentation::{HighlightStyle, OpacityPercent, Rgb, Rgba, TagAppearance, TagStyle},
    Config,
};

#[derive(Clone)]
pub(crate) struct SlotPageSnapshot {
    interface: Result<Vec<settings::SettingDescriptor>, SettingsError>,
    font: fonts::Snapshot,
    bar: crate::automexia::preferences::InformationBarPreferences,
    context: BarContextAvailability,
    git_controls_available: bool,
    tags: TagAppearance,
    tables: rio_backend::config::presentation::TableAppearance,
    table_base: rio_backend::config::presentation::TableAppearance,
    table_user: rio_backend::config::presentation::TableAppearance,
    controls: rio_backend::config::presentation::WindowControlsAppearance,
    controls_base: rio_backend::config::presentation::WindowControlsAppearance,
    controls_user: rio_backend::config::presentation::WindowControlsAppearance,
    timestamps: rio_backend::config::presentation::TimestampAppearance,
    timestamp_base: rio_backend::config::presentation::TimestampAppearance,
    timestamp_user: rio_backend::config::presentation::TimestampAppearance,
    palette: Colors,
    foreground: [u8; 3],
    terminal_background: [f32; 4],
    terminal_foreground: [f32; 4],
    terminal_success: [f32; 4],
    terminal_failure: [f32; 4],
    terminal_neutral: [f32; 4],
}

impl SlotPageSnapshot {
    pub(crate) fn preview_window_controls(
        &self,
    ) -> (
        rio_backend::config::presentation::WindowControlsAppearance,
        Colors,
    ) {
        (self.controls, self.palette)
    }
    pub(crate) fn preview_fonts(
        &self,
    ) -> (&rio_backend::sugarloaf::font::SugarloafFonts, f32, Colors) {
        (&self.font.fonts, self.font.line_height, self.font.palette)
    }
    pub(crate) fn preview_timestamps(
        &self,
    ) -> (
        rio_backend::config::presentation::TimestampAppearance,
        Colors,
    ) {
        (self.timestamps, self.palette)
    }
    pub(crate) fn preview_tables(
        &self,
    ) -> (rio_backend::config::presentation::TableAppearance, Colors) {
        (self.tables, self.palette)
    }
    pub(crate) fn preview_devops_enabled(&self) -> bool {
        self.context.devops
    }

    pub(crate) fn preview_context(&self) -> BarContextAvailability {
        self.context
    }

    pub(crate) fn slot_available(&self, id: &str) -> bool {
        // Git's switch lives in its tag editor. Keep this one re-enable entry
        // while installed, but never after removal. Its preview stays hidden.
        (id == "git" && self.git_controls_available && !self.context.git)
            || self.bar.recipe().slot_available(id, self.context)
    }

    pub(crate) fn preview_recipe(
        &self,
    ) -> automexia_ui_model::information_bar::BarRecipe {
        self.bar.recipe()
    }

    pub(crate) fn preview_appearance(&self) -> TagAppearance {
        self.tags
    }

    pub(crate) fn preview_foreground(&self) -> [u8; 3] {
        self.foreground
    }

    pub(crate) fn preview_terminal_colors(&self) -> ([f32; 4], [f32; 4]) {
        (self.terminal_background, self.terminal_foreground)
    }

    pub(crate) fn preview_command_colors(&self) -> [[f32; 4]; 3] {
        [
            self.terminal_success,
            self.terminal_failure,
            self.terminal_neutral,
        ]
    }
}

pub(crate) fn slot_page_snapshot_with_config(
    preferences: &UserPreferences,
    effective: &Config,
    base: &Config,
    market: &[MarketItem],
) -> SlotPageSnapshot {
    // Reuse validated, bounded extension descriptors. A missing or invalid
    // inventory must never turn a saved On preference into admission.
    let extensions = settings_extensions::extension_settings(market, |id| {
        preferences.extension_feature_enabled(id)
    })
    .unwrap_or_default();
    let enabled = |id| {
        extensions.iter().any(|entry| {
            entry.id.as_str() == id
                && entry.value == SettingValue::Boolean(true)
                && entry.availability.reason().is_none()
        })
    };
    let [red, green, blue, _] = effective
        .colors
        .foreground
        .map(|channel| (channel.clamp(0.0, 1.0) * 255.0) as u8);
    SlotPageSnapshot {
        interface: interface::descriptors(
            base,
            effective,
            preferences,
            &effective.colors,
        ),
        font: fonts::Snapshot::new(base, effective, preferences),
        bar: preferences.visual.information_bar.clone(),
        context: BarContextAvailability {
            devops: enabled(settings_extensions::DEVOPS_CONTEXT_STATUS_ID),
            git: enabled(settings_extensions::DEVOPS_GIT_STATUS_ID),
        },
        git_controls_available: extensions.iter().any(|entry| {
            entry.id.as_str() == settings_extensions::DEVOPS_GIT_STATUS_ID
                && entry.availability.reason().is_none()
        }),
        tags: effective.presentation.tags,
        tables: effective.presentation.tables,
        table_base: base.presentation.tables,
        table_user: preferences.visual.tables,
        controls: effective.presentation.window_controls,
        controls_base: base.presentation.window_controls,
        controls_user: preferences.visual.window_controls,
        timestamps: effective.presentation.timestamps,
        timestamp_base: base.presentation.timestamps,
        timestamp_user: preferences.visual.timestamps,
        palette: effective.colors,
        foreground: [red, green, blue],
        terminal_background: effective.colors.background.0,
        terminal_foreground: effective.colors.foreground,
        terminal_success: effective.colors.green,
        terminal_failure: effective.colors.red,
        terminal_neutral: effective.colors.blue,
    }
}

#[cfg(test)]
pub(crate) fn slot_page_snapshot(
    preferences: &UserPreferences,
    effective: &Config,
) -> SlotPageSnapshot {
    slot_page_snapshot_with_config(
        preferences,
        effective,
        &Config::default(),
        &test_installed_extensions(),
    )
}

#[cfg(test)]
pub(crate) fn test_installed_extensions() -> [MarketItem; 1] {
    [MarketItem {
        id: crate::automexia::builtins::devops::ID.into(),
        name: "DevOps".into(),
        description: "Fixture".into(),
        installed: true,
    }]
}

pub(crate) fn interface_control_section(id: &str) -> Option<&'static str> {
    if !id.starts_with("interface.header.") {
        return None;
    }
    Some(match id.trim_start_matches("interface.header.") {
        "height" | "background" | "background-opacity" | "border" | "border-width" => {
            "Header"
        }
        _ => "Tabs",
    })
}

pub(crate) fn interface_page_catalog(
    full: &Catalog,
    snapshot: &SlotPageSnapshot,
    key: &str,
) -> Result<Catalog, SettingsError> {
    let prefix = key.rsplit_once('.').ok_or(SettingsError::UnknownSetting)?.0;
    let mut entries: Vec<_> = snapshot
        .interface
        .clone()?
        .into_iter()
        .filter(|row| row.id.as_str().starts_with(prefix))
        .collect();
    entries.sort_by_key(|row| interface_control_section(row.id.as_str()) == Some("Tabs"));
    Catalog::new(full.revision(), entries)
}

pub(crate) fn window_controls_page_catalog(
    full: &Catalog,
    snapshot: &SlotPageSnapshot,
) -> Result<Catalog, SettingsError> {
    Catalog::new(
        full.revision(),
        window_controls::descriptors(
            snapshot.controls_base,
            snapshot.controls,
            snapshot.controls_user,
            &snapshot.palette,
            snapshot.controls.style.unwrap_or_default(),
        )?,
    )
}

pub(crate) fn table_page_catalog(
    full: &Catalog,
    snapshot: &SlotPageSnapshot,
) -> Result<Catalog, SettingsError> {
    let mut rows = vec![full
        .get(&settings::SettingId::new(settings::INLINE_TABLES)?)
        .ok_or(SettingsError::UnknownSetting)?
        .clone()];
    rows.extend(tables::descriptors(
        &snapshot.table_base,
        &snapshot.tables,
        &snapshot.table_user,
        &snapshot.palette,
    )?);
    append_opacity_controls(&mut rows)?;
    Catalog::new(full.revision(), rows)
}

fn table_settings_catalog(
    revision: u64,
    base: &Config,
    preferences: &UserPreferences,
    palette: &Colors,
) -> Result<Catalog, SettingsError> {
    let mut effective = base.presentation.tables;
    preferences.visual.tables.overlay(&mut effective);
    let mut rows = tables::descriptors(
        &base.presentation.tables,
        &effective,
        &preferences.visual.tables,
        palette,
    )?;
    append_opacity_controls(&mut rows)?;
    Catalog::new(revision, rows)
}

pub(crate) fn font_page_catalog(
    full: &Catalog,
    snapshot: &SlotPageSnapshot,
) -> Result<Catalog, SettingsError> {
    let mut rows = vec![full
        .get(&settings::SettingId::new(settings::FONT_SIZE)?)
        .ok_or(SettingsError::UnknownSetting)?
        .clone()];
    rows[0].label = "Terminal font size".into();
    rows.extend(fonts::descriptors(&snapshot.font)?);
    Catalog::new(full.revision(), rows)
}
fn font_settings_catalog(
    revision: u64,
    base: &Config,
    preferences: &UserPreferences,
    palette: &Colors,
) -> Result<Catalog, SettingsError> {
    let mut effective = preferences.apply_to(base);
    effective.colors = *palette;
    Catalog::new(
        revision,
        fonts::descriptors(&fonts::Snapshot::new(base, &effective, preferences))?,
    )
}

pub(crate) fn timestamp_page_catalog(
    full: &Catalog,
    snapshot: &SlotPageSnapshot,
) -> Result<Catalog, SettingsError> {
    let mut rows = vec![full
        .get(&settings::SettingId::new(settings::COMMAND_TIMESTAMPS)?)
        .ok_or(SettingsError::UnknownSetting)?
        .clone()];
    rows[0].label = "Show date and time".into();
    rows[0].description =
        "Hide or show the clock; result controls below are independent.".into();
    rows.extend(timestamps::descriptors(
        &snapshot.timestamp_base,
        &snapshot.timestamps,
        &snapshot.timestamp_user,
        &snapshot.palette,
    )?);
    append_opacity_controls(&mut rows)?;
    Catalog::new(full.revision(), rows)
}

fn timestamp_settings_catalog(
    revision: u64,
    base: &Config,
    preferences: &UserPreferences,
    palette: &Colors,
) -> Result<Catalog, SettingsError> {
    let mut effective = base.presentation.timestamps;
    preferences.visual.timestamps.overlay(&mut effective);
    let mut rows = timestamps::descriptors(
        &base.presentation.timestamps,
        &effective,
        &preferences.visual.timestamps,
        palette,
    )?;
    append_opacity_controls(&mut rows)?;
    Catalog::new(revision, rows)
}

fn values(config: &Config) -> CoreValues {
    CoreValues {
        inline_tables: config.presentation.inline_tables,
        output_highlighting: config.presentation.output_highlighting,
        command_output_highlighting: config.presentation.command_output_highlighting,
        kubernetes_highlighting: config.presentation.kubernetes_highlighting,
        command_timestamps: config.presentation.command_timestamps,
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum CustomizationArea {
    #[default]
    Workflow,
    Terminal,
}
impl CustomizationArea {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Workflow => "Workflow & Output",
            Self::Terminal => "Terminal Appearance",
        }
    }
    pub(crate) fn includes(self, group: &CustomizationGroup) -> bool {
        let terminal = group.key.as_str().starts_with("interface.")
            || matches!(
                group.key.as_str(),
                settings::FONT_SIZE | settings::APPEARANCE_THEME | WINDOW_CONTROLS
            );
        terminal == (self == Self::Terminal)
    }
}

/// A navigation projection of one immutable settings snapshot. `root_action`
/// opens `members` in the Settings sheet; it is not a persisted setting edit.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct CustomizationGroup {
    pub(crate) key: settings::SettingId,
    pub(crate) label: String,
    pub(crate) members: Vec<settings::SettingId>,
    pub(crate) root_action: settings::SettingDescriptor,
}

/// A user-confirmed reset target from currently admitted catalogue pages.
/// Application owns live publication, persistence and the optional undo point.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum CustomizationResetScope {
    All,
    Area(CustomizationArea),
    Group(settings::SettingId),
    Tag(String),
    OutputSeverity(String),
    CommandOutputBand(String),
    KubernetesSeverity(String),
    Package(settings::SettingId),
    PackageFeature(settings::SettingId),
}

/// Clear only the selected customization overlays. Explicit core On values
/// make reset defaults independent of a user's configured feature switches;
/// optional extension/package features retain their declared defaults.
pub(crate) fn reset_customizations(
    current: &UserPreferences,
    scope: &CustomizationResetScope,
    packages: Option<&PackageCustomizationPages>,
) -> Result<UserPreferences, SettingsError> {
    use crate::automexia::presentation::{
        OUTPUT_BACKGROUND_BINDINGS, OUTPUT_COLOR_BINDINGS,
    };
    let mut next = current.clone();
    match scope {
        CustomizationResetScope::Area(CustomizationArea::Terminal) => {
            next.visual.interface = Default::default();
            next.visual.window_controls = Default::default();
            next.fonts = Default::default();
            next.font_size = None;
            next.theme_selection = None;
            next.appearance_theme = None;
        }
        CustomizationResetScope::Area(CustomizationArea::Workflow) => {
            next =
                reset_customizations(current, &CustomizationResetScope::All, packages)?;
            next.visual.interface = current.visual.interface.clone();
            next.visual.window_controls = current.visual.window_controls;
            next.fonts = current.fonts.clone();
            next.font_size = current.font_size;
            next.theme_selection = current.theme_selection.clone();
            next.appearance_theme = current.appearance_theme;
            next.color_favorites = current.color_favorites.clone();
        }
        CustomizationResetScope::All => {
            let shortcuts = std::mem::take(&mut next.shortcuts);
            next = UserPreferences::default();
            next.shortcuts = shortcuts;
            next.presentation.inline_tables = Some(true);
            next.presentation.output_highlighting = Some(true);
            next.presentation.command_output_highlighting = Some(true);
            next.presentation.kubernetes_highlighting = Some(true);
            next.presentation.command_timestamps = Some(true);
            next.visual.tags.enabled = Some(true);
        }
        CustomizationResetScope::Group(key) => match key.as_str() {
            crate::automexia::presentation::TAG_ENABLED => {
                next.visual.tags = Default::default();
                next.visual.tags.enabled = Some(true);
                next.visual.information_bar = Default::default();
                for id in [
                    settings_extensions::DEVOPS_CONTEXT_STATUS_ID,
                    settings_extensions::DEVOPS_GIT_STATUS_ID,
                ] {
                    next.reset_extension_feature(id)
                        .map_err(|_| SettingsError::InvalidValue)?;
                }
            }
            settings::OUTPUT_HIGHLIGHTING => {
                next.presentation.output_highlighting = Some(true);
                next.visual.highlight = Default::default();
            }
            settings::COMMAND_OUTPUT_HIGHLIGHTING => {
                next.presentation.command_output_highlighting = Some(true);
                next.presentation.output_highlighting = Some(true);
                next.visual.command_output = Default::default();
                next.visual.highlight = Default::default();
            }
            settings::KUBERNETES_HIGHLIGHTING => {
                next.presentation.kubernetes_highlighting = Some(true);
                next.visual.kubernetes = Default::default();
            }
            settings::INLINE_TABLES => {
                next.presentation.inline_tables = Some(true);
                next.visual.tables = Default::default();
            }
            settings::COMMAND_TIMESTAMPS => {
                next.presentation.command_timestamps = Some(true);
                next.visual.timestamps = Default::default();
            }
            settings::APPEARANCE_THEME => {
                next.appearance_theme = None;
                next.theme_selection = None;
            }
            WINDOW_CONTROLS => {
                next.visual.window_controls = Default::default();
            }
            settings::FONT_SIZE => {
                next.font_size = None;
                next.fonts = Default::default();
            }
            id if settings_extensions::is_known_boolean_feature(id) => next
                .reset_extension_feature(id)
                .map_err(|_| SettingsError::InvalidValue)?,
            id if interface::reset(&mut next, id) => {}
            _ => return Err(SettingsError::UnknownSetting),
        },
        CustomizationResetScope::Tag(slot_id) => {
            use automexia_ui_model::information_bar::{
                preset_recipe, role_from_id, validate_recipe,
            };
            let bar = &mut next.visual.information_bar;
            let mut recipe = bar.recipe();
            let default = default_slot(bar.preset.unwrap_or_default(), slot_id)
                .ok_or(SettingsError::UnknownSetting)?;
            // Standard tags remain available in the editor even when a valid
            // imported recipe omits them. Reset restores the same bounded slot
            // that an ordinary edit would create, without inventing custom tags.
            if role_from_id(slot_id).is_none()
                && !recipe.slots.iter().any(|slot| slot.id == *slot_id)
            {
                return Err(SettingsError::UnknownSetting);
            }
            let index = ensure_recipe_slot(&mut recipe, &default)?;
            recipe.slots[index] = default;
            if bar.use_custom {
                if let Some(position) = preset_recipe(bar.preset.unwrap_or_default())
                    .slots
                    .iter()
                    .position(|slot| slot.id == *slot_id)
                {
                    let slot = recipe.slots.remove(index);
                    recipe.slots.insert(position.min(recipe.slots.len()), slot);
                }
            }
            validate_recipe(&recipe).map_err(|_| SettingsError::InvalidValue)?;
            bar.source_drafts.remove(slot_id);
            if bar.use_custom {
                bar.custom_recipe = Some(recipe);
            }
            if slot_id == "git" {
                next.reset_extension_feature(settings_extensions::DEVOPS_GIT_STATUS_ID)
                    .map_err(|_| SettingsError::InvalidValue)?;
            }
        }
        CustomizationResetScope::OutputSeverity(severity) => {
            let color_id = format!("output.colors.{severity}");
            let background_id = format!("output.backgrounds.{severity}");
            let color = OUTPUT_COLOR_BINDINGS
                .iter()
                .find(|binding| binding.id == color_id)
                .ok_or(SettingsError::UnknownSetting)?;
            if !OUTPUT_BACKGROUND_BINDINGS
                .iter()
                .any(|binding| binding.id == background_id)
            {
                return Err(SettingsError::UnknownSetting);
            }
            (color.write)(&mut next.visual.highlight.colors, None);
            match background_id.as_str() {
                "output.backgrounds.error" => {
                    next.visual.highlight.error_background = None
                }
                "output.backgrounds.warning" => {
                    next.visual.highlight.warning_background = None
                }
                "output.backgrounds.success" => {
                    next.visual.highlight.success_background = None
                }
                "output.backgrounds.info" => next.visual.highlight.info_background = None,
                "output.backgrounds.debug" => {
                    next.visual.highlight.debug_background = None
                }
                _ => return Err(SettingsError::UnknownSetting),
            }
        }
        CustomizationResetScope::CommandOutputBand(kind) => match kind.as_str() {
            "success" => next.visual.command_output.success = None,
            "failure" => next.visual.command_output.failure = None,
            "neutral" => next.visual.command_output.neutral = None,
            _ => return Err(SettingsError::UnknownSetting),
        },
        CustomizationResetScope::KubernetesSeverity(severity) => {
            let color_id = format!("kubernetes.colors.{severity}");
            let background_id = format!("kubernetes.backgrounds.{severity}");
            let color = crate::automexia::presentation::KUBERNETES_COLOR_BINDINGS
                .iter()
                .find(|binding| binding.id == color_id)
                .ok_or(SettingsError::UnknownSetting)?;
            if !crate::automexia::presentation::KUBERNETES_BACKGROUND_BINDINGS
                .iter()
                .any(|binding| binding.id == background_id)
            {
                return Err(SettingsError::UnknownSetting);
            }
            (color.write)(&mut next.visual.kubernetes.colors, None);
            match severity.as_str() {
                "error" => next.visual.kubernetes.error_background = None,
                "warning" => next.visual.kubernetes.warning_background = None,
                "success" => next.visual.kubernetes.success_background = None,
                "info" => next.visual.kubernetes.info_background = None,
                "debug" => next.visual.kubernetes.debug_background = None,
                _ => return Err(SettingsError::UnknownSetting),
            }
        }
        CustomizationResetScope::Package(key) => {
            next.package_overrides = packages
                .ok_or(SettingsError::Unavailable)?
                .reset_package(&next.package_overrides, key)?;
        }
        CustomizationResetScope::PackageFeature(key) => {
            next.package_overrides = packages
                .ok_or(SettingsError::Unavailable)?
                .reset_feature(&next.package_overrides, key)?;
        }
    }
    Ok(next)
}

fn category_description(
    summary: &str,
    enabled: Option<bool>,
    unavailable: bool,
) -> String {
    let prefix = match (enabled, unavailable) {
        (Some(true), true) => "Currently on; unavailable. ",
        (Some(false), true) => "Currently off; unavailable. ",
        (None, true) => "Currently unavailable. ",
        (Some(true), false) => "Currently on. ",
        (Some(false), false) => "Currently off. ",
        (None, false) => return summary.into(),
    };
    if !unavailable
        && summary.len() > settings::MAX_DESCRIPTION_BYTES.saturating_sub(prefix.len())
    {
        return summary.into();
    }
    // Source descriptions are already catalog-validated. Trim only at a UTF-8
    // boundary so a maximum-sized extension description still fits the state.
    let mut end = summary
        .len()
        .min(settings::MAX_DESCRIPTION_BYTES.saturating_sub(prefix.len()));
    while !summary.is_char_boundary(end) {
        end -= 1;
    }
    format!("{prefix}{}", &summary[..end])
}

fn customization_group(
    catalog: &Catalog,
    label: &str,
    summary: &str,
    anchor: &str,
    enable_id: Option<&str>,
    belongs: impl Fn(&str) -> bool,
    keywords: &[&str],
) -> Option<CustomizationGroup> {
    let mut members = Vec::new();
    if let Some(enable_id) = enable_id {
        members.extend(
            catalog
                .entries()
                .iter()
                .filter(|entry| entry.id.as_str() == enable_id)
                .map(|entry| entry.id.clone()),
        );
    }
    members.extend(
        catalog
            .entries()
            .iter()
            .filter(|entry| entry.id.as_str() != enable_id.unwrap_or_default())
            .filter(|entry| belongs(entry.id.as_str()))
            .map(|entry| entry.id.clone()),
    );
    let key = members
        .iter()
        .find(|id| id.as_str() == anchor)
        .or_else(|| members.first())?
        .clone();
    let enabled = enable_id
        .and_then(|id| {
            catalog
                .entries()
                .iter()
                .find(|entry| entry.id.as_str() == id)
        })
        .and_then(|entry| match &entry.value {
            SettingValue::Boolean(value) => Some(*value),
            _ => None,
        });
    let unavailable = enable_id
        .and_then(|id| {
            catalog
                .entries()
                .iter()
                .find(|entry| entry.id.as_str() == id)
        })
        .or_else(|| catalog.get(&key))
        .is_some_and(|entry| entry.availability.reason().is_some());
    let root_action = settings::SettingDescriptor {
        id: key.clone(),
        owner: settings::SettingOwner::Core,
        section: settings::Section::Customizations,
        label: label.into(),
        description: category_description(summary, enabled, unavailable),
        keywords: keywords.iter().map(|keyword| (*keyword).into()).collect(),
        kind: settings::SettingKind::Action,
        value: SettingValue::Action,
        default: SettingValue::Action,
        origin: ValueOrigin::Default,
        // A disabled or unavailable feature can still be opened to see its
        // controls, retained preferences, and the actual availability reason.
        availability: settings::Availability::Available,
        scope: settings::ChangeScope::Immediate,
    };
    Some(CustomizationGroup {
        key,
        label: label.into(),
        members,
        root_action,
    })
}

/// Build a bounded category list from settings already admitted by the host.
/// In particular, an extension appears only while its installed feature row
/// exists in this inventory-qualified catalogue. No manifest capability or
/// package setting is synthesized here.
pub(crate) fn customization_groups(catalog: &Catalog) -> Vec<CustomizationGroup> {
    let mut groups = Vec::new();
    if let Some(mut group) = customization_group(
        catalog,
        "Information tags",
        "Edit each tag in the preview.",
        crate::automexia::presentation::TAG_ENABLED,
        Some(crate::automexia::presentation::TAG_ENABLED),
        |id| {
            id == crate::automexia::presentation::TAG_ENABLED
                || id == crate::automexia::presentation::TAG_FORMAT
                || id == "tags.add-slot"
                || id == "tags.bar-style"
                || id == "tags.bar-arrangement"
                || id == "tags.spacing"
                || id == "tags.style"
                || id == "tags.opacity"
        },
        &[
            "Production Operating system WSL Windows Git",
            "Kubernetes Docker Azure AWS Google Cloud",
            "Other cloud Terraform Environment User",
            "gcp unknown cloud",
            "shell opacity color style",
        ],
    ) {
        if let Some(context) = catalog.entries().iter().find(|entry| {
            entry.id.as_str() == settings_extensions::DEVOPS_CONTEXT_STATUS_ID
        }) {
            group.members.insert(1, context.id.clone());
        }
        groups.push(group);
    }
    if let Some(mut group) = customization_group(
        catalog,
        "Terminal output colors",
        "Edit command results, logs and informational output in the preview.",
        settings::COMMAND_OUTPUT_HIGHLIGHTING,
        Some(settings::COMMAND_OUTPUT_HIGHLIGHTING),
        |id| {
            matches!(
                id,
                settings::COMMAND_OUTPUT_HIGHLIGHTING
                    | "command_output.pulse"
                    | settings::OUTPUT_HIGHLIGHTING
                    | "output.style"
            )
        },
        &[
            "command result success failure neutral",
            "pulse completed command band",
            "recognized log path pwd notice Docker",
            "error",
            "warning",
            "success",
            "info debug",
            "background text color",
        ],
    ) {
        let state = |id| {
            catalog
                .entries()
                .iter()
                .find(|entry| entry.id.as_str() == id)
                .map_or("unavailable", |entry| match &entry.value {
                    SettingValue::Boolean(true) => "on",
                    SettingValue::Boolean(false) => "off",
                    _ => "unavailable",
                })
        };
        group.root_action.description = format!(
            "Commands {} · Highlights {}. Edit colors in the preview.",
            state(settings::COMMAND_OUTPUT_HIGHLIGHTING),
            state(settings::OUTPUT_HIGHLIGHTING),
        );
        groups.push(group);
    }
    if let Some(group) = customization_group(
        catalog,
        "Kubernetes status colors",
        "Edit Kubernetes status rows in the preview.",
        settings::KUBERNETES_HIGHLIGHTING,
        Some(settings::KUBERNETES_HIGHLIGHTING),
        |id| id == settings::KUBERNETES_HIGHLIGHTING || id == "kubernetes.style",
        &[
            "kubernetes pod readiness status",
            "running unknown crashloopbackoff",
            "error warning success info debug",
            "background text color",
        ],
    ) {
        groups.push(group);
    }
    for (label, summary, id, keywords) in [
        (
            "Inline tables",
            "Style borders, headers and alternating rows or columns.",
            settings::INLINE_TABLES,
            [
                "border dashed dotted double opacity",
                "header colors",
                "rows columns stripes zebra checkerboard",
                "wrap",
            ],
        ),
        (
            "Command timestamps",
            "Format and place date, time and command results.",
            settings::COMMAND_TIMESTAMPS,
            [
                "time date format",
                "position alignment",
                "clock timezone",
                "completion colors opacity",
            ],
        ),
    ] {
        if let Some(group) = customization_group(
            catalog,
            label,
            summary,
            id,
            Some(id),
            |row| {
                row == id || (id == settings::INLINE_TABLES && row.starts_with("tables."))
            },
            &keywords,
        ) {
            groups.push(group);
        }
    }
    for (id, label, summary) in interface::PAGES {
        let prefix = id.rsplit_once('.').map_or(id, |(prefix, _)| prefix);
        if let Some(group) = customization_group(
            catalog,
            label,
            summary,
            id,
            None,
            |row| row.starts_with(prefix),
            &["terminal interface appearance chrome"],
        ) {
            groups.push(group);
        }
    }
    for (label, summary, id, keywords) in [
        (
            "Theme",
            "Browse, preview and customize theme palettes.",
            settings::APPEARANCE_THEME,
            ["light", "dark", "system"],
        ),
        (
            "Window controls",
            "Choose a button style and customize its appearance.",
            WINDOW_CONTROLS,
            [
                "minimize maximize restore close",
                "buttons glass circles outline",
                "size spacing colors opacity",
            ],
        ),
        (
            "Fonts",
            "Customize font family, size, styles and colors.",
            settings::FONT_SIZE,
            [
                "family type weight",
                "text size spacing ligatures",
                "colors palette rendering",
            ],
        ),
    ] {
        if let Some(mut group) = customization_group(
            catalog,
            label,
            summary,
            id,
            None,
            |row| row == id,
            &keywords,
        ) {
            if id == settings::APPEARANCE_THEME {
                group.root_action.description = summary.into();
            }
            groups.push(group);
        }
    }
    if let Some(group) = customization_group(
        catalog,
        "Profiles",
        "Create, edit and open named terminals.",
        "profiles.open",
        None,
        |id| id == "profiles.open",
        &["shell executable arguments environment WSL SSH Work Personal"],
    ) {
        groups.push(group);
    }
    for entry in catalog.entries() {
        let settings::SettingOwner::Extension(owner) = &entry.owner else {
            continue;
        };
        if entry.section != settings::Section::Customizations {
            continue;
        }
        if matches!(
            entry.id.as_str(),
            settings_extensions::DEVOPS_CONTEXT_STATUS_ID
                | settings_extensions::DEVOPS_GIT_STATUS_ID
        ) {
            continue;
        }
        let enabled = match &entry.value {
            SettingValue::Boolean(value) => Some(*value),
            _ => None,
        };
        let root_action = settings::SettingDescriptor {
            id: entry.id.clone(),
            owner: settings::SettingOwner::Extension(owner.clone()),
            section: settings::Section::Customizations,
            label: entry.label.clone(),
            description: category_description(
                &entry.description,
                enabled,
                entry.availability.reason().is_some(),
            ),
            keywords: entry.keywords.clone(),
            kind: settings::SettingKind::Action,
            value: SettingValue::Action,
            default: SettingValue::Action,
            origin: entry.origin,
            availability: settings::Availability::Available,
            scope: settings::ChangeScope::Immediate,
        };
        groups.push(CustomizationGroup {
            key: entry.id.clone(),
            label: entry.label.clone(),
            members: vec![entry.id.clone()],
            root_action,
        });
    }
    // Each base group owns a validated catalogue member. Extra slot pages are
    // projected separately only while the Customizations sheet is open.
    groups
}

pub(crate) fn slot_page_actions(
    snapshot: &SlotPageSnapshot,
) -> Result<Vec<settings::SettingDescriptor>, SettingsError> {
    use automexia_ui_model::information_bar::{role_id, role_label, STANDARD_ROLES};
    let mut actions = Vec::with_capacity(16);
    let recipe = snapshot.bar.recipe();
    let specs = STANDARD_ROLES
        .into_iter()
        .map(|role| {
            (
                role_id(role).to_owned(),
                format!("Tag slot: {}", role_label(role)),
            )
        })
        .chain((1..=3).filter_map(|number| {
            let id = format!("custom-{number}");
            recipe
                .slots
                .iter()
                .any(|slot| slot.id == id)
                .then(|| (id, format!("Custom tag {number}")))
        }));
    for (slot_id, label) in specs {
        if !snapshot.slot_available(&slot_id) {
            continue;
        }
        let mut action = visual_row(
            &format!("tags.slot.{slot_id}.page"),
            label.clone(),
            settings::SettingKind::Action,
            SettingValue::Action,
            SettingValue::Action,
            ValueOrigin::Default,
        )?;
        action.description = "Edit this tag in the preview.".into();
        action.keywords = vec!["information bar tag slot icon text".into()];
        actions.push(action);
    }
    Ok(actions)
}

/// Contextual preview pages reuse the admitted descriptors and edit owner.
/// The stable ID is checked against the known semantic set before lookup so
/// a preview target cannot manufacture an arbitrary setting.
pub(crate) fn tag_role_color_catalog(
    full: &Catalog,
    role_id: &str,
) -> Result<Catalog, SettingsError> {
    let bindings = &crate::automexia::presentation::TAG_COLOR_BINDINGS;
    let binding = bindings
        .iter()
        .find(|binding| binding.id.strip_prefix("tags.colors.") == Some(role_id))
        .or_else(|| {
            let role = automexia_ui_model::information_bar::role_from_id(role_id)?;
            bindings.iter().find(|binding| binding.role == role)
        })
        .ok_or(SettingsError::UnknownSetting)?;
    let id = settings::SettingId::new(binding.id)?;
    let mut entry = full
        .get(&id)
        .cloned()
        .ok_or(SettingsError::UnknownSetting)?;
    entry.label = "Default role color".into();
    entry.description = "Fallback color for this role.".into();
    Catalog::new(full.revision(), vec![entry])
}

/// Build the controls for a tag selected in the live preview. The role color
/// remains a separate inherited default; the slot color affects this tag only.
pub(crate) fn selected_tag_catalog(
    full: &Catalog,
    snapshot: &SlotPageSnapshot,
    slot_id: &str,
) -> Result<Catalog, SettingsError> {
    use automexia_ui_model::information_bar::{
        preset_recipe, role_id, BarIconSource, BarTextSource,
    };

    if slot_id == "git" && snapshot.git_controls_available && !snapshot.context.git {
        let feature = full
            .entries()
            .iter()
            .find(|entry| entry.id.as_str() == settings_extensions::DEVOPS_GIT_STATUS_ID)
            .cloned()
            .ok_or(SettingsError::Unavailable)?;
        return Catalog::new(full.revision(), vec![feature]);
    }

    let mut entries = slot_page_catalog(full.revision(), snapshot, slot_id)?
        .entries()
        .to_vec();
    let recipe = snapshot.preview_recipe();
    let default_recipe = preset_recipe(snapshot.bar.preset.unwrap_or_default());
    let slot = recipe
        .slots
        .iter()
        .find(|slot| slot.id == slot_id)
        .or_else(|| default_recipe.slots.iter().find(|slot| slot.id == slot_id));
    let inherited_role = slot.and_then(|slot| match (&slot.text, &slot.icon) {
        (BarTextSource::Role(role), _) => Some(*role),
        (BarTextSource::None, BarIconSource::Role(role)) => Some(*role),
        _ => None,
    });
    if let Some(role) = inherited_role {
        let role_catalog = tag_role_color_catalog(full, role_id(role))?;
        entries.extend(role_catalog.entries().iter().cloned());
    }
    if slot_id == "git"
        || inherited_role == Some(automexia_extension_api::SegmentRole::Git)
    {
        if let Some(git_feature) = full
            .entries()
            .iter()
            .find(|entry| entry.id.as_str() == settings_extensions::DEVOPS_GIT_STATUS_ID)
        {
            entries.push(git_feature.clone());
        }
    }
    Catalog::new(full.revision(), entries)
}

pub(crate) fn output_severity_catalog(
    full: &Catalog,
    severity: &str,
) -> Result<Catalog, SettingsError> {
    if !["error", "warning", "success", "info", "debug"].contains(&severity) {
        return Err(SettingsError::UnknownSetting);
    }
    let entries = ["output.colors", "output.backgrounds", "output.opacity"]
        .into_iter()
        .map(|prefix| {
            let id = settings::SettingId::new(format!("{prefix}.{severity}"))?;
            full.get(&id).cloned().ok_or(SettingsError::UnknownSetting)
        })
        .collect::<Result<Vec<_>, _>>()?;
    Catalog::new(full.revision(), entries)
}

pub(crate) fn command_output_band_catalog(
    full: &Catalog,
    kind: &str,
) -> Result<Catalog, SettingsError> {
    if !["success", "failure", "neutral"].contains(&kind) {
        return Err(SettingsError::UnknownSetting);
    }
    let entries = ["command_output.backgrounds", "command_output.opacity"]
        .into_iter()
        .map(|prefix| {
            let id = settings::SettingId::new(format!("{prefix}.{kind}"))?;
            full.get(&id).cloned().ok_or(SettingsError::UnknownSetting)
        })
        .collect::<Result<Vec<_>, _>>()?;
    Catalog::new(full.revision(), entries)
}

pub(crate) fn kubernetes_severity_catalog(
    full: &Catalog,
    severity: &str,
) -> Result<Catalog, SettingsError> {
    if !["error", "warning", "success", "info", "debug"].contains(&severity) {
        return Err(SettingsError::UnknownSetting);
    }
    let entries = [
        "kubernetes.colors",
        "kubernetes.backgrounds",
        "kubernetes.opacity",
    ]
    .into_iter()
    .map(|prefix| {
        let id = settings::SettingId::new(format!("{prefix}.{severity}"))?;
        full.get(&id).cloned().ok_or(SettingsError::UnknownSetting)
    })
    .collect::<Result<Vec<_>, _>>()?;
    Catalog::new(full.revision(), entries)
}

/// Keep every navigation entry when optional descriptions and search terms would
/// push a valid source snapshot past the root catalogue's byte budget.
pub(crate) fn customization_root_catalog(
    source: &Catalog,
    groups: &[CustomizationGroup],
) -> Result<Catalog, SettingsError> {
    let revision = source.revision();
    let mut entries: Vec<_> = groups
        .iter()
        .map(|group| group.root_action.clone())
        .collect();
    match Catalog::new(revision, entries.clone()) {
        Ok(root) => return Ok(root),
        Err(SettingsError::Capacity) => {}
        Err(error) => return Err(error),
    }
    for entry in &mut entries {
        if matches!(entry.owner, settings::SettingOwner::Extension(_)) {
            entry.description.clear();
            entry.keywords.clear();
        }
    }
    match Catalog::new(revision, entries.clone()) {
        Ok(root) => return Ok(root),
        Err(SettingsError::Capacity) => {}
        Err(error) => return Err(error),
    }
    for entry in &mut entries {
        entry.description.clear();
        entry.keywords.clear();
    }
    Catalog::new(revision, entries)
}

#[cfg(test)]
pub(crate) fn catalog(
    revision: u64,
    base: &Config,
    preferences: &UserPreferences,
    market: &[MarketItem],
) -> Result<Catalog, SettingsError> {
    catalog_with_palette(revision, base, preferences, market, &base.colors)
}

pub(crate) fn catalog_with_palette(
    revision: u64,
    base: &Config,
    preferences: &UserPreferences,
    market: &[MarketItem],
    palette: &Colors,
) -> Result<Catalog, SettingsError> {
    let effective = preferences.apply_to(base);
    let origin = |value: Option<bool>| {
        if value.is_some() {
            ValueOrigin::User
        } else {
            ValueOrigin::Configuration
        }
    };
    let mut entries = settings::core_descriptors(
        values(&effective),
        values(base),
        CoreOrigins {
            inline_tables: origin(preferences.presentation.inline_tables),
            output_highlighting: origin(preferences.presentation.output_highlighting),
            command_output_highlighting: origin(
                preferences.presentation.command_output_highlighting,
            ),
            kubernetes_highlighting: origin(
                preferences.presentation.kubernetes_highlighting,
            ),
            command_timestamps: origin(preferences.presentation.command_timestamps),
        },
    );
    if let Some(highlighting) = entries
        .iter_mut()
        .find(|entry| entry.id.as_str() == settings::OUTPUT_HIGHLIGHTING)
    {
        highlighting.description = "Color detected status output.".into();
    }
    let theme = match preferences.appearance_theme {
        Some(rio_backend::config::theme::AppearanceTheme::Light) => {
            settings::AppearanceChoice::Light
        }
        Some(rio_backend::config::theme::AppearanceTheme::Dark) => {
            settings::AppearanceChoice::Dark
        }
        None => settings::AppearanceChoice::Configuration,
    };
    let mut appearance = settings::appearance_descriptors(settings::AppearanceValues {
        font_size: f64::from(effective.fonts.size),
        configured_font_size: f64::from(base.fonts.size),
        font_min: f64::from(crate::automexia::preferences::MIN_FONT_POINTS),
        font_max: f64::from(crate::automexia::preferences::MAX_FONT_POINTS),
        font_origin: if preferences.font_size.is_some() {
            ValueOrigin::User
        } else {
            ValueOrigin::Configuration
        },
        theme,
    });
    if let Some(row) = appearance
        .iter_mut()
        .find(|row| row.id.as_str() == settings::APPEARANCE_THEME)
    {
        row.description = match base.force_theme {
            Some(rio_backend::config::theme::AppearanceTheme::Light) => {
                "Configuration uses Light."
            }
            Some(rio_backend::config::theme::AppearanceTheme::Dark) => {
                "Configuration uses Dark."
            }
            None => "Configuration follows system appearance.",
        }
        .into();
        let adaptive = base
            .adaptive_colors
            .as_ref()
            .is_some_and(|colors| colors.light.is_some() && colors.dark.is_some());
        if !adaptive {
            row.availability = settings::Availability::Unavailable {
                reason: "Set adaptive Light and Dark themes in configuration.".into(),
            };
        }
    }
    entries.push(crate::settings_view::profiles_settings_entry()?);
    entries.extend(appearance);
    entries.extend(visual_descriptors(base, &effective, preferences, palette)?);
    entries.extend(
        interface::descriptors(base, &effective, preferences, palette)?
            .into_iter()
            .filter(|row| {
                interface::PAGES
                    .iter()
                    .any(|(id, _, _)| row.id.as_str() == *id)
            }),
    );
    entries.extend(
        settings_extensions::extension_settings(market, |id| {
            preferences.extension_feature_enabled(id)
        })
        .map_err(|_| SettingsError::InvalidDescriptor)?,
    );
    Catalog::new(revision, entries)
}

/// Validate against current membership and revision before publishing any override.
#[cfg(test)]
pub(crate) fn apply_edit(
    revision: u64,
    base: &Config,
    preferences: &UserPreferences,
    market: &[MarketItem],
    edit: &Edit,
) -> Result<UserPreferences, SettingsError> {
    apply_edit_with_palette(revision, base, preferences, market, edit, &base.colors)
}

pub(crate) fn apply_edit_with_palette(
    revision: u64,
    base: &Config,
    preferences: &UserPreferences,
    market: &[MarketItem],
    edit: &Edit,
    palette: &Colors,
) -> Result<UserPreferences, SettingsError> {
    if edit.id.as_str().starts_with("interface.") {
        Catalog::new(
            revision,
            interface::descriptors(
                base,
                &preferences.apply_to(base),
                preferences,
                palette,
            )?,
        )?
        .validate_edit(edit)?;
        return interface::apply(base, preferences, edit, palette);
    }
    if let Some(rest) = edit.id.as_str().strip_prefix("tags.slot.") {
        let (slot_id, field) =
            rest.split_once('.').ok_or(SettingsError::UnknownSetting)?;
        if field == "page" {
            return Err(SettingsError::InvalidValue);
        }
        let snapshot = slot_page_snapshot_with_config(
            preferences,
            &preferences.apply_to(base),
            base,
            market,
        );
        if !snapshot.slot_available(slot_id) {
            return Err(SettingsError::Unavailable);
        }
        slot_page_catalog(revision, &snapshot, slot_id)?.validate_edit(edit)?;
    } else if edit.id.as_str().starts_with("window-controls.") {
        window_controls::catalog(revision, base, preferences, palette, edit.id.as_str())?
            .validate_edit(edit)?;
        return window_controls::apply(base, preferences, edit, palette);
    } else if edit.id.as_str().starts_with("tables.") {
        table_settings_catalog(revision, base, preferences, palette)?
            .validate_edit(edit)?;
    } else if edit.id.as_str().starts_with("fonts.") {
        font_settings_catalog(revision, base, preferences, palette)?
            .validate_edit(edit)?;
    } else if edit.id.as_str().starts_with("timestamps.") {
        timestamp_settings_catalog(revision, base, preferences, palette)?
            .validate_edit(edit)?;
    } else {
        catalog_with_palette(revision, base, preferences, market, palette)?
            .validate_edit(edit)?;
    }
    // Opacity is an editor for the existing RGBA alpha byte, not a second
    // preference/schema. Resolve the visible palette before preserving RGB.
    let opacity_edit;
    let edit = if let Some((domain, status)) = edit
        .id
        .as_str()
        .split_once(".opacity.")
        .filter(|(domain, _)| {
            matches!(
                *domain,
                "command_output" | "output" | "kubernetes" | "tables" | "timestamps"
            )
        }) {
        let id = settings::SettingId::new(format!("{domain}.backgrounds.{status}"))?;
        let full = if domain == "tables" {
            table_settings_catalog(revision, base, preferences, palette)?
        } else if domain == "timestamps" {
            timestamp_settings_catalog(revision, base, preferences, palette)?
        } else {
            catalog_with_palette(revision, base, preferences, market, palette)?
        };
        let row = full.get(&id).ok_or(SettingsError::UnknownSetting)?;
        let (SettingValue::Color(mut color), SettingValue::Color(default)) =
            (&row.value, &row.default)
        else {
            return Err(SettingsError::InvalidValue);
        };
        color[3] = match edit.change {
            Change::Set(SettingValue::Number(percent)) => {
                (percent * 255.0 / 100.0).round() as u8
            }
            Change::Reset => default[3],
            _ => return Err(SettingsError::InvalidValue),
        };
        opacity_edit = Edit {
            revision,
            id,
            change: if matches!(edit.change, Change::Reset) && color == *default {
                Change::Reset
            } else {
                Change::Set(SettingValue::Color(color))
            },
        };
        &opacity_edit
    } else {
        edit
    };
    let mut candidate = preferences.clone();
    match edit.id.as_str() {
        settings::FONT_SIZE => {
            candidate.font_size = match edit.change {
                Change::Reset => None,
                Change::Set(SettingValue::Number(value)) => Some(value as f32),
                _ => return Err(SettingsError::InvalidValue),
            };
            return Ok(candidate);
        }
        settings::APPEARANCE_THEME => {
            use rio_backend::config::theme::AppearanceTheme;
            candidate.appearance_theme = match &edit.change {
                Change::Reset => None,
                Change::Set(SettingValue::Choice(value)) => match value.as_str() {
                    "configuration" => None,
                    "light" => Some(AppearanceTheme::Light),
                    "dark" => Some(AppearanceTheme::Dark),
                    _ => return Err(SettingsError::InvalidValue),
                },
                _ => return Err(SettingsError::InvalidValue),
            };
            return Ok(candidate);
        }
        _ => {}
    }
    if fonts::apply(&mut candidate, edit)?
        || apply_slot_edit(&mut candidate, edit)?
        || apply_visual_edit(&mut candidate, edit)?
    {
        return Ok(candidate);
    }
    let value = match edit.change {
        Change::Reset => None,
        Change::Set(SettingValue::Boolean(value)) => Some(value),
        _ => return Err(SettingsError::InvalidValue),
    };
    match edit.id.as_str() {
        settings::INLINE_TABLES => candidate.presentation.inline_tables = value,
        settings::OUTPUT_HIGHLIGHTING => {
            candidate.presentation.output_highlighting = value
        }
        settings::COMMAND_OUTPUT_HIGHLIGHTING => {
            candidate.presentation.command_output_highlighting = value
        }
        settings::KUBERNETES_HIGHLIGHTING => {
            candidate.presentation.kubernetes_highlighting = value
        }
        settings::COMMAND_TIMESTAMPS => candidate.presentation.command_timestamps = value,
        id if settings_extensions::is_known_boolean_feature(id) => {
            match value {
                Some(value) => candidate.set_extension_feature_enabled(id, value),
                None => candidate.reset_extension_feature(id),
            }
            .map_err(|_| SettingsError::InvalidValue)?;
        }
        _ => return Err(SettingsError::UnknownSetting),
    }
    Ok(candidate)
}

fn apply_slot_edit(
    candidate: &mut UserPreferences,
    edit: &Edit,
) -> Result<bool, SettingsError> {
    use automexia_ui_model::information_bar::{
        preset_recipe, role_from_id, validate_recipe, BarIconSource, BarLane,
        BarTextSource,
    };
    let Some(rest) = edit.id.as_str().strip_prefix("tags.slot.") else {
        return Ok(false);
    };
    let Some((slot_id, field)) = rest.split_once('.') else {
        return Err(SettingsError::UnknownSetting);
    };
    let custom_number = slot_id
        .strip_prefix("custom-")
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|number| (1..=3).contains(number));
    let role = role_from_id(slot_id)
        .or_else(|| custom_number.map(|_| automexia_extension_api::SegmentRole::User))
        .ok_or(SettingsError::UnknownSetting)?;
    let bar = &mut candidate.visual.information_bar;
    let mut remembered = bar.source_drafts.remove(slot_id).unwrap_or_default();
    let mut recipe = bar.recipe();
    let preset = preset_recipe(bar.preset.unwrap_or_default());
    let default = default_slot(bar.preset.unwrap_or_default(), slot_id)
        .ok_or(SettingsError::UnknownSetting)?;
    let index = ensure_recipe_slot(&mut recipe, &default)?;
    if field == "remove" {
        if custom_number.is_none() || !matches!(edit.change, Change::Activate) {
            return Err(SettingsError::InvalidValue);
        }
        recipe.slots.remove(index);
        bar.custom_recipe = Some(recipe);
        bar.use_custom = true;
        return Ok(true);
    }
    if field == "order" {
        let target = match &edit.change {
            Change::Reset => preset
                .slots
                .iter()
                .position(|slot| slot.id == slot_id)
                .unwrap_or(index),
            Change::Set(SettingValue::Number(value)) => {
                (*value as usize).saturating_sub(1)
            }
            _ => return Err(SettingsError::InvalidValue),
        }
        .min(recipe.slots.len() - 1);
        let slot = recipe.slots.remove(index);
        recipe.slots.insert(target, slot);
    } else {
        let slot = recipe
            .slots
            .get_mut(index)
            .ok_or(SettingsError::UnknownSetting)?;
        match (field, &edit.change) {
            ("enabled", Change::Reset) => slot.enabled = default.enabled,
            ("enabled", Change::Set(SettingValue::Boolean(value))) => {
                slot.enabled = *value
            }
            ("text", Change::Reset) => {
                remembered.literal = None;
                remembered.text_role = None;
                slot.text = default.text.clone();
                if matches!(slot.text, BarTextSource::None)
                    && !matches!(
                        slot.icon,
                        BarIconSource::Role(_) | BarIconSource::Fixed(_)
                    )
                {
                    remembered.coerced_icon = Some(slot.icon.clone());
                    slot.icon = default.icon.clone();
                }
                if matches!(slot.text, BarTextSource::None) {
                    slot.prefix.clear();
                    slot.suffix.clear();
                }
            }
            ("text", Change::Set(SettingValue::Choice(value))) => {
                match &slot.text {
                    BarTextSource::Literal(value) => {
                        remembered.literal = Some(value.clone())
                    }
                    BarTextSource::Role(role) => remembered.text_role = Some(*role),
                    BarTextSource::None => {}
                }
                slot.text = if value == "literal" {
                    BarTextSource::Literal(
                        remembered
                            .literal
                            .clone()
                            .unwrap_or_else(|| "Custom label".into()),
                    )
                } else if value == "none" {
                    if !matches!(
                        slot.icon,
                        BarIconSource::Role(_) | BarIconSource::Fixed(_)
                    ) {
                        remembered.coerced_icon = Some(slot.icon.clone());
                        slot.icon = BarIconSource::Role(role);
                    }
                    slot.prefix.clear();
                    slot.suffix.clear();
                    BarTextSource::None
                } else {
                    let selected =
                        role_from_id(value).ok_or(SettingsError::InvalidValue)?;
                    remembered.text_role = Some(selected);
                    BarTextSource::Role(selected)
                };
            }
            ("literal", Change::Reset) => {
                remembered.literal = None;
                slot.text = default.text.clone();
                if matches!(slot.text, BarTextSource::None)
                    && !matches!(
                        slot.icon,
                        BarIconSource::Role(_) | BarIconSource::Fixed(_)
                    )
                {
                    remembered.coerced_icon = Some(slot.icon.clone());
                    slot.icon = default.icon.clone();
                }
                if matches!(slot.text, BarTextSource::None) {
                    slot.prefix.clear();
                    slot.suffix.clear();
                }
            }
            ("literal", Change::Set(SettingValue::Text(value))) => {
                remembered.literal = Some(value.clone());
                slot.text = BarTextSource::Literal(value.clone())
            }
            ("icon", Change::Reset) => {
                remembered.icon_role = None;
                remembered.fixed_icon = None;
                remembered.coerced_icon = None;
                slot.icon = default.icon.clone();
                if matches!(slot.text, BarTextSource::None)
                    && !matches!(
                        slot.icon,
                        BarIconSource::Role(_) | BarIconSource::Fixed(_)
                    )
                {
                    slot.text = default.text.clone();
                }
            }
            ("icon", Change::Set(SettingValue::Choice(value))) => {
                remembered.coerced_icon = None;
                match slot.icon {
                    BarIconSource::Role(role) => remembered.icon_role = Some(role),
                    BarIconSource::Fixed(icon) => remembered.fixed_icon = Some(icon),
                    BarIconSource::None | BarIconSource::TextSource => {}
                }
                slot.icon = match value.as_str() {
                    "text-source" => BarIconSource::TextSource,
                    "context" => {
                        BarIconSource::Role(remembered.icon_role.unwrap_or(role))
                    }
                    "fixed" => BarIconSource::Fixed(
                        remembered
                            .fixed_icon
                            .unwrap_or(automexia_extension_api::IconKind::Windows),
                    ),
                    "none" => BarIconSource::None,
                    _ => return Err(SettingsError::InvalidValue),
                };
                if matches!(slot.text, BarTextSource::None)
                    && !matches!(
                        slot.icon,
                        BarIconSource::Role(_) | BarIconSource::Fixed(_)
                    )
                {
                    slot.text = BarTextSource::Role(role);
                }
            }
            ("icon-context", Change::Reset) => {
                remembered.icon_role = None;
                remembered.coerced_icon = None;
                slot.icon = BarIconSource::Role(role);
            }
            ("icon-context", Change::Set(SettingValue::Choice(value))) => {
                remembered.coerced_icon = None;
                let selected = role_from_id(value).ok_or(SettingsError::InvalidValue)?;
                remembered.icon_role = Some(selected);
                slot.icon = BarIconSource::Role(selected);
            }
            ("icon-fixed", Change::Reset) => {
                remembered.fixed_icon = None;
                remembered.coerced_icon = None;
                slot.icon =
                    BarIconSource::Fixed(automexia_extension_api::IconKind::Windows)
            }
            ("icon-fixed", Change::Set(SettingValue::Choice(value))) => {
                remembered.coerced_icon = None;
                let icon = FIXED_ICONS
                    .iter()
                    .find(|entry| entry.0 == value)
                    .map(|entry| entry.2)
                    .ok_or(SettingsError::InvalidValue)?;
                remembered.fixed_icon = Some(icon);
                slot.icon = BarIconSource::Fixed(icon);
            }
            ("lane", Change::Reset) => slot.lane = default.lane,
            ("lane", Change::Set(SettingValue::Choice(value))) => {
                slot.lane = match value.as_str() {
                    "leading" => BarLane::Leading,
                    "trailing" => BarLane::Trailing,
                    _ => return Err(SettingsError::InvalidValue),
                };
            }
            ("color", Change::Reset) => slot.color = None,
            ("color", Change::Set(SettingValue::Color([red, green, blue, 255]))) => {
                slot.color = Some([*red, *green, *blue])
            }
            ("prefix", Change::Reset) => slot.prefix = default.prefix.clone(),
            ("prefix", Change::Set(SettingValue::Text(value))) => {
                slot.prefix = value.clone()
            }
            ("suffix", Change::Reset) => slot.suffix = default.suffix.clone(),
            ("suffix", Change::Set(SettingValue::Text(value))) => {
                slot.suffix = value.clone()
            }
            _ => return Err(SettingsError::InvalidValue),
        }
        if matches!(field, "text" | "literal")
            && !matches!(slot.text, BarTextSource::None)
        {
            if let Some(icon) = remembered.coerced_icon.take() {
                slot.icon = icon;
            }
        }
    }
    validate_recipe(&recipe).map_err(|_| SettingsError::InvalidValue)?;
    bar.custom_recipe = Some(recipe);
    bar.use_custom = true;
    if !remembered.is_empty() {
        bar.source_drafts.insert(slot_id.to_owned(), remembered);
    }
    Ok(true)
}

fn visual_row(
    id: &str,
    label: String,
    kind: settings::SettingKind,
    value: SettingValue,
    default: SettingValue,
    origin: ValueOrigin,
) -> Result<settings::SettingDescriptor, SettingsError> {
    Ok(settings::SettingDescriptor {
        id: settings::SettingId::new(id)?,
        owner: settings::SettingOwner::Core,
        section: settings::Section::Customizations,
        label,
        description: "Reset uses configured appearance.".into(),
        keywords: vec!["colors appearance style".into()],
        kind,
        value,
        default,
        origin,
        availability: settings::Availability::Available,
        scope: settings::ChangeScope::Immediate,
    })
}

fn visual_origin(user: bool, configured: bool, inherited: ValueOrigin) -> ValueOrigin {
    if user {
        ValueOrigin::User
    } else if configured {
        ValueOrigin::Configuration
    } else {
        inherited
    }
}

fn choice_kind(values: &[(&str, &str)]) -> settings::SettingKind {
    settings::SettingKind::Choice {
        options: values
            .iter()
            .map(|(value, label)| settings::ChoiceOption {
                value: (*value).into(),
                label: (*label).into(),
            })
            .collect(),
    }
}

fn role_choices(availability: BarContextAvailability) -> Vec<settings::ChoiceOption> {
    automexia_ui_model::information_bar::STANDARD_ROLES
        .into_iter()
        .filter(|role| availability.role_available(*role))
        .map(|role| settings::ChoiceOption {
            value: automexia_ui_model::information_bar::role_id(role).into(),
            label: automexia_ui_model::information_bar::role_label(role).into(),
        })
        .collect()
}

const FIXED_ICONS: [(&str, &str, automexia_extension_api::IconKind); 10] = [
    ("wsl", "WSL", automexia_extension_api::IconKind::Wsl),
    (
        "windows",
        "Windows",
        automexia_extension_api::IconKind::Windows,
    ),
    (
        "docker",
        "Docker",
        automexia_extension_api::IconKind::Docker,
    ),
    (
        "kubernetes",
        "Kubernetes",
        automexia_extension_api::IconKind::Kubernetes,
    ),
    ("cloud", "Cloud", automexia_extension_api::IconKind::Cloud),
    (
        "terraform",
        "Terraform",
        automexia_extension_api::IconKind::Terraform,
    ),
    ("git", "Git", automexia_extension_api::IconKind::Git),
    (
        "environment",
        "Environment",
        automexia_extension_api::IconKind::Environment,
    ),
    ("user", "User", automexia_extension_api::IconKind::User),
    (
        "production",
        "Production",
        automexia_extension_api::IconKind::Production,
    ),
];

fn fixed_icon_id(icon: automexia_extension_api::IconKind) -> &'static str {
    FIXED_ICONS
        .iter()
        .find(|entry| entry.2 == icon)
        .map_or("windows", |entry| entry.0)
}

fn custom_slot(number: usize) -> automexia_ui_model::information_bar::BarSlot {
    use automexia_ui_model::information_bar::{
        BarIconSource, BarLane, BarSlot, BarTextSource,
    };
    BarSlot {
        id: format!("custom-{number}"),
        enabled: true,
        text: BarTextSource::Literal("Custom label".into()),
        icon: BarIconSource::None,
        lane: BarLane::Leading,
        color: None,
        prefix: String::new(),
        suffix: String::new(),
    }
}

fn ensure_recipe_slot(
    recipe: &mut automexia_ui_model::information_bar::BarRecipe,
    default: &automexia_ui_model::information_bar::BarSlot,
) -> Result<usize, SettingsError> {
    if let Some(index) = recipe.slots.iter().position(|slot| slot.id == default.id) {
        return Ok(index);
    }
    if recipe.slots.len() >= automexia_ui_model::information_bar::MAX_BAR_SLOTS {
        return Err(SettingsError::Capacity);
    }
    recipe.slots.push(default.clone());
    Ok(recipe.slots.len() - 1)
}

fn default_slot(
    preset: automexia_ui_model::information_bar::InformationBarPreset,
    slot_id: &str,
) -> Option<automexia_ui_model::information_bar::BarSlot> {
    use automexia_ui_model::information_bar::{preset_recipe, InformationBarPreset};
    let custom_number = slot_id
        .strip_prefix("custom-")
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|number| (1..=3).contains(number));
    preset_recipe(preset)
        .slots
        .into_iter()
        .find(|slot| slot.id == slot_id)
        .or_else(|| {
            // Keep migrated/custom recipes editable if a later preset revision
            // no longer lists one of their canonical tags.
            preset_recipe(InformationBarPreset::RoundedCapsules)
                .slots
                .into_iter()
                .find(|slot| slot.id == slot_id)
        })
        .or_else(|| custom_number.map(custom_slot))
}

pub(crate) fn slot_page_catalog(
    revision: u64,
    snapshot: &SlotPageSnapshot,
    slot_id: &str,
) -> Result<Catalog, SettingsError> {
    if !snapshot
        .preview_recipe()
        .slot_available(slot_id, snapshot.preview_context())
    {
        return Err(SettingsError::Unavailable);
    }
    let rows = slot_descriptors(snapshot, slot_id)?;
    if rows.is_empty() {
        return Err(SettingsError::UnknownSetting);
    }
    Catalog::new(revision, rows)
}

fn slot_descriptors(
    snapshot: &SlotPageSnapshot,
    only_slot: &str,
) -> Result<Vec<settings::SettingDescriptor>, SettingsError> {
    use automexia_ui_model::information_bar::{
        preset_recipe, role_id, role_label, BarIconSource, BarLane, BarTextSource,
        MAX_BAR_AFFIX_BYTES, MAX_BAR_LITERAL_BYTES, STANDARD_ROLES,
    };
    let chosen = &snapshot.bar;
    let recipe = chosen.recipe();
    let default_recipe = preset_recipe(chosen.preset.unwrap_or_default());
    let mut rows = Vec::with_capacity(12);
    let slot_specs = STANDARD_ROLES
        .into_iter()
        .map(|role| {
            (
                role_id(role).to_owned(),
                role_label(role).to_owned(),
                role,
                None,
            )
        })
        .chain((1..=3).filter_map(|number| {
            let id = format!("custom-{number}");
            recipe.slots.iter().any(|slot| slot.id == id).then(|| {
                (
                    id,
                    format!("Custom tag {number}"),
                    automexia_extension_api::SegmentRole::User,
                    Some(number),
                )
            })
        }));
    for (id, slot_label, role, custom_number) in slot_specs {
        if id != only_slot {
            continue;
        }
        let prefix = format!("tags.slot.{id}.");
        let default = default_slot(chosen.preset.unwrap_or_default(), &id)
            .ok_or(SettingsError::InvalidDescriptor)?;
        let slot = recipe
            .slots
            .iter()
            .find(|slot| slot.id == id)
            .unwrap_or(&default);
        let position = recipe
            .slots
            .iter()
            .position(|slot| slot.id == id)
            .or_else(|| default_recipe.slots.iter().position(|slot| slot.id == id))
            .ok_or(SettingsError::InvalidDescriptor)?
            + 1;
        let default_position = default_recipe
            .slots
            .iter()
            .position(|slot| slot.id == id)
            .map(|index| index + 1)
            .unwrap_or(position);
        let origin = if chosen.use_custom {
            ValueOrigin::User
        } else {
            ValueOrigin::Default
        };
        let mut add = |field: &str,
                       label: &str,
                       description: &str,
                       kind: settings::SettingKind,
                       value: SettingValue,
                       default_value: SettingValue| {
            let mut row = visual_row(
                &format!("{prefix}{field}"),
                format!("{slot_label} {label}"),
                kind,
                value,
                default_value,
                origin,
            )?;
            row.description = description.into();
            row.keywords = vec!["information bar slot icon text order".into()];
            rows.push(row);
            Ok::<(), SettingsError>(())
        };
        add(
            "enabled",
            "enabled",
            "Show when detected; keep choices when off.",
            settings::SettingKind::Boolean,
            SettingValue::Boolean(slot.enabled),
            SettingValue::Boolean(default.enabled),
        )?;
        let mut text_options = role_choices(snapshot.preview_context());
        text_options.push(settings::ChoiceOption {
            value: "literal".into(),
            label: "Custom text".into(),
        });
        text_options.push(settings::ChoiceOption {
            value: "none".into(),
            label: "Icon only".into(),
        });
        let text_value = |text: &BarTextSource| match text {
            BarTextSource::Role(role) => role_id(*role),
            BarTextSource::Literal(_) => "literal",
            BarTextSource::None => "none",
        };
        add(
            "text",
            "text source",
            "Context, custom text, or icon only.",
            settings::SettingKind::Choice {
                options: text_options,
            },
            SettingValue::Choice(text_value(&slot.text).into()),
            SettingValue::Choice(text_value(&default.text).into()),
        )?;
        let literal = |text: &BarTextSource| match text {
            BarTextSource::Literal(value) => value.clone(),
            _ => "Custom label".into(),
        };
        add(
            "literal",
            "custom text",
            "Custom label; selects it as tag text.",
            settings::SettingKind::Text {
                max_bytes: MAX_BAR_LITERAL_BYTES,
                allow_empty: false,
            },
            SettingValue::Text(literal(&slot.text)),
            SettingValue::Text(literal(&default.text)),
        )?;
        let icon_mode = |icon: &BarIconSource| match icon {
            BarIconSource::TextSource => "text-source",
            BarIconSource::Role(_) => "context",
            BarIconSource::Fixed(_) => "fixed",
            BarIconSource::None => "none",
        };
        add(
            "icon",
            "icon source",
            "Choose the icon source.",
            choice_kind(&[
                ("text-source", "Text source"),
                ("context", "Other context"),
                ("fixed", "Fixed icon"),
                ("none", "No icon"),
            ]),
            SettingValue::Choice(icon_mode(&slot.icon).into()),
            SettingValue::Choice(icon_mode(&default.icon).into()),
        )?;
        let icon_role = |icon: &BarIconSource| match icon {
            BarIconSource::Role(role) => *role,
            _ => role,
        };
        add(
            "icon-context",
            "icon context",
            "Icon from another context.",
            settings::SettingKind::Choice {
                options: role_choices(snapshot.preview_context()),
            },
            SettingValue::Choice(role_id(icon_role(&slot.icon)).into()),
            SettingValue::Choice(role_id(icon_role(&default.icon)).into()),
        )?;
        let icon_fixed = |icon: &BarIconSource| match icon {
            BarIconSource::Fixed(icon) => *icon,
            _ => automexia_extension_api::IconKind::Windows,
        };
        add(
            "icon-fixed",
            "fixed icon",
            "Icon independent of context.",
            settings::SettingKind::Choice {
                options: FIXED_ICONS
                    .iter()
                    .map(|(value, label, _)| settings::ChoiceOption {
                        value: (*value).into(),
                        label: (*label).into(),
                    })
                    .collect(),
            },
            SettingValue::Choice(fixed_icon_id(icon_fixed(&slot.icon)).into()),
            SettingValue::Choice(fixed_icon_id(icon_fixed(&default.icon)).into()),
        )?;
        let lane = |lane: BarLane| match lane {
            BarLane::Leading => "leading",
            BarLane::Trailing => "trailing",
        };
        add(
            "lane",
            "side",
            "Leading or trailing side.",
            choice_kind(&[("leading", "Leading"), ("trailing", "Trailing")]),
            SettingValue::Choice(lane(slot.lane).into()),
            SettingValue::Choice(lane(default.lane).into()),
        )?;
        add(
            "order",
            "order",
            "Position among tags.",
            settings::SettingKind::Number {
                min: 1.0,
                max: recipe.slots.len().max(default_recipe.slots.len()) as f64,
                step: 1.0,
            },
            SettingValue::Number(position as f64),
            SettingValue::Number(default_position as f64),
        )?;
        // Inherited color follows the value that owns semantics in the painted
        // item. A fixed icon cannot change the text source's identity or tint.
        let source_role = match slot.text {
            BarTextSource::Role(source) => Some(source),
            BarTextSource::None => match slot.icon {
                BarIconSource::Role(source) => Some(source),
                _ => None,
            },
            BarTextSource::Literal(_) => None,
        };
        let inherited_color = source_role.map_or(snapshot.foreground, |source| {
            crate::automexia::presentation::tag_anchor(&snapshot.tags, source)
        });
        let current_color = slot.color.unwrap_or(inherited_color);
        let default_color = inherited_color;
        add(
            "color",
            "This tag color",
            "Text and tint for this tag.",
            settings::SettingKind::Color { alpha: false },
            SettingValue::Color([
                current_color[0],
                current_color[1],
                current_color[2],
                255,
            ]),
            SettingValue::Color([
                default_color[0],
                default_color[1],
                default_color[2],
                255,
            ]),
        )?;
        for (field, value, inherited) in [
            ("prefix", &slot.prefix, &default.prefix),
            ("suffix", &slot.suffix, &default.suffix),
        ] {
            let mut row = visual_row(
                &format!("{prefix}{field}"),
                format!("{slot_label} {field}"),
                settings::SettingKind::Text {
                    max_bytes: MAX_BAR_AFFIX_BYTES,
                    allow_empty: true,
                },
                SettingValue::Text(value.clone()),
                SettingValue::Text(inherited.clone()),
                origin,
            )?;
            row.description = format!("Optional {field}; 24 UTF-8 bytes max.");
            row.keywords = vec!["information bar slot text".into()];
            if matches!(slot.text, BarTextSource::None) {
                row.availability = settings::Availability::Unavailable {
                    reason: "Choose a text source before adding a prefix or suffix."
                        .into(),
                };
            }
            rows.push(row);
        }
        if custom_number.is_some() {
            let mut remove = visual_row(
                &format!("{prefix}remove"),
                format!("Remove {slot_label}"),
                settings::SettingKind::Action,
                SettingValue::Action,
                SettingValue::Action,
                ValueOrigin::User,
            )?;
            remove.description = "Delete this tag and its saved choices.".into();
            remove.keywords = vec!["information bar custom tag remove".into()];
            rows.push(remove);
        }
    }
    Ok(rows)
}

fn tag_style_value(style: TagStyle) -> SettingValue {
    SettingValue::Choice(
        match style {
            TagStyle::Tinted => "tinted",
            TagStyle::Plain => "plain",
        }
        .into(),
    )
}

fn output_style_value(style: HighlightStyle) -> SettingValue {
    SettingValue::Choice(
        match style {
            HighlightStyle::Foreground => "foreground",
            HighlightStyle::Background => "background",
            HighlightStyle::Both => "both",
        }
        .into(),
    )
}

// The foreground editor accepts RGB only. A configured terminal palette may
// carry alpha; retain that alpha in the renderer until the user sets an RGB
// override, but show the same color as an opaque editor value.
fn editor_rgb(color: [u8; 4]) -> SettingValue {
    SettingValue::Color([color[0], color[1], color[2], 255])
}

fn visual_descriptors(
    base: &Config,
    effective: &Config,
    preferences: &UserPreferences,
    palette: &Colors,
) -> Result<Vec<settings::SettingDescriptor>, SettingsError> {
    use crate::automexia::presentation::{
        self, COMMAND_OUTPUT_BACKGROUND_BINDINGS, KUBERNETES_BACKGROUND_BINDINGS,
        KUBERNETES_COLOR_BINDINGS, OUTPUT_BACKGROUND_BINDINGS, OUTPUT_COLOR_BINDINGS,
        TAG_COLOR_BINDINGS,
    };
    let mut rows = Vec::with_capacity(48);
    rows.push(window_controls::style_row(
        base.presentation.window_controls,
        effective.presentation.window_controls,
        preferences.visual.window_controls,
    )?);
    let mut enabled = visual_row(
        presentation::TAG_ENABLED,
        "Show information tags".into(),
        settings::SettingKind::Boolean,
        SettingValue::Boolean(effective.presentation.tags.enabled),
        SettingValue::Boolean(base.presentation.tags.enabled),
        visual_origin(
            preferences.visual.tags.enabled.is_some(),
            true,
            ValueOrigin::Configuration,
        ),
    )?;
    enabled.description = "Show tags beside prompts.".into();
    enabled.keywords = vec!["context information tags visibility".into()];
    rows.push(enabled);
    let bar_preferences = &preferences.visual.information_bar;
    let selected_preset = bar_preferences.preset.unwrap_or_default();
    let mut format_options: Vec<_> =
        automexia_ui_model::information_bar::InformationBarPreset::ALL
            .into_iter()
            .map(|preset| settings::ChoiceOption {
                value: preset.id().into(),
                label: preset.label().into(),
            })
            .collect();
    if bar_preferences.custom_recipe.is_some() {
        format_options.push(settings::ChoiceOption {
            value: "custom".into(),
            label: "Custom layout".into(),
        });
    }
    let mut format = visual_row(
        presentation::TAG_FORMAT,
        "Layout".into(),
        settings::SettingKind::Choice {
            options: format_options,
        },
        SettingValue::Choice(
            if bar_preferences.use_custom {
                "custom"
            } else {
                selected_preset.id()
            }
            .into(),
        ),
        SettingValue::Choice(
            automexia_ui_model::information_bar::InformationBarPreset::default()
                .id()
                .into(),
        ),
        visual_origin(
            bar_preferences.preset.is_some() || bar_preferences.use_custom,
            false,
            ValueOrigin::Default,
        ),
    )?;
    format.description = "Choose a preset or custom layout.".into();
    format.keywords = vec!["context information tags format layout preset".into()];
    rows.push(format);
    use automexia_ui_model::information_bar::{BarArrangement, BarVisualStyle};
    let selected_recipe = bar_preferences.recipe();
    let default_recipe =
        automexia_ui_model::information_bar::preset_recipe(selected_preset);
    let shape_choices: Vec<_> = BarVisualStyle::ALL
        .into_iter()
        .map(|style| (style.id(), style.label()))
        .collect();
    let arrangement_id = |arrangement: BarArrangement| match arrangement {
        BarArrangement::Flow => "flow",
        BarArrangement::Split => "split",
        BarArrangement::TwoLine => "two-line",
        BarArrangement::Adaptive => "adaptive",
        BarArrangement::Dashboard => "dashboard",
        BarArrangement::Compact => "compact",
    };
    let mut bar_style = visual_row(
        "tags.bar-style",
        "Shape".into(),
        choice_kind(&shape_choices),
        SettingValue::Choice(selected_recipe.visual.id().into()),
        SettingValue::Choice(default_recipe.visual.id().into()),
        visual_origin(bar_preferences.use_custom, false, ValueOrigin::Default),
    )?;
    bar_style.description = "Shape of your custom layout.".into();
    rows.push(bar_style);
    let mut bar_arrangement = visual_row(
        "tags.bar-arrangement",
        "Arrangement".into(),
        choice_kind(&[
            ("flow", "Flow"),
            ("split", "Left and right"),
            ("two-line", "Two-line"),
            ("adaptive", "Adaptive"),
            ("dashboard", "Dashboard"),
            ("compact", "Compact"),
        ]),
        SettingValue::Choice(arrangement_id(selected_recipe.arrangement).into()),
        SettingValue::Choice(arrangement_id(default_recipe.arrangement).into()),
        visual_origin(bar_preferences.use_custom, false, ValueOrigin::Default),
    )?;
    bar_arrangement.description = "How your tags align.".into();
    rows.push(bar_arrangement);
    let mut spacing = visual_row(
        "tags.spacing",
        "Space between tags".into(),
        settings::SettingKind::Number {
            min: 0.0,
            max: f64::from(automexia_ui_model::information_bar::MAX_BAR_SPACING_PERCENT),
            step: 1.0,
        },
        SettingValue::Number(f64::from(selected_recipe.spacing_percent)),
        SettingValue::Number(f64::from(default_recipe.spacing_percent)),
        visual_origin(bar_preferences.use_custom, false, ValueOrigin::Default),
    )?;
    spacing.description = "0% none; 100% preset gap.".into();
    spacing.keywords = vec!["information header tag gap spacing density".into()];
    rows.push(spacing);
    if selected_recipe.slots.len() < automexia_ui_model::information_bar::MAX_BAR_SLOTS
        && (1..=3).any(|number| {
            !selected_recipe
                .slots
                .iter()
                .any(|slot| slot.id == format!("custom-{number}"))
        })
    {
        let mut add_slot = visual_row(
            "tags.add-slot",
            "Add custom tag".into(),
            settings::SettingKind::Action,
            SettingValue::Action,
            SettingValue::Action,
            ValueOrigin::Default,
        )?;
        add_slot.description = "Add a display-only tag.".into();
        add_slot.keywords = vec!["new custom information tag slot".into()];
        rows.push(add_slot);
    }
    let mut style = visual_row(
        presentation::TAG_STYLE,
        "Context tag style".into(),
        choice_kind(&[("tinted", "Tinted"), ("plain", "Plain")]),
        tag_style_value(effective.presentation.tags.style),
        tag_style_value(base.presentation.tags.style),
        visual_origin(
            preferences.visual.tags.style.is_some(),
            true,
            ValueOrigin::Configuration,
        ),
    )?;
    style.description = "Tinted background or plain text.".into();
    rows.push(style);
    let mut opacity = visual_row(
        presentation::TAG_OPACITY,
        "Context tag background opacity".into(),
        settings::SettingKind::Number {
            min: 0.0,
            max: 100.0,
            step: 1.0,
        },
        SettingValue::Number(f64::from(effective.presentation.tags.opacity.get())),
        SettingValue::Number(f64::from(base.presentation.tags.opacity.get())),
        visual_origin(
            preferences.visual.tags.opacity.is_some(),
            true,
            ValueOrigin::Configuration,
        ),
    )?;
    opacity.description = "Tint opacity, 0-100%.".into();
    rows.push(opacity);
    for binding in &TAG_COLOR_BINDINGS {
        let current =
            presentation::tag_anchor(&effective.presentation.tags, binding.role);
        let default = presentation::tag_anchor(&base.presentation.tags, binding.role);
        let mut row = visual_row(
            binding.id,
            format!("{} tag color", binding.label),
            settings::SettingKind::Color { alpha: false },
            SettingValue::Color([current[0], current[1], current[2], 255]),
            SettingValue::Color([default[0], default[1], default[2], 255]),
            visual_origin(
                (binding.read)(&preferences.visual.tags.colors).is_some(),
                (binding.read)(&base.presentation.tags.colors).is_some(),
                ValueOrigin::Default,
            ),
        )?;
        row.description = "Tag text and tint color.".into();
        row.keywords = vec!["context information tags color".into()];
        rows.push(row);
    }
    let mut style = visual_row(
        presentation::OUTPUT_STYLE,
        "Log highlight style".into(),
        choice_kind(&[
            ("foreground", "Text"),
            ("background", "Background"),
            ("both", "Text and background"),
        ]),
        output_style_value(effective.presentation.highlight.style),
        output_style_value(base.presentation.highlight.style),
        visual_origin(
            preferences.visual.highlight.style.is_some(),
            true,
            ValueOrigin::Configuration,
        ),
    )?;
    style.description = "Text, background, or both.".into();
    rows.push(style);
    for binding in &OUTPUT_COLOR_BINDINGS {
        let mut row = visual_row(
            binding.id,
            format!("{} output text color", binding.label),
            settings::SettingKind::Color { alpha: false },
            editor_rgb(presentation::output_foreground(
                &effective.presentation.highlight,
                palette,
                binding.severity,
            )),
            editor_rgb(presentation::output_foreground(
                &base.presentation.highlight,
                palette,
                binding.severity,
            )),
            visual_origin(
                (binding.read)(&preferences.visual.highlight.colors).is_some(),
                (binding.read)(&base.presentation.highlight.colors).is_some(),
                ValueOrigin::Configuration,
            ),
        )?;
        row.description = "Detected status text color.".into();
        row.keywords = vec!["output status severity highlight color".into()];
        rows.push(row);
    }
    for binding in &OUTPUT_BACKGROUND_BINDINGS {
        let mut row = visual_row(
            binding.id,
            binding.label.into(),
            settings::SettingKind::Color { alpha: true },
            SettingValue::Color(binding.editor_color(&effective.presentation.highlight)),
            SettingValue::Color(binding.editor_color(&base.presentation.highlight)),
            visual_origin(
                match binding.severity {
                    automexia_extension_api::SemanticSeverity::Error => {
                        preferences.visual.highlight.error_background.is_some()
                    }
                    automexia_extension_api::SemanticSeverity::Warning => {
                        preferences.visual.highlight.warning_background.is_some()
                    }
                    automexia_extension_api::SemanticSeverity::Success => {
                        preferences.visual.highlight.success_background.is_some()
                    }
                    automexia_extension_api::SemanticSeverity::Info => {
                        preferences.visual.highlight.info_background.is_some()
                    }
                    automexia_extension_api::SemanticSeverity::Debug => {
                        preferences.visual.highlight.debug_background.is_some()
                    }
                },
                (binding.read)(&base.presentation.highlight).is_some(),
                ValueOrigin::Default,
            ),
        )?;
        row.description = if binding.default_in_both {
            "Status background and opacity."
        } else {
            "Choose a background tint and opacity."
        }
        .into();
        row.keywords = vec!["output background transparency highlight".into()];
        rows.push(row);
    }
    let mut pulse = visual_row(
        presentation::COMMAND_OUTPUT_PULSE,
        "Pulse completed-command color".into(),
        settings::SettingKind::Boolean,
        SettingValue::Boolean(effective.presentation.command_output.pulse),
        SettingValue::Boolean(base.presentation.command_output.pulse),
        visual_origin(
            preferences.visual.command_output.pulse.is_some(),
            true,
            ValueOrigin::Configuration,
        ),
    )?;
    pulse.description = "Brief color pulse after completion.".into();
    pulse.keywords = vec!["command output completion pulse motion".into()];
    rows.push(pulse);
    for binding in &COMMAND_OUTPUT_BACKGROUND_BINDINGS {
        let current = (binding.read)(&effective.presentation.command_output).map_or_else(
            || {
                presentation::normalized_to_u8(
                    binding.resolve(&effective.presentation.command_output, palette),
                )
            },
            Rgba::bytes,
        );
        let default = (binding.read)(&base.presentation.command_output).map_or_else(
            || {
                presentation::normalized_to_u8(
                    binding.resolve(&base.presentation.command_output, palette),
                )
            },
            Rgba::bytes,
        );
        let selected = match binding.kind {
            presentation::CommandOutputKind::Success => {
                preferences.visual.command_output.success.is_some()
            }
            presentation::CommandOutputKind::Failure => {
                preferences.visual.command_output.failure.is_some()
            }
            presentation::CommandOutputKind::Neutral => {
                preferences.visual.command_output.neutral.is_some()
            }
        };
        let mut row = visual_row(
            binding.id,
            format!("{} background", binding.label),
            settings::SettingKind::Color { alpha: true },
            SettingValue::Color(current),
            SettingValue::Color(default),
            visual_origin(
                selected,
                (binding.read)(&base.presentation.command_output).is_some(),
                ValueOrigin::Default,
            ),
        )?;
        row.description = "Completed-command tint and opacity.".into();
        row.keywords = vec!["command result output color background".into()];
        rows.push(row);
    }
    let mut kubernetes_style = visual_row(
        presentation::KUBERNETES_STYLE,
        "Kubernetes highlight style".into(),
        choice_kind(&[
            ("foreground", "Text"),
            ("background", "Background"),
            ("both", "Text and background"),
        ]),
        output_style_value(effective.presentation.kubernetes.style),
        output_style_value(base.presentation.kubernetes.style),
        visual_origin(
            preferences.visual.kubernetes.style.is_some(),
            true,
            ValueOrigin::Configuration,
        ),
    )?;
    kubernetes_style.description = "Kubernetes status text, background, or both.".into();
    rows.push(kubernetes_style);
    for binding in &KUBERNETES_COLOR_BINDINGS {
        let mut row = visual_row(
            binding.id,
            format!("{} Kubernetes text color", binding.label),
            settings::SettingKind::Color { alpha: false },
            editor_rgb(presentation::output_foreground(
                &effective.presentation.kubernetes,
                palette,
                binding.severity,
            )),
            editor_rgb(presentation::output_foreground(
                &base.presentation.kubernetes,
                palette,
                binding.severity,
            )),
            visual_origin(
                (binding.read)(&preferences.visual.kubernetes.colors).is_some(),
                (binding.read)(&base.presentation.kubernetes.colors).is_some(),
                ValueOrigin::Configuration,
            ),
        )?;
        row.description = "Recognized Kubernetes status text color.".into();
        row.keywords = vec!["kubernetes pod status text severity color".into()];
        rows.push(row);
    }
    for binding in &KUBERNETES_BACKGROUND_BINDINGS {
        let selected = match binding.severity {
            automexia_extension_api::SemanticSeverity::Error => {
                preferences.visual.kubernetes.error_background.is_some()
            }
            automexia_extension_api::SemanticSeverity::Warning => {
                preferences.visual.kubernetes.warning_background.is_some()
            }
            automexia_extension_api::SemanticSeverity::Success => {
                preferences.visual.kubernetes.success_background.is_some()
            }
            automexia_extension_api::SemanticSeverity::Info => {
                preferences.visual.kubernetes.info_background.is_some()
            }
            automexia_extension_api::SemanticSeverity::Debug => {
                preferences.visual.kubernetes.debug_background.is_some()
            }
        };
        let mut row = visual_row(
            binding.id,
            format!("{} Kubernetes background", binding.label),
            settings::SettingKind::Color { alpha: true },
            SettingValue::Color(binding.editor_color(&effective.presentation.kubernetes)),
            SettingValue::Color(binding.editor_color(&base.presentation.kubernetes)),
            visual_origin(
                selected,
                (binding.read)(&base.presentation.kubernetes).is_some(),
                ValueOrigin::Default,
            ),
        )?;
        row.description = "Kubernetes status tint and opacity.".into();
        row.keywords = vec!["kubernetes pod background transparency color".into()];
        rows.push(row);
    }
    // Keep the controls alongside the background in each preview detail page.
    // All validation, direct typing, keyboard stepping and reset use the
    // existing numeric editor and the single color preference owner.
    append_opacity_controls(&mut rows)?;
    Ok(rows)
}

fn append_opacity_controls(
    rows: &mut Vec<settings::SettingDescriptor>,
) -> Result<(), SettingsError> {
    let opacity = rows
        .iter()
        .filter_map(|row| {
            let (domain, status) = row.id.as_str().split_once(".backgrounds.")?;
            let (SettingValue::Color(color), SettingValue::Color(default)) =
                (&row.value, &row.default)
            else {
                return None;
            };
            Some(
                visual_row(
                    &format!("{domain}.opacity.{status}"),
                    if matches!(domain, "tables" | "timestamps") {
                        format!("{} opacity (%)", row.label)
                    } else {
                        "Background opacity (%)".into()
                    },
                    settings::SettingKind::Number {
                        min: 0.0,
                        max: 100.0,
                        step: 1.0,
                    },
                    SettingValue::Number((f64::from(color[3]) * 100.0 / 255.0).round()),
                    SettingValue::Number((f64::from(default[3]) * 100.0 / 255.0).round()),
                    row.origin,
                )
                .map(|mut row| {
                    row.description = "0: transparent · 100: solid.".into();
                    row
                }),
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    rows.extend(opacity);
    Ok(())
}

fn rgb_change(change: &Change) -> Result<Option<Rgb>, SettingsError> {
    match change {
        Change::Reset => Ok(None),
        Change::Set(SettingValue::Color([red, green, blue, 255])) => {
            Ok(Some(Rgb::from_bytes([*red, *green, *blue])))
        }
        _ => Err(SettingsError::InvalidValue),
    }
}

fn apply_visual_edit(
    candidate: &mut UserPreferences,
    edit: &Edit,
) -> Result<bool, SettingsError> {
    if tables::apply(candidate, edit)? {
        return Ok(true);
    }
    if timestamps::apply(candidate, edit)? {
        return Ok(true);
    }
    use crate::automexia::presentation::{
        self, COMMAND_OUTPUT_BACKGROUND_BINDINGS, KUBERNETES_BACKGROUND_BINDINGS,
        KUBERNETES_COLOR_BINDINGS, OUTPUT_BACKGROUND_BINDINGS, OUTPUT_COLOR_BINDINGS,
        TAG_COLOR_BINDINGS,
    };
    match edit.id.as_str() {
        presentation::TAG_ENABLED => {
            candidate.visual.tags.enabled = match &edit.change {
                Change::Reset => None,
                Change::Set(SettingValue::Boolean(value)) => Some(*value),
                _ => return Err(SettingsError::InvalidValue),
            };
            return Ok(true);
        }
        presentation::TAG_FORMAT => {
            let selected = match &edit.change {
                Change::Reset => None,
                Change::Set(SettingValue::Choice(value)) if value == "custom" => {
                    if candidate.visual.information_bar.custom_recipe.is_none() {
                        return Err(SettingsError::InvalidValue);
                    }
                    candidate.visual.information_bar.use_custom = true;
                    return Ok(true);
                }
                Change::Set(SettingValue::Choice(value)) => Some(
                    automexia_ui_model::information_bar::InformationBarPreset::from_id(
                        value,
                    )
                    .ok_or(SettingsError::InvalidValue)?,
                ),
                _ => return Err(SettingsError::InvalidValue),
            };
            candidate.visual.information_bar.preset = selected;
            candidate.visual.information_bar.use_custom = false;
            return Ok(true);
        }
        "tags.add-slot" => {
            if !matches!(edit.change, Change::Activate) {
                return Err(SettingsError::InvalidValue);
            }
            let bar = &mut candidate.visual.information_bar;
            let mut recipe = bar.recipe();
            if recipe.slots.len() >= automexia_ui_model::information_bar::MAX_BAR_SLOTS {
                return Err(SettingsError::Capacity);
            }
            let number = (1..=3)
                .find(|number| {
                    !recipe
                        .slots
                        .iter()
                        .any(|slot| slot.id == format!("custom-{number}"))
                })
                .ok_or(SettingsError::Capacity)?;
            recipe.slots.push(custom_slot(number));
            bar.custom_recipe = Some(recipe);
            bar.use_custom = true;
            return Ok(true);
        }
        "tags.bar-style" | "tags.bar-arrangement" | "tags.spacing" => {
            use automexia_ui_model::information_bar::{BarArrangement, BarVisualStyle};
            let bar = &mut candidate.visual.information_bar;
            let mut recipe = bar.recipe();
            let default = automexia_ui_model::information_bar::preset_recipe(
                bar.preset.unwrap_or_default(),
            );
            if edit.id.as_str() == "tags.bar-style" {
                recipe.visual = match &edit.change {
                    Change::Reset => default.visual,
                    Change::Set(SettingValue::Choice(value)) => {
                        BarVisualStyle::from_id(value)
                            .ok_or(SettingsError::InvalidValue)?
                    }
                    _ => return Err(SettingsError::InvalidValue),
                };
            } else if edit.id.as_str() == "tags.bar-arrangement" {
                recipe.arrangement = match &edit.change {
                    Change::Reset => default.arrangement,
                    Change::Set(SettingValue::Choice(value)) => match value.as_str() {
                        "flow" => BarArrangement::Flow,
                        "split" => BarArrangement::Split,
                        "two-line" => BarArrangement::TwoLine,
                        "adaptive" => BarArrangement::Adaptive,
                        "dashboard" => BarArrangement::Dashboard,
                        "compact" => BarArrangement::Compact,
                        _ => return Err(SettingsError::InvalidValue),
                    },
                    _ => return Err(SettingsError::InvalidValue),
                };
            } else {
                recipe.spacing_percent = match &edit.change {
                    Change::Reset => default.spacing_percent,
                    Change::Set(SettingValue::Number(value)) => {
                        // The catalogue already enforces the finite 0..300 step lattice.
                        value.round() as u16
                    }
                    _ => return Err(SettingsError::InvalidValue),
                };
            }
            bar.custom_recipe = Some(recipe);
            bar.use_custom = true;
            return Ok(true);
        }
        presentation::TAG_STYLE => {
            candidate.visual.tags.style = match &edit.change {
                Change::Reset => None,
                Change::Set(SettingValue::Choice(value)) => Some(match value.as_str() {
                    "tinted" => TagStyle::Tinted,
                    "plain" => TagStyle::Plain,
                    _ => return Err(SettingsError::InvalidValue),
                }),
                _ => return Err(SettingsError::InvalidValue),
            };
            return Ok(true);
        }
        presentation::TAG_OPACITY => {
            candidate.visual.tags.opacity = match &edit.change {
                Change::Reset => None,
                Change::Set(SettingValue::Number(value)) => {
                    // Catalog validation has checked the finite 0..100 integer lattice.
                    // Round accepted floating-point tolerance instead of truncating it.
                    Some(
                        OpacityPercent::new(value.round() as u8)
                            .map_err(|_| SettingsError::InvalidValue)?,
                    )
                }
                _ => return Err(SettingsError::InvalidValue),
            };
            return Ok(true);
        }
        presentation::OUTPUT_STYLE => {
            candidate.visual.highlight.style = match &edit.change {
                Change::Reset => None,
                Change::Set(SettingValue::Choice(value)) => Some(match value.as_str() {
                    "foreground" => HighlightStyle::Foreground,
                    "background" => HighlightStyle::Background,
                    "both" => HighlightStyle::Both,
                    _ => return Err(SettingsError::InvalidValue),
                }),
                _ => return Err(SettingsError::InvalidValue),
            };
            return Ok(true);
        }
        presentation::KUBERNETES_STYLE => {
            candidate.visual.kubernetes.style = match &edit.change {
                Change::Reset => None,
                Change::Set(SettingValue::Choice(value)) => Some(match value.as_str() {
                    "foreground" => HighlightStyle::Foreground,
                    "background" => HighlightStyle::Background,
                    "both" => HighlightStyle::Both,
                    _ => return Err(SettingsError::InvalidValue),
                }),
                _ => return Err(SettingsError::InvalidValue),
            };
            return Ok(true);
        }
        presentation::COMMAND_OUTPUT_PULSE => {
            candidate.visual.command_output.pulse = match &edit.change {
                Change::Reset => None,
                Change::Set(SettingValue::Boolean(value)) => Some(*value),
                _ => return Err(SettingsError::InvalidValue),
            };
            return Ok(true);
        }
        _ => {}
    }
    if let Some(binding) = TAG_COLOR_BINDINGS
        .iter()
        .find(|binding| binding.id == edit.id.as_str())
    {
        (binding.write)(&mut candidate.visual.tags.colors, rgb_change(&edit.change)?);
        return Ok(true);
    }
    if let Some(binding) = OUTPUT_COLOR_BINDINGS
        .iter()
        .find(|binding| binding.id == edit.id.as_str())
    {
        (binding.write)(
            &mut candidate.visual.highlight.colors,
            rgb_change(&edit.change)?,
        );
        return Ok(true);
    }
    if let Some(binding) = OUTPUT_BACKGROUND_BINDINGS
        .iter()
        .find(|binding| binding.id == edit.id.as_str())
    {
        let value = match &edit.change {
            Change::Reset => None,
            Change::Set(SettingValue::Color(value)) => Some(Rgba::from_bytes(*value)),
            _ => return Err(SettingsError::InvalidValue),
        };
        // Background overrides belong to the preference overlay, not a second palette.
        match binding.severity {
            automexia_extension_api::SemanticSeverity::Error => {
                candidate.visual.highlight.error_background = value
            }
            automexia_extension_api::SemanticSeverity::Warning => {
                candidate.visual.highlight.warning_background = value
            }
            automexia_extension_api::SemanticSeverity::Success => {
                candidate.visual.highlight.success_background = value
            }
            automexia_extension_api::SemanticSeverity::Info => {
                candidate.visual.highlight.info_background = value
            }
            automexia_extension_api::SemanticSeverity::Debug => {
                candidate.visual.highlight.debug_background = value
            }
        }
        return Ok(true);
    }
    if let Some(binding) = COMMAND_OUTPUT_BACKGROUND_BINDINGS
        .iter()
        .find(|binding| binding.id == edit.id.as_str())
    {
        let value = match &edit.change {
            Change::Reset => None,
            Change::Set(SettingValue::Color(value)) => Some(Rgba::from_bytes(*value)),
            _ => return Err(SettingsError::InvalidValue),
        };
        match binding.kind {
            presentation::CommandOutputKind::Success => {
                candidate.visual.command_output.success = value
            }
            presentation::CommandOutputKind::Failure => {
                candidate.visual.command_output.failure = value
            }
            presentation::CommandOutputKind::Neutral => {
                candidate.visual.command_output.neutral = value
            }
        }
        return Ok(true);
    }
    if let Some(binding) = KUBERNETES_COLOR_BINDINGS
        .iter()
        .find(|binding| binding.id == edit.id.as_str())
    {
        (binding.write)(
            &mut candidate.visual.kubernetes.colors,
            rgb_change(&edit.change)?,
        );
        return Ok(true);
    }
    if let Some(binding) = KUBERNETES_BACKGROUND_BINDINGS
        .iter()
        .find(|binding| binding.id == edit.id.as_str())
    {
        let value = match &edit.change {
            Change::Reset => None,
            Change::Set(SettingValue::Color(value)) => Some(Rgba::from_bytes(*value)),
            _ => return Err(SettingsError::InvalidValue),
        };
        match binding.severity {
            automexia_extension_api::SemanticSeverity::Error => {
                candidate.visual.kubernetes.error_background = value
            }
            automexia_extension_api::SemanticSeverity::Warning => {
                candidate.visual.kubernetes.warning_background = value
            }
            automexia_extension_api::SemanticSeverity::Success => {
                candidate.visual.kubernetes.success_background = value
            }
            automexia_extension_api::SemanticSeverity::Info => {
                candidate.visual.kubernetes.info_background = value
            }
            automexia_extension_api::SemanticSeverity::Debug => {
                candidate.visual.kubernetes.debug_background = value
            }
        }
        return Ok(true);
    }
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use settings::{SettingId, SettingOwner};

    #[test]
    fn temporary_global_defaults_enable_core_features_without_mutating_saved_choices() {
        let mut saved = UserPreferences::default();
        saved.presentation.inline_tables = Some(false);
        saved.presentation.output_highlighting = Some(false);
        saved.presentation.command_timestamps = Some(false);
        saved.visual.tags.enabled = Some(false);
        saved.font_size = Some(19.0);
        saved
            .set_extension_feature_enabled(
                settings_extensions::DEVOPS_CONTEXT_STATUS_ID,
                false,
            )
            .unwrap();
        let original = saved.clone();
        let preview =
            reset_customizations(&saved, &CustomizationResetScope::All, None).unwrap();
        assert_eq!(
            saved, original,
            "the saved snapshot is never changed in place"
        );
        assert_eq!(preview.presentation.inline_tables, Some(true));
        assert_eq!(preview.presentation.output_highlighting, Some(true));
        assert_eq!(preview.presentation.command_timestamps, Some(true));
        assert_eq!(preview.visual.tags.enabled, Some(true));
        assert_eq!(preview.font_size, None);
        assert!(
            preview.extension_features.is_empty(),
            "extension defaults belong to their declarations"
        );
    }

    #[test]
    fn local_tag_group_reset_preserves_other_categories_and_previous_choices() {
        let mut saved = UserPreferences::default();
        saved.visual.tags.enabled = Some(false);
        saved.visual.tags.style = Some(TagStyle::Plain);
        saved.presentation.output_highlighting = Some(false);
        saved.presentation.inline_tables = Some(false);
        saved
            .set_extension_feature_enabled(
                settings_extensions::DEVOPS_CONTEXT_STATUS_ID,
                false,
            )
            .unwrap();
        saved
            .set_extension_feature_enabled(
                settings_extensions::DEVOPS_GIT_STATUS_ID,
                false,
            )
            .unwrap();
        let preview = reset_customizations(
            &saved,
            &CustomizationResetScope::Group(SettingId::new("tags.enabled").unwrap()),
            None,
        )
        .unwrap();
        assert_eq!(preview.visual.tags.enabled, Some(true));
        assert_eq!(preview.visual.tags.style, None);
        assert_eq!(preview.presentation.output_highlighting, Some(false));
        assert_eq!(preview.presentation.inline_tables, Some(false));
        assert_eq!(saved.visual.tags.enabled, Some(false));
        assert_eq!(
            preview
                .extension_feature_enabled(settings_extensions::DEVOPS_CONTEXT_STATUS_ID),
            None
        );
        assert_eq!(
            preview.extension_feature_enabled(settings_extensions::DEVOPS_GIT_STATUS_ID),
            None
        );
        assert_eq!(
            saved
                .extension_feature_enabled(settings_extensions::DEVOPS_CONTEXT_STATUS_ID),
            Some(false)
        );
    }

    #[test]
    fn git_tag_reset_restores_its_feature_without_changing_other_tag_preferences() {
        let mut saved = UserPreferences::default();
        saved
            .set_extension_feature_enabled(
                settings_extensions::DEVOPS_GIT_STATUS_ID,
                false,
            )
            .unwrap();
        saved
            .set_extension_feature_enabled(
                settings_extensions::DEVOPS_CONTEXT_STATUS_ID,
                false,
            )
            .unwrap();
        let preview = reset_customizations(
            &saved,
            &CustomizationResetScope::Tag("git".into()),
            None,
        )
        .unwrap();
        assert_eq!(
            preview.extension_feature_enabled(settings_extensions::DEVOPS_GIT_STATUS_ID),
            None
        );
        assert_eq!(
            preview
                .extension_feature_enabled(settings_extensions::DEVOPS_CONTEXT_STATUS_ID),
            Some(false)
        );
        assert_eq!(
            saved.extension_feature_enabled(settings_extensions::DEVOPS_GIT_STATUS_ID),
            Some(false)
        );
    }

    #[test]
    fn selected_tag_reset_restores_only_its_recipe_slot_and_discards_its_source_draft() {
        use automexia_ui_model::information_bar::{BarTextSource, InformationBarPreset};
        let mut saved = UserPreferences::default();
        let bar = &mut saved.visual.information_bar;
        bar.preset = Some(InformationBarPreset::RoundedCapsules);
        let mut recipe = bar.recipe();
        let windows = recipe
            .slots
            .iter_mut()
            .find(|slot| slot.id == "windows")
            .unwrap();
        windows.text = BarTextSource::Literal("Different".into());
        let user = recipe
            .slots
            .iter_mut()
            .find(|slot| slot.id == "user")
            .unwrap();
        user.text = BarTextSource::Literal("Keep me".into());
        let windows_position = recipe
            .slots
            .iter()
            .position(|slot| slot.id == "windows")
            .unwrap();
        let windows = recipe.slots.remove(windows_position);
        recipe.slots.push(windows);
        bar.custom_recipe = Some(recipe);
        bar.use_custom = true;
        bar.source_drafts
            .insert("windows".into(), Default::default());
        let preview = reset_customizations(
            &saved,
            &CustomizationResetScope::Tag("windows".into()),
            None,
        )
        .unwrap();
        let recipe = preview.visual.information_bar.recipe();
        assert_eq!(
            recipe.slots.iter().position(|slot| slot.id == "windows"),
            Some(windows_position),
        );
        assert_ne!(
            recipe
                .slots
                .iter()
                .find(|slot| slot.id == "windows")
                .unwrap()
                .text,
            BarTextSource::Literal("Different".into())
        );
        assert_eq!(
            recipe
                .slots
                .iter()
                .find(|slot| slot.id == "user")
                .unwrap()
                .text,
            BarTextSource::Literal("Keep me".into())
        );
        assert!(!preview
            .visual
            .information_bar
            .source_drafts
            .contains_key("windows"));
        assert_eq!(
            saved
                .visual
                .information_bar
                .recipe()
                .slots
                .iter()
                .find(|slot| slot.id == "windows")
                .unwrap()
                .text,
            BarTextSource::Literal("Different".into())
        );
    }

    #[test]
    fn canonical_tag_remains_editable_and_resettable_across_preset_changes() {
        use automexia_ui_model::information_bar::{preset_recipe, InformationBarPreset};
        let mut saved = UserPreferences::default();
        let original_windows = preset_recipe(InformationBarPreset::RoundedCapsules)
            .slots
            .into_iter()
            .find(|slot| slot.id == "windows")
            .unwrap();
        let mut recipe = preset_recipe(InformationBarPreset::RoundedCapsules);
        recipe
            .slots
            .iter_mut()
            .find(|slot| slot.id == "windows")
            .unwrap()
            .enabled = false;
        saved.visual.information_bar.preset =
            Some(InformationBarPreset::GitFirstDeveloperBar);
        saved.visual.information_bar.custom_recipe = Some(recipe);
        saved.visual.information_bar.use_custom = true;
        let base = Config::default();
        let snapshot = slot_page_snapshot(&saved, &saved.apply_to(&base));
        assert!(slot_page_catalog(1, &snapshot, "windows").is_ok());
        let preview = reset_customizations(
            &saved,
            &CustomizationResetScope::Tag("windows".into()),
            None,
        )
        .unwrap();
        assert_eq!(
            preview
                .visual
                .information_bar
                .recipe()
                .slots
                .iter()
                .find(|slot| slot.id == "windows")
                .unwrap(),
            &original_windows,
        );
    }
    #[test]
    fn reset_materializes_omitted_canonical_tags_without_changing_other_slots() {
        use automexia_ui_model::information_bar::{
            preset_recipe, role_id, validate_recipe, InformationBarPreset, STANDARD_ROLES,
        };

        let preset = preset_recipe(InformationBarPreset::RoundedCapsules);
        for role in STANDARD_ROLES {
            let slot_id = role_id(role);
            let mut recipe = preset.clone();
            recipe.slots.retain(|slot| slot.id != slot_id);
            validate_recipe(&recipe).unwrap();
            let mut saved = UserPreferences::default();
            saved.visual.information_bar.custom_recipe = Some(recipe);
            saved.visual.information_bar.use_custom = true;
            let original = saved.clone();
            let base = Config::default();
            let snapshot = slot_page_snapshot(&saved, &saved.apply_to(&base));
            let full = catalog(7, &base, &saved, &[]).unwrap();
            assert!(selected_tag_catalog(&full, &snapshot, slot_id).is_ok());

            let reset = reset_customizations(
                &saved,
                &CustomizationResetScope::Tag(slot_id.into()),
                None,
            )
            .unwrap();
            assert_eq!(reset.visual.information_bar.recipe(), preset);
            assert_eq!(saved, original, "Reset must keep the saved snapshot intact");
        }
    }

    #[test]
    fn reset_omitted_canonical_tag_rejects_capacity_without_losing_other_slots() {
        use automexia_ui_model::information_bar::{validate_recipe, MAX_BAR_SLOTS};

        let mut saved = UserPreferences::default();
        let mut recipe = saved.visual.information_bar.recipe();
        recipe.slots.retain(|slot| slot.id != "windows");
        let mut number = 1;
        while recipe.slots.len() < MAX_BAR_SLOTS {
            recipe.slots.push(custom_slot(number));
            number += 1;
        }
        validate_recipe(&recipe).unwrap();
        saved.visual.information_bar.custom_recipe = Some(recipe);
        saved.visual.information_bar.use_custom = true;
        let original = saved.clone();
        assert_eq!(
            reset_customizations(
                &saved,
                &CustomizationResetScope::Tag("windows".into()),
                None,
            ),
            Err(SettingsError::Capacity),
        );
        assert_eq!(saved, original);
        assert_eq!(
            reset_customizations(
                &saved,
                &CustomizationResetScope::Tag("custom-3".into()),
                None,
            )
            .unwrap()
            .visual
            .information_bar
            .recipe()
            .slots
            .len(),
            MAX_BAR_SLOTS,
            "Existing tags remain resettable when the recipe is full",
        );
    }

    fn edit(id: &str, change: Change) -> Edit {
        Edit {
            revision: 7,
            id: SettingId::new(id).unwrap(),
            change,
        }
    }
    fn installed() -> Vec<MarketItem> {
        vec![MarketItem {
            id: "automexia.devops".into(),
            name: "ignored".into(),
            description: "ignored".into(),
            installed: true,
        }]
    }
    fn tag_actions(
        prefs: &UserPreferences,
        base: &Config,
    ) -> Vec<settings::SettingDescriptor> {
        slot_page_actions(&slot_page_snapshot(prefs, &prefs.apply_to(base))).unwrap()
    }
    #[test]
    fn authored_core_help_is_brief_and_category_state_comes_first() {
        let base = Config::default();
        let prefs = UserPreferences::default();
        let full = catalog(7, &base, &prefs, &[]).unwrap();
        let snapshot = slot_page_snapshot(&prefs, &prefs.apply_to(&base));
        let tag = selected_tag_catalog(&full, &snapshot, "windows").unwrap();
        for entry in full.entries().iter().chain(tag.entries()) {
            if matches!(&entry.owner, SettingOwner::Core) {
                assert!(
                    entry.description.split_whitespace().count() <= 10,
                    "{} has overly long help: {}",
                    entry.id.as_str(),
                    entry.description
                );
            }
        }
        for group in customization_groups(&full) {
            if matches!(&group.root_action.owner, SettingOwner::Core) {
                assert!(
                    group.root_action.description.split_whitespace().count() <= 12,
                    "{} has overly long category help: {}",
                    group.key.as_str(),
                    group.root_action.description
                );
            }
        }
        let tags = customization_groups(&full)
            .into_iter()
            .find(|group| group.label == "Information tags")
            .unwrap();
        assert!(tags.root_action.description.starts_with("Currently on."));
    }
    #[test]
    fn resetting_a_presentation_override_inherits_configuration_and_preserves_other_preferences(
    ) {
        let mut base = Config::default();
        base.presentation.inline_tables = false;
        let prefs = UserPreferences {
            font_size: Some(19.0),
            ..UserPreferences::default()
        };
        let enabled = apply_edit(
            7,
            &base,
            &prefs,
            &[],
            &edit(
                settings::INLINE_TABLES,
                Change::Set(SettingValue::Boolean(true)),
            ),
        )
        .unwrap();
        assert!(enabled.apply_to(&base).presentation.inline_tables);
        let reset = apply_edit(
            7,
            &base,
            &enabled,
            &[],
            &edit(settings::INLINE_TABLES, Change::Reset),
        )
        .unwrap();
        assert_eq!(reset.presentation.inline_tables, None);
        assert!(!reset.apply_to(&base).presentation.inline_tables);
        assert_eq!(reset.font_size, Some(19.0));
        assert_eq!(reset, prefs);
    }
    #[test]
    fn disabled_extension_feature_remains_listed_but_uninstall_revokes_its_edit() {
        let base = Config::default();
        let prefs = UserPreferences::default();
        let request = edit(
            settings_extensions::DEVOPS_CONTEXT_STATUS_ID,
            Change::Set(SettingValue::Boolean(false)),
        );
        let disabled = apply_edit(7, &base, &prefs, &installed(), &request).unwrap();
        let snapshot = catalog(7, &base, &disabled, &installed()).unwrap();
        assert_eq!(
            snapshot.get(&request.id).unwrap().value,
            SettingValue::Boolean(false)
        );
        assert!(matches!(
            snapshot.get(&request.id).unwrap().owner,
            SettingOwner::Extension(_)
        ));
        assert_eq!(
            apply_edit(7, &base, &disabled, &[], &request),
            Err(SettingsError::UnknownSetting)
        );
        assert_eq!(
            apply_edit(8, &base, &disabled, &installed(), &request),
            Err(SettingsError::StaleRevision)
        );
    }

    #[test]
    fn information_bar_presets_are_available_in_the_information_tags_category() {
        let snapshot =
            catalog(7, &Config::default(), &UserPreferences::default(), &[]).unwrap();
        let format_id = settings::SettingId::new("tags.format").unwrap();
        let format = snapshot
            .get(&format_id)
            .expect("information bar format is a customization control");
        assert_eq!(
            format.value,
            SettingValue::Choice("rounded-capsules".into())
        );
        let settings::SettingKind::Choice { options } = &format.kind else {
            panic!("information bar format must be a choice control");
        };
        let presets: std::collections::BTreeSet<_> =
            options.iter().map(|option| option.value.as_str()).collect();
        for preset in automexia_ui_model::information_bar::InformationBarPreset::ALL {
            assert!(
                presets.contains(preset.id()),
                "missing preset: {}",
                preset.id()
            );
        }
        let groups = customization_groups(&snapshot);
        assert!(groups[0].members.contains(&format_id));
    }

    #[test]
    fn slot_customization_is_reachable_and_changes_only_its_own_recipe_slot() {
        let base = Config::default();
        let initial = UserPreferences::default();
        let inventory = catalog(7, &base, &initial, &[]).unwrap();
        let groups = customization_groups(&inventory);
        assert!(groups.iter().any(|group| group.label == "Information tags"));
        let windows = tag_actions(&initial, &base)
            .into_iter()
            .find(|action| action.label == "Tag slot: Windows")
            .expect("Windows slot has a short customization page");
        assert_eq!(windows.id.as_str(), "tags.slot.windows.page");
        let detail = slot_page_catalog(
            7,
            &slot_page_snapshot(&initial, &initial.apply_to(&base)),
            "windows",
        )
        .unwrap();
        assert!(detail
            .get(&SettingId::new("tags.slot.windows.text").unwrap())
            .is_some());
        assert!(detail
            .get(&SettingId::new("tags.slot.windows.icon").unwrap())
            .is_some());
        let changed = apply_edit(
            7,
            &base,
            &initial,
            &[],
            &edit(
                "tags.slot.windows.text",
                Change::Set(SettingValue::Choice("user".into())),
            ),
        )
        .unwrap();
        assert!(changed.visual.information_bar.use_custom);
        let recipe = changed.visual.information_bar.recipe();
        let windows = recipe
            .slots
            .iter()
            .find(|slot| slot.id == "windows")
            .unwrap();
        assert_eq!(
            windows.text,
            automexia_ui_model::information_bar::BarTextSource::Role(
                automexia_extension_api::SegmentRole::User
            )
        );
        assert_eq!(
            windows.icon,
            automexia_ui_model::information_bar::BarIconSource::TextSource
        );
        assert_eq!(
            recipe
                .slots
                .iter()
                .find(|slot| slot.id == "git")
                .unwrap()
                .text,
            automexia_ui_model::information_bar::BarTextSource::Role(
                automexia_extension_api::SegmentRole::Git
            )
        );
    }

    #[test]
    fn inherited_slot_color_preview_tracks_the_rendered_text_source() {
        let mut base = Config::default();
        base.presentation.tags.colors.windows = Some(Rgb::from_bytes([21, 31, 41]));
        base.presentation.tags.colors.user = Some(Rgb::from_bytes([131, 141, 151]));
        base.colors.foreground = [0.2, 0.4, 0.6, 1.0];
        let initial = UserPreferences::default();
        let user_text = apply_edit(
            7,
            &base,
            &initial,
            &[],
            &edit(
                "tags.slot.windows.text",
                Change::Set(SettingValue::Choice("user".into())),
            ),
        )
        .unwrap();
        let user_text = apply_edit(
            7,
            &base,
            &user_text,
            &[],
            &edit(
                "tags.slot.windows.icon-fixed",
                Change::Set(SettingValue::Choice("windows".into())),
            ),
        )
        .unwrap();
        let catalog = slot_page_catalog(
            7,
            &slot_page_snapshot(&user_text, &user_text.apply_to(&base)),
            "windows",
        )
        .unwrap();
        let color = catalog
            .get(&SettingId::new("tags.slot.windows.color").unwrap())
            .unwrap();
        assert_eq!(color.value, SettingValue::Color([131, 141, 151, 255]));
        assert_eq!(color.default, SettingValue::Color([131, 141, 151, 255]));

        let literal = apply_edit(
            7,
            &base,
            &user_text,
            &[],
            &edit(
                "tags.slot.windows.text",
                Change::Set(SettingValue::Choice("literal".into())),
            ),
        )
        .unwrap();
        let catalog = slot_page_catalog(
            7,
            &slot_page_snapshot(&literal, &literal.apply_to(&base)),
            "windows",
        )
        .unwrap();
        assert_eq!(
            catalog
                .get(&SettingId::new("tags.slot.windows.color").unwrap())
                .unwrap()
                .value,
            SettingValue::Color([51, 102, 153, 255]),
        );
    }

    #[test]
    fn slot_edit_supports_independent_icon_text_visibility_order_and_safe_literal() {
        use automexia_ui_model::information_bar::{BarIconSource, BarTextSource};
        let base = Config::default();
        let mut prefs = UserPreferences::default();
        for (id, change) in [
            (
                "tags.slot.windows.text",
                Change::Set(SettingValue::Choice("user".into())),
            ),
            (
                "tags.slot.windows.icon-fixed",
                Change::Set(SettingValue::Choice("windows".into())),
            ),
            (
                "tags.slot.windows.order",
                Change::Set(SettingValue::Number(1.0)),
            ),
            (
                "tags.slot.windows.enabled",
                Change::Set(SettingValue::Boolean(false)),
            ),
        ] {
            prefs = apply_edit(7, &base, &prefs, &[], &edit(id, change)).unwrap();
        }
        let recipe = prefs.visual.information_bar.recipe();
        assert_eq!(recipe.slots[0].id, "windows");
        assert!(!recipe.slots[0].enabled);
        assert_eq!(
            recipe.slots[0].text,
            BarTextSource::Role(automexia_extension_api::SegmentRole::User)
        );
        assert_eq!(
            recipe.slots[0].icon,
            BarIconSource::Fixed(automexia_extension_api::IconKind::Windows)
        );
        assert_eq!(
            recipe
                .slots
                .iter()
                .filter(|slot| slot.id == "windows")
                .count(),
            1
        );
        let visible = apply_edit(
            7,
            &base,
            &prefs,
            &[],
            &edit(
                "tags.slot.windows.enabled",
                Change::Set(SettingValue::Boolean(true)),
            ),
        )
        .unwrap();
        assert!(visible.visual.information_bar.recipe().slots[0].enabled);
        let literal = apply_edit(
            7,
            &base,
            &visible,
            &[],
            &edit(
                "tags.slot.windows.literal",
                Change::Set(SettingValue::Text("Mon utilisateur".into())),
            ),
        )
        .unwrap();
        assert_eq!(
            literal.visual.information_bar.recipe().slots[0].text,
            BarTextSource::Literal("Mon utilisateur".into())
        );
        let reset = apply_edit(
            7,
            &base,
            &literal,
            &[],
            &edit("tags.slot.windows.literal", Change::Reset),
        )
        .unwrap();
        assert_eq!(
            reset.visual.information_bar.recipe().slots[0].text,
            BarTextSource::Role(automexia_extension_api::SegmentRole::Windows)
        );
        assert_eq!(
            apply_edit(
                7,
                &base,
                &literal,
                &[],
                &edit(
                    "tags.slot.windows.literal",
                    Change::Set(SettingValue::Text("\u{001b}[31m".into()))
                )
            ),
            Err(SettingsError::InvalidValue)
        );
    }

    #[test]
    fn slot_pages_and_catalog_stay_bounded_with_extension_rows() {
        let base = Config::default();
        let preferences = UserPreferences::default();
        let snapshot = catalog(7, &base, &preferences, &installed()).unwrap();
        assert!(snapshot.entries().len() <= settings::MAX_SETTINGS);
        let groups = customization_groups(&snapshot);
        assert_eq!(groups.len(), 13);
        assert_eq!(
            groups
                .iter()
                .filter(|g| g.key.as_str().starts_with("interface."))
                .count(),
            4
        );
        assert!(groups
            .iter()
            .any(|group| group.key.as_str() == "profiles.open"));
        let actions = tag_actions(&preferences, &base);
        assert_eq!(actions.len(), 13);
        assert!(actions
            .iter()
            .all(|action| action.label.starts_with("Tag slot: ")));
        assert_eq!(
            slot_page_catalog(
                7,
                &slot_page_snapshot(&UserPreferences::default(), &base),
                "windows"
            )
            .unwrap()
            .entries()
            .len(),
            11
        );
        assert!(customization_root_catalog(&snapshot, &groups).is_ok());
    }

    #[test]
    fn lazy_slot_pages_leave_catalog_room_for_near_capacity_extension_features() {
        let base = Config::default();
        let preferences = UserPreferences::default();
        let initial = catalog(7, &base, &preferences, &[]).unwrap();
        let mut entries = initial.entries().to_vec();
        for index in 0..180 {
            let mut row = settings::SettingDescriptor::boolean(
                SettingId::new(format!("extension.fixture.feature{index}")).unwrap(),
                settings::Section::Customizations,
                format!("Feature {index}"),
                "Installed extension feature",
                true,
                true,
            );
            row.owner = SettingOwner::Extension("fixture".into());
            entries.push(row);
        }
        let source = Catalog::new(7, entries)
            .expect("slot pages must not crowd out installed features");
        let groups = customization_groups(&source);
        assert_eq!(
            groups
                .iter()
                .filter(|group| group.label.starts_with("Feature "))
                .count(),
            180
        );
        assert_eq!(tag_actions(&preferences, &base).len(), 13);
        assert!(source
            .get(&SettingId::new("tags.slot.windows.page").unwrap())
            .is_none());
        assert!(customization_root_catalog(&source, &groups).is_ok());
        let detail =
            slot_page_catalog(7, &slot_page_snapshot(&preferences, &base), "windows")
                .unwrap();
        assert_eq!(detail.entries().len(), 11);
        assert!(detail
            .get(&SettingId::new("tags.slot.windows.literal").unwrap())
            .is_some());
    }

    #[test]
    fn custom_tags_can_be_added_edited_and_removed_without_losing_other_slots() {
        let base = Config::default();
        let initial = UserPreferences::default();
        let added = apply_edit(
            7,
            &base,
            &initial,
            &[],
            &edit("tags.add-slot", Change::Activate),
        )
        .unwrap();
        let recipe = added.visual.information_bar.recipe();
        assert_eq!(recipe.slots.last().unwrap().id, "custom-1");
        assert!(tag_actions(&added, &base)
            .iter()
            .any(|action| action.label == "Custom tag 1"));
        let edited = apply_edit(
            7,
            &base,
            &added,
            &[],
            &edit(
                "tags.slot.custom-1.literal",
                Change::Set(SettingValue::Text("Build complete".into())),
            ),
        )
        .unwrap();
        assert_eq!(
            edited
                .visual
                .information_bar
                .recipe()
                .slots
                .last()
                .unwrap()
                .text,
            automexia_ui_model::information_bar::BarTextSource::Literal(
                "Build complete".into()
            )
        );
        let removed = apply_edit(
            7,
            &base,
            &edited,
            &[],
            &edit("tags.slot.custom-1.remove", Change::Activate),
        )
        .unwrap();
        assert_eq!(removed.visual.information_bar.recipe().slots.len(), 13);
        assert!(!tag_actions(&removed, &base)
            .iter()
            .any(|action| action.label == "Custom tag 1"));
        assert_eq!(
            removed.visual.information_bar.recipe().slots[0].id,
            "production"
        );
    }

    #[test]
    fn slot_source_modes_restore_saved_literal_and_icon_choices() {
        use automexia_extension_api::{IconKind, SegmentRole};
        use automexia_ui_model::information_bar::{BarIconSource, BarTextSource};
        let base = Config::default();
        let mut saved = UserPreferences::default();
        for (id, change) in [
            (
                "tags.slot.windows.literal",
                Change::Set(SettingValue::Text("My OS".into())),
            ),
            (
                "tags.slot.windows.text",
                Change::Set(SettingValue::Choice("git".into())),
            ),
            (
                "tags.slot.windows.text",
                Change::Set(SettingValue::Choice("literal".into())),
            ),
            (
                "tags.slot.windows.icon",
                Change::Set(SettingValue::Choice("fixed".into())),
            ),
            (
                "tags.slot.windows.icon-fixed",
                Change::Set(SettingValue::Choice("docker".into())),
            ),
            (
                "tags.slot.windows.icon",
                Change::Set(SettingValue::Choice("context".into())),
            ),
            (
                "tags.slot.windows.icon-context",
                Change::Set(SettingValue::Choice("kubernetes".into())),
            ),
            (
                "tags.slot.windows.icon",
                Change::Set(SettingValue::Choice("fixed".into())),
            ),
        ] {
            saved = apply_edit(
                7,
                &base,
                &saved,
                &crate::settings_catalog::test_installed_extensions(),
                &edit(id, change),
            )
            .unwrap();
        }
        let slot = saved
            .visual
            .information_bar
            .recipe()
            .slots
            .into_iter()
            .find(|slot| slot.id == "windows")
            .unwrap();
        assert_eq!(slot.text, BarTextSource::Literal("My OS".into()));
        assert_eq!(slot.icon, BarIconSource::Fixed(IconKind::Docker));
        assert_eq!(
            saved.visual.information_bar.source_drafts["windows"].icon_role,
            Some(SegmentRole::Kubernetes)
        );
        let context = apply_edit(
            7,
            &base,
            &saved,
            &crate::settings_catalog::test_installed_extensions(),
            &edit(
                "tags.slot.windows.icon",
                Change::Set(SettingValue::Choice("context".into())),
            ),
        )
        .unwrap();
        let slot = context
            .visual
            .information_bar
            .recipe()
            .slots
            .into_iter()
            .find(|slot| slot.id == "windows")
            .unwrap();
        assert_eq!(slot.icon, BarIconSource::Role(SegmentRole::Kubernetes));
    }

    #[test]
    fn removing_custom_slot_discards_its_remembered_sources() {
        let base = Config::default();
        let added = apply_edit(
            7,
            &base,
            &UserPreferences::default(),
            &[],
            &edit("tags.add-slot", Change::Activate),
        )
        .unwrap();
        let remembered = apply_edit(
            7,
            &base,
            &added,
            &[],
            &edit(
                "tags.slot.custom-1.literal",
                Change::Set(SettingValue::Text("Saved".into())),
            ),
        )
        .unwrap();
        let remembered = apply_edit(
            7,
            &base,
            &remembered,
            &[],
            &edit(
                "tags.slot.custom-1.text",
                Change::Set(SettingValue::Choice("user".into())),
            ),
        )
        .unwrap();
        assert!(remembered
            .visual
            .information_bar
            .source_drafts
            .contains_key("custom-1"));
        let removed = apply_edit(
            7,
            &base,
            &remembered,
            &[],
            &edit("tags.slot.custom-1.remove", Change::Activate),
        )
        .unwrap();
        assert!(!removed
            .visual
            .information_bar
            .source_drafts
            .contains_key("custom-1"));
    }

    #[test]
    fn icon_only_coercion_restores_prior_icon_mode_unless_explicitly_changed() {
        use automexia_ui_model::information_bar::{BarIconSource, BarTextSource};
        let base = Config::default();
        let initial = UserPreferences::default();
        let icon_only = apply_edit(
            7,
            &base,
            &initial,
            &[],
            &edit(
                "tags.slot.windows.text",
                Change::Set(SettingValue::Choice("none".into())),
            ),
        )
        .unwrap();
        assert!(matches!(
            icon_only
                .visual
                .information_bar
                .recipe()
                .slots
                .into_iter()
                .find(|slot| slot.id == "windows")
                .unwrap()
                .icon,
            BarIconSource::Role(_)
        ));
        let restored = apply_edit(
            7,
            &base,
            &icon_only,
            &[],
            &edit(
                "tags.slot.windows.text",
                Change::Set(SettingValue::Choice("windows".into())),
            ),
        )
        .unwrap();
        let slot = restored
            .visual
            .information_bar
            .recipe()
            .slots
            .into_iter()
            .find(|slot| slot.id == "windows")
            .unwrap();
        assert_eq!(
            slot.text,
            BarTextSource::Role(automexia_extension_api::SegmentRole::Windows)
        );
        assert_eq!(slot.icon, BarIconSource::TextSource);
        let explicit = apply_edit(
            7,
            &base,
            &icon_only,
            &[],
            &edit(
                "tags.slot.windows.icon",
                Change::Set(SettingValue::Choice("fixed".into())),
            ),
        )
        .unwrap();
        let explicit = apply_edit(
            7,
            &base,
            &explicit,
            &[],
            &edit(
                "tags.slot.windows.text",
                Change::Set(SettingValue::Choice("windows".into())),
            ),
        )
        .unwrap();
        assert!(matches!(
            explicit
                .visual
                .information_bar
                .recipe()
                .slots
                .into_iter()
                .find(|slot| slot.id == "windows")
                .unwrap()
                .icon,
            BarIconSource::Fixed(_)
        ));
    }

    #[test]
    fn editing_standard_slot_preserves_loaded_extra_recipe_slots() {
        let base = Config::default();
        let mut preferences = UserPreferences::default();
        let mut recipe = preferences.visual.information_bar.recipe();
        let mut extra = custom_slot(1);
        extra.id = "imported-note".into();
        extra.text = automexia_ui_model::information_bar::BarTextSource::Literal(
            "Retained note".into(),
        );
        recipe.slots.push(extra.clone());
        preferences.visual.information_bar.custom_recipe = Some(recipe);
        preferences.visual.information_bar.use_custom = true;
        let changed = apply_edit(
            7,
            &base,
            &preferences,
            &[],
            &edit(
                "tags.slot.windows.enabled",
                Change::Set(SettingValue::Boolean(false)),
            ),
        )
        .unwrap();
        assert_eq!(
            changed.visual.information_bar.recipe().slots.last(),
            Some(&extra)
        );
        assert!(catalog(7, &base, &changed, &[]).is_ok());
    }

    #[test]
    fn icon_only_choice_and_icon_reset_keep_recipe_valid() {
        let base = Config::default();
        let initial = UserPreferences::default();
        let icon_only = apply_edit(
            7,
            &base,
            &initial,
            &[],
            &edit(
                "tags.slot.windows.text",
                Change::Set(SettingValue::Choice("none".into())),
            ),
        )
        .unwrap();
        assert!(automexia_ui_model::information_bar::validate_recipe(
            &icon_only.visual.information_bar.recipe()
        )
        .is_ok());
        let reset = apply_edit(
            7,
            &base,
            &icon_only,
            &[],
            &edit("tags.slot.windows.icon", Change::Reset),
        )
        .unwrap();
        let windows = reset
            .visual
            .information_bar
            .recipe()
            .slots
            .into_iter()
            .find(|slot| slot.id == "windows")
            .unwrap();
        assert_eq!(
            windows.text,
            automexia_ui_model::information_bar::BarTextSource::Role(
                automexia_extension_api::SegmentRole::Windows
            )
        );
        assert_eq!(
            windows.icon,
            automexia_ui_model::information_bar::BarIconSource::TextSource
        );
    }

    #[test]
    fn information_bar_format_edit_updates_recipe_and_reset_keeps_custom_draft() {
        use automexia_ui_model::information_bar::{preset_recipe, InformationBarPreset};

        let base = Config::default();
        let default = UserPreferences::default();
        let selected = apply_edit(
            7,
            &base,
            &default,
            &[],
            &edit(
                crate::automexia::presentation::TAG_FORMAT,
                Change::Set(SettingValue::Choice("git-first-developer-bar".into())),
            ),
        )
        .unwrap();
        assert_eq!(
            selected.visual.information_bar.recipe(),
            preset_recipe(InformationBarPreset::GitFirstDeveloperBar)
        );
        assert_eq!(
            catalog(7, &base, &selected, &[])
                .unwrap()
                .get(&SettingId::new("tags.format").unwrap())
                .unwrap()
                .value,
            SettingValue::Choice("git-first-developer-bar".into())
        );
        assert_eq!(
            apply_edit(
                7,
                &base,
                &selected,
                &[],
                &edit(
                    crate::automexia::presentation::TAG_FORMAT,
                    Change::Set(SettingValue::Choice("custom".into())),
                ),
            ),
            Err(SettingsError::InvalidValue)
        );
        let mut with_draft = selected.clone();
        with_draft.visual.information_bar.custom_recipe =
            Some(preset_recipe(InformationBarPreset::FloatingCards));
        let custom = apply_edit(
            7,
            &base,
            &with_draft,
            &[],
            &edit(
                crate::automexia::presentation::TAG_FORMAT,
                Change::Set(SettingValue::Choice("custom".into())),
            ),
        )
        .unwrap();
        assert!(custom.visual.information_bar.use_custom);
        let reset = apply_edit(
            7,
            &base,
            &custom,
            &[],
            &edit(crate::automexia::presentation::TAG_FORMAT, Change::Reset),
        )
        .unwrap();
        assert_eq!(
            reset.visual.information_bar.recipe(),
            preset_recipe(InformationBarPreset::RoundedCapsules)
        );
        assert!(!reset.visual.information_bar.use_custom);
        assert_eq!(
            reset.visual.information_bar.custom_recipe,
            with_draft.visual.information_bar.custom_recipe
        );
    }

    #[test]
    fn linked_arrows_are_selectable_with_independent_tag_spacing() {
        let base = Config::default();
        let initial = UserPreferences::default();
        let mut changed = apply_edit(
            7,
            &base,
            &initial,
            &[],
            &edit(
                "tags.bar-style",
                Change::Set(SettingValue::Choice("linked-arrows".into())),
            ),
        )
        .unwrap();
        for spacing in [0.0, 100.0, 300.0] {
            changed = apply_edit(
                7,
                &base,
                &changed,
                &[],
                &edit("tags.spacing", Change::Set(SettingValue::Number(spacing))),
            )
            .unwrap();
            let inventory = catalog(7, &base, &changed, &[]).unwrap();
            assert_eq!(
                inventory
                    .get(&SettingId::new("tags.bar-style").unwrap())
                    .unwrap()
                    .value,
                SettingValue::Choice("linked-arrows".into())
            );
            assert_eq!(
                changed.visual.information_bar.recipe().spacing_percent,
                spacing as u16
            );
        }
        assert_eq!(initial, UserPreferences::default());
    }

    #[test]
    fn all_reference_shapes_keep_their_spacing_and_reset_through_the_catalog() {
        use automexia_ui_model::information_bar::BarVisualStyle;
        let base = Config::default();
        let initial = UserPreferences::default();
        for style in BarVisualStyle::ALL {
            let selected = apply_edit(
                7,
                &base,
                &initial,
                &[],
                &edit(
                    "tags.bar-style",
                    Change::Set(SettingValue::Choice(style.id().into())),
                ),
            )
            .unwrap();
            for gap in [0.0, 100.0, 300.0] {
                let spaced = apply_edit(
                    7,
                    &base,
                    &selected,
                    &[],
                    &edit("tags.spacing", Change::Set(SettingValue::Number(gap))),
                )
                .unwrap();
                let recipe = spaced.visual.information_bar.recipe();
                assert_eq!(recipe.visual, style);
                assert_eq!(recipe.spacing_percent, gap as u16);
                let reset = apply_edit(
                    7,
                    &base,
                    &spaced,
                    &[],
                    &edit("tags.bar-style", Change::Reset),
                )
                .unwrap();
                assert_eq!(
                    reset.visual.information_bar.recipe().spacing_percent,
                    gap as u16
                );
                assert_eq!(
                    reset.visual.information_bar.recipe().visual,
                    initial.visual.information_bar.recipe().visual
                );
            }
        }
    }

    #[test]
    fn information_header_spacing_is_editable_bounded_and_preserved_across_formats() {
        let base = Config::default();
        let initial = UserPreferences::default();
        let spacing_id = SettingId::new("tags.spacing").unwrap();
        let inventory = catalog(7, &base, &initial, &[]).unwrap();
        assert!(customization_groups(&inventory)[0]
            .members
            .contains(&spacing_id));
        assert_eq!(
            inventory.get(&spacing_id).unwrap().value,
            SettingValue::Number(100.0)
        );

        let changed = apply_edit(
            7,
            &base,
            &initial,
            &[],
            &edit("tags.spacing", Change::Set(SettingValue::Number(165.0))),
        )
        .unwrap();
        assert_eq!(changed.visual.information_bar.recipe().spacing_percent, 165);
        assert!(changed.visual.information_bar.use_custom);
        assert_eq!(
            apply_edit(
                7,
                &base,
                &changed,
                &[],
                &edit("tags.spacing", Change::Set(SettingValue::Number(301.0))),
            ),
            Err(SettingsError::InvalidValue)
        );

        let selected = apply_edit(
            7,
            &base,
            &changed,
            &[],
            &edit(
                crate::automexia::presentation::TAG_FORMAT,
                Change::Set(SettingValue::Choice("floating-cards".into())),
            ),
        )
        .unwrap();
        assert_eq!(
            selected.visual.information_bar.recipe().spacing_percent,
            100
        );
        let restored = apply_edit(
            7,
            &base,
            &selected,
            &[],
            &edit(
                crate::automexia::presentation::TAG_FORMAT,
                Change::Set(SettingValue::Choice("custom".into())),
            ),
        )
        .unwrap();
        assert_eq!(
            restored.visual.information_bar.recipe().spacing_percent,
            165
        );
        let reset = apply_edit(
            7,
            &base,
            &restored,
            &[],
            &edit("tags.spacing", Change::Reset),
        )
        .unwrap();
        assert_eq!(reset.visual.information_bar.recipe().spacing_percent, 100);
    }

    #[test]
    fn highlight_backgrounds_have_direct_numeric_opacity_controls() {
        let full =
            catalog(1, &Config::default(), &UserPreferences::default(), &[]).unwrap();
        for (domain, statuses) in [
            ("command_output", &["success", "failure", "neutral"][..]),
            (
                "output",
                &["error", "warning", "success", "info", "debug"][..],
            ),
            (
                "kubernetes",
                &["error", "warning", "success", "info", "debug"][..],
            ),
        ] {
            for status in statuses {
                let id = SettingId::new(format!("{domain}.opacity.{status}")).unwrap();
                let row = full.get(&id).expect("missing explicit opacity control");
                assert_eq!(
                    row.kind,
                    settings::SettingKind::Number {
                        min: 0.0,
                        max: 100.0,
                        step: 1.0
                    }
                );
            }
        }
    }

    #[test]
    fn status_opacity_reports_no_implicit_background_in_both_style() {
        let mut base = Config::default();
        let initial = UserPreferences::default();
        for domain in ["output", "kubernetes"] {
            let id = SettingId::new(format!("{domain}.opacity.success")).unwrap();
            base.presentation.highlight.style = HighlightStyle::Both;
            base.presentation.kubernetes.style = HighlightStyle::Both;
            assert_eq!(
                catalog(1, &base, &initial, &[])
                    .unwrap()
                    .get(&id)
                    .unwrap()
                    .value,
                SettingValue::Number(0.0),
                "a text-only default must not claim visible background opacity"
            );
            base.presentation.highlight.style = HighlightStyle::Background;
            base.presentation.kubernetes.style = HighlightStyle::Background;
            assert_eq!(
                catalog(2, &base, &initial, &[])
                    .unwrap()
                    .get(&id)
                    .unwrap()
                    .value,
                SettingValue::Number(31.0)
            );
        }
    }

    #[test]
    fn opacity_edits_preserve_rgb_other_domains_and_saved_configuration() {
        let mut base = Config::default();
        base.presentation.command_output.success = Some(Rgba::from_bytes([7, 8, 9, 71]));
        base.presentation.highlight.error_background =
            Some(Rgba::from_bytes([11, 12, 13, 91]));
        base.presentation.kubernetes.warning_background =
            Some(Rgba::from_bytes([17, 18, 19, 111]));
        for (domain, status, rgb, default_alpha) in [
            ("command_output", "success", [7, 8, 9], 71),
            ("output", "error", [11, 12, 13], 91),
            ("kubernetes", "warning", [17, 18, 19], 111),
        ] {
            let id = SettingId::new(format!("{domain}.opacity.{status}")).unwrap();
            let color_id =
                SettingId::new(format!("{domain}.backgrounds.{status}")).unwrap();
            let initial = UserPreferences::default();
            for percent in [0.0, 1.0, 25.0, 50.0, 99.0, 100.0] {
                let edited = apply_edit(
                    1,
                    &base,
                    &initial,
                    &[],
                    &Edit {
                        revision: 1,
                        id: id.clone(),
                        change: Change::Set(SettingValue::Number(percent)),
                    },
                )
                .unwrap();
                let full = catalog(2, &base, &edited, &[]).unwrap();
                let alpha = (percent * 255.0 / 100.0).round() as u8;
                assert_eq!(
                    full.get(&color_id).unwrap().value,
                    SettingValue::Color([rgb[0], rgb[1], rgb[2], alpha])
                );
                assert_eq!(full.get(&id).unwrap().value, SettingValue::Number(percent));
                let restored = apply_edit(
                    2,
                    &base,
                    &edited,
                    &[],
                    &Edit {
                        revision: 2,
                        id: id.clone(),
                        change: Change::Reset,
                    },
                )
                .unwrap();
                assert_eq!(restored, initial, "reset removes a pure alpha override");
            }
            for invalid in [-1.0, 101.0, 0.5, f64::NAN, f64::INFINITY] {
                assert!(apply_edit(
                    1,
                    &base,
                    &initial,
                    &[],
                    &Edit {
                        revision: 1,
                        id: id.clone(),
                        change: Change::Set(SettingValue::Number(invalid)),
                    }
                )
                .is_err());
            }
            let custom = apply_edit(
                1,
                &base,
                &initial,
                &[],
                &Edit {
                    revision: 1,
                    id: color_id.clone(),
                    change: Change::Set(SettingValue::Color([21, 22, 23, 0])),
                },
            )
            .unwrap();
            let reset = apply_edit(
                2,
                &base,
                &custom,
                &[],
                &Edit {
                    revision: 2,
                    id,
                    change: Change::Reset,
                },
            )
            .unwrap();
            assert_eq!(
                catalog(3, &base, &reset, &[])
                    .unwrap()
                    .get(&color_id)
                    .unwrap()
                    .value,
                SettingValue::Color([21, 22, 23, default_alpha])
            );
        }
        assert_eq!(
            base.presentation.command_output.success.unwrap().bytes(),
            [7, 8, 9, 71]
        );
        assert_eq!(
            base.presentation
                .highlight
                .error_background
                .unwrap()
                .bytes(),
            [11, 12, 13, 91]
        );
        assert_eq!(
            base.presentation
                .kubernetes
                .warning_background
                .unwrap()
                .bytes(),
            [17, 18, 19, 111]
        );
    }

    #[test]
    fn preview_element_catalogs_reuse_only_admitted_color_descriptors() {
        let full =
            catalog(7, &Config::default(), &UserPreferences::default(), &[]).unwrap();
        for binding in &crate::automexia::presentation::TAG_COLOR_BINDINGS {
            let role = binding.id.strip_prefix("tags.colors.").unwrap();
            let page = tag_role_color_catalog(&full, role).unwrap();
            assert_eq!(page.revision(), full.revision());
            assert_eq!(page.entries().len(), 1);
            assert_eq!(page.entries()[0].id.as_str(), binding.id);
            assert_eq!(page.entries()[0].label, "Default role color");
        }
        for severity in ["error", "warning", "success", "info", "debug"] {
            let page = output_severity_catalog(&full, severity).unwrap();
            assert_eq!(page.revision(), full.revision());
            assert_eq!(page.entries().len(), 3);
            assert_eq!(
                page.entries()[0].id.as_str(),
                format!("output.colors.{severity}")
            );
            assert_eq!(
                page.entries()[1].id.as_str(),
                format!("output.backgrounds.{severity}")
            );
        }
        for invalid in ["", "other", "error.hidden", "../error"] {
            assert!(matches!(
                output_severity_catalog(&full, invalid),
                Err(SettingsError::UnknownSetting)
            ));
            assert!(matches!(
                tag_role_color_catalog(&full, invalid),
                Err(SettingsError::UnknownSetting)
            ));
        }
    }

    #[test]
    fn every_standard_text_and_icon_context_resolves_its_inherited_color_control() {
        use automexia_ui_model::information_bar::{role_id, STANDARD_ROLES};

        let base = Config::default();
        for role in STANDARD_ROLES {
            let role_id = role_id(role);
            let binding = crate::automexia::presentation::TAG_COLOR_BINDINGS
                .iter()
                .find(|binding| binding.role == role)
                .unwrap();
            let original = UserPreferences::default();
            for source in ["text", "icon-context"] {
                let prepared = if source == "icon-context" {
                    apply_edit(
                        7,
                        &base,
                        &original,
                        &crate::settings_catalog::test_installed_extensions(),
                        &edit(
                            "tags.slot.windows.text",
                            Change::Set(SettingValue::Choice("none".into())),
                        ),
                    )
                    .unwrap()
                } else {
                    original.clone()
                };
                let selected = apply_edit(
                    7,
                    &base,
                    &prepared,
                    &crate::settings_catalog::test_installed_extensions(),
                    &edit(
                        &format!("tags.slot.windows.{source}"),
                        Change::Set(SettingValue::Choice(role_id.into())),
                    ),
                )
                .unwrap();
                let full = catalog(
                    7,
                    &base,
                    &selected,
                    &crate::settings_catalog::test_installed_extensions(),
                )
                .unwrap();
                let snapshot = slot_page_snapshot(&selected, &selected.apply_to(&base));
                let detail = selected_tag_catalog(&full, &snapshot, "windows").unwrap();
                assert!(
                    detail.get(&SettingId::new(binding.id).unwrap()).is_some(),
                    "{source} source {role_id} lost its inherited color control"
                );
                let direct = tag_role_color_catalog(&full, role_id).unwrap();
                assert_eq!(direct.entries()[0].id.as_str(), binding.id);
            }
        }
    }

    #[test]
    fn customization_projection_groups_one_feature_per_root_action_without_duplicate_members(
    ) {
        let base = Config::default();
        let prefs = UserPreferences::default();
        let snapshot = catalog(7, &base, &prefs, &installed()).unwrap();
        let groups = customization_groups(&snapshot);
        assert_eq!(
            groups.first().map(|group| group.label.as_str()),
            Some("Information tags")
        );
        assert!(groups.iter().all(|group| group.label != "DevOps detection"));
        assert!(groups.iter().all(|group| group.label != "Git branch tag"));
        assert_eq!(groups.len(), 13);
        assert_eq!(
            groups
                .iter()
                .filter(|g| g.key.as_str().starts_with("interface."))
                .count(),
            4
        );
        assert!(groups
            .iter()
            .any(|group| group.key.as_str() == "profiles.open"));
        assert_eq!(tag_actions(&prefs, &base).len(), 13);
        assert_eq!(groups[0].members[0].as_str(), "tags.enabled");
        assert_eq!(
            groups[0].members[1].as_str(),
            settings_extensions::DEVOPS_CONTEXT_STATUS_ID
        );
        assert_eq!(groups[0].members.len(), 9);
        assert!(groups[0]
            .members
            .iter()
            .any(|id| { id.as_str() == settings_extensions::DEVOPS_CONTEXT_STATUS_ID }));
        assert!(groups[0]
            .members
            .iter()
            .all(|id| !id.as_str().starts_with("tags.colors.")));
        assert!(snapshot
            .get(&SettingId::new("tags.colors.kubernetes").unwrap())
            .is_some());
        assert!(groups[0].root_action.description.contains("Currently on"));
        let output = groups
            .iter()
            .find(|group| group.label == "Terminal output colors")
            .unwrap();
        assert!(output
            .root_action
            .description
            .contains("Commands on · Highlights on"));
        assert_eq!(
            output.members[0].as_str(),
            settings::COMMAND_OUTPUT_HIGHLIGHTING
        );
        assert_eq!(output.members.len(), 4);
        assert!(output
            .members
            .iter()
            .any(|id| id.as_str() == settings::OUTPUT_HIGHLIGHTING));
        assert!(output.members.iter().all(|id| {
            !id.as_str().starts_with("output.colors.")
                && !id.as_str().starts_with("output.backgrounds.")
        }));
        assert!(snapshot
            .get(&SettingId::new("output.colors.error").unwrap())
            .is_some());
        assert_eq!(
            groups
                .iter()
                .find(|group| group.label == "Inline tables")
                .unwrap()
                .members[0]
                .as_str(),
            settings::INLINE_TABLES
        );
        assert_eq!(
            groups
                .iter()
                .find(|group| group.label == "Command timestamps")
                .unwrap()
                .members[0]
                .as_str(),
            settings::COMMAND_TIMESTAMPS
        );
        let git = selected_tag_catalog(
            &snapshot,
            &slot_page_snapshot(&prefs, &prefs.apply_to(&base)),
            "git",
        )
        .unwrap();
        assert!(git
            .get(&SettingId::new(settings_extensions::DEVOPS_GIT_STATUS_ID).unwrap())
            .is_some());
        let mut seen = std::collections::BTreeSet::new();
        for group in &groups {
            assert_eq!(group.key, group.root_action.id);
            assert!(matches!(
                group.root_action.kind,
                settings::SettingKind::Action
            ));
            assert!(group.members.iter().all(|id| snapshot.get(id).is_some()));
            assert!(group.members.iter().all(|id| seen.insert(id.as_str())));
        }
        // The navigation actions are valid rows in their own root catalogue,
        // but the app must intercept them rather than submit setting edits.
        let roots = Catalog::new(
            snapshot.revision(),
            groups
                .iter()
                .map(|group| group.root_action.clone())
                .collect(),
        )
        .unwrap();
        assert_eq!(roots.entries().len(), groups.len());
    }

    #[test]
    fn customization_projection_follows_installed_features_and_retains_disabled_controls()
    {
        let base = Config::default();
        let mut prefs = UserPreferences::default();
        prefs.presentation.command_output_highlighting = Some(false);
        prefs
            .set_extension_feature_enabled(
                settings_extensions::DEVOPS_CONTEXT_STATUS_ID,
                false,
            )
            .unwrap();
        let with_extension = catalog(7, &base, &prefs, &installed()).unwrap();
        let groups = customization_groups(&with_extension);
        assert!(groups[0].root_action.description.contains("Currently on"));
        assert!(groups
            .iter()
            .find(|group| group.label == "Terminal output colors")
            .unwrap()
            .root_action
            .description
            .contains("Commands off · Highlights on"));
        assert!(groups.iter().all(|group| {
            group.key.as_str() != settings_extensions::DEVOPS_CONTEXT_STATUS_ID
                && group.key.as_str() != settings_extensions::DEVOPS_GIT_STATUS_ID
        }));
        assert!(groups[0]
            .members
            .iter()
            .any(|id| id.as_str() == settings_extensions::DEVOPS_CONTEXT_STATUS_ID));
        assert_eq!(
            with_extension
                .get(
                    &SettingId::new(settings_extensions::DEVOPS_CONTEXT_STATUS_ID)
                        .unwrap()
                )
                .unwrap()
                .value,
            SettingValue::Boolean(false)
        );

        let without_extension = catalog(8, &base, &prefs, &[]).unwrap();
        let groups = customization_groups(&without_extension);
        assert_eq!(groups.len(), 13);
        assert_eq!(
            groups
                .iter()
                .filter(|g| g.key.as_str().starts_with("interface."))
                .count(),
            4
        );
        assert!(groups
            .iter()
            .any(|group| group.key.as_str() == "profiles.open"));
        assert!(groups.iter().all(|group| {
            group.key.as_str() != settings_extensions::DEVOPS_CONTEXT_STATUS_ID
                && group.key.as_str() != settings_extensions::DEVOPS_GIT_STATUS_ID
        }));
        let output = groups
            .iter()
            .find(|group| group.label == "Terminal output colors")
            .unwrap();
        assert!(output
            .root_action
            .description
            .contains("Commands off · Highlights on"));
        assert!(without_extension
            .get(&output.members[0])
            .is_some_and(|row| row.availability.reason().is_none()));
    }

    #[test]
    fn git_tag_preference_is_independent_default_on_and_persists() {
        let base = Config::default();
        let market = installed();
        let original = UserPreferences::default();
        let before = catalog(7, &base, &original, &market).unwrap();
        let git_id = SettingId::new(settings_extensions::DEVOPS_GIT_STATUS_ID).unwrap();
        let context_id =
            SettingId::new(settings_extensions::DEVOPS_CONTEXT_STATUS_ID).unwrap();
        assert_eq!(
            before.get(&git_id).unwrap().value,
            SettingValue::Boolean(true)
        );
        assert_eq!(
            before.get(&context_id).unwrap().value,
            SettingValue::Boolean(true)
        );

        let changed = apply_edit(
            7,
            &base,
            &original,
            &market,
            &edit(
                settings_extensions::DEVOPS_GIT_STATUS_ID,
                Change::Set(SettingValue::Boolean(false)),
            ),
        )
        .unwrap();
        assert_eq!(
            changed.extension_feature_enabled(settings_extensions::DEVOPS_GIT_STATUS_ID),
            Some(false)
        );
        assert_eq!(
            changed
                .extension_feature_enabled(settings_extensions::DEVOPS_CONTEXT_STATUS_ID),
            None
        );
        let root = tempfile::tempdir().unwrap();
        crate::automexia::preferences::write_to_root(root.path(), &changed).unwrap();
        let restored =
            crate::automexia::preferences::load_from_root(root.path()).preferences;
        let reopened = catalog(7, &base, &restored, &market).unwrap();
        assert_eq!(
            reopened.get(&git_id).unwrap().value,
            SettingValue::Boolean(false)
        );
        assert_eq!(
            reopened.get(&context_id).unwrap().value,
            SettingValue::Boolean(true)
        );

        let reset = apply_edit(
            7,
            &base,
            &restored,
            &market,
            &edit(settings_extensions::DEVOPS_GIT_STATUS_ID, Change::Reset),
        )
        .unwrap();
        assert_eq!(reset, original);
    }

    #[test]
    fn unavailable_extension_summaries_do_not_create_empty_customization_categories() {
        let mut market = installed();
        market.push(MarketItem {
            id: "automexia.devops-aws".into(),
            name: "ignored".into(),
            description: "ignored".into(),
            installed: true,
        });
        let snapshot =
            catalog(7, &Config::default(), &UserPreferences::default(), &market).unwrap();
        let summary_id =
            SettingId::new("extension.automexia.devops-aws.availability").unwrap();
        let summary = snapshot.get(&summary_id).unwrap();
        assert_eq!(summary.section, settings::Section::Extensions);
        assert!(summary.availability.reason().is_some());
        assert!(customization_groups(&snapshot)
            .iter()
            .all(|group| group.key != summary_id));
    }

    #[test]
    fn theme_gallery_is_available_without_adaptive_configuration() {
        let snapshot =
            catalog(7, &Config::default(), &UserPreferences::default(), &[]).unwrap();
        let groups = customization_groups(&snapshot);
        let output = groups
            .iter()
            .find(|group| group.label == "Terminal output colors")
            .unwrap();
        assert!(output
            .root_action
            .description
            .contains("Commands on · Highlights on"));
        assert!(!output.root_action.description.contains("unavailable"));
        assert!(matches!(
            output.root_action.availability,
            settings::Availability::Available
        ));
        let theme = groups.iter().find(|group| group.label == "Theme").unwrap();
        assert!(matches!(
            theme.root_action.availability,
            settings::Availability::Available
        ));
        assert!(!theme.root_action.description.contains("unavailable"));
    }

    #[test]
    fn customization_projection_accepts_maximum_valid_extension_description() {
        let source = catalog(
            7,
            &Config::default(),
            &UserPreferences::default(),
            &installed(),
        )
        .unwrap();
        let mut entries = source.entries().to_vec();
        let extension = entries
            .iter_mut()
            .find(|entry| matches!(entry.owner, SettingOwner::Extension(_)))
            .unwrap();
        extension.description = "é".repeat(settings::MAX_DESCRIPTION_BYTES / 2);
        assert_eq!(extension.description.len(), settings::MAX_DESCRIPTION_BYTES);
        let extension_id = extension.id.clone();
        let source =
            Catalog::new(7, entries).expect("maximum valid extension descriptor");
        let groups = customization_groups(&source);
        let roots = Catalog::new(
            7,
            groups
                .iter()
                .map(|group| group.root_action.clone())
                .collect(),
        )
        .expect("category projection must preserve valid extension membership");
        assert!(roots.get(&extension_id).is_none());
        let information = groups
            .iter()
            .find(|group| group.label == "Information tags")
            .unwrap();
        assert!(information.members.contains(&extension_id));
        assert_eq!(
            source.get(&extension_id).unwrap().description,
            "é".repeat(settings::MAX_DESCRIPTION_BYTES / 2)
        );
    }

    #[test]
    fn near_capacity_extension_catalog_still_has_a_bounded_category_root() {
        let mut entries = Vec::new();
        for index in 0..settings::MAX_SETTINGS {
            let mut entry = settings::SettingDescriptor::boolean(
                SettingId::new(format!("extension.fixture.feature{index}")).unwrap(),
                settings::Section::Customizations,
                format!("Feature {index}"),
                "x".repeat(495),
                true,
                true,
            );
            entry.owner = SettingOwner::Extension("fixture".into());
            entry.keywords =
                vec!["k".repeat(settings::MAX_KEYWORD_BYTES); settings::MAX_KEYWORDS];
            let mut candidate = entries.clone();
            candidate.push(entry);
            if Catalog::new(7, candidate.clone()).is_err() {
                break;
            }
            entries = candidate;
        }
        assert!(entries.len() > 100);
        let source = Catalog::new(7, entries).unwrap();
        let groups = customization_groups(&source);
        assert_eq!(groups.len(), source.entries().len());
        let root = customization_root_catalog(&source, &groups)
            .expect("valid source must not fall back to a flat list");
        assert_eq!(root.entries().len(), groups.len());
    }

    #[test]
    fn information_tags_toggle_is_core_owned_independent_and_resettable() {
        let mut base = Config::default();
        base.presentation.tags.enabled = false;
        let mut preferences = UserPreferences::default();
        preferences
            .set_extension_feature_enabled(
                settings_extensions::DEVOPS_CONTEXT_STATUS_ID,
                false,
            )
            .unwrap();
        let original = preferences.clone();
        let row = catalog(7, &base, &preferences, &[])
            .unwrap()
            .get(&SettingId::new("tags.enabled").unwrap())
            .unwrap()
            .clone();
        assert_eq!(row.value, SettingValue::Boolean(false));
        assert_eq!(row.origin, ValueOrigin::Configuration);
        assert!(matches!(row.owner, SettingOwner::Core));
        assert_eq!(row.availability, settings::Availability::Available);

        let enabled = apply_edit(
            7,
            &base,
            &preferences,
            &[],
            &edit("tags.enabled", Change::Set(SettingValue::Boolean(true))),
        )
        .unwrap();
        assert_eq!(enabled.visual.tags.enabled, Some(true));
        assert!(enabled.apply_to(&base).presentation.tags.enabled);
        assert_eq!(enabled.extension_features, original.extension_features);
        let root = tempfile::tempdir().unwrap();
        crate::automexia::preferences::write_to_root(root.path(), &enabled).unwrap();
        let restored = crate::automexia::preferences::load_from_root(root.path());
        assert_eq!(restored.preferences, enabled);
        let grouped = customization_groups(&catalog(7, &base, &enabled, &[]).unwrap());
        assert!(grouped[0].root_action.description.contains("Currently on"));

        let reset = apply_edit(
            7,
            &base,
            &enabled,
            &[],
            &edit("tags.enabled", Change::Reset),
        )
        .unwrap();
        assert_eq!(reset, original);
        assert!(!reset.apply_to(&base).presentation.tags.enabled);
        assert!(
            customization_groups(&catalog(7, &base, &reset, &[]).unwrap())[0]
                .root_action
                .description
                .contains("Currently off")
        );
    }
}
/// Only a successfully loaded inventory may remove persisted installed-owner
/// overrides. A disabled feature remains installed and must remain listed.
pub(crate) fn prune_removed_extension_features(
    preferences: &mut UserPreferences,
    market: &[MarketItem],
    inventory_ready: bool,
) -> bool {
    if !inventory_ready {
        return false;
    }
    let previous_len = preferences.extension_features.len();
    preferences.extension_features.retain(|record| {
        match settings_extensions::feature_owner(&record.id) {
            Some(owner) => market.iter().any(|item| item.id == owner && item.installed),
            // Schema validation owns unknown IDs. This operation removes only
            // known installed-owner records after a confirmed inventory load.
            None => true,
        }
    });
    preferences.extension_features.len() != previous_len
}

#[cfg(test)]
mod inventory_tests {
    use super::*;
    #[test]
    fn devops_preview_admission_matches_membership_and_independent_switches() {
        let base = Config::default();
        for installed in [false, true] {
            for context in [false, true] {
                for git in [false, true] {
                    let mut prefs = UserPreferences::default();
                    prefs
                        .set_extension_feature_enabled(
                            settings_extensions::DEVOPS_CONTEXT_STATUS_ID,
                            context,
                        )
                        .unwrap();
                    prefs
                        .set_extension_feature_enabled(
                            settings_extensions::DEVOPS_GIT_STATUS_ID,
                            git,
                        )
                        .unwrap();
                    let mut market = test_installed_extensions();
                    market[0].installed = installed;
                    let snapshot =
                        slot_page_snapshot_with_config(&prefs, &base, &base, &market);
                    assert_eq!(
                        snapshot.slot_available("kubernetes"),
                        installed && context
                    );
                    assert_eq!(snapshot.slot_available("git"), installed);
                    assert_eq!(snapshot.preview_context().git, installed && git);
                    assert!(snapshot.slot_available("user"));
                    let full = catalog(1, &base, &prefs, &market).unwrap();
                    let visible = visible_controls(full.clone(), &full).unwrap();
                    for (role, enabled) in [
                        ("kubernetes", installed && context),
                        ("git", installed && git),
                        ("user", true),
                    ] {
                        assert_eq!(
                            visible
                                .get(
                                    &settings::SettingId::new(format!(
                                        "tags.colors.{role}"
                                    ))
                                    .unwrap()
                                )
                                .is_some(),
                            enabled
                        );
                    }
                    let controls = slot_page_catalog(1, &snapshot, "user").unwrap();
                    for id in ["tags.slot.user.text", "tags.slot.user.icon-context"] {
                        let settings::SettingKind::Choice { options } = &controls
                            .get(&settings::SettingId::new(id).unwrap())
                            .unwrap()
                            .kind
                        else {
                            panic!("expected source choices");
                        };
                        assert_eq!(
                            options.iter().any(|option| option.value == "kubernetes"),
                            installed && context
                        );
                        assert_eq!(
                            options.iter().any(|option| option.value == "git"),
                            installed && git
                        );
                    }
                    if !installed || !context {
                        assert!(matches!(
                            slot_page_catalog(1, &snapshot, "kubernetes"),
                            Err(SettingsError::Unavailable)
                        ));
                        let edit = Edit {
                            revision: 1,
                            id: settings::SettingId::new("tags.slot.kubernetes.enabled")
                                .unwrap(),
                            change: Change::Set(SettingValue::Boolean(true)),
                        };
                        assert!(matches!(
                            apply_edit(1, &base, &prefs, &market, &edit),
                            Err(SettingsError::Unavailable)
                        ));
                    }
                }
            }
        }
    }

    #[test]
    fn devops_preview_missing_or_invalid_inventory_fails_closed_without_losing_style() {
        let base = Config::default();
        let mut prefs = UserPreferences::default();
        let mut recipe = prefs.visual.information_bar.recipe();
        recipe.slots[0].color = Some([17, 43, 91]);
        prefs.visual.information_bar.use_custom = true;
        prefs.visual.information_bar.custom_recipe = Some(recipe.clone());
        let item = test_installed_extensions()[0].clone();
        for market in [
            vec![],
            vec![item.clone(), item.clone()],
            vec![item; settings_extensions::MAX_EXTENSION_SNAPSHOT_ITEMS + 1],
        ] {
            let snapshot = slot_page_snapshot_with_config(&prefs, &base, &base, &market);
            assert!(!snapshot.slot_available("kubernetes"));
            assert!(!snapshot.slot_available("git"));
            assert_eq!(snapshot.preview_recipe(), recipe);
        }
        let restored = slot_page_snapshot_with_config(
            &prefs,
            &base,
            &base,
            &test_installed_extensions(),
        );
        assert!(restored.slot_available("kubernetes"));
        assert_eq!(restored.preview_recipe(), recipe);
    }

    #[test]
    fn settings_inventory_prunes_only_after_ready_and_preserves_unrelated_overrides() {
        let mut prefs = UserPreferences {
            font_size: Some(21.0),
            ..UserPreferences::default()
        };
        prefs.presentation.inline_tables = Some(false);
        prefs
            .set_extension_feature_enabled(
                settings_extensions::DEVOPS_CONTEXT_STATUS_ID,
                false,
            )
            .unwrap();
        let original = prefs.clone();
        assert!(!prune_removed_extension_features(&mut prefs, &[], false));
        assert_eq!(
            prefs, original,
            "Loading/unavailable inventory cannot imply uninstall"
        );
        assert!(prune_removed_extension_features(&mut prefs, &[], true));
        assert_eq!(
            prefs
                .extension_feature_enabled(settings_extensions::DEVOPS_CONTEXT_STATUS_ID),
            None
        );
        assert_eq!(prefs.font_size, Some(21.0));
        assert_eq!(prefs.presentation.inline_tables, Some(false));
        assert!(
            !prune_removed_extension_features(&mut prefs, &[], true),
            "No duplicate save on repeated inventory event"
        );
    }
    #[test]
    fn settings_inventory_disabled_installed_feature_survives_but_uninstall_removes_it() {
        let mut prefs = UserPreferences::default();
        prefs
            .set_extension_feature_enabled(
                settings_extensions::DEVOPS_CONTEXT_STATUS_ID,
                false,
            )
            .unwrap();
        let mut market = vec![MarketItem {
            id: "automexia.devops".into(),
            name: "ignored".into(),
            description: "ignored".into(),
            installed: true,
        }];
        assert!(!prune_removed_extension_features(&mut prefs, &market, true));
        assert_eq!(
            prefs
                .extension_feature_enabled(settings_extensions::DEVOPS_CONTEXT_STATUS_ID),
            Some(false)
        );
        market[0].installed = false;
        assert!(prune_removed_extension_features(&mut prefs, &market, true));
        let snapshot = catalog(9, &Config::default(), &prefs, &market).unwrap();
        assert!(snapshot
            .get(
                &settings::SettingId::new(settings_extensions::DEVOPS_CONTEXT_STATUS_ID)
                    .unwrap()
            )
            .is_none());
    }
}

#[cfg(test)]
mod dependency_tests {
    use super::*;
    use settings::{Availability, SettingId};

    #[test]
    fn terminal_output_category_reports_commands_and_logs_independently() {
        for commands in [false, true] {
            for logs in [false, true] {
                let mut saved = UserPreferences::default();
                saved.presentation.command_output_highlighting = Some(commands);
                saved.presentation.output_highlighting = Some(logs);
                let snapshot = catalog(1, &Config::default(), &saved, &[]).unwrap();
                let groups = customization_groups(&snapshot);
                let terminal = groups
                    .iter()
                    .find(|group| group.label == "Terminal output colors")
                    .unwrap();
                let expected = format!(
                    "Commands {} · Highlights {}.",
                    if commands { "on" } else { "off" },
                    if logs { "on" } else { "off" },
                );
                assert!(terminal.root_action.description.starts_with(&expected));
                assert!(!terminal.root_action.description.contains("Currently"));
                assert!(terminal
                    .members
                    .iter()
                    .any(|id| id.as_str() == settings::COMMAND_OUTPUT_HIGHLIGHTING));
                assert!(terminal
                    .members
                    .iter()
                    .any(|id| id.as_str() == settings::OUTPUT_HIGHLIGHTING));
            }
        }
    }

    #[test]
    fn terminal_log_and_kubernetes_color_controls_have_independent_categories_and_edits()
    {
        let base = Config::default();
        let mut saved = UserPreferences::default();
        let ids = [
            "terminal.command_output_highlighting",
            settings::OUTPUT_HIGHLIGHTING,
            "terminal.kubernetes_highlighting",
        ];
        let initial = catalog(4, &base, &saved, &[]).unwrap();
        let groups = customization_groups(&initial);
        let terminal = groups
            .iter()
            .find(|group| group.label == "Terminal output colors")
            .unwrap();
        assert_eq!(terminal.key.as_str(), ids[0]);
        assert!(terminal.members.iter().any(|id| id.as_str() == ids[1]));
        let kubernetes = groups
            .iter()
            .find(|group| group.label == "Kubernetes status colors")
            .unwrap();
        assert_eq!(kubernetes.key.as_str(), ids[2]);
        for (index, id) in ids.into_iter().enumerate() {
            saved = apply_edit(
                4 + index as u64,
                &base,
                &saved,
                &[],
                &Edit {
                    revision: 4 + index as u64,
                    id: SettingId::new(id).unwrap(),
                    change: Change::Set(SettingValue::Boolean(false)),
                },
            )
            .unwrap();
            let updated = catalog(5 + index as u64, &base, &saved, &[]).unwrap();
            for (other_index, other) in ids.into_iter().enumerate() {
                assert_eq!(
                    updated.get(&SettingId::new(other).unwrap()).unwrap().value,
                    SettingValue::Boolean(other_index > index),
                );
            }
        }
    }

    #[test]
    fn command_log_and_kubernetes_palette_resets_stay_in_their_own_category() {
        let base = Config::default();
        let market = [];
        let mut saved = UserPreferences::default();
        for (revision, id, value) in [
            (1, "command_output.backgrounds.success", [11, 22, 33, 44]),
            (2, "output.backgrounds.error", [55, 66, 77, 88]),
            (3, "kubernetes.backgrounds.warning", [99, 111, 123, 135]),
        ] {
            saved = apply_edit(
                revision,
                &base,
                &saved,
                &market,
                &Edit {
                    revision,
                    id: SettingId::new(id).unwrap(),
                    change: Change::Set(SettingValue::Color(value)),
                },
            )
            .unwrap();
        }
        assert_eq!(
            saved.visual.command_output.success.unwrap().bytes(),
            [11, 22, 33, 44]
        );
        assert_eq!(
            saved.visual.highlight.error_background.unwrap().bytes(),
            [55, 66, 77, 88]
        );
        assert_eq!(
            saved.visual.kubernetes.warning_background.unwrap().bytes(),
            [99, 111, 123, 135]
        );

        let command_only = reset_customizations(
            &saved,
            &CustomizationResetScope::CommandOutputBand("success".into()),
            None,
        )
        .unwrap();
        assert!(command_only.visual.command_output.success.is_none());
        assert_eq!(command_only.visual.highlight, saved.visual.highlight);
        assert_eq!(command_only.visual.kubernetes, saved.visual.kubernetes);

        let kubernetes_group = reset_customizations(
            &saved,
            &CustomizationResetScope::Group(
                SettingId::new(settings::KUBERNETES_HIGHLIGHTING).unwrap(),
            ),
            None,
        )
        .unwrap();
        assert!(kubernetes_group
            .visual
            .kubernetes
            .warning_background
            .is_none());
        assert_eq!(
            kubernetes_group.visual.command_output,
            saved.visual.command_output
        );
        assert_eq!(kubernetes_group.visual.highlight, saved.visual.highlight);

        let terminal_group = reset_customizations(
            &saved,
            &CustomizationResetScope::Group(
                SettingId::new(settings::COMMAND_OUTPUT_HIGHLIGHTING).unwrap(),
            ),
            None,
        )
        .unwrap();
        assert!(terminal_group.visual.command_output.success.is_none());
        assert!(terminal_group.visual.highlight.error_background.is_none());
        assert_eq!(terminal_group.visual.kubernetes, saved.visual.kubernetes);
        assert!(matches!(
            reset_customizations(
                &saved,
                &CustomizationResetScope::KubernetesSeverity("invalid".into()),
                None
            ),
            Err(SettingsError::UnknownSetting)
        ));
    }

    #[test]
    fn settings_highlighting_is_available_without_optional_extension_membership() {
        let mut prefs = UserPreferences::default();
        prefs.presentation.output_highlighting = Some(false);
        let before = prefs.clone();
        let snapshot = catalog(4, &Config::default(), &prefs, &[]).unwrap();
        let id = SettingId::new(settings::OUTPUT_HIGHLIGHTING).unwrap();
        let entry = snapshot.get(&id).unwrap();
        assert_eq!(entry.availability, Availability::Available);
        assert_eq!(entry.value, SettingValue::Boolean(false));
        for control in [
            crate::automexia::presentation::OUTPUT_STYLE,
            crate::automexia::presentation::OUTPUT_COLOR_BINDINGS[0].id,
            crate::automexia::presentation::OUTPUT_BACKGROUND_BINDINGS[0].id,
        ] {
            assert_eq!(
                snapshot
                    .get(&SettingId::new(control).unwrap())
                    .unwrap()
                    .availability,
                Availability::Available,
                "core output control {control} must not depend on an optional extension"
            );
        }
        assert_eq!(prefs, before);
        let changed = apply_edit(
            4,
            &Config::default(),
            &prefs,
            &[],
            &Edit {
                revision: 4,
                id,
                change: Change::Set(SettingValue::Boolean(true)),
            },
        )
        .unwrap();
        assert_eq!(changed.presentation.output_highlighting, Some(true));
    }
    #[test]
    fn settings_highlighting_remains_available_when_installed_context_feature_is_off() {
        let mut prefs = UserPreferences::default();
        prefs
            .set_extension_feature_enabled(
                settings_extensions::DEVOPS_CONTEXT_STATUS_ID,
                false,
            )
            .unwrap();
        let market = [MarketItem {
            id: "automexia.devops".into(),
            name: "ignored".into(),
            description: "ignored".into(),
            installed: true,
        }];
        let id = SettingId::new(settings::OUTPUT_HIGHLIGHTING).unwrap();
        let snapshot = catalog(4, &Config::default(), &prefs, &market).unwrap();
        assert_eq!(
            snapshot.get(&id).unwrap().availability,
            Availability::Available
        );
        let changed = apply_edit(
            4,
            &Config::default(),
            &prefs,
            &market,
            &Edit {
                revision: 4,
                id,
                change: Change::Set(SettingValue::Boolean(false)),
            },
        )
        .unwrap();
        assert_eq!(changed.presentation.output_highlighting, Some(false));
        assert_eq!(
            changed
                .extension_feature_enabled(settings_extensions::DEVOPS_CONTEXT_STATUS_ID),
            Some(false)
        );
    }
}

#[cfg(test)]
mod appearance_tests {
    use super::*;
    use rio_backend::config::theme::AppearanceTheme;
    fn adaptive_base() -> Config {
        let mut base = Config::default();
        base.adaptive_colors = Some(rio_backend::config::theme::AdaptiveColors {
            light: Some(base.colors),
            dark: Some(base.colors),
        });
        base
    }
    const FONT: &str = "appearance.font_size";
    const THEME: &str = "appearance.theme";
    fn request(id: &str, change: Change) -> Edit {
        Edit {
            revision: 9,
            id: settings::SettingId::new(id).unwrap(),
            change,
        }
    }
    #[test]
    fn appearance_catalogue_exposes_fractional_font_and_configuration_inheritance() {
        let mut base = adaptive_base();
        base.fonts.size = 18.25;
        base.force_theme = Some(AppearanceTheme::Light);
        let prefs = UserPreferences {
            font_size: Some(21.5),
            ..UserPreferences::default()
        };
        let catalog = catalog(9, &base, &prefs, &[]).unwrap();
        let font = catalog
            .get(&settings::SettingId::new(FONT).unwrap())
            .unwrap();
        assert_eq!(font.value, SettingValue::Number(21.5));
        assert_eq!(font.default, SettingValue::Number(18.25));
        assert_eq!(font.origin, ValueOrigin::User);
        let theme = catalog
            .get(&settings::SettingId::new(THEME).unwrap())
            .unwrap();
        assert_eq!(theme.value, SettingValue::Choice("configuration".into()));
        assert!(theme.description.contains("Light"));
    }
    #[test]
    fn appearance_font_set_reset_preserves_other_preferences_and_rejects_invalid_values()
    {
        let mut base = adaptive_base();
        base.fonts.size = 18.25;
        let mut prefs = UserPreferences {
            appearance_theme: Some(AppearanceTheme::Dark),
            ..UserPreferences::default()
        };
        prefs.presentation.inline_tables = Some(false);
        let selected = apply_edit(
            9,
            &base,
            &prefs,
            &[],
            &request(FONT, Change::Set(SettingValue::Number(22.5))),
        )
        .unwrap();
        assert_eq!(selected.font_size, Some(22.5));
        let reset =
            apply_edit(9, &base, &selected, &[], &request(FONT, Change::Reset)).unwrap();
        assert_eq!(reset, prefs);
        assert_eq!(reset.apply_to(&base).fonts.size, 18.25);
        for value in [f64::NAN, f64::INFINITY, 5.5, 100.5] {
            assert!(apply_edit(
                9,
                &base,
                &prefs,
                &[],
                &request(FONT, Change::Set(SettingValue::Number(value)))
            )
            .is_err());
        }
        assert_eq!(
            apply_edit(10, &base, &prefs, &[], &request(FONT, Change::Reset)),
            Err(SettingsError::StaleRevision)
        );
    }
    #[test]
    fn appearance_choices_use_existing_overlay_and_reset_to_configured_force_theme() {
        let mut base = adaptive_base();
        base.force_theme = Some(AppearanceTheme::Light);
        let prefs = UserPreferences {
            font_size: Some(21.5),
            ..UserPreferences::default()
        };
        let dark = apply_edit(
            9,
            &base,
            &prefs,
            &[],
            &request(THEME, Change::Set(SettingValue::Choice("dark".into()))),
        )
        .unwrap();
        assert_eq!(dark.appearance_theme, Some(AppearanceTheme::Dark));
        for change in [
            Change::Reset,
            Change::Set(SettingValue::Choice("configuration".into())),
        ] {
            let reset =
                apply_edit(9, &base, &dark, &[], &request(THEME, change)).unwrap();
            assert_eq!(reset, prefs);
            assert_eq!(
                reset.apply_to(&base).force_theme,
                Some(AppearanceTheme::Light)
            );
        }
        assert!(apply_edit(
            9,
            &base,
            &prefs,
            &[],
            &request(THEME, Change::Set(SettingValue::Choice("system".into())))
        )
        .is_err());
    }
    #[test]
    fn appearance_unsupported_config_font_keeps_other_settings_available_without_rewriting_it(
    ) {
        for size in [0.0, 101.0, f32::INFINITY, f32::NAN] {
            let mut base = Config::default();
            base.fonts.size = size;
            let prefs = UserPreferences::default();
            let snapshot = catalog(9, &base, &prefs, &[]).unwrap();
            let font = snapshot
                .get(&settings::SettingId::new(FONT).unwrap())
                .unwrap();
            assert!(matches!(
                font.availability,
                settings::Availability::Unavailable { .. }
            ));
            assert!(snapshot
                .get(&settings::SettingId::new(settings::INLINE_TABLES).unwrap())
                .is_some());
            assert_eq!(base.fonts.size.to_bits(), size.to_bits());
            assert_eq!(prefs, UserPreferences::default());
        }
    }
    #[test]
    fn appearance_fixed_palette_explains_unavailability_and_retains_the_saved_choice() {
        let base = Config::default();
        let prefs = UserPreferences {
            appearance_theme: Some(AppearanceTheme::Dark),
            ..UserPreferences::default()
        };
        let snapshot = catalog(9, &base, &prefs, &[]).unwrap();
        let row = snapshot
            .get(&settings::SettingId::new(THEME).unwrap())
            .unwrap();
        assert!(
            matches!(&row.availability,settings::Availability::Unavailable{reason} if reason.contains("adaptive"))
        );
        assert_eq!(row.value, SettingValue::Choice("dark".into()));
        assert_eq!(
            apply_edit(
                9,
                &base,
                &prefs,
                &[],
                &request(THEME, Change::Set(SettingValue::Choice("light".into())))
            ),
            Err(SettingsError::Unavailable)
        );
        assert_eq!(prefs.appearance_theme, Some(AppearanceTheme::Dark));
    }
}

#[cfg(test)]
mod visual_catalog_tests {
    use super::*;
    use settings::{SettingId, SettingKind};
    fn installed() -> Vec<MarketItem> {
        vec![MarketItem {
            id: "automexia.devops".into(),
            name: "Example".into(),
            description: "Fixture".into(),
            installed: true,
        }]
    }
    fn request(id: &str, change: Change) -> Edit {
        Edit {
            revision: 41,
            id: SettingId::new(id).unwrap(),
            change,
        }
    }
    // Discover controls from their production catalogs so adding a customization
    // automatically adds its value boundaries, persistence and reset coverage.
    fn customization_inventory(
        base: &Config,
        prefs: &UserPreferences,
    ) -> Vec<settings::SettingDescriptor> {
        let current = prefs.apply_to(base);
        let market = installed();
        let full =
            catalog_with_palette(41, base, prefs, &market, &current.colors).unwrap();
        let mut rows: Vec<_> = full
            .entries()
            .iter()
            .filter(|row| {
                row.id.as_str().starts_with("tags.")
                    || row.id.as_str().starts_with("output.")
                    || row.id.as_str().starts_with("command_output.")
                    || row.id.as_str().starts_with("kubernetes.")
                    || matches!(
                        row.id.as_str(),
                        settings::FONT_SIZE
                            | settings::INLINE_TABLES
                            | settings::OUTPUT_HIGHLIGHTING
                            | settings::COMMAND_OUTPUT_HIGHLIGHTING
                            | settings::KUBERNETES_HIGHLIGHTING
                            | settings::COMMAND_TIMESTAMPS
                    )
            })
            .cloned()
            .collect();
        rows.extend(
            interface::descriptors(base, &current, prefs, &current.colors).unwrap(),
        );
        rows.extend(
            table_settings_catalog(41, base, prefs, &current.colors)
                .unwrap()
                .entries()
                .iter()
                .cloned(),
        );
        rows.extend(
            timestamp_settings_catalog(41, base, prefs, &current.colors)
                .unwrap()
                .entries()
                .iter()
                .cloned(),
        );
        rows.extend(
            font_settings_catalog(41, base, prefs, &current.colors)
                .unwrap()
                .entries()
                .iter()
                .cloned(),
        );
        for style in rio_backend::config::presentation::WindowControlStyle::ALL {
            rows.extend(
                window_controls::catalog(
                    41,
                    base,
                    prefs,
                    &current.colors,
                    &format!("window-controls.{}.background", style.id()),
                )
                .unwrap()
                .entries()
                .iter()
                .cloned(),
            );
        }
        rows.sort_by(|a, b| a.id.as_str().cmp(b.id.as_str()));
        rows.dedup_by(|a, b| a.id == b.id);
        rows
    }

    #[test]
    fn customization_catalog_boundaries_apply_persist_and_reset_each_control() {
        let base = Config::default();
        let original = UserPreferences::default();
        let root = tempfile::tempdir().unwrap();
        let rows = customization_inventory(&base, &original);
        let mut covered = 0;
        let mut cases = 0;
        for row in rows {
            if row.availability.reason().is_some() {
                continue;
            }
            let samples = match &row.kind {
                SettingKind::Boolean => {
                    vec![SettingValue::Boolean(false), SettingValue::Boolean(true)]
                }
                SettingKind::Choice { options } => options
                    .iter()
                    .map(|o| SettingValue::Choice(o.value.clone()))
                    .collect(),
                SettingKind::Number { min, max, step }
                | SettingKind::ContinuousNumber { min, max, step } => {
                    [*min, min + ((max - min) / step / 2.0).floor() * step, *max]
                        .map(SettingValue::Number)
                        .to_vec()
                }
                SettingKind::Color { alpha } => [0, 64, 128, 192, 255]
                    .into_iter()
                    .map(|a| {
                        SettingValue::Color([
                            a,
                            255 - a,
                            a / 2,
                            if *alpha { a } else { 255 },
                        ])
                    })
                    .collect(),
                SettingKind::Text { .. } if row.id.as_str() == "fonts.family" => {
                    vec![SettingValue::Text("Example Mono".into())]
                }
                SettingKind::Text { .. } if row.id.as_str() == "fonts.features" => {
                    vec![SettingValue::Text("ss01=1,zero=1".into())]
                }
                SettingKind::Text { .. } => vec![row.value.clone()],
                SettingKind::Action => continue,
            };
            covered += 1;
            for value in samples {
                cases += 1;
                let edit = request(row.id.as_str(), Change::Set(value.clone()));
                let changed = apply_edit(41, &base, &original, &installed(), &edit)
                    .unwrap_or_else(|e| panic!("{} {value:?}: {e:?}", row.id.as_str()));
                crate::automexia::preferences::write_to_root(root.path(), &changed)
                    .unwrap();
                let loaded = crate::automexia::preferences::load_from_root(root.path());
                assert!(loaded.warning.is_none(), "{} load warning", row.id.as_str());
                assert_eq!(
                    loaded.preferences,
                    changed,
                    "{} persistence",
                    row.id.as_str()
                );
                let refreshed = customization_inventory(&base, &loaded.preferences);
                let actual = &refreshed.iter().find(|r| r.id == row.id).unwrap().value;
                match (actual, &value) {
                    (SettingValue::Number(a), SettingValue::Number(b)) => assert!(
                        (a - b).abs() < 0.005,
                        "{} effect: {a} != {b}",
                        row.id.as_str()
                    ),
                    _ => {
                        assert_eq!(actual, &value, "{} effective value", row.id.as_str())
                    }
                }
                let reset = apply_edit(
                    41,
                    &base,
                    &changed,
                    &installed(),
                    &request(row.id.as_str(), Change::Reset),
                )
                .unwrap();
                let restored = customization_inventory(&base, &reset);
                let restored = &restored.iter().find(|r| r.id == row.id).unwrap().value;
                assert_eq!(restored, &row.default, "{} reset value", row.id.as_str());
                // Shape/arrangement edits intentionally retain the custom-layout
                // draft. Their per-field reset restores the preset field without
                // discarding other saved layout work (covered by the layout tests).
                let mut without_draft = reset;
                if matches!(
                    row.id.as_str(),
                    "tags.bar-style"
                        | "tags.bar-arrangement"
                        | "tags.spacing"
                        | crate::automexia::presentation::TAG_FORMAT
                ) {
                    assert_eq!(
                        without_draft.visual.information_bar.recipe(),
                        original.visual.information_bar.recipe()
                    );
                    without_draft.visual.information_bar =
                        original.visual.information_bar.clone();
                }
                assert_eq!(
                    without_draft,
                    original,
                    "{} reset isolation",
                    row.id.as_str()
                );
            }
        }
        assert!(covered >= 200, "only {covered} controls checked");
        assert!(cases >= 700, "only {cases} cases checked");
    }

    #[test]
    fn visual_catalog_exposes_every_context_role_color() {
        let rows = catalog(
            41,
            &Config::default(),
            &UserPreferences::default(),
            &installed(),
        )
        .unwrap();
        for role in [
            "production",
            "ubuntu_wsl",
            "windows",
            "git",
            "kubernetes",
            "docker",
            "azure",
            "aws",
            "gcp",
            "unknown_cloud",
            "terraform",
            "environment",
            "user",
        ] {
            let id = SettingId::new(format!("tags.colors.{role}")).unwrap();
            let row = rows
                .get(&id)
                .unwrap_or_else(|| panic!("missing context role {role}"));
            assert_eq!(row.kind, SettingKind::Color { alpha: false });
            assert!(matches!(row.value, SettingValue::Color([_, _, _, 255])));
            assert_eq!(row.owner, settings::SettingOwner::Core);
        }
    }
    #[test]
    fn visual_catalog_exposes_consumed_highlight_colors_and_appearance_choices() {
        let rows = catalog(
            41,
            &Config::default(),
            &UserPreferences::default(),
            &installed(),
        )
        .unwrap();
        for severity in ["error", "warning", "success", "info", "debug"] {
            let row = rows
                .get(&SettingId::new(format!("output.colors.{severity}")).unwrap())
                .expect("missing highlight foreground");
            assert_eq!(row.kind, SettingKind::Color { alpha: false });
        }
        for severity in ["error", "warning"] {
            let row = rows
                .get(&SettingId::new(format!("output.backgrounds.{severity}")).unwrap())
                .expect("missing consumed background");
            assert_eq!(row.kind, SettingKind::Color { alpha: true });
        }
        for id in ["tags.style", "tags.opacity", "output.style"] {
            assert!(
                rows.get(&SettingId::new(id).unwrap()).is_some(),
                "missing appearance control {id}"
            );
        }
    }
    #[test]
    fn visual_catalog_color_edit_survives_restart_and_reset_preserves_other_values() {
        let base = Config::default();
        let original = UserPreferences {
            font_size: Some(23.5),
            ..UserPreferences::default()
        };
        let selected = apply_edit(
            41,
            &base,
            &original,
            &installed(),
            &request(
                "tags.colors.kubernetes",
                Change::Set(SettingValue::Color([12, 34, 56, 255])),
            ),
        )
        .unwrap();
        let root = tempfile::tempdir().unwrap();
        crate::automexia::preferences::write_to_root(root.path(), &selected).unwrap();
        let restored = crate::automexia::preferences::load_from_root(root.path());
        assert_eq!(restored.preferences, selected);
        assert!(restored.warning.is_none());
        let rows = catalog(41, &base, &restored.preferences, &installed()).unwrap();
        let row = rows
            .get(&SettingId::new("tags.colors.kubernetes").unwrap())
            .unwrap();
        assert_eq!(row.value, SettingValue::Color([12, 34, 56, 255]));
        assert_eq!(row.origin, ValueOrigin::User);
        let reset = apply_edit(
            41,
            &base,
            &restored.preferences,
            &installed(),
            &request("tags.colors.kubernetes", Change::Reset),
        )
        .unwrap();
        assert_eq!(reset, original);
        assert_eq!(
            apply_edit(
                42,
                &base,
                &selected,
                &installed(),
                &request("tags.colors.kubernetes", Change::Reset)
            ),
            Err(SettingsError::StaleRevision)
        );
    }
    #[test]
    fn visual_choices_and_percent_validate_real_edit_boundaries() {
        let base = Config::default();
        let original = UserPreferences::default();
        let market = installed();
        let selected = apply_edit(
            41,
            &base,
            &original,
            &market,
            &request(
                "tags.style",
                Change::Set(SettingValue::Choice("plain".into())),
            ),
        )
        .unwrap();
        assert_eq!(selected.visual.tags.style, Some(TagStyle::Plain));
        let selected = apply_edit(
            41,
            &base,
            &selected,
            &market,
            &request(
                "tags.opacity",
                Change::Set(SettingValue::Number(12.9999999999)),
            ),
        )
        .unwrap();
        assert_eq!(selected.visual.tags.opacity.unwrap().get(), 13);
        let selected = apply_edit(
            41,
            &base,
            &selected,
            &market,
            &request(
                "output.style",
                Change::Set(SettingValue::Choice("background".into())),
            ),
        )
        .unwrap();
        assert_eq!(
            selected.visual.highlight.style,
            Some(HighlightStyle::Background)
        );
        for value in [f64::NAN, f64::INFINITY, -1.0, 12.25, 101.0] {
            assert_eq!(
                apply_edit(
                    41,
                    &base,
                    &original,
                    &market,
                    &request("tags.opacity", Change::Set(SettingValue::Number(value)))
                ),
                Err(SettingsError::InvalidValue)
            );
        }
        assert_eq!(
            apply_edit(
                41,
                &base,
                &original,
                &market,
                &request(
                    "tags.colors.git",
                    Change::Set(SettingValue::Color([1, 2, 3, 0]))
                )
            ),
            Err(SettingsError::InvalidValue)
        );
        assert_eq!(
            apply_edit(
                41,
                &base,
                &original,
                &market,
                &request(
                    "output.colors.error",
                    Change::Set(SettingValue::Color([1, 2, 3, 128]))
                )
            ),
            Err(SettingsError::InvalidValue)
        );
        let reset = apply_edit(
            41,
            &base,
            &selected,
            &market,
            &request("tags.opacity", Change::Reset),
        )
        .unwrap();
        assert_eq!(reset.visual.tags.opacity, None);
        assert_eq!(reset.visual.tags.style, Some(TagStyle::Plain));
        assert_eq!(
            reset.visual.highlight.style,
            Some(HighlightStyle::Background)
        );
    }

    #[test]
    fn visual_all_fixed_color_edits_are_independent_and_resettable() {
        let base = Config::default();
        let original = UserPreferences::default();
        let market = installed();
        for binding in &crate::automexia::presentation::TAG_COLOR_BINDINGS {
            let selected = apply_edit(
                41,
                &base,
                &original,
                &market,
                &request(
                    binding.id,
                    Change::Set(SettingValue::Color([12, 34, 56, 255])),
                ),
            )
            .unwrap();
            assert_eq!(
                (binding.read)(&selected.visual.tags.colors),
                Some(Rgb::from_bytes([12, 34, 56]))
            );
            let restored = apply_edit(
                41,
                &base,
                &selected,
                &market,
                &request(binding.id, Change::Reset),
            )
            .unwrap();
            assert_eq!(restored, original);
        }
        for binding in &crate::automexia::presentation::OUTPUT_COLOR_BINDINGS {
            let selected = apply_edit(
                41,
                &base,
                &original,
                &market,
                &request(
                    binding.id,
                    Change::Set(SettingValue::Color([12, 34, 56, 255])),
                ),
            )
            .unwrap();
            assert_eq!(
                (binding.read)(&selected.visual.highlight.colors),
                Some(Rgb::from_bytes([12, 34, 56]))
            );
            let restored = apply_edit(
                41,
                &base,
                &selected,
                &market,
                &request(binding.id, Change::Reset),
            )
            .unwrap();
            assert_eq!(restored, original);
        }
        for binding in &crate::automexia::presentation::OUTPUT_BACKGROUND_BINDINGS {
            let selected = apply_edit(
                41,
                &base,
                &original,
                &market,
                &request(
                    binding.id,
                    Change::Set(SettingValue::Color([12, 34, 56, 78])),
                ),
            )
            .unwrap();
            let effective = selected.apply_to(&base);
            assert_eq!(
                (binding.read)(&effective.presentation.highlight),
                Some(Rgba::from_bytes([12, 34, 56, 78]))
            );
            let rows = catalog(41, &base, &selected, &market).unwrap();
            assert_eq!(
                rows.get(&SettingId::new(binding.id).unwrap())
                    .unwrap()
                    .value,
                SettingValue::Color([12, 34, 56, 78])
            );
            let restored = apply_edit(
                41,
                &base,
                &selected,
                &market,
                &request(binding.id, Change::Reset),
            )
            .unwrap();
            assert_eq!(restored, original);
        }
    }

    #[test]
    fn visual_catalog_uses_explicit_resolved_palette_and_configured_override() {
        let mut base = Config::default();
        let preferences = UserPreferences::default();
        let mut palette = base.colors;
        palette.red = [1.0, 0.0, 0.0, 1.0];
        let id = SettingId::new("output.colors.error").unwrap();
        let rows = catalog_with_palette(41, &base, &preferences, &installed(), &palette)
            .unwrap();
        let row = rows.get(&id).unwrap();
        assert_eq!(row.value, SettingValue::Color([255, 0, 0, 255]));
        assert_eq!(row.default, row.value);
        base.presentation.highlight.colors.error = Some(Rgb::from_bytes([9, 8, 7]));
        let rows = catalog_with_palette(41, &base, &preferences, &installed(), &palette)
            .unwrap();
        let row = rows.get(&id).unwrap();
        assert_eq!(row.value, SettingValue::Color([9, 8, 7, 255]));
        assert_eq!(row.default, row.value);
        assert_eq!(row.origin, ValueOrigin::Configuration);
    }

    #[test]
    fn translucent_palette_color_does_not_close_settings_or_change_renderer_alpha() {
        let mut base = Config::default();
        base.colors.red = [1.0, 0.0, 0.0, 0.5];
        base.colors.cyan = [0.0, 1.0, 1.0, 0.25];
        let original = UserPreferences::default();
        for market in [&installed()[..], &[][..]] {
            let rows = catalog(41, &base, &original, market).unwrap();
            for (id, expected) in [
                ("output.colors.error", [255, 0, 0, 255]),
                ("output.colors.info", [0, 255, 255, 255]),
            ] {
                let row = rows.get(&SettingId::new(id).unwrap()).unwrap();
                assert_eq!(row.value, SettingValue::Color(expected));
                assert_eq!(row.default, SettingValue::Color(expected));
            }
        }
        assert_eq!(
            crate::automexia::presentation::output_foreground(
                &base.presentation.highlight,
                &base.colors,
                automexia_extension_api::SemanticSeverity::Error,
            )[3],
            127
        );
        let selected = apply_edit(
            41,
            &base,
            &original,
            &installed(),
            &request(
                "output.colors.error",
                Change::Set(SettingValue::Color([9, 8, 7, 255])),
            ),
        )
        .unwrap();
        let reset = apply_edit(
            41,
            &base,
            &selected,
            &installed(),
            &request("output.colors.error", Change::Reset),
        )
        .unwrap();
        assert_eq!(reset, original);
        assert_eq!(
            crate::automexia::presentation::output_foreground(
                &reset.apply_to(&base).presentation.highlight,
                &base.colors,
                automexia_extension_api::SemanticSeverity::Error,
            )[3],
            127
        );
    }

    #[test]
    fn visual_output_settings_remain_available_without_extensions_and_retain_preferences()
    {
        let base = Config::default();
        let chosen = UserPreferences {
            visual: crate::automexia::preferences::VisualPreferences {
                highlight:
                    crate::automexia::preferences::HighlightAppearancePreferences {
                        colors: rio_backend::config::presentation::HighlightColors {
                            error: Some(Rgb::from_bytes([1, 2, 3])),
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                ..Default::default()
            },
            ..Default::default()
        };
        let rows = catalog(41, &base, &chosen, &[]).unwrap();
        for id in [
            "output.style",
            "output.colors.error",
            "output.backgrounds.error",
        ] {
            let row = rows.get(&SettingId::new(id).unwrap()).unwrap();
            assert!(row.availability.reason().is_none());
            assert!(
                apply_edit(41, &base, &chosen, &[], &request(id, Change::Reset)).is_ok()
            );
        }
        assert_eq!(
            rows.get(&SettingId::new("output.colors.error").unwrap())
                .unwrap()
                .value,
            SettingValue::Color([1, 2, 3, 255])
        );
        assert!(rows
            .get(&SettingId::new("tags.colors.git").unwrap())
            .unwrap()
            .availability
            .reason()
            .is_none());
        let installed_rows = catalog(41, &base, &chosen, &installed()).unwrap();
        assert!(installed_rows
            .get(&SettingId::new("output.colors.error").unwrap())
            .unwrap()
            .availability
            .reason()
            .is_none());
    }
}

#[cfg(test)]
mod fonts_entry_regression {
    use super::*;

    #[test]
    fn fonts_customization_replaces_the_size_only_group() {
        let catalog =
            catalog(1, &Config::default(), &UserPreferences::default(), &[]).unwrap();
        let groups = customization_groups(&catalog);
        let group = groups
            .iter()
            .find(|g| g.key.as_str() == settings::FONT_SIZE)
            .unwrap();
        assert_eq!(group.label, "Fonts");
    }
}

#[cfg(test)]
mod window_controls_entry_regression {
    use super::*;
    #[test]
    fn window_controls_have_a_discoverable_customization_page() {
        let catalog =
            catalog(1, &Config::default(), &UserPreferences::default(), &[]).unwrap();
        let groups = customization_groups(&catalog);
        assert!(groups.iter().any(|g| g.label == "Window controls"));
    }
}
