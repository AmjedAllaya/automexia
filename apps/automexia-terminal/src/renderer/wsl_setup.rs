//! Layout, painting, and accessibility for the terminal's WSL setup dialog.

use super::responsive::{elide_end, Viewport};
use super::ui_theme::{color_u8, UiTheme, CARD_RADIUS, CONTROL_RADIUS, MODAL_SHADOW};
pub(crate) use rio_backend::event::WslSetupChoice as Choice;
use rio_backend::sugarloaf::{text::DrawOpts, Sugarloaf};

const OWNER: u64 = u64::MAX - 21;
const ORDER: u8 = 20;
const MAX_DETAILS: usize = 4;
const MAX_TEXT_BYTES: usize = 768;
const MAX_LINES: usize = 6;
const LINE_HEIGHT: f32 = 17.0;
const DETAIL_PADDING: f32 = 12.0;
const DISMISS_WIDTH: f32 = 88.0;
const BASH_BUTTON_WIDTH: f32 = 224.0;
const EZA_BUTTON_WIDTH: f32 = 112.0;
const RESIZE_HINT: &str = "Enlarge the window to review WSL setup.";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Tone {
    Info,
    Warning,
    Error,
}

pub(crate) struct WslSetupDetail {
    pub label: &'static str,
    pub description: String,
    pub emphasized: bool,
}

pub(crate) struct WslSetupDialog {
    pub title: &'static str,
    pub subtitle: Option<String>,
    pub details: Vec<WslSetupDetail>,
    pub note: Option<String>,
    pub tone: Tone,
    pub can_install_eza: bool,
    pub can_install_bash: bool,
}

impl WslSetupDialog {
    pub(crate) fn offers(&self, choice: Choice) -> bool {
        match choice {
            Choice::Dismiss => true,
            Choice::InstallBash => self.can_install_bash,
            Choice::InstallEza => self.can_install_eza,
        }
    }

    pub(crate) fn action_label(&self, choice: Choice) -> &'static str {
        match choice {
            Choice::Dismiss if self.can_install_bash || self.can_install_eza => "Cancel",
            Choice::Dismiss => "Close",
            Choice::InstallBash => "Install Bash integration",
            Choice::InstallEza => "Install eza",
        }
    }

    pub(crate) fn summary(&self) -> String {
        use std::fmt::Write;

        let mut summary = self.title.to_owned();
        if let Some(subtitle) = self.subtitle.as_deref() {
            let _ = write!(summary, "\n{}", bounded_text(subtitle));
        }
        for detail in self.details.iter().take(MAX_DETAILS) {
            let _ = write!(
                summary,
                "\n{}. {}",
                detail.label,
                bounded_text(&detail.description)
            );
        }
        if let Some(note) = self.note.as_deref() {
            let _ = write!(summary, "\n{}", bounded_text(note));
        }
        summary
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

    #[cfg(any(test, feature = "native-gui-test-hooks"))]
    fn bounds(self, scale: f32) -> [f32; 4] {
        [
            self.x * scale,
            self.y * scale,
            self.width * scale,
            self.height * scale,
        ]
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
            radius.min(self.width.min(self.height) * 0.5),
            ORDER,
        );
    }
}

pub(crate) struct Layout {
    viewport: Viewport,
    card: Rect,
    dismiss: Rect,
    install_bash: Option<Rect>,
    install_eza: Option<Rect>,
    inset: f32,
    read_details: bool,
}

