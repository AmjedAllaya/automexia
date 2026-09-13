// grid/tests.rs was originally taken from Alacritty
// https://github.com/alacritty/alacritty/blob/e35e5ad14fce8456afdd89f2b392b9924bb27471/alacritty_terminal/src/grid/tests.rs
// which is licensed under Apache 2.0 license.

use super::*;

use crate::crosswords::square::Square;

impl GridSquare for usize {
    fn is_empty(&self) -> bool {
        *self == 0
    }

    fn reset(&mut self, template: &Self) {
        *self = *template;
    }
}

// Scroll up moves lines upward.
#[test]
fn scroll_up() {
    let mut grid = Grid::<usize>::new(10, 1, 0);
    for i in 0..10 {
        grid[Line(i as i32)][Column(0)] = i;
    }

    grid.scroll_up(&(Line(0)..Line(10)), 2);

    assert_eq!(grid[Line(0)][Column(0)], 2);
    assert_eq!(grid[Line(0)].occ, 1);
    assert_eq!(grid[Line(1)][Column(0)], 3);
    assert_eq!(grid[Line(1)].occ, 1);
    assert_eq!(grid[Line(2)][Column(0)], 4);
    assert_eq!(grid[Line(2)].occ, 1);
    assert_eq!(grid[Line(3)][Column(0)], 5);
    assert_eq!(grid[Line(3)].occ, 1);
    assert_eq!(grid[Line(4)][Column(0)], 6);
    assert_eq!(grid[Line(4)].occ, 1);
    assert_eq!(grid[Line(5)][Column(0)], 7);
    assert_eq!(grid[Line(5)].occ, 1);
    assert_eq!(grid[Line(6)][Column(0)], 8);
    assert_eq!(grid[Line(6)].occ, 1);
    assert_eq!(grid[Line(7)][Column(0)], 9);
    assert_eq!(grid[Line(7)].occ, 1);
    assert_eq!(grid[Line(8)][Column(0)], 0); // was 0.
    assert_eq!(grid[Line(8)].occ, 0);
    assert_eq!(grid[Line(9)][Column(0)], 0); // was 1.
    assert_eq!(grid[Line(9)].occ, 0);
}

// Scroll down moves lines downward.
#[test]
fn scroll_down() {
    let mut grid = Grid::<usize>::new(10, 1, 0);
    for i in 0..10 {
        grid[Line(i as i32)][Column(0)] = i;
    }

    grid.scroll_down(&(Line(0)..Line(10)), 2);

    assert_eq!(grid[Line(0)][Column(0)], 0); // was 8.
    assert_eq!(grid[Line(0)].occ, 0);
    assert_eq!(grid[Line(1)][Column(0)], 0); // was 9.
    assert_eq!(grid[Line(1)].occ, 0);
    assert_eq!(grid[Line(2)][Column(0)], 0);
    assert_eq!(grid[Line(2)].occ, 1);
    assert_eq!(grid[Line(3)][Column(0)], 1);
    assert_eq!(grid[Line(3)].occ, 1);
    assert_eq!(grid[Line(4)][Column(0)], 2);
    assert_eq!(grid[Line(4)].occ, 1);
    assert_eq!(grid[Line(5)][Column(0)], 3);
    assert_eq!(grid[Line(5)].occ, 1);
    assert_eq!(grid[Line(6)][Column(0)], 4);
    assert_eq!(grid[Line(6)].occ, 1);
    assert_eq!(grid[Line(7)][Column(0)], 5);
    assert_eq!(grid[Line(7)].occ, 1);
    assert_eq!(grid[Line(8)][Column(0)], 6);
    assert_eq!(grid[Line(8)].occ, 1);
    assert_eq!(grid[Line(9)][Column(0)], 7);
    assert_eq!(grid[Line(9)].occ, 1);
}

