use crate::{MAX_IDENTIFIER_BYTES, MAX_PARAMETER_BYTES};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct ActionId(String);

impl ActionId {
    pub fn new(value: impl Into<String>) -> Result<Self, &'static str> {
        let value = value.into();
        if value.is_empty() || value.len() > MAX_IDENTIFIER_BYTES {
            return Err("action identifier length is invalid");
        }
        if !value.bytes().all(|byte| {
            byte.is_ascii_lowercase()
                || byte.is_ascii_digit()
                || matches!(byte, b'_' | b'-' | b'.')
        }) {
            return Err("action identifier contains unsupported characters");
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for ActionId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::new(value).map_err(serde::de::Error::custom)
    }
}

impl fmt::Display for ActionId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParameterKind {
    None,
    Text,
    OptionalText,
    Integer,
    UnsignedInteger,
    Float,
    Direction,
    Table,
    PathDisposition,
    LaunchPolicy,
    Toggle,
    TabTarget,
    Resize,
    CopyMode,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionCapability {
    InputWrite,
    ClipboardRead,
    ClipboardWrite,
    TerminalMutation,
    LayoutMutation,
    WindowMutation,
    Configuration,
    FilesystemWrite,
    ExternalOpen,
    Inspector,
    TopologyLifecycle,
    DeliberateCrash,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SupportLevel {
    Supported,
    Adapted,
    Unavailable,
    DeprecatedUnsafe,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct ActionSchema {
    pub id: &'static str,
    pub aliases: &'static [&'static str],
    pub parameter: ParameterKind,
    pub capability: ActionCapability,
    pub support: SupportLevel,
}

macro_rules! action {
    ($id:literal, [$($alias:literal),* $(,)?], $parameter:ident, $capability:ident, $support:ident) => {
        ActionSchema {
            id: $id,
            aliases: &[$($alias),*],
            parameter: ParameterKind::$parameter,
            capability: ActionCapability::$capability,
            support: SupportLevel::$support,
        }
    };
}

// This list intentionally mirrors `ghostty +list-actions` for v1.3.1. Actions
// without an Automexia owner remain visible but fail compilation in strict
// mode; unsupported behavior is never silently converted into PTY input.
static ACTIONS: &[ActionSchema] = &[
    action!("ignore", [], None, InputWrite, Supported),
    action!("unbind", [], None, InputWrite, Supported),
    action!("csi", [], Text, InputWrite, Supported),
    action!("esc", [], Text, InputWrite, Supported),
    action!("text", [], Text, InputWrite, Supported),
    action!("cursor_key", [], Text, InputWrite, Supported),
    action!(
        "reset",
        ["reset_terminal"],
        None,
        TerminalMutation,
        Supported
    ),
    action!(
        "copy_to_clipboard",
        ["copy"],
        CopyMode,
        ClipboardWrite,
        Supported
    ),
    action!(
        "paste_from_clipboard",
        ["paste"],
        None,
        ClipboardRead,
        Supported
    ),
    action!("paste_from_selection", [], None, ClipboardRead, Adapted),
    action!(
        "copy_url_to_clipboard",
        [],
        None,
        ClipboardWrite,
        Unavailable
    ),
    action!(
        "copy_title_to_clipboard",
        [],
        None,
        ClipboardWrite,
        Unavailable
    ),
    action!("increase_font_size", [], Float, Configuration, Adapted),
    action!("decrease_font_size", [], Float, Configuration, Adapted),
    action!("reset_font_size", [], None, Configuration, Supported),
    action!("set_font_size", [], Float, Configuration, Adapted),
    action!("search", [], OptionalText, TerminalMutation, Unavailable),
    action!(
        "search_selection",
        ["search_from_selection"],
        None,
        TerminalMutation,
        Supported
    ),
    action!(
        "navigate_search",
        [],
        Direction,
        TerminalMutation,
        Supported
    ),
    action!("start_search", [], None, TerminalMutation, Supported),
    action!("end_search", [], None, TerminalMutation, Supported),
    action!("clear_screen", [], None, TerminalMutation, Supported),
    action!("clear_history", [], None, TerminalMutation, Supported),
    action!(
        "clear_screen_and_history",
        [],
        None,
        TerminalMutation,
        Supported
    ),
    action!("select_all", [], None, TerminalMutation, Supported),
    action!("scroll_to_top", [], None, TerminalMutation, Supported),
    action!("scroll_to_bottom", [], None, TerminalMutation, Supported),
    action!("scroll_to_selection", [], None, TerminalMutation, Supported),
    action!(
        "scroll_to_row",
        [],
        UnsignedInteger,
        TerminalMutation,
        Supported
    ),
    action!(
        "scroll_page_up",
        ["scroll_page:up"],
        None,
        TerminalMutation,
        Supported
    ),
    action!(
        "scroll_page_down",
        ["scroll_page:down"],
        None,
        TerminalMutation,
        Supported
    ),
    action!(
        "scroll_page_fractional",
        [],
        Float,
        TerminalMutation,
        Supported
    ),
    action!(
        "scroll_page_lines",
        [],
        Integer,
        TerminalMutation,
        Supported
    ),
    action!(
        "adjust_selection",
        [],
        Direction,
        TerminalMutation,
        Supported
    ),
    action!("jump_to_prompt", [], Integer, TerminalMutation, Supported),
    action!(
        "write_scrollback_file",
        [],
        PathDisposition,
        FilesystemWrite,
        Supported
    ),
    action!(
        "write_screen_file",
        ["export_visible_screen"],
        PathDisposition,
        FilesystemWrite,
        Supported
    ),
    action!(
        "write_selection_file",
        [],
        PathDisposition,
        FilesystemWrite,
        Supported
    ),
    action!(
        "new_window",
        ["create_window"],
        LaunchPolicy,
        WindowMutation,
        Supported
    ),
    action!(
        "new_tab",
        ["create_tab"],
        LaunchPolicy,
        WindowMutation,
        Supported
    ),
    action!("previous_tab", [], None, WindowMutation, Supported),
    action!("next_tab", [], None, WindowMutation, Supported),
    action!("last_tab", [], None, WindowMutation, Supported),
    action!(
        "goto_tab",
        ["select_tab"],
        Integer,
        WindowMutation,
        Supported
    ),
    action!("move_tab", [], Direction, WindowMutation, Supported),
    action!("toggle_tab_overview", [], None, WindowMutation, Unavailable),
    action!(
        "prompt_surface_title",
        [],
        None,
        WindowMutation,
        Unavailable
    ),
    action!("prompt_tab_title", [], None, WindowMutation, Unavailable),
    action!("set_surface_title", [], Text, WindowMutation, Unavailable),
    action!("set_tab_title", [], Text, WindowMutation, Unavailable),
    action!("new_split", [], Direction, LayoutMutation, Supported),
    action!(
        "goto_split",
        ["focus_split"],
        Direction,
        LayoutMutation,
        Supported
    ),
    action!("goto_window", [], Integer, WindowMutation, Unavailable),
    action!("toggle_split_zoom", [], None, LayoutMutation, Supported),
    action!("toggle_readonly", [], None, TerminalMutation, Unavailable),
    action!("resize_split", [], Resize, LayoutMutation, Supported),
    action!("equalize_splits", [], None, LayoutMutation, Supported),
    action!("reset_window_size", [], None, WindowMutation, Unavailable),
    action!(
        "inspector",
        ["toggle_inspector"],
        Toggle,
        Inspector,
        Supported
    ),
    action!("show_gtk_inspector", [], None, Inspector, Unavailable),
    action!(
        "show_on_screen_keyboard",
        [],
        None,
        WindowMutation,
        Unavailable
    ),
    action!(
        "open_config",
        ["open_config_editor"],
        None,
        Configuration,
        Supported
    ),
    action!("reload_config", [], None, Configuration, Supported),
    action!("close_surface", [], None, WindowMutation, Supported),
    action!("close_tab", [], TabTarget, WindowMutation, Supported),
    action!("close_window", [], None, WindowMutation, Supported),
    action!(
        "close_all_windows",
        [],
        None,
        WindowMutation,
        DeprecatedUnsafe
    ),
    action!("toggle_maximize", [], None, WindowMutation, Supported),
    action!("toggle_fullscreen", [], None, WindowMutation, Supported),
    action!(
        "toggle_window_decorations",
        [],
        None,
        WindowMutation,
        Unavailable
    ),
    action!(
        "toggle_window_float_on_top",
        [],
        None,
        WindowMutation,
        Unavailable
    ),
    action!("toggle_secure_input", [], None, Configuration, Unavailable),
    action!(
        "toggle_mouse_reporting",
        [],
        None,
        TerminalMutation,
        Unavailable
    ),
    action!(
        "toggle_command_palette",
        ["open_command_palette"],
        None,
        WindowMutation,
        Supported
    ),
    action!(
        "toggle_quick_terminal",
        ["toggle_quake"],
        None,
        WindowMutation,
        Adapted
    ),
    action!("toggle_visibility", [], None, WindowMutation, Unavailable),
    action!(
        "toggle_background_opacity",
        [],
        None,
        WindowMutation,
        Unavailable
    ),
    action!("check_for_updates", [], None, Configuration, Unavailable),
    action!("undo", ["undo_topology"], None, TopologyLifecycle, Adapted),
    action!("redo", ["redo_topology"], None, TopologyLifecycle, Adapted),
    action!("end_key_sequence", [], None, InputWrite, Supported),
    action!("activate_key_table", [], Table, InputWrite, Supported),
    action!("activate_key_table_once", [], Table, InputWrite, Supported),
    action!("deactivate_key_table", [], None, InputWrite, Supported),
    action!("deactivate_all_key_tables", [], None, InputWrite, Supported),
    action!("quit", [], None, WindowMutation, Supported),
    action!("crash", [], None, DeliberateCrash, DeprecatedUnsafe),
];

pub fn action_schemas() -> &'static [ActionSchema] {
    ACTIONS
}

pub fn resolve_action(value: &str) -> Option<&'static ActionSchema> {
    let normalized = value.trim().to_ascii_lowercase().replace('-', "_");
    ACTIONS.iter().find(|schema| {
        schema.id == normalized || schema.aliases.iter().any(|alias| *alias == normalized)
    })
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ActionInvocation {
    pub id: ActionId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parameter: Option<String>,
}

impl fmt::Display for ActionInvocation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.id.fmt(formatter)?;
        if let Some(parameter) = &self.parameter {
            formatter.write_str(":")?;
            formatter.write_str(parameter)?;
        }
        Ok(())
    }
}

