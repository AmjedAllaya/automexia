//! Application-owned settings sheet. Effects remain typed application intents.
use crate::automexia::package_customizations::{
    PackageCustomizationPages, PackageInventoryStatus,
};
use crate::automexia::ui::command_info::{pack_with_tag_joins, Label};
use crate::renderer::ui_theme::{color_u8, UiTheme};
use crate::settings_catalog::{
    command_output_band_catalog, customization_groups, customization_root_catalog,
    kubernetes_severity_catalog, output_severity_catalog, selected_tag_catalog,
    slot_page_actions, tag_role_color_catalog, CustomizationGroup,
    CustomizationResetScope, SlotPageSnapshot,
};
#[cfg(test)]
use automexia_ui_model::information_bar::tag_surface_geometry;
use automexia_ui_model::information_bar::{
    bar_layout_hints, prompt_tag_metrics, resolve_recipe, tag_surface_paint_layers,
    BarArrangement, BarVisualStyle, TagSurfaceGeometry,
};
use automexia_ui_model::settings::{
    Catalog, Change, Edit, FocusMove, Section, SettingDescriptor, SettingId, SettingKind,
    SettingValue, ValueOrigin, ViewState, MAX_QUERY_BYTES,
};
use rio_backend::config::colors::{ColorBuilder, Format};
use rio_backend::sugarloaf::{
    text::{DrawOpts, Text},
    Sugarloaf,
};
use rio_window::{
    event::{ElementState, Ime, MouseButton, MouseScrollDelta, TouchPhase, WindowEvent},
    keyboard::{Key, ModifiersState, NamedKey},
};
use unicode_segmentation::UnicodeSegmentation;

