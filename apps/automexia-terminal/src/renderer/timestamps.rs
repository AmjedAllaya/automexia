//! Effect-free completion formatting and placement shared by terminal and preview.
use crate::automexia::ui::command_info::{
    pack_with_tag_joins, Band, CompletionPart, CompletionSpan, Label,
};
use rio_backend::config::{colors::Colors, presentation::*};
use rio_backend::crosswords::grid::row::SemanticCommandTimestamp;
use std::ops::Range;

#[derive(Debug, Default)]
pub(crate) struct TimestampText {
    pub text: String,
    pub spans: Vec<CompletionSpan>,
    runs: Vec<(Range<usize>, TimestampPosition)>,
}

impl TimestampText {
    pub(crate) fn new(
        appearance: TimestampAppearance,
        enabled: bool,
        completed_at: Option<SemanticCommandTimestamp>,
        exit_code: Option<i32>,
        elapsed_ms: Option<u64>,
    ) -> Self {
        let result = result_text(appearance, exit_code, elapsed_ms);
        let (date, clock) = if enabled {
            completed_at
                .and_then(|stamp| date_time(appearance, stamp))
                .unwrap_or_default()
        } else {
            (String::new(), String::new())
        };
        use CompletionPart::{Date, Result, Time};
        let order = match appearance.order.unwrap_or_default() {
            TimestampOrder::ResultDateTime => [Result, Date, Time],
            TimestampOrder::ResultTimeDate => [Result, Time, Date],
            TimestampOrder::DateTimeResult => [Date, Time, Result],
            TimestampOrder::TimeDateResult => [Time, Date, Result],
            TimestampOrder::DateResultTime => [Date, Result, Time],
            TimestampOrder::TimeResultDate => [Time, Result, Date],
        };
        let value = |part| match part {
            Result => &result,
            Date => &date,
            Time => &clock,
        };
        let position = |part| match part {
            Result => appearance.result_position.unwrap_or_default(),
            Date => appearance.date_position.unwrap_or_default(),
            Time => appearance.time_position.unwrap_or_default(),
        };
        let mut label = Self::default();
        for lane in [
            TimestampPosition::AboveLeft,
            TimestampPosition::AboveRight,
            TimestampPosition::Left,
            TimestampPosition::Right,
            TimestampPosition::BelowLeft,
            TimestampPosition::BelowRight,
        ] {
            let start = label.text.len();
            let mut previous = None;
            for part in order {
                let text = value(part);
                if text.is_empty() || position(part) != lane {
                    continue;
                }
                let part_start = label.text.len();
                if let Some(previous) = previous {
                    let separator = if part != Result && previous != Result {
                        appearance
                            .date_time_separator
                            .unwrap_or(TimestampSeparator::Space)
                    } else {
                        appearance.separator.unwrap_or_default()
                    };
                    label.text.push_str(separator.text());
                }
                label.text.push_str(text);
                label.spans.push(CompletionSpan {
                    bytes: part_start..label.text.len(),
                    part,
                });
                previous = Some(part);
            }
            if label.text.len() > start {
                label.runs.push((start..label.text.len(), lane));
            }
        }
        label
    }

    /// Pack all metadata through the tag band's existing projection. Temporary
    /// label indices are mapped back to tag indices or the singular completion.
    #[allow(clippy::too_many_arguments)] // Mirrors the existing bounded tag packer.
    pub(crate) fn pack<'a>(
        &'a self,
        tags: &[Label<'a>],
        width: f32,
        gap: f32,
        breaks: &[usize],
        trailing: Option<usize>,
        overlap: f32,
        mut measure: impl FnMut(bool, &str) -> f32,
    ) -> Option<Band> {
        let mut labels = Vec::with_capacity(tags.len() + self.runs.len());
        let mut owners = Vec::with_capacity(labels.capacity());
        let mut row_breaks = Vec::new();
        let add = |position,
                   labels: &mut Vec<Label<'a>>,
                   owners: &mut Vec<(usize, usize)>| {
            if let Some((range, _)) = self.runs.iter().find(|(_, lane)| *lane == position)
            {
                labels.push(Label {
                    text: &self.text[range.clone()],
                    leading: 0.0,
                    padding: 0.0,
                    align_end: position.right(),
                });
                owners.push((tags.len(), range.start));
            }
        };
        add(TimestampPosition::AboveLeft, &mut labels, &mut owners);
        add(TimestampPosition::AboveRight, &mut labels, &mut owners);
        let middle = labels.len();
        if middle > 0 {
            row_breaks.push(middle);
        }
        add(TimestampPosition::Left, &mut labels, &mut owners);
        let tag_start = labels.len();
        for (index, tag) in tags.iter().enumerate() {
            labels.push(Label {
                text: tag.text,
                leading: tag.leading,
                padding: tag.padding,
                align_end: tag.align_end,
            });
            owners.push((index, 0));
        }
        row_breaks.extend(breaks.iter().map(|index| index + tag_start));
        add(TimestampPosition::Right, &mut labels, &mut owners);
        let below = labels.len();
        if below > middle {
            row_breaks.push(below);
        }
        add(TimestampPosition::BelowLeft, &mut labels, &mut owners);
        add(TimestampPosition::BelowRight, &mut labels, &mut owners);
        row_breaks.retain(|index| *index > 0 && *index < labels.len());
        row_breaks.sort_unstable();
        row_breaks.dedup();
        let mut band = pack_with_tag_joins(
            &labels,
            width,
            gap,
            &row_breaks,
            trailing.map(|index| index + tag_start),
            overlap,
            |index, text| measure(owners[index].0 == tags.len(), text),
        )?;
        for fragment in &mut band.fragments {
            let (owner, offset) = owners[fragment.item];
            fragment.item = owner;
            fragment.bytes.start += offset;
            fragment.bytes.end += offset;
        }
        Some(band)
    }
}

