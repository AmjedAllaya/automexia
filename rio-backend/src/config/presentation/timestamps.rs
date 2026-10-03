//! Bounded, optional completion-label choices. Omitted values follow configuration.
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
choices!(TimestampDateFormat, YearMonthDay,
    YearMonthDay => ("year-month-day", "Year / month / day"),
    DayMonthYear => ("day-month-year", "Day / month / year"),
    MonthDayYear => ("month-day-year", "Month / day / year"),
    DayMonthName => ("day-month-name", "3 Oct 2026"),
    MonthNameDay => ("month-name-day", "Oct 3, 2026"),
    Hidden => ("hidden", "Hidden"),
);
choices!(TimestampDateSeparator, Dash,
    Dash => ("dash", "Dash  -"), Slash => ("slash", "Slash  /"),
    Dot => ("dot", "Dot  ."), Space => ("space", "Space"),
);
choices!(TimestampTimeFormat, Hour24,
    Hour24 => ("24-hour", "24 hour"), Hour12 => ("12-hour", "12 hour · AM / PM"),
    Hidden => ("hidden", "Hidden"),
);
choices!(TimestampPrecision, Seconds,
    Minutes => ("minutes", "Hours and minutes"),
    Seconds => ("seconds", "Include seconds"),
    Milliseconds => ("milliseconds", "Include milliseconds"),
);
choices!(TimestampZone, Recorded,
    Recorded => ("recorded", "Local time at completion"), Utc => ("utc", "UTC"),
);
choices!(TimestampPosition, Right,
    Left => ("left", "Before tags · left"), Right => ("right", "After tags · right"),
    AboveLeft => ("above-left", "Above tags · left"),
    AboveRight => ("above-right", "Above tags · right"),
    BelowLeft => ("below-left", "Below tags · left"),
    BelowRight => ("below-right", "Below tags · right"),
);
impl TimestampPosition {
    pub const fn right(self) -> bool {
        matches!(self, Self::Right | Self::AboveRight | Self::BelowRight)
    }
}
choices!(TimestampOrder, ResultDateTime,
    ResultDateTime => ("result-date-time", "Result · Date · Time"),
    ResultTimeDate => ("result-time-date", "Result · Time · Date"),
    DateTimeResult => ("date-time-result", "Date · Time · Result"),
    TimeDateResult => ("time-date-result", "Time · Date · Result"),
    DateResultTime => ("date-result-time", "Date · Result · Time"),
    TimeResultDate => ("time-result-date", "Time · Result · Date"),
);
choices!(TimestampSeparator, Dot,
    Dot => ("dot", "Dot  ·"), Space => ("space", "Space"),
    Pipe => ("pipe", "Bar  |"), Dash => ("dash", "Dash  –"),
);
impl TimestampSeparator {
    pub const fn text(self) -> &'static str {
        match self {
            Self::Dot => "  ·  ",
            Self::Space => " ",
            Self::Pipe => " | ",
            Self::Dash => " – ",
        }
    }
}
choices!(TimestampDurationFormat, Auto,
    Auto => ("auto", "Automatic · 104ms / 2s"),
    Milliseconds => ("milliseconds", "Milliseconds · 104ms"),
    Seconds => ("seconds", "Seconds · 0.104s"),
    Clock => ("clock", "Clock · 00:00:00.104"),
);
choices!(TimestampSize, Normal,
    Small => ("small", "Small"), Normal => ("normal", "Normal"), Large => ("large", "Large"),
);
impl TimestampSize {
    pub const fn scale(self) -> f32 {
        match self {
            Self::Small => 0.8,
            Self::Normal => 1.0,
            Self::Large => 1.15,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub struct TimestampAppearance {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_format: Option<TimestampDateFormat>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_separator: Option<TimestampDateSeparator>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_format: Option<TimestampTimeFormat>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub precision: Option<TimestampPrecision>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timezone: Option<TimestampZone>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weekday: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub zone_label: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_position: Option<TimestampPosition>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_position: Option<TimestampPosition>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result_position: Option<TimestampPosition>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order: Option<TimestampOrder>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub separator: Option<TimestampSeparator>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_time_separator: Option<TimestampSeparator>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub show_status: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub show_duration: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub show_exit_code: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_format: Option<TimestampDurationFormat>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<TimestampSize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bold: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_colors: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_color: Option<Rgb>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_color: Option<Rgb>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result_color: Option<Rgb>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background: Option<Rgba>,
}
impl TimestampAppearance {
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }
    pub fn overlay(&self, base: &mut Self) {
        macro_rules! apply { ($($field:ident),+ $(,)?) => { $(
            if self.$field.is_some() { base.$field = self.$field; }
        )+ }; }
        apply!(
            date_format,
            date_separator,
            time_format,
            precision,
            timezone,
            weekday,
            zone_label,
            date_position,
            time_position,
            result_position,
            order,
            separator,
            date_time_separator,
            show_status,
            show_duration,
            show_exit_code,
            duration_format,
            size,
            bold,
            status_colors,
            date_color,
            time_color,
            result_color,
            background
        );
    }
}
