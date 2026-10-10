//! Bounded, renderer-independent information-bar recipes over admitted context.

use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};

use automexia_extension_api::{DetailsAction, Freshness, IconKind, SegmentRole};
use serde::{Deserialize, Serialize};

mod shapes;
pub use shapes::{
    connected_tag_geometry, tag_join_overlap, ConnectedTagGeometry, TagShapePosition,
};

use crate::Segment;

/// The terminal's prompt-band packer admits at most this many logical items.
pub const MAX_BAR_SLOTS: usize = 16;
pub const MAX_BAR_ITEM_BYTES: usize = 1024;
pub const MAX_BAR_LITERAL_BYTES: usize = 128;
pub const MAX_BAR_AFFIX_BYTES: usize = 24;
pub const MAX_BAR_SLOT_ID_BYTES: usize = 32;
pub const MAX_BAR_RECIPE_TEXT_BYTES: usize = 2048;
pub const MAX_BAR_SOURCE_DRAFT_BYTES: usize =
    MAX_BAR_RECIPE_TEXT_BYTES + MAX_BAR_SLOTS * MAX_BAR_SLOT_ID_BYTES;
pub const MAX_BAR_SPACING_PERCENT: u16 = 300;

const fn default_bar_spacing_percent() -> u16 {
    100
}

/// Stable names for the twelve user-selectable information-bar formats.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum InformationBarPreset {
    #[default]
    RoundedCapsules,
    MinimalFlat,
    BreadcrumbPath,
    LeftRightSplit,
    TwoLinePrompt,
    HexagonalHoneycomb,
    FloatingCards,
    ThinUnderlineTabs,
    GitFirstDeveloperBar,
    AdaptiveContextualBar,
    TerminalDashboardStrip,
    CompactIconMode,
}

impl InformationBarPreset {
    pub const ALL: [Self; 12] = [
        Self::RoundedCapsules,
        Self::MinimalFlat,
        Self::BreadcrumbPath,
        Self::LeftRightSplit,
        Self::TwoLinePrompt,
        Self::HexagonalHoneycomb,
        Self::FloatingCards,
        Self::ThinUnderlineTabs,
        Self::GitFirstDeveloperBar,
        Self::AdaptiveContextualBar,
        Self::TerminalDashboardStrip,
        Self::CompactIconMode,
    ];

    pub const fn id(self) -> &'static str {
        match self {
            Self::RoundedCapsules => "rounded-capsules",
            Self::MinimalFlat => "minimal-flat",
            Self::BreadcrumbPath => "breadcrumb-path",
            Self::LeftRightSplit => "left-right-split",
            Self::TwoLinePrompt => "two-line-prompt",
            Self::HexagonalHoneycomb => "hexagonal-honeycomb",
            Self::FloatingCards => "floating-cards",
            Self::ThinUnderlineTabs => "thin-underline-tabs",
            Self::GitFirstDeveloperBar => "git-first-developer-bar",
            Self::AdaptiveContextualBar => "adaptive-contextual-bar",
            Self::TerminalDashboardStrip => "terminal-dashboard-strip",
            Self::CompactIconMode => "compact-icon-mode",
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::RoundedCapsules => "Rounded capsules",
            Self::MinimalFlat => "Minimal flat",
            Self::BreadcrumbPath => "Breadcrumb path",
            Self::LeftRightSplit => "Left and right split",
            Self::TwoLinePrompt => "Two-line prompt",
            Self::HexagonalHoneycomb => "Hexagonal honeycomb",
            Self::FloatingCards => "Floating cards",
            Self::ThinUnderlineTabs => "Thin underline tabs",
            Self::GitFirstDeveloperBar => "Git-first developer bar",
            Self::AdaptiveContextualBar => "Adaptive contextual bar",
            Self::TerminalDashboardStrip => "Terminal dashboard strip",
            Self::CompactIconMode => "Compact icon mode",
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|preset| preset.id() == id)
    }
}

/// Paint selection; the shared model owns bounded geometry and the renderer
/// owns contrast, clipping, and the native drawing primitives.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BarVisualStyle {
    #[default]
    Capsule,
    Flat,
    Chevron,
    Hexagon,
    Card,
    Underline,
    LinkedArrows,
    Puzzle,
    Slanted,
    PillArrows,
    Ribbon,
    CutCorners,
    Alternating,
    TopNotch,
    Wedges,
}

impl BarVisualStyle {
    pub const ALL: [Self; 15] = [
        Self::Capsule,
        Self::Flat,
        Self::Chevron,
        Self::Hexagon,
        Self::Card,
        Self::Underline,
        Self::LinkedArrows,
        Self::Puzzle,
        Self::Slanted,
        Self::PillArrows,
        Self::Ribbon,
        Self::CutCorners,
        Self::Alternating,
        Self::TopNotch,
        Self::Wedges,
    ];

    pub fn id(self) -> &'static str {
        match self {
            Self::Capsule => "capsule",
            Self::Flat => "flat",
            Self::Chevron => "chevron",
            Self::Hexagon => "hexagon",
            Self::Card => "card",
            Self::Underline => "underline",
            Self::LinkedArrows => "linked-arrows",
            Self::Puzzle => "puzzle",
            Self::Slanted => "slanted",
            Self::PillArrows => "pill-arrows",
            Self::Ribbon => "ribbon",
            Self::CutCorners => "cut-corners",
            Self::Alternating => "alternating",
            Self::TopNotch => "top-notch",
            Self::Wedges => "wedges",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Capsule => "Capsule",
            Self::Flat => "Flat",
            Self::Chevron => "Chevron",
            Self::Hexagon => "Hexagonal chips",
            Self::Card => "Card",
            Self::Underline => "Underline",
            Self::LinkedArrows => "Soft chevrons",
            Self::Puzzle => "Puzzle joins",
            Self::Slanted => "Slanted tags",
            Self::PillArrows => "Pills and arrows",
            Self::Ribbon => "Folded ribbon",
            Self::CutCorners => "Cut corners",
            Self::Alternating => "Alternating triangles",
            Self::TopNotch => "Top notch",
            Self::Wedges => "Separator wedges",
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|style| style.id() == id)
    }

    pub fn is_legacy(self) -> bool {
        !shapes::is_connected_style(self)
    }
}

