//! Full primitive raster evidence, separate from the lightweight layout recorder.
use super::*;
use rio_backend::sugarloaf::renderer::Renderer as PrimitiveRenderer;

struct ActualCanvas {
    renderer: PrimitiveRenderer,
    text: Text,
    fonts: rio_backend::sugarloaf::font::FontLibrary,
    scale: f32,
}
impl ActualCanvas {
    fn new(scale: f32) -> Self {
        let fonts = crate::visual_quality::fonts();
        let mut text = Text::new(&fonts);
        text.init_cpu();
        text.set_scale_factor(scale);
        Self {
            renderer: PrimitiveRenderer::controlled_cpu(),
            text,
            fonts,
            scale,
        }
    }
    fn pixels(&mut self, size: [u32; 2]) -> Vec<u32> {
        let mut pixels = vec![0x00112233; (size[0] * size[1]) as usize];
        self.renderer
            .capture_cpu_primitives(&mut pixels, size[0], size[1], &self.text)
            .unwrap();
        pixels
    }
}
impl Canvas for ActualCanvas {
    fn text(&mut self) -> &mut Text {
        &mut self.text
    }
    fn rect(&mut self, bounds: [f32; 4], color: [f32; 4]) {
        let [x, y, w, h] = bounds.map(|v| v * self.scale);
        self.renderer.rect(x, y, w, h, color, 0.0, 30);
    }
    fn rounded_rect(&mut self, bounds: [f32; 4], radius: f32, color: [f32; 4]) {
        let [x, y, w, h] = bounds.map(|v| v * self.scale);
        self.renderer
            .rounded_rect(x, y, w, h, color, 0.0, radius * self.scale, 30);
    }
    fn polygon(&mut self, points: &[(f32, f32)], color: [f32; 4]) {
        let points: Vec<_> = points
            .iter()
            .map(|(x, y)| (x * self.scale, y * self.scale))
            .collect();
        self.renderer.polygon_with_order(&points, 0.0, color, 30);
    }
    fn line(&mut self, from: (f32, f32), to: (f32, f32), width: f32, color: [f32; 4]) {
        let s = self.scale;
        self.renderer.line(
            from.0 * s,
            from.1 * s,
            to.0 * s,
            to.1 * s,
            width * s,
            0.0,
            color,
            30,
        );
    }
    fn arc(
        &mut self,
        center: (f32, f32),
        radius: f32,
        angles: (f32, f32),
        width: f32,
        color: [f32; 4],
    ) {
        let s = self.scale;
        self.renderer.arc(
            center.0 * s,
            center.1 * s,
            radius * s,
            angles.0,
            angles.1,
            width * s,
            0.0,
            color,
        );
    }
}

#[test]
fn visual_quality_real_shapes_detect_corner_and_one_pixel_mutations() {
    let render = |radius: f32, left: f32| {
        let mut canvas = ActualCanvas::new(1.0);
        canvas.rounded_rect([left, 4.0, 24.0, 24.0], radius, [0.0, 1.0, 1.0, 1.0]);
        canvas.pixels([32, 32])
    };
    let correct = render(8.0, 4.0);
    let square = render(0.0, 4.0);
    let shifted = render(8.0, 5.0);
    assert_eq!(
        correct[4 * 32 + 4],
        0x00112233,
        "corner must remain outside the card"
    );
    assert_eq!(
        square[4 * 32 + 4],
        0x0000ffff,
        "mutation must fill that corner"
    );
    assert_ne!(
        correct, square,
        "bounding-box substitution is not a valid pixel oracle"
    );
    assert_ne!(correct, shifted, "one physical pixel must be detected");
    assert_eq!(
        correct,
        render(8.0, 4.0),
        "restoration must return exact pixels"
    );
}

