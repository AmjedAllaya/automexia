//! System-wide hotkey registration for the quake window. Uses the
//! `global-hotkey` crate: Carbon `RegisterEventHotKey` on macOS (no
//! accessibility permission needed), `RegisterHotKey` on Windows and
//! `XGrabKey` on X11. Pure Wayland has no global hotkey API; users
//! bind a compositor key to a regular Automexia binding instead.

use crate::event::EventProxy;
use global_hotkey::{hotkey::HotKey, GlobalHotKeyManager};
use rio_backend::event::{RioEvent, RioEventType};

/// Keeps the OS hotkey registrations alive for the app's lifetime and owns the
/// exact set required for failure-safe runtime replacement.
pub struct GlobalHotkeys {
    manager: GlobalHotKeyManager,
    registered: Vec<(String, HotKey)>,
}

trait HotkeyRegistry {
    fn register_hotkey(&self, hotkey: HotKey) -> Result<(), String>;
    fn unregister_hotkey(&self, hotkey: HotKey) -> Result<(), String>;
}

impl HotkeyRegistry for GlobalHotKeyManager {
    fn register_hotkey(&self, hotkey: HotKey) -> Result<(), String> {
        self.register(hotkey).map_err(|error| error.to_string())
    }

    fn unregister_hotkey(&self, hotkey: HotKey) -> Result<(), String> {
        self.unregister(hotkey).map_err(|error| error.to_string())
    }
}

fn parse_quake_hotkeys(
    keys: &[rio_backend::config::bindings::KeyBinding],
) -> Result<Vec<(String, HotKey)>, String> {
    let mut parsed = Vec::new();
    for trigger in quake_triggers(keys) {
        let hotkey = parse_hotkey(&trigger)
            .map_err(|error| format!("quake hotkey '{trigger}': {error}"))?;
        if !parsed.iter().any(|(_, existing)| *existing == hotkey) {
            parsed.push((trigger, hotkey));
        }
    }
    Ok(parsed)
}

fn same_hotkey_set(left: &[(String, HotKey)], right: &[(String, HotKey)]) -> bool {
    left.len() == right.len()
        && left
            .iter()
            .all(|(_, hotkey)| right.iter().any(|(_, other)| hotkey == other))
}

fn replace_registered_hotkeys<R: HotkeyRegistry>(
    registry: &R,
    current: &mut Vec<(String, HotKey)>,
    next: Vec<(String, HotKey)>,
) -> Result<(), String> {
    if same_hotkey_set(current, &next) {
        *current = next;
        return Ok(());
    }

    let additions: Vec<_> = next
        .iter()
        .filter(|(_, hotkey)| !current.iter().any(|(_, old)| old == hotkey))
        .cloned()
        .collect();
    let removals: Vec<_> = current
        .iter()
        .filter(|(_, hotkey)| !next.iter().any(|(_, new)| new == hotkey))
        .cloned()
        .collect();

    let mut registered_additions = Vec::new();
    for (trigger, hotkey) in &additions {
        if let Err(error) = registry.register_hotkey(*hotkey) {
            for (_, added) in registered_additions.iter().rev() {
                let _ = registry.unregister_hotkey(*added);
            }
            return Err(format!(
                "quake hotkey '{trigger}' could not be registered: {error}"
            ));
        }
        registered_additions.push((trigger.clone(), *hotkey));
    }

    let mut removed = Vec::new();
    for (trigger, hotkey) in &removals {
        if let Err(error) = registry.unregister_hotkey(*hotkey) {
            let mut rollback_errors = Vec::new();
            for (old_trigger, old) in removed.iter().rev() {
                if let Err(rollback) = registry.register_hotkey(*old) {
                    rollback_errors.push(format!("restore '{old_trigger}': {rollback}"));
                }
            }
            for (added_trigger, added) in registered_additions.iter().rev() {
                if let Err(rollback) = registry.unregister_hotkey(*added) {
                    rollback_errors.push(format!("remove '{added_trigger}': {rollback}"));
                }
            }
            let rollback = if rollback_errors.is_empty() {
                String::new()
            } else {
                format!("; rollback errors: {}", rollback_errors.join(", "))
            };
            return Err(format!(
                "obsolete quake hotkey '{trigger}' could not be unregistered: {error}{rollback}"
            ));
        }
        removed.push((trigger.clone(), *hotkey));
    }

    for (trigger, _) in &additions {
        tracing::info!("registered global hotkey: {trigger}");
    }
    *current = next;
    Ok(())
}

impl GlobalHotkeys {
    pub fn try_replace(
        &mut self,
        keys: &[rio_backend::config::bindings::KeyBinding],
    ) -> Result<(), String> {
        let next = parse_quake_hotkeys(keys)?;
        replace_registered_hotkeys(&self.manager, &mut self.registered, next)
    }

