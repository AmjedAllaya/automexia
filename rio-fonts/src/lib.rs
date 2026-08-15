// Copyright (c) 2023-present, Raphael Amorim.
//
// This source code is licensed under the MIT license found in the
// LICENSE file in the root directory of this source tree.

//! Font concerns shared by Rio's renderers, with no rendering, GPU, or
//! windowing dependencies.
//!
//! - [`nerd_font`]: the Nerd Fonts patcher's per-glyph scaling and
//!   alignment rules (by way of ghostty's generated table), so every
//!   renderer places Powerline separators and icon glyphs identically.
//! - The symbols-only Nerd Font itself, embedded behind the
//!   `symbols-nerd-font` feature for renderers that want icon glyphs
//!   available with no font file shipped or installed (the same call
//!   libghostty makes).

#![forbid(unsafe_code)]

pub mod nerd_font;

/// Symbols-only Nerd Font as raw TTF bytes.
#[cfg(feature = "symbols-nerd-font")]
pub static SYMBOLS_NERD_FONT: &[u8] =
    include_bytes!("../resources/SymbolsNerdFontMono/SymbolsNerdFontMono-Regular.ttf");

#[cfg(test)]
mod tests {
    const CATEGORY_FOLDER_BADGES: &[(u32, &str)] = &[
        (0xF0250, "sensitive/locked"),
        (0xF107F, "configuration"),
        (0xF0C82, "logs/text"),
        (0xF19F6, "source/file"),
        (0xF10B7, "documentation/information"),
        (0xF197E, "tests/check"),
        (0xF0D0B, "build/sync"),
        (0xF024F, "assets/image"),
        (0xF0253, "packages/multiple"),
        (0xE5FB, "repository/git"),
        (0xF19FC, "tools/wrench"),
        (0xF12E3, "data/table"),
        (0xF0ABA, "ephemeral/clock"),
        (0xF0870, "infrastructure/network"),
        (0xF06EB, "packaging/archive"),
        (0xE5FF, "generic folder"),
    ];

    #[test]
    fn embedded_symbols_font_contains_every_category_folder_badge() {
        let font = include_bytes!(
            "../resources/SymbolsNerdFontMono/SymbolsNerdFontMono-Regular.ttf"
        );
        let face =
            ttf_parser::Face::parse(font, 0).expect("bundled Symbols Nerd Font is valid");

        for &(codepoint, role) in CATEGORY_FOLDER_BADGES {
            let character =
                char::from_u32(codepoint).expect("folder badge is valid Unicode");
            assert!(
                face.glyph_index(character).is_some(),
                "bundled Symbols Nerd Font is missing {role} badge U+{codepoint:04X}"
            );
        }
    }
}
