//! Shared font-selection and color-emoji policy.
//!
//! Explicit user/session choices win. Installed emoji families are registered
//! after the configured text faces, before general system fallback. Missing
//! scripts use the native cascade (including its locale-sensitive CJK choice).
//! No fonts are downloaded and no fallback changes terminal cell dimensions.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FallbackStage {
    SessionGlyph,
    UserSymbolMap,
    ConfiguredStyle,
    ConfiguredRegular,
    InstalledColorEmoji,
    NativeScriptCascade,
    MissingGlyph,
}

pub const FALLBACK_CHAIN: &[FallbackStage] = &[
    FallbackStage::SessionGlyph,
    FallbackStage::UserSymbolMap,
    FallbackStage::ConfiguredStyle,
    FallbackStage::ConfiguredRegular,
    FallbackStage::InstalledColorEmoji,
    FallbackStage::NativeScriptCascade,
    FallbackStage::MissingGlyph,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Platform {
    Windows,
    MacOs,
    Unix,
}

/// Candidates are optional installed fonts, never installation requirements.
/// Explicitly configured monochrome faces remain authoritative when they cover
/// a cluster. Otherwise use a color face when installed, then native fallback.
pub const fn color_emoji_families(platform: Platform) -> &'static [&'static str] {
    match platform {
        Platform::Windows => &["Segoe UI Emoji", "Noto Color Emoji"],
        Platform::MacOs => &["Apple Color Emoji", "Noto Color Emoji"],
        Platform::Unix => &["Noto Color Emoji", "Twemoji Mozilla"],
    }
}

pub const fn native_platform() -> Platform {
    if cfg!(target_os = "windows") {
        Platform::Windows
    } else if cfg!(target_os = "macos") {
        Platform::MacOs
    } else {
        Platform::Unix
    }
}

/// Color glyphs keep their font-provided palette. ANSI foreground tint applies
/// to monochrome glyphs only; selection/cursor backgrounds remain independent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColorEmojiPolicy {
    PreserveFontPalette,
}

pub const COLOR_EMOJI_POLICY: ColorEmojiPolicy = ColorEmojiPolicy::PreserveFontPalette;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_choices_precede_optional_emoji_and_native_script_fallback() {
        assert_eq!(FALLBACK_CHAIN[0], FallbackStage::SessionGlyph);
        assert_eq!(FALLBACK_CHAIN[1], FallbackStage::UserSymbolMap);
        assert_eq!(FALLBACK_CHAIN[4], FallbackStage::InstalledColorEmoji);
        assert_eq!(FALLBACK_CHAIN.last(), Some(&FallbackStage::MissingGlyph));
    }

    #[test]
    fn every_platform_has_a_bounded_deterministic_emoji_chain() {
        for platform in [Platform::Windows, Platform::MacOs, Platform::Unix] {
            let families = color_emoji_families(platform);
            assert_eq!(families.len(), 2);
            assert_ne!(families[0], families[1]);
            assert!(families
                .iter()
                .all(|name| !name.is_empty() && name.len() < 64));
        }
        assert_eq!(color_emoji_families(Platform::Windows)[0], "Segoe UI Emoji");
        assert_eq!(
            color_emoji_families(Platform::MacOs)[0],
            "Apple Color Emoji"
        );
        assert_eq!(color_emoji_families(Platform::Unix)[0], "Noto Color Emoji");
        assert_eq!(COLOR_EMOJI_POLICY, ColorEmojiPolicy::PreserveFontPalette);
    }
}