    pub fn is_empty(&self) -> bool {
        self.registered.is_empty()
    }
}

/// Register every configured `ToggleQuake` chord as one transaction. Invalid
/// triggers or OS registration failures return an error; partial registrations
/// are rolled back instead of silently weakening the active configuration.
pub fn setup(
    event_proxy: EventProxy,
    keys: &[rio_backend::config::bindings::KeyBinding],
) -> Result<Option<GlobalHotkeys>, String> {
    let hotkeys = parse_quake_hotkeys(keys)?;
    if hotkeys.is_empty() {
        return Ok(None);
    }

    let manager = GlobalHotKeyManager::new()
        .map_err(|error| format!("global hotkeys unavailable: {error}"))?;
    let mut registered = Vec::new();
    replace_registered_hotkeys(&manager, &mut registered, hotkeys)?;

    // The crate's event receiver is a process-wide channel; one listener serves
    // every manager, including managers updated after configuration reload.
    static LISTENER: std::sync::Once = std::sync::Once::new();
    LISTENER.call_once(move || {
        std::thread::spawn(move || {
            use global_hotkey::{GlobalHotKeyEvent, HotKeyState};
            while let Ok(event) = GlobalHotKeyEvent::receiver().recv() {
                if event.state == HotKeyState::Pressed {
                    event_proxy
                        .send_event(RioEventType::Rio(RioEvent::ToggleQuake), unsafe {
                            rio_window::window::WindowId::dummy().into()
                        });
                }
            }
        });
    });

    Ok(Some(GlobalHotkeys {
        manager,
        registered,
    }))
}

/// Triggers for every `ToggleQuake` binding in the config, in the
/// format `parse_hotkey` accepts.
pub fn quake_triggers(keys: &[rio_backend::config::bindings::KeyBinding]) -> Vec<String> {
    keys.iter()
        .filter(|binding| binding.action.to_lowercase() == "togglequake")
        .map(|binding| {
            let mods = binding.with.replace(' ', "").replace('|', "+");
            if mods.is_empty() {
                binding.key.clone()
            } else {
                format!("{}+{}", mods, binding.key)
            }
        })
        .collect()
}

/// Parse a binding-style trigger ("super+shift+q", "f12") into a
/// `HotKey`. Accepts the same modifier names as `[bindings]` `with`.
fn parse_hotkey(trigger: &str) -> Result<HotKey, String> {
    use global_hotkey::hotkey::{Code, Modifiers};

    let lowered = trigger.to_lowercase();
    let mut modifiers = Modifiers::empty();
    let mut code = None;

    for part in lowered.split('+') {
        match part.trim() {
            "cmd" | "super" | "command" => modifiers |= Modifiers::SUPER,
            "ctrl" | "control" => modifiers |= Modifiers::CONTROL,
            "alt" | "option" => modifiers |= Modifiers::ALT,
            "shift" => modifiers |= Modifiers::SHIFT,
            // The bindings parser accepts "none" as a no-op modifier.
            "none" => {}
            "escape" | "esc" => code = Some(Code::Escape),
            "space" => code = Some(Code::Space),
            "enter" | "return" => code = Some(Code::Enter),
            "tab" => code = Some(Code::Tab),
            "back" | "backspace" => code = Some(Code::Backspace),
            "delete" => code = Some(Code::Delete),
            "insert" => code = Some(Code::Insert),
            "home" => code = Some(Code::Home),
            "end" => code = Some(Code::End),
            "pageup" => code = Some(Code::PageUp),
            "pagedown" => code = Some(Code::PageDown),
            "up" => code = Some(Code::ArrowUp),
            "down" => code = Some(Code::ArrowDown),
            "left" => code = Some(Code::ArrowLeft),
            "right" => code = Some(Code::ArrowRight),
            "f1" => code = Some(Code::F1),
            "f2" => code = Some(Code::F2),
            "f3" => code = Some(Code::F3),
            "f4" => code = Some(Code::F4),
            "f5" => code = Some(Code::F5),
            "f6" => code = Some(Code::F6),
            "f7" => code = Some(Code::F7),
            "f8" => code = Some(Code::F8),
            "f9" => code = Some(Code::F9),
            "f10" => code = Some(Code::F10),
            "f11" => code = Some(Code::F11),
            "f12" => code = Some(Code::F12),
            "`" | "grave" | "backquote" => code = Some(Code::Backquote),
            "'" | "quote" => code = Some(Code::Quote),
            "," | "comma" => code = Some(Code::Comma),
            "." | "period" => code = Some(Code::Period),
            "/" | "slash" => code = Some(Code::Slash),
            "\\" | "backslash" => code = Some(Code::Backslash),
            ";" | "semicolon" => code = Some(Code::Semicolon),
            "-" | "minus" => code = Some(Code::Minus),
            "=" | "equal" => code = Some(Code::Equal),
            "[" | "bracketleft" => code = Some(Code::BracketLeft),
            "]" | "bracketright" => code = Some(Code::BracketRight),
            "numpadenter" => code = Some(Code::NumpadEnter),
            "numpadadd" => code = Some(Code::NumpadAdd),
            "numpadsubtract" => code = Some(Code::NumpadSubtract),
            "numpadmultiply" => code = Some(Code::NumpadMultiply),
            "numpaddivide" => code = Some(Code::NumpadDivide),
            "numpaddecimal" => code = Some(Code::NumpadDecimal),
            "numpadcomma" => code = Some(Code::NumpadComma),
            "numpadequals" => code = Some(Code::NumpadEqual),
            "numpad0" => code = Some(Code::Numpad0),
            "numpad1" => code = Some(Code::Numpad1),
            "numpad2" => code = Some(Code::Numpad2),
            "numpad3" => code = Some(Code::Numpad3),
            "numpad4" => code = Some(Code::Numpad4),
            "numpad5" => code = Some(Code::Numpad5),
            "numpad6" => code = Some(Code::Numpad6),
            "numpad7" => code = Some(Code::Numpad7),
            "numpad8" => code = Some(Code::Numpad8),
            "numpad9" => code = Some(Code::Numpad9),
            key if key.len() == 1 => {
                let ch = key.chars().next().unwrap();
                code = Some(match ch {
                    'a'..='z' => letter_code(ch),
                    '0'..='9' => digit_code(ch),
                    _ => return Err(format!("unsupported key '{key}'")),
                });
            }
            other => return Err(format!("unknown key or modifier '{other}'")),
        }
    }

    let code = code.ok_or_else(|| "no key in trigger".to_string())?;
    Ok(HotKey::new(
        (!modifiers.is_empty()).then_some(modifiers),
        code,
    ))
}