#[path = "settings_table_preview.rs"]
mod table_preview;
#[path = "settings_timestamp_preview.rs"]
mod timestamp_preview;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Rect {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}
impl Rect {
    fn array(self) -> [f32; 4] {
        [self.x, self.y, self.width, self.height]
    }
    fn contains(self, x: f32, y: f32) -> bool {
        x >= self.x && x < self.x + self.width && y >= self.y && y < self.y + self.height
    }
    fn intersect(self, other: Self) -> Option<Self> {
        let x = self.x.max(other.x);
        let y = self.y.max(other.y);
        let width = (self.x + self.width).min(other.x + other.width) - x;
        let height = (self.y + self.height).min(other.y + other.height) - y;
        (width > 0.0 && height > 0.0).then_some(Self {
            x,
            y,
            width,
            height,
        })
    }
}
#[derive(Clone, Copy, Debug, Default)]
struct Geometry {
    viewport: Rect,
    card: Rect,
    search: Rect,
    back: Rect,
    body: Rect,
    preview: Rect,
    reset: Rect,
    restore: Rect,
    close: Rect,
    status: Rect,
}
#[derive(Clone, Debug)]
struct Row {
    id: SettingId,
    bounds: Rect,
    control: Rect,
    label: Rect,
    help: Rect,
    navigation: bool,
    lines: Vec<String>,
    label_lines: usize,
    help_line: f32,
    value_lines: Vec<String>,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Focus {
    #[default]
    Search,
    List,
    PreviewButton,
    Preview,
    Reset,
    Restore,
    Close,
}
const MAX_COLOR_BYTES: usize = 9;
const SELECTED_CONTROLS_UNAVAILABLE: &str =
    "Selected controls unavailable. Retry or go back.";
const MAX_NUMERIC_BYTES: usize = 24;
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum ColorFocus {
    #[default]
    Hex,
    Apply,
    Cancel,
    Reset,
}
#[derive(Clone, Debug)]
struct ColorEditor {
    id: SettingId,
    revision: u64,
    alpha: bool,
    text_limits: Option<(usize, bool)>,
    draft: String,
    caret: usize,
    anchor: Option<usize>,
    focus: ColorFocus,
    composing: bool,
    last_valid_color: Option<[u8; 4]>,
    feedback: Option<String>,
}

#[derive(Clone, Debug)]
struct NumericEditor {
    id: SettingId,
    revision: u64,
    draft: String,
    caret: usize,
    anchor: Option<usize>,
    composing: bool,
}

fn numeric_fragment(value: &str) -> bool {
    value.len() <= MAX_NUMERIC_BYTES
        && value.bytes().all(|byte| {
            byte.is_ascii_digit() || matches!(byte, b'.' | b'-' | b'+' | b'e' | b'E')
        })
}

fn numeric_zones(bounds: Rect, _font: f32) -> (Rect, Rect, Rect) {
    let button = bounds.width * 0.25;
    (
        Rect {
            width: button,
            ..bounds
        },
        Rect {
            x: bounds.x + button,
            width: (bounds.width - button * 2.0).max(0.0),
            ..bounds
        },
        Rect {
            x: bounds.x + bounds.width - button,
            width: button,
            ..bounds
        },
    )
}

fn safe_editor_text(value: &str, max_bytes: usize, allow_empty: bool) -> bool {
    value.len() <= max_bytes
        && (allow_empty || !value.trim().is_empty())
        && !value.chars().any(|ch| {
            ch.is_control()
                || matches!(ch, '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
        })
}

fn editor_value(editor: &ColorEditor) -> Option<SettingValue> {
    if let Some((max_bytes, allow_empty)) = editor.text_limits {
        return safe_editor_text(&editor.draft, max_bytes, allow_empty)
            .then(|| SettingValue::Text(editor.draft.clone()));
    }
    parse_color(&editor.draft, editor.alpha).map(SettingValue::Color)
}

fn editor_matches(entry: &SettingDescriptor, editor: &ColorEditor) -> bool {
    match (&entry.kind, editor.text_limits) {
        (SettingKind::Color { alpha }, None) => *alpha == editor.alpha,
        (
            SettingKind::Text {
                max_bytes,
                allow_empty,
            },
            Some(limits),
        ) => (*max_bytes, *allow_empty) == limits,
        _ => false,
    }
}
#[derive(Clone, Copy, Debug, Default)]
struct ColorGeometry {
    card: Rect,
    title: Rect,
    name: Rect,
    input: Rect,
    preview: Rect,
    help: Rect,
    apply: Rect,
    cancel: Rect,
    reset: Rect,
}
#[derive(Clone, Debug, PartialEq, Eq)]
enum Target {
    Confirmation(bool),
    Search,
    Back,
    Control(SettingId, i32),
    NumericInput(SettingId),
    PreviewButton,
    PreviewItem(SettingId),
    Color(ColorFocus),
    Reset,
    Restore,
    Close,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum CustomizationIntent {
    Reset {
        revision: u64,
        scope: CustomizationResetScope,
    },
    RestoreSaved,
}

enum ConfirmedSettingsAction {
    Customization(CustomizationIntent),
    Setting(Edit),
}

struct SettingsConfirmation {
    action: ConfirmedSettingsAction,
    revision: u64,
    title: String,
    description: &'static str,
    accept_label: &'static str,
    accept_selected: bool,
}

#[derive(Clone, Copy, Default)]
struct ConfirmationGeometry {
    card: Rect,
    cancel: Rect,
    accept: Rect,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct EventResult {
    pub(crate) consumed: bool,
    pub(crate) redraw: bool,
    pub(crate) closed: bool,
}
trait Canvas {
    fn text(&mut self) -> &mut Text;
    fn rect(&mut self, bounds: [f32; 4], color: [f32; 4]);
    fn rounded_rect(&mut self, bounds: [f32; 4], radius: f32, color: [f32; 4]);
    fn polygon(&mut self, points: &[(f32, f32)], color: [f32; 4]);
    fn line(&mut self, from: (f32, f32), to: (f32, f32), width: f32, color: [f32; 4]);
    fn arc(
        &mut self,
        center: (f32, f32),
        radius: f32,
        angles: (f32, f32),
        width: f32,
        color: [f32; 4],
    );
}
impl Canvas for Sugarloaf<'_> {
    fn text(&mut self) -> &mut Text {
        self.text_mut()
    }
    fn rect(&mut self, [x, y, width, height]: [f32; 4], color: [f32; 4]) {
        Sugarloaf::rect(self, None, x, y, width, height, color, 0.0, 30);
    }
    fn rounded_rect(
        &mut self,
        [x, y, width, height]: [f32; 4],
        radius: f32,
        color: [f32; 4],
    ) {
        Sugarloaf::rounded_rect(self, None, x, y, width, height, color, 0.0, radius, 30);
    }
    fn polygon(&mut self, points: &[(f32, f32)], color: [f32; 4]) {
        Sugarloaf::polygon_with_order(self, points, 0.0, color, 30);
    }
    fn line(&mut self, from: (f32, f32), to: (f32, f32), width: f32, color: [f32; 4]) {
        Sugarloaf::line(self, from.0, from.1, to.0, to.1, width, 0.0, color, 30);
    }
    fn arc(
        &mut self,
        center: (f32, f32),
        radius: f32,
        angles: (f32, f32),
        width: f32,
        color: [f32; 4],
    ) {
        Sugarloaf::arc(
            self, center.0, center.1, radius, angles.0, angles.1, width, 0.0, color,
        );
    }
}

#[derive(Default)]
pub(crate) struct SettingsView {
    catalog: Option<Catalog>,
    view: Option<ViewState>,
    customizations: Option<CustomizationNavigation>,
    focus: Focus,
    pending: Option<Edit>,
    pending_customization: Option<CustomizationIntent>,
    confirmation: Option<SettingsConfirmation>,
    confirmation_geometry: ConfirmationGeometry,
    temporary_customizations: bool,
    color_editor: Option<ColorEditor>,
    numeric_editor: Option<NumericEditor>,
    color_geometry: ColorGeometry,
    preedit: String,
    caret: usize,
    anchor: Option<usize>,
    status: String,
    package_notice: PackageSettingsNotice,
    #[cfg(feature = "native-gui-test-hooks")]
    package_inventory_ready: bool,
    saving: Option<u64>,
    width: f32,
    height: f32,
    font: f32,
    geometry: Geometry,
    caret_rect: Rect,
    rows: Vec<Row>,
    compact_lines: Vec<String>,
    layout_dirty: bool,
    reveal_focus: bool,
    scroll: f32,
    content_height: f32,
    pointer: Option<(f32, f32)>,
    pressed: Option<Target>,
    touch: Option<(u64, (f32, f32), f32)>,
    preview_button: Rect,
    preview_tag_list_area: Rect,
    preview_targets: Vec<(SettingId, Rect)>,
    preview_tag_shapes: Vec<(
        Rect,
        automexia_ui_model::information_bar::ConnectedTagGeometry,
    )>,
    preview_order: Vec<SettingId>,
    preview_item_rows: Vec<(SettingId, usize)>,
    preview_edit_mode: bool,
    preview_selected: Option<SettingId>,
    preview_reveal_selection: bool,
    preview_scroll: usize,
    preview_visible_rows: usize,
    preview_total_rows: usize,
    preview_tag_list_scroll: usize,
    preview_tag_list_visible_rows: usize,
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum PackageSettingsNotice {
    #[default]
    None,
    Loading,
    Unavailable,
    ProjectionFailed,
}

impl PackageSettingsNotice {
    fn message(self) -> Option<&'static str> {
        match self {
            Self::None => None,
            Self::Loading => Some("Loading package settings..."),
            Self::Unavailable => Some("Package settings unavailable. Reopen to retry."),
            Self::ProjectionFailed => {
                Some("Package settings could not be displayed. Reopen to retry.")
            }
        }
    }

    #[cfg(feature = "native-gui-test-hooks")]
    fn id(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Loading => "loading",
            Self::Unavailable => "unavailable",
            Self::ProjectionFailed => "projection-failed",
        }
    }
}
struct CustomizationNavigation {
    full_catalog: Catalog,
    groups: Vec<CustomizationGroup>,
    root_catalog: Catalog,
    root_view: Option<ViewState>,
    active_key: Option<SettingId>,
    package_pages: Option<PackageCustomizationPages>,
    active_package: Option<SettingId>,
    package_view: Option<ViewState>,
    slot_pages: Option<SlotPageSnapshot>,
    active_slot: Option<SettingId>,
    active_slot_label: Option<String>,
    slot_parent_view: Option<ViewState>,
}

fn slot_id_from_page(key: &SettingId) -> Option<&str> {
    key.as_str()
        .strip_prefix("tags.slot.")?
        .strip_suffix(".page")
}
impl SettingsView {
    fn title(&self) -> &str {
        let Some(navigation) = &self.customizations else {
            return "Settings";
        };
        if let Some(label) = &navigation.active_slot_label {
            return label;
        }
        if let Some(package_key) = &navigation.active_package {
            let package = navigation.package_pages.as_ref().and_then(|pages| {
                pages.packages.iter().find(|page| &page.key == package_key)
            });
            if let Some(package) = package {
                if let Some(feature_key) = &navigation.active_key {
                    return package
                        .features
                        .iter()
                        .find(|feature| &feature.key == feature_key)
                        .map_or(package.action.label.as_str(), |feature| {
                            feature.action.label.as_str()
                        });
                }
                return package.action.label.as_str();
            }
        }
        navigation
            .active_key
            .as_ref()
            .and_then(|key| navigation.groups.iter().find(|group| &group.key == key))
            .map_or("Customizations", |group| group.label.as_str())
    }
    fn is_category_root(&self) -> bool {
        self.customizations.as_ref().is_some_and(|navigation| {
            navigation.active_key.is_none() && navigation.active_package.is_none()
        })
    }
    fn is_category_detail(&self) -> bool {
        self.customizations.as_ref().is_some_and(|navigation| {
            navigation.active_key.is_some() || navigation.active_package.is_some()
        })
    }

    #[cfg(test)]
    fn back_destination(&self) -> &'static str {
        if self
            .customizations
            .as_ref()
            .is_some_and(|navigation| navigation.active_slot.is_some())
        {
            if self
                .customizations
                .as_ref()
                .and_then(|navigation| navigation.active_slot.as_ref())
                .is_some_and(|slot| {
                    slot.as_str().starts_with("output.severity.")
                        || slot.as_str().starts_with("command_output.band.")
                })
            {
                "Terminal output colors"
            } else if self
                .customizations
                .as_ref()
                .and_then(|navigation| navigation.active_slot.as_ref())
                .is_some_and(|slot| slot.as_str().starts_with("kubernetes.severity."))
            {
                "Kubernetes status colors"
            } else {
                "Information tags"
            }
        } else {
            "categories"
        }
    }

    fn is_package_list(&self) -> bool {
        self.customizations.as_ref().is_some_and(|navigation| {
            navigation.active_package.is_some() && navigation.active_key.is_none()
        })
    }

    #[cfg(test)]
    pub(crate) fn open_customizations(
        &mut self,
        catalog: Catalog,
        packages: Option<PackageCustomizationPages>,
    ) {
        self.open_customizations_with_slots(catalog, packages, None);
    }

    pub(crate) fn open_customizations_with_slots(
        &mut self,
        catalog: Catalog,
        packages: Option<PackageCustomizationPages>,
        slot_pages: Option<SlotPageSnapshot>,
    ) {
        self.open_with_section(catalog, Some(Section::Customizations));
        if let Some(navigation) = self.customizations.as_mut() {
            navigation.package_pages = packages;
            navigation.slot_pages = slot_pages;
            self.rebuild_customization_root();
        }
    }

    pub(crate) fn open(&mut self, catalog: Catalog) {
        self.open_with_section(catalog, None);
    }

    pub(crate) fn open_with_section(
        &mut self,
        catalog: Catalog,
        section: Option<automexia_ui_model::settings::Section>,
    ) {
        self.close();
        if section == Some(Section::Customizations) {
            let mut groups = customization_groups(&catalog);
            let mut unavailable = false;
            let root_catalog =
                customization_root_catalog(&catalog, &groups).or_else(|_| {
                    unavailable = true;
                    groups.clear();
                    Catalog::new(catalog.revision(), Vec::new())
                });
            if let Ok(root_catalog) = root_catalog {
                let root_view = ViewState::new(&root_catalog, 6);
                self.view = Some(root_view);
                self.catalog = Some(root_catalog.clone());
                self.customizations = Some(CustomizationNavigation {
                    full_catalog: catalog,
                    groups,
                    root_catalog,
                    root_view: None,
                    active_key: None,
                    package_pages: None,
                    active_package: None,
                    package_view: None,
                    slot_pages: None,
                    active_slot: None,
                    active_slot_label: None,
                    slot_parent_view: None,
                });
                self.focus = Focus::Search;
                self.layout_dirty = true;
                self.reveal_focus = true;
                if unavailable {
                    self.status = "Customizations are temporarily unavailable.".into();
                }
                return;
            }
            self.status = "Customizations are temporarily unavailable.".into();
            return;
        }
        let mut view = ViewState::new(&catalog, 6);
        view.set_section(section, &catalog);
        self.view = Some(view);
        self.catalog = Some(catalog);
        self.focus = Focus::Search;
        self.layout_dirty = true;
        self.reveal_focus = true;
    }
    pub(crate) fn close(&mut self) {
        self.catalog = None;
        self.view = None;
        self.customizations = None;
        self.pending = None;
        self.pending_customization = None;
        self.confirmation = None;
        self.confirmation_geometry = ConfirmationGeometry::default();
        self.temporary_customizations = false;
        self.color_editor = None;
        self.numeric_editor = None;
        self.color_geometry = ColorGeometry::default();
        self.preedit.clear();
        self.status.clear();
        self.package_notice = PackageSettingsNotice::None;
        #[cfg(feature = "native-gui-test-hooks")]
        {
            self.package_inventory_ready = false;
        }
        self.saving = None;
        self.rows.clear();
        self.compact_lines.clear();
        self.pressed = None;
        self.touch = None;
        self.preview_targets.clear();
        self.preview_tag_shapes.clear();
        self.preview_order.clear();
        self.preview_item_rows.clear();
        self.preview_button = Rect::default();
        self.preview_tag_list_area = Rect::default();
        self.preview_edit_mode = false;
        self.preview_selected = None;
        self.preview_reveal_selection = false;
        self.preview_scroll = 0;
        self.preview_visible_rows = 0;
        self.preview_total_rows = 0;
        self.preview_tag_list_scroll = 0;
        self.preview_tag_list_visible_rows = 0;
        self.pointer = None;
        self.scroll = 0.0;
        self.caret = 0;
        self.anchor = None;
    }
    pub(crate) fn is_open(&self) -> bool {
        self.catalog.is_some()
    }
    pub(crate) fn suspend_input(&mut self) {
        self.pressed = None;
        self.touch = None;
        self.preedit.clear();
        if let Some(editor) = &mut self.color_editor {
            editor.composing = false;
        }
        if let Some(editor) = &mut self.numeric_editor {
            editor.composing = false;
        }
    }
    #[cfg(feature = "native-gui-test-hooks")]
    pub(crate) fn native_test_snapshot(&self) -> serde_json::Value {
        // Geometry and validated setting identities only; never saved values,
        // provider context, search queries or user-entered labels.
        let ready = self.is_open() && !self.layout_dirty;
        let recipe = self
            .customizations
            .as_ref()
            .and_then(|navigation| navigation.slot_pages.as_ref())
            .map(SlotPageSnapshot::preview_recipe);
        let targets: Vec<_> = self
            .preview_targets
            .iter()
            .filter(|_| ready)
            .map(|(id, bounds)| {
                serde_json::json!({
                    "id": id.as_str(), "bounds": bounds.array(),
                    "roster": bounds.y >= self.preview_tag_list_area.y,
                })
            })
            .collect();
        let controls: Vec<_> = self
            .rows
            .iter()
            .filter(|_| ready)
            .filter_map(|row| {
                Some(serde_json::json!({
                    "id": row.id.as_str(),
                    "bounds": row.control.intersect(self.geometry.body)?.array(),
                }))
            })
            .collect();
        serde_json::json!({
            "open": self.is_open(), "ready": ready,
            "active_slot": self.customizations.as_ref()
                .and_then(|navigation| navigation.active_slot.as_ref()).map(SettingId::as_str),
            "active_category": self.customizations.as_ref()
                .and_then(|navigation| navigation.active_key.as_ref()).map(SettingId::as_str),
            "selected": self.preview_selected.as_ref().map(SettingId::as_str),
            "preview_edit_mode": self.preview_edit_mode,
            "confirmation": self.confirmation.as_ref().map(|confirmation| serde_json::json!({
                "title": confirmation.title,
                "cancel": self.confirmation_geometry.cancel.array(),
                "accept": self.confirmation_geometry.accept.array(),
                "accept_selected": confirmation.accept_selected,
            })),
            "color_editor": self.color_editor.as_ref().map(|editor| serde_json::json!({
                "id": editor.id.as_str(),
                "input": self.color_geometry.input.array(),
                "apply": self.color_geometry.apply.array(),
                "cancel": self.color_geometry.cancel.array(),
            })),
            "numeric_editor": self.numeric_editor.as_ref().map(|editor| serde_json::json!({
                "id": editor.id.as_str(),
            })),
            "pointer": self.pointer, "pressed": self.pressed.is_some(),
            "devops_detection": self.preview_bool(crate::automexia::settings_extensions::DEVOPS_CONTEXT_STATUS_ID),
            "targets": targets, "controls": controls,
            "search_button": self.geometry.search.array(),
            "search_bytes": self.query().len(),
            "close_button": self.geometry.close.array(),
            "tag_shape": recipe.as_ref().map(|recipe| recipe.visual.id()),
            "tag_spacing": recipe.as_ref().map(|recipe| recipe.spacing_percent),
            "edit_button": self.preview_button.array(),
            "reset_button": self.geometry.reset.array(),
            "restore_button": self.geometry.restore.array(),
            "temporary_defaults": self.temporary_customizations,
            "save_pending": self.saving.is_some(),
            "package_settings_notice": self.package_notice.id(),
            "package_inventory_ready": self.package_inventory_ready,
            "tags_enabled": self.preview_bool(crate::automexia::presentation::TAG_ENABLED),
            "command_output_enabled": self.preview_bool(automexia_ui_model::settings::COMMAND_OUTPUT_HIGHLIGHTING),
            "log_output_enabled": self.preview_bool(automexia_ui_model::settings::OUTPUT_HIGHLIGHTING),
            "kubernetes_enabled": self.preview_bool(automexia_ui_model::settings::KUBERNETES_HIGHLIGHTING),
        })
    }
    #[cfg(test)]
    pub(crate) fn refresh(&mut self, catalog: Catalog) {
        self.refresh_with_packages(catalog, None);
    }

    #[cfg(test)]
    pub(crate) fn refresh_with_packages(
        &mut self,
        catalog: Catalog,
        packages: Option<PackageCustomizationPages>,
    ) {
        self.refresh_with_resources(catalog, packages, None);
    }

    pub(crate) fn refresh_with_resources(
        &mut self,
        catalog: Catalog,
        packages: Option<PackageCustomizationPages>,
        slot_pages: Option<SlotPageSnapshot>,
    ) {
        if !self.is_open() {
            return;
        }
        if self
            .confirmation
            .as_ref()
            .is_some_and(|confirmation| confirmation.revision != catalog.revision())
        {
            self.dismiss_confirmation();
        }
        if self.customizations.is_some() {
            self.refresh_customizations(catalog, packages, slot_pages);
            return;
        }
        if self
            .catalog
            .as_ref()
            .is_some_and(|previous| previous.revision() != catalog.revision())
        {
            self.pending = None;
            self.pressed = None;
        }
        if self.color_editor.as_ref().is_some_and(|editor| {
            editor.revision != catalog.revision()
                || catalog.get(&editor.id).is_none_or(|entry| {
                    entry.availability.reason().is_some()
                        || !editor_matches(entry, editor)
                })
        }) {
            self.cancel_color();
        }
        if self.numeric_editor.as_ref().is_some_and(|editor| {
            editor.revision != catalog.revision()
                || catalog.get(&editor.id).is_none_or(|entry| {
                    entry.availability.reason().is_some()
                        || !matches!(
                            entry.kind,
                            SettingKind::Number { .. }
                                | SettingKind::ContinuousNumber { .. }
                        )
                })
        }) {
            self.cancel_numeric();
        }
        if let Some(view) = &mut self.view {
            view.refresh(&catalog);
        }
        self.catalog = Some(catalog);
        self.layout_dirty = true;
        self.reveal_focus = true;
    }
    fn rebuild_customization_root(&mut self) {
        let Some(navigation) = self.customizations.as_mut() else {
            return;
        };
        let Ok(root) = package_root_catalog(navigation) else {
            self.status = "Customizations are temporarily unavailable.".into();
            return;
        };
        navigation.root_catalog = root.clone();
        self.catalog = Some(root.clone());
        self.view = Some(ViewState::new(&root, 6));
        self.layout_dirty = true;
        self.reveal_focus = true;
    }

    fn refresh_customizations(
        &mut self,
        catalog: Catalog,
        packages: Option<PackageCustomizationPages>,
        slot_pages: Option<SlotPageSnapshot>,
    ) {
        let Some(mut navigation) = self.customizations.take() else {
            return;
        };
        let mut groups = customization_groups(&catalog);
        let mut unavailable = false;
        navigation.full_catalog = catalog;
        navigation.slot_pages = slot_pages;
        navigation.groups = groups.clone();
        navigation.package_pages = packages;
        let root_catalog = package_root_catalog(&navigation).or_else(|_| {
            unavailable = true;
            groups.clear();
            Catalog::new(navigation.full_catalog.revision(), Vec::new())
        });
        let Ok(root_catalog) = root_catalog else {
            self.status = "Customizations are temporarily unavailable.".into();
            self.customizations = Some(navigation);
            return;
        };
        if self.catalog.as_ref().is_some_and(|previous| {
            previous.revision() != navigation.full_catalog.revision()
        }) {
            self.pending = None;
            self.pending_customization = None;
            self.pressed = None;
        }
        if self.color_editor.as_ref().is_some_and(|editor| {
            editor.revision != navigation.full_catalog.revision()
                || self
                    .catalog
                    .as_ref()
                    .and_then(|catalog| catalog.get(&editor.id))
                    .is_none_or(|entry| {
                        entry.availability.reason().is_some()
                            || !editor_matches(entry, editor)
                    })
        }) {
            self.cancel_color();
        }
        if self
            .numeric_editor
            .as_ref()
            .is_some_and(|editor| editor.revision != navigation.full_catalog.revision())
        {
            self.cancel_numeric();
        }
        navigation.root_catalog = root_catalog;
        if let Some(root_view) = &mut navigation.root_view {
            root_view.refresh(&navigation.root_catalog);
        }
        let active_key = navigation.active_key.clone();
        let mut return_to_root = false;
        if let Some(package_key) = navigation.active_package.as_ref() {
            let active_package = navigation.package_pages.as_ref().and_then(|pages| {
                pages
                    .packages
                    .iter()
                    .find(|package| &package.key == package_key)
            });
            if let Some(package) = active_package {
                let current = if let Some(key) = active_key.as_ref() {
                    package
                        .features
                        .iter()
                        .any(|feature| &feature.key == key)
                        .then(|| navigation.package_pages.as_ref()?.detail_catalog(key))
                        .flatten()
                } else {
                    navigation
                        .package_pages
                        .as_ref()
                        .and_then(|pages| pages.feature_catalog(&package.key))
                };
                if let Some(current) = current {
                    if let Some(view) = &mut self.view {
                        view.refresh(&current);
                    }
                    self.catalog = Some(current);
                } else {
                    return_to_root = true;
                }
            } else {
                return_to_root = true;
            }
        } else if let Some(key) = active_key {
            if let Some(group) = navigation.groups.iter().find(|group| group.key == key) {
                let detail = if let Some(key) = navigation.active_slot.as_ref() {
                    preview_detail_catalog(&navigation, key)
                } else {
                    detail_catalog_with_slots(
                        &navigation.full_catalog,
                        group,
                        navigation.slot_pages.as_ref(),
                    )
                };
                if let Some(detail_catalog) = detail {
                    if let Some(view) = &mut self.view {
                        view.refresh(&detail_catalog);
                    }
                    self.catalog = Some(detail_catalog);
                    if self.status == SELECTED_CONTROLS_UNAVAILABLE {
                        self.status.clear();
                    }
                } else if navigation
                    .active_slot
                    .as_ref()
                    .is_some_and(|key| preview_page_still_available(&navigation, key))
                {
                    // The selected page still exists, but its descriptors are
                    // incomplete. Keep the editor open and disable stale edits
                    // until a later resource snapshot can rebuild it.
                    if let Ok(empty) =
                        Catalog::new(navigation.full_catalog.revision(), Vec::new())
                    {
                        self.view = Some(ViewState::new(&empty, 6));
                        self.catalog = Some(empty);
                    }
                    self.pending = None;
                    self.color_editor = None;
                    self.numeric_editor = None;
                    self.focus = Focus::List;
                    self.status = SELECTED_CONTROLS_UNAVAILABLE.into();
                } else {
                    return_to_root = true;
                }
            } else {
                return_to_root = true;
            }
        } else {
            if let Some(view) = &mut self.view {
                view.refresh(&navigation.root_catalog);
            }
            self.catalog = Some(navigation.root_catalog.clone());
        }
        self.customizations = Some(navigation);
        if return_to_root {
            self.back_to_categories();
        }
        if unavailable {
            self.status = "Customizations are temporarily unavailable.".into();
        }
        self.preview_reveal_selection = true;
        self.layout_dirty = true;
        self.reveal_focus = true;
    }
    fn enter_category(&mut self) {
        let Some(key) = self.view.as_ref().and_then(ViewState::focused).cloned() else {
            return;
        };
        let Some(navigation) = self.customizations.as_mut() else {
            return;
        };
        if navigation.active_key.is_some() {
            if navigation.active_slot.is_some() || navigation.active_package.is_some() {
                return;
            }
            let Some(slot_id) = slot_id_from_page(&key) else {
                return;
            };
            let Some(detail) = navigation.slot_pages.as_ref().and_then(|snapshot| {
                selected_tag_catalog(&navigation.full_catalog, snapshot, slot_id).ok()
            }) else {
                return;
            };
            navigation.active_slot_label = self
                .catalog
                .as_ref()
                .and_then(|catalog| catalog.get(&key))
                .map(|row| row.label.clone());
            navigation.slot_parent_view = self.view.take();
            navigation.active_slot = Some(key);
            self.view = Some(ViewState::new(&detail, 6));
            self.catalog = Some(detail);
            self.focus = Focus::List;
            self.preedit.clear();
            self.caret = 0;
            self.anchor = None;
            self.scroll = 0.0;
            self.layout_dirty = true;
            self.reveal_focus = true;
            self.pressed = None;
            self.status.clear();
            return;
        }
        if let Some(package_key) = navigation.active_package.as_ref() {
            let Some(detail) = navigation
                .package_pages
                .as_ref()
                .and_then(|pages| pages.detail_catalog(&key))
            else {
                return;
            };
            if !navigation.package_pages.as_ref().is_some_and(|pages| {
                pages.packages.iter().any(|package| {
                    &package.key == package_key
                        && package.features.iter().any(|feature| feature.key == key)
                })
            }) {
                return;
            }
            navigation.package_view = self.view.take();
            navigation.active_key = Some(key);
            self.view = Some(ViewState::new(&detail, 6));
            self.catalog = Some(detail);
            self.focus = Focus::List;
            self.preedit.clear();
            self.caret = 0;
            self.anchor = None;
            self.scroll = 0.0;
            self.layout_dirty = true;
            self.reveal_focus = true;
            self.pressed = None;
            self.status.clear();
            return;
        }
        if let Some(package) = navigation
            .package_pages
            .as_ref()
            .and_then(|pages| pages.packages.iter().find(|package| package.key == key))
        {
            let Some(features) = navigation
                .package_pages
                .as_ref()
                .and_then(|pages| pages.feature_catalog(&package.key))
            else {
                return;
            };
            navigation.root_view = self.view.take();
            navigation.active_package = Some(key);
            self.view = Some(ViewState::new(&features, 6));
            self.catalog = Some(features);
            self.focus = Focus::List;
            self.preedit.clear();
            self.caret = 0;
            self.anchor = None;
            self.scroll = 0.0;
            self.layout_dirty = true;
            self.reveal_focus = true;
            self.pressed = None;
            self.status.clear();
            return;
        }
        let Some(group) = navigation.groups.iter().find(|group| group.key == key) else {
            return;
        };
        let detail = detail_catalog_with_slots(
            &navigation.full_catalog,
            group,
            navigation.slot_pages.as_ref(),
        );
        let Some(detail) = detail else {
            return;
        };
        navigation.root_view = self.view.take();
        navigation.active_key = Some(key);
        self.view = Some(ViewState::new(&detail, 6));
        self.catalog = Some(detail);
        self.focus = Focus::List;
        self.preedit.clear();
        self.caret = 0;
        self.anchor = None;
        self.scroll = 0.0;
        self.layout_dirty = true;
        self.reveal_focus = true;
        self.pressed = None;
        self.status.clear();
    }

    fn preview_items(&self) -> Vec<(SettingId, String)> {
        self.preview_order
            .iter()
            .filter_map(|id| Some((id.clone(), self.preview_item_label(id)?)))
            .collect()
    }

    fn preview_item_label(&self, key: &SettingId) -> Option<String> {
        if key.as_str() == "tags.add-slot" {
            return Some("Add custom tag".into());
        }
        if let Some(slot_id) = slot_id_from_page(key) {
            return self
                .customizations
                .as_ref()?
                .slot_pages
                .as_ref()
                .and_then(|snapshot| slot_page_actions(snapshot).ok())?
                .into_iter()
                .find(|entry| slot_id_from_page(&entry.id) == Some(slot_id))
                .map(|entry| entry.label);
        }
        if let Some(kind) = key.as_str().strip_prefix("command_output.band.") {
            return Some(
                match kind {
                    "success" => "Successful command color",
                    "failure" => "Failed command color",
                    "neutral" => "Unknown command result color",
                    _ => return None,
                }
                .into(),
            );
        }
        if let Some(severity) = key.as_str().strip_prefix("kubernetes.severity.") {
            return crate::automexia::presentation::KUBERNETES_COLOR_BINDINGS
                .iter()
                .find(|binding| binding.id == format!("kubernetes.colors.{severity}"))
                .map(|binding| format!("{} Kubernetes colors", binding.label));
        }
        let severity = key.as_str().strip_prefix("output.severity.")?;
        crate::automexia::presentation::OUTPUT_COLOR_BINDINGS
            .iter()
            .find(|binding| binding.id == format!("output.colors.{severity}"))
            .map(|binding| format!("{} output colors", binding.label))
    }

    fn preview_selector_available(&self) -> bool {
        self.customizations
            .as_ref()
            .and_then(|navigation| navigation.active_key.as_ref())
            .is_some_and(|key| {
                matches!(
                    key.as_str(),
                    crate::automexia::presentation::TAG_ENABLED
                        | automexia_ui_model::settings::COMMAND_OUTPUT_HIGHLIGHTING
                        | automexia_ui_model::settings::KUBERNETES_HIGHLIGHTING
                )
            })
    }

    fn is_tag_preview(&self) -> bool {
        self.customizations
            .as_ref()
            .and_then(|navigation| navigation.active_key.as_ref())
            .is_some_and(|key| {
                key.as_str() == crate::automexia::presentation::TAG_ENABLED
            })
    }

    fn start_preview_edit(&mut self) {
        if !self.preview_selector_available() || self.requires_larger_window() {
            return;
        }
        if self
            .customizations
            .as_ref()
            .is_some_and(|navigation| navigation.active_slot.is_some())
        {
            self.back_to_categories();
            if !self.preview_selector_available() {
                return;
            }
        }
        let items = self.preview_items();
        if self
            .preview_selected
            .as_ref()
            .is_none_or(|selected| !items.iter().any(|(id, _)| id == selected))
        {
            self.preview_selected = items.first().map(|(id, _)| id.clone());
            self.preview_scroll = 0;
        }
        self.preview_edit_mode = true;
        self.preview_reveal_selection = true;
        self.focus = Focus::Preview;
        self.layout_dirty = true;
        self.pressed = None;
    }

    fn stop_preview_edit(&mut self) {
        self.preview_edit_mode = false;
        self.focus = Focus::PreviewButton;
        self.pressed = None;
        self.layout_dirty = true;
    }

    fn activate_preview_button(&mut self) {
        if self.preview_edit_mode {
            self.stop_preview_edit();
        } else {
            self.start_preview_edit();
        }
    }

    fn move_preview(&mut self, key: &Key, reverse: bool) {
        let items = self.preview_items();
        if items.is_empty() {
            return;
        }
        let current = self
            .preview_selected
            .as_ref()
            .and_then(|selected| items.iter().position(|(id, _)| id == selected))
            .unwrap_or(0);
        let page = self.preview_page_rows().max(1);
        let next = match key {
            Key::Named(NamedKey::Tab) if reverse => {
                (current + items.len() - 1) % items.len()
            }
            Key::Named(NamedKey::Tab) => (current + 1) % items.len(),
            Key::Named(NamedKey::ArrowDown | NamedKey::ArrowRight) => {
                current.saturating_add(1)
            }
            Key::Named(NamedKey::ArrowUp | NamedKey::ArrowLeft) => {
                current.saturating_sub(1)
            }
            Key::Named(NamedKey::PageDown) => current.saturating_add(page),
            Key::Named(NamedKey::PageUp) => current.saturating_sub(page),
            Key::Named(NamedKey::Home) => 0,
            Key::Named(NamedKey::End) => items.len() - 1,
            _ => return,
        }
        .min(items.len() - 1);
        self.preview_selected = Some(items[next].0.clone());
        self.preview_reveal_selection = true;
        if let Some((_, row)) = self
            .preview_item_rows
            .iter()
            .find(|(id, _)| id == &items[next].0)
        {
            if *row < self.preview_scroll {
                self.preview_scroll = *row;
            } else if *row >= self.preview_scroll.saturating_add(page) {
                self.preview_scroll = row + 1 - page;
            }
        }
        if self.is_tag_preview() {
            // The roster owns selection order, including the Add action. Keep
            // it visible even when the same tag also has a graphic sample.
            let page = self.preview_tag_list_visible_rows.max(1);
            if next < self.preview_tag_list_scroll {
                self.preview_tag_list_scroll = next;
            } else if next >= self.preview_tag_list_scroll + page {
                self.preview_tag_list_scroll = next + 1 - page;
            }
        }
        self.pressed = None;
        self.layout_dirty = true;
    }

    fn preview_page_rows(&self) -> usize {
        self.preview_visible_rows.max(1)
    }

    fn enter_preview_item(&mut self, key: SettingId) {
        if key.as_str() == "tags.add-slot" {
            if self.pending.is_none() {
                if let Some(catalog) = self.catalog.as_ref() {
                    let edit = Edit {
                        revision: catalog.revision(),
                        id: key,
                        change: Change::Activate,
                    };
                    if let Ok(edit) = catalog.validate_edit(&edit) {
                        self.pending = Some(edit);
                        self.status.clear();
                    }
                }
            }
            return;
        }
        let label = self.preview_item_label(&key);
        let Some(navigation) = self.customizations.as_mut() else {
            return;
        };
        if navigation.active_package.is_some() {
            return;
        }
        let Some(detail) = preview_detail_catalog(navigation, &key) else {
            return;
        };
        navigation.active_slot_label = label;
        if navigation.active_slot.is_none() {
            navigation.slot_parent_view = self.view.take();
        }
        navigation.active_slot = Some(key.clone());
        self.preview_edit_mode = false;
        self.preview_selected = Some(key);
        self.preview_reveal_selection = true;
        self.view = Some(ViewState::new(&detail, 6));
        self.catalog = Some(detail);
        self.focus = Focus::List;
        self.preedit.clear();
        self.caret = 0;
        self.anchor = None;
        self.scroll = 0.0;
        self.layout_dirty = true;
        self.reveal_focus = true;
        self.pressed = None;
        self.status.clear();
    }
    fn back_to_categories(&mut self) {
        self.cancel_numeric();
        self.preview_edit_mode = false;
        let Some(navigation) = self.customizations.as_mut() else {
            return;
        };
        if navigation.active_slot.take().is_some() {
            navigation.active_slot_label = None;
            let detail = navigation
                .active_key
                .as_ref()
                .and_then(|key| navigation.groups.iter().find(|group| &group.key == key))
                .and_then(|group| {
                    detail_catalog_with_slots(
                        &navigation.full_catalog,
                        group,
                        navigation.slot_pages.as_ref(),
                    )
                });
            if let Some(detail) = detail {
                self.view = Some(
                    navigation
                        .slot_parent_view
                        .take()
                        .unwrap_or_else(|| ViewState::new(&detail, 6)),
                );
                if let Some(view) = &mut self.view {
                    view.refresh(&detail);
                    self.caret = view.query().len();
                }
                self.catalog = Some(detail);
                self.pending = None;
                self.color_editor = None;
                self.focus = Focus::List;
                self.preedit.clear();
                self.anchor = None;
                self.scroll = 0.0;
                self.layout_dirty = true;
                self.reveal_focus = true;
                self.pressed = None;
                self.status.clear();
                return;
            }
        }
        if navigation.active_package.is_some() && navigation.active_key.take().is_some() {
            let package_key = navigation.active_package.as_ref();
            let Some(features) = package_key
                .and_then(|key| navigation.package_pages.as_ref()?.feature_catalog(key))
            else {
                navigation.active_package = None;
                navigation.package_view = None;
                self.catalog = Some(navigation.root_catalog.clone());
                self.view = Some(ViewState::new(&navigation.root_catalog, 6));
                self.pending = None;
                self.color_editor = None;
                self.layout_dirty = true;
                return;
            };
            self.catalog = Some(features.clone());
            self.view = Some(
                navigation
                    .package_view
                    .take()
                    .unwrap_or_else(|| ViewState::new(&features, 6)),
            );
            if let Some(view) = &mut self.view {
                view.refresh(&features);
                self.caret = view.query().len();
            }
            self.pending = None;
            self.color_editor = None;
            self.focus = Focus::List;
            self.preedit.clear();
            self.anchor = None;
            self.scroll = 0.0;
            self.layout_dirty = true;
            self.reveal_focus = true;
            self.pressed = None;
            self.status.clear();
            return;
        }
        if navigation.active_package.take().is_none()
            && navigation.active_key.take().is_none()
        {
            return;
        }
        navigation.package_view = None;
        navigation.slot_parent_view = None;
        navigation.active_slot_label = None;
        self.preview_edit_mode = false;
        self.preview_selected = None;
        self.preview_scroll = 0;
        self.preview_tag_list_scroll = 0;
        self.color_editor = None;
        self.pending = None;
        self.catalog = Some(navigation.root_catalog.clone());
        self.view = Some(
            navigation
                .root_view
                .take()
                .unwrap_or_else(|| ViewState::new(&navigation.root_catalog, 6)),
        );
        if let Some(view) = &mut self.view {
            view.refresh(&navigation.root_catalog);
            self.caret = view.query().len();
        }
        self.focus = Focus::List;
        self.preedit.clear();
        self.anchor = None;
        self.scroll = 0.0;
        self.layout_dirty = true;
        self.reveal_focus = true;
        self.pressed = None;
        self.status.clear();
    }
    pub(crate) fn fit(&mut self, width: f32, height: f32, font: f32) {
        let clean = |value: f32, max: f32| {
            if value.is_finite() {
                value.clamp(0.0, max)
            } else {
                0.0
            }
        };
        let width = clean(width, 32768.0);
        let height = clean(height, 32768.0);
        let font = if font.is_finite() {
            font.clamp(10.0, 64.0)
        } else {
            14.0
        };
        if (width, height, font) != (self.width, self.height, self.font) {
            self.width = width;
            self.height = height;
            self.font = font;
            self.preview_reveal_selection = true;
            self.layout_dirty = true;
            self.reveal_focus = true;
            self.pressed = None;
            self.preedit.clear();
            if let Some(editor) = &mut self.color_editor {
                editor.composing = false;
            }
            if let Some(editor) = &mut self.numeric_editor {
                editor.composing = false;
            }
            if self.color_editor.is_some() && self.color_requires_larger_window() {
                self.cancel_color();
            }
            if self.requires_larger_window() {
                self.pending = None;
                self.preedit.clear();
                self.anchor = None;
                self.cancel_numeric();
            }
        }
    }
    pub(crate) fn event(
        &mut self,
        event: &WindowEvent,
        modifiers: ModifiersState,
        scale: f64,
    ) -> EventResult {
        if !self.is_open() {
            return EventResult::default();
        }
        let scale = if scale.is_finite() && scale > 0.0 {
            scale
        } else {
            1.0
        };
        let mut result = EventResult {
            consumed: true,
            redraw: true,
            closed: false,
        };
        match event {
            WindowEvent::Ime(_) | WindowEvent::MouseWheel { .. }
                if self.confirmation.is_some() => {}
            WindowEvent::KeyboardInput {
                event,
                is_synthetic,
                ..
            } => {
                if event.state == ElementState::Pressed && !is_synthetic {
                    self.key(
                        &event.logical_key,
                        event.text.as_deref(),
                        modifiers,
                        event.repeat,
                    );
                }
            }
            WindowEvent::Ime(Ime::Commit(text)) => {
                // A commit belongs to an existing text editor. It must never
                // reopen a canceled numeric draft or act on a focused switch.
                let accepts_commit = self.numeric_editor.is_some()
                    || self
                        .color_editor
                        .as_ref()
                        .is_some_and(|editor| editor.focus == ColorFocus::Hex)
                    || (self.color_editor.is_none() && self.focus == Focus::Search);
                self.preedit.clear();
                if let Some(editor) = &mut self.color_editor {
                    editor.composing = false;
                }
                if let Some(editor) = &mut self.numeric_editor {
                    editor.composing = false;
                }
                if accepts_commit {
                    self.paste(text);
                }
            }
            WindowEvent::Ime(Ime::Preedit(text, _)) => {
                self.preedit.clear();
                if let Some(editor) = &mut self.color_editor {
                    editor.composing =
                        editor.focus == ColorFocus::Hex && !text.is_empty();
                    let start = editor.anchor.unwrap_or(editor.caret).min(editor.caret);
                    let end = editor.anchor.unwrap_or(editor.caret).max(editor.caret);
                    let max = editor.text_limits.map_or(MAX_COLOR_BYTES, |(max, _)| max);
                    let valid = if editor.text_limits.is_some() {
                        safe_editor_text(text, max, true)
                    } else {
                        text.bytes().all(|byte| byte.is_ascii_graphic())
                    };
                    if editor.focus == ColorFocus::Hex
                        && valid
                        && editor.draft.len() - (end - start) + text.len() <= max
                    {
                        self.preedit = text.clone();
                    }
                } else if let Some(editor) = &mut self.numeric_editor {
                    editor.composing = !text.is_empty();
                    let start = editor.anchor.unwrap_or(editor.caret).min(editor.caret);
                    let end = editor.anchor.unwrap_or(editor.caret).max(editor.caret);
                    if numeric_fragment(text)
                        && editor.draft.len() - (end - start) + text.len()
                            <= MAX_NUMERIC_BYTES
                    {
                        self.preedit = text.clone();
                    }
                } else if self.focus == Focus::Search
                    && !self.requires_larger_window()
                    && self.query().len().saturating_add(text.len()) <= MAX_QUERY_BYTES
                {
                    if let Some((mut probe, catalog)) =
                        self.view.clone().zip(self.catalog.as_ref())
                    {
                        if probe.set_query(text, catalog).is_ok() {
                            self.preedit = text.clone();
                        }
                    }
                }
            }
            WindowEvent::Ime(Ime::Disabled) => {
                if let Some(editor) = &mut self.color_editor {
                    editor.composing = false;
                }
                if let Some(editor) = &mut self.numeric_editor {
                    editor.composing = false;
                }
                self.preedit.clear();
            }
            WindowEvent::Ime(Ime::Enabled) => {}
            WindowEvent::CursorMoved { position, .. } => {
                let point = ((position.x / scale) as f32, (position.y / scale) as f32);
                self.pointer =
                    (point.0.is_finite() && point.1.is_finite()).then_some(point);
            }
            WindowEvent::CursorLeft { .. } => {
                self.pointer = None;
                self.pressed = None;
            }
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state,
                ..
            } => {
                let target = self.pointer.and_then(|(x, y)| self.target_at(x, y));
                if *state == ElementState::Pressed {
                    self.pressed = target;
                } else if let Some(pressed) = self.pressed.take() {
                    if target.as_ref() == Some(&pressed) {
                        self.activate_target(pressed);
                    }
                }
            }
            WindowEvent::MouseInput { .. } => {}
            WindowEvent::MouseWheel { delta, .. } => {
                let amount = match delta {
                    MouseScrollDelta::LineDelta(_, y) => *y * self.font.max(10.0) * 3.0,
                    MouseScrollDelta::PixelDelta(point) => (point.y / scale) as f32,
                };
                if (self.preview_edit_mode || self.is_tag_preview())
                    && self
                        .pointer
                        .is_some_and(|(x, y)| self.geometry.preview.contains(x, y))
                {
                    self.scroll_preview(-amount, self.pointer);
                } else {
                    self.scroll_by(-amount);
                }
            }
            WindowEvent::Touch(touch) => {
                let x = (touch.location.x / scale) as f32;
                let y = (touch.location.y / scale) as f32;
                if !x.is_finite() || !y.is_finite() {
                    self.touch = None;
                    self.pressed = None;
                    return result;
                }
                match touch.phase {
                    TouchPhase::Started if self.touch.is_none() => {
                        self.touch = Some((touch.id, (x, y), y));
                        self.pressed = self.target_at(x, y);
                    }
                    TouchPhase::Moved => {
                        if let Some((id, origin, previous)) = self.touch {
                            if id == touch.id {
                                let delta = previous - y;
                                if (origin.1 - y).abs() > 2.0
                                    || (origin.0 - x).abs() > 2.0
                                {
                                    self.pressed = None;
                                }
                                if (self.preview_edit_mode || self.is_tag_preview())
                                    && self.geometry.preview.contains(origin.0, origin.1)
                                {
                                    self.scroll_preview(delta, Some(origin));
                                } else {
                                    self.scroll_by(delta);
                                }
                                self.touch = Some((id, origin, y));
                            }
                        }
                    }
                    TouchPhase::Ended | TouchPhase::Cancelled
                        if self.touch.is_some_and(|(id, _, _)| id == touch.id) =>
                    {
                        self.touch = None;
                        if let Some(target) = self.pressed.take() {
                            if touch.phase == TouchPhase::Ended
                                && self.target_at(x, y).as_ref() == Some(&target)
                            {
                                self.activate_target(target);
                            }
                        }
                    }
                    _ => {}
                }
            }
            WindowEvent::DroppedFile(_)
            | WindowEvent::HoveredFile(_)
            | WindowEvent::HoveredFileCancelled => {}
            WindowEvent::Focused(false) => {
                self.dismiss_confirmation();
                self.cancel_color();
                self.cancel_numeric();
                self.preedit.clear();
                self.pressed = None;
                self.touch = None;
                result.consumed = false;
            }
            _ => {
                result.consumed = false;
                result.redraw = false;
            }
        }
        result.closed = !self.is_open();
        result
    }
    pub(crate) fn draw(&mut self, sugarloaf: &mut Sugarloaf, theme: UiTheme) {
        if !self.is_open() {
            return;
        }
        sugarloaf.begin_modal_layer();
        self.paint(sugarloaf, theme);
        sugarloaf.end_modal_layer();
    }
    pub(crate) fn take_edit(&mut self) -> Option<Edit> {
        self.pending.take()
    }
    pub(crate) fn take_customization_intent(&mut self) -> Option<CustomizationIntent> {
        self.pending_customization.take()
    }
    pub(crate) fn set_temporary_customizations(&mut self, temporary: bool) {
        let mut changed = self.temporary_customizations != temporary;
        if changed {
            self.dismiss_confirmation();
        }
        self.temporary_customizations = temporary;
        if changed && temporary {
            // A write submitted before Reset may complete after Restore saved.
            // Retire only this view's receipt token; the writer still flushes.
            self.saving = None;
        }
        if !temporary && self.focus == Focus::Restore {
            self.focus = Focus::Reset;
            changed = true;
        }
        if changed {
            self.layout_dirty = true;
        }
    }
    pub(crate) fn save_started(&mut self, revision: u64) {
        if !self.is_open() {
            return;
        }
        self.saving = Some(revision);
        self.status = "Saving settings...".into();
    }
    pub(crate) fn save_completed(&mut self, revision: u64, success: bool) {
        if self.saving.is_none_or(|pending| revision < pending) {
            return;
        }
        self.saving = None;
        if self.temporary_customizations {
            self.status = "Temporary preview only. Restore saved to return.".into();
        } else if success {
            self.status = "Saved".into();
        } else {
            self.save_failed();
        }
    }
    pub(crate) fn requires_larger_window(&self) -> bool {
        let font = self.font.max(10.0);
        let margin = if self.width < 360.0 || self.height < 300.0 {
            4.0
        } else {
            16.0
        };
        let width = (self.width - 2.0 * margin).max(0.0);
        let height = (self.height - 2.0 * margin).max(0.0);
        let pad = 8.0_f32.min(width * 0.1);
        // Room for full-height search/footer controls and a two-line label with
        // its value. Below this geometry hidden controls cannot safely be edited.
        width < font * 6.0 + pad * 4.0 || height < font * 1.45 * 7.0 + pad * 8.0 + 4.0
    }

