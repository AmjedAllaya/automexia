//! Retained immediate-mode shaping data; not a general-purpose cache.

use super::ShapedRun;
use rustc_hash::{FxHashMap, FxHasher};
use std::{
    collections::VecDeque,
    hash::{Hash, Hasher},
    sync::Arc,
};

const MAX_ENTRIES: usize = 512;
const MAX_PAYLOAD_BYTES: usize = 2 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(super) struct ShapeKey {
    pub font_id: u32,
    pub size: u16,
    pub style_flags: u8,
}

struct Entry {
    key: ShapeKey,
    text: Box<str>,
    run: Arc<ShapedRun>,
    payload_bytes: usize,
}

#[derive(Default)]
pub(super) struct ShapeCache {
    entries: FxHashMap<u64, Entry>,
    order: VecDeque<u64>,
    payload_bytes: usize,
}

fn hash(key: ShapeKey, text: &str) -> u64 {
    let mut hasher = FxHasher::default();
    key.hash(&mut hasher);
    text.hash(&mut hasher);
    hasher.finish()
}

impl ShapeCache {
    pub(super) fn get(&self, key: ShapeKey, text: &str) -> Option<Arc<ShapedRun>> {
        self.get_hashed(hash(key, text), key, text)
    }

    fn get_hashed(&self, hash: u64, key: ShapeKey, text: &str) -> Option<Arc<ShapedRun>> {
        let entry = self.entries.get(&hash)?;
        // A digest is an index, never proof of text/font identity.
        (entry.key == key && entry.text.as_ref() == text).then(|| Arc::clone(&entry.run))
    }

    pub(super) fn insert(&mut self, key: ShapeKey, text: &str, run: Arc<ShapedRun>) {
        self.insert_hashed(hash(key, text), key, text, run);
    }

    fn insert_hashed(
        &mut self,
        hash: u64,
        key: ShapeKey,
        text: &str,
        run: Arc<ShapedRun>,
    ) {
        let Some(payload_bytes) = run
            .glyphs
            .capacity()
            .checked_mul(std::mem::size_of::<super::ShapedGlyph>())
            .and_then(|bytes| bytes.checked_add(text.len()))
            .filter(|bytes| *bytes <= MAX_PAYLOAD_BYTES)
        else {
            // Oversized runs can still be drawn, but may not displace or grow
            // retained state. Transient shaping and allocator overhead are separate.
            return;
        };
        if let Some(previous) = self.entries.remove(&hash) {
            self.payload_bytes -= previous.payload_bytes;
            self.order.retain(|entry| *entry != hash);
        }
        while self.entries.len() >= MAX_ENTRIES
            || self.payload_bytes > MAX_PAYLOAD_BYTES - payload_bytes
        {
            let Some(oldest) = self.order.pop_front() else {
                return;
            };
            if let Some(previous) = self.entries.remove(&oldest) {
                self.payload_bytes -= previous.payload_bytes;
            }
        }
        self.entries.insert(
            hash,
            Entry {
                key,
                text: text.into(),
                run,
                payload_bytes,
            },
        );
        self.order.push_back(hash);
        self.payload_bytes += payload_bytes;
    }

