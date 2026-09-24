use super::*;
use automexia_ui_model::settings::{
    core_descriptors, Availability, CoreOrigins, CoreValues, COMMAND_TIMESTAMPS,
    INLINE_TABLES, MAX_QUERY_BYTES,
};
use rio_backend::sugarloaf::{
    font::{constants, FontData, FontLibrary, FontLibraryData},
    text::Text,
};
use rio_window::event::{DeviceId, Ime};

fn catalog(revision: u64) -> Catalog {
    Catalog::new(
        revision,
        core_descriptors(
            CoreValues::default(),
            CoreValues::default(),
            CoreOrigins::default(),
        ),
    )
    .unwrap()
}
fn opened() -> SettingsView {
    let mut view = SettingsView::default();
    view.fit(720.0, 560.0, 14.0);
    view.open(catalog(1));
    view
}
fn named(view: &mut SettingsView, key: NamedKey) {
    view.key(&Key::Named(key), None, ModifiersState::empty(), false);
}

#[test]
fn settings_keyboard_changes_one_option_and_reset_is_an_explicit_separate_intent() {
    let mut view = opened();
    named(&mut view, NamedKey::Tab);
    named(&mut view, NamedKey::Space);
    let edit = view.take_edit().unwrap();
    assert_eq!(edit.id.as_str(), INLINE_TABLES);
    assert_eq!(edit.change, Change::Set(SettingValue::Boolean(false)));
    named(&mut view, NamedKey::Tab);
    named(&mut view, NamedKey::Enter);
    assert_eq!(view.take_edit().unwrap().change, Change::Reset);
}

#[test]
fn queue_repeat_and_catalogue_refresh_cannot_apply_stale_or_duplicate_edits() {
    let mut view = opened();
    named(&mut view, NamedKey::Tab);
    view.key(
        &Key::Named(NamedKey::Space),
        None,
        ModifiersState::empty(),
        true,
    );
    assert!(view.take_edit().is_none());
    named(&mut view, NamedKey::Space);
    named(&mut view, NamedKey::ArrowDown);
    named(&mut view, NamedKey::Space);
    assert_eq!(view.take_edit().unwrap().id.as_str(), INLINE_TABLES);
    assert!(view.take_edit().is_none());
    named(&mut view, NamedKey::Space);
    view.refresh(catalog(2));
    assert!(view.take_edit().is_none());
}

#[test]
fn escape_closes_clears_draft_and_pending_intent_and_traps_no_later_events() {
    let mut view = opened();
    view.paste("table");
    named(&mut view, NamedKey::Tab);
    named(&mut view, NamedKey::Space);
    named(&mut view, NamedKey::Escape);
    assert!(!view.is_open());
    assert!(view.take_edit().is_none());
    assert!(
        !view
            .event(
                &WindowEvent::Ime(Ime::Commit("shell text".into())),
                ModifiersState::empty(),
                1.0
            )
            .consumed
    );
    view.open(catalog(2));
    assert_eq!(view.query(), "");
}

#[test]
fn actual_ime_and_drop_events_stay_inside_settings_and_queries_are_bounded() {
    let mut view = opened();
    assert!(
        view.event(
            &WindowEvent::Ime(Ime::Preedit("界".into(), Some((0, 3)))),
            ModifiersState::empty(),
            1.0
        )
        .consumed
    );
    assert_eq!(view.preedit, "界");
    assert_eq!(view.query(), "");
    view.event(
        &WindowEvent::Ime(Ime::Commit("table".into())),
        ModifiersState::empty(),
        1.0,
    );
    assert_eq!(view.query(), "table");
    assert!(view.preedit.is_empty());
    let before = view.query().to_owned();
    assert!(!view.paste(&"q".repeat(MAX_QUERY_BYTES + 1)));
    assert_eq!(view.query(), before);
    assert!(
        view.event(
            &WindowEvent::DroppedFile("fictional.txt".into()),
            ModifiersState::empty(),
            1.0
        )
        .consumed
    );
    assert!(view.take_edit().is_none());
    view.event(
        &WindowEvent::Ime(Ime::Preedit("draft".into(), None)),
        ModifiersState::empty(),
        1.0,
    );
    view.event(&WindowEvent::Focused(false), ModifiersState::empty(), 1.0);
    assert!(view.preedit.is_empty());
}

