//! Field accents for recognizable package progress reported as plain text.

use super::{
    OperationalPart, OperationalRow, OperationalSpan, MAX_OPERATIONAL_SPANS,
    MAX_ROW_BYTES,
};
use automexia_extension_api::SemanticSeverity;

/// Identify complete, anchored APT and dpkg messages without allocating or
/// interpreting the contents of repository locations and package paths.
pub fn operational_progress(text: &str) -> Option<OperationalRow> {
    if text.len() > MAX_ROW_BYTES {
        return None;
    }
    let starts_at = text.trim_start_matches(|ch: char| ch == '\0' || ch.is_whitespace());
    let offset = text.len() - starts_at.len();
    let text = starts_at.trim_end_matches(|ch: char| ch == '\0' || ch.is_whitespace());

    let mut row = apt_acquire(text)
        .or_else(|| apt_summary(text))
        .or_else(|| dpkg_stage(text))
        .or_else(|| apt_notice(text))
        .or_else(|| apt_space_notice(text))
        .or_else(|| tzdata_report(text))?;
    if text.chars().any(|ch| ch.is_control() && ch != '\t') {
        return None;
    }
    for field in row.spans.iter_mut().flatten() {
        field.start += offset;
        field.end += offset;
    }
    Some(row)
}

impl OperationalRow {
    fn phase(severity: SemanticSeverity, label: &str) -> Self {
        let mut spans = [None; MAX_OPERATIONAL_SPANS];
        spans[0] = Some(span(0, label, OperationalPart::Phase));
        Self { severity, spans }
    }
}

fn span(start: usize, text: &str, role: OperationalPart) -> OperationalSpan {
    OperationalSpan {
        start,
        end: start + text.len(),
        role,
    }
}

fn apt_acquire(text: &str) -> Option<OperationalRow> {
    let severity = match text.as_bytes().get(..4)? {
        b"Get:" | b"Hit:" => SemanticSeverity::Info,
        b"Ign:" => SemanticSeverity::Warning,
        b"Err:" => SemanticSeverity::Error,
        _ => return None,
    };
    let (label, description) = text.split_once(char::is_whitespace)?;
    if !is_digits(label.get(4..)?) {
        return None;
    }
    let description = description.trim_start();
    let repository = description.split_ascii_whitespace().next()?;
    if !is_repository_location(repository) {
        return None;
    }

    let mut row = OperationalRow::phase(severity, label);
    // Optical media labels can contain spaces, so the first token is only a
    // reliable source field for URLs and file locations.
    if !repository.starts_with("cdrom:") {
        row.spans[1] = Some(span(
            text.len() - description.len(),
            repository,
            OperationalPart::Detail,
        ));
    }
    let target = match text.rsplit_once(" [") {
        Some((target, tail)) if tail.strip_suffix(']').is_some_and(is_size) => {
            let start = target.len() + 1;
            row.spans[4] =
                Some(span(start, &text[start..], OperationalPart::Measurement));
            target
        }
        _ => text,
    };
    if let Some([package, version]) = apt_binary_package(target.trim_end()) {
        row.spans[2] = Some(package);
        row.spans[3] = Some(version);
    }
    Some(row)
}

fn is_repository_location(value: &str) -> bool {
    let Some((scheme, location)) = value.split_once(':') else {
        return false;
    };
    match scheme {
        "http" | "https" | "ftp" => location
            .strip_prefix("//")
            .is_some_and(|host| !host.is_empty() && !host.starts_with('/')),
        "file" => location.starts_with('/'),
        "cdrom" => location.starts_with('['),
        _ => false,
    }
}

fn apt_binary_package(target: &str) -> Option<[OperationalSpan; 2]> {
    // Binary rows end with “suite/component index-arch package arch version”.
    // Metadata rows such as InRelease have a different target description.
    let mut fields = target.split_whitespace().rev();
    let version = fields.next()?;
    let architecture = fields.next()?;
    let package = fields.next()?;
    let index_architecture = fields.next()?;
    let suite = fields.next()?;
    if !is_version(version)
        || !is_architecture(architecture)
        || !is_package(package)
        || !is_architecture(index_architecture)
        || !suite.contains('/')
    {
        return None;
    }
    let through_architecture = target.strip_suffix(version)?.trim_end();
    let through_package = through_architecture.strip_suffix(architecture)?.trim_end();
    let start = through_package.len() - package.len();
    Some([
        span(start, package, OperationalPart::Subject),
        span(
            target.len() - version.len(),
            version,
            OperationalPart::Version,
        ),
    ])
}

