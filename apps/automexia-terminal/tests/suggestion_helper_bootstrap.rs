use std::io::Cursor;

use automexia_command_productivity::suggestions::{
    RouteIdentity, ShellKind, SuggestionCapability,
};
use automexia_terminal::automexia::suggestions::{
    decode_helper_bootstrap, encode_helper_bootstrap, read_helper_bootstrap,
    HelperBootstrap, HelperEndpointLocator, HelperSessionBinding,
};

fn bootstrap() -> HelperBootstrap {
    HelperBootstrap {
        schema: 1,
        endpoint: HelperEndpointLocator::WindowsPipe(
            r"\\.\pipe\Automexia.Suggestions.123.9".into(),
        ),
        binding: HelperSessionBinding {
            route: RouteIdentity {
                application_generation: 1,
                window_id: 2,
                tab_id: 3,
                pane_id: 4,
                session_id: 5,
                shell: ShellKind::PowerShell,
                editor_version: "2.4.5".into(),
                endpoint_instance: 9,
            },
            capability: SuggestionCapability::from_bytes([0x5a; 32]),
            prompt_generation: 6,
            source_revision: 7,
        },
    }
}

#[test]
fn inherited_bootstrap_round_trips_without_debugging_locator_or_capability() {
    let bootstrap = bootstrap();
    let encoded = encode_helper_bootstrap(&bootstrap).unwrap();
    let decoded = decode_helper_bootstrap(&encoded).unwrap();
    assert_eq!(decoded.schema, 1);
    assert_eq!(decoded.binding.route, bootstrap.binding.route);
    assert!(decoded
        .binding
        .capability
        .constant_time_eq(&bootstrap.binding.capability));
    assert!(matches!(
        decoded.endpoint,
        HelperEndpointLocator::WindowsPipe(_)
    ));

    let streamed = read_helper_bootstrap(&mut Cursor::new(encoded)).unwrap();
    assert_eq!(streamed.binding.route, bootstrap.binding.route);
    let debug = format!("{bootstrap:?}");
    assert!(!debug.contains("Automexia.Suggestions"));
    assert!(!debug.contains("90"));
}

#[test]
fn unknown_fields_invalid_locator_trailing_and_oversized_prefix_fail_closed() {
    let encoded = encode_helper_bootstrap(&bootstrap()).unwrap();
    let mut trailing = encoded.clone();
    trailing.push(0);
    assert!(decode_helper_bootstrap(&trailing).is_err());

    let mut unknown = serde_json::to_value(bootstrap()).unwrap();
    unknown
        .as_object_mut()
        .unwrap()
        .insert("secret".into(), true.into());
    let payload = serde_json::to_vec(&unknown).unwrap();
    let mut frame = (payload.len() as u32).to_le_bytes().to_vec();
    frame.extend(payload);
    assert!(decode_helper_bootstrap(&frame).is_err());

    let mut invalid = bootstrap();
    invalid.endpoint = HelperEndpointLocator::UnixSocket("relative.sock".into());
    assert!(encode_helper_bootstrap(&invalid).is_err());

    let mut oversized = (16_385_u32).to_le_bytes().to_vec();
    oversized.extend_from_slice(b"not-read");
    assert!(read_helper_bootstrap(&mut Cursor::new(oversized)).is_err());
}
