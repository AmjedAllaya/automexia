use super::refresh_output_classifications;
use crate::automexia::output_semantics::{OutputClassification, OutputDomain};
use crate::context::renderable::RenderableContent;
use automexia_extension_api::SemanticSeverity;
use rio_backend::crosswords::{Crosswords, CrosswordsSize};
use rio_backend::event::{TerminalDamage, VoidListener, WindowId};
use rio_backend::performer::handler::Processor;

fn wrapped_fixture() -> (Crosswords<VoidListener>, RenderableContent) {
    let mut terminal = Crosswords::new(
        CrosswordsSize::new(24, 8),
        rio_backend::ansi::CursorShape::Block,
        VoidListener,
        WindowId::from(0),
        7,
        64,
    );
    // Exactly two physical rows of one resource name; all status fields live
    // on its final wrapped fragment. The next line is unrelated plain output.
    let source = format!(
        "pod-{}  0/1  Unknown  0  3m\r\nordinary.txt\r\n",
        "x".repeat(44)
    );
    Processor::default().advance(&mut terminal, source.as_bytes());
    let mut content = RenderableContent {
        columns: 24,
        screen_lines: 8,
        ..Default::default()
    };
    snapshot(&mut terminal, &mut content, TerminalDamage::Full);
    (terminal, content)
}

fn snapshot(
    terminal: &mut Crosswords<VoidListener>,
    content: &mut RenderableContent,
    damage: TerminalDamage,
) {
    terminal.snapshot_visible(
        &damage,
        content.columns,
        &mut content.visible_rows,
        &mut content.style_table,
        &mut content.extras,
    );
    terminal.reset_damage();
    content.frame_damage = damage;
}

fn consumed_rows(content: &mut RenderableContent) {
    for row in &mut content.visible_rows {
        row.dirty = false;
    }
    content.frame_damage = TerminalDamage::Noop;
}

#[test]
fn input_accents_refresh_through_snapshot_owner_and_remain_pane_local() {
    let (mut terminal, mut content) = wrapped_fixture();
    let (_, mut other) = wrapped_fixture();
    terminal
        .user_vars
        .insert("automexia_shell".into(), "1".into());
    terminal
        .user_vars
        .insert("automexia_shell_name".into(), "bash".into());
    Processor::default().advance(
        &mut terminal,
        b"\x1b[2J\x1b[H\x1b]133;A;aid=99\x07> \x1b]133;B\x07docker ps -a",
    );
    snapshot(&mut terminal, &mut content, TerminalDamage::Full);
    refresh_output_classifications(&mut content, TerminalDamage::Full, true, false);
    refresh_output_classifications(&mut other, TerminalDamage::Full, true, false);
    let expected = content.input_accents.rows.clone();
    assert_eq!(expected[0].iter().flatten().count(), 10);
    consumed_rows(&mut content);
    let allocation = content.input_accents.rows.as_ptr();
    for damage in [TerminalDamage::Noop, TerminalDamage::CursorOnly] {
        refresh_output_classifications(&mut content, damage, true, false);
        assert_eq!(content.input_accents.rows.as_ptr(), allocation);
        assert_eq!(content.input_accents.rows, expected);
        assert!(content.visible_rows.iter().all(|row| !row.dirty));
    }
    refresh_output_classifications(&mut content, TerminalDamage::Noop, false, false);
    assert!(content.input_accents.rows.iter().all(Vec::is_empty));
    assert!(other.input_accents.rows.iter().all(Vec::is_empty));
    refresh_output_classifications(&mut content, TerminalDamage::Noop, true, false);
    assert_eq!(content.input_accents.rows, expected);
}

#[test]
fn bash_multiline_input_keeps_token_colors_after_submission_only_on_owned_rows() {
    use crate::renderer::command_input::CommandSpanKind::{Argument, Command, Option};

    let mut terminal = Crosswords::new(
        CrosswordsSize::new(80, 12),
        rio_backend::ansi::CursorShape::Block,
        VoidListener,
        WindowId::from(0),
        7,
        64,
    );
    terminal
        .user_vars
        .insert("automexia_shell".into(), "1".into());
    terminal
        .user_vars
        .insert("automexia_shell_name".into(), "bash".into());
    let mut content = RenderableContent {
        columns: 80,
        screen_lines: 12,
        ..Default::default()
    };
    let mut parser = Processor::default();
    parser.advance(
        &mut terminal,
        b"\x1b]133;A;aid=1\x07> \x1b]133;B\x07python3 --version\r\nnode --version\r\nnpm --version\r\ndocker version\r\ndocker compose version\r\nopenssl version",
    );
    snapshot(&mut terminal, &mut content, TerminalDamage::Full);
    refresh_output_classifications(&mut content, TerminalDamage::Full, true, false);
    let input = &content.input_accents.rows;
    assert!(input[0][..2].iter().all(|span| span.is_none()));
    assert_eq!(input[0][2], Some(Command));
    assert_eq!(input[0][10], Some(Option));
    for row in &input[1..6] {
        assert_eq!(row[0], Some(Command));
    }
    assert_eq!(input[1][5], Some(Option));
    assert_eq!(input[2][4], Some(Option));
    assert_eq!(input[3][7], Some(Argument));
    assert_eq!(input[4][7], Some(Argument));
    assert_eq!(input[4][15], Some(Argument));
    assert_eq!(input[5][8], Some(Argument));
    let before_submit = input[..6].to_vec();

    parser.advance(
        &mut terminal,
        b"\r\n\x1b]133;C\x07node --version\r\n\x1b]133;D;0\x07\x1b]133;A;aid=2\x07> \x1b]133;B\x07sudo apt install nodejs\r\n\x1b]133;C\x07[sudo: authenticate] Password:",
    );
    snapshot(&mut terminal, &mut content, TerminalDamage::Full);
    refresh_output_classifications(&mut content, TerminalDamage::Full, true, false);
    let input = &content.input_accents.rows;
    assert_eq!(input[..6], before_submit);
    assert!(
        input[6].is_empty(),
        "output that resembles a command stays unowned"
    );
    assert_eq!(input[7][2], Some(Command));
    for column in [7, 11, 19] {
        assert_eq!(input[7][column], Some(Argument));
    }
    assert!(
        input[8].is_empty(),
        "the program-owned password prompt stays unowned"
    );
}

