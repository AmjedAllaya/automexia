use super::command_input::{visit_command_spans, CommandSpanKind};
use super::responsive::{elide_end, Viewport};
use super::text_fit::fit_end;
use super::ui_theme::{color_u8, UiTheme, CARD_RADIUS, CONTROL_RADIUS, MODAL_SHADOW};
use rio_backend::crosswords::grid::row::PromptInputShell;
use rio_backend::sugarloaf::{text::DrawOpts, Sugarloaf};
use smallvec::SmallVec;
use std::ops::Range;

const TITLE: &str = "Review multiline paste";
const WARNING: &str =
    "Paste may run commands immediately in shells without bracketed paste.";
const BASH_BATCH_HINT: &str =
    "Bash with set -e can exit on failure. Choose Bash batch to keep the shell open.";
const UNBRACKETED_BATCH_HINT: &str =
    "Plain paste may run immediately. Bash batch inserts one line for review.";
const PREPARED_BATCH_HINT: &str =
    "Use an empty Bash prompt. Errors stop the batch. Shell changes stay local.";
const RESIZE_TO_REVIEW: &str = "Enlarge the window to review the paste.";
const LINE_HEIGHT: f32 = 20.0;
const MAX_VISIBLE_LINES: usize = 12;
const MAX_DISPLAY_CHARS: usize = 256;
const MODAL_ORDER: u8 = 20;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Choice {
    Cancel,
    BashBatch,
    #[default]
    Paste,
}

impl Choice {
    fn label(self) -> &'static str {
        match self {
            Self::Cancel => "Cancel",
            Self::BashBatch => "Bash batch",
            Self::Paste => "Paste",
        }
    }

    fn accessibility_id(self) -> u64 {
        match self {
            Self::Cancel => 2,
            Self::Paste => 3,
            Self::BashBatch => 4,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BatchReview {
    Literal,
    Available { bracketed: bool },
    Prepared,
}

impl BatchReview {
    pub(crate) fn is_available(self) -> bool {
        matches!(self, Self::Available { .. })
    }

    fn title(self) -> &'static str {
        match self {
            Self::Literal | Self::Available { .. } => TITLE,
            Self::Prepared => "Review Bash batch",
        }
    }

    fn hint(self) -> &'static str {
        match self {
            Self::Literal => WARNING,
            Self::Available { bracketed: true } => BASH_BATCH_HINT,
            Self::Available { bracketed: false } => UNBRACKETED_BATCH_HINT,
            Self::Prepared => PREPARED_BATCH_HINT,
        }
    }
}

pub(crate) struct LineIndex {
    offsets: Vec<u32>,
    first: usize,
    horizontal: usize,
    longest: usize,
    shell: PromptInputShell,
}

impl LineIndex {
    pub(crate) fn new(text: &str) -> Self {
        Self::for_shell(text, PromptInputShell::Posix)
    }

    pub(crate) fn for_shell(text: &str, shell: PromptInputShell) -> Self {
        // The payload is already capped at the clipboard paste byte limit.
        // Store offsets, so painting a page does not scan or copy the payload.
        let mut offsets = vec![0];
        let mut end = 0;
        let mut longest = 0;
        for line in text.split_inclusive('\n') {
            longest = longest.max(line.strip_suffix('\n').unwrap_or(line).len());
            end += line.len();
            if line.ends_with('\n') {
                offsets.push(end as u32);
            }
        }
        Self {
            offsets,
            first: 0,
            horizontal: 0,
            longest,
            shell,
        }
    }

    pub(crate) fn scroll(&mut self, lines: isize, page_size: usize) {
        self.first = self
            .first
            .saturating_add_signed(lines)
            .min(self.offsets.len().saturating_sub(page_size));
    }

    pub(crate) fn len(&self) -> usize {
        self.offsets.len()
    }

    pub(crate) fn scroll_horizontal(&mut self, bytes: isize) {
        self.horizontal = self
            .horizontal
            .saturating_add_signed(bytes)
            .min(self.longest.saturating_sub(1));
    }

