//! CLI-only, bounded help layout and terminal dimensions.

use std::env;
use std::io::{self, Write};

use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

const DEFAULT_HELP_WIDTH: usize = 100;
const MIN_HELP_WIDTH: usize = 24;
const MAX_HELP_WIDTH: usize = 240;
const FULL_ART: &str = include_str!("ascii-brand.txt");

fn bounded_dimension(value: &str, minimum: usize, maximum: usize) -> Option<usize> {
    if value.is_empty()
        || value.len() > 3
        || !value.bytes().all(|byte| byte.is_ascii_digit())
    {
        return None;
    }
    value
        .parse::<usize>()
        .ok()
        .filter(|dimension| *dimension >= minimum)
        .map(|dimension| dimension.min(maximum))
}

fn destination_dimension(
    native: Option<usize>,
    hint: Option<usize>,
    minimum: usize,
    maximum: usize,
) -> Option<usize> {
    native
        .filter(|value| *value > 0)
        .map(|value| value.clamp(minimum, maximum))
        .or(hint)
}

#[cfg(windows)]
fn native_dimensions() -> Option<(usize, usize)> {
    use windows_sys::Win32::System::Console::{
        GetConsoleScreenBufferInfo, GetStdHandle, CONSOLE_SCREEN_BUFFER_INFO,
        STD_OUTPUT_HANDLE,
    };

    // SAFETY: GetStdHandle returns a borrowed process handle. The initialized output
    // buffer is read only after GetConsoleScreenBufferInfo reports success.
    let handle = unsafe { GetStdHandle(STD_OUTPUT_HANDLE) };
    if handle.is_null() || handle == windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE
    {
        return None;
    }
    let mut info = std::mem::MaybeUninit::<CONSOLE_SCREEN_BUFFER_INFO>::uninit();
    // SAFETY: `info` points to writable storage for the exact Win32 output struct.
    if unsafe { GetConsoleScreenBufferInfo(handle, info.as_mut_ptr()) } == 0 {
        return None;
    }
    // SAFETY: The preceding Win32 call succeeded and fully initialized `info`.
    let info = unsafe { info.assume_init() };
    let width = i32::from(info.srWindow.Right) - i32::from(info.srWindow.Left) + 1;
    let height = i32::from(info.srWindow.Bottom) - i32::from(info.srWindow.Top) + 1;
    Some((usize::try_from(width).ok()?, usize::try_from(height).ok()?))
}

#[cfg(unix)]
fn native_dimensions() -> Option<(usize, usize)> {
    let mut size = libc::winsize {
        ws_row: 0,
        ws_col: 0,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };
    // SAFETY: TIOCGWINSZ writes the fixed winsize struct and has no retained pointer.
    let result = unsafe { libc::ioctl(libc::STDOUT_FILENO, libc::TIOCGWINSZ, &mut size) };
    (result == 0).then_some((usize::from(size.ws_col), usize::from(size.ws_row)))
}

pub(super) fn dimensions() -> (Option<usize>, Option<usize>) {
    let native = native_dimensions();
    // Shell dimension hints can outlive a pane resize. Trust the destination
    // handle when it is available so automatic artwork never wraps or clips.
    let columns = destination_dimension(
        native.map(|(width, _)| width),
        env::var("COLUMNS")
            .ok()
            .and_then(|value| bounded_dimension(&value, MIN_HELP_WIDTH, MAX_HELP_WIDTH)),
        MIN_HELP_WIDTH,
        MAX_HELP_WIDTH,
    );
    let rows = destination_dimension(
        native.map(|(_, height)| height),
        env::var("LINES")
            .ok()
            .and_then(|value| bounded_dimension(&value, 1, 999)),
        1,
        999,
    );
    (columns, rows)
}

pub(super) fn help_width() -> usize {
    dimensions().0.unwrap_or(DEFAULT_HELP_WIDTH)
}

pub(super) fn compact_wordmark() -> &'static str {
    FULL_ART.lines().last().unwrap_or("Automexia").trim()
}

fn full_art_fits(dimensions: (Option<usize>, Option<usize>), extra_lines: usize) -> bool {
    let (Some(columns), Some(rows)) = dimensions else {
        return false;
    };
    let art_width = FULL_ART.lines().map(visible_width).max().unwrap_or(0);
    let art_rows = FULL_ART.lines().count() + extra_lines;
    columns >= art_width && rows >= art_rows
}

