//! Draw-only CP5 suggestion overlay.
//!
//! Candidate production, ranking, editor state, insertion, endpoints, and
//! capabilities never enter this module.

use automexia_ui_model::suggestions::{Rect, SuggestionSurface, SuggestionSurfaceKind};
use rio_backend::config::colors::Colors;
use rio_backend::sugarloaf::text::DrawOpts;
use rio_backend::sugarloaf::Sugarloaf;
use unicode_segmentation::UnicodeSegmentation;

use super::ui_theme::{
    color_u8, UiTheme, BRAND_AMBER, BRAND_CORAL, BRAND_CYAN, BRAND_LIME, BRAND_PURPLE,
};

const ORDER: u8 = 28;
const DEPTH_SHADOW: f32 = 0.04;
const DEPTH_SURFACE: f32 = 0.09;
const DEPTH_ROW: f32 = 0.14;
const HEADER_FONT: f32 = 10.0;
const VALUE_FONT: f32 = 13.0;
const META_FONT: f32 = 9.0;

#[derive(Clone, Copy, Debug, PartialEq)]
struct RowLayout {
    bounds: Rect,
    icon: Rect,
    value_x: f32,
    value_width: f32,
    meta_x: f32,
    meta_width: f32,
}

fn row_layout(surface: &SuggestionSurface, index: usize) -> Option<RowLayout> {
    let y = surface.bounds.y + surface.header_height + surface.row_height * index as f32;
    let bounds = Rect::new(
        surface.bounds.x + surface.border_width,
        y,
        surface.bounds.width - surface.border_width * 2.0,
        surface.row_height,
    )?;
    if bounds.bottom() > surface.bounds.bottom() {
        return None;
    }
    let padding = (8.0 * surface.border_width.max(1.0)).min(16.0);
    let icon_size = (surface.row_height - 10.0).clamp(14.0, 22.0);
    let icon = Rect::new(
        bounds.x + padding,
        bounds.y + (bounds.height - icon_size) / 2.0,
        icon_size,
        icon_size,
    )?;
    let value_x = icon.right() + 8.0;
    let meta_width = if surface.show_freshness {
        (bounds.width * 0.28).clamp(72.0, 180.0)
    } else {
        0.0
    };
    let value_width = (bounds.right() - padding - meta_width - value_x).max(1.0);
    Some(RowLayout {
        bounds,
        icon,
        value_x,
        value_width,
        meta_x: bounds.right() - padding - meta_width,
        meta_width,
    })
}

#[derive(Default)]
pub struct SuggestionOverlay {
    surface: Option<SuggestionSurface>,
    row_bounds: Vec<Rect>,
}

impl SuggestionOverlay {
    pub fn set_surface(&mut self, surface: SuggestionSurface) {
        self.surface = Some(surface);
    }

    pub fn dismiss(&mut self) {
        self.surface = None;
        self.row_bounds.clear();
    }

    pub fn is_active(&self) -> bool {
        self.surface.is_some()
    }

    pub fn surface(&self) -> Option<&SuggestionSurface> {
        self.surface.as_ref()
    }

