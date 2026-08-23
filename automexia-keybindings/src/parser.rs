use crate::{
    ActionInvocation, BindingOperation, BindingOrigin, BindingPolicy, BindingScope,
    BindingSpec, Consumption, KeyAtom, ModePredicate, Modifiers, NamedKey, Trigger,
    MAX_CHAIN_ACTIONS, MAX_PARAMETER_BYTES,
};
use serde::{Deserialize, Serialize};

const MAX_BINDING_LINE_BYTES: usize = MAX_PARAMETER_BYTES + 512;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParseError {
    LineTooLong,
    InvalidFormat,
    InvalidFlag,
    InvalidTable,
    InvalidTrigger,
    InvalidAction,
    UnexpectedChain,
    ChainTooLong,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ParsedLine {
    Binding(BindingSpec),
    Chain(ActionInvocation),
}

pub fn parse_binding_line(
    input: &str,
    origin: BindingOrigin,
) -> Result<ParsedLine, ParseError> {
    let input = input.trim();
    if input.is_empty() || input.len() > MAX_BINDING_LINE_BYTES {
        return Err(if input.is_empty() {
            ParseError::InvalidFormat
        } else {
            ParseError::LineTooLong
        });
    }

    let delimiter = binding_delimiter(input).ok_or(ParseError::InvalidFormat)?;
    let mut trigger_text = &input[..delimiter];
    let action_text = &input[delimiter + 1..];
    if trigger_text == "chain" {
        return parse_action(action_text)
            .map(ParsedLine::Chain)
            .map_err(|_| ParseError::InvalidAction);
    }

    let mut scope = BindingScope::FocusedSurface;
    let mut consumption = Consumption::Consumed;
    let mut performable = false;
    while let Some((prefix, rest)) = trigger_text.split_once(':') {
        match prefix {
            "all" => scope = BindingScope::AllSurfaces,
            "global" => scope = BindingScope::OperatingSystemGlobal,
            "unconsumed" => consumption = Consumption::Unconsumed,
            "performable" => performable = true,
            _ => break,
        }
        trigger_text = rest;
    }

    let (table, trigger_text) = match trigger_text.split_once('/') {
        Some((table, trigger)) => {
            validate_table(table)?;
            (Some(table.to_string()), trigger)
        }
        None => (None, trigger_text),
    };
    let sequence = trigger_text
        .split('>')
        .map(parse_trigger)
        .collect::<Result<Vec<_>, _>>()?;

    let operation = if action_text.trim().eq_ignore_ascii_case("unbind") {
        BindingOperation::Unbind
    } else {
        BindingOperation::Bind(vec![
            parse_action(action_text).map_err(|_| ParseError::InvalidAction)?
        ])
    };
    let spec = BindingSpec {
        sequence,
        table,
        predicate: ModePredicate::default(),
        scope,
        origin,
        priority: 0,
        policy: BindingPolicy {
            consumption,
            performable,
        },
        operation,
    };
    spec.validate().map_err(|_| ParseError::InvalidFormat)?;
    Ok(ParsedLine::Binding(spec))
}

pub fn parse_binding_lines<'a>(
    lines: impl IntoIterator<Item = &'a str>,
    origin: BindingOrigin,
) -> Result<Vec<BindingSpec>, ParseError> {
    let mut bindings: Vec<BindingSpec> = Vec::new();
    for line in lines {
        match parse_binding_line(line, origin)? {
            ParsedLine::Binding(binding) => bindings.push(binding),
            ParsedLine::Chain(action) => {
                let Some(previous) = bindings.last_mut() else {
                    return Err(ParseError::UnexpectedChain);
                };
                let BindingOperation::Bind(actions) = &mut previous.operation else {
                    return Err(ParseError::UnexpectedChain);
                };
                if actions.len() >= MAX_CHAIN_ACTIONS {
                    return Err(ParseError::ChainTooLong);
                }
                actions.push(action);
            }
        }
    }
    Ok(bindings)
}

fn binding_delimiter(input: &str) -> Option<usize> {
    let bytes = input.as_bytes();
    for (index, byte) in bytes.iter().enumerate() {
        if *byte != b'=' {
            continue;
        }
        if bytes
            .get(index + 1)
            .is_some_and(|next| matches!(next, b'+' | b'='))
        {
            continue;
        }
        return Some(index);
    }
    None
}

fn validate_table(table: &str) -> Result<(), ParseError> {
    if table.is_empty()
        || !table.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.')
        })
    {
        return Err(ParseError::InvalidTable);
    }
    Ok(())
}