fn render_about(interactive: bool, dimensions: (Option<usize>, Option<usize>)) -> String {
    let brand = if interactive && full_art_fits(dimensions, 1) {
        FULL_ART.to_owned()
    } else {
        format!("{}\n", compact_wordmark())
    };
    format!("{brand}Automexia Terminal {}\n", env!("CARGO_PKG_VERSION"))
}

fn render_logo(interactive: bool, dimensions: (Option<usize>, Option<usize>)) -> String {
    if !interactive || full_art_fits(dimensions, 0) {
        FULL_ART.to_owned()
    } else {
        format!("{}\n", compact_wordmark())
    }
}

fn write_allow_broken_pipe(writer: &mut impl Write, content: &str) -> io::Result<()> {
    match writer.write_all(content.as_bytes()) {
        Err(error) if error.kind() == io::ErrorKind::BrokenPipe => Ok(()),
        result => result,
    }
}

pub(super) fn write_about(writer: &mut impl Write, interactive: bool) -> io::Result<()> {
    // Environment hints are useful for wrapping help through old shell
    // pipelines, but only a measured destination can authorize full artwork.
    let measured = native_dimensions();
    write_allow_broken_pipe(
        writer,
        &render_about(
            interactive,
            (measured.map(|size| size.0), measured.map(|size| size.1)),
        ),
    )
}

pub(super) fn write_logo(writer: &mut impl Write, interactive: bool) -> io::Result<()> {
    let measured = native_dimensions();
    write_allow_broken_pipe(
        writer,
        &render_logo(
            interactive,
            (measured.map(|size| size.0), measured.map(|size| size.1)),
        ),
    )
}

fn visible_width(text: &str) -> usize {
    UnicodeWidthStr::width(text)
}

fn push_wrapped(
    result: &mut String,
    first_prefix: &str,
    rest_prefix: &str,
    text: &str,
    width: usize,
) {
    let mut line = first_prefix.to_owned();
    let mut has_word = false;
    for word in text.split_whitespace() {
        let separator = usize::from(has_word);
        if has_word && visible_width(&line) + separator + visible_width(word) > width {
            result.push_str(line.trim_end());
            result.push('\n');
            line.clear();
            line.push_str(rest_prefix);
            has_word = false;
        }
        if has_word {
            line.push(' ');
        }
        let available = width.saturating_sub(visible_width(&line));
        if visible_width(word) <= available {
            line.push_str(word);
            has_word = true;
            continue;
        }
        for grapheme in word.graphemes(true) {
            if visible_width(&line) + visible_width(grapheme) > width {
                result.push_str(line.trim_end());
                result.push('\n');
                line.clear();
                line.push_str(rest_prefix);
            }
            line.push_str(grapheme);
        }
        has_word = true;
    }
    result.push_str(line.trim_end());
    result.push('\n');
}