#[test]
fn scroll_down_with_history() {
    let mut grid = Grid::<usize>::new(10, 1, 1);
    grid.increase_scroll_limit(1);
    for i in 0..10 {
        grid[Line(i as i32)][Column(0)] = i;
    }

    grid.scroll_down(&(Line(0)..Line(10)), 2);

    assert_eq!(grid[Line(0)][Column(0)], 0); // was 8.
    assert_eq!(grid[Line(0)].occ, 0);
    assert_eq!(grid[Line(1)][Column(0)], 0); // was 9.
    assert_eq!(grid[Line(1)].occ, 0);
    assert_eq!(grid[Line(2)][Column(0)], 0);
    assert_eq!(grid[Line(2)].occ, 1);
    assert_eq!(grid[Line(3)][Column(0)], 1);
    assert_eq!(grid[Line(3)].occ, 1);
    assert_eq!(grid[Line(4)][Column(0)], 2);
    assert_eq!(grid[Line(4)].occ, 1);
    assert_eq!(grid[Line(5)][Column(0)], 3);
    assert_eq!(grid[Line(5)].occ, 1);
    assert_eq!(grid[Line(6)][Column(0)], 4);
    assert_eq!(grid[Line(6)].occ, 1);
    assert_eq!(grid[Line(7)][Column(0)], 5);
    assert_eq!(grid[Line(7)].occ, 1);
    assert_eq!(grid[Line(8)][Column(0)], 6);
    assert_eq!(grid[Line(8)].occ, 1);
    assert_eq!(grid[Line(9)][Column(0)], 7);
    assert_eq!(grid[Line(9)].occ, 1);
}

// Test that GridIterator works.
#[test]
fn test_iter() {
    let assert_indexed = |value: usize, indexed: Option<Indexed<&usize>>| {
        assert_eq!(Some(&value), indexed.map(|indexed| indexed.square));
    };

    let mut grid = Grid::<usize>::new(5, 5, 0);
    for i in 0..5 {
        for j in 0..5 {
            grid[Line(i)][Column(j)] = i as usize * 5 + j;
        }
    }

    let mut iter = grid.iter_from(Pos::new(Line(0), Column(0)));

    assert_eq!(None, iter.prev());
    assert_indexed(1, iter.next());
    assert_eq!(Column(1), iter.pos().col);
    assert_eq!(0, iter.pos().row);

    assert_indexed(2, iter.next());
    assert_indexed(3, iter.next());
    assert_indexed(4, iter.next());

    // Test line-wrapping.
    assert_indexed(5, iter.next());
    assert_eq!(Column(0), iter.pos().col);
    assert_eq!(1, iter.pos().row);

    assert_indexed(4, iter.prev());
    assert_eq!(Column(4), iter.pos().col);
    assert_eq!(0, iter.pos().row);

    // Make sure iter.cell() returns the current iterator position.
    assert_eq!(&4, iter.square());

    // Test that iter ends at end of grid.
    let mut final_iter = grid.iter_from(Pos {
        row: Line(4),
        col: Column(4),
    });
    assert_eq!(None, final_iter.next());
    assert_indexed(23, final_iter.prev());
}

#[test]
fn shrink_reflow() {
    let mut grid = Grid::<Square>::new(1, 5, 2);
    grid[Line(0)][Column(0)] = cell('1');
    grid[Line(0)][Column(1)] = cell('2');
    grid[Line(0)][Column(2)] = cell('3');
    grid[Line(0)][Column(3)] = cell('4');
    grid[Line(0)][Column(4)] = cell('5');

    grid.resize(true, 1, 2);

    assert_eq!(grid.total_lines(), 3);

    assert_eq!(grid[Line(-2)].len(), 2);
    assert_eq!(grid[Line(-2)][Column(0)], cell('1'));
    assert_eq!(grid[Line(-2)][Column(1)], wrap_cell('2'));

    assert_eq!(grid[Line(-1)].len(), 2);
    assert_eq!(grid[Line(-1)][Column(0)], cell('3'));
    assert_eq!(grid[Line(-1)][Column(1)], wrap_cell('4'));

    assert_eq!(grid[Line(0)].len(), 2);
    assert_eq!(grid[Line(0)][Column(0)], cell('5'));
    assert_eq!(grid[Line(0)][Column(1)], Square::default());
}

