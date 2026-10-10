//! One fixed application binding between settings and terminal presentation.
//! No discovery, provider authority, I/O or arbitrary role registry lives here.

use automexia_extension_api::{SegmentRole, SemanticSeverity};
use rio_backend::config::{
    colors::Colors,
    presentation::{
        CommandOutputAppearance, HighlightAppearance, HighlightColors, Rgb, Rgba,
        TagAppearance, TagColors,
    },
};

pub const TAG_ENABLED: &str = "tags.enabled";
pub const TAG_FORMAT: &str = "tags.format";
pub const TAG_STYLE: &str = "tags.style";
pub const TAG_OPACITY: &str = "tags.opacity";
pub const OUTPUT_STYLE: &str = "output.style";
pub const KUBERNETES_STYLE: &str = "kubernetes.style";
pub const COMMAND_OUTPUT_PULSE: &str = "command_output.pulse";

#[derive(Clone, Copy)]
pub struct TagColorBinding {
    pub id: &'static str,
    pub label: &'static str,
    pub role: SegmentRole,
    pub read: fn(&TagColors) -> Option<Rgb>,
    pub write: fn(&mut TagColors, Option<Rgb>),
}

macro_rules! tag_binding {
    ($role:ident, $field:ident, $label:literal) => {
        TagColorBinding {
            id: concat!("tags.colors.", stringify!($field)),
            label: $label,
            role: SegmentRole::$role,
            read: |colors| colors.$field,
            write: |colors, value| colors.$field = value,
        }
    };
}

pub const TAG_COLOR_BINDINGS: [TagColorBinding; 13] = [
    tag_binding!(Production, production, "Production"),
    tag_binding!(UbuntuWsl, ubuntu_wsl, "Operating system"),
    tag_binding!(Windows, windows, "Windows"),
    tag_binding!(Git, git, "Git"),
    tag_binding!(Kubernetes, kubernetes, "Kubernetes"),
    tag_binding!(Docker, docker, "Docker"),
    tag_binding!(Azure, azure, "Azure"),
    tag_binding!(Aws, aws, "AWS"),
    tag_binding!(Gcp, gcp, "Google Cloud"),
    tag_binding!(UnknownCloud, unknown_cloud, "Other cloud"),
    tag_binding!(Terraform, terraform, "Terraform"),
    tag_binding!(Environment, environment, "Environment"),
    tag_binding!(User, user, "User"),
];

#[derive(Clone, Copy)]
pub struct OutputColorBinding {
    pub id: &'static str,
    pub label: &'static str,
    pub severity: SemanticSeverity,
    pub read: fn(&HighlightColors) -> Option<Rgb>,
    pub write: fn(&mut HighlightColors, Option<Rgb>),
    pub palette: fn(&Colors) -> [f32; 4],
}

macro_rules! output_binding {
    ($severity:ident, $field:ident, $label:literal, $palette:ident) => {
        OutputColorBinding {
            id: concat!("output.colors.", stringify!($field)),
            label: $label,
            severity: SemanticSeverity::$severity,
            read: |colors| colors.$field,
            write: |colors, value| colors.$field = value,
            palette: |colors| colors.$palette,
        }
    };
}

pub const OUTPUT_COLOR_BINDINGS: [OutputColorBinding; 5] = [
    output_binding!(Error, error, "Error", red),
    output_binding!(Warning, warning, "Warning", yellow),
    output_binding!(Success, success, "Success", green),
    output_binding!(Info, info, "Information", cyan),
    output_binding!(Debug, debug, "Debug", blue),
];

/// The Kubernetes palette shares color mechanics, not values or setting IDs.
pub const KUBERNETES_COLOR_BINDINGS: [OutputColorBinding; 5] = [
    OutputColorBinding {
        id: "kubernetes.colors.error",
        ..OUTPUT_COLOR_BINDINGS[0]
    },
    OutputColorBinding {
        id: "kubernetes.colors.warning",
        ..OUTPUT_COLOR_BINDINGS[1]
    },
    OutputColorBinding {
        id: "kubernetes.colors.success",
        ..OUTPUT_COLOR_BINDINGS[2]
    },
    OutputColorBinding {
        id: "kubernetes.colors.info",
        ..OUTPUT_COLOR_BINDINGS[3]
    },
    OutputColorBinding {
        id: "kubernetes.colors.debug",
        ..OUTPUT_COLOR_BINDINGS[4]
    },
];

#[derive(Clone, Copy)]
pub struct OutputBackgroundBinding {
    pub id: &'static str,
    pub label: &'static str,
    pub severity: SemanticSeverity,
    pub read: fn(&HighlightAppearance) -> Option<Rgba>,
    pub write: fn(&mut HighlightAppearance, Option<Rgba>),
    /// The fallback color shown by Settings and used in Background mode.
    pub default: [u8; 4],
    /// Only Error and Warning had default backgrounds before customization.
    pub default_in_both: bool,
}

