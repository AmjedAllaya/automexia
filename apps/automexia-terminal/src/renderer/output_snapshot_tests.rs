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
    assert_eq!(expected[0].iter().filter(|&&on| on).count(), 8);
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