fn result_text(
    appearance: TimestampAppearance,
    exit_code: Option<i32>,
    elapsed_ms: Option<u64>,
) -> String {
    let mut parts = Vec::with_capacity(3);
    if appearance.show_status.unwrap_or(true) {
        parts.push(
            match exit_code {
                Some(0) => "✓",
                Some(_) => "×",
                None => "•",
            }
            .into(),
        );
    }
    if appearance.show_exit_code.unwrap_or(false) {
        parts.push(
            exit_code.map_or_else(|| "exit ?".into(), |code| format!("exit {code}")),
        );
    }
    if appearance.show_duration.unwrap_or(true) {
        parts.push(elapsed_ms.map_or_else(
            || "done".into(),
            |ms| match appearance.duration_format.unwrap_or_default() {
                TimestampDurationFormat::Auto => {
                    super::command_results::format_duration(ms)
                }
                TimestampDurationFormat::Milliseconds => format!("{ms}ms"),
                TimestampDurationFormat::Seconds => {
                    format!("{}.{:03}s", ms / 1000, ms % 1000)
                }
                TimestampDurationFormat::Clock => format!(
                    "{:02}:{:02}:{:02}.{:03}",
                    ms / 3_600_000,
                    ms / 60_000 % 60,
                    ms / 1000 % 60,
                    ms % 1000
                ),
            },
        ));
    }
    parts.join("  ")
}

fn date_time(
    appearance: TimestampAppearance,
    stamp: SemanticCommandTimestamp,
) -> Option<(String, String)> {
    use time::{Date, Month, OffsetDateTime, PrimitiveDateTime, Time};
    // The epoch is terminal-owned, but validate conversions rather than indexing
    // calendar arrays or accepting an out-of-range persisted/synthetic value.
    let utc =
        OffsetDateTime::from_unix_timestamp_nanos(i128::from(stamp.unix_ms) * 1_000_000)
            .ok()?;
    let recorded = PrimitiveDateTime::new(
        Date::from_calendar_date(
            i32::from(stamp.year),
            Month::try_from(stamp.month).ok()?,
            stamp.day,
        )
        .ok()?,
        Time::from_hms_milli(
            stamp.hour,
            stamp.minute,
            stamp.second,
            (stamp.unix_ms % 1000) as u16,
        )
        .ok()?,
    );
    let value = if appearance.timezone.unwrap_or_default() == TimestampZone::Utc {
        PrimitiveDateTime::new(utc.date(), utc.time())
    } else {
        recorded
    };
    let mut date = match appearance.date_format.unwrap_or_default() {
        TimestampDateFormat::Hidden => String::new(),
        TimestampDateFormat::DayMonthName => format!(
            "{} {} {:04}",
            value.day(),
            month_name(value.month()),
            value.year()
        ),
        TimestampDateFormat::MonthNameDay => format!(
            "{} {}, {:04}",
            month_name(value.month()),
            value.day(),
            value.year()
        ),
        format => {
            let sep = match appearance.date_separator.unwrap_or_default() {
                TimestampDateSeparator::Dash => '-',
                TimestampDateSeparator::Slash => '/',
                TimestampDateSeparator::Dot => '.',
                TimestampDateSeparator::Space => ' ',
            };
            match format {
                TimestampDateFormat::DayMonthYear => format!(
                    "{:02}{sep}{:02}{sep}{:04}",
                    value.day(),
                    value.month() as u8,
                    value.year()
                ),
                TimestampDateFormat::MonthDayYear => format!(
                    "{:02}{sep}{:02}{sep}{:04}",
                    value.month() as u8,
                    value.day(),
                    value.year()
                ),
                _ => format!(
                    "{:04}{sep}{:02}{sep}{:02}",
                    value.year(),
                    value.month() as u8,
                    value.day()
                ),
            }
        }
    };
    if !date.is_empty() && appearance.weekday.unwrap_or(false) {
        date = format!("{} {date}", value.weekday());
    }
    let mut clock = String::new();
    let format = appearance.time_format.unwrap_or_default();
    if format != TimestampTimeFormat::Hidden {
        let hour = if format == TimestampTimeFormat::Hour12 {
            (value.hour() + 11) % 12 + 1
        } else {
            value.hour()
        };
        clock = format!("{hour:02}:{:02}", value.minute());
        let precision = appearance.precision.unwrap_or_default();
        if precision != TimestampPrecision::Minutes {
            clock.push_str(&format!(":{:02}", value.second()));
        }
        if precision == TimestampPrecision::Milliseconds {
            clock.push_str(&format!(".{:03}", value.millisecond()));
        }
        if format == TimestampTimeFormat::Hour12 {
            clock.push_str(if value.hour() < 12 { " AM" } else { " PM" });
        }
    }
    if appearance.zone_label.unwrap_or(false) && (!date.is_empty() || !clock.is_empty()) {
        let zone = if appearance.timezone.unwrap_or_default() == TimestampZone::Utc {
            "UTC".into()
        } else {
            let offset = recorded.assume_utc().unix_timestamp() - utc.unix_timestamp();
            if offset.abs() <= 86_399 {
                format!(
                    "UTC{}{:02}:{:02}",
                    if offset < 0 { '-' } else { '+' },
                    offset.abs() / 3600,
                    offset.abs() / 60 % 60
                )
            } else {
                "Local".into()
            }
        };
        let target = if clock.is_empty() {
            &mut date
        } else {
            &mut clock
        };
        target.push(' ');
        target.push_str(&zone);
    }
    Some((date, clock))
}

