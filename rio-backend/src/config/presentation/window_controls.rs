//! Bounded, optional caption-control profiles shared by config and UI preferences.
use super::{OpacityPercent, Rgb, Rgba};
use serde::{Deserialize, Serialize};

macro_rules! choices {
    ($name:ident, $default:ident, $( $variant:ident => ($id:literal, $label:literal) ),+ $(,)?) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
        pub enum $name { $( #[serde(rename = $id)] $variant ),+ }
        impl Default for $name { fn default() -> Self { Self::$default } }
        impl $name {
            pub const ALL: &'static [Self] = &[$(Self::$variant),+];
            pub const fn id(self) -> &'static str { match self { $(Self::$variant => $id),+ } }
            pub const fn label(self) -> &'static str { match self { $(Self::$variant => $label),+ } }
            pub fn from_id(id: &str) -> Option<Self> { Self::ALL.iter().copied().find(|value| value.id() == id) }
        }
    };
}

choices!(WindowControlStyle, Soft,
    Soft => ("soft", "Soft"), Glass => ("glass", "Glass"),
    Outline => ("outline", "Outline"), Circles => ("circles", "Circles"),
);
choices!(WindowControlSize, Standard,
    Compact => ("compact", "Compact"), Standard => ("standard", "Standard"),
    Large => ("large", "Large"),
);
choices!(WindowControlSpacing, Balanced,
    Tight => ("tight", "Tight"), Balanced => ("balanced", "Balanced"),
    Airy => ("airy", "Airy"),
);
choices!(WindowControlWeight, Regular,
    Fine => ("fine", "Fine"), Regular => ("regular", "Regular"),
    Bold => ("bold", "Bold"),
);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub struct WindowControlProfile {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<WindowControlSize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spacing: Option<WindowControlSpacing>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub roundness: Option<OpacityPercent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon_size: Option<WindowControlSize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon_weight: Option<WindowControlWeight>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hover_strength: Option<OpacityPercent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inactive_opacity: Option<OpacityPercent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minimize: Option<Rgb>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maximize: Option<Rgb>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub close: Option<Rgb>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background: Option<Rgba>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub border: Option<Rgba>,
}

impl WindowControlProfile {
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }
    pub fn overlay(self, base: &mut Self) {
        macro_rules! apply { ($($field:ident),+) => { $(
            if self.$field.is_some() { base.$field = self.$field; }
        )+ }; }
        apply!(
            size,
            spacing,
            roundness,
            icon_size,
            icon_weight,
            hover_strength,
            inactive_opacity,
            minimize,
            maximize,
            close,
            background,
            border
        );
    }
}

/// Exactly four slots: switching styles never discards another style's edits.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub struct WindowControlsAppearance {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<WindowControlStyle>,
    #[serde(skip_serializing_if = "WindowControlProfile::is_empty")]
    pub soft: WindowControlProfile,
    #[serde(skip_serializing_if = "WindowControlProfile::is_empty")]
    pub glass: WindowControlProfile,
    #[serde(skip_serializing_if = "WindowControlProfile::is_empty")]
    pub outline: WindowControlProfile,
    #[serde(skip_serializing_if = "WindowControlProfile::is_empty")]
    pub circles: WindowControlProfile,
}

impl WindowControlsAppearance {
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }
    pub fn profile(&self, style: WindowControlStyle) -> &WindowControlProfile {
        match style {
            WindowControlStyle::Soft => &self.soft,
            WindowControlStyle::Glass => &self.glass,
            WindowControlStyle::Outline => &self.outline,
            WindowControlStyle::Circles => &self.circles,
        }
    }
    pub fn profile_mut(
        &mut self,
        style: WindowControlStyle,
    ) -> &mut WindowControlProfile {
        match style {
            WindowControlStyle::Soft => &mut self.soft,
            WindowControlStyle::Glass => &mut self.glass,
            WindowControlStyle::Outline => &mut self.outline,
            WindowControlStyle::Circles => &mut self.circles,
        }
    }
    pub fn overlay(self, base: &mut Self) {
        if self.style.is_some() {
            base.style = self.style;
        }
        for &style in WindowControlStyle::ALL {
            self.profile(style).overlay(base.profile_mut(style));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn window_controls_are_admitted_by_the_existing_presentation_parser() {
        let parsed: super::super::Presentation=toml::from_str("[window-controls]\nstyle = 'glass'\n[window-controls.glass]\nicon-size = 'large'\n").unwrap();
        assert_eq!(
            parsed.window_controls.style,
            Some(WindowControlStyle::Glass)
        );
        assert_eq!(
            parsed.window_controls.glass.icon_size,
            Some(WindowControlSize::Large)
        );
        assert_eq!(
            toml::from_str::<super::super::Presentation>(
                &toml::to_string(&parsed).unwrap()
            )
            .unwrap(),
            parsed
        );
    }
    #[test]
    fn window_controls_profiles_round_trip_and_overlay_independently() {
        let base = "style = 'outline'\n[soft]\nicon-weight = 'bold'\n[outline]\nbackground = '#12345680'\nroundness = 80\n";
        let mut base: WindowControlsAppearance = toml::from_str(base).unwrap();
        let user: WindowControlsAppearance = toml::from_str("style = 'circles'\n[outline]\nspacing = 'airy'\n[circles]\nclose = '#ab1234'\n").unwrap();
        user.overlay(&mut base);
        assert_eq!(base.soft.icon_weight, Some(WindowControlWeight::Bold));
        assert_eq!(base.outline.background.unwrap().bytes(), [18, 52, 86, 128]);
        assert_eq!(base.outline.roundness.unwrap().get(), 80);
        assert_eq!(base.outline.spacing, Some(WindowControlSpacing::Airy));
        assert_eq!(base.circles.close.unwrap().bytes(), [171, 18, 52]);
        assert_eq!(base.style, Some(WindowControlStyle::Circles));
        assert_eq!(
            toml::from_str::<WindowControlsAppearance>(&toml::to_string(&base).unwrap())
                .unwrap(),
            base
        );
    }
    #[test]
    fn window_controls_reject_unknown_and_out_of_bounds_profiles() {
        for input in [
            "style = 'future'",
            "[future]",
            "[soft]\nroundness = 101",
            "[glass]\nhover-strength = -1",
            "[circles]\ninactive-opacity = 0.5",
            "[outline]\nsize = 'huge'",
            "[soft]\nclose = '#12345600'",
            "[soft]\nunknown = true",
        ] {
            assert!(
                toml::from_str::<WindowControlsAppearance>(input).is_err(),
                "{input}"
            );
        }
    }
}
