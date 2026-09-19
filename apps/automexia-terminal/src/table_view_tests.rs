use super::*;

fn view() -> TableView {
    let mut view = TableView::default();
    let table = Table::detect(
        (0..40)
            .map(|row| format!("row{row:02}     value-{row:02}       detail-{row:02}"))
            .collect(),
        cell_width,
    )
    .unwrap();
    view.open(7, Ok(table));
    view.fit(240.0, 220.0, 14.0, 9.0);
    view
}

#[test]
fn table_view_navigation_is_local_bounded_and_reversible() {
    let mut view = view();
    let original = view.copy_text().unwrap();
    let mods = ModifiersState::empty();
    for _ in 0..200 {
        assert_eq!(
            view.key(&Key::Named(NamedKey::ArrowRight), mods, true),
            Effect::Consumed
        );
        view.key(&Key::Named(NamedKey::ArrowDown), mods, true);
    }
    assert!(view.viewport.column() > 0);
    assert!(view.viewport.row() > 0);
    view.key(&Key::Named(NamedKey::Home), ModifiersState::CONTROL, true);
    assert_eq!((view.viewport.column(), view.viewport.row()), (0, 0));
    assert_eq!(view.copy_text().unwrap(), original);
    assert!(!original.contains('│'));
    assert_eq!(
        view.key(&Key::Character("v".into()), ModifiersState::CONTROL, true),
        Effect::Consumed
    );
    assert_eq!(
        view.key(&Key::Named(NamedKey::Escape), mods, false),
        Effect::Consumed
    );
    assert_eq!(
        view.key(&Key::Named(NamedKey::Escape), mods, true),
        Effect::Close
    );
    view.close();
    assert!(view.content.is_none());
    assert!(view.copy_text().is_none());
    assert_eq!(
        view.key(&Key::Character("x".into()), mods, true),
        Effect::Pass
    );
}

#[test]
fn table_view_keyboard_focus_exposes_copy_and_back() {
    let mut view = view();
    let mods = ModifiersState::empty();
    view.key(&Key::Named(NamedKey::Tab), mods, true);
    assert_eq!(
        view.key(&Key::Named(NamedKey::Enter), mods, true),
        Effect::Copy
    );
    view.key(&Key::Named(NamedKey::Tab), ModifiersState::SHIFT, true);
    assert_eq!(
        view.key(&Key::Named(NamedKey::Enter), mods, true),
        Effect::Close
    );
    assert_eq!(
        view.key(&Key::Character("c".into()), ModifiersState::SUPER, true),
        Effect::Copy
    );
}

#[test]
fn table_view_fractional_scroll_invalid_deltas_and_route_cleanup() {
    let mut view = view();
    for _ in 0..4 {
        view.scroll(0.25, 0.25);
    }
    assert_eq!((view.viewport.column(), view.viewport.row()), (1, 1));
    view.scroll(f32::NAN, f32::INFINITY);
    assert_eq!((view.viewport.column(), view.viewport.row()), (1, 1));
    view.retain_route(7);
    assert!(view.is_open());
    view.retain_route(8);
    assert!(!view.is_open());
    assert_eq!(view.wheel, [0.0; 2]);
}

#[test]
fn table_view_tiny_to_8k_layouts_never_overlap_or_escape() {
    let mut view = view();
    let source = view.copy_text().unwrap();
    for scale in [1.0, 1.25, 2.0, 3.0] {
        for (w, h) in [
            (0.0, 0.0),
            (1.0, 1.0),
            (100.0, 80.0),
            (240.0, 120.0),
            (800.0, 600.0),
            (7680.0, 4320.0),
        ] {
            view.fit(w / scale, h / scale, 14.0, 9.0);
            let l = view.layout;
            for r in [l.body, l.track, l.back, l.copy] {
                assert!(r.w >= 0.0 && r.h >= 0.0);
                assert!(r.x + r.w <= l.width + 0.01);
                assert!(r.y + r.h <= l.height + 0.01);
            }
            if l.copy.w > 0.0 {
                assert!(l.back.x + l.back.w <= l.copy.x);
            }
            view.key(&Key::Named(NamedKey::End), ModifiersState::CONTROL, true);
            assert_eq!(view.copy_text().unwrap(), source);
        }
    }
}