    pub(crate) fn ime_cursor_area(&self) -> Option<[f32; 4]> {
        if !self.is_open()
            || self.confirmation.is_some()
            || (self
                .color_editor
                .as_ref()
                .map_or(self.focus != Focus::Search, |editor| {
                    editor.focus != ColorFocus::Hex
                })
                && self.numeric_editor.is_none())
            || self.layout_dirty
            || self.requires_larger_window()
        {
            return None;
        }
        let input = if self.color_editor.is_some() {
            self.color_geometry.input
        } else if let Some(editor) = &self.numeric_editor {
            self.rows
                .iter()
                .find(|row| row.id == editor.id)
                .and_then(|row| {
                    numeric_zones(row.control, self.font)
                        .1
                        .intersect(self.geometry.body)
                })?
        } else {
            self.geometry.search
        };
        self.caret_rect.intersect(input).map(Rect::array)
    }
    pub(crate) fn save_failed(&mut self) {
        self.saving = None;
        if self.temporary_customizations {
            self.set_status("Temporary preview only. Restore saved to return.");
        } else {
            self.set_status(
                "Active this session; could not be saved. Change a value to retry.",
            );
        }
    }
    pub(crate) fn restore_save_status(
        &mut self,
        status: crate::automexia::preferences::PreferenceSaveStatus,
    ) {
        use crate::automexia::preferences::PreferenceSaveStatus;
        if !self.is_open() || self.temporary_customizations {
            return;
        }
        match status {
            PreferenceSaveStatus::Pending(revision) => self.save_started(revision),
            PreferenceSaveStatus::Failed => self.save_failed(),
            PreferenceSaveStatus::Idle | PreferenceSaveStatus::Saved(_) => {}
        }
    }
    pub(crate) fn set_status(&mut self, message: &str) {
        if !self.is_open() {
            return;
        }
        if message.len() <= 256 && !message.chars().any(char::is_control) {
            self.status = message.into();
        } else {
            self.status = "Settings could not be applied.".into();
        }
        if let Some(editor) = &mut self.color_editor {
            editor.feedback = (!self.status.is_empty()).then(|| self.status.clone());
        }
    }
    pub(crate) fn set_package_inventory_status(
        &mut self,
        status: PackageInventoryStatus,
        projection_failed: bool,
    ) {
        #[cfg(feature = "native-gui-test-hooks")]
        {
            self.package_inventory_ready =
                self.customizations.is_some() && status == PackageInventoryStatus::Ready;
        }
        self.package_notice = if self.customizations.is_none() {
            PackageSettingsNotice::None
        } else if projection_failed {
            PackageSettingsNotice::ProjectionFailed
        } else {
            match status {
                PackageInventoryStatus::Loading => PackageSettingsNotice::Loading,
                PackageInventoryStatus::Unavailable => PackageSettingsNotice::Unavailable,
                PackageInventoryStatus::Unloaded | PackageInventoryStatus::Ready => {
                    PackageSettingsNotice::None
                }
            }
        };
    }

    /// Resource notices survive navigation without becoming editor feedback or
    /// replacing save/validation errors. Preview context stays visible too.
    fn status_message(&self) -> Option<&str> {
        let preview_status = matches!(
            self.status.as_str(),
            "Temporary defaults. Restore saved to return."
                | "Temporary preview only. Restore saved to return."
                | "Temporary preview active. Restore saved in Customizations."
        );
        if !self.status.is_empty()
            && self.status != "Saved"
            && !(self.temporary_customizations && preview_status)
        {
            return Some(&self.status);
        }
        if self.temporary_customizations {
            return Some(match self.package_notice {
                PackageSettingsNotice::Loading => {
                    "Preview only. Loading package settings..."
                }
                PackageSettingsNotice::Unavailable
                | PackageSettingsNotice::ProjectionFailed => {
                    "Preview only. Packages unavailable; reopen to retry."
                }
                PackageSettingsNotice::None if preview_status => &self.status,
                PackageSettingsNotice::None => {
                    "Temporary defaults. Restore saved to return."
                }
            });
        }
        self.package_notice
            .message()
            .or_else(|| (!self.status.is_empty()).then_some(self.status.as_str()))
    }
    pub(crate) fn paste(&mut self, text: &str) -> bool {
        if self.confirmation.is_some() {
            return false;
        }
        if self.color_editor.is_some() {
            return self.paste_color(text);
        }
        if self.numeric_editor.is_some() {
            return self.paste_numeric(text);
        }
        if self.focus == Focus::List
            && !text.is_empty()
            && numeric_fragment(text)
            && self.focused_entry().is_some_and(|entry| {
                matches!(
                    entry.kind,
                    SettingKind::Number { .. } | SettingKind::ContinuousNumber { .. }
                )
            })
        {
            self.open_numeric();
            return self.paste_numeric(text);
        }
        if !self.is_open()
            || self.requires_larger_window()
            || self.focus != Focus::Search
            || text.len() > MAX_QUERY_BYTES
        {
            return false;
        }
        let query = self.query();
        let end = self.caret.min(query.len());
        let start = self.anchor.unwrap_or(end).min(end);
        let finish = self.anchor.unwrap_or(end).max(end).min(query.len());
        if !query.is_char_boundary(start) || !query.is_char_boundary(finish) {
            return false;
        }
        let mut candidate = query.to_owned();
        candidate.replace_range(start..finish, text);
        let Some((view, catalog)) = self.view.as_mut().zip(self.catalog.as_ref()) else {
            return false;
        };
        if view.set_query(&candidate, catalog).is_err() {
            self.status =
                "Search text is too long or contains unsupported characters.".into();
            return false;
        }
        self.caret = start + text.len();
        self.anchor = None;
        self.layout_dirty = true;
        self.scroll = 0.0;
        self.reveal_focus = false;
        self.pressed = None;
        true
    }

    /// Selection from the focused editor only; never from terminal history.
    pub(crate) fn clipboard_selection(&self) -> Option<String> {
        if !self.is_open()
            || self.confirmation.is_some()
            || self.requires_larger_window()
            || !self.preedit.is_empty()
        {
            return None;
        }
        let (value, caret, anchor) = if let Some(editor) = &self.color_editor {
            if editor.focus != ColorFocus::Hex
                || editor.composing
                || self.color_requires_larger_window()
            {
                return None;
            }
            (editor.draft.as_str(), editor.caret, editor.anchor?)
        } else if let Some(editor) = &self.numeric_editor {
            if editor.composing {
                return None;
            }
            (editor.draft.as_str(), editor.caret, editor.anchor?)
        } else if self.focus == Focus::Search {
            (self.query(), self.caret, self.anchor?)
        } else {
            return None;
        };
        if caret == anchor {
            return None;
        }
        let selected = value.get(caret.min(anchor)..caret.max(anchor))?.to_owned();
        Some(selected)
    }
    #[cfg(test)]
    pub(crate) fn accessibility_summary(&self) -> String {
        if let Some(confirmation) = &self.confirmation {
            return format!(
                "{} {} Cancel: Escape. {}: Y. Tab selects; Enter activates {}.",
                confirmation.title,
                confirmation.description,
                confirmation.accept_label,
                if confirmation.accept_selected {
                    confirmation.accept_label
                } else {
                    "Cancel"
                }
            );
        }
        let mut summary = self.accessibility_summary_without_package_notice();
        if self.is_open() {
            if let Some(notice) = self.package_notice.message() {
                summary.push(' ');
                summary.push_str(notice);
            }
        }
        summary
    }

    #[cfg(test)]
    fn accessibility_summary_without_package_notice(&self) -> String {
        if !self.is_open() {
            return String::new();
        }
        if self.requires_larger_window() {
            return format!(
                "{}. Enlarge the window to use these controls. Escape {}.",
                self.title(),
                if self.is_category_detail() {
                    if self.back_destination() == "Information tags" {
                        "returns to Information tags"
                    } else {
                        "returns to categories"
                    }
                } else {
                    "closes the sheet"
                }
            );
        }
        if let Some(editor) = &self.numeric_editor {
            let valid = editor
                .draft
                .parse::<f64>()
                .ok()
                .filter(|value| value.is_finite())
                .map(|value| Edit {
                    revision: editor.revision,
                    id: editor.id.clone(),
                    change: Change::Set(SettingValue::Number(value)),
                })
                .is_some_and(|edit| {
                    self.catalog
                        .as_ref()
                        .is_some_and(|catalog| catalog.validate_edit(&edit).is_ok())
                });
            return format!("Edit number. Draft: {}. {}. Enter applies, Escape cancels, Tab applies and moves focus. The minus and plus buttons remain available.", editor.draft,
                if valid { "Ready to apply" } else { "Invalid number or outside the allowed range and step" });
        }
        if let Some(editor) = &self.color_editor {
            let name = self
                .catalog
                .as_ref()
                .and_then(|catalog| catalog.get(&editor.id))
                .map_or("Value", |entry| entry.label.as_str());
            let valid = editor_value(editor).is_some();
            if editor.text_limits.is_some() {
                return format!("Edit text. {name}. Draft: {}. {}. Tab changes focus between text, Apply, Cancel and Reset. Escape cancels the draft.", editor.draft,
                    if valid { "Ready to apply" } else { "Invalid text; Apply unavailable" });
            }
            return format!("Edit color. {name}. Hex value: {}. {}. Tab changes focus between Hex, Apply, Cancel and Reset. Escape cancels the color draft. Reset restores configuration.", editor.draft, if valid { "Ready to apply" } else { "Invalid color; Apply unavailable" });
        }
        let mut text = format!(
            "{}. Search: {}. {}",
            self.title(),
            self.query(),
            self.status
        );
        if let Some(entry) = self.focused_entry() {
            text.push_str(&format!(
                " {}. {}. {}. {}.",
                entry.section.label(),
                entry.label,
                display_value(entry),
                entry.description
            ));
            if let Some(reason) = entry.availability.reason() {
                text.push_str(reason);
            }
        }
        if self.is_category_root() {
            text.push_str(
                " Tab: focus. Enter: open. R: confirm Reset all. S: confirm Restore saved when available. C or Esc: close.",
            );
        } else if self.is_category_detail() {
            text.push_str(&format!(" Tab: focus. Arrows: choose. R: confirm reset of this part. S: confirm Restore saved when available. C: close. Esc: back to {}.", self.back_destination()));
            if self.preview_selector_available() {
                text.push_str(" Click a preview element to edit it. E or the preview Edit button starts keyboard selection outside text fields. From item controls, E returns to preview selection and Escape returns to shared controls.");
            }
            if self.focus == Focus::PreviewButton {
                text.push_str(" Preview Edit button focused. Enter or Space enables selection in the live preview.");
            } else if self.focus == Focus::Preview {
                let name = self
                    .preview_items()
                    .into_iter()
                    .find(|(id, _)| self.preview_selected.as_ref() == Some(id))
                    .map_or_else(|| "No element".to_owned(), |(_, label)| label);
                text.push_str(&format!(" Preview element: {name}. Tab and Shift+Tab cycle only preview elements. Arrows, Home, End and Page keys also select; Enter or Space opens its controls; Done or Escape leaves preview selection."));
            }
        } else {
            text.push_str(
                " Tab: focus. Arrows: choose. R: confirm reset of this value. C or Esc: close.",
            );
        }
        if self.temporary_customizations {
            text.push_str(" Preview only. Restore saved: previous choices.");
        }
        text
    }
    fn query(&self) -> &str {
        self.view.as_ref().map_or("", ViewState::query)
    }
    pub(crate) fn key(
        &mut self,
        key: &Key,
        text: Option<&str>,
        modifiers: ModifiersState,
        repeat: bool,
    ) {
        if !self.is_open() {
            return;
        }
        if self.confirmation.is_some() {
            self.confirmation_key(key, modifiers, repeat);
            return;
        }
        if self.color_editor.is_some() {
            self.color_key(key, text, modifiers, repeat);
            return;
        }
        if self.numeric_editor.is_some() {
            self.numeric_key(key, text, modifiers, repeat);
            return;
        }
        if self.is_category_detail()
            && modifiers.alt_key()
            && matches!(key, Key::Named(NamedKey::ArrowLeft))
        {
            if self.preview_edit_mode
                && self.focus == Focus::Preview
                && self
                    .customizations
                    .as_ref()
                    .is_some_and(|navigation| navigation.active_slot.is_none())
            {
                self.stop_preview_edit();
                return;
            }
            self.back_to_categories();
            return;
        }
        if matches!(key, Key::Named(NamedKey::Escape)) {
            if self.preedit.is_empty() {
                if self.preview_edit_mode
                    && self.focus == Focus::Preview
                    && self
                        .customizations
                        .as_ref()
                        .is_some_and(|navigation| navigation.active_slot.is_none())
                {
                    self.stop_preview_edit();
                } else if self.is_category_detail() {
                    self.back_to_categories();
                } else {
                    self.close();
                }
            } else {
                self.preedit.clear();
            }
            return;
        }
        if self.requires_larger_window() || !self.preedit.is_empty() {
            return;
        }
        if !repeat
            && self.focus != Focus::Search
            && !modifiers.control_key()
            && !modifiers.super_key()
            && !modifiers.alt_key()
        {
            if let Key::Character(value) = key {
                if value.eq_ignore_ascii_case("r") {
                    self.request_reset();
                    return;
                }
                if value.eq_ignore_ascii_case("s") && self.customizations.is_some() {
                    self.request_restore_saved();
                    return;
                }
                if value.eq_ignore_ascii_case("c") {
                    self.close();
                    return;
                }
            }
        }
        // The focused editor owns text. This mnemonic belongs only to the
        // existing customization view, never to the application or the PTY.
        if !repeat
            && self.focus != Focus::Search
            && !modifiers.control_key()
            && !modifiers.super_key()
            && !modifiers.alt_key()
            && matches!(key, Key::Character(value) if value.eq_ignore_ascii_case("e"))
            && self.preview_selector_available()
        {
            self.start_preview_edit();
            return;
        }
        if matches!(key, Key::Named(NamedKey::Tab)) {
            if self.preview_edit_mode && self.focus == Focus::Preview {
                self.move_preview(key, modifiers.shift_key());
                return;
            }
            let mut stops = vec![Focus::Search, Focus::List];
            if self.preview_selector_available() {
                stops.push(Focus::PreviewButton);
            }
            stops.push(Focus::Reset);
            if self.customizations.is_some() && self.temporary_customizations {
                stops.push(Focus::Restore);
            }
            stops.push(Focus::Close);
            let index = stops
                .iter()
                .position(|stop| *stop == self.focus)
                .unwrap_or(0);
            let next = if modifiers.shift_key() {
                (index + stops.len() - 1) % stops.len()
            } else {
                (index + 1) % stops.len()
            };
            self.focus = stops[next];
            self.reveal_focus = self.focus == Focus::List;
            return;
        }
        if self.focus == Focus::PreviewButton {
            if !repeat && matches!(key, Key::Named(NamedKey::Enter | NamedKey::Space)) {
                self.activate_preview_button();
            }
            return;
        }
        if self.focus == Focus::Preview {
            if !repeat && matches!(key, Key::Named(NamedKey::Enter | NamedKey::Space)) {
                if let Some(selected) = self.preview_selected.clone() {
                    self.enter_preview_item(selected);
                }
            } else {
                self.move_preview(key, false);
            }
            return;
        }
        if self.focus == Focus::Search {
            if (modifiers.control_key() || modifiers.super_key())
                && matches!(key,Key::Character(value) if value.eq_ignore_ascii_case("a"))
            {
                self.anchor = Some(0);
                self.caret = self.query().len();
                return;
            }
            let old_caret = self.caret;
            let selecting = matches!(
                key,
                Key::Named(
                    NamedKey::ArrowLeft
                        | NamedKey::ArrowRight
                        | NamedKey::Home
                        | NamedKey::End
                )
            );
            match key {
                Key::Named(NamedKey::ArrowDown) => {
                    self.focus = Focus::List;
                    self.reveal_focus = true;
                }
                Key::Named(NamedKey::ArrowLeft) => {
                    self.caret = self.query()[..self.caret]
                        .grapheme_indices(true)
                        .next_back()
                        .map_or(0, |(start, _)| start);
                }
                Key::Named(NamedKey::ArrowRight) => {
                    self.caret = self.query()[self.caret..]
                        .graphemes(true)
                        .next()
                        .map_or(self.caret, |next| self.caret + next.len());
                }
                Key::Named(NamedKey::Home) => {
                    self.caret = 0;
                }
                Key::Named(NamedKey::End) => {
                    self.caret = self.query().len();
                }
                Key::Named(NamedKey::Backspace) => {
                    if self.anchor == Some(self.caret) {
                        self.anchor = None;
                    }
                    if self.anchor.is_none() {
                        self.anchor = self.query()[..self.caret]
                            .grapheme_indices(true)
                            .next_back()
                            .map(|(start, _)| start);
                    }
                    self.paste("");
                }
                Key::Named(NamedKey::Delete) => {
                    if self.anchor == Some(self.caret) {
                        self.anchor = None;
                    }
                    if self.anchor.is_none() {
                        self.anchor = self.query()[self.caret..]
                            .graphemes(true)
                            .next()
                            .map(|next| self.caret + next.len());
                    }
                    self.paste("");
                }
                _ if !modifiers.control_key()
                    && !modifiers.super_key()
                    && !modifiers.alt_key() =>
                {
                    if let Some(text) = text {
                        self.paste(text);
                    }
                }
                _ => {}
            }
            if selecting {
                if modifiers.shift_key() {
                    self.anchor.get_or_insert(old_caret);
                } else {
                    self.anchor = None;
                }
            }
            return;
        }
        if self.focus == Focus::List {
            let movement = match key {
                Key::Named(NamedKey::ArrowDown) => Some(FocusMove::Next),
                Key::Named(NamedKey::ArrowUp) => Some(FocusMove::Previous),
                Key::Named(NamedKey::PageDown) => Some(FocusMove::PageNext),
                Key::Named(NamedKey::PageUp) => Some(FocusMove::PagePrevious),
                Key::Named(NamedKey::Home) => Some(FocusMove::First),
                Key::Named(NamedKey::End) => Some(FocusMove::Last),
                _ => None,
            };
            if let Some(movement) = movement {
                if let Some(view) = &mut self.view {
                    view.move_focus(movement);
                }
                self.reveal_focus = true;
                return;
            }
            if !repeat
                && matches!(key, Key::Named(NamedKey::Enter))
                && self.focused_entry().is_some_and(|entry| {
                    matches!(
                        entry.kind,
                        SettingKind::Number { .. } | SettingKind::ContinuousNumber { .. }
                    )
                })
            {
                self.open_numeric();
                return;
            }
            if !modifiers.control_key()
                && !modifiers.super_key()
                && !modifiers.alt_key()
                && text.is_some_and(|value| !value.is_empty() && numeric_fragment(value))
                && self.focused_entry().is_some_and(|entry| {
                    matches!(
                        entry.kind,
                        SettingKind::Number { .. } | SettingKind::ContinuousNumber { .. }
                    )
                })
            {
                self.open_numeric();
                self.paste_numeric(text.unwrap_or_default());
                return;
            }
            if !repeat {
                match key {
                    Key::Named(NamedKey::ArrowLeft) => self.change_value(-1),
                    Key::Named(
                        NamedKey::ArrowRight | NamedKey::Enter | NamedKey::Space,
                    ) => self.change_value(1),
                    _ => {}
                }
            }
        } else if !repeat && matches!(key, Key::Named(NamedKey::Enter | NamedKey::Space))
        {
            match self.focus {
                Focus::Close => self.close(),
                Focus::Reset => self.request_reset(),
                Focus::Restore => self.request_restore_saved(),
                Focus::List if self.is_category_root() || self.is_package_list() => {
                    self.enter_category()
                }
                _ => {}
            }
        }
    }
    fn focused_entry(&self) -> Option<&SettingDescriptor> {
        self.catalog.as_ref()?.get(self.view.as_ref()?.focused()?)
    }
    fn queue(&mut self, change: Change) {
        if self.pending.is_some() || self.is_category_root() || self.is_package_list() {
            return;
        }
        let Some((catalog, id)) = self
            .catalog
            .as_ref()
            .zip(self.view.as_ref().and_then(ViewState::focused))
        else {
            return;
        };
        let edit = Edit {
            revision: catalog.revision(),
            id: id.clone(),
            change,
        };
        match catalog.validate_edit(&edit) {
            Ok(edit) => {
                self.pending = Some(edit);
                self.status.clear();
            }
            Err(_) => {
                self.status = self
                    .focused_entry()
                    .and_then(|entry| entry.availability.reason())
                    .unwrap_or("This setting cannot be changed.")
                    .into();
            }
        }
    }
    fn request_reset(&mut self) {
        if self.customizations.is_some() {
            self.request_customization_reset();
            return;
        }
        let Some((catalog, entry)) = self.catalog.as_ref().zip(self.focused_entry())
        else {
            return;
        };
        let edit = Edit {
            revision: catalog.revision(),
            id: entry.id.clone(),
            change: Change::Reset,
        };
        if catalog.validate_edit(&edit).is_ok() {
            self.ask_confirmation(
                ConfirmedSettingsAction::Setting(edit),
                format!("Reset {}?", entry.label),
                "Restore this setting to its configured default.",
                "Reset",
            );
        }
    }

    fn ask_confirmation(
        &mut self,
        action: ConfirmedSettingsAction,
        title: String,
        description: &'static str,
        accept_label: &'static str,
    ) {
        if self.confirmation.is_some()
            || self.pending.is_some()
            || self.pending_customization.is_some()
        {
            return;
        }
        let Some(catalog) = &self.catalog else {
            return;
        };
        self.confirmation = Some(SettingsConfirmation {
            action,
            revision: catalog.revision(),
            title,
            description,
            accept_label,
            accept_selected: false,
        });
        if let Some(editor) = &mut self.numeric_editor {
            editor.composing = false;
        }
        if let Some(editor) = &mut self.color_editor {
            editor.composing = false;
        }
        self.pressed = None;
        self.touch = None;
        self.preedit.clear();
        self.caret_rect = Rect::default();
        self.layout_dirty = true;
    }

    fn dismiss_confirmation(&mut self) {
        if self.confirmation.take().is_some() {
            self.confirmation_geometry = ConfirmationGeometry::default();
            self.pressed = None;
            self.touch = None;
            self.layout_dirty = true;
        }
    }

    fn accept_confirmation(&mut self) {
        if !self.confirmation_fits() {
            return;
        }
        let Some(confirmation) = self.confirmation.take() else {
            return;
        };
        self.confirmation_geometry = ConfirmationGeometry::default();
        self.pressed = None;
        self.touch = None;
        self.layout_dirty = true;
        if self
            .catalog
            .as_ref()
            .is_none_or(|catalog| catalog.revision() != confirmation.revision)
            || self.pending.is_some()
            || self.pending_customization.is_some()
        {
            return;
        }
        match confirmation.action {
            ConfirmedSettingsAction::Customization(CustomizationIntent::RestoreSaved)
                if !self.temporary_customizations =>
            {
                return
            }
            ConfirmedSettingsAction::Customization(intent) => {
                self.pending_customization = Some(intent)
            }
            ConfirmedSettingsAction::Setting(edit) => {
                if self
                    .catalog
                    .as_ref()
                    .is_none_or(|catalog| catalog.validate_edit(&edit).is_err())
                {
                    return;
                }
                self.pending = Some(edit);
            }
        }
        self.cancel_color();
        self.cancel_numeric();
        self.status.clear();
    }

