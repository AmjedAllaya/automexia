use automexia_ui_model::settings::*;

fn core() -> Catalog {
    Catalog::new(
        7,
        core_descriptors(
            CoreValues::default(),
            CoreValues::default(),
            CoreOrigins::default(),
        ),
    )
    .unwrap()
}

fn edit(id: &str, change: Change) -> Edit {
    Edit {
        revision: 7,
        id: SettingId::new(id).unwrap(),
        change,
    }
}

#[test]
fn output_preferences_preserve_legacy_defaults_and_independent_values() {
    let current = CoreValues {
        inline_tables: false,
        ..CoreValues::default()
    };
    let configured = CoreValues {
        command_timestamps: false,
        ..CoreValues::default()
    };
    let origins = CoreOrigins {
        inline_tables: ValueOrigin::User,
        ..CoreOrigins::default()
    };
    let catalog =
        Catalog::new(7, core_descriptors(current, configured, origins)).unwrap();
    assert_eq!(catalog.entries().len(), 3);
    let tables = catalog
        .get(&SettingId::new(INLINE_TABLES).unwrap())
        .unwrap();
    assert_eq!(tables.value, SettingValue::Boolean(false));
    assert_eq!(tables.default, SettingValue::Boolean(true));
    assert_eq!(tables.origin, ValueOrigin::User);
    assert_eq!(
        catalog
            .get(&SettingId::new(COMMAND_TIMESTAMPS).unwrap())
            .unwrap()
            .default,
        SettingValue::Boolean(false)
    );
    assert_eq!(
        catalog
            .get(&SettingId::new(OUTPUT_HIGHLIGHTING).unwrap())
            .unwrap()
            .value,
        SettingValue::Boolean(true)
    );
}

#[test]
fn setting_false_and_reset_are_distinct_validated_intents_without_snapshot_mutation() {
    let catalog = core();
    let set = edit(INLINE_TABLES, Change::Set(SettingValue::Boolean(false)));
    assert_eq!(catalog.validate_edit(&set).unwrap(), set);
    let reset = edit(INLINE_TABLES, Change::Reset);
    assert_eq!(catalog.validate_edit(&reset).unwrap(), reset);
    assert_eq!(catalog.entries()[0].value, SettingValue::Boolean(true));
}

#[test]
fn stale_unknown_wrong_type_and_unavailable_edits_are_rejected() {
    let catalog = core();
    let mut request = edit(INLINE_TABLES, Change::Set(SettingValue::Boolean(false)));
    request.revision = 6;
    assert_eq!(
        catalog.validate_edit(&request),
        Err(SettingsError::StaleRevision)
    );
    assert_eq!(
        catalog.validate_edit(&edit("terminal.missing", Change::Reset)),
        Err(SettingsError::UnknownSetting)
    );
    assert_eq!(
        catalog
            .validate_edit(&edit(INLINE_TABLES, Change::Set(SettingValue::Number(1.0)))),
        Err(SettingsError::InvalidValue)
    );
    let mut rows = catalog.entries().to_vec();
    rows[0].availability = Availability::Unavailable {
        reason: "Unavailable in this build".into(),
    };
    let unavailable = Catalog::new(7, rows).unwrap();
    assert_eq!(
        unavailable.validate_edit(&edit(INLINE_TABLES, Change::Reset)),
        Err(SettingsError::Unavailable)
    );
}

#[test]
fn extension_namespace_is_bound_to_its_declared_owner() {
    let mut row = SettingDescriptor::boolean(
        SettingId::new("extension.automexia-devops.context").unwrap(),
        Section::Extensions,
        "Context badges",
        "Show installed extension context",
        true,
        true,
    );
    row.owner = SettingOwner::Extension("automexia-devops".into());
    row.origin = ValueOrigin::Extension;
    assert!(Catalog::new(1, vec![row.clone()]).is_ok());
    row.owner = SettingOwner::Extension("another-extension".into());
    assert_eq!(
        Catalog::new(1, vec![row.clone()]).unwrap_err(),
        SettingsError::InvalidDescriptor
    );
    row.owner = SettingOwner::Core;
    assert_eq!(
        Catalog::new(1, vec![row]).unwrap_err(),
        SettingsError::InvalidDescriptor
    );
}

