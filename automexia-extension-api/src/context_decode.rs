//! Count admission for the existing v1 context wire collection.
use std::fmt;

use serde::de::{self, DeserializeSeed, Deserializer, SeqAccess, Visitor};

use crate::{ContractError, StatusSegment, MAX_STATUS_SEGMENTS};

struct RejectExcess;

impl<'de> DeserializeSeed<'de> for RejectExcess {
    type Value = ();

    fn deserialize<D: Deserializer<'de>>(self, _: D) -> Result<(), D::Error> {
        // Reject without visiting the next object, even if its payload is huge
        // or malformed. Transport byte/deadline limits remain a separate duty.
        Err(de::Error::custom(ContractError::TooMany {
            field: "status segments",
            maximum: MAX_STATUS_SEGMENTS,
        }))
    }
}

pub(super) fn segments<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<StatusSegment>, D::Error> {
    struct SegmentsVisitor;

    impl<'de> Visitor<'de> for SegmentsVisitor {
        type Value = Vec<StatusSegment>;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("a bounded context segment sequence")
        }

        fn visit_seq<A: SeqAccess<'de>>(
            self,
            mut sequence: A,
        ) -> Result<Self::Value, A::Error> {
            // Never reserve memory using a producer-controlled size hint.
            let mut items = Vec::new();
            while items.len() < MAX_STATUS_SEGMENTS {
                let Some(item) = sequence.next_element()? else {
                    return Ok(items);
                };
                items.push(item);
            }
            sequence.next_element_seed(RejectExcess)?;
            Ok(items)
        }
    }

    deserializer.deserialize_seq(SegmentsVisitor)
}