    fn confirmation_key(&mut self, key: &Key, modifiers: ModifiersState, repeat: bool) {
        if repeat
            || modifiers.control_key()
            || modifiers.super_key()
            || modifiers.alt_key()
        {
            return;
        }
        if matches!(key, Key::Named(NamedKey::Escape))
            || matches!(key, Key::Character(value) if value.eq_ignore_ascii_case("n"))
        {
            self.dismiss_confirmation();
            return;
        }
        // A tiny resized window still permits cancellation, never blind approval.
        if !self.confirmation_fits() {
            return;
        }
        let Some(confirmation) = self.confirmation.as_mut() else {
            return;
        };
        match key {
            Key::Named(NamedKey::Tab | NamedKey::ArrowLeft | NamedKey::ArrowRight) => {
                confirmation.accept_selected = !confirmation.accept_selected;
            }
            Key::Named(NamedKey::Enter | NamedKey::Space) => {
                if confirmation.accept_selected {
                    self.accept_confirmation();
                } else {
                    self.dismiss_confirmation();
                }
            }
            Key::Character(value) if value.eq_ignore_ascii_case("y") => {
                self.accept_confirmation()
            }
            _ => {}
        }
    }

    fn confirmation_fits(&self) -> bool {
        self.width >= self.font.max(10.0) * 17.0 + 32.0
            && self.height >= self.font.max(10.0) * 1.45 * 8.0 + 48.0
    }

    fn paint_confirmation(&mut self, canvas: &mut impl Canvas, theme: UiTheme) {
        let Some(confirmation) = &self.confirmation else {
            return;
        };
        let viewport = Rect {
            x: 0.0,
            y: 0.0,
            width: self.width,
            height: self.height,
        };
        let font = self.font.max(10.0);
        let line = font * 1.45;
        let width = (self.width - 16.0).max(0.0).min(520.0_f32.max(font * 30.0));
        let title_lines = wrapped(
            &confirmation.title,
            (width - 48.0).max(1.0),
            canvas.text(),
            &DrawOpts {
                font_size: font,
                bold: true,
                ..DrawOpts::default()
            },
        );
        let description_lines = wrapped(
            confirmation.description,
            (width - 48.0).max(1.0),
            canvas.text(),
            &DrawOpts {
                font_size: font * 0.85,
                ..DrawOpts::default()
            },
        );
        let title_height = title_lines.len().min(2) as f32 * line;
        let description_height = description_lines.len().min(3) as f32 * line * 0.85;
        let button_height = (line + 8.0).max(32.0);
        let height = (40.0
            + title_height
            + 8.0
            + description_height
            + 20.0
            + button_height
            + 8.0
            + line * 0.75)
            .min((self.height - 16.0).max(0.0));
        let card = Rect {
            x: (self.width - width) * 0.5,
            y: (self.height - height) * 0.5,
            width,
            height,
        };
        self.confirmation_geometry = ConfirmationGeometry {
            card,
            ..Default::default()
        };
        let card = self.confirmation_geometry.card;
        // Render only this dialog on the Settings modal pass. Underlying labels
        // must not be drawn above an opaque confirmation surface.
        rect(canvas, viewport, theme.background, viewport);
        rounded_surface(canvas, card, theme.surface, viewport);
        self.layout_dirty = false;
        self.caret_rect = Rect::default();
        let body = Rect {
            x: card.x + 20.0,
            y: card.y + 20.0,
            width: (card.width - 40.0).max(0.0),
            height: line,
        };
        if !self.confirmation_fits() {
            label(
                canvas,
                body,
                "Enlarge window",
                font.min(14.0),
                theme.text,
                true,
                card,
            );
            shortcut_hint(
                canvas,
                Rect {
                    y: body.y + line,
                    ..body
                },
                "Esc: cancel",
                font.min(14.0),
                theme,
                card,
            );
            return;
        }
        for (index, text) in title_lines.iter().take(2).enumerate() {
            label(
                canvas,
                Rect {
                    y: body.y + index as f32 * line,
                    ..body
                },
                text,
                font,
                theme.text,
                true,
                card,
            );
        }
        for (index, text) in description_lines.iter().take(3).enumerate() {
            label(
                canvas,
                Rect {
                    y: body.y + title_height + 8.0 + index as f32 * line * 0.85,
                    ..body
                },
                text,
                font * 0.85,
                theme.muted_text,
                false,
                card,
            );
        }
        let button_width = ((body.width - 8.0) * 0.5).min(font * 10.0);
        let cancel = Rect {
            x: body.x + body.width - button_width * 2.0 - 8.0,
            y: body.y + title_height + 8.0 + description_height + 20.0,
            width: button_width,
            height: button_height,
        };
        let accept = Rect {
            x: cancel.x + cancel.width + 8.0,
            ..cancel
        };
        self.confirmation_geometry.cancel = cancel;
        self.confirmation_geometry.accept = accept;
        action_button(
            canvas,
            cancel,
            ("Cancel", "Esc"),
            font,
            (!confirmation.accept_selected, true),
            theme,
            card,
        );
        action_button(
            canvas,
            accept,
            (confirmation.accept_label, "Y"),
            font,
            (confirmation.accept_selected, true),
            theme,
            card,
        );
        shortcut_hint(
            canvas,
            Rect {
                y: cancel.y + cancel.height + 4.0,
                ..body
            },
            "Tab: choose | Enter: select",
            font * 0.75,
            theme,
            card,
        );
    }