#[test]
fn search_and_focus_preserve_complete_values_and_unavailable_explanations() {
    let mut rows = catalog(1).entries().to_vec();
    rows[0].availability = Availability::Unavailable {
        reason: "Unavailable in this build".into(),
    };
    let mut view = opened();
    view.refresh(Catalog::new(2, rows).unwrap());
    assert!(view.paste("table"));
    named(&mut view, NamedKey::Tab);
    named(&mut view, NamedKey::Space);
    assert!(view.take_edit().is_none());
    let summary = view.accessibility_summary();
    assert!(summary.contains("Format detected tables"));
    assert!(summary.contains("Unavailable in this build"));
}

#[test]
fn save_status_ignores_old_receipts_and_reports_session_only_failures() {
    let mut view = opened();
    view.save_started(3);
    view.save_started(4);
    view.save_completed(3, false);
    assert!(view.accessibility_summary().contains("Saving"));
    view.save_completed(4, false);
    assert!(view.accessibility_summary().contains("session"));
    view.save_started(5);
    view.save_completed(5, true);
    assert!(view.accessibility_summary().contains("Saved"));
    view.save_failed();
    assert!(view.accessibility_summary().contains("session"));
}

struct Raster {
    text: Text,
    rects: Vec<([f32; 4], [f32; 4])>,
    scale: f32,
}
impl Raster {
    fn new(scale: f32) -> Self {
        let mut data = FontLibraryData::default();
        data.insert(
            FontData::from_static_slice(constants::FONT_CASCADIA_CODE_NF).unwrap(),
        );
        for _ in 0..3 {
            data.insert_alias(0);
        }
        let fonts = FontLibrary {
            inner: std::sync::Arc::new(parking_lot::RwLock::new(data)),
        };
        let mut text = Text::new(&fonts);
        text.init_cpu();
        text.set_scale_factor(scale);
        Self {
            text,
            rects: Vec::new(),
            scale,
        }
    }
    fn pixels(&mut self, width: u32, height: u32, glyphs: bool) -> Vec<u32> {
        let mut pixels = vec![0x00112233; width as usize * height as usize];
        for ([left, top, w, h], color) in &self.rects {
            let p = ((color[0] * 255.0) as u32) << 16
                | ((color[1] * 255.0) as u32) << 8
                | (color[2] * 255.0) as u32;
            for y in 0..height {
                for x in 0..width {
                    let px = (x as f32 + 0.5) / self.scale;
                    let py = (y as f32 + 0.5) / self.scale;
                    if px >= *left && px < left + w && py >= *top && py < top + h {
                        pixels[(y * width + x) as usize] = p;
                    }
                }
            }
        }
        if glyphs {
            self.text.render_cpu_base(&mut pixels, width, height);
            self.text.render_cpu_modal(&mut pixels, width, height);
        }
        pixels
    }
}
impl Canvas for Raster {
    fn text(&mut self) -> &mut Text {
        &mut self.text
    }
    fn rect(&mut self, bounds: [f32; 4], color: [f32; 4]) {
        self.rects.push((bounds, color));
    }
}
fn theme() -> UiTheme {
    UiTheme::resolve(
        [0.02, 0.04, 0.06, 1.0],
        [0.95, 0.97, 1.0, 1.0],
        [0.7, 0.75, 0.8, 1.0],
    )
}