    #[cfg(test)]
    pub(super) fn len(&self) -> usize {
        self.entries.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::ShapedGlyph;

    const KEY: ShapeKey = ShapeKey {
        font_id: 0,
        size: 14,
        style_flags: 0,
    };

    fn run(capacity: usize) -> Arc<ShapedRun> {
        let mut glyphs = Vec::with_capacity(capacity);
        if capacity > 0 {
            glyphs.push(ShapedGlyph {
                id: 1,
                x: 0.0,
                y: 0.0,
                advance: 8.0,
                cluster: 0,
            });
        }
        Arc::new(ShapedRun {
            font_id: 0,
            size_u16: 14,
            synthetic_bold: false,
            synthetic_italic: false,
            is_color: false,
            #[cfg(not(target_os = "macos"))]
            wght_variation: None,
            ascent_px: 12,
            glyphs,
        })
    }

    fn assert_accounting(cache: &ShapeCache) {
        let independent_payload: usize = cache
            .entries
            .values()
            .map(|entry| {
                entry.text.len()
                    + entry.run.glyphs.capacity() * std::mem::size_of::<ShapedGlyph>()
            })
            .sum();
        assert_eq!(cache.payload_bytes, independent_payload);
        assert!(independent_payload <= MAX_PAYLOAD_BYTES);
        assert!(cache.entries.len() <= MAX_ENTRIES);
        let unique: std::collections::HashSet<_> = cache.order.iter().collect();
        assert_eq!(unique.len(), cache.entries.len());
        assert_eq!(cache.order.len(), cache.entries.len());
        assert!(cache
            .order
            .iter()
            .all(|key| cache.entries.contains_key(key)));
    }

    #[test]
    fn forced_digest_collisions_cannot_return_another_text_or_font_run() {
        let mut cache = ShapeCache::default();
        for key in [
            KEY,
            ShapeKey { size: 15, ..KEY },
            ShapeKey { font_id: 1, ..KEY },
            ShapeKey {
                style_flags: 1,
                ..KEY
            },
        ] {
            let retained = run(1);
            cache.insert_hashed(7, KEY, "first", Arc::clone(&retained));
            assert!(cache.get_hashed(7, KEY, "second").is_none());
            if key != KEY {
                assert!(cache.get_hashed(7, key, "first").is_none());
            }
            let replacement = run(1);
            cache.insert_hashed(7, key, "second", Arc::clone(&replacement));
            assert!(cache.get_hashed(7, KEY, "first").is_none());
            assert!(Arc::ptr_eq(
                &cache.get_hashed(7, key, "second").unwrap(),
                &replacement
            ));
            assert_eq!(cache.len(), 1);
            assert_accounting(&cache);
        }
    }

    #[test]
    fn entry_limit_fifo_and_drop_release_retained_runs() {
        assert_eq!(MAX_ENTRIES, 512);
        let mut cache = ShapeCache::default();
        let first = run(1);
        let first_weak = Arc::downgrade(&first);
        cache.insert(KEY, "0", first);
        for index in 1..MAX_ENTRIES {
            cache.insert(KEY, &index.to_string(), run(1));
            assert_accounting(&cache);
        }
        assert_eq!(cache.len(), MAX_ENTRIES);
        let order = cache.order.clone();
        assert!(cache.get(KEY, "0").is_some());
        assert_eq!(
            cache.order, order,
            "hits do not grow or reorder FIFO bookkeeping"
        );
        cache.insert(KEY, "512", run(1));
        assert!(first_weak.upgrade().is_none());
        assert!(cache.get(KEY, "0").is_none());
        assert!(cache.get(KEY, "1").is_some());
        let last_weak = Arc::downgrade(&cache.get(KEY, "512").unwrap());
        assert_accounting(&cache);
        drop(cache);
        assert!(last_weak.upgrade().is_none());
    }

    #[test]
    fn exact_payload_limit_and_one_byte_over_preserve_prior_entries() {
        assert_eq!(MAX_PAYLOAD_BYTES, 2 * 1024 * 1024);
        let mut cache = ShapeCache::default();
        let retained = run(1);
        let glyph_bytes = retained.glyphs.capacity() * std::mem::size_of::<ShapedGlyph>();
        let mut text = "x".repeat(MAX_PAYLOAD_BYTES - glyph_bytes);
        cache.insert(KEY, &text, Arc::clone(&retained));
        assert_eq!(cache.payload_bytes, MAX_PAYLOAD_BYTES);
        text.push('x');
        cache.insert(KEY, &text, run(1));
        assert_eq!(cache.len(), 1);
        assert_eq!(cache.payload_bytes, MAX_PAYLOAD_BYTES);
        assert!(cache.get(KEY, &text).is_none());
        assert!(cache.get(KEY, &text[..text.len() - 1]).is_some());
        cache.insert(KEY, "small", run(1));
        assert_eq!(cache.len(), 1);
        assert!(cache.get(KEY, "small").is_some());
        assert_accounting(&cache);
    }

    #[test]
    fn spare_glyph_capacity_is_charged_even_when_only_one_glyph_is_used() {
        let mut cache = ShapeCache::default();
        cache.insert(KEY, "prior", run(1));
        let oversized = run(MAX_PAYLOAD_BYTES / std::mem::size_of::<ShapedGlyph>() + 1);
        assert_eq!(oversized.glyphs.len(), 1);
        let weak = Arc::downgrade(&oversized);
        cache.insert(KEY, "oversized", oversized);
        assert!(weak.upgrade().is_none());
        assert!(cache.get(KEY, "oversized").is_none());
        assert!(cache.get(KEY, "prior").is_some());
        assert_accounting(&cache);
    }
}