#[test]
fn shrink_reflow_twice() {
    let mut grid = Grid::<Square>::new(1, 5, 2);
    grid[Line(0)][Column(0)] = cell('1');
    grid[Line(0)][Column(1)] = cell('2');
    grid[Line(0)][Column(2)] = cell('3');
    grid[Line(0)][Column(3)] = cell('4');
    grid[Line(0)][Column(4)] = cell('5');

    grid.resize(true, 1, 4);
    grid.resize(true, 1, 2);

    assert_eq!(grid.total_lines(), 3);

    assert_eq!(grid[Line(-2)].len(), 2);
    assert_eq!(grid[Line(-2)][Column(0)], cell('1'));
    assert_eq!(grid[Line(-2)][Column(1)], wrap_cell('2'));

    assert_eq!(grid[Line(-1)].len(), 2);
    assert_eq!(grid[Line(-1)][Column(0)], cell('3'));
    assert_eq!(grid[Line(-1)][Column(1)], wrap_cell('4'));

    assert_eq!(grid[Line(0)].len(), 2);
    assert_eq!(grid[Line(0)][Column(0)], cell('5'));
    assert_eq!(grid[Line(0)][Column(1)], Square::default());
}

#[test]
fn shrink_reflow_empty_cell_inside_line() {
    let mut grid = Grid::<Square>::new(1, 5, 3);
    grid[Line(0)][Column(0)] = cell('1');
    grid[Line(0)][Column(1)] = Square::default();
    grid[Line(0)][Column(2)] = cell('3');
    grid[Line(0)][Column(3)] = cell('4');
    grid[Line(0)][Column(4)] = Square::default();

    grid.resize(true, 1, 2);

    assert_eq!(grid.total_lines(), 2);

    assert_eq!(grid[Line(-1)].len(), 2);
    assert_eq!(grid[Line(-1)][Column(0)], cell('1'));
    assert_eq!(grid[Line(-1)][Column(1)], wrap_cell('\0'));

    assert_eq!(grid[Line(0)].len(), 2);
    assert_eq!(grid[Line(0)][Column(0)], cell('3'));
    assert_eq!(grid[Line(0)][Column(1)], cell('4'));

    grid.resize(true, 1, 1);

    assert_eq!(grid.total_lines(), 4);

    assert_eq!(grid[Line(-3)].len(), 1);
    assert_eq!(grid[Line(-3)][Column(0)], wrap_cell('1'));

    assert_eq!(grid[Line(-2)].len(), 1);
    assert_eq!(grid[Line(-2)][Column(0)], wrap_cell('\0'));

    assert_eq!(grid[Line(-1)].len(), 1);
    assert_eq!(grid[Line(-1)][Column(0)], wrap_cell('3'));

    assert_eq!(grid[Line(0)].len(), 1);
    assert_eq!(grid[Line(0)][Column(0)], cell('4'));
}

#[test]
fn grow_reflow() {
    let mut grid = Grid::<Square>::new(2, 2, 0);
    grid[Line(0)][Column(0)] = cell('1');
    grid[Line(0)][Column(1)] = wrap_cell('2');
    grid[Line(1)][Column(0)] = cell('3');
    grid[Line(1)][Column(1)] = Square::default();

    grid.resize(true, 2, 3);

    assert_eq!(grid.total_lines(), 2);

    assert_eq!(grid[Line(0)].len(), 3);
    assert_eq!(grid[Line(0)][Column(0)], cell('1'));
    assert_eq!(grid[Line(0)][Column(1)], cell('2'));
    assert_eq!(grid[Line(0)][Column(2)], cell('3'));

    // Make sure rest of grid is empty.
    assert_eq!(grid[Line(1)].len(), 3);
    assert_eq!(grid[Line(1)][Column(0)], Square::default());
    assert_eq!(grid[Line(1)][Column(1)], Square::default());
    assert_eq!(grid[Line(1)][Column(2)], Square::default());
}

