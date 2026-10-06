//! User-configurable presentation of ordinary terminal output.

mod appearance;
mod interface;
pub use interface::*;
mod tables;
mod timestamps;
mod window_controls;
pub use appearance::{
    AppearanceValueError, CommandOutputAppearance, HighlightAppearance, HighlightColors,
    HighlightStyle, OpacityPercent, Rgb, Rgba, TagAppearance, TagColors, TagStyle,
};
pub use tables::{TableAppearance, TableBanding, TableBorderStyle, TableBorderWeight};
pub use timestamps::*;
pub use window_controls::*;

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
    pub tables: TableAppearance,
    pub timestamps: TimestampAppearance,
    pub window_controls: WindowControlsAppearance,
    pub interface: TerminalInterfaceAppearance,
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
            tables: TableAppearance::default(),
            timestamps: TimestampAppearance::default(),
            window_controls: WindowControlsAppearance::default(),
            interface: TerminalInterfaceAppearance::default(),
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
            tables: TableAppearance,
            timestamps: TimestampAppearance,
            window_controls: WindowControlsAppearance,
            interface: TerminalInterfaceAppearance,
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
                    tables: TableAppearance::default(),
                    timestamps: TimestampAppearance::default(),
                    window_controls: WindowControlsAppearance::default(),
                    interface: TerminalInterfaceAppearance::default(),
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
            tables: fields.tables,
            timestamps: fields.timestamps,
            window_controls: fields.window_controls,
            interface: fields.interface,
        })
    }
}

#[cfg(test)]
mod timestamp_config_tests {
    use super::*;

    #[test]
    fn terminal_appearance_config_round_trips_footer_visibility_and_style() {
        let source = "[interface.footer]\nvisible = false\nheight = 40\nfont-size = 14\nbackground = '#12345680'\nshow-clock = false\n[interface.header]\nbackground = '#23456780'\ntab-radius = 12\n";
        let parsed: Presentation =
            toml::from_str(source).expect("terminal appearance config");
        let output = toml::to_string(&parsed).unwrap();
        assert!(output.contains("visible = false"));
        assert!(output.contains("show-clock = false"));
        assert_eq!(toml::from_str::<Presentation>(&output).unwrap(), parsed);
    }

    #[test]
    fn command_timestamp_customization_config_is_admitted_and_round_trips() {
        let source = "[timestamps]\ndate-format = 'day-month-year'\ntime-format = '12-hour'\ndate-position = 'above-left'\ntime-position = 'below-right'\nbackground = '#12345680'\n";
        let parsed: Presentation =
            toml::from_str(source).expect("timestamp appearance is configurable");
        let encoded = toml::to_string(&parsed).unwrap();
        assert!(encoded.contains("date-format = \"day-month-year\""));
        assert!(encoded.contains("time-position = \"below-right\""));
        assert_eq!(toml::from_str::<Presentation>(&encoded).unwrap(), parsed);
    }
}