    fn visible(&self, page_size: usize) -> std::ops::Range<usize> {
        let first = self.first.min(self.offsets.len().saturating_sub(page_size));
        first..(first + page_size).min(self.offsets.len())
    }

    fn line<'a>(&self, text: &'a str, index: usize) -> &'a str {
        let first = self.offsets[index] as usize;
        let end = self
            .offsets
            .get(index + 1)
            .map_or(text.len(), |next| *next as usize - 1);
        &text[first..end]
    }

    fn display_line(&self, text: &str, index: usize) -> String {
        let (line, start) = self.line_with_visible_start(text, index);
        display_line(&line[start..], start > 0)
    }

    fn highlighted_line(&self, text: &str, index: usize) -> HighlightedLine {
        let (line, start) = self.line_with_visible_start(text, index);
        // A displayed character covers at most four bytes of the source line.
        let visible_end = start + MAX_DISPLAY_CHARS * 4;
        let mut spans = SmallVec::<[(CommandSpanKind, Range<usize>); 8]>::new();
        // Use the simple common shell accents on the source line, so scrolling
        // and display substitutions cannot change which words are accented.
        visit_command_spans(line, self.shell, |kind, range| {
            if range.end > start && range.start < visible_end {
                spans.push((kind, range));
            }
        });
        let mut spans = spans.into_iter().peekable();
        let mut kinds = SmallVec::new();
        let text = project_line(&line[start..], start > 0, |offset, displayed| {
            let offset = start + offset;
            while spans.peek().is_some_and(|(_, range)| range.end <= offset) {
                let _ = spans.next();
            }
            if let Some((kind, _)) =
                spans.peek().filter(|(_, range)| range.contains(&offset))
            {
                kinds.resize(displayed.start, None);
                kinds.resize(displayed.end, Some(*kind));
            }
        });
        HighlightedLine { text, kinds }
    }

    fn line_with_visible_start<'a>(
        &self,
        text: &'a str,
        index: usize,
    ) -> (&'a str, usize) {
        let line = self.line(text, index);
        let start = line.floor_char_boundary(self.horizontal.min(line.len()));
        (line, start)
    }
}

struct HighlightedLine {
    text: String,
    kinds: SmallVec<[Option<CommandSpanKind>; MAX_DISPLAY_CHARS]>,
}

impl HighlightedLine {
    fn kind_at(&self, byte: usize) -> Option<CommandSpanKind> {
        self.kinds.get(byte).copied().flatten()
    }
}

#[derive(Clone, Copy, Debug)]
struct Rect {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}

impl Rect {
    fn contains(self, x: f32, y: f32) -> bool {
        x >= self.x && x < self.x + self.width && y >= self.y && y < self.y + self.height
    }

    fn paint(self, sugarloaf: &mut Sugarloaf, color: [f32; 4], radius: f32) {
        sugarloaf.rounded_rect(
            None,
            self.x,
            self.y,
            self.width,
            self.height,
            color,
            0.0,
            radius,
            MODAL_ORDER,
        );
    }
}

pub(crate) struct Layout {
    card: Rect,
    preview: Rect,
    buttons: SmallVec<[(Choice, Rect); 3]>,
    batch_review: BatchReview,
    inset: f32,
    scale: f32,
    compact: bool,
    pub(crate) page_size: usize,
}