#[test]
fn controlled_settings_raster_has_text_and_no_escape_at_fractional_or_narrow_sizes() {
    for (width, height, font, scale) in [
        (720.0, 560.0, 14.0, 1.0),
        (320.0, 300.0, 18.0, 1.25),
        (280.0, 240.0, 28.0, 2.0),
        (80.0, 100.0, 20.0, 1.25),
    ] {
        let mut view = opened();
        view.fit(width, height, font);
        let mut raster = Raster::new(scale);
        view.paint(&mut raster, theme());
        assert!(!raster.rects.is_empty());
        for ([x, y, w, h], _) in &raster.rects {
            assert!(*x >= 0.0 && *y >= 0.0 && *w >= 0.0 && *h >= 0.0);
            assert!(x + w <= width + 0.001 && y + h <= height + 0.001);
        }
        let pw = (width * scale).ceil() as u32 + 20;
        let ph = (height * scale).ceil() as u32 + 20;
        let blank = raster.pixels(pw, ph, false);
        let pixels = raster.pixels(pw, ph, true);
        if let Some(directory) = std::env::var_os("AUTOMEXIA_SETTINGS_PREVIEW_DIR") {
            let directory = std::path::PathBuf::from(directory);
            std::fs::create_dir_all(&directory)
                .expect("create controlled settings preview directory");
            image_rs::RgbImage::from_fn(pw, ph, |x, y| {
                let pixel = pixels[(y * pw + x) as usize];
                image_rs::Rgb([(pixel >> 16) as u8, (pixel >> 8) as u8, pixel as u8])
            })
            .save(
                directory
                    .join(format!("settings-{}-{}.png", width as u32, height as u32)),
            )
            .expect("write fictional controlled settings preview");
        }

        assert!(pixels != blank, "settings text must rasterize at {width}x{height}, font {font}, scale {scale}");
        for y in 0..ph {
            for x in 0..pw {
                if x as f32 >= width * scale || y as f32 >= height * scale {
                    assert_eq!(pixels[(y * pw + x) as usize], 0x00112233);
                }
            }
        }
    }
}

#[test]
fn pointer_toggle_uses_same_control_geometry_and_requires_matching_press_release() {
    let mut view = opened();
    let mut raster = Raster::new(1.25);
    view.paint(&mut raster, theme());
    let control = view.rows[0].control;
    let x = control.x + control.width * 0.5;
    let y = control.y + control.height * 0.5;
    // SAFETY: dummy IDs are used only to construct events for this pure adapter.
    let device = unsafe { DeviceId::dummy() };
    view.event(
        &WindowEvent::CursorMoved {
            device_id: device,
            position: rio_window::dpi::PhysicalPosition::new(
                (x * 1.25) as f64,
                (y * 1.25) as f64,
            ),
        },
        ModifiersState::empty(),
        1.25,
    );
    view.event(
        &WindowEvent::MouseInput {
            device_id: device,
            state: ElementState::Pressed,
            button: MouseButton::Left,
        },
        ModifiersState::empty(),
        1.25,
    );
    view.event(
        &WindowEvent::MouseInput {
            device_id: device,
            state: ElementState::Released,
            button: MouseButton::Left,
        },
        ModifiersState::empty(),
        1.25,
    );
    assert_eq!(view.take_edit().unwrap().id.as_str(), INLINE_TABLES);
}

#[test]
fn focus_scrolling_reaches_last_setting_and_preserves_row_clipping() {
    let mut view = opened();
    view.fit(300.0, 270.0, 18.0);
    named(&mut view, NamedKey::Tab);
    named(&mut view, NamedKey::End);
    let mut raster = Raster::new(1.0);
    view.paint(&mut raster, theme());
    let target = view
        .rows
        .iter()
        .find(|row| row.id.as_str() == COMMAND_TIMESTAMPS)
        .unwrap();
    let body = view.geometry.body;
    assert!(target.control.y >= body.y - 0.01);
    assert!(target.control.y + target.control.height <= body.y + body.height + 0.01);
}

#[test]
fn choices_and_numbers_have_exact_visible_current_values_and_directional_edits() {
    let mut rows = catalog(1).entries().to_vec();
    rows[0].kind = SettingKind::Choice {
        options: vec![
            automexia_ui_model::settings::ChoiceOption {
                value: "system".into(),
                label: "Follow system".into(),
            },
            automexia_ui_model::settings::ChoiceOption {
                value: "dark".into(),
                label: "Dark".into(),
            },
        ],
    };
    rows[0].value = SettingValue::Choice("system".into());
    rows[0].default = rows[0].value.clone();
    rows[1].kind = SettingKind::Number {
        min: 0.0,
        max: 1.0,
        step: 0.1,
    };
    rows[1].value = SettingValue::Number(0.3);
    rows[1].default = rows[1].value.clone();
    let mut view = opened();
    view.refresh(Catalog::new(2, rows).unwrap());
    named(&mut view, NamedKey::Tab);
    named(&mut view, NamedKey::ArrowRight);
    assert_eq!(
        view.take_edit().unwrap().change,
        Change::Set(SettingValue::Choice("dark".into()))
    );
    named(&mut view, NamedKey::ArrowDown);
    named(&mut view, NamedKey::ArrowLeft);
    let Change::Set(SettingValue::Number(value)) = view.take_edit().unwrap().change
    else {
        panic!("expected numeric intent")
    };
    assert!((value - 0.2).abs() < 0.00001);
}

