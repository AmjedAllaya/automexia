use automexia_ecosystem::*;
use serde_json::{json, Value};
const FIXTURE: &[u8] =
    include_bytes!("../../tests/fixtures/ecosystem/settings-metadata-v1.json");
fn manifest() -> EcosystemManifest {
    EcosystemManifest {
        schema_version: 1,
        publisher_id: "example.publisher".into(),
        extension_id: "example.inspect".into(),
        version: "1.0.0".into(),
        display_name: "Example".into(),
        description: String::new(),
        kind: ExtensionKind::Component,
        compatibility: Compatibility {
            sdk_major: 1,
            sdk_minor_minimum: 0,
            sdk_minor_maximum: 0,
        },
        world: WIT_WORLD.into(),
        imports: vec![],
        capabilities: vec![],
        action_pack_entry: None,
    }
}
fn fixture() -> Value {
    serde_json::from_slice(FIXTURE).unwrap()
}
fn decode(v: &Value) -> Result<ValidatedSettingsMetadata, SettingsMetadataError> {
    decode_settings_metadata(&serde_json::to_vec(v).unwrap(), &manifest())
}
fn empty_feature(id: &str) -> Value {
    json!({"id":id,"label":"Feature","description":"","default_enabled":true,"options":[]})
}
fn option(id: &str) -> Value {
    json!({"id":id,"label":"Option","description":"","definition":{"kind":"boolean","default":false}})
}
#[test]
fn literal_metadata_preserves_all_declared_kinds_and_defaults() {
    let parsed = decode_settings_metadata(FIXTURE, &manifest()).unwrap();
    let doc = parsed.document();
    assert_eq!(doc.publisher_id, "example.publisher");
    assert_eq!(doc.features.len(), 1);
    let f = &doc.features[0];
    assert_eq!(
        (f.id.as_str(), f.label.as_str(), f.default_enabled),
        ("summary", "Summary", true)
    );
    assert_eq!(
        f.options[0].definition,
        SettingsOptionDefinition::Boolean { default: false }
    );
    assert_eq!(
        f.options[1].definition,
        SettingsOptionDefinition::Choice {
            default: "full".into(),
            choices: vec![
                SettingsChoice {
                    value: "short".into(),
                    label: "Short".into()
                },
                SettingsChoice {
                    value: "full".into(),
                    label: "Full".into()
                }
            ]
        }
    );
    assert_eq!(
        f.options[2].definition,
        SettingsOptionDefinition::Integer {
            default: 5,
            min: 1,
            max: 9,
            step: 2
        }
    );
}
#[test]
fn metadata_identity_binds_each_verified_manifest_dimension() {
    for key in ["publisher_id", "extension_id", "extension_version"] {
        let mut v = fixture();
        v[key] = json!("different");
        assert_eq!(
            decode(&v),
            Err(SettingsMetadataError::IdentityMismatch),
            "{key}"
        );
    }
    let mut m = manifest();
    m.publisher_id = "a".repeat(128);
    m.extension_id = "e.1suffix".into();
    let mut v = fixture();
    v["publisher_id"] = json!(m.publisher_id);
    v["extension_id"] = json!(m.extension_id);
    assert!(
        decode_settings_metadata(&serde_json::to_vec(&v).unwrap(), &m).is_ok(),
        "valid v1 identities must not inherit UI ID restrictions"
    );
}
#[test]
fn metadata_schema_is_exact_and_unknown_fields_do_not_disappear() {
    let mut v = fixture();
    v["schema_version"] = json!(2);
    assert_eq!(decode(&v), Err(SettingsMetadataError::UnsupportedSchema));
    for target in [0, 1, 2, 3] {
        let mut v = fixture();
        let object = match target {
            0 => &mut v,
            1 => &mut v["features"][0],
            2 => &mut v["features"][0]["options"][0],
            _ => &mut v["features"][0]["options"][0]["definition"],
        };
        object["unknown"] = json!(true);
        assert_eq!(decode(&v), Err(SettingsMetadataError::MalformedJson));
    }
    let mut v = fixture();
    v["features"][0]["options"][0]["definition"]["kind"] = json!("action");
    assert_eq!(decode(&v), Err(SettingsMetadataError::MalformedJson));
}
#[test]
fn duplicate_json_members_are_rejected_before_projection() {
    let bytes = String::from_utf8(FIXTURE.to_vec()).unwrap().replacen(
        "\"schema_version\": 1",
        "\"schema_version\": 1, \"schema_version\": 1",
        1,
    );
    assert_eq!(
        decode_settings_metadata(bytes.as_bytes(), &manifest()),
        Err(SettingsMetadataError::MalformedJson)
    );
}
#[test]
fn local_ids_have_independent_exact_bounds_and_grammar() {
    for id in ["", "Upper", "a.b", "1bad", "a/b", "a\\b", "a:b", "é", "a b"] {
        let mut v = fixture();
        v["features"][0]["id"] = json!(id);
        assert_eq!(decode(&v), Err(SettingsMetadataError::InvalidId));
    }
    let mut v = fixture();
    v["features"][0]["id"] = json!("a".repeat(32));
    assert!(decode(&v).is_ok());
    v["features"][0]["id"] = json!("a".repeat(33));
    assert_eq!(decode(&v), Err(SettingsMetadataError::InvalidId));
    let mut v = fixture();
    v["features"][0]["options"][0]["id"] = json!("wrong.option");
    assert_eq!(decode(&v), Err(SettingsMetadataError::InvalidId));
}
#[test]
fn plain_text_limits_and_unicode_reject_controls_bidi_and_non_normalized_text() {
    for label in [
        "",
        " ",
        "control\n",
        "\u{202e}hidden",
        "\u{200f}hidden",
        "e\u{301}",
    ] {
        let mut v = fixture();
        v["features"][0]["label"] = json!(label);
        assert_eq!(decode(&v), Err(SettingsMetadataError::InvalidText));
    }
    let mut v = fixture();
    v["features"][0]["label"] = json!("é".repeat(64));
    v["features"][0]["description"] = json!("d".repeat(512));
    assert!(decode(&v).is_ok());
    v["features"][0]["label"] = json!("x".repeat(129));
    assert_eq!(decode(&v), Err(SettingsMetadataError::InvalidText));
    let mut v = fixture();
    v["features"][0]["options"][0]["description"] = json!("d".repeat(513));
    assert_eq!(decode(&v), Err(SettingsMetadataError::InvalidText));
}
#[test]
fn feature_and_option_counts_are_bounded_independently() {
    let mut v = fixture();
    v["features"] = json!([]);
    assert!(decode(&v).is_ok());
    v["features"] =
        Value::Array((0..16).map(|i| empty_feature(&format!("f{i}"))).collect());
    assert!(decode(&v).is_ok());
    v["features"]
        .as_array_mut()
        .unwrap()
        .push(empty_feature("extra"));
    assert_eq!(decode(&v), Err(SettingsMetadataError::TooManyFeatures));
    let mut v = fixture();
    v["features"][0]["options"] =
        Value::Array((0..8).map(|i| option(&format!("o{i}"))).collect());
    assert!(decode(&v).is_ok());
    v["features"][0]["options"]
        .as_array_mut()
        .unwrap()
        .push(option("extra"));
    assert_eq!(decode(&v), Err(SettingsMetadataError::TooManyOptions));
}
#[test]
fn total_controls_include_feature_enable_rows_at_exact_limit() {
    let mut v = fixture();
    v["features"] = Value::Array(
        (0..8)
            .map(|i| {
                let mut f = empty_feature(&format!("f{i}"));
                f["options"] =
                    Value::Array((0..7).map(|j| option(&format!("o{j}"))).collect());
                f
            })
            .collect(),
    );
    assert!(decode(&v).is_ok());
    v["features"][0]["options"]
        .as_array_mut()
        .unwrap()
        .push(option("extra"));
    assert_eq!(decode(&v), Err(SettingsMetadataError::TooManyControls));
}
#[test]
fn duplicate_features_or_options_reject_the_whole_document() {
    let mut v = fixture();
    let copy = v["features"][0].clone();
    v["features"].as_array_mut().unwrap().push(copy);
    assert_eq!(decode(&v), Err(SettingsMetadataError::DuplicateFeature));
    let mut v = fixture();
    v["features"][0]["options"][1]["id"] = json!("compact");
    assert_eq!(decode(&v), Err(SettingsMetadataError::DuplicateOption));
}
#[test]
fn choice_values_and_defaults_are_unique_bounded_and_declared() {
    let mut v = fixture();
    let choices = Value::Array(
        (0..16)
            .map(|i| json!({"value":format!("c{i}"),"label":"Choice"}))
            .collect(),
    );
    v["features"][0]["options"][1]["definition"] =
        json!({"kind":"choice","default":"c0","choices":choices});
    assert!(decode(&v).is_ok());
    v["features"][0]["options"][1]["definition"]["choices"]
        .as_array_mut()
        .unwrap()
        .push(json!({"value":"extra","label":"Choice"}));
    assert_eq!(decode(&v), Err(SettingsMetadataError::TooManyChoices));
    let mut v = fixture();
    v["features"][0]["options"][1]["definition"]["choices"] = json!([]);
    assert_eq!(decode(&v), Err(SettingsMetadataError::TooManyChoices));
    let mut v = fixture();
    v["features"][0]["options"][1]["definition"]["choices"][1]["value"] = json!("short");
    assert_eq!(decode(&v), Err(SettingsMetadataError::DuplicateChoice));
    let mut v = fixture();
    v["features"][0]["options"][1]["definition"]["default"] = json!("unknown");
    assert_eq!(decode(&v), Err(SettingsMetadataError::InvalidDefault));
}
#[test]
fn integer_range_steps_and_default_use_checked_exact_arithmetic() {
    let mut v = fixture();
    for definition in [
        json!({"kind":"integer","default":0,"min":1,"max":0,"step":1}),
        json!({"kind":"integer","default":0,"min":0,"max":1,"step":0}),
        json!({"kind":"integer","default":0,"min":0,"max":1000001,"step":1}),
    ] {
        v["features"][0]["options"][2]["definition"] = definition;
        assert_eq!(decode(&v), Err(SettingsMetadataError::InvalidRange));
    }
    for default in [-1, 2, 11] {
        let mut v = fixture();
        v["features"][0]["options"][2]["definition"]["default"] = json!(default);
        assert_eq!(decode(&v), Err(SettingsMetadataError::InvalidDefault));
    }
    let mut v = fixture();
    v["features"][0]["options"][2]["definition"] = json!({"kind":"integer","default":2147483647i64,"min":-2147483648i64,"max":2147483647i64,"step":4294967295u64});
    assert!(decode(&v).is_ok());
    v["features"][0]["options"][2]["definition"]["default"] = json!(2147483648i64);
    assert_eq!(decode(&v), Err(SettingsMetadataError::MalformedJson));
}
#[test]
fn exact_byte_ceiling_and_json_depth_are_enforced() {
    let mut bytes = FIXTURE.to_vec();
    bytes.resize(65536, b' ');
    assert!(decode_settings_metadata(&bytes, &manifest()).is_ok());
    bytes.push(b' ');
    assert_eq!(
        decode_settings_metadata(&bytes, &manifest()),
        Err(SettingsMetadataError::SourceTooLarge)
    );
    let bytes = format!("{}0{}", "[".repeat(17), "]".repeat(17));
    assert_eq!(
        decode_settings_metadata(bytes.as_bytes(), &manifest()),
        Err(SettingsMetadataError::MalformedJson)
    );
}
#[test]
fn diagnostics_do_not_echo_untrusted_content() {
    let mut v = fixture();
    v["features"][0]["id"] = json!("PRIVATE_SENTINEL invalid");
    let error = decode(&v).unwrap_err();
    assert_eq!(error, SettingsMetadataError::InvalidId);
    assert!(!format!("{error:?} {error}").contains("PRIVATE_SENTINEL"));
}