impl Layout {
    pub(crate) fn new(dimensions: (f32, f32, f32), dialog: &WslSetupDialog) -> Self {
        let viewport = Viewport::from_physical(dimensions.0, dimensions.1, dimensions.2);
        let margin = if viewport.width < 320.0 || viewport.height < 220.0 {
            4.0
        } else {
            18.0
        };
        let width = viewport.fitted_surface(556.0, margin).min(viewport.width);
        let available_height = (viewport.height - margin * 2.0)
            .max(1.0)
            .min(viewport.height);
        let inset = 20.0_f32.min(width / 8.0).min(available_height / 8.0);
        let inner_width = width - inset * 2.0;
        let gap = 8.0_f32.min(inner_width / 8.0);
        let bash_width = if dialog.can_install_bash {
            BASH_BUTTON_WIDTH + gap
        } else {
            0.0
        };
        let eza_width = if dialog.can_install_eza {
            EZA_BUTTON_WIDTH + gap
        } else {
            0.0
        };
        let subtitle = rows(dialog.subtitle.as_deref().unwrap_or_default(), inner_width);
        let note = rows(dialog.note.as_deref().unwrap_or_default(), inner_width);
        let mut details_height = 0.0;
        let mut details_fit = dialog.details.len() <= MAX_DETAILS;
        for detail in dialog.details.iter().take(MAX_DETAILS) {
            let count = rows(&detail.description, inner_width - DETAIL_PADDING * 2.0);
            details_fit &= count.is_some();
            details_height += 34.0 + count.unwrap_or(MAX_LINES) as f32 * LINE_HEIGHT;
        }
        let note_gap = if dialog.note.is_some() { 8.0 } else { 0.0 };
        let content_height = 70.0
            + inset * 2.0
            + details_height
            + (subtitle.unwrap_or(MAX_LINES) + note.unwrap_or(MAX_LINES)) as f32
                * LINE_HEIGHT
            + note_gap;
        // Key and pointer consent is unavailable while the destination or
        // prerequisites cannot be shown. Dismiss remains reachable.
        let read_details = width >= 200.0
            && content_height <= available_height
            && DISMISS_WIDTH + bash_width + eza_width <= inner_width
            && subtitle.is_some()
            && note.is_some()
            && details_fit;
        let height = if read_details {
            content_height
        } else {
            available_height.min(102.0)
        };
        let card = Rect {
            x: (viewport.width - width) * 0.5,
            y: (viewport.height - height) * 0.5,
            width,
            height,
        };
        let button_height =
            (if read_details { 32.0_f32 } else { 26.0 }).min(height / 2.0);
        let button_y = card.y + height - button_height - inset.min(height * 0.25);
        let right = card.x + width - inset;
        let button = |right, width| Rect {
            x: right - width,
            y: button_y,
            width,
            height: button_height,
        };
        let install_eza = (read_details && dialog.can_install_eza)
            .then(|| button(right, EZA_BUTTON_WIDTH));
        let bash_right = install_eza.map_or(right, |rect| rect.x - gap);
        let install_bash = (read_details && dialog.can_install_bash)
            .then(|| button(bash_right, BASH_BUTTON_WIDTH));
        let dismiss_right = install_bash.map_or(bash_right, |rect| rect.x - gap);
        Self {
            viewport,
            card,
            dismiss: button(dismiss_right, DISMISS_WIDTH.min(inner_width)),
            install_bash,
            install_eza,
            inset,
            read_details,
        }
    }

    fn buttons(&self) -> impl Iterator<Item = (Choice, Rect)> {
        [
            (Choice::Dismiss, Some(self.dismiss)),
            (Choice::InstallBash, self.install_bash),
            (Choice::InstallEza, self.install_eza),
        ]
        .into_iter()
        .filter_map(|(choice, rect)| rect.map(|rect| (choice, rect)))
    }

    pub(crate) fn hit(&self, (x, y): (f32, f32)) -> Option<Choice> {
        self.buttons().find_map(|(choice, rect)| {
            rect.contains(x / self.viewport.scale, y / self.viewport.scale)
                .then_some(choice)
        })
    }

    pub(crate) fn is_enabled(&self, choice: Choice) -> bool {
        match choice {
            Choice::Dismiss => true,
            Choice::InstallBash => self.install_bash.is_some(),
            Choice::InstallEza => self.install_eza.is_some(),
        }
    }

    pub(crate) fn next_focus(&self, current: Choice, backwards: bool) -> Choice {
        let order = if backwards {
            [Choice::Dismiss, Choice::InstallEza, Choice::InstallBash]
        } else {
            [Choice::Dismiss, Choice::InstallBash, Choice::InstallEza]
        };
        order
            .into_iter()
            .chain(order)
            .skip_while(|choice| *choice != current)
            .skip(1)
            .find(|choice| self.is_enabled(*choice))
            .unwrap_or_default()
    }

    #[cfg(feature = "native-gui-test-hooks")]
    pub(crate) fn bounds(&self, choice: Choice) -> Option<[f32; 4]> {
        self.buttons().find_map(|(candidate, rect)| {
            (choice == candidate).then(|| rect.bounds(self.viewport.scale))
        })
    }