#[test]
fn a_newer_coalesced_persistence_receipt_finishes_an_earlier_waiting_edit() {
    let mut view = opened();
    view.save_started(11);
    view.save_completed(12, true);
    assert!(view.accessibility_summary().contains("Saved"));
    view.save_started(13);
    view.save_completed(14, false);
    assert!(view.accessibility_summary().contains("session"));
}

#[test]
fn first_value_stays_with_its_label_before_secondary_details_in_a_narrow_sheet() {
    let mut view = opened();
    view.fit(320.0, 300.0, 18.0);
    let mut raster = Raster::new(1.0);
    view.paint(&mut raster, theme());
    let first = &view.rows[0];
    let body = view.geometry.body;
    assert!(first.control.y >= body.y);
    assert!(
        first.control.y + first.control.height <= body.y + body.height,
        "initial On/Off control must be visible before descriptive details"
    );
}

#[test]
fn pointer_left_and_right_controls_change_a_numeric_value_in_the_visible_direction() {
    let mut rows = catalog(1).entries().to_vec();
    rows[0].kind = SettingKind::Number {
        min: 0.0,
        max: 1.0,
        step: 0.1,
    };
    rows[0].value = SettingValue::Number(0.3);
    rows[0].default = rows[0].value.clone();
    let mut view = opened();
    view.refresh(Catalog::new(2, rows).unwrap());
    let mut raster = Raster::new(1.0);
    view.paint(&mut raster, theme());
    let bounds = view.rows[0].control;
    for (fraction, expected) in [(0.15, 0.2), (0.85, 0.4)] {
        let target = view
            .target_at(
                bounds.x + bounds.width * fraction,
                bounds.y + bounds.height * 0.5,
            )
            .unwrap();
        view.activate_target(target);
        let Change::Set(SettingValue::Number(value)) = view.take_edit().unwrap().change
        else {
            panic!("numeric control expected")
        };
        assert!((value - expected).abs() < 0.00001);
    }
}

#[test]
fn selecting_search_text_has_visible_pixels_and_replaces_graphemes_without_splitting_them(
) {
    let mut view = opened();
    assert!(view.paste("table"));
    let mut plain = Raster::new(1.0);
    view.paint(&mut plain, theme());
    let plain_pixels = plain.pixels(720, 560, true);
    view.key(
        &Key::Character("a".into()),
        None,
        ModifiersState::CONTROL,
        false,
    );
    let mut selected = Raster::new(1.0);
    view.paint(&mut selected, theme());
    let selected_pixels = selected.pixels(720, 560, true);
    assert!(
        plain_pixels != selected_pixels,
        "text selection must be visible"
    );
    assert!(view.paste("界é"));
    assert_eq!(view.query(), "界é");
    named(&mut view, NamedKey::Backspace);
    assert_eq!(view.query(), "界");
}

#[test]
fn ime_popup_geometry_tracks_search_and_unsafe_preedit_is_not_presented() {
    let mut view = opened();
    let mut raster = Raster::new(1.0);
    view.paint(&mut raster, theme());
    let [x, y, w, h] = view.ime_cursor_area().unwrap();
    let search = view.geometry.search;
    assert!(
        x >= search.x
            && y >= search.y
            && x + w <= search.x + search.width
            && y + h <= search.y + search.height
    );
    view.event(
        &WindowEvent::Ime(Ime::Preedit("a\u{202e}b".into(), None)),
        ModifiersState::empty(),
        1.0,
    );
    assert!(view.preedit.is_empty());
    named(&mut view, NamedKey::Tab);
    assert!(view.ime_cursor_area().is_none());
}