fn parse_trigger(input: &str) -> Result<Trigger, ParseError> {
    let input = input.trim();
    if input.is_empty() {
        return Err(ParseError::InvalidTrigger);
    }

    // A literal plus key is printed by Ghostty as the final `+`, for example
    // `ctrl++`. Split from the right so it remains an atom rather than an empty
    // modifier token.
    let (modifier_text, key_text) = if let Some(prefix) = input.strip_suffix('+') {
        (prefix.strip_suffix('+').unwrap_or(prefix), "+")
    } else if let Some((modifiers, key)) = input.rsplit_once('+') {
        (modifiers, key)
    } else {
        ("", input)
    };
    let modifiers = if modifier_text.is_empty() {
        Modifiers::empty()
    } else {
        Modifiers::from_names(modifier_text.split('+'))
            .map_err(|_| ParseError::InvalidFlag)?
    };
    Trigger::new(parse_key(key_text)?, modifiers).map_err(|_| ParseError::InvalidTrigger)
}

fn parse_key(token: &str) -> Result<KeyAtom, ParseError> {
    let normalized = token.trim().to_ascii_lowercase().replace('-', "_");
    if normalized == "catch_all" {
        return Ok(KeyAtom::CatchAll);
    }
    if let Some(physical) = token.strip_prefix("physical:") {
        return Ok(KeyAtom::Physical(physical.to_string()));
    }
    if let Some(named) = NamedKey::parse(&normalized) {
        return Ok(KeyAtom::Named(named));
    }
    if let Some(digit) = normalized.strip_prefix("digit_") {
        if digit.len() == 1 && digit.as_bytes()[0].is_ascii_digit() {
            return Ok(KeyAtom::Physical(format!("Digit{digit}")));
        }
    }
    let logical = match normalized.as_str() {
        "plus" => "+".to_string(),
        "minus" => "-".to_string(),
        _ => normalized,
    };
    if logical.is_empty()
        || logical.len() > crate::MAX_IDENTIFIER_BYTES
        || logical.chars().any(char::is_control)
    {
        return Err(ParseError::InvalidTrigger);
    }
    Ok(KeyAtom::Logical(logical))
}
fn parse_action(input: &str) -> Result<ActionInvocation, &'static str> {
    let input = input.trim();
    let (id, parameter) = match input.split_once(':') {
        Some((id, parameter)) => (id, Some(parameter.to_string())),
        None => (input, None),
    };
    ActionInvocation::new(id, parameter)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BindingOperation, BindingScope, Consumption, KeyAtom};

    #[test]
    fn ghostty_flags_tables_sequences_and_chains_parse_without_raw_evaluation() {
        let bindings = parse_binding_lines(
            [
                "unconsumed:performable:nav/ctrl+a>physical:KeyB=activate_key_table:edit",
                "chain=toggle_split_zoom",
            ],
            BindingOrigin::User,
        )
        .unwrap();
        assert_eq!(bindings.len(), 1);
        assert_eq!(bindings[0].table.as_deref(), Some("nav"));
        assert_eq!(bindings[0].policy.consumption, Consumption::Unconsumed);
        assert!(bindings[0].policy.performable);
        assert!(matches!(bindings[0].sequence[1].key, KeyAtom::Physical(_)));
        assert!(
            matches!(&bindings[0].operation, BindingOperation::Bind(actions) if actions.len() == 2)
        );
    }

    #[test]
    fn all_and_global_sequences_fail_closed() {
        assert_eq!(
            parse_binding_line("global:ctrl+a>ctrl+b=quit", BindingOrigin::User),
            Err(ParseError::InvalidFormat)
        );
        assert_eq!(
            parse_binding_line("all:ctrl+a>ctrl+b=quit", BindingOrigin::User),
            Err(ParseError::InvalidFormat)
        );
    }

    #[test]
    fn equals_key_uses_the_second_delimiter() {
        let ParsedLine::Binding(binding) =
            parse_binding_line("ctrl+==reset", BindingOrigin::User).unwrap()
        else {
            panic!("expected binding");
        };
        assert_eq!(binding.scope, BindingScope::FocusedSurface);
        assert_eq!(binding.sequence[0].key, KeyAtom::Logical("=".into()));
    }

    #[test]
    fn malformed_or_oversized_lines_are_rejected() {
        assert!(parse_binding_line("", BindingOrigin::User).is_err());
        assert!(parse_binding_line(
            &"x".repeat(MAX_BINDING_LINE_BYTES + 1),
            BindingOrigin::User
        )
        .is_err());
        assert!(
            parse_binding_line("ctrl+a=unknown_action", BindingOrigin::User).is_err()
        );
    }
}
