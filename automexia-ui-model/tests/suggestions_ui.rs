use automexia_devops::suggestions::{
    Candidate, CandidateFreshness, CandidateKind, CandidateRisk, CandidateSource,
    QuoteContext, RankedCandidate, ReplacementSpan,
};
use automexia_ui_model::suggestions::{
    project_surface, AnnouncementGate, Navigation, Point, Rect, SuggestionGeometry,
    SuggestionInteraction, SuggestionSurfaceKind, SurfaceRequest,
};

fn ranked(id: u64, display: &str) -> RankedCandidate {
    RankedCandidate {
        candidate: Candidate {
            request_id: 1,
            candidate_id: id,
            insertion: display.into(),
            display: display.into(),
            description: "reviewed local candidate".into(),
            kind: CandidateKind::Command,
            source: CandidateSource::NativeShell,
            freshness: CandidateFreshness::Current,
            replacement_span: ReplacementSpan { start: 0, end: 2 },
            quoting: QuoteContext::Unquoted,
            public_context: false,
            risk: CandidateRisk::ReadOnly,
        },
        exact_prefix: true,
        native_rank: u16::try_from(id).unwrap(),
        word_boundary_prefix: true,
        case_insensitive_prefix: true,
        frequency: 0,
        fuzzy_score: 8,
        matched_graphemes: vec![0, 1],
    }
}

fn request() -> SurfaceRequest {
    SurfaceRequest {
        pane: Rect::new(100.0, 80.0, 900.0, 600.0).unwrap(),
        cursor: Rect::new(300.0, 480.0, 12.0, 24.0).unwrap(),
        exclusions: vec![
            Rect::new(100.0, 80.0, 900.0, 40.0).unwrap(),
            Rect::new(100.0, 640.0, 900.0, 40.0).unwrap(),
        ],
        scale: 1.0,
        row_height: 24.0,
        preferred_width: 520.0,
        reduced_motion: false,
        high_contrast: false,
        selected: 0,
        pointer_highlight: None,
    }
}

#[test]
fn normal_surface_is_pane_owned_and_avoids_cursor_footer_tabs_and_modal() {
    let modal = Rect::new(600.0, 160.0, 350.0, 260.0).unwrap();
    let mut input = request();
    input.exclusions.push(modal);
    let candidates = (1..=20)
        .map(|id| ranked(id, &format!("candidate-{id}")))
        .collect::<Vec<_>>();
    let surface = project_surface(&input, &candidates).unwrap();

    assert_eq!(surface.kind, SuggestionSurfaceKind::Listbox);
    assert!(input.pane.contains(surface.bounds));
    assert!(!surface.bounds.intersects(input.cursor));
    assert!(!surface.bounds.intersects(modal));
    assert_eq!(surface.options.len(), 12);
    assert_eq!(surface.options[0].position, 1);
    assert_eq!(surface.options[0].set_size, 20);
    assert!(surface.options[0].selected);
    assert_eq!(surface.opacity_duration_ms, 120);
}

#[test]
fn impossible_geometry_falls_back_to_noninteractive_compact_hint() {
    let input = SurfaceRequest {
        pane: Rect::new(0.0, 0.0, 120.0, 48.0).unwrap(),
        cursor: Rect::new(0.0, 16.0, 12.0, 24.0).unwrap(),
        exclusions: vec![Rect::new(0.0, 0.0, 120.0, 48.0).unwrap()],
        scale: 3.0,
        row_height: 36.0,
        preferred_width: 520.0,
        reduced_motion: true,
        high_contrast: true,
        selected: 0,
        pointer_highlight: None,
    };
    let surface = project_surface(&input, &[ranked(1, "get")]).unwrap();

    assert_eq!(surface.kind, SuggestionSurfaceKind::CompactHint);
    assert!(surface.options.is_empty());
    assert!(!surface.interactive);
    assert_eq!(surface.opacity_duration_ms, 0);
    assert!(surface
        .accessible_name
        .contains("Native completion available"));
}

#[test]
fn option_semantics_are_concise_distinct_and_not_color_dependent() {
    let surface = project_surface(
        &request(),
        &[
            ranked(1, "get pods"),
            ranked(2, "get services"),
            ranked(3, "get namespaces"),
        ],
    )
    .unwrap();
    let option = &surface.options[0];

    assert_eq!(surface.role, "listbox");
    assert_eq!(option.role, "option");
    assert!(option.accessible_name.starts_with("get pods"));
    assert!(option.accessible_name.contains("native shell"));
    assert!(option.accessible_name.contains("current"));
    assert!(option.accessible_name.contains("read only"));
    assert!(!option.accessible_name.contains('#'));
    assert!(option.accessible_name.len() <= 240);
}

#[test]
fn keyboard_selection_and_pointer_highlight_remain_separate() {
    let mut interaction = SuggestionInteraction::new(8, 2).unwrap();
    assert_eq!(interaction.selected(), 2);
    interaction.navigate(Navigation::Next);
    assert_eq!(interaction.selected(), 3);
    interaction.navigate(Navigation::PageDown);
    assert_eq!(interaction.selected(), 7);
    interaction.navigate(Navigation::Home);
    assert_eq!(interaction.selected(), 0);
    interaction.navigate(Navigation::End);
    assert_eq!(interaction.selected(), 7);

    interaction.pointer_moved(Some(4));
    assert_eq!(interaction.selected(), 7);
    assert_eq!(interaction.pointer_highlight(), Some(4));
    assert_eq!(interaction.pointer_accept(), Some(4));
    assert_eq!(interaction.selected(), 7);
}