#[test]
fn table_view_ime_and_file_drop_cannot_reach_the_shell() {
    let mut view = view();
    assert_eq!(
        view.event(
            &WindowEvent::Ime(rio_window::event::Ime::Commit("untrusted input".into())),
            ModifiersState::empty(),
            1.0
        ),
        Effect::Consumed
    );
    assert_eq!(
        view.event(
            &WindowEvent::HoveredFileCancelled,
            ModifiersState::empty(),
            1.0
        ),
        Effect::Consumed
    );
    view.dragging = true;
    view.pressed = Some(true);
    view.event(&WindowEvent::Focused(false), ModifiersState::empty(), 1.0);
    assert!(!view.dragging);
    assert_eq!(view.pressed, None);
}

#[test]
fn table_view_scroll_track_reaches_both_edges() {
    let mut view = view();
    let track = view.layout.track;
    view.pointer = (track.x + track.w, track.y);
    view.pan_track();
    assert!(view.viewport.column() > 0);
    view.pointer = (track.x, track.y);
    view.pan_track();
    assert_eq!(view.viewport.column(), 0);
}

struct Canvas {
    text: rio_backend::sugarloaf::text::Text,
    rects: Vec<(Rect, [f32; 4])>,
}

#[test]
fn table_view_copy_request_does_not_invent_platform_confirmation() {
    let view = view();
    let theme = UiTheme::resolve(ui_theme::CARD, ui_theme::TEXT, ui_theme::MUTED_TEXT);
    let mut before = Canvas::new();
    view.draw(&mut before, theme);
    let pixels = before.pixels(240, 220);
    // Clipboard::set has no success result. Preparing a copy must not paint a
    // success badge when the platform might reject the actual clipboard write.
    assert!(view.copy_text().is_some());
    let mut after = Canvas::new();
    view.draw(&mut after, theme);
    assert_eq!(pixels, after.pixels(240, 220));
}

#[test]
fn table_view_narrow_error_guidance_wraps_and_has_no_dead_copy_action() {
    let theme = UiTheme::resolve(ui_theme::CARD, ui_theme::TEXT, ui_theme::MUTED_TEXT);
    let c = color_u8(theme.background);
    let background = (c[0] as u32) << 16 | (c[1] as u32) << 8 | c[2] as u32;
    for error in [
        TableError::NotTable,
        TableError::InvalidText,
        TableError::Capacity,
    ] {
        let mut view = TableView::default();
        view.open(7, Err(error));
        view.fit(280.0, 200.0, 14.0, 9.0);
        view.key(&Key::Named(NamedKey::Tab), ModifiersState::empty(), true);
        assert!(!view.copy_focused);
        assert_eq!(
            view.key(&Key::Character("c".into()), ModifiersState::CONTROL, true),
            Effect::Consumed
        );
        let mut canvas = Canvas::new();
        view.draw(&mut canvas, theme);
        let pixels = canvas.pixels(280, 200);
        assert!(
            pixels[78 * 280..100 * 280].iter().any(|p| *p != background),
            "long error guidance must paint the second line"
        );
    }
}
impl Canvas {
    fn new() -> Self {
        use rio_backend::sugarloaf::font::{
            constants, FontData, FontLibrary, FontLibraryData,
        };
        let mut data = FontLibraryData::default();
        data.insert(
            FontData::from_static_slice(constants::FONT_CASCADIA_CODE_NF).unwrap(),
        );
        let fonts = FontLibrary {
            inner: std::sync::Arc::new(parking_lot::RwLock::new(data)),
        };
        let mut text = rio_backend::sugarloaf::text::Text::new(&fonts);
        text.init_cpu();
        Self {
            text,
            rects: Vec::new(),
        }
    }
    fn pixels(&mut self, width: u32, height: u32) -> Vec<u32> {
        let mut pixels = vec![0; width as usize * height as usize];
        // Independent pixel-centre rasterizer; no production hit-test/layout
        // helper computes coverage. The real font shaper paints the labels.
        for (rect, color) in &self.rects {
            let c = color_u8(*color);
            let pixel = (c[0] as u32) << 16 | (c[1] as u32) << 8 | c[2] as u32;
            for y in 0..height {
                for x in 0..width {
                    let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
                    if px >= rect.x
                        && px < rect.x + rect.w
                        && py >= rect.y
                        && py < rect.y + rect.h
                    {
                        pixels[(y * width + x) as usize] = pixel;
                    }
                }
            }
        }
        self.text.render_cpu_base(&mut pixels, width, height);
        self.text.render_cpu_modal(&mut pixels, width, height);
        pixels
    }
}
impl TableCanvas for Canvas {
    fn text_mut(&mut self) -> &mut rio_backend::sugarloaf::text::Text {
        &mut self.text
    }
    fn begin_modal_layer(&mut self) {
        // This isolated surface has no underlying UI. CPU evidence tests the
        // exact draw stream, not the native compositor's modal phase ordering.
    }
    fn end_modal_layer(&mut self) {}
    fn paint_rect(&mut self, rect: Rect, color: [f32; 4]) {
        self.rects.push((rect, color));
    }
}