fn letter_code(ch: char) -> global_hotkey::hotkey::Code {
    use global_hotkey::hotkey::Code::*;
    match ch {
        'a' => KeyA,
        'b' => KeyB,
        'c' => KeyC,
        'd' => KeyD,
        'e' => KeyE,
        'f' => KeyF,
        'g' => KeyG,
        'h' => KeyH,
        'i' => KeyI,
        'j' => KeyJ,
        'k' => KeyK,
        'l' => KeyL,
        'm' => KeyM,
        'n' => KeyN,
        'o' => KeyO,
        'p' => KeyP,
        'q' => KeyQ,
        'r' => KeyR,
        's' => KeyS,
        't' => KeyT,
        'u' => KeyU,
        'v' => KeyV,
        'w' => KeyW,
        'x' => KeyX,
        'y' => KeyY,
        'z' => KeyZ,
        _ => unreachable!(),
    }
}

fn digit_code(ch: char) -> global_hotkey::hotkey::Code {
    use global_hotkey::hotkey::Code::*;
    match ch {
        '0' => Digit0,
        '1' => Digit1,
        '2' => Digit2,
        '3' => Digit3,
        '4' => Digit4,
        '5' => Digit5,
        '6' => Digit6,
        '7' => Digit7,
        '8' => Digit8,
        '9' => Digit9,
        _ => unreachable!(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use global_hotkey::hotkey::{Code, Modifiers};

    #[test]
    fn parse_hotkey_forms() {
        let hk = parse_hotkey("super+shift+q").unwrap();
        assert_eq!(
            hk,
            HotKey::new(Some(Modifiers::SUPER | Modifiers::SHIFT), Code::KeyQ)
        );
        let hk = parse_hotkey("control+`").unwrap();
        assert_eq!(hk, HotKey::new(Some(Modifiers::CONTROL), Code::Backquote));
        let hk = parse_hotkey("f12").unwrap();
        assert_eq!(hk, HotKey::new(None, Code::F12));
        assert!(parse_hotkey("super+banana").is_err());
        assert!(parse_hotkey("super+shift").is_err());
        // Parity with the bindings parser.
        let hk = parse_hotkey("none+f12").unwrap();
        assert_eq!(hk, HotKey::new(None, Code::F12));
        let hk = parse_hotkey("control+numpad5").unwrap();
        assert_eq!(hk, HotKey::new(Some(Modifiers::CONTROL), Code::Numpad5));
        let hk = parse_hotkey("shift+insert").unwrap();
        assert_eq!(hk, HotKey::new(Some(Modifiers::SHIFT), Code::Insert));
    }

    #[derive(Default)]
    struct FakeRegistry {
        active: std::cell::RefCell<Vec<HotKey>>,
        fail_register: std::cell::Cell<Option<HotKey>>,
        fail_unregister: std::cell::Cell<Option<HotKey>>,
    }

    impl HotkeyRegistry for FakeRegistry {
        fn register_hotkey(&self, hotkey: HotKey) -> Result<(), String> {
            if self.fail_register.get() == Some(hotkey) {
                return Err("injected register failure".to_string());
            }
            let mut active = self.active.borrow_mut();
            if active.contains(&hotkey) {
                return Err("already registered".to_string());
            }
            active.push(hotkey);
            Ok(())
        }

        fn unregister_hotkey(&self, hotkey: HotKey) -> Result<(), String> {
            if self.fail_unregister.get() == Some(hotkey) {
                return Err("injected unregister failure".to_string());
            }
            let mut active = self.active.borrow_mut();
            let Some(index) = active.iter().position(|active| *active == hotkey) else {
                return Err("not registered".to_string());
            };
            active.remove(index);
            Ok(())
        }
    }

    fn binding(
        key: &str,
        with: &str,
        action: &str,
    ) -> rio_backend::config::bindings::KeyBinding {
        rio_backend::config::bindings::KeyBinding {
            key: key.into(),
            with: with.into(),
            action: action.into(),
            esc: String::new(),
            mode: String::new(),
        }
    }

    #[test]
    fn quake_hotkey_parse_is_all_or_nothing() {
        let keys = vec![
            binding("q", "control", "ToggleQuake"),
            binding("banana", "super", "ToggleQuake"),
        ];
        assert!(parse_quake_hotkeys(&keys).is_err());
    }

    #[test]
    fn failed_hotkey_addition_keeps_the_previous_registration() {
        let old = HotKey::new(Some(Modifiers::CONTROL), Code::KeyA);
        let replacement = HotKey::new(Some(Modifiers::CONTROL), Code::KeyB);
        let registry = FakeRegistry::default();
        registry.active.borrow_mut().push(old);
        registry.fail_register.set(Some(replacement));
        let mut current = vec![("control+a".to_string(), old)];
        let next = vec![("control+b".to_string(), replacement)];

        assert!(replace_registered_hotkeys(&registry, &mut current, next).is_err());
        assert_eq!(current, vec![("control+a".to_string(), old)]);
        assert_eq!(*registry.active.borrow(), vec![old]);
    }

    #[test]
    fn partial_hotkey_addition_is_rolled_back_before_removal() {
        let old = HotKey::new(Some(Modifiers::CONTROL), Code::KeyA);
        let first = HotKey::new(Some(Modifiers::CONTROL), Code::KeyB);
        let failing = HotKey::new(Some(Modifiers::CONTROL), Code::KeyC);
        let registry = FakeRegistry::default();
        registry.active.borrow_mut().push(old);
        registry.fail_register.set(Some(failing));
        let mut current = vec![("control+a".to_string(), old)];
        let next = vec![
            ("control+b".to_string(), first),
            ("control+c".to_string(), failing),
        ];

        assert!(replace_registered_hotkeys(&registry, &mut current, next).is_err());
        assert_eq!(current, vec![("control+a".to_string(), old)]);
        assert_eq!(*registry.active.borrow(), vec![old]);
    }

    #[test]
    fn failed_hotkey_removal_rolls_back_new_registration() {
        let old = HotKey::new(Some(Modifiers::CONTROL), Code::KeyA);
        let replacement = HotKey::new(Some(Modifiers::CONTROL), Code::KeyB);
        let registry = FakeRegistry::default();
        registry.active.borrow_mut().push(old);
        registry.fail_unregister.set(Some(old));
        let mut current = vec![("control+a".to_string(), old)];
        let next = vec![("control+b".to_string(), replacement)];

        assert!(replace_registered_hotkeys(&registry, &mut current, next).is_err());
        assert_eq!(current, vec![("control+a".to_string(), old)]);
        assert_eq!(*registry.active.borrow(), vec![old]);
    }

    #[test]
    fn quake_triggers_from_bindings() {
        let keys = vec![
            binding("q", "super | shift", "ToggleQuake"),
            binding("f12", "", "togglequake"),
            binding("f12", "none", "togglequake"),
            binding("w", "super", "quit"),
        ];
        let triggers = quake_triggers(&keys);
        assert_eq!(triggers, vec!["super+shift+q", "f12", "none+f12"]);
        for trigger in &triggers {
            assert!(parse_hotkey(trigger).is_ok(), "{trigger} must parse");
        }
    }
}
