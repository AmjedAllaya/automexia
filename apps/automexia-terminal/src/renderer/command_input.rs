//! Display-only accents for plain integrated command input. This deliberately
//! does not parse a shell language, validate commands, or consult the environment.
use rio_backend::crosswords::grid::row::{PromptInputShell, Row};
use rio_backend::crosswords::square::{Square, Wide};
use std::ops::Range;

pub(crate) const ACCENT: [u8; 4] = [181, 140, 255, 255];
const MAX_ROWS: usize = 512;
const MAX_CELLS: usize = 65_536;
const MAX_INPUT_BYTES: usize = 8_192;

#[derive(Default)]
pub(crate) struct InputAccents {
    pub(crate) rows: Vec<Vec<bool>>,
    pending: Vec<Vec<bool>>,
    text: String,
    cells: Vec<(usize, usize, usize)>,
    ranges: Vec<Range<usize>>,
}

impl InputAccents {
    /// Called only with source/mode damage by the existing pane snapshot owner.
    /// A later wrapped edit invalidates every earlier fragment it recolors.
    pub(crate) fn refresh(
        &mut self,
        rows: &mut [Row<Square>],
        cols: usize,
        eligible: bool,
    ) {
        self.pending.resize_with(rows.len().min(MAX_ROWS), Vec::new);
        for row in &mut self.pending {
            row.clear();
        }
        self.text.clear();
        self.cells.clear();
        let mut shell = None;
        let mut complete = false;
        let mut scanned = 0usize;
        if eligible {
            for (y, row) in rows.iter().take(MAX_ROWS).enumerate() {
                let count = cols.min(row.len());
                scanned = scanned.saturating_add(count);
                if scanned > MAX_CELLS {
                    break;
                }
                let Some(input) = row
                    .semantic_input
                    .filter(|input| input.shell != PromptInputShell::Native)
                else {
                    self.text.clear();
                    self.cells.clear();
                    shell = None;
                    continue;
                };
                if !input.continuation {
                    self.text.clear();
                    self.cells.clear();
                    shell = Some(input.shell);
                    complete = true;
                } else if shell != Some(input.shell) {
                    complete = false;
                }
                if !complete || input.column >= count {
                    continue;
                }
                let end = row.inner[..count]
                    .iter()
                    .rposition(|cell| !matches!(cell.c(), '\0' | ' '))
                    .map_or(input.column, |index| index + 1);
                let wraps = row
                    .inner
                    .get(count.saturating_sub(1))
                    .is_some_and(|cell| cell.wrapline());
                let end = if wraps { count } else { end };
                for (x, cell) in row.inner.iter().enumerate().take(end).skip(input.column)
                {
                    if matches!(cell.wide(), Wide::Spacer | Wide::LeadingSpacer) {
                        continue;
                    }
                    let c = if cell.c() == '\0' { ' ' } else { cell.c() };
                    if self.text.len() + c.len_utf8() > MAX_INPUT_BYTES {
                        complete = false;
                        break;
                    }
                    self.cells.push((self.text.len(), y, x));
                    self.text.push(c);
                }
                if !wraps {
                    if complete {
                        accent_ranges(&self.text, input.shell, &mut self.ranges);
                        let mut range = 0;
                        for &(byte, y, x) in &self.cells {
                            while self.ranges.get(range).is_some_and(|r| byte >= r.end) {
                                range += 1;
                            }
                            if self.ranges.get(range).is_some_and(|r| r.contains(&byte)) {
                                let mask = &mut self.pending[y];
                                mask.resize(mask.len().max(x + 1), false);
                                mask[x] = true;
                            }
                        }
                    }
                    self.text.clear();
                    self.cells.clear();
                    shell = None;
                }
            }
        }
        self.text.clear();
        self.cells.clear();
        for (y, row) in rows.iter_mut().enumerate() {
            if self.rows.get(y) != self.pending.get(y) {
                row.dirty = true;
            }
        }
        std::mem::swap(&mut self.rows, &mut self.pending);
    }
}