#[test]
fn wrapped_status_tail_damage_rebuilds_unchanged_prefixes_in_the_real_snapshot_owner() {
    let (mut terminal, mut content) = wrapped_fixture();
    refresh_output_classifications(&mut content, TerminalDamage::Full, true, false);
    let warning = Some(OutputClassification {
        domain: OutputDomain::Kubernetes,
        severity: Some(SemanticSeverity::Warning),
    });
    let success = Some(OutputClassification {
        domain: OutputDomain::Kubernetes,
        severity: Some(SemanticSeverity::Success),
    });
    assert_eq!(&content.output_classifications[..3], &[warning; 3]);
    assert_eq!(content.output_classifications[3], None);
    consumed_rows(&mut content);
    let cursor = terminal.cursor();
    // Only READY and STATUS change. Restore the cursor to its prior position,
    // matching in-place watch output that produces no resize or cursor change.
    Processor::default().advance(&mut terminal, b"\x1b[3;3H1/1  Running\x1b[5;1H");
    assert_eq!(terminal.cursor(), cursor);
    assert_eq!(terminal.peek_damage_event(), Some(TerminalDamage::Partial));
    snapshot(&mut terminal, &mut content, TerminalDamage::Partial);
    assert!(!content.visible_rows[0].dirty);
    assert!(!content.visible_rows[1].dirty);
    assert!(content.visible_rows[2].dirty);
    assert!(!content.visible_rows[3].dirty);
    refresh_output_classifications(&mut content, TerminalDamage::Partial, true, false);
    assert_eq!(&content.output_classifications[..3], &[success; 3]);
    assert!(content.visible_rows[..3].iter().all(|row| row.dirty));
    assert!(
        !content.visible_rows[3].dirty,
        "unrelated output must remain cached"
    );
    assert_eq!(content.frame_damage, TerminalDamage::Partial);
    assert_eq!(
        &content.output_background_protected[..4],
        &[true, true, true, false]
    );
}

#[test]
fn package_progress_tail_damage_repaints_fields_even_when_status_matches() {
    use crate::automexia::output_semantics::OperationalPart;
    use crate::grid_emit::output_accents_at;

    let mut terminal = Crosswords::new(
        CrosswordsSize::new(24, 8),
        rio_backend::ansi::CursorShape::Block,
        VoidListener,
        WindowId::from(0),
        7,
        64,
    );
    let mut parser = Processor::default();
    parser.advance(
        &mut terminal,
        b"Get:1 https://repo.invalid stable/main amd64 pkg-error amd64 1.2-3 [241 kB]\r\nplain.txt\r\n",
    );
    let mut content = RenderableContent {
        columns: 24,
        screen_lines: 8,
        ..Default::default()
    };
    snapshot(&mut terminal, &mut content, TerminalDamage::Full);
    refresh_output_classifications(&mut content, TerminalDamage::Full, true, false);
    let classified = content.output_classifications.clone();
    let spans = output_accents_at(&content.output_accents, 1).unwrap();
    assert!(spans
        .iter()
        .flatten()
        .any(|span| span.role == OperationalPart::Subject));
    assert_eq!(
        classified[1].unwrap().severity,
        Some(SemanticSeverity::Info)
    );

    consumed_rows(&mut content);
    let cursor = terminal.cursor();
    // An in-place rewrite invalidates the package version in the third row.
    // The phase stays informational, but the second row loses its package color.
    parser.advance(&mut terminal, b"\x1b[3;14Hx\x1b[6;1H");
    assert_eq!(terminal.cursor(), cursor);
    assert_eq!(terminal.peek_damage_event(), Some(TerminalDamage::Partial));
    snapshot(&mut terminal, &mut content, TerminalDamage::Partial);
    assert!(!content.visible_rows[1].dirty);
    refresh_output_classifications(&mut content, TerminalDamage::Partial, true, false);
    assert_eq!(content.output_classifications, classified);
    assert!(content.visible_rows[1].dirty);
    assert!(!content.visible_rows[4].dirty);
    let spans = output_accents_at(&content.output_accents, 1).unwrap();
    let remaining: Vec<_> = spans.iter().flatten().map(|span| span.role).collect();
    assert_eq!(remaining, [OperationalPart::Detail]);

    refresh_output_classifications(&mut content, TerminalDamage::Noop, false, false);
    assert!(content.output_accents.is_empty());
    assert_eq!(content.frame_damage, TerminalDamage::Full);
}