/// Surface geometry in local fragment coordinates. Both the terminal painter
/// and the customization preview use this bounded, allocation-free shape.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TagSurfaceGeometry {
    Connected(ConnectedTagGeometry),
    Capsule {
        radius: f32,
        stroke: f32,
    },
    Flat {
        stroke: f32,
    },
    Chevron {
        points: [(f32, f32); 6],
        stroke: f32,
    },
    Hexagon {
        points: [(f32, f32); 6],
        stroke: f32,
    },
    Card {
        radius: f32,
        stroke: f32,
    },
    Underline {
        baseline_y: f32,
        stroke: f32,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TagSurfacePaintLayers {
    pub fill: bool,
    pub outline: bool,
}

/// Plain or zero-opacity tags still show their selected silhouette with a
/// subtle text-color stroke, without introducing a background tint.
pub fn tag_surface_paint_layers(
    style: BarVisualStyle,
    fill_alpha: f32,
) -> TagSurfacePaintLayers {
    let fill = fill_alpha.is_finite() && fill_alpha > 0.0;
    TagSurfacePaintLayers {
        fill: fill && style != BarVisualStyle::Underline,
        outline: !fill
            || matches!(style, BarVisualStyle::Card | BarVisualStyle::Underline),
    }
}

/// Return geometry that fits entirely inside the measured tag fragment.
/// Starting a chevron at its concave notch makes its triangle fan stay inside
/// the intended surface, including when a pane is narrow.
pub fn tag_surface_geometry(
    style: BarVisualStyle,
    width: f32,
    height: f32,
) -> Option<TagSurfaceGeometry> {
    if !width.is_finite() || !height.is_finite() || width <= 1.0 || height <= 1.0 {
        return None;
    }
    let stroke = width.min(height).min(1.0);
    let inset = stroke * 0.5;
    let inner_width = (width - stroke).max(0.0);
    let inner_height = (height - stroke).max(0.0);
    Some(match style {
        BarVisualStyle::Capsule => TagSurfaceGeometry::Capsule {
            radius: (inner_height * 0.5).min(inner_width * 0.5),
            stroke,
        },
        BarVisualStyle::Flat => TagSurfaceGeometry::Flat { stroke },
        BarVisualStyle::Chevron => {
            let tip = (inner_height * 0.28).min(inner_width * 0.15);
            let middle = inset + inner_height * 0.5;
            TagSurfaceGeometry::Chevron {
                points: [
                    (inset + tip, middle),
                    (inset, inset),
                    (inset + inner_width - tip, inset),
                    (inset + inner_width, middle),
                    (inset + inner_width - tip, inset + inner_height),
                    (inset, inset + inner_height),
                ],
                stroke,
            }
        }
        BarVisualStyle::Hexagon => {
            let tip = (inner_height * 0.3).min(inner_width * 0.2);
            let middle = inset + inner_height * 0.5;
            TagSurfaceGeometry::Hexagon {
                points: [
                    (inset + tip, inset),
                    (inset + inner_width - tip, inset),
                    (inset + inner_width, middle),
                    (inset + inner_width - tip, inset + inner_height),
                    (inset + tip, inset + inner_height),
                    (inset, middle),
                ],
                stroke,
            }
        }
        BarVisualStyle::Card => TagSurfaceGeometry::Card {
            radius: (inner_height * 0.18).min(inner_width * 0.2),
            stroke,
        },
        BarVisualStyle::Underline => TagSurfaceGeometry::Underline {
            baseline_y: height - inset,
            stroke,
        },
        _ => TagSurfaceGeometry::Connected(connected_tag_geometry(
            style,
            width,
            height,
            TagShapePosition::default(),
        )?),
    })
}

pub fn tag_fragment_geometry(
    style: BarVisualStyle,
    width: f32,
    height: f32,
    position: TagShapePosition,
) -> Option<TagSurfaceGeometry> {
    if shapes::is_connected_style(style) {
        connected_tag_geometry(style, width, height, position)
            .map(TagSurfaceGeometry::Connected)
    } else {
        tag_surface_geometry(style, width, height)
    }
}

/// Logical-pixel measurements for one information tag at a given terminal
/// row height. Preview and terminal paint use the same sizing formula.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PromptTagMetrics {
    pub font_size: f32,
    pub icon_size: f32,
    pub icon_slot: f32,
    pub height: f32,
    pub padding_x: f32,
    pub icon_gap: f32,
    pub tag_gap: f32,
    pub radius: f32,
}

pub fn prompt_tag_metrics(row_height: f32) -> PromptTagMetrics {
    let row_height = row_height.max(1.0);
    let font_size = (row_height * 0.62).clamp(4.0, 14.0).min(row_height);
    let vertical_padding = (row_height * 0.08).clamp(1.0, 2.0);
    let height = (font_size + vertical_padding * 2.0).min(row_height);
    let padding_x = (font_size * 0.35).clamp(3.0, 6.0);
    let icon_gap = (font_size * 0.28).clamp(2.0, 5.0);
    let tag_gap = (font_size * 0.35).clamp(3.0, 6.0);
    let icon_size = font_size * 1.05;
    let icon_slot = font_size * 1.20;
    let radius = (height * 0.22).clamp(1.0, 5.0).min(height * 0.5);
    PromptTagMetrics {
        font_size,
        icon_size,
        icon_slot,
        height,
        padding_x,
        icon_gap,
        tag_gap,
        radius,
    }
}