impl Layout {
    pub(crate) fn new(
        (width, height, scale): (f32, f32, f32),
        lines: usize,
        batch_review: BatchReview,
    ) -> Self {
        let viewport = Viewport::from_physical(width, height, scale);
        let margin = if viewport.width < 280.0 || viewport.height < 180.0 {
            4.0
        } else {
            16.0
        };
        let width = viewport.fitted_surface(680.0, margin);
        let available_height = (viewport.height - margin * 2.0).max(1.0);
        let compact = available_height < 170.0 || width < 300.0;
        let inset = if compact { 8.0_f32 } else { 20.0 }.min(width / 4.0);
        let header = if compact { 44.0 } else { 78.0 };
        let stack_batch = batch_review.is_available() && width < 240.0;
        let footer = if stack_batch {
            68.0
        } else if compact {
            36.0
        } else {
            74.0
        };
        let available_preview = (available_height - header - footer - 12.0).max(0.0);
        let page_size = ((available_preview / LINE_HEIGHT) as usize)
            .min(MAX_VISIBLE_LINES)
            .min(lines);
        let preview_height = if page_size == 0 {
            0.0
        } else {
            page_size as f32 * LINE_HEIGHT + 12.0
        };
        let height = available_height.min(header + footer + preview_height);
        let card = Rect {
            x: (viewport.width - width).max(0.0) / 2.0,
            y: (viewport.height - height).max(0.0) / 2.0,
            width,
            height,
        };
        let preview = Rect {
            x: card.x + inset,
            y: card.y + header,
            width: width - inset * 2.0,
            height: preview_height,
        };
        let show_batch = batch_review.is_available() && page_size > 0;
        let stacked = show_batch && stack_batch;
        let button_height = (if compact { 24.0_f32 } else { 32.0 })
            .min(height / if stacked { 4.0 } else { 2.0 });
        let gap = 8.0_f32.min(preview.width / 4.0);
        let columns = if show_batch && !stacked { 3.0 } else { 2.0 };
        let button_width = ((preview.width - (columns - 1.0) * gap) / columns).min(94.0);
        let paste = Rect {
            x: preview.x + preview.width - button_width,
            y: card.y + height - button_height - inset.min(height / 4.0),
            width: button_width,
            height: button_height,
        };
        let cancel = Rect {
            x: paste.x - (columns - 1.0) * (gap + button_width),
            ..paste
        };
        let mut buttons = SmallVec::new();
        buttons.push((Choice::Cancel, cancel));
        if show_batch {
            let batch = if stacked {
                let width = preview.width.min(94.0);
                Rect {
                    x: preview.x + preview.width - width,
                    y: paste.y - button_height - gap.min(height / 4.0),
                    width,
                    ..paste
                }
            } else {
                Rect {
                    x: paste.x - gap - button_width,
                    ..paste
                }
            };
            buttons.push((Choice::BashBatch, batch));
        }
        buttons.push((Choice::Paste, paste));
        Self {
            card,
            preview,
            buttons,
            batch_review,
            inset,
            scale: viewport.scale,
            compact,
            page_size,
        }
    }