    pub fn render(&mut self, sugarloaf: &mut Sugarloaf, colors: &Colors) {
        self.row_bounds.clear();
        let Some(surface) = self.surface.as_ref() else {
            return;
        };
        let muted = colors.dim_foreground.unwrap_or(colors.tabs);
        let theme = UiTheme::resolve(colors.background.0, colors.foreground, muted);
        let outline = if surface.high_contrast {
            theme.text
        } else {
            theme.outline
        };

        sugarloaf.rounded_rect(
            None,
            surface.bounds.x + 2.0,
            surface.bounds.y + 4.0,
            surface.bounds.width,
            surface.bounds.height,
            [0.0, 0.0, 0.0, 0.44],
            DEPTH_SHADOW,
            surface.corner_radius,
            ORDER,
        );
        sugarloaf.rounded_rect(
            None,
            surface.bounds.x,
            surface.bounds.y,
            surface.bounds.width,
            surface.bounds.height,
            outline,
            DEPTH_SURFACE,
            surface.corner_radius,
            ORDER,
        );
        let inset = surface.border_width.max(1.0);
        sugarloaf.rounded_rect(
            None,
            surface.bounds.x + inset,
            surface.bounds.y + inset,
            (surface.bounds.width - inset * 2.0).max(0.0),
            (surface.bounds.height - inset * 2.0).max(0.0),
            theme.background,
            DEPTH_ROW,
            (surface.corner_radius - inset).max(0.0),
            ORDER,
        );

        if surface.kind == SuggestionSurfaceKind::CompactHint {
            draw_text(
                sugarloaf,
                surface.bounds.x + 10.0,
                surface.bounds.y + (surface.bounds.height - VALUE_FONT) / 2.0,
                &surface.accessible_name,
                surface.bounds.width - 20.0,
                VALUE_FONT,
                theme.text,
            );
            return;
        }

        draw_text(
            sugarloaf,
            surface.bounds.x + 12.0,
            surface.bounds.y + (surface.header_height - HEADER_FONT) / 2.0,
            "SUGGESTIONS",
            surface.bounds.width * 0.6,
            HEADER_FONT,
            BRAND_CYAN,
        );
        draw_text_right(
            sugarloaf,
            surface.bounds.right() - 12.0,
            surface.bounds.y + (surface.header_height - HEADER_FONT) / 2.0,
            "LOCAL",
            HEADER_FONT,
            theme.muted_text,
        );

        for (index, option) in surface.options.iter().enumerate() {
            let Some(layout) = row_layout(surface, index) else {
                break;
            };
            self.row_bounds.push(layout.bounds);
            if option.selected || option.pointer_highlighted {
                let color = if option.selected {
                    [BRAND_CYAN[0], BRAND_CYAN[1], BRAND_CYAN[2], 0.18]
                } else {
                    [theme.raised[0], theme.raised[1], theme.raised[2], 0.82]
                };
                sugarloaf.rounded_rect(
                    None,
                    layout.bounds.x + 3.0,
                    layout.bounds.y + 2.0,
                    layout.bounds.width - 6.0,
                    layout.bounds.height - 4.0,
                    color,
                    DEPTH_ROW,
                    6.0,
                    ORDER,
                );
            }

            let kind_color = kind_color(&option.kind);
            sugarloaf.rounded_rect(
                None,
                layout.icon.x,
                layout.icon.y,
                layout.icon.width,
                layout.icon.height,
                kind_color,
                DEPTH_ROW,
                5.0,
                ORDER,
            );
            draw_text_center(
                sugarloaf,
                layout.icon,
                kind_icon(&option.kind),
                META_FONT,
                theme.background,
            );

            let value_y = if surface.show_description {
                layout.bounds.y + 5.0
            } else {
                layout.bounds.y + (layout.bounds.height - VALUE_FONT) / 2.0
            };
            draw_matched_text(
                sugarloaf,
                layout.value_x,
                value_y,
                &option.display,
                &option.matched_graphemes,
                layout.value_width,
                VALUE_FONT,
                theme.text,
                BRAND_CYAN,
            );
            if surface.show_description {
                draw_text(
                    sugarloaf,
                    layout.value_x,
                    layout.bounds.y + layout.bounds.height - META_FONT - 5.0,
                    &option.description,
                    layout.value_width,
                    META_FONT,
                    theme.muted_text,
                );
            }
            if surface.show_freshness {
                draw_text_right(
                    sugarloaf,
                    layout.meta_x + layout.meta_width,
                    layout.bounds.y + 5.0,
                    &option.source,
                    META_FONT,
                    theme.muted_text,
                );
                let risk_color = if option.risk == "destructive" {
                    BRAND_CORAL
                } else if option.risk == "changes state" {
                    BRAND_AMBER
                } else {
                    BRAND_LIME
                };
                draw_text_right(
                    sugarloaf,
                    layout.meta_x + layout.meta_width,
                    layout.bounds.y + layout.bounds.height - META_FONT - 5.0,
                    &format!("{} / {}", option.freshness, option.risk),
                    META_FONT,
                    risk_color,
                );
            }
        }
    }
}

fn kind_icon(kind: &str) -> &'static str {
    match kind {
        "command" => ">",
        "option" => "-",
        "argument" => "A",
        "path" => "/",
        "history" => "H",
        "context" => "C",
        "action" => "!",
        _ => "?",
    }
}

fn kind_color(kind: &str) -> [f32; 4] {
    match kind {
        "command" => BRAND_CYAN,
        "option" | "argument" => BRAND_PURPLE,
        "path" => BRAND_LIME,
        "history" => BRAND_AMBER,
        "action" => BRAND_CORAL,
        _ => BRAND_CYAN,
    }
}

fn draw_text(
    sugarloaf: &mut Sugarloaf,
    x: f32,
    y: f32,
    text: &str,
    maximum_width: f32,
    font_size: f32,
    color: [f32; 4],
) {
    let options = DrawOpts {
        font_size,
        color: color_u8(color),
        ..DrawOpts::default()
    };
    let display = elide_end(sugarloaf, text, maximum_width, &options);
    sugarloaf.text_mut().draw(x, y, &display, &options);
}

