//! Optional UI overrides projected onto the existing configuration owners.
use rio_backend::config::{
    presentation::{OpacityPercent, Rgb, TerminalInterfaceAppearance, UiPixels},
    Config,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub struct InterfacePreferences {
    #[serde(skip_serializing_if = "TerminalInterfaceAppearance::is_empty")]
    pub appearance: TerminalInterfaceAppearance,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opacity: Option<OpacityPercent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opacity_cells: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blur: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub padding_top: Option<UiPixels<0, 64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub padding_bottom: Option<UiPixels<0, 64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub padding_left: Option<UiPixels<0, 64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub padding_right: Option<UiPixels<0, 64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pane_padding: Option<UiPixels<0, 32>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pane_margin: Option<UiPixels<0, 32>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub row_gap: Option<UiPixels<0, 32>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub column_gap: Option<UiPixels<0, 32>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub border_width: Option<UiPixels<0, 8>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub border_color: Option<Rgb>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inactive_opacity: Option<OpacityPercent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tab_width: Option<UiPixels<80, 280>>,
}

impl InterfacePreferences {
    pub fn is_empty(&self) -> bool {
        self == &Self::default()
    }
    pub fn apply_to(&self, config: &mut Config) {
        if let Some(v) = self.opacity {
            config.window.opacity = f32::from(v.get()) / 100.0;
        }
        if let Some(v) = self.opacity_cells {
            config.window.opacity_cells = v;
        }
        if let Some(v) = self.blur {
            config.window.blur = if v {
                rio_backend::config::window::WindowBlur::System
            } else {
                rio_backend::config::window::WindowBlur::Off
            };
        }
        macro_rules! pixels { ($($field:ident => $target:expr),* $(,)?) => { $(
            if let Some(v) = self.$field { $target = v.get(); }
        )* }; }
        pixels!(padding_top => config.margin.top, padding_bottom => config.margin.bottom,
            padding_left => config.margin.left, padding_right => config.margin.right,
            row_gap => config.panel.row_gap, column_gap => config.panel.column_gap,
            border_width => config.panel.border_width, tab_width => config.navigation.max_tab_width);
        if let Some(v) = self.pane_padding {
            config.panel.padding = rio_backend::config::layout::Margin::all(v.get());
        }
        if let Some(v) = self.pane_margin {
            config.panel.margin = rio_backend::config::layout::Margin::all(v.get());
        }
        if let Some(v) = self.inactive_opacity {
            config.navigation.unfocused_split_opacity =
                (f32::from(v.get()) / 100.0).max(0.15);
        }
        // Split colors follow the same resolved palette as all other UI colors.
        if let Some(v) = self.border_color {
            let bytes = v.bytes();
            let color = [
                f32::from(bytes[0]) / 255.0,
                f32::from(bytes[1]) / 255.0,
                f32::from(bytes[2]) / 255.0,
                1.0,
            ];
            config.colors.split = color;
            if let Some(adaptive) = config.adaptive_colors.as_mut() {
                for palette in [&mut adaptive.light, &mut adaptive.dark]
                    .into_iter()
                    .flatten()
                {
                    palette.split = color;
                }
            }
        }
    }
}
