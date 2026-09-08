// Forked verbatim (modulo module paths) from
// https://github.com/wezterm/wezterm/blob/main/wezterm-char-props/src/emoji.rs
// (MIT, Copyright (c) 2018-Present Wez Furlong).

use crate::emoji_variation::VARIATION_MAP;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Presentation {
    Text,
    Emoji,
}

impl Presentation {
    /// Returns the default presentation followed
    /// by the explicit presentation if specified
    /// by a variation selector
    pub fn for_grapheme(s: &str) -> (Self, Option<Self>) {
        if let Some((a, b)) = VARIATION_MAP.get(s) {
            return (*a, Some(*b));
        }
        let mut presentation = Self::Text;
        for c in s.chars() {
            if Self::for_char(c) == Self::Emoji {
                presentation = Self::Emoji;
                break;
            }
            // Note that `c` may be some other combining
            // sequence that doesn't definitively indicate
            // that we're text, so we only positively
            // change presentation when we identify an
            // emoji char.
        }
        (presentation, None)
    }

    pub fn for_char(c: char) -> Self {
        if crate::emoji_presentation::EMOJI_PRESENTATION.contains_u32(c as u32) {
            Self::Emoji
        } else {
            Self::Text
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Presentation;

    #[test]
    fn every_generated_variation_has_an_exact_lookup_and_rejects_suffixes() {
        let table = &crate::emoji_variation::VARIATION_MAP;
        // Enumerating the source entries is independent of the hash lookup:
        // the PHF 0.14 update compiled but silently lost known Unicode keys.
        assert_eq!(table.entries.len(), 708);
        for (key, expected) in table.entries {
            assert_eq!(table.get(key), Some(expected), "variation {key:?}");
            assert_eq!(
                Presentation::for_grapheme(key),
                (expected.0, Some(expected.1))
            );
            #[cfg(feature = "std")]
            assert_eq!(table.get(&format!("{key}x")), None);
        }
        for absent in ["", "plain", "\u{fe0e}", "\u{fe0f}", "a\u{fe0f}"] {
            assert_eq!(table.get(absent), None);
        }
    }

    #[test]
    fn generated_variation_map_matches_its_phf_runtime() {
        assert_eq!(
            Presentation::for_grapheme("\u{1F39F}\u{FE0F}"),
            (Presentation::Text, Some(Presentation::Emoji))
        );
        assert_eq!(
            Presentation::for_grapheme("\u{1F44D}\u{FE0E}"),
            (Presentation::Emoji, Some(Presentation::Text))
        );
    }
}