fn apt_summary(text: &str) -> Option<OperationalRow> {
    let (size, details) = text.strip_prefix("Fetched ")?.split_once(" in ")?;
    let (time, rate) = details.strip_suffix(')')?.split_once(" (")?;
    let mut time_fields = time.split_ascii_whitespace();
    let is_time_part = |part: &str| {
        let index = part
            .find(|ch: char| !ch.is_ascii_digit())
            .unwrap_or(part.len());
        index > 0 && matches!(&part[index..], "s" | "min" | "h" | "d")
    };
    if !is_size(size)
        || !time_fields.next().is_some_and(is_time_part)
        || !time_fields.all(is_time_part)
        || !rate.strip_suffix("/s").is_some_and(is_size)
    {
        return None;
    }
    let mut row = OperationalRow::phase(SemanticSeverity::Success, "Fetched");
    row.spans[1] = Some(span("Fetched ".len(), size, OperationalPart::Measurement));
    row.spans[2] = Some(span(
        text.len() - details.len(),
        time,
        OperationalPart::Measurement,
    ));
    let rate_start = text.len() - rate.len() - 1;
    row.spans[3] = Some(span(rate_start, rate, OperationalPart::Measurement));
    Some(row)
}

fn dpkg_stage(text: &str) -> Option<OperationalRow> {
    if let Some(tail) = text.strip_prefix("(Reading database ... ") {
        let measure = tail
            .strip_suffix(" files and directories currently installed.)")
            .filter(|count| is_digits(count))
            .or_else(|| {
                let percent = tail.strip_suffix(')').unwrap_or(tail);
                percent
                    .strip_suffix('%')
                    .filter(|count| is_digits(count))
                    .map(|_| percent)
            })?;
        let mut row =
            OperationalRow::phase(SemanticSeverity::Info, "(Reading database ...");
        row.spans[1] = Some(span(
            text.len() - tail.len(),
            measure,
            OperationalPart::Measurement,
        ));
        return Some(row);
    }

    let (label, subject) = if let Some(package) =
        text.strip_prefix("Selecting previously unselected package ")
    {
        (
            "Selecting previously unselected package",
            package
                .strip_suffix('.')
                .filter(|package| is_package(package))?,
        )
    } else if let Some(archive) = text.strip_prefix("Preparing to unpack ") {
        let archive = archive.strip_suffix(" ...")?;
        if !(archive.ends_with(".deb") || archive.ends_with(".udeb")) {
            return None;
        }
        ("Preparing to unpack", archive)
    } else {
        return dpkg_versioned_stage(text);
    };

    let mut row = OperationalRow::phase(SemanticSeverity::Info, label);
    row.spans[1] = Some(span(label.len() + 1, subject, OperationalPart::Subject));
    Some(row)
}

fn dpkg_versioned_stage(text: &str) -> Option<OperationalRow> {
    let (label, tail) = [
        "Unpacking",
        "Setting up",
        "Processing triggers for",
        "Removing",
    ]
    .into_iter()
    .find_map(|label| Some((label, text.strip_prefix(label)?.strip_prefix(' ')?)))?;
    let (package, versions) = tail.strip_suffix(" ...")?.split_once(" (")?;
    let versions = versions.strip_suffix(')')?;
    let (version, previous) = versions
        .split_once(") over (")
        .map_or((versions, None), |(version, previous)| {
            (version, Some(previous))
        });
    if !is_package(package)
        || !is_version(version)
        || previous.is_some_and(|v| !is_version(v))
    {
        return None;
    }

    let mut row = OperationalRow::phase(SemanticSeverity::Info, label);
    row.spans[1] = Some(span(label.len() + 1, package, OperationalPart::Subject));
    row.spans[2] = Some(span(
        label.len() + 1 + package.len() + " (".len(),
        version,
        OperationalPart::Version,
    ));
    row.spans[3] = previous.map(|previous| {
        span(
            text.len() - ") ...".len() - previous.len(),
            previous,
            OperationalPart::Detail,
        )
    });
    Some(row)
}