impl ActionInvocation {
    pub fn new(id: &str, parameter: Option<String>) -> Result<Self, &'static str> {
        let schema = resolve_action(id).ok_or("unknown action")?;
        validate_parameter(schema.parameter, parameter.as_deref())?;
        Ok(Self {
            id: ActionId::new(schema.id)?,
            parameter,
        })
    }

    pub fn schema(&self) -> &'static ActionSchema {
        resolve_action(self.id.as_str()).expect("validated action IDs remain registered")
    }
}

fn validate_parameter(
    kind: ParameterKind,
    value: Option<&str>,
) -> Result<(), &'static str> {
    if value.is_some_and(|value| value.len() > MAX_PARAMETER_BYTES) {
        return Err("action parameter length is invalid");
    }
    if value.is_some_and(str::is_empty) && !matches!(kind, ParameterKind::OptionalText) {
        return Err("action parameter is empty");
    }
    match kind {
        ParameterKind::None if value.is_some() => {
            Err("action does not accept a parameter")
        }
        ParameterKind::None | ParameterKind::OptionalText => Ok(()),
        ParameterKind::Text => value.map(|_| ()).ok_or("action requires a parameter"),
        ParameterKind::Integer => value
            .ok_or("action requires an integer")?
            .parse::<i32>()
            .map(|_| ())
            .map_err(|_| "action parameter is not an integer"),
        ParameterKind::UnsignedInteger => value
            .ok_or("action requires an unsigned integer")?
            .parse::<usize>()
            .map(|_| ())
            .map_err(|_| "action parameter is not an unsigned integer"),
        ParameterKind::Float => value
            .ok_or("action requires a finite number")?
            .parse::<f32>()
            .map_err(|_| "action parameter is not a number")
            .and_then(|number| {
                number
                    .is_finite()
                    .then_some(())
                    .ok_or("action parameter is not finite")
            }),
        ParameterKind::Direction => {
            let value = value.ok_or("action requires a direction")?;
            const DIRECTIONS: &[&str] = &[
                "left",
                "right",
                "up",
                "down",
                "previous",
                "next",
                "start",
                "end",
                "home",
                "page_up",
                "page_down",
                "word_left",
                "word_right",
                "line_start",
                "line_end",
                "above",
                "below",
                "auto",
            ];
            DIRECTIONS
                .contains(&value)
                .then_some(())
                .ok_or("unknown direction")
        }
        ParameterKind::Table => {
            validate_identifier(value.ok_or("action requires a key table")?)
        }
        ParameterKind::PathDisposition => {
            let value = value.ok_or("file action requires a disposition")?;
            let mut parts = value.split(',');
            let disposition = parts.next().unwrap_or_default();
            if !matches!(disposition, "copy" | "paste" | "open") {
                return Err("unknown file disposition");
            }
            if let Some(format) = parts.next() {
                if !matches!(format, "plain" | "escape") || parts.next().is_some() {
                    return Err("unknown file format");
                }
            }
            Ok(())
        }
        ParameterKind::LaunchPolicy => {
            if let Some(value) = value {
                matches!(value, "inherit" | "default" | "cwd")
                    .then_some(())
                    .ok_or("unknown launch policy")
            } else {
                Ok(())
            }
        }
        ParameterKind::Toggle => {
            if let Some(value) = value {
                matches!(value, "toggle" | "show" | "hide" | "on" | "off")
                    .then_some(())
                    .ok_or("unknown toggle value")
            } else {
                Ok(())
            }
        }
        ParameterKind::TabTarget => {
            if let Some(value) = value {
                matches!(value, "this" | "other" | "right")
                    .then_some(())
                    .ok_or("unknown tab target")
            } else {
                Ok(())
            }
        }
        ParameterKind::Resize => {
            let value = value.ok_or("resize requires direction and amount")?;
            let (direction, amount) = value
                .split_once(',')
                .ok_or("resize requires direction and amount")?;
            validate_parameter(ParameterKind::Direction, Some(direction))?;
            let amount = amount.parse::<u16>().map_err(|_| "invalid resize amount")?;
            (1..=1_000)
                .contains(&amount)
                .then_some(())
                .ok_or("resize amount is out of range")
        }
        ParameterKind::CopyMode => {
            if let Some(value) = value {
                matches!(value, "mixed" | "plain" | "selection")
                    .then_some(())
                    .ok_or("unknown clipboard copy mode")
            } else {
                Ok(())
            }
        }
    }
}