#[test]
fn grow_reflow_multiline() {
    let mut grid = Grid::<Square>::new(3, 2, 0);
    grid[Line(0)][Column(0)] = cell('1');
    grid[Line(0)][Column(1)] = wrap_cell('2');
    grid[Line(1)][Column(0)] = cell('3');
    grid[Line(1)][Column(1)] = wrap_cell('4');
    grid[Line(2)][Column(0)] = cell('5');
    grid[Line(2)][Column(1)] = cell('6');

    grid.resize(true, 3, 6);

    assert_eq!(grid.total_lines(), 3);

    assert_eq!(grid[Line(0)].len(), 6);
    assert_eq!(grid[Line(0)][Column(0)], cell('1'));
    assert_eq!(grid[Line(0)][Column(1)], cell('2'));
    assert_eq!(grid[Line(0)][Column(2)], cell('3'));
    assert_eq!(grid[Line(0)][Column(3)], cell('4'));
    assert_eq!(grid[Line(0)][Column(4)], cell('5'));
    assert_eq!(grid[Line(0)][Column(5)], cell('6'));

    // Make sure rest of grid is empty.
    for r in (1..3).map(Line::from) {
        assert_eq!(grid[r].len(), 6);
        for c in 0..6 {
            assert_eq!(grid[r][Column(c)], Square::default());
        }
    }
}

#[test]
fn semantic_prompt_rows_survive_shrink_and_grow_reflow() {
    use crate::crosswords::grid::row::{
        SemanticCommandResult, SemanticCommandTimestamp, SemanticPrompt,
    };

    let completed_at = Some(SemanticCommandTimestamp {
        unix_ms: 1_777_575_942_000,
        year: 2026,
        month: 8,
        day: 26,
        hour: 19,
        minute: 5,
        second: 42,
    });

    let mut grid = Grid::<Square>::new(3, 8, 8);
    grid[Line(0)].set_semantic_prompt(SemanticPrompt::Prompt, Some(42));
    grid[Line(0)].set_semantic_command_result(SemanticCommandResult {
        id: 77,
        exit_code: Some(17),
        elapsed_ms: Some(1_234),
        completed_at,
    });
    grid[Line(1)].set_semantic_prompt(SemanticPrompt::PromptContinuation, Some(42));
    for (column, character) in "12345678".chars().enumerate() {
        grid[Line(1)][Column(column)] = cell(character);
    }

    grid.resize(true, 3, 4);
    let rows_after_shrink = grid.raw.rows().collect::<Vec<_>>();
    assert!(rows_after_shrink
        .iter()
        .any(|row| { row.semantic_prompt == SemanticPrompt::Prompt && row.is_clear() }));
    assert!(rows_after_shrink.iter().any(|row| {
        row.semantic_prompt == SemanticPrompt::Prompt
            && row.semantic_prompt_id == Some(42)
            && row.semantic_command_result
                == Some(SemanticCommandResult {
                    id: 77,
                    exit_code: Some(17),
                    elapsed_ms: Some(1_234),
                    completed_at,
                })
    }));
    assert!(rows_after_shrink
        .iter()
        .filter(|row| !row.is_clear())
        .all(|row| {
            row.semantic_prompt == SemanticPrompt::PromptContinuation
                && row.semantic_prompt_id == Some(42)
        }));

    grid.resize(true, 3, 8);
    let rows_after_grow = grid.raw.rows().collect::<Vec<_>>();
    assert!(rows_after_grow
        .iter()
        .any(|row| { row.semantic_prompt == SemanticPrompt::Prompt && row.is_clear() }));
    assert!(rows_after_grow.iter().any(|row| {
        row.semantic_prompt == SemanticPrompt::Prompt
            && row.semantic_prompt_id == Some(42)
            && row.semantic_command_result
                == Some(SemanticCommandResult {
                    id: 77,
                    exit_code: Some(17),
                    elapsed_ms: Some(1_234),
                    completed_at,
                })
    }));
    assert!(rows_after_grow
        .iter()
        .filter(|row| !row.is_clear())
        .all(|row| {
            row.semantic_prompt == SemanticPrompt::PromptContinuation
                && row.semantic_prompt_id == Some(42)
        }));
}