fn apt_notice(text: &str) -> Option<OperationalRow> {
    if text == "Preconfiguring packages ..." {
        return Some(OperationalRow::phase(
            SemanticSeverity::Info,
            "Preconfiguring packages",
        ));
    }
    for label in [
        "Reading package lists...",
        "Building dependency tree...",
        "Reading state information...",
    ] {
        if let Some(tail) = text.strip_prefix(label) {
            return match tail {
                "" => Some(OperationalRow::phase(SemanticSeverity::Info, label)),
                " Done" => Some(OperationalRow::phase(SemanticSeverity::Success, text)),
                _ => {
                    let measure = tail.strip_prefix(' ')?;
                    if !measure.strip_suffix('%').is_some_and(is_digits) {
                        return None;
                    }
                    let mut row = OperationalRow::phase(SemanticSeverity::Info, label);
                    row.spans[1] = Some(span(
                        label.len() + 1,
                        measure,
                        OperationalPart::Measurement,
                    ));
                    Some(row)
                }
            };
        }
    }
    for label in ["Continue?", "Do you want to continue?"] {
        if let Some(tail) = text
            .strip_prefix(label)
            .and_then(|tail| tail.strip_prefix(' '))
        {
            let (choices, answer) = tail.split_once(' ').unwrap_or((tail, ""));
            if !matches!(choices, "[Y/n]" | "[y/N]")
                || !["", "y", "n", "yes", "no"]
                    .iter()
                    .any(|choice| answer.trim().eq_ignore_ascii_case(choice))
            {
                return None;
            }
            let mut row = OperationalRow::phase(SemanticSeverity::Warning, label);
            row.spans[1] = Some(span(label.len() + 1, choices, OperationalPart::Subject));
            return Some(row);
        }
    }
    if text.strip_prefix("E: ").is_some_and(|tail| {
        tail.starts_with("Unable to fetch some archives,")
            || is_apt_fetch_diagnostic(tail)
    }) {
        return Some(OperationalRow::phase(SemanticSeverity::Error, "E:"));
    }
    if text
        .strip_prefix("W: ")
        .is_some_and(is_apt_fetch_diagnostic)
    {
        return Some(OperationalRow::phase(SemanticSeverity::Warning, "W:"));
    }
    None
}

fn apt_space_notice(text: &str) -> Option<OperationalRow> {
    const DOWNLOAD: &str = "Need to get";
    const DISK: &str = "After this operation,";
    if let Some(sizes) = text
        .strip_prefix(DOWNLOAD)
        .and_then(|rest| rest.strip_prefix(' '))
        .and_then(|rest| rest.strip_suffix(" of archives."))
    {
        let (remaining, total) = sizes
            .split_once('/')
            .map_or((sizes, None), |(remaining, total)| (remaining, Some(total)));
        if !is_size(remaining) || total.is_some_and(|size| !is_size(size)) {
            return None;
        }
        let first = DOWNLOAD.len() + 1;
        let mut row = OperationalRow::phase(SemanticSeverity::Info, DOWNLOAD);
        row.spans[1] = Some(span(first, remaining, OperationalPart::Measurement));
        row.spans[2] = total.map(|total| {
            span(
                first + remaining.len() + 1,
                total,
                OperationalPart::Measurement,
            )
        });
        return Some(row);
    }

    let detail = text.strip_prefix(DISK)?.strip_prefix(' ')?;
    let size = detail
        .strip_suffix(" of additional disk space will be used.")
        .or_else(|| detail.strip_suffix(" disk space will be freed."))?;
    if !is_size(size) {
        return None;
    }
    let mut row = OperationalRow::phase(SemanticSeverity::Info, DISK);
    row.spans[1] = Some(span(DISK.len() + 1, size, OperationalPart::Measurement));
    Some(row)
}

