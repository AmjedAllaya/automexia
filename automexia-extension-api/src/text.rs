//! Unicode-safe text normalization shared by contract producers and consumers.
//!
//! Extension producers must keep labels within contract and presentation bounds.
//! They must not depend on a renderer or UI crate to do so. These helpers operate
//! on grapheme clusters so truncation never splits a visible character.

use unicode_segmentation::UnicodeSegmentation;

/// Trim `value` and retain at most `max_graphemes` visible grapheme clusters.
///
/// When truncation is required, the final cluster is an ellipsis. A zero limit
/// returns an empty string.
pub fn compact_label(value: &str, max_graphemes: usize) -> String {
    let value = value.trim();
    let graphemes = value.graphemes(true).collect::<Vec<_>>();
    if graphemes.len() <= max_graphemes {
        return value.to_string();
    }
    if max_graphemes == 0 {
        return String::new();
    }
    let keep = max_graphemes.saturating_sub(1);
    format!("{}…", graphemes[..keep].concat())
}

/// Trim `value` and compact its middle to at most `max_graphemes` clusters.
pub fn compact_middle(value: &str, max_graphemes: usize) -> String {
    let value = value.trim();
    let graphemes = value.graphemes(true).collect::<Vec<_>>();
    if graphemes.len() <= max_graphemes {
        return value.to_string();
    }
    if max_graphemes < 5 {
        return compact_label(value, max_graphemes);
    }
    let left = (max_graphemes - 1) / 2;
    let right = max_graphemes - left - 1;
    format!(
        "{}…{}",
        graphemes[..left].concat(),
        graphemes[graphemes.len() - right..].concat()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn label_compaction_trims_and_honors_all_limits() {
        assert_eq!(compact_label("  context  ", 7), "context");
        assert_eq!(compact_label("context", 7), "context");
        assert_eq!(compact_label("context", 4), "con…");
        assert_eq!(compact_label("context", 1), "…");
        assert_eq!(compact_label("context", 0), "");
    }

    #[test]
    fn compaction_never_splits_unicode_graphemes() {
        assert_eq!(compact_label("a\u{301}bcdef", 4), "a\u{301}bc…");
        assert_eq!(
            compact_middle("ab\u{1f468}\u{200d}\u{1f4bb}cdef", 5),
            "ab…ef"
        );
    }

    #[test]
    fn middle_compaction_uses_readable_fallback_for_tiny_limits() {
        assert_eq!(compact_middle("abcdefgh", 6), "ab…fgh");
        assert_eq!(compact_middle("abcdefgh", 4), "abc…");
        assert_eq!(compact_middle("abcdefgh", 0), "");
    }
}
