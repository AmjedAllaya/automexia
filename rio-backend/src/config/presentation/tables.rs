//! Optional table appearance overrides. Defaults preserve existing table paint.
use super::{Rgb, Rgba};
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

choices!(TableBorderStyle, Solid,
    None => ("none", "None"), Solid => ("solid", "Solid"),
    Dashed => ("dashed", "Dashed"), Dotted => ("dotted", "Dotted"),
    Double => ("double", "Double"),
);
choices!(TableBorderWeight, Thin,
    Thin => ("thin", "Thin"), Medium => ("medium", "Medium"), Thick => ("thick", "Thick"),
);
impl TableBorderWeight {
    pub const fn pixels(self) -> f32 {
        match self {
            Self::Thin => 1.0,
            Self::Medium => 2.0,
            Self::Thick => 3.0,
        }
    }
}
choices!(TableBanding, None,
    None => ("none", "Off"), Rows => ("rows", "Alternating rows"),
    Columns => ("columns", "Alternating columns"),
    Checkerboard => ("checkerboard", "Checkerboard"),
);
impl TableBanding {
    /// Indices count logical data rows/cells, never wrapped physical lines.
    pub const fn alternate(self, row: usize, column: usize) -> bool {
        match self {
            Self::None => false,
            Self::Rows => row % 2 == 1,
            Self::Columns => column % 2 == 1,
            Self::Checkerboard => (row % 2 + column % 2) % 2 == 1,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct TableAppearance {
    #[serde(rename = "border-style", skip_serializing_if = "Option::is_none")]
    pub border_style: Option<TableBorderStyle>,
    #[serde(rename = "border-weight", skip_serializing_if = "Option::is_none")]
    pub border_weight: Option<TableBorderWeight>,
    #[serde(rename = "row-lines", skip_serializing_if = "Option::is_none")]
    pub row_lines: Option<bool>,
    #[serde(rename = "column-lines", skip_serializing_if = "Option::is_none")]
    pub column_lines: Option<bool>,
    #[serde(rename = "outer-border", skip_serializing_if = "Option::is_none")]
    pub outer_border: Option<bool>,
    #[serde(rename = "header-separator", skip_serializing_if = "Option::is_none")]
    pub header_separator: Option<bool>,
    #[serde(rename = "header-bold", skip_serializing_if = "Option::is_none")]
    pub header_bold: Option<bool>,
    #[serde(rename = "banding", skip_serializing_if = "Option::is_none")]
    pub banding: Option<TableBanding>,
    #[serde(rename = "border-color", skip_serializing_if = "Option::is_none")]
    pub border_color: Option<Rgba>,
    #[serde(rename = "header-foreground", skip_serializing_if = "Option::is_none")]
    pub header_foreground: Option<Rgb>,
    #[serde(rename = "header-background", skip_serializing_if = "Option::is_none")]
    pub header_background: Option<Rgba>,
    #[serde(rename = "body-foreground", skip_serializing_if = "Option::is_none")]
    pub body_foreground: Option<Rgb>,
    #[serde(rename = "body-background", skip_serializing_if = "Option::is_none")]
    pub body_background: Option<Rgba>,
    #[serde(
        rename = "alternate-foreground",
        skip_serializing_if = "Option::is_none"
    )]
    pub alternate_foreground: Option<Rgb>,
    #[serde(
        rename = "alternate-background",
        skip_serializing_if = "Option::is_none"
    )]
    pub alternate_background: Option<Rgba>,
}

impl TableAppearance {
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }

    /// Apply only user-selected fields; other fields keep following config/theme.
    pub fn overlay(&self, base: &mut Self) {
        macro_rules! apply { ($($field:ident),+ $(,)?) => { $(
            if self.$field.is_some() { base.$field = self.$field; }
        )+ }; }
        apply!(
            border_style,
            border_weight,
            row_lines,
            column_lines,
            outer_border,
            header_separator,
            header_bold,
            banding,
            border_color,
            header_foreground,
            header_background,
            body_foreground,
            body_background,
            alternate_foreground,
            alternate_background
        );
    }
}