/// Layout hints consumed by the shared fragment packer. Adaptive saves space
/// only in narrow panes; all recipes retain their chosen tag order and data.
pub fn bar_layout_hints(
    recipe: &BarRecipe,
    items: &[ResolvedBarItem],
    metrics: PromptTagMetrics,
    available_width: f32,
) -> (f32, f32, Vec<usize>, Option<usize>) {
    let narrow_adaptive = recipe.arrangement == BarArrangement::Adaptive
        && available_width.is_finite()
        && available_width < metrics.font_size * 30.0;
    let padding = match recipe.visual {
        BarVisualStyle::Capsule => metrics.padding_x,
        BarVisualStyle::Flat | BarVisualStyle::Underline => metrics.padding_x * 0.7,
        BarVisualStyle::Chevron | BarVisualStyle::Hexagon => metrics.padding_x + 3.0,
        BarVisualStyle::Card => metrics.padding_x + 2.0,
        _ => metrics.padding_x + shapes::shape_depth(recipe.visual, metrics.height),
    };
    let format_gap = match recipe.arrangement {
        BarArrangement::Dashboard => 1.0,
        BarArrangement::Compact => 2.0,
        BarArrangement::Adaptive if narrow_adaptive => 0.5,
        _ => match recipe.visual {
            BarVisualStyle::Flat => 1.0,
            BarVisualStyle::Underline => metrics.tag_gap + 2.0,
            BarVisualStyle::Card => metrics.tag_gap + 3.0,
            _ => metrics.tag_gap,
        },
    };
    let gap = format_gap * f32::from(recipe.spacing_percent) / 100.0;
    let trailing_start = items
        .iter()
        .position(|item| item.lane == BarLane::Trailing)
        .filter(|index| *index > 0 && *index < items.len());
    let (break_before, trailing_start) = match recipe.arrangement {
        BarArrangement::Split => (Vec::new(), trailing_start),
        BarArrangement::TwoLine if items.len() > 1 => (
            vec![trailing_start.unwrap_or(items.len().div_ceil(2))],
            None,
        ),
        _ => (Vec::new(), None),
    };
    let padding = if recipe.arrangement == BarArrangement::Compact || narrow_adaptive {
        if shapes::is_connected_style(recipe.visual) {
            padding
        } else {
            padding.min(2.0)
        }
    } else {
        padding
    };
    (padding, gap, break_before, trailing_start)
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BarArrangement {
    #[default]
    Flow,
    Split,
    TwoLine,
    Adaptive,
    Dashboard,
    Compact,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BarLane {
    #[default]
    Leading,
    Trailing,
}

/// Literal text is display-only. It cannot name a provider, run a command, or
/// interpolate raw shell/session fields. `None` is reserved for icon-only slots.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BarTextSource {
    Role(SegmentRole),
    Literal(String),
    None,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BarIconSource {
    TextSource,
    Role(SegmentRole),
    Fixed(IconKind),
    None,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct BarSlot {
    pub id: String,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    pub text: BarTextSource,
    pub icon: BarIconSource,
    pub lane: BarLane,
    /// Desired anchor only; paint must correct it for the actual tag surface.
    pub color: Option<[u8; 3]>,
    pub prefix: String,
    pub suffix: String,
}

/// Inactive editor choices for one slot. These values never resolve context
/// or affect paint until a user explicitly selects their corresponding mode.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub struct BarSlotSourceDraft {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub literal: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_role: Option<SegmentRole>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon_role: Option<SegmentRole>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fixed_icon: Option<IconKind>,
    /// Original icon mode while icon-only text temporarily requires a source.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub coerced_icon: Option<BarIconSource>,
}

impl BarSlotSourceDraft {
    pub fn is_empty(&self) -> bool {
        self.literal.is_none()
            && self.text_role.is_none()
            && self.icon_role.is_none()
            && self.fixed_icon.is_none()
            && self.coerced_icon.is_none()
    }
}

const fn default_enabled() -> bool {
    true
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct BarRecipe {
    pub visual: BarVisualStyle,
    pub arrangement: BarArrangement,
    /// Percentage of the selected format's native gap between adjacent tags.
    #[serde(default = "default_bar_spacing_percent")]
    pub spacing_percent: u16,
    pub slots: Vec<BarSlot>,
}

/// Effective host admission, separate from persisted slot choices. A missing
/// extension disables both features; switching discovery off can keep Git on.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BarContextAvailability {
    pub devops: bool,
    pub git: bool,
}

impl BarContextAvailability {
    pub fn role_available(self, role: SegmentRole) -> bool {
        if role == SegmentRole::Git {
            self.git
        } else {
            !devops_role(role) || self.devops
        }
    }

    fn slot_available(self, slot: &BarSlot) -> bool {
        role_from_id(&slot.id).is_none_or(|role| self.role_available(role))
            && !matches!(slot.text, BarTextSource::Role(role) if !self.role_available(role))
            && !matches!(slot.icon, BarIconSource::Role(role) if !self.role_available(role))
    }
}

impl BarRecipe {
    /// Project visibility without changing saved slot choices. Both terminal
    /// bands and the editor use this gate, including literal and icon-only tags.
    pub fn with_devops_context(&self, enabled: bool) -> Cow<'_, Self> {
        self.with_context(BarContextAvailability {
            devops: enabled,
            git: true,
        })
    }

    pub fn with_context(&self, availability: BarContextAvailability) -> Cow<'_, Self> {
        if (availability.devops && availability.git)
            || self
                .slots
                .iter()
                .all(|slot| availability.slot_available(slot))
        {
            return Cow::Borrowed(self);
        }
        let mut visible = self.clone();
        visible
            .slots
            .retain(|slot| availability.slot_available(slot));
        Cow::Owned(visible)
    }

    /// The editor also lists disabled/default slots absent from a preset.
    pub fn slot_visible_with_devops(&self, id: &str, enabled: bool) -> bool {
        self.slot_available(
            id,
            BarContextAvailability {
                devops: enabled,
                git: true,
            },
        )
    }

    pub fn slot_available(&self, id: &str, availability: BarContextAvailability) -> bool {
        self.slots.iter().find(|slot| slot.id == id).map_or_else(
            || role_from_id(id).is_none_or(|role| availability.role_available(role)),
            |slot| availability.slot_available(slot),
        )
    }
}

fn devops_role(role: SegmentRole) -> bool {
    !matches!(
        role,
        SegmentRole::UbuntuWsl
            | SegmentRole::Windows
            | SegmentRole::Git
            | SegmentRole::User
    )
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BarRecipeError {
    Empty,
    TooManySlots,
    TooLarge,
    InvalidSlotId,
    DuplicateSlotId,
    InvalidText,
    InvalidIconOnly,
    InvalidSpacing,
}

/// One ready-to-pack display item, retaining only the admitted source's
/// semantics. A fixed icon never changes the text source's identity or action.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedBarItem {
    pub slot_id: String,
    pub value: String,
    pub icon_only: bool,
    pub accessibility_label: String,
    pub source_role: Option<SegmentRole>,
    pub icon: Option<IconKind>,
    pub lane: BarLane,
    pub color: Option<[u8; 3]>,
    /// Only meaningful as provider state when `source_role` is present.
    pub freshness: Freshness,
    pub observed_at_ms: u64,
    pub details_action: Option<DetailsAction>,
}

pub const STANDARD_ROLES: [SegmentRole; 13] = [
    SegmentRole::Production,
    SegmentRole::UbuntuWsl,
    SegmentRole::Windows,
    SegmentRole::Git,
    SegmentRole::Kubernetes,
    SegmentRole::Docker,
    SegmentRole::Azure,
    SegmentRole::Aws,
    SegmentRole::Gcp,
    SegmentRole::UnknownCloud,
    SegmentRole::Terraform,
    SegmentRole::Environment,
    SegmentRole::User,
];

pub const fn role_id(role: SegmentRole) -> &'static str {
    match role {
        SegmentRole::Production => "production",
        SegmentRole::UbuntuWsl => "ubuntu-wsl",
        SegmentRole::Windows => "windows",
        SegmentRole::Git => "git",
        SegmentRole::Kubernetes => "kubernetes",
        SegmentRole::Docker => "docker",
        SegmentRole::Azure => "azure",
        SegmentRole::Aws => "aws",
        SegmentRole::Gcp => "gcp",
        SegmentRole::UnknownCloud => "unknown-cloud",
        SegmentRole::Terraform => "terraform",
        SegmentRole::Environment => "environment",
        SegmentRole::User => "user",
    }
}

pub fn role_from_id(id: &str) -> Option<SegmentRole> {
    STANDARD_ROLES.into_iter().find(|role| role_id(*role) == id)
}

pub const fn role_label(role: SegmentRole) -> &'static str {
    match role {
        SegmentRole::Production => "Production",
        SegmentRole::UbuntuWsl => "Operating system",
        SegmentRole::Windows => "Windows",
        SegmentRole::Git => "Git",
        SegmentRole::Kubernetes => "Kubernetes",
        SegmentRole::Docker => "Docker",
        SegmentRole::Azure => "Azure",
        SegmentRole::Aws => "AWS",
        SegmentRole::Gcp => "Google Cloud",
        SegmentRole::UnknownCloud => "Other cloud",
        SegmentRole::Terraform => "Terraform",
        SegmentRole::Environment => "Environment",
        SegmentRole::User => "User",
    }
}

fn fixed_icon_name(icon: IconKind) -> &'static str {
    match icon {
        IconKind::Wsl => "WSL icon",
        IconKind::Windows => "Windows icon",
        IconKind::Docker => "Docker icon",
        IconKind::Kubernetes => "Kubernetes icon",
        IconKind::Cloud => "Cloud icon",
        IconKind::Terraform => "Terraform icon",
        IconKind::Git => "Git icon",
        IconKind::Environment => "Environment icon",
        IconKind::User => "User icon",
        IconKind::Production => "Production icon",
    }
}

