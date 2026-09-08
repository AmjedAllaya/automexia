//! Private draw-only suggestion labels; layout and activation stay with callers.

use rio_backend::sugarloaf::text::{DrawOpts, Text};
use unicode_segmentation::UnicodeSegmentation;

use super::text_fit::{fit_end, FittedText};

pub(super) fn draw_text(
    renderer: &mut Text,
    (x, y): (f32, f32),
    value: &str,
    maximum_width: f32,
    options: &DrawOpts,
) {
    let display = fit_plain(renderer, value, maximum_width, options);
    renderer.draw(x, y, &display.display, options);
}

pub(super) fn draw_matched_text(
    renderer: &mut Text,
    (x, y): (f32, f32),
    value: &str,
    matched_graphemes: &[usize],
    maximum_width: f32,
    normal: &DrawOpts,
    highlighted: &DrawOpts,
) {
    let display = fit_end(value, maximum_width, "...", |candidate, retained| {
        fitted_runs(candidate, retained, matched_graphemes)
            .map(|(matched, run)| {
                renderer.measure(run, if matched { highlighted } else { normal })
            })
            .sum()
    });
    let mut current_x = x;
    for (matched, run) in
        fitted_runs(&display.display, display.retained, matched_graphemes)
    {
        current_x += renderer.draw(
            current_x,
            y,
            run,
            if matched { highlighted } else { normal },
        );
    }
}

fn fit_plain<'a>(
    renderer: &mut Text,
    text: &'a str,
    maximum_width: f32,
    options: &DrawOpts,
) -> FittedText<'a> {
    fit_end(text, maximum_width, "...", |candidate, _| {
        renderer.measure(candidate, options)
    })
}

fn matched_grapheme_runs<'a>(
    text: &'a str,
    matched: &'a [usize],
) -> impl Iterator<Item = (bool, &'a str)> {
    let mut graphemes = text.grapheme_indices(true).enumerate().peekable();
    std::iter::from_fn(move || {
        let (index, (start, _)) = graphemes.next()?;
        let is_matched = matched.binary_search(&index).is_ok();
        while let Some(&(index, _)) = graphemes.peek() {
            if matched.binary_search(&index).is_ok() != is_matched {
                break;
            }
            graphemes.next();
        }
        let end = graphemes
            .peek()
            .map(|(_, (offset, _))| *offset)
            .unwrap_or(text.len());
        Some((is_matched, &text[start..end]))
    })
}

fn fitted_runs<'a>(
    display: &'a str,
    retained: Option<usize>,
    matched: &'a [usize],
) -> impl Iterator<Item = (bool, &'a str)> {
    // A retained count identifies an appended marker, including when a Unicode
    // Prepend character joins the first dot in the display's segmentation.
    // Literal source dots have no generated-marker provenance.
    let (source, mut marker) = if retained.is_some() {
        display
            .strip_suffix("...")
            .map_or(("", None), |source| (source, Some("...")))
    } else {
        (display, None)
    };
    let mut runs = matched_grapheme_runs(source, matched).peekable();
    std::iter::from_fn(move || {
        if let Some((matched, run)) = runs.next() {
            if !matched && marker.is_some() && runs.peek().is_none() {
                marker = None;
                // Keep adjacent neutral source and marker in one shaped run.
                return Some((false, &display[source.len() - run.len()..]));
            }
            return Some((matched, run));
        }
        marker.take().map(|marker| (false, marker))
    })
}

#[cfg(test)]
#[path = "suggestion_text_tests.rs"]
mod tests;