    fn choices(&self) -> impl Iterator<Item = (Choice, Rect)> + '_ {
        self.buttons.iter().copied()
    }

    pub(crate) fn hit(&self, (x, y): (f32, f32)) -> Option<Choice> {
        self.choices().find_map(|(choice, rect)| {
            (self.is_enabled(choice) && rect.contains(x / self.scale, y / self.scale))
                .then_some(choice)
        })
    }

    pub(crate) fn is_enabled(&self, choice: Choice) -> bool {
        match choice {
            Choice::Cancel => true,
            Choice::BashBatch => self.page_size > 0 && self.batch_review.is_available(),
            Choice::Paste => self.page_size > 0,
        }
    }

    pub(crate) fn focused_choice(&self, choice: Choice) -> Choice {
        if self.is_enabled(choice) {
            choice
        } else {
            Choice::Cancel
        }
    }

    pub(crate) fn next_choice(&self, selected: Choice, reverse: bool) -> Choice {
        if self.page_size == 0 {
            return Choice::Cancel;
        }
        let focused = self.focused_choice(selected);
        let position = self
            .buttons
            .iter()
            .position(|(choice, _)| *choice == focused)
            .unwrap_or(0);
        let count = self.buttons.len();
        let step = if reverse { count - 1 } else { 1 };
        self.buttons[(position + step) % count].0
    }

    pub(crate) fn render(
        &self,
        sugarloaf: &mut Sugarloaf,
        theme: UiTheme,
        index: &LineIndex,
        text: &str,
        selected: Choice,
        hovered: Option<Choice>,
    ) {
        let selected = self.focused_choice(selected);
        sugarloaf.replace_modal_layer();
        Rect {
            x: self.card.x + 6.0,
            y: self.card.y + 8.0,
            ..self.card
        }
        .paint(sugarloaf, MODAL_SHADOW, CARD_RADIUS);
        self.card.paint(sugarloaf, theme.background, CARD_RADIUS);
        let title = DrawOpts {
            font_size: if self.compact { 13.0 } else { 19.0 },
            color: color_u8(theme.text),
            bold: true,
            ..DrawOpts::default()
        };
        let detail = DrawOpts {
            font_size: if self.compact { 10.0 } else { 12.0 },
            color: color_u8(theme.muted_text),
            ..DrawOpts::default()
        };
        let code = DrawOpts {
            font_size: 13.0,
            color: color_u8(theme.text),
            ..DrawOpts::default()
        };
        let title_y = self.card.y + self.inset;
        if self.card.height >= 42.0 {
            draw(
                sugarloaf,
                self.preview.x,
                title_y,
                self.batch_review.title(),
                self.preview.width,
                &title,
            );
        }
        if self.card.height >= 72.0 {
            draw(
                sugarloaf,
                self.preview.x,
                title_y + if self.compact { 17.0 } else { 30.0 },
                if self.page_size > 0 {
                    self.batch_review.hint()
                } else {
                    RESIZE_TO_REVIEW
                },
                self.preview.width,
                &DrawOpts {
                    color: color_u8(theme.warning),
                    ..detail
                },
            );
        }

        if self.page_size > 0 {
            self.preview.paint(sugarloaf, theme.surface, CONTROL_RADIUS);
            let visible = index.visible(self.page_size);
            let gutter = (index.offsets.len().ilog10() as f32 + 1.0) * 8.0 + 19.0;
            for (row, line) in visible.clone().enumerate() {
                let y = self.preview.y + 6.0 + row as f32 * LINE_HEIGHT;
                draw(
                    sugarloaf,
                    self.preview.x + 8.0,
                    y,
                    &(line + 1).to_string(),
                    gutter - 13.0,
                    &detail,
                );
                draw_code(
                    sugarloaf,
                    [self.preview.x + gutter, y],
                    &index.highlighted_line(text, line),
                    self.preview.width - gutter - 10.0,
                    &code,
                    theme,
                );
            }
            if !self.compact {
                let status = format!(
                    "Lines {}–{} of {} · Arrow keys scroll",
                    visible.start + 1,
                    visible.end,
                    index.len()
                );
                draw(
                    sugarloaf,
                    self.preview.x,
                    self.preview.y + self.preview.height + 7.0,
                    &status,
                    self.preview.width,
                    &detail,
                );
            }
        }

        for (choice, rect) in self.choices() {
            let enabled = self.is_enabled(choice);
            let highlighted = enabled && (choice == selected || hovered == Some(choice));
            rect.paint(
                sugarloaf,
                if highlighted {
                    theme.raised
                } else {
                    theme.surface
                },
                CONTROL_RADIUS,
            );
            let opts = DrawOpts {
                color: color_u8(if !enabled {
                    theme.muted_text
                } else if choice == selected {
                    theme.accent
                } else {
                    theme.text
                }),
                bold: true,
                ..detail
            };
            let label = elide_end(
                sugarloaf,
                choice.label(),
                (rect.width - 4.0).max(0.0),
                &opts,
            );
            let width = sugarloaf.text_mut().measure(&label, &opts);
            if rect.height >= opts.font_size && !label.is_empty() {
                sugarloaf.text_mut().draw(
                    rect.x + (rect.width - width) * 0.5,
                    rect.y + (rect.height - opts.font_size) * 0.5,
                    &label,
                    &opts,
                );
            }
        }
    }

    pub(crate) fn accessibility_surface(
        &self,
        dimensions: (f32, f32, f32),
        index: &LineIndex,
        text: &str,
        selected: Choice,
    ) -> automexia_ui_model::accessibility::Surface {
        use accesskit::{Node, Role};
        use automexia_ui_model::accessibility::{physical_bounds, Surface};
        let viewport =
            accesskit::Rect::new(0.0, 0.0, dimensions.0 as f64, dimensions.1 as f64);
        let bounds = |rect: Rect| {
            physical_bounds(
                [rect.x, rect.y, rect.width, rect.height],
                self.scale,
                viewport,
            )
        };
        let mut surface = Surface::dialog(
            u64::MAX - 9,
            self.batch_review.title(),
            bounds(self.card).unwrap_or(viewport),
        );
        let mut explanation = Node::new(Role::Label);
        explanation.set_label(if self.page_size > 0 {
            format!("{} Use the arrow keys to scroll, Tab to switch buttons, Enter to choose, or Escape to cancel.", self.batch_review.hint())
        } else {
            RESIZE_TO_REVIEW.into()
        });
        surface.push(1, explanation, false);
        for (row, line) in index.visible(self.page_size).enumerate() {
            let mut node = Node::new(Role::Label);
            node.set_label(format!(
                "Line {}. {}",
                line + 1,
                index.display_line(text, line)
            ));
            surface.push(10 + row as u64, node, false);
        }
        for (choice, rect) in self.choices() {
            let mut button = Node::new(Role::Button);
            button.set_label(choice.label());
            if let Some(bounds) = bounds(rect) {
                button.set_bounds(bounds);
            }
            if !self.is_enabled(choice) {
                button.set_disabled();
            }
            surface.push(
                choice.accessibility_id(),
                button,
                choice == self.focused_choice(selected),
            );
        }
        surface
    }
}