fn month_name(month: time::Month) -> &'static str {
    use time::Month::*;
    match month {
        January => "Jan",
        February => "Feb",
        March => "Mar",
        April => "Apr",
        May => "May",
        June => "Jun",
        July => "Jul",
        August => "Aug",
        September => "Sep",
        October => "Oct",
        November => "Nov",
        December => "Dec",
    }
}

pub(crate) struct TimestampPaint {
    pub appearance: TimestampAppearance,
    pub colors: Colors,
    pub exit_code: Option<i32>,
    pub origin: [f32; 2],
    /// Row height, base text size and tag height in logical pixels.
    pub metrics: [f32; 3],
    pub clip: [f32; 4],
}

impl TimestampPaint {
    pub(crate) fn bounds(
        &self,
        fragment: &crate::automexia::ui::command_info::Fragment,
    ) -> [f32; 4] {
        let [row, _, height] = self.metrics;
        let inset = automexia_ui_model::prompt_context_top_inset(row, height, true)
            .unwrap_or(0.0);
        [
            self.origin[0] + 2.0 + fragment.x,
            self.origin[1] + fragment.row as f32 * row + inset,
            fragment.width,
            height,
        ]
    }

    pub(crate) fn draw(
        &self,
        engine: &mut rio_backend::sugarloaf::text::Text,
        text: &str,
        spans: &[CompletionSpan],
        fragment: &crate::automexia::ui::command_info::Fragment,
    ) {
        let [x, y, _, height] = self.bounds(fragment);
        let font = self.metrics[1]
            * self.appearance.size.unwrap_or_default().scale()
            * fragment.text_scale;
        let mut x = x + fragment.padding + fragment.leading;
        for span in spans {
            let start = span.bytes.start.max(fragment.bytes.start);
            let end = span.bytes.end.min(fragment.bytes.end);
            if start >= end {
                continue;
            }
            let Some(value) = text.get(start..end) else {
                continue;
            };
            let opts = rio_backend::sugarloaf::text::DrawOpts {
                font_size: font,
                color: color(self.appearance, self.colors, self.exit_code, span.part)
                    .map(|c| (c.clamp(0.0, 1.0) * 255.0).round() as u8),
                bold: self.appearance.bold.unwrap_or(false),
                ..Default::default()
            };
            engine.draw_clipped(
                x,
                y + (height - font) * 0.5 - 1.0,
                value,
                &opts,
                self.clip,
            );
            x += engine.measure(value, &opts);
        }
    }

    pub(crate) fn background(&self) -> [f32; 4] {
        self.appearance
            .background
            .map(|rgba| rgba.bytes().map(|c| f32::from(c) / 255.0))
            .unwrap_or([0.0; 4])
    }
}

pub(crate) fn color(
    appearance: TimestampAppearance,
    colors: Colors,
    exit_code: Option<i32>,
    part: CompletionPart,
) -> [f32; 4] {
    let custom = match part {
        CompletionPart::Date => appearance.date_color,
        CompletionPart::Time => appearance.time_color,
        CompletionPart::Result => appearance.result_color,
    };
    custom
        .map(|rgb| rgb.rgba_bytes().map(|channel| f32::from(channel) / 255.0))
        .unwrap_or_else(|| {
            if !appearance.status_colors.unwrap_or(true) {
                colors.foreground
            } else {
                match exit_code {
                    Some(0) => colors.green,
                    Some(_) => colors.red,
                    None => colors.blue,
                }
            }
        })
}

#[cfg(test)]
#[path = "timestamps_tests.rs"]
mod tests;