    fn request_customization_reset(&mut self) {
        if self.pending.is_some() || self.pending_customization.is_some() {
            return;
        }
        let Some(navigation) = self.customizations.as_ref() else {
            return;
        };
        let scope = if let Some(slot_key) = navigation.active_slot.as_ref() {
            if let Some(slot_id) = slot_id_from_page(slot_key) {
                CustomizationResetScope::Tag(slot_id.to_owned())
            } else if let Some(severity) =
                slot_key.as_str().strip_prefix("output.severity.")
            {
                CustomizationResetScope::OutputSeverity(severity.to_owned())
            } else if let Some(kind) =
                slot_key.as_str().strip_prefix("command_output.band.")
            {
                CustomizationResetScope::CommandOutputBand(kind.to_owned())
            } else if let Some(severity) =
                slot_key.as_str().strip_prefix("kubernetes.severity.")
            {
                CustomizationResetScope::KubernetesSeverity(severity.to_owned())
            } else {
                return;
            }
        } else if let Some(package_key) = navigation.active_package.as_ref() {
            navigation.active_key.as_ref().map_or_else(
                || CustomizationResetScope::Package(package_key.clone()),
                |feature_key| {
                    CustomizationResetScope::PackageFeature(feature_key.clone())
                },
            )
        } else if let Some(key) = navigation.active_key.as_ref() {
            CustomizationResetScope::Group(key.clone())
        } else {
            CustomizationResetScope::All
        };
        let intent = CustomizationIntent::Reset {
            revision: navigation.full_catalog.revision(),
            scope,
        };
        self.ask_confirmation(
            ConfirmedSettingsAction::Customization(intent),
            if self.is_category_root() {
                "Reset all customizations?".into()
            } else {
                format!("Reset {}?", self.title())
            },
            "Preview defaults temporarily. Saved files stay unchanged.",
            "Reset",
        );
    }
    fn request_restore_saved(&mut self) {
        if self.temporary_customizations
            && self.pending.is_none()
            && self.pending_customization.is_none()
        {
            self.ask_confirmation(
                ConfirmedSettingsAction::Customization(CustomizationIntent::RestoreSaved),
                "Restore saved customizations?".into(),
                "Discard temporary changes and return to your saved choices.",
                "Restore",
            );
        }
    }
    fn change_value(&mut self, direction: i32) {
        if self.is_category_root() || self.is_package_list() {
            self.enter_category();
            return;
        }
        if self.customizations.as_ref().is_some_and(|navigation| {
            navigation.active_key.is_some() && navigation.active_slot.is_none()
        }) && self
            .focused_entry()
            .is_some_and(|entry| slot_id_from_page(&entry.id).is_some())
        {
            self.enter_category();
            return;
        }
        let Some(entry) = self.focused_entry() else {
            return;
        };
        if matches!(
            entry.kind,
            SettingKind::Color { .. } | SettingKind::Text { .. }
        ) {
            self.open_color();
            return;
        }
        let change = match (&entry.kind, &entry.value) {
            (SettingKind::Boolean, SettingValue::Boolean(value)) => {
                Change::Set(SettingValue::Boolean(!value))
            }
            (SettingKind::Choice { options }, SettingValue::Choice(value)) => {
                let Some(index) =
                    options.iter().position(|option| &option.value == value)
                else {
                    return;
                };
                let next = if direction < 0 {
                    index
                        .checked_sub(1)
                        .unwrap_or(options.len().saturating_sub(1))
                } else {
                    (index + 1) % options.len()
                };
                let Some(option) = options.get(next) else {
                    return;
                };
                Change::Set(SettingValue::Choice(option.value.clone()))
            }
            (
                SettingKind::ContinuousNumber { min, max, step },
                SettingValue::Number(value),
            ) => {
                let next = (value + step * f64::from(direction)).clamp(*min, *max);
                Change::Set(SettingValue::Number(next))
            }
            (SettingKind::Number { min, max, step }, SettingValue::Number(value)) => {
                let index = ((value - min) / step).round() + f64::from(direction);
                let next = (min + index * step).clamp(*min, *max);
                Change::Set(SettingValue::Number(next))
            }
            (SettingKind::Action, SettingValue::Action) => Change::Activate,
            _ => return,
        };
        self.queue(change);
    }
    fn open_numeric(&mut self) {
        if self.pending.is_some() || self.numeric_editor.is_some() {
            return;
        }
        let Some((catalog, entry)) = self.catalog.as_ref().zip(self.focused_entry())
        else {
            return;
        };
        let SettingValue::Number(value) = &entry.value else {
            return;
        };
        if !matches!(
            entry.kind,
            SettingKind::Number { .. } | SettingKind::ContinuousNumber { .. }
        ) || entry.availability.reason().is_some()
        {
            return;
        }
        let draft = number_label(entry, *value);
        self.numeric_editor = Some(NumericEditor {
            id: entry.id.clone(),
            revision: catalog.revision(),
            caret: draft.len(),
            anchor: Some(0),
            draft,
            composing: false,
        });
        self.focus = Focus::List;
        self.preedit.clear();
        self.pressed = None;
        self.layout_dirty = true;
    }
    fn cancel_numeric(&mut self) {
        if self.numeric_editor.take().is_some() {
            self.preedit.clear();
            self.pressed = None;
            self.layout_dirty = true;
            self.reveal_focus = true;
        }
    }
    fn paste_numeric(&mut self, text: &str) -> bool {
        let Some(editor) = &mut self.numeric_editor else {
            return false;
        };
        if editor.composing || !numeric_fragment(text) {
            return false;
        }
        let start = editor.anchor.unwrap_or(editor.caret).min(editor.caret);
        let end = editor.anchor.unwrap_or(editor.caret).max(editor.caret);
        if editor.draft.len() - (end - start) + text.len() > MAX_NUMERIC_BYTES {
            return false;
        }
        editor.draft.replace_range(start..end, text);
        editor.caret = start + text.len();
        editor.anchor = None;
        self.status.clear();
        self.layout_dirty = true;
        true
    }
    fn commit_numeric(&mut self) -> bool {
        let Some(editor) = &self.numeric_editor else {
            return false;
        };
        if editor.composing || self.pending.is_some() {
            return false;
        }
        let value = editor
            .draft
            .parse::<f64>()
            .ok()
            .filter(|value| value.is_finite());
        let edit = value.map(|value| Edit {
            revision: editor.revision,
            id: editor.id.clone(),
            change: Change::Set(SettingValue::Number(value)),
        });
        let Some(edit) = edit.filter(|edit| {
            self.catalog
                .as_ref()
                .is_some_and(|catalog| catalog.validate_edit(edit).is_ok())
        }) else {
            self.status = "Enter a number within the allowed range and step.".into();
            return false;
        };
        self.pending = Some(edit);
        self.cancel_numeric();
        self.status.clear();
        true
    }
    fn numeric_key(
        &mut self,
        key: &Key,
        text: Option<&str>,
        modifiers: ModifiersState,
        repeat: bool,
    ) {
        if matches!(key, Key::Named(NamedKey::Escape)) {
            if self
                .numeric_editor
                .as_ref()
                .is_some_and(|editor| editor.composing)
            {
                self.preedit.clear();
                if let Some(editor) = &mut self.numeric_editor {
                    editor.composing = false;
                }
            } else {
                self.cancel_numeric();
            }
            return;
        }
        if self.requires_larger_window()
            || self
                .numeric_editor
                .as_ref()
                .is_some_and(|editor| editor.composing)
        {
            return;
        }
        if matches!(key, Key::Named(NamedKey::Enter)) && !repeat {
            self.commit_numeric();
            return;
        }
        if matches!(key, Key::Named(NamedKey::Tab)) && !repeat {
            if self.commit_numeric() {
                self.key(key, None, modifiers, false);
            }
            return;
        }
        let command = modifiers.control_key() || modifiers.super_key();
        let Some(editor) = &mut self.numeric_editor else {
            return;
        };
        if command
            && matches!(key, Key::Character(value) if value.eq_ignore_ascii_case("a"))
        {
            editor.anchor = Some(0);
            editor.caret = editor.draft.len();
            return;
        }
        if command || modifiers.alt_key() {
            return;
        }
        let old = editor.caret;
        match key {
            Key::Named(NamedKey::ArrowLeft) => {
                editor.caret = editor.caret.saturating_sub(1);
            }
            Key::Named(NamedKey::ArrowRight) => {
                editor.caret = (editor.caret + 1).min(editor.draft.len());
            }
            Key::Named(NamedKey::Home) => editor.caret = 0,
            Key::Named(NamedKey::End) => editor.caret = editor.draft.len(),
            Key::Named(NamedKey::Backspace | NamedKey::Delete) => {
                if editor.anchor == Some(editor.caret) {
                    editor.anchor = None;
                }
                if editor.anchor.is_none() {
                    editor.anchor =
                        Some(if matches!(key, Key::Named(NamedKey::Backspace)) {
                            editor.caret.saturating_sub(1)
                        } else {
                            (editor.caret + 1).min(editor.draft.len())
                        });
                }
                self.paste_numeric("");
                return;
            }
            _ => {
                if let Some(text) = text {
                    self.paste_numeric(text);
                }
                return;
            }
        }
        if modifiers.shift_key() {
            editor.anchor.get_or_insert(old);
        } else {
            editor.anchor = None;
        }
        self.layout_dirty = true;
    }
    fn color_requires_larger_window(&self) -> bool {
        let font = self.font.max(10.0);
        self.width < font * 13.0 + 32.0 || self.height < font * 1.45 * 10.0 + 72.0
    }
    fn open_color(&mut self) {
        if self.pending.is_some() {
            return;
        }
        let Some(entry) = self.focused_entry() else {
            return;
        };
        let (alpha, text_limits) = match (&entry.kind, &entry.value) {
            (SettingKind::Color { alpha }, SettingValue::Color(_)) => (*alpha, None),
            (
                SettingKind::Text {
                    max_bytes,
                    allow_empty,
                },
                SettingValue::Text(_),
            ) => (false, Some((*max_bytes, *allow_empty))),
            _ => return,
        };
        if let Some(reason) = entry.availability.reason() {
            self.status = reason.into();
            return;
        }
        if self.color_requires_larger_window() {
            self.set_status("Enlarge the window to edit this color.");
            return;
        }
        let draft = match &entry.value {
            SettingValue::Text(value) => value.clone(),
            _ => display_value(entry),
        };
        self.color_editor = Some(ColorEditor {
            id: entry.id.clone(),
            revision: self.catalog.as_ref().map_or(0, Catalog::revision),
            alpha,
            text_limits,
            caret: draft.len(),
            anchor: Some(0),
            draft,
            focus: ColorFocus::Hex,
            composing: false,
            last_valid_color: match &entry.value {
                SettingValue::Color(color) => Some(*color),
                _ => None,
            },
            feedback: None,
        });
        self.preedit.clear();
        self.pressed = None;
        self.touch = None;
        self.layout_dirty = true;
    }
    fn cancel_color(&mut self) {
        if self.color_editor.take().is_none() {
            return;
        }
        self.preedit.clear();
        self.pressed = None;
        self.touch = None;
        self.caret_rect = Rect::default();
        self.focus = Focus::List;
        self.layout_dirty = true;
        self.reveal_focus = true;
    }
    fn paste_color(&mut self, text: &str) -> bool {
        if self.color_requires_larger_window() {
            return false;
        }
        let Some(editor) = &mut self.color_editor else {
            return false;
        };
        let max = editor.text_limits.map_or(MAX_COLOR_BYTES, |(max, _)| max);
        if text.len() > max
            || if editor.text_limits.is_some() {
                !safe_editor_text(text, max, true)
            } else {
                !text.bytes().all(|byte| byte.is_ascii_graphic())
            }
        {
            return false;
        }
        if editor.focus != ColorFocus::Hex || editor.composing {
            return false;
        }
        let start = editor.anchor.unwrap_or(editor.caret).min(editor.caret);
        let end = editor.anchor.unwrap_or(editor.caret).max(editor.caret);
        if editor.draft.len() - (end - start) + text.len() > max {
            return false;
        }
        // Keyboard and mouse paths maintain UTF-8 boundaries for this draft.
        if !editor.draft.is_char_boundary(start) || !editor.draft.is_char_boundary(end) {
            return false;
        }
        editor.draft.replace_range(start..end, text);
        if editor.text_limits.is_none() {
            if let Some(color) = parse_color(&editor.draft, editor.alpha) {
                editor.last_valid_color = Some(color);
            }
        }
        editor.caret = start + text.len();
        editor.anchor = None;
        editor.feedback = None;
        self.pressed = None;
        self.layout_dirty = true;
        true
    }
    fn activate_color(&mut self, focus: ColorFocus) {
        if focus == ColorFocus::Cancel {
            self.cancel_color();
            return;
        }
        let Some(editor) = &self.color_editor else {
            return;
        };
        let change = match focus {
            ColorFocus::Apply => {
                if editor.composing {
                    return;
                }
                let Some(value) = editor_value(editor) else {
                    return;
                };
                Change::Set(value)
            }
            ColorFocus::Reset => Change::Reset,
            _ => return,
        };
        let edit = Edit {
            revision: editor.revision,
            id: editor.id.clone(),
            change,
        };
        if self.pending.is_none()
            && self
                .catalog
                .as_ref()
                .is_some_and(|catalog| catalog.validate_edit(&edit).is_ok())
        {
            if focus == ColorFocus::Reset {
                let title = self
                    .catalog
                    .as_ref()
                    .and_then(|catalog| catalog.get(&edit.id))
                    .map_or_else(
                        || "Reset this value?".into(),
                        |entry| format!("Reset {}?", entry.label),
                    );
                self.ask_confirmation(
                    ConfirmedSettingsAction::Setting(edit),
                    title,
                    "Restore this value to its configured default.",
                    "Reset",
                );
                return;
            }
            self.pending = Some(edit);
            self.status.clear();
            self.cancel_color();
        }
    }
    fn color_key(
        &mut self,
        key: &Key,
        text: Option<&str>,
        modifiers: ModifiersState,
        repeat: bool,
    ) {
        if matches!(key, Key::Named(NamedKey::Escape)) {
            if self
                .color_editor
                .as_ref()
                .is_some_and(|editor| editor.composing)
            {
                self.preedit.clear();
                if let Some(editor) = &mut self.color_editor {
                    editor.composing = false;
                }
            } else {
                self.cancel_color();
            }
            return;
        }
        if self
            .color_editor
            .as_ref()
            .is_some_and(|editor| editor.composing)
            || self.color_requires_larger_window()
        {
            return;
        }
        let Some(editor) = &mut self.color_editor else {
            return;
        };
        let command = modifiers.control_key() || modifiers.super_key();
        if command
            && matches!(key, Key::Character(value) if value.eq_ignore_ascii_case("a"))
            && editor.focus == ColorFocus::Hex
        {
            editor.anchor = Some(0);
            editor.caret = editor.draft.len();
            return;
        }
        let cycle = match key {
            Key::Named(NamedKey::Tab) => Some(if modifiers.shift_key() { -1 } else { 1 }),
            Key::Named(NamedKey::ArrowLeft | NamedKey::ArrowUp)
                if editor.focus != ColorFocus::Hex =>
            {
                Some(-1)
            }
            Key::Named(NamedKey::ArrowRight | NamedKey::ArrowDown)
                if editor.focus != ColorFocus::Hex =>
            {
                Some(1)
            }
            _ => None,
        };
        if let Some(direction) = cycle {
            let index = match editor.focus {
                ColorFocus::Hex => 0,
                ColorFocus::Apply => 1,
                ColorFocus::Cancel => 2,
                ColorFocus::Reset => 3,
            };
            editor.focus = [
                ColorFocus::Hex,
                ColorFocus::Apply,
                ColorFocus::Cancel,
                ColorFocus::Reset,
            ][((index + direction + 4) % 4) as usize];
            self.preedit.clear();
            self.pressed = None;
            return;
        }
        if command || modifiers.alt_key() {
            return;
        }
        if !repeat && matches!(key, Key::Named(NamedKey::Enter)) {
            let focus = if editor.focus == ColorFocus::Hex {
                ColorFocus::Apply
            } else {
                editor.focus
            };
            self.activate_color(focus);
            return;
        }
        if editor.focus != ColorFocus::Hex {
            if !repeat
                && matches!(key, Key::Character(value) if value.eq_ignore_ascii_case("r"))
            {
                self.activate_color(ColorFocus::Reset);
                return;
            }
            if !repeat
                && matches!(key, Key::Character(value) if value.eq_ignore_ascii_case("a"))
            {
                self.activate_color(ColorFocus::Apply);
                return;
            }
            if !repeat && matches!(key, Key::Named(NamedKey::Space)) {
                let focus = editor.focus;
                self.activate_color(focus);
            }
            return;
        }
        let old = editor.caret;
        match key {
            Key::Named(NamedKey::ArrowLeft) => {
                editor.caret = editor.draft[..editor.caret]
                    .grapheme_indices(true)
                    .next_back()
                    .map_or(0, |(index, _)| index);
            }
            Key::Named(NamedKey::ArrowRight) => {
                editor.caret = editor.draft[editor.caret..]
                    .graphemes(true)
                    .next()
                    .map_or(editor.caret, |next| editor.caret + next.len());
            }
            Key::Named(NamedKey::Home) => editor.caret = 0,
            Key::Named(NamedKey::End) => editor.caret = editor.draft.len(),
            Key::Named(NamedKey::Backspace | NamedKey::Delete) => {
                if editor.anchor == Some(editor.caret) {
                    editor.anchor = None;
                }
                if editor.anchor.is_none() {
                    editor.anchor =
                        Some(if matches!(key, Key::Named(NamedKey::Backspace)) {
                            editor.draft[..editor.caret]
                                .grapheme_indices(true)
                                .next_back()
                                .map_or(0, |(index, _)| index)
                        } else {
                            editor.draft[editor.caret..]
                                .graphemes(true)
                                .next()
                                .map_or(editor.caret, |next| editor.caret + next.len())
                        });
                }
                self.paste_color("");
                return;
            }
            _ => {
                if let Some(text) = text {
                    self.paste_color(text);
                }
                return;
            }
        }
        if modifiers.shift_key() {
            editor.anchor.get_or_insert(old);
        } else {
            editor.anchor = None;
        }
        self.pressed = None;
    }
    fn prepare_color(&mut self, viewport: Rect) {
        let line = self.font.max(10.0) * 1.45;
        let width = (self.font.max(10.0) * 13.0 + 16.0)
            .max(520.0)
            .min(self.width - 32.0);
        let height = line * 10.0 + 40.0;
        let card = Rect {
            x: (self.width - width) * 0.5,
            y: (self.height - height) * 0.5,
            width,
            height,
        };
        let area = Rect {
            x: card.x + 8.0,
            y: card.y + 8.0,
            width: width - 16.0,
            height: line,
        };
        let split = width >= 480.0;
        let form = if split {
            Rect {
                width: (area.width - 12.0) * 0.5,
                ..area
            }
        } else {
            area
        };
        let input = Rect {
            y: form.y + line * 3.0 + 8.0,
            height: line + 8.0,
            ..form
        };
        let preview = if split {
            Rect {
                x: form.x + form.width + 12.0,
                y: area.y + line + 4.0,
                width: form.width,
                height: line * 6.0 + 12.0,
            }
        } else {
            Rect {
                y: input.y + input.height + 8.0,
                height: line + 8.0,
                ..area
            }
        };
        let button_width = (area.width - 16.0) / 3.0;
        let apply = Rect {
            y: card.y + height - line - 16.0,
            height: line + 8.0,
            width: button_width,
            ..area
        };
        self.color_geometry = ColorGeometry {
            card,
            title: area,
            name: Rect {
                y: form.y + line + 4.0,
                height: line * 2.0,
                ..form
            },
            input,
            preview,
            help: Rect {
                y: if split {
                    input.y + input.height + 8.0
                } else {
                    preview.y + preview.height + 8.0
                },
                height: line * 2.0,
                ..form
            },
            apply,
            cancel: Rect {
                x: apply.x + button_width + 8.0,
                ..apply
            },
            reset: Rect {
                x: apply.x + (button_width + 8.0) * 2.0,
                ..apply
            },
        };
        self.geometry.viewport = viewport;
        self.rows.clear();
        self.caret_rect = Rect::default();
    }
    fn paint_color(&mut self, canvas: &mut impl Canvas, theme: UiTheme) {
        let Some(editor) = &self.color_editor else {
            return;
        };
        let Some(entry) = self
            .catalog
            .as_ref()
            .and_then(|catalog| catalog.get(&editor.id))
        else {
            return;
        };
        let g = self.color_geometry;
        let viewport = self.geometry.viewport;
        let font = self.font.max(10.0);
        let line = font * 1.45;
        rect(canvas, viewport, theme.background, viewport);
        rounded_surface(canvas, g.card, theme.surface, viewport);
        label(
            canvas,
            g.title,
            if editor.text_limits.is_some() {
                "Edit text"
            } else {
                "Edit color"
            },
            font,
            theme.text,
            true,
            g.card,
        );
        let opts = DrawOpts {
            font_size: font,
            color: color_u8(theme.text),
            ..DrawOpts::default()
        };
        let mut names = wrapped(&entry.label, g.name.width - 8.0, canvas.text(), &opts);
        if names.len() > 2 {
            names.truncate(2);
            if let Some(last) = names.last_mut() {
                while !last.is_empty()
                    && canvas.text().measure(&format!("{last}…"), &opts)
                        > g.name.width - 8.0
                {
                    let end = last
                        .grapheme_indices(true)
                        .next_back()
                        .map_or(0, |(start, _)| start);
                    last.truncate(end);
                }
                last.push('…');
            }
        }
        for (index, name) in names.iter().enumerate() {
            label(
                canvas,
                Rect {
                    y: g.name.y + index as f32 * line,
                    height: line,
                    ..g.name
                },
                name,
                font,
                theme.text,
                false,
                g.name,
            );
        }
        control(
            canvas,
            g.input,
            editor.focus == ColorFocus::Hex,
            theme,
            g.card,
        );
        let display = format!(
            "{}{}{}",
            &editor.draft[..editor.caret],
            self.preedit,
            &editor.draft[editor.caret..]
        );
        let caret_width = canvas.text().measure(&editor.draft[..editor.caret], &opts);
        let shift = (caret_width - (g.input.width - 16.0)).max(0.0);
        self.caret_rect = Rect {
            x: g.input.x + 8.0 + caret_width - shift,
            y: g.input.y + 3.0,
            width: 1.0,
            height: g.input.height - 6.0,
        };
        if let Some(anchor) = editor.anchor.filter(|anchor| *anchor != editor.caret) {
            let left = canvas
                .text()
                .measure(&editor.draft[..anchor.min(editor.caret)], &opts);
            let right = canvas
                .text()
                .measure(&editor.draft[..anchor.max(editor.caret)], &opts);
            rect(
                canvas,
                Rect {
                    x: g.input.x + 8.0 + left - shift,
                    y: g.input.y + 3.0,
                    width: right - left,
                    height: g.input.height - 6.0,
                },
                theme.raised,
                g.input,
            );
        }
        canvas.text().draw_clipped(
            g.input.x + 8.0 - shift,
            g.input.y + 2.0,
            &display,
            &opts,
            g.input.array(),
        );
        if editor.focus == ColorFocus::Hex && self.preedit.is_empty() {
            rect(canvas, self.caret_rect, theme.outline, g.input);
        }
        let draft = editor_value(editor);
        if editor.text_limits.is_some() {
            label(
                canvas,
                g.preview,
                "Display text only · never evaluated",
                font * 0.85,
                theme.muted_text,
                false,
                g.preview,
            );
        } else {
            let color = match &entry.value {
                SettingValue::Color(color) => *color,
                _ => return,
            };
            let draft_color = draft
                .as_ref()
                .and_then(|value| match value {
                    SettingValue::Color(color) => Some(*color),
                    _ => None,
                })
                .or(editor.last_valid_color);
            for (index, (value, caption)) in [
                (Some(color), "Current"),
                (
                    draft_color,
                    if draft.is_some() {
                        "Draft"
                    } else {
                        "Last valid draft"
                    },
                ),
            ]
            .into_iter()
            .enumerate()
            {
                let split = g.preview.x > g.input.x + g.input.width;
                let section = if split {
                    Rect {
                        y: g.preview.y + index as f32 * g.preview.height * 0.5,
                        height: g.preview.height * 0.5,
                        ..g.preview
                    }
                } else {
                    Rect {
                        x: g.preview.x + index as f32 * g.preview.width * 0.5,
                        width: g.preview.width * 0.5,
                        ..g.preview
                    }
                };
                if let Some(value) = value {
                    self.paint_color_graphic(
                        canvas,
                        section,
                        editor.id.as_str(),
                        value,
                        theme,
                    );
                }
                label(
                    canvas,
                    Rect {
                        x: section.x + 4.0,
                        width: (section.width - 8.0).max(0.0),
                        ..section
                    },
                    caption,
                    (font * 0.58).max(8.0),
                    theme.muted_text,
                    false,
                    g.preview,
                );
            }
        }
        let help = if let Some(feedback) = editor.feedback.as_deref() {
            feedback
        } else if let Some((max, allow_empty)) = editor.text_limits {
            if draft.is_none() {
                "Text is too long or contains a control character. Apply unavailable."
            } else if allow_empty {
                if max <= 24 {
                    "Optional prefix or suffix. Up to 24 UTF-8 bytes."
                } else {
                    "Literal label. Up to 128 UTF-8 bytes."
                }
            } else {
                "Enter a visible label. Up to 128 UTF-8 bytes."
            }
        } else if draft.is_none() {
            if editor.alpha {
                "Use #RRGGBB or #RRGGBBAA. Apply unavailable."
            } else {
                "Use #RRGGBB. Apply unavailable."
            }
        } else if editor.alpha {
            "#RRGGBBAA includes opacity (00–FF)."
        } else {
            "#RRGGBB uses an opaque color."
        };
        for (index, value) in wrapped(help, g.help.width - 8.0, canvas.text(), &opts)
            .iter()
            .take(2)
            .enumerate()
        {
            label(
                canvas,
                Rect {
                    y: g.help.y + index as f32 * line,
                    height: line,
                    ..g.help
                },
                value,
                font * 0.85,
                theme.muted_text,
                false,
                g.help,
            );
        }
        for (bounds, focus, caption, key) in [
            (g.apply, ColorFocus::Apply, "Apply", "A"),
            (g.cancel, ColorFocus::Cancel, "Cancel", "Esc"),
            (g.reset, ColorFocus::Reset, "Reset", "R"),
        ] {
            action_button(
                canvas,
                bounds,
                (caption, key),
                font,
                (
                    editor.focus == focus,
                    focus != ColorFocus::Apply || draft.is_some(),
                ),
                theme,
                g.card,
            );
        }
    }
    fn paint_color_graphic(
        &self,
        canvas: &mut impl Canvas,
        bounds: Rect,
        id: &str,
        color: [u8; 4],
        theme: UiTheme,
    ) {
        let (terminal_background, terminal_foreground) = self
            .customizations
            .as_ref()
            .and_then(|navigation| navigation.slot_pages.as_ref())
            .map_or(
                (theme.background, theme.text),
                SlotPageSnapshot::preview_terminal_colors,
            );
        let graphic = Rect {
            x: bounds.x + 4.0,
            y: bounds.y + 11.0,
            width: (bounds.width - 8.0).max(0.0),
            height: (bounds.height - 13.0).clamp(0.0, 24.0),
        };
        if graphic.width <= 0.0 || graphic.height <= 0.0 {
            return;
        }
        rect(canvas, graphic, terminal_background, bounds);
        if id.starts_with("tags.colors.") || id.starts_with("tags.slot.") {
            let name = tag_color_graphic_name(id);
            let style = self
                .customizations
                .as_ref()
                .and_then(|navigation| navigation.slot_pages.as_ref())
                .map_or(BarVisualStyle::Capsule, |snapshot| {
                    snapshot.preview_recipe().visual
                });
            let opacity = self
                .customizations
                .as_ref()
                .and_then(|navigation| navigation.slot_pages.as_ref())
                .map_or(75, |snapshot| {
                    let appearance = snapshot.preview_appearance();
                    if appearance.style
                        == rio_backend::config::presentation::TagStyle::Plain
                        || style == BarVisualStyle::Underline
                    {
                        0
                    } else {
                        appearance.opacity.get()
                    }
                });
            let tag = automexia_ui_model::context_tag_colors(
                terminal_background,
                [color[0], color[1], color[2]],
                opacity,
            );
            paint_sample_tag_surface(canvas, graphic, style, tag);
            label(
                canvas,
                graphic,
                &format!("{name} tag"),
                (self.font * 0.64).max(9.0),
                tag.foreground,
                false,
                graphic,
            );
        } else if id.starts_with("output.backgrounds.") {
            rect(
                canvas,
                graphic,
                color.map(|channel| f32::from(channel) / 255.0),
                graphic,
            );
            let severity = id.strip_prefix("output.backgrounds.").unwrap_or("status");
            label(
                canvas,
                graphic,
                &format!("{severity}: sample"),
                (self.font * 0.64).max(9.0),
                terminal_foreground,
                false,
                graphic,
            );
        } else if id.starts_with("output.colors.") {
            let severity = id.strip_prefix("output.colors.").unwrap_or("status");
            label(
                canvas,
                graphic,
                &format!("{severity}: sample"),
                (self.font * 0.64).max(9.0),
                color.map(|channel| f32::from(channel) / 255.0),
                false,
                graphic,
            );
        } else {
            color_swatch(canvas, graphic, color, theme, graphic);
        }
    }
    fn target_at(&self, x: f32, y: f32) -> Option<Target> {
        if self.confirmation.is_some() {
            if self.layout_dirty {
                return None;
            }
            let g = self.confirmation_geometry;
            return [(g.cancel, false), (g.accept, true)]
                .into_iter()
                .find(|(bounds, _)| bounds.contains(x, y))
                .map(|(_, accept)| Target::Confirmation(accept));
        }
        if let Some(editor) = &self.color_editor {
            if self.layout_dirty {
                return None;
            }
            let g = self.color_geometry;
            return [
                (g.input, ColorFocus::Hex),
                (g.apply, ColorFocus::Apply),
                (g.cancel, ColorFocus::Cancel),
                (g.reset, ColorFocus::Reset),
            ]
            .into_iter()
            .find(|(bounds, focus)| {
                bounds.contains(x, y)
                    && (*focus != ColorFocus::Apply
                        || (!editor.composing && editor_value(editor).is_some()))
            })
            .map(|(_, focus)| Target::Color(focus));
        }
        if self.layout_dirty || !self.geometry.card.contains(x, y) {
            return None;
        }
        if self.geometry.close.contains(x, y) {
            return Some(
                if self.requires_larger_window() && self.is_category_detail() {
                    Target::Back
                } else {
                    Target::Close
                },
            );
        }
        if self.is_category_detail() && self.geometry.back.contains(x, y) {
            return Some(Target::Back);
        }
        if self.geometry.reset.contains(x, y) {
            return Some(Target::Reset);
        }
        if self.temporary_customizations && self.geometry.restore.contains(x, y) {
            return Some(Target::Restore);
        }
        if self.geometry.search.contains(x, y) {
            return Some(Target::Search);
        }
        if self.preview_button.contains(x, y) {
            return Some(Target::PreviewButton);
        }
        if self.geometry.preview.contains(x, y) {
            if !self.preview_edit_mode && !self.is_tag_preview() {
                return None;
            }
            return self
                .preview_targets
                .iter()
                .find(|(_, bounds)| {
                    bounds.contains(x, y)
                        && self
                            .preview_tag_shapes
                            .iter()
                            .find(|(area, _)| area == bounds)
                            .is_none_or(|(_, shape)| {
                                shape.contains((x - bounds.x, y - bounds.y))
                            })
                })
                .map(|(id, _)| Target::PreviewItem(id.clone()));
        }
        if self.geometry.body.contains(x, y) {
            return self
                .rows
                .iter()
                .find(|row| {
                    row.control.contains(x, y)
                        || ((self.is_category_root()
                            || slot_id_from_page(&row.id).is_some())
                            && row.bounds.contains(x, y))
                })
                .map(|row| {
                    if self
                        .catalog
                        .as_ref()
                        .and_then(|catalog| catalog.get(&row.id))
                        .is_some_and(|entry| {
                            matches!(
                                entry.kind,
                                SettingKind::Number { .. }
                                    | SettingKind::ContinuousNumber { .. }
                            )
                        })
                    {
                        let (decrement, input, _) = numeric_zones(row.control, self.font);
                        if input.contains(x, y) {
                            return Target::NumericInput(row.id.clone());
                        }
                        return Target::Control(
                            row.id.clone(),
                            if decrement.contains(x, y) { -1 } else { 1 },
                        );
                    }
                    let directional = self
                        .catalog
                        .as_ref()
                        .and_then(|catalog| catalog.get(&row.id))
                        .is_some_and(|entry| {
                            matches!(
                                entry.kind,
                                SettingKind::Choice { .. }
                                    | SettingKind::Number { .. }
                                    | SettingKind::ContinuousNumber { .. }
                            )
                        });
                    let direction =
                        if directional && x < row.control.x + row.control.width * 0.5 {
                            -1
                        } else {
                            1
                        };
                    Target::Control(row.id.clone(), direction)
                });
        }
        None
    }
    fn activate_target(&mut self, target: Target) {
        if self.confirmation.is_some() {
            match target {
                Target::Confirmation(true) => self.accept_confirmation(),
                Target::Confirmation(false) => self.dismiss_confirmation(),
                _ => {}
            }
            return;
        }
        // Clicking another control explicitly transfers focus out of preview
        // selection. A stale keyboard-mode flag must not alter its Tab order.
        if self.preview_edit_mode
            && !matches!(target, Target::PreviewButton | Target::PreviewItem(_))
        {
            self.preview_edit_mode = false;
            self.layout_dirty = true;
        }
        if !matches!(target, Target::Reset | Target::Restore)
            && self.numeric_editor.as_ref().is_some_and(
                |editor| !matches!(&target, Target::NumericInput(id) if id == &editor.id),
            )
        {
            self.cancel_numeric();
        }
        match target {
            Target::Confirmation(_) => {}
            Target::Color(focus) => {
                if let Some(editor) = &mut self.color_editor {
                    editor.composing = false;
                    editor.focus = focus;
                    if focus == ColorFocus::Hex {
                        editor.caret = editor.draft.len();
                        editor.anchor = Some(0);
                    }
                    self.preedit.clear();
                    self.activate_color(focus);
                }
            }
            Target::Close => self.close(),
            Target::Back => self.back_to_categories(),
            Target::Search => {
                self.focus = Focus::Search;
                self.caret = self.query().len();
                self.anchor = None;
            }
            Target::Reset => {
                if self.numeric_editor.is_none() {
                    self.focus = Focus::Reset;
                }
                self.request_reset();
            }
            Target::Restore => {
                if self.numeric_editor.is_none() {
                    self.focus = Focus::Restore;
                }
                self.request_restore_saved();
            }
            Target::Control(id, direction) => {
                if self.view.as_mut().is_some_and(|view| view.focus(&id)) {
                    self.focus = Focus::List;
                    self.change_value(direction);
                }
            }
            Target::NumericInput(id) => {
                if self.view.as_mut().is_some_and(|view| view.focus(&id)) {
                    self.open_numeric();
                }
            }
            Target::PreviewButton => self.activate_preview_button(),
            Target::PreviewItem(id) => {
                self.preview_selected = Some(id.clone());
                self.focus = Focus::Preview;
                self.enter_preview_item(id);
            }
        }
    }
    fn scroll_preview(&mut self, delta: f32, point: Option<(f32, f32)>) {
        if self.confirmation.is_some() {
            return;
        }
        if !delta.is_finite() || delta == 0.0 {
            return;
        }
        let steps = (delta.abs() / (self.font.max(10.0) * 1.55 + 4.0))
            .ceil()
            .max(1.0) as usize;
        if self.is_tag_preview()
            && point.is_some_and(|(x, y)| self.preview_tag_list_area.contains(x, y))
        {
            let hidden_count = self.preview_order.len();
            let max =
                hidden_count.saturating_sub(self.preview_tag_list_visible_rows.max(1));
            self.preview_tag_list_scroll = if delta >= 0.0 {
                self.preview_tag_list_scroll.saturating_add(steps).min(max)
            } else {
                self.preview_tag_list_scroll.saturating_sub(steps)
            };
        } else {
            let max = self
                .preview_total_rows
                .saturating_sub(self.preview_page_rows());
            self.preview_scroll = if delta >= 0.0 {
                self.preview_scroll.saturating_add(steps).min(max)
            } else {
                self.preview_scroll.saturating_sub(steps)
            };
        }
        self.pressed = None;
        self.layout_dirty = true;
    }
    fn scroll_by(&mut self, delta: f32) {
        if self.confirmation.is_some() {
            return;
        }
        if self.color_editor.is_some() || !delta.is_finite() || delta == 0.0 {
            return;
        }
        self.scroll = (self.scroll + delta).clamp(
            0.0,
            (self.content_height - self.geometry.body.height).max(0.0),
        );
        self.layout_dirty = true;
        self.reveal_focus = false;
        self.pressed = None;
    }
    fn prepare(&mut self, text: &mut Text) {
        if !self.layout_dirty && !self.reveal_focus {
            return;
        }
        let viewport = Rect {
            x: 0.0,
            y: 0.0,
            width: self.width,
            height: self.height,
        };
        if self.color_editor.is_some() {
            self.prepare_color(viewport);
            self.layout_dirty = false;
            self.reveal_focus = false;
            return;
        }
        let margin: f32 = if self.width < 360.0 || self.height < 300.0 {
            4.0
        } else {
            16.0
        };
        let card = Rect {
            x: margin.min(self.width * 0.5),
            y: margin.min(self.height * 0.5),
            width: (self.width - 2.0 * margin).clamp(
                0.0,
                if self.is_category_root() {
                    760.0
                } else {
                    960.0
                },
            ),
            height: (self.height - 2.0 * margin).max(0.0),
        };
        let card = Rect {
            x: (self.width - card.width) * 0.5,
            ..card
        };
        let pad = 12.0_f32.min(card.width * 0.1);
        let line = self.font.max(10.0) * 1.45;
        if self.requires_larger_window() {
            let opts = DrawOpts {
                font_size: self.font.max(10.0),
                ..DrawOpts::default()
            };
            let width = (card.width - 2.0 * pad).max(0.0);
            self.compact_lines = if text.measure("Enlarge window", &opts) <= width {
                let mut lines = vec!["Enlarge window".into()];
                lines.extend(wrapped("to use these controls.", width, text, &opts));
                lines
            } else {
                wrapped("More room", width, text, &opts)
            };
            let mut content_height = self.compact_lines.len() as f32 * line;
            let control_height = line + 4.0;
            let close_label = if self.is_category_detail() {
                "Back"
            } else {
                "Close"
            };
            let show_close = content_height + control_height + pad * 3.0 <= card.height
                && text.measure(close_label, &opts) + 8.0 <= width;
            let close = if show_close {
                Rect {
                    x: card.x + pad,
                    y: card.y + card.height - pad - control_height,
                    width,
                    height: control_height,
                }
            } else {
                Rect::default()
            };
            if !show_close && content_height + line <= card.height {
                self.compact_lines.push("Esc".into());
                content_height += line;
            }
            let available = if show_close {
                (close.y - card.y - pad).max(0.0)
            } else {
                card.height
            };
            let body = Rect {
                x: card.x + pad,
                y: card.y + ((available - content_height) * 0.5).max(0.0),
                width,
                height: content_height.min(available),
            };
            self.geometry = Geometry {
                viewport,
                card,
                body,
                close,
                ..Geometry::default()
            };
            self.rows.clear();
            self.scroll = 0.0;
            self.content_height = 0.0;
            self.caret_rect = Rect::default();
            self.layout_dirty = false;
            self.reveal_focus = false;
            return;
        }
        self.compact_lines.clear();
        let header = line * 2.0 + pad * 3.0;
        let footer = line * 2.0 + pad * 3.0;
        let search = Rect {
            x: card.x + pad,
            y: card.y + header * 0.5,
            width: (card.width - 2.0 * pad).max(0.0),
            height: (header * 0.5 - pad).max(0.0),
        };
        let back = if self.is_category_detail() {
            Rect {
                x: card.x + pad,
                y: card.y + pad,
                width: (self.font.max(10.0) * 7.0).min((card.width - 2.0 * pad) * 0.4),
                height: line + 2.0,
            }
        } else {
            Rect::default()
        };
        let content = Rect {
            x: card.x + pad,
            y: card.y + header,
            width: (card.width - 2.0 * pad).max(0.0),
            height: (card.height - header - footer).max(0.0),
        };
        let show_preview = self.is_category_detail();
        let split = show_preview && content.width >= (self.font * 23.0).max(620.0);
        let (body, preview) = if split {
            let gutter = pad * 1.5;
            let left = (content.width - gutter) * 0.5;
            (
                Rect {
                    width: left,
                    ..content
                },
                Rect {
                    x: content.x + left + gutter,
                    width: (content.width - left - gutter).max(0.0),
                    ..content
                },
            )
        } else if show_preview {
            let gutter = pad;
            let editing_preview = (self.preview_edit_mode
                && self.preview_selector_available())
                || self.is_tag_preview();
            let tag_preview = self.is_tag_preview();
            let table_preview = self
                .customizations
                .as_ref()
                .and_then(|navigation| navigation.active_key.as_ref())
                .is_some_and(|key| {
                    key.as_str() == automexia_ui_model::settings::INLINE_TABLES
                });
            let timestamp_preview = self
                .customizations
                .as_ref()
                .and_then(|navigation| navigation.active_key.as_ref())
                .is_some_and(|key| {
                    key.as_str() == automexia_ui_model::settings::COMMAND_TIMESTAMPS
                });
            let preview_height = (content.height
                * if tag_preview {
                    0.68
                } else if editing_preview {
                    0.5
                } else {
                    0.32
                })
            .clamp(
                self.font
                    * if tag_preview {
                        10.5
                    } else if editing_preview {
                        9.5
                    } else if timestamp_preview {
                        // Keep date, tags, time and a command visible when
                        // timestamp components occupy three separate rows.
                        8.5
                    } else if table_preview {
                        // Keep a header and both stripe colors visible in the
                        // stacked layout, without reducing the sample text size.
                        7.0
                    } else {
                        5.5
                    },
                line * if tag_preview {
                    10.0
                } else if editing_preview {
                    8.0
                } else {
                    7.0
                },
            )
            .min((content.height - line * 2.0 - gutter).max(0.0));
            let list_height = (content.height - preview_height - gutter).max(0.0);
            (
                Rect {
                    height: list_height,
                    ..content
                },
                Rect {
                    y: content.y + list_height + gutter,
                    height: preview_height,
                    ..content
                },
            )
        } else {
            (content, Rect::default())
        };
        let button_h = (line + 8.0).max(32.0);
        let customization_buttons = self.customizations.is_some();
        let button_count = if customization_buttons { 3.0 } else { 2.0 };
        let button_w = ((content.width - pad * (button_count - 1.0)) / button_count)
            .clamp(0.0, self.font.max(10.0) * 12.0 + 16.0);
        let button_y = card.y + card.height - footer + pad.min(footer * 0.1);
        let reset = Rect {
            x: content.x,
            y: button_y,
            width: button_w,
            height: button_h,
        };
        let restore = if customization_buttons {
            Rect {
                x: content.x + button_w + pad,
                y: button_y,
                width: button_w,
                height: button_h,
            }
        } else {
            Rect::default()
        };
        let close = Rect {
            x: content.x + content.width - button_w,
            y: button_y,
            width: button_w,
            height: button_h,
        };
        let status = Rect {
            x: content.x,
            y: button_y + button_h + pad.min(footer * 0.1),
            width: content.width,
            height: (card.y + card.height
                - pad
                - button_y
                - button_h
                - pad.min(footer * 0.1))
            .max(0.0),
        };
        self.geometry = Geometry {
            viewport,
            card,
            search,
            back,
            body,
            preview,
            reset,
            restore,
            close,
            status,
        };
        let opts = DrawOpts {
            font_size: self.font.max(10.0),
            ..DrawOpts::default()
        };
        let help_font = (self.font.max(10.0) * 0.76).max(9.0);
        let help_line = help_font * 1.35;
        let help_opts = DrawOpts {
            font_size: help_font,
            ..DrawOpts::default()
        };
        let mut rows = Vec::new();
        let mut top = 0.0;
        if let Some((view, catalog)) = self.view.as_ref().zip(self.catalog.as_ref()) {
            for id in view.filtered_ids() {
                let Some(entry) = catalog.get(id) else {
                    continue;
                };
                let width = (body.width - 2.0 * pad).max(1.0);
                let navigation =
                    self.is_category_root() || slot_id_from_page(id).is_some();
                let value = if navigation {
                    "›".into()
                } else {
                    display_value(entry)
                };
                let desired_control = if navigation {
                    36.0
                } else {
                    match entry.kind {
                        SettingKind::Boolean => self.font * 5.2 + 16.0,
                        SettingKind::Number { .. }
                        | SettingKind::ContinuousNumber { .. } => self.font * 10.0,
                        _ => (text.measure(&value, &opts) + 24.0)
                            .clamp(self.font * 7.0, self.font * 15.0),
                    }
                };
                let min_label = self.font
                    * if matches!(entry.kind, SettingKind::Boolean) {
                        8.0
                    } else {
                        10.0
                    };
                let inline = navigation || width - desired_control - pad >= min_label;
                let control_width = if inline {
                    desired_control.min(width)
                } else {
                    width
                };
                let text_width = if inline {
                    (width - control_width - pad).max(1.0)
                } else {
                    width
                };
                let label_opts = DrawOpts {
                    bold: navigation,
                    ..opts
                };
                let mut lines =
                    wrapped(&entry.label, text_width - 8.0, text, &label_opts);
                let label_lines = lines.len();
                lines.extend(visual_description_lines(
                    entry,
                    text_width - 8.0,
                    text,
                    &help_opts,
                ));
                if let Some(reason) = entry.availability.reason() {
                    lines.extend(wrapped(reason, text_width - 8.0, text, &help_opts));
                }
                let value_width = if matches!(entry.kind, SettingKind::Color { .. }) {
                    (control_width - line - pad - 8.0).max(1.0)
                } else if matches!(entry.kind, SettingKind::Boolean) {
                    (control_width - 40.0).max(1.0)
                } else {
                    (control_width - 16.0).max(1.0)
                };
                let value_lines = wrapped(&value, value_width, text, &opts);
                let control_h = (value_lines.len().max(1) as f32 * line + 8.0).max(32.0);
                let help_lines = lines.len().saturating_sub(label_lines);
                let text_height =
                    label_lines as f32 * line + 4.0 + help_lines as f32 * help_line;
                let height = if inline {
                    text_height.max(control_h) + pad * 2.0
                } else {
                    text_height + control_h + pad * 3.0
                };
                rows.push(Row {
                    id: id.clone(),
                    bounds: Rect {
                        x: body.x,
                        y: top,
                        width: body.width,
                        height,
                    },
                    control: Rect {
                        x: if inline {
                            body.x + body.width - pad - control_width
                        } else {
                            body.x + pad
                        },
                        y: if inline {
                            top + pad
                        } else {
                            top + pad * 2.0 + text_height
                        },
                        width: control_width,
                        height: control_h,
                    },
                    label: Rect {
                        x: body.x + pad,
                        y: top + pad,
                        width: text_width,
                        height: label_lines as f32 * line,
                    },
                    help: Rect {
                        x: body.x + pad,
                        y: top + pad + label_lines as f32 * line + 4.0,
                        width: text_width,
                        height: help_lines as f32 * help_line,
                    },
                    navigation,
                    lines,
                    label_lines,
                    value_lines,
                    help_line,
                });
                top += height + 4.0;
            }
        }
        self.content_height = top;
        self.scroll = self.scroll.clamp(0.0, (top - body.height).max(0.0));
        if self.reveal_focus && self.focus == Focus::List {
            if let Some(row) = rows.iter().find(|row| {
                self.view.as_ref().and_then(ViewState::focused) == Some(&row.id)
            }) {
                let reveal_top =
                    if row.control.y + row.control.height - row.bounds.y <= body.height {
                        row.bounds.y
                    } else {
                        row.control.y
                    };
                if reveal_top < self.scroll {
                    self.scroll = reveal_top;
                }
                if row.control.y + row.control.height > self.scroll + body.height {
                    self.scroll =
                        (row.control.y + row.control.height - body.height).max(0.0);
                }
            }
        }
        for row in &mut rows {
            row.bounds.y += body.y - self.scroll;
            row.control.y += body.y - self.scroll;
            row.label.y += body.y - self.scroll;
            row.help.y += body.y - self.scroll;
        }
        self.rows = rows;
        self.layout_dirty = false;
        self.reveal_focus = false;
        if let Some(view) = &mut self.view {
            view.set_viewport_rows(
                self.rows
                    .iter()
                    .filter(|row| {
                        row.control.y >= body.y
                            && row.control.y + row.control.height <= body.y + body.height
                    })
                    .count()
                    .max(1),
            );
        }
    }
    fn paint(&mut self, canvas: &mut impl Canvas, theme: UiTheme) {
        if !self.is_open() {
            return;
        }
        if self.confirmation.is_some() {
            self.paint_confirmation(canvas, theme);
            return;
        }
        self.prepare(canvas.text());
        if self.color_editor.is_some() {
            self.paint_color(canvas, theme);
            return;
        }
        let g = self.geometry;
        rect(canvas, g.viewport, theme.background, g.viewport);
        rounded_surface(canvas, g.card, theme.surface, g.viewport);
        let font = self.font.max(10.0);
        let line = font * 1.45;
        let help_font = (font * 0.76).max(9.0);
        let pad = 12.0_f32.min(g.card.width * 0.1);
        if self.requires_larger_window() {
            for (index, value) in self.compact_lines.iter().enumerate() {
                label(
                    canvas,
                    Rect {
                        y: g.body.y + index as f32 * line,
                        height: line,
                        ..g.body
                    },
                    value,
                    font,
                    theme.text,
                    false,
                    g.body,
                );
            }
            if g.close.height > 0.0 {
                control(canvas, g.close, true, theme, g.card);
                label(
                    canvas,
                    g.close,
                    if self.is_category_detail() {
                        "Esc: Back"
                    } else {
                        "Esc: Close"
                    },
                    font,
                    theme.text,
                    false,
                    g.card,
                );
            }
            return;
        }

        let title = Rect {
            x: if self.is_category_detail() {
                g.back.x + g.back.width + pad
            } else {
                g.search.x
            },
            y: g.card.y + pad,
            width: if self.is_category_detail() {
                (g.search.x + g.search.width - g.back.x - g.back.width - pad).max(0.0)
            } else {
                g.search.width
            },
            height: (g.search.y - g.card.y - pad).max(0.0),
        };
        if self.is_category_detail() {
            action_button(
                canvas,
                g.back,
                ("Back", "Esc"),
                font * 0.9,
                (false, true),
                theme,
                g.card,
            );
        }
        label(canvas, title, self.title(), font, theme.text, true, g.card);
        control(canvas, g.search, self.focus == Focus::Search, theme, g.card);
        let query = self.query();
        let display = if query.is_empty() && self.preedit.is_empty() {
            if self.is_category_root() {
                "Search customizations".into()
            } else if self.is_category_detail() {
                "Search this feature".into()
            } else {
                "Search settings".into()
            }
        } else {
            format!(
                "{}{}{}",
                &query[..self.caret],
                self.preedit,
                &query[self.caret..]
            )
        };
        let opts = DrawOpts {
            font_size: font,
            color: color_u8(if query.is_empty() && self.preedit.is_empty() {
                theme.muted_text
            } else {
                theme.text
            }),
            ..DrawOpts::default()
        };
        let caret_width = canvas.text().measure(&query[..self.caret], &opts);
        let shift = (caret_width - (g.search.width - pad * 3.0)).max(0.0);
        let selection =
            self.anchor
                .filter(|anchor| *anchor != self.caret)
                .map(|anchor| {
                    let start = anchor.min(self.caret);
                    let end = anchor.max(self.caret);
                    let left = canvas.text().measure(&query[..start], &opts);
                    let right = canvas.text().measure(&query[..end], &opts);
                    Rect {
                        x: g.search.x + pad + left - shift,
                        y: g.search.y + 3.0,
                        width: (right - left).max(0.0),
                        height: (g.search.height - 6.0).max(0.0),
                    }
                });
        self.caret_rect = Rect {
            x: g.search.x + pad + caret_width - shift,
            y: g.search.y + 3.0,
            width: 1.0,
            height: (g.search.height - 6.0).max(0.0),
        };
        if let Some(selection) = selection {
            rect(canvas, selection, theme.raised, g.search);
        }

        if let Some(clip) = g.search.intersect(g.card) {
            canvas.text().draw_clipped(
                g.search.x + pad - shift,
                g.search.y + font.min(g.search.height) * 0.15,
                &display,
                &opts,
                clip.array(),
            );
        }
        if self.focus == Focus::Search && self.preedit.is_empty() {
            rect(canvas, self.caret_rect, theme.outline, g.search);
        }
        for row in &self.rows {
            if row.bounds.intersect(g.body).is_none() {
                continue;
            }
            let selected = self.focus == Focus::List
                && self.view.as_ref().and_then(ViewState::focused) == Some(&row.id);
            let hovered = self
                .pointer
                .is_some_and(|(x, y)| g.body.contains(x, y) && row.bounds.contains(x, y));
            rounded_surface(
                canvas,
                row.bounds,
                if selected || hovered {
                    theme.raised
                } else {
                    theme.surface
                },
                g.body,
            );
            if selected {
                rect(
                    canvas,
                    Rect {
                        x: row.bounds.x,
                        y: row.bounds.y + 8.0,
                        width: 2.0,
                        height: (row.bounds.height - 16.0).max(0.0),
                    },
                    theme.outline,
                    g.body,
                );
            }
            for (index, value) in row.lines.iter().enumerate() {
                let heading = index < row.label_lines;
                let area = if heading { row.label } else { row.help };
                let bounds = Rect {
                    y: if heading {
                        area.y + index as f32 * line
                    } else {
                        area.y + (index - row.label_lines) as f32 * row.help_line
                    },
                    height: if heading { line } else { row.help_line },
                    ..area
                };
                label(
                    canvas,
                    bounds,
                    value,
                    if index < row.label_lines {
                        font
                    } else {
                        help_font
                    },
                    if index < row.label_lines {
                        theme.text
                    } else {
                        theme.muted_text
                    },
                    heading && row.navigation,
                    g.body,
                );
            }
            if row.navigation {
                label(
                    canvas,
                    centered_label(row.control, font),
                    "›",
                    font,
                    if selected || hovered {
                        theme.outline
                    } else {
                        theme.muted_text
                    },
                    false,
                    g.body,
                );
                continue;
            }
            control(canvas, row.control, selected, theme, g.body);
            let swatch_width = if let Some(SettingValue::Color(color)) = self
                .catalog
                .as_ref()
                .and_then(|catalog| catalog.get(&row.id))
                .map(|entry| &entry.value)
            {
                let size = (line - 4.0).min(row.control.height - 8.0);
                color_swatch(
                    canvas,
                    Rect {
                        x: row.control.x + 4.0,
                        y: row.control.y + (row.control.height - size) * 0.5,
                        width: size,
                        height: size,
                    },
                    *color,
                    theme,
                    g.body,
                );
                line + pad
            } else {
                0.0
            };
            if let Some(entry) = self
                .catalog
                .as_ref()
                .and_then(|catalog| catalog.get(&row.id))
            {
                if let SettingValue::Boolean(enabled) = entry.value {
                    let switch = Rect {
                        x: row.control.x + 8.0,
                        y: row.control.y + (row.control.height - 14.0) * 0.5,
                        width: 26.0,
                        height: 14.0,
                    };
                    rounded_fill(
                        canvas,
                        switch,
                        7.0,
                        if enabled { theme.outline } else { theme.raised },
                        g.body,
                    );
                    rounded_fill(
                        canvas,
                        Rect {
                            x: switch.x + if enabled { 14.0 } else { 2.0 },
                            y: switch.y + 2.0,
                            width: 10.0,
                            height: 10.0,
                        },
                        5.0,
                        if enabled {
                            theme.background
                        } else {
                            theme.muted_text
                        },
                        g.body,
                    );
                    label(
                        canvas,
                        centered_label(
                            Rect {
                                x: row.control.x + 38.0,
                                width: (row.control.width - 42.0).max(0.0),
                                ..row.control
                            },
                            font,
                        ),
                        if enabled { "On" } else { "Off" },
                        font,
                        theme.text,
                        false,
                        g.body,
                    );
                    continue;
                }
                if matches!(
                    entry.kind,
                    SettingKind::Number { .. } | SettingKind::ContinuousNumber { .. }
                ) {
                    let (decrement, input, increment) = numeric_zones(row.control, font);
                    let editor = self
                        .numeric_editor
                        .as_ref()
                        .filter(|editor| editor.id == row.id);
                    if editor.is_some() {
                        rect(canvas, input, theme.raised, g.body);
                    }
                    label(
                        canvas,
                        centered_label(decrement, font),
                        "−",
                        font,
                        theme.text,
                        false,
                        g.body,
                    );
                    label(
                        canvas,
                        centered_label(increment, font),
                        "+",
                        font,
                        theme.text,
                        false,
                        g.body,
                    );
                    let value = editor.map_or_else(
                        || match &entry.value {
                            SettingValue::Number(value) => number_label(entry, *value),
                            _ => String::new(),
                        },
                        |editor| editor.draft.clone(),
                    );
                    let opts = DrawOpts {
                        font_size: font,
                        color: color_u8(theme.text),
                        ..DrawOpts::default()
                    };
                    let caret = editor.map_or(value.len(), |editor| editor.caret);
                    let prefix = &value[..caret.min(value.len())];
                    let shift = (canvas.text().measure(prefix, &opts)
                        - (input.width - 16.0))
                        .max(0.0);
                    if let Some(editor) = editor {
                        if let Some(anchor) =
                            editor.anchor.filter(|anchor| *anchor != caret)
                        {
                            let left =
                                canvas.text().measure(&value[..anchor.min(caret)], &opts);
                            let right =
                                canvas.text().measure(&value[..anchor.max(caret)], &opts);
                            rect(
                                canvas,
                                Rect {
                                    x: input.x + 8.0 + left - shift,
                                    y: input.y + 3.0,
                                    width: (right - left).max(0.0),
                                    height: (input.height - 6.0).max(0.0),
                                },
                                theme.outline,
                                input.intersect(g.body).unwrap_or_default(),
                            );
                        }
                        self.caret_rect = Rect {
                            x: input.x + 8.0 + canvas.text().measure(prefix, &opts)
                                - shift,
                            y: input.y + 3.0,
                            width: 1.0,
                            height: (input.height - 6.0).max(0.0),
                        };
                    }
                    let shown = if editor.is_some() {
                        format!("{}{}{}", prefix, self.preedit, &value[caret..])
                    } else {
                        value.clone()
                    };
                    if let Some(clip) = input.intersect(g.body) {
                        canvas.text().draw_clipped(
                            input.x + 8.0 - shift,
                            centered_label(input, font).y + 2.0,
                            &shown,
                            &opts,
                            clip.array(),
                        );
                    }
                    if editor.is_some() && self.preedit.is_empty() {
                        rect(
                            canvas,
                            self.caret_rect,
                            theme.outline,
                            input.intersect(g.body).unwrap_or_default(),
                        );
                    }
                    continue;
                }
            }
            for (index, value) in row.value_lines.iter().enumerate() {
                label(
                    canvas,
                    Rect {
                        x: row.control.x + swatch_width + 4.0,
                        y: row.control.y
                            + (row.control.height - row.value_lines.len() as f32 * line)
                                * 0.5
                            + index as f32 * line,
                        width: (row.control.width - swatch_width - 8.0).max(0.0),
                        height: line,
                    },
                    value,
                    font,
                    theme.text,
                    false,
                    g.body,
                );
            }
        }
        if self.rows.is_empty() {
            label(
                canvas,
                g.body,
                "No settings match this search.",
                font,
                theme.muted_text,
                false,
                g.body,
            );
        }
        self.paint_preview(canvas, theme);
        action_button(
            canvas,
            g.reset,
            (
                if self.customizations.is_none() || g.reset.width < 115.0 {
                    "Reset"
                } else if self.is_category_root() {
                    "Reset all"
                } else if self
                    .customizations
                    .as_ref()
                    .is_some_and(|navigation| navigation.active_slot.is_some())
                {
                    if self
                        .customizations
                        .as_ref()
                        .and_then(|navigation| navigation.active_slot.as_ref())
                        .is_some_and(|slot| {
                            slot.as_str().starts_with("output.severity.")
                                || slot.as_str().starts_with("command_output.band.")
                                || slot.as_str().starts_with("kubernetes.severity.")
                        })
                    {
                        "Reset colors"
                    } else {
                        "Reset tag"
                    }
                } else {
                    "Reset default"
                },
                "R",
            ),
            font,
            (self.focus == Focus::Reset, true),
            theme,
            g.card,
        );
        if self.customizations.is_some() {
            action_button(
                canvas,
                g.restore,
                (
                    if g.restore.width < 126.0 {
                        "Restore"
                    } else {
                        "Restore saved"
                    },
                    "S",
                ),
                font,
                (self.focus == Focus::Restore, self.temporary_customizations),
                theme,
                g.card,
            );
        }
        action_button(
            canvas,
            g.close,
            ("Close", "C"),
            font,
            (self.focus == Focus::Close, true),
            theme,
            g.card,
        );
        let feedback = self.status_message();
        let preview_hints = self.preview_selector_available()
            && feedback.is_none_or(|status| status == "Saved");
        let status = if let Some(status) = feedback.filter(|status| *status != "Saved") {
            status
        } else if self.preview_edit_mode && self.focus == Focus::Preview {
            if g.status.width < 620.0 {
                "Tab: next | Enter: edit | Esc: done"
            } else {
                "Tab / Shift+Tab: select | Enter: edit | Esc: done"
            }
        } else if preview_hints {
            match (g.status.width < 620.0, feedback == Some("Saved")) {
                (true, true) => "E: edit | Saved | Esc: back",
                (true, false) => "E: edit | Esc: back",
                (false, true) => "E: edit preview | Saved | Tab: focus | Esc: back",
                (false, false) => "E: edit preview | Tab: focus | Esc: back",
            }
        } else if let Some(status) = feedback {
            status
        } else if self.is_category_detail() {
            if g.status.width < 420.0 {
                "Tab: focus | Esc: back"
            } else {
                "Tab: focus | Esc: back | Alt+Left: back"
            }
        } else if g.status.width < 420.0 {
            "Tab: focus | Esc: close"
        } else {
            "Tab: focus | Arrows: navigate | Esc: close"
        };
        if preview_hints {
            shortcut_hint(canvas, g.status, status, font * 0.85, theme, g.card);
        } else {
            label(
                canvas,
                g.status,
                status,
                font * 0.85,
                theme.muted_text,
                false,
                g.card,
            );
        }
    }