#[test]
fn semantic_command_boundary_survives_shrink_and_grow_reflow() {
    use crate::crosswords::grid::row::{
        SemanticCommandBoundary, SemanticCommandResult, SemanticCommandTimestamp,
        SemanticPrompt,
    };

    let result = SemanticCommandResult {
        id: 91,
        exit_code: Some(7),
        elapsed_ms: Some(2_500),
        completed_at: Some(SemanticCommandTimestamp {
            unix_ms: 1_777_575_942_000,
            year: 2026,
            month: 8,
            day: 26,
            hour: 19,
            minute: 5,
            second: 42,
        }),
    };
    let boundary = SemanticCommandBoundary {
        source_prompt_id: Some(41),
        result,
    };
    let mut grid = Grid::<Square>::new(3, 8, 8);
    grid[Line(0)].set_semantic_prompt(SemanticPrompt::Prompt, Some(41));
    grid[Line(0)].set_semantic_command_result(result);
    for (column, character) in "output!!".chars().enumerate() {
        grid[Line(1)][Column(column)] = cell(character);
    }
    grid[Line(2)].set_semantic_prompt(SemanticPrompt::Prompt, Some(42));
    grid[Line(2)].set_semantic_command_boundary(boundary);

    for columns in [4usize, 8] {
        grid.resize(true, 3, columns);
        let boundary_rows = grid
            .raw
            .rows()
            .filter(|row| row.semantic_command_boundary.is_some())
            .collect::<Vec<_>>();
        assert_eq!(boundary_rows.len(), 1, "columns={columns}");
        assert_eq!(boundary_rows[0].semantic_prompt, SemanticPrompt::Prompt);
        assert_eq!(boundary_rows[0].semantic_prompt_id, Some(42));
        assert_eq!(boundary_rows[0].semantic_command_boundary, Some(boundary));
    }
}

#[test]
fn multiple_three_row_prompts_keep_order_and_text_through_reflow() {
    use crate::crosswords::grid::row::SemanticPrompt;

    fn write_row(grid: &mut Grid<Square>, line: i32, text: &str) {
        for (column, character) in text.chars().enumerate() {
            grid[Line(line)][Column(column)] = cell(character);
        }
    }

    fn meaningful_rows(
        grid: &Grid<Square>,
    ) -> Vec<(SemanticPrompt, Option<u64>, String)> {
        (0..grid.screen_lines())
            .filter_map(|line| {
                let row = &grid[Line(line as i32)];
                let text = row
                    .inner
                    .iter()
                    .map(|square| square.c())
                    .collect::<String>()
                    .trim_end_matches(['\0', ' '])
                    .to_string();
                (row.semantic_prompt != SemanticPrompt::None || !text.is_empty())
                    .then_some((row.semantic_prompt, row.semantic_prompt_id, text))
            })
            .collect()
    }

    let mut grid = Grid::<Square>::new(12, 52, 100);
    grid[Line(0)].set_semantic_prompt(SemanticPrompt::Prompt, Some(1));
    grid[Line(1)].set_semantic_prompt(SemanticPrompt::PromptContinuation, Some(1));
    write_row(&mut grid, 1, "/workspace/automexia/standalone");
    grid[Line(2)].set_semantic_prompt(SemanticPrompt::PromptContinuation, Some(1));
    write_row(&mut grid, 2, "λ pwd");
    write_row(&mut grid, 3, "/workspace/automexia/standalone");
    grid[Line(4)].set_semantic_prompt(SemanticPrompt::Prompt, Some(2));
    grid[Line(5)].set_semantic_prompt(SemanticPrompt::PromptContinuation, Some(2));
    write_row(&mut grid, 5, "/workspace/automexia/standalone");
    grid[Line(6)].set_semantic_prompt(SemanticPrompt::PromptContinuation, Some(2));
    write_row(&mut grid, 6, "λ echo ok");
    write_row(&mut grid, 7, "ok");
    grid[Line(8)].set_semantic_prompt(SemanticPrompt::Prompt, Some(3));
    grid[Line(9)].set_semantic_prompt(SemanticPrompt::PromptContinuation, Some(3));
    write_row(&mut grid, 9, "/workspace/automexia/standalone");
    grid[Line(10)].set_semantic_prompt(SemanticPrompt::PromptContinuation, Some(3));
    write_row(&mut grid, 10, "λ ");
    grid.cursor.pos = Pos::new(Line(10), Column(2));

    let before = meaningful_rows(&grid);
    grid.resize(true, 9, 24);
    grid.resize(true, 12, 52);
    let after = meaningful_rows(&grid);

    assert_eq!(after, before);
    for generation in 1..=3 {
        let prompt = after
            .iter()
            .position(|(kind, id, _)| {
                *kind == SemanticPrompt::Prompt && *id == Some(generation)
            })
            .expect("semantic prompt row was lost");
        assert_eq!(
            after.get(prompt + 1).map(|row| row.0),
            Some(SemanticPrompt::PromptContinuation),
            "prompt {generation} detached from its path row"
        );
        assert_eq!(after[prompt + 1].1, Some(generation));
        assert_eq!(after[prompt + 2].0, SemanticPrompt::PromptContinuation);
        assert_eq!(after[prompt + 2].1, Some(generation));
    }
}