/// Accent simple command words and unquoted options; quoted arguments, values,
/// redirection targets, assignment prefixes and arguments after `--` stay plain.
/// Complex substitutions are intentionally left to native shell highlighters.
fn accent_ranges(text: &str, shell: PromptInputShell, ranges: &mut Vec<Range<usize>>) {
    ranges.clear();
    if text.len() > MAX_INPUT_BYTES
        || text.contains(['`', '\n', '\r'])
        || text.contains("$(")
        || text.contains("${")
    {
        return;
    }
    let bytes = text.as_bytes();
    let mut index = 0;
    let mut command = true;
    let mut options = true;
    let mut redirect = false;
    while index < bytes.len() {
        let c = bytes[index];
        if c.is_ascii_whitespace() {
            index += 1;
            continue;
        }
        if shell == PromptInputShell::Posix && c == b'#' {
            break;
        }
        if matches!(c, b'|' | b'&') || (shell == PromptInputShell::Posix && c == b';') {
            command = true;
            options = true;
            redirect = false;
            index += 1;
            continue;
        }
        if matches!(c, b'<' | b'>') {
            redirect = true;
            index += 1;
            continue;
        }
        let start = index;
        let mut quote = None;
        let mut quoted = false;
        while index < bytes.len() {
            let c = bytes[index];
            let escape = if shell == PromptInputShell::Cmd {
                b'^'
            } else {
                b'\\'
            };
            if c == escape && quote != Some(b'\'') {
                index = (index + 2).min(bytes.len());
                continue;
            }
            if quote == Some(c) {
                quote = None;
                quoted = true;
                index += 1;
                continue;
            }
            if quote.is_none()
                && (c == b'"' || (shell == PromptInputShell::Posix && c == b'\''))
            {
                quote = Some(c);
                quoted = true;
                index += 1;
                continue;
            }
            if quote.is_none()
                && (c.is_ascii_whitespace()
                    || matches!(c, b'|' | b'&' | b'<' | b'>')
                    || (shell == PromptInputShell::Posix && c == b';'))
            {
                break;
            }
            index += 1;
        }
        let word = &text[start..index];
        // A numeric redirection descriptor is not a command word.
        if bytes.get(index).is_some_and(|c| matches!(c, b'<' | b'>'))
            && word.bytes().all(|c| c.is_ascii_digit())
        {
            continue;
        }
        if redirect {
            redirect = false;
            continue;
        }
        if command && shell == PromptInputShell::Posix && assignment(word) {
            continue;
        }
        let option = options
            && !quoted
            && (word == "--"
                || word.strip_prefix('-').is_some_and(|s| {
                    s.starts_with('-')
                        || s.as_bytes().first().is_some_and(u8::is_ascii_alphabetic)
                })
                || (shell == PromptInputShell::Cmd
                    && word.strip_prefix('/').is_some_and(|s| {
                        !s.is_empty()
                            && !s.contains(['/', '\\'])
                            && s.as_bytes()
                                .first()
                                .is_some_and(|c| c.is_ascii_alphabetic() || *c == b'?')
                    })));
        if quote.is_none() && (command || option) {
            // Values do not acquire the option-name accent.
            let end = if option {
                word.find(['=', ':']).map_or(index, |offset| start + offset)
            } else {
                index
            };
            ranges.push(start..end);
        }
        command = false;
        if word == "--" {
            options = false;
        }
    }
}

