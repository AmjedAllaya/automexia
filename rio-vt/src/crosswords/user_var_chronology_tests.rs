//! Wire-level accepted-write chronology and content-free rejection gaps.
use super::*;
use crate::event::VoidListener;
use crate::performer::handler::Processor;
use base64::{engine::general_purpose::STANDARD, Engine as _};

fn terminal() -> Crosswords<VoidListener> {
    Crosswords::new(
        CrosswordsSize::new(80, 8),
        CursorShape::Block,
        VoidListener {},
        WindowId::from(0),
        0,
        128,
    )
}
fn wire(name: &str, value: &[u8]) -> Vec<u8> {
    format!("\x1b]1337;SetUserVar={name}={}\x07", STANDARD.encode(value)).into_bytes()
}
fn publish(
    p: &mut Processor,
    t: &mut Crosswords<VoidListener>,
    name: &str,
    value: &[u8],
) {
    p.advance(t, &wire(name, value));
}
fn stamp(t: &Crosswords<VoidListener>, name: &str) -> (u64, Option<u64>) {
    let stamp = t
        .user_var_write_stamp(name)
        .expect("accepted write has provenance");
    (stamp.latest.get(), stamp.previous.map(NonZeroU64::get))
}

fn visible_rows(t: &Crosswords<VoidListener>) -> Vec<grid::row::Row<square::Square>> {
    (0..t.grid.screen_lines() as i32)
        .map(|line| t.grid[Line(line)].clone())
        .collect()
}

fn assert_metadata_invalidation_only(
    t: &Crosswords<VoidListener>,
    previous_rows: &[grid::row::Row<square::Square>],
) {
    assert_eq!(t.peek_damage_event(), Some(TerminalDamage::Partial));
    assert_eq!(
        visible_rows(t),
        previous_rows,
        "metadata invalidation changed cells"
    );
}

#[test]
fn user_var_chronology_new_terminal_is_valid_and_has_no_provenance() {
    let t = terminal();
    assert!(t.user_var_chronology_valid());
    assert_eq!(t.user_var_write_stamp("unknown"), None);
    assert_eq!(t.last_user_var_rejection(), None);
    assert_eq!(std::mem::size_of::<UserVarWriteStamp>(), 16);
}

#[test]
fn user_var_chronology_equal_and_empty_writes_keep_previous_successful_serial() {
    let mut t = terminal();
    let mut p = Processor::default();
    publish(&mut p, &mut t, "first", b"same");
    publish(&mut p, &mut t, "other", b"value");
    publish(&mut p, &mut t, "first", b"same");
    assert_eq!(stamp(&t, "first"), (3, Some(1)));
    publish(&mut p, &mut t, "first", b"");
    assert_eq!(stamp(&t, "first"), (4, Some(3)));
    assert_eq!(stamp(&t, "other"), (2, None));
    assert_eq!(t.user_vars["first"], "");
    assert_eq!(t.last_user_var_rejection(), None);
}

#[test]
fn user_var_chronology_malformed_recognized_frames_mark_rejections_only() {
    let mut t = terminal();
    let mut p = Processor::default();
    publish(&mut p, &mut t, "kept", b"old");
    let mut frames = vec![
        b"\x1b]1337;SetUserVar\x07".to_vec(),
        b"\x1b]1337;SetUserVar=missing_value\x07".to_vec(),
        b"\x1b]1337;SetUserVar==eA==\x07".to_vec(),
        b"\x1b]1337;SetUserVar=kept=!!!\x07".to_vec(),
        b"\x1b]1337;SetUserVar=kept=/w==\x07".to_vec(),
        b"\x1b]1337;SetUserVar=\xff=eA==\x07".to_vec(),
    ];
    frames.push(wire(&"n".repeat(129), b"x"));
    frames.push(wire("kept", &vec![b'x'; 8193]));
    frames.push(wire("kept", &vec![b'x'; 8194]));
    for (index, frame) in frames.into_iter().enumerate() {
        t.reset_damage();
        let previous_rows = visible_rows(&t);
        p.advance(&mut t, &frame);
        assert_eq!(
            t.last_user_var_rejection().map(NonZeroU64::get),
            Some(index as u64 + 2)
        );
        assert_eq!(stamp(&t, "kept"), (1, None));
        assert_eq!(t.user_vars.len(), 1);
        assert_eq!(t.user_vars["kept"], "old");
        assert_metadata_invalidation_only(&t, &previous_rows);
    }
    publish(&mut p, &mut t, "kept", b"recovered");
    assert_eq!(stamp(&t, "kept"), (11, Some(1)));
    assert_eq!(t.last_user_var_rejection().map(NonZeroU64::get), Some(10));
}