    pub(crate) fn render(
        &self,
        sugarloaf: &mut Sugarloaf,
        theme: UiTheme,
        dialog: &WslSetupDialog,
        selected: Choice,
        hovered: Option<Choice>,
    ) {
        sugarloaf.replace_modal_layer();
        if self.card.width <= 0.0 || self.card.height <= 0.0 {
            return;
        }
        Rect {
            x: (self.card.x + 5.0).min(self.viewport.width - self.card.width),
            y: (self.card.y + 7.0).min(self.viewport.height - self.card.height),
            ..self.card
        }
        .paint(sugarloaf, MODAL_SHADOW, CARD_RADIUS);
        self.card.paint(sugarloaf, theme.background, CARD_RADIUS);

        let left = self.card.x + self.inset;
        let top = self.card.y + self.inset;
        let width = self.card.width - self.inset * 2.0;
        let text = DrawOpts {
            font_size: 12.0,
            color: color_u8(theme.text),
            ..DrawOpts::default()
        };
        let muted = DrawOpts {
            color: color_u8(theme.muted_text),
            ..text
        };
        let tone = match dialog.tone {
            Tone::Info => theme.accent,
            Tone::Warning => theme.warning,
            Tone::Error => theme.danger,
        };
        if self.dismiss.y - top >= 20.0 {
            draw(
                sugarloaf,
                dialog.title,
                left,
                top,
                width,
                &DrawOpts {
                    font_size: 18.0,
                    color: color_u8(if dialog.tone == Tone::Error {
                        tone
                    } else {
                        theme.text
                    }),
                    bold: true,
                    ..text
                },
            );
        }
        if self.read_details {
            let mut y = top + 26.0;
            if let Some(subtitle) = dialog.subtitle.as_deref() {
                y = draw_lines(sugarloaf, subtitle, left, y, width, &muted);
            }
            for detail in dialog.details.iter().take(MAX_DETAILS) {
                let detail_width = width - DETAIL_PADDING * 2.0;
                let count = rows(&detail.description, detail_width).unwrap_or(MAX_LINES);
                let height = 30.0 + count as f32 * LINE_HEIGHT;
                Rect {
                    x: left,
                    y,
                    width,
                    height,
                }
                .paint(sugarloaf, theme.surface, CONTROL_RADIUS);
                let text_x = left + DETAIL_PADDING;
                draw(
                    sugarloaf,
                    detail.label,
                    text_x,
                    y + 7.0,
                    detail_width,
                    &DrawOpts {
                        bold: true,
                        color: color_u8(if detail.emphasized {
                            tone
                        } else {
                            theme.text
                        }),
                        ..text
                    },
                );
                draw_lines(
                    sugarloaf,
                    &detail.description,
                    text_x,
                    y + 22.0,
                    detail_width,
                    &text,
                );
                y += height + 4.0;
            }
            if let Some(note) = dialog.note.as_deref() {
                draw_lines(sugarloaf, note, left, y + 4.0, width, &muted);
            }
        } else if self.dismiss.y - top >= 43.0 {
            draw(sugarloaf, RESIZE_HINT, left, top + 24.0, width, &muted);
        }

        let focused = if self.is_enabled(selected) {
            selected
        } else {
            Choice::Dismiss
        };
        for (choice, rect) in self.buttons() {
            let primary = choice != Choice::Dismiss;
            let selected = choice == focused;
            let fill = if primary || hovered == Some(choice) {
                theme.raised
            } else {
                theme.surface
            };
            if selected && rect.width >= 4.0 && rect.height >= 4.0 {
                rect.paint(sugarloaf, theme.outline, CONTROL_RADIUS);
                Rect {
                    x: rect.x + 1.0,
                    y: rect.y + 1.0,
                    width: rect.width - 2.0,
                    height: rect.height - 2.0,
                }
                .paint(sugarloaf, fill, CONTROL_RADIUS - 1.0);
            } else {
                rect.paint(sugarloaf, fill, CONTROL_RADIUS);
            }
            if rect.height >= text.font_size {
                let opts = DrawOpts {
                    color: color_u8(if primary { theme.accent } else { theme.text }),
                    bold: true,
                    ..text
                };
                let label = elide_end(
                    sugarloaf,
                    dialog.action_label(choice),
                    (rect.width - 6.0).max(0.0),
                    &opts,
                );
                let label_width = sugarloaf.text_mut().measure(&label, &opts);
                sugarloaf.text_mut().draw(
                    rect.x + (rect.width - label_width) * 0.5,
                    rect.y + (rect.height - opts.font_size) * 0.5,
                    &label,
                    &opts,
                );
            }
        }
    }

