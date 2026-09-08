use automexia_extension_api::{
    compact_label, compact_label_preserving_whitespace, compact_middle,
    split_grapheme_prefix,
};

#[test]
fn compaction_matches_independently_segmented_cluster_fixtures() {
    // Literal clusters are the oracle, not another call to the segmentation helper.
    let fixtures: &[&[&str]] = &[
        &[],
        &["a"],
        &["a", "b", "c", "d", "e", "f", "g", "h"],
        &["a\u{301}", "👩🏽\u{200d}💻", "中", "🇫🇷", "ב", "z", "🙂"],
        &["क\u{94d}ष", "e\u{308}", "ש", "文", "x", "y"],
    ];
    for clusters in fixtures {
        let value = clusters.concat();
        for limit in (0..=clusters.len() + 2).chain([usize::MAX]) {
            let expected_end = if clusters.len() <= limit {
                value.clone()
            } else if limit == 0 {
                String::new()
            } else {
                format!("{}…", clusters[..limit - 1].concat())
            };
            let expected_middle = if clusters.len() <= limit || limit < 5 {
                expected_end.clone()
            } else {
                let left = (limit - 1) / 2;
                let right = limit - left - 1;
                format!(
                    "{}…{}",
                    clusters[..left].concat(),
                    clusters[clusters.len() - right..].concat()
                )
            };
            assert_eq!(compact_label(&value, limit), expected_end);
            assert_eq!(compact_middle(&value, limit), expected_middle);
            let padded = format!("\t\u{2003}{value}\u{a0}\r\n");
            assert_eq!(compact_label(&padded, limit), expected_end);
            assert_eq!(compact_middle(&padded, limit), expected_middle);
        }
    }
}

#[test]
fn long_compaction_preserves_exact_endpoints_and_does_not_modify_input() {
    let value = format!("start-{}-end", "e\u{301}👩🏽\u{200d}💻中".repeat(4096));
    let before = value.clone();
    assert_eq!(compact_label(&value, 6), "start…");
    assert_eq!(compact_middle(&value, 10), "star…中-end");
    assert_eq!(value, before);
    assert_eq!(compact_label(" \t\r\n", usize::MAX), "");
}

#[test]
fn borrowed_prefixes_preserve_whitespace_clusters_and_original_storage() {
    let clusters = [" ", "e\u{301}", "👩🏽\u{200d}💻", "\r\n", "中", " "];
    let value = clusters.concat();
    for limit in (0..=clusters.len() + 2).chain([usize::MAX]) {
        let keep = limit.min(clusters.len());
        let (prefix, remainder) = split_grapheme_prefix(&value, limit);
        assert_eq!(prefix, clusters[..keep].concat());
        assert_eq!(remainder, clusters[keep..].concat());
        assert_eq!(prefix.as_ptr(), value.as_ptr());
        assert_eq!(remainder.as_ptr(), value[prefix.len()..].as_ptr());
        let expected = if keep == clusters.len() {
            value.clone()
        } else if limit == 0 {
            String::new()
        } else {
            format!("{}…", clusters[..keep - 1].concat())
        };
        assert_eq!(compact_label_preserving_whitespace(&value, limit), expected);
    }
    assert_eq!(split_grapheme_prefix("", usize::MAX), ("", ""));
    assert_eq!(compact_label_preserving_whitespace("   ", 2), " …");
    assert_eq!(compact_label("   ", 2), "");
}

#[test]
fn prefix_cut_does_not_split_a_large_single_cluster() {
    let cluster = format!("a{}", "\u{301}".repeat(4096));
    let value = format!("{cluster}b");
    assert_eq!(split_grapheme_prefix(&value, 1), (cluster.as_str(), "b"));
    assert_eq!(compact_label_preserving_whitespace(&value, 1), "…");
    assert_eq!(compact_label_preserving_whitespace(&value, 2), value);
}