/// Presets describe distinct presentation choices over the same admitted
/// context. They do not invent the paths, metrics, SSH facts or Git change
/// counts pictured in reference artwork; unavailable roles simply disappear.
pub fn preset_recipe(preset: InformationBarPreset) -> BarRecipe {
    let mut roles = STANDARD_ROLES.to_vec();
    if preset == InformationBarPreset::GitFirstDeveloperBar {
        roles.retain(|role| *role != SegmentRole::Git);
        roles.insert(0, SegmentRole::Git);
    } else if preset == InformationBarPreset::AdaptiveContextualBar {
        roles = vec![
            SegmentRole::Production,
            SegmentRole::Environment,
            SegmentRole::Git,
            SegmentRole::Kubernetes,
            SegmentRole::Docker,
            SegmentRole::Azure,
            SegmentRole::Aws,
            SegmentRole::Gcp,
            SegmentRole::UnknownCloud,
            SegmentRole::Terraform,
            SegmentRole::UbuntuWsl,
            SegmentRole::Windows,
            SegmentRole::User,
        ];
    }
    let visual = match preset {
        InformationBarPreset::RoundedCapsules | InformationBarPreset::LeftRightSplit => {
            BarVisualStyle::Capsule
        }
        InformationBarPreset::MinimalFlat
        | InformationBarPreset::TwoLinePrompt
        | InformationBarPreset::AdaptiveContextualBar
        | InformationBarPreset::CompactIconMode => BarVisualStyle::Flat,
        InformationBarPreset::BreadcrumbPath
        | InformationBarPreset::GitFirstDeveloperBar => BarVisualStyle::Chevron,
        InformationBarPreset::HexagonalHoneycomb => BarVisualStyle::Hexagon,
        InformationBarPreset::FloatingCards
        | InformationBarPreset::TerminalDashboardStrip => BarVisualStyle::Card,
        InformationBarPreset::ThinUnderlineTabs => BarVisualStyle::Underline,
    };
    let arrangement = match preset {
        InformationBarPreset::LeftRightSplit => BarArrangement::Split,
        InformationBarPreset::TwoLinePrompt => BarArrangement::TwoLine,
        InformationBarPreset::AdaptiveContextualBar => BarArrangement::Adaptive,
        InformationBarPreset::TerminalDashboardStrip => BarArrangement::Dashboard,
        InformationBarPreset::CompactIconMode => BarArrangement::Compact,
        _ => BarArrangement::Flow,
    };
    let slots = roles
        .into_iter()
        .map(|role| {
            let icon_only = preset == InformationBarPreset::CompactIconMode
                && matches!(role, SegmentRole::UbuntuWsl | SegmentRole::Windows);
            let lane = if preset == InformationBarPreset::LeftRightSplit
                && matches!(role, SegmentRole::Environment | SegmentRole::User)
            {
                BarLane::Trailing
            } else {
                BarLane::Leading
            };
            BarSlot {
                id: role_id(role).into(),
                enabled: true,
                text: if icon_only {
                    BarTextSource::None
                } else {
                    BarTextSource::Role(role)
                },
                icon: if icon_only {
                    BarIconSource::Role(role)
                } else {
                    BarIconSource::TextSource
                },
                lane,
                color: None,
                prefix: String::new(),
                suffix: String::new(),
            }
        })
        .collect();
    BarRecipe {
        visual,
        arrangement,
        spacing_percent: default_bar_spacing_percent(),
        slots,
    }
}

fn safe_text(value: &str, maximum: usize, allow_empty: bool) -> bool {
    value.len() <= maximum
        && (allow_empty || !value.trim().is_empty())
        && !value.chars().any(|ch| {
            ch.is_control()
                || matches!(ch, '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
        })
}

fn valid_slot_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= MAX_BAR_SLOT_ID_BYTES
        && id.as_bytes()[0].is_ascii_lowercase()
        && id.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-'
        })
}

/// Bound inactive choices to the same display-only language as active slots.
/// The map is persisted beside recipes so older snapshots remain readable.
pub fn validate_source_drafts(
    drafts: &BTreeMap<String, BarSlotSourceDraft>,
) -> Result<(), BarRecipeError> {
    if drafts.len() > MAX_BAR_SLOTS {
        return Err(BarRecipeError::TooManySlots);
    }
    let mut bytes = 0usize;
    let mut literal_bytes = 0usize;
    for (id, draft) in drafts {
        if !valid_slot_id(id) {
            return Err(BarRecipeError::InvalidSlotId);
        }
        bytes = bytes.saturating_add(id.len());
        if let Some(literal) = &draft.literal {
            if !safe_text(literal, MAX_BAR_LITERAL_BYTES, false) {
                return Err(BarRecipeError::InvalidText);
            }
            literal_bytes = literal_bytes.saturating_add(literal.len());
            bytes = bytes.saturating_add(literal.len());
        }
    }
    if literal_bytes > MAX_BAR_RECIPE_TEXT_BYTES || bytes > MAX_BAR_SOURCE_DRAFT_BYTES {
        return Err(BarRecipeError::TooLarge);
    }
    Ok(())
}

/// Validate before persistence and again at presentation boundaries. No recipe
/// can ask a provider to refresh or read session facts during projection.
pub fn validate_recipe(recipe: &BarRecipe) -> Result<(), BarRecipeError> {
    if recipe.spacing_percent > MAX_BAR_SPACING_PERCENT {
        return Err(BarRecipeError::InvalidSpacing);
    }
    if recipe.slots.is_empty() {
        return Err(BarRecipeError::Empty);
    }
    if recipe.slots.len() > MAX_BAR_SLOTS {
        return Err(BarRecipeError::TooManySlots);
    }
    let mut ids = BTreeSet::new();
    let mut bytes = 0usize;
    for slot in &recipe.slots {
        if !valid_slot_id(&slot.id) {
            return Err(BarRecipeError::InvalidSlotId);
        }
        if !ids.insert(slot.id.as_str()) {
            return Err(BarRecipeError::DuplicateSlotId);
        }
        if !safe_text(&slot.prefix, MAX_BAR_AFFIX_BYTES, true)
            || !safe_text(&slot.suffix, MAX_BAR_AFFIX_BYTES, true)
        {
            return Err(BarRecipeError::InvalidText);
        }
        bytes =
            bytes.saturating_add(slot.id.len() + slot.prefix.len() + slot.suffix.len());
        match &slot.text {
            BarTextSource::Role(_) => {}
            BarTextSource::Literal(value) => {
                if !safe_text(value, MAX_BAR_LITERAL_BYTES, false) {
                    return Err(BarRecipeError::InvalidText);
                }
                bytes = bytes.saturating_add(value.len());
            }
            BarTextSource::None => {
                if !matches!(slot.icon, BarIconSource::Role(_) | BarIconSource::Fixed(_))
                    || !slot.prefix.is_empty()
                    || !slot.suffix.is_empty()
                {
                    return Err(BarRecipeError::InvalidIconOnly);
                }
            }
        }
    }
    if bytes > MAX_BAR_RECIPE_TEXT_BYTES {
        return Err(BarRecipeError::TooLarge);
    }
    Ok(())
}

fn admitted_segment(segments: &[Segment], role: SegmentRole) -> Option<&Segment> {
    segments.iter().find(|segment| {
        segment.role == role
            && safe_text(&segment.value, MAX_BAR_ITEM_BYTES, true)
            && safe_text(&segment.accessibility_label, MAX_BAR_ITEM_BYTES, false)
    })
}

