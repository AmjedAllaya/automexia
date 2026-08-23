use crate::renderer::responsive::{elide_end, Viewport};
use crate::renderer::ui_theme::{
    color_u8, UiTheme, BRAND_CYAN, BRAND_PURPLE, MODAL_SHADOW,
};
use rio_backend::config::colors::Colors;
use rio_backend::sugarloaf::text::DrawOpts;
use rio_backend::sugarloaf::Sugarloaf;

const IDEAL_WIDTH: f32 = 560.0;
const IDEAL_HEIGHT: f32 = 280.0;
const MARGIN: f32 = 18.0;
const PADDING: f32 = 24.0;
const ACTION_HEIGHT: f32 = 42.0;
const ORDER: u8 = 1;
const TITLE: &str = "Welcome to Automexia";
const SUBTITLE: &str = "Save time. Do more with less effort. Stay flexible.";

#[derive(Clone, Copy, Debug, PartialEq)]
struct Rect {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct WelcomeLayout {
    card: Rect,
    action: Rect,
    compact: bool,
    tiny: bool,
}

fn welcome_layout(dimensions: (f32, f32, f32)) -> WelcomeLayout {
    let viewport = Viewport::from_physical(dimensions.0, dimensions.1, dimensions.2);
    let margin = if viewport.width < 280.0 || viewport.height < 180.0 {
        6.0
    } else {
        MARGIN
    };
    let width = IDEAL_WIDTH.min((viewport.width - margin * 2.0).max(1.0));
    let height = IDEAL_HEIGHT.min((viewport.height - margin * 2.0).max(1.0));
    let card = Rect {
        x: ((viewport.width - width) * 0.5).max(0.0),
        y: ((viewport.height - height) * 0.5).max(0.0),
        width,
        height,
    };
    let compact = width < 420.0 || height < 260.0;
    let tiny = width < 250.0 || height < 145.0;
    let inset = if tiny {
        6.0
    } else if compact {
        14.0
    } else {
        PADDING
    };
    let action_height = ACTION_HEIGHT.min((height - inset * 2.0).max(24.0));
    let action = Rect {
        x: card.x + inset,
        y: (card.y + height - inset - action_height).max(card.y),
        width: (width - inset * 2.0).max(1.0),
        height: action_height,
    };
    WelcomeLayout {
        card,
        action,
        compact,
        tiny,
    }
}

#[inline]
pub fn screen(sugarloaf: &mut Sugarloaf, colors: &Colors) {
    let window = sugarloaf.window_size();
    let scale = sugarloaf.scale_factor();
    let dimensions = (window.width, window.height, scale);
    let viewport = Viewport::from_physical(dimensions.0, dimensions.1, dimensions.2);
    let layout = welcome_layout(dimensions);
    let theme = UiTheme::resolve(colors.background.0, colors.foreground, colors.black);

    sugarloaf.rect(
        None,
        0.0,
        0.0,
        viewport.width,
        viewport.height,
        theme.background,
        0.0,
        0,
    );

    rounded(
        sugarloaf,
        layout.card.x + 7.0,
        layout.card.y + 9.0,
        layout.card.width,
        layout.card.height,
        MODAL_SHADOW,
        16.0,
    );
    rounded(
        sugarloaf,
        layout.card.x,
        layout.card.y,
        layout.card.width,
        layout.card.height,
        BRAND_PURPLE,
        16.0,
    );
    rounded(
        sugarloaf,
        layout.card.x + 1.0,
        layout.card.y + 1.0,
        (layout.card.width - 2.0).max(1.0),
        (layout.card.height - 2.0).max(1.0),
        theme.background,
        15.0,
    );

    let inset = if layout.tiny {
        6.0
    } else if layout.compact {
        14.0
    } else {
        PADDING
    };
    let icon_size = if layout.tiny {
        30.0
    } else if layout.compact {
        40.0
    } else {
        52.0
    };
    let icon_x = layout.card.x + (layout.card.width - icon_size) * 0.5;
    let icon_y = layout.card.y + inset;
    draw_terminal_mark(sugarloaf, icon_x, icon_y, icon_size, theme.background);

    let title_size = if layout.tiny {
        13.0
    } else if layout.compact {
        17.0
    } else {
        21.0
    };
    let title_y = icon_y + icon_size + if layout.tiny { 5.0 } else { 13.0 };
    draw_centered(
        sugarloaf,
        TITLE,
        layout.card.x + inset,
        title_y,
        (layout.card.width - inset * 2.0).max(1.0),
        title_size,
        theme.text,
        true,
    );

    if !layout.tiny {
        let subtitle_y = title_y + title_size + 12.0;
        draw_centered(
            sugarloaf,
            SUBTITLE,
            layout.card.x + inset,
            subtitle_y,
            (layout.card.width - inset * 2.0).max(1.0),
            if layout.compact { 11.5 } else { 12.5 },
            theme.muted_text,
            false,
        );
    }

    rounded(
        sugarloaf,
        layout.action.x,
        layout.action.y,
        layout.action.width,
        layout.action.height,
        theme.surface,
        9.0,
    );
    rounded(
        sugarloaf,
        layout.action.x,
        layout.action.y,
        layout.action.width,
        2.0,
        BRAND_CYAN,
        1.0,
    );
    let action_label = if layout.tiny {
        "Enter · continue"
    } else if layout.compact {
        "Enter · get started"
    } else {
        "Press Enter to get started"
    };
    draw_centered(
        sugarloaf,
        action_label,
        layout.action.x + 8.0,
        layout.action.y + (layout.action.height - 12.5) * 0.5,
        (layout.action.width - 16.0).max(1.0),
        12.5,
        BRAND_CYAN,
        true,
    );
}

fn draw_terminal_mark(
    sugarloaf: &mut Sugarloaf,
    x: f32,
    y: f32,
    size: f32,
    background: [f32; 4],
) {
    rounded(sugarloaf, x, y, size, size, BRAND_PURPLE, 10.0);
    rounded(
        sugarloaf,
        x + 2.0,
        y + 2.0,
        (size - 4.0).max(1.0),
        (size - 4.0).max(1.0),
        background,
        8.0,
    );
    let opts = DrawOpts {
        font_size: (size * 0.38).max(10.0),
        color: color_u8(BRAND_CYAN),
        bold: true,
        ..DrawOpts::default()
    };
    sugarloaf
        .text_mut()
        .draw(x + size * 0.19, y + size * 0.27, ">_", &opts);
}

#[allow(clippy::too_many_arguments)]
fn draw_centered(
    sugarloaf: &mut Sugarloaf,
    text: &str,
    x: f32,
    y: f32,
    width: f32,
    font_size: f32,
    color: [f32; 4],
    bold: bool,
) {
    let opts = DrawOpts {
        font_size,
        color: color_u8(color),
        bold,
        ..DrawOpts::default()
    };
    let visible = elide_end(sugarloaf, text, width, font_size);
    let measured = sugarloaf.text_mut().measure(&visible, &opts);
    sugarloaf.text_mut().draw(
        x + ((width - measured) * 0.5).max(0.0),
        y,
        &visible,
        &opts,
    );
}

fn rounded(
    sugarloaf: &mut Sugarloaf,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    color: [f32; 4],
    radius: f32,
) {
    sugarloaf.rounded_rect(None, x, y, width, height, color, 0.05, radius, ORDER);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn welcome_surface_fits_small_and_normal_windows() {
        for dimensions in [
            (180.0, 90.0, 1.0),
            (320.0, 220.0, 1.0),
            (1_200.0, 800.0, 2.0),
        ] {
            let viewport =
                Viewport::from_physical(dimensions.0, dimensions.1, dimensions.2);
            let layout = welcome_layout(dimensions);
            assert!(layout.card.x >= 0.0 && layout.card.y >= 0.0);
            assert!(layout.card.x + layout.card.width <= viewport.width);
            assert!(layout.card.y + layout.card.height <= viewport.height);
            assert!(layout.action.x >= layout.card.x);
            assert!(layout.action.y >= layout.card.y);
            assert!(
                layout.action.x + layout.action.width
                    <= layout.card.x + layout.card.width
            );
            assert!(
                layout.action.y + layout.action.height
                    <= layout.card.y + layout.card.height
            );
        }
    }

    #[test]
    fn welcome_surface_is_dpi_invariant() {
        assert_eq!(
            welcome_layout((600.0, 400.0, 1.0)),
            welcome_layout((1_200.0, 800.0, 2.0))
        );
    }

    #[test]
    fn normal_action_meets_minimum_target_height() {
        let layout = welcome_layout((800.0, 600.0, 1.0));
        assert!(layout.action.width >= 24.0);
        assert!(layout.action.height >= 24.0);
    }

    #[test]
    fn welcome_copy_is_product_specific_and_does_not_expose_a_path() {
        assert!(TITLE.contains("Automexia"));
        assert_eq!(
            SUBTITLE,
            "Save time. Do more with less effort. Stay flexible."
        );
        for text in [TITLE, SUBTITLE] {
            assert!(!text.contains('/'));
            assert!(!text.contains('\\'));
            assert!(!text.contains(':'));
        }
    }
}