    fn preview_entry(&self, id: &str) -> Option<&SettingDescriptor> {
        self.customizations
            .as_ref()
            .and_then(|navigation| {
                navigation
                    .full_catalog
                    .entries()
                    .iter()
                    .find(|entry| entry.id.as_str() == id)
            })
            .or_else(|| {
                self.catalog
                    .as_ref()?
                    .entries()
                    .iter()
                    .find(|entry| entry.id.as_str() == id)
            })
    }

    fn preview_bool(&self, id: &str) -> bool {
        matches!(
            self.preview_entry(id).map(|entry| &entry.value),
            Some(SettingValue::Boolean(true))
        )
    }

    fn preview_color(&self, id: &str, fallback: [u8; 4]) -> [f32; 4] {
        let bytes = match self.preview_entry(id).map(|entry| &entry.value) {
            Some(SettingValue::Color(bytes)) => *bytes,
            _ => fallback,
        };
        bytes.map(|channel| f32::from(channel) / 255.0)
    }

    fn paint_preview(&mut self, canvas: &mut impl Canvas, theme: UiTheme) {
        self.preview_targets.clear();
        self.preview_tag_shapes.clear();
        self.preview_order.clear();
        self.preview_item_rows.clear();
        self.preview_total_rows = 0;
        self.preview_visible_rows = 0;
        self.preview_tag_list_visible_rows = 0;
        self.preview_button = Rect::default();
        self.preview_tag_list_area = Rect::default();
        let panel = self.geometry.preview;
        if panel.width <= 0.0 || panel.height <= 0.0 {
            return;
        }
        rounded_surface(
            canvas,
            panel,
            if matches!(self.focus, Focus::Preview | Focus::PreviewButton) {
                theme.outline
            } else {
                theme.raised
            },
            panel,
        );
        let inner = Rect {
            x: panel.x + 1.0,
            y: panel.y + 1.0,
            width: (panel.width - 2.0).max(0.0),
            height: (panel.height - 2.0).max(0.0),
        };
        rounded_surface(canvas, inner, theme.background, panel);
        let font = self.font.max(10.0);
        let caption = (font * 0.70).max(9.0);
        let title_height = font * 1.45;
        let header = Rect {
            x: inner.x + 8.0,
            y: inner.y + 4.0,
            width: (inner.width - 16.0).max(0.0),
            height: title_height.min((inner.height - 4.0).max(0.0)),
        };
        let mut title = header;
        if self.preview_selector_available() {
            let button_label = if self.preview_edit_mode {
                "Esc: Done"
            } else {
                "E: Edit"
            };
            let button_width = canvas.text().measure(
                button_label,
                &DrawOpts {
                    font_size: caption,
                    bold: true,
                    ..DrawOpts::default()
                },
            ) + 16.0;
            self.preview_button = Rect {
                x: header.x + (header.width - button_width).max(0.0),
                width: button_width.min(header.width),
                ..header
            };
            title.width = (self.preview_button.x - header.x - 8.0).max(0.0);
            control(
                canvas,
                self.preview_button,
                self.focus == Focus::PreviewButton,
                theme,
                inner,
            );
            shortcut_hint(
                canvas,
                self.preview_button,
                button_label,
                caption,
                theme,
                inner,
            );
        }
        label(
            canvas,
            title,
            if self.preview_edit_mode {
                "Editing"
            } else {
                "Preview"
            },
            caption,
            hint_accent(
                theme,
                if self.preview_edit_mode {
                    crate::renderer::ui_theme::BRAND_LIME
                } else {
                    crate::renderer::ui_theme::BRAND_CYAN
                },
            ),
            true,
            inner,
        );
        let mut sample = Rect {
            x: inner.x + 10.0,
            y: inner.y + title_height + 8.0,
            width: (inner.width - 20.0).max(0.0),
            height: (inner.height - title_height - 14.0).max(0.0),
        };
        if sample.width <= 0.0 || sample.height <= 0.0 {
            return;
        }
        if self.preview_selector_available() && self.is_tag_preview() {
            if self
                .customizations
                .as_ref()
                .and_then(|navigation| navigation.slot_pages.as_ref())
                .is_some_and(|snapshot| !snapshot.preview_devops_enabled())
            {
                let notice_height = (font * 1.45).min(sample.height * 0.18);
                label(
                    canvas,
                    Rect {
                        height: notice_height,
                        ..sample
                    },
                    "DevOps tags hidden",
                    caption,
                    hint_accent(theme, crate::renderer::ui_theme::BRAND_AMBER),
                    false,
                    sample,
                );
                sample.y += notice_height + 3.0;
                sample.height = (sample.height - notice_height - 3.0).max(0.0);
            }
            let row_height = (font * 1.5).max(16.0);
            let tag_count = self
                .customizations
                .as_ref()
                .and_then(|navigation| navigation.slot_pages.as_ref())
                .and_then(|snapshot| slot_page_actions(snapshot).ok())
                .map_or(0, |tags| tags.len())
                + 1;
            let columns = if sample.width >= 340.0 { 2 } else { 1 };
            let list_height = (tag_count.div_ceil(columns) as f32 * row_height)
                .min(sample.height * 0.70)
                .min((sample.height - row_height * 1.5).max(0.0));
            if list_height > 0.0 {
                self.preview_tag_list_area = Rect {
                    y: sample.y + sample.height - list_height,
                    height: list_height,
                    ..sample
                };
                sample.height = (sample.height - list_height - 4.0).max(0.0);
            }
        }
        let key = self
            .customizations
            .as_ref()
            .and_then(|navigation| navigation.active_key.as_ref())
            .map(SettingId::as_str);
        let mut interactive = Vec::new();
        match key {
            Some("tags.enabled") => {
                self.paint_tag_preview(canvas, sample, theme, &mut interactive)
            }
            Some(automexia_ui_model::settings::COMMAND_OUTPUT_HIGHLIGHTING) => {
                self.paint_output_preview(canvas, sample, theme, &mut interactive)
            }
            Some(automexia_ui_model::settings::KUBERNETES_HIGHLIGHTING) => {
                self.paint_kubernetes_preview(canvas, sample, theme, &mut interactive)
            }
            Some(automexia_ui_model::settings::INLINE_TABLES) => {
                self.paint_table_preview(canvas, sample, theme)
            }
            Some(automexia_ui_model::settings::COMMAND_TIMESTAMPS) => {
                self.paint_timestamp_preview(canvas, sample, theme)
            }
            Some(automexia_ui_model::settings::APPEARANCE_THEME) => {
                self.paint_theme_preview(canvas, sample, theme)
            }
            Some(automexia_ui_model::settings::FONT_SIZE) => {
                self.paint_font_preview(canvas, sample, theme)
            }
            _ => self.paint_generic_preview(canvas, sample, theme),
        }
        self.preview_targets = interactive;
        if self.is_tag_preview() {
            self.paint_tag_roster(canvas, theme);
        }
        if self.preview_edit_mode
            && self
                .preview_selected
                .as_ref()
                .is_none_or(|selected| !self.preview_order.contains(selected))
        {
            self.preview_selected = self.preview_order.first().cloned();
        }
        self.preview_reveal_selection = false;
    }