impl OutputBackgroundBinding {
    pub fn resolve(&self, appearance: &HighlightAppearance) -> [u8; 4] {
        (self.read)(appearance).map_or(self.default, Rgba::bytes)
    }

    pub fn editor_color(&self, appearance: &HighlightAppearance) -> [u8; 4] {
        let mut color = self.resolve(appearance);
        // Both historically enables only error/warning backgrounds by default.
        // Show that as zero opacity rather than claiming an invisible tint.
        if appearance.style == rio_backend::config::presentation::HighlightStyle::Both
            && !self.default_in_both
            && (self.read)(appearance).is_none()
        {
            color[3] = 0;
        }
        color
    }
}

pub const OUTPUT_BACKGROUND_BINDINGS: [OutputBackgroundBinding; 5] = [
    OutputBackgroundBinding {
        id: "output.backgrounds.error",
        label: "Error background",
        severity: SemanticSeverity::Error,
        read: |appearance| appearance.error_background,
        write: |appearance, value| appearance.error_background = value,
        default: [105, 12, 25, 86],
        default_in_both: true,
    },
    OutputBackgroundBinding {
        id: "output.backgrounds.warning",
        label: "Warning background",
        severity: SemanticSeverity::Warning,
        read: |appearance| appearance.warning_background,
        write: |appearance, value| appearance.warning_background = value,
        default: [96, 69, 0, 78],
        default_in_both: true,
    },
    OutputBackgroundBinding {
        id: "output.backgrounds.success",
        label: "Success background",
        severity: SemanticSeverity::Success,
        read: |appearance| appearance.success_background,
        write: |appearance, value| appearance.success_background = value,
        default: [9, 87, 46, 78],
        default_in_both: false,
    },
    OutputBackgroundBinding {
        id: "output.backgrounds.info",
        label: "Information background",
        severity: SemanticSeverity::Info,
        read: |appearance| appearance.info_background,
        write: |appearance, value| appearance.info_background = value,
        default: [0, 70, 95, 78],
        default_in_both: false,
    },
    OutputBackgroundBinding {
        id: "output.backgrounds.debug",
        label: "Debug background",
        severity: SemanticSeverity::Debug,
        read: |appearance| appearance.debug_background,
        write: |appearance, value| appearance.debug_background = value,
        default: [43, 45, 89, 78],
        default_in_both: false,
    },
];