#[test]
fn ids_metadata_duplicates_and_catalogue_capacity_are_bounded() {
    for bad in [
        "",
        "Terminal.table",
        "terminal..table",
        "terminal./table",
        "terminal.table ",
        "terminal.1table",
    ] {
        assert_eq!(SettingId::new(bad), Err(SettingsError::InvalidId));
    }
    assert_eq!(
        SettingId::new("a".repeat(MAX_ID_BYTES + 1)),
        Err(SettingsError::InvalidId)
    );
    let rows = core().entries().to_vec();
    assert_eq!(
        Catalog::new(1, vec![rows[0].clone(), rows[0].clone()]).unwrap_err(),
        SettingsError::DuplicateId
    );
    assert_eq!(
        Catalog::new(1, vec![rows[0].clone(); MAX_SETTINGS + 1]).unwrap_err(),
        SettingsError::Capacity
    );
    let mut row = rows[0].clone();
    row.description = "x".repeat(MAX_DESCRIPTION_BYTES + 1);
    assert_eq!(
        Catalog::new(1, vec![row]).unwrap_err(),
        SettingsError::InvalidText
    );
}

#[test]
fn empty_explanation_and_bidi_control_metadata_are_rejected() {
    let mut rows = core().entries().to_vec();
    rows[0].availability = Availability::Unavailable {
        reason: String::new(),
    };
    assert_eq!(
        Catalog::new(1, rows).unwrap_err(),
        SettingsError::InvalidText
    );
    let mut rows = core().entries().to_vec();
    rows[0].label = "Looks safe\u{202e}hidden".into();
    assert_eq!(
        Catalog::new(1, rows).unwrap_err(),
        SettingsError::InvalidText
    );
}

#[test]
fn choices_validate_membership_and_actions_do_not_accept_stored_values() {
    let mut choice = core().entries()[0].clone();
    choice.kind = SettingKind::Choice {
        options: vec![
            ChoiceOption {
                value: "system".into(),
                label: "Follow system".into(),
            },
            ChoiceOption {
                value: "dark".into(),
                label: "Dark".into(),
            },
        ],
    };
    choice.value = SettingValue::Choice("system".into());
    choice.default = choice.value.clone();
    let catalog = Catalog::new(7, vec![choice.clone()]).unwrap();
    assert!(catalog
        .validate_edit(&edit(
            INLINE_TABLES,
            Change::Set(SettingValue::Choice("dark".into()))
        ))
        .is_ok());
    assert_eq!(
        catalog.validate_edit(&edit(
            INLINE_TABLES,
            Change::Set(SettingValue::Choice("unlisted".into()))
        )),
        Err(SettingsError::InvalidValue)
    );
    choice.kind = SettingKind::Action;
    choice.value = SettingValue::Action;
    choice.default = SettingValue::Action;
    let action = Catalog::new(7, vec![choice]).unwrap();
    assert!(action
        .validate_edit(&edit(INLINE_TABLES, Change::Activate))
        .is_ok());
    assert_eq!(
        action.validate_edit(&edit(INLINE_TABLES, Change::Reset)),
        Err(SettingsError::InvalidValue)
    );
    assert_eq!(
        action.validate_edit(&edit(INLINE_TABLES, Change::Set(SettingValue::Action))),
        Err(SettingsError::InvalidValue)
    );
}