#[test]
fn readable_settings_controls_keep_minimum_height_or_request_more_room() {
    for (width, height, font) in [
        (720.0, 560.0, 14.0),
        (320.0, 300.0, 18.0),
        (280.0, 240.0, 28.0),
        (80.0, 100.0, 20.0),
    ] {
        let mut view = opened();
        view.fit(width, height, font);
        let mut raster = Raster::new(1.0);
        view.paint(&mut raster, theme());
        assert_eq!(view.font, font, "Chosen text size cannot silently shrink");
        if !view.requires_larger_window() {
            for bounds in [
                view.geometry.search,
                view.geometry.reset,
                view.geometry.close,
            ] {
                assert!(
                    bounds.height >= font * 1.2 + 4.0,
                    "Interactive text is clipped at {width}x{height}, font {font}: {}",
                    bounds.height
                );
            }
        }
    }
}

#[test]
fn compact_settings_explicitly_explains_resize_and_rasterizes_without_viewport_escape() {
    for (width, height, font, scale) in
        [(280.0, 240.0, 28.0, 2.0), (80.0, 100.0, 20.0, 1.25)]
    {
        let mut view = opened();
        view.fit(width, height, font);
        let mut raster = Raster::new(scale);
        view.paint(&mut raster, theme());
        assert!(
            view.requires_larger_window(),
            "A readable label and its control cannot fit"
        );
        assert!(view.accessibility_summary().contains("Enlarge the window"));
        assert!(
            view.rows.is_empty(),
            "No partially clipped interactive rows in compact state"
        );
        let width_px = (width * scale).ceil() as u32;
        let height_px = (height * scale).ceil() as u32;
        let blank = raster.pixels(width_px + 8, height_px + 8, false);
        let pixels = raster.pixels(width_px + 8, height_px + 8, true);
        assert!(
            pixels != blank,
            "Compact resize instruction must have visible glyphs"
        );
        for y in 0..height_px + 8 {
            for x in 0..width_px + 8 {
                if x >= width_px || y >= height_px {
                    assert_eq!(pixels[(y * (width_px + 8) + x) as usize], 0x00112233);
                }
            }
        }
    }
}

#[test]
fn compact_settings_blocks_invisible_edits_and_recovers_after_resize() {
    let mut view = opened();
    view.fit(280.0, 240.0, 28.0);
    named(&mut view, NamedKey::Tab);
    named(&mut view, NamedKey::Space);
    assert!(view.take_edit().is_none(), "Hidden option cannot be edited");
    assert!(
        !view.paste("table"),
        "Clipboard paste cannot edit a hidden search field"
    );
    assert_eq!(view.query(), "");
    view.event(
        &WindowEvent::Ime(Ime::Preedit("draft".into(), None)),
        ModifiersState::empty(),
        1.0,
    );
    assert!(view.preedit.is_empty());
    let mut raster = Raster::new(1.0);
    view.paint(&mut raster, theme());
    assert!(view.ime_cursor_area().is_none());
    view.fit(320.0, 300.0, 18.0);
    view.paint(&mut Raster::new(1.0), theme());
    assert!(
        !view.requires_larger_window(),
        "Ordinary narrow pane remains usable"
    );
    named(&mut view, NamedKey::Tab);
    named(&mut view, NamedKey::Space);
    assert!(view.take_edit().is_some());
    view.fit(80.0, 100.0, 20.0);
    named(&mut view, NamedKey::Escape);
    assert!(!view.is_open(), "Escape remains reachable at every size");
}

#[test]
fn measured_wrap_never_emits_a_whitespace_only_line_between_words() {
    let mut raster = Raster::new(1.0);
    let opts = DrawOpts {
        font_size: 20.0,
        ..DrawOpts::default()
    };
    let width = raster
        .text
        .measure("More", &opts)
        .max(raster.text.measure("room", &opts))
        + 0.1;
    assert!(
        raster.text.measure("More ", &opts) > width,
        "Fixture splits only trailing whitespace"
    );
    assert_eq!(
        wrapped("More room", width, &mut raster.text, &opts),
        vec!["More", "room"]
    );
    let wide = raster.text.measure("wide 界é", &opts) + 0.1;
    assert_eq!(
        wrapped("wide 界é", wide, &mut raster.text, &opts),
        vec!["wide 界é"],
        "Meaningful spaces and combining graphemes stay intact"
    );
}
