use std::{collections::BTreeMap, fmt};

use serde::{
    de::{self, DeserializeOwned, Error as _, MapAccess, SeqAccess, Visitor},
    Deserialize, Deserializer,
};

use crate::Limits;

#[derive(Debug)]
pub enum StrictJsonError {
    SourceTooLarge { actual: usize, maximum: usize },
    Decode(serde_json::Error),
    TooDeep { maximum: usize },
    StringTooLong { maximum: usize },
}

impl fmt::Display for StrictJsonError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SourceTooLarge { actual, maximum } => {
                write!(
                    formatter,
                    "JSON source is {actual} bytes; limit is {maximum}"
                )
            }
            Self::Decode(error) => write!(formatter, "strict JSON rejected: {error}"),
            Self::TooDeep { maximum } => {
                write!(formatter, "JSON depth exceeds {maximum}")
            }
            Self::StringTooLong { maximum } => {
                write!(formatter, "JSON string exceeds {maximum} bytes")
            }
        }
    }
}

impl std::error::Error for StrictJsonError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Decode(error) => Some(error),
            _ => None,
        }
    }
}

#[derive(Debug)]
enum StrictValue {
    Null,
    Bool(bool),
    Number(serde_json::Number),
    String(String),
    Array(Vec<Self>),
    Object(BTreeMap<String, Self>),
}

impl StrictValue {
    fn into_json(self) -> serde_json::Value {
        match self {
            Self::Null => serde_json::Value::Null,
            Self::Bool(value) => serde_json::Value::Bool(value),
            Self::Number(value) => serde_json::Value::Number(value),
            Self::String(value) => serde_json::Value::String(value),
            Self::Array(values) => serde_json::Value::Array(
                values.into_iter().map(Self::into_json).collect(),
            ),
            Self::Object(values) => serde_json::Value::Object(
                values
                    .into_iter()
                    .map(|(key, value)| (key, value.into_json()))
                    .collect(),
            ),
        }
    }
}

struct StrictValueVisitor;

impl<'de> Visitor<'de> for StrictValueVisitor {
    type Value = StrictValue;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a JSON value without duplicate object keys")
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E> {
        Ok(StrictValue::Null)
    }

    fn visit_none<E>(self) -> Result<Self::Value, E> {
        Ok(StrictValue::Null)
    }

    fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E> {
        Ok(StrictValue::Bool(value))
    }

    fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E> {
        Ok(StrictValue::Number(value.into()))
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E> {
        Ok(StrictValue::Number(value.into()))
    }

    fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        serde_json::Number::from_f64(value)
            .map(StrictValue::Number)
            .ok_or_else(|| E::custom("non-finite JSON number"))
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(StrictValue::String(value.to_owned()))
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E> {
        Ok(StrictValue::String(value))
    }

    fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut values = Vec::with_capacity(sequence.size_hint().unwrap_or(0).min(1024));
        while let Some(value) = sequence.next_element::<StrictValue>()? {
            values.push(value);
        }
        Ok(StrictValue::Array(values))
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut values = BTreeMap::new();
        while let Some(key) = map.next_key::<String>()? {
            if values.contains_key(&key) {
                return Err(A::Error::custom(format!("duplicate JSON key {key:?}")));
            }
            values.insert(key, map.next_value::<StrictValue>()?);
        }
        Ok(StrictValue::Object(values))
    }
}

impl<'de> Deserialize<'de> for StrictValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(StrictValueVisitor)
    }
}

pub fn decode_strict_json<T: DeserializeOwned>(
    source: &[u8],
    maximum_bytes: usize,
) -> Result<T, StrictJsonError> {
    if source.len() > maximum_bytes {
        return Err(StrictJsonError::SourceTooLarge {
            actual: source.len(),
            maximum: maximum_bytes,
        });
    }
    let mut deserializer = serde_json::Deserializer::from_slice(source);
    let value =
        StrictValue::deserialize(&mut deserializer).map_err(StrictJsonError::Decode)?;
    deserializer.end().map_err(StrictJsonError::Decode)?;
    let value = value.into_json();
    validate_shape(&value, 1)?;
    serde_json::from_value(value).map_err(StrictJsonError::Decode)
}

fn validate_shape(
    value: &serde_json::Value,
    depth: usize,
) -> Result<(), StrictJsonError> {
    if depth > Limits::MANIFEST_DEPTH {
        return Err(StrictJsonError::TooDeep {
            maximum: Limits::MANIFEST_DEPTH,
        });
    }
    match value {
        serde_json::Value::String(value) => validate_string(value),
        serde_json::Value::Array(values) => {
            for value in values {
                validate_shape(value, depth + 1)?;
            }
            Ok(())
        }
        serde_json::Value::Object(values) => {
            for (key, value) in values {
                validate_string(key)?;
                validate_shape(value, depth + 1)?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

fn validate_string(value: &str) -> Result<(), StrictJsonError> {
    if value.len() > Limits::MANIFEST_STRING_BYTES {
        return Err(StrictJsonError::StringTooLong {
            maximum: Limits::MANIFEST_STRING_BYTES,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;

    #[derive(Debug, Deserialize, PartialEq)]
    #[serde(deny_unknown_fields)]
    struct Wire {
        id: String,
    }

    #[test]
    fn duplicate_unknown_trailing_and_oversized_inputs_fail_closed() {
        assert!(decode_strict_json::<Wire>(br#"{"id":"a","id":"b"}"#, 64).is_err());
        assert!(decode_strict_json::<Wire>(br#"{"id":"a","extra":1}"#, 64).is_err());
        assert!(decode_strict_json::<Wire>(br#"{"id":"a"} false"#, 64).is_err());
        assert!(matches!(
            decode_strict_json::<Wire>(br#"{"id":"a"}"#, 2),
            Err(StrictJsonError::SourceTooLarge { .. })
        ));
    }
}