#[test]
fn active_three_row_prompt_restores_its_complete_path_after_extreme_reflow() {
    use crate::crosswords::grid::row::SemanticPrompt;

    let path = "<REDACTED_LOCAL_VALUE>";
    let mut grid = Grid::<Square>::new(3, 120, 512);
    grid[Line(0)].set_semantic_prompt(SemanticPrompt::Prompt, Some(77));
    grid[Line(1)].set_semantic_prompt(SemanticPrompt::PromptContinuation, Some(77));
    for (column, character) in path.chars().enumerate() {
        grid[Line(1)][Column(column)] = cell(character);
    }
    grid[Line(2)].set_semantic_prompt(SemanticPrompt::PromptContinuation, Some(77));
    grid[Line(2)][Column(0)] = cell('λ');
    grid.cursor.pos = Pos::new(Line(2), Column(2));

    for _ in 0..12 {
        grid.resize(true, 3, 20);
        grid.resize(true, 16, 84);
        grid.resize(true, 32, 180);
        grid.resize(true, 64, 360);
    }
    grid.resize(true, 30, 120);

    let visible_text = (0..grid.screen_lines())
        .map(|line| {
            grid[Line(line as i32)]
                .inner
                .iter()
                .map(|square| square.c())
                .collect::<String>()
                .trim_end_matches(['\0', ' '])
                .to_string()
        })
        .collect::<Vec<_>>();
    assert!(
        visible_text.iter().any(|row| row == path),
        "complete active path must return to the visible viewport: {visible_text:?}"
    );
}

#[test]
fn grow_reflow_disabled() {
    let mut grid = Grid::<Square>::new(2, 2, 0);
    grid[Line(0)][Column(0)] = cell('1');
    grid[Line(0)][Column(1)] = wrap_cell('2');
    grid[Line(1)][Column(0)] = cell('3');
    grid[Line(1)][Column(1)] = Square::default();

    grid.resize(false, 2, 3);

    assert_eq!(grid.total_lines(), 2);

    assert_eq!(grid[Line(0)].len(), 3);
    assert_eq!(grid[Line(0)][Column(0)], cell('1'));
    assert_eq!(grid[Line(0)][Column(1)], wrap_cell('2'));
    assert_eq!(grid[Line(0)][Column(2)], Square::default());

    assert_eq!(grid[Line(1)].len(), 3);
    assert_eq!(grid[Line(1)][Column(0)], cell('3'));
    assert_eq!(grid[Line(1)][Column(1)], Square::default());
    assert_eq!(grid[Line(1)][Column(2)], Square::default());
}