#[test]
fn finite_numeric_ranges_and_steps_are_validated_before_publication() {
    let mut number = core().entries()[0].clone();
    number.kind = SettingKind::Number {
        min: 0.0,
        max: 1.0,
        step: 0.1,
    };
    number.value = SettingValue::Number(0.3);
    number.default = SettingValue::Number(1.0);
    let catalog = Catalog::new(7, vec![number.clone()]).unwrap();
    assert!(catalog
        .validate_edit(&edit(INLINE_TABLES, Change::Set(SettingValue::Number(0.6))))
        .is_ok());
    for invalid in [f64::NAN, f64::INFINITY, -0.1, 1.1, 0.25] {
        assert_eq!(
            catalog.validate_edit(&edit(
                INLINE_TABLES,
                Change::Set(SettingValue::Number(invalid))
            )),
            Err(SettingsError::InvalidValue)
        );
    }
    number.kind = SettingKind::Number {
        min: 1.0,
        max: 0.0,
        step: 0.0,
    };
    assert_eq!(
        Catalog::new(7, vec![number]).unwrap_err(),
        SettingsError::InvalidDescriptor
    );
}

#[test]
fn search_uses_all_tokens_aliases_ids_and_section_without_hiding_focused_matches() {
    let catalog = core();
    let mut view = ViewState::new(&catalog, 2);
    view.set_query("wrap cells", &catalog).unwrap();
    assert_eq!(
        view.filtered_ids(),
        &[SettingId::new(INLINE_TABLES).unwrap()]
    );
    assert_eq!(view.focused().unwrap().as_str(), INLINE_TABLES);
    view.set_query("TIMESTAMP", &catalog).unwrap();
    assert_eq!(view.focused().unwrap().as_str(), COMMAND_TIMESTAMPS);
    view.set_query("", &catalog).unwrap();
    assert_eq!(view.focused().unwrap().as_str(), COMMAND_TIMESTAMPS);
    view.set_section(Some(Section::Extensions), &catalog);
    assert!(view.filtered_ids().is_empty());
    assert!(view.focused().is_none());
    assert_eq!(view.visible_range(), 0..0);
}

#[test]
fn focus_navigation_and_resize_keep_target_visible_and_scroll_clamped() {
    let catalog = core();
    let mut view = ViewState::new(&catalog, 1);
    assert_eq!(view.visible_range(), 0..1);
    view.move_focus(FocusMove::Last);
    assert_eq!(view.focused().unwrap().as_str(), COMMAND_TIMESTAMPS);
    assert_eq!(view.visible_range(), 2..3);
    view.move_focus(FocusMove::Previous);
    assert_eq!(view.visible_range(), 1..2);
    view.set_viewport_rows(usize::MAX);
    assert_eq!(view.visible_range(), 0..3);
    view.move_focus(FocusMove::First);
    view.move_focus(FocusMove::Previous);
    assert_eq!(view.focused().unwrap().as_str(), INLINE_TABLES);
    view.set_viewport_rows(0);
    assert_eq!(view.visible_range(), 0..1);
}

#[test]
fn invalid_query_does_not_replace_state_and_snapshot_refresh_removes_stale_ids() {
    let catalog = core();
    let mut view = ViewState::new(&catalog, 2);
    view.set_query("table", &catalog).unwrap();
    let before = view.clone();
    assert_eq!(
        view.set_query(&"q".repeat(MAX_QUERY_BYTES + 1), &catalog),
        Err(SettingsError::InvalidQuery)
    );
    assert_eq!(view, before);
    assert_eq!(
        view.set_query("bad\nquery", &catalog),
        Err(SettingsError::InvalidQuery)
    );
    let replacement = Catalog::new(8, vec![]).unwrap();
    view.refresh(&replacement);
    assert!(view.focused().is_none());
    assert!(view.filtered_ids().is_empty());
}

#[test]
fn unavailable_entries_remain_searchable_focusable_and_explain_their_state() {
    let mut rows = core().entries().to_vec();
    rows[0].availability = Availability::Unavailable {
        reason: "Extension is disabled".into(),
    };
    let catalog = Catalog::new(7, rows).unwrap();
    let mut view = ViewState::new(&catalog, 1);
    view.set_query("disabled", &catalog).unwrap();
    assert_eq!(view.focused().unwrap().as_str(), INLINE_TABLES);
    assert_eq!(
        catalog.entries()[0].availability.reason(),
        Some("Extension is disabled")
    );
}