#[test]
fn hit_testing_never_escapes_the_visible_rows_or_active_pane() {
    let candidates = (1..=4)
        .map(|id| ranked(id, &format!("candidate-{id}")))
        .collect::<Vec<_>>();
    let surface = project_surface(&request(), &candidates).unwrap();

    assert_eq!(
        surface.hit_test(Point::new(
            surface.bounds.x + 8.0,
            surface.bounds.y + surface.header_height + 5.0,
        )),
        Some(0)
    );
    assert_eq!(surface.hit_test(Point::new(0.0, 0.0)), None);
    assert_eq!(
        surface.hit_test(Point::new(
            surface.bounds.right() + 1.0,
            surface.bounds.y + 20.0,
        )),
        None
    );
}

#[test]
fn accessibility_announcements_are_coalesced_without_arbitrary_sleeps() {
    let mut gate = AnnouncementGate::new(250);
    assert!(gate.publish(1_000, "3 suggestions").is_some());
    assert!(gate.publish(1_100, "4 suggestions").is_none());
    assert_eq!(gate.publish(1_250, "4 suggestions"), Some("4 suggestions"));
    assert!(gate.publish(1_500, "4 suggestions").is_none());
    assert_eq!(
        gate.publish(1_750, "No suggestions"),
        Some("No suggestions")
    );
}

#[test]
fn geometry_rejects_non_finite_and_zero_sized_rectangles() {
    assert!(Rect::new(f32::NAN, 0.0, 10.0, 10.0).is_none());
    assert!(Rect::new(0.0, 0.0, 0.0, 10.0).is_none());
    assert!(Rect::new(0.0, 0.0, 10.0, -1.0).is_none());
    let geometry = SuggestionGeometry::default();
    assert!(geometry.minimum_width > 0.0);
    assert!(geometry.minimum_rows >= 1);
}
#[test]
fn surface_geometry_matrix_is_exact_and_cursor_anchored() {
    let candidates = (1..=3)
        .map(|id| ranked(id, &format!("candidate-{id}")))
        .collect::<Vec<_>>();
    let cases = [
        (
            "normal-100",
            SurfaceRequest {
                exclusions: Vec::new(),
                ..request()
            },
            (300.0, 370.0, 520.0, 110.0, true, true),
        ),
        (
            "ultrawide-100",
            SurfaceRequest {
                pane: Rect::new(0.0, 0.0, 3_440.0, 1_440.0).unwrap(),
                cursor: Rect::new(1_700.0, 1_100.0, 12.0, 24.0).unwrap(),
                exclusions: Vec::new(),
                ..request()
            },
            (1_700.0, 990.0, 520.0, 110.0, true, true),
        ),
        (
            "four-k-200",
            SurfaceRequest {
                pane: Rect::new(0.0, 0.0, 3_840.0, 2_160.0).unwrap(),
                cursor: Rect::new(1_920.0, 1_600.0, 24.0, 48.0).unwrap(),
                exclusions: Vec::new(),
                scale: 2.0,
                row_height: 48.0,
                preferred_width: 1_040.0,
                ..request()
            },
            (1_920.0, 1_380.0, 1_040.0, 220.0, true, true),
        ),
        (
            "eight-k-300",
            SurfaceRequest {
                pane: Rect::new(0.0, 0.0, 7_680.0, 4_320.0).unwrap(),
                cursor: Rect::new(3_840.0, 3_200.0, 36.0, 72.0).unwrap(),
                exclusions: Vec::new(),
                scale: 3.0,
                row_height: 72.0,
                preferred_width: 1_920.0,
                ..request()
            },
            (3_840.0, 2_870.0, 1_920.0, 330.0, true, true),
        ),
    ];

    for (name, input, expected) in cases {
        let surface = project_surface(&input, &candidates).unwrap();
        assert_eq!(surface.kind, SuggestionSurfaceKind::Listbox, "{name}");
        assert_eq!(
            (
                surface.bounds.x,
                surface.bounds.y,
                surface.bounds.width,
                surface.bounds.height,
                surface.show_description,
                surface.show_freshness,
            ),
            expected,
            "{name}",
        );
        assert!(input.pane.contains(surface.bounds), "{name}");
        assert!(!surface.bounds.intersects(input.cursor), "{name}");
    }
}

#[test]
fn narrow_pane_uses_compact_hint_instead_of_compressed_rows() {
    let input = SurfaceRequest {
        pane: Rect::new(0.0, 0.0, 220.0, 600.0).unwrap(),
        cursor: Rect::new(20.0, 300.0, 12.0, 24.0).unwrap(),
        exclusions: Vec::new(),
        ..request()
    };
    let surface = project_surface(&input, &[ranked(1, "get")]).unwrap();
    assert_eq!(surface.kind, SuggestionSurfaceKind::CompactHint);
    assert!(!surface.interactive);
    assert!(!surface.show_description);
    assert!(!surface.show_freshness);
}
