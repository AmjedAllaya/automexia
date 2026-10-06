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

#[cfg(feature = "native-gui-test-hooks")]
#[path = "settings_visual_quality_tests.rs"]
mod visual_quality_tests;

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

#[test]
fn live_terminal_backdrop_settings_paint_only_opaque_cards() {
    // Exercise actual paint dispatch, including routes that used to draw their
    // own whole-window fill. The terminal owns all pixels outside the card.
    for entry in crate::automexia::theme_gallery::builtins() {
        let theme = UiTheme::from_colors(&entry.theme.unwrap().colors);
        for (width, height, scale) in [
            (320.0, 420.0, 1.0),
            (960.0, 620.0, 1.5),
            (1920.0, 1080.0, 2.0),
        ] {
            for page in [
                "settings",
                "customizations",
                "detail",
                "color",
                "reset",
                "gallery",
            ] {
                let mut view = match page {
                    "settings" => opened(),
                    "color" => opened_color(true),
                    _ => workflow_tag_view(),
                };
                if page == "customizations" {
                    named(&mut view, NamedKey::Escape);
                } else if page == "reset" {
                    view.request_reset();
                } else if page == "gallery" {
                    let base = rio_backend::config::Config::default();
                    view.set_theme_context(ThemeContext {
                        configured: rio_backend::config::theme::Theme {
                            colors: base.colors,
                        },
                        saved: None,
                        font_colors: false,
                    });
                    view.open_theme_gallery();
                    view.theme_inventory(
                        crate::automexia::theme_gallery::builtins(),
                        false,
                    );
                    assert!(view.gallery.is_some());
                }
                view.fit(width, height, 16.0);
                let mut raster = Raster::new(scale);
                view.paint(&mut raster, theme);
                let card = if view.confirmation.is_some() {
                    view.confirmation_geometry.card
                } else if view.color_editor.is_some() {
                    view.color_geometry.card
                } else {
                    view.geometry.card
                };
                assert!(card.x > 0.0 && card.y > 0.0, "{page}: terminal margin");
                assert!(raster.rects.iter().any(|(bounds, color)|
                    *bounds == card.array() && color[3] == 1.0),
                    "{page}: opaque card must remain");
                for ([x, y, w, h], _) in raster.rects {
                    assert!(x >= card.x - 0.01 && y >= card.y - 0.01
                        && x + w <= card.x + card.width + 0.01
                        && y + h <= card.y + card.height + 0.01,
                        "{page}: paint {x},{y},{w},{h} covers the live terminal outside {card:?}");
                }
            }
        }
    }
}

#[test]
#[ignore = "same-host paint microbenchmark; run explicitly with --ignored"]
fn live_terminal_backdrop_paint_benchmark() {
    let mut reports = Vec::new();
    for (width, height) in [(960.0, 620.0), (1920.0, 1080.0)] {
        let mut view = workflow_tag_view();
        view.fit(width, height, 16.0);
        let mut raster = Raster::new(1.0);
        let mut samples = Vec::new();
        for iteration in 0..220 {
            raster.rects.clear();
            raster.shapes.clear();
            raster.text.clear();
            let start = std::time::Instant::now();
            view.paint(std::hint::black_box(&mut raster), theme());
            let elapsed = start.elapsed().as_nanos();
            std::hint::black_box(&raster.rects);
            if iteration >= 20 {
                samples.push(elapsed);
            }
        }
        samples.sort_unstable();
        reports.push(serde_json::json!({
            "viewport": [width, height], "warmup": 20, "samples": samples.len(),
            "paint_ns": {"p50": samples[99], "p95": samples[189]},
            "rectangles": raster.rects.len(),
            "scope": "production settings layout/shaping/draw emission; excludes GPU/present"
        }));
    }
    println!("{}", serde_json::json!({"live_terminal_backdrop": reports}));
}

#[test]
fn native_semantics_use_painted_settings_bounds_focus_and_modal_isolation() {
    use accesskit::Role;
    for scale in [1.0, 1.25, 1.5, 2.0] {
        let mut view = opened();
        let mut raster = Raster::new(scale);
        let theme = theme();
        view.paint(&mut raster, theme);
        let viewport =
            accesskit::Rect::new(0.0, 0.0, 720.0 * scale as f64, 560.0 * scale as f64);
        let surface = view.accessibility_surface(scale, viewport);
        let focus = surface
            .elements
            .iter()
            .find(|element| element.key == surface.focus)
            .unwrap();
        assert_eq!(focus.node.role(), Role::SearchInput);
        let bounds = focus.node.bounds().unwrap();
        assert_eq!(
            bounds.x0,
            f64::from(view.geometry.search.x) * f64::from(scale)
        );
        assert!(surface
            .elements
            .iter()
            .any(|item| item.node.role() == Role::CheckBox));
        named(&mut view, NamedKey::Tab);
        view.paint(&mut raster, theme);
        let surface = view.accessibility_surface(scale, viewport);
        assert_ne!(
            surface
                .elements
                .iter()
                .find(|element| element.key == surface.focus)
                .unwrap()
                .node
                .role(),
            Role::SearchInput
        );
        view.request_reset();
        view.paint(&mut raster, theme);
        let surface = view.accessibility_surface(scale, viewport);
        assert!(!surface
            .elements
            .iter()
            .any(|item| item.node.role() == Role::SearchInput));
        assert!(surface
            .elements
            .iter()
            .any(|item| item.node.label() == Some("Cancel")));
    }
}

#[test]
fn menu_back_walks_outward_without_reopening_or_repeating_pages() {
    for (key, modifiers) in [
        (NamedKey::ArrowLeft, ModifiersState::ALT),
        (NamedKey::Backspace, ModifiersState::empty()),
    ] {
        let base = rio_backend::config::Config::default();
        let mut view = SettingsView::default();
        view.fit(1000.0, 800.0, 16.0);
        view.open_with_section(
            crate::settings_catalog::catalog(1, &base, &Default::default(), &[]).unwrap(),
            Some(Section::Customizations),
        );
        assert!(view.paste("output"));
        named(&mut view, NamedKey::Tab);
        named(&mut view, NamedKey::Enter);
        assert_eq!(view.title(), "Terminal output colors");
        view.key(&Key::Named(key), None, modifiers, true);
        assert_eq!(
            view.title(),
            "Terminal output colors",
            "held Back stays put"
        );
        view.key(&Key::Named(key), None, modifiers, false);
        assert!(view.is_category_root());
        assert_eq!(view.query(), "output");
        view.key(&Key::Named(key), None, modifiers, true);
        assert!(view.is_open(), "a held Back must not dismiss its parent");
        view.key(&Key::Named(key), None, modifiers, false);
        assert!(!view.is_open(), "the next distinct Back leaves the root");
        assert!(view.take_edit().is_none());
    }
}

#[test]
fn menu_back_unwinds_tag_detail_preview_and_confirmation_without_edits() {
    for (key, modifiers) in [
        (NamedKey::ArrowLeft, ModifiersState::ALT),
        (NamedKey::Backspace, ModifiersState::empty()),
    ] {
        let mut view = workflow_tag_view();
        view.start_preview_edit();
        view.preview_selected =
            Some(SettingId::new("tags.slot.kubernetes.page").unwrap());
        named(&mut view, NamedKey::Enter);
        assert_eq!(view.title(), "Tag slot: Kubernetes");
        view.activate_target(Target::Reset);
        assert!(view.confirmation.is_some());
        view.key(&Key::Named(key), None, modifiers, false);
        assert!(view.confirmation.is_none());
        assert_eq!(view.title(), "Tag slot: Kubernetes");
        view.key(&Key::Named(key), None, modifiers, false);
        assert_eq!(view.title(), "Information tags");
        assert_eq!(view.focus, Focus::List);
        view.key(&Key::Named(key), None, modifiers, false);
        assert!(view.is_category_root());
        view.key(&Key::Named(key), None, modifiers, false);
        assert!(!view.is_open());
        assert!(view.take_back_to_menu());
        assert!(
            !view.take_back_to_menu(),
            "return location is consumed once"
        );
        assert!(view.take_edit().is_none());
        assert!(view.take_customization_intent().is_none());
    }
}

#[test]
fn menu_back_preserves_text_deletion_composition_and_numeric_cancel() {
    let mut view = workflow_numeric_view();
    assert!(view.paste("Number"));
    named(&mut view, NamedKey::Backspace);
    assert_eq!(view.query(), "Numbe");
    assert!(view.is_open());
    view.event(
        &WindowEvent::Ime(Ime::Preedit("字".into(), None)),
        ModifiersState::empty(),
        1.0,
    );
    view.key(
        &Key::Named(NamedKey::ArrowLeft),
        None,
        ModifiersState::ALT,
        false,
    );
    assert!(!view.preedit.is_empty());
    assert!(view.is_open());
    named(&mut view, NamedKey::Escape);
    view.focus = Focus::List;
    named(&mut view, NamedKey::Enter);
    assert!(view.numeric_editor.is_some());
    assert!(view.paste("123"));
    named(&mut view, NamedKey::Backspace);
    assert_eq!(view.numeric_editor.as_ref().unwrap().draft, "12");
    view.key(
        &Key::Named(NamedKey::ArrowLeft),
        None,
        ModifiersState::ALT,
        false,
    );
    assert!(view.numeric_editor.is_none());
    assert!(view.is_open());
    assert!(view.take_edit().is_none());
}

#[test]
fn menu_back_button_exits_preview_before_leaving_its_page() {
    let mut view = workflow_tag_view();
    view.start_preview_edit();
    view.activate_target(Target::Back);
    assert!(!view.preview_edit_mode);
    assert_eq!(view.title(), "Information tags");
    view.activate_target(Target::Back);
    assert!(view.is_category_root());
    assert!(view.take_edit().is_none());
}

#[test]
fn menu_back_color_text_owns_delete_and_alt_left_cancels_only_editor() {
    let mut view = opened_color(true);
    assert!(replace_color(&mut view, "#12345680"));
    named(&mut view, NamedKey::Backspace);
    assert!(view.color_editor.is_some());
    view.key(
        &Key::Named(NamedKey::ArrowLeft),
        None,
        ModifiersState::ALT,
        false,
    );
    assert!(view.color_editor.is_none());
    assert!(view.is_open());
    assert!(view.take_edit().is_none());
}

#[test]
fn settings_toggle_and_customizations_focus_share_one_catalog_without_cross_editing() {
    use automexia_ui_model::settings::{
        Section, COMMAND_OUTPUT_HIGHLIGHTING, OUTPUT_HIGHLIGHTING,
    };
    let base = rio_backend::config::Config::default();
    let preferences = crate::automexia::preferences::UserPreferences::default();
    let market = [crate::automexia::marketplace::MarketItem {
        id: crate::automexia::builtins::devops::ID.into(),
        name: "DevOps".into(),
        description: "Status coloring".into(),
        installed: true,
    }];
    let snapshot =
        crate::settings_catalog::catalog(1, &base, &preferences, &market).unwrap();
    let mut view = SettingsView::default();
    view.fit(720.0, 560.0, 14.0);
    view.open(snapshot.clone());
    assert!(view
        .view
        .as_ref()
        .unwrap()
        .filtered_ids()
        .iter()
        .any(|id| id.as_str() == COMMAND_OUTPUT_HIGHLIGHTING));
    named(&mut view, NamedKey::Tab);
    named(&mut view, NamedKey::ArrowDown);
    assert_eq!(
        view.view.as_ref().unwrap().focused().unwrap().as_str(),
        COMMAND_OUTPUT_HIGHLIGHTING
    );
    named(&mut view, NamedKey::Space);
    let edit = view.take_edit().unwrap();
    assert_eq!(edit.id.as_str(), COMMAND_OUTPUT_HIGHLIGHTING);
    assert_eq!(edit.change, Change::Set(SettingValue::Boolean(false)));
    let changed =
        crate::settings_catalog::apply_edit(1, &base, &preferences, &market, &edit)
            .unwrap();
    assert_eq!(
        changed.presentation.command_output_highlighting,
        Some(false)
    );

    view.open_with_section(snapshot, Some(Section::Customizations));
    assert!(view
        .accessibility_summary()
        .starts_with("Workflow & Output."));
    let categories = view.catalog.as_ref().unwrap();
    let labels: Vec<_> = categories
        .entries()
        .iter()
        .map(|entry| entry.label.as_str())
        .collect();
    assert!(labels.contains(&"Information tags"));
    assert!(labels.contains(&"Terminal output colors"));
    assert!(labels.contains(&"Kubernetes status colors"));
    assert!(labels.contains(&"Inline tables"));
    assert!(labels.contains(&"Command timestamps"));
    assert!(
        !labels.contains(&"Theme"),
        "Theme has only one home in Terminal Appearance"
    );
    assert!(!labels.contains(&"Fonts"));
    assert!(!labels.contains(&"Window controls"));
    assert!(!labels.iter().any(|label| label.contains("DevOps")));
    assert!(!labels.contains(&"Git branch tag"));
    assert!(categories
        .entries()
        .iter()
        .all(|entry| entry.kind == SettingKind::Action));
    let output_category = categories
        .entries()
        .iter()
        .find(|entry| entry.label == "Terminal output colors")
        .unwrap()
        .id
        .clone();
    view.view.as_mut().unwrap().focus(&output_category);
    view.focus = Focus::List;
    named(&mut view, NamedKey::Enter);
    assert!(
        view.take_edit().is_none(),
        "opening a category must never queue a setting edit"
    );
    let output_page = view.view.as_ref().unwrap().filtered_ids();
    assert_eq!(
        output_page.first().unwrap().as_str(),
        COMMAND_OUTPUT_HIGHLIGHTING
    );
    assert!(output_page
        .iter()
        .any(|id| id.as_str() == OUTPUT_HIGHLIGHTING));
    assert!(output_page.iter().any(|id| id.as_str() == "output.style"));
    assert!(!output_page
        .iter()
        .any(|id| id.as_str().starts_with("tags.")));
    assert_eq!(view.title(), "Terminal output colors");
    named(&mut view, NamedKey::Space);
    let detail_edit = view.take_edit().unwrap();
    assert_eq!(detail_edit.id.as_str(), COMMAND_OUTPUT_HIGHLIGHTING);
    assert_eq!(
        detail_edit.change,
        Change::Set(SettingValue::Boolean(false))
    );
    named(&mut view, NamedKey::Escape);
    assert!(
        view.is_open(),
        "Escape from a feature page returns to categories"
    );
    assert_eq!(
        view.view.as_ref().unwrap().focused(),
        Some(&output_category)
    );
    let information_tags = view
        .catalog
        .as_ref()
        .unwrap()
        .entries()
        .iter()
        .find(|entry| entry.label == "Information tags")
        .unwrap()
        .id
        .clone();
    view.view.as_mut().unwrap().focus(&information_tags);
    named(&mut view, NamedKey::Enter);
    assert!(view.view.as_ref().unwrap().filtered_ids().iter().any(|id| {
        id.as_str() == crate::automexia::settings_extensions::DEVOPS_CONTEXT_STATUS_ID
    }));
    let uninstalled = [crate::automexia::marketplace::MarketItem {
        installed: false,
        ..market[0].clone()
    }];
    view.refresh(
        crate::settings_catalog::catalog(2, &base, &preferences, &uninstalled).unwrap(),
    );
    assert_eq!(view.title(), "Information tags");
    assert!(view.take_edit().is_none());
    assert!(
        !view.view.as_ref().unwrap().filtered_ids().iter().any(|id| {
            id.as_str() == crate::automexia::settings_extensions::DEVOPS_CONTEXT_STATUS_ID
        })
    );
    named(&mut view, NamedKey::Escape);
    assert!(view.is_category_root());
    assert!(view.take_edit().is_none());
}

#[test]
fn git_preview_tag_edits_only_the_git_feature_and_repaints_its_saved_value() {
    use crate::automexia::settings_extensions::{
        DEVOPS_CONTEXT_STATUS_ID, DEVOPS_GIT_STATUS_ID,
    };

    let base = rio_backend::config::Config::default();
    let preferences = crate::automexia::preferences::UserPreferences::default();
    let market = [crate::automexia::marketplace::MarketItem {
        id: crate::automexia::builtins::devops::ID.into(),
        name: "DevOps".into(),
        description: "Local context".into(),
        installed: true,
    }];
    let full = crate::settings_catalog::catalog(1, &base, &preferences, &market).unwrap();
    let git_id = SettingId::new(DEVOPS_GIT_STATUS_ID).unwrap();
    let mut view = SettingsView::default();
    view.fit(720.0, 560.0, 14.0);
    view.open_customizations_with_slots(
        full,
        None,
        Some(crate::settings_catalog::slot_page_snapshot(
            &preferences,
            &base,
        )),
    );
    assert!(view
        .view
        .as_mut()
        .unwrap()
        .focus(&SettingId::new("tags.enabled").unwrap()));
    view.focus = Focus::List;
    named(&mut view, NamedKey::Enter);
    view.start_preview_edit();
    view.enter_preview_item(SettingId::new("tags.slot.git.page").unwrap());
    assert_eq!(view.title(), "Tag slot: Git");
    assert!(view.view.as_mut().unwrap().focus(&git_id));
    assert_eq!(
        view.catalog.as_ref().unwrap().get(&git_id).unwrap().value,
        SettingValue::Boolean(true)
    );
    named(&mut view, NamedKey::Space);
    let edit = view.take_edit().unwrap();
    assert_eq!(edit.id, git_id);
    assert_eq!(edit.change, Change::Set(SettingValue::Boolean(false)));
    let changed =
        crate::settings_catalog::apply_edit(1, &base, &preferences, &market, &edit)
            .unwrap();
    assert_eq!(
        changed.extension_feature_enabled(DEVOPS_GIT_STATUS_ID),
        Some(false)
    );
    assert_eq!(
        changed.extension_feature_enabled(DEVOPS_CONTEXT_STATUS_ID),
        None
    );
    view.refresh_with_resources(
        crate::settings_catalog::catalog(2, &base, &changed, &market).unwrap(),
        None,
        Some(crate::settings_catalog::slot_page_snapshot(
            &changed,
            &changed.apply_to(&base),
        )),
    );
    assert_eq!(
        view.catalog.as_ref().unwrap().get(&git_id).unwrap().value,
        SettingValue::Boolean(false)
    );
    assert_eq!(
        view.catalog.as_ref().unwrap().entries().len(),
        1,
        "only the re-enable switch remains while Git is off"
    );
    view.paint(&mut Raster::new(1.0), theme());
    assert!(view
        .preview_order
        .iter()
        .any(|id| id.as_str() == "tags.slot.git.page"));
    assert!(!view
        .preview_targets
        .iter()
        .any(|(id, bounds)| id.as_str() == "tags.slot.git.page"
            && bounds.y < view.preview_tag_list_area.y));
    assert!(view.view.as_mut().unwrap().focus(&git_id));
    named(&mut view, NamedKey::Space);
    let reenable = view.take_edit().unwrap();
    assert_eq!(reenable.change, Change::Set(SettingValue::Boolean(true)));
    assert!(
        crate::settings_catalog::apply_edit(2, &base, &changed, &market, &reenable)
            .unwrap()
            .extension_feature_enabled(DEVOPS_GIT_STATUS_ID)
            .unwrap()
    );
    view.refresh_with_resources(
        crate::settings_catalog::catalog(3, &base, &changed, &[]).unwrap(),
        None,
        Some(crate::settings_catalog::slot_page_snapshot_with_config(
            &changed,
            &changed.apply_to(&base),
            &base,
            &[],
        )),
    );
    assert_eq!(view.title(), "Information tags");
    assert!(view.customizations.as_ref().unwrap().active_slot.is_none());
    assert!(view.catalog.as_ref().unwrap().get(&git_id).is_none());
    assert!(view.take_edit().is_none());
}

#[test]
fn signed_package_feature_uses_bounded_nested_pages_and_uninstall_closes_stale_detail() {
    use automexia_ecosystem::{
        decode_settings_metadata, Compatibility, EcosystemManifest, ExtensionKind,
        WIT_WORLD,
    };
    use automexia_ecosystem_runtime::{
        CommittedSettingsSnapshot, InstalledPackageSettings,
    };
    let manifest = EcosystemManifest {
        schema_version: 1,
        publisher_id: "example.publisher".into(),
        extension_id: "example.inspect".into(),
        version: "1.0.0".into(),
        display_name: "Example Inspect".into(),
        description: String::new(),
        kind: ExtensionKind::Component,
        compatibility: Compatibility {
            sdk_major: 1,
            sdk_minor_minimum: 0,
            sdk_minor_maximum: 0,
        },
        world: WIT_WORLD.into(),
        imports: vec![],
        capabilities: vec![],
        action_pack_entry: None,
    };
    let metadata = decode_settings_metadata(
        include_bytes!("../../../tests/fixtures/ecosystem/settings-metadata-v1.json"),
        &manifest,
    )
    .unwrap();
    let committed = CommittedSettingsSnapshot {
        revision: 4,
        packages: vec![InstalledPackageSettings {
            extension_id: manifest.extension_id.clone(),
            publisher_id: manifest.publisher_id.clone(),
            version: manifest.version.clone(),
            package_sha256: "a".repeat(64),
            metadata: Ok(Some(metadata)),
        }],
    };
    let pages = crate::automexia::package_customizations::PackageCustomizationPages::from_committed(
        7,
        &committed,
        &[],
    )
    .unwrap();
    let base = rio_backend::config::Config::default();
    let prefs = crate::automexia::preferences::UserPreferences::default();
    let source = crate::settings_catalog::catalog(7, &base, &prefs, &[]).unwrap();
    let mut view = SettingsView::default();
    view.fit(420.0, 440.0, 14.0);
    view.open_customizations(source, Some(pages));
    let package_key = view
        .catalog
        .as_ref()
        .unwrap()
        .entries()
        .iter()
        .find(|entry| entry.label == "example.inspect")
        .unwrap()
        .id
        .clone();
    assert!(view.paste("example.inspect"));
    view.view.as_mut().unwrap().focus(&package_key);
    let refreshed = crate::automexia::package_customizations::PackageCustomizationPages::from_committed(
        8,
        &committed,
        &[],
    )
    .unwrap();
    view.refresh_with_packages(
        crate::settings_catalog::catalog(8, &base, &prefs, &[]).unwrap(),
        Some(refreshed),
    );
    assert_eq!(view.query(), "example.inspect");
    assert_eq!(view.view.as_ref().unwrap().focused(), Some(&package_key));
    view.focus = Focus::List;
    named(&mut view, NamedKey::Enter);
    assert_eq!(view.catalog.as_ref().unwrap().entries().len(), 1);
    assert_eq!(view.catalog.as_ref().unwrap().entries()[0].label, "Summary");
    assert!(view.take_edit().is_none());
    named(&mut view, NamedKey::Enter);
    assert_eq!(view.catalog.as_ref().unwrap().entries().len(), 4);
    assert!(view
        .accessibility_summary()
        .contains("execution is unavailable"));
    named(&mut view, NamedKey::Space);
    assert!(view.take_edit().is_some());
    let removed = CommittedSettingsSnapshot {
        revision: 5,
        packages: Vec::new(),
    };
    let pages = crate::automexia::package_customizations::PackageCustomizationPages::from_committed(
        9,
        &removed,
        &[],
    )
    .unwrap();
    let source = crate::settings_catalog::catalog(9, &base, &prefs, &[]).unwrap();
    view.refresh_with_packages(source, Some(pages));
    assert!(view.is_category_root());
    assert!(view.take_edit().is_none());
    assert!(!view
        .catalog
        .as_ref()
        .unwrap()
        .entries()
        .iter()
        .any(|entry| entry.label == "example.inspect"));
}

#[test]
fn maximum_installed_package_pages_leave_core_customizations_available() {
    use automexia_ui_model::settings::{ChangeScope, SettingOwner};
    let base = rio_backend::config::Config::default();
    let preferences = crate::automexia::preferences::UserPreferences::default();
    let source = crate::settings_catalog::catalog(1, &base, &preferences, &[]).unwrap();
    let pages = crate::automexia::package_customizations::PackageCustomizationPages {
        revision: 1,
        store_revision: 1,
        packages: (0..128)
            .map(|index| {
                let alias = format!("pkg_{index}");
                let key = SettingId::new(format!("extension.{alias}.package")).unwrap();
                crate::automexia::package_customizations::PackagePage {
                    publisher_id: "example.publisher".into(),
                    extension_id: format!("example.extension-{index}"),
                    key: key.clone(),
                    action: SettingDescriptor {
                        id: key,
                        owner: SettingOwner::Extension(alias),
                        section: Section::Customizations,
                        label: format!("example.package{index}"),
                        description: "Saved preferences only".into(),
                        keywords: Vec::new(),
                        kind: SettingKind::Action,
                        value: SettingValue::Action,
                        default: SettingValue::Action,
                        origin: ValueOrigin::Extension,
                        availability: Availability::Available,
                        scope: ChangeScope::Immediate,
                    },
                    features: Vec::new(),
                }
            })
            .collect(),
    };
    let mut view = SettingsView::default();
    view.open_customizations(source, Some(pages));
    let root = view.catalog.as_ref().unwrap();
    assert_eq!(
        root.entries()
            .iter()
            .filter(|row| row.label.starts_with("example.package"))
            .count(),
        128
    );
    assert!(root
        .entries()
        .iter()
        .any(|row| row.label == "Information tags"));
    assert!(root
        .entries()
        .iter()
        .any(|row| row.label == "Terminal output colors"));
    assert_ne!(view.status, "Customizations are temporarily unavailable.");
}

#[test]
fn customization_search_is_scoped_to_each_level_and_back_restores_category_focus() {
    let base = rio_backend::config::Config::default();
    let mut view = SettingsView::default();
    view.fit(480.0, 420.0, 16.0);
    view.open_with_section(
        crate::settings_catalog::catalog(1, &base, &Default::default(), &[]).unwrap(),
        Some(Section::Customizations),
    );
    assert!(view.paste("output"));
    assert_eq!(view.view.as_ref().unwrap().filtered_ids().len(), 1);
    named(&mut view, NamedKey::Tab);
    named(&mut view, NamedKey::Enter);
    assert!(view.is_category_detail());
    assert_eq!(view.query(), "");
    view.focus = Focus::Search;
    assert!(view.paste("logs"));
    assert!(view
        .view
        .as_ref()
        .unwrap()
        .filtered_ids()
        .iter()
        .all(|id| id.as_str() == automexia_ui_model::settings::OUTPUT_HIGHLIGHTING));
    view.key(
        &Key::Named(NamedKey::ArrowLeft),
        None,
        ModifiersState::ALT,
        false,
    );
    assert!(view.is_category_root());
    assert_eq!(view.query(), "output");
    assert_eq!(view.view.as_ref().unwrap().filtered_ids().len(), 1);
    assert!(view.accessibility_summary().contains("Enter: open"));
    named(&mut view, NamedKey::Escape);
    assert!(!view.is_open());
}

#[test]
fn customization_root_search_finds_categories_by_their_controls() {
    let base = rio_backend::config::Config::default();
    let mut view = SettingsView::default();
    view.fit(480.0, 420.0, 16.0);
    view.open_with_section(
        crate::settings_catalog::catalog(1, &base, &Default::default(), &[]).unwrap(),
        Some(Section::Customizations),
    );
    for (query, expected) in crate::automexia::presentation::TAG_COLOR_BINDINGS
        .iter()
        .map(|binding| (binding.label, "Information tags"))
        .chain([
            ("gcp", "Information tags"),
            ("unknown cloud", "Information tags"),
            ("opacity", "Information tags"),
            ("Docker", "Terminal output colors"),
        ])
    {
        view.key(
            &Key::Character("a".into()),
            None,
            ModifiersState::CONTROL,
            false,
        );
        assert!(view.paste(query));
        let visible = view.view.as_ref().unwrap().filtered_ids();
        assert!(
            visible.iter().any(|id| view
                .catalog
                .as_ref()
                .unwrap()
                .get(id)
                .is_some_and(|entry| entry.label == expected)),
            "query {query} must still find {expected}"
        );
    }
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
    confirm_requested_settings_action(&mut view);
    assert_eq!(view.take_edit().unwrap().change, Change::Reset);
}

#[test]
fn customization_root_reset_does_not_open_the_focused_category() {
    let base = rio_backend::config::Config::default();
    let preferences = crate::automexia::preferences::UserPreferences::default();
    let mut view = SettingsView::default();
    view.fit(720.0, 560.0, 14.0);
    view.open_customizations(
        crate::settings_catalog::catalog(1, &base, &preferences, &[]).unwrap(),
        None,
    );
    assert!(view.is_category_root());
    view.focus = Focus::Reset;
    named(&mut view, NamedKey::Enter);
    assert!(
        view.is_category_root(),
        "Reset all must not navigate into a category"
    );
    confirm_requested_settings_action(&mut view);
    assert_eq!(
        view.take_customization_intent(),
        Some(CustomizationIntent::Reset {
            revision: 1,
            scope: CustomizationResetScope::Area(CustomizationArea::Workflow),
        })
    );
}

#[test]
fn customization_reset_targets_current_category_and_restore_is_a_separate_action() {
    let base = rio_backend::config::Config::default();
    let preferences = crate::automexia::preferences::UserPreferences::default();
    let mut view = SettingsView::default();
    view.fit(720.0, 560.0, 14.0);
    view.open_customizations(
        crate::settings_catalog::catalog(1, &base, &preferences, &[]).unwrap(),
        None,
    );
    let output =
        SettingId::new(automexia_ui_model::settings::COMMAND_OUTPUT_HIGHLIGHTING)
            .unwrap();
    assert!(view.view.as_mut().unwrap().focus(&output));
    view.focus = Focus::List;
    named(&mut view, NamedKey::Enter);
    assert!(view.is_category_detail());
    view.focus = Focus::Reset;
    named(&mut view, NamedKey::Enter);
    confirm_requested_settings_action(&mut view);
    assert_eq!(
        view.take_customization_intent(),
        Some(CustomizationIntent::Reset {
            revision: 1,
            scope: CustomizationResetScope::Group(output),
        })
    );
    assert!(
        view.take_edit().is_none(),
        "category reset must not reset only a focused row"
    );
    view.set_temporary_customizations(true);
    view.focus = Focus::Restore;
    named(&mut view, NamedKey::Enter);
    confirm_requested_settings_action(&mut view);
    assert_eq!(
        view.take_customization_intent(),
        Some(CustomizationIntent::RestoreSaved)
    );
    view.activate_target(Target::Restore);
    confirm_requested_settings_action(&mut view);
    assert_eq!(
        view.take_customization_intent(),
        Some(CustomizationIntent::RestoreSaved)
    );
    view.set_temporary_customizations(false);
    assert_eq!(view.focus, Focus::Reset);
    named(&mut view, NamedKey::Tab);
    assert_eq!(view.focus, Focus::Close);
}