    fn paint_tag_roster(&mut self, canvas: &mut impl Canvas, theme: UiTheme) {
        let area = self.preview_tag_list_area;
        if area.width <= 0.0 || area.height <= 0.0 {
            return;
        }
        let Some(snapshot) = self
            .customizations
            .as_ref()
            .and_then(|navigation| navigation.slot_pages.as_ref())
        else {
            return;
        };
        let recipe = snapshot.preview_recipe();
        let mut tags = slot_page_actions(snapshot).unwrap_or_default();
        if self
            .customizations
            .as_ref()
            .is_some_and(|navigation| navigation.active_slot.is_none())
        {
            if let Some(add) = self.catalog.as_ref().and_then(|catalog| {
                SettingId::new("tags.add-slot")
                    .ok()
                    .and_then(|id| catalog.get(&id))
            }) {
                tags.push(add.clone());
            }
        }
        let row_height = (self.font.max(10.0) * 1.5).max(16.0);
        let columns = if area.width >= 340.0 { 2 } else { 1 };
        let page_rows = (area.height / row_height).floor().max(1.0) as usize;
        let capacity = page_rows * columns;
        self.preview_tag_list_visible_rows = capacity;
        if self.preview_reveal_selection {
            if let Some(index) = tags
                .iter()
                .position(|entry| Some(&entry.id) == self.preview_selected.as_ref())
            {
                if index < self.preview_tag_list_scroll {
                    self.preview_tag_list_scroll = index;
                } else if index >= self.preview_tag_list_scroll + capacity {
                    self.preview_tag_list_scroll = index + 1 - capacity;
                }
            }
        }
        self.preview_tag_list_scroll = self
            .preview_tag_list_scroll
            .min(tags.len().saturating_sub(capacity));
        // Sample availability and clipping must never reorder keyboard targets.
        self.preview_order = tags.iter().map(|entry| entry.id.clone()).collect();
        for (offset, entry) in tags
            .iter()
            .skip(self.preview_tag_list_scroll)
            .take(capacity)
            .enumerate()
        {
            let column_width = area.width / columns as f32;
            let row = Rect {
                x: area.x + (offset % columns) as f32 * column_width,
                y: area.y + (offset / columns) as f32 * row_height,
                width: column_width,
                height: row_height
                    .min(area.height - (offset / columns) as f32 * row_height),
            };
            if row.height <= 0.0 {
                break;
            }
            let row = Rect {
                width: (row.width - 3.0).max(0.0),
                height: (row.height - 2.0).max(0.0),
                ..row
            };
            let selected = self.preview_selected.as_ref() == Some(&entry.id);
            let hovered = self.pointer.is_some_and(|(x, y)| row.contains(x, y));
            if selected || hovered {
                control(canvas, row, selected, theme, area);
            }
            let state = slot_id_from_page(&entry.id)
                .and_then(|slot_id| recipe.slots.iter().find(|slot| slot.id == slot_id));
            let is_add = entry.id.as_str() == "tags.add-slot";
            let state_width = (self.font * 3.5).min(row.width * 0.26);
            let state_rect = Rect {
                x: row.x + 4.0,
                width: state_width,
                ..row
            };
            label(
                canvas,
                state_rect,
                if is_add {
                    "+"
                } else if state.is_some_and(|slot| slot.enabled) {
                    "On"
                } else {
                    "Off"
                },
                (self.font * 0.76).clamp(10.0, 16.0),
                if is_add || state.is_some_and(|slot| slot.enabled) {
                    theme.text
                } else {
                    theme.muted_text
                },
                true,
                area,
            );
            label(
                canvas,
                Rect {
                    x: state_rect.x + state_rect.width,
                    width: (row.width - state_rect.width - 6.0).max(0.0),
                    ..row
                },
                entry
                    .label
                    .strip_prefix("Tag slot: ")
                    .unwrap_or(&entry.label),
                (self.font * 0.76).clamp(10.0, 16.0),
                if state.is_some_and(|slot| slot.enabled) || is_add {
                    theme.text
                } else {
                    theme.muted_text
                },
                false,
                area,
            );
            if self.preview_selected.as_ref() == Some(&entry.id) {
                paint_preview_selection_border(canvas, row, area, theme);
            }
            if let Some(hit) = row.intersect(area) {
                self.preview_targets.push((entry.id.clone(), hit));
            }
        }
    }

    fn paint_tag_preview(
        &mut self,
        canvas: &mut impl Canvas,
        sample: Rect,
        theme: UiTheme,
        interactive: &mut Vec<(SettingId, Rect)>,
    ) {
        let Some(snapshot) = self
            .customizations
            .as_ref()
            .and_then(|navigation| navigation.slot_pages.as_ref())
            .cloned()
        else {
            self.paint_generic_preview(canvas, sample, theme);
            return;
        };
        let configured_recipe = snapshot.preview_recipe();
        let recipe =
            configured_recipe.with_devops_context(snapshot.preview_devops_enabled());
        let appearance = snapshot.preview_appearance();
        let foreground = snapshot.preview_foreground();
        let (terminal_background, _) = snapshot.preview_terminal_colors();
        let enabled = self.preview_bool("tags.enabled") && appearance.enabled;
        if !enabled {
            label(
                canvas,
                sample,
                "Information tags hidden",
                self.font * 0.8,
                theme.muted_text,
                false,
                sample,
            );
            return;
        }
        // Paint the sample on the actual terminal canvas color; application
        // chrome intentionally uses a different palette.
        rect(canvas, sample, terminal_background, sample);
        let row_height = (self.font * 1.8).clamp(18.0, 30.0);
        let metrics = prompt_tag_metrics(row_height);
        let top_inset = automexia_ui_model::prompt_context_top_inset(
            row_height,
            metrics.height,
            true,
        )
        .unwrap_or(0.0);
        let opts = DrawOpts {
            font_size: metrics.font_size,
            ..DrawOpts::default()
        };
        // Fictional context uses the same visibility projection as terminal
        // bands; toggling discovery never changes the saved recipe.
        let segments = preview_segments();
        let mut items = resolve_recipe(&recipe, &segments).unwrap_or_default();
        if matches!(
            recipe.arrangement,
            BarArrangement::Split | BarArrangement::TwoLine
        ) {
            items.sort_by_key(|item| {
                item.lane == automexia_ui_model::information_bar::BarLane::Trailing
            });
        }
        let available_width = (sample.width - 4.0).max(1.0);
        let (padding, gap, break_before, trailing_start) =
            bar_layout_hints(&recipe, &items, metrics, available_width);
        let labels: Vec<_> = items
            .iter()
            .map(|item| Label {
                text: &item.value,
                leading: item
                    .icon
                    .map_or(0.0, |_| metrics.icon_slot + metrics.icon_gap),
                padding,
                align_end: false,
            })
            .collect();
        let Some(band) = pack_with_tag_joins(
            &labels,
            available_width,
            gap,
            &break_before,
            trailing_start,
            automexia_ui_model::information_bar::tag_join_overlap(
                recipe.visual,
                metrics.height,
                available_width,
            ),
            |_, value| canvas.text().measure(value, &opts),
        ) else {
            return;
        };
        self.preview_total_rows = band
            .fragments
            .iter()
            .map(|fragment| fragment.row + 1)
            .max()
            .unwrap_or(0);
        self.preview_visible_rows =
            (sample.height / row_height).floor().max(1.0) as usize;
        self.preview_scroll = self.preview_scroll.min(
            self.preview_total_rows
                .saturating_sub(self.preview_visible_rows),
        );
        for fragment in &band.fragments {
            if let Some(item) = items.get(fragment.item) {
                if let Ok(id) = SettingId::new(format!("tags.slot.{}.page", item.slot_id))
                {
                    if !self.preview_order.contains(&id) {
                        self.preview_order.push(id.clone());
                        self.preview_item_rows.push((id, fragment.row));
                    }
                }
            }
        }
        if self.preview_reveal_selection {
            if let Some((_, row)) = self
                .preview_item_rows
                .iter()
                .find(|(id, _)| Some(id) == self.preview_selected.as_ref())
            {
                if *row < self.preview_scroll {
                    self.preview_scroll = *row;
                } else if *row >= self.preview_scroll + self.preview_visible_rows {
                    self.preview_scroll = row + 1 - self.preview_visible_rows;
                }
            }
        }
        let mut count = 0usize;
        for fragment in &band.fragments {
            let Some(item) = items.get(fragment.item) else {
                continue;
            };
            let bounds = Rect {
                x: sample.x + 2.0 + fragment.x,
                y: sample.y
                    + (fragment.row as isize - self.preview_scroll as isize) as f32
                        * row_height
                    + top_inset,
                width: fragment.width,
                height: metrics.height,
            };
            if bounds.y + bounds.height > sample.y + sample.height {
                continue;
            }
            if bounds.y < sample.y {
                continue;
            }
            let bounds = Rect {
                width: bounds.width.min(sample.x + sample.width - bounds.x),
                ..bounds
            };
            if let Some(hit) = bounds.intersect(sample) {
                if let Ok(id) = SettingId::new(format!("tags.slot.{}.page", item.slot_id))
                {
                    interactive.push((id, hit));
                    if let Some(shape) =
                        automexia_ui_model::information_bar::connected_tag_geometry(
                            recipe.visual,
                            bounds.width,
                            bounds.height,
                            fragment.shape_position,
                        )
                    {
                        self.preview_tag_shapes.push((hit, shape));
                    }
                }
            }
            let anchor = item.color.unwrap_or_else(|| {
                item.source_role.map_or(foreground, |role| {
                    crate::automexia::presentation::tag_anchor(&appearance, role)
                })
            });
            let color = Self::paint_sample_tag(
                canvas,
                bounds,
                anchor,
                recipe.visual,
                &appearance,
                terminal_background,
                fragment.shape_position,
            );
            if fragment.leading > 0.0 {
                if let Some(icon) = item.icon {
                    let glyph = automexia_ui_model::icon_glyph(icon);
                    let icon_opts = DrawOpts {
                        font_size: metrics.icon_size,
                        color: color_u8(color),
                        ..DrawOpts::default()
                    };
                    canvas.text().draw_clipped(
                        bounds.x + fragment.padding,
                        bounds.y + (metrics.height - metrics.icon_size) * 0.5 - 1.0,
                        glyph,
                        &icon_opts,
                        bounds.array(),
                    );
                }
            }
            if let Some(value) = item
                .value
                .get(fragment.bytes.clone())
                .filter(|_| !item.icon_only)
            {
                let text_opts = DrawOpts {
                    font_size: metrics.font_size * fragment.text_scale,
                    color: color_u8(color),
                    ..DrawOpts::default()
                };
                canvas.text().draw_clipped(
                    bounds.x + fragment.padding + fragment.leading,
                    bounds.y + (metrics.height - text_opts.font_size) * 0.5 - 1.0,
                    value,
                    &text_opts,
                    bounds.array(),
                );
            }
            count += 1;
        }
        // Selection is the final graphic layer, including when neighboring
        // shapes touch. Later tags must not paint over its focus outline.
        if let Some(selected) = self.preview_selected.as_ref() {
            for (_, bounds) in interactive.iter().filter(|(id, _)| id == selected) {
                paint_preview_selection_border(canvas, *bounds, sample, theme);
            }
        }
        if count == 0 {
            label(
                canvas,
                sample,
                "No visible sample tags",
                metrics.font_size,
                theme.muted_text,
                false,
                sample,
            );
        }
    }

    fn paint_sample_tag(
        canvas: &mut impl Canvas,
        bounds: Rect,
        anchor: [u8; 3],
        style: BarVisualStyle,
        appearance: &rio_backend::config::presentation::TagAppearance,
        terminal_background: [f32; 4],
        position: automexia_ui_model::information_bar::TagShapePosition,
    ) -> [f32; 4] {
        let opacity = if appearance.style
            == rio_backend::config::presentation::TagStyle::Plain
            || style == BarVisualStyle::Underline
        {
            0
        } else {
            appearance.opacity.get()
        };
        let colors =
            automexia_ui_model::context_tag_colors(terminal_background, anchor, opacity);
        paint_sample_tag_surface_in_run(canvas, bounds, style, colors, position);
        colors.foreground
    }

    fn preview_output_colors(
        &self,
        foreground: &crate::automexia::presentation::OutputColorBinding,
        background: &crate::automexia::presentation::OutputBackgroundBinding,
        terminal_foreground: [f32; 4],
    ) -> ([f32; 4], Option<[f32; 4]>) {
        self.preview_status_colors(
            foreground,
            background,
            terminal_foreground,
            automexia_ui_model::settings::OUTPUT_HIGHLIGHTING,
            "output.style",
        )
    }

    fn preview_status_colors(
        &self,
        foreground: &crate::automexia::presentation::OutputColorBinding,
        background: &crate::automexia::presentation::OutputBackgroundBinding,
        terminal_foreground: [f32; 4],
        enabled_id: &str,
        style_id: &str,
    ) -> ([f32; 4], Option<[f32; 4]>) {
        let enabled = self.preview_bool(enabled_id);
        let style = self
            .preview_entry(style_id)
            .and_then(|entry| match &entry.value {
                SettingValue::Choice(value) => Some(value.as_str()),
                _ => None,
            })
            .unwrap_or("both");
        let text = if enabled && style != "background" {
            self.preview_color(foreground.id, [255, 255, 255, 255])
        } else {
            terminal_foreground
        };
        let selected = self
            .preview_entry(background.id)
            .is_some_and(|entry| entry.origin != ValueOrigin::Default);
        let fill = (enabled
            && (style == "background"
                || (style == "both" && (background.default_in_both || selected))))
            .then(|| self.preview_color(background.id, background.default));
        (text, fill)
    }

    fn paint_output_preview(
        &mut self,
        canvas: &mut impl Canvas,
        sample: Rect,
        theme: UiTheme,
        interactive: &mut Vec<(SettingId, Rect)>,
    ) {
        use crate::automexia::presentation::{
            COMMAND_OUTPUT_BACKGROUND_BINDINGS, OUTPUT_BACKGROUND_BINDINGS,
            OUTPUT_COLOR_BINDINGS,
        };
        let (terminal_background, terminal_foreground) = self
            .customizations
            .as_ref()
            .and_then(|navigation| navigation.slot_pages.as_ref())
            .map_or(
                (theme.background, theme.text),
                SlotPageSnapshot::preview_terminal_colors,
            );
        rect(canvas, sample, terminal_background, sample);
        let font = (self.font * 0.72).clamp(9.0, 16.0);
        let line = font * 1.55;
        let command_examples = [
            "Build complete · exit 0",
            "Build failed · exit 1",
            "Command finished · status unknown",
        ];
        let examples = [
            "Log error: operation failed",
            "Log warning: retrying",
            "Log success: ready",
            "Log info: 3 resources",
            "Log debug: elapsed 104ms",
        ];
        let row_height = line + 3.0;
        self.preview_total_rows =
            COMMAND_OUTPUT_BACKGROUND_BINDINGS.len() + OUTPUT_COLOR_BINDINGS.len();
        self.preview_visible_rows =
            (sample.height / row_height).floor().max(1.0) as usize;
        self.preview_scroll = self.preview_scroll.min(
            self.preview_total_rows
                .saturating_sub(self.preview_visible_rows),
        );
        let command_palette = self
            .customizations
            .as_ref()
            .and_then(|navigation| navigation.slot_pages.as_ref())
            .map_or_else(
                || {
                    let colors = rio_backend::config::colors::Colors::default();
                    [colors.green, colors.red, colors.blue]
                },
                SlotPageSnapshot::preview_command_colors,
            );
        for (index, (binding, text)) in COMMAND_OUTPUT_BACKGROUND_BINDINGS
            .iter()
            .zip(command_examples)
            .enumerate()
        {
            let Some(kind) = binding.id.strip_prefix("command_output.backgrounds.")
            else {
                continue;
            };
            let Ok(id) = SettingId::new(format!("command_output.band.{kind}")) else {
                continue;
            };
            self.preview_order.push(id.clone());
            self.preview_item_rows.push((id.clone(), index));
            if index < self.preview_scroll {
                continue;
            }
            let bounds = Rect {
                y: sample.y + (index - self.preview_scroll) as f32 * row_height,
                height: line,
                ..sample
            };
            if bounds.y + bounds.height > sample.y + sample.height {
                continue;
            }
            if self
                .preview_bool(automexia_ui_model::settings::COMMAND_OUTPUT_HIGHLIGHTING)
            {
                let inherited = command_palette[index];
                let fallback = [inherited[0], inherited[1], inherited[2], 0.099];
                let color = if self
                    .preview_entry(binding.id)
                    .is_some_and(|entry| entry.origin == ValueOrigin::Default)
                {
                    fallback
                } else {
                    self.preview_color(
                        binding.id,
                        fallback.map(|channel| (channel.clamp(0.0, 1.0) * 255.0) as u8),
                    )
                };
                rect(canvas, bounds, color, sample);
            }
            label(
                canvas,
                bounds,
                text,
                font,
                terminal_foreground,
                false,
                sample,
            );
            if self.preview_edit_mode && self.preview_selected.as_ref() == Some(&id) {
                rect(
                    canvas,
                    Rect {
                        height: 1.0,
                        ..bounds
                    },
                    theme.text,
                    sample,
                );
            }
            interactive.push((id, bounds));
        }
        for (index, ((foreground_binding, background_binding), text)) in
            OUTPUT_COLOR_BINDINGS
                .iter()
                .zip(OUTPUT_BACKGROUND_BINDINGS.iter())
                .zip(examples)
                .enumerate()
        {
            let Some(severity) = foreground_binding.id.strip_prefix("output.colors.")
            else {
                continue;
            };
            let Ok(id) = SettingId::new(format!("output.severity.{severity}")) else {
                continue;
            };
            self.preview_order.push(id.clone());
            let row = index + COMMAND_OUTPUT_BACKGROUND_BINDINGS.len();
            self.preview_item_rows.push((id.clone(), row));
            if row < self.preview_scroll {
                continue;
            }
            let bounds = Rect {
                y: sample.y + (row - self.preview_scroll) as f32 * row_height,
                height: line,
                ..sample
            };
            if bounds.y + bounds.height > sample.y + sample.height {
                continue;
            }
            let (foreground, background) = self.preview_output_colors(
                foreground_binding,
                background_binding,
                terminal_foreground,
            );
            if let Some(background) = background {
                rect(canvas, bounds, background, sample);
            }
            label(canvas, bounds, text, font, foreground, false, sample);
            if self.preview_edit_mode && self.preview_selected.as_ref() == Some(&id) {
                rect(
                    canvas,
                    Rect {
                        height: 1.0,
                        ..bounds
                    },
                    theme.text,
                    sample,
                );
            }
            interactive.push((id, bounds));
        }
    }

    fn paint_kubernetes_preview(
        &mut self,
        canvas: &mut impl Canvas,
        sample: Rect,
        theme: UiTheme,
        interactive: &mut Vec<(SettingId, Rect)>,
    ) {
        use crate::automexia::presentation::{
            KUBERNETES_BACKGROUND_BINDINGS, KUBERNETES_COLOR_BINDINGS,
        };
        let (terminal_background, terminal_foreground) = self
            .customizations
            .as_ref()
            .and_then(|navigation| navigation.slot_pages.as_ref())
            .map_or(
                (theme.background, theme.text),
                SlotPageSnapshot::preview_terminal_colors,
            );
        rect(canvas, sample, terminal_background, sample);
        let font = (self.font * 0.72).clamp(9.0, 16.0);
        let line = font * 1.55;
        let row_height = line + 3.0;
        // The second warning row demonstrates that Unknown and incomplete
        // Running readiness share the warning palette. Debug is a palette
        // sample; it does not claim an active Kubernetes health state.
        let examples = [
            (0, "pod-a  0/1  CrashLoopBackOff"),
            (1, "worker  0/1  Running"),
            (1, "cache  0/1  Unknown"),
            (2, "api  1/1  Running"),
            (3, "NAME  READY  STATUS"),
            (4, "Debug palette · no active status"),
        ];
        self.preview_total_rows = examples.len();
        self.preview_visible_rows =
            (sample.height / row_height).floor().max(1.0) as usize;
        self.preview_scroll = self.preview_scroll.min(
            self.preview_total_rows
                .saturating_sub(self.preview_visible_rows),
        );
        for (row, (severity, text)) in examples.into_iter().enumerate() {
            let foreground_binding = &KUBERNETES_COLOR_BINDINGS[severity];
            let background_binding = &KUBERNETES_BACKGROUND_BINDINGS[severity];
            let Some(name) = foreground_binding.id.strip_prefix("kubernetes.colors.")
            else {
                continue;
            };
            let Ok(id) = SettingId::new(format!("kubernetes.severity.{name}")) else {
                continue;
            };
            if !self.preview_order.contains(&id) {
                self.preview_order.push(id.clone());
                self.preview_item_rows.push((id.clone(), row));
            }
            if row < self.preview_scroll {
                continue;
            }
            let bounds = Rect {
                y: sample.y + (row - self.preview_scroll) as f32 * row_height,
                height: line,
                ..sample
            };
            if bounds.y + bounds.height > sample.y + sample.height {
                continue;
            }
            let (foreground, background) = self.preview_status_colors(
                foreground_binding,
                background_binding,
                terminal_foreground,
                automexia_ui_model::settings::KUBERNETES_HIGHLIGHTING,
                "kubernetes.style",
            );
            if let Some(background) = background {
                rect(canvas, bounds, background, sample);
            }
            label(canvas, bounds, text, font, foreground, false, sample);
            if self.preview_edit_mode && self.preview_selected.as_ref() == Some(&id) {
                rect(
                    canvas,
                    Rect {
                        height: 1.0,
                        ..bounds
                    },
                    theme.text,
                    sample,
                );
            }
            interactive.push((id, bounds));
        }
    }

    fn paint_theme_preview(
        &self,
        canvas: &mut impl Canvas,
        sample: Rect,
        theme: UiTheme,
    ) {
        let selected = self
            .preview_entry(automexia_ui_model::settings::APPEARANCE_THEME)
            .and_then(|entry| match &entry.value {
                SettingValue::Choice(value) => Some(value.as_str()),
                _ => None,
            })
            .unwrap_or("system");
        let font = (self.font * 0.70).clamp(9.0, 15.0);
        let (background, foreground) = self
            .customizations
            .as_ref()
            .and_then(|navigation| navigation.slot_pages.as_ref())
            .map_or(
                (theme.background, theme.text),
                SlotPageSnapshot::preview_terminal_colors,
            );
        let caption = match selected {
            "light" => "Light appearance",
            "dark" => "Dark appearance",
            _ => "System appearance",
        };
        rect(canvas, sample, background, sample);
        label(
            canvas,
            Rect {
                height: font * 1.6,
                ..sample
            },
            caption,
            font,
            foreground,
            true,
            sample,
        );
        label(
            canvas,
            Rect {
                y: sample.y + font * 1.8,
                ..sample
            },
            "λ  sample command",
            font,
            foreground,
            false,
            sample,
        );
    }

    fn paint_font_preview(&self, canvas: &mut impl Canvas, sample: Rect, theme: UiTheme) {
        let (background, foreground) = self
            .customizations
            .as_ref()
            .and_then(|navigation| navigation.slot_pages.as_ref())
            .map_or(
                (theme.background, theme.text),
                SlotPageSnapshot::preview_terminal_colors,
            );
        rect(canvas, sample, background, sample);
        let size = self
            .preview_entry(automexia_ui_model::settings::FONT_SIZE)
            .and_then(|entry| match &entry.value {
                SettingValue::Number(size) if size.is_finite() => Some(*size as f32),
                _ => None,
            })
            .unwrap_or(self.font);
        let caption = (self.font * 0.70).clamp(9.0, 15.0);
        label(
            canvas,
            Rect {
                height: caption * 1.6,
                ..sample
            },
            &format!("{size} pt text"),
            caption,
            theme.muted_text,
            false,
            sample,
        );
        let letters = Rect {
            y: sample.y + caption * 1.8,
            height: (sample.height - caption * 1.8).max(0.0),
            ..sample
        };
        label(
            canvas,
            letters,
            "Aa 0123 λ",
            size,
            foreground,
            false,
            sample,
        );
    }

    fn paint_generic_preview(
        &self,
        canvas: &mut impl Canvas,
        sample: Rect,
        theme: UiTheme,
    ) {
        let Some(entry) = self.focused_entry() else {
            return;
        };
        let font = (self.font * 0.75).clamp(9.0, 16.0);
        let value = display_value(entry);
        label(
            canvas,
            Rect {
                height: font * 1.5,
                ..sample
            },
            &entry.label,
            font,
            theme.text,
            true,
            sample,
        );
        let value_bounds = Rect {
            y: sample.y + font * 1.7,
            height: font * 1.7,
            ..sample
        };
        match &entry.value {
            SettingValue::Color(color) => {
                color_swatch(
                    canvas,
                    Rect {
                        width: font * 1.6,
                        height: font * 1.6,
                        ..value_bounds
                    },
                    *color,
                    theme,
                    sample,
                );
                label(
                    canvas,
                    Rect {
                        x: value_bounds.x + font * 1.9,
                        width: (value_bounds.width - font * 1.9).max(0.0),
                        ..value_bounds
                    },
                    &value,
                    font,
                    theme.muted_text,
                    false,
                    sample,
                );
            }
            SettingValue::Boolean(enabled) => {
                let switch = Rect {
                    width: font * 3.1,
                    height: font * 1.5,
                    ..value_bounds
                };
                rect(
                    canvas,
                    switch,
                    if *enabled {
                        theme.outline
                    } else {
                        theme.raised
                    },
                    sample,
                );
                let knob = Rect {
                    x: switch.x
                        + if *enabled {
                            switch.width - font * 1.3
                        } else {
                            2.0
                        },
                    y: switch.y + 2.0,
                    width: font * 1.15,
                    height: (switch.height - 4.0).max(0.0),
                };
                rect(canvas, knob, theme.text, sample);
                label(
                    canvas,
                    Rect {
                        x: switch.x + switch.width + 8.0,
                        width: (sample.width - switch.width - 8.0).max(0.0),
                        ..value_bounds
                    },
                    &value,
                    font,
                    theme.muted_text,
                    false,
                    sample,
                );
            }
            _ => label(
                canvas,
                value_bounds,
                &value,
                font,
                theme.muted_text,
                false,
                sample,
            ),
        }
        if let Some(reason) = entry.availability.reason() {
            let unavailable = Rect {
                y: value_bounds.y + value_bounds.height + 4.0,
                height: (sample.height - value_bounds.height - font * 1.7 - 4.0).max(0.0),
                ..sample
            };
            label(
                canvas,
                unavailable,
                reason,
                (font * 0.82).max(9.0),
                theme.muted_text,
                false,
                sample,
            );
        }
    }
}