    pub(crate) fn accessibility_surface(
        &self,
        dialog: &WslSetupDialog,
        selected: Choice,
    ) -> automexia_ui_model::accessibility::Surface {
        use accesskit::{Node, Role};
        use automexia_ui_model::accessibility::{physical_bounds, Surface};

        let viewport = accesskit::Rect::new(
            0.0,
            0.0,
            (self.viewport.width * self.viewport.scale) as f64,
            (self.viewport.height * self.viewport.scale) as f64,
        );
        let bounds = |rect: Rect| {
            physical_bounds(
                [rect.x, rect.y, rect.width, rect.height],
                self.viewport.scale,
                viewport,
            )
        };
        let mut surface =
            Surface::dialog(OWNER, dialog.title, bounds(self.card).unwrap_or(viewport));
        let mut description = Node::new(Role::Label);
        let mut summary = dialog.summary();
        if !self.read_details {
            summary.push('\n');
            summary.push_str(RESIZE_HINT);
        }
        description.set_label(summary);
        surface.push(1, description, false);
        let focused = if self.is_enabled(selected) {
            selected
        } else {
            Choice::Dismiss
        };
        for (choice, rect) in self.buttons() {
            let mut button = Node::new(Role::Button);
            button.set_label(dialog.action_label(choice));
            if let Some(bounds) = bounds(rect) {
                button.set_bounds(bounds);
            }
            let id = match choice {
                Choice::Dismiss => 2,
                Choice::InstallEza => 3,
                Choice::InstallBash => 4,
            };
            surface.push(id, button, choice == focused);
        }
        surface
    }
}

fn bounded_text(value: &str) -> &str {
    &value[..value.floor_char_boundary(value.len().min(MAX_TEXT_BYTES))]
}

fn wrapped(value: &str, width: f32) -> impl Iterator<Item = &str> {
    // The dialog uses the terminal's fixed-width font. Reserve extra width
    // for larger shaped advances, then measure the painted line as usual.
    let columns = (width / 10.5).max(1.0) as usize;
    let mut remaining = bounded_text(value).trim();
    std::iter::from_fn(move || {
        if remaining.is_empty() {
            return None;
        }
        let limit = remaining
            .char_indices()
            .nth(columns)
            .map_or(remaining.len(), |(start, _)| start);
        let head = &remaining[..limit];
        let end = head.find('\n').unwrap_or_else(|| {
            if limit == remaining.len()
                || remaining[limit..].starts_with(char::is_whitespace)
            {
                limit
            } else {
                head.rfind(char::is_whitespace)
                    .filter(|&end| end > 0)
                    .unwrap_or(limit)
            }
        });
        let line = remaining[..end].trim_end();
        remaining = remaining[end..].trim_start();
        Some(line)
    })
}

fn rows(value: &str, width: f32) -> Option<usize> {
    let lines = wrapped(value, width).take(MAX_LINES + 1).count();
    (lines <= MAX_LINES && value.len() <= MAX_TEXT_BYTES).then_some(lines)
}

fn draw_lines(
    sugarloaf: &mut Sugarloaf,
    value: &str,
    x: f32,
    mut y: f32,
    width: f32,
    opts: &DrawOpts,
) -> f32 {
    for line in wrapped(value, width).take(MAX_LINES) {
        draw(sugarloaf, line, x, y, width, opts);
        y += LINE_HEIGHT;
    }
    y
}

