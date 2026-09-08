//! Unicode-safe text normalization shared by contract producers and consumers.
//!
//! Extension producers must keep labels within contract and presentation bounds.
//! They must not depend on a renderer or UI crate to do so. These helpers operate
//! on grapheme clusters so truncation never splits a visible character.

use unicode_segmentation::UnicodeSegmentation;

/// Split off at most `max_graphemes` extended grapheme clusters without copying.
///
/// Whitespace is preserved. Both slices borrow `value`; concatenating them
/// recovers the original bytes. This limits clusters, not bytes, cells or pixels:
/// a single cluster may contain many code points. Callers own input byte limits.
pub fn split_grapheme_prefix(value: &str, max_graphemes: usize) -> (&str, &str) {
    // Every nonempty cluster occupies at least one byte. This sufficient
    // fit check avoids segmentation of short labels without assuming ASCII.
    if value.len() <= max_graphemes {
        return value.split_at(value.len());
    }
    let end = value
        .grapheme_indices(true)
        .take(max_graphemes)
        .last()
        .map_or(0, |(index, cluster)| index + cluster.len());
    value.split_at(end)
}

/// Trim `value` and retain at most `max_graphemes` visible grapheme clusters.
///
/// When truncation is required, the final cluster is an ellipsis. A zero limit
/// returns an empty string.
pub fn compact_label(value: &str, max_graphemes: usize) -> String {
    compact_label_preserving_whitespace(value.trim(), max_graphemes)
}

/// Preserve whitespace and compact the end to at most `max_graphemes` clusters.
///
/// The ellipsis counts toward the limit. This is a display transformation, not
/// validation or a replacement for the original command, target or full label.
pub fn compact_label_preserving_whitespace(value: &str, max_graphemes: usize) -> String {
    if max_graphemes == 0 {
        return String::new();
    }
    let (prefix, remainder) = split_grapheme_prefix(value, max_graphemes);
    if remainder.is_empty() {
        return value.to_string();
    }
    let last = prefix
        .grapheme_indices(true)
        .next_back()
        .map_or(0, |(index, _)| index);
    format!("{}…", &prefix[..last])
}

/// Trim `value` and compact its middle to at most `max_graphemes` clusters.
pub fn compact_middle(value: &str, max_graphemes: usize) -> String {
    let value = value.trim();
    if max_graphemes < 5 {
        return compact_label_preserving_whitespace(value, max_graphemes);
    }
    if split_grapheme_prefix(value, max_graphemes).1.is_empty() {
        return value.to_string();
    }
    let left = (max_graphemes - 1) / 2;
    let right = max_graphemes - left - 1;
    let prefix = split_grapheme_prefix(value, left).0;
    let suffix_start = value
        .grapheme_indices(true)
        .rev()
        .nth(right - 1)
        .map_or(0, |(index, _)| index);
    format!("{prefix}…{}", &value[suffix_start..])
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
