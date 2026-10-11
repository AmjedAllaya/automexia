//! Sparse, typed core overrides. Values are applied only through UserPreferences.
//! No executable, environment, path, credential or arbitrary TOML is persisted here.
use automexia_ui_model::settings::{Change, SettingValue, SettingsError};
use rio_backend::config::Config;
use serde::{Deserialize, Serialize};

macro_rules! general_preferences {
    (booleans { $( $flag:ident => $($flag_path:ident).+; )* }
     numbers { $( $number:ident: $ty:ty => $($number_path:ident).+, $min:literal, $max:literal; )* }) => {
        #[derive(Clone, Debug, Default, PartialEq, Deserialize, Serialize)]
        #[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
        pub struct GeneralPreferences {
            #[serde(skip_serializing_if = "Option::is_none")]
            pub cursor_shape: Option<rio_backend::ansi::CursorShape>,
            $(#[serde(skip_serializing_if = "Option::is_none")] pub $flag: Option<bool>,)*
            $(#[serde(skip_serializing_if = "Option::is_none")] pub $number: Option<$ty>,)*
        }
        impl GeneralPreferences {
            pub fn is_empty(&self) -> bool {
                self.cursor_shape.is_none() $(&& self.$flag.is_none())* $(&& self.$number.is_none())*
            }
            pub fn is_valid(&self) -> bool {
                true $(&& self.$number.is_none_or(|v| {
                    let value = v as f64;
                    value.is_finite() && ($min as f64..=$max as f64).contains(&value)
                }))*
            }
            pub fn apply_to(&self, config: &mut Config) {
                if let Some(shape) = self.cursor_shape { config.cursor.shape = shape; }
                $(if let Some(value) = self.$flag { config.$($flag_path).+ = value; })*
                $(if let Some(value) = self.$number { config.$($number_path).+ = value; })*
            }
            pub fn contains(&self, key: &str) -> bool {
                match key {
                    "cursor_shape" => self.cursor_shape.is_some(),
                    $(stringify!($flag) => self.$flag.is_some(),)*
                    $(stringify!($number) => self.$number.is_some(),)*
                    _ => false,
                }
            }
            pub fn value(key: &str, config: &Config) -> Option<SettingValue> {
                Some(match key {
                    "cursor_shape" => SettingValue::Choice(match config.cursor.shape {
                        rio_backend::ansi::CursorShape::Block => "block",
                        rio_backend::ansi::CursorShape::Underline => "underline",
                        rio_backend::ansi::CursorShape::Beam => "beam",
                        rio_backend::ansi::CursorShape::Hidden => "hidden",
                    }.into()),
                    $(stringify!($flag) => SettingValue::Boolean(config.$($flag_path).+),)*
                    $(stringify!($number) => SettingValue::Number(config.$($number_path).+ as f64),)*
                    _ => return None,
                })
            }
            pub fn bounds(key: &str) -> Option<(f64, f64)> {
                match key {
                    $(stringify!($number) => Some(($min as f64, $max as f64)),)*
                    _ => None,
                }
            }
            pub fn set(&mut self, key: &str, change: &Change) -> Result<(), SettingsError> {
                match key {
                    "cursor_shape" => self.cursor_shape = match change {
                        Change::Reset => None,
                        Change::Set(SettingValue::Choice(value)) => Some(match value.as_str() {
                            "block" => rio_backend::ansi::CursorShape::Block,
                            "underline" => rio_backend::ansi::CursorShape::Underline,
                            "beam" => rio_backend::ansi::CursorShape::Beam,
                            "hidden" => rio_backend::ansi::CursorShape::Hidden,
                            _ => return Err(SettingsError::InvalidValue),
                        }),
                        _ => return Err(SettingsError::InvalidValue),
                    },
                    $(stringify!($flag) => self.$flag = match change {
                        Change::Reset => None,
                        Change::Set(SettingValue::Boolean(value)) => Some(*value),
                        _ => return Err(SettingsError::InvalidValue),
                    },)*
                    $(stringify!($number) => self.$number = match change {
                        Change::Reset => None,
                        Change::Set(SettingValue::Number(value)) if value.is_finite()
                            && ($min as f64..=$max as f64).contains(value)
                            && (*value as $ty) as f64 == *value => Some(*value as $ty),
                        _ => return Err(SettingsError::InvalidValue),
                    },)*
                    _ => return Err(SettingsError::UnknownSetting),
                }
                Ok(())
            }
        }
    };
}
general_preferences! {
    booleans {
        copy_on_select => copy_on_select;
        confirm_before_quit => confirm_before_quit;
        hide_cursor_when_typing => hide_cursor_when_typing;
        selection_foreground => ignore_selection_fg_color;
        bright_bold => draw_bold_text_with_light_colors;
        scrollbar => enable_scroll_bar;
        audio_bell => bell.audio;
        cursor_blink => cursor.blinking;
        alt_escape => keyboard.disable_ctlseqs_alt;
        ime_position => keyboard.ime_cursor_positioning;
        pause_unfocused => renderer.disable_unfocused_render;
        pause_occluded => renderer.disable_occluded_render;
        cpu_renderer => renderer.use_cpu;
        inherit_directory => navigation.current_working_directory;
        hide_single_tab => navigation.hide_if_single;
        editor_split => navigation.open_config_with_split;
        recovery_enabled => session_recovery.enabled;
        recovery_history => session_recovery.save_history;
    }
    numbers {
        cursor_interval: u64 => cursor.blinking_interval, 350, 1200;
        scroll_multiplier: f64 => scroll.multiplier, 0.1, 100.0;
        scroll_divider: f64 => scroll.divider, 0.1, 100.0;
        history_limit: usize => scrollback_history_limit, 0, 1000000;
        window_width: i32 => window.width, 320, 16384;
        window_height: i32 => window.height, 200, 16384;
    }
}

/// Preserve choices owned by already-created process-wide services. Saving an
/// override still affects the next launch, but unrelated live edits must never
/// claim to replace the renderer or reinitialize the recovery manager.
#[doc(hidden)]
pub fn retain_restart_settings(running: &Config, candidate: &mut Config) {
    candidate.renderer.use_cpu = running.renderer.use_cpu;
    candidate.session_recovery.enabled = running.session_recovery.enabled;
    candidate.session_recovery.save_history = running.session_recovery.save_history;
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sparse_overrides_reset_to_the_current_config_and_reject_invalid_values() {
        let mut prefs = GeneralPreferences::default();
        let mut base = Config {
            copy_on_select: true,
            ..Default::default()
        };
        prefs
            .set("copy_on_select", &Change::Set(SettingValue::Boolean(false)))
            .unwrap();
        let mut effective = base.clone();
        prefs.apply_to(&mut effective);
        assert!(!effective.copy_on_select);
        prefs.set("copy_on_select", &Change::Reset).unwrap();
        prefs.apply_to(&mut base);
        assert!(base.copy_on_select);
        assert!(prefs.is_empty());
        for value in [f64::NAN, f64::INFINITY, -1.0, 1000001.0, 2.5] {
            assert!(prefs
                .set("history_limit", &Change::Set(SettingValue::Number(value)))
                .is_err());
        }
        for value in [0.0, 0.09, f64::NAN, 100.1] {
            assert!(prefs
                .set("scroll_divider", &Change::Set(SettingValue::Number(value)))
                .is_err());
        }
        assert!(prefs.set("env_vars", &Change::Reset).is_err());
    }
    #[test]
    fn restart_choices_stay_deferred_across_unrelated_edits_and_config_reloads() {
        let running = Config::default();
        let mut prefs = GeneralPreferences {
            cpu_renderer: Some(!running.renderer.use_cpu),
            recovery_enabled: Some(!running.session_recovery.enabled),
            recovery_history: Some(!running.session_recovery.save_history),
            copy_on_select: Some(!running.copy_on_select),
            history_limit: Some(12345),
            ..Default::default()
        };
        for iteration in 0..3 {
            // Both a cold launch and a reload first resolve saved preferences.
            let mut candidate = running.clone();
            candidate.fonts.size += iteration as f32;
            prefs.apply_to(&mut candidate);
            assert_ne!(candidate.renderer.use_cpu, running.renderer.use_cpu);
            assert_ne!(
                candidate.session_recovery.enabled,
                running.session_recovery.enabled
            );
            assert_ne!(
                candidate.session_recovery.save_history,
                running.session_recovery.save_history
            );
            retain_restart_settings(&running, &mut candidate);
            assert_eq!(candidate.renderer.use_cpu, running.renderer.use_cpu);
            assert_eq!(candidate.session_recovery, running.session_recovery);
            assert_eq!(candidate.copy_on_select, prefs.copy_on_select.unwrap());
            assert_eq!(candidate.scrollback_history_limit, 12345);
            prefs.copy_on_select = Some(!candidate.copy_on_select);
        }
    }

    #[test]
    fn serialized_overrides_are_bounded_typed_and_deny_unknown_fields() {
        let prefs = GeneralPreferences {
            window_width: Some(1200),
            cursor_blink: Some(false),
            ..Default::default()
        };
        let encoded = toml::to_string(&prefs).unwrap();
        assert_eq!(
            toml::from_str::<GeneralPreferences>(&encoded).unwrap(),
            prefs
        );
        assert!(!encoded.contains("env"));
        assert!(toml::from_str::<GeneralPreferences>("shell = 'unsafe'").is_err());
        assert!(toml::from_str::<GeneralPreferences>("copy-on-select = 'yes'").is_err());
        assert!(!GeneralPreferences {
            scroll_divider: Some(0.0),
            ..Default::default()
        }
        .is_valid());
    }
}