fn tzdata_report(text: &str) -> Option<OperationalRow> {
    const ZONE: &str = "Current default time zone:";
    if let Some(zone) = text.strip_prefix(ZONE) {
        let quoted = zone.trim_start();
        let zone = quoted.strip_prefix('\'')?.strip_suffix('\'')?;
        if !zone.as_bytes().first().is_some_and(u8::is_ascii_alphabetic)
            || !zone.split('/').all(|part| {
                !part.is_empty()
                    && part.bytes().all(|byte| {
                        byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'+' | b'-')
                    })
            })
        {
            return None;
        }
        let mut row = OperationalRow::phase(SemanticSeverity::Info, ZONE);
        row.spans[1] = Some(span(
            text.len() - zone.len() - 1,
            zone,
            OperationalPart::Subject,
        ));
        return Some(row);
    }

    for label in ["Local time is now:", "Universal Time is now:"] {
        if let Some(value) = text.strip_prefix(label) {
            let value = value.trim_start().strip_suffix('.')?;
            if !is_reported_time(value) {
                return None;
            }
            let mut row = OperationalRow::phase(SemanticSeverity::Info, label);
            row.spans[1] = Some(span(
                text.len() - value.len() - 1,
                value,
                OperationalPart::Measurement,
            ));
            return Some(row);
        }
    }

    if text == "Run 'dpkg-reconfigure tzdata' if you wish to change it." {
        let mut row = OperationalRow::phase(SemanticSeverity::Info, "Run");
        row.spans[1] = Some(span(
            "Run '".len(),
            "dpkg-reconfigure tzdata",
            OperationalPart::Subject,
        ));
        return Some(row);
    }
    None
}

fn is_reported_time(value: &str) -> bool {
    let mut fields = value.split_ascii_whitespace();
    let [weekday, month, day, clock, zone, year] =
        std::array::from_fn(|_| fields.next().unwrap_or_default());
    let weekdays = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
    let months = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov",
        "Dec",
    ];
    weekdays.contains(&weekday)
        && months.contains(&month)
        && day.len() <= 2
        && is_digits(day)
        && clock.len() == 8
        && clock.as_bytes().get(2) == Some(&b':')
        && clock.as_bytes().get(5) == Some(&b':')
        && clock.split(':').all(is_digits)
        && !zone.is_empty()
        && zone
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'-'))
        && year.len() == 4
        && is_digits(year)
        && fields.next().is_none()
}

fn is_apt_fetch_diagnostic(text: &str) -> bool {
    text.strip_prefix("Failed to fetch ")
        .and_then(|tail| tail.split_ascii_whitespace().next())
        .is_some_and(is_repository_location)
}

fn is_digits(text: &str) -> bool {
    !text.is_empty() && text.bytes().all(|byte| byte.is_ascii_digit())
}

fn is_size(text: &str) -> bool {
    let (number, unit) = text.split_once(' ').unwrap_or(("", ""));
    matches!(unit, "B" | "kB" | "MB" | "GB" | "TB" | "PB")
        && number.split(['.', ',']).all(is_digits)
}

fn is_package(text: &str) -> bool {
    let name = if let Some((name, architecture)) = text.split_once(':') {
        if !is_architecture(architecture) {
            return false;
        }
        name
    } else {
        text
    };
    name.as_bytes()
        .first()
        .is_some_and(u8::is_ascii_alphanumeric)
        && name.bytes().all(|byte| {
            byte.is_ascii_lowercase()
                || byte.is_ascii_digit()
                || matches!(byte, b'+' | b'-' | b'.')
        })
}

fn is_architecture(text: &str) -> bool {
    text.as_bytes()
        .first()
        .is_some_and(u8::is_ascii_alphanumeric)
        && text.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-'
        })
}

fn is_version(text: &str) -> bool {
    text.as_bytes().first().is_some_and(u8::is_ascii_digit)
        && text.bytes().all(|byte| {
            byte.is_ascii_alphanumeric()
                || matches!(byte, b'.' | b'+' | b'-' | b'~' | b':')
        })
}