#[allow(clippy::too_many_arguments)]
fn draw_matched_text(
    sugarloaf: &mut Sugarloaf,
    x: f32,
    y: f32,
    text: &str,
    matched_graphemes: &[usize],
    maximum_width: f32,
    font_size: f32,
    color: [f32; 4],
    matched_color: [f32; 4],
) {
    let measurement = DrawOpts {
        font_size,
        color: color_u8(color),
        ..DrawOpts::default()
    };
    let display = elide_end(sugarloaf, text, maximum_width, &measurement);
    let mut current_x = x;
    for (matched, run) in matched_grapheme_runs(&display, matched_graphemes) {
        let options = DrawOpts {
            font_size,
            color: color_u8(if matched { matched_color } else { color }),
            ..DrawOpts::default()
        };
        sugarloaf.text_mut().draw(current_x, y, &run, &options);
        current_x += sugarloaf.text_mut().measure(&run, &options);
    }
}

fn matched_grapheme_runs(text: &str, matched: &[usize]) -> Vec<(bool, String)> {
    let mut runs = Vec::<(bool, String)>::new();
    for (index, grapheme) in text.graphemes(true).enumerate() {
        let is_matched = matched.binary_search(&index).is_ok();
        if let Some((last_matched, value)) = runs.last_mut() {
            if *last_matched == is_matched {
                value.push_str(grapheme);
                continue;
            }
        }
        runs.push((is_matched, grapheme.to_string()));
    }
    runs
}
fn draw_text_right(
    sugarloaf: &mut Sugarloaf,
    right: f32,
    y: f32,
    text: &str,
    font_size: f32,
    color: [f32; 4],
) {
    let options = DrawOpts {
        font_size,
        color: color_u8(color),
        ..DrawOpts::default()
    };
    let width = sugarloaf.text_mut().measure(text, &options);
    sugarloaf
        .text_mut()
        .draw((right - width).max(0.0), y, text, &options);
}

fn draw_text_center(
    sugarloaf: &mut Sugarloaf,
    rect: Rect,
    text: &str,
    font_size: f32,
    color: [f32; 4],
) {
    let options = DrawOpts {
        font_size,
        color: color_u8(color),
        ..DrawOpts::default()
    };
    let width = sugarloaf.text_mut().measure(text, &options);
    sugarloaf.text_mut().draw(
        rect.x + (rect.width - width) / 2.0,
        rect.y + (rect.height - font_size) / 2.0,
        text,
        &options,
    );
}

fn elide_end(
    sugarloaf: &mut Sugarloaf,
    text: &str,
    maximum_width: f32,
    options: &DrawOpts,
) -> String {
    if maximum_width <= 0.0 {
        return String::new();
    }
    if sugarloaf.text_mut().measure(text, options) <= maximum_width {
        return text.to_string();
    }
    let graphemes = text.graphemes(true).collect::<Vec<_>>();
    for count in (0..graphemes.len()).rev() {
        let candidate = format!("{}...", graphemes[..count].concat());
        if sugarloaf.text_mut().measure(&candidate, options) <= maximum_width {
            return candidate;
        }
    }
    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn row_geometry_never_crosses_surface_bounds() {
        let mut surface: SuggestionSurface = serde_json::from_str(
            r#"{
                "kind":"listbox",
                "role":"listbox",
                "accessible_name":"Local suggestions, 2 results",
                "bounds":{"x":100.0,"y":200.0,"width":520.0,"height":86.0},
                "options":[],
                "selected":0,
                "interactive":true,
                "opacity_duration_ms":0,
                "high_contrast":false,
                "border_width":1.0,
                "corner_radius":10.0,
                "header_height":30.0,
                "row_height":24.0,
                "show_description":true,
                "show_freshness":true
            }"#,
        )
        .unwrap();
        surface.options = Vec::new();
        let first = row_layout(&surface, 0).unwrap();
        let second = row_layout(&surface, 1).unwrap();
        assert!(surface.bounds.contains(first.bounds));
        assert!(surface.bounds.contains(second.bounds));
        assert!(first.bounds.contains(first.icon));
        assert!(first.value_x + first.value_width <= first.meta_x);
        assert!(row_layout(&surface, 3).is_none());
    }

    #[test]
    fn kind_icons_are_redundant_with_text_labels() {
        for kind in [
            "command", "option", "argument", "path", "history", "context", "action",
        ] {
            assert_ne!(kind_icon(kind), "?");
        }
        assert_eq!(kind_icon("unknown"), "?");
    }

    #[test]
    fn matched_runs_preserve_combining_and_joined_graphemes() {
        assert_eq!(
            matched_grapheme_runs("e\u{301}cho 👨\u{200d}💻", &[0, 5]),
            vec![
                (true, "e\u{301}".into()),
                (false, "cho ".into()),
                (true, "👨\u{200d}💻".into()),
            ]
        );
    }
}