fn assignment(word: &str) -> bool {
    let Some((name, _)) = word.split_once('=') else {
        return false;
    };
    !name.is_empty()
        && name.bytes().enumerate().all(|(i, c)| {
            c == b'_' || c.is_ascii_alphabetic() || (i > 0 && c.is_ascii_digit())
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn accented(text: &str, shell: PromptInputShell) -> Vec<&str> {
        let mut spans = Vec::new();
        accent_ranges(text, shell, &mut spans);
        spans.iter().map(|r| &text[r.clone()]).collect()
    }

    #[test]
    fn command_and_options_respect_quotes_dialects_and_boundaries() {
        use PromptInputShell::*;
        assert_eq!(
            accented("docker ps -a --format 'x -a'", Posix),
            ["docker", "-a", "--format"]
        );
        assert_eq!(accented("DIR /W /? C:/work/file", Cmd), ["DIR", "/W", "/?"]);
        assert_eq!(accented("ls /tmp -- -file", Posix), ["ls", "--"]);
        assert_eq!(
            accented("MODE=dev cargo test --locked | sort -r", Posix),
            ["cargo", "--locked", "sort", "-r"]
        );
        assert_eq!(accented("echo \"unfinished --option", Posix), ["echo"]);
        assert_eq!(accented("echo -1.5 > -filename", Posix), ["echo"]);
        assert_eq!(accented("> file command -a", Posix), ["command", "-a"]);
        assert_eq!(accented("écho --日本語", Posix), ["écho", "--日本語"]);
        assert!(accented("echo $(command -a)", Posix).is_empty());
        assert!(accented(&"x".repeat(MAX_INPUT_BYTES + 1), Posix).is_empty());
        assert_eq!(
            accented("echo --name=value -- -foo", Posix),
            ["echo", "--name", "--"]
        );
        assert_eq!(
            accented("2>file command -a # comment | false -x", Posix),
            ["command", "-a"]
        );
        assert_eq!(accented("echo /name:value", Cmd), ["echo", "/name"]);
    }

    fn input_row(
        text: &str,
        width: usize,
        start: usize,
        continuation: bool,
        wrapped: bool,
    ) -> Row<Square> {
        let mut row = Row::<Square>::new(width);
        for (cell, c) in row.inner.iter_mut().zip(text.chars()) {
            cell.set_c(c);
        }
        row.semantic_input = Some(rio_backend::crosswords::grid::row::SemanticInput {
            column: start,
            shell: PromptInputShell::Posix,
            command_complete: false,
            continuation,
        });
        row.inner[width - 1].set_wrapline(wrapped);
        row
    }

    #[test]
    fn input_accents_follow_wrapped_quotes_edits_and_clipping() {
        let mut rows = vec![
            input_row("> docker", 8, 2, false, true),
            input_row(" '-a' -x", 8, 0, true, false),
        ];
        let mut cache = InputAccents::default();
        cache.refresh(&mut rows, 8, true);
        assert_eq!(
            cache.rows[0],
            [false, false, true, true, true, true, true, true]
        );
        assert_eq!(
            cache.rows[1],
            [false, false, false, false, false, false, true, true]
        );
        // Editing a later fragment can suppress accents in an earlier fragment.
        rows[1] = input_row(" $(ls)", 8, 0, true, false);
        rows[0].dirty = false;
        cache.refresh(&mut rows, 8, true);
        assert!(cache.rows.iter().all(Vec::is_empty));
        assert!(rows[0].dirty);
        cache.refresh(&mut rows[1..], 8, true);
        assert!(
            cache.rows.iter().all(Vec::is_empty),
            "offscreen starts are not guessed"
        );
        let mut output = input_row("docker", 8, 0, false, false);
        output.semantic_input = None;
        cache.refresh(&mut [output], 8, true);
        assert!(cache.rows.iter().all(Vec::is_empty));
    }

    #[test]
    fn input_accent_cache_is_bounded_mode_local_and_unicode_safe() {
        let mut rows = vec![input_row("> écho", 8, 2, false, false)];
        let mut cache = InputAccents::default();
        cache.refresh(&mut rows, 8, true);
        assert_eq!(cache.rows[0], [false, false, true, true, true, true]);
        rows[0].dirty = false;
        cache.refresh(&mut rows, 8, true);
        assert!(!rows[0].dirty, "unchanged accent masks do not force redraw");
        cache.refresh(&mut rows, 8, false);
        assert!(rows[0].dirty);
        assert!(cache.rows[0].is_empty());
        let mut oversized = vec![input_row(
            &"a".repeat(MAX_INPUT_BYTES + 1),
            MAX_INPUT_BYTES + 1,
            0,
            false,
            false,
        )];
        cache.refresh(&mut oversized, MAX_INPUT_BYTES + 1, true);
        assert!(cache.rows[0].is_empty());
        let mut many = vec![input_row("cmd", 256, 0, false, false); MAX_ROWS + 1];
        cache.refresh(&mut many, 256, true);
        assert!(cache.rows.len() <= MAX_ROWS);
        assert!(cache.rows.iter().map(Vec::len).sum::<usize>() <= MAX_CELLS);
        for text in [
            "\\é -a",
            "echo \\界 -x",
            "cmd \"界",
            "🚀 -a",
            "'é' -x",
            "^界 /a",
        ] {
            for shell in [PromptInputShell::Posix, PromptInputShell::Cmd] {
                let mut ranges = Vec::new();
                accent_ranges(text, shell, &mut ranges);
                assert!(ranges
                    .iter()
                    .all(|r| text.is_char_boundary(r.start)
                        && text.is_char_boundary(r.end)));
            }
        }
    }
}
