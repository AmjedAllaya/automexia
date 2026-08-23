use automexia_keybindings::ProfileId;
use serde::{Deserialize, Serialize};

use super::defaults::{
    default_disable_ctlseqs_alt, default_forward_to_ime_modifier_mask,
    default_ime_cursor_positioning,
};

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct Keyboard {
    /// Selects Automexia defaults or an explicit versioned compatibility
    /// profile. The moving `ghostty` alias resolves in automexia-keybindings.
    #[serde(default, rename = "binding-profile")]
    pub binding_profile: ProfileId,

    /// Strict mode rejects a candidate profile when an action is unavailable.
    /// Permissive mode must be chosen explicitly and retains diagnostics while
    /// dropping only unsupported bindings.
    #[serde(default = "default_binding_strict", rename = "binding-strict")]
    pub binding_strict: bool,
    // Disable ctlseqs with ALT keys
    // For example: Terminal.app does not deal with ctlseqs with ALT keys
    #[serde(
        default = "default_disable_ctlseqs_alt",
        rename = "disable-ctlseqs-alt"
    )]
    pub disable_ctlseqs_alt: bool,

    // Enable IME cursor positioning
    // When enabled, the IME input popup will appear at the cursor position
    #[serde(
        default = "default_ime_cursor_positioning",
        rename = "ime-cursor-positioning"
    )]
    pub ime_cursor_positioning: bool,

    // Modifier mask deciding when a key event is forwarded to the macOS IME.
    // A key event is forwarded when no modifier is pressed, or when the
    // pressed modifiers are all contained in this mask. Otherwise the event is handled
    // directly by the application without going through the IME.
    //
    // Accepted values (case-insensitive): "shift", "ctrl", "alt", "super".
    // Useful for input methods like SKK that need to receive Ctrl+key
    // combinations directly.
    #[serde(
        default = "default_forward_to_ime_modifier_mask",
        rename = "forward-to-ime-modifier-mask"
    )]
    pub forward_to_ime_modifier_mask: Vec<String>,
}

const fn default_binding_strict() -> bool {
    true
}
#[allow(clippy::derivable_impls)]
impl Default for Keyboard {
    fn default() -> Keyboard {
        Keyboard {
            binding_profile: ProfileId::Automexia,
            binding_strict: default_binding_strict(),
            #[cfg(target_os = "macos")]
            disable_ctlseqs_alt: true,
            #[cfg(not(target_os = "macos"))]
            disable_ctlseqs_alt: false,
            ime_cursor_positioning: default_ime_cursor_positioning(),
            forward_to_ime_modifier_mask: default_forward_to_ime_modifier_mask(),
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_defaults_preserve_automexia_and_strict_validation() {
        let keyboard = Keyboard::default();
        assert_eq!(keyboard.binding_profile, ProfileId::Automexia);
        assert!(keyboard.binding_strict);
    }

    #[test]
    fn explicit_versioned_and_moving_profiles_deserialize() {
        #[derive(Deserialize)]
        struct Root {
            keyboard: Keyboard,
        }

        let versioned: Root = toml::from_str(
            "[keyboard]\nbinding-profile = 'ghostty-1.3'\nbinding-strict = false\n",
        )
        .unwrap();
        assert_eq!(versioned.keyboard.binding_profile, ProfileId::Ghostty13);
        assert!(!versioned.keyboard.binding_strict);

        let moving: Root =
            toml::from_str("[keyboard]\nbinding-profile = 'ghostty'\n").unwrap();
        assert_eq!(moving.keyboard.binding_profile, ProfileId::Ghostty);
        assert!(moving.keyboard.binding_strict);
    }
}
