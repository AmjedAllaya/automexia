use automexia_connectivity::connections::{
    parse_profile_json, prepare_direct_openssh, validate_direct_openssh_arguments,
    ConnectionModelErrorCode, DirectOpenSshTunnelConfirmation, DirectOpenSshTunnelEvent,
    DirectOpenSshTunnelLifecycle, DirectOpenSshTunnelLifecycleOwner,
    DirectOpenSshTunnelState, DirectOpenSshTunnelTransport, TunnelDefinitionV1,
    TunnelKind, TunnelLifetime,
};
use automexia_extension_api::SessionId;
use serde_json::{json, Value};

fn tunnel(
    id: &str,
    kind: &str,
    bind_address: &str,
    listen_port: u16,
    destination_host: Option<&str>,
    destination_port: Option<u16>,
) -> Value {
    json!({
        "schema_version": 1,
        "id": id,
        "kind": kind,
        "bind_address": bind_address,
        "listen_port": listen_port,
        "destination_host": destination_host,
        "destination_port": destination_port,
        "lifetime": "session"
    })
}

fn profile_value(tunnels: Vec<Value>) -> Value {
    json!({
        "schema_version": 1,
        "id": "profile-tunnels",
        "revision": 7,
        "display_name": "Reviewed tunnel fixture",
        "description": "Synthetic public metadata",
        "tags": ["ssh"],
        "favorite": false,
        "environment": {
            "kind": "development",
            "label": "Development",
            "risk": "development"
        },
        "provider": "ssh",
        "transport": {
            "kind": "open-ssh-explicit",
            "host": "ssh.example.invalid",
            "port": null,
            "user": null,
            "proxy_jump": []
        },
        "public_target": "ssh.example.invalid",
        "identity": {
            "kind": "agent",
            "reference": "identity-tunnels",
            "public_label": "external OpenSSH agent",
            "owner": "open-ssh"
        },
        "capsule": {
            "revision": 3,
            "public_environment": [],
            "context_references": []
        },
        "recipe_references": [],
        "tunnels": tunnels,
        "destination_preference": "pane-tab",
        "source": {
            "kind": "user",
            "reference": "source-tunnels",
            "revision": "8"
        },
        "approval_fingerprint": null,
        "created_at_ms": 1,
        "updated_at_ms": 2,
        "last_used_at_ms": null
    })
}

fn parse_profile(
    value: &Value,
) -> automexia_connectivity::connections::ConnectionProfileV1 {
    parse_profile_json(&serde_json::to_vec(value).unwrap())
        .unwrap()
        .into_inner()
}

fn all_tunnel_profile() -> automexia_connectivity::connections::ConnectionProfileV1 {
    parse_profile(&profile_value(vec![
        tunnel(
            "database",
            "local",
            "127.0.0.1",
            15_432,
            Some("database.internal"),
            Some(5_432),
        ),
        tunnel(
            "callback",
            "remote",
            "::1",
            18_080,
            Some("127.0.0.1"),
            Some(8_080),
        ),
        tunnel("socks", "dynamic", "localhost", 10_080, None, None),
    ]))
}

#[test]
fn local_remote_and_dynamic_tunnels_compile_to_exact_config_free_argv() {
    let prepared = prepare_direct_openssh(&all_tunnel_profile()).unwrap();
    let arguments = prepared.arguments();

    assert_eq!(&arguments[..2], ["-F", "none"]);
    assert!(arguments
        .iter()
        .any(|value| value == "-oExitOnForwardFailure=yes"));
    assert!(arguments.iter().any(|value| value == "-oCompression=no"));
    assert!(arguments.iter().any(|value| value == "-oGatewayPorts=no"));
    assert!(!arguments
        .iter()
        .any(|value| value == "-oClearAllForwardings=yes"));
    assert!(arguments
        .windows(2)
        .any(|pair| pair == ["-L", "127.0.0.1:15432:database.internal:5432"]));
    assert!(arguments
        .windows(2)
        .any(|pair| pair == ["-R", "[::1]:18080:127.0.0.1:8080"]));
    assert!(arguments
        .windows(2)
        .any(|pair| pair == ["-D", "127.0.0.1:10080"]));
    assert_eq!(
        arguments.last().map(String::as_str),
        Some("ssh.example.invalid")
    );

    let descriptors = prepared.tunnel_plan().descriptors();
    assert_eq!(descriptors.len(), 3);
    assert_eq!(descriptors[0].kind(), TunnelKind::Local);
    assert_eq!(
        descriptors[0].transport(),
        DirectOpenSshTunnelTransport::Tcp
    );
    assert_eq!(
        descriptors[0].lifecycle_owner(),
        DirectOpenSshTunnelLifecycleOwner::OpenSshSession
    );
    assert_eq!(descriptors[0].listen_endpoint(), "127.0.0.1:15432");
    assert_eq!(
        descriptors[0].target_endpoint(),
        Some("database.internal:5432")
    );
    assert_eq!(
        descriptors[1].confirmation(),
        DirectOpenSshTunnelConfirmation::StrongEveryUse
    );
    assert!(prepared.tunnel_plan().requires_strong_confirmation());
}