#[test]
fn user_var_chronology_unrelated_osc_commands_do_not_consume_serials() {
    let mut t = terminal();
    let mut p = Processor::default();
    p.advance(&mut t, b"\x1b]1337;SetUserVariable=k=eA==\x07\x1b]13370;SetUserVar=k=eA==\x07\x1b]1337;Unknown\x07\x1b]0;title\x07");
    assert_eq!(t.last_user_var_rejection(), None);
    publish(&mut p, &mut t, "key", b"value");
    assert_eq!(stamp(&t, "key"), (1, None));
}

#[test]
fn user_var_chronology_saturated_count_marks_gap_then_updates_existing_key() {
    let mut t = terminal();
    let mut p = Processor::default();
    for i in 0..128 {
        publish(&mut p, &mut t, &format!("k{i:03}"), b"v");
    }
    publish(&mut p, &mut t, "overflow", b"");
    assert_eq!(t.last_user_var_rejection().map(NonZeroU64::get), Some(129));
    assert_eq!(t.user_var_write_stamp("overflow"), None);
    publish(&mut p, &mut t, "k000", b"changed");
    publish(&mut p, &mut t, "k000", b"");
    assert_eq!(stamp(&t, "k000"), (131, Some(130)));
    assert_eq!(t.user_vars.len(), 128);
    assert_eq!(t.user_var_write_stamps.len(), 128);
}

#[test]
fn user_var_chronology_byte_saturation_and_clear_preserve_rejection_order() {
    let mut t = terminal();
    let mut p = Processor::default();
    for i in 0..8 {
        let length = if i == 7 { 8176 } else { 8192 };
        publish(&mut p, &mut t, &format!("k{i}"), &vec![b'x'; length]);
    }
    assert_eq!(
        t.user_vars
            .iter()
            .map(|(k, v)| k.len() + v.len())
            .sum::<usize>(),
        65_536
    );
    publish(&mut p, &mut t, "x", b"");
    publish(&mut p, &mut t, "k7", &vec![b'x'; 8177]);
    assert_eq!(t.last_user_var_rejection().map(NonZeroU64::get), Some(10));
    assert_eq!(stamp(&t, "k7"), (8, None));
    publish(&mut p, &mut t, "k0", b"");
    publish(&mut p, &mut t, "x", b"new");
    assert_eq!(stamp(&t, "k0"), (11, Some(1)));
    assert_eq!(stamp(&t, "x"), (12, None));
}

#[test]
fn user_var_chronology_first_rejected_attempt_is_not_legacy_without_metadata() {
    let mut t = terminal();
    // Trusted host setup has no accepted-wire history; the first wire attempt
    // still must publish a rejection gap when this inherited map is full.
    for i in 0..128 {
        t.user_vars.insert(format!("k{i:03}"), String::new());
    }
    let mut p = Processor::default();
    publish(&mut p, &mut t, "frame_begin", b"1");
    assert!(t.user_var_chronology_valid());
    assert_eq!(t.last_user_var_rejection().map(NonZeroU64::get), Some(1));
    assert!(t.user_var_write_stamps.is_empty());
    publish(&mut p, &mut t, "k000", b"new");
    assert_eq!(stamp(&t, "k000"), (2, None));
}

#[test]
fn user_var_chronology_raw_osc_overflow_reports_rejection_without_dispatching_prefix() {
    let mut t = terminal();
    let mut p = Processor::default();
    publish(&mut p, &mut t, "kept", b"old");
    let mut huge = b"\x1b]1337;SetUserVar=kept=".to_vec();
    huge.extend(vec![b'A'; 1024 * 1024 + 64]);
    p.advance(&mut t, &huge);
    assert_eq!(
        t.last_user_var_rejection(),
        None,
        "unfinished OSC is not an event"
    );
    t.reset_damage();
    let previous_rows = visible_rows(&t);
    p.advance(&mut t, b"\x07");
    assert_eq!(t.last_user_var_rejection().map(NonZeroU64::get), Some(2));
    assert_eq!(t.user_vars["kept"], "old");
    assert_eq!(stamp(&t, "kept"), (1, None));
    assert_metadata_invalidation_only(&t, &previous_rows);
    let mut unrelated = b"\x1b]0;".to_vec();
    unrelated.extend(vec![b'A'; 1024 * 1024 + 64]);
    unrelated.push(7);
    p.advance(&mut t, &unrelated);
    assert_eq!(t.last_user_var_rejection().map(NonZeroU64::get), Some(2));
    publish(&mut p, &mut t, "kept", b"new");
    assert_eq!(stamp(&t, "kept"), (3, Some(1)));
}

