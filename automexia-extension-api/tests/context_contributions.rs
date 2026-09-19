use std::io::{self, Read};

use automexia_extension_api::{ContextContribution, ContractError, MAX_STATUS_SEGMENTS};

// Literal v1 wire data is independent of the production serializer.
const SEGMENT: &str = r#"{"version":1,"id":"scope","label":"example","accessibility_label":"Example scope","role":"environment","icon":"environment","priority":10,"freshness":"current","observed_at_ms":7,"details_action":null}"#;
const PREFIX: &str = r#"{"version":1,"extension_id":"devops","session_id":1,"capsule_revision":2,"source_revision":3,"generated_at_ms":7,"freshness":"current","segments":["#;

fn wire(count: usize) -> String {
    let items = (0..count)
        .map(|index| SEGMENT.replace("\"scope\"", &format!("\"scope-{index}\"")))
        .collect::<Vec<_>>();
    format!("{PREFIX}{}]}}", items.join(","))
}

struct ObservedReader<'a> {
    remaining: &'a [u8],
    consumed: usize,
}

impl Read for ObservedReader<'_> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        // One byte per read also covers fragmented input; consumption is the
        // oracle, not the decoder's eventual error after reading a huge tail.
        let count = buffer.len().min(self.remaining.len()).min(1);
        buffer[..count].copy_from_slice(&self.remaining[..count]);
        self.remaining = &self.remaining[count..];
        self.consumed += count;
        Ok(count)
    }
}

#[test]
fn context_wire_preserves_exact_count_boundaries_and_values() {
    assert_eq!(MAX_STATUS_SEGMENTS, 64, "v1 compatibility ceiling changed");
    for count in [0, 1, 32, 63, 64] {
        let source = wire(count);
        let value: ContextContribution = serde_json::from_str(&source).unwrap();
        assert_eq!(value.segments.len(), count);
        assert_eq!(value.capsule_revision, 2);
        assert_eq!(value.source_revision, 3);
        assert_eq!(value.generated_at_ms, 7);
        for (index, segment) in value.segments.iter().enumerate() {
            assert_eq!(segment.id.as_str(), format!("scope-{index}"));
            assert_eq!(segment.label.as_str(), "example");
            assert_eq!(segment.accessibility_label.as_str(), "Example scope");
            assert_eq!(segment.observed_at_ms, 7);
        }
        assert_eq!(
            serde_json::to_value(&value).unwrap(),
            serde_json::from_str::<serde_json::Value>(&source).unwrap()
        );
    }
}

#[test]
fn context_wire_rejects_before_decoding_an_excess_element() {
    let boundary = wire(64);
    let prefix = format!("{},", boundary.strip_suffix("]}").unwrap());
    for tail in [
        SEGMENT.to_string(),
        "null".into(),
        "[".into(),
        format!("\"{}\"", "fictional-tail-".repeat(100_000)),
        vec![SEGMENT; 1024].join(","),
    ] {
        let source = format!("{prefix}{tail}]}}");
        let mut reader = ObservedReader {
            remaining: source.as_bytes(),
            consumed: 0,
        };
        let error = serde_json::from_reader::<_, ContextContribution>(&mut reader)
            .expect_err("over-count context accepted");
        assert!(
            reader.consumed <= prefix.len() + 1,
            "context decoder consumed excess payload bytes"
        );
        assert!(error.to_string().contains("status segments"));
    }
}

#[test]
fn context_wire_still_validates_every_in_limit_element_and_envelope() {
    for source in [
        wire(1).replace("\"version\":1", "\"version\":2"),
        wire(1).replace("\"label\":\"example\"", "\"label\":null"),
        format!("{PREFIX}null]}}"),
        wire(1).trim_end_matches('}').to_string(),
        wire(0).replace("\"segments\":[]", "\"segments\":[],\"extra\":true"),
        wire(0).replace("\"segments\":[]", "\"segments\":[],\"segments\":[]"),
    ] {
        assert!(serde_json::from_str::<ContextContribution>(&source).is_err());
    }
    let source = wire(64);
    let mut reader = ObservedReader {
        remaining: source.as_bytes(),
        consumed: 0,
    };
    let value: ContextContribution = serde_json::from_reader(&mut reader).unwrap();
    assert_eq!(value.segments.len(), 64);
    assert_eq!(reader.consumed, source.len());
}

#[test]
fn context_typed_constructor_retains_the_same_limit() {
    for count in [0, 1, 63, 64, 65] {
        let mut value: ContextContribution =
            serde_json::from_str(&wire(count.min(64))).unwrap();
        if count == 65 {
            value.segments.push(serde_json::from_str(SEGMENT).unwrap());
        }
        let result = ContextContribution::new(
            value.extension_id,
            value.session_id,
            value.capsule_revision,
            value.source_revision,
            value.freshness,
            value.segments,
        );
        if count <= 64 {
            assert_eq!(result.unwrap().segments.len(), count);
        } else {
            assert_eq!(
                result.unwrap_err(),
                ContractError::TooMany {
                    field: "status segments",
                    maximum: 64
                }
            );
        }
    }
}