#[test]
fn independent_argument_validator_rejects_tunnel_tampering() {
    let prepared = prepare_direct_openssh(&all_tunnel_profile()).unwrap();
    let original = prepared.arguments().to_vec();
    validate_direct_openssh_arguments(&original).unwrap();

    let mut missing_config_sentinel = original.clone();
    missing_config_sentinel.remove(1);
    assert!(validate_direct_openssh_arguments(&missing_config_sentinel).is_err());

    let mut gateway_mismatch = original.clone();
    let gateway = gateway_mismatch
        .iter_mut()
        .find(|argument| argument.starts_with("-oGatewayPorts="))
        .unwrap();
    *gateway = "-oGatewayPorts=yes".into();
    assert!(validate_direct_openssh_arguments(&gateway_mismatch).is_err());

    let mut reordered = original.clone();
    let local_flag = reordered
        .iter()
        .position(|argument| argument == "-L")
        .unwrap();
    reordered.swap(2, local_flag);
    assert!(validate_direct_openssh_arguments(&reordered).is_err());

    let mut extra_forward = original;
    let destination = extra_forward.pop().unwrap();
    extra_forward.extend([
        "-L".into(),
        "127.0.0.1:19999:private.invalid;unreviewed:443".into(),
        destination,
    ]);
    assert!(validate_direct_openssh_arguments(&extra_forward).is_err());
}

#[test]
fn non_loopback_and_production_tunnels_require_strong_confirmation() {
    let mut value = profile_value(vec![tunnel(
        "shared-socks",
        "dynamic",
        "0.0.0.0",
        10_080,
        None,
        None,
    )]);
    value["environment"] = json!({
        "kind": "production",
        "label": "Production",
        "risk": "production"
    });
    let prepared = prepare_direct_openssh(&parse_profile(&value)).unwrap();

    assert!(prepared
        .arguments()
        .iter()
        .any(|value| value == "-oGatewayPorts=yes"));
    assert_eq!(
        prepared.tunnel_plan().descriptors()[0].confirmation(),
        DirectOpenSshTunnelConfirmation::StrongEveryUse
    );
    assert!(prepared
        .plan()
        .warnings
        .iter()
        .any(|warning| warning == "non-loopback-listener-review-required"));
    assert!(prepared
        .plan()
        .warnings
        .iter()
        .any(|warning| warning == "production-review-required"));
}

#[test]
fn tunnel_endpoints_are_exact_and_listener_collision_domains_are_distinct() {
    let hostile = profile_value(vec![tunnel(
        "bad",
        "dynamic",
        "127.0.0.1;malicious",
        10_080,
        None,
        None,
    )]);
    assert_eq!(
        parse_profile_json(&serde_json::to_vec(&hostile).unwrap())
            .unwrap_err()
            .code,
        ConnectionModelErrorCode::UnsafeText
    );

    let duplicate_remote = profile_value(vec![
        tunnel(
            "remote-a",
            "remote",
            "127.0.0.1",
            18_080,
            Some("127.0.0.1"),
            Some(8_080),
        ),
        tunnel(
            "remote-b",
            "remote",
            "localhost",
            18_080,
            Some("127.0.0.1"),
            Some(8_081),
        ),
    ]);
    assert_eq!(
        parse_profile_json(&serde_json::to_vec(&duplicate_remote).unwrap())
            .unwrap_err()
            .code,
        ConnectionModelErrorCode::DuplicateId
    );

    let separate_sides = profile_value(vec![
        tunnel(
            "local",
            "local",
            "127.0.0.1",
            18_080,
            Some("127.0.0.1"),
            Some(8_080),
        ),
        tunnel(
            "remote",
            "remote",
            "127.0.0.1",
            18_080,
            Some("127.0.0.1"),
            Some(8_081),
        ),
    ]);
    assert!(parse_profile_json(&serde_json::to_vec(&separate_sides).unwrap()).is_ok());
}

#[test]
fn config_dependent_routes_with_tunnels_fail_closed_and_endpoint_changes_are_stale() {
    let profile = all_tunnel_profile();
    let prepared = prepare_direct_openssh(&profile).unwrap();
    let mut changed = profile.clone();
    changed.tunnels[0].listen_port = 25_432;
    assert_eq!(
        prepared.validate_current(&changed).unwrap_err().code,
        ConnectionModelErrorCode::InvalidTransition
    );

    let mut alias = profile_value(vec![tunnel(
        "database",
        "local",
        "127.0.0.1",
        15_432,
        Some("database.internal"),
        Some(5_432),
    )]);
    alias["transport"] = json!({
        "kind": "open-ssh-alias",
        "alias": "production",
        "host": "ssh.example.invalid",
        "port": null,
        "user": null,
        "proxy_jump": []
    });
    alias["source"] = json!({
        "kind": "open-ssh-inventory",
        "reference": "source-inventory",
        "revision": "10"
    });
    assert_eq!(
        prepare_direct_openssh(&parse_profile(&alias))
            .unwrap_err()
            .code,
        ConnectionModelErrorCode::InvalidPolicy
    );
}