pub(super) fn wrap_help(help: &str, width: usize) -> String {
    let width = width.clamp(MIN_HELP_WIDTH, MAX_HELP_WIDTH);
    let mut result = String::with_capacity(help.len() + help.len() / 8);
    for line in help.lines() {
        if visible_width(line) <= width {
            result.push_str(line);
            result.push('\n');
            continue;
        }
        if let Some(usage) = line.strip_prefix("Usage: ") {
            push_wrapped(&mut result, "Usage: ", "       ", usage, width);
            continue;
        }
        let indentation = line.bytes().take_while(|byte| *byte == b' ').count();
        let content = &line[indentation..];
        let description = content
            .as_bytes()
            .windows(2)
            .position(|pair| pair == b"  ")
            .map(|index| {
                indentation
                    + index
                    + content[index..]
                        .bytes()
                        .take_while(|byte| *byte == b' ')
                        .count()
            });
        if let Some(start) = description {
            if width.saturating_sub(start) < 24 {
                let label = line[indentation..start].trim_end();
                push_wrapped(
                    &mut result,
                    &line[..indentation],
                    &line[..indentation],
                    label,
                    width,
                );
                let continuation = " ".repeat(indentation + 2);
                push_wrapped(
                    &mut result,
                    &continuation,
                    &continuation,
                    &line[start..],
                    width,
                );
            } else {
                let prefix = &line[..start];
                push_wrapped(
                    &mut result,
                    prefix,
                    &" ".repeat(start),
                    &line[start..],
                    width,
                );
            }
        } else {
            let prefix = &line[..indentation];
            push_wrapped(&mut result, prefix, prefix, content, width);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dimension_hint_is_bounded_and_ascii_only() {
        assert_eq!(bounded_dimension("40", 24, 240), Some(40));
        assert_eq!(bounded_dimension("999", 24, 240), Some(240));
        for invalid in ["", "0", "23", "-40", "40x", "４０", "1000", " 80", "80 "] {
            assert_eq!(bounded_dimension(invalid, 24, 240), None, "{invalid:?}");
        }
        assert_eq!(
            destination_dimension(Some(80), Some(160), 24, 240),
            Some(80)
        );
        assert_eq!(
            destination_dimension(Some(0), Some(160), 24, 240),
            Some(160)
        );
        assert_eq!(destination_dimension(None, None, 24, 240), None);
    }

    #[test]
    fn wrapping_preserves_unicode_graphemes_and_indented_descriptions() {
        let source = "Commands:\n  sample    Café e\u{301} 👩‍💻 and nested documentation descriptions that must stay aligned\n";
        let wrapped = wrap_help(source, 40);
        assert!(wrapped.contains("Café e\u{301} 👩‍💻"));
        assert!(wrapped.lines().all(|line| visible_width(line) <= 40));
        assert!(wrapped.lines().any(|line| line.starts_with("            ")));
    }

    #[test]
    fn narrow_commands_stack_the_label_and_keep_all_source_characters() {
        let source = "  google             Open a Google search in the default browser\n";
        let wrapped = wrap_help(source, 40);
        assert!(wrapped.starts_with("  google\n    Open a Google search"));
        assert!(wrapped.lines().all(|line| visible_width(line) <= 40));
        let non_whitespace = |text: &str| {
            text.chars()
                .filter(|character| !character.is_whitespace())
                .collect::<String>()
        };
        assert_eq!(non_whitespace(&wrapped), non_whitespace(source));
        let option = "      --title-placeholder <title-placeholder>\n";
        assert_eq!(visible_width(option.trim_end()), 45);
        let wrapped_option = wrap_help(option, 40);
        assert!(
            wrapped_option
                .lines()
                .all(|line| line.chars().count() <= 40),
            "{wrapped_option:?}"
        );
    }

    #[test]
    fn exact_art_has_one_wordmark_and_compact_fallbacks_fit() {
        use sha2::{Digest, Sha256};

        let digest = Sha256::digest(FULL_ART.as_bytes());
        assert_eq!(
            &digest[..],
            &[
                0xe1, 0x06, 0x20, 0x7c, 0x00, 0x53, 0x2d, 0xa1, 0xc7, 0x06, 0x0e, 0x40,
                0x87, 0x64, 0x86, 0x89, 0x36, 0xdd, 0xc2, 0x52, 0x94, 0xf4, 0x33, 0x81,
                0xfc, 0xba, 0x4c, 0x44, 0x86, 0x12, 0x38, 0x21,
            ]
        );
        assert!(FULL_ART.starts_with("                                 +++++++*\n"));
        assert!(FULL_ART.ends_with("\n    A U T O M E X I A\n"));
        assert_eq!(FULL_ART.matches("A U T O M E X I A").count(), 1);
        let art_width = FULL_ART.lines().map(visible_width).max().unwrap();
        let art_rows = FULL_ART.lines().count();
        assert!(render_logo(true, (Some(art_width), Some(art_rows))).contains("+++++++*"));
        assert_eq!(
            render_logo(true, (Some(40), Some(12))),
            "A U T O M E X I A\n"
        );
        assert_eq!(render_logo(true, (None, None)), "A U T O M E X I A\n");
        assert_eq!(render_logo(false, (None, None)), FULL_ART);
        assert!(render_about(true, (Some(art_width), Some(art_rows + 1)))
            .contains("+++++++*"));
        assert!(render_about(false, (None, None)).starts_with("A U T O M E X I A\n"));
        assert!(render_about(false, (None, None)).contains(env!("CARGO_PKG_VERSION")));
    }

    #[test]
    fn branding_treats_broken_pipe_as_an_expected_reader_exit() {
        struct ClosedReader;
        impl Write for ClosedReader {
            fn write(&mut self, _: &[u8]) -> io::Result<usize> {
                Err(io::ErrorKind::BrokenPipe.into())
            }
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }
        assert!(write_allow_broken_pipe(&mut ClosedReader, FULL_ART).is_ok());
    }
}
