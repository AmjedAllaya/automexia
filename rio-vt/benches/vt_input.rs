// VT input throughput: where the bytes go when a program floods the
// terminal. Separates the SIMD plain-text path from escape-dense input so
// regressions in either show up on their own.

use criterion::{criterion_group, criterion_main, Criterion, Throughput};

use rio_vt::ansi::CursorShape;
use rio_vt::crosswords::grid::row::Row;
use rio_vt::crosswords::grid::Scroll;
use rio_vt::crosswords::pos::{Column, Line, Pos, Side};
use rio_vt::crosswords::square::{Extras, Square};
use rio_vt::crosswords::style::Style;
use rio_vt::crosswords::{Crosswords, CrosswordsSize};
use rio_vt::event::{TerminalDamage, VoidListener, WindowId};
use rio_vt::performer::handler::Processor;
use rio_vt::selection::{Anchor, Selection, SelectionMotion, SelectionType};

const COLS: usize = 120;
const ROWS: usize = 40;

fn term() -> Crosswords<VoidListener> {
    Crosswords::new(
        CrosswordsSize::new(COLS, ROWS),
        CursorShape::Block,
        VoidListener {},
        WindowId::from(0),
        0,
        2000,
    )
}

fn plain(total: usize) -> Vec<u8> {
    let line =
        "the quick brown fox jumps over the lazy dog 0123456789 lorem ipsum dolor sit amet\r\n";
    line.repeat(total / line.len() + 1).into_bytes()
}

/// The benchmark suite's ANSI mix: 256-color pairs, truecolor bold,
/// italic/underline toggles, resets between every word.
fn ansi_mixed(total: usize) -> Vec<u8> {
    let mut line = String::new();
    for i in 0..10 {
        line += &format!(
            "\x1b[38;5;{}m\x1b[48;5;{}mword{i}\x1b[0m ",
            (i * 37) % 256,
            (i * 53 + 8) % 256
        );
        line += &format!(
            "\x1b[1;38;2;{};{};{}mbold\x1b[0m ",
            (i * 31) % 256,
            (i * 67) % 256,
            (i * 13) % 256
        );
        line += if i % 2 == 1 {
            "\x1b[3mital\x1b[0m "
        } else {
            "\x1b[4munder\x1b[0m "
        };
    }
    line += "\r\n";
    line.repeat(total / line.len() + 1).into_bytes()
}

fn semantic_result_stream(commands: usize, verified_status: bool) -> Vec<u8> {
    let mut stream = String::with_capacity(commands.saturating_mul(96));
    for generation in 1..=commands {
        if verified_status {
            stream.push_str(&format!(
                "\x1b]133;A;aid={generation}\x07> \x1b]133;B\x07echo {generation}\r\n\
                 \x1b]133;C\x07result-{generation}\r\n\x1b]133;D;0\x07"
            ));
        } else {
            stream.push_str(&format!(
                "\x1b]133;A\x07> \x1b]133;B\x07dir\r\nresult-{generation}\r\n\x1b]133;D\x07"
            ));
        }
    }
    stream.into_bytes()
}

fn semantic_result_term() -> Crosswords<VoidListener> {
    Crosswords::new(
        CrosswordsSize::new(COLS, ROWS),
        CursorShape::Block,
        VoidListener {},
        WindowId::from(0),
        0,
        4_000,
    )
}

fn semantic_overflow_result_stream(output_rows: usize) -> Vec<u8> {
    let mut stream = String::with_capacity(output_rows.saturating_mul(24));
    stream.push_str("\x1b]133;A;aid=1\x07> \x1b]133;B\x07overflow\r\n\x1b]133;C\x07");
    for row in 0..output_rows {
        stream.push_str(&format!("overflow-result-{row}\r\n"));
    }
    stream.push_str("\x1b]133;D;0\x07\x1b]133;A;aid=2\x07> ");
    stream.into_bytes()
}