fn untouched_canonical_slot(
    slot: &BarSlot,
    role: SegmentRole,
    arrangement: BarArrangement,
) -> bool {
    let default_lane = if arrangement == BarArrangement::Split
        && matches!(role, SegmentRole::Environment | SegmentRole::User)
    {
        BarLane::Trailing
    } else {
        BarLane::Leading
    };
    let default_source = matches!((&slot.text, &slot.icon),
        (BarTextSource::Role(text), BarIconSource::TextSource) if *text == role)
        || matches!((&slot.text, &slot.icon),
            (BarTextSource::None, BarIconSource::Role(icon)) if *icon == role);
    slot.id == role_id(role)
        && default_source
        && slot.lane == default_lane
        && slot.color.is_none()
        && slot.prefix.is_empty()
        && slot.suffix.is_empty()
}

/// Resolve only values already projected for this prompt/session. Unknown roles
/// are omitted; a missing optional icon never silently substitutes a fake one.
pub fn resolve_recipe(
    recipe: &BarRecipe,
    segments: &[Segment],
) -> Result<Vec<ResolvedBarItem>, BarRecipeError> {
    validate_recipe(recipe)?;
    let mut resolved = Vec::with_capacity(recipe.slots.len());
    for slot in &recipe.slots {
        if !slot.enabled {
            continue;
        }
        let text_segment = match slot.text {
            BarTextSource::Role(role) => admitted_segment(segments, role),
            _ => None,
        };
        if matches!(slot.text, BarTextSource::Role(_))
            && text_segment.is_none_or(|segment| segment.value.trim().is_empty())
        {
            continue;
        }
        let icon_segment = match slot.icon {
            BarIconSource::Role(role) => admitted_segment(segments, role),
            _ => None,
        };
        if matches!(slot.text, BarTextSource::None)
            && matches!(slot.icon, BarIconSource::Role(_))
            && icon_segment.is_none()
        {
            continue;
        }
        let (value, accessible, semantic) = match &slot.text {
            BarTextSource::Role(_) => {
                let Some(segment) = text_segment else {
                    continue;
                };
                (
                    format!("{}{}{}", slot.prefix, segment.value, slot.suffix),
                    format!(
                        "{}{}{}",
                        slot.prefix, segment.accessibility_label, slot.suffix
                    ),
                    Some(segment),
                )
            }
            BarTextSource::Literal(literal) => (
                format!("{}{}{}", slot.prefix, literal, slot.suffix),
                format!("{}{}{}", slot.prefix, literal, slot.suffix),
                None,
            ),
            BarTextSource::None => (
                String::new(),
                match slot.icon {
                    BarIconSource::Fixed(icon) => fixed_icon_name(icon).into(),
                    _ => icon_segment.map_or_else(String::new, |segment| {
                        segment.accessibility_label.clone()
                    }),
                },
                icon_segment,
            ),
        };
        if !safe_text(
            &value,
            MAX_BAR_ITEM_BYTES,
            matches!(slot.text, BarTextSource::None),
        ) || !safe_text(&accessible, MAX_BAR_ITEM_BYTES, false)
        {
            continue;
        }
        let icon = match slot.icon {
            BarIconSource::TextSource => text_segment.map(|segment| segment.icon),
            BarIconSource::Role(_) => icon_segment.map(|segment| segment.icon),
            BarIconSource::Fixed(icon) => Some(icon),
            BarIconSource::None => None,
        };
        resolved.push(ResolvedBarItem {
            slot_id: slot.id.clone(),
            value,
            icon_only: matches!(slot.text, BarTextSource::None),
            accessibility_label: accessible,
            source_role: semantic.map(|segment| segment.role),
            icon,
            lane: slot.lane,
            color: slot.color,
            freshness: semantic.map_or(Freshness::Current, |segment| segment.freshness),
            observed_at_ms: semantic.map_or(0, |segment| segment.observed_at_ms),
            details_action: semantic.and_then(|segment| segment.details_action.clone()),
        });
    }
    // Presets keep one dormant slot per possible role. If a user repurposes
    // another slot to that role, the untouched preset slot must not appear as
    // a second copy when the role becomes available. An explicitly styled or
    // repositioned canonical slot remains available for deliberate repetition.
    let repurposed_roles: Vec<_> = resolved
        .iter()
        .filter_map(|item| {
            let role = item.source_role?;
            (item.slot_id != role_id(role)).then_some(role)
        })
        .collect();
    if !repurposed_roles.is_empty() {
        resolved.retain(|item| {
            let Some(role) = item.source_role else {
                return true;
            };
            !repurposed_roles.contains(&role)
                || recipe
                    .slots
                    .iter()
                    .find(|slot| slot.id == item.slot_id)
                    .is_none_or(|slot| {
                        !untouched_canonical_slot(slot, role, recipe.arrangement)
                    })
        });
    }
    Ok(resolved)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Segment;
    use automexia_extension_api::{DetailsAction, Freshness, IconKind, SegmentRole};

    fn segment(role: SegmentRole, value: &str, icon: IconKind) -> Segment {
        Segment {
            value: value.into(),
            accessibility_label: format!("{role:?} {value}"),
            role,
            icon,
            priority: 10,
            freshness: Freshness::Current,
            observed_at_ms: 4,
            details_action: None,
        }
    }

    fn slot(text: BarTextSource, icon: BarIconSource) -> BarSlot {
        BarSlot {
            id: "identity".into(),
            enabled: true,
            text,
            icon,
            lane: BarLane::Leading,
            color: None,
            prefix: String::new(),
            suffix: String::new(),
        }
    }

    #[test]
    fn devops_membership_projection_gates_git_custom_sources_and_preserves_recipes() {
        for preset in InformationBarPreset::ALL {
            let mut recipe = preset_recipe(preset);
            for (id, text, icon) in [
                (
                    "custom-1",
                    BarTextSource::Literal("team".into()),
                    BarIconSource::None,
                ),
                (
                    "custom-2",
                    BarTextSource::Role(SegmentRole::Git),
                    BarIconSource::None,
                ),
                (
                    "custom-3",
                    BarTextSource::Literal("cluster".into()),
                    BarIconSource::Role(SegmentRole::Kubernetes),
                ),
            ] {
                let mut custom = slot(text, icon);
                custom.id = id.into();
                recipe.slots.push(custom);
            }
            let saved = recipe.clone();
            for (devops, git) in [
                (true, true),
                (false, true),
                (false, false),
                (true, false),
                (true, true),
            ] {
                let availability = BarContextAvailability { devops, git };
                let visible = recipe.with_context(availability);
                for entry in &visible.slots {
                    assert!(
                        !matches!(entry.text, BarTextSource::Role(SegmentRole::Git))
                            || git
                    );
                    assert!(
                        !matches!(
                            entry.icon,
                            BarIconSource::Role(SegmentRole::Kubernetes)
                        ) || devops
                    );
                }
                assert!(visible.slots.iter().any(|entry| entry.id == "custom-1"));
                assert_eq!(
                    visible.slots.iter().any(|entry| entry.id == "custom-2"),
                    git
                );
                assert_eq!(
                    visible.slots.iter().any(|entry| entry.id == "custom-3"),
                    devops
                );
                assert_eq!(recipe.slot_available("git", availability), git);
                assert_eq!(recipe.slot_available("kubernetes", availability), devops);
                assert!(recipe.slot_available("user", availability));
                assert_eq!(recipe, saved);
                if devops && git {
                    assert!(matches!(visible, Cow::Borrowed(_)));
                }
            }
        }
    }

    #[test]
    fn devops_visibility_preserves_every_preset_and_individual_choice() {
        for preset in InformationBarPreset::ALL {
            let mut recipe = preset_recipe(preset);
            for slot in &mut recipe.slots {
                if slot.id == "terraform" {
                    slot.enabled = false;
                    slot.color = Some([17, 43, 91]);
                }
            }
            let saved = recipe.clone();
            for enabled in [true, false, true, false, true] {
                let visible = recipe.with_devops_context(enabled);
                if enabled {
                    assert_eq!(*visible, saved);
                    assert!(matches!(visible, Cow::Borrowed(_)));
                } else {
                    assert!(visible.slots.iter().all(|slot| matches!(
                        slot.id.as_str(),
                        "ubuntu-wsl" | "windows" | "git" | "user"
                    )));
                    for id in [
                        "production",
                        "kubernetes",
                        "docker",
                        "azure",
                        "aws",
                        "gcp",
                        "unknown-cloud",
                        "terraform",
                        "environment",
                    ] {
                        assert!(!recipe.slot_visible_with_devops(id, false));
                    }
                }
                assert_eq!(recipe, saved, "visibility must not edit the stored recipe");
            }
        }
    }

    #[test]
    fn devops_visibility_covers_custom_sources_and_keeps_independent_literals() {
        let mut recipe = preset_recipe(Default::default());
        recipe.slots = vec![
            slot(
                BarTextSource::Literal("team".into()),
                BarIconSource::Fixed(IconKind::Kubernetes),
            ),
            slot(
                BarTextSource::Role(SegmentRole::Kubernetes),
                BarIconSource::TextSource,
            ),
            slot(BarTextSource::None, BarIconSource::Role(SegmentRole::Aws)),
            slot(
                BarTextSource::Literal("cluster".into()),
                BarIconSource::Role(SegmentRole::Docker),
            ),
            slot(
                BarTextSource::Literal("workspace".into()),
                BarIconSource::None,
            ),
        ];
        for (entry, id) in recipe.slots.iter_mut().zip([
            "custom-1",
            "custom-2",
            "custom-3",
            "windows",
            "terraform",
        ]) {
            entry.id = id.into();
        }
        let visible = recipe.with_devops_context(false);
        assert_eq!(visible.slots.len(), 1);
        assert_eq!(visible.slots[0].id, "custom-1");
        assert_eq!(resolve_recipe(&visible, &[]).unwrap()[0].value, "team");
        assert_eq!(*recipe.with_devops_context(true), recipe);
        assert_eq!(recipe.slots.len(), 5);
        assert!(recipe.slot_visible_with_devops("custom-1", false));
        assert!(!recipe.slot_visible_with_devops("custom-2", false));
        assert!(!recipe.slot_visible_with_devops("custom-3", false));
        assert!(!recipe.slot_visible_with_devops("windows", false));
        assert!(!recipe.slot_visible_with_devops("terraform", false));
    }

    #[test]
    fn repurposed_slot_supersedes_an_untouched_default_for_the_same_source() {
        for source in [
            SegmentRole::Production,
            SegmentRole::Azure,
            SegmentRole::Aws,
            SegmentRole::Gcp,
        ] {
            let mut recipe = preset_recipe(InformationBarPreset::RoundedCapsules);
            let windows = recipe
                .slots
                .iter_mut()
                .find(|slot| slot.id == "windows")
                .unwrap();
            windows.text = BarTextSource::Role(source);
            let resolved = resolve_recipe(
                &recipe,
                &[
                    segment(SegmentRole::Windows, "Windows", IconKind::Windows),
                    segment(source, "sample", IconKind::Cloud),
                ],
            )
            .unwrap();
            let matching: Vec<_> = resolved
                .iter()
                .filter(|item| item.source_role == Some(source))
                .collect();
            assert_eq!(matching.len(), 1, "{source:?} appears only once");
            assert_eq!(matching[0].slot_id, "windows");
        }
    }

    #[test]
    fn separately_customized_canonical_slot_can_intentionally_repeat_a_source() {
        let mut recipe = preset_recipe(InformationBarPreset::RoundedCapsules);
        recipe
            .slots
            .iter_mut()
            .find(|slot| slot.id == "windows")
            .unwrap()
            .text = BarTextSource::Role(SegmentRole::Aws);
        recipe
            .slots
            .iter_mut()
            .find(|slot| slot.id == "aws")
            .unwrap()
            .color = Some([20, 30, 40]);
        let resolved = resolve_recipe(
            &recipe,
            &[segment(SegmentRole::Aws, "sample", IconKind::Cloud)],
        )
        .unwrap();
        assert_eq!(
            resolved
                .iter()
                .filter(|item| item.source_role == Some(SegmentRole::Aws))
                .count(),
            2
        );
    }

    fn recipe(slots: Vec<BarSlot>) -> BarRecipe {
        BarRecipe {
            visual: BarVisualStyle::Capsule,
            arrangement: BarArrangement::Flow,
            spacing_percent: default_bar_spacing_percent(),
            slots,
        }
    }

    #[test]
    fn disabled_slot_retains_sources_without_publishing_a_tag() {
        let mut hidden = slot(
            BarTextSource::Role(SegmentRole::User),
            BarIconSource::Fixed(IconKind::Windows),
        );
        hidden.enabled = false;
        let recipe = recipe(vec![hidden.clone()]);
        assert!(validate_recipe(&recipe).is_ok());
        assert!(resolve_recipe(
            &recipe,
            &[segment(SegmentRole::User, "example", IconKind::User)]
        )
        .unwrap()
        .is_empty());
        let restored = BarRecipe {
            slots: vec![BarSlot {
                enabled: true,
                ..hidden
            }],
            ..recipe
        };
        assert_eq!(
            resolve_recipe(
                &restored,
                &[segment(SegmentRole::User, "example", IconKind::User)]
            )
            .unwrap()
            .len(),
            1
        );
    }

    #[test]
    fn windows_icon_can_show_sanitized_username_without_changing_its_semantics() {
        let sources = [
            segment(SegmentRole::Windows, "Windows", IconKind::Windows),
            segment(SegmentRole::User, "alice", IconKind::User),
        ];
        let result = resolve_recipe(
            &recipe(vec![slot(
                BarTextSource::Role(SegmentRole::User),
                BarIconSource::Fixed(IconKind::Windows),
            )]),
            &sources,
        )
        .unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].value, "alice");
        assert_eq!(result[0].icon, Some(IconKind::Windows));
        assert_eq!(result[0].source_role, Some(SegmentRole::User));
        assert_eq!(result[0].accessibility_label, "User alice");
        assert_eq!(result[0].observed_at_ms, 4);
    }

    #[test]
    fn real_session_projection_swaps_windows_icon_and_user_text_across_shells() {
        let mut session = automexia_extension_api::SessionFacts {
            session_id: 7,
            cwd: None,
            title: "PowerShell".into(),
            distro: None,
            os_name: Some("Windows".into()),
            os_version: None,
            shell_name: Some("PowerShell".into()),
            shell_user: Some("alice".into()),
            shell_path: None,
            environment: Default::default(),
            shell_integration: true,
            shell_pid: 0,
        };
        let configured = recipe(vec![slot(
            BarTextSource::Role(SegmentRole::User),
            BarIconSource::Role(SegmentRole::Windows),
        )]);
        let windows =
            resolve_recipe(&configured, &crate::immediate_session_segments(&session))
                .unwrap();
        assert_eq!(windows.len(), 1);
        assert_eq!(windows[0].value, "alice");
        assert_eq!(windows[0].icon, Some(IconKind::Windows));
        assert_eq!(windows[0].source_role, Some(SegmentRole::User));
        session.shell_name = Some("zsh".into());
        session.os_name = Some("macOS".into());
        let unix =
            resolve_recipe(&configured, &crate::immediate_session_segments(&session))
                .unwrap();
        assert_eq!(unix.len(), 1);
        assert_eq!(unix[0].value, "alice");
        assert_eq!(unix[0].icon, None);
    }

    #[test]
    fn missing_context_omits_items_and_icon_only_uses_real_source_semantics() {
        let mut git = segment(SegmentRole::Git, "main", IconKind::Git);
        git.accessibility_label = "Git branch main".into();
        git.freshness = Freshness::Stale;
        git.details_action = Some(DetailsAction::show("devops.git").unwrap());
        let mut missing = slot(
            BarTextSource::Role(SegmentRole::Kubernetes),
            BarIconSource::TextSource,
        );
        missing.id = "missing".into();
        let mut icon_only =
            slot(BarTextSource::None, BarIconSource::Role(SegmentRole::Git));
        icon_only.id = "git-icon".into();
        let mut no_icon = slot(
            BarTextSource::Role(SegmentRole::Git),
            BarIconSource::Role(SegmentRole::Docker),
        );
        no_icon.id = "git-text".into();
        let result =
            resolve_recipe(&recipe(vec![missing, icon_only, no_icon]), &[git]).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].value, "");
        assert!(result[0].icon_only);
        assert_eq!(result[0].icon, Some(IconKind::Git));
        assert_eq!(result[0].accessibility_label, "Git branch main");
        assert_eq!(result[0].freshness, Freshness::Stale);
        assert!(result[0].details_action.is_some());
        assert_eq!(result[1].value, "main");
        assert_eq!(result[1].icon, None);
        assert_eq!(result[1].source_role, Some(SegmentRole::Git));
    }

    #[test]
    fn all_twelve_preset_ids_are_stable_unique_and_resolve_without_fake_values() {
        let mut ids = std::collections::BTreeSet::new();
        let mut signatures = std::collections::BTreeSet::new();
        assert_eq!(InformationBarPreset::ALL.len(), 12);
        assert_eq!(InformationBarPreset::default().id(), "rounded-capsules");
        for preset in InformationBarPreset::ALL {
            assert!(ids.insert(preset.id()));
            assert_eq!(InformationBarPreset::from_id(preset.id()), Some(preset));
            assert_eq!(
                serde_json::to_string(&preset).unwrap(),
                format!("\"{}\"", preset.id())
            );
            let design = preset_recipe(preset);
            validate_recipe(&design).unwrap();
            assert!(!design.slots.is_empty());
            assert!(resolve_recipe(&design, &[]).unwrap().is_empty());
            assert!(signatures.insert(format!("{design:?}")));
        }
        assert_eq!(InformationBarPreset::from_id("unknown"), None);
    }

    #[test]
    fn custom_recipe_rejects_unsafe_and_oversized_configuration() {
        let mut duplicate = recipe(vec![
            slot(BarTextSource::Literal("one".into()), BarIconSource::None),
            slot(BarTextSource::Literal("two".into()), BarIconSource::None),
        ]);
        assert!(validate_recipe(&duplicate).is_err());
        duplicate.slots[1].id = "second".into();
        for value in ["bad\ntext", "bad\u{202e}text", "bad\u{2066}text"] {
            duplicate.slots[0].text = BarTextSource::Literal(value.into());
            assert!(validate_recipe(&duplicate).is_err());
            duplicate.slots[0].text = BarTextSource::Literal("ok".into());
            duplicate.slots[0].prefix = value.into();
            assert!(validate_recipe(&duplicate).is_err());
            duplicate.slots[0].prefix.clear();
            duplicate.slots[0].suffix = value.into();
            assert!(validate_recipe(&duplicate).is_err());
            duplicate.slots[0].suffix.clear();
        }
        duplicate.slots[0].text =
            BarTextSource::Literal("x".repeat(MAX_BAR_LITERAL_BYTES + 1));
        assert!(validate_recipe(&duplicate).is_err());
        let many = recipe(
            (0..=MAX_BAR_SLOTS)
                .map(|n| {
                    let mut entry =
                        slot(BarTextSource::Literal("x".into()), BarIconSource::None);
                    entry.id = format!("s{n}");
                    entry
                })
                .collect(),
        );
        assert!(validate_recipe(&many).is_err());
        let at_limit = recipe(
            (0..MAX_BAR_SLOTS)
                .map(|n| {
                    let mut entry = slot(
                        BarTextSource::Literal("x".repeat(120)),
                        BarIconSource::None,
                    );
                    entry.id = format!("s{n}");
                    entry
                })
                .collect(),
        );
        validate_recipe(&at_limit).unwrap();
        assert_eq!(resolve_recipe(&at_limit, &[]).unwrap().len(), MAX_BAR_SLOTS);
        let too_large = recipe(
            (0..MAX_BAR_SLOTS)
                .map(|n| {
                    let mut entry = slot(
                        BarTextSource::Literal("x".repeat(MAX_BAR_LITERAL_BYTES)),
                        BarIconSource::None,
                    );
                    entry.id = format!("s{n}");
                    entry
                })
                .collect(),
        );
        assert_eq!(validate_recipe(&too_large), Err(BarRecipeError::TooLarge));
        let empty_icon = recipe(vec![slot(BarTextSource::None, BarIconSource::None)]);
        assert_eq!(
            validate_recipe(&empty_icon),
            Err(BarRecipeError::InvalidIconOnly)
        );
    }

    #[test]
    fn fixed_icon_only_is_accessible_without_claiming_provider_identity_or_action() {
        let configured = recipe(vec![slot(
            BarTextSource::None,
            BarIconSource::Fixed(IconKind::Windows),
        )]);
        validate_recipe(&configured).unwrap();
        let result = resolve_recipe(&configured, &[]).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].value, "");
        assert!(result[0].icon_only);
        assert_eq!(result[0].icon, Some(IconKind::Windows));
        assert_eq!(result[0].accessibility_label, "Windows icon");
        assert_eq!(result[0].source_role, None);
        assert_eq!(result[0].details_action, None);
        assert_eq!(result[0].observed_at_ms, 0);
    }

    #[test]
    fn unsafe_segment_text_is_not_rendered_and_serde_rejects_unknown_recipe_fields() {
        let configured = recipe(vec![slot(
            BarTextSource::Role(SegmentRole::User),
            BarIconSource::TextSource,
        )]);
        let unsafe_source = segment(SegmentRole::User, "alice\nadmin", IconKind::User);
        assert!(resolve_recipe(&configured, &[unsafe_source])
            .unwrap()
            .is_empty());
        let overlong_source = segment(
            SegmentRole::User,
            &"x".repeat(MAX_BAR_ITEM_BYTES + 1),
            IconKind::User,
        );
        assert!(resolve_recipe(&configured, &[overlong_source])
            .unwrap()
            .is_empty());
        let serialized = serde_json::to_string(&configured).unwrap();
        assert_eq!(
            serde_json::from_str::<BarRecipe>(&serialized).unwrap(),
            configured
        );
        let mut unknown: serde_json::Value = serde_json::from_str(&serialized).unwrap();
        unknown["unexpected"] = serde_json::json!(true);
        assert!(serde_json::from_value::<BarRecipe>(unknown).is_err());
    }

    #[test]
    fn spacing_roundtrips_and_older_recipes_keep_the_preset_gap() {
        let mut configured = preset_recipe(InformationBarPreset::FloatingCards);
        configured.spacing_percent = 275;
        validate_recipe(&configured).unwrap();
        let serialized = serde_json::to_value(&configured).unwrap();
        assert_eq!(serialized["spacing-percent"], serde_json::json!(275));
        assert_eq!(
            serde_json::from_value::<BarRecipe>(serialized.clone()).unwrap(),
            configured
        );

        let mut old_recipe = serialized;
        old_recipe
            .as_object_mut()
            .unwrap()
            .remove("spacing-percent");
        let restored: BarRecipe = serde_json::from_value(old_recipe).unwrap();
        assert_eq!(restored.spacing_percent, 100);

        configured.spacing_percent = MAX_BAR_SPACING_PERCENT + 1;
        assert_eq!(
            validate_recipe(&configured),
            Err(BarRecipeError::InvalidSpacing)
        );
    }

    #[test]
    fn inactive_source_drafts_roundtrip_and_reject_unsafe_or_unbounded_data() {
        let mut drafts = BTreeMap::new();
        drafts.insert(
            "windows".into(),
            BarSlotSourceDraft {
                literal: Some("Custom label".into()),
                text_role: Some(SegmentRole::User),
                icon_role: Some(SegmentRole::Git),
                fixed_icon: Some(IconKind::Windows),
                coerced_icon: Some(BarIconSource::TextSource),
            },
        );
        validate_source_drafts(&drafts).unwrap();
        let encoded = serde_json::to_string(&drafts).unwrap();
        let restored: BTreeMap<String, BarSlotSourceDraft> =
            serde_json::from_str(&encoded).unwrap();
        assert_eq!(restored, drafts);

        drafts.get_mut("windows").unwrap().literal = Some("unsafe\ntext".into());
        assert_eq!(
            validate_source_drafts(&drafts),
            Err(BarRecipeError::InvalidText)
        );
        drafts.get_mut("windows").unwrap().literal =
            Some("x".repeat(MAX_BAR_LITERAL_BYTES + 1));
        assert_eq!(
            validate_source_drafts(&drafts),
            Err(BarRecipeError::InvalidText)
        );
        drafts.get_mut("windows").unwrap().literal = Some("safe".into());
        drafts.insert("../invalid".into(), BarSlotSourceDraft::default());
        assert_eq!(
            validate_source_drafts(&drafts),
            Err(BarRecipeError::InvalidSlotId)
        );
        drafts.remove("../invalid");
        for index in 0..MAX_BAR_SLOTS {
            drafts.insert(format!("slot-{index}"), BarSlotSourceDraft::default());
        }
        assert_eq!(
            validate_source_drafts(&drafts),
            Err(BarRecipeError::TooManySlots)
        );
    }

    #[test]
    fn tag_shapes_are_bounded_and_chevron_fan_does_not_fill_its_notch() {
        fn triangle_area(a: (f32, f32), b: (f32, f32), c: (f32, f32)) -> f32 {
            ((b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0)).abs() * 0.5
        }
        for (width, height) in [(2.0, 2.0), (12.0, 24.0), (80.0, 16.0)] {
            for style in [
                BarVisualStyle::Capsule,
                BarVisualStyle::Flat,
                BarVisualStyle::Chevron,
                BarVisualStyle::Hexagon,
                BarVisualStyle::Card,
                BarVisualStyle::Underline,
            ] {
                let geometry = tag_surface_geometry(style, width, height).unwrap();
                if let TagSurfaceGeometry::Chevron { points, stroke }
                | TagSurfaceGeometry::Hexagon { points, stroke } = geometry
                {
                    assert!(stroke > 0.0 && stroke <= 1.0);
                    assert!(points.iter().all(|(x, y)| {
                        *x >= 0.0 && *x <= width && *y >= 0.0 && *y <= height
                    }));
                    let polygon_area: f32 = (0..points.len())
                        .map(|i| {
                            let a = points[i];
                            let b = points[(i + 1) % points.len()];
                            a.0 * b.1 - b.0 * a.1
                        })
                        .sum::<f32>()
                        .abs()
                        * 0.5;
                    let fan_area: f32 = (1..points.len() - 1)
                        .map(|i| triangle_area(points[0], points[i], points[i + 1]))
                        .sum();
                    assert!((polygon_area - fan_area).abs() < 0.01);
                }
            }
        }
        assert!(tag_surface_geometry(BarVisualStyle::Chevron, f32::NAN, 16.0).is_none());
        assert!(tag_surface_geometry(BarVisualStyle::Card, 0.0, 16.0).is_none());
        assert!(tag_surface_geometry(BarVisualStyle::Card, 0.5, 16.0).is_none());
    }

    #[test]
    fn plain_shapes_keep_silhouette_without_filling_the_tag() {
        for style in [
            BarVisualStyle::Capsule,
            BarVisualStyle::Flat,
            BarVisualStyle::Chevron,
            BarVisualStyle::Hexagon,
            BarVisualStyle::Card,
            BarVisualStyle::Underline,
        ] {
            assert_eq!(
                tag_surface_paint_layers(style, 0.0),
                TagSurfacePaintLayers {
                    fill: false,
                    outline: true,
                }
            );
            let tinted = tag_surface_paint_layers(style, 0.12);
            assert_eq!(tinted.fill, style != BarVisualStyle::Underline);
            assert_eq!(
                tinted.outline,
                matches!(style, BarVisualStyle::Card | BarVisualStyle::Underline)
            );
        }
    }

    #[test]
    fn preview_and_terminal_metrics_and_adaptive_layout_share_width_decision() {
        let metrics = prompt_tag_metrics(24.0);
        assert_eq!(metrics.font_size, 14.0);
        assert!((metrics.height - 17.84).abs() < 0.001);
        let mut recipe = preset_recipe(InformationBarPreset::AdaptiveContextualBar);
        let (narrow_padding, narrow_gap, _, _) =
            bar_layout_hints(&recipe, &[], metrics, 30.0 * metrics.font_size - 1.0);
        let (wide_padding, wide_gap, _, _) =
            bar_layout_hints(&recipe, &[], metrics, 30.0 * metrics.font_size);
        assert_eq!(narrow_padding, 2.0);
        assert_eq!(narrow_gap, 0.5);
        assert!(wide_padding > narrow_padding);
        assert_eq!(wide_gap, 1.0);
        recipe.spacing_percent = 200;
        let (_, doubled_gap, _, _) = bar_layout_hints(&recipe, &[], metrics, 100.0);
        assert_eq!(doubled_gap, 1.0);
    }
}