#[test]
fn table_view_parser_to_pixels_restores_exact_columns_after_extreme_navigation() {
    use rio_backend::{
        ansi::CursorShape,
        crosswords::{Crosswords, CrosswordsSize},
        event::{VoidListener, WindowId},
        performer::handler::Processor,
    };
    let mut terminal = Crosswords::new(
        CrosswordsSize::new(100, 24),
        CursorShape::Block,
        VoidListener {},
        WindowId::from(0),
        0,
        2000,
    );
    Processor::default().advance(&mut terminal, b"NAME                 READY    STATUS       RESTARTS    AGE\r\nexample-api          1/1      Running      0           2d\r\nexample-job          0/1      Completed    0           1d\r\nexample-cache        0/1      Pending      1           1d\r\n\r\nlambda ");
    let mut view = TableView::default();
    view.open(7, crate::automexia::table_output::capture(&terminal));
    let source = view.copy_text().unwrap();
    let theme = UiTheme::resolve(ui_theme::CARD, ui_theme::TEXT, ui_theme::MUTED_TEXT);
    view.fit(760.0, 260.0, 14.0, 9.0);
    let mut before = Canvas::new();
    view.draw(&mut before, theme);
    // Independent fixture geometry: 16px margin, 56px toolbar, 22px rows.
    let row_lines: Vec<_> = before
        .rects
        .iter()
        .filter(|(r, _)| r.h == 1.0 && r.w == 728.0)
        .map(|(r, _)| r.y)
        .collect();
    assert_eq!(row_lines, [77.0, 99.0, 121.0, 143.0]);
    let pixels = before.pixels(760, 260);
    for (w, h) in [(1.0, 1.0), (130.0, 180.0), (7680.0, 4320.0), (320.0, 180.0)] {
        view.fit(w, h, 14.0, 9.0);
        view.key(&Key::Named(NamedKey::End), ModifiersState::CONTROL, true);
    }
    view.key(&Key::Named(NamedKey::Home), ModifiersState::CONTROL, true);
    view.fit(760.0, 260.0, 14.0, 9.0);
    let mut after = Canvas::new();
    view.draw(&mut after, theme);
    assert_eq!(pixels, after.pixels(760, 260));
    assert_eq!(view.copy_text().unwrap(), source);
    let line = after
        .rects
        .iter_mut()
        .find(|(r, _)| r.y == 77.0 && r.h == 1.0)
        .unwrap();
    line.0.y += 1.0;
    assert_ne!(
        pixels,
        after.pixels(760, 260),
        "one-pixel separator mutation must be visible"
    );
    if let Some(path) = std::env::var_os("AUTOMEXIA_CORE_TABLE_PREVIEW") {
        image_rs::RgbImage::from_fn(760, 260, |x, y| {
            let pixel = pixels[(y * 760 + x) as usize];
            image_rs::Rgb([(pixel >> 16) as u8, (pixel >> 8) as u8, pixel as u8])
        })
        .save(path)
        .expect("write controlled core-table preview");
    }
}