fn draw(
    sugarloaf: &mut Sugarloaf,
    value: &str,
    x: f32,
    y: f32,
    width: f32,
    opts: &DrawOpts,
) {
    let fit = elide_end(sugarloaf, value, width, opts);
    sugarloaf.text_mut().draw(x, y, &fit, opts);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn offer() -> WslSetupDialog {
        WslSetupDialog {
            title: "Set up WSL",
            subtitle: Some("Ubuntu 26.04 · Bash".into()),
            details: vec![WslSetupDetail {
                label: "eza",
                description: "Adds file and folder icons to listings.".into(),
                emphasized: true,
            }],
            note: Some("APT opens in a separate terminal tab.".into()),
            tone: Tone::Warning,
            can_install_bash: true,
            can_install_eza: true,
        }
    }

    #[test]
    fn wsl_setup_button_bounds_match_scaled_pointer_hits() {
        let layout = Layout::new((1050.0, 585.0, 1.5), &offer());
        assert!(layout.is_enabled(Choice::InstallBash));
        assert!(layout.is_enabled(Choice::InstallEza));
        assert_eq!(Choice::default(), Choice::Dismiss);
        for (choice, rect) in layout.buttons() {
            let [x, y, width, height] = rect.bounds(1.5);
            assert_eq!(
                layout.hit((x + width / 2.0, y + height / 2.0)),
                Some(choice)
            );
        }
        assert_eq!(layout.hit((0.0, 0.0)), None);
    }

    #[test]
    fn wsl_setup_keyboard_and_accessibility_distinguish_the_two_actions() {
        let mut dialog = offer();
        let dimensions = (1050.0, 585.0, 1.5);
        let layout = Layout::new(dimensions, &dialog);
        assert_eq!(
            layout.next_focus(Choice::Dismiss, false),
            Choice::InstallBash
        );
        assert_eq!(
            layout.next_focus(Choice::InstallBash, false),
            Choice::InstallEza
        );
        assert_eq!(
            layout.next_focus(Choice::InstallEza, false),
            Choice::Dismiss
        );
        assert_eq!(layout.next_focus(Choice::Dismiss, true), Choice::InstallEza);
        assert_eq!(
            layout.next_focus(Choice::InstallEza, true),
            Choice::InstallBash
        );
        assert_eq!(
            layout.next_focus(Choice::InstallBash, true),
            Choice::Dismiss
        );

        let surface = layout.accessibility_surface(&dialog, Choice::InstallBash);
        let actions: Vec<_> = surface
            .elements
            .iter()
            .filter(|element| element.node.role() == accesskit::Role::Button)
            .map(|element| (element.node.label(), element.key == surface.focus))
            .collect();
        assert_eq!(
            actions,
            [
                (Some("Cancel"), false),
                (Some("Install Bash integration"), true),
                (Some("Install eza"), false),
            ]
        );

        dialog.can_install_eza = false;
        let layout = Layout::new(dimensions, &dialog);
        assert!(layout.is_enabled(Choice::InstallBash));
        assert!(!layout.is_enabled(Choice::InstallEza));
        assert_eq!(
            layout.next_focus(Choice::Dismiss, true),
            Choice::InstallBash
        );
    }

    #[test]
    fn wsl_setup_install_requires_space_for_the_disclosure() {
        let mut dialog = offer();
        for dimensions in [
            (176.0, 96.0, 1.0),
            (320.0, 164.0, 2.0),
            (400.0, 800.0, 1.0),
            (0.0, 0.0, f32::NAN),
        ] {
            let layout = Layout::new(dimensions, &dialog);
            assert!(layout.is_enabled(Choice::Dismiss));
            assert!(!layout.is_enabled(Choice::InstallBash));
            assert!(!layout.is_enabled(Choice::InstallEza));
            assert!(layout.card.width * layout.viewport.scale <= dimensions.0);
            assert!(layout.card.height * layout.viewport.scale <= dimensions.1);
        }
        dialog.note = Some("x".repeat(MAX_TEXT_BYTES + 1));
        let layout = Layout::new((1000.0, 800.0, 1.0), &dialog);
        assert!(!layout.is_enabled(Choice::InstallBash));
        assert!(!layout.is_enabled(Choice::InstallEza));
    }

    #[test]
    fn wsl_setup_wraps_disclosure_without_losing_distro_text() {
        let distro = "LongDistributionNameThatCannotFitOnOneLine";
        let lines = wrapped(distro, 80.0).collect::<Vec<_>>();
        assert!(lines.len() > 1);
        assert_eq!(lines.concat(), distro);
        assert_eq!(
            wrapped("APT opens in a tab", 100.0).collect::<Vec<_>>(),
            ["APT opens", "in a tab"]
        );
    }

    #[test]
    #[ignore = "same-host layout and hit-testing microbenchmark"]
    fn wsl_setup_layout_benchmark() {
        use std::{hint::black_box, time::Instant};

        let mut dialog = offer();
        dialog.details.push(WslSetupDetail {
            label: "Shell integration",
            description: "The managed startup hook is missing. Add it to enable command colors and prompt tags.".into(),
            emphasized: true,
        });
        for dimensions in [(900.0, 640.0, 1.0), (960.0, 690.0, 1.5)] {
            let expected = Layout::new(dimensions, &dialog);
            let points = [expected.install_bash, expected.install_eza].map(|rect| {
                let rect = rect.expect("benchmark viewport fits the setup offer");
                (
                    (rect.x + rect.width / 2.0) * dimensions.2,
                    (rect.y + rect.height / 2.0) * dimensions.2,
                )
            });
            let mut samples = Vec::with_capacity(1000);
            for iteration in 0..1100 {
                let start = Instant::now();
                let layout = Layout::new(black_box(dimensions), black_box(&dialog));
                let hits = black_box(points.map(|point| layout.hit(black_box(point))));
                let elapsed = start.elapsed().as_nanos();
                assert_eq!(hits, [Some(Choice::InstallBash), Some(Choice::InstallEza)]);
                if iteration >= 100 {
                    samples.push(elapsed);
                }
            }
            samples.sort_unstable();
            println!(
                "{}",
                serde_json::json!({
                    "benchmark": "wsl_setup_layout_and_hit", "dimensions": dimensions,
                    "warmup": 100, "samples": samples.len(), "p50_ns": samples[499],
                    "p95_ns": samples[949], "excludes": ["text shaping", "painting", "presentation"]
                })
            );
        }
    }
}