fn settings_golden_page(page: &str, name: &str) {
    for entry in crate::automexia::theme_gallery::builtins() {
        let colors = entry.theme.unwrap().colors;
        let theme = UiTheme::from_colors(&colors);
        let base = rio_backend::config::Config {
            colors,
            ..Default::default()
        };
        let theme_name = entry.name.to_ascii_lowercase().replace(' ', "-");
        for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
            let (width, height) = if scale == 1.75 {
                (480.0, 640.0)
            } else {
                (960.0, 740.0)
            };
            let size = [(width * scale) as u32, (height * scale) as u32];
            let mut view = match page {
                "color" => {
                    let mut view = opened_color(true);
                    view.set_color_favorites(&[[35, 120, 180, 255], [180, 70, 90, 255]]);
                    view
                }
                "font" => {
                    let mut view = installed_font_picker();
                    view.font_picker_inventory(
                        vec![
                            "Cascadia Code".into(),
                            "Example Mono".into(),
                            "Example Sans".into(),
                        ],
                        false,
                    );
                    view
                }
                "root" | "interface-root" | "gallery" => {
                    let mut view = dependent_settings_view(
                        &base,
                        &Default::default(),
                        COMMAND_TIMESTAMPS,
                    );
                    named(&mut view, NamedKey::Escape);
                    if page == "interface-root" {
                        view.show_terminal_appearance();
                    } else if page == "gallery" {
                        view.set_theme_context(ThemeContext {
                            configured: rio_backend::config::theme::Theme { colors },
                            saved: crate::automexia::theme_gallery::builtins()[0]
                                .selection(),
                            font_colors: false,
                        });
                        view.open_theme_gallery();
                        view.theme_inventory(
                            crate::automexia::theme_gallery::builtins(),
                            false,
                        );
                        named(&mut view, NamedKey::ArrowDown);
                    }
                    view
                }
                _ => dependent_settings_view(&base, &Default::default(), page),
            };
            view.fit(width, height, 16.0);
            let mut canvas = ActualCanvas::new(scale);
            view.paint(&mut canvas, theme);
            let pixels = canvas.pixels(size);
            let mut second = ActualCanvas::new(scale);
            view.paint(&mut second, theme);
            assert_eq!(
                pixels,
                second.pixels(size),
                "stable layout and atlas: {name}"
            );
            assert!(pixels.iter().filter(|p| **p != 0x00112233).count() > 5000);
            let card = if view.color_editor.is_some() {
                view.color_geometry.card
            } else {
                view.geometry.card
            };
            let bounds = [card.x, card.y, card.width, card.height].map(|v| v * scale);
            // A card must not silently grow a full-window backdrop. Test real
            // pixels wholly outside its measured bounds. A physical pixel
            // intersecting a fractional edge is allowed antialias coverage;
            // checking its center would mislabel that coverage as leakage.
            for y in 0..size[1] {
                for x in 0..size[0] {
                    if (x as f32 + 1.0) <= bounds[0]
                        || (y as f32 + 1.0) <= bounds[1]
                        || x as f32 >= bounds[0] + bounds[2]
                        || y as f32 >= bounds[1] + bounds[3]
                    {
                        assert_eq!(
                            pixels[(y * size[0] + x) as usize],
                            0x00112233,
                            "{name}: escaped card at {x},{y}"
                        );
                    }
                }
            }
            crate::visual_quality::export_with_fonts(
                &format!("settings-{name}-{theme_name}-{}", (scale * 100.0) as u32),
                "correct",
                &pixels,
                size,
                scale,
                &theme_name,
                vec![
                    ("card", bounds),
                    ("viewport", [0.0, 0.0, size[0] as f32, size[1] as f32]),
                ],
                &crate::visual_quality::font_hashes(&canvas.fonts),
            );
        }
    }
}

// Separate cases retain the full matrix within the existing per-test deadline.
macro_rules! settings_cases {
    ($($test:ident => ($page:expr, $name:literal)),+ $(,)?) => {$ (
        #[test]
        fn $test() { settings_golden_page($page, $name); }
    )+};
}
settings_cases! {
    visual_quality_settings_header => ("interface.header.background", "header"),
    visual_quality_settings_footer => ("interface.footer.visible", "footer"),
    visual_quality_settings_panes => ("interface.panes.padding", "panes"),
    visual_quality_settings_background => ("interface.background.opacity", "background"),
    visual_quality_settings_window_controls => (crate::settings_catalog::WINDOW_CONTROLS, "window-controls"),
    visual_quality_settings_fonts => (automexia_ui_model::settings::FONT_SIZE, "fonts"),
    visual_quality_settings_timestamps => (COMMAND_TIMESTAMPS, "timestamps"),
    visual_quality_settings_tables => (INLINE_TABLES, "tables"),
    visual_quality_settings_tags => ("tags.enabled", "tags"),
    visual_quality_settings_output => ("terminal.command_output_highlighting", "output"),
    visual_quality_settings_kubernetes => ("terminal.kubernetes_highlighting", "kubernetes"),
    visual_quality_settings_customizations => ("root", "customizations"),
    visual_quality_settings_interface => ("interface-root", "terminal-interface"),
    visual_quality_settings_gallery => ("gallery", "theme-gallery"),
    visual_quality_settings_color_picker => ("color", "color-picker"),
    visual_quality_settings_font_picker => ("font", "font-picker"),
}