/// Same word rhythm, only two styles alternating: isolates SGR parse and
/// dispatch cost from style-table churn.
fn ansi_two_styles(total: usize) -> Vec<u8> {
    let mut line = String::new();
    for i in 0..30 {
        line += if i % 2 == 0 {
            "\x1b[31mword\x1b[0m "
        } else {
            "\x1b[44mword\x1b[0m "
        };
    }
    line += "\r\n";
    line.repeat(total / line.len() + 1).into_bytes()
}

/// Escape density without SGR payloads: bare reset sequences between
/// words, measuring the state-machine and dispatch floor.
fn ansi_bare_csi(total: usize) -> Vec<u8> {
    let mut line = String::new();
    for _ in 0..30 {
        line += "\x1b[mword ";
    }
    line += "\r\n";
    line.repeat(total / line.len() + 1).into_bytes()
}

/// Sink that counts instead of mutating a grid: parser cost in isolation.
#[derive(Default)]
struct NoopPerform {
    printed: u64,
    csis: u64,
}

impl rio_vt::performer::parser::Perform for NoopPerform {
    fn print_codepoints(&mut self, codepoints: &[u32]) {
        self.printed += codepoints.len() as u64;
    }
    fn csi_dispatch(
        &mut self,
        _params: &rio_vt::performer::parser::Params,
        _intermediates: &[u8],
        _ignore: bool,
        _action: char,
    ) {
        self.csis += 1;
    }
}

