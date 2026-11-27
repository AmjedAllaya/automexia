//! Display-only accents for plain integrated command input. This deliberately
//! does not parse a shell language, validate commands, or consult the environment.
use super::ui_theme::{color_u8, UiTheme};
use rio_backend::crosswords::grid::row::{PromptInputShell, Row};
use rio_backend::crosswords::square::{CellFlags, Square, Wide};
use std::ops::Range;

const MAX_ROWS: usize = 512;
const MAX_CELLS: usize = 65_536;
const MAX_INPUT_BYTES: usize = 8_192;

#[derive(Default)]
pub(crate) struct InputAccents {
    pub(crate) rows: Vec<Vec<Option<CommandSpanKind>>>,
    pending: Vec<Vec<Option<CommandSpanKind>>>,
    text: String,
    cells: Vec<(usize, usize, usize)>,
    ranges: Vec<(CommandSpanKind, Range<usize>)>,
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
        let mut owner = None;
        let mut soft_wrap = false;
        // Both an over-budget soft line and unresolved hard-line syntax block
        // continuation coloring until another exact input start is present.
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
                    owner = None;
                    continue;
                };
                let region = (input.shell, row.semantic_prompt_id);
                if !input.continuation {
                    self.text.clear();
                    self.cells.clear();
                    owner = Some(region);
                    complete = true;
                } else if owner != Some(region) || (!soft_wrap && region.1.is_none()) {
                    self.text.clear();
                    self.cells.clear();
                    owner = None;
                    continue;
                }
                if !complete || input.column > count {
                    owner = None;
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
                soft_wrap = wraps;
                let end = if wraps { count } else { end };
                for (x, cell) in row.inner.iter().enumerate().take(end).skip(input.column)
                {
                    if matches!(cell.wide(), Wide::Spacer | Wide::LeadingSpacer)
                        || cell.contains_cell_flag(CellFlags::REFLOW_PADDING)
                    {
                        continue;
                    }
                    let c = if cell.is_bg_only() || cell.c() == '\0' {
                        ' '
                    } else {
                        cell.c()
                    };
                    if self.text.len() + c.len_utf8() > MAX_INPUT_BYTES {
                        complete = false;
                        break;
                    }
                    self.cells.push((self.text.len(), y, x));
                    self.text.push(c);
                }
                if !wraps {
                    if complete {
                        self.ranges.clear();
                        let ranges = &mut self.ranges;
                        complete =
                            visit_command_spans(&self.text, input.shell, |kind, span| {
                                ranges.push((kind, span));
                            });
                        let mut range = 0;
                        for &(byte, y, x) in &self.cells {
                            while self
                                .ranges
                                .get(range)
                                .is_some_and(|(_, r)| byte >= r.end)
                            {
                                range += 1;
                            }
                            if let Some((kind, _)) =
                                self.ranges.get(range).filter(|(_, r)| r.contains(&byte))
                            {
                                let roles = &mut self.pending[y];
                                roles.resize(roles.len().max(x + 1), None);
                                roles[x] = Some(*kind);
                            }
                        }
                    }
                    self.text.clear();
                    self.cells.clear();
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CommandSpanKind {
    Command,
    Option,
    Argument,
    Variable,
    Quoted,
    Operator,
    Comment,
}

impl CommandSpanKind {
    pub(crate) fn ink(self, theme: &UiTheme) -> [u8; 4] {
        color_u8(match self {
            Self::Command => theme.accent,
            Self::Option => theme.purple,
            Self::Argument => theme.warning,
            Self::Variable => theme.danger,
            Self::Quoted => theme.success,
            Self::Operator => theme.blue,
            Self::Comment => theme.muted_text,
        })
    }
}

/// Visit ordered byte ranges for simple, display-only shell accents. Return
/// whether another hard line owned by this prompt can start with fresh state.
pub(super) fn visit_command_spans(
    text: &str,
    shell: PromptInputShell,
    mut visit: impl FnMut(CommandSpanKind, Range<usize>),
) -> bool {
    use CommandSpanKind::{Command, Comment, Operator, Option, Variable};

    if text.len() > MAX_INPUT_BYTES
        || text.contains('`')
        || text.contains("$(")
        || text.contains("${")
        || text.chars().any(|ch| ch.is_control() && ch != '\t')
    {
        return false;
    }
    let bytes = text.as_bytes();
    let mut index = 0;
    let mut command = true;
    let mut options = true;
    let mut redirect = false;
    let mut assignments = false;
    let mut here_document = false;
    while index < bytes.len() {
        let c = bytes[index];
        if c.is_ascii_whitespace() {
            index += 1;
            continue;
        }
        if shell == PromptInputShell::Posix && c == b'#' {
            visit(Comment, index..bytes.len());
            break;
        }
        if let Some((end, heredoc)) = redirect_end(bytes, index, shell) {
            here_document |= heredoc;
            redirect = true;
            visit(Operator, index..end);
            index = end;
            continue;
        }
        if matches!(c, b'|' | b'&') || (shell == PromptInputShell::Posix && c == b';') {
            command = true;
            options = true;
            redirect = false;
            assignments = false;
            let width = 1 + usize::from(bytes.get(index + 1) == Some(&c));
            visit(Operator, index..index + width);
            index += width;
            continue;
        }
        let start = index;
        let Some(end) = word_end(text, start, shell) else {
            if c == b'"' || (shell == PromptInputShell::Posix && c == b'\'') {
                visit(CommandSpanKind::Quoted, start..bytes.len());
            }
            return false;
        };
        index = end;
        let word = &text[start..index];
        if redirect {
            visit(value_kind(word, shell), start..index);
            redirect = false;
            continue;
        }
        if command && shell == PromptInputShell::Cmd && word.eq_ignore_ascii_case("rem") {
            visit(Comment, start..bytes.len());
            break;
        }
        if let Some((name, value)) = word.split_once('=').filter(|(name, _)| {
            (command || assignments)
                && shell == PromptInputShell::Posix
                && identifier(name)
        }) {
            let separator = start + name.len();
            visit(Variable, start..separator);
            visit(Operator, separator..separator + 1);
            if !value.is_empty() {
                visit(value_kind(value, shell), separator + 1..index);
            }
            continue;
        }
        let mut parts = word.splitn(2, |c| {
            c == '=' || (shell == PromptInputShell::Cmd && c == ':')
        });
        let name = parts.next().unwrap_or(word);
        if options && is_option(name, shell) {
            let separator = start + name.len();
            visit(Option, start..separator);
            if let Some(value) = parts.next() {
                visit(Operator, separator..separator + 1);
                if !value.is_empty() {
                    visit(value_kind(value, shell), separator + 1..index);
                }
            }
        } else if command {
            visit(Command, start..index);
            assignments = matches!(
                word,
                "export" | "readonly" | "local" | "declare" | "typeset"
            );
        } else {
            visit(value_kind(word, shell), start..index);
        }
        command = false;
        if word == "--" {
            options = false;
        }
    }
    !here_document && !redirect
}

fn word_end(text: &str, mut index: usize, shell: PromptInputShell) -> Option<usize> {
    let bytes = text.as_bytes();
    let escape = if shell == PromptInputShell::Cmd {
        b'^'
    } else {
        b'\\'
    };
    let mut quote = None;
    while let Some(&c) = bytes.get(index) {
        if c == escape && quote != Some(b'\'') {
            let escaped = text.get(index + 1..)?.chars().next()?;
            index += 1 + escaped.len_utf8();
            continue;
        }
        if quote == Some(c) {
            quote = None;
        } else if quote.is_none() {
            if c == b'"' || (shell == PromptInputShell::Posix && c == b'\'') {
                quote = Some(c);
            } else if c.is_ascii_whitespace()
                || matches!(c, b'|' | b'&' | b'<' | b'>')
                || (shell == PromptInputShell::Posix && c == b';')
            {
                break;
            }
        }
        index += 1;
    }
    quote.is_none().then_some(index)
}

fn redirect_end(
    bytes: &[u8],
    start: usize,
    shell: PromptInputShell,
) -> Option<(usize, bool)> {
    let tail = bytes.get(start..)?;
    let descriptor = tail.iter().take_while(|byte| byte.is_ascii_digit()).count();
    let tail = tail.get(descriptor..)?;
    let posix = shell == PromptInputShell::Posix;
    let (len, heredoc) = match tail {
        [b'<', b'<', b'<', ..] if posix => (3, false),
        [b'<', b'<', b'-', ..] if posix => (3, true),
        [b'<', b'<', ..] if posix => (2, true),
        [b'&', b'>', b'>', ..] if posix && descriptor == 0 => (3, false),
        [b'&', b'>', ..] if posix && descriptor == 0 => (2, false),
        [b'>', b'>' | b'&' | b'|', ..] | [b'<', b'&' | b'>', ..] => (2, false),
        [b'<' | b'>', ..] => (1, false),
        _ => return None,
    };
    Some((start + descriptor + len, heredoc))
}

fn is_option(name: &str, shell: PromptInputShell) -> bool {
    !name.contains(['\'', '"'])
        && (name.strip_prefix('-').is_some_and(|tail| {
            tail.starts_with('-')
                || tail.as_bytes().first().is_some_and(u8::is_ascii_alphabetic)
        }) || (shell == PromptInputShell::Cmd
            && name.strip_prefix('/').is_some_and(|tail| {
                !tail.contains(['/', '\\'])
                    && tail
                        .as_bytes()
                        .first()
                        .is_some_and(|c| c.is_ascii_alphabetic() || *c == b'?')
            })))
}

fn value_kind(value: &str, shell: PromptInputShell) -> CommandSpanKind {
    let variable = match shell {
        PromptInputShell::Posix => value.strip_prefix('$'),
        PromptInputShell::Cmd => value
            .strip_prefix('%')
            .and_then(|name| name.strip_suffix('%')),
        PromptInputShell::Native => None,
    };
    if value.len() >= 2
        && (value.starts_with('"') && value.ends_with('"')
            || (shell == PromptInputShell::Posix
                && value.starts_with('\'')
                && value.ends_with('\'')))
    {
        CommandSpanKind::Quoted
    } else if variable.is_some_and(identifier) {
        CommandSpanKind::Variable
    } else {
        CommandSpanKind::Argument
    }
}

fn identifier(name: &str) -> bool {
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
        visit_command_spans(text, shell, |kind, range| {
            if matches!(kind, CommandSpanKind::Command | CommandSpanKind::Option) {
                spans.push(&text[range]);
            }
        });
        spans
    }

    #[test]
    fn command_arguments_values_and_operators_keep_distinct_roles() {
        use CommandSpanKind::*;
        use PromptInputShell::Posix;
        let text = "MODE=dev sudo apt install nodejs --label=\"équipe 1\" 2>&1 | echo $MODE # ready";
        let mut actual = Vec::new();
        visit_command_spans(text, Posix, |kind, range| actual.push((kind, &text[range])));
        assert_eq!(
            actual,
            [
                (Variable, "MODE"),
                (Operator, "="),
                (Argument, "dev"),
                (Command, "sudo"),
                (Argument, "apt"),
                (Argument, "install"),
                (Argument, "nodejs"),
                (Option, "--label"),
                (Operator, "="),
                (Quoted, "\"équipe 1\""),
                (Operator, "2>&"),
                (Argument, "1"),
                (Operator, "|"),
                (Command, "echo"),
                (Variable, "$MODE"),
                (Comment, "# ready"),
            ],
        );
    }

    #[test]
    fn an_open_quote_can_be_colored_while_typing_without_guessing_the_next_line() {
        use CommandSpanKind::{Command, Quoted};
        let text = "echo \"partially typed";
        let mut actual = Vec::new();
        let complete =
            visit_command_spans(text, PromptInputShell::Posix, |kind, range| {
                actual.push((kind, &text[range]));
            });
        assert!(!complete);
        assert_eq!(actual, [(Command, "echo"), (Quoted, "\"partially typed")]);
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
        row.semantic_prompt_id = Some(7);
        row.inner[width - 1].set_wrapline(wrapped);
        row
    }

    #[test]
    fn same_prompt_hard_lines_keep_roles_in_live_input_and_history() {
        use CommandSpanKind::{Argument, Command};
        let mut rows = vec![
            input_row("> docker", 20, 2, false, false),
            input_row("sudo apt install", 20, 0, true, false),
            input_row("This is output", 20, 0, true, false),
        ];
        rows[2].semantic_prompt_id = Some(8);
        let mut cache = InputAccents::default();
        cache.refresh(&mut rows, 20, true);
        assert_eq!(cache.rows[0][2], Some(Command));
        assert_eq!(cache.rows[1][0], Some(Command));
        assert_eq!(cache.rows[1][5], Some(Argument));
        assert!(cache.rows[2].is_empty());
        for row in &mut rows[..2] {
            row.semantic_input.as_mut().unwrap().command_complete = true;
        }
        cache.refresh(&mut rows, 20, true);
        assert_eq!(cache.rows[1][5], Some(Argument));
        cache.refresh(&mut rows[1..], 20, true);
        assert!(cache.rows.iter().all(Vec::is_empty));
    }

    #[test]
    fn unresolved_hard_line_syntax_never_accents_its_following_source() {
        for first in [
            "cat <<EOF",
            "echo \"opening quote",
            "echo \\",
            "echo $(cmd)",
        ] {
            let mut rows = vec![
                input_row(first, 24, 0, false, false),
                input_row("sudo apt install", 24, 0, true, false),
            ];
            let mut cache = InputAccents::default();
            cache.refresh(&mut rows, 24, true);
            assert!(cache.rows[1].is_empty(), "{first}");
        }
    }

    #[test]
    fn role_changes_mark_a_row_dirty_even_when_token_extents_match() {
        let mut rows = vec![input_row("ls -a", 8, 0, false, false)];
        let mut cache = InputAccents::default();
        cache.refresh(&mut rows, 8, true);
        rows[0].inner[3].set_c('a');
        rows[0].dirty = false;
        cache.refresh(&mut rows, 8, true);
        assert!(rows[0].dirty);
        assert_eq!(cache.rows[0][3], Some(CommandSpanKind::Argument));
    }

    #[test]
    fn input_accents_follow_wrapped_quotes_edits_and_clipping() {
        use CommandSpanKind::{Command, Option, Quoted};
        let mut rows = vec![
            input_row("> docker", 8, 2, false, true),
            input_row(" '-a' -x", 8, 0, true, false),
        ];
        let mut cache = InputAccents::default();
        cache.refresh(&mut rows, 8, true);
        assert_eq!(
            cache.rows[0],
            [
                None,
                None,
                Some(Command),
                Some(Command),
                Some(Command),
                Some(Command),
                Some(Command),
                Some(Command)
            ]
        );
        assert_eq!(
            cache.rows[1],
            [
                None,
                Some(Quoted),
                Some(Quoted),
                Some(Quoted),
                Some(Quoted),
                None,
                Some(Option),
                Some(Option)
            ]
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
        use CommandSpanKind::Command;
        let mut rows = vec![input_row("> écho", 8, 2, false, false)];
        let mut cache = InputAccents::default();
        cache.refresh(&mut rows, 8, true);
        assert_eq!(
            cache.rows[0],
            [
                None,
                None,
                Some(Command),
                Some(Command),
                Some(Command),
                Some(Command)
            ]
        );
        rows[0].dirty = false;
        cache.refresh(&mut rows, 8, true);
        assert!(!rows[0].dirty, "unchanged roles do not force redraw");
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
                visit_command_spans(text, shell, |_, range| ranges.push(range));
                assert!(ranges.iter().all(|r| r.start < r.end
                    && text.is_char_boundary(r.start)
                    && text.is_char_boundary(r.end)));
                assert!(ranges.windows(2).all(|pair| pair[0].end <= pair[1].start));
            }
        }
    }
}