#[test]
fn user_var_chronology_fragmentation_keeps_equal_write_and_rejection_order() {
    let mut data = wire("\u{00e9}", b"same");
    data.extend(wire("other", b"two"));
    data.extend(wire("\u{00e9}", b"same"));
    data.extend(wire("\u{00e9}", b""));
    data.extend(b"\x1b]1337;SetUserVar=bad=!!!\x07");
    data.extend(wire("\u{00e9}", b"last"));
    for split in 0..=data.len() {
        let mut t = terminal();
        let mut p = Processor::default();
        p.advance(&mut t, &data[..split]);
        p.advance(&mut t, &data[split..]);
        assert_eq!(stamp(&t, "\u{00e9}"), (6, Some(4)));
        assert_eq!(stamp(&t, "other"), (2, None));
        assert_eq!(t.last_user_var_rejection().map(NonZeroU64::get), Some(5));
    }
}

#[test]
fn user_var_chronology_prunes_removed_keys_and_keeps_side_storage_bounded() {
    let mut t = terminal();
    let mut p = Processor::default();
    for i in 0..128 {
        publish(&mut p, &mut t, &format!("k{i:03}"), b"v");
    }
    assert_eq!(t.user_var_write_stamps.len(), 128);
    let mut victim = "k000".to_string();
    for i in 0..256 {
        t.user_vars.remove(&victim);
        assert_eq!(t.user_var_write_stamp(&victim), None);
        let replacement = format!("r{i:03}");
        publish(&mut p, &mut t, &replacement, b"v");
        assert!(!t.user_var_write_stamps.contains_key(&victim));
        assert_eq!(t.user_var_write_stamps.len(), 128);
        assert!(
            t.user_var_write_stamps
                .keys()
                .map(String::len)
                .sum::<usize>()
                <= 16 * 1024
        );
        victim = replacement;
    }
}

#[test]
fn user_var_chronology_reset_retains_history_and_new_terminal_is_independent() {
    let mut first = terminal();
    let mut p = Processor::default();
    publish(&mut p, &mut first, "key", b"one");
    p.advance(&mut first, b"\x1bc");
    assert_eq!(stamp(&first, "key"), (1, None));
    publish(&mut p, &mut first, "key", b"two");
    assert_eq!(stamp(&first, "key"), (2, Some(1)));
    let mut second = terminal();
    let mut other = Processor::default();
    publish(&mut other, &mut second, "key", b"one");
    assert_eq!(stamp(&second, "key"), (1, None));
}

#[test]
fn user_var_chronology_direct_setter_failures_are_content_free_events() {
    let mut t = terminal();
    t.set_user_var("key".into(), "old".into());
    t.reset_damage();
    let previous_rows = visible_rows(&t);
    t.set_user_var(String::new(), "x".into());
    t.set_user_var("key".into(), "x".repeat(8193));
    assert_eq!(t.last_user_var_rejection().map(NonZeroU64::get), Some(3));
    assert_eq!(stamp(&t, "key"), (1, None));
    assert_eq!(t.user_vars["key"], "old");
    assert_metadata_invalidation_only(&t, &previous_rows);
    t.set_user_var("key".into(), "new".into());
    assert_eq!(stamp(&t, "key"), (4, Some(1)));
}

#[test]
fn user_var_chronology_checked_exhaustion_never_wraps_or_mutates_prompt_state() {
    let mut t = terminal();
    let mut p = Processor::default();
    p.advance(&mut t, b"\x1b]133;A;aid=5\x07");
    publish(&mut p, &mut t, "automexia_prompt_active", b"1");
    t.user_var_clock = u64::MAX - 1;
    publish(&mut p, &mut t, "automexia_prompt_active", b"1");
    assert_eq!(stamp(&t, "automexia_prompt_active"), (u64::MAX, Some(1)));
    assert!(t.user_var_chronology_valid());
    t.reset_damage();
    let previous_rows = visible_rows(&t);
    publish(&mut p, &mut t, "automexia_prompt_active", b"0");
    assert!(!t.user_var_chronology_valid());
    assert_eq!(t.user_var_clock, u64::MAX);
    assert_eq!(t.user_var_write_stamp("automexia_prompt_active"), None);
    assert_eq!(t.user_vars["automexia_prompt_active"], "1");
    assert!(t.active_semantic_prompt.is_some());
    assert_metadata_invalidation_only(&t, &previous_rows);
    t.reset_damage();
    publish(&mut p, &mut t, "later", b"ignored");
    assert_eq!(
        t.peek_damage_event(),
        None,
        "already invalid chronology must not reschedule damage"
    );
    assert!(!t.user_vars.contains_key("later"));
    assert_eq!(t.user_var_clock, u64::MAX);
}

