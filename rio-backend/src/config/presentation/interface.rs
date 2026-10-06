//! Bounded, capability-free terminal chrome. Missing fields inherit the theme/config.
use super::{Rgb, Rgba};
use serde::{Deserialize, Serialize};

/// Logical pixels, validated at both the configuration and preference boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiPixels<const MIN: u16, const MAX: u16>(u32);

impl<const MIN: u16, const MAX: u16> UiPixels<MIN, MAX> {
    pub fn new(value: u16) -> Option<Self> {
        Self::from_decimal(f32::from(value))
    }
    pub fn from_decimal(value: f32) -> Option<Self> {
        (value.is_finite() && (f32::from(MIN)..=f32::from(MAX)).contains(&value))
            .then(|| Self((value * 1000.0).round() as u32))
    }
    pub fn get(self) -> f32 {
        self.0 as f32 / 1000.0
    }
}
impl<const MIN: u16, const MAX: u16> Serialize for UiPixels<MIN, MAX> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_f32(self.get())
    }
}
impl<'de, const MIN: u16, const MAX: u16> Deserialize<'de> for UiPixels<MIN, MAX> {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Self::from_decimal(f32::deserialize(d)?).ok_or_else(|| {
            serde::de::Error::custom("UI dimension outside supported range")
        })
    }
}

macro_rules! appearance {
    ($name:ident { $($field:ident: $ty:ty),* $(,)? }) => {
        #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
        #[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
        pub struct $name {
            $(#[serde(skip_serializing_if = "Option::is_none")]
            pub $field: Option<$ty>,)*
        }
        impl $name {
            pub fn is_empty(&self) -> bool { self == &Self::default() }
            pub fn overlay(self, base: &mut Self) {
                $(if self.$field.is_some() { base.$field = self.$field; })*
            }
        }
    };
}

appearance!(FooterAppearance {
    visible: bool,
    height: UiPixels<24, 72>,
    padding: UiPixels<0, 32>,
    font_size: UiPixels<8, 24>,
    bold: bool,
    background: Rgba,
    text: Rgb,
    muted_text: Rgb,
    border: Rgba,
    border_width: UiPixels<0, 4>,
    show_clock: bool,
    show_dimensions: bool,
    show_encoding: bool,
    show_line_ending: bool,
    show_pane: bool,
    show_tab: bool,
    show_context: bool,
    show_selection: bool,
});

impl FooterAppearance {
    pub fn is_visible(self) -> bool {
        self.visible.unwrap_or(true)
    }
    pub fn font_size(self) -> f32 {
        self.font_size.map_or(12.0, UiPixels::get)
    }
    pub fn height(self) -> f32 {
        self.height
            .map_or(32.0, UiPixels::get)
            .max(self.font_size() + 12.0)
    }
    /// One reservation for layout, paint, input and accessibility. Tiny panes
    /// retain at least 80 logical pixels for terminal content.
    pub fn reserved_height(self, panel_height: f32, scale: f32) -> f32 {
        if !self.is_visible()
            || !panel_height.is_finite()
            || !scale.is_finite()
            || scale <= f32::EPSILON
            || panel_height / scale < 80.0 + self.height()
        {
            0.0
        } else {
            self.height() * scale
        }
    }
}

appearance!(HeaderAppearance {
    background: Rgba,
    border: Rgba,
    border_width: UiPixels<0, 4>,
    text: Rgb,
    inactive_text: Rgb,
    active_tab: Rgba,
    inactive_tab: Rgba,
    tab_radius: UiPixels<0, 24>,
    tab_gap: UiPixels<0, 20>,
    font_size: UiPixels<10, 20>,
});

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub struct TerminalInterfaceAppearance {
    #[serde(skip_serializing_if = "FooterAppearance::is_empty")]
    pub footer: FooterAppearance,
    #[serde(skip_serializing_if = "HeaderAppearance::is_empty")]
    pub header: HeaderAppearance,
}

impl TerminalInterfaceAppearance {
    pub fn is_empty(&self) -> bool {
        self.footer.is_empty() && self.header.is_empty()
    }
    pub fn overlay(self, base: &mut Self) {
        self.footer.overlay(&mut base.footer);
        self.header.overlay(&mut base.header);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn footer_reservation_hides_and_scales_without_starving_terminal_content() {
        let mut footer = FooterAppearance::default();
        for scale in [1.0, 1.25, 1.5, 2.0, 3.0] {
            assert_eq!(footer.reserved_height(500.0 * scale, scale), 32.0 * scale);
            assert_eq!(footer.reserved_height(111.0 * scale, scale), 0.0);
        }
        footer.height = UiPixels::new(24);
        footer.font_size = UiPixels::new(24);
        assert_eq!(footer.height(), 36.0);
        assert_eq!(footer.reserved_height(115.0, 1.0), 0.0);
        footer.visible = Some(false);
        assert_eq!(footer.reserved_height(500.0, 1.0), 0.0);
        assert_eq!(footer.reserved_height(f32::NAN, 1.0), 0.0);
        assert_eq!(footer.reserved_height(500.0, 0.0), 0.0);
    }
    #[test]
    fn interface_rejects_invalid_fields_and_preserves_independent_overrides() {
        for bad in [
            "[footer]\nheight = 999",
            "[footer]\nfont-size = 7",
            "[header]\ntab-gap = -1",
            "[header]\nfont-size = nan",
            "[footer]\nunknown = true",
        ] {
            assert!(
                toml::from_str::<TerminalInterfaceAppearance>(bad).is_err(),
                "{bad}"
            );
        }
        let mut base: TerminalInterfaceAppearance = toml::from_str(
            "[footer]\nheight = 40\nvisible = true\n[header]\ntab-radius = 9",
        )
        .unwrap();
        let user: TerminalInterfaceAppearance =
            toml::from_str("[footer]\nvisible = false").unwrap();
        user.overlay(&mut base);
        assert!(!base.footer.is_visible());
        assert_eq!(base.footer.height(), 40.0);
        assert_eq!(base.header.tab_radius.unwrap().get(), 9.0);
    }
}