#[test]
fn shrink_reflow_disabled() {
    let mut grid = Grid::<Square>::new(1, 5, 2);
    grid[Line(0)][Column(0)] = cell('1');
    grid[Line(0)][Column(1)] = cell('2');
    grid[Line(0)][Column(2)] = cell('3');
    grid[Line(0)][Column(3)] = cell('4');
    grid[Line(0)][Column(4)] = cell('5');

    grid.resize(false, 1, 2);

    assert_eq!(grid.total_lines(), 1);

    assert_eq!(grid[Line(0)].len(), 2);
    assert_eq!(grid[Line(0)][Column(0)], cell('1'));
    assert_eq!(grid[Line(0)][Column(1)], cell('2'));
}

// https://github.com/rust-lang/rust-clippy/pull/6375
#[allow(clippy::all)]
fn cell(c: char) -> Square {
    let mut cell = Square::default();
    cell.set_c(c);
    cell
}

fn wrap_cell(c: char) -> Square {
    let mut cell = cell(c);
    cell.set_wrapline(true);
    cell
}

fn wide_cell(c: char) -> Square {
    let mut cell = cell(c);
    cell.set_wide(crate::crosswords::square::Wide::Wide);
    cell
}

fn spacer_cell() -> Square {
    let mut cell = Square::default();
    cell.set_wide(crate::crosswords::square::Wide::Spacer);
    cell
}

#[test]
fn shrink_reflow_remap_tracks_displaced_wide_char() {
    // A wrapped tail of exactly `columns - 1` cells is buffered into
    // the next row, whose first cell is a wide char. The spacer logic
    // displaces that wide char into the following push, so the remap
    // must record the row after the one receiving the buffered tail.
    let mut grid = Grid::<Square>::new(3, 8, 4);
    for (n, c) in "1234567".chars().enumerate() {
        grid[Line(0)][Column(n)] = cell(c);
    }
    grid[Line(0)][Column(6)] = wrap_cell('7');
    grid[Line(1)][Column(0)] = wide_cell('W');
    grid[Line(1)][Column(1)] = spacer_cell();
    grid[Line(1)][Column(2)] = cell('x');

    grid.track_reflow_remap = true;
    grid.resize(true, 3, 4);
    grid.track_reflow_remap = false;

    let remap = grid.reflow_remap.take().expect("remap must be recorded");
    assert_eq!(remap.base_abs, 0);
    // Old row 0 lands at 0; the wide-char row's first cell lands at 2
    // (position 1 holds the buffered "567" tail plus the spacer); the
    // trailing blank row lands at 3.
    assert_eq!(remap.new_pos, vec![0, 2, 3]);

    // Cross-check against where the wide char actually sits.
    let total = grid.total_lines() as i32;
    let screen = grid.screen_lines() as i32;
    let wide_line = Line(2 - (total - screen));
    assert_eq!(grid[wide_line][Column(0)], wide_cell('W'));
}

#[test]
fn grow_reflow_remap_tracks_unmerged_wide_char() {
    // The merge target has exactly one free column, so only a leading
    // spacer is appended and the wide char stays on its own row. The
    // remap must record the pushed remainder row, not the merge
    // target.
    let mut grid = Grid::<Square>::new(2, 4, 2);
    for (n, c) in "1234".chars().enumerate() {
        grid[Line(0)][Column(n)] = cell(c);
    }
    grid[Line(0)][Column(3)] = wrap_cell('4');
    grid[Line(1)][Column(0)] = wide_cell('W');
    grid[Line(1)][Column(1)] = spacer_cell();

    grid.track_reflow_remap = true;
    grid.resize(true, 2, 5);
    grid.track_reflow_remap = false;

    let remap = grid.reflow_remap.take().expect("remap must be recorded");
    assert_eq!(remap.base_abs, 0);
    assert_eq!(remap.new_pos, vec![0, 1]);
    assert_eq!(grid[Line(1)][Column(0)], wide_cell('W'));
}