fn bench(c: &mut Criterion) {
    c.bench_function("processor_default_lazy_control_buffers", |b| {
        b.iter(Processor::default);
    });

    const BYTES: usize = 4 * 1024 * 1024;
    let cases: [(&str, Vec<u8>); 4] = [
        ("plain", plain(BYTES)),
        ("ansi_mixed", ansi_mixed(BYTES)),
        ("ansi_two_styles", ansi_two_styles(BYTES)),
        ("ansi_bare_csi", ansi_bare_csi(BYTES)),
    ];

    let mut group = c.benchmark_group("vt_input");
    group.sample_size(20);
    for (name, bytes) in &cases {
        group.throughput(Throughput::Bytes(bytes.len() as u64));
        group.bench_function(*name, |b| {
            b.iter_batched(
                || (term(), Processor::default()),
                |(mut crosswords, mut processor)| {
                    processor.advance(&mut crosswords, bytes);
                    crosswords
                },
                criterion::BatchSize::LargeInput,
            );
        });
        // Parser in isolation: the gap to the full path above is the
        // handler + grid cost.
        group.bench_function(format!("{name}_parser_only"), |b| {
            b.iter_batched(
                || {
                    (
                        rio_vt::performer::parser::Parser::new(),
                        NoopPerform::default(),
                    )
                },
                |(mut parser, mut sink)| {
                    parser.advance(&mut sink, bytes);
                    sink
                },
                criterion::BatchSize::LargeInput,
            );
        });
    }
    group.finish();

    // Common interactive fast path: local UI actions and cursor motion reuse
    // the already-materialized terminal snapshot when no grid row changed.
    let mut crosswords = term();
    let mut processor = Processor::default();
    processor.advance(&mut crosswords, &ansi_mixed(64 * 1024));
    let mut rows: Vec<Row<Square>> = Vec::new();
    let mut styles: Vec<Style> = Vec::new();
    let mut extras = rustc_hash::FxHashMap::<u16, Extras>::default();
    crosswords.snapshot_visible(
        &TerminalDamage::Full,
        COLS,
        &mut rows,
        &mut styles,
        &mut extras,
    );
    c.bench_function("snapshot_visible_noop", |b| {
        b.iter(|| {
            crosswords.snapshot_visible(
                &TerminalDamage::Noop,
                COLS,
                &mut rows,
                &mut styles,
                &mut extras,
            )
        });
    });

    c.bench_function("row_rebuild_full_snapshot", |b| {
        b.iter(|| {
            crosswords.snapshot_visible(
                &TerminalDamage::Full,
                COLS,
                &mut rows,
                &mut styles,
                &mut extras,
            )
        });
    });

    // Worst-case interactive word extension: a full 4K-cell identifier.
    // This stays allocation-free and bounds work to the terminal grid.
    const SELECTION_COLUMNS: usize = 4096;
    let mut selection_terminal = Crosswords::new(
        CrosswordsSize::new(SELECTION_COLUMNS, 1),
        CursorShape::Block,
        VoidListener {},
        WindowId::from(0),
        0,
        0,
    );
    for column in 0..SELECTION_COLUMNS {
        selection_terminal.grid[Line(0)][Column(column)].set_c('x');
    }
    let selection_start = Anchor::new(Pos::new(Line(0), Column(0)), Side::Left);
    c.bench_function("keyboard_selection_word_motion_4k", |b| {
        b.iter(|| {
            std::hint::black_box(
                selection_terminal
                    .selection_motion_target(selection_start, SelectionMotion::WordRight),
            )
        })
    });

    // Adversarial but realistic retained history: finding the previous word
    // through 1,000 blank 120-column rows remains allocation-free.
    const HISTORY_COLUMNS: usize = 120;
    const HISTORY_SCREEN_ROWS: usize = 40;
    const HISTORY_ROWS: usize = 1_000;
    let mut history_terminal = Crosswords::new(
        CrosswordsSize::new(HISTORY_COLUMNS, HISTORY_SCREEN_ROWS),
        CursorShape::Block,
        VoidListener {},
        WindowId::from(0),
        0,
        HISTORY_ROWS,
    );
    history_terminal
        .grid
        .scroll_up(&(Line(0)..Line(HISTORY_SCREEN_ROWS as i32)), HISTORY_ROWS);
    let history_end = Anchor::new(
        Pos::new(
            Line((HISTORY_SCREEN_ROWS - 1) as i32),
            Column(HISTORY_COLUMNS - 1),
        ),
        Side::Right,
    );
    c.bench_function("keyboard_selection_word_motion_120k_scrollback", |b| {
        b.iter(|| {
            std::hint::black_box(
                history_terminal
                    .selection_motion_target(history_end, SelectionMotion::WordLeft),
            )
        })
    });
    // Prompt/path layout is VT reflow, not renderer state. Alternate between
    // a narrow and a wide grid so the benchmark includes both wrap and unwrap
    // of the same semantic prompt without shell or GPU noise.
    let prompt_path =
        "/workspace/équipe-🚀/platform/kubernetes/production/automexia-terminal";
    let prompt_stream = format!(
        "\x1b]1337;SetUserVar=automexia_prompt_active=MQ==\x07\
         \x1b]133;A;aid=701\x07 \r\n\
         \x1b]133;P;k=c;aid=701\x07{prompt_path}\r\n\
         \x1b]133;P;k=c;aid=701\x07λ \x1b]133;B\x07cargo test"
    );
    let mut prompt_terminal = Crosswords::new(
        CrosswordsSize::new(160, 24),
        CursorShape::Block,
        VoidListener {},
        WindowId::from(0),
        0,
        20_000,
    );
    let mut prompt_processor = Processor::default();
    prompt_processor.advance(&mut prompt_terminal, prompt_stream.as_bytes());
    let mut narrow = true;
    c.bench_function("prompt_layout_resize_reflow", |b| {
        b.iter(|| {
            let columns = if narrow { 24 } else { 240 };
            narrow = !narrow;
            prompt_terminal.resize(CrosswordsSize::new(columns, 40));
            std::hint::black_box(&prompt_terminal);
        })
    });

    // Shell history navigation is a clear-and-repaint workload. Keep a deep
    // scrollback behind the active semantic prompt so this benchmark catches
    // accidental O(scrollback) work in Up Arrow/Ctrl+R handling.
    let mut history_terminal = Crosswords::new(
        CrosswordsSize::new(COLS, ROWS),
        CursorShape::Block,
        VoidListener {},
        WindowId::from(0),
        0,
        20_000,
    );
    let mut history_processor = Processor::default();
    let history = "historical command output\r\n".repeat(15_000);
    history_processor.advance(&mut history_terminal, history.as_bytes());
    history_processor.advance(
        &mut history_terminal,
        b"\x1b]133;A;aid=501\x07 \r\n\x1b]133;P;k=c;aid=501\x07/workspace\r\n\x1b]133;P;k=c;aid=501\x07lambda \x1b]133;B\x07",
    );
    c.bench_function("history_navigation_repaint_deep_scrollback", |b| {
        b.iter(|| {
            history_processor.advance(
                &mut history_terminal,
                b"\r\x1b[2Klambda cargo test -p rio-vt",
            )
        });
    });

    // Command jumps are user-triggered and bounded by the configured
    // scrollback ring. Alternate between two distant OSC 133 prompt marks so
    // both directions exercise the worst retained-history scan without any
    // PTY, shell, renderer, or allocation work in the timed loop.
    let mut command_jump_terminal = Crosswords::new(
        CrosswordsSize::new(COLS, ROWS),
        CursorShape::Block,
        VoidListener {},
        WindowId::from(0),
        0,
        20_000,
    );
    let mut command_jump_processor = Processor::default();
    command_jump_processor.advance(
        &mut command_jump_terminal,
        b"\x1b]133;A;aid=1\x07> \x1b]133;B\x07old-command\r\n\x1b]133;C\x07",
    );
    let command_jump_history = "retained command output\r\n".repeat(15_000);
    command_jump_processor
        .advance(&mut command_jump_terminal, command_jump_history.as_bytes());
    command_jump_processor.advance(
        &mut command_jump_terminal,
        b"\x1b]133;D;0\x07\x1b]133;A;aid=2\x07> \x1b]133;B\x07current-command",
    );
    c.bench_function("command_prompt_jump_15000_rows", |b| {
        b.iter(|| {
            assert!(command_jump_terminal.scroll_to_prompt(false));
            assert!(command_jump_terminal.scroll_to_prompt(true));
            std::hint::black_box(command_jump_terminal.display_offset());
        })
    });
    // Completion decoration is fed by bounded OSC 133 row metadata. Measure
    // both the fully timed/status-bearing lifecycle and CMD's boundary-only
    // lifecycle so compatibility cannot make the PTY output path unbounded.
    const RESULT_COMMANDS: usize = 256;
    let verified_results = semantic_result_stream(RESULT_COMMANDS, true);
    let boundary_only_results = semantic_result_stream(RESULT_COMMANDS, false);
    let mut result_group = c.benchmark_group("command_result_lifecycle");
    result_group.sample_size(20);
    result_group.throughput(Throughput::Elements(RESULT_COMMANDS as u64));
    for (name, bytes) in [
        ("verified_status", &verified_results),
        ("boundary_only", &boundary_only_results),
    ] {
        result_group.bench_function(name, |b| {
            b.iter_batched(
                || (semantic_result_term(), Processor::default()),
                |(mut crosswords, mut processor)| {
                    processor.advance(&mut crosswords, bytes);
                    std::hint::black_box(crosswords)
                },
                criterion::BatchSize::SmallInput,
            )
        });
    }
    result_group.finish();

    let overflow_result = semantic_overflow_result_stream(512);
    let mut overflow_group = c.benchmark_group("command_result_viewport_overflow");
    overflow_group.sample_size(20);
    overflow_group.throughput(Throughput::Bytes(overflow_result.len() as u64));
    overflow_group.bench_function("512_output_rows", |b| {
        b.iter_batched(
            || (semantic_result_term(), Processor::default()),
            |(mut crosswords, mut processor)| {
                processor.advance(&mut crosswords, &overflow_result);
                std::hint::black_box(crosswords)
            },
            criterion::BatchSize::SmallInput,
        )
    });
    overflow_group.finish();

    // This interaction reproduces the state path behind completion badges
    // after a window resize: reflow retained command metadata, jump to a
    // neighbouring prompt in each direction, then publish the visible rows.
    // Keep the renderer out of the VT benchmark while still measuring every
    // terminal-owned operation that feeds its bounded projection.
    let mut resize_navigation_terminal = semantic_result_term();
    let mut resize_navigation_processor = Processor::default();
    resize_navigation_processor.advance(
        &mut resize_navigation_terminal,
        &semantic_result_stream(256, true),
    );
    let mut narrow = true;
    c.bench_function("command_result_resize_navigation_256", |b| {
        let mut visible_rows = Vec::new();
        let mut styles = Vec::new();
        let mut extras = rustc_hash::FxHashMap::default();
        b.iter(|| {
            let columns = if narrow { 23 } else { COLS };
            narrow = !narrow;
            resize_navigation_terminal.resize(CrosswordsSize::new(columns, ROWS));
            resize_navigation_terminal.scroll_display(Scroll::Bottom);
            assert!(resize_navigation_terminal.scroll_to_prompt(false));
            assert!(resize_navigation_terminal.scroll_to_prompt(true));
            resize_navigation_terminal.snapshot_visible(
                &TerminalDamage::Full,
                resize_navigation_terminal.columns(),
                &mut visible_rows,
                &mut styles,
                &mut extras,
            );
            std::hint::black_box(&visible_rows);
        })
    });

    // Measure reflow, retained selection serialization, scrolling and the
    // renderer-facing snapshot together. Setup is outside the timed path;
    // the history cap exceeds both layouts so eviction cannot hide work.
    let selected_text = "retained output with spaces and Unicode 界e\u{301}";
    for (label, selected, scrolled) in [
        ("unselected", false, false),
        ("selected", true, false),
        ("scrolled", false, true),
    ] {
        let mut terminal = Crosswords::new(
            CrosswordsSize::new(180, 40),
            CursorShape::Block,
            VoidListener {},
            WindowId::from(0),
            0,
            60_000,
        );
        let mut processor = Processor::default();
        let line = format!("{}\r\n", "bounded history ".repeat(9));
        for _ in 0..10_000 {
            processor.advance(&mut terminal, line.as_bytes());
        }
        let start = terminal.grid.cursor.pos;
        processor.advance(&mut terminal, selected_text.as_bytes());
        if selected {
            let mut selection = Selection::new(SelectionType::Simple, start, Side::Left);
            let end = Pos::new(
                terminal.grid.cursor.pos.row,
                terminal.grid.cursor.pos.col - 1,
            );
            selection.update(end, Side::Right);
            terminal.selection = Some(selection);
        }
        let mut narrow = true;
        if scrolled {
            terminal.scroll_display(Scroll::Delta(5_000));
        }
        let mut visible_rows = Vec::new();
        let mut styles = Vec::new();
        let mut extras = rustc_hash::FxHashMap::default();
        c.bench_function(
            &format!("selection_resize_copy_snapshot_10k_{label}"),
            |b| {
                b.iter(|| {
                    terminal
                        .resize(CrosswordsSize::new(if narrow { 40 } else { 180 }, 40));
                    narrow = !narrow;
                    if !scrolled {
                        terminal.scroll_display(Scroll::Top);
                        terminal.scroll_display(Scroll::Bottom);
                    }
                    assert_eq!(
                        terminal.selection_to_string().as_deref(),
                        selected.then_some(selected_text)
                    );
                    terminal.snapshot_visible(
                        &TerminalDamage::Full,
                        terminal.columns(),
                        &mut visible_rows,
                        &mut styles,
                        &mut extras,
                    );
                    std::hint::black_box(&visible_rows);
                });
            },
        );
    }
}

criterion_group!(benches, bench);
criterion_main!(benches);