fn draw(
    sugarloaf: &mut Sugarloaf,
    x: f32,
    y: f32,
    text: &str,
    width: f32,
    opts: &DrawOpts,
) {
    let value = elide_end(sugarloaf, text, width, opts);
    sugarloaf.text_mut().draw(x, y, &value, opts);
}

fn display_line(line: &str, starts_offscreen: bool) -> String {
    project_line(line, starts_offscreen, |_, _| {})
}

fn project_line(
    line: &str,
    starts_offscreen: bool,
    mut projected: impl FnMut(usize, Range<usize>),
) -> String {
    let mut result = String::new();
    if starts_offscreen {
        result.push('…');
    }
    for (index, (source, ch)) in line.char_indices().enumerate() {
        if index == MAX_DISPLAY_CHARS {
            result.push('…');
            break;
        }
        let start = result.len();
        match ch {
            '\t' => result.push_str("    "),
            // Expose visual reordering controls without changing clipboard bytes.
            '\u{061c}'
            | '\u{200e}'..='\u{200f}'
            | '\u{202a}'..='\u{202e}'
            | '\u{2066}'..='\u{2069}' => {
                result.extend(ch.escape_unicode());
            }
            _ => result.push(ch),
        }
        projected(source, start..result.len());
    }
    result
}

fn draw_code(
    sugarloaf: &mut Sugarloaf,
    [x, y]: [f32; 2],
    line: &HighlightedLine,
    width: f32,
    opts: &DrawOpts,
    theme: UiTheme,
) {
    let fitted = fit_end(&line.text, width, "…", |text, _| {
        sugarloaf.text_mut().measure(text, opts)
    });
    let suffix = fitted.retained.map_or(0, |_| '…'.len_utf8());
    let end = fitted.display.len().saturating_sub(suffix);
    sugarloaf
        .text_mut()
        .draw_colored(x, y, &fitted.display, opts, |byte| {
            line.kind_at(byte)
                .filter(|_| byte < end)
                .map_or(opts.color, |kind| kind.ink(&theme))
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clipboard_preview_distinguishes_commands_flags_and_values() {
        let text =
            "python3 --version\ndocker compose version\necho '界' | sort -r # note";
        let index = LineIndex::new(text);
        let line = index.highlighted_line(text, 0);
        assert_eq!(line.text, "python3 --version");
        assert_eq!(line.kind_at(0), Some(CommandSpanKind::Command));
        assert_eq!(line.kind_at(7), None);
        assert_eq!(line.kind_at(8), Some(CommandSpanKind::Option));

        let line = index.highlighted_line(text, 1);
        assert_eq!(line.kind_at(0), Some(CommandSpanKind::Command));
        assert_eq!(line.kind_at(7), Some(CommandSpanKind::Argument));
        assert_eq!(line.kind_at(15), Some(CommandSpanKind::Argument));

        let line = index.highlighted_line(text, 2);
        for (part, expected) in [
            ("'界'", CommandSpanKind::Quoted),
            ("|", CommandSpanKind::Operator),
            ("sort", CommandSpanKind::Command),
            ("-r", CommandSpanKind::Option),
            ("# note", CommandSpanKind::Comment),
        ] {
            assert_eq!(line.kind_at(line.text.find(part).unwrap()), Some(expected));
        }
    }

    #[test]
    fn clipboard_preview_uses_cmd_switch_colors_for_native_cmd() {
        let text = r"dir /W C:\workspace";
        let cmd =
            LineIndex::for_shell(text, PromptInputShell::Cmd).highlighted_line(text, 0);
        let posix = LineIndex::new(text).highlighted_line(text, 0);
        assert_eq!(cmd.text, text);
        assert_eq!(cmd.kind_at(4), Some(CommandSpanKind::Option));
        assert_eq!(posix.kind_at(4), Some(CommandSpanKind::Argument));
        assert_eq!(cmd.kind_at(7), Some(CommandSpanKind::Argument));
    }

    #[test]
    fn clipboard_preview_retains_syntax_context_when_scrolled() {
        let text = "echo\t'value --fake' -- --literal";
        let mut index = LineIndex::new(text);
        let line = index.highlighted_line(text, 0);
        assert_eq!(line.text, "echo    'value --fake' -- --literal");
        assert_eq!(line.kind_at(8), Some(CommandSpanKind::Quoted));

        index.scroll_horizontal(7);
        let line = index.highlighted_line(text, 0);
        assert_eq!(line.text, index.display_line(text, 0));
        assert!(line.text.starts_with('…'));
        assert_eq!(line.kind_at(0), None);
        assert_eq!(
            line.kind_at(line.text.find("--fake").unwrap()),
            Some(CommandSpanKind::Quoted)
        );
        assert_eq!(
            line.kind_at(line.text.find("--literal").unwrap()),
            Some(CommandSpanKind::Argument)
        );

        let unsupported = "echo $(f --arg)";
        let line = LineIndex::new(unsupported).highlighted_line(unsupported, 0);
        assert_eq!(line.text, unsupported);
        assert_eq!(line.kind_at(0), None);
    }

    #[test]
    fn clipboard_preview_colors_stay_readable_on_light_and_dark_cards() {
        for (background, foreground) in [
            ([0.04, 0.07, 0.14, 1.0], [1.0; 4]),
            ([0.95, 0.96, 0.98, 1.0], [0.0, 0.0, 0.0, 1.0]),
        ] {
            let theme = UiTheme::resolve(background, foreground, foreground);
            let command = CommandSpanKind::Command.ink(&theme);
            let option = CommandSpanKind::Option.ink(&theme);
            let argument = CommandSpanKind::Argument.ink(&theme);
            assert_ne!(command, option);
            assert_ne!(command, argument);
            assert_ne!(option, argument);
            assert_ne!(command, color_u8(theme.text));
            for kind in [
                CommandSpanKind::Command,
                CommandSpanKind::Option,
                CommandSpanKind::Argument,
                CommandSpanKind::Variable,
                CommandSpanKind::Quoted,
                CommandSpanKind::Operator,
                CommandSpanKind::Comment,
            ] {
                let color = kind.ink(&theme).map(|value| f32::from(value) / 255.0);
                assert!(
                    automexia_ui_model::contrast_ratio(color, theme.surface)
                        >= automexia_ui_model::MIN_TEXT_CONTRAST
                );
            }
        }
    }

    #[test]
    fn clipboard_preview_preserves_blank_lines_and_pages_through_unicode() {
        let text = "one\n\n\t界\u{202e}two\n";
        let mut index = LineIndex::new(text);
        assert_eq!(index.visible(2), 0..2);
        assert_eq!(index.line(text, 1), "");
        index.scroll(isize::MAX, 2);
        assert_eq!(index.visible(2), 2..4);
        assert_eq!(index.display_line(text, 2), "    界\\u{202e}two");
        index.scroll_horizontal(2);
        assert_eq!(index.display_line(text, 2), "…界\\u{202e}two");
        index.scroll_horizontal(isize::MIN);
        assert_eq!(index.display_line(text, 2), "    界\\u{202e}two");
        assert_eq!(index.line(text, 3), "");
        assert_eq!(index.visible(3), 1..4);
        index.scroll(isize::MIN, 3);
        assert_eq!(index.visible(3), 0..3);
        assert_eq!(
            display_line(&"界".repeat(1024), false),
            "界".repeat(256) + "…"
        );
    }

    #[test]
    fn clipboard_preview_buttons_stay_inside_the_card_at_supported_scales() {
        for dimensions in [(80.0, 60.0, 1.0), (800.0, 600.0, 1.0), (1200.0, 800.0, 2.0)] {
            let layout = Layout::new(dimensions, 3, BatchReview::Literal);
            for (choice, button) in layout.choices() {
                assert!(button.x >= layout.card.x);
                assert!(button.y >= layout.card.y);
                assert!(button.x + button.width <= layout.card.x + layout.card.width);
                assert!(button.y + button.height <= layout.card.y + layout.card.height);
                assert_eq!(
                    layout.hit((
                        (button.x + 1.0) * layout.scale,
                        (button.y + 1.0) * layout.scale
                    )),
                    layout.is_enabled(choice).then_some(choice)
                );
            }
            assert_eq!(layout.hit((0.0, 0.0)), None);
        }
    }

    #[test]
    fn bash_batch_choice_fits_and_joins_the_focus_order_only_while_available() {
        for dimensions in [(210.0, 300.0, 1.0), (1400.0, 900.0, 1.5)] {
            let layout =
                Layout::new(dimensions, 3, BatchReview::Available { bracketed: true });
            assert_eq!(layout.next_choice(Choice::Cancel, false), Choice::BashBatch);
            assert_eq!(layout.next_choice(Choice::BashBatch, false), Choice::Paste);
            assert_eq!(layout.next_choice(Choice::Paste, true), Choice::BashBatch);
            assert_eq!(layout.next_choice(Choice::BashBatch, true), Choice::Cancel);
            for (choice, button) in layout.choices() {
                assert!(button.x >= layout.card.x);
                assert!(button.y >= layout.preview.y + layout.preview.height);
                assert!(button.x + button.width <= layout.card.x + layout.card.width);
                assert!(button.y + button.height <= layout.card.y + layout.card.height);
                assert_eq!(
                    layout.hit((
                        (button.x + button.width * 0.5) * layout.scale,
                        (button.y + button.height * 0.5) * layout.scale,
                    )),
                    Some(choice)
                );
            }
        }

        let available = BatchReview::Available { bracketed: false };
        assert_eq!(available.hint(), UNBRACKETED_BATCH_HINT);
        let legacy = Layout::new((800.0, 600.0, 1.0), 3, available);
        assert!(legacy.is_enabled(Choice::BashBatch));
        let compact = Layout::new((80.0, 60.0, 1.0), 3, available);
        assert_eq!(compact.next_choice(Choice::Cancel, false), Choice::Cancel);
        assert!(!compact.is_enabled(Choice::BashBatch));

        let prepared = Layout::new((800.0, 600.0, 1.0), 3, BatchReview::Prepared);
        assert!(!prepared.is_enabled(Choice::BashBatch));
        assert_eq!(prepared.focused_choice(Choice::BashBatch), Choice::Cancel);
        assert_eq!(prepared.next_choice(Choice::Cancel, false), Choice::Paste);
        assert_eq!(prepared.choices().count(), 2);
    }
}