#[test]
fn reset_restore_and_close_have_separate_mouse_targets_in_narrow_and_wide_sheets() {
    let base = rio_backend::config::Config::default();
    let preferences = crate::automexia::preferences::UserPreferences::default();
    for (width, height) in [(320.0, 420.0), (960.0, 620.0)] {
        let mut view = SettingsView::default();
        view.fit(width, height, 16.0);
        view.open_customizations(
            crate::settings_catalog::catalog(1, &base, &preferences, &[]).unwrap(),
            None,
        );
        view.set_temporary_customizations(true);
        view.paint(&mut Raster::new(1.0), theme());
        let g = view.geometry;
        assert!(g.reset.x + g.reset.width <= g.restore.x);
        assert!(g.restore.x + g.restore.width <= g.close.x);
        assert!(g.close.x + g.close.width <= g.card.x + g.card.width);
        for (rect, target) in [
            (g.reset, Target::Reset),
            (g.restore, Target::Restore),
            (g.close, Target::Close),
        ] {
            assert_eq!(
                view.target_at(rect.x + rect.width * 0.5, rect.y + rect.height * 0.5),
                Some(target),
            );
        }
        view.set_temporary_customizations(false);
        assert_eq!(
            view.target_at(
                g.restore.x + g.restore.width * 0.5,
                g.restore.y + g.restore.height * 0.5
            ),
            None,
        );
    }
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

#[test]
fn earlier_save_receipts_never_label_a_temporary_preview_as_saved() {
    let mut view = opened();
    view.save_started(8);
    view.set_temporary_customizations(true);
    view.set_status("Temporary preview only. Restore saved to return.");
    view.save_completed(8, true);
    assert!(view.saving.is_none());
    assert!(!view.accessibility_summary().contains(". Saved"));
    assert!(view.accessibility_summary().contains("Temporary preview"));
    view.save_started(9);
    view.save_completed(9, false);
    assert!(view.accessibility_summary().contains("Temporary preview"));
    view.save_failed();
    assert!(view.accessibility_summary().contains("Temporary preview"));
    view.set_temporary_customizations(false);
    view.set_status("Previous choices restored. Saved files were unchanged.");
    view.save_completed(8, true);
    assert!(view
        .accessibility_summary()
        .contains("Previous choices restored"));
    assert_ne!(view.status, "Saved");
    view.set_temporary_customizations(true);
    view.layout_dirty = false;
    view.set_temporary_customizations(true);
    assert!(
        !view.layout_dirty,
        "unchanged preview state must not trigger layout work"
    );

    let mut delayed = opened();
    delayed.save_started(10);
    delayed.set_temporary_customizations(true);
    assert!(
        delayed.saving.is_none(),
        "only the view receipt token is retired"
    );
    delayed.set_temporary_customizations(false);
    delayed.set_status("Previous choices restored. Saved files were unchanged.");
    delayed.save_completed(10, true);
    assert_ne!(delayed.status, "Saved");
}

#[derive(Clone, Debug, PartialEq)]
enum ShapeCall {
    Rounded([f32; 4], f32),
    Polygon(Vec<(f32, f32)>),
    Line((f32, f32), (f32, f32), f32),
    Arc((f32, f32), f32, (f32, f32), f32),
}

struct Raster {
    text: Text,
    rects: Vec<([f32; 4], [f32; 4])>,
    shapes: Vec<ShapeCall>,
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
            shapes: Vec::new(),
            scale,
        }
    }
    fn pixels(&mut self, width: u32, height: u32, glyphs: bool) -> Vec<u32> {
        let mut pixels = vec![0x00112233; width as usize * height as usize];
        for ([left, top, w, h], color) in &self.rects {
            let alpha = color[3].clamp(0.0, 1.0);
            let source = [color[0], color[1], color[2]]
                .map(|channel| channel.clamp(0.0, 1.0) * 255.0);
            for y in 0..height {
                for x in 0..width {
                    let px = (x as f32 + 0.5) / self.scale;
                    let py = (y as f32 + 0.5) / self.scale;
                    if px >= *left && px < left + w && py >= *top && py < top + h {
                        let index = (y * width + x) as usize;
                        let prior = pixels[index];
                        let red = (source[0] * alpha
                            + ((prior >> 16) & 0xff) as f32 * (1.0 - alpha))
                            .round() as u32;
                        let green = (source[1] * alpha
                            + ((prior >> 8) & 0xff) as f32 * (1.0 - alpha))
                            .round() as u32;
                        let blue = (source[2] * alpha
                            + (prior & 0xff) as f32 * (1.0 - alpha))
                            .round() as u32;
                        pixels[index] = (red << 16) | (green << 8) | blue;
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
    fn rounded_rect(&mut self, bounds: [f32; 4], radius: f32, color: [f32; 4]) {
        self.shapes.push(ShapeCall::Rounded(bounds, radius));
        self.rect(bounds, color);
    }
    fn polygon(&mut self, points: &[(f32, f32)], color: [f32; 4]) {
        self.shapes.push(ShapeCall::Polygon(points.to_vec()));
        let left = points
            .iter()
            .map(|point| point.0)
            .fold(f32::INFINITY, f32::min);
        let right = points
            .iter()
            .map(|point| point.0)
            .fold(f32::NEG_INFINITY, f32::max);
        let top = points
            .iter()
            .map(|point| point.1)
            .fold(f32::INFINITY, f32::min);
        let bottom = points
            .iter()
            .map(|point| point.1)
            .fold(f32::NEG_INFINITY, f32::max);
        self.rect([left, top, right - left, bottom - top], color);
    }
    fn line(&mut self, from: (f32, f32), to: (f32, f32), width: f32, color: [f32; 4]) {
        self.shapes.push(ShapeCall::Line(from, to, width));
        self.rect(
            [
                from.0.min(to.0) - width * 0.5,
                from.1.min(to.1) - width * 0.5,
                (from.0 - to.0).abs() + width,
                (from.1 - to.1).abs() + width,
            ],
            color,
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
        self.shapes
            .push(ShapeCall::Arc(center, radius, angles, width));
        self.rect(
            [
                center.0 - radius,
                center.1 - radius,
                radius * 2.0 + width,
                radius * 2.0 + width,
            ],
            color,
        );
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
fn gallery_current_highlight_stays_distinct_from_keyboard_preview_across_palettes() {
    use crate::automexia::theme_gallery;
    let library = theme_gallery::builtins();
    for entry in &library {
        let palette = entry.theme.as_ref().unwrap();
        let theme = UiTheme::from_colors(&palette.colors);
        for (width, height, font, scale) in [
            (500.0, 900.0, 16.0, 1.0),
            (1280.0, 900.0, 20.0, 1.25),
            (1600.0, 1000.0, 24.0, 2.0),
        ] {
            let mut view = opened();
            view.fit(width, height, font);
            view.set_theme_context(ThemeContext {
                configured: palette.clone(),
                saved: library[0].selection(),
                font_colors: false,
            });
            view.open_theme_gallery();
            view.theme_inventory(library.clone(), false);
            view.take_theme_intent();
            named(&mut view, NamedKey::ArrowDown);
            let mut raster = Raster::new(scale);
            view.paint(&mut raster, theme);
            let surface = view.accessibility_surface(
                scale,
                accesskit::Rect::new(
                    0.0,
                    0.0,
                    (width * scale) as f64,
                    (height * scale) as f64,
                ),
            );
            let current: Vec<_> = surface
                .elements
                .iter()
                .filter(|e| {
                    e.node
                        .label()
                        .is_some_and(|label| label.contains("Current theme"))
                })
                .collect();
            assert_eq!(current.len(), 1, "one explicit applied theme");
            assert_eq!(
                current[0].node.author_id(),
                Some("theme:builtin:aurora-night")
            );
            assert_eq!(current[0].node.is_selected(), Some(false));
            let preview = surface
                .elements
                .iter()
                .find(|e| e.node.author_id() == Some("theme:builtin:solar-dusk"))
                .unwrap();
            assert_eq!(preview.node.is_selected(), Some(true));
            let logical = |node: &accesskit::Node| {
                let bounds = node.bounds().unwrap();
                Rect {
                    x: bounds.x0 as f32 / scale,
                    y: bounds.y0 as f32 / scale,
                    width: bounds.width() as f32 / scale,
                    height: bounds.height() as f32 / scale,
                }
            };
            let applied = logical(&current[0].node);
            let preview = logical(&preview.node);
            let fill_at = |card: Rect, inset: f32| {
                raster
                    .rects
                    .iter()
                    .find(|(r, _)| {
                        (r[0] - card.x - inset).abs() < 0.01
                            && (r[1] - card.y - inset).abs() < 0.01
                            && (r[2] - card.width + inset * 2.0).abs() < 0.01
                            && (r[3] - card.height + inset * 2.0).abs() < 0.01
                    })
                    .unwrap()
                    .1
            };
            let fill = fill_at(applied, 1.0);
            assert_ne!(
                fill, theme.background,
                "applied row keeps a full-card highlight without focus"
            );
            assert_eq!(fill[3], 1.0);
            assert_eq!(
                fill_at(preview, 2.0),
                theme.background,
                "preview has a focus outline, not the applied fill"
            );
            let markers: Vec<_> = raster
                .rects
                .iter()
                .filter(|(r, c)| {
                    r[0] >= applied.x
                        && r[1] >= applied.y
                        && r[0] + r[2] <= applied.x + applied.width + 0.01
                        && r[1] + r[3] <= applied.y + applied.height + 0.01
                        && automexia_ui_model::contrast_ratio(*c, fill) >= 3.0
                })
                .collect();
            assert!(
                markers.iter().any(|(r, _)| (r[2] - 3.0).abs() < 0.01),
                "persistent edge marker"
            );
            assert!(
                markers
                    .iter()
                    .any(|(r, _)| r[2] >= font * 8.0 && r[3] < font * 1.5),
                "current badge fits within the card"
            );
            if scale == 1.0 {
                if let Some(directory) =
                    std::env::var_os("AUTOMEXIA_SETTINGS_PREVIEW_DIR")
                {
                    let pixels = raster.pixels(width as u32, height as u32, true);
                    let directory = std::path::PathBuf::from(directory);
                    std::fs::create_dir_all(&directory).unwrap();
                    image_rs::RgbImage::from_fn(width as u32, height as u32, |x, y| {
                        let pixel = pixels[(y * width as u32 + x) as usize];
                        image_rs::Rgb([
                            (pixel >> 16) as u8,
                            (pixel >> 8) as u8,
                            pixel as u8,
                        ])
                    })
                    .save(directory.join(format!(
                        "gallery-current-{}.png",
                        entry.id.replace(':', "-")
                    )))
                    .unwrap();
                }
            }
        }
    }
}

#[test]
fn gallery_inventory_change_cancels_pointer_press_on_reindexed_rows() {
    let mut view = opened();
    view.fit(1280.0, 900.0, 16.0);
    let library = crate::automexia::theme_gallery::builtins();
    view.set_theme_context(ThemeContext {
        configured: library[0].theme.clone().unwrap(),
        saved: library[2].selection(),
        font_colors: false,
    });
    view.open_theme_gallery();
    let mut raster = Raster::new(1.0);
    view.paint(&mut raster, theme());
    let surface =
        view.accessibility_surface(1.0, accesskit::Rect::new(0.0, 0.0, 1280.0, 900.0));
    let bounds = surface
        .elements
        .iter()
        .find(|e| e.node.author_id() == Some("theme:saved"))
        .unwrap()
        .node
        .bounds()
        .unwrap();
    let row = Rect {
        x: bounds.x0 as f32,
        y: bounds.y0 as f32,
        width: bounds.width() as f32,
        height: bounds.height() as f32,
    };
    pointer_event(&mut view, row, 1.0, ElementState::Pressed);
    assert!(view.pressed.is_some());
    view.theme_inventory(library, false);
    assert!(view.pressed.is_none());
    view.take_theme_intent();
    view.paint(&mut Raster::new(1.0), theme());
    pointer_event(&mut view, row, 1.0, ElementState::Released);
    assert!(
        view.take_theme_intent().is_none(),
        "old saved row must not activate a different theme"
    );
}

#[test]
fn gallery_preview_sample_glyphs_do_not_overlap_caption() {
    let library = crate::automexia::theme_gallery::builtins();
    let palette = library[1].theme.clone().unwrap();
    let ui = UiTheme::from_colors(&library[0].theme.as_ref().unwrap().colors);
    for (width, height, font, scale) in [
        (500.0, 900.0, 16.0, 1.0),
        (640.0, 900.0, 20.0, 1.25),
        (1600.0, 1000.0, 24.0, 2.0),
    ] {
        let mut view = opened();
        view.fit(width, height, font);
        view.set_theme_context(ThemeContext {
            configured: palette.clone(),
            saved: None,
            font_colors: false,
        });
        view.open_theme_gallery();
        view.theme_inventory(library.clone(), false);
        let mut raster = Raster::new(scale);
        view.paint(&mut raster, ui);
        let (p, _) = raster
            .rects
            .iter()
            .find(|(r, c)| *c == palette.colors.background.0 && r[3] > 100.0)
            .unwrap();
        let in_preview = |glyph: &&rio_backend::sugarloaf::text::TextInstance| {
            glyph.pos[0] >= p[0] * scale
                && glyph.pos[0] < (p[0] + p[2]) * scale
                && glyph.pos[1] >= p[1] * scale
                && glyph.pos[1] < (p[1] + p[3]) * scale
        };
        let glyphs: Vec<_> = raster.text.instances().iter().filter(in_preview).collect();
        let caption_top = glyphs
            .iter()
            .filter(|g| g.color == color_u8(ui.muted_text))
            .map(|g| g.pos[1] + g.bearings[1] as f32)
            .reduce(f32::min)
            .unwrap();
        let sample: Vec<_> = glyphs
            .iter()
            .filter(|g| g.color != color_u8(ui.muted_text))
            .collect();
        assert!(!sample.is_empty());
        assert!(
            sample
                .iter()
                .all(|g| g.pos[1] + g.bearings[1] as f32 + g.glyph_size[1] as f32
                    <= caption_top),
            "sample glyphs must end before the preview caption"
        );
    }
}

#[test]
fn tag_preview_dispatches_each_shared_shape_geometry_in_filled_and_plain_modes() {
    let bounds = Rect {
        x: 23.0,
        y: 41.0,
        width: 92.0,
        height: 20.0,
    };
    let styles = BarVisualStyle::ALL;
    for style in styles {
        let expected = tag_surface_geometry(style, bounds.width, bounds.height).unwrap();
        for alpha in [0, 75] {
            let colors = automexia_ui_model::context_tag_colors(
                [0.02, 0.04, 0.06, 1.0],
                [60, 190, 240],
                alpha,
            );
            let mut raster = Raster::new(1.0);
            paint_sample_tag_surface(&mut raster, bounds, style, colors);
            match (expected, alpha) {
                (TagSurfaceGeometry::Connected(shape), 0) => {
                    assert_eq!(raster.shapes.len(), shape.points().len());
                    assert!(raster
                        .shapes
                        .iter()
                        .all(|call| matches!(call, ShapeCall::Line(..))));
                }
                (TagSurfaceGeometry::Connected(shape), _) => {
                    assert_eq!(
                        raster.shapes.len(),
                        shape.triangles().count() + usize::from(shape.fold.is_some())
                    );
                    assert!(raster.shapes.iter().all(|call| matches!(call, ShapeCall::Polygon(points) if points.len() == 3)));
                }
                (TagSurfaceGeometry::Capsule { radius, .. }, 0) => {
                    assert_eq!(raster.shapes.iter().filter(|shape| matches!(shape, ShapeCall::Arc(_, actual, _, _) if (*actual - radius).abs() < 0.001)).count(), 4);
                }
                (TagSurfaceGeometry::Capsule { radius, .. }, _) => assert!(raster
                    .shapes
                    .contains(&ShapeCall::Rounded(bounds.array(), radius))),
                (TagSurfaceGeometry::Flat { .. }, 0) => assert_eq!(
                    raster
                        .shapes
                        .iter()
                        .filter(|shape| matches!(shape, ShapeCall::Line(..)))
                        .count(),
                    4
                ),
                (TagSurfaceGeometry::Flat { .. }, _) => {
                    assert!(raster.rects.iter().any(|(rect, _)| *rect == bounds.array()))
                }
                (
                    TagSurfaceGeometry::Chevron { points, .. }
                    | TagSurfaceGeometry::Hexagon { points, .. },
                    0,
                ) => {
                    let start = (bounds.x + points[0].0, bounds.y + points[0].1);
                    let next = (bounds.x + points[1].0, bounds.y + points[1].1);
                    assert!(raster.shapes.iter().any(|shape| matches!(shape,
                        ShapeCall::Line(a, b, _) if *a == start && *b == next)));
                    assert_eq!(raster.shapes.len(), 6);
                }
                (
                    TagSurfaceGeometry::Chevron { points, .. }
                    | TagSurfaceGeometry::Hexagon { points, .. },
                    _,
                ) => {
                    let absolute = points.map(|(x, y)| (bounds.x + x, bounds.y + y));
                    assert_eq!(
                        raster.shapes,
                        vec![ShapeCall::Polygon(absolute.to_vec())]
                    );
                }
                (TagSurfaceGeometry::Card { radius, .. }, 0) => {
                    assert_eq!(raster.shapes.iter().filter(|shape| matches!(shape, ShapeCall::Arc(_, actual, _, _) if (*actual - radius).abs() < 0.001)).count(), 4);
                }
                (TagSurfaceGeometry::Card { radius, .. }, _) => {
                    assert!(raster
                        .shapes
                        .contains(&ShapeCall::Rounded(bounds.array(), radius)));
                    assert!(raster
                        .shapes
                        .iter()
                        .any(|shape| matches!(shape, ShapeCall::Arc(..))));
                }
                (TagSurfaceGeometry::Underline { .. }, _) => assert_eq!(
                    raster
                        .shapes
                        .iter()
                        .filter(|shape| matches!(shape, ShapeCall::Line(..)))
                        .count(),
                    1
                ),
            }
        }
    }
}

#[test]
fn joined_preview_tips_sockets_and_gaps_select_only_the_painted_tag() {
    let base = rio_backend::config::Config::default();
    for style in BarVisualStyle::ALL
        .into_iter()
        .filter(|style| !style.is_legacy())
    {
        let initial = crate::automexia::preferences::UserPreferences::default();
        let selected = crate::settings_catalog::apply_edit(
            1,
            &base,
            &initial,
            &[],
            &Edit {
                revision: 1,
                id: SettingId::new("tags.bar-style").unwrap(),
                change: Change::Set(SettingValue::Choice(style.id().into())),
            },
        )
        .unwrap();
        for gap in [0.0, 300.0] {
            let saved = crate::settings_catalog::apply_edit(
                1,
                &base,
                &selected,
                &[],
                &Edit {
                    revision: 1,
                    id: SettingId::new("tags.spacing").unwrap(),
                    change: Change::Set(SettingValue::Number(gap)),
                },
            )
            .unwrap();
            let mut view = SettingsView::default();
            view.fit(960.0, 620.0, 16.0);
            view.open_customizations_with_slots(
                crate::settings_catalog::catalog(1, &base, &saved, &[]).unwrap(),
                None,
                Some(crate::settings_catalog::slot_page_snapshot(&saved, &base)),
            );
            view.view
                .as_mut()
                .unwrap()
                .focus(&SettingId::new("tags.enabled").unwrap());
            view.focus = Focus::List;
            named(&mut view, NamedKey::Enter);
            view.paint(&mut Raster::new(1.0), theme());
            assert!(view.preview_tag_shapes.len() >= 3);
            let mut checked = 0;
            // Scan the real overlapping preview bounds, including concave sockets
            // and empty corners. A rectangular hit test fails on these points.
            for (bounds, shape) in &view.preview_tag_shapes {
                for y in [1.25, bounds.height * 0.5, bounds.height - 1.25] {
                    for x in (0..bounds.width.ceil() as usize).map(|x| x as f32 + 0.25) {
                        let point = (bounds.x + x, bounds.y + y);
                        let owners: Vec<_> = view
                            .preview_tag_shapes
                            .iter()
                            .filter(|(area, polygon)| {
                                area.contains(point.0, point.1)
                                    && polygon
                                        .contains((point.0 - area.x, point.1 - area.y))
                            })
                            .collect();
                        assert!(
                            owners.len() <= 1,
                            "{style:?} overlaps filled tag surfaces"
                        );
                        let expected = owners
                            .first()
                            .and_then(|(area, _)| {
                                view.preview_targets
                                    .iter()
                                    .find(|(_, target)| target == area)
                            })
                            .map(|(id, _)| id);
                        match (view.target_at(point.0, point.1), expected) {
                            (Some(Target::PreviewItem(actual)), Some(expected)) => assert_eq!(&actual, expected),
                            (None, None) => (),
                            (actual, expected) => panic!("{style:?} incorrect hit at {point:?}: {actual:?}, {expected:?}"),
                        }
                        if !shape.contains((x, y)) {
                            checked += 1;
                        }
                    }
                }
            }
            assert!(checked > 0, "must exercise cutout points");
        }
    }
}

#[test]
fn live_preview_lists_all_tags_and_selects_disabled_tags() {
    let base = rio_backend::config::Config::default();
    let mut preferences = crate::automexia::preferences::UserPreferences::default();
    preferences = crate::settings_catalog::apply_edit(
        1,
        &base,
        &preferences,
        &[],
        &Edit {
            revision: 1,
            id: SettingId::new("tags.slot.windows.enabled").unwrap(),
            change: Change::Set(SettingValue::Boolean(false)),
        },
    )
    .unwrap();
    for (width, height) in [(960.0, 620.0), (320.0, 420.0)] {
        let mut view = SettingsView::default();
        view.fit(width, height, 16.0);
        view.open_customizations_with_slots(
            crate::settings_catalog::catalog(1, &base, &preferences, &[]).unwrap(),
            None,
            Some(crate::settings_catalog::slot_page_snapshot(
                &preferences,
                &preferences.apply_to(&base),
            )),
        );
        assert!(view
            .view
            .as_mut()
            .unwrap()
            .focus(&SettingId::new("tags.enabled").unwrap()));
        view.focus = Focus::List;
        named(&mut view, NamedKey::Enter);
        assert!(view
            .catalog
            .as_ref()
            .unwrap()
            .entries()
            .iter()
            .all(|entry| !entry.id.as_str().starts_with("tags.slot.")
                && !entry.id.as_str().starts_with("tags.colors.")));
        let mut raster = Raster::new(1.0);
        view.paint(&mut raster, theme());
        assert!(view
            .preview_order
            .iter()
            .any(|id| id.as_str() == "tags.slot.windows.page"));
        assert!(!view
            .preview_targets
            .iter()
            .any(|(id, _)| id.as_str() == "tags.colors.gcp"));
        view.focus = Focus::PreviewButton;
        named(&mut view, NamedKey::Enter);
        let mut raster = Raster::new(1.0);
        view.paint(&mut raster, theme());
        assert!(view.preview_tag_list_area.height > 0.0);
        assert!(view
            .preview_order
            .iter()
            .any(|id| id.as_str() == "tags.slot.gcp.page"));
        assert!(view
            .preview_order
            .iter()
            .any(|id| id.as_str() == "tags.slot.terraform.page"));
        let windows_id = SettingId::new("tags.slot.windows.page").unwrap();
        for _ in 0..view.preview_order.len() {
            if view.preview_selected.as_ref() == Some(&windows_id) {
                break;
            }
            view.focus = Focus::Preview;
            named(&mut view, NamedKey::ArrowDown);
        }
        assert_eq!(view.preview_selected.as_ref(), Some(&windows_id));
        let mut raster = Raster::new(1.0);
        view.paint(&mut raster, theme());
        if let Some(directory) = std::env::var_os("AUTOMEXIA_SETTINGS_PREVIEW_DIR") {
            let directory = std::path::PathBuf::from(directory);
            std::fs::create_dir_all(&directory).unwrap();
            let pixels = raster.pixels(width as u32, height as u32, true);
            image_rs::RgbImage::from_fn(width as u32, height as u32, |x, y| {
                let pixel = pixels[(y * width as u32 + x) as usize];
                image_rs::Rgb([(pixel >> 16) as u8, (pixel >> 8) as u8, pixel as u8])
            })
            .save(
                directory
                    .join(format!("tag-edit-{}x{}.png", width as u32, height as u32)),
            )
            .unwrap();
        }
        let hidden_row = view
            .preview_targets
            .iter()
            .find(|(id, _)| id.as_str() == "tags.slot.windows.page")
            .map(|(_, bounds)| *bounds)
            .expect("the hidden tag has a muted selectable chip");
        assert!(
            hidden_row.height >= view.font.max(10.0) * 1.1,
            "the narrow hidden chip remains tall enough to select"
        );
        pointer_event(&mut view, hidden_row, 1.0, ElementState::Pressed);
        pointer_event(&mut view, hidden_row, 1.0, ElementState::Released);
        assert_eq!(view.title(), "Tag slot: Windows");
        assert!(view
            .catalog
            .as_ref()
            .unwrap()
            .get(&SettingId::new("tags.colors.windows").unwrap())
            .is_none());
        assert!(view
            .view
            .as_mut()
            .unwrap()
            .focus(&SettingId::new("tags.slot.windows.enabled").unwrap()));
        named(&mut view, NamedKey::Enter);
        let enabled = crate::settings_catalog::apply_edit(
            1,
            &base,
            &preferences,
            &[],
            &view.take_edit().unwrap(),
        )
        .unwrap();
        view.refresh_with_resources(
            crate::settings_catalog::catalog(2, &base, &enabled, &[]).unwrap(),
            None,
            Some(crate::settings_catalog::slot_page_snapshot(
                &enabled,
                &enabled.apply_to(&base),
            )),
        );
        assert!(view
            .catalog
            .as_ref()
            .unwrap()
            .get(&SettingId::new("tags.colors.windows").unwrap())
            .is_some());
        assert!(view
            .catalog
            .as_ref()
            .unwrap()
            .entries()
            .iter()
            .any(|entry| entry.label == "Default role color"));
        named(&mut view, NamedKey::Escape);
        assert_eq!(view.focus, Focus::List);
        assert!(
            view.preview_selected.as_ref().unwrap().as_str() == "tags.slot.windows.page"
        );
    }

    let mut view = SettingsView::default();
    view.fit(320.0, 420.0, 16.0);
    view.open_with_section(
        crate::settings_catalog::catalog(1, &base, &Default::default(), &[]).unwrap(),
        Some(Section::Customizations),
    );
    assert!(view.view.as_mut().unwrap().focus(
        &SettingId::new(automexia_ui_model::settings::COMMAND_OUTPUT_HIGHLIGHTING)
            .unwrap()
    ));
    view.focus = Focus::List;
    named(&mut view, NamedKey::Enter);
    assert_eq!(view.catalog.as_ref().unwrap().entries().len(), 4);
    let mut raster = Raster::new(1.0);
    view.paint(&mut raster, theme());
    assert_eq!(view.preview_items().len(), 8);
    view.start_preview_edit();
    named(&mut view, NamedKey::End);
    assert_eq!(
        view.preview_selected.as_ref().unwrap().as_str(),
        "output.severity.debug"
    );
    view.paint(&mut raster, theme());
    assert!(
        view.preview_scroll > 0,
        "short panes scroll the rendered rows"
    );
    assert!(view
        .preview_targets
        .iter()
        .any(|(id, _)| id.as_str() == "output.severity.debug"));
    named(&mut view, NamedKey::Enter);
    assert_eq!(view.catalog.as_ref().unwrap().entries().len(), 3);
    assert_eq!(view.title(), "Debug output colors");
}

#[test]
fn color_editor_graphic_tracks_valid_unsaved_draft_and_keeps_last_valid_on_error() {
    let base = rio_backend::config::Config::default();
    let mut view = SettingsView::default();
    view.fit(960.0, 620.0, 16.0);
    view.open_with_section(
        crate::settings_catalog::catalog(1, &base, &Default::default(), &[]).unwrap(),
        Some(Section::Customizations),
    );
    assert!(view.view.as_mut().unwrap().focus(
        &SettingId::new(automexia_ui_model::settings::COMMAND_OUTPUT_HIGHLIGHTING)
            .unwrap()
    ));
    view.focus = Focus::List;
    named(&mut view, NamedKey::Enter);
    view.enter_preview_item(SettingId::new("output.severity.error").unwrap());
    let color_id = SettingId::new("output.backgrounds.error").unwrap();
    assert!(view.view.as_mut().unwrap().focus(&color_id));
    view.focus = Focus::List;
    named(&mut view, NamedKey::Enter);
    assert!(replace_color(&mut view, "#0AC81EB4"));
    let mut valid = Raster::new(1.0);
    view.paint(&mut valid, theme());
    let expected = [10.0 / 255.0, 200.0 / 255.0, 30.0 / 255.0, 180.0 / 255.0];
    let preview = view.color_geometry.preview;
    let has_draft = |raster: &Raster| {
        raster.rects.iter().any(|(bounds, color)| {
            bounds[0] >= preview.x
                && bounds[1] >= preview.y
                && color
                    .iter()
                    .zip(expected)
                    .all(|(actual, expected)| (actual - expected).abs() < 0.001)
        })
    };
    assert!(
        has_draft(&valid),
        "graphic preview paints the exact unsaved RGBA draft"
    );
    assert_eq!(
        view.catalog.as_ref().unwrap().get(&color_id).unwrap().value,
        SettingValue::Color([105, 12, 25, 86])
    );
    assert!(replace_color(&mut view, "#123"));
    let mut invalid = Raster::new(1.0);
    view.paint(&mut invalid, theme());
    assert!(
        has_draft(&invalid),
        "invalid input keeps the last valid graphic"
    );
    assert!(view.take_edit().is_none());
}

#[test]
fn plain_tag_color_draft_uses_an_outline_in_both_current_and_draft_samples() {
    let base = rio_backend::config::Config::default();
    let preferences = crate::settings_catalog::apply_edit(
        1,
        &base,
        &Default::default(),
        &[],
        &Edit {
            revision: 1,
            id: SettingId::new("tags.style").unwrap(),
            change: Change::Set(SettingValue::Choice("plain".into())),
        },
    )
    .unwrap();
    let mut view = SettingsView::default();
    view.fit(960.0, 620.0, 16.0);
    view.open_customizations_with_slots(
        crate::settings_catalog::catalog(1, &base, &preferences, &[]).unwrap(),
        None,
        Some(crate::settings_catalog::slot_page_snapshot(
            &preferences,
            &preferences.apply_to(&base),
        )),
    );
    assert!(view
        .view
        .as_mut()
        .unwrap()
        .focus(&SettingId::new("tags.enabled").unwrap()));
    view.focus = Focus::List;
    named(&mut view, NamedKey::Enter);
    view.enter_preview_item(SettingId::new("tags.colors.windows").unwrap());
    view.focus = Focus::List;
    named(&mut view, NamedKey::Enter);
    assert!(replace_color(&mut view, "#55AAFF"));
    let mut raster = Raster::new(1.0);
    view.paint(&mut raster, theme());
    assert!(raster
        .shapes
        .iter()
        .any(|shape| matches!(shape, ShapeCall::Arc(..))));
    assert!(
        !raster.shapes.iter().any(|shape| match shape {
            ShapeCall::Rounded([x, y, width, height], _) => {
                let bounds = Rect {
                    x: *x,
                    y: *y,
                    width: *width,
                    height: *height,
                };
                bounds.intersect(view.color_geometry.preview) == Some(bounds)
            }
            _ => false,
        }),
        "Plain keeps the shared capsule outline without a filled rounded surface"
    );
}

#[test]
fn every_information_bar_preset_has_a_bounded_live_sample_and_shared_arrangement() {
    use automexia_ui_model::information_bar::InformationBarPreset;
    let base = rio_backend::config::Config::default();
    for preset in InformationBarPreset::ALL {
        let preferences = crate::settings_catalog::apply_edit(
            1,
            &base,
            &Default::default(),
            &[],
            &Edit {
                revision: 1,
                id: SettingId::new("tags.format").unwrap(),
                change: Change::Set(SettingValue::Choice(preset.id().into())),
            },
        )
        .unwrap();
        let mut view = SettingsView::default();
        view.fit(960.0, 620.0, 16.0);
        view.open_customizations_with_slots(
            crate::settings_catalog::catalog(1, &base, &preferences, &[]).unwrap(),
            None,
            Some(crate::settings_catalog::slot_page_snapshot(
                &preferences,
                &preferences.apply_to(&base),
            )),
        );
        assert!(view
            .view
            .as_mut()
            .unwrap()
            .focus(&SettingId::new("tags.enabled").unwrap()));
        view.focus = Focus::List;
        named(&mut view, NamedKey::Enter);
        let mut raster = Raster::new(1.0);
        view.paint(&mut raster, theme());
        assert!(
            !view.preview_targets.is_empty(),
            "preset {} has sample fragments",
            preset.id()
        );
        for (_, bounds) in &view.preview_targets {
            assert!(
                bounds.x >= view.geometry.preview.x
                    && bounds.y >= view.geometry.preview.y
            );
            assert!(
                bounds.x + bounds.width
                    <= view.geometry.preview.x + view.geometry.preview.width + 0.001
            );
            assert!(
                bounds.y + bounds.height
                    <= view.geometry.preview.y + view.geometry.preview.height + 0.001
            );
        }
        if preset == InformationBarPreset::LeftRightSplit {
            let user = view
                .preview_targets
                .iter()
                .find(|(id, _)| id.as_str() == "tags.slot.user.page")
                .unwrap()
                .1;
            let windows = view
                .preview_targets
                .iter()
                .find(|(id, _)| id.as_str() == "tags.slot.windows.page")
                .unwrap()
                .1;
            assert!(
                user.x > windows.x + windows.width,
                "shared split packer places trailing lane after leading tags"
            );
        }
        if preset == InformationBarPreset::TwoLinePrompt {
            let min_y = view
                .preview_targets
                .iter()
                .map(|(_, bounds)| bounds.y)
                .min_by(f32::total_cmp)
                .unwrap();
            let max_y = view
                .preview_targets
                .iter()
                .map(|(_, bounds)| bounds.y)
                .max_by(f32::total_cmp)
                .unwrap();
            assert!(
                max_y > min_y,
                "shared two-line break produces separate tag rows"
            );
        }
    }
}

#[test]
fn table_preview_uses_saved_appearance_and_ignores_status_switches() {
    use rio_backend::config::presentation::{Rgba, TableBanding, TableBorderStyle};
    let base = rio_backend::config::Config::default();
    let mut prefs = crate::automexia::preferences::UserPreferences::default();
    prefs.visual.tables.banding = Some(TableBanding::Rows);
    prefs.visual.tables.border_style = Some(TableBorderStyle::None);
    prefs.visual.tables.header_background = Some(Rgba::from_bytes([20, 30, 40, 255]));
    prefs.visual.tables.body_background = Some(Rgba::from_bytes([50, 60, 70, 255]));
    prefs.visual.tables.alternate_background = Some(Rgba::from_bytes([80, 90, 100, 255]));
    let mut view = SettingsView::default();
    view.fit(960.0, 620.0, 16.0);
    view.open_customizations_with_slots(
        crate::settings_catalog::catalog(1, &base, &prefs, &[]).unwrap(),
        None,
        Some(crate::settings_catalog::slot_page_snapshot(
            &prefs,
            &prefs.apply_to(&base),
        )),
    );
    assert!(view
        .view
        .as_mut()
        .unwrap()
        .focus(&SettingId::new(INLINE_TABLES).unwrap()));
    view.focus = Focus::List;
    named(&mut view, NamedKey::Enter);
    let sample = Rect {
        x: 10.0,
        y: 10.0,
        width: 500.0,
        height: 300.0,
    };
    let mut original = Raster::new(1.0);
    view.paint_table_preview(&mut original, sample, theme());
    for expected in [[20u8, 30, 40], [50, 60, 70], [80, 90, 100]] {
        assert!(original.rects.iter().any(|(_, color)| color[..3]
            .iter()
            .zip(expected)
            .all(|(v, e)| (v - f32::from(e) / 255.0).abs() < 0.001)));
    }
    for kube in [false, true] {
        prefs.presentation.kubernetes_highlighting = Some(kube);
        prefs.presentation.output_highlighting = Some(!kube);
        view.refresh_with_resources(
            crate::settings_catalog::catalog(2, &base, &prefs, &[]).unwrap(),
            None,
            Some(crate::settings_catalog::slot_page_snapshot(
                &prefs,
                &prefs.apply_to(&base),
            )),
        );
        let mut painted = Raster::new(1.0);
        view.paint_table_preview(&mut painted, sample, theme());
        assert_eq!(
            painted.rects, original.rects,
            "table sample has no semantic statuses"
        );
    }
    prefs.presentation.inline_tables = Some(false);
    view.refresh_with_resources(
        crate::settings_catalog::catalog(3, &base, &prefs, &[]).unwrap(),
        None,
        Some(crate::settings_catalog::slot_page_snapshot(
            &prefs,
            &prefs.apply_to(&base),
        )),
    );
    let mut disabled = Raster::new(1.0);
    view.paint_table_preview(&mut disabled, sample, theme());
    assert!(disabled
        .rects
        .iter()
        .all(|(_, color)| *color == base.colors.background.0));
    assert_eq!(prefs.visual.tables.banding, Some(TableBanding::Rows));
}

#[test]
fn window_controls_preview_is_bounded_nonexecuting_and_keyboard_editable() {
    use crate::settings_catalog::WINDOW_CONTROLS;
    use rio_backend::config::presentation::WindowControlStyle;
    let base = rio_backend::config::Config::default();
    for (width, height) in [(960.0, 620.0), (580.0, 520.0)] {
        let mut prefs = crate::automexia::preferences::UserPreferences::default();
        let mut view = SettingsView::default();
        view.fit(width, height, 16.0);
        view.open_customizations_with_slots(
            crate::settings_catalog::catalog(1, &base, &prefs, &[]).unwrap(),
            None,
            Some(crate::settings_catalog::slot_page_snapshot_with_config(
                &prefs,
                &base,
                &base,
                &crate::settings_catalog::test_installed_extensions(),
            )),
        );
        view.show_terminal_appearance();
        assert!(view
            .view
            .as_mut()
            .unwrap()
            .focus(&SettingId::new(WINDOW_CONTROLS).unwrap()));
        view.focus = Focus::List;
        named(&mut view, NamedKey::Enter);
        assert_eq!(
            view.customizations
                .as_ref()
                .unwrap()
                .active_key
                .as_ref()
                .unwrap()
                .as_str(),
            WINDOW_CONTROLS
        );
        assert!(view
            .view
            .as_mut()
            .unwrap()
            .focus(&SettingId::new(WINDOW_CONTROLS).unwrap()));
        view.focus = Focus::List;
        named(&mut view, NamedKey::ArrowRight);
        let edit = view.take_edit().unwrap();
        assert_eq!(
            edit.change,
            Change::Set(SettingValue::Choice("glass".into()))
        );
        prefs =
            crate::settings_catalog::apply_edit(1, &base, &prefs, &[], &edit).unwrap();
        view.refresh_with_resources(
            crate::settings_catalog::catalog(2, &base, &prefs, &[]).unwrap(),
            None,
            Some(crate::settings_catalog::slot_page_snapshot_with_config(
                &prefs,
                &prefs.apply_to(&base),
                &base,
                &crate::settings_catalog::test_installed_extensions(),
            )),
        );
        assert_eq!(
            prefs.visual.window_controls.style,
            Some(WindowControlStyle::Glass)
        );
        let mut raster = Raster::new(1.25);
        view.paint(&mut raster, theme());
        assert!(
            view.preview_targets
                .iter()
                .all(|(id, _)| window_controls_preview::choice_style(id).is_some()),
            "only style choices have targets, never caption actions"
        );
        assert!(view.take_edit().is_none());
        for sample in [
            Rect {
                x: 10.0,
                y: 10.0,
                width: 150.0,
                height: 74.0,
            },
            Rect {
                x: 10.0,
                y: 10.0,
                width: 400.0,
                height: 560.0,
            },
        ] {
            let mut raster = Raster::new(1.25);
            view.paint_window_controls_preview(&mut raster, sample, theme());
            assert!(!raster.shapes.is_empty());
            for ([x, y, w, h], _) in raster.rects {
                assert!(
                    x >= sample.x
                        && y >= sample.y
                        && x + w <= sample.x + sample.width + 0.01
                        && y + h <= sample.y + sample.height + 0.01
                );
            }
        }
        named(&mut view, NamedKey::Escape);
        assert!(view.is_open());
        assert!(view.customizations.as_ref().unwrap().active_key.is_none());
    }
}

#[test]
fn window_style_choices_are_selectable_cards_separate_from_state_examples() {
    let base = rio_backend::config::Config::default();
    let mut view = dependent_settings_view(
        &base,
        &Default::default(),
        crate::settings_catalog::WINDOW_CONTROLS,
    );
    view.fit(1280.0, 900.0, 16.0);
    view.paint(&mut Raster::new(1.0), UiTheme::from_colors(&base.colors));
    assert_eq!(
        view.preview_targets.len(),
        4,
        "four style choices, no caption state action targets"
    );
    let glass = view
        .preview_targets
        .iter()
        .find(|(id, _)| id.as_str() == "window-controls.choose.glass")
        .unwrap()
        .1;
    pointer_event(&mut view, glass, 1.0, ElementState::Pressed);
    pointer_event(&mut view, glass, 1.0, ElementState::Released);
    let edit = view.take_edit().unwrap();
    assert_eq!(edit.id.as_str(), crate::settings_catalog::WINDOW_CONTROLS);
    assert_eq!(
        edit.change,
        Change::Set(SettingValue::Choice("glass".into()))
    );
    assert!(view.is_open());
}

#[test]
fn window_style_choices_support_preview_keyboard_and_reject_unknown_choices() {
    let base = rio_backend::config::Config::default();
    let mut view = dependent_settings_view(
        &base,
        &Default::default(),
        crate::settings_catalog::WINDOW_CONTROLS,
    );
    view.fit(1280.0, 900.0, 16.0);
    view.paint(&mut Raster::new(1.0), UiTheme::from_colors(&base.colors));
    preview_edit_key(&mut view);
    assert_eq!(view.focus, Focus::Preview);
    named(&mut view, NamedKey::ArrowRight);
    named(&mut view, NamedKey::Enter);
    assert_eq!(
        view.take_edit().unwrap().change,
        Change::Set(SettingValue::Choice("glass".into()))
    );
    view.enter_preview_item(SettingId::new("window-controls.choose.unknown").unwrap());
    assert!(view.take_edit().is_none());
    named(&mut view, NamedKey::Escape);
    assert!(view.is_open());
}

#[test]
fn window_style_choices_highlight_whole_card_and_expose_one_selected_radio() {
    use rio_backend::config::presentation::WindowControlStyle;
    for entry in crate::automexia::theme_gallery::builtins() {
        let base = rio_backend::config::Config {
            colors: entry.theme.unwrap().colors,
            ..Default::default()
        };
        let theme = UiTheme::from_colors(&base.colors);
        for &style in WindowControlStyle::ALL {
            let mut prefs = crate::automexia::preferences::UserPreferences::default();
            prefs.visual.window_controls.style = Some(style);
            for (width, height, scale) in [
                (960.0, 740.0, 1.0),
                (1280.0, 900.0, 1.5),
                (1920.0, 1080.0, 2.0),
            ] {
                let mut view = dependent_settings_view(
                    &base,
                    &prefs,
                    crate::settings_catalog::WINDOW_CONTROLS,
                );
                view.fit(width, height, 16.0);
                let mut raster = Raster::new(scale);
                view.paint(&mut raster, theme);
                assert_eq!(view.preview_targets.len(), 4);
                for (id, card) in &view.preview_targets {
                    assert!(card.width >= 96.0 && card.height >= 28.0);
                    let fill = raster
                        .rects
                        .iter()
                        .find(|(rect, _)| *rect == card.array())
                        .unwrap()
                        .1;
                    if window_controls_preview::choice_style(id) == Some(style) {
                        assert_ne!(
                            fill, theme.surface,
                            "selected highlight covers the whole card"
                        );
                        assert_eq!(fill[3], 1.0);
                        let strokes = raster
                            .rects
                            .iter()
                            .filter(|(rect, color)| {
                                rect[0] >= card.x
                                    && rect[1] >= card.y
                                    && rect[0] + rect[2] <= card.x + card.width + 0.01
                                    && rect[1] + rect[3] <= card.y + card.height + 0.01
                                    && automexia_ui_model::contrast_ratio(*color, fill)
                                        >= 3.0
                            })
                            .count();
                        assert!(
                            strokes >= 4,
                            "visible border and non-color selection mark"
                        );
                    } else {
                        assert_eq!(fill, theme.surface);
                    }
                }
                let surface = view.accessibility_surface(
                    scale,
                    accesskit::Rect::new(
                        0.0,
                        0.0,
                        (width * scale) as f64,
                        (height * scale) as f64,
                    ),
                );
                let radios: Vec<_> = surface
                    .elements
                    .iter()
                    .filter(|e| e.node.role() == accesskit::Role::RadioButton)
                    .collect();
                assert_eq!(radios.len(), 4);
                let selected: Vec<_> = radios
                    .iter()
                    .filter(|e| e.node.toggled() == Some(accesskit::Toggled::True))
                    .collect();
                assert_eq!(selected.len(), 1);
                assert_eq!(
                    selected[0].node.author_id(),
                    Some(format!("window-controls.choose.{}", style.id()).as_str())
                );
                let bottom = view
                    .preview_targets
                    .iter()
                    .map(|(_, r)| r.y + r.height)
                    .fold(0.0, f32::max);
                assert!(
                    view.target_at(view.geometry.preview.x + 24.0, bottom + 75.0)
                        .is_none(),
                    "state examples cannot execute window actions"
                );
            }
        }
    }
}

#[test]
fn window_style_choices_compact_layout_and_refresh_preserve_one_selection() {
    use rio_backend::config::presentation::WindowControlStyle;
    let base = rio_backend::config::Config::default();
    let mut prefs = crate::automexia::preferences::UserPreferences::default();
    let mut view =
        dependent_settings_view(&base, &prefs, crate::settings_catalog::WINDOW_CONTROLS);
    for (width, height) in [(150.0, 74.0), (260.0, 280.0), (400.0, 560.0)] {
        let sample = Rect {
            x: 10.0,
            y: 10.0,
            width,
            height,
        };
        let mut raster = Raster::new(1.25);
        let targets = view.paint_window_controls_preview(
            &mut raster,
            sample,
            UiTheme::from_colors(&base.colors),
        );
        assert_eq!(targets.len(), if height < 100.0 { 0 } else { 4 });
        assert!(!raster.shapes.is_empty());
        for ([x, y, w, h], _) in raster.rects {
            assert!(
                x >= sample.x - 0.01
                    && y >= sample.y - 0.01
                    && x + w <= sample.x + width + 0.01
                    && y + h <= sample.y + height + 0.01
            );
        }
    }
    view.fit(1280.0, 900.0, 16.0);
    for (index, &style) in WindowControlStyle::ALL.iter().cycle().take(12).enumerate() {
        prefs.visual.window_controls.style = Some(style);
        view.refresh_with_resources(
            crate::settings_catalog::catalog(index as u64 + 2, &base, &prefs, &[])
                .unwrap(),
            None,
            Some(crate::settings_catalog::slot_page_snapshot_with_config(
                &prefs,
                &prefs.apply_to(&base),
                &base,
                &crate::settings_catalog::test_installed_extensions(),
            )),
        );
        view.paint(&mut Raster::new(1.0), UiTheme::from_colors(&base.colors));
        assert_eq!(view.selected_window_control_style(), style);
        assert_eq!(
            view.preview_targets.len(),
            4,
            "redraws never accumulate choices"
        );
    }
    named(&mut view, NamedKey::Escape);
    view.enter_preview_item(SettingId::new("window-controls.choose.glass").unwrap());
    assert!(
        view.take_edit().is_none(),
        "a stale style card cannot edit another page"
    );
}

#[test]
#[ignore = "same-host production style choice and state preview paint benchmark"]
fn window_style_choices_paint_benchmark() {
    let base = rio_backend::config::Config::default();
    let mut view = dependent_settings_view(
        &base,
        &Default::default(),
        crate::settings_catalog::WINDOW_CONTROLS,
    );
    let mut raster = Raster::new(1.0);
    for (width, height) in [(960.0, 740.0), (1920.0, 1080.0)] {
        view.fit(width, height, 16.0);
        let mut samples = Vec::with_capacity(200);
        for iteration in 0..220 {
            raster.rects.clear();
            raster.shapes.clear();
            raster.text.clear();
            let start = std::time::Instant::now();
            view.paint(
                std::hint::black_box(&mut raster),
                UiTheme::from_colors(&base.colors),
            );
            let ns = start.elapsed().as_nanos();
            assert_eq!(view.preview_targets.len(), 4);
            assert!(raster.rects.len() < 1200, "preview draw work is bounded");
            if iteration >= 20 {
                samples.push(ns);
            }
        }
        samples.sort_unstable();
        println!(
            "{}",
            serde_json::json!({"benchmark":"window_style_choices_paint","viewport":[width,height],"samples":200,"p50_ns":samples[99],"p95_ns":samples[189],"excludes":["GPU","presentation"]})
        );
    }
}

#[test]
fn inline_table_menu_keyboard_edits_refresh_preview_and_survive_layout_changes() {
    use rio_backend::config::presentation::{Rgba, TableBanding};
    let base = rio_backend::config::Config::default();
    let mut prefs = crate::automexia::preferences::UserPreferences::default();
    prefs.visual.tables.row_lines = Some(false);
    prefs.visual.tables.column_lines = Some(false);
    prefs.visual.tables.alternate_background = Some(Rgba::from_bytes([30, 50, 70, 128]));
    for (width, height, scale) in [
        (960.0, 620.0, 1.0),
        (640.0, 480.0, 1.25),
        (1280.0, 800.0, 2.0),
    ] {
        let original = prefs.clone();
        let mut view = SettingsView::default();
        view.fit(width, height, 16.0);
        view.open_customizations_with_slots(
            crate::settings_catalog::catalog(1, &base, &original, &[]).unwrap(),
            None,
            Some(crate::settings_catalog::slot_page_snapshot_with_config(
                &original,
                &original.apply_to(&base),
                &base,
                &crate::settings_catalog::test_installed_extensions(),
            )),
        );
        assert!(view
            .view
            .as_mut()
            .unwrap()
            .focus(&SettingId::new(INLINE_TABLES).unwrap()));
        view.focus = Focus::List;
        named(&mut view, NamedKey::Enter);
        let id = SettingId::new("tables.banding").unwrap();
        assert!(view.view.as_mut().unwrap().focus(&id));
        view.focus = Focus::List;
        named(&mut view, NamedKey::ArrowRight);
        let edit = view.take_edit().unwrap();
        assert_eq!(edit.id, id);
        assert_eq!(
            edit.change,
            Change::Set(SettingValue::Choice("rows".into()))
        );
        let changed =
            crate::settings_catalog::apply_edit(1, &base, &original, &[], &edit).unwrap();
        assert_eq!(changed.visual.tables.banding, Some(TableBanding::Rows));
        view.refresh_with_resources(
            crate::settings_catalog::catalog(2, &base, &changed, &[]).unwrap(),
            None,
            Some(crate::settings_catalog::slot_page_snapshot_with_config(
                &changed,
                &changed.apply_to(&base),
                &base,
                &crate::settings_catalog::test_installed_extensions(),
            )),
        );
        assert_eq!(view.title(), "Inline tables");
        assert_eq!(view.catalog.as_ref().unwrap().entries().len(), 20);
        let mut raster = Raster::new(scale);
        view.paint(&mut raster, theme());
        let stripe = [30.0 / 255.0, 50.0 / 255.0, 70.0 / 255.0];
        assert!(
            raster.rects.iter().any(|(bounds, color)| {
                bounds[0] >= view.geometry.preview.x
                    && bounds[1] >= view.geometry.preview.y
                    && (0..3).all(|i| {
                        (color[i]
                            - (stripe[i] * 128.0 / 255.0
                                + base.colors.background.0[i] * 127.0 / 255.0))
                            .abs()
                            < 0.001
                    })
            }),
            "the compact preview must show an alternate data row"
        );
        assert!(!raster.rects.is_empty());
        let (w, h) = ((width * scale) as u32, (height * scale) as u32);
        let pixels = raster.pixels(w, h, true);
        assert!(pixels.iter().any(|pixel| *pixel != 0x00112233));
        if let Some(directory) = std::env::var_os("AUTOMEXIA_SETTINGS_PREVIEW_DIR") {
            let directory = std::path::PathBuf::from(directory);
            std::fs::create_dir_all(&directory).unwrap();
            image_rs::RgbImage::from_fn(w, h, |x, y| {
                let pixel = pixels[(y * w + x) as usize];
                image_rs::Rgb([(pixel >> 16) as u8, (pixel >> 8) as u8, pixel as u8])
            })
            .save(directory.join(format!("inline-tables-{w}x{h}.png")))
            .unwrap();
        }
        named(&mut view, NamedKey::Escape);
        assert_eq!(view.title(), "Workflow & Output");
    }
}

#[test]
fn preview_tag_mouse_activation_is_direct_and_keyboard_uses_edit_button() {
    let base = rio_backend::config::Config::default();
    let preferences = crate::automexia::preferences::UserPreferences::default();
    let mut view = SettingsView::default();
    view.fit(960.0, 620.0, 16.0);
    view.open_customizations_with_slots(
        crate::settings_catalog::catalog(1, &base, &preferences, &[]).unwrap(),
        None,
        Some(crate::settings_catalog::slot_page_snapshot(
            &preferences,
            &base,
        )),
    );
    assert!(view
        .view
        .as_mut()
        .unwrap()
        .focus(&SettingId::new("tags.enabled").unwrap()));
    view.focus = Focus::List;
    named(&mut view, NamedKey::Enter);
    let mut raster = Raster::new(1.0);
    view.paint(&mut raster, theme());
    let (live_id, live_tag) = view
        .preview_targets
        .iter()
        .find(|(id, _)| id.as_str() == "tags.slot.windows.page")
        .unwrap()
        .clone();
    let x = live_tag.x + live_tag.width * 0.5;
    let y = live_tag.y + live_tag.height * 0.5;
    assert_eq!(view.target_at(x, y), Some(Target::PreviewItem(live_id)));
    pointer_event(&mut view, live_tag, 1.0, ElementState::Pressed);
    pointer_event(&mut view, live_tag, 1.0, ElementState::Released);
    assert_eq!(view.title(), "Tag slot: Windows");
    named(&mut view, NamedKey::Escape);
    assert_eq!(view.title(), "Information tags");
    view.paint(&mut raster, theme());
    let button = view.preview_button;
    pointer_event(&mut view, button, 1.0, ElementState::Pressed);
    pointer_event(&mut view, button, 1.0, ElementState::Released);
    assert!(view.preview_edit_mode);
    view.paint(&mut raster, theme());
    let (selected, target) = view.preview_targets[0].clone();
    assert!(view.preview_tag_list_area.height > 0.0);
    assert!(view.preview_targets.iter().any(|(id, _)| id == &selected));
    pointer_event(&mut view, target, 1.0, ElementState::Pressed);
    pointer_event(&mut view, target, 1.0, ElementState::Released);
    assert_eq!(
        view.customizations.as_ref().unwrap().active_slot.as_ref(),
        Some(&selected)
    );
}

#[test]
fn every_enabled_tag_has_sample_data_without_live_detection() {
    let base = rio_backend::config::Config::default();
    let preferences = crate::automexia::preferences::UserPreferences::default();
    let mut view = SettingsView::default();
    view.fit(1200.0, 1000.0, 16.0);
    view.open_customizations_with_slots(
        crate::settings_catalog::catalog(1, &base, &preferences, &[]).unwrap(),
        None,
        Some(crate::settings_catalog::slot_page_snapshot(
            &preferences,
            &base,
        )),
    );
    view.view
        .as_mut()
        .unwrap()
        .focus(&SettingId::new("tags.enabled").unwrap());
    view.focus = Focus::List;
    named(&mut view, NamedKey::Enter);
    view.paint(&mut Raster::new(1.0), theme());
    for role in automexia_ui_model::information_bar::STANDARD_ROLES {
        let id = format!(
            "tags.slot.{}.page",
            automexia_ui_model::information_bar::role_id(role)
        );
        assert!(
            view.preview_item_rows
                .iter()
                .any(|(key, _)| key.as_str() == id),
            "enabled {id} needs a graphic sample even with detection unavailable"
        );
    }
}

#[test]
fn tag_keyboard_selection_keeps_roster_order_and_reveals_add_action() {
    let base = rio_backend::config::Config::default();
    let preferences = crate::automexia::preferences::UserPreferences::default();
    let mut view = SettingsView::default();
    view.fit(320.0, 420.0, 16.0);
    view.open_customizations_with_slots(
        crate::settings_catalog::catalog(1, &base, &preferences, &[]).unwrap(),
        None,
        Some(crate::settings_catalog::slot_page_snapshot(
            &preferences,
            &base,
        )),
    );
    view.view
        .as_mut()
        .unwrap()
        .focus(&SettingId::new("tags.enabled").unwrap());
    view.focus = Focus::List;
    named(&mut view, NamedKey::Enter);
    view.paint(&mut Raster::new(1.0), theme());
    let button = view.preview_button;
    pointer_event(&mut view, button, 1.0, ElementState::Pressed);
    pointer_event(&mut view, button, 1.0, ElementState::Released);
    named(&mut view, NamedKey::Home);
    for role in automexia_ui_model::information_bar::STANDARD_ROLES {
        view.paint(&mut Raster::new(1.0), theme());
        let id = format!(
            "tags.slot.{}.page",
            automexia_ui_model::information_bar::role_id(role)
        );
        assert_eq!(
            view.preview_selected.as_ref().unwrap().as_str(),
            id,
            "navigation must not reorder as the selected sample changes"
        );
        assert!(
            view.preview_targets
                .iter()
                .any(|(key, rect)| key.as_str() == id
                    && rect.y >= view.preview_tag_list_area.y),
            "the selected roster row must scroll into view"
        );
        named(&mut view, NamedKey::ArrowRight);
    }
    view.paint(&mut Raster::new(1.0), theme());
    assert_eq!(
        view.preview_selected.as_ref().unwrap().as_str(),
        "tags.add-slot"
    );
    assert!(view
        .preview_targets
        .iter()
        .any(|(key, _)| key.as_str() == "tags.add-slot"));
    named(&mut view, NamedKey::Enter);
    assert_eq!(view.take_edit().unwrap().id.as_str(), "tags.add-slot");
}

#[test]
fn every_tag_roster_row_opens_its_editor_across_redraws_and_scales() {
    let base = rio_backend::config::Config::default();
    let preferences = crate::automexia::preferences::UserPreferences::default();
    for scale in [1.0, 1.25, 2.0] {
        let mut view = SettingsView::default();
        view.fit(1200.0, 1000.0, 16.0);
        view.open_customizations_with_slots(
            crate::settings_catalog::catalog(1, &base, &preferences, &[]).unwrap(),
            None,
            Some(crate::settings_catalog::slot_page_snapshot(
                &preferences,
                &base,
            )),
        );
        view.view
            .as_mut()
            .unwrap()
            .focus(&SettingId::new("tags.enabled").unwrap());
        view.focus = Focus::List;
        named(&mut view, NamedKey::Enter);
        for role in automexia_ui_model::information_bar::STANDARD_ROLES {
            let id = SettingId::new(format!(
                "tags.slot.{}.page",
                automexia_ui_model::information_bar::role_id(role)
            ))
            .unwrap();
            view.paint(&mut Raster::new(1.0), theme());
            let bounds = view
                .preview_targets
                .iter()
                .find(|(key, rect)| key == &id && rect.y >= view.preview_tag_list_area.y)
                .unwrap()
                .1;
            pointer_event(&mut view, bounds, scale, ElementState::Pressed);
            view.fit(1200.0, 1000.0, 16.0);
            view.paint(&mut Raster::new(1.0), theme());
            pointer_event(&mut view, bounds, scale, ElementState::Released);
            assert_eq!(
                view.customizations.as_ref().unwrap().active_slot.as_ref(),
                Some(&id),
                "failed to open {id:?} at scale {scale}"
            );
            named(&mut view, NamedKey::Escape);
        }
    }
}

#[test]
fn all_tag_roster_exposes_optional_tags_and_adds_a_custom_tag() {
    let base = rio_backend::config::Config::default();
    let preferences = crate::automexia::preferences::UserPreferences::default();
    let mut view = SettingsView::default();
    view.fit(960.0, 620.0, 16.0);
    view.open_customizations_with_slots(
        crate::settings_catalog::catalog(
            1,
            &base,
            &preferences,
            &crate::settings_catalog::test_installed_extensions(),
        )
        .unwrap(),
        None,
        Some(crate::settings_catalog::slot_page_snapshot(
            &preferences,
            &base,
        )),
    );
    assert!(view
        .view
        .as_mut()
        .unwrap()
        .focus(&SettingId::new("tags.enabled").unwrap()));
    view.focus = Focus::List;
    named(&mut view, NamedKey::Enter);
    view.paint(&mut Raster::new(1.0), theme());
    for role in ["kubernetes", "terraform"] {
        let id = SettingId::new(format!("tags.slot.{role}.page")).unwrap();
        assert!(
            view.preview_order.contains(&id),
            "{role} must be listed even without sample data"
        );
    }
    let snapshot = view
        .customizations
        .as_ref()
        .unwrap()
        .slot_pages
        .as_ref()
        .unwrap();
    let terraform_index = crate::settings_catalog::slot_page_actions(snapshot)
        .unwrap()
        .iter()
        .position(|entry| entry.id.as_str() == "tags.slot.terraform.page")
        .unwrap();
    view.preview_tag_list_scroll = terraform_index;
    view.paint(&mut Raster::new(1.0), theme());
    let terraform = view
        .preview_targets
        .iter()
        .find(|(id, bounds)| {
            id.as_str() == "tags.slot.terraform.page"
                && bounds.y >= view.preview_tag_list_area.y
        })
        .map(|(_, bounds)| *bounds)
        .unwrap();
    pointer_event(&mut view, terraform, 1.0, ElementState::Pressed);
    pointer_event(&mut view, terraform, 1.0, ElementState::Released);
    assert_eq!(
        view.customizations
            .as_ref()
            .unwrap()
            .active_slot
            .as_ref()
            .unwrap()
            .as_str(),
        "tags.slot.terraform.page"
    );
    assert_eq!(
        view.catalog
            .as_ref()
            .unwrap()
            .get(&SettingId::new("tags.slot.terraform.enabled").unwrap())
            .unwrap()
            .value,
        SettingValue::Boolean(true)
    );
    assert!(view
        .view
        .as_mut()
        .unwrap()
        .focus(&SettingId::new("tags.slot.terraform.enabled").unwrap()));
    named(&mut view, NamedKey::Enter);
    let toggle = view
        .take_edit()
        .expect("the tag toggle reaches the shared settings owner");
    let disabled = crate::settings_catalog::apply_edit(
        1,
        &base,
        &preferences,
        &crate::settings_catalog::test_installed_extensions(),
        &toggle,
    )
    .unwrap();
    view.refresh_with_resources(
        crate::settings_catalog::catalog(
            2,
            &base,
            &disabled,
            &crate::settings_catalog::test_installed_extensions(),
        )
        .unwrap(),
        None,
        Some(crate::settings_catalog::slot_page_snapshot(
            &disabled,
            &disabled.apply_to(&base),
        )),
    );
    assert_eq!(
        view.catalog
            .as_ref()
            .unwrap()
            .get(&SettingId::new("tags.slot.terraform.enabled").unwrap())
            .unwrap()
            .value,
        SettingValue::Boolean(false)
    );
    named(&mut view, NamedKey::Escape);
    view.paint(&mut Raster::new(1.0), theme());
    assert!(!view
        .preview_item_rows
        .iter()
        .any(|(id, _)| id.as_str() == "tags.slot.terraform.page"));
    let disabled_row = view
        .preview_targets
        .iter()
        .find(|(id, bounds)| {
            id.as_str() == "tags.slot.terraform.page"
                && bounds.y >= view.preview_tag_list_area.y
        })
        .unwrap()
        .1;
    pointer_event(&mut view, disabled_row, 1.0, ElementState::Pressed);
    pointer_event(&mut view, disabled_row, 1.0, ElementState::Released);
    assert_eq!(view.title(), "Tag slot: Terraform");
    named(&mut view, NamedKey::Escape);
    view.focus = Focus::Search;
    assert!(view.paste("Kubernetes"));
    view.preview_tag_list_scroll = usize::MAX;
    view.paint(&mut Raster::new(1.0), theme());
    let add = view
        .preview_targets
        .iter()
        .find(|(id, _)| id.as_str() == "tags.add-slot")
        .map(|(_, bounds)| *bounds)
        .expect("add custom tag remains discoverable");
    pointer_event(&mut view, add, 1.0, ElementState::Pressed);
    pointer_event(&mut view, add, 1.0, ElementState::Released);
    let pending = view
        .take_edit()
        .expect("add action reaches the shared settings owner");
    assert_eq!(pending.id.as_str(), "tags.add-slot");
    let updated = crate::settings_catalog::apply_edit(
        2,
        &base,
        &disabled,
        &crate::settings_catalog::test_installed_extensions(),
        &pending,
    )
    .unwrap();
    assert!(updated
        .visual
        .information_bar
        .recipe()
        .slots
        .iter()
        .any(|slot| slot.id == "custom-1"));
}

#[test]
fn narrow_tag_roster_scrolls_with_the_mouse_before_keyboard_edit_mode() {
    let base = rio_backend::config::Config::default();
    let preferences = crate::automexia::preferences::UserPreferences::default();
    let mut view = SettingsView::default();
    view.fit(320.0, 420.0, 16.0);
    view.open_customizations_with_slots(
        crate::settings_catalog::catalog(1, &base, &preferences, &[]).unwrap(),
        None,
        Some(crate::settings_catalog::slot_page_snapshot(
            &preferences,
            &base,
        )),
    );
    assert!(view
        .view
        .as_mut()
        .unwrap()
        .focus(&SettingId::new("tags.enabled").unwrap()));
    view.focus = Focus::List;
    named(&mut view, NamedKey::Enter);
    view.paint(&mut Raster::new(1.0), theme());
    assert!(!view.preview_edit_mode);
    let area = view.preview_tag_list_area;
    assert!(area.height >= 16.0);
    view.pointer = Some((area.x + area.width * 0.5, area.y + area.height * 0.5));
    // SAFETY: this ID is confined to constructing a pure adapter event.
    let device_id = unsafe { DeviceId::dummy() };
    let result = view.event(
        &WindowEvent::MouseWheel {
            device_id,
            delta: MouseScrollDelta::LineDelta(0.0, -1.0),
            phase: TouchPhase::Moved,
        },
        ModifiersState::empty(),
        1.0,
    );
    assert!(result.consumed);
    assert!(view.preview_tag_list_scroll > 0);
    let mut target = None;
    for _ in 0..13 {
        view.paint(&mut Raster::new(1.0), theme());
        target = view
            .preview_targets
            .iter()
            .find(|(id, bounds)| {
                id.as_str() == "tags.slot.terraform.page"
                    && bounds.y >= view.preview_tag_list_area.y
            })
            .map(|(_, bounds)| *bounds);
        if target.is_some() {
            break;
        }
        view.event(
            &WindowEvent::MouseWheel {
                device_id,
                delta: MouseScrollDelta::LineDelta(0.0, -0.25),
                phase: TouchPhase::Moved,
            },
            ModifiersState::empty(),
            1.0,
        );
    }
    let target = target.expect("wheel scrolling must reach Terraform");
    pointer_event(&mut view, target, 1.0, ElementState::Pressed);
    pointer_event(&mut view, target, 1.0, ElementState::Released);
    view.paint(&mut Raster::new(1.0), theme());
    assert_eq!(view.title(), "Tag slot: Terraform");
    assert!(
        view.preview_targets
            .iter()
            .any(|(id, bounds)| id.as_str() == "tags.slot.terraform.page"
                && bounds.y < view.preview_tag_list_area.y),
        "selecting a roster row must reveal its graphic sample in a short pane"
    );
}

#[test]
fn selected_tag_controls_replace_the_left_half_while_the_live_tags_remain_clickable() {
    let base = rio_backend::config::Config::default();
    let mut preferences = crate::automexia::preferences::UserPreferences::default();
    preferences
        .set_extension_feature_enabled(
            crate::automexia::settings_extensions::DEVOPS_CONTEXT_STATUS_ID,
            true,
        )
        .unwrap();
    let market = [crate::automexia::marketplace::MarketItem {
        id: crate::automexia::builtins::devops::ID.into(),
        name: "DevOps".into(),
        description: "Local context".into(),
        installed: true,
    }];
    let mut view = SettingsView::default();
    view.fit(960.0, 620.0, 16.0);
    view.open_customizations_with_slots(
        crate::settings_catalog::catalog(1, &base, &preferences, &market).unwrap(),
        None,
        Some(crate::settings_catalog::slot_page_snapshot(
            &preferences,
            &base,
        )),
    );
    assert!(view
        .view
        .as_mut()
        .unwrap()
        .focus(&SettingId::new("tags.enabled").unwrap()));
    view.focus = Focus::List;
    named(&mut view, NamedKey::Enter);
    let mut raster = Raster::new(1.0);
    view.paint(&mut raster, theme());
    view.start_preview_edit();
    view.paint(&mut raster, theme());
    let windows = view
        .preview_targets
        .iter()
        .find(|(id, _)| id.as_str() == "tags.slot.windows.page")
        .unwrap()
        .1;
    pointer_event(&mut view, windows, 1.0, ElementState::Pressed);
    pointer_event(&mut view, windows, 1.0, ElementState::Released);
    assert_eq!(view.title(), "Tag slot: Windows");
    assert!(view.geometry.preview.x > view.geometry.body.x);
    assert!(view
        .catalog
        .as_ref()
        .unwrap()
        .entries()
        .iter()
        .any(|entry| { entry.label.contains("This tag color") }));
    assert!(view
        .catalog
        .as_ref()
        .unwrap()
        .entries()
        .iter()
        .any(|entry| { entry.label == "Default role color" }));
    view.paint(&mut raster, theme());
    let kubernetes = view
        .preview_targets
        .iter()
        .find(|(id, _)| id.as_str() == "tags.slot.kubernetes.page")
        .unwrap()
        .1;
    pointer_event(&mut view, kubernetes, 1.0, ElementState::Pressed);
    pointer_event(&mut view, kubernetes, 1.0, ElementState::Released);
    assert_eq!(view.title(), "Tag slot: Kubernetes");
    named(&mut view, NamedKey::Escape);
    assert_eq!(view.title(), "Information tags");
    assert_eq!(
        view.preview_selected.as_ref().unwrap().as_str(),
        "tags.slot.kubernetes.page"
    );
}

#[test]
fn devops_uninstall_removes_preview_and_editor_targets_with_saved_switch_on() {
    let base = rio_backend::config::Config::default();
    let mut preferences = crate::automexia::preferences::UserPreferences::default();
    preferences
        .set_extension_feature_enabled(
            crate::automexia::settings_extensions::DEVOPS_CONTEXT_STATUS_ID,
            true,
        )
        .unwrap();
    let market = [crate::automexia::marketplace::MarketItem {
        id: crate::automexia::builtins::devops::ID.into(),
        name: "DevOps".into(),
        description: "Local context".into(),
        installed: false,
    }];
    let mut view = SettingsView::default();
    view.fit(960.0, 620.0, 16.0);
    view.open_customizations_with_slots(
        crate::settings_catalog::catalog(1, &base, &preferences, &market).unwrap(),
        None,
        Some(crate::settings_catalog::slot_page_snapshot_with_config(
            &preferences,
            &base,
            &base,
            &market,
        )),
    );
    view.view
        .as_mut()
        .unwrap()
        .focus(&SettingId::new("tags.enabled").unwrap());
    view.focus = Focus::List;
    named(&mut view, NamedKey::Enter);
    let mut raster = Raster::new(1.0);
    view.paint(&mut raster, theme());
    for role in [
        "kubernetes",
        "docker",
        "terraform",
        "git",
        "production",
        "environment",
    ] {
        let id = format!("tags.slot.{role}.page");
        assert!(
            !view.preview_order.iter().any(|entry| entry.as_str() == id),
            "uninstalled extension still exposes {role}"
        );
        assert!(!view
            .preview_targets
            .iter()
            .any(|(entry, _)| entry.as_str() == id));
    }
    for role in ["windows", "ubuntu-wsl", "user"] {
        assert!(view
            .preview_order
            .iter()
            .any(|entry| entry.as_str() == format!("tags.slot.{role}.page")));
    }
    if let Some(directory) = std::env::var_os("AUTOMEXIA_SETTINGS_PREVIEW_DIR") {
        let directory = std::path::PathBuf::from(directory);
        std::fs::create_dir_all(&directory).unwrap();
        let pixels = raster.pixels(960, 620, true);
        image_rs::RgbImage::from_fn(960, 620, |x, y| {
            let pixel = pixels[(y * 960 + x) as usize];
            image_rs::Rgb([(pixel >> 16) as u8, (pixel >> 8) as u8, pixel as u8])
        })
        .save(directory.join("devops-uninstalled.png"))
        .unwrap();
    }
}

#[test]
fn devops_inventory_refresh_closes_removed_editor_and_reinstall_restores_choices() {
    let base = rio_backend::config::Config::default();
    let mut prefs = crate::automexia::preferences::UserPreferences::default();
    let mut recipe = prefs.visual.information_bar.recipe();
    recipe
        .slots
        .iter_mut()
        .find(|slot| slot.id == "kubernetes")
        .unwrap()
        .color = Some([17, 43, 91]);
    prefs.visual.information_bar.use_custom = true;
    prefs.visual.information_bar.custom_recipe = Some(recipe.clone());
    let mut market = crate::settings_catalog::test_installed_extensions();
    let mut view = SettingsView::default();
    view.fit(960.0, 620.0, 16.0);
    view.open_customizations_with_slots(
        crate::settings_catalog::catalog(1, &base, &prefs, &market).unwrap(),
        None,
        Some(crate::settings_catalog::slot_page_snapshot_with_config(
            &prefs, &base, &base, &market,
        )),
    );
    view.view
        .as_mut()
        .unwrap()
        .focus(&SettingId::new("tags.enabled").unwrap());
    view.focus = Focus::List;
    named(&mut view, NamedKey::Enter);
    view.paint(&mut Raster::new(1.0), theme());
    view.enter_preview_item(SettingId::new("tags.slot.kubernetes.page").unwrap());
    assert!(view.customizations.as_ref().unwrap().active_slot.is_some());
    for (revision, installed) in [(2, false), (3, true), (4, false), (5, true)] {
        market[0].installed = installed;
        view.refresh_with_resources(
            crate::settings_catalog::catalog(revision, &base, &prefs, &market).unwrap(),
            None,
            Some(crate::settings_catalog::slot_page_snapshot_with_config(
                &prefs, &base, &base, &market,
            )),
        );
        assert!(view.pending.is_none());
        assert!(view.customizations.as_ref().unwrap().active_slot.is_none());
        if view.title() != "Information tags" {
            view.view
                .as_mut()
                .unwrap()
                .focus(&SettingId::new("tags.enabled").unwrap());
            view.focus = Focus::List;
            named(&mut view, NamedKey::Enter);
        }
        view.paint(&mut Raster::new(1.0), theme());
        assert_eq!(
            view.preview_order
                .iter()
                .any(|id| id.as_str() == "tags.slot.kubernetes.page"),
            installed
        );
        assert_eq!(
            view.preview_order
                .iter()
                .any(|id| id.as_str() == "tags.slot.git.page"),
            installed
        );
        view.start_preview_edit();
        for _ in 0..view.preview_order.len() + 1 {
            named(&mut view, NamedKey::Tab);
            assert_eq!(view.focus, Focus::Preview);
            assert!(view
                .preview_order
                .contains(view.preview_selected.as_ref().unwrap()));
        }
        assert_eq!(prefs.visual.information_bar.recipe(), recipe);
    }
}

#[test]
fn devops_toggle_hides_preview_tags_and_keyboard_targets() {
    let base = rio_backend::config::Config::default();
    let market = [crate::automexia::marketplace::MarketItem {
        id: crate::automexia::builtins::devops::ID.into(),
        name: "DevOps".into(),
        description: "Local context".into(),
        installed: true,
    }];
    for (width, height) in [(960_u32, 620_u32), (320, 420)] {
        for enabled in [true, false] {
            let mut preferences =
                crate::automexia::preferences::UserPreferences::default();
            preferences
                .set_extension_feature_enabled(
                    crate::automexia::settings_extensions::DEVOPS_CONTEXT_STATUS_ID,
                    enabled,
                )
                .unwrap();
            let mut view = SettingsView::default();
            view.fit(width as f32, height as f32, 16.0);
            view.open_customizations_with_slots(
                crate::settings_catalog::catalog(1, &base, &preferences, &market)
                    .unwrap(),
                None,
                Some(crate::settings_catalog::slot_page_snapshot(
                    &preferences,
                    &base,
                )),
            );
            assert!(view
                .view
                .as_mut()
                .unwrap()
                .focus(&SettingId::new("tags.enabled").unwrap()));
            view.focus = Focus::List;
            named(&mut view, NamedKey::Enter);
            assert!(view
                .catalog
                .as_ref()
                .unwrap()
                .get(
                    &SettingId::new(
                        crate::automexia::settings_extensions::DEVOPS_CONTEXT_STATUS_ID
                    )
                    .unwrap()
                )
                .is_some());
            let mut raster = Raster::new(1.0);
            view.paint(&mut raster, theme());
            for role in [
                "production",
                "kubernetes",
                "docker",
                "azure",
                "aws",
                "gcp",
                "unknown-cloud",
                "terraform",
                "environment",
            ] {
                let page = format!("tags.slot.{role}.page");
                assert_eq!(
                    view.preview_order.iter().any(|id| id.as_str() == page),
                    enabled,
                    "{role} at {width}px, DevOps enabled={enabled}"
                );
                if !enabled {
                    assert!(!view
                        .preview_targets
                        .iter()
                        .any(|(id, _)| id.as_str() == page));
                }
            }
            assert!(view
                .preview_order
                .iter()
                .any(|id| id.as_str() == "tags.slot.git.page"));
            if width == 960 {
                let sample_has = |role: &str| {
                    view.preview_targets.iter().any(|(id, bounds)| {
                        id.as_str() == role && bounds.y < view.preview_tag_list_area.y
                    })
                };
                assert!(sample_has("tags.slot.windows.page"));
                assert!(sample_has("tags.slot.git.page"));
                assert!(sample_has("tags.slot.user.page"));
                assert_eq!(sample_has("tags.slot.kubernetes.page"), enabled);
                assert_eq!(sample_has("tags.slot.terraform.page"), enabled);
            }
            assert!(view.geometry.preview.height > 100.0);
            if let Some(directory) = std::env::var_os("AUTOMEXIA_SETTINGS_PREVIEW_DIR") {
                let directory = std::path::PathBuf::from(directory);
                std::fs::create_dir_all(&directory).unwrap();
                let pixels = raster.pixels(width, height, true);
                image_rs::RgbImage::from_fn(width, height, |x, y| {
                    let pixel = pixels[(y * width + x) as usize];
                    image_rs::Rgb([(pixel >> 16) as u8, (pixel >> 8) as u8, pixel as u8])
                })
                .save(directory.join(format!(
                    "devops-showcase-{}-{}x{}.png",
                    if enabled { "on" } else { "off" },
                    width,
                    height
                )))
                .unwrap();
            }
        }
    }
}

#[test]
fn devops_toggle_restores_choices_and_recovers_preview_selection() {
    let base = rio_backend::config::Config::default();
    let market = [crate::automexia::marketplace::MarketItem {
        id: crate::automexia::builtins::devops::ID.into(),
        name: "DevOps".into(),
        description: "Local context".into(),
        installed: true,
    }];
    let mut preferences = crate::automexia::preferences::UserPreferences::default();
    let mut recipe = preferences.visual.information_bar.recipe();
    let terraform = recipe
        .slots
        .iter_mut()
        .find(|slot| slot.id == "terraform")
        .unwrap();
    terraform.enabled = false;
    terraform.color = Some([17, 43, 91]);
    preferences.visual.information_bar.use_custom = true;
    preferences.visual.information_bar.custom_recipe = Some(recipe.clone());
    let mut view = SettingsView::default();
    view.fit(960.0, 620.0, 16.0);
    view.open_customizations_with_slots(
        crate::settings_catalog::catalog(1, &base, &preferences, &market).unwrap(),
        None,
        Some(crate::settings_catalog::slot_page_snapshot(
            &preferences,
            &base,
        )),
    );
    assert!(view
        .view
        .as_mut()
        .unwrap()
        .focus(&SettingId::new("tags.enabled").unwrap()));
    view.focus = Focus::List;
    named(&mut view, NamedKey::Enter);
    view.paint(&mut Raster::new(1.0), theme());
    view.start_preview_edit();
    view.preview_selected = Some(SettingId::new("tags.slot.kubernetes.page").unwrap());

    for (revision, enabled) in [(2, false), (3, true), (4, false), (5, true)] {
        preferences
            .set_extension_feature_enabled(
                crate::automexia::settings_extensions::DEVOPS_CONTEXT_STATUS_ID,
                enabled,
            )
            .unwrap();
        view.refresh_with_resources(
            crate::settings_catalog::catalog(revision, &base, &preferences, &market)
                .unwrap(),
            None,
            Some(crate::settings_catalog::slot_page_snapshot(
                &preferences,
                &base,
            )),
        );
        view.paint(&mut Raster::new(1.0), theme());
        assert_eq!(view.title(), "Information tags");
        assert!(view.preview_edit_mode);
        assert!(view
            .preview_order
            .contains(view.preview_selected.as_ref().unwrap()));
        assert_eq!(
            view.preview_order
                .iter()
                .any(|id| id.as_str() == "tags.slot.kubernetes.page"),
            enabled
        );
        assert_eq!(
            view.preview_order
                .iter()
                .any(|id| id.as_str() == "tags.slot.terraform.page"),
            enabled
        );
        assert!(
            !view.preview_targets.iter().any(|(id, bounds)| {
                id.as_str() == "tags.slot.terraform.page"
                    && bounds.y < view.preview_tag_list_area.y
            }),
            "an individually disabled tag must not be re-enabled by the group toggle"
        );
        let saved = view
            .customizations
            .as_ref()
            .unwrap()
            .slot_pages
            .as_ref()
            .unwrap();
        assert_eq!(saved.preview_recipe(), recipe);
        assert_eq!(preferences.visual.information_bar.recipe(), recipe);
        for _ in 0..view.preview_order.len() + 1 {
            named(&mut view, NamedKey::Tab);
            assert_eq!(view.focus, Focus::Preview);
            assert!(view
                .preview_order
                .contains(view.preview_selected.as_ref().unwrap()));
        }
    }
}

#[test]
fn selected_tag_stays_in_preview_when_source_wraps_to_an_optional_role() {
    let base = rio_backend::config::Config::default();
    let original = crate::automexia::preferences::UserPreferences::default();
    let mut view = SettingsView::default();
    view.fit(960.0, 620.0, 16.0);
    view.open_customizations_with_slots(
        crate::settings_catalog::catalog(
            1,
            &base,
            &original,
            &crate::settings_catalog::test_installed_extensions(),
        )
        .unwrap(),
        None,
        Some(crate::settings_catalog::slot_page_snapshot(
            &original, &base,
        )),
    );
    assert!(view
        .view
        .as_mut()
        .unwrap()
        .focus(&SettingId::new("tags.enabled").unwrap()));
    view.focus = Focus::List;
    named(&mut view, NamedKey::Enter);
    view.start_preview_edit();
    view.paint(&mut Raster::new(1.0), theme());
    let windows = SettingId::new("tags.slot.windows.page").unwrap();
    view.enter_preview_item(windows.clone());
    assert_eq!(view.title(), "Tag slot: Windows");

    // The last text-source choice is icon-only. One more step wraps to the
    // optional Production role, which the ordinary preview sample omits.
    let changed = crate::settings_catalog::apply_edit(
        1,
        &base,
        &original,
        &crate::settings_catalog::test_installed_extensions(),
        &Edit {
            revision: 1,
            id: SettingId::new("tags.slot.windows.text").unwrap(),
            change: Change::Set(SettingValue::Choice("production".into())),
        },
    )
    .unwrap();
    view.refresh_with_resources(
        crate::settings_catalog::catalog(
            2,
            &base,
            &changed,
            &crate::settings_catalog::test_installed_extensions(),
        )
        .unwrap(),
        None,
        Some(crate::settings_catalog::slot_page_snapshot(
            &changed,
            &changed.apply_to(&base),
        )),
    );
    view.paint(&mut Raster::new(1.0), theme());
    assert_eq!(view.title(), "Tag slot: Windows");
    assert!(view.preview_targets.iter().any(|(id, _)| id == &windows));
}

#[test]
fn repurposed_tag_does_not_duplicate_its_default_source_in_the_live_preview() {
    let base = rio_backend::config::Config::default();
    let original = crate::automexia::preferences::UserPreferences::default();
    let changed = crate::settings_catalog::apply_edit(
        1,
        &base,
        &original,
        &crate::settings_catalog::test_installed_extensions(),
        &Edit {
            revision: 1,
            id: SettingId::new("tags.slot.windows.text").unwrap(),
            change: Change::Set(SettingValue::Choice("aws".into())),
        },
    )
    .unwrap();
    let mut view = SettingsView::default();
    view.fit(960.0, 620.0, 16.0);
    view.open_customizations_with_slots(
        crate::settings_catalog::catalog(
            2,
            &base,
            &changed,
            &crate::settings_catalog::test_installed_extensions(),
        )
        .unwrap(),
        None,
        Some(crate::settings_catalog::slot_page_snapshot(
            &changed,
            &changed.apply_to(&base),
        )),
    );
    assert!(view
        .view
        .as_mut()
        .unwrap()
        .focus(&SettingId::new("tags.enabled").unwrap()));
    view.focus = Focus::List;
    named(&mut view, NamedKey::Enter);
    view.enter_preview_item(SettingId::new("tags.slot.windows.page").unwrap());
    view.paint(&mut Raster::new(1.0), theme());
    let sample_ids: Vec<_> = view
        .preview_targets
        .iter()
        .filter(|(_, bounds)| bounds.y < view.preview_tag_list_area.y)
        .map(|(id, _)| id.as_str())
        .collect();
    assert_eq!(
        sample_ids
            .iter()
            .filter(|id| **id == "tags.slot.windows.page")
            .count(),
        1
    );
    assert!(!sample_ids.contains(&"tags.slot.aws.page"));
}

#[test]
fn tag_source_choice_keeps_its_editor_through_cloud_wsl_and_wrap_boundaries() {
    let base = rio_backend::config::Config::default();
    let source = SettingId::new("tags.slot.windows.text").unwrap();
    let page = SettingId::new("tags.slot.windows.page").unwrap();
    for (initial, expected, by_mouse) in [
        ("gcp", "unknown-cloud", true),
        ("production", "ubuntu-wsl", false),
        ("none", "production", false),
    ] {
        let original = crate::automexia::preferences::UserPreferences::default();
        let prepared = crate::settings_catalog::apply_edit(
            1,
            &base,
            &original,
            &crate::settings_catalog::test_installed_extensions(),
            &Edit {
                revision: 1,
                id: source.clone(),
                change: Change::Set(SettingValue::Choice(initial.into())),
            },
        )
        .unwrap();
        let mut view = SettingsView::default();
        view.fit(960.0, 620.0, 16.0);
        view.open_customizations_with_slots(
            crate::settings_catalog::catalog(
                2,
                &base,
                &prepared,
                &crate::settings_catalog::test_installed_extensions(),
            )
            .unwrap(),
            None,
            Some(crate::settings_catalog::slot_page_snapshot(
                &prepared,
                &prepared.apply_to(&base),
            )),
        );
        assert!(view
            .view
            .as_mut()
            .unwrap()
            .focus(&SettingId::new("tags.enabled").unwrap()));
        view.focus = Focus::List;
        named(&mut view, NamedKey::Enter);
        view.start_preview_edit();
        view.paint(&mut Raster::new(1.0), theme());
        view.enter_preview_item(page.clone());
        assert_eq!(view.title(), "Tag slot: Windows");
        assert!(view.view.as_mut().unwrap().focus(&source));
        view.focus = Focus::List;
        if by_mouse {
            view.paint(&mut Raster::new(1.0), theme());
            let control = view
                .rows
                .iter()
                .find(|row| row.id == source)
                .unwrap()
                .control;
            pointer_event(&mut view, control, 1.0, ElementState::Pressed);
            pointer_event(&mut view, control, 1.0, ElementState::Released);
        } else {
            named(&mut view, NamedKey::ArrowRight);
        }
        let edit = view.take_edit().expect("source change must be queued");
        assert_eq!(edit.id, source);
        assert_eq!(
            edit.change,
            Change::Set(SettingValue::Choice(expected.into()))
        );
        let changed = crate::settings_catalog::apply_edit(
            2,
            &base,
            &prepared,
            &crate::settings_catalog::test_installed_extensions(),
            &edit,
        )
        .unwrap();
        view.refresh_with_resources(
            crate::settings_catalog::catalog(
                3,
                &base,
                &changed,
                &crate::settings_catalog::test_installed_extensions(),
            )
            .unwrap(),
            None,
            Some(crate::settings_catalog::slot_page_snapshot(
                &changed,
                &changed.apply_to(&base),
            )),
        );
        view.paint(&mut Raster::new(1.0), theme());
        assert_eq!(view.title(), "Tag slot: Windows", "source {expected}");
        assert_eq!(
            view.customizations.as_ref().unwrap().active_slot.as_ref(),
            Some(&page)
        );
        assert_eq!(view.focus, Focus::List);
        assert!(view.preview_targets.iter().any(|(id, _)| id == &page));
        assert_eq!(
            view.catalog.as_ref().unwrap().get(&source).unwrap().value,
            SettingValue::Choice(expected.into())
        );
        assert!(view.view.as_mut().unwrap().focus(&source));
        named(&mut view, NamedKey::ArrowRight);
        assert_eq!(
            view.take_edit().unwrap().id,
            source,
            "the next arrow must still edit this tag, not its parent page"
        );
    }
}

#[test]
fn temporarily_incomplete_tag_color_catalog_keeps_editor_and_recovers() {
    let base = rio_backend::config::Config::default();
    let original = crate::automexia::preferences::UserPreferences::default();
    let page = SettingId::new("tags.slot.windows.page").unwrap();
    let source = SettingId::new("tags.slot.windows.text").unwrap();
    let changed = crate::settings_catalog::apply_edit(
        1,
        &base,
        &original,
        &crate::settings_catalog::test_installed_extensions(),
        &Edit {
            revision: 1,
            id: source.clone(),
            change: Change::Set(SettingValue::Choice("unknown-cloud".into())),
        },
    )
    .unwrap();
    let mut view = SettingsView::default();
    view.fit(960.0, 620.0, 16.0);
    view.open_customizations_with_slots(
        crate::settings_catalog::catalog(
            1,
            &base,
            &original,
            &crate::settings_catalog::test_installed_extensions(),
        )
        .unwrap(),
        None,
        Some(crate::settings_catalog::slot_page_snapshot(
            &original, &base,
        )),
    );
    assert!(view
        .view
        .as_mut()
        .unwrap()
        .focus(&SettingId::new("tags.enabled").unwrap()));
    view.focus = Focus::List;
    named(&mut view, NamedKey::Enter);
    view.start_preview_edit();
    view.paint(&mut Raster::new(1.0), theme());
    view.enter_preview_item(page.clone());

    let complete = crate::settings_catalog::catalog(
        2,
        &base,
        &changed,
        &crate::settings_catalog::test_installed_extensions(),
    )
    .unwrap();
    let incomplete = Catalog::new(
        2,
        complete
            .entries()
            .iter()
            .filter(|entry| entry.id.as_str() != "tags.colors.unknown_cloud")
            .cloned()
            .collect(),
    )
    .unwrap();
    let snapshot =
        crate::settings_catalog::slot_page_snapshot(&changed, &changed.apply_to(&base));
    view.refresh_with_resources(incomplete, None, Some(snapshot));
    assert_eq!(view.title(), "Tag slot: Windows");
    assert_eq!(
        view.customizations.as_ref().unwrap().active_slot.as_ref(),
        Some(&page)
    );
    assert!(view.status.contains("unavailable"));
    named(&mut view, NamedKey::ArrowRight);
    assert!(
        view.take_edit().is_none(),
        "stale controls must not accept edits"
    );

    view.refresh_with_resources(
        crate::settings_catalog::catalog(
            3,
            &base,
            &changed,
            &crate::settings_catalog::test_installed_extensions(),
        )
        .unwrap(),
        None,
        Some(crate::settings_catalog::slot_page_snapshot(
            &changed,
            &changed.apply_to(&base),
        )),
    );
    assert_eq!(view.title(), "Tag slot: Windows");
    assert!(!view.status.contains("unavailable"));
    assert_eq!(
        view.catalog.as_ref().unwrap().get(&source).unwrap().value,
        SettingValue::Choice("unknown-cloud".into())
    );
}

#[test]
fn inherited_color_graphic_uses_role_labels_for_canonical_setting_ids() {
    assert_eq!(
        tag_color_graphic_name("tags.colors.unknown_cloud"),
        "Other cloud"
    );
    assert_eq!(
        tag_color_graphic_name("tags.colors.ubuntu_wsl"),
        "Ubuntu / WSL"
    );
    assert_eq!(
        tag_color_graphic_name("tags.slot.unknown-cloud.color"),
        "Other cloud"
    );
    assert_eq!(
        tag_color_graphic_name("tags.slot.custom-1.color"),
        "custom 1"
    );
}

#[test]
fn selected_tag_controls_and_live_preview_remain_visible_at_wide_and_narrow_sizes() {
    let base = rio_backend::config::Config::default();
    let preferences = crate::automexia::preferences::UserPreferences::default();
    for (width, height) in [(960.0, 620.0), (320.0, 420.0)] {
        let mut view = SettingsView::default();
        view.fit(width, height, 16.0);
        view.open_customizations_with_slots(
            crate::settings_catalog::catalog(1, &base, &preferences, &[]).unwrap(),
            None,
            Some(crate::settings_catalog::slot_page_snapshot(
                &preferences,
                &base,
            )),
        );
        assert!(view
            .view
            .as_mut()
            .unwrap()
            .focus(&SettingId::new("tags.enabled").unwrap()));
        view.focus = Focus::List;
        named(&mut view, NamedKey::Enter);
        view.paint(&mut Raster::new(1.0), theme());
        view.start_preview_edit();
        view.paint(&mut Raster::new(1.0), theme());
        let (selected, bounds) = view.preview_targets.first().cloned().unwrap();
        pointer_event(&mut view, bounds, 1.0, ElementState::Pressed);
        pointer_event(&mut view, bounds, 1.0, ElementState::Released);
        assert_eq!(
            view.customizations.as_ref().unwrap().active_slot,
            Some(selected)
        );
        let mut raster = Raster::new(1.0);
        view.paint(&mut raster, theme());
        assert!(!view.rows.is_empty());
        assert!(!view.preview_targets.is_empty());
        assert!(view
            .geometry
            .body
            .intersect(view.geometry.preview)
            .is_none());
        if let Some(directory) = std::env::var_os("AUTOMEXIA_SETTINGS_PREVIEW_DIR") {
            let directory = std::path::PathBuf::from(directory);
            std::fs::create_dir_all(&directory).unwrap();
            let pixels = raster.pixels(width as u32, height as u32, true);
            image_rs::RgbImage::from_fn(width as u32, height as u32, |x, y| {
                let pixel = pixels[(y * width as u32 + x) as usize];
                image_rs::Rgb([(pixel >> 16) as u8, (pixel >> 8) as u8, pixel as u8])
            })
            .save(directory.join(format!(
                "tag-selected-{}x{}.png",
                width as u32, height as u32
            )))
            .unwrap();
        }
    }
}

#[test]
fn selected_preview_tag_has_a_visible_border_on_all_sides() {
    let base = rio_backend::config::Config::default();
    let preferences = crate::automexia::preferences::UserPreferences::default();
    for (width, height) in [(960_u32, 620_u32), (320, 420)] {
        let mut view = SettingsView::default();
        view.fit(width as f32, height as f32, 16.0);
        view.open_customizations_with_slots(
            crate::settings_catalog::catalog(1, &base, &preferences, &[]).unwrap(),
            None,
            Some(crate::settings_catalog::slot_page_snapshot(
                &preferences,
                &base,
            )),
        );
        assert!(view
            .view
            .as_mut()
            .unwrap()
            .focus(&SettingId::new("tags.enabled").unwrap()));
        view.focus = Focus::List;
        named(&mut view, NamedKey::Enter);
        view.paint(&mut Raster::new(1.0), theme());
        view.start_preview_edit();
        view.paint(&mut Raster::new(1.0), theme());
        let id = view.preview_targets.first().unwrap().0.clone();
        view.preview_selected = Some(id.clone());
        let mut selected = Raster::new(1.0);
        view.paint(&mut selected, theme());
        let bounds = view
            .preview_targets
            .iter()
            .find(|(candidate, _)| candidate == &id)
            .unwrap()
            .1;
        assert_eq!(view.preview_selected.as_ref(), Some(&id));
        assert!(bounds.height > 8.0 && bounds.width > 8.0);
        let other = if id.as_str() == "tags.slot.user.page" {
            SettingId::new("tags.slot.windows.page").unwrap()
        } else {
            SettingId::new("tags.slot.user.page").unwrap()
        };
        view.preview_selected = Some(other);
        let mut plain = Raster::new(1.0);
        view.paint(&mut plain, theme());
        let plain = plain.pixels(width, height, true);
        let selected = selected.pixels(width, height, true);
        let left_x = (bounds.x + 1.0).round() as usize;
        let top_y = (bounds.y + 3.0).ceil() as usize;
        let bottom_y = (bounds.y + bounds.height - 3.0).floor() as usize;
        let changed_left = (top_y..bottom_y)
            .filter(|y| {
                plain[y * width as usize + left_x]
                    != selected[y * width as usize + left_x]
            })
            .count();
        assert!(
            changed_left * 2 >= bottom_y - top_y,
            "left border is too subtle"
        );
        let right_x = (bounds.x + bounds.width - 2.0).floor() as usize;
        let changed_right = (top_y..bottom_y)
            .filter(|y| {
                plain[y * width as usize + right_x]
                    != selected[y * width as usize + right_x]
            })
            .count();
        assert!(
            changed_right * 2 >= bottom_y - top_y,
            "right border is too subtle: {width}x{height}, {bounds:?}, {changed_right} of {}", bottom_y - top_y
        );
        let bottom_y = (bounds.y + bounds.height - 1.0).floor() as usize;
        let left_x = (bounds.x + 4.0).ceil() as usize;
        let right_x = (bounds.x + bounds.width - 4.0).floor() as usize;
        let changed_bottom = (left_x..right_x)
            .filter(|x| {
                plain[bottom_y * width as usize + x]
                    != selected[bottom_y * width as usize + x]
            })
            .count();
        assert!(
            changed_bottom * 2 >= right_x - left_x,
            "bottom border is too subtle"
        );
    }
}

#[test]
fn timestamp_preview_uses_the_runtime_status_duration_separator_spacing() {
    assert_eq!(
        timestamp_preview_label(true),
        "✓  104ms  ·  2026-09-30 12:34:56"
    );
    assert_eq!(timestamp_preview_label(false), "✓  104ms");
}

#[test]
fn timestamp_menu_edits_refresh_preview_keep_focus_and_confirm_resets() {
    use rio_backend::config::presentation::*;
    let base = rio_backend::config::Config::default();
    let mut original = crate::automexia::preferences::UserPreferences::default();
    original.visual.timestamps.date_position = Some(TimestampPosition::AboveLeft);
    original.visual.timestamps.time_position = Some(TimestampPosition::BelowRight);
    original.visual.timestamps.date_color = Some(Rgb::from_bytes([255, 190, 70]));
    original.visual.timestamps.time_color = Some(Rgb::from_bytes([30, 210, 255]));
    original.visual.timestamps.background = Some(Rgba::from_bytes([70, 90, 110, 90]));
    for (width, height, scale) in [
        (960.0, 620.0, 1.0),
        (640.0, 480.0, 1.25),
        (1280.0, 800.0, 2.0),
    ] {
        let mut view = SettingsView::default();
        view.fit(width, height, 16.0);
        view.open_customizations_with_slots(
            crate::settings_catalog::catalog(1, &base, &original, &[]).unwrap(),
            None,
            Some(crate::settings_catalog::slot_page_snapshot_with_config(
                &original,
                &original.apply_to(&base),
                &base,
                &crate::settings_catalog::test_installed_extensions(),
            )),
        );
        assert!(view
            .view
            .as_mut()
            .unwrap()
            .focus(&SettingId::new(COMMAND_TIMESTAMPS).unwrap()));
        view.focus = Focus::List;
        named(&mut view, NamedKey::Enter);
        assert_eq!(view.title(), "Command timestamps");
        assert_eq!(view.catalog.as_ref().unwrap().entries().len(), 23);
        // All three parts occupy different rows: joining controls have no effect.
        for hidden in [
            "timestamps.order",
            "timestamps.separator",
            "timestamps.date-time-separator",
        ] {
            assert!(view
                .catalog
                .as_ref()
                .unwrap()
                .get(&SettingId::new(hidden).unwrap())
                .is_none());
        }
        let id = SettingId::new("timestamps.time-format").unwrap();
        assert!(view.view.as_mut().unwrap().focus(&id));
        view.focus = Focus::List;
        named(&mut view, NamedKey::ArrowRight);
        let edit = view.take_edit().unwrap();
        assert_eq!(edit.id, id);
        assert_eq!(
            edit.change,
            Change::Set(SettingValue::Choice("12-hour".into()))
        );
        let changed =
            crate::settings_catalog::apply_edit(1, &base, &original, &[], &edit).unwrap();
        view.refresh_with_resources(
            crate::settings_catalog::catalog(2, &base, &changed, &[]).unwrap(),
            None,
            Some(crate::settings_catalog::slot_page_snapshot_with_config(
                &changed,
                &changed.apply_to(&base),
                &base,
                &crate::settings_catalog::test_installed_extensions(),
            )),
        );
        assert_eq!(view.title(), "Command timestamps");
        assert!(timestamp_preview::sample_text(
            changed.visual.timestamps,
            true,
            Some(0),
            Some(104)
        )
        .text
        .contains("PM"));
        let mut raster = Raster::new(scale);
        view.paint(&mut raster, theme());
        let (w, h) = ((width * scale) as u32, (height * scale) as u32);
        let pixels = raster.pixels(w, h, true);
        if let Some(directory) = std::env::var_os("AUTOMEXIA_SETTINGS_PREVIEW_DIR") {
            let directory = std::path::PathBuf::from(directory);
            std::fs::create_dir_all(&directory).unwrap();
            image_rs::RgbImage::from_fn(w, h, |x, y| {
                let pixel = pixels[(y * w + x) as usize];
                image_rs::Rgb([(pixel >> 16) as u8, (pixel >> 8) as u8, pixel as u8])
            })
            .save(directory.join(format!("timestamps-{w}x{h}.png")))
            .unwrap();
        }
        view.activate_target(Target::Reset);
        assert!(view.confirmation.is_some());
        named(&mut view, NamedKey::Escape);
        assert!(view.take_customization_intent().is_none());
        view.activate_target(Target::Reset);
        confirm_requested_settings_action(&mut view);
        assert_eq!(
            view.take_customization_intent(),
            Some(CustomizationIntent::Reset {
                revision: 2,
                scope: CustomizationResetScope::Group(
                    SettingId::new(COMMAND_TIMESTAMPS).unwrap()
                )
            })
        );
        view.set_temporary_customizations(true);
        view.activate_target(Target::Restore);
        confirm_requested_settings_action(&mut view);
        assert_eq!(
            view.take_customization_intent(),
            Some(CustomizationIntent::RestoreSaved)
        );
        named(&mut view, NamedKey::Escape);
        assert_eq!(view.title(), "Workflow & Output");
    }
}

#[test]
fn timestamp_preview_paints_the_effective_terminal_success_accent() {
    let mut base = rio_backend::config::Config::default();
    base.colors.green = [1.0, 0.0, 1.0, 1.0];
    let preferences = crate::automexia::preferences::UserPreferences::default();
    let snapshot = crate::settings_catalog::slot_page_snapshot(&preferences, &base);
    assert_eq!(snapshot.preview_timestamps().1.green, base.colors.green);
    let mut view = SettingsView::default();
    view.fit(960.0, 620.0, 16.0);
    view.open_customizations_with_slots(
        crate::settings_catalog::catalog(1, &base, &preferences, &[]).unwrap(),
        None,
        Some(snapshot),
    );
    assert!(view
        .view
        .as_mut()
        .unwrap()
        .focus(&SettingId::new(COMMAND_TIMESTAMPS).unwrap()));
    view.focus = Focus::List;
    named(&mut view, NamedKey::Enter);
    let mut raster = Raster::new(1.0);
    view.paint(&mut raster, theme());
    let preview = view.geometry.preview;
    let pixels = raster.pixels(960, 620, true);
    let accent_visible = (preview.y.max(0.0) as u32
        ..(preview.y + preview.height).min(620.0) as u32)
        .any(|y| {
            (preview.x.max(0.0) as u32..(preview.x + preview.width).min(960.0) as u32)
                .any(|x| {
                    let pixel = pixels[(y * 960 + x) as usize];
                    ((pixel >> 16) & 0xff) > 80
                        && (pixel & 0xff) > 80
                        && ((pixel >> 8) & 0xff) < 80
                })
        });
    assert!(
        accent_visible,
        "sample glyphs use the effective terminal success accent"
    );
}

#[test]
fn customization_preview_splits_wide_sheets_and_stacks_on_narrow_sheets() {
    let base = rio_backend::config::Config::default();
    let source =
        crate::settings_catalog::catalog(1, &base, &Default::default(), &[]).unwrap();
    for (width, height, font, side_by_side) in
        [(960.0, 620.0, 16.0, true), (320.0, 420.0, 18.0, false)]
    {
        let mut view = SettingsView::default();
        view.fit(width, height, font);
        view.open_with_section(source.clone(), Some(Section::Customizations));
        let key =
            SettingId::new(automexia_ui_model::settings::COMMAND_OUTPUT_HIGHLIGHTING)
                .unwrap();
        assert!(view.view.as_mut().unwrap().focus(&key));
        view.focus = Focus::List;
        named(&mut view, NamedKey::Enter);
        let mut raster = Raster::new(1.0);
        view.paint(&mut raster, theme());
        let g = view.geometry;
        assert!(g.preview.width > 0.0 && g.preview.height > 0.0);
        assert!(g.body.width > 0.0 && g.body.height > 0.0);
        assert!(g.body.intersect(g.preview).is_none());
        if side_by_side {
            assert!(g.preview.x >= g.body.x + g.body.width);
            assert!(g.preview.width >= g.body.width * 0.8);
        } else {
            assert!(g.preview.y >= g.body.y + g.body.height);
        }
        for ([x, y, w, h], _) in &raster.rects {
            assert!(
                *x >= 0.0
                    && *y >= 0.0
                    && x + w <= width + 0.001
                    && y + h <= height + 0.001
            );
        }
        if let Some(directory) = std::env::var_os("AUTOMEXIA_SETTINGS_PREVIEW_DIR") {
            let directory = std::path::PathBuf::from(directory);
            std::fs::create_dir_all(&directory).unwrap();
            let pixels = raster.pixels(width as u32, height as u32, true);
            image_rs::RgbImage::from_fn(width as u32, height as u32, |x, y| {
                let pixel = pixels[(y * width as u32 + x) as usize];
                image_rs::Rgb([(pixel >> 16) as u8, (pixel >> 8) as u8, pixel as u8])
            })
            .save(directory.join(format!(
                "customization-preview-{}x{}.png",
                width as u32, height as u32
            )))
            .unwrap();
        }
    }
}

#[test]
fn information_tag_preview_repaints_spacing_after_saved_edit_without_leaving_the_page() {
    let base = rio_backend::config::Config::default();
    let original = crate::automexia::preferences::UserPreferences::default();
    let source = crate::settings_catalog::catalog(1, &base, &original, &[]).unwrap();
    let mut view = SettingsView::default();
    view.fit(960.0, 620.0, 16.0);
    view.open_customizations_with_slots(
        source,
        None,
        Some(crate::settings_catalog::slot_page_snapshot(
            &original, &base,
        )),
    );
    assert!(view
        .view
        .as_mut()
        .unwrap()
        .focus(&SettingId::new("tags.enabled").unwrap()));
    view.focus = Focus::List;
    named(&mut view, NamedKey::Enter);
    let mut before = Raster::new(1.0);
    view.paint(&mut before, theme());
    if let Some(directory) = std::env::var_os("AUTOMEXIA_SETTINGS_PREVIEW_DIR") {
        let directory = std::path::PathBuf::from(directory);
        std::fs::create_dir_all(&directory).unwrap();
        let pixels = before.pixels(960, 620, true);
        image_rs::RgbImage::from_fn(960, 620, |x, y| {
            let pixel = pixels[(y * 960 + x) as usize];
            image_rs::Rgb([(pixel >> 16) as u8, (pixel >> 8) as u8, pixel as u8])
        })
        .save(directory.join("information-tags-preview-960x620.png"))
        .unwrap();
    }
    let user_left = |view: &SettingsView| -> f32 {
        view.preview_targets
            .iter()
            .find(|(id, bounds)| {
                id.as_str() == "tags.slot.user.page"
                    && bounds.y < view.preview_tag_list_area.y
            })
            .map(|(_, bounds)| bounds.x)
            .expect("the user sample tag must be visible")
    };
    let first = user_left(&view);
    let saved = crate::settings_catalog::apply_edit(
        1,
        &base,
        &original,
        &[],
        &Edit {
            revision: 1,
            id: SettingId::new("tags.spacing").unwrap(),
            change: Change::Set(SettingValue::Number(300.0)),
        },
    )
    .unwrap();
    view.refresh_with_resources(
        crate::settings_catalog::catalog(2, &base, &saved, &[]).unwrap(),
        None,
        Some(crate::settings_catalog::slot_page_snapshot(
            &saved,
            &saved.apply_to(&base),
        )),
    );
    let mut after = Raster::new(1.0);
    view.paint(&mut after, theme());
    let second = user_left(&view);
    assert_eq!(view.title(), "Information tags");
    assert!(
        second > first,
        "larger saved spacing moves the next tag in-place: before={first:?}, after={second:?}"
    );
    let compact = crate::settings_catalog::apply_edit(
        1,
        &base,
        &original,
        &[],
        &Edit {
            revision: 1,
            id: SettingId::new("tags.spacing").unwrap(),
            change: Change::Set(SettingValue::Number(0.0)),
        },
    )
    .unwrap();
    view.refresh_with_resources(
        crate::settings_catalog::catalog(3, &base, &compact, &[]).unwrap(),
        None,
        Some(crate::settings_catalog::slot_page_snapshot(
            &compact,
            &compact.apply_to(&base),
        )),
    );
    let mut zero = Raster::new(1.0);
    view.paint(&mut zero, theme());
    let compact_left = user_left(&view);
    assert!(compact_left < first, "zero spacing packs tags closer");
    for raster in [&before, &after, &zero] {
        for ([x, y, width, height], _) in &raster.rects {
            assert!(*x >= 0.0 && *y >= 0.0 && x + width <= 960.0 && y + height <= 620.0);
        }
    }
}

#[test]
fn customization_helper_copy_is_smaller_than_controls_without_changing_hit_targets() {
    let base = rio_backend::config::Config::default();
    let source =
        crate::settings_catalog::catalog(1, &base, &Default::default(), &[]).unwrap();
    let mut view = SettingsView::default();
    view.fit(800.0, 560.0, 24.0);
    view.open_with_section(source, Some(Section::Customizations));
    assert!(view
        .view
        .as_mut()
        .unwrap()
        .focus(&SettingId::new("tags.enabled").unwrap()));
    view.focus = Focus::List;
    named(&mut view, NamedKey::Enter);
    let mut raster = Raster::new(1.0);
    view.paint(&mut raster, theme());
    let row = view
        .rows
        .iter()
        .find(|row| row.id.as_str() == "tags.enabled")
        .unwrap();
    assert!(row.help_line < view.font * 1.45);
    assert!(row.control.y >= row.bounds.y + view.font * 1.45);
    let center = (
        row.control.x + row.control.width * 0.5,
        row.control.y + row.control.height * 0.5,
    );
    assert_eq!(
        view.target_at(center.0, center.1),
        Some(Target::Control(row.id.clone(), 1))
    );
}

#[test]
fn menu_polish_navigation_rows_are_compact_whole_row_links() {
    let mut view = workflow_tag_view();
    named(&mut view, NamedKey::Escape);
    view.fit(960.0, 620.0, 16.0);
    view.paint(&mut Raster::new(1.0), theme());
    let row = &view.rows[0];
    assert!(
        row.bounds.height <= 90.0,
        "navigation needs no separate Open field"
    );
    assert!(
        row.control.width <= 48.0,
        "navigation uses a trailing chevron"
    );
    let target = view.target_at(row.bounds.x + 16.0, row.bounds.y + 16.0);
    assert_eq!(target, Some(Target::Control(row.id.clone(), 1)));
    view.activate_target(target.unwrap());
    assert_eq!(view.title(), "Information tags");
}

#[test]
fn menu_polish_page_keys_follow_the_visible_compact_rows() {
    let mut view = workflow_tag_view();
    named(&mut view, NamedKey::Escape);
    view.fit(960.0, 620.0, 16.0);
    view.paint(&mut Raster::new(1.0), theme());
    named(&mut view, NamedKey::Home);
    named(&mut view, NamedKey::PageDown);
    assert_eq!(
        view.view.as_ref().unwrap().focused().unwrap().as_str(),
        "profiles.open",
        "a page should advance past five rows with the two-line shortcut footer"
    );
}

#[test]
fn menu_polish_controls_fit_beside_labels_and_stack_on_narrow_panes() {
    for (width, height) in [(960.0, 620.0), (320.0, 420.0)] {
        let mut view = workflow_tag_view();
        view.fit(width, height, 16.0);
        view.paint(&mut Raster::new(1.0), theme());
        let row = view
            .rows
            .iter()
            .find(|row| row.id.as_str() == "tags.enabled")
            .unwrap();
        if width > 600.0 {
            assert!(row.control.x > row.bounds.x + row.bounds.width * 0.5);
            assert!(row.bounds.height < 100.0);
        }
        assert!(row.control.x >= row.bounds.x);
        assert!(row.control.x + row.control.width <= row.bounds.x + row.bounds.width);
        assert!(row.control.y + row.control.height <= row.bounds.y + row.bounds.height);
        assert_eq!(
            view.target_at(
                row.control.x + row.control.width * 0.5,
                row.control.y + row.control.height * 0.5
            ),
            Some(Target::Control(row.id.clone(), 1))
        );
    }
}

#[test]
fn menu_polish_confirmation_sizes_to_content_and_keeps_safe_actions() {
    let mut view = workflow_tag_view();
    view.fit(960.0, 620.0, 16.0);
    view.activate_target(Target::Reset);
    let mut raster = Raster::new(1.0);
    view.paint(&mut raster, theme());
    let g = view.confirmation_geometry;
    assert!(
        g.card.height <= 210.0,
        "a short confirmation must not leave a blank body"
    );
    assert!(g.cancel.x + g.cancel.width < g.accept.x);
    assert!(g.cancel.height >= 32.0 && g.accept.height >= 32.0);
    assert!(!view.confirmation.as_ref().unwrap().accept_selected);
    named(&mut view, NamedKey::Enter);
    assert!(view.confirmation.is_none());
    assert!(view.take_customization_intent().is_none());
}

#[test]
fn menu_polish_fractional_controls_keep_rounding_without_escaping_clip() {
    let bounds = Rect {
        x: 100.25,
        y: 100.25,
        width: 32.1,
        height: 32.1,
    };
    let clip = Rect {
        x: 0.0,
        y: 0.0,
        width: 500.0,
        height: 500.0,
    };
    let mut raster = Raster::new(1.5);
    control(&mut raster, bounds, false, theme(), clip);
    assert_eq!(
        raster
            .shapes
            .iter()
            .filter(|shape| matches!(shape, ShapeCall::Rounded(..)))
            .count(),
        2,
        "fully contained fractional controls must keep both rounded layers"
    );
    let mut clipped = Raster::new(1.5);
    let viewport = Rect {
        y: 110.0,
        height: 8.0,
        ..clip
    };
    control(&mut clipped, bounds, true, theme(), viewport);
    assert!(!clipped.rects.is_empty());
    assert!(clipped
        .rects
        .iter()
        .all(|([_, y, _, h], _)| *y >= 110.0 && y + h <= 118.0));
}

#[test]
fn output_preview_uses_the_saved_background_color_on_the_same_page() {
    let base = rio_backend::config::Config::default();
    let original = crate::automexia::preferences::UserPreferences::default();
    let mut view = SettingsView::default();
    view.fit(960.0, 620.0, 16.0);
    view.open_with_section(
        crate::settings_catalog::catalog(1, &base, &original, &[]).unwrap(),
        Some(Section::Customizations),
    );
    let category =
        SettingId::new(automexia_ui_model::settings::COMMAND_OUTPUT_HIGHLIGHTING)
            .unwrap();
    assert!(view.view.as_mut().unwrap().focus(&category));
    view.focus = Focus::List;
    named(&mut view, NamedKey::Enter);
    let mut before = Raster::new(1.0);
    view.paint(&mut before, theme());
    let changed = crate::settings_catalog::apply_edit(
        1,
        &base,
        &original,
        &[],
        &Edit {
            revision: 1,
            id: SettingId::new("output.backgrounds.error").unwrap(),
            change: Change::Set(SettingValue::Color([10, 200, 30, 180])),
        },
    )
    .unwrap();
    view.refresh_with_resources(
        crate::settings_catalog::catalog(2, &base, &changed, &[]).unwrap(),
        None,
        None,
    );
    let mut after = Raster::new(1.0);
    view.paint(&mut after, theme());
    assert_eq!(view.title(), "Terminal output colors");
    let expected = [10.0 / 255.0, 200.0 / 255.0, 30.0 / 255.0, 180.0 / 255.0];
    assert!(after.rects.iter().any(|(bounds, color)| {
        bounds[0] >= view.geometry.preview.x
            && bounds[1] >= view.geometry.preview.y
            && color
                .iter()
                .zip(expected)
                .all(|(actual, expected)| (actual - expected).abs() < 0.001)
    }));
    assert_ne!(before.rects, after.rects);
}

#[test]
fn output_color_categories_offer_separate_keyboard_and_pointer_preview_editors() {
    let base = rio_backend::config::Config::default();
    let saved = crate::automexia::preferences::UserPreferences::default();
    let mut view = SettingsView::default();
    view.fit(960.0, 620.0, 16.0);
    view.open_customizations_with_slots(
        crate::settings_catalog::catalog(1, &base, &saved, &[]).unwrap(),
        None,
        Some(crate::settings_catalog::slot_page_snapshot(&saved, &base)),
    );
    assert!(view
        .view
        .as_mut()
        .unwrap()
        .focus(&SettingId::new("terminal.command_output_highlighting").unwrap()));
    view.focus = Focus::List;
    named(&mut view, NamedKey::Enter);
    assert_eq!(view.title(), "Terminal output colors");
    view.paint(&mut Raster::new(1.0), theme());
    assert!(view
        .preview_items()
        .iter()
        .any(|(id, _)| { id.as_str() == "command_output.band.success" }));
    assert!(view
        .preview_items()
        .iter()
        .any(|(id, _)| { id.as_str() == "output.severity.error" }));
    view.start_preview_edit();
    view.preview_selected = Some(SettingId::new("command_output.band.success").unwrap());
    named(&mut view, NamedKey::Enter);
    assert_eq!(
        view.customizations
            .as_ref()
            .unwrap()
            .active_slot
            .as_ref()
            .unwrap()
            .as_str(),
        "command_output.band.success"
    );
    named(&mut view, NamedKey::Escape);
    assert_eq!(view.title(), "Terminal output colors");
    assert!(!view.preview_edit_mode);
    named(&mut view, NamedKey::Escape);
    assert_eq!(view.title(), "Workflow & Output");

    assert!(view
        .view
        .as_mut()
        .unwrap()
        .focus(&SettingId::new("terminal.kubernetes_highlighting").unwrap()));
    view.focus = Focus::List;
    named(&mut view, NamedKey::Enter);
    assert_eq!(view.title(), "Kubernetes status colors");
    view.paint(&mut Raster::new(1.0), theme());
    view.start_preview_edit();
    view.paint(&mut Raster::new(1.0), theme());
    let warning = view
        .preview_targets
        .iter()
        .find(|(id, _)| id.as_str() == "kubernetes.severity.warning")
        .unwrap()
        .1;
    pointer_event(&mut view, warning, 1.0, ElementState::Pressed);
    pointer_event(&mut view, warning, 1.0, ElementState::Released);
    assert_eq!(
        view.customizations
            .as_ref()
            .unwrap()
            .active_slot
            .as_ref()
            .unwrap()
            .as_str(),
        "kubernetes.severity.warning"
    );
    view.focus = Focus::Reset;
    named(&mut view, NamedKey::Enter);
    confirm_requested_settings_action(&mut view);
    assert_eq!(
        view.take_customization_intent(),
        Some(CustomizationIntent::Reset {
            revision: 1,
            scope: CustomizationResetScope::KubernetesSeverity("warning".into()),
        })
    );
    named(&mut view, NamedKey::Escape);
    assert_eq!(view.title(), "Kubernetes status colors");
    view.focus = Focus::Reset;
    named(&mut view, NamedKey::Enter);
    confirm_requested_settings_action(&mut view);
    assert_eq!(
        view.take_customization_intent(),
        Some(CustomizationIntent::Reset {
            revision: 1,
            scope: CustomizationResetScope::Group(
                SettingId::new("terminal.kubernetes_highlighting").unwrap()
            ),
        })
    );
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
fn keyboard_reveal_exposes_entire_information_tag_row() {
    let mut view = workflow_tag_view();
    view.fit(600.0, 900.0, 18.0);
    let mut raster = Raster::new(1.0);
    named(&mut view, NamedKey::End);
    view.paint(&mut raster, theme());
    let row = view.rows.last().unwrap();
    assert_eq!(row.id.as_str(), "tags.opacity");
    assert!(row.bounds.height <= view.geometry.body.height);
    assert!(
        row.bounds.y + row.bounds.height
            <= view.geometry.body.y + view.geometry.body.height + 0.01,
        "keyboard focus reveals the complete row, not just its control: {:?} in {:?}",
        row.bounds,
        view.geometry.body
    );
    if let Some(directory) = std::env::var_os("AUTOMEXIA_SETTINGS_PREVIEW_DIR") {
        let directory = std::path::PathBuf::from(directory);
        std::fs::create_dir_all(&directory).unwrap();
        let pixels = raster.pixels(600, 900, true);
        image_rs::RgbImage::from_fn(600, 900, |x, y| {
            let pixel = pixels[(y * 600 + x) as usize];
            image_rs::Rgb([(pixel >> 16) as u8, (pixel >> 8) as u8, pixel as u8])
        })
        .save(directory.join("keyboard-reveal-information-tags.png"))
        .unwrap();
    }
}

#[test]
fn keyboard_reveal_font_picker_last_row_and_resize_remain_wholly_visible() {
    let mut view = installed_font_picker();
    view.font_picker_inventory(
        (0..48).map(|i| format!("Example Mono {i:02}")).collect(),
        false,
    );
    named(&mut view, NamedKey::ArrowDown);
    named(&mut view, NamedKey::End);
    for (width, height, font, scale) in [
        (960.0, 620.0, 16.0, 1.0),
        (480.0, 560.0, 16.0, 1.25),
        (900.0, 700.0, 24.0, 2.0),
    ] {
        view.fit(width, height, font);
        let mut raster = Raster::new(scale);
        view.paint(&mut raster, theme());
        let picker = view.font_picker.as_ref().unwrap();
        let (_, row) = picker
            .targets
            .iter()
            .find(|(target, _)| *target == FontPickerTarget::Row(picker.selected))
            .unwrap();
        assert!(
            row.y >= view.geometry.body.y
                && row.y + row.height
                    <= view.geometry.body.y + view.geometry.body.height + 0.01
        );
        assert_eq!(view.font_picker_selection(), Some("Example Mono 47"));
    }
}

fn assert_keyboard_reveal(view: &SettingsView) {
    let id = view.view.as_ref().unwrap().focused().unwrap();
    let row = view.rows.iter().find(|row| &row.id == id).unwrap();
    let body = view.geometry.body;
    let required = if row.bounds.height <= body.height {
        row.bounds
    } else {
        row.control
    };
    // If even the input is taller than the viewport, its trailing portion
    // fills the viewport. Complete containment is physically impossible.
    let required = if required.height > body.height {
        Rect {
            y: required.y + required.height - body.height,
            height: body.height,
            ..required
        }
    } else {
        required
    };
    assert!(
        required.y >= body.y - 0.01
            && required.y + required.height <= body.y + body.height + 0.01,
        "{}: {} must be completely visible: {required:?} in {body:?}",
        view.title(),
        id.as_str()
    );
    assert!(
        view.scroll >= 0.0
            && view.scroll <= (view.content_height - body.height).max(0.0) + 0.01
    );
}

#[test]
fn keyboard_reveal_all_settings_pages_arrows_paging_resize_and_mouse_return() {
    let base = rio_backend::config::Config::default();
    let preferences = crate::automexia::preferences::UserPreferences::default();
    let market = crate::settings_catalog::test_installed_extensions();
    let full = crate::settings_catalog::catalog(1, &base, &preferences, &market).unwrap();
    let mut pages = vec![None]; // The general Settings list includes extension controls.
    pages.extend(
        customization_groups(&full)
            .into_iter()
            .map(|group| Some(group.key)),
    );
    let mut exercised = 0;
    for (width, height, font) in [
        (600.0, 900.0, 18.0),
        (960.0, 620.0, 16.0),
        (1280.0, 900.0, 18.0),
        (420.0, 600.0, 20.0),
        (350.0, 380.0, 22.0),
    ] {
        for page in &pages {
            let mut view = SettingsView::default();
            view.fit(width, height, font);
            if let Some(page) = page {
                view.open_customizations_with_slots(
                    full.clone(),
                    None,
                    Some(crate::settings_catalog::slot_page_snapshot_with_config(
                        &preferences,
                        &base,
                        &base,
                        &market,
                    )),
                );
                if page.as_str().starts_with("interface.")
                    || matches!(
                        page.as_str(),
                        crate::settings_catalog::WINDOW_CONTROLS
                            | automexia_ui_model::settings::FONT_SIZE
                            | automexia_ui_model::settings::APPEARANCE_THEME
                    )
                {
                    view.show_terminal_appearance();
                }
                assert!(view.view.as_mut().unwrap().focus(page));
                view.focus = Focus::List;
                named(&mut view, NamedKey::Enter);
            } else {
                view.open(full.clone());
                named(&mut view, NamedKey::Tab);
            }
            // Theme uses its own picker, checked independently below.
            if view.gallery.is_some() {
                continue;
            }
            let mut raster = Raster::new(1.25);
            let count = view.view.as_ref().unwrap().filtered_ids().len();
            assert!(count > 0);
            for key in [NamedKey::ArrowDown, NamedKey::ArrowUp] {
                for _ in 0..count + 2 {
                    named(&mut view, key);
                    view.prepare(&mut raster.text);
                    assert_keyboard_reveal(&view);
                    exercised += 1;
                }
            }
            for key in [
                NamedKey::End,
                NamedKey::Home,
                NamedKey::PageDown,
                NamedKey::PageUp,
            ] {
                named(&mut view, key);
                view.prepare(&mut raster.text);
                assert_keyboard_reveal(&view);
            }
            named(&mut view, NamedKey::End);
            view.prepare(&mut raster.text);
            let last = view.view.as_ref().unwrap().focused().cloned();
            view.scroll_by(-100_000.0);
            view.prepare(&mut raster.text);
            assert_eq!(view.scroll, 0.0, "wheel must not snap back to focus");
            named(&mut view, NamedKey::ArrowDown);
            view.prepare(&mut raster.text);
            assert_eq!(view.view.as_ref().unwrap().focused(), last.as_ref());
            assert_keyboard_reveal(&view);
            // A repeated Down at the end must neither crop nor drift the row.
            let settled = view.scroll;
            for _ in 0..4 {
                view.key(
                    &Key::Named(NamedKey::ArrowDown),
                    None,
                    ModifiersState::empty(),
                    true,
                );
                view.prepare(&mut raster.text);
                assert_keyboard_reveal(&view);
                assert!((view.scroll - settled).abs() < 0.01);
            }
            view.fit(width, height + 37.0, font + 0.5);
            view.paint(&mut raster, theme());
            assert_keyboard_reveal(&view);
        }
    }
    assert!(
        exercised > 200,
        "real catalog coverage must remain nonempty"
    );
}

#[test]
#[ignore = "same-host keyboard reveal/layout/paint benchmark; run explicitly"]
fn keyboard_reveal_navigation_paint_benchmark() {
    for (width, height) in [(600.0, 900.0), (1280.0, 900.0)] {
        let mut view = workflow_tag_view();
        view.fit(width, height, 18.0);
        let mut raster = Raster::new(1.25);
        let mut samples = Vec::new();
        for iteration in 0..220 {
            raster.rects.clear();
            raster.shapes.clear();
            raster.text.clear();
            let start = std::time::Instant::now();
            named(
                &mut view,
                if iteration % 2 == 0 {
                    NamedKey::End
                } else {
                    NamedKey::Home
                },
            );
            view.paint(std::hint::black_box(&mut raster), theme());
            let elapsed = start.elapsed().as_nanos();
            assert_keyboard_reveal(&view);
            assert!(raster.rects.len() < 2000, "draw output remains bounded");
            if iteration >= 20 {
                samples.push(elapsed);
            }
        }
        samples.sort_unstable();
        println!(
            "{}",
            serde_json::json!({"benchmark":"keyboard_reveal_navigation_paint",
            "viewport":[width,height], "scale":1.25, "samples":200,
            "p50_ns":samples[99], "p95_ns":samples[189], "excludes":["GPU","presentation"]})
        );
    }
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

#[test]
fn long_extension_copy_is_visually_bounded_but_remains_searchable_and_accessible() {
    use automexia_ui_model::settings::{Section, SettingOwner};

    let description = format!("Currently off. {} searchable-tail", "é ".repeat(150));
    let mut entry = SettingDescriptor::boolean(
        SettingId::new("extension.fixture.long").unwrap(),
        Section::Extensions,
        "Long extension option",
        description.clone(),
        false,
        false,
    );
    entry.owner = SettingOwner::Extension("fixture".into());
    entry.availability = Availability::Unavailable {
        reason: "Requires setup.".into(),
    };
    let catalog = Catalog::new(1, vec![entry]).unwrap();
    let mut search = ViewState::new(&catalog, 6);
    search.set_query("searchable-tail", &catalog).unwrap();
    assert_eq!(search.filtered_ids().len(), 1);

    for width in [320.0, 720.0] {
        let mut view = SettingsView::default();
        view.fit(width, 560.0, 14.0);
        view.open(catalog.clone());
        view.paint(&mut Raster::new(1.0), theme());
        assert!(!view.requires_larger_window());
        let row = &view.rows[0];
        assert!(row.lines[row.label_lines].starts_with("Currently off."));
        assert!(row.lines[row.label_lines + 1].ends_with('…'));
        assert!(row.lines.iter().any(|line| line == "Requires setup."));
        assert!(!row
            .lines
            .iter()
            .any(|line| line.contains("searchable-tail")));
        assert!(view.accessibility_summary().contains(&description));
    }
}

#[test]
fn appearance_continuous_font_control_preserves_fractions_for_keys_and_pointer() {
    let mut rows = catalog(1).entries().to_vec();
    rows[0].kind = SettingKind::ContinuousNumber {
        min: 6.0,
        max: 100.0,
        step: 1.0,
    };
    rows[0].value = SettingValue::Number(21.5);
    rows[0].default = SettingValue::Number(18.25);
    let mut view = opened();
    view.refresh(Catalog::new(2, rows).unwrap());
    let mut raster = Raster::new(1.0);
    view.paint(&mut raster, theme());
    assert!(view.accessibility_summary().contains("21.5"));
    named(&mut view, NamedKey::Tab);
    for (key, value) in [(NamedKey::ArrowRight, 22.5), (NamedKey::ArrowLeft, 20.5)] {
        named(&mut view, key);
        assert_eq!(
            view.take_edit().unwrap().change,
            Change::Set(SettingValue::Number(value))
        );
    }
    let bounds = view.rows[0].control;
    for (fraction, value) in [(0.15, 20.5), (0.85, 22.5)] {
        let target = view
            .target_at(
                bounds.x + bounds.width * fraction,
                bounds.y + bounds.height * 0.5,
            )
            .unwrap();
        view.activate_target(target);
        assert_eq!(
            view.take_edit().unwrap().change,
            Change::Set(SettingValue::Number(value))
        );
    }
}

#[test]
fn numeric_controls_accept_direct_fractions_without_losing_step_buttons() {
    let mut rows = catalog(1).entries().to_vec();
    rows[0].kind = SettingKind::ContinuousNumber {
        min: 6.0,
        max: 100.0,
        step: 1.0,
    };
    rows[0].value = SettingValue::Number(21.5);
    rows[0].default = SettingValue::Number(18.0);
    let mut view = opened();
    view.refresh(Catalog::new(2, rows).unwrap());
    view.paint(&mut Raster::new(1.0), theme());
    let bounds = view.rows[0].control;
    let target = view
        .target_at(
            bounds.x + bounds.width * 0.5,
            bounds.y + bounds.height * 0.5,
        )
        .unwrap();
    view.activate_target(target);
    assert!(view.paste("18.25"));
    named(&mut view, NamedKey::Enter);
    assert_eq!(
        view.take_edit().unwrap().change,
        Change::Set(SettingValue::Number(18.25))
    );
    assert!(view.numeric_editor.is_none());
    view.paint(&mut Raster::new(1.0), theme());
    for (fraction, expected) in [(0.1, 20.5), (0.9, 22.5)] {
        let target = view
            .target_at(
                bounds.x + bounds.width * fraction,
                bounds.y + bounds.height * 0.5,
            )
            .unwrap();
        view.activate_target(target);
        assert_eq!(
            view.take_edit().unwrap().change,
            Change::Set(SettingValue::Number(expected))
        );
    }
}

#[test]
fn typed_number_rejects_invalid_nonfinite_out_of_range_and_off_step_values() {
    let mut rows = catalog(1).entries().to_vec();
    rows[0].kind = SettingKind::Number {
        min: 0.0,
        max: 300.0,
        step: 1.0,
    };
    rows[0].value = SettingValue::Number(100.0);
    rows[0].default = SettingValue::Number(100.0);
    let mut view = opened();
    view.refresh(Catalog::new(2, rows).unwrap());
    view.paint(&mut Raster::new(1.0), theme());
    let bounds = view.rows[0].control;
    view.activate_target(
        view.target_at(
            bounds.x + bounds.width * 0.5,
            bounds.y + bounds.height * 0.5,
        )
        .unwrap(),
    );
    for draft in ["301", "1.5", "NaN", "１２", "1\n2"] {
        assert!(!view.paste(draft) || matches!(draft, "301" | "1.5"));
        named(&mut view, NamedKey::Enter);
        assert!(
            view.take_edit().is_none(),
            "invalid draft {draft} queued an edit"
        );
        assert!(view.numeric_editor.is_some());
        let editor = view.numeric_editor.as_mut().unwrap();
        editor.anchor = Some(0);
        editor.caret = editor.draft.len();
    }
    assert!(view.paste("300"));
    named(&mut view, NamedKey::Enter);
    assert_eq!(
        view.take_edit().unwrap().change,
        Change::Set(SettingValue::Number(300.0))
    );
}

#[test]
fn numeric_keyboard_and_ime_are_owned_by_settings_until_apply_or_cancel() {
    let mut rows = catalog(1).entries().to_vec();
    rows[0].kind = SettingKind::Number {
        min: 0.0,
        max: 300.0,
        step: 1.0,
    };
    rows[0].value = SettingValue::Number(100.0);
    rows[0].default = SettingValue::Number(100.0);
    let mut view = opened();
    view.refresh(Catalog::new(2, rows).unwrap());
    named(&mut view, NamedKey::Tab);
    view.key(
        &Key::Character("2".into()),
        Some("2"),
        ModifiersState::empty(),
        false,
    );
    assert_eq!(view.numeric_editor.as_ref().unwrap().draft, "2");
    assert!(
        view.event(
            &WindowEvent::Ime(Ime::Preedit("5".into(), Some((0, 1)))),
            ModifiersState::empty(),
            1.0,
        )
        .consumed
    );
    assert_eq!(view.preedit, "5");
    assert!(
        view.event(
            &WindowEvent::Ime(Ime::Commit("5".into())),
            ModifiersState::empty(),
            1.0,
        )
        .consumed
    );
    assert_eq!(view.numeric_editor.as_ref().unwrap().draft, "25");
    named(&mut view, NamedKey::Enter);
    assert_eq!(
        view.take_edit().unwrap().change,
        Change::Set(SettingValue::Number(25.0))
    );
    assert!(view.is_open());
    named(&mut view, NamedKey::Enter);
    assert!(view.numeric_editor.is_some());
    assert!(
        view.event(
            &WindowEvent::Ime(Ime::Preedit("１２".into(), Some((0, 6)))),
            ModifiersState::empty(),
            1.0,
        )
        .consumed
    );
    assert!(view.preedit.is_empty());
    assert!(
        view.event(
            &WindowEvent::Ime(Ime::Commit("１２".into())),
            ModifiersState::empty(),
            1.0,
        )
        .consumed
    );
    assert_eq!(view.numeric_editor.as_ref().unwrap().draft, "100");
    named(&mut view, NamedKey::Escape);
    assert!(view.numeric_editor.is_none());
    assert!(view.take_edit().is_none());
    assert!(view.is_open());
}

#[test]
fn appearance_font_labels_use_exact_shortest_float_roundtrips_without_precision_artifacts(
) {
    let mut row = catalog(1).entries()[0].clone();
    row.kind = SettingKind::ContinuousNumber {
        min: 6.0,
        max: 100.0,
        step: 1.0,
    };
    for (value, label) in [
        (f64::from(21.4f32), "-   21.4   +"),
        (18.0000000001, "-   18.0000000001   +"),
    ] {
        row.value = SettingValue::Number(value);
        assert_eq!(display_value(&row), label);
    }
}

#[test]
fn appearance_actual_catalogue_controls_rasterize_and_keep_values_at_narrow_scales() {
    use crate::automexia::preferences::UserPreferences;
    use rio_backend::config::{
        theme::{AdaptiveColors, AppearanceTheme},
        Config,
    };
    let mut base = Config::default();
    base.fonts.size = 18.25;
    base.adaptive_colors = Some(AdaptiveColors {
        light: Some(base.colors),
        dark: Some(base.colors),
    });
    let prefs = UserPreferences {
        font_size: Some(21.5),
        appearance_theme: Some(AppearanceTheme::Light),
        ..UserPreferences::default()
    };
    for (width, height, font, scale) in [
        (720.0, 560.0, 21.5, 1.0),
        (360.0, 360.0, 18.0, 1.25),
        (280.0, 240.0, 28.0, 2.0),
    ] {
        let mut view = SettingsView::default();
        view.fit(width, height, font);
        view.open(crate::settings_catalog::catalog(4, &base, &prefs, &[]).unwrap());
        view.paste("appearance");
        let mut raster = Raster::new(scale);
        view.paint(&mut raster, theme());
        let summary = view.accessibility_summary();
        if !view.requires_larger_window() {
            assert!(summary.contains("21.5"));
            named(&mut view, NamedKey::Tab);
            named(&mut view, NamedKey::ArrowDown);
            assert!(view.accessibility_summary().contains("Light"));
            raster = Raster::new(scale);
            view.paint(&mut raster, theme());
            assert!(!view.rows.is_empty());
        }
        let pw = (width * scale).ceil() as u32 + 10;
        let ph = (height * scale).ceil() as u32 + 10;
        let blank = raster.pixels(pw, ph, false);
        let pixels = raster.pixels(pw, ph, true);
        assert!(
            pixels != blank,
            "appearance text absent at {width}x{height}"
        );
        for y in 0..ph {
            for x in 0..pw {
                if x as f32 >= width * scale || y as f32 >= height * scale {
                    assert_eq!(pixels[(y * pw + x) as usize], 0x00112233);
                }
            }
        }
        if let Some(directory) = std::env::var_os("AUTOMEXIA_SETTINGS_PREVIEW_DIR") {
            let directory = std::path::PathBuf::from(directory);
            std::fs::create_dir_all(&directory).unwrap();
            image_rs::RgbImage::from_fn(pw, ph, |x, y| {
                let pixel = pixels[(y * pw + x) as usize];
                image_rs::Rgb([(pixel >> 16) as u8, (pixel >> 8) as u8, pixel as u8])
            })
            .save(
                directory
                    .join(format!("appearance-{}-{}.png", width as u32, height as u32)),
            )
            .unwrap();
        }
    }
}

fn color_catalog(revision: u64, alpha: bool) -> Catalog {
    let mut entry = SettingDescriptor::boolean(
        SettingId::new("appearance.test_color").unwrap(),
        automexia_ui_model::settings::Section::Appearance,
        "Test color",
        "Choose an exact color",
        true,
        true,
    );
    entry.kind = SettingKind::Color { alpha };
    entry.value = SettingValue::Color([17, 34, 51, 255]);
    entry.default = SettingValue::Color([255, 128, 0, 255]);
    Catalog::new(revision, vec![entry]).unwrap()
}

fn text_catalog(revision: u64) -> Catalog {
    let mut entry = SettingDescriptor::boolean(
        SettingId::new("tags.slot.windows.literal").unwrap(),
        Section::Customizations,
        "Windows custom text",
        "Choose display text",
        true,
        true,
    );
    entry.kind = SettingKind::Text {
        max_bytes: 128,
        allow_empty: false,
    };
    entry.value = SettingValue::Text("Custom label".into());
    entry.default = SettingValue::Text("Custom label".into());
    Catalog::new(revision, vec![entry]).unwrap()
}

#[test]
fn information_bar_text_editor_handles_mouse_keyboard_ime_and_unsafe_drafts() {
    let mut view = opened();
    view.refresh(text_catalog(2));
    named(&mut view, NamedKey::Tab);
    named(&mut view, NamedKey::Enter);
    assert!(view.color_editor.is_some());
    assert!(view.take_edit().is_none());
    view.key(
        &Key::Character("a".into()),
        None,
        ModifiersState::CONTROL,
        false,
    );
    assert!(view.paste("Nom d'utilisateur"));
    named(&mut view, NamedKey::ArrowLeft);
    view.event(
        &WindowEvent::Ime(Ime::Commit("é".into())),
        ModifiersState::empty(),
        1.0,
    );
    assert!(view.accessibility_summary().contains("Edit text"));
    assert!(!view.paste("\u{001b}[31m"));
    assert!(!view.paste(&"x".repeat(129)));
    named(&mut view, NamedKey::Enter);
    assert_eq!(
        view.take_edit().unwrap().change,
        Change::Set(SettingValue::Text("Nom d'utilisateuér".into()))
    );
    assert!(view.color_editor.is_none());

    view.refresh(text_catalog(3));
    named(&mut view, NamedKey::Enter);
    let mut raster = Raster::new(1.0);
    view.paint(&mut raster, theme());
    let apply = view.color_geometry.apply;
    pointer_event(&mut view, apply, 1.0, ElementState::Pressed);
    pointer_event(&mut view, apply, 1.0, ElementState::Released);
    assert_eq!(
        view.take_edit().unwrap().change,
        Change::Set(SettingValue::Text("Custom label".into()))
    );
}

#[test]
fn information_bar_text_editor_keeps_unicode_graphemes_and_mouse_selection_on_boundaries()
{
    let mut view = opened();
    view.refresh(text_catalog(2));
    named(&mut view, NamedKey::Tab);
    named(&mut view, NamedKey::Enter);
    view.key(
        &Key::Character("a".into()),
        None,
        ModifiersState::CONTROL,
        false,
    );
    assert!(view.paste("e\u{301}🙂"));
    named(&mut view, NamedKey::Home);
    named(&mut view, NamedKey::ArrowRight);
    named(&mut view, NamedKey::Backspace);
    assert_eq!(view.color_editor.as_ref().unwrap().draft, "🙂");
    let mut raster = Raster::new(1.0);
    view.paint(&mut raster, theme());
    let input = view.color_geometry.input;
    pointer_event(&mut view, input, 1.0, ElementState::Pressed);
    pointer_event(&mut view, input, 1.0, ElementState::Released);
    assert!(view.paste("日本語"));
    view.event(
        &WindowEvent::Ime(Ime::Commit("🧪".into())),
        ModifiersState::empty(),
        1.0,
    );
    assert_eq!(view.color_editor.as_ref().unwrap().draft, "日本語🧪");
    named(&mut view, NamedKey::Enter);
    assert_eq!(
        view.take_edit().unwrap().change,
        Change::Set(SettingValue::Text("日本語🧪".into()))
    );
}

#[test]
fn narrow_information_tag_slot_page_opens_by_keyboard_and_edits_text_by_mouse() {
    let base = rio_backend::config::Config::default();
    let preferences = crate::settings_catalog::apply_edit(
        1,
        &base,
        &Default::default(),
        &[],
        &Edit {
            revision: 1,
            id: SettingId::new("tags.slot.windows.text").unwrap(),
            change: Change::Set(SettingValue::Choice("literal".into())),
        },
    )
    .unwrap();
    let snapshot = crate::settings_catalog::catalog(1, &base, &preferences, &[]).unwrap();
    let mut view = SettingsView::default();
    view.fit(320.0, 360.0, 18.0);
    let pages = crate::settings_catalog::slot_page_snapshot(
        &preferences,
        &preferences.apply_to(&base),
    );
    view.open_customizations_with_slots(snapshot, None, Some(pages));
    let category = SettingId::new("tags.enabled").unwrap();
    assert!(view.view.as_mut().unwrap().focus(&category));
    view.focus = Focus::List;
    named(&mut view, NamedKey::Enter);
    assert_eq!(view.title(), "Information tags");
    let page = SettingId::new("tags.slot.windows.page").unwrap();
    view.focus = Focus::List;
    named(&mut view, NamedKey::Tab);
    assert_eq!(view.focus, Focus::PreviewButton);
    let mut raster = Raster::new(1.0);
    view.paint(&mut raster, theme());
    named(&mut view, NamedKey::Enter);
    assert_eq!(view.focus, Focus::Preview);
    view.preview_selected = Some(page.clone());
    named(&mut view, NamedKey::Enter);
    assert_eq!(view.title(), "Tag slot: Windows");
    assert_eq!(view.catalog.as_ref().unwrap().entries().len(), 9);
    assert!(view
        .catalog
        .as_ref()
        .unwrap()
        .get(&SettingId::new("tags.slot.windows.literal").unwrap())
        .is_some());
    assert!(view.take_edit().is_none());
    view.back_to_categories();
    assert_eq!(view.title(), "Information tags");
    assert_eq!(view.preview_selected.as_ref(), Some(&page));
    assert_eq!(view.focus, Focus::List);
    preview_edit_key(&mut view);
    assert_eq!(view.focus, Focus::Preview);
    named(&mut view, NamedKey::Enter);
    assert_eq!(view.title(), "Tag slot: Windows");
    let literal = SettingId::new("tags.slot.windows.literal").unwrap();
    assert!(view.view.as_mut().unwrap().focus(&literal));
    view.reveal_focus = true;
    view.paint(&mut raster, theme());
    let target = view
        .rows
        .iter()
        .find(|row| row.id == literal)
        .unwrap()
        .control;
    pointer_event(&mut view, target, 1.0, ElementState::Pressed);
    pointer_event(&mut view, target, 1.0, ElementState::Released);
    assert!(view.color_editor.is_some());
    assert!(view.paste("Mon compte"));
    let mut editor_raster = Raster::new(1.0);
    view.paint(&mut editor_raster, theme());
    let apply = view.color_geometry.apply;
    pointer_event(&mut view, apply, 1.0, ElementState::Pressed);
    pointer_event(&mut view, apply, 1.0, ElementState::Released);
    let edit = view.take_edit().unwrap();
    assert_eq!(edit.id, literal);
    assert_eq!(
        edit.change,
        Change::Set(SettingValue::Text("Mon compte".into()))
    );
    named(&mut view, NamedKey::Escape);
    assert_eq!(view.title(), "Information tags");
    assert_eq!(view.focus, Focus::List);
    named(&mut view, NamedKey::Escape);
    assert!(view.is_category_root());
    assert!(view.is_open());
}

#[test]
fn information_tag_slot_search_mouse_open_and_back_preserve_parent_query() {
    let base = rio_backend::config::Config::default();
    let snapshot =
        crate::settings_catalog::catalog(1, &base, &Default::default(), &[]).unwrap();
    let mut view = SettingsView::default();
    view.fit(320.0, 360.0, 18.0);
    let pages = crate::settings_catalog::slot_page_snapshot(&Default::default(), &base);
    view.open_customizations_with_slots(snapshot, None, Some(pages));
    assert!(view
        .catalog
        .as_ref()
        .unwrap()
        .get(&SettingId::new("tags.slot.windows.page").unwrap())
        .is_none());
    let category = SettingId::new("tags.enabled").unwrap();
    assert!(view.view.as_mut().unwrap().focus(&category));
    view.focus = Focus::List;
    named(&mut view, NamedKey::Enter);
    view.focus = Focus::Search;
    assert!(view.paste("Tag slot: Windows"));
    let page = SettingId::new("tags.slot.windows.page").unwrap();
    let mut raster = Raster::new(1.0);
    view.paint(&mut raster, theme());
    let button = view.preview_button;
    pointer_event(&mut view, button, 1.0, ElementState::Pressed);
    pointer_event(&mut view, button, 1.0, ElementState::Released);
    assert_eq!(view.focus, Focus::Preview);
    assert!(
        view.preview_items().len() > 1,
        "searching controls does not replace or filter the live preview"
    );
    view.preview_tag_list_scroll = 2;
    view.paint(&mut raster, theme());
    let visible_label = view
        .preview_targets
        .iter()
        .find(|(id, _)| id == &page)
        .unwrap()
        .1;
    pointer_event(&mut view, visible_label, 1.0, ElementState::Pressed);
    pointer_event(&mut view, visible_label, 1.0, ElementState::Released);
    assert_eq!(view.title(), "Tag slot: Windows");
    assert!(view.take_edit().is_none());
    named(&mut view, NamedKey::Escape);
    assert_eq!(view.title(), "Information tags");
    assert_eq!(view.query(), "Tag slot: Windows");
    assert_eq!(view.preview_selected.as_ref(), Some(&page));
    assert_eq!(view.focus, Focus::List);
    named(&mut view, NamedKey::Escape);
    assert!(view.is_category_root());
}

#[test]
fn active_information_tag_slot_refreshes_its_controls_after_a_saved_edit() {
    let base = rio_backend::config::Config::default();
    let original = crate::automexia::preferences::UserPreferences::default();
    let mut view = SettingsView::default();
    view.fit(480.0, 480.0, 16.0);
    let catalog = crate::settings_catalog::catalog(1, &base, &original, &[]).unwrap();
    let pages = crate::settings_catalog::slot_page_snapshot(&original, &base);
    view.open_customizations_with_slots(catalog, None, Some(pages));
    assert!(view
        .view
        .as_mut()
        .unwrap()
        .focus(&SettingId::new("tags.enabled").unwrap()));
    view.focus = Focus::List;
    named(&mut view, NamedKey::Enter);
    let mut raster = Raster::new(1.0);
    view.paint(&mut raster, theme());
    view.start_preview_edit();
    view.preview_selected = Some(SettingId::new("tags.slot.windows.page").unwrap());
    named(&mut view, NamedKey::Enter);
    let literal = SettingId::new("tags.slot.windows.literal").unwrap();
    let saved = crate::settings_catalog::apply_edit(
        1,
        &base,
        &original,
        &[],
        &Edit {
            revision: 1,
            id: literal.clone(),
            change: Change::Set(SettingValue::Text("Custom Windows".into())),
        },
    )
    .unwrap();
    let updated = crate::settings_catalog::catalog(2, &base, &saved, &[]).unwrap();
    let pages =
        crate::settings_catalog::slot_page_snapshot(&saved, &saved.apply_to(&base));
    view.refresh_with_resources(updated, None, Some(pages));
    assert_eq!(view.title(), "Tag slot: Windows");
    assert_eq!(
        view.catalog.as_ref().unwrap().get(&literal).unwrap().value,
        SettingValue::Text("Custom Windows".into())
    );
}

#[test]
fn color_editor_keyboard_draft_applies_exact_typed_color_without_changing_search() {
    let mut view = opened();
    view.refresh(color_catalog(2, false));
    named(&mut view, NamedKey::Tab);
    named(&mut view, NamedKey::Enter);
    assert!(view.take_edit().is_none(), "opening a color is not an edit");
    view.key(
        &Key::Character("a".into()),
        None,
        ModifiersState::CONTROL,
        false,
    );
    assert!(
        view.paste("#01aBc0"),
        "the color draft must own text input after opening"
    );
    assert_eq!(
        view.query(),
        "",
        "draft color text must not become a search"
    );
    named(&mut view, NamedKey::Enter);
    let edit = view
        .take_edit()
        .expect("explicit Apply produces the existing typed intent");
    assert_eq!(edit.revision, 2);
    assert_eq!(edit.id.as_str(), "appearance.test_color");
    assert_eq!(
        edit.change,
        Change::Set(SettingValue::Color([1, 171, 192, 255]))
    );
    assert!(view.is_open());
    assert_eq!(view.focus, Focus::List);
    assert_eq!(
        view.catalog.as_ref().unwrap().entries()[0].value,
        SettingValue::Color([17, 34, 51, 255])
    );
}

fn opened_color(alpha: bool) -> SettingsView {
    let mut view = opened();
    view.refresh(color_catalog(2, alpha));
    named(&mut view, NamedKey::Tab);
    named(&mut view, NamedKey::Enter);
    assert!(view.color_editor.is_some());
    assert!(view.take_edit().is_none());
    view
}

#[test]
fn shared_color_picker_keyboard_tab_starts_with_palette_actions() {
    let mut view = opened_color(false);
    view.paint(&mut Raster::new(1.0), theme());
    for expected in [
        ColorFocus::Suggested,
        ColorFocus::Favorites,
        ColorFocus::FavoriteToggle,
        ColorFocus::Apply,
        ColorFocus::Cancel,
        ColorFocus::Reset,
        ColorFocus::Hex,
    ] {
        named(&mut view, NamedKey::Tab);
        assert_eq!(view.color_editor.as_ref().unwrap().focus, expected);
        assert!(view.take_edit().is_none());
        assert!(view.take_color_favorite_intent().is_none());
    }
}

#[test]
fn shared_color_picker_keyboard_group_enters_wrapping_swatches_and_escape_returns() {
    for group in [ColorFocus::Suggested, ColorFocus::Favorites] {
        let mut view = opened_color(false);
        view.set_color_favorites(&[[1, 2, 3, 255], [4, 5, 6, 255]]);
        view.paint(&mut Raster::new(1.0), theme());
        named(&mut view, NamedKey::Tab);
        if group == ColorFocus::Favorites {
            named(&mut view, NamedKey::Tab);
        }
        let original = view.color_editor.as_ref().unwrap().draft.clone();
        named(&mut view, NamedKey::Enter);
        assert_eq!(
            view.color_editor.as_ref().unwrap().focus,
            ColorFocus::Swatch(0)
        );
        let count = view.color_palette_colors().len();
        for index in 1..=count * 2 {
            named(&mut view, NamedKey::Tab);
            assert_eq!(
                view.color_editor.as_ref().unwrap().focus,
                ColorFocus::Swatch(index % count)
            );
        }
        view.key(
            &Key::Named(NamedKey::Tab),
            None,
            ModifiersState::SHIFT,
            false,
        );
        assert_eq!(
            view.color_editor.as_ref().unwrap().focus,
            ColorFocus::Swatch(count - 1)
        );
        named(&mut view, NamedKey::ArrowRight);
        assert_eq!(
            view.color_editor.as_ref().unwrap().focus,
            ColorFocus::Swatch(0)
        );
        named(&mut view, NamedKey::Escape);
        let editor = view
            .color_editor
            .as_ref()
            .expect("Escape leaves the grid, not the picker");
        assert_eq!(editor.focus, group);
        assert_eq!(editor.draft, original);
        view.key(
            &Key::Named(NamedKey::Escape),
            None,
            ModifiersState::empty(),
            true,
        );
        assert!(
            view.color_editor.is_some(),
            "held Escape must not discard the draft"
        );
        assert!(view.take_edit().is_none());
        named(&mut view, NamedKey::Escape);
        assert!(view.color_editor.is_none());
    }
}

#[test]
fn shared_color_picker_keyboard_shortcuts_grid_bounds_and_explicit_selection() {
    for count in [0, 1, 7, 8, 9, 16] {
        let mut view = opened_color(false);
        let favorites: Vec<_> = (0..count).map(|i| [i as u8, 90, 120, 255]).collect();
        view.set_color_favorites(&favorites);
        view.paint(&mut Raster::new(1.0), theme());
        named(&mut view, NamedKey::F2);
        let first = if count == 0 {
            ColorFocus::Favorites
        } else {
            ColorFocus::Swatch(0)
        };
        assert_eq!(view.color_editor.as_ref().unwrap().focus, first);
        for _ in 0..3 {
            for _ in 0..count.max(1) {
                named(&mut view, NamedKey::Tab);
            }
            assert_eq!(view.color_editor.as_ref().unwrap().focus, first);
        }
        for key in [
            NamedKey::ArrowLeft,
            NamedKey::ArrowUp,
            NamedKey::ArrowRight,
            NamedKey::ArrowDown,
        ] {
            for _ in 0..count.max(1) {
                named(&mut view, key);
            }
            assert_eq!(view.color_editor.as_ref().unwrap().focus, first);
        }
        let draft = view.color_editor.as_ref().unwrap().draft.clone();
        view.key(
            &Key::Character("a".into()),
            Some("a"),
            ModifiersState::empty(),
            false,
        );
        view.key(
            &Key::Character("r".into()),
            Some("r"),
            ModifiersState::empty(),
            false,
        );
        assert!(view.confirmation.is_none());
        assert!(view.take_edit().is_none());
        assert_eq!(view.color_editor.as_ref().unwrap().draft, draft);
        named(&mut view, NamedKey::Enter);
        if count == 0 {
            assert_eq!(
                view.color_editor.as_ref().unwrap().focus,
                ColorFocus::Favorites
            );
            named(&mut view, NamedKey::Escape);
            assert!(view.color_editor.is_some());
        } else {
            assert_eq!(view.color_editor.as_ref().unwrap().focus, ColorFocus::Hex);
            assert!(
                view.take_edit().is_none(),
                "choosing is distinct from applying"
            );
            named(&mut view, NamedKey::F3);
            assert_eq!(
                view.take_color_favorite_intent(),
                Some(ColorFavoriteIntent::Forget(favorites[0]))
            );
            view.key(
                &Key::Named(NamedKey::F3),
                None,
                ModifiersState::empty(),
                true,
            );
            assert!(view.take_color_favorite_intent().is_none());
        }
        named(&mut view, NamedKey::F1);
        assert!(matches!(
            view.color_editor.as_ref().unwrap().focus,
            ColorFocus::Swatch(_)
        ));
        named(&mut view, NamedKey::Escape);
        assert_eq!(
            view.color_editor.as_ref().unwrap().focus,
            ColorFocus::Suggested
        );
    }
}

#[test]
fn shared_color_picker_keyboard_favorite_refresh_preserves_grid_and_resize_exits_safely()
{
    let mut view = opened_color(false);
    view.set_color_favorites(&[[1, 2, 3, 255], [4, 5, 6, 255]]);
    view.paint(&mut Raster::new(1.0), theme());
    named(&mut view, NamedKey::F2);
    named(&mut view, NamedKey::Tab);
    view.set_color_favorites(&[[4, 5, 6, 255], [1, 2, 3, 255]]);
    assert_eq!(
        view.color_editor.as_ref().unwrap().focus,
        ColorFocus::Swatch(0)
    );
    named(&mut view, NamedKey::Delete);
    assert_eq!(
        view.take_color_favorite_intent(),
        Some(ColorFavoriteIntent::Forget([4, 5, 6, 255]))
    );
    view.set_color_favorites(&[[1, 2, 3, 255]]);
    named(&mut view, NamedKey::Tab);
    assert_eq!(
        view.color_editor.as_ref().unwrap().focus,
        ColorFocus::Swatch(0)
    );
    view.set_color_favorites(&[]);
    named(&mut view, NamedKey::Tab);
    assert_eq!(
        view.color_editor.as_ref().unwrap().focus,
        ColorFocus::Favorites
    );
    named(&mut view, NamedKey::Escape);
    named(&mut view, NamedKey::Tab);
    assert_eq!(
        view.color_editor.as_ref().unwrap().focus,
        ColorFocus::FavoriteToggle
    );
    named(&mut view, NamedKey::F1);
    let focus = view.color_editor.as_ref().unwrap().focus;
    view.set_color_favorites(&[[7, 8, 9, 255]]);
    assert_eq!(
        view.color_editor.as_ref().unwrap().focus,
        focus,
        "favorite refresh cannot steal suggested focus"
    );
    view.fit(320., 360., 18.);
    view.paint(&mut Raster::new(1.0), theme());
    assert!(view.color_palette_controls().is_empty());
    assert_eq!(view.color_editor.as_ref().unwrap().focus, ColorFocus::Hex);
    named(&mut view, NamedKey::F1);
    assert_eq!(view.color_editor.as_ref().unwrap().focus, ColorFocus::Hex);
    assert!(replace_color(&mut view, "#112233"));
}

#[test]
fn shared_color_picker_keyboard_shortcuts_respect_modifiers_ime_and_text_editors() {
    let mut view = opened_color(false);
    view.paint(&mut Raster::new(1.0), theme());
    for modifiers in [
        ModifiersState::CONTROL,
        ModifiersState::SUPER,
        ModifiersState::ALT,
        ModifiersState::SHIFT,
    ] {
        for key in [NamedKey::F1, NamedKey::F2, NamedKey::F3, NamedKey::Tab] {
            if modifiers == ModifiersState::SHIFT && key == NamedKey::Tab {
                continue;
            }
            view.key(&Key::Named(key), None, modifiers, false);
            assert_eq!(view.color_editor.as_ref().unwrap().focus, ColorFocus::Hex);
            assert!(view.take_color_favorite_intent().is_none());
        }
    }
    view.event(
        &WindowEvent::Ime(Ime::Preedit("#AB".into(), None)),
        ModifiersState::empty(),
        1.0,
    );
    for key in [NamedKey::F1, NamedKey::F2, NamedKey::F3] {
        named(&mut view, key);
    }
    assert_eq!(view.color_editor.as_ref().unwrap().focus, ColorFocus::Hex);
    assert!(view.color_editor.as_ref().unwrap().composing);
    assert!(view.take_color_favorite_intent().is_none());
    named(&mut view, NamedKey::Escape);
    assert!(replace_color(&mut view, "invalid"));
    named(&mut view, NamedKey::F3);
    assert!(view.take_color_favorite_intent().is_none());
    assert!(replace_color(&mut view, "#123456"));
    named(&mut view, NamedKey::F3);
    assert_eq!(
        view.take_color_favorite_intent(),
        Some(ColorFavoriteIntent::Remember([18, 52, 86, 255]))
    );
    view.key(
        &Key::Named(NamedKey::F3),
        None,
        ModifiersState::empty(),
        true,
    );
    assert!(view.take_color_favorite_intent().is_none());
    assert!(view.take_edit().is_none());

    let mut text = opened();
    text.refresh(text_catalog(2));
    named(&mut text, NamedKey::Tab);
    named(&mut text, NamedKey::Enter);
    text.paint(&mut Raster::new(1.0), theme());
    let original = text.color_editor.as_ref().unwrap().draft.clone();
    for key in [NamedKey::F1, NamedKey::F2, NamedKey::F3] {
        named(&mut text, key);
    }
    assert_eq!(text.color_editor.as_ref().unwrap().draft, original);
    assert_eq!(text.color_editor.as_ref().unwrap().focus, ColorFocus::Hex);
    assert!(text.take_color_favorite_intent().is_none());
    named(&mut text, NamedKey::Tab);
    assert_eq!(text.color_editor.as_ref().unwrap().focus, ColorFocus::Apply);
}

#[test]
fn shared_color_picker_keyboard_offers_swatches_without_implicit_apply() {
    let mut view = opened_color(false);
    view.paint(&mut Raster::new(1.0), theme());
    named(&mut view, NamedKey::ArrowDown);
    assert_ne!(
        view.color_editor.as_ref().unwrap().focus,
        ColorFocus::Hex,
        "Down from custom input must reach suggested colors"
    );
    named(&mut view, NamedKey::Enter);
    assert!(
        view.take_edit().is_none(),
        "choosing a swatch only updates the draft"
    );
    assert_eq!(view.color_editor.as_ref().unwrap().focus, ColorFocus::Hex);
    named(&mut view, NamedKey::Enter);
    assert!(matches!(
        view.take_edit().map(|edit| edit.change),
        Some(Change::Set(SettingValue::Color(_)))
    ));
}
fn replace_color(view: &mut SettingsView, text: &str) -> bool {
    view.key(
        &Key::Character("a".into()),
        None,
        ModifiersState::CONTROL,
        false,
    );
    view.paste(text)
}
fn pointer_event(view: &mut SettingsView, bounds: Rect, scale: f64, state: ElementState) {
    // SAFETY: this ID is confined to constructing pure adapter events.
    let device = unsafe { DeviceId::dummy() };
    assert!(
        view.event(
            &WindowEvent::CursorMoved {
                device_id: device,
                position: rio_window::dpi::PhysicalPosition::new(
                    f64::from(bounds.x + bounds.width * 0.5) * scale,
                    f64::from(bounds.y + bounds.height * 0.5) * scale,
                ),
            },
            ModifiersState::empty(),
            scale
        )
        .consumed
    );
    assert!(
        view.event(
            &WindowEvent::MouseInput {
                device_id: device,
                state,
                button: MouseButton::Left,
            },
            ModifiersState::empty(),
            scale
        )
        .consumed
    );
}

#[test]
fn customization_categories_support_mouse_open_and_back_at_narrow_scale() {
    let base = rio_backend::config::Config::default();
    for (width, height, font, scale) in
        [(720.0, 560.0, 14.0, 1.0), (320.0, 360.0, 18.0, 1.25)]
    {
        let mut view = SettingsView::default();
        view.fit(width, height, font);
        view.open_with_section(
            crate::settings_catalog::catalog(1, &base, &Default::default(), &[]).unwrap(),
            Some(Section::Customizations),
        );
        let mut raster = Raster::new(scale as f32);
        view.paint(&mut raster, theme());
        assert!(!view.requires_larger_window());
        let first_row = view.rows[0].bounds;
        let category = Rect {
            x: first_row.x + 12.0,
            y: first_row.y + 12.0,
            width: 1.0,
            height: 1.0,
        };
        assert!(!view.rows[0].control.contains(category.x, category.y));
        pointer_event(&mut view, category, scale, ElementState::Pressed);
        pointer_event(&mut view, category, scale, ElementState::Released);
        assert!(view.is_category_detail());
        assert!(view.take_edit().is_none());
        let mut detail_raster = Raster::new(scale as f32);
        view.paint(&mut detail_raster, theme());
        let back = view.geometry.back;
        assert!(back.width > 0.0 && back.height > 0.0);
        assert!(back.x + back.width <= width && back.y + back.height <= height);
        for ([x, y, w, h], _) in &detail_raster.rects {
            assert!(
                *x >= 0.0
                    && *y >= 0.0
                    && x + w <= width + 0.001
                    && y + h <= height + 0.001
            );
        }
        pointer_event(&mut view, back, scale, ElementState::Pressed);
        pointer_event(&mut view, back, scale, ElementState::Released);
        assert!(view.is_category_root());
        assert!(view.take_edit().is_none());
    }
}

#[test]
fn compact_customization_detail_returns_to_categories_without_activating_hidden_controls()
{
    let base = rio_backend::config::Config::default();
    let mut view = SettingsView::default();
    view.fit(720.0, 560.0, 14.0);
    view.open_with_section(
        crate::settings_catalog::catalog(1, &base, &Default::default(), &[]).unwrap(),
        Some(Section::Customizations),
    );
    named(&mut view, NamedKey::Tab);
    named(&mut view, NamedKey::Enter);
    assert!(view.is_category_detail());
    view.fit(80.0, 100.0, 20.0);
    assert!(view.requires_larger_window());
    assert!(view
        .accessibility_summary()
        .contains("returns to categories"));
    named(&mut view, NamedKey::Space);
    assert!(view.take_edit().is_none());
    named(&mut view, NamedKey::Escape);
    assert!(view.is_category_root());
    assert!(view.is_open());
}

#[test]
fn color_editor_repeats_and_invalid_or_oversized_drafts_never_apply() {
    let mut view = opened_color(false);
    for _ in 0..8 {
        view.key(
            &Key::Named(NamedKey::Enter),
            Some("\r"),
            ModifiersState::empty(),
            true,
        );
    }
    assert!(view.color_editor.is_some());
    assert!(view.take_edit().is_none());
    for invalid in ["", "#123", "#GG1122", "112233", "#11223380", "##11223"] {
        assert!(replace_color(&mut view, invalid));
        named(&mut view, NamedKey::Enter);
        assert!(view.take_edit().is_none());
        assert!(view.color_editor.is_some());
        assert!(view.accessibility_summary().contains("Apply unavailable"));
    }
    for rejected in [
        "#112233\n",
        "#112233\u{202e}",
        "界",
        "#112233445566",
        "#112233 ",
    ] {
        let before = view.color_editor.as_ref().unwrap().draft.clone();
        assert!(!replace_color(&mut view, rejected));
        assert_eq!(view.color_editor.as_ref().unwrap().draft, before);
        assert!(view.take_edit().is_none());
    }
    assert!(replace_color(&mut view, "#00FF80"));
    view.key(
        &Key::Named(NamedKey::Enter),
        None,
        ModifiersState::empty(),
        true,
    );
    assert!(view.take_edit().is_none());
    named(&mut view, NamedKey::Enter);
    for _ in 0..8 {
        view.key(
            &Key::Named(NamedKey::Enter),
            None,
            ModifiersState::empty(),
            true,
        );
    }
    assert!(view.color_editor.is_none());
    assert_eq!(
        view.take_edit().unwrap().change,
        Change::Set(SettingValue::Color([0, 255, 128, 255]))
    );
    assert!(view.take_edit().is_none());
}

#[test]
fn color_editor_alpha_preserves_exact_bytes_and_rgb_defaults_to_opaque() {
    for (token, expected) in [
        ("#12345600", [18, 52, 86, 0]),
        ("#12345601", [18, 52, 86, 1]),
        ("#abcdefFE", [171, 205, 239, 254]),
        ("#ABCDEF", [171, 205, 239, 255]),
    ] {
        let mut view = opened_color(true);
        assert!(replace_color(&mut view, token));
        named(&mut view, NamedKey::Enter);
        assert_eq!(
            view.take_edit().unwrap().change,
            Change::Set(SettingValue::Color(expected))
        );
    }
    // Characterize the existing parser's normalized-alpha adapter at every byte.
    for alpha in 0..=255 {
        assert_eq!(
            parse_color(&format!("#123456{alpha:02X}"), true),
            Some([18, 52, 86, alpha])
        );
    }
}

#[test]
fn color_editor_cancel_reset_and_focus_cycle_are_explicit_and_restore_the_list() {
    let mut view = opened_color(true);
    assert!(replace_color(&mut view, "#00000000"));
    named(&mut view, NamedKey::Tab);
    assert_eq!(view.color_editor.as_ref().unwrap().focus, ColorFocus::Apply);
    named(&mut view, NamedKey::Tab);
    assert_eq!(
        view.color_editor.as_ref().unwrap().focus,
        ColorFocus::Cancel
    );
    named(&mut view, NamedKey::Enter);
    assert!(view.is_open());
    assert!(view.color_editor.is_none());
    assert_eq!(view.focus, Focus::List);
    assert!(view.take_edit().is_none());
    named(&mut view, NamedKey::Enter);
    view.key(
        &Key::Named(NamedKey::Tab),
        None,
        ModifiersState::SHIFT,
        false,
    );
    assert_eq!(view.color_editor.as_ref().unwrap().focus, ColorFocus::Reset);
    named(&mut view, NamedKey::Enter);
    confirm_requested_settings_action(&mut view);
    assert_eq!(view.take_edit().unwrap().change, Change::Reset);
    assert!(view.color_editor.is_none());
    assert_eq!(
        view.catalog.as_ref().unwrap().entries()[0].value,
        SettingValue::Color([17, 34, 51, 255])
    );
    named(&mut view, NamedKey::Enter);
    named(&mut view, NamedKey::Escape);
    assert!(view.color_editor.is_none());
    assert!(view.is_open());
    named(&mut view, NamedKey::Escape);
    assert!(!view.is_open());
}

#[test]
fn color_editor_ime_is_bounded_cancels_composition_before_draft_and_never_edits_search() {
    let mut view = opened_color(false);
    for preedit in ["界", "#123456789ABC", "\n", "#445566"] {
        assert!(
            view.event(
                &WindowEvent::Ime(Ime::Preedit(preedit.into(), None)),
                ModifiersState::empty(),
                1.0
            )
            .consumed
        );
        named(&mut view, NamedKey::Enter);
        assert!(view.take_edit().is_none(), "composition Enter cannot Apply");
        named(&mut view, NamedKey::Escape);
        assert!(
            view.color_editor.is_some(),
            "first Escape cancels composition"
        );
        assert!(view.preedit.is_empty());
    }
    view.event(
        &WindowEvent::Ime(Ime::Commit("界".into())),
        ModifiersState::empty(),
        1.0,
    );
    assert_eq!(view.color_editor.as_ref().unwrap().draft, "#112233");
    view.event(
        &WindowEvent::Ime(Ime::Commit("#445566".into())),
        ModifiersState::empty(),
        1.0,
    );
    assert_eq!(view.query(), "");
    named(&mut view, NamedKey::Enter);
    assert_eq!(
        view.take_edit().unwrap().change,
        Change::Set(SettingValue::Color([68, 85, 102, 255]))
    );
    // A late commit after cancellation cannot become a search or terminal edit.
    named(&mut view, NamedKey::Enter);
    named(&mut view, NamedKey::Escape);
    view.event(
        &WindowEvent::Ime(Ime::Commit("#778899".into())),
        ModifiersState::empty(),
        1.0,
    );
    assert_eq!(view.query(), "");
    assert!(view.take_edit().is_none());
}

#[test]
fn color_editor_refresh_cancels_revision_removal_unavailable_and_kind_replacement() {
    for scenario in 0..4 {
        let mut view = opened_color(false);
        assert!(replace_color(&mut view, "#445566"));
        let mut entries = color_catalog(2, false).entries().to_vec();
        match scenario {
            1 => entries.clear(),
            2 => {
                entries[0].availability = Availability::Unavailable {
                    reason: "Removed capability".into(),
                }
            }
            3 => {
                entries[0].kind = SettingKind::Boolean;
                entries[0].value = SettingValue::Boolean(true);
                entries[0].default = SettingValue::Boolean(false);
            }
            _ => {}
        }
        view.refresh(Catalog::new(if scenario == 0 { 3 } else { 2 }, entries).unwrap());
        assert!(view.color_editor.is_none());
        assert!(view.preedit.is_empty());
        assert!(view.take_edit().is_none());
        assert!(view.pressed.is_none());
    }
}

#[test]
fn color_editor_pointer_requires_current_geometry_and_matching_release_and_cancel() {
    let mut view = opened();
    view.refresh(color_catalog(2, true));
    let mut raster = Raster::new(1.25);
    view.paint(&mut raster, theme());
    let control = view.rows[0].control;
    pointer_event(&mut view, control, 1.25, ElementState::Pressed);
    pointer_event(&mut view, control, 1.25, ElementState::Released);
    assert!(view.color_editor.is_some());
    assert!(view.take_edit().is_none());
    view.paint(&mut raster, theme());
    assert!(view.ime_cursor_area().is_some());
    let apply = view.color_geometry.apply;
    pointer_event(&mut view, apply, 1.25, ElementState::Pressed);
    let cancel = view.color_geometry.cancel;
    pointer_event(&mut view, cancel, 1.25, ElementState::Released);
    assert!(view.take_edit().is_none());
    assert!(view.color_editor.is_some());
    pointer_event(&mut view, apply, 1.25, ElementState::Pressed);
    view.fit(680.0, 530.0, 14.0);
    view.paint(&mut raster, theme());
    let apply = view.color_geometry.apply;
    pointer_event(&mut view, apply, 1.25, ElementState::Released);
    assert!(
        view.take_edit().is_none(),
        "resize invalidates captured activation"
    );
    let cancel = view.color_geometry.cancel;
    pointer_event(&mut view, cancel, 1.25, ElementState::Pressed);
    pointer_event(&mut view, cancel, 1.25, ElementState::Released);
    assert!(view.color_editor.is_none());
    assert!(view.is_open());
    assert!(view.take_edit().is_none());
    named(&mut view, NamedKey::Enter);
    view.event(&WindowEvent::Focused(false), ModifiersState::empty(), 1.25);
    assert!(view.color_editor.is_none());
    assert!(view.pressed.is_none());
}

#[test]
fn color_editor_controlled_draw_and_caret_are_contained_with_exact_hex_semantics() {
    for (width, height, font, scale) in [
        (720.0, 560.0, 14.0, 1.0),
        (320.0, 360.0, 18.0, 1.25),
        (1100.0, 1100.0, 64.0, 2.0),
    ] {
        let mut view = opened_color(true);
        view.fit(width, height, font);
        assert!(replace_color(&mut view, "#12345680"));
        let mut raster = Raster::new(scale);
        view.paint(&mut raster, theme());
        assert!(view.accessibility_summary().contains("#12345680"));
        let caret = view.ime_cursor_area().unwrap();
        let input = view.color_geometry.input;
        assert!(caret[0] >= input.x && caret[0] + caret[2] <= input.x + input.width);
        assert!(caret[1] >= input.y && caret[1] + caret[3] <= input.y + input.height);
        for ([x, y, w, h], _) in &raster.rects {
            assert!(*x >= 0.0 && *y >= 0.0 && *w >= 0.0 && *h >= 0.0);
            assert!(x + w <= width + 0.001 && y + h <= height + 0.001);
        }
        assert!(
            raster.rects.iter().any(|(_, color)| color
                == &[18.0 / 255.0, 52.0 / 255.0, 86.0 / 255.0, 128.0 / 255.0]),
            "draft draw data preserves exact RGBA"
        );
        let pw = (width * scale).ceil() as u32 + 8;
        let ph = (height * scale).ceil() as u32 + 8;
        let blank = raster.pixels(pw, ph, false);
        let pixels = raster.pixels(pw, ph, true);
        assert_ne!(pixels, blank, "popup text must rasterize");
        for y in 0..ph {
            for x in 0..pw {
                if x as f32 >= width * scale || y as f32 >= height * scale {
                    assert_eq!(pixels[(y * pw + x) as usize], 0x00112233);
                }
            }
        }
    }
    let mut view = opened_color(false);
    assert!(replace_color(&mut view, "#445566"));
    view.fit(80.0, 100.0, 20.0);
    assert!(view.color_editor.is_none());
    assert!(view.take_edit().is_none());
    assert!(view.ime_cursor_area().is_none());
}

#[test]
fn color_editor_pointer_apply_is_typed_and_stale_or_invalid_apply_stays_blocked() {
    let mut view = opened_color(true);
    assert!(replace_color(&mut view, "#76543280"));
    let mut raster = Raster::new(1.0);
    view.paint(&mut raster, theme());
    let apply = view.color_geometry.apply;
    pointer_event(&mut view, apply, 1.0, ElementState::Pressed);
    pointer_event(&mut view, apply, 1.0, ElementState::Released);
    assert!(view.color_editor.is_none());
    named(&mut view, NamedKey::Enter);
    assert!(
        view.color_editor.is_none(),
        "the pending edit retains its single owner"
    );
    assert_eq!(
        view.take_edit().unwrap().change,
        Change::Set(SettingValue::Color([118, 84, 50, 128]))
    );
    named(&mut view, NamedKey::Enter);
    assert!(replace_color(&mut view, "#GG0000"));
    view.paint(&mut raster, theme());
    let apply = view.color_geometry.apply;
    pointer_event(&mut view, apply, 1.0, ElementState::Pressed);
    pointer_event(&mut view, apply, 1.0, ElementState::Released);
    assert!(view.color_editor.is_some());
    assert!(view.take_edit().is_none());
    assert!(replace_color(&mut view, "#123456"));
    view.paint(&mut raster, theme());
    let apply = view.color_geometry.apply;
    pointer_event(&mut view, apply, 1.0, ElementState::Pressed);
    view.refresh(color_catalog(3, true));
    pointer_event(&mut view, apply, 1.0, ElementState::Released);
    assert!(view.color_editor.is_none());
    assert!(view.take_edit().is_none());
}

fn workflow_numeric_view() -> SettingsView {
    let entries = (0..5)
        .map(|index| {
            let mut entry = SettingDescriptor::boolean(
                SettingId::new(format!("workflow.number.n{index}")).unwrap(),
                Section::Customizations,
                format!("Number {index}"),
                "Enter a value.",
                true,
                true,
            );
            entry.kind = SettingKind::Number {
                min: 0.0,
                max: 300.0,
                step: 1.0,
            };
            entry.value = SettingValue::Number(100.0);
            entry.default = SettingValue::Number(100.0);
            entry
        })
        .collect();
    let mut view = SettingsView::default();
    view.fit(720.0, 560.0, 14.0);
    view.open(Catalog::new(1, entries).unwrap());
    view.paint(&mut Raster::new(1.0), theme());
    view
}

fn workflow_tag_view() -> SettingsView {
    let base = rio_backend::config::Config::default();
    let preferences = crate::automexia::preferences::UserPreferences::default();
    let mut view = SettingsView::default();
    view.fit(320.0, 420.0, 16.0);
    view.open_customizations_with_slots(
        crate::settings_catalog::catalog(
            1,
            &base,
            &preferences,
            &crate::settings_catalog::test_installed_extensions(),
        )
        .unwrap(),
        None,
        Some(crate::settings_catalog::slot_page_snapshot(
            &preferences,
            &base,
        )),
    );
    assert!(view
        .view
        .as_mut()
        .unwrap()
        .focus(&SettingId::new("tags.enabled").unwrap()));
    view.focus = Focus::List;
    named(&mut view, NamedKey::Enter);
    view.paint(&mut Raster::new(1.0), theme());
    view
}

fn preview_edit_key(view: &mut SettingsView) {
    view.key(
        &Key::Character("e".into()),
        Some("e"),
        ModifiersState::empty(),
        false,
    );
}

#[test]
fn preview_keyboard_shortcut_enters_without_traversing_the_sheet() {
    for focus in [
        Focus::List,
        Focus::PreviewButton,
        Focus::Reset,
        Focus::Restore,
        Focus::Close,
    ] {
        let mut view = workflow_tag_view();
        view.focus = focus;
        preview_edit_key(&mut view);
        assert_eq!(view.focus, Focus::Preview);
        assert!(view.preview_edit_mode);
        assert_eq!(view.title(), "Information tags");
        assert!(view.take_edit().is_none());
        assert!(view.take_customization_intent().is_none());
    }
}

#[test]
fn preview_keyboard_tab_wraps_and_reveals_every_tag_without_leaving_preview() {
    for (width, height) in [(960.0, 620.0), (320.0, 420.0)] {
        let mut view = workflow_tag_view();
        view.fit(width, height, 16.0);
        view.paint(&mut Raster::new(1.0), theme());
        // Button activation must have exactly the same focus ownership as E.
        view.focus = Focus::PreviewButton;
        named(&mut view, NamedKey::Enter);
        named(&mut view, NamedKey::Home);
        let mut expected: Vec<_> = automexia_ui_model::information_bar::STANDARD_ROLES
            .into_iter()
            .map(|role| {
                format!(
                    "tags.slot.{}.page",
                    automexia_ui_model::information_bar::role_id(role)
                )
            })
            .collect();
        expected.push("tags.add-slot".into());
        for id in expected.iter().chain(expected.iter()) {
            view.paint(&mut Raster::new(1.0), theme());
            assert_eq!(view.focus, Focus::Preview);
            assert!(view.preview_edit_mode);
            assert_eq!(view.preview_selected.as_ref().unwrap().as_str(), id);
            assert!(view
                .preview_targets
                .iter()
                .any(|(key, _)| key.as_str() == id));
            named(&mut view, NamedKey::Tab);
            assert!(
                view.take_edit().is_none(),
                "navigation must never activate Add"
            );
        }
        view.key(
            &Key::Named(NamedKey::Tab),
            None,
            ModifiersState::SHIFT,
            false,
        );
        assert_eq!(
            view.preview_selected.as_ref().unwrap().as_str(),
            "tags.add-slot"
        );
        named(&mut view, NamedKey::Escape);
        assert_eq!(view.title(), "Information tags");
        assert_eq!(view.focus, Focus::PreviewButton);
        assert!(!view.preview_edit_mode);
        named(&mut view, NamedKey::Tab);
        assert_eq!(view.focus, Focus::Reset);
    }
}

#[test]
fn preview_keyboard_detail_suspends_selection_and_e_restores_parent_preview() {
    let mut view = workflow_tag_view();
    view.start_preview_edit();
    let selected = SettingId::new("tags.slot.kubernetes.page").unwrap();
    view.preview_selected = Some(selected.clone());
    named(&mut view, NamedKey::Enter);
    assert_eq!(view.title(), "Tag slot: Kubernetes");
    assert_eq!(view.focus, Focus::List);
    assert!(!view.preview_edit_mode);
    named(&mut view, NamedKey::Tab);
    assert_eq!(view.focus, Focus::PreviewButton);
    preview_edit_key(&mut view);
    assert_eq!(view.title(), "Information tags");
    assert_eq!(view.focus, Focus::Preview);
    assert_eq!(view.preview_selected, Some(selected));
    named(&mut view, NamedKey::Enter);
    named(&mut view, NamedKey::Escape);
    assert_eq!(view.title(), "Information tags");
    assert_eq!(view.focus, Focus::List);
    assert!(!view.preview_edit_mode);
    named(&mut view, NamedKey::Escape);
    assert!(view.is_category_root());
}

#[test]
fn preview_keyboard_e_respects_text_drafts_modifiers_composition_and_repeats() {
    let mut view = workflow_tag_view();
    view.focus = Focus::Search;
    preview_edit_key(&mut view);
    assert_eq!(view.query(), "e");
    assert!(!view.preview_edit_mode);
    view.focus = Focus::List;
    for modifiers in [
        ModifiersState::CONTROL,
        ModifiersState::ALT,
        ModifiersState::SUPER,
    ] {
        view.key(&Key::Character("e".into()), Some("e"), modifiers, false);
        assert!(!view.preview_edit_mode);
    }
    view.key(
        &Key::Character("e".into()),
        Some("e"),
        ModifiersState::empty(),
        true,
    );
    assert!(!view.preview_edit_mode);
    view.preedit = "compose".into();
    preview_edit_key(&mut view);
    assert!(!view.preview_edit_mode);
    named(&mut view, NamedKey::Escape);
    assert!(view.preedit.is_empty());
    assert_eq!(view.title(), "Information tags");
    view.enter_preview_item(SettingId::new("tags.slot.windows.page").unwrap());
    let base = rio_backend::config::Config::default();
    let preferences = crate::settings_catalog::apply_edit(
        1,
        &base,
        &Default::default(),
        &[],
        &Edit {
            revision: 1,
            id: SettingId::new("tags.slot.windows.text").unwrap(),
            change: Change::Set(SettingValue::Choice("literal".into())),
        },
    )
    .unwrap();
    view.refresh_with_resources(
        crate::settings_catalog::catalog(2, &base, &preferences, &[]).unwrap(),
        None,
        Some(crate::settings_catalog::slot_page_snapshot(
            &preferences,
            &preferences.apply_to(&base),
        )),
    );
    view.view
        .as_mut()
        .unwrap()
        .focus(&SettingId::new("tags.slot.windows.literal").unwrap());
    named(&mut view, NamedKey::Enter);
    assert!(view.color_editor.is_some());
    assert!(view.paste("Nam"));
    preview_edit_key(&mut view);
    assert_eq!(view.color_editor.as_ref().unwrap().draft, "Name");
    assert!(!view.preview_edit_mode);
}

#[test]
fn preview_keyboard_preserves_numeric_and_color_editor_input_ownership() {
    let mut view = workflow_tag_view();
    view.view
        .as_mut()
        .unwrap()
        .focus(&SettingId::new("tags.spacing").unwrap());
    named(&mut view, NamedKey::Enter);
    assert!(view.numeric_editor.is_some());
    preview_edit_key(&mut view);
    assert!(view.numeric_editor.is_some());
    assert!(!view.preview_edit_mode);
    named(&mut view, NamedKey::Escape);
    view.enter_preview_item(SettingId::new("tags.slot.windows.page").unwrap());
    view.view
        .as_mut()
        .unwrap()
        .focus(&SettingId::new("tags.slot.windows.color").unwrap());
    named(&mut view, NamedKey::Enter);
    assert!(view.color_editor.is_some());
    assert!(view.paste("#ABCD"));
    preview_edit_key(&mut view);
    assert_eq!(view.color_editor.as_ref().unwrap().draft, "#ABCDe");
    named(&mut view, NamedKey::Escape);
    assert_eq!(view.title(), "Tag slot: Windows");
    assert!(!view.preview_edit_mode);
    preview_edit_key(&mut view);
    assert_eq!(view.title(), "Information tags");
    assert_eq!(view.focus, Focus::Preview);
}

#[test]
fn preview_keyboard_handles_empty_small_and_refreshed_views_without_losing_selection() {
    let mut view = workflow_tag_view();
    view.preview_order.clear(); // A feature has opened before its first paint.
    preview_edit_key(&mut view);
    named(&mut view, NamedKey::Tab);
    assert_eq!(view.focus, Focus::Preview);
    assert!(view.preview_selected.is_none());
    view.paint(&mut Raster::new(1.0), theme());
    named(&mut view, NamedKey::Home);
    named(&mut view, NamedKey::ArrowRight);
    let selected = view.preview_selected.clone();
    let base = rio_backend::config::Config::default();
    let saved = crate::automexia::preferences::UserPreferences::default();
    view.refresh_with_resources(
        crate::settings_catalog::catalog(2, &base, &saved, &[]).unwrap(),
        None,
        Some(crate::settings_catalog::slot_page_snapshot(&saved, &base)),
    );
    view.fit(960.0, 620.0, 16.0);
    view.paint(&mut Raster::new(1.0), theme());
    assert_eq!(view.preview_selected, selected);
    assert_eq!(view.focus, Focus::Preview);
    view.fit(50.0, 50.0, 16.0);
    assert!(view.requires_larger_window());
    named(&mut view, NamedKey::Escape);
    assert!(!view.preview_edit_mode);
    preview_edit_key(&mut view);
    assert!(
        !view.preview_edit_mode,
        "invisible controls must not activate"
    );
    view.close();
    preview_edit_key(&mut view);
    assert!(!view.is_open());
}

#[test]
fn preview_keyboard_same_mode_is_used_for_colors_and_explicit_pointer_focus() {
    for category in [
        "terminal.command_output_highlighting",
        "terminal.kubernetes_highlighting",
    ] {
        let mut view = workflow_tag_view();
        view.back_to_categories();
        view.view
            .as_mut()
            .unwrap()
            .focus(&SettingId::new(category).unwrap());
        named(&mut view, NamedKey::Enter);
        view.paint(&mut Raster::new(1.0), theme());
        preview_edit_key(&mut view);
        named(&mut view, NamedKey::Home);
        let first = view.preview_selected.clone();
        named(&mut view, NamedKey::End);
        let last = view.preview_selected.clone();
        assert_ne!(first, last);
        named(&mut view, NamedKey::Tab);
        assert_eq!(view.preview_selected, first);
        view.key(
            &Key::Named(NamedKey::Tab),
            None,
            ModifiersState::SHIFT,
            false,
        );
        assert_eq!(view.preview_selected, last);
        named(&mut view, NamedKey::Enter);
        assert!(!view.preview_edit_mode);
        view.paint(&mut Raster::new(1.0), theme());
        let button = view.preview_button;
        pointer_event(&mut view, button, 1.0, ElementState::Pressed);
        pointer_event(&mut view, button, 1.0, ElementState::Released);
        assert!(view.customizations.as_ref().unwrap().active_slot.is_none());
        assert_eq!(view.preview_selected, last);
        assert_eq!(view.focus, Focus::Preview);
        view.paint(&mut Raster::new(1.0), theme());
        let search = view.geometry.search;
        pointer_event(&mut view, search, 1.0, ElementState::Pressed);
        pointer_event(&mut view, search, 1.0, ElementState::Released);
        assert_eq!(view.focus, Focus::Search);
        assert!(!view.preview_edit_mode);
        preview_edit_key(&mut view);
        assert_eq!(view.query(), "e");
    }
}

fn workflow_touch(view: &mut SettingsView, phase: TouchPhase, x: f32, y: f32) {
    // SAFETY: this device identity is confined to pure adapter test events.
    let device_id = unsafe { DeviceId::dummy() };
    assert!(
        view.event(
            &WindowEvent::Touch(rio_window::event::Touch {
                device_id,
                phase,
                location: rio_window::dpi::PhysicalPosition::new(
                    f64::from(x),
                    f64::from(y)
                ),
                force: None,
                id: 7,
            }),
            ModifiersState::empty(),
            1.0,
        )
        .consumed
    );
}

#[test]
fn workflow_clicking_another_number_transfers_draft_ownership_to_that_input() {
    let mut view = workflow_numeric_view();
    let first = view.rows[0].control;
    let second_id = view.rows[1].id.clone();
    pointer_event(&mut view, first, 1.0, ElementState::Pressed);
    pointer_event(&mut view, first, 1.0, ElementState::Released);
    assert!(view.paste("25"));
    view.paint(&mut Raster::new(1.0), theme());
    let second = view
        .rows
        .iter()
        .find(|row| row.id == second_id)
        .unwrap()
        .control;
    pointer_event(&mut view, second, 1.0, ElementState::Pressed);
    pointer_event(&mut view, second, 1.0, ElementState::Released);
    assert_eq!(view.numeric_editor.as_ref().unwrap().id, second_id);
    assert!(view.paste("75"));
    named(&mut view, NamedKey::Enter);
    let edit = view
        .take_edit()
        .expect("the second number must accept its own draft");
    assert_eq!(edit.id, second_id);
    assert_eq!(edit.change, Change::Set(SettingValue::Number(75.0)));
    assert!(view.take_edit().is_none());
}

#[test]
fn workflow_touch_scrolls_the_tag_roster_without_a_mouse_and_reaches_add() {
    let mut view = workflow_tag_view();
    assert!(view.pointer.is_none());
    let sample_scroll = view.preview_scroll;
    let area = view.preview_tag_list_area;
    let x = area.x + area.width * 0.5;
    let start_y = area.y + area.height * 0.8;
    let end_y = area.y + area.height * 0.2;
    workflow_touch(&mut view, TouchPhase::Started, x, start_y);
    workflow_touch(&mut view, TouchPhase::Moved, x, end_y);
    workflow_touch(&mut view, TouchPhase::Ended, x, end_y);
    assert!(
        view.preview_tag_list_scroll > 0,
        "touch must scroll the roster under the finger"
    );
    assert_eq!(
        view.preview_scroll, sample_scroll,
        "the separate graphic sample must stay still"
    );
    assert!(
        view.take_edit().is_none(),
        "a drag must never activate its starting tag"
    );
    let mut add = None;
    for _ in 0..16 {
        view.paint(&mut Raster::new(1.0), theme());
        add = view
            .preview_targets
            .iter()
            .find(|(id, bounds)| {
                id.as_str() == "tags.add-slot" && bounds.y >= view.preview_tag_list_area.y
            })
            .map(|(_, bounds)| *bounds);
        if add.is_some() {
            break;
        }
        workflow_touch(&mut view, TouchPhase::Started, x, start_y);
        workflow_touch(&mut view, TouchPhase::Moved, x, end_y);
        workflow_touch(&mut view, TouchPhase::Ended, x, end_y);
    }
    let add = add.expect("all roster entries, including Add, must be reachable by touch");
    let x = add.x + add.width * 0.5;
    let y = add.y + add.height * 0.5;
    workflow_touch(&mut view, TouchPhase::Started, x, y);
    workflow_touch(&mut view, TouchPhase::Ended, x, y);
    assert_eq!(view.take_edit().unwrap().id.as_str(), "tags.add-slot");
}

#[test]
fn workflow_horizontal_wheel_and_stationary_touch_do_not_scroll_preview_surfaces() {
    // SAFETY: this ID is used only to construct pure adapter events.
    let device_id = unsafe { DeviceId::dummy() };
    for roster in [false, true] {
        for delta in [
            MouseScrollDelta::LineDelta(3.0, 0.0),
            MouseScrollDelta::PixelDelta(rio_window::dpi::PhysicalPosition::new(
                12.0, 0.0,
            )),
        ] {
            let mut view = workflow_tag_view();
            let area = if roster {
                view.preview_tag_list_area
            } else {
                view.preview_button
            };
            view.pointer = Some((area.x + area.width * 0.5, area.y + area.height * 0.5));
            let before = (view.preview_scroll, view.preview_tag_list_scroll);
            view.event(
                &WindowEvent::MouseWheel {
                    device_id,
                    delta,
                    phase: TouchPhase::Moved,
                },
                ModifiersState::empty(),
                1.0,
            );
            assert_eq!((view.preview_scroll, view.preview_tag_list_scroll), before,
                "horizontal wheel input must not become vertical scrolling (roster={roster})");
        }
        let mut view = workflow_tag_view();
        let area = if roster {
            view.preview_tag_list_area
        } else {
            view.preview_button
        };
        let x = area.x + area.width * 0.5;
        let y = area.y + area.height * 0.5;
        let before = (view.preview_scroll, view.preview_tag_list_scroll);
        workflow_touch(&mut view, TouchPhase::Started, x, y);
        workflow_touch(&mut view, TouchPhase::Moved, x, y);
        workflow_touch(&mut view, TouchPhase::Cancelled, x, y);
        assert_eq!((view.preview_scroll, view.preview_tag_list_scroll), before);
        assert!(view.take_edit().is_none());
    }
}

#[test]
fn workflow_touch_keeps_its_scroll_owner_when_crossing_the_preview_boundary() {
    let mut view = workflow_tag_view();
    let area = view.preview_tag_list_area;
    let x = area.x + area.width * 0.5;
    workflow_touch(&mut view, TouchPhase::Started, x, area.y + 8.0);
    // Moving into the sample must still scroll the list where the gesture began.
    workflow_touch(&mut view, TouchPhase::Moved, x, area.y - 20.0);
    assert!(view.preview_tag_list_scroll > 0);
    assert_eq!(view.preview_scroll, 0);
    workflow_touch(&mut view, TouchPhase::Cancelled, x, area.y - 20.0);
    assert!(view.touch.is_none());
    assert!(view.pressed.is_none());
    workflow_touch(&mut view, TouchPhase::Started, f32::NAN, area.y);
    assert!(view.touch.is_none());
}

#[test]
fn workflow_numeric_escape_cancels_composition_before_the_draft() {
    for composing in ["6", "１２"] {
        let mut view = workflow_numeric_view();
        let input = view.rows[0].control;
        pointer_event(&mut view, input, 1.0, ElementState::Pressed);
        pointer_event(&mut view, input, 1.0, ElementState::Released);
        assert!(view.paste("25"));
        view.event(
            &WindowEvent::Ime(Ime::Preedit(composing.into(), None)),
            ModifiersState::empty(),
            1.0,
        );
        named(&mut view, NamedKey::Escape);
        assert_eq!(
            view.numeric_editor
                .as_ref()
                .map(|editor| editor.draft.as_str()),
            Some("25"),
            "first Escape must retain the numeric draft, including rejected composition"
        );
        assert!(view.preedit.is_empty());
        assert!(view.take_edit().is_none());
        named(&mut view, NamedKey::Escape);
        assert!(view.numeric_editor.is_none());
        assert!(view.is_open());
        view.event(
            &WindowEvent::Ime(Ime::Commit("6".into())),
            ModifiersState::empty(),
            1.0,
        );
        assert!(
            view.numeric_editor.is_none(),
            "a late IME commit must not reopen the canceled editor"
        );
        assert!(view.take_edit().is_none());
        view.key(
            &Key::Character("7".into()),
            Some("7"),
            ModifiersState::empty(),
            false,
        );
        assert_eq!(
            view.numeric_editor.as_ref().unwrap().draft,
            "7",
            "new intentional typing must still open a number"
        );
    }
}

#[test]
fn workflow_same_number_click_preserves_draft_and_clipboard_cut_is_explicit() {
    let mut view = workflow_numeric_view();
    let input = view.rows[0].control;
    pointer_event(&mut view, input, 1.0, ElementState::Pressed);
    pointer_event(&mut view, input, 1.0, ElementState::Released);
    assert!(view.paste("25"));
    view.paint(&mut Raster::new(1.0), theme());
    pointer_event(&mut view, input, 1.0, ElementState::Pressed);
    pointer_event(&mut view, input, 1.0, ElementState::Released);
    assert_eq!(view.numeric_editor.as_ref().unwrap().draft, "25");
    view.key(
        &Key::Character("a".into()),
        None,
        ModifiersState::CONTROL,
        false,
    );
    assert_eq!(view.clipboard_selection().as_deref(), Some("25"));
    assert_eq!(view.numeric_editor.as_ref().unwrap().draft, "25");
    assert!(view.paste(""));
    assert!(!view.commit_numeric());
    assert!(view.take_edit().is_none());
    named(&mut view, NamedKey::Escape);
    named(&mut view, NamedKey::Enter);
    view.event(
        &WindowEvent::Ime(Ime::Commit("75".into())),
        ModifiersState::empty(),
        1.0,
    );
    named(&mut view, NamedKey::Enter);
    assert_eq!(
        view.take_edit().unwrap().change,
        Change::Set(SettingValue::Number(75.0))
    );
}

#[test]
fn workflow_reopened_sheet_restores_writer_feedback_without_overriding_temporary_defaults(
) {
    use crate::automexia::preferences::{
        PreferenceSaveStatus, PreferenceWriter, UserPreferences,
    };
    let root = tempfile::tempdir().unwrap();
    let mut writer = PreferenceWriter::new(root.path().into());
    assert_eq!(
        writer.submit(UserPreferences {
            font_size: Some(0.0),
            ..Default::default()
        }),
        0
    );
    assert!(writer.take_error().is_some());
    let mut view = opened();
    view.restore_save_status(writer.save_status());
    let failure = view.status.clone();
    assert!(failure.contains("session"));
    view.close();
    view.open(catalog(2));
    view.restore_save_status(writer.save_status());
    assert_eq!(view.status, failure);
    view.restore_save_status(PreferenceSaveStatus::Pending(7));
    assert_eq!(view.saving, Some(7));
    view.save_completed(6, true);
    assert_eq!(view.saving, Some(7));
    view.save_completed(7, true);
    assert_eq!(view.status, "Saved");
    view.set_temporary_customizations(true);
    let temporary = view.status.clone();
    view.restore_save_status(PreferenceSaveStatus::Failed);
    assert_eq!(view.status, temporary);
    assert!(view.saving.is_none());
}

#[test]
fn workflow_active_numeric_selection_and_ime_stay_clipped_during_scrolling() {
    let mut view = workflow_numeric_view();
    // Compact rows fit the old viewport without scrolling. Keep this fixture
    // deliberately short so it still exercises a partially clipped editor.
    view.fit(720.0, 360.0, 14.0);
    view.paint(&mut Raster::new(1.0), theme());
    let id = view.rows[1].id.clone();
    let input = view.rows[1].control;
    pointer_event(&mut view, input, 1.0, ElementState::Pressed);
    pointer_event(&mut view, input, 1.0, ElementState::Released);
    view.paint(&mut Raster::new(1.0), theme());
    let input = view.rows.iter().find(|row| row.id == id).unwrap().control;
    let delta = input.y - view.geometry.body.y + input.height * 0.5;
    // SAFETY: this ID is confined to pure adapter wheel events.
    let device_id = unsafe { DeviceId::dummy() };
    view.event(
        &WindowEvent::MouseWheel {
            device_id,
            delta: MouseScrollDelta::PixelDelta(rio_window::dpi::PhysicalPosition::new(
                0.0,
                -f64::from(delta),
            )),
            phase: TouchPhase::Moved,
        },
        ModifiersState::empty(),
        1.0,
    );
    let mut raster = Raster::new(1.0);
    view.paint(&mut raster, theme());
    let input = numeric_zones(
        view.rows.iter().find(|row| row.id == id).unwrap().control,
        view.font,
    )
    .1;
    let body = view.geometry.body;
    assert!(
        input.y < body.y && input.y + input.height > body.y,
        "fixture must partly clip the active numeric input"
    );
    let decorations: Vec<_> = raster
        .rects
        .iter()
        .filter(|(bounds, color)| {
            *color == theme().outline
                && bounds[2] < 40.0
                && bounds[3] > 4.0
                && bounds[0] >= input.x
                && bounds[0] < input.x + input.width
                && bounds[1] >= input.y
                && bounds[1] < input.y + input.height
        })
        .collect();
    assert!(
        !decorations.is_empty(),
        "the visible numeric selection/caret must be painted"
    );
    for (bounds, _) in decorations {
        assert!(
            bounds[1] >= body.y && bounds[1] + bounds[3] <= body.y + body.height,
            "numeric selection or caret escaped list clipping: {bounds:?}, body={body:?}"
        );
    }
    let ime = view
        .ime_cursor_area()
        .expect("partially visible input retains an IME anchor");
    assert!(
        ime[1] >= body.y && ime[1] + ime[3] <= body.y + body.height,
        "IME anchor must describe only the visible input: {ime:?}"
    );
}

#[test]
fn workflow_search_shift_selection_replaces_whole_graphemes() {
    for command in [NamedKey::ArrowLeft, NamedKey::Home, NamedKey::End] {
        let mut view = opened();
        assert!(view.paste("界éx"));
        if command == NamedKey::End {
            named(&mut view, NamedKey::Home);
        }
        view.key(&Key::Named(command), None, ModifiersState::SHIFT, false);
        assert!(view.paste("Q"));
        assert_eq!(
            view.query(),
            if command == NamedKey::ArrowLeft {
                "界éQ"
            } else {
                "Q"
            }
        );
    }
}

#[test]
fn workflow_save_failure_does_not_recommend_resetting_user_choices() {
    let mut view = opened();
    view.save_failed();
    assert!(view.status.contains("could not be saved"));
    assert!(
        !view.status.contains("Reset"),
        "temporary defaults are not a retry-saving action"
    );
}

#[test]
fn workflow_color_editor_shows_clipboard_failure_and_clears_it_after_editing() {
    let mut view = opened_color(false);
    let mut before = Raster::new(1.0);
    view.paint(&mut before, theme());
    view.set_status("Could not copy. Selection kept; try again.");
    let mut after = Raster::new(1.0);
    view.paint(&mut after, theme());
    assert!(
        before.pixels(720, 560, true) != after.pixels(720, 560, true),
        "an editor clipboard error must be visible without closing the modal"
    );
    assert_eq!(view.clipboard_selection().as_deref(), Some("#112233"));
    assert!(view.paste("#112233"));
    // Reselect the same text to compare the same focus/caret pixels.
    view.key(
        &Key::Character("a".into()),
        None,
        ModifiersState::CONTROL,
        false,
    );
    let mut recovered = Raster::new(1.0);
    view.paint(&mut recovered, theme());
    assert!(
        before.pixels(720, 560, true) == recovered.pixels(720, 560, true),
        "successful editing must restore normal help"
    );
}

#[test]
fn workflow_output_severity_footer_names_the_color_reset_action() {
    let base = rio_backend::config::Config::default();
    let mut view = SettingsView::default();
    view.fit(960.0, 620.0, 16.0);
    view.open_with_section(
        crate::settings_catalog::catalog(1, &base, &Default::default(), &[]).unwrap(),
        Some(Section::Customizations),
    );
    assert!(view.view.as_mut().unwrap().focus(
        &SettingId::new(automexia_ui_model::settings::COMMAND_OUTPUT_HIGHLIGHTING)
            .unwrap()
    ));
    view.focus = Focus::List;
    named(&mut view, NamedKey::Enter);
    view.paint(&mut Raster::new(1.0), theme());
    view.start_preview_edit();
    view.preview_selected = Some(SettingId::new("output.severity.error").unwrap());
    named(&mut view, NamedKey::Enter);
    assert_eq!(view.title(), "Error output colors");
    let mut actual = Raster::new(1.0);
    view.paint(&mut actual, theme());
    let bounds = view.geometry.reset;
    let mut expected = Raster::new(1.0);
    // Compare the caption's glyphs independently of the rounded surface and
    // shortcut keycap. A generic Reset caption must still fail this oracle.
    let caption_font = 14.4;
    let caption = Rect {
        x: bounds.x + 6.0,
        y: bounds.y + (bounds.height - caption_font * 1.45) * 0.5,
        width: 140.0,
        height: caption_font * 1.45,
    };
    label(
        &mut expected,
        caption,
        "Reset colors",
        caption_font,
        theme().text,
        false,
        bounds,
    );
    let mut actual_pixels = vec![0; 960 * 620];
    let mut expected_pixels = vec![0; 960 * 620];
    actual.text.render_cpu_base(&mut actual_pixels, 960, 620);
    expected
        .text
        .render_cpu_base(&mut expected_pixels, 960, 620);
    actual.text.render_cpu_modal(&mut actual_pixels, 960, 620);
    expected
        .text
        .render_cpu_modal(&mut expected_pixels, 960, 620);
    assert!(expected_pixels.iter().any(|pixel| *pixel != 0));
    for y in caption.y.ceil() as usize..(caption.y + caption.height).floor() as usize {
        for x in caption.x.ceil() as usize..(caption.x + caption.width).floor() as usize {
            assert_eq!(
                actual_pixels[y * 960 + x],
                expected_pixels[y * 960 + x],
                "output color footer must say Reset colors"
            );
        }
    }
    pointer_event(&mut view, bounds, 1.0, ElementState::Pressed);
    pointer_event(&mut view, bounds, 1.0, ElementState::Released);
    confirm_requested_settings_action(&mut view);
    assert!(
        matches!(view.take_customization_intent(), Some(CustomizationIntent::Reset {
        scope: CustomizationResetScope::OutputSeverity(ref severity), ..
    }) if severity == "error")
    );
}

#[test]
fn workflow_table_preview_uses_the_selected_header_background_rgba() {
    let base = rio_backend::config::Config::default();
    let mut original = crate::automexia::preferences::UserPreferences::default();
    let log_color = [220, 10, 20, 111];
    original.visual.highlight.warning_background = Some(
        rio_backend::config::presentation::Rgba::from_bytes(log_color),
    );
    let changed = crate::settings_catalog::apply_edit(
        1,
        &base,
        &original,
        &[],
        &Edit {
            revision: 1,
            id: SettingId::new("tables.backgrounds.header").unwrap(),
            change: Change::Set(SettingValue::Color([10, 200, 30, 180])),
        },
    )
    .unwrap();
    let mut view = SettingsView::default();
    view.fit(960.0, 620.0, 16.0);
    view.open_customizations_with_slots(
        crate::settings_catalog::catalog(2, &base, &changed, &[]).unwrap(),
        None,
        Some(crate::settings_catalog::slot_page_snapshot(
            &changed,
            &changed.apply_to(&base),
        )),
    );
    assert!(view
        .view
        .as_mut()
        .unwrap()
        .focus(&SettingId::new(INLINE_TABLES).unwrap()));
    view.focus = Focus::List;
    named(&mut view, NamedKey::Enter);
    let mut raster = Raster::new(1.0);
    view.paint(&mut raster, theme());
    let alpha = 180.0 / 255.0;
    let expected = [10.0 / 255.0, 200.0 / 255.0, 30.0 / 255.0, 1.0];
    let expected = std::array::from_fn::<_, 4, _>(|i| {
        if i == 3 {
            1.0
        } else {
            expected[i] * alpha + base.colors.background.0[i] * (1.0 - alpha)
        }
    });
    assert!(
        raster.rects.iter().any(|(bounds, color)| {
            bounds[0] >= view.geometry.preview.x
                && bounds[1] >= view.geometry.preview.y
                && color
                    .iter()
                    .zip(expected)
                    .all(|(actual, expected)| (actual - expected).abs() < 0.001)
        }),
        "table header must preview the configured background with its opacity"
    );
    let log_color = log_color.map(|channel| f32::from(channel) / 255.0);
    assert!(
        !raster.rects.iter().any(|(_, color)| color
            .iter()
            .zip(log_color)
            .all(|(actual, expected)| (actual - expected).abs() < 0.001)),
        "the generic log palette must not color the ordinary table sample"
    );
}

#[test]
fn workflow_font_preview_labels_and_draws_the_supported_size_boundaries() {
    let base = rio_backend::config::Config::default();
    for size in [6.0, 100.0] {
        let preferences = crate::automexia::preferences::UserPreferences {
            font_size: Some(size),
            ..Default::default()
        };
        let mut view = SettingsView::default();
        view.fit(960.0, 620.0, 16.0);
        view.open_customizations(
            crate::settings_catalog::catalog(1, &base, &preferences, &[]).unwrap(),
            None,
        );
        let sample = Rect {
            x: 10.0,
            y: 10.0,
            width: 700.0,
            height: 240.0,
        };
        let mut actual = Raster::new(1.0);
        view.paint_font_preview(&mut actual, sample, theme());
        let mut expected = Raster::new(1.0);
        let row = Rect {
            y: sample.y + 11.2 * 1.8,
            height: size,
            ..sample
        };
        rect(&mut expected, sample, theme().background, sample);
        label(
            &mut expected,
            row,
            "Aa 0123 λ",
            size,
            theme().text,
            false,
            sample,
        );
        let actual_pixels = actual.pixels(720, 260, true);
        let expected_pixels = expected.pixels(720, 260, true);
        for y in (row.y.ceil() as usize)..((row.y + row.height).floor() as usize).min(250)
        {
            assert_eq!(
                &actual_pixels[y * 720 + 10..y * 720 + 710],
                &expected_pixels[y * 720 + 10..y * 720 + 710],
                "preview must draw the actual {size} pt text without silently scaling it"
            );
        }
    }
}

#[test]
fn fonts_menu_navigation_edit_refresh_reset_restore_and_scaled_preview() {
    use crate::automexia::font_preferences::FontColor;
    use rio_backend::config::presentation::Rgb;
    let base = rio_backend::config::Config::default();
    let mut original = crate::automexia::preferences::UserPreferences::default();
    original
        .fonts
        .colors
        .insert(FontColor::Foreground, Rgb::from_bytes([220, 210, 170]));
    original.fonts.line_height = Some(1.4);
    for (width, height, scale) in [
        (960.0, 620.0, 1.0),
        (640.0, 480.0, 1.25),
        (1280.0, 800.0, 2.0),
    ] {
        let mut view = SettingsView::default();
        view.fit(width, height, 16.0);
        view.open_customizations_with_slots(
            crate::settings_catalog::catalog(1, &base, &original, &[]).unwrap(),
            None,
            Some(crate::settings_catalog::slot_page_snapshot_with_config(
                &original,
                &original.apply_to(&base),
                &base,
                &crate::settings_catalog::test_installed_extensions(),
            )),
        );
        view.show_terminal_appearance();
        assert!(view
            .view
            .as_mut()
            .unwrap()
            .focus(&SettingId::new(automexia_ui_model::settings::FONT_SIZE).unwrap()));
        view.focus = Focus::List;
        named(&mut view, NamedKey::Enter);
        assert_eq!(view.title(), "Fonts");
        assert_eq!(view.catalog.as_ref().unwrap().entries().len(), 34);
        let id = SettingId::new("fonts.line-height").unwrap();
        assert!(view.view.as_mut().unwrap().focus(&id));
        named(&mut view, NamedKey::ArrowRight);
        let edit = view.take_edit().unwrap();
        assert_eq!(edit.id, id);
        let changed =
            crate::settings_catalog::apply_edit(1, &base, &original, &[], &edit).unwrap();
        assert!((changed.fonts.line_height.unwrap() - 1.5).abs() < 0.001);
        view.refresh_with_resources(
            crate::settings_catalog::catalog(2, &base, &changed, &[]).unwrap(),
            None,
            Some(crate::settings_catalog::slot_page_snapshot_with_config(
                &changed,
                &changed.apply_to(&base),
                &base,
                &crate::settings_catalog::test_installed_extensions(),
            )),
        );
        assert_eq!(view.title(), "Fonts");
        let mut raster = Raster::new(scale);
        view.paint(&mut raster, theme());
        let (w, h) = ((width * scale) as u32, (height * scale) as u32);
        let pixels = raster.pixels(w, h, true);
        if let Some(directory) = std::env::var_os("AUTOMEXIA_SETTINGS_PREVIEW_DIR") {
            let directory = std::path::PathBuf::from(directory);
            std::fs::create_dir_all(&directory).unwrap();
            image_rs::RgbImage::from_fn(w, h, |x, y| {
                let pixel = pixels[(y * w + x) as usize];
                image_rs::Rgb([(pixel >> 16) as u8, (pixel >> 8) as u8, pixel as u8])
            })
            .save(directory.join(format!("fonts-{w}x{h}.png")))
            .unwrap();
        }
        view.activate_target(Target::Reset);
        assert!(view.confirmation.is_some());
        named(&mut view, NamedKey::Escape);
        assert!(view.take_customization_intent().is_none());
        view.activate_target(Target::Reset);
        confirm_requested_settings_action(&mut view);
        assert_eq!(
            view.take_customization_intent(),
            Some(CustomizationIntent::Reset {
                revision: 2,
                scope: CustomizationResetScope::Group(
                    SettingId::new(automexia_ui_model::settings::FONT_SIZE).unwrap()
                )
            })
        );
        view.set_temporary_customizations(true);
        view.activate_target(Target::Restore);
        confirm_requested_settings_action(&mut view);
        assert_eq!(
            view.take_customization_intent(),
            Some(CustomizationIntent::RestoreSaved)
        );
        named(&mut view, NamedKey::Escape);
        assert_eq!(view.title(), "Terminal Appearance");
    }
}

#[test]
fn installed_font_picker_opens_from_the_real_fonts_control() {
    let mut view = installed_font_picker();
    assert!(
        view.color_editor.is_none(),
        "Font family must open a font list, not the manual text editor"
    );
    assert!(view.accessibility_summary().contains("Choose font"));
    assert!(
        view.take_edit().is_none(),
        "Opening the picker must not save a preference"
    );
}

fn installed_font_picker() -> SettingsView {
    let base = rio_backend::config::Config::default();
    let preferences = crate::automexia::preferences::UserPreferences::default();
    let mut view = SettingsView::default();
    view.fit(960.0, 620.0, 16.0);
    view.open_customizations_with_slots(
        crate::settings_catalog::catalog(1, &base, &preferences, &[]).unwrap(),
        None,
        Some(crate::settings_catalog::slot_page_snapshot_with_config(
            &preferences,
            &base,
            &base,
            &crate::settings_catalog::test_installed_extensions(),
        )),
    );
    view.show_terminal_appearance();
    view.view
        .as_mut()
        .unwrap()
        .focus(&SettingId::new(automexia_ui_model::settings::FONT_SIZE).unwrap());
    view.focus = Focus::List;
    named(&mut view, NamedKey::Enter);
    view.view
        .as_mut()
        .unwrap()
        .focus(&SettingId::new("fonts.family").unwrap());
    named(&mut view, NamedKey::Enter);
    view
}

#[test]
fn installed_font_picker_search_preview_apply_cancel_and_reopen() {
    let mut view = installed_font_picker();
    let original = view.font_picker_session().unwrap();
    assert!(matches!(
        view.take_font_picker_intent(),
        Some(FontPickerIntent::Load)
    ));
    // Typing while discovery is active must not cancel the inventory worker.
    assert!(view.paste("Mono"));
    assert!(view.take_font_picker_intent().is_none());
    view.font_picker_inventory(
        vec![
            "Example Mono".into(),
            "Other Mono".into(),
            "Example Sans".into(),
        ],
        false,
    );
    assert_eq!(view.font_picker_selection(), Some("Example Mono"));
    named(&mut view, NamedKey::ArrowDown);
    assert!(
        matches!(view.take_font_picker_intent(),Some(FontPickerIntent::Preview(s)) if s=="Example Mono")
    );
    named(&mut view, NamedKey::ArrowDown);
    assert!(
        matches!(view.take_font_picker_intent(),Some(FontPickerIntent::Preview(s)) if s=="Other Mono")
    );
    named(&mut view, NamedKey::ArrowDown);
    assert_eq!(view.font_picker_selection(), Some("Example Mono"));
    named(&mut view, NamedKey::Enter);
    assert!(
        matches!(view.take_font_picker_intent(),Some(FontPickerIntent::Apply(s)) if s=="Example Mono")
    );
    assert!(
        view.take_edit().is_none(),
        "Only the application may validate/save prepared fonts"
    );
    named(&mut view, NamedKey::Escape);
    assert!(view.font_picker_session().is_none());
    assert!(matches!(
        view.take_font_picker_intent(),
        Some(FontPickerIntent::Cancel)
    ));
    assert_eq!(view.title(), "Fonts");
    named(&mut view, NamedKey::Enter);
    assert!(view.font_picker_session().unwrap().0 > original.0);
    assert_eq!(view.query(), "");
}

#[test]
fn installed_font_picker_empty_search_clipboard_ime_and_compact_isolation() {
    let mut view = installed_font_picker();
    view.take_font_picker_intent();
    view.font_picker_inventory(vec!["字体 Mono".into(), "Other Mono".into()], false);
    view.event(
        &WindowEvent::Ime(Ime::Preedit("字体".into(), Some((0, 6)))),
        ModifiersState::empty(),
        1.0,
    );
    named(&mut view, NamedKey::Enter);
    assert!(view.take_font_picker_intent().is_none());
    view.event(
        &WindowEvent::Ime(Ime::Commit("字体".into())),
        ModifiersState::empty(),
        1.0,
    );
    assert_eq!(view.query(), "字体");
    assert_eq!(view.font_picker_selection(), Some("字体 Mono"));
    view.key(
        &Key::Character("a".into()),
        None,
        ModifiersState::CONTROL,
        false,
    );
    assert_eq!(view.clipboard_selection(), Some("字体".into()));
    assert!(view.paste("No such family"));
    assert_eq!(view.font_picker_selection(), None);
    assert!(matches!(
        view.take_font_picker_intent(),
        Some(FontPickerIntent::Cancel)
    ));
    named(&mut view, NamedKey::Enter);
    assert!(view.take_font_picker_intent().is_none());
    assert!(!view.paste("\n"));
    assert!(!view.paste(&"a".repeat(MAX_QUERY_BYTES + 1)));
    view.fit(200.0, 180.0, 16.0);
    assert!(!view.paste("Mono"));
    named(&mut view, NamedKey::Enter);
    assert!(view.take_font_picker_intent().is_none());
    named(&mut view, NamedKey::Escape);
    assert!(view.font_picker_session().is_none());
}

#[test]
fn installed_font_picker_selection_highlight_clips_and_accessibility_follow_focus() {
    for entry in crate::automexia::theme_gallery::builtins() {
        for (width, height, scale) in [
            (480.0, 560.0, 1.0),
            (960.0, 620.0, 1.5),
            (1920.0, 1080.0, 2.0),
        ] {
            let mut view = installed_font_picker();
            view.fit(width, height, 16.0);
            view.font_picker_inventory(
                vec!["Example Mono".into(), "Other Mono".into()],
                false,
            );
            named(&mut view, NamedKey::ArrowDown);
            let mut raster = Raster::new(scale);
            let theme = UiTheme::from_colors(&entry.theme.as_ref().unwrap().colors);
            view.paint(&mut raster, theme);
            let selected = view
                .font_picker
                .as_ref()
                .unwrap()
                .targets
                .iter()
                .find_map(|(target, bounds)| {
                    (*target
                        == FontPickerTarget::Row(
                            view.font_picker.as_ref().unwrap().selected,
                        ))
                    .then_some(*bounds)
                })
                .unwrap();
            assert!(
                raster
                    .rects
                    .iter()
                    .any(|(bounds, color)| *bounds == selected.array()
                        && *color == theme.raised),
                "Full-row selection must be visible"
            );
            let card = view.geometry.card;
            for ([x, y, w, h], _) in raster.rects {
                assert!(
                    x >= card.x - 0.01
                        && y >= card.y - 0.01
                        && x + w <= card.x + card.width + 0.01
                        && y + h <= card.y + card.height + 0.01
                );
            }
            let surface = view
                .font_picker_accessibility_surface(
                    scale,
                    accesskit::Rect::new(
                        0.0,
                        0.0,
                        (width * scale) as f64,
                        (height * scale) as f64,
                    ),
                )
                .unwrap();
            assert!(surface
                .elements
                .iter()
                .any(|e| e.node.label() == Some("Example Mono")));
            // A pointer row selects a preview, never implicitly saves it.
            view.take_font_picker_intent();
            view.font_picker_activate(FontPickerTarget::Row(1));
            assert!(matches!(
                view.take_font_picker_intent(),
                Some(FontPickerIntent::Preview(_))
            ));
            assert!(view.take_edit().is_none());
        }
    }
}

#[test]
fn installed_font_picker_rejects_stale_revision_and_retains_original_search() {
    let mut view = installed_font_picker();
    view.font_picker_inventory(vec!["Example Mono".into()], false);
    view.invalidate_font_picker(2);
    assert!(view.font_picker_session().is_none());
    assert!(matches!(
        view.take_font_picker_intent(),
        Some(FontPickerIntent::Cancel)
    ));
    assert_eq!(view.title(), "Fonts");
}

#[test]
fn installed_font_picker_large_text_tab_order_and_hidden_controls() {
    let mut view = installed_font_picker();
    view.font_picker_inventory(vec!["Example Mono".into()], false);
    view.fit(1920.0, 1080.0, 32.0);
    assert!(!view.requires_larger_window());
    let mut raster = Raster::new(2.0);
    view.paint(&mut raster, theme());
    for focus in [
        Focus::List,
        Focus::PreviewButton,
        Focus::Reset,
        Focus::Close,
        Focus::Search,
    ] {
        named(&mut view, NamedKey::Tab);
        assert_eq!(view.focus, focus);
    }
    assert!(view
        .font_picker
        .as_ref()
        .unwrap()
        .targets
        .iter()
        .any(|(t, _)| *t == FontPickerTarget::Apply));
    view.fit(280.0, 180.0, 32.0);
    view.paint(&mut raster, theme());
    assert_eq!(view.font_picker.as_ref().unwrap().targets.len(), 1);
    assert_eq!(
        view.font_picker.as_ref().unwrap().targets[0].0,
        FontPickerTarget::Back
    );
}

fn package_notice_view() -> SettingsView {
    let mut view = SettingsView::default();
    view.fit(960.0, 620.0, 16.0);
    view.open_customizations(
        crate::settings_catalog::catalog(
            1,
            &rio_backend::config::Config::default(),
            &Default::default(),
            &[],
        )
        .unwrap(),
        None,
    );
    view
}

fn assert_package_notice_footer(view: &mut SettingsView, message: &str) {
    let mut actual = Raster::new(1.0);
    view.paint(&mut actual, theme());
    let bounds = view.geometry.status;
    let mut expected = Raster::new(1.0);
    if message.starts_with("Tab:") {
        shortcut_hint(
            &mut expected,
            bounds,
            message,
            view.font * 0.85,
            theme(),
            view.geometry.card,
        );
    } else {
        label(
            &mut expected,
            bounds,
            message,
            view.font * 0.85,
            theme().muted_text,
            false,
            view.geometry.card,
        );
    }
    let mut actual_pixels = vec![0; 960 * 620];
    let mut expected_pixels = vec![0; 960 * 620];
    actual.text.render_cpu_base(&mut actual_pixels, 960, 620);
    actual.text.render_cpu_modal(&mut actual_pixels, 960, 620);
    expected
        .text
        .render_cpu_base(&mut expected_pixels, 960, 620);
    expected
        .text
        .render_cpu_modal(&mut expected_pixels, 960, 620);
    assert!(expected_pixels.iter().any(|pixel| *pixel != 0));
    for y in bounds.y.ceil() as usize..(bounds.y + bounds.height).floor() as usize {
        for x in bounds.x.ceil() as usize..(bounds.x + bounds.width).floor() as usize {
            assert_eq!(
                actual_pixels[y * 960 + x],
                expected_pixels[y * 960 + x],
                "rendered package status must say {message}"
            );
        }
    }
}

#[test]
fn package_notice_async_failure_is_visible_and_core_controls_stay_editable() {
    let mut view = package_notice_view();
    view.set_package_inventory_status(PackageInventoryStatus::Loading, false);
    view.set_package_inventory_status(PackageInventoryStatus::Unavailable, false);
    let failure = "Package settings unavailable. Reopen to retry.";
    assert_package_notice_footer(&mut view, failure);
    assert!(view.accessibility_summary().contains(failure));
    assert!(view
        .view
        .as_mut()
        .unwrap()
        .focus(&SettingId::new(INLINE_TABLES).unwrap()));
    view.focus = Focus::List;
    named(&mut view, NamedKey::Enter);
    assert_package_notice_footer(&mut view, failure);
    named(&mut view, NamedKey::Space);
    assert!(
        matches!(view.take_edit(), Some(Edit { id, change: Change::Set(SettingValue::Boolean(false)), .. }) if id.as_str() == INLINE_TABLES)
    );
    view.set_package_inventory_status(PackageInventoryStatus::Ready, false);
    assert!(!view
        .accessibility_summary()
        .contains("Package settings unavailable"));
    assert_package_notice_footer(
        &mut view,
        "Tab: focus | Arrows: navigate | Esc / Backspace / Alt+Left: back",
    );
}

#[test]
fn package_notice_loading_and_projection_failure_clear_after_recovery_or_close() {
    let mut view = package_notice_view();
    for (status, failed, message) in [
        (
            PackageInventoryStatus::Loading,
            false,
            "Loading package settings...",
        ),
        (
            PackageInventoryStatus::Ready,
            true,
            "Package settings could not be displayed. Reopen to retry.",
        ),
    ] {
        view.set_package_inventory_status(status, failed);
        assert_package_notice_footer(&mut view, message);
        assert!(view.accessibility_summary().contains(message));
    }
    view.set_package_inventory_status(PackageInventoryStatus::Ready, false);
    assert_package_notice_footer(
        &mut view,
        "Tab: focus | Arrows: navigate | Esc / Backspace / Alt+Left: back",
    );
    view.set_package_inventory_status(PackageInventoryStatus::Unavailable, false);
    view.close();
    view.open(catalog(2));
    view.set_package_inventory_status(PackageInventoryStatus::Unavailable, false);
    assert!(!view.accessibility_summary().contains("Package settings"));
    assert_package_notice_footer(
        &mut view,
        "Tab: focus | Arrows: navigate | Esc / Backspace / Alt+Left: back",
    );
}

#[test]
fn package_notice_preserves_save_failure_pending_receipts_and_temporary_defaults() {
    let mut view = package_notice_view();
    view.save_failed();
    let failure = view.status.clone();
    for state in [
        PackageInventoryStatus::Loading,
        PackageInventoryStatus::Unavailable,
        PackageInventoryStatus::Ready,
    ] {
        view.set_package_inventory_status(state, false);
        assert_eq!(view.status, failure);
        assert_package_notice_footer(&mut view, &failure);
    }
    view.save_started(7);
    view.set_package_inventory_status(PackageInventoryStatus::Unavailable, false);
    assert_eq!(view.saving, Some(7));
    assert_package_notice_footer(&mut view, "Saving settings...");
    view.save_completed(6, true);
    assert_eq!(view.saving, Some(7));
    view.save_completed(7, true);
    assert_package_notice_footer(
        &mut view,
        "Package settings unavailable. Reopen to retry.",
    );
    view.set_temporary_customizations(true);
    view.set_status("Temporary preview only. Restore saved to return.");
    for state in [
        PackageInventoryStatus::Loading,
        PackageInventoryStatus::Unavailable,
        PackageInventoryStatus::Ready,
    ] {
        view.set_package_inventory_status(state, false);
        let expected = match state {
            PackageInventoryStatus::Loading => {
                "Preview only. Loading package settings..."
            }
            PackageInventoryStatus::Unavailable => {
                "Preview only. Packages unavailable; reopen to retry."
            }
            _ => "Temporary preview only. Restore saved to return.",
        };
        assert_package_notice_footer(&mut view, expected);
        assert!(view.temporary_customizations);
    }
}

#[test]
fn package_notice_does_not_replace_color_editor_feedback_or_draft() {
    let mut view = package_notice_view();
    assert!(view.view.as_mut().unwrap().focus(
        &SettingId::new(automexia_ui_model::settings::COMMAND_OUTPUT_HIGHLIGHTING)
            .unwrap()
    ));
    view.focus = Focus::List;
    named(&mut view, NamedKey::Enter);
    view.paint(&mut Raster::new(1.0), theme());
    view.start_preview_edit();
    named(&mut view, NamedKey::Enter);
    named(&mut view, NamedKey::Enter);
    assert!(view.color_editor.is_some());
    view.set_status("Could not copy. Selection kept; try again.");
    let draft = view.color_editor.as_ref().unwrap().draft.clone();
    let feedback = view.color_editor.as_ref().unwrap().feedback.clone();
    let mut before = Raster::new(1.0);
    view.paint(&mut before, theme());
    view.set_package_inventory_status(PackageInventoryStatus::Unavailable, false);
    assert_eq!(view.color_editor.as_ref().unwrap().draft, draft);
    assert_eq!(view.color_editor.as_ref().unwrap().feedback, feedback);
    let mut after = Raster::new(1.0);
    view.paint(&mut after, theme());
    assert!(before.pixels(960, 620, true) == after.pixels(960, 620, true));
    assert!(view
        .accessibility_summary()
        .contains("Package settings unavailable"));
    assert!(view.take_edit().is_none());
}

#[test]
fn background_opacity_can_be_typed_in_each_preview_detail_without_losing_selection() {
    let base = rio_backend::config::Config::default();
    for (category, item, opacity) in [
        (
            "terminal.command_output_highlighting",
            "command_output.band.failure",
            "command_output.opacity.failure",
        ),
        (
            "terminal.command_output_highlighting",
            "output.severity.error",
            "output.opacity.error",
        ),
        (
            "terminal.kubernetes_highlighting",
            "kubernetes.severity.warning",
            "kubernetes.opacity.warning",
        ),
    ] {
        let initial = crate::automexia::preferences::UserPreferences::default();
        let mut view = SettingsView::default();
        view.fit(960.0, 620.0, 16.0);
        view.open_with_section(
            crate::settings_catalog::catalog(1, &base, &initial, &[]).unwrap(),
            Some(Section::Customizations),
        );
        assert!(view
            .view
            .as_mut()
            .unwrap()
            .focus(&SettingId::new(category).unwrap()));
        view.focus = Focus::List;
        named(&mut view, NamedKey::Enter);
        view.paint(&mut Raster::new(1.0), theme());
        view.start_preview_edit();
        view.preview_selected = Some(SettingId::new(item).unwrap());
        named(&mut view, NamedKey::Enter);
        let id = SettingId::new(opacity).unwrap();
        assert!(view.view.as_mut().unwrap().focus(&id));
        view.focus = Focus::List;
        named(&mut view, NamedKey::Enter);
        assert_eq!(view.numeric_editor.as_ref().unwrap().id, id);
        assert!(view.paste("25"));
        named(&mut view, NamedKey::Enter);
        let edit = view.take_edit().unwrap();
        assert_eq!(edit.change, Change::Set(SettingValue::Number(25.0)));
        let changed =
            crate::settings_catalog::apply_edit(1, &base, &initial, &[], &edit).unwrap();
        view.refresh(crate::settings_catalog::catalog(2, &base, &changed, &[]).unwrap());
        view.paint(&mut Raster::new(1.0), theme());
        assert_eq!(
            view.catalog.as_ref().unwrap().get(&id).unwrap().value,
            SettingValue::Number(25.0)
        );
        assert_eq!(
            view.customizations
                .as_ref()
                .unwrap()
                .active_slot
                .as_ref()
                .unwrap()
                .as_str(),
            item
        );
        named(&mut view, NamedKey::Escape);
        assert!(view.customizations.as_ref().unwrap().active_slot.is_none());
        assert!(!view.preview_edit_mode);
        assert_eq!(view.preview_selected.as_ref().unwrap().as_str(), item);
    }
}

#[test]
fn editable_preview_footer_exposes_e_in_shared_and_item_controls_after_saving() {
    for detail in [false, true] {
        for saved in [false, true] {
            let mut view = workflow_tag_view();
            if detail {
                view.start_preview_edit();
                named(&mut view, NamedKey::Enter);
            }
            if saved {
                view.set_status("Saved");
            }
            let mut actual = Raster::new(1.0);
            view.paint(&mut actual, theme());
            let bounds = view.geometry.status;
            let accent = automexia_ui_model::ensure_contrast(
                crate::renderer::ui_theme::BRAND_CYAN,
                theme().raised,
                automexia_ui_model::MIN_TEXT_CONTRAST + 0.1,
            );
            let mut expected = Raster::new(1.0);
            label(
                &mut expected,
                bounds,
                "E:",
                view.font * 0.85,
                accent,
                true,
                view.geometry.card,
            );
            let mut actual_pixels = vec![0; 960 * 620];
            let mut expected_pixels = vec![0; 960 * 620];
            actual.text.render_cpu_base(&mut actual_pixels, 960, 620);
            actual.text.render_cpu_modal(&mut actual_pixels, 960, 620);
            expected
                .text
                .render_cpu_base(&mut expected_pixels, 960, 620);
            expected
                .text
                .render_cpu_modal(&mut expected_pixels, 960, 620);
            let mut compared = 0;
            for y in
                bounds.y.ceil() as usize..(bounds.y + view.font * 1.25).floor() as usize
            {
                for x in bounds.x.ceil() as usize..(bounds.x + 20.0).floor() as usize {
                    let index = y * 960 + x;
                    assert_eq!(
                        actual_pixels[index], expected_pixels[index],
                        "E hint missing: detail={detail}, saved={saved}"
                    );
                    compared += usize::from(expected_pixels[index] != 0);
                }
            }
            assert!(compared > 0, "compare real shortcut glyph pixels");
        }
    }
}

#[test]
fn compact_preview_header_keeps_edit_and_done_reachable_without_changing_settings() {
    for category in [
        "tags.enabled",
        "terminal.command_output_highlighting",
        "terminal.kubernetes_highlighting",
    ] {
        for (width, height, scale) in [(320.0, 420.0, 1.25), (960.0, 620.0, 1.0)] {
            let mut view = workflow_tag_view();
            view.back_to_categories();
            assert!(view
                .view
                .as_mut()
                .unwrap()
                .focus(&SettingId::new(category).unwrap()));
            view.focus = Focus::List;
            named(&mut view, NamedKey::Enter);
            view.fit(width, height, 16.0);
            view.paint(&mut Raster::new(scale as f32), theme());
            let panel = view.geometry.preview;
            let button = view.preview_button;
            assert!(button.width > 0.0 && button.height > 0.0);
            assert!(button.y - panel.y < 8.0, "edit belongs in the title row");
            assert!(button.x + button.width <= panel.x + panel.width);
            assert!(!view.preview_targets.is_empty());
            assert!(view
                .preview_targets
                .iter()
                .all(|(_, bounds)| bounds.y >= button.y + button.height));
            pointer_event(&mut view, button, scale, ElementState::Pressed);
            pointer_event(&mut view, button, scale, ElementState::Released);
            assert!(view.preview_edit_mode);
            assert_eq!(view.focus, Focus::Preview);
            named(&mut view, NamedKey::Tab);
            let selected = view.preview_selected.clone();
            view.paint(&mut Raster::new(scale as f32), theme());
            let done = view.preview_button;
            pointer_event(&mut view, done, scale, ElementState::Pressed);
            // A redraw between press and release must preserve the Done action.
            view.paint(&mut Raster::new(scale as f32), theme());
            pointer_event(&mut view, done, scale, ElementState::Released);
            assert!(!view.preview_edit_mode);
            assert_eq!(view.focus, Focus::PreviewButton);
            assert_eq!(view.preview_selected, selected);
            named(&mut view, NamedKey::Enter);
            assert!(view.preview_edit_mode);
            assert_eq!(view.preview_selected, selected);
            named(&mut view, NamedKey::Escape);
            assert!(!view.preview_edit_mode);
            assert!(view.take_edit().is_none());
            assert!(view.take_customization_intent().is_none());
        }
    }
}

#[test]
fn preview_shortcuts_do_not_replace_save_or_loading_feedback() {
    let mut view = workflow_tag_view();
    for editing in [false, true] {
        if editing {
            preview_edit_key(&mut view);
        }
        for message in [
            "Cannot save settings.",
            "Saving...",
            "Temporary preview only. Restore saved to return.",
        ] {
            view.set_status(message);
            assert_package_notice_footer(&mut view, message);
        }
    }
}

#[test]
fn customization_reset_and_restore_wait_for_confirmation_and_escape_cancels() {
    let mut view = workflow_tag_view();
    view.back_to_categories();
    for target in [Target::Reset, Target::Restore] {
        view.set_temporary_customizations(true);
        view.activate_target(target.clone());
        assert!(
            view.take_customization_intent().is_none(),
            "request must wait for confirmation"
        );
        view.key(
            &Key::Character("y".into()),
            Some("y"),
            ModifiersState::empty(),
            true,
        );
        view.key(
            &Key::Character("y".into()),
            Some("y"),
            ModifiersState::CONTROL,
            false,
        );
        assert!(view.confirmation.is_some());
        assert!(view.take_customization_intent().is_none());
        named(&mut view, NamedKey::Escape);
        assert!(view.is_open() && view.is_category_root());
        assert!(view.take_customization_intent().is_none());
        view.activate_target(target.clone());
        named(&mut view, NamedKey::Tab);
        named(&mut view, NamedKey::Enter);
        assert_eq!(
            view.take_customization_intent(),
            Some(if target == Target::Reset {
                CustomizationIntent::Reset {
                    revision: 1,
                    scope: CustomizationResetScope::Area(CustomizationArea::Workflow),
                }
            } else {
                CustomizationIntent::RestoreSaved
            })
        );
        assert!(view.take_edit().is_none());
    }
}

fn confirm_requested_settings_action(view: &mut SettingsView) {
    assert!(view.confirmation.is_some());
    assert!(!view.confirmation.as_ref().unwrap().accept_selected);
    assert!(view.take_edit().is_none());
    assert!(view.take_customization_intent().is_none());
    named(view, NamedKey::Tab);
    named(view, NamedKey::Enter);
    assert!(view.confirmation.is_none());
}

fn settings_letter(view: &mut SettingsView, letter: &str) {
    view.key(
        &Key::Character(letter.into()),
        Some(letter),
        ModifiersState::empty(),
        false,
    );
}

#[test]
fn settings_button_shortcuts_respect_text_modifiers_repeat_and_cancel_focus() {
    let mut view = workflow_tag_view();
    for key in ["r", "s", "c"] {
        view.focus = Focus::List;
        for (modifiers, repeat) in [
            (ModifiersState::CONTROL, false),
            (ModifiersState::ALT, false),
            (ModifiersState::SUPER, false),
            (ModifiersState::empty(), true),
        ] {
            view.key(&Key::Character(key.into()), Some(key), modifiers, repeat);
            assert!(view.is_open() && view.confirmation.is_none());
        }
    }
    settings_letter(&mut view, "s");
    assert!(
        view.confirmation.is_none(),
        "Restore is disabled without a temporary preview"
    );
    view.start_preview_edit();
    let selection = view.preview_selected.clone();
    settings_letter(&mut view, "r");
    assert!(view.confirmation.is_some());
    named(&mut view, NamedKey::Enter); // Cancel is the default.
    assert!(view.confirmation.is_none());
    assert!(view.take_customization_intent().is_none());
    assert_eq!(view.focus, Focus::Preview);
    assert_eq!(view.preview_selected, selection);
    view.set_temporary_customizations(true);
    settings_letter(&mut view, "s");
    assert_eq!(view.confirmation.as_ref().unwrap().accept_label, "Restore");
    settings_letter(&mut view, "n");
    assert!(view.confirmation.is_none());
    assert!(view.take_customization_intent().is_none());
    view.focus = Focus::Search;
    for key in ["r", "s", "c"] {
        settings_letter(&mut view, key);
    }
    assert_eq!(view.query(), "rsc");
    assert!(view.is_open() && view.confirmation.is_none());
    view.focus = Focus::List;
    settings_letter(&mut view, "c");
    assert!(!view.is_open());
}

#[test]
fn settings_confirmation_blocks_background_input_and_preserves_numeric_draft_on_cancel() {
    let mut view = workflow_tag_view();
    assert!(view
        .view
        .as_mut()
        .unwrap()
        .focus(&SettingId::new("tags.spacing").unwrap()));
    view.focus = Focus::List;
    named(&mut view, NamedKey::Enter);
    assert!(view.paste("12"));
    view.event(
        &WindowEvent::Ime(Ime::Preedit("3".into(), None)),
        ModifiersState::empty(),
        1.0,
    );
    view.activate_target(Target::Reset);
    assert!(view.confirmation.is_some());
    let draft = view.numeric_editor.as_ref().unwrap().draft.clone();
    assert!(!view.paste("99"));
    assert!(view.clipboard_selection().is_none());
    assert!(view.ime_cursor_area().is_none());
    view.event(
        &WindowEvent::Ime(Ime::Commit("88".into())),
        ModifiersState::empty(),
        1.0,
    );
    let scroll = view.scroll;
    view.scroll_by(100.0);
    view.activate_target(Target::Close);
    settings_letter(&mut view, "e");
    assert_eq!(view.scroll, scroll);
    assert!(view.is_open() && view.confirmation.is_some());
    named(&mut view, NamedKey::Escape);
    assert_eq!(view.numeric_editor.as_ref().unwrap().draft, draft);
    assert!(!view.numeric_editor.as_ref().unwrap().composing);
    assert!(view.take_edit().is_none());
    assert!(view.take_customization_intent().is_none());
    named(&mut view, NamedKey::Enter);
    assert_eq!(
        view.take_edit().unwrap().change,
        Change::Set(SettingValue::Number(12.0))
    );
}

#[test]
fn settings_confirmation_rejects_stale_refresh_and_tiny_or_resized_clicks() {
    let mut view = workflow_tag_view();
    view.activate_target(Target::Reset);
    let base = rio_backend::config::Config::default();
    view.refresh(
        crate::settings_catalog::catalog(2, &base, &Default::default(), &[]).unwrap(),
    );
    assert!(view.confirmation.is_none());
    assert!(view.take_customization_intent().is_none());
    view.fit(960.0, 620.0, 16.0);
    view.activate_target(Target::Reset);
    view.paint(&mut Raster::new(1.0), theme());
    let accept = view.confirmation_geometry.accept;
    pointer_event(&mut view, accept, 1.0, ElementState::Pressed);
    view.fit(320.0, 420.0, 16.0);
    view.paint(&mut Raster::new(1.0), theme());
    pointer_event(&mut view, accept, 1.0, ElementState::Released);
    assert!(view.confirmation.is_some());
    assert!(view.take_customization_intent().is_none());
    view.fit(100.0, 100.0, 16.0);
    view.paint(&mut Raster::new(1.0), theme());
    settings_letter(&mut view, "y");
    assert!(view.confirmation.is_some());
    assert!(view.take_customization_intent().is_none());
    named(&mut view, NamedKey::Escape);
    assert!(view.confirmation.is_none() && view.is_open());
}

#[test]
fn color_reset_confirmation_cancel_retains_draft_and_y_resets_only_that_value() {
    let mut view = opened_color(false);
    assert!(replace_color(&mut view, "#ABCDEF"));
    view.activate_color(ColorFocus::Reset);
    assert!(view.confirmation.is_some());
    named(&mut view, NamedKey::Escape);
    assert_eq!(view.color_editor.as_ref().unwrap().draft, "#ABCDEF");
    assert!(view.take_edit().is_none());
    view.activate_color(ColorFocus::Reset);
    let id = view.color_editor.as_ref().unwrap().id.clone();
    settings_letter(&mut view, "y");
    assert_eq!(
        view.take_edit(),
        Some(Edit {
            revision: 2,
            id,
            change: Change::Reset
        })
    );
    assert!(view.color_editor.is_none() && view.confirmation.is_none());
    assert!(view.take_customization_intent().is_none());
}

#[test]
fn settings_confirmation_is_opaque_bounded_and_pointer_confirmation_is_one_shot() {
    for (width, height, scale) in [(320.0, 420.0, 1.25), (960.0, 620.0, 1.0)] {
        let mut view = workflow_tag_view();
        view.fit(width, height, 16.0);
        view.activate_target(Target::Reset);
        let mut raster = Raster::new(scale as f32);
        view.paint(&mut raster, theme());
        assert!(view.accessibility_summary().contains("Cancel: Escape"));
        assert!(raster.rects.iter().all(|([x, y, w, h], color)| *x >= 0.0
            && *y >= 0.0
            && x + w <= width + 0.001
            && y + h <= height + 0.001
            && color[3] == 1.0));
        let bounds = view.confirmation_geometry.accept;
        assert!(bounds.width > 0.0 && bounds.height > 0.0);
        if let Some(directory) = std::env::var_os("AUTOMEXIA_SETTINGS_PREVIEW_DIR") {
            let directory = std::path::PathBuf::from(directory);
            std::fs::create_dir_all(&directory).unwrap();
            let w = (width * scale as f32) as u32;
            let h = (height * scale as f32) as u32;
            let pixels = raster.pixels(w, h, true);
            image_rs::RgbImage::from_fn(w, h, |x, y| {
                let p = pixels[(y * w + x) as usize];
                image_rs::Rgb([(p >> 16) as u8, (p >> 8) as u8, p as u8])
            })
            .save(directory.join(format!("settings-confirm-{w}x{h}.png")))
            .unwrap();
        }
        pointer_event(&mut view, bounds, scale, ElementState::Pressed);
        view.paint(&mut Raster::new(scale as f32), theme());
        pointer_event(&mut view, bounds, scale, ElementState::Released);
        assert!(matches!(
            view.take_customization_intent(),
            Some(CustomizationIntent::Reset {
                scope: CustomizationResetScope::Group(_),
                ..
            })
        ));
        pointer_event(&mut view, bounds, scale, ElementState::Released);
        assert!(view.take_customization_intent().is_none());
    }
}

fn dependent_settings_view(
    base: &rio_backend::config::Config,
    preferences: &crate::automexia::preferences::UserPreferences,
    category: &str,
) -> SettingsView {
    let mut view = SettingsView::default();
    view.fit(960.0, 740.0, 16.0);
    view.open_customizations_with_slots(
        crate::settings_catalog::catalog(1, base, preferences, &[]).unwrap(),
        None,
        Some(crate::settings_catalog::slot_page_snapshot_with_config(
            preferences,
            &preferences.apply_to(base),
            base,
            &crate::settings_catalog::test_installed_extensions(),
        )),
    );
    if category.starts_with("interface.")
        || matches!(
            category,
            crate::settings_catalog::WINDOW_CONTROLS
                | automexia_ui_model::settings::FONT_SIZE
                | automexia_ui_model::settings::APPEARANCE_THEME
        )
    {
        view.show_terminal_appearance();
    }
    assert!(view
        .view
        .as_mut()
        .unwrap()
        .focus(&SettingId::new(category).unwrap()));
    view.focus = Focus::List;
    named(&mut view, NamedKey::Enter);
    view
}

#[test]
fn dependent_settings_hidden_date_and_time_remove_only_ineffective_controls() {
    use rio_backend::config::presentation::{TimestampDateFormat, TimestampTimeFormat};
    let mut base = rio_backend::config::Config::default();
    base.presentation.timestamps.date_format = Some(TimestampDateFormat::Hidden);
    base.presentation.timestamps.time_format = Some(TimestampTimeFormat::Hidden);
    let mut view =
        dependent_settings_view(&base, &Default::default(), COMMAND_TIMESTAMPS);
    view.paint(&mut Raster::new(1.0), theme());
    let page = view.catalog.as_ref().unwrap();
    for id in [
        "timestamps.date-separator",
        "timestamps.date-position",
        "timestamps.weekday",
        "timestamps.colors.date",
        "timestamps.precision",
        "timestamps.time-position",
        "timestamps.colors.time",
        "timestamps.timezone",
        "timestamps.zone-label",
        "timestamps.date-time-separator",
        "timestamps.separator",
        "timestamps.order",
    ] {
        assert!(
            page.get(&SettingId::new(id).unwrap()).is_none(),
            "ineffective control {id}"
        );
        assert!(!view.rows.iter().any(|row| row.id.as_str() == id));
    }
    for id in [
        COMMAND_TIMESTAMPS,
        "timestamps.date-format",
        "timestamps.time-format",
        "timestamps.show-status",
        "timestamps.show-duration",
        "timestamps.duration-format",
        "timestamps.result-position",
        "timestamps.colors.result",
    ] {
        assert!(
            page.get(&SettingId::new(id).unwrap()).is_some(),
            "lost independent control {id}"
        );
    }
}

#[test]
fn dependent_settings_refresh_preserves_choices_and_removes_stale_focus_and_search() {
    use crate::automexia::preferences::UserPreferences;
    use rio_backend::config::presentation::{
        TimestampDateFormat, TimestampDateSeparator,
    };
    let base = rio_backend::config::Config::default();
    let mut prefs = UserPreferences::default();
    prefs.visual.timestamps.date_separator = Some(TimestampDateSeparator::Slash);
    let mut view = dependent_settings_view(&base, &prefs, COMMAND_TIMESTAMPS);
    let id = SettingId::new("timestamps.date-separator").unwrap();
    assert!(view.view.as_mut().unwrap().focus(&id));
    prefs.visual.timestamps.date_format = Some(TimestampDateFormat::Hidden);
    let refresh = |view: &mut SettingsView, prefs: &UserPreferences, revision| {
        view.refresh_with_resources(
            crate::settings_catalog::catalog(revision, &base, prefs, &[]).unwrap(),
            None,
            Some(crate::settings_catalog::slot_page_snapshot_with_config(
                prefs,
                &prefs.apply_to(&base),
                &base,
                &crate::settings_catalog::test_installed_extensions(),
            )),
        );
    };
    refresh(&mut view, &prefs, 2);
    assert_ne!(view.view.as_ref().unwrap().focused(), Some(&id));
    assert!(!view.view.as_mut().unwrap().focus(&id));
    view.view
        .as_mut()
        .unwrap()
        .set_query(id.as_str(), view.catalog.as_ref().unwrap())
        .unwrap();
    assert!(view.view.as_ref().unwrap().filtered_ids().is_empty());
    view.paint(&mut Raster::new(1.5), theme());
    assert!(!view.rows.iter().any(|row| row.id == id));
    assert!(view.take_edit().is_none());
    prefs.visual.timestamps.date_format = Some(TimestampDateFormat::YearMonthDay);
    refresh(&mut view, &prefs, 3);
    assert_eq!(
        view.catalog.as_ref().unwrap().get(&id).unwrap().value,
        SettingValue::Choice("slash".into())
    );
    assert_eq!(
        view.catalog.as_ref().unwrap().get(&id).unwrap().origin,
        ValueOrigin::User
    );
    assert_eq!(view.view.as_ref().unwrap().filtered_ids(), &[id]);
}

#[test]
fn dependent_settings_same_revision_refresh_cancels_newly_hidden_editor() {
    use rio_backend::config::presentation::TimestampDateFormat;
    let base = rio_backend::config::Config::default();
    let mut prefs = crate::automexia::preferences::UserPreferences::default();
    let mut view = dependent_settings_view(&base, &prefs, COMMAND_TIMESTAMPS);
    assert!(view
        .view
        .as_mut()
        .unwrap()
        .focus(&SettingId::new("timestamps.colors.date").unwrap()));
    view.focus = Focus::List;
    named(&mut view, NamedKey::Enter);
    assert!(view.color_editor.is_some());
    prefs.visual.timestamps.date_format = Some(TimestampDateFormat::Hidden);
    view.refresh_with_resources(
        crate::settings_catalog::catalog(1, &base, &prefs, &[]).unwrap(),
        None,
        Some(crate::settings_catalog::slot_page_snapshot_with_config(
            &prefs,
            &prefs.apply_to(&base),
            &base,
            &crate::settings_catalog::test_installed_extensions(),
        )),
    );
    assert!(view.color_editor.is_none());
    assert!(view.take_edit().is_none());
    view.paint(&mut Raster::new(1.0), theme());
    let surface =
        view.accessibility_surface(1.0, accesskit::Rect::new(0.0, 0.0, 960.0, 740.0));
    assert!(!surface
        .elements
        .iter()
        .any(|element| element.node.author_id() == Some("timestamps.colors.date")));
}

#[test]
fn dependent_settings_disabled_highlights_have_no_preview_editor_targets() {
    let base = rio_backend::config::Config::default();
    let mut prefs = crate::automexia::preferences::UserPreferences::default();
    prefs.presentation.command_output_highlighting = Some(false);
    prefs.presentation.output_highlighting = Some(false);
    prefs.presentation.kubernetes_highlighting = Some(false);
    prefs.visual.tags.enabled = Some(false);
    for category in [
        "terminal.command_output_highlighting",
        "terminal.kubernetes_highlighting",
        "tags.enabled",
    ] {
        let mut view = dependent_settings_view(&base, &prefs, category);
        view.paint(&mut Raster::new(1.0), theme());
        assert!(!view.preview_selector_available(), "{category}");
        assert!(view.preview_targets.is_empty(), "{category}");
        assert!(view.preview_order.is_empty(), "{category}");
        view.start_preview_edit();
        assert!(!view.preview_edit_mode, "{category}");
        assert!(
            view.catalog
                .as_ref()
                .unwrap()
                .entries()
                .iter()
                .all(|row| row.kind == SettingKind::Boolean),
            "only enabling switches should remain in {category}"
        );
    }
    prefs.presentation.output_highlighting = Some(true);
    let mut view =
        dependent_settings_view(&base, &prefs, "terminal.command_output_highlighting");
    view.paint(&mut Raster::new(1.0), theme());
    assert!(view.preview_selector_available());
    assert!(!view.preview_order.is_empty());
    assert!(view
        .preview_order
        .iter()
        .all(|id| id.as_str().starts_with("output.severity.")));
}

#[test]
#[ignore = "same-host settings refresh/paint benchmark; run explicitly with --ignored"]
fn dependent_settings_refresh_paint_benchmark() {
    use rio_backend::config::presentation::TimestampDateFormat;
    let base = rio_backend::config::Config::default();
    let mut prefs = crate::automexia::preferences::UserPreferences::default();
    let mut view = dependent_settings_view(&base, &prefs, COMMAND_TIMESTAMPS);
    let mut raster = Raster::new(1.0);
    let mut reports = Vec::new();
    for (width, height) in [(720.0, 560.0), (1920.0, 1080.0)] {
        view.fit(width, height, 16.0);
        let mut samples = Vec::new();
        for iteration in 0..220 {
            prefs.visual.timestamps.date_format = Some(if iteration % 2 == 0 {
                TimestampDateFormat::Hidden
            } else {
                TimestampDateFormat::YearMonthDay
            });
            raster.rects.clear();
            raster.shapes.clear();
            raster.text.clear();
            let start = std::time::Instant::now();
            view.refresh_with_resources(
                crate::settings_catalog::catalog(iteration + 2, &base, &prefs, &[])
                    .unwrap(),
                None,
                Some(crate::settings_catalog::slot_page_snapshot_with_config(
                    &prefs,
                    &prefs.apply_to(&base),
                    &base,
                    &crate::settings_catalog::test_installed_extensions(),
                )),
            );
            view.paint(std::hint::black_box(&mut raster), theme());
            let elapsed = start.elapsed().as_nanos();
            assert_eq!(
                view.catalog
                    .as_ref()
                    .unwrap()
                    .entries()
                    .iter()
                    .any(|row| row.id.as_str() == "timestamps.date-separator"),
                iteration % 2 != 0
            );
            if iteration >= 20 {
                samples.push(elapsed);
            }
        }
        samples.sort_unstable();
        reports.push(serde_json::json!({"viewport": [width, height], "samples": samples.len(), "warmup": 20, "refresh_and_paint_ns": {"p50": samples[99], "p95": samples[189]}, "scope": "production catalog, dependency projection, focus/layout/shaping/draw emission; excludes GPU/present"}));
    }
    println!("{}", serde_json::json!({"dependent_settings": reports}));
}

#[test]
fn shared_color_picker_custom_apply_favorites_cancel_reset_and_invalid_are_distinct() {
    for action in [ColorFocus::Apply, ColorFocus::Cancel, ColorFocus::Reset] {
        let mut view = opened_color(true);
        assert!(replace_color(&mut view, "#12345678"));
        view.activate_color(action);
        if action == ColorFocus::Apply {
            assert!(matches!(
                view.take_color_favorite_intent(),
                Some(ColorFavoriteIntent::Remember([18, 52, 86, 120]))
            ));
            assert!(view.take_edit().is_some());
        } else {
            assert!(view.take_color_favorite_intent().is_none());
            assert!(view.take_edit().is_none());
        }
    }
    let mut view = opened_color(false);
    assert!(replace_color(&mut view, "invalid"));
    view.activate_color(ColorFocus::Apply);
    view.activate_color(ColorFocus::FavoriteToggle);
    assert!(view.take_color_favorite_intent().is_none());
    assert!(view.take_edit().is_none());
    assert!(view.color_editor.is_some());
}

#[test]
fn shared_color_picker_favorites_are_exact_contextual_and_removable() {
    for alpha in [false, true] {
        let mut view = opened_color(alpha);
        view.set_color_favorites(&[[1, 2, 3, 4], [5, 6, 7, 255]]);
        view.activate_color(ColorFocus::Favorites);
        view.paint(&mut Raster::new(1.0), theme());
        let colors = view.color_palette_colors();
        assert_eq!(colors.len(), if alpha { 2 } else { 1 });
        let first = colors[0];
        let bounds = view
            .color_palette_controls()
            .into_iter()
            .find(|(f, _)| *f == ColorFocus::Swatch(0))
            .unwrap()
            .1;
        pointer_event(&mut view, bounds, 1.0, ElementState::Pressed);
        pointer_event(&mut view, bounds, 1.0, ElementState::Released);
        assert_eq!(
            editor_value(view.color_editor.as_ref().unwrap()),
            Some(SettingValue::Color(first))
        );
        assert!(view.take_edit().is_none());
        assert!(view.take_color_favorite_intent().is_none());
        view.activate_color(ColorFocus::FavoriteToggle);
        assert!(
            matches!(view.take_color_favorite_intent(), Some(ColorFavoriteIntent::Forget(c)) if c == first)
        );
        named(&mut view, NamedKey::F2);
        named(&mut view, NamedKey::Delete);
        assert!(
            matches!(view.take_color_favorite_intent(), Some(ColorFavoriteIntent::Forget(c)) if c == first)
        );
        view.set_color_favorites(&[]);
        assert_eq!(
            view.color_editor.as_ref().unwrap().focus,
            ColorFocus::Favorites
        );
        assert!(view.color_palette_colors().is_empty());
    }
}

#[test]
fn shared_color_picker_ignores_stale_pointer_and_text_editor_has_no_palette() {
    let mut view = opened_color(false);
    view.set_color_favorites(&[[1, 2, 3, 255]]);
    view.activate_color(ColorFocus::Favorites);
    view.paint(&mut Raster::new(1.0), theme());
    let old = view.color_editor.as_ref().unwrap().draft.clone();
    let rect = view
        .color_palette_controls()
        .into_iter()
        .find(|(f, _)| *f == ColorFocus::Swatch(0))
        .unwrap()
        .1;
    pointer_event(&mut view, rect, 1.0, ElementState::Pressed);
    view.set_color_favorites(&[[4, 5, 6, 255]]);
    pointer_event(&mut view, rect, 1.0, ElementState::Released);
    assert_eq!(view.color_editor.as_ref().unwrap().draft, old);
    let mut text = opened();
    text.refresh(text_catalog(2));
    named(&mut text, NamedKey::Tab);
    named(&mut text, NamedKey::Enter);
    text.paint(&mut Raster::new(1.0), theme());
    assert!(text.color_palette_controls().is_empty());
    named(&mut text, NamedKey::ArrowDown);
    assert_eq!(text.color_editor.as_ref().unwrap().focus, ColorFocus::Hex);
    assert!(text.take_color_favorite_intent().is_none());
}

#[test]
fn shared_color_picker_layout_and_semantics_across_themes_and_scales() {
    for descriptor in crate::automexia::theme_gallery::builtins() {
        let theme = UiTheme::from_colors(&descriptor.theme.unwrap().colors);
        for (width, height, font, scale) in [
            (720., 560., 14., 1.),
            (340., 540., 16., 1.5),
            (960., 740., 24., 2.),
        ] {
            let mut view = opened_color(true);
            view.fit(width, height, font);
            let mut raster = Raster::new(scale);
            view.paint(&mut raster, theme);
            assert!(!view.color_requires_larger_window());
            let g = view.color_geometry;
            assert!(
                g.palette.y >= g.help.y + g.help.height,
                "help must not overlap suggestions"
            );
            assert!(
                g.palette.y >= g.preview.y + g.preview.height,
                "preview must not overlap suggestions"
            );
            assert!(g.palette.y + g.palette.height <= g.apply.y);
            for (_, rect) in view.color_palette_controls() {
                assert!(rect.width >= 24.0 && rect.height >= 24.0);
                assert!(
                    rect.x >= g.card.x && rect.x + rect.width <= g.card.x + g.card.width
                );
                assert!(
                    rect.y >= g.card.y
                        && rect.y + rect.height <= g.card.y + g.card.height
                );
            }
            assert!(view.color_palette_snapshot().is_object());
            named(&mut view, NamedKey::ArrowDown);
            named(&mut view, NamedKey::Enter);
            view.paint(&mut Raster::new(scale), theme);
            let surface = view.accessibility_surface(
                scale,
                accesskit::Rect::new(
                    0.,
                    0.,
                    f64::from(width * scale),
                    f64::from(height * scale),
                ),
            );
            let swatches: Vec<_> = surface
                .elements
                .iter()
                .filter(|e| {
                    e.node
                        .label()
                        .is_some_and(|label| label.starts_with("Color #"))
                })
                .collect();
            assert_eq!(swatches.len(), view.color_palette_colors().len());
            for (caption, shortcut) in [
                ("Suggested colors", "F1:"),
                ("Favorite colors", "F2:"),
                ("Save favorite", "F3:"),
            ] {
                let element = surface
                    .elements
                    .iter()
                    .find(|e| e.node.label() == Some(caption))
                    .unwrap();
                assert!(element
                    .node
                    .description()
                    .is_some_and(|text| text.starts_with(shortcut)));
            }
            assert_eq!(
                swatches
                    .iter()
                    .filter(|e| e.node.toggled() == Some(accesskit::Toggled::True))
                    .count(),
                1
            );
            assert!(surface
                .elements
                .iter()
                .any(|e| e.node.label() == Some("Save favorite")));
        }
    }
}

#[test]
fn shared_color_picker_selection_keeps_swatch_pixels_and_small_windows_keep_hex() {
    let mut view = opened_color(false);
    view.set_color_favorites(&[[203, 74, 35, 255]]);
    view.activate_color(ColorFocus::Favorites);
    view.paint(&mut Raster::new(1.), theme());
    view.activate_color(ColorFocus::Swatch(0));
    let mut raster = Raster::new(1.);
    view.paint(&mut raster, theme());
    let bounds = view
        .color_palette_controls()
        .into_iter()
        .find(|(focus, _)| *focus == ColorFocus::Swatch(0))
        .unwrap()
        .1;
    let pixel = raster.pixels(720, 560, false)
        [(bounds.y as usize + 14) * 720 + bounds.x as usize + 4];
    assert_eq!(
        pixel, 0x00cb4a23,
        "selection must not paint over the color being chosen"
    );
    view.fit(320., 360., 18.);
    view.paint(&mut Raster::new(1.), theme());
    assert!(view.color_palette_controls().is_empty());
    assert!(replace_color(&mut view, "#112233"));
    named(&mut view, NamedKey::ArrowDown);
    assert_eq!(view.color_editor.as_ref().unwrap().focus, ColorFocus::Hex);
    named(&mut view, NamedKey::Enter);
    assert!(matches!(
        view.take_edit().unwrap().change,
        Change::Set(SettingValue::Color([17, 34, 51, 255]))
    ));
}

#[test]
fn interface_footer_keyboard_toggle_hides_dependents_and_preserves_choices() {
    use crate::automexia::preferences::UserPreferences;
    use crate::settings_catalog::{apply_edit, catalog, slot_page_snapshot_with_config};
    let base = rio_backend::config::Config::default();
    let mut prefs = UserPreferences::default();
    prefs.visual.interface.appearance.footer.font_size =
        rio_backend::config::presentation::UiPixels::new(18);
    let mut view = dependent_settings_view(&base, &prefs, "interface.footer.visible");
    let id = SettingId::new("interface.footer.visible").unwrap();
    for revision in [1, 2] {
        assert!(view.view.as_mut().unwrap().focus(&id));
        view.focus = Focus::List;
        named(&mut view, NamedKey::Enter);
        let edit = view.take_edit().expect("one Enter edits footer visibility");
        prefs = apply_edit(revision, &base, &prefs, &[], &edit).unwrap();
        view.refresh_with_resources(
            catalog(revision + 1, &base, &prefs, &[]).unwrap(),
            None,
            Some(slot_page_snapshot_with_config(
                &prefs,
                &prefs.apply_to(&base),
                &base,
                &[],
            )),
        );
        view.paint(&mut Raster::new(1.5), theme());
        assert_eq!(
            view.catalog.as_ref().unwrap().entries().len() == 1,
            revision == 1
        );
        assert_eq!(
            prefs
                .visual
                .interface
                .appearance
                .footer
                .font_size
                .unwrap()
                .get(),
            18.0
        );
    }
    named(&mut view, NamedKey::Escape);
    assert_eq!(view.title(), "Terminal Appearance");
    assert!(view
        .view
        .as_ref()
        .unwrap()
        .filtered_ids()
        .iter()
        .all(|id| !matches!(id.as_str(), "tags.enabled" | COMMAND_TIMESTAMPS)));
}

#[test]
#[ignore = "same-host appearance catalog/layout/paint benchmark; excludes GPU/present"]
fn interface_refresh_paint_benchmark() {
    let base = rio_backend::config::Config::default();
    let mut prefs = crate::automexia::preferences::UserPreferences::default();
    let mut view = dependent_settings_view(&base, &prefs, "interface.footer.visible");
    let mut samples = Vec::new();
    let mut renderer = crate::renderer::Renderer::new(&base);
    for i in 0..220 {
        prefs.visual.interface.appearance.header.height =
            rio_backend::config::presentation::UiPixels::new(32 + (i % 65) as u16);
        prefs.visual.interface.appearance.footer.height =
            rio_backend::config::presentation::UiPixels::new(24 + (i % 49) as u16);
        prefs.visual.interface.appearance.footer.visible = Some(i % 2 == 0);
        prefs.visual.interface.opacity = Some(
            rio_backend::config::presentation::OpacityPercent::new(20 + (i % 81) as u8)
                .unwrap(),
        );
        prefs.visual.interface.appearance.header.background =
            Some(rio_backend::config::presentation::Rgba::from_bytes([
                230,
                180,
                110,
                (i % 256) as u8,
            ]));
        let mut raster = Raster::new(1.5);
        let start = std::time::Instant::now();
        let effective = prefs.apply_to(&base);
        renderer.update_config(&effective);
        assert_eq!(
            renderer.island.as_ref().unwrap().appearance,
            effective.presentation.interface.header
        );
        view.refresh_with_resources(
            crate::settings_catalog::catalog(i + 2, &base, &prefs, &[]).unwrap(),
            None,
            Some(crate::settings_catalog::slot_page_snapshot_with_config(
                &prefs,
                &prefs.apply_to(&base),
                &base,
                &[],
            )),
        );
        view.paint(std::hint::black_box(&mut raster), theme());
        let elapsed = start.elapsed().as_nanos();
        assert_eq!(
            view.catalog.as_ref().unwrap().entries().len() > 1,
            i % 2 == 0
        );
        assert_eq!(
            prefs
                .apply_to(&base)
                .presentation
                .interface
                .footer
                .reserved_height(600.0, 1.5)
                > 0.0,
            i % 2 == 0
        );
        if i >= 20 {
            samples.push(elapsed);
        }
    }
    samples.sort_unstable();
    println!(
        "{}",
        serde_json::json!({"terminal_appearance":{"warmup":20,"samples":samples.len(),"refresh_paint_ns":{"p50":samples[99],"p95":samples[189]},"scope":"catalog, dependency projection, layout and draw emission; excludes GPU/present"}})
    );
}

#[test]
fn interface_pages_paint_inside_card_across_themes_and_scaling() {
    for entry in crate::automexia::theme_gallery::builtins() {
        let colors = entry.theme.unwrap().colors;
        let theme = UiTheme::from_colors(&colors);
        let base = rio_backend::config::Config {
            colors,
            ..Default::default()
        };
        for (width, height, scale) in [
            (360.0, 540.0, 1.0),
            (960.0, 740.0, 1.5),
            (1500.0, 980.0, 2.0),
        ] {
            for page in [
                "interface.header.background",
                "interface.footer.visible",
                "interface.panes.padding",
                "interface.background.opacity",
            ] {
                let mut view = dependent_settings_view(&base, &Default::default(), page);
                view.fit(width, height, 16.0);
                let mut raster = Raster::new(scale);
                view.paint(&mut raster, theme);
                assert_eq!(
                    view.geometry.preview.width, 0.0,
                    "live terminal is the preview"
                );
                let card = view.geometry.card;
                for ([x, y, w, h], _) in &raster.rects {
                    assert!(
                        *x >= card.x - 0.01
                            && *y >= card.y - 0.01
                            && x + w <= card.x + card.width + 0.01
                            && y + h <= card.y + card.height + 0.01,
                        "{page} covers the terminal outside its card"
                    );
                }
                assert!(!view.rows.is_empty());
                for row in &view.rows {
                    if matches!(
                        view.catalog.as_ref().unwrap().get(&row.id).unwrap().kind,
                        SettingKind::Color { .. }
                    ) {
                        assert_eq!(
                            row.value_lines.len(),
                            1,
                            "color hex must fit beside its swatch"
                        );
                    }
                }
                if width == 960.0
                    && matches!(
                        page,
                        "interface.footer.visible" | "interface.header.background"
                    )
                {
                    if let Some(directory) =
                        std::env::var_os("AUTOMEXIA_SETTINGS_PREVIEW_DIR")
                    {
                        let directory = std::path::PathBuf::from(directory);
                        std::fs::create_dir_all(&directory).unwrap();
                        let (w, h) = ((width * scale) as u32, (height * scale) as u32);
                        let pixels = raster.pixels(w, h, true);
                        image_rs::RgbImage::from_fn(w, h, |x, y| {
                            let p = pixels[(y * w + x) as usize];
                            image_rs::Rgb([(p >> 16) as u8, (p >> 8) as u8, p as u8])
                        })
                        .save(directory.join(format!(
                            "terminal-{}-{}.png",
                            if page.contains("header") {
                                "header"
                            } else {
                                "footer"
                            },
                            entry.name.replace(' ', "-")
                        )))
                        .unwrap();
                    }
                }
            }
        }
    }
}

#[test]
fn interface_header_tabs_sections_are_ordered_searchable_and_do_not_steal_focus() {
    let base = rio_backend::config::Config::default();
    for (width, height, scale) in [
        (360.0, 540.0, 1.0),
        (960.0, 740.0, 1.5),
        (1500.0, 980.0, 2.0),
    ] {
        let mut view = dependent_settings_view(
            &base,
            &Default::default(),
            "interface.header.background",
        );
        view.fit(width, height, 16.0);
        view.paint(&mut Raster::new(scale), theme());
        let headings: Vec<_> = view.rows.iter().filter_map(|row| row.section).collect();
        assert_eq!(
            headings.iter().map(|(label, _)| *label).collect::<Vec<_>>(),
            ["Header", "Tabs"]
        );
        let ids = view.view.as_ref().unwrap().filtered_ids().to_vec();
        let split = ids
            .iter()
            .position(|id| id.as_str() == "interface.header.text")
            .unwrap();
        assert!(ids[..split]
            .iter()
            .any(|id| id.as_str() == "interface.header.background-opacity"));
        for id in ids {
            assert!(view.view.as_mut().unwrap().focus(&id));
            view.focus = Focus::List;
            view.reveal_focus = true;
            view.layout_dirty = true;
            view.paint(&mut Raster::new(scale), theme());
            let row = view.rows.iter().find(|row| row.id == id).unwrap();
            assert!(row.bounds.y >= view.geometry.body.y - 0.01);
            assert!(
                row.bounds.y + row.bounds.height
                    <= view.geometry.body.y + view.geometry.body.height + 0.01
            );
            if let Some((_, section)) = row.section {
                assert!(section.y + section.height < row.bounds.y);
            }
            assert_eq!(view.view.as_ref().unwrap().focused(), Some(&id));
        }
        view.focus = Focus::Search;
        assert!(view.paste("Inactive tab"));
        view.paint(&mut Raster::new(scale), theme());
        assert!(view
            .rows
            .iter()
            .any(|row| row.id.as_str() == "interface.header.inactive-text"));
        assert!(view
            .rows
            .iter()
            .all(|row| crate::settings_catalog::interface_control_section(
                row.id.as_str()
            ) == Some("Tabs")));
        assert_eq!(view.rows[0].section.unwrap().0, "Tabs");
        named(&mut view, NamedKey::Escape);
        view.paint(&mut Raster::new(scale), theme());
        assert_eq!(view.title(), "Terminal Appearance");
        assert!(view.rows.iter().all(|row| row.section.is_none()));
    }
}