pub const KUBERNETES_BACKGROUND_BINDINGS: [OutputBackgroundBinding; 5] = [
    OutputBackgroundBinding {
        id: "kubernetes.backgrounds.error",
        ..OUTPUT_BACKGROUND_BINDINGS[0]
    },
    OutputBackgroundBinding {
        id: "kubernetes.backgrounds.warning",
        ..OUTPUT_BACKGROUND_BINDINGS[1]
    },
    OutputBackgroundBinding {
        id: "kubernetes.backgrounds.success",
        ..OUTPUT_BACKGROUND_BINDINGS[2]
    },
    OutputBackgroundBinding {
        id: "kubernetes.backgrounds.info",
        ..OUTPUT_BACKGROUND_BINDINGS[3]
    },
    OutputBackgroundBinding {
        id: "kubernetes.backgrounds.debug",
        ..OUTPUT_BACKGROUND_BINDINGS[4]
    },
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandOutputKind {
    Success,
    Failure,
    Neutral,
}

#[derive(Clone, Copy)]
pub struct CommandOutputBackgroundBinding {
    pub id: &'static str,
    pub label: &'static str,
    pub kind: CommandOutputKind,
    pub read: fn(&CommandOutputAppearance) -> Option<Rgba>,
    pub write: fn(&mut CommandOutputAppearance, Option<Rgba>),
}

impl CommandOutputBackgroundBinding {
    pub fn resolve(
        &self,
        appearance: &CommandOutputAppearance,
        palette: &Colors,
    ) -> [f32; 4] {
        command_output_background(appearance, palette, self.kind)
    }
}

pub const COMMAND_OUTPUT_BACKGROUND_BINDINGS: [CommandOutputBackgroundBinding; 3] = [
    CommandOutputBackgroundBinding {
        id: "command_output.backgrounds.success",
        label: "Successful command",
        kind: CommandOutputKind::Success,
        read: |appearance| appearance.success,
        write: |appearance, value| appearance.success = value,
    },
    CommandOutputBackgroundBinding {
        id: "command_output.backgrounds.failure",
        label: "Failed command",
        kind: CommandOutputKind::Failure,
        read: |appearance| appearance.failure,
        write: |appearance, value| appearance.failure = value,
    },
    CommandOutputBackgroundBinding {
        id: "command_output.backgrounds.neutral",
        label: "Unknown result",
        kind: CommandOutputKind::Neutral,
        read: |appearance| appearance.neutral,
        write: |appearance, value| appearance.neutral = value,
    },
];

/// Resolve command backgrounds from their own appearance, retaining the exact
/// historical normalized alpha rather than quantizing it through an alpha byte.
pub fn command_output_background(
    appearance: &CommandOutputAppearance,
    palette: &Colors,
    kind: CommandOutputKind,
) -> [f32; 4] {
    let (custom, fallback) = match kind {
        CommandOutputKind::Success => (appearance.success, palette.green),
        CommandOutputKind::Failure => (appearance.failure, palette.red),
        CommandOutputKind::Neutral => (appearance.neutral, palette.blue),
    };
    custom.map_or(
        [fallback[0], fallback[1], fallback[2], 0.099],
        Rgba::to_color_array,
    )
}

/// Exact existing normalized sRGB conversion; retains truncation compatibility.
pub fn normalized_to_u8(color: [f32; 4]) -> [u8; 4] {
    color.map(|channel| (channel.clamp(0.0, 1.0) * 255.0) as u8)
}

pub fn tag_anchor(appearance: &TagAppearance, role: SegmentRole) -> [u8; 3] {
    TAG_COLOR_BINDINGS
        .iter()
        .find(|binding| binding.role == role)
        .and_then(|binding| (binding.read)(&appearance.colors))
        .map_or_else(|| automexia_ui_model::segment_anchor_rgb(role), Rgb::bytes)
}

pub fn output_foreground(
    appearance: &HighlightAppearance,
    palette: &Colors,
    severity: SemanticSeverity,
) -> [u8; 4] {
    OUTPUT_COLOR_BINDINGS
        .iter()
        .find(|binding| binding.severity == severity)
        .map(|binding| {
            (binding.read)(&appearance.colors).map_or_else(
                || normalized_to_u8((binding.palette)(palette)),
                Rgb::rgba_bytes,
            )
        })
        // The fixed registry covers all current severities. A later enum/registry
        // mismatch degrades to ordinary text and cannot invent a highlight.
        .unwrap_or_else(|| normalized_to_u8(palette.foreground))
}

pub fn output_background(
    appearance: &HighlightAppearance,
    severity: SemanticSeverity,
) -> Option<[u8; 4]> {
    OUTPUT_BACKGROUND_BINDINGS
        .iter()
        .find(|binding| binding.severity == severity)
        .and_then(|binding| {
            ((binding.read)(appearance).is_some()
                || appearance.style
                    == rio_backend::config::presentation::HighlightStyle::Background
                || binding.default_in_both)
                .then(|| binding.resolve(appearance))
        })
}

#[cfg(test)]
mod output_domain_tests {
    use super::*;

    #[test]
    fn command_bands_keep_theme_channels_exact_alpha_and_independent_overrides() {
        let palette = Colors::default();
        let mut appearance = CommandOutputAppearance::default();
        for (binding, expected) in COMMAND_OUTPUT_BACKGROUND_BINDINGS.iter().zip([
            palette.green,
            palette.red,
            palette.blue,
        ]) {
            assert_eq!(
                binding.resolve(&appearance, &palette),
                [expected[0], expected[1], expected[2], 0.099]
            );
            let custom = Rgba::from_bytes([17, 34, 51, 68]);
            (binding.write)(&mut appearance, Some(custom));
            assert_eq!(
                binding.resolve(&appearance, &palette),
                custom.to_color_array()
            );
            (binding.write)(&mut appearance, None);
            assert_eq!(appearance, CommandOutputAppearance::default());
        }
    }

    #[test]
    fn semantic_domain_bindings_are_unique_and_keep_status_mechanics_identical() {
        let mut ids = std::collections::BTreeSet::new();
        for bindings in [&OUTPUT_COLOR_BINDINGS, &KUBERNETES_COLOR_BINDINGS] {
            for binding in bindings {
                assert!(ids.insert(binding.id));
            }
        }
        for bindings in [&OUTPUT_BACKGROUND_BINDINGS, &KUBERNETES_BACKGROUND_BINDINGS] {
            for binding in bindings {
                assert!(ids.insert(binding.id));
            }
        }
        for binding in &COMMAND_OUTPUT_BACKGROUND_BINDINGS {
            assert!(ids.insert(binding.id));
        }
        for (log, kubernetes) in OUTPUT_BACKGROUND_BINDINGS
            .iter()
            .zip(KUBERNETES_BACKGROUND_BINDINGS.iter())
        {
            assert_eq!(log.severity, kubernetes.severity);
            assert_eq!(
                log.resolve(&HighlightAppearance::default()),
                kubernetes.resolve(&HighlightAppearance::default())
            );
        }
    }
}