#[cfg(test)]
fn timestamp_preview_label(shown: bool) -> String {
    timestamp_preview::sample_text(Default::default(), shown, Some(0), Some(104)).text
}

fn preview_segments() -> Vec<automexia_ui_model::Segment> {
    use automexia_extension_api::Freshness;
    automexia_ui_model::information_bar::STANDARD_ROLES
        .into_iter()
        .map(|role| {
            let value = preview_role_value(role).to_owned();
            automexia_ui_model::Segment {
                accessibility_label: value.clone(),
                value,
                role,
                icon: preview_role_icon(role),
                priority: 10,
                freshness: Freshness::Current,
                observed_at_ms: 0,
                details_action: None,
            }
        })
        .collect()
}

fn paint_sample_tag_surface(
    canvas: &mut impl Canvas,
    bounds: Rect,
    style: BarVisualStyle,
    paint: automexia_ui_model::ContextTagColors,
) {
    paint_sample_tag_surface_in_run(canvas, bounds, style, paint, Default::default());
}

fn paint_sample_tag_surface_in_run(
    canvas: &mut impl Canvas,
    bounds: Rect,
    style: BarVisualStyle,
    paint: automexia_ui_model::ContextTagColors,
    position: automexia_ui_model::information_bar::TagShapePosition,
) {
    let Some(geometry) = automexia_ui_model::information_bar::tag_fragment_geometry(
        style,
        bounds.width,
        bounds.height,
        position,
    ) else {
        return;
    };
    let layers = tag_surface_paint_layers(style, paint.background[3]);
    let mut outline = paint.foreground;
    outline[3] = 0.45;
    match geometry {
        TagSurfaceGeometry::Connected(shape) => {
            let offset = |p: (f32, f32)| (bounds.x + p.0, bounds.y + p.1);
            if layers.fill {
                for triangle in shape.triangles() {
                    canvas.polygon(&triangle.map(offset), paint.background);
                }
            } else {
                for (a, b) in shape
                    .points()
                    .iter()
                    .zip(shape.points().iter().cycle().skip(1))
                {
                    canvas.line(offset(*a), offset(*b), shape.stroke, outline);
                }
            }
            if layers.fill {
                if let Some(fold) = shape.fold {
                    let color = [
                        paint.background[0] * 0.55,
                        paint.background[1] * 0.55,
                        paint.background[2] * 0.55,
                        paint.background[3],
                    ];
                    canvas.polygon(&fold.map(offset), color);
                }
            }
        }
        TagSurfaceGeometry::Capsule { radius, stroke } => {
            if layers.fill {
                canvas.rounded_rect(bounds.array(), radius, paint.background);
            } else {
                preview_rounded_outline(canvas, bounds, radius, stroke, outline);
            }
        }
        TagSurfaceGeometry::Flat { stroke } => {
            if layers.fill {
                canvas.rect(bounds.array(), paint.background);
            } else {
                preview_rect_outline(canvas, bounds, stroke, outline);
            }
        }
        TagSurfaceGeometry::Chevron { points, stroke }
        | TagSurfaceGeometry::Hexagon { points, stroke } => {
            let points = points.map(|(x, y)| (bounds.x + x, bounds.y + y));
            if layers.fill {
                canvas.polygon(&points, paint.background);
            } else {
                for index in 0..points.len() {
                    canvas.line(
                        points[index],
                        points[(index + 1) % points.len()],
                        stroke,
                        outline,
                    );
                }
            }
        }
        TagSurfaceGeometry::Card { radius, stroke } => {
            if layers.fill {
                canvas.rounded_rect(bounds.array(), radius, paint.background);
            }
            preview_rounded_outline(canvas, bounds, radius, stroke, outline);
        }
        TagSurfaceGeometry::Underline { baseline_y, stroke } => {
            canvas.line(
                (bounds.x + stroke * 0.5, bounds.y + baseline_y),
                (
                    bounds.x + bounds.width - stroke * 0.5,
                    bounds.y + baseline_y,
                ),
                stroke,
                paint.foreground,
            );
        }
    }
}

fn preview_rect_outline(
    canvas: &mut impl Canvas,
    bounds: Rect,
    stroke: f32,
    color: [f32; 4],
) {
    let inset = stroke * 0.5;
    let points = [
        (bounds.x + inset, bounds.y + inset),
        (bounds.x + bounds.width - inset, bounds.y + inset),
        (
            bounds.x + bounds.width - inset,
            bounds.y + bounds.height - inset,
        ),
        (bounds.x + inset, bounds.y + bounds.height - inset),
    ];
    for index in 0..points.len() {
        canvas.line(
            points[index],
            points[(index + 1) % points.len()],
            stroke,
            color,
        );
    }
}

fn paint_preview_selection_border(
    canvas: &mut impl Canvas,
    bounds: Rect,
    clip: Rect,
    theme: UiTheme,
) {
    let Some(bounds) = bounds.intersect(clip) else {
        return;
    };
    if bounds.width < 6.0 || bounds.height < 6.0 {
        return;
    }
    // The light outer stroke remains visible on dark tags; the dark inner
    // stroke separates it from pale user-chosen tag colors. Both stay inside
    // the tag hit target so adjacent tags and pane edges are not covered.
    let outer_stroke = 2.5;
    preview_rect_outline(canvas, bounds, outer_stroke, theme.text);
    let inner = Rect {
        x: bounds.x + outer_stroke,
        y: bounds.y + outer_stroke,
        width: bounds.width - 2.0 * outer_stroke,
        height: bounds.height - 2.0 * outer_stroke,
    };
    if inner.width >= 2.0 && inner.height >= 2.0 {
        preview_rect_outline(canvas, inner, 1.0, theme.background);
    }
}

fn preview_rounded_outline(
    canvas: &mut impl Canvas,
    bounds: Rect,
    radius: f32,
    stroke: f32,
    color: [f32; 4],
) {
    if radius < stroke * 0.5 {
        preview_rect_outline(canvas, bounds, stroke, color);
        return;
    }
    let inset = stroke * 0.5;
    let left = bounds.x + inset + radius;
    let right = bounds.x + bounds.width - inset - radius;
    let top = bounds.y + inset + radius;
    let bottom = bounds.y + bounds.height - inset - radius;
    for (from, to) in [
        ((left, bounds.y + inset), (right, bounds.y + inset)),
        (
            (bounds.x + bounds.width - inset, top),
            (bounds.x + bounds.width - inset, bottom),
        ),
        (
            (right, bounds.y + bounds.height - inset),
            (left, bounds.y + bounds.height - inset),
        ),
        ((bounds.x + inset, bottom), (bounds.x + inset, top)),
    ] {
        canvas.line(from, to, stroke, color);
    }
    for (cx, cy, start, end) in [
        (right, top, -90.0, 0.0),
        (right, bottom, 0.0, 90.0),
        (left, bottom, 90.0, 180.0),
        (left, top, 180.0, 270.0),
    ] {
        canvas.arc((cx, cy), radius, (start, end), stroke, color);
    }
}

fn preview_role_value(role: automexia_extension_api::SegmentRole) -> &'static str {
    use automexia_extension_api::SegmentRole;
    match role {
        SegmentRole::Production => "prod",
        SegmentRole::UbuntuWsl => "WSL",
        SegmentRole::Windows => "Windows",
        SegmentRole::Git => "main",
        SegmentRole::Kubernetes => "demo",
        SegmentRole::Docker => "Docker",
        SegmentRole::Azure => "Azure",
        SegmentRole::Aws => "AWS",
        SegmentRole::Gcp => "GCP",
        SegmentRole::UnknownCloud => "Cloud",
        SegmentRole::Terraform => "Terraform",
        SegmentRole::Environment => "dev",
        SegmentRole::User => "user",
    }
}

fn preview_role_icon(
    role: automexia_extension_api::SegmentRole,
) -> automexia_extension_api::IconKind {
    use automexia_extension_api::{IconKind, SegmentRole};
    match role {
        SegmentRole::Production => IconKind::Production,
        SegmentRole::UbuntuWsl => IconKind::Wsl,
        SegmentRole::Windows => IconKind::Windows,
        SegmentRole::Git => IconKind::Git,
        SegmentRole::Kubernetes => IconKind::Kubernetes,
        SegmentRole::Docker => IconKind::Docker,
        SegmentRole::Azure
        | SegmentRole::Aws
        | SegmentRole::Gcp
        | SegmentRole::UnknownCloud => IconKind::Cloud,
        SegmentRole::Terraform => IconKind::Terraform,
        SegmentRole::Environment => IconKind::Environment,
        SegmentRole::User => IconKind::User,
    }
}
fn package_root_catalog(
    navigation: &CustomizationNavigation,
) -> Result<Catalog, automexia_ui_model::settings::SettingsError> {
    let core = customization_root_catalog(&navigation.full_catalog, &navigation.groups)?;
    let mut entries = core.entries().to_vec();
    if let Some(packages) = &navigation.package_pages {
        entries.extend(packages.root_actions().cloned());
    }
    Catalog::new(core.revision(), entries)
}

fn detail_catalog(full: &Catalog, group: &CustomizationGroup) -> Option<Catalog> {
    let entries = group
        .members
        .iter()
        .map(|id| full.get(id).cloned())
        .collect::<Option<Vec<_>>>()?;
    Catalog::new(full.revision(), entries).ok()
}
fn detail_catalog_with_slots(
    full: &Catalog,
    group: &CustomizationGroup,
    snapshot: Option<&SlotPageSnapshot>,
) -> Option<Catalog> {
    if group.key.as_str() == automexia_ui_model::settings::INLINE_TABLES {
        if let Some(snapshot) = snapshot {
            return crate::settings_catalog::table_page_catalog(full, snapshot).ok();
        }
    }
    if group.key.as_str() == automexia_ui_model::settings::COMMAND_TIMESTAMPS {
        if let Some(snapshot) = snapshot {
            return crate::settings_catalog::timestamp_page_catalog(full, snapshot).ok();
        }
    }
    detail_catalog(full, group)
}
fn preview_detail_catalog(
    navigation: &CustomizationNavigation,
    key: &SettingId,
) -> Option<Catalog> {
    if let Some(slot_id) = slot_id_from_page(key) {
        return selected_tag_catalog(
            &navigation.full_catalog,
            navigation.slot_pages.as_ref()?,
            slot_id,
        )
        .ok();
    }
    if let Some(role_id) = key.as_str().strip_prefix("tags.colors.") {
        return tag_role_color_catalog(&navigation.full_catalog, role_id).ok();
    }
    if let Some(severity) = key.as_str().strip_prefix("output.severity.") {
        return output_severity_catalog(&navigation.full_catalog, severity).ok();
    }
    if let Some(kind) = key.as_str().strip_prefix("command_output.band.") {
        return command_output_band_catalog(&navigation.full_catalog, kind).ok();
    }
    if let Some(severity) = key.as_str().strip_prefix("kubernetes.severity.") {
        return kubernetes_severity_catalog(&navigation.full_catalog, severity).ok();
    }
    None
}

fn preview_page_still_available(
    navigation: &CustomizationNavigation,
    key: &SettingId,
) -> bool {
    if let Some(slot_id) = slot_id_from_page(key) {
        if automexia_ui_model::information_bar::role_from_id(slot_id).is_some() {
            return true;
        }
        return navigation.slot_pages.as_ref().is_some_and(|snapshot| {
            snapshot
                .preview_recipe()
                .slots
                .iter()
                .any(|slot| slot.id == slot_id)
        });
    }
    if key.as_str().starts_with("tags.colors.") {
        return crate::automexia::presentation::TAG_COLOR_BINDINGS
            .iter()
            .any(|binding| binding.id == key.as_str());
    }
    if let Some(kind) = key.as_str().strip_prefix("command_output.band.") {
        return ["success", "failure", "neutral"].contains(&kind);
    }
    if let Some(severity) = key.as_str().strip_prefix("kubernetes.severity.") {
        return ["error", "warning", "success", "info", "debug"].contains(&severity);
    }
    key.as_str()
        .strip_prefix("output.severity.")
        .is_some_and(|severity| {
            ["error", "warning", "success", "info", "debug"].contains(&severity)
        })
}

fn tag_color_graphic_name(id: &str) -> String {
    use automexia_ui_model::information_bar::{role_from_id, role_label};

    let role_id = id.strip_prefix("tags.colors.").or_else(|| {
        id.strip_prefix("tags.slot.")
            .and_then(|rest| rest.split_once('.').map(|(slot, _)| slot))
    });
    let role = crate::automexia::presentation::TAG_COLOR_BINDINGS
        .iter()
        .find(|binding| binding.id == id)
        .map(|binding| binding.role)
        .or_else(|| role_id.and_then(role_from_id));
    role.map(role_label)
        .map(str::to_owned)
        .unwrap_or_else(|| role_id.unwrap_or("Custom").replace(['-', '_'], " "))
}
fn display_value(entry: &SettingDescriptor) -> String {
    match &entry.value {
        SettingValue::Boolean(value) => if *value { "On" } else { "Off" }.into(),
        SettingValue::Choice(value) => {
            if let SettingKind::Choice { options } = &entry.kind {
                options
                    .iter()
                    .find(|option| &option.value == value)
                    .map_or_else(
                        || value.clone(),
                        |option| format!("< {} >", option.label),
                    )
            } else {
                value.clone()
            }
        }
        SettingValue::Text(value) => {
            if value.is_empty() {
                "Empty".into()
            } else {
                value.clone()
            }
        }
        SettingValue::Number(value) => format!("-   {}   +", number_label(entry, *value)),
        SettingValue::Color([red, green, blue, alpha]) => {
            if matches!(entry.kind, SettingKind::Color { alpha: true }) {
                format!("#{red:02X}{green:02X}{blue:02X}{alpha:02X}")
            } else {
                format!("#{red:02X}{green:02X}{blue:02X}")
            }
        }
        SettingValue::Action if entry.id.as_str() == "tags.add-slot" => "Add".into(),
        SettingValue::Action
            if entry.id.as_str().starts_with("tags.slot.")
                && entry.id.as_str().ends_with(".remove") =>
        {
            "Remove".into()
        }
        SettingValue::Action => "Open".into(),
    }
}
fn number_label(entry: &SettingDescriptor, value: f64) -> String {
    // Font preferences are f32; preserve their shortest round-trip label.
    // Other continuous f64 values retain their full representation.
    if matches!(entry.kind, SettingKind::ContinuousNumber { .. })
        && f64::from(value as f32) == value
    {
        (value as f32).to_string()
    } else {
        value.to_string()
    }
}
fn parse_color(value: &str, alpha: bool) -> Option<[u8; 4]> {
    if !value.starts_with('#') || !(value.len() == 7 || (alpha && value.len() == 9)) {
        return None;
    }
    let color = ColorBuilder::from_hex(value.to_owned(), Format::SRGB0_255).ok()?;
    // The strict parser supplies exact bounded sRGB bytes and normalized alpha.
    Some([
        color.red as u8,
        color.green as u8,
        color.blue as u8,
        (color.alpha * 255.0).round() as u8,
    ])
}
fn color_swatch(
    canvas: &mut impl Canvas,
    bounds: Rect,
    color: [u8; 4],
    theme: UiTheme,
    clip: Rect,
) {
    control(canvas, bounds, false, theme, clip);
    let inner = Rect {
        x: bounds.x + 2.0,
        y: bounds.y + 2.0,
        width: (bounds.width - 4.0).max(0.0),
        height: (bounds.height - 4.0).max(0.0),
    };
    for y in 0..2 {
        for x in 0..2 {
            rect(
                canvas,
                Rect {
                    x: inner.x + inner.width * x as f32 * 0.5,
                    y: inner.y + inner.height * y as f32 * 0.5,
                    width: inner.width * 0.5,
                    height: inner.height * 0.5,
                },
                if (x + y) % 2 == 0 {
                    [0.75, 0.75, 0.75, 1.0]
                } else {
                    [0.3, 0.3, 0.3, 1.0]
                },
                clip,
            );
        }
    }
    rect(
        canvas,
        inner,
        color.map(|channel| f32::from(channel) / 255.0),
        clip,
    );
}
fn wrapped(value: &str, width: f32, text: &mut Text, opts: &DrawOpts) -> Vec<String> {
    let mut result = Vec::new();
    let mut line = String::new();
    for word in value.split_inclusive(char::is_whitespace) {
        let mut next = line.clone();
        next.push_str(word);
        if text.measure(&next, opts) <= width {
            line = next;
            continue;
        }
        if !line.trim_end().is_empty() {
            result.push(line.trim_end().to_owned());
        }
        line.clear();
        let word = word.trim_start();
        if text.measure(word, opts) <= width {
            line = word.into();
            continue;
        }
        for grapheme in word.graphemes(true) {
            let mut next = line.clone();
            next.push_str(grapheme);
            if !line.is_empty() && text.measure(&next, opts) > width {
                result.push(line);
                line = grapheme.into();
            } else {
                line = next;
            }
        }
    }
    if !line.trim_end().is_empty() {
        result.push(line.trim_end().to_owned());
    }
    result
}

/// Keep untrusted extension prose compact in the sheet while retaining the
/// complete descriptor for search and accessibility. Availability reasons are
/// laid out separately and never clipped by this helper.
fn visual_description_lines(
    entry: &SettingDescriptor,
    width: f32,
    text: &mut Text,
    opts: &DrawOpts,
) -> Vec<String> {
    use automexia_ui_model::settings::SettingOwner;

    let is_extension = matches!(&entry.owner, SettingOwner::Extension(_));
    let mut lines = wrapped(&entry.description, width, text, opts);
    if is_extension && lines.len() > 2 {
        lines.truncate(2);
        if let Some(last) = lines.last_mut() {
            while !last.is_empty() && text.measure(&format!("{last}…"), opts) > width {
                let end = last
                    .grapheme_indices(true)
                    .next_back()
                    .map_or(0, |(start, _)| start);
                last.truncate(end);
            }
            if text.measure("…", opts) <= width {
                last.push('…');
            }
        }
    }
    lines
}

fn rect(canvas: &mut impl Canvas, bounds: Rect, color: [f32; 4], clip: Rect) {
    if let Some(bounds) = bounds.intersect(clip) {
        canvas.rect(bounds.array(), color);
    }
}
fn rounded_fill(
    canvas: &mut impl Canvas,
    bounds: Rect,
    radius: f32,
    color: [f32; 4],
    clip: Rect,
) {
    if let Some(visible) = bounds.intersect(clip) {
        // Subtracting intersected edges can change a fully contained f32 width
        // by one ULP. Test containment directly so fractional DPI stays rounded.
        if bounds.x >= clip.x
            && bounds.y >= clip.y
            && bounds.x + bounds.width <= clip.x + clip.width
            && bounds.y + bounds.height <= clip.y + clip.height
        {
            canvas.rounded_rect(
                bounds.array(),
                radius.min(bounds.height * 0.5).min(bounds.width * 0.5),
                color,
            );
        } else {
            // Canvas rounded primitives have no scissor. Keep a partially
            // scrolled control inside its panel instead of leaking its corners.
            canvas.rect(visible.array(), color);
        }
    }
}
fn rounded_surface(canvas: &mut impl Canvas, bounds: Rect, color: [f32; 4], clip: Rect) {
    rounded_fill(
        canvas,
        bounds,
        crate::renderer::ui_theme::CONTROL_RADIUS,
        color,
        clip,
    );
}
fn centered_label(bounds: Rect, font: f32) -> Rect {
    Rect {
        y: bounds.y + ((bounds.height - font * 1.45) * 0.5).max(0.0),
        height: (font * 1.45).min(bounds.height),
        ..bounds
    }
}
fn control(
    canvas: &mut impl Canvas,
    bounds: Rect,
    focused: bool,
    theme: UiTheme,
    clip: Rect,
) {
    rounded_surface(
        canvas,
        bounds,
        if focused { theme.outline } else { theme.raised },
        clip,
    );
    let border = if focused { 2.0 } else { 1.0 };
    rounded_fill(
        canvas,
        Rect {
            x: bounds.x + border,
            y: bounds.y + border,
            width: (bounds.width - 2.0 * border).max(0.0),
            height: (bounds.height - 2.0 * border).max(0.0),
        },
        crate::renderer::ui_theme::CONTROL_RADIUS - border,
        theme.background,
        clip,
    );
}
fn hint_accent(theme: UiTheme, color: [f32; 4]) -> [f32; 4] {
    automexia_ui_model::ensure_contrast(
        color,
        theme.raised,
        automexia_ui_model::MIN_TEXT_CONTRAST + 0.1,
    )
}

fn action_button(
    canvas: &mut impl Canvas,
    bounds: Rect,
    text: (&str, &str),
    font: f32,
    state: (bool, bool),
    theme: UiTheme,
    clip: Rect,
) {
    control(canvas, bounds, state.0, theme, clip);
    let font = font * 0.9;
    let opts = DrawOpts {
        font_size: font * 0.8,
        bold: true,
        ..DrawOpts::default()
    };
    let key_width = canvas.text().measure(text.1, &opts) + 12.0;
    let caption_width = (bounds.width - key_width - 20.0).max(0.0);
    let mut caption = text.0;
    let caption_opts = DrawOpts {
        font_size: font,
        ..DrawOpts::default()
    };
    if canvas.text().measure(caption, &caption_opts) + 8.0 > caption_width {
        caption = match caption {
            "Reset all" | "Reset default" | "Reset tag" | "Reset colors" => "Reset",
            "Restore saved" => "Restore",
            other => other,
        };
    }
    let measured = canvas.text().measure(caption, &caption_opts) + 8.0;
    let caption_font = (font * (caption_width / measured).min(1.0))
        .max(9.0)
        .min(font);
    label(
        canvas,
        centered_label(
            Rect {
                x: bounds.x + 6.0,
                width: caption_width,
                ..bounds
            },
            caption_font,
        ),
        caption,
        caption_font,
        if state.1 {
            theme.text
        } else {
            theme.muted_text
        },
        false,
        clip,
    );
    let key = Rect {
        x: bounds.x + (bounds.width - key_width - 7.0).max(0.0),
        y: bounds.y + (bounds.height - font * 1.4) * 0.5,
        width: key_width.min(bounds.width),
        height: font * 1.4,
    };
    rounded_fill(
        canvas,
        key,
        crate::renderer::ui_theme::KEYCAP_RADIUS,
        theme.surface,
        clip,
    );
    label(
        canvas,
        centered_label(key, font * 0.8),
        text.1,
        font * 0.8,
        if state.1 {
            hint_accent(theme, crate::renderer::ui_theme::BRAND_CYAN)
        } else {
            theme.muted_text
        },
        true,
        clip,
    );
}

// Only call with UI-owned hints; save errors and other feedback stay ordinary text.
fn shortcut_hint(
    canvas: &mut impl Canvas,
    bounds: Rect,
    value: &str,
    font: f32,
    theme: UiTheme,
    clip: Rect,
) {
    let mut remaining = bounds;
    let mut run = |text: &str, color: [f32; 4], bold: bool| {
        label(canvas, remaining, text, font, color, bold, clip);
        let width = canvas.text().measure(
            text,
            &DrawOpts {
                font_size: font,
                bold,
                ..DrawOpts::default()
            },
        );
        remaining.x += width;
        remaining.width = (remaining.width - width).max(0.0);
    };
    for (index, part) in value.split(" | ").enumerate() {
        if index > 0 {
            run(" | ", theme.muted_text, false);
        }
        if let Some((key, action)) = part.split_once(':') {
            let accent = hint_accent(theme, crate::renderer::ui_theme::BRAND_CYAN);
            run(key, accent, true);
            run(":", accent, true);
            run(action, theme.text, false);
        } else if part == "Saved" {
            run(
                part,
                hint_accent(theme, crate::renderer::ui_theme::BRAND_LIME),
                false,
            );
        } else {
            run(part, theme.text, false);
        }
    }
}

fn label(
    canvas: &mut impl Canvas,
    bounds: Rect,
    value: &str,
    font: f32,
    color: [f32; 4],
    bold: bool,
    clip: Rect,
) {
    if let Some(clip) = bounds.intersect(clip) {
        let opts = DrawOpts {
            font_size: font,
            color: color_u8(color),
            bold,
            ..DrawOpts::default()
        };
        canvas.text().draw_clipped(
            bounds.x + 4.0,
            bounds.y + 2.0,
            value,
            &opts,
            clip.array(),
        );
    }
}

#[cfg(test)]
#[path = "settings_view_tests.rs"]
mod tests;
