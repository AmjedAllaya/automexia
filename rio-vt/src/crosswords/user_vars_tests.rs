//! Real OSC dispatch regressions for retained terminal metadata.
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
    processor: &mut Processor,
    terminal: &mut Crosswords<VoidListener>,
    name: &str,
    value: &[u8],
) {
    processor.advance(terminal, &wire(name, value));
}

fn retained_bytes(terminal: &Crosswords<VoidListener>) -> usize {
    terminal
        .user_vars
        .iter()
        .map(|(name, value)| name.len() + value.len())
        .sum()
}

fn fill_count(processor: &mut Processor, terminal: &mut Crosswords<VoidListener>) {
    for index in 0..128 {
        publish(processor, terminal, &format!("k{index:03}"), b"v");
    }
}

fn fill_bytes(processor: &mut Processor, terminal: &mut Crosswords<VoidListener>) {
    for index in 0..8 {
        let remaining = 65_536 - retained_bytes(terminal);
        if remaining == 0 {
            break;
        }
        assert!(remaining >= 4);
        publish(
            processor,
            terminal,
            &format!("k{index:03}"),
            &vec![b'x'; (remaining - 4).min(8192)],
        );
    }
    assert_eq!(retained_bytes(terminal), 65_536);
}

#[test]
fn user_var_bounds_count_rejects_new_keys_and_keeps_existing_entries() {
    let mut terminal = terminal();
    let mut processor = Processor::default();
    fill_count(&mut processor, &mut terminal);
    assert_eq!(terminal.user_vars.len(), 128);
    for index in 0..512 {
        publish(
            &mut processor,
            &mut terminal,
            &format!("overflow{index}"),
            b"v",
        );
    }
    assert_eq!(terminal.user_vars.len(), 128);
    assert!(terminal.user_vars.keys().all(|name| name.starts_with('k')));
}

#[test]
fn user_var_bounds_saturated_replacement_and_empty_clear_keep_dictionary_semantics() {
    let mut terminal = terminal();
    let mut processor = Processor::default();
    fill_count(&mut processor, &mut terminal);
    publish(&mut processor, &mut terminal, "k000", b"replacement");
    assert_eq!(terminal.user_vars["k000"], "replacement");
    publish(&mut processor, &mut terminal, "k000", b"");
    assert_eq!(terminal.user_vars.get("k000").map(String::as_str), Some(""));
    assert_eq!(terminal.user_vars.len(), 128);
    publish(&mut processor, &mut terminal, "unknown_empty", b"");
    assert!(!terminal.user_vars.contains_key("unknown_empty"));
}

#[test]
fn user_var_bounds_exact_total_counts_names_and_replacement_without_eviction() {
    let mut terminal = terminal();
    let mut processor = Processor::default();
    fill_bytes(&mut processor, &mut terminal);
    let previous = terminal.user_vars.clone();
    publish(&mut processor, &mut terminal, "x", b"");
    assert!(
        terminal.user_vars == previous,
        "name bytes exceed the exact budget"
    );
    let current = terminal.user_vars["k007"].clone();
    publish(
        &mut processor,
        &mut terminal,
        "k007",
        format!("{current}x").as_bytes(),
    );
    assert!(
        terminal.user_vars == previous,
        "replacement is one byte over budget"
    );
    publish(&mut processor, &mut terminal, "k000", b"");
    assert_eq!(retained_bytes(&terminal), 65_536 - 8192);
    publish(&mut processor, &mut terminal, "new", b"ok");
    assert_eq!(terminal.user_vars["new"], "ok");
    assert_eq!(terminal.user_vars["k007"], current);
}

#[test]
fn user_var_bounds_name_and_value_limits_are_utf8_bytes_not_characters() {
    let mut terminal = terminal();
    let mut processor = Processor::default();
    let name = "\u{00e9}".repeat(64);
    let value = "\u{00e9}".repeat(4096);
    assert_eq!((name.len(), value.len()), (128, 8192));
    publish(&mut processor, &mut terminal, &name, value.as_bytes());
    assert!(
        terminal.user_vars[&name] == value,
        "exact UTF-8 boundary was not preserved"
    );
    let previous = terminal.user_vars.clone();
    publish(&mut processor, &mut terminal, &format!("{name}x"), b"small");
    assert!(
        terminal.user_vars == previous,
        "129-byte Unicode name must be rejected"
    );
    // 8192 and 8193 bytes can have the same padded base64 length. The
    // post-decode limit is necessary in addition to the encoded-length limit.
    publish(
        &mut processor,
        &mut terminal,
        &name,
        format!("{value}x").as_bytes(),
    );
    assert!(
        terminal.user_vars == previous,
        "8193-byte Unicode value must be rejected"
    );
}

