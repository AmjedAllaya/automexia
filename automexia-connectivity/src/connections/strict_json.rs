//! JSON decoding that rejects duplicate object member names at every depth.
//!
//! Serde's ordinary struct decoder rejects unknown fields but, like many JSON
//! receivers, accepts a repeated known field and keeps one value. Connection
//! records cross trust and persistence boundaries, so ambiguous objects are
//! rejected before they can reach a typed model.

use std::{collections::HashSet, fmt};

use serde::{
    de::{DeserializeOwned, Error as _, MapAccess, SeqAccess, Visitor},
    Deserialize, Deserializer,
};
use serde_json::{Map, Number, Value};

struct UniqueJsonValue(Value);

impl<'de> Deserialize<'de> for UniqueJsonValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(UniqueJsonVisitor)
    }
}

struct UniqueJsonVisitor;

impl<'de> Visitor<'de> for UniqueJsonVisitor {
    type Value = UniqueJsonValue;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("JSON without duplicate object member names")
    }

    fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E> {
        Ok(UniqueJsonValue(Value::Bool(value)))
    }

    fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E> {
        Ok(UniqueJsonValue(Value::Number(Number::from(value))))
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E> {
        Ok(UniqueJsonValue(Value::Number(Number::from(value))))
    }

    fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Number::from_f64(value)
            .map(Value::Number)
            .map(UniqueJsonValue)
            .ok_or_else(|| E::custom("non-finite JSON number"))
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(UniqueJsonValue(Value::String(value.to_owned())))
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E> {
        Ok(UniqueJsonValue(Value::String(value)))
    }

    fn visit_none<E>(self) -> Result<Self::Value, E> {
        Ok(UniqueJsonValue(Value::Null))
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E> {
        Ok(UniqueJsonValue(Value::Null))
    }

    fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        UniqueJsonValue::deserialize(deserializer)
    }

    fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut values = Vec::with_capacity(sequence.size_hint().unwrap_or(0));
        while let Some(value) = sequence.next_element::<UniqueJsonValue>()? {
            values.push(value.0);
        }
        Ok(UniqueJsonValue(Value::Array(values)))
    }

    fn visit_map<A>(self, mut object: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let capacity = object.size_hint().unwrap_or(0);
        let mut names = HashSet::with_capacity(capacity);
        let mut values = Map::with_capacity(capacity);
        while let Some(name) = object.next_key::<String>()? {
            if !names.insert(name.clone()) {
                return Err(A::Error::custom("duplicate JSON object member name"));
            }
            let value = object.next_value::<UniqueJsonValue>()?;
            values.insert(name, value.0);
        }
        Ok(UniqueJsonValue(Value::Object(values)))
    }
}

#[doc(hidden)]
pub fn from_json_slice_without_duplicate_keys<T>(
    bytes: &[u8],
) -> Result<T, serde_json::Error>
where
    T: DeserializeOwned,
{
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    let value = UniqueJsonValue::deserialize(&mut deserializer)?;
    deserializer.end()?;
    serde_json::from_value(value.0)
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::from_json_slice_without_duplicate_keys;

    #[derive(Debug, Deserialize, PartialEq, Eq)]
    #[serde(deny_unknown_fields)]
    struct Fixture {
        name: String,
        nested: Nested,
    }

    #[derive(Debug, Deserialize, PartialEq, Eq)]
    #[serde(deny_unknown_fields)]
    struct Nested {
        enabled: bool,
    }

    #[test]
    fn rejects_literal_escaped_and_nested_duplicates() {
        assert!(from_json_slice_without_duplicate_keys::<Fixture>(
            br#"{"name":"one","name":"two","nested":{"enabled":true}}"#,
        )
        .is_err());
        assert!(from_json_slice_without_duplicate_keys::<Fixture>(
            br#"{"name":"one","\u006eame":"two","nested":{"enabled":true}}"#,
        )
        .is_err());
        assert!(from_json_slice_without_duplicate_keys::<Fixture>(
            br#"{"name":"one","nested":{"enabled":true,"enabled":false}}"#,
        )
        .is_err());
    }

    #[test]
    fn preserves_unambiguous_typed_decoding_and_rejects_trailing_data() {
        let decoded = from_json_slice_without_duplicate_keys::<Fixture>(
            br#"{"name":"one","nested":{"enabled":true}}"#,
        )
        .unwrap();
        assert_eq!(decoded.name, "one");
        assert!(decoded.nested.enabled);
        assert!(from_json_slice_without_duplicate_keys::<Fixture>(
            br#"{"name":"one","nested":{"enabled":true}} null"#,
        )
        .is_err());
    }
}
