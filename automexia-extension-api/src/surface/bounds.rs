use std::{fmt, marker::PhantomData};

use serde::{
    de::{self, DeserializeSeed, SeqAccess, Visitor},
    Deserialize, Deserializer, Serialize,
};

use crate::ContractError;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct Cost {
    pub cells: usize,
    pub text: usize,
}

pub(super) trait Charged {
    fn cost(&self) -> Cost;
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub(super) struct List<T, const N: usize, const C: usize, const B: usize>(Box<[T]>);

impl<T: Charged, const N: usize, const C: usize, const B: usize> List<T, N, C, B> {
    pub fn new(items: Vec<T>) -> Result<Self, ContractError> {
        let mut cost = Cost::default();
        if items.len() > N {
            return Err(ContractError::TooMany {
                field: "surface items",
                maximum: N,
            });
        }
        for item in &items {
            Self::charge(&mut cost, item)?;
        }
        Ok(Self(items.into_boxed_slice()))
    }

    fn charge(total: &mut Cost, item: &T) -> Result<(), ContractError> {
        let next = item.cost();
        // The limits bound arithmetic as well as retained data, on 32-bit targets too.
        if next.cells > C.saturating_sub(total.cells)
            || next.text > B.saturating_sub(total.text)
        {
            return Err(ContractError::InvalidValue("surface aggregate budget"));
        }
        total.cells += next.cells;
        total.text += next.text;
        Ok(())
    }

    pub fn as_slice(&self) -> &[T] {
        &self.0
    }
    pub fn cost(&self) -> Cost {
        self.0.iter().fold(Cost::default(), |mut sum, item| {
            let next = item.cost();
            sum.cells += next.cells;
            sum.text += next.text;
            sum
        })
    }
}

// At the count boundary reject the next element without decoding or allocating it.
struct RejectElement;
impl<'de> DeserializeSeed<'de> for RejectElement {
    type Value = ();
    fn deserialize<D: Deserializer<'de>>(self, _: D) -> Result<(), D::Error> {
        Err(de::Error::custom("surface item limit"))
    }
}

impl<
        'de,
        T: Deserialize<'de> + Charged,
        const N: usize,
        const C: usize,
        const B: usize,
    > Deserialize<'de> for List<T, N, C, B>
{
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct ListVisitor<T, const N: usize, const C: usize, const B: usize>(
            PhantomData<T>,
        );
        impl<
                'de,
                T: Deserialize<'de> + Charged,
                const N: usize,
                const C: usize,
                const B: usize,
            > Visitor<'de> for ListVisitor<T, N, C, B>
        {
            type Value = List<T, N, C, B>;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a bounded surface sequence")
            }
            fn visit_seq<A: SeqAccess<'de>>(
                self,
                mut seq: A,
            ) -> Result<Self::Value, A::Error> {
                let mut items = Vec::new();
                let mut cost = Cost::default();
                // A hostile deserializer's size hint must not reserve memory for us.
                while items.len() < N {
                    let Some(item) = seq.next_element::<T>()? else {
                        return Ok(List(items.into_boxed_slice()));
                    };
                    List::<T, N, C, B>::charge(&mut cost, &item)
                        .map_err(de::Error::custom)?;
                    items.push(item);
                }
                seq.next_element_seed(RejectElement)?;
                Ok(List(items.into_boxed_slice()))
            }
        }
        deserializer.deserialize_seq(ListVisitor::<T, N, C, B>(PhantomData))
    }
}

pub(super) fn display_text(value: &str, max: usize) -> Result<(), ContractError> {
    if value.len() > max {
        return Err(ContractError::TooLong {
            field: "surface text",
            maximum: max,
        });
    }
    if value.chars().any(|c| c.is_control() || matches!(c, '\u{061c}' | '\u{200e}' | '\u{200f}' | '\u{2028}'..='\u{202e}' | '\u{2066}'..='\u{2069}')) {
        return Err(ContractError::InvalidCharacter("surface text"));
    }
    Ok(())
}

pub(super) fn text<'de, D: Deserializer<'de>, const N: usize>(
    deserializer: D,
) -> Result<Box<str>, D::Error> {
    struct TextVisitor<const N: usize>;
    impl<'de, const N: usize> Visitor<'de> for TextVisitor<N> {
        type Value = Box<str>;
        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.write_str("bounded display text")
        }
        fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
            display_text(value, N).map_err(E::custom)?;
            Ok(value.into())
        }
        fn visit_string<E: de::Error>(self, value: String) -> Result<Self::Value, E> {
            display_text(&value, N).map_err(E::custom)?;
            Ok(value.into_boxed_str())
        }
    }
    deserializer.deserialize_str(TextVisitor::<N>)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::de::value::{Error, SeqDeserializer};

    #[derive(Debug)]
    struct Item(u64);
    impl Charged for Item {
        fn cost(&self) -> Cost {
            Cost {
                cells: self.0 as usize,
                text: 0,
            }
        }
    }
    impl<'de> Deserialize<'de> for Item {
        fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
            let value = u64::deserialize(d)?;
            // The third value is deliberately poisonous: the count guard must
            // reject it before invoking the element's decoder.
            assert_ne!(value, 3, "over-count element was decoded");
            Ok(Self(value))
        }
    }
    struct HostileHint {
        next: u64,
        end: u64,
    }
    impl Iterator for HostileHint {
        type Item = u64;
        fn next(&mut self) -> Option<u64> {
            if self.next > self.end {
                None
            } else {
                let value = self.next;
                self.next += 1;
                Some(value)
            }
        }
        fn size_hint(&self) -> (usize, Option<usize>) {
            (usize::MAX, Some(usize::MAX))
        }
    }
    #[test]
    fn hostile_size_hint_never_drives_allocation_and_overcount_never_decodes_item() {
        for (end, succeeds) in [(0, true), (1, true), (2, true), (3, false)] {
            let decoder = SeqDeserializer::<_, Error>::new(HostileHint { next: 1, end });
            assert_eq!(
                List::<Item, 2, 3, 0>::deserialize(decoder).is_ok(),
                succeeds
            );
        }
    }
    #[test]
    fn aggregate_budget_is_enforced_during_iteration() {
        let decoder = SeqDeserializer::<_, Error>::new([2_u64, 2].into_iter());
        assert!(List::<Item, 8, 3, 0>::deserialize(decoder).is_err());
    }
}
