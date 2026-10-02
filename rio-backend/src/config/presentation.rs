//! User-configurable presentation of ordinary terminal output.

mod appearance;
pub use appearance::{
    AppearanceValueError, CommandOutputAppearance, HighlightAppearance, HighlightColors,
    HighlightStyle, OpacityPercent, Rgb, Rgba, TagAppearance, TagColors, TagStyle,
};

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Presentation {
    pub inline_tables: bool,
    pub output_highlighting: bool,
    pub command_output_highlighting: bool,
    pub kubernetes_highlighting: bool,
    pub command_timestamps: bool,
    pub tags: TagAppearance,
    pub highlight: HighlightAppearance,
    pub command_output: CommandOutputAppearance,
    pub kubernetes: HighlightAppearance,
}

impl Default for Presentation {
    fn default() -> Self {
        Self {
            inline_tables: true,
            output_highlighting: true,
            command_output_highlighting: true,
            kubernetes_highlighting: true,
            command_timestamps: true,
            tags: TagAppearance::default(),
            highlight: HighlightAppearance::default(),
            command_output: CommandOutputAppearance::default(),
            kubernetes: HighlightAppearance::default(),
        }
    }
}

/// Older config files used the log palette and switch for Kubernetes too.
/// Resolve that compatibility only while loading; the resulting independent
/// values never follow subsequent edits to another presentation domain.
impl<'de> Deserialize<'de> for Presentation {
    fn deserialize<D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
        struct ConfigFields {
            inline_tables: bool,
            output_highlighting: bool,
            command_output_highlighting: bool,
            kubernetes_highlighting: Option<bool>,
            command_timestamps: bool,
            tags: TagAppearance,
            highlight: HighlightAppearance,
            command_output: CommandOutputAppearance,
            kubernetes: Option<HighlightAppearance>,
        }
        impl Default for ConfigFields {
            fn default() -> Self {
                let base = Presentation::default();
                Self {
                    inline_tables: base.inline_tables,
                    output_highlighting: base.output_highlighting,
                    command_output_highlighting: base.command_output_highlighting,
                    kubernetes_highlighting: None,
                    command_timestamps: base.command_timestamps,
                    tags: base.tags,
                    highlight: base.highlight,
                    command_output: base.command_output,
                    kubernetes: None,
                }
            }
        }
        let fields = ConfigFields::deserialize(deserializer)?;
        Ok(Self {
            inline_tables: fields.inline_tables,
            output_highlighting: fields.output_highlighting,
            command_output_highlighting: fields.command_output_highlighting,
            kubernetes_highlighting: fields
                .kubernetes_highlighting
                .unwrap_or(fields.output_highlighting),
            command_timestamps: fields.command_timestamps,
            tags: fields.tags,
            highlight: fields.highlight,
            command_output: fields.command_output,
            kubernetes: fields.kubernetes.unwrap_or(fields.highlight),
        })
    }
}