#[test]
#[ignore = "same-host package output viewport projection benchmark"]
fn package_progress_viewport_benchmark() {
    use std::{hint::black_box, time::Instant};

    let records = [
        "Get:1 https://repo.invalid stable/main amd64 demo amd64 1.2.3 [241 kB]",
        "Unpacking demo:amd64 (1.2.3) over (1.2.2) ...",
        "Universal Time is now: Mon Jan 5 08:00:00 UTC 2026.",
    ];
    let batches = 16;
    let source = (records.join("\r\n") + "\r\nordinary output\r\n").repeat(batches);
    let screen_lines = 120;
    for columns in [40, 120] {
        let mut terminal = Crosswords::new(
            CrosswordsSize::new(columns, screen_lines),
            rio_backend::ansi::CursorShape::Block,
            VoidListener,
            WindowId::from(0),
            7,
            64,
        );
        Processor::default().advance(&mut terminal, source.as_bytes());
        let mut content = RenderableContent {
            columns,
            screen_lines,
            ..Default::default()
        };
        snapshot(&mut terminal, &mut content, TerminalDamage::Full);
        let expected_rows = batches
            * records
                .iter()
                .map(|line| line.len().div_ceil(columns))
                .sum::<usize>();
        let mut samples = Vec::with_capacity(200);
        for iteration in 0..220 {
            let start = Instant::now();
            refresh_output_classifications(
                black_box(&mut content),
                TerminalDamage::Full,
                true,
                false,
            );
            let elapsed = start.elapsed();
            assert_eq!(content.output_accents.len(), expected_rows);
            if iteration >= 20 {
                samples.push(elapsed.as_nanos());
            }
        }
        samples.sort_unstable();
        println!(
            "{}",
            serde_json::json!({
                "benchmark": "package_progress_viewport",
                "columns": columns,
                "rows": screen_lines,
                "package_records": records.len() * batches,
                "samples": samples.len(),
                "p50_ns": samples[99],
                "p95_ns": samples[189],
                "excludes": ["PTY I/O", "VT parsing", "glyph rendering", "presentation"],
            })
        );
    }
}

#[test]
fn output_snapshot_idle_frames_reuse_cache_and_mode_changes_repaint_only_its_owner() {
    let (_, mut content) = wrapped_fixture();
    let (_, mut other_pane) = wrapped_fixture();
    refresh_output_classifications(&mut content, TerminalDamage::Full, true, false);
    refresh_output_classifications(&mut other_pane, TerminalDamage::Full, true, false);
    consumed_rows(&mut content);
    consumed_rows(&mut other_pane);
    let original = content.output_classifications.clone();
    let allocation = content.output_classifications.as_ptr();
    let other_original = other_pane.output_classifications.clone();
    for damage in [TerminalDamage::Noop, TerminalDamage::CursorOnly] {
        content.frame_damage = damage;
        refresh_output_classifications(&mut content, damage, true, false);
        assert_eq!(content.output_classifications, original);
        assert_eq!(content.output_classifications.as_ptr(), allocation);
        assert!(content.visible_rows.iter().all(|row| !row.dirty));
        assert_eq!(content.frame_damage, damage);
    }
    content.frame_damage = TerminalDamage::Noop;
    content.command_rows.clear_before(2);
    content.command_rows.rebuild(6, &[]);
    content.command_rows.settle(Some(4), 6);
    assert_eq!(content.command_rows.visual_row(2), 0);
    refresh_output_classifications(&mut content, TerminalDamage::Noop, false, false);
    content.command_rows.rebuild(6, &[]);
    content.command_rows.settle(Some(4), 6);
    assert_eq!(
        content.command_rows.visual_row(2),
        2,
        "alternate/mouse screens retire host-clear placement"
    );
    assert!(!content.output_colors_eligible);
    assert!(content.output_classifications.is_empty());
    assert!(content.output_background_protected.is_empty());
    assert_eq!(content.frame_damage, TerminalDamage::Full);
    content.frame_damage = TerminalDamage::Noop;
    refresh_output_classifications(&mut content, TerminalDamage::Noop, true, false);
    assert!(content.output_colors_eligible);
    assert_eq!(content.output_classifications, original);
    assert_eq!(content.frame_damage, TerminalDamage::Full);
    assert_eq!(other_pane.output_classifications, other_original);
    assert_eq!(other_pane.frame_damage, TerminalDamage::Noop);
    assert!(other_pane.visible_rows.iter().all(|row| !row.dirty));
}