#[test]
fn lifecycle_rejects_stale_scope_and_never_claims_ready_without_owner_evidence() {
    let prepared = prepare_direct_openssh(&all_tunnel_profile()).unwrap();
    let session = SessionId::new(17);
    let mut lifecycle =
        DirectOpenSshTunnelLifecycle::new(session, 3, prepared.tunnel_plan(), 100)
            .unwrap();

    assert!(lifecycle
        .statuses()
        .iter()
        .all(|status| status.state() == DirectOpenSshTunnelState::Planned));
    assert_eq!(lifecycle.ready_count(), 0);

    lifecycle
        .apply(DirectOpenSshTunnelEvent::new(
            session,
            3,
            "database",
            DirectOpenSshTunnelState::Starting,
            101,
        ))
        .unwrap();
    lifecycle
        .apply(DirectOpenSshTunnelEvent::new(
            session,
            3,
            "database",
            DirectOpenSshTunnelState::Ready,
            102,
        ))
        .unwrap();
    assert_eq!(lifecycle.ready_count(), 1);

    let stale = lifecycle
        .apply(DirectOpenSshTunnelEvent::new(
            SessionId::new(18),
            3,
            "socks",
            DirectOpenSshTunnelState::Ready,
            103,
        ))
        .unwrap_err();
    assert_eq!(stale.code, ConnectionModelErrorCode::InvalidTransition);

    lifecycle
        .apply(DirectOpenSshTunnelEvent::new(
            session,
            3,
            "callback",
            DirectOpenSshTunnelState::Starting,
            104,
        ))
        .unwrap();
    lifecycle
        .apply(DirectOpenSshTunnelEvent::new(
            session,
            3,
            "callback",
            DirectOpenSshTunnelState::Collision,
            105,
        ))
        .unwrap();
    assert!(lifecycle
        .apply(DirectOpenSshTunnelEvent::new(
            session,
            3,
            "callback",
            DirectOpenSshTunnelState::Ready,
            106,
        ))
        .is_err());

    lifecycle.close_all(107).unwrap();
    assert!(lifecycle.all_terminal());
    assert_eq!(lifecycle.ready_count(), 0);
}

#[test]
fn one_ten_and_fifty_fake_sessions_release_every_tunnel_state() {
    let prepared = prepare_direct_openssh(&all_tunnel_profile()).unwrap();
    for session_count in [1_u64, 10, 50] {
        let mut lifecycles = (1..=session_count)
            .map(|id| {
                DirectOpenSshTunnelLifecycle::new(
                    SessionId::new(id),
                    1,
                    prepared.tunnel_plan(),
                    1,
                )
                .unwrap()
            })
            .collect::<Vec<_>>();
        assert_eq!(
            lifecycles
                .iter()
                .map(|state| state.statuses().len())
                .sum::<usize>(),
            usize::try_from(session_count).unwrap() * 3
        );
        for lifecycle in &mut lifecycles {
            lifecycle.close_all(2).unwrap();
        }
        assert!(lifecycles
            .iter()
            .all(DirectOpenSshTunnelLifecycle::all_terminal));
    }
}

#[test]
fn typed_builders_default_every_listener_to_loopback_and_session_lifetime() {
    let local = TunnelDefinitionV1::local_loopback(
        "database",
        15_432,
        "database.internal",
        5_432,
    );
    let remote =
        TunnelDefinitionV1::remote_loopback("callback", 18_080, "127.0.0.1", 8_080);
    let dynamic = TunnelDefinitionV1::dynamic_loopback("socks", 10_080);

    for tunnel in [&local, &remote, &dynamic] {
        assert_eq!(tunnel.bind_address, "127.0.0.1");
        assert_eq!(tunnel.lifetime, TunnelLifetime::Session);
        assert!(tunnel.is_loopback());
    }
    let mut alternate_loopback = dynamic.clone();
    alternate_loopback.bind_address = "127.0.0.2".into();
    assert!(alternate_loopback.is_loopback());
    alternate_loopback.bind_address = "127.0.0.01".into();
    assert!(!alternate_loopback.is_loopback());
    alternate_loopback.bind_address = "128.0.0.1".into();
    assert!(!alternate_loopback.is_loopback());

    assert_eq!(local.kind, TunnelKind::Local);
    assert_eq!(remote.kind, TunnelKind::Remote);
    assert_eq!(dynamic.kind, TunnelKind::Dynamic);
    assert_eq!(local.destination_host.as_deref(), Some("database.internal"));
    assert_eq!(dynamic.destination_host, None);
}
