use automexia_connectivity::connections::*;

fn source(endpoint: SshAgentEndpoint) -> CredentialSourceV1 {
    CredentialSourceV1 {
        schema_version: 1,
        id: "work-vault".into(),
        revision: 1,
        display_name: "Work vault".into(),
        provider: CredentialProvider::OnePassword,
        endpoint,
    }
}

#[test]
fn supported_vaults_keep_private_keys_with_an_external_agent() {
    for provider in CredentialProvider::ALL {
        let mut source = source(SshAgentEndpoint::System);
        source.provider = provider;
        let plan = plan_ssh_credentials(&source, CredentialPlatform::Windows).unwrap();
        assert_eq!(plan.source_id(), "work-vault");
        assert!(plan.environment_override().is_none());
        assert!(!plan.execution_enabled());
        assert!(!provider.setup_url().is_empty());
    }
}

#[test]
fn unix_socket_is_a_single_environment_value_not_command_text() {
    let path = "/run/user/1000/agent directory/ssh.sock";
    let source = source(SshAgentEndpoint::UnixSocket { path: path.into() });
    for platform in [CredentialPlatform::Linux, CredentialPlatform::MacOs] {
        let plan = plan_ssh_credentials(&source, platform).unwrap();
        assert_eq!(plan.environment_override(), Some(("SSH_AUTH_SOCK", path)));
        assert_eq!(
            plan.identity_agent_option(),
            Some("-oIdentityAgent=SSH_AUTH_SOCK")
        );
        assert!(!format!("{plan:?}").contains(path));
    }
    assert!(plan_ssh_credentials(&source, CredentialPlatform::Windows).is_err());
}

#[test]
fn vault_records_reject_secrets_unknown_fields_duplicate_keys_and_wrong_versions() {
    let value = serde_json::to_value(CredentialSourcesDocumentV1 {
        schema_version: 1,
        sources: vec![source(SshAgentEndpoint::System)],
    })
    .unwrap();
    for field in ["password", "token", "private_key", "session_key"] {
        let mut invalid = value.clone();
        invalid["sources"][0][field] = "must-not-be-stored".into();
        assert!(
            parse_credential_sources_json(&serde_json::to_vec(&invalid).unwrap())
                .is_err()
        );
    }
    let mut invalid = value;
    invalid["schema_version"] = 2.into();
    assert!(
        parse_credential_sources_json(&serde_json::to_vec(&invalid).unwrap()).is_err()
    );
    assert!(parse_credential_sources_json(
        br#"{"schema_version":1,"schema_version":1,"sources":[]}"#
    )
    .is_err());
}

#[test]
fn source_validation_rejects_ambiguous_or_hostile_agent_paths() {
    for path in [
        "relative/socket",
        "~/socket",
        "$SSH_AUTH_SOCK",
        "/tmp/../socket",
        "/tmp/socket\n",
        "/tmp/\u{202e}socket",
        "//server/socket",
        "/tmp/%h/socket",
        "/tmp/${HOME}/socket",
    ] {
        let source = source(SshAgentEndpoint::UnixSocket { path: path.into() });
        assert!(
            validate_credential_source(&source).is_err(),
            "accepted hostile path"
        );
    }
    assert!(
        validate_credential_source(&source(SshAgentEndpoint::UnixSocket {
            path: format!("/{}", "a".repeat(MAX_AGENT_ENDPOINT_BYTES))
        }))
        .is_err()
    );
}

#[test]
fn source_document_bounds_and_ids_are_checked_before_planning() {
    let mut sources = vec![source(SshAgentEndpoint::System); 2];
    assert!(validate_credential_sources(&CredentialSourcesDocumentV1 {
        schema_version: 1,
        sources: sources.clone()
    })
    .is_err());
    sources[1].id = "another-vault".into();
    let document = CredentialSourcesDocumentV1 {
        schema_version: 1,
        sources,
    };
    validate_credential_sources(&document).unwrap();
    let parsed =
        parse_credential_sources_json(&serde_json::to_vec(&document).unwrap()).unwrap();
    assert_eq!(parsed, document);
    assert!(
        parse_credential_sources_json(&vec![b' '; MAX_CREDENTIAL_SOURCES_BYTES + 1])
            .is_err()
    );
}

#[test]
fn a_changed_source_invalidates_the_connection_binding() {
    let mut source = source(SshAgentEndpoint::System);
    let plan = plan_ssh_credentials(&source, CredentialPlatform::Linux).unwrap();
    assert!(plan.validate_current(&source).is_ok());
    source.endpoint = SshAgentEndpoint::UnixSocket {
        path: "/run/agent.sock".into(),
    };
    assert!(plan.validate_current(&source).is_err());
    source.endpoint = SshAgentEndpoint::System;
    source.revision += 1;
    assert!(plan.validate_current(&source).is_err());
}

#[test]
fn validated_documents_always_fit_the_serialized_read_budget() {
    let sources = (0..MAX_CREDENTIAL_SOURCES)
        .map(|index| {
            let mut source = source(SshAgentEndpoint::UnixSocket {
                path: format!("/{}", "\"".repeat(MAX_AGENT_ENDPOINT_BYTES - 1)),
            });
            source.id = format!("source-{index}");
            source.display_name = "\"".repeat(256);
            source
        })
        .collect();
    let document = CredentialSourcesDocumentV1 {
        schema_version: 1,
        sources,
    };
    assert!(serde_json::to_vec(&document).unwrap().len() > MAX_CREDENTIAL_SOURCES_BYTES);
    assert!(validate_credential_sources(&document).is_err());
}