fn validate_identifier(value: &str) -> Result<(), &'static str> {
    if value.is_empty()
        || value.len() > MAX_IDENTIFIER_BYTES
        || !value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.')
        })
    {
        return Err("identifier is invalid");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aliases_resolve_to_stable_ids() {
        let action = ActionInvocation::new("copy", None).unwrap();
        assert_eq!(action.id.as_str(), "copy_to_clipboard");
    }

    #[test]
    fn parameters_are_schema_checked_and_bounded() {
        assert!(ActionInvocation::new("quit", Some("now".into())).is_err());
        assert!(ActionInvocation::new("text", None).is_err());
        assert!(ActionInvocation::new("resize_split", Some("left,0".into())).is_err());
        assert!(ActionInvocation::new("resize_split", Some("left,10".into())).is_ok());
        assert!(
            ActionInvocation::new("text", Some("x".repeat(MAX_PARAMETER_BYTES + 1)))
                .is_err()
        );
    }

    #[test]
    fn ghostty_numeric_parameters_preserve_float_and_unsigned_contracts() {
        for action in ["increase_font_size", "decrease_font_size", "set_font_size"] {
            let invocation = ActionInvocation::new(action, Some("13.5".into()))
                .expect("finite fractional font size");
            assert_eq!(invocation.schema().parameter, ParameterKind::Float);
            assert_eq!(invocation.schema().support, SupportLevel::Adapted);
        }
        for invalid in ["NaN", "inf", "-inf"] {
            assert!(ActionInvocation::new("set_font_size", Some(invalid.into())).is_err());
            assert!(ActionInvocation::new(
                "scroll_page_fractional",
                Some(invalid.into())
            )
            .is_err());
        }

        let row = ActionInvocation::new("scroll_to_row", Some("42".into()))
            .expect("absolute non-negative row");
        assert_eq!(row.schema().parameter, ParameterKind::UnsignedInteger);
        assert_eq!(row.schema().support, SupportLevel::Supported);
        assert!(ActionInvocation::new("scroll_to_row", Some("-1".into())).is_err());

        let fractional =
            ActionInvocation::new("scroll_page_fractional", Some("-1.5".into()))
                .expect("finite fractional page");
        assert_eq!(fractional.schema().parameter, ParameterKind::Float);
        assert_eq!(fractional.schema().support, SupportLevel::Supported);
    }

    #[test]
    fn exact_upstream_catalog_is_registered_and_dangerous_actions_fail_closed() {
        let fixture = include_str!("../fixtures/ghostty/1.3.1/linux/actions.txt");
        let actions = fixture.lines().collect::<Vec<_>>();
        assert_eq!(actions.len(), 85);
        for action in actions {
            assert!(
                resolve_action(action).is_some(),
                "missing upstream action: {action}"
            );
        }
        assert_eq!(
            resolve_action("close_all_windows").unwrap().support,
            SupportLevel::DeprecatedUnsafe
        );
        assert_eq!(
            resolve_action("crash").unwrap().support,
            SupportLevel::DeprecatedUnsafe
        );
    }
}