#[test]
fn user_var_chronology_rejection_at_last_serial_then_overflow_stays_invalid() {
    let mut t = terminal();
    let mut p = Processor::default();
    t.user_var_clock = u64::MAX - 1;
    p.advance(&mut t, b"\x1b]1337;SetUserVar=key=!!!\x07");
    assert_eq!(
        t.last_user_var_rejection().map(NonZeroU64::get),
        Some(u64::MAX)
    );
    assert!(t.user_var_chronology_valid());
    p.advance(&mut t, b"\x1b]1337;SetUserVar=key=!!!\x07");
    assert!(!t.user_var_chronology_valid());
    assert!(t.user_vars.is_empty());
    assert_eq!(t.user_var_clock, u64::MAX);
}

#[test]
fn user_var_chronology_terminator_and_value_controls_remain_supported() {
    for terminator in ["\x07", "\x1b\\"] {
        for (encoded, expected) in [("eA==", "x"), ("eA", "x"), ("", "")] {
            let frame = format!("\x1b]1337;SetUserVar=key={encoded}{terminator}");
            for split in 0..=frame.len() {
                let mut t = terminal();
                let mut p = Processor::default();
                p.advance(&mut t, &frame.as_bytes()[..split]);
                p.advance(&mut t, &frame.as_bytes()[split..]);
                assert_eq!(t.user_vars["key"], expected);
                assert_eq!(stamp(&t, "key"), (1, None));
                assert_eq!(t.last_user_var_rejection(), None);
            }
        }
    }
}

#[test]
fn user_var_chronology_extra_parameters_never_accept_a_valid_prefix() {
    for terminator in ["\x07", "\x1b\\"] {
        for body in [
            "SetUserVar=key=bmV3;invalid",
            "SetUserVar=key=bmV3;",
            "SetUserVar=key=;invalid",
            "SetUserVar=automexia_prompt_active=MA==;invalid",
        ] {
            let frame = format!("\x1b]1337;{body}{terminator}");
            for split in 0..=frame.len() {
                let mut t = terminal();
                let mut p = Processor::default();
                p.advance(&mut t, b"\x1b]133;A;aid=5\x07");
                publish(&mut p, &mut t, "key", b"old");
                publish(&mut p, &mut t, "automexia_prompt_active", b"1");
                let cursor = (t.grid.cursor.pos, t.grid.cursor.should_wrap);
                t.reset_damage();
                let previous_rows = visible_rows(&t);
                p.advance(&mut t, &frame.as_bytes()[..split]);
                p.advance(&mut t, &frame.as_bytes()[split..]);
                assert_eq!(t.last_user_var_rejection().map(NonZeroU64::get), Some(3));
                assert_eq!(t.user_vars["key"], "old");
                assert_eq!(t.user_vars["automexia_prompt_active"], "1");
                assert_eq!(stamp(&t, "key"), (1, None));
                assert_eq!(stamp(&t, "automexia_prompt_active"), (2, None));
                assert_eq!(t.user_vars.len(), 2);
                assert!(t.active_semantic_prompt.is_some());
                assert_eq!(t.semantic_prompt_id, Some(5));
                assert_eq!((t.grid.cursor.pos, t.grid.cursor.should_wrap), cursor);
                assert_metadata_invalidation_only(&t, &previous_rows);
                publish(&mut p, &mut t, "key", b"recovered");
                assert_eq!(stamp(&t, "key"), (4, Some(1)));
                assert_eq!(t.user_vars["key"], "recovered");
                assert_eq!(t.last_user_var_rejection().map(NonZeroU64::get), Some(3));
                publish(&mut p, &mut t, "automexia_prompt_active", b"0");
                assert_eq!(stamp(&t, "automexia_prompt_active"), (5, Some(2)));
                assert!(t.active_semantic_prompt.is_none());
            }
        }
    }
}