#[test]
fn user_var_bounds_invalid_encodings_and_empty_name_preserve_previous_value() {
    let mut terminal = terminal();
    let mut processor = Processor::default();
    publish(&mut processor, &mut terminal, "kept", b"old");
    for frame in [
        b"\x1b]1337;SetUserVar=kept=!!!\x07".as_slice(),
        b"\x1b]1337;SetUserVar=kept=/w==\x07".as_slice(),
        b"\x1b]1337;SetUserVar=\xff=eA==\x07".as_slice(),
        b"\x1b]1337;SetUserVar==eA==\x07".as_slice(),
    ] {
        processor.advance(&mut terminal, frame);
    }
    assert_eq!(terminal.user_vars.len(), 1);
    assert!(
        terminal.user_vars["kept"] == "old",
        "rejected frame replaced the old value"
    );
    assert_eq!(terminal.grid.cursor.pos, Pos::new(Line(0), Column(0)));
}

#[test]
fn user_var_bounds_rejection_does_not_end_active_prompt_or_damage_cells() {
    for full_count in [true, false] {
        let mut terminal = terminal();
        let mut processor = Processor::default();
        processor.advance(&mut terminal, b"\x1b]133;A;aid=5\x07");
        assert!(terminal.active_semantic_prompt.is_some());
        if full_count {
            fill_count(&mut processor, &mut terminal);
        } else {
            publish(
                &mut processor,
                &mut terminal,
                "automexia_prompt_active",
                b"",
            );
            fill_bytes(&mut processor, &mut terminal);
        }
        terminal.reset_damage();
        let previous = terminal.user_vars.clone();
        publish(
            &mut processor,
            &mut terminal,
            "automexia_prompt_active",
            b"0",
        );
        assert!(
            terminal.active_semantic_prompt.is_some(),
            "rejected data changed lifecycle state"
        );
        assert!(
            terminal.user_vars == previous,
            "rejected update changed the dictionary"
        );
        assert_eq!(
            terminal.peek_damage_event(),
            None,
            "rejected data scheduled metadata damage"
        );
    }
}

#[test]
fn user_var_bounds_fragmented_oversize_sequence_recovers_to_metadata_and_text() {
    let mut data = wire("kept", b"old");
    data.extend(wire("kept", &vec![b'x'; 8193]));
    data.extend(wire("\u{00e9}\u{1f680}", "value e\u{301}".as_bytes()));
    data.extend(b"OK");
    for chunk_size in [1, 2, 7, 31, data.len()] {
        let mut terminal = terminal();
        let mut processor = Processor::default();
        for chunk in data.chunks(chunk_size) {
            processor.advance(&mut terminal, chunk);
        }
        assert_eq!(terminal.user_vars.len(), 2);
        assert!(
            terminal.user_vars["kept"] == "old",
            "rejected frame replaced the old value"
        );
        assert_eq!(terminal.user_vars["\u{00e9}\u{1f680}"], "value e\u{301}");
        assert_eq!(terminal.grid.cursor.pos, Pos::new(Line(0), Column(2)));
        assert_eq!(terminal.grid[Line(0)].inner[0].c(), 'O');
        assert_eq!(terminal.grid[Line(0)].inner[1].c(), 'K');
    }
}

#[test]
fn user_var_bounds_every_split_preserves_unicode_and_empty_clear() {
    let mut data = wire("\u{00e9}\u{1f680}", "value e\u{301}".as_bytes());
    data.extend(wire("\u{00e9}\u{1f680}", b""));
    for split in 0..=data.len() {
        let mut terminal = terminal();
        let mut processor = Processor::default();
        processor.advance(&mut terminal, &data[..split]);
        processor.advance(&mut terminal, &data[split..]);
        assert_eq!(terminal.user_vars.len(), 1);
        assert_eq!(terminal.user_vars["\u{00e9}\u{1f680}"], "");
        assert_eq!(terminal.grid.cursor.pos, Pos::new(Line(0), Column(0)));
    }
}

#[test]
fn user_var_bounds_direct_setter_and_session_isolation_share_the_same_limits() {
    let mut first = terminal();
    let mut second = terminal();
    let mut processor = Processor::default();
    fill_count(&mut processor, &mut first);
    first.set_user_var("direct".into(), "value".into());
    assert_eq!(first.user_vars.len(), 128);
    first.set_user_var("k000".into(), "x".repeat(8193));
    assert_eq!(first.user_vars["k000"], "v");
    publish(&mut processor, &mut second, "direct", b"value");
    assert_eq!(second.user_vars.len(), 1);
    assert_eq!(second.user_vars["direct"], "value");
    second.set_user_var("n".repeat(129), "value".into());
    second.set_user_var(String::new(), "value".into());
    assert_eq!(second.user_vars.len(), 1);
}
