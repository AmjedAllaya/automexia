use std::collections::{HashMap, HashSet};

use serde::de::DeserializeOwned;

use super::model::*;
use super::openssh_tunnels::{
    canonical_tunnel_bind_address, canonical_tunnel_destination_host,
};

fn error(
    code: ConnectionModelErrorCode,
    field: &'static str,
    detail: &'static str,
) -> ConnectionModelError {
    ConnectionModelError::new(code, field, detail)
}

fn parse_strict<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, ConnectionModelError> {
    if bytes.len() > MAX_DOCUMENT_BYTES {
        return Err(error(
            ConnectionModelErrorCode::LimitExceeded,
            "document",
            "document exceeds the fixed byte ceiling",
        ));
    }
    serde_json::from_slice(bytes).map_err(|_| {
        error(
            ConnectionModelErrorCode::MalformedSchema,
            "document",
            "document does not match the strict public schema",
        )
    })
}

pub fn parse_profile_json(
    bytes: &[u8],
) -> Result<ValidatedConnectionProfile, ConnectionModelError> {
    let profile: ConnectionProfileV1 = parse_strict(bytes)?;
    validate_profile(&profile)?;
    Ok(ValidatedConnectionProfile::from_validated(profile))
}

pub fn parse_recipe_json(
    bytes: &[u8],
) -> Result<ValidatedAutomationRecipe, ConnectionModelError> {
    let recipe: AutomationRecipeV1 = parse_strict(bytes)?;
    validate_recipe(&recipe)?;
    Ok(ValidatedAutomationRecipe::from_validated(recipe))
}

pub fn parse_connection_definition_json(
    bytes: &[u8],
) -> Result<ConnectionDefinition, ConnectionModelError> {
    let value = parse_strict(bytes)?;
    validate_connection_definition(&value)?;
    Ok(value)
}

pub fn parse_connection_observation_json(
    bytes: &[u8],
) -> Result<ConnectionObservation, ConnectionModelError> {
    let value = parse_strict(bytes)?;
    validate_connection_observation(&value)?;
    Ok(value)
}

pub fn parse_connection_intent_json(
    bytes: &[u8],
) -> Result<ConnectionIntent, ConnectionModelError> {
    let value = parse_strict(bytes)?;
    validate_connection_intent(&value)?;
    Ok(value)
}

pub fn parse_connection_review_json(
    bytes: &[u8],
) -> Result<ConnectionReview, ConnectionModelError> {
    let value = parse_strict(bytes)?;
    validate_connection_review(&value)?;
    Ok(value)
}

pub fn parse_connection_receipt_json(
    bytes: &[u8],
) -> Result<ConnectionReceipt, ConnectionModelError> {
    let value = parse_strict(bytes)?;
    validate_connection_receipt(&value)?;
    Ok(value)
}

fn validate_schema(
    version: u16,
    field: &'static str,
) -> Result<(), ConnectionModelError> {
    if version != CONNECTION_SCHEMA_VERSION {
        return Err(error(
            ConnectionModelErrorCode::UnsupportedVersion,
            field,
            "unsupported schema version",
        ));
    }
    Ok(())
}

pub(super) fn contains_hostile_format(character: char) -> bool {
    matches!(
        character,
        '\u{061c}'
            | '\u{200b}'..='\u{200f}'
            | '\u{202a}'..='\u{202e}'
            | '\u{2060}'..='\u{206f}'
            | '\u{feff}'
    )
}

pub(super) fn validate_text(
    value: &str,
    field: &'static str,
    allow_empty: bool,
) -> Result<(), ConnectionModelError> {
    if value.len() > MAX_STRING_BYTES {
        return Err(error(
            ConnectionModelErrorCode::LimitExceeded,
            field,
            "text exceeds the fixed byte ceiling",
        ));
    }
    if !allow_empty && value.trim().is_empty() {
        return Err(error(
            ConnectionModelErrorCode::UnsafeText,
            field,
            "text must not be empty",
        ));
    }
    if value
        .chars()
        .any(|character| character.is_control() || contains_hostile_format(character))
    {
        return Err(error(
            ConnectionModelErrorCode::UnsafeText,
            field,
            "control and bidirectional formatting characters are forbidden",
        ));
    }
    Ok(())
}

fn validate_target(value: &str, field: &'static str) -> Result<(), ConnectionModelError> {
    validate_text(value, field, false)?;
    if value.trim_start().starts_with('-') {
        return Err(error(
            ConnectionModelErrorCode::UnsafeText,
            field,
            "option-like targets are forbidden",
        ));
    }
    Ok(())
}

fn validate_identifier(
    value: &str,
    field: &'static str,
) -> Result<(), ConnectionModelError> {
    if value.is_empty()
        || value.len() > MAX_IDENTIFIER_BYTES
        || !value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || b"._-".contains(&byte)
        })
        || !value
            .as_bytes()
            .first()
            .is_some_and(u8::is_ascii_alphanumeric)
    {
        return Err(error(
            ConnectionModelErrorCode::InvalidIdentifier,
            field,
            "identifier must be bounded lowercase ASCII",
        ));
    }
    Ok(())
}

fn validate_reference(
    value: &OpaqueReference,
    field: &'static str,
) -> Result<(), ConnectionModelError> {
    validate_identifier(value.as_str(), field)
}

fn validate_digest(value: &str, field: &'static str) -> Result<(), ConnectionModelError> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(error(
            ConnectionModelErrorCode::InvalidFingerprint,
            field,
            "fingerprint must be a lowercase 256-bit hexadecimal digest",
        ));
    }
    Ok(())
}

fn validate_unique_texts(
    values: &[String],
    field: &'static str,
    maximum: usize,
) -> Result<(), ConnectionModelError> {
    if values.len() > maximum {
        return Err(error(
            ConnectionModelErrorCode::LimitExceeded,
            field,
            "collection exceeds its fixed item ceiling",
        ));
    }
    let mut seen = HashSet::with_capacity(values.len());
    for value in values {
        validate_text(value, field, false)?;
        if !seen.insert(value.as_str()) {
            return Err(error(
                ConnectionModelErrorCode::DuplicateId,
                field,
                "duplicate values are forbidden",
            ));
        }
    }
    Ok(())
}

fn validate_unique_references(
    values: &[OpaqueReference],
    field: &'static str,
    maximum: usize,
) -> Result<(), ConnectionModelError> {
    if values.len() > maximum {
        return Err(error(
            ConnectionModelErrorCode::LimitExceeded,
            field,
            "collection exceeds its fixed item ceiling",
        ));
    }
    let mut seen = HashSet::with_capacity(values.len());
    for value in values {
        validate_reference(value, field)?;
        if !seen.insert(value.as_str()) {
            return Err(error(
                ConnectionModelErrorCode::DuplicateId,
                field,
                "duplicate references are forbidden",
            ));
        }
    }
    Ok(())
}

fn validate_source(source: &ConnectionSource) -> Result<(), ConnectionModelError> {
    validate_reference(&source.reference, "source.reference")?;
    validate_text(&source.revision, "source.revision", false)
}

fn validate_identity(identity: &IdentityReference) -> Result<(), ConnectionModelError> {
    validate_reference(&identity.reference, "identity.reference")?;
    validate_text(&identity.public_label, "identity.public_label", false)?;
    validate_identifier(&identity.owner, "identity.owner")
}

fn validate_environment(
    environment: &EnvironmentClassification,
) -> Result<(), ConnectionModelError> {
    validate_text(&environment.label, "environment.label", false)?;
    let expected = match environment.kind {
        EnvironmentKind::Local => EnvironmentRisk::Local,
        EnvironmentKind::Development => EnvironmentRisk::Development,
        EnvironmentKind::Test => EnvironmentRisk::Test,
        EnvironmentKind::Staging => EnvironmentRisk::Staging,
        EnvironmentKind::Production => EnvironmentRisk::Production,
        EnvironmentKind::Custom => return Ok(()),
    };
    if environment.risk != expected {
        return Err(error(
            ConnectionModelErrorCode::InvalidPolicy,
            "environment.risk",
            "built-in environment classification and risk disagree",
        ));
    }
    Ok(())
}

fn looks_secret_bearing_name(name: &str) -> bool {
    let folded = name.to_ascii_uppercase();
    [
        "PASSWORD",
        "PASSphrase".to_ascii_uppercase().as_str(),
        "TOKEN",
        "SECRET",
        "PRIVATE_KEY",
        "ACCESS_KEY",
        "CREDENTIAL",
        "COOKIE",
    ]
    .iter()
    .any(|part| folded.contains(part))
}

fn validate_environment_name(name: &str) -> Result<(), ConnectionModelError> {
    validate_text(name, "capsule.public_environment.name", false)?;
    if name.len() > MAX_IDENTIFIER_BYTES
        || looks_secret_bearing_name(name)
        || !name.bytes().all(|byte| {
            byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_'
        })
        || name
            .as_bytes()
            .first()
            .is_some_and(|byte| byte.is_ascii_digit())
    {
        return Err(error(
            ConnectionModelErrorCode::InvalidIdentifier,
            "capsule.public_environment.name",
            "only public non-secret environment names are accepted",
        ));
    }
    Ok(())
}

fn validate_capsule(
    capsule: &EnvironmentCapsuleTemplate,
) -> Result<(), ConnectionModelError> {
    if capsule.revision == 0 {
        return Err(error(
            ConnectionModelErrorCode::InvalidIdentifier,
            "capsule.revision",
            "capsule revision must be nonzero",
        ));
    }
    if capsule.public_environment.len() > MAX_VARIABLES_PER_RECIPE {
        return Err(error(
            ConnectionModelErrorCode::LimitExceeded,
            "capsule.public_environment",
            "capsule environment exceeds its fixed item ceiling",
        ));
    }
    let mut names = HashSet::with_capacity(capsule.public_environment.len());
    for binding in &capsule.public_environment {
        validate_environment_name(&binding.name)?;
        validate_text(&binding.value, "capsule.public_environment.value", true)?;
        if !names.insert(binding.name.as_str()) {
            return Err(error(
                ConnectionModelErrorCode::DuplicateId,
                "capsule.public_environment.name",
                "duplicate public environment names are forbidden",
            ));
        }
    }
    validate_unique_references(
        &capsule.context_references,
        "capsule.context_references",
        MAX_CAPABILITIES,
    )
}

fn validate_transport(
    transport: &TransportDescriptor,
) -> Result<(), ConnectionModelError> {
    let text = |value: &String, field| validate_target(value, field);
    match transport {
        TransportDescriptor::OpenSshAlias {
            alias,
            host,
            port,
            user,
            proxy_jump,
        } => {
            text(alias, "transport.alias")?;
            if let Some(host) = host {
                text(host, "transport.host")?;
            }
            if port == &Some(0) {
                return Err(error(
                    ConnectionModelErrorCode::InvalidPolicy,
                    "transport.port",
                    "port zero is forbidden",
                ));
            }
            if let Some(user) = user {
                validate_target(user, "transport.user")?;
            }
            validate_unique_texts(proxy_jump, "transport.proxy_jump", MAX_JUMPS)?;
            for jump in proxy_jump {
                validate_target(jump, "transport.proxy_jump")?;
            }
            Ok(())
        }
        TransportDescriptor::OpenSshExplicit {
            host,
            port,
            user,
            proxy_jump,
        } => {
            text(host, "transport.host")?;
            if port == &Some(0) {
                return Err(error(
                    ConnectionModelErrorCode::InvalidPolicy,
                    "transport.port",
                    "port zero is forbidden",
                ));
            }
            if let Some(user) = user {
                validate_target(user, "transport.user")?;
            }
            validate_unique_texts(proxy_jump, "transport.proxy_jump", MAX_JUMPS)?;
            for jump in proxy_jump {
                validate_target(jump, "transport.proxy_jump")?;
            }
            Ok(())
        }
        TransportDescriptor::AwsSessionManager {
            target,
            profile_reference,
            region,
            document,
        } => {
            text(target, "transport.target")?;
            validate_reference(profile_reference, "transport.profile_reference")?;
            text(region, "transport.region")?;
            if let Some(document) = document {
                text(document, "transport.document")?;
            }
            Ok(())
        }
        TransportDescriptor::AzureBastion {
            bastion,
            resource_group,
            vm_resource_id,
            subscription_reference,
        } => {
            text(bastion, "transport.bastion")?;
            text(resource_group, "transport.resource_group")?;
            text(vm_resource_id, "transport.vm_resource_id")?;
            validate_reference(subscription_reference, "transport.subscription_reference")
        }
        TransportDescriptor::GcpIap {
            instance,
            project,
            zone,
            configuration_reference,
        } => {
            text(instance, "transport.instance")?;
            text(project, "transport.project")?;
            text(zone, "transport.zone")?;
            validate_reference(
                configuration_reference,
                "transport.configuration_reference",
            )
        }
        TransportDescriptor::KubernetesExec {
            context,
            namespace,
            workload,
            container,
            shell,
        } => {
            text(context, "transport.context")?;
            text(namespace, "transport.namespace")?;
            text(workload, "transport.workload")?;
            if let Some(container) = container {
                text(container, "transport.container")?;
            }
            validate_identifier(shell, "transport.shell")
        }
        TransportDescriptor::OpenShiftRsh {
            context,
            namespace,
            workload,
            container,
        } => {
            text(context, "transport.context")?;
            text(namespace, "transport.namespace")?;
            text(workload, "transport.workload")?;
            if let Some(container) = container {
                text(container, "transport.container")?;
            }
            Ok(())
        }
        TransportDescriptor::TeleportSsh {
            proxy,
            cluster,
            target,
            login,
        } => {
            for value in [proxy, cluster, login].into_iter().flatten() {
                text(value, "transport.teleport")?;
            }
            text(target, "transport.target")
        }
        TransportDescriptor::LocalContainer {
            runtime,
            container,
            user,
            shell,
        } => {
            validate_identifier(runtime, "transport.runtime")?;
            text(container, "transport.container")?;
            if let Some(user) = user {
                text(user, "transport.user")?;
            }
            validate_identifier(shell, "transport.shell")
        }
    }
}

fn validate_provider_transport(
    provider: ProviderKind,
    transport: &TransportDescriptor,
) -> Result<(), ConnectionModelError> {
    let compatible = matches!(
        (provider, transport.kind()),
        (ProviderKind::Ssh, TransportKind::OpenSsh)
            | (ProviderKind::Aws, TransportKind::AwsSessionManager)
            | (ProviderKind::Azure, TransportKind::AzureBastion)
            | (ProviderKind::Gcp, TransportKind::GcpIap)
            | (ProviderKind::Kubernetes, TransportKind::KubernetesExec)
            | (ProviderKind::OpenShift, TransportKind::OpenShiftRsh)
            | (ProviderKind::Teleport, TransportKind::TeleportSsh)
            | (ProviderKind::None, TransportKind::LocalContainer)
            | (ProviderKind::LocalContainer, TransportKind::LocalContainer)
    );
    if compatible {
        Ok(())
    } else {
        Err(error(
            ConnectionModelErrorCode::InvalidPolicy,
            "provider",
            "provider and transport are incompatible",
        ))
    }
}

fn validate_tunnel(tunnel: &TunnelDefinitionV1) -> Result<(), ConnectionModelError> {
    validate_schema(tunnel.schema_version, "tunnel.schema_version")?;
    validate_identifier(&tunnel.id, "tunnel.id")?;
    canonical_tunnel_bind_address(&tunnel.bind_address)?;
    if tunnel.listen_port == 0 {
        return Err(error(
            ConnectionModelErrorCode::InvalidPolicy,
            "tunnel.listen_port",
            "port zero is forbidden",
        ));
    }
    match tunnel.kind {
        TunnelKind::Dynamic => {
            if tunnel.destination_host.is_some() || tunnel.destination_port.is_some() {
                return Err(error(
                    ConnectionModelErrorCode::InvalidPolicy,
                    "tunnel.destination",
                    "dynamic forwarding must not name a fixed destination",
                ));
            }
        }
        TunnelKind::Local | TunnelKind::Remote => {
            let Some(host) = &tunnel.destination_host else {
                return Err(error(
                    ConnectionModelErrorCode::InvalidPolicy,
                    "tunnel.destination_host",
                    "forwarding destination is required",
                ));
            };
            canonical_tunnel_destination_host(host)?;
            if tunnel.destination_port.is_none() || tunnel.destination_port == Some(0) {
                return Err(error(
                    ConnectionModelErrorCode::InvalidPolicy,
                    "tunnel.destination_port",
                    "nonzero forwarding destination port is required",
                ));
            }
        }
    }
    Ok(())
}

fn validate_tunnels(tunnels: &[TunnelDefinitionV1]) -> Result<(), ConnectionModelError> {
    if tunnels.len() > MAX_TUNNELS_PER_PROFILE {
        return Err(error(
            ConnectionModelErrorCode::LimitExceeded,
            "tunnels",
            "tunnel collection exceeds its fixed item ceiling",
        ));
    }
    let mut ids = HashSet::with_capacity(tunnels.len());
    let mut listeners = HashSet::with_capacity(tunnels.len());
    for tunnel in tunnels {
        validate_tunnel(tunnel)?;
        if !ids.insert(tunnel.id.as_str()) {
            return Err(error(
                ConnectionModelErrorCode::DuplicateId,
                "tunnels.id",
                "duplicate tunnel IDs are forbidden",
            ));
        }
        let listener_side_is_remote = tunnel.kind == TunnelKind::Remote;
        let bind_address = canonical_tunnel_bind_address(&tunnel.bind_address)?;
        if !listeners.insert((listener_side_is_remote, bind_address, tunnel.listen_port))
        {
            return Err(error(
                ConnectionModelErrorCode::DuplicateId,
                "tunnels.listen_port",
                "duplicate local listener endpoints are forbidden",
            ));
        }
    }
    Ok(())
}

pub fn validate_connection_definition(
    definition: &ConnectionDefinition,
) -> Result<(), ConnectionModelError> {
    validate_schema(definition.schema_version, "schema_version")?;
    validate_identifier(&definition.id, "id")?;
    validate_text(&definition.display_name, "display_name", false)?;
    validate_text(&definition.description, "description", true)?;
    validate_unique_texts(&definition.tags, "tags", MAX_TAGS_PER_PROFILE)?;
    validate_source(&definition.source)?;
    validate_transport(&definition.transport)?;
    validate_provider_transport(definition.provider, &definition.transport)?;
    validate_target(&definition.public_destination, "public_destination")?;
    validate_reference(
        &definition.environment_template_reference,
        "environment_template_reference",
    )?;
    validate_identity(&definition.identity)?;
    validate_unique_references(
        &definition.jump_references,
        "jump_references",
        MAX_JUMPS,
    )?;
    validate_tunnels(&definition.tunnel_templates)
}

pub fn validate_profile(
    profile: &ConnectionProfileV1,
) -> Result<(), ConnectionModelError> {
    validate_schema(profile.schema_version, "schema_version")?;
    validate_identifier(&profile.id, "id")?;
    if profile.revision == 0 {
        return Err(error(
            ConnectionModelErrorCode::InvalidIdentifier,
            "revision",
            "profile revision must be nonzero",
        ));
    }
    validate_text(&profile.display_name, "display_name", false)?;
    validate_text(&profile.description, "description", true)?;
    validate_unique_texts(&profile.tags, "tags", MAX_TAGS_PER_PROFILE)?;
    validate_environment(&profile.environment)?;
    validate_transport(&profile.transport)?;
    validate_provider_transport(profile.provider, &profile.transport)?;
    validate_target(&profile.public_target, "public_target")?;
    validate_identity(&profile.identity)?;
    validate_capsule(&profile.capsule)?;
    validate_unique_texts(
        &profile.jump_profile_references,
        "jump_profile_references",
        MAX_JUMPS,
    )?;
    if profile.recipe_references.len() > MAX_RECIPES {
        return Err(error(
            ConnectionModelErrorCode::LimitExceeded,
            "recipe_references",
            "recipe references exceed their fixed item ceiling",
        ));
    }
    let mut recipe_ids = HashSet::with_capacity(profile.recipe_references.len());
    for recipe in &profile.recipe_references {
        validate_identifier(&recipe.id, "recipe_references.id")?;
        if recipe.revision == 0 {
            return Err(error(
                ConnectionModelErrorCode::InvalidIdentifier,
                "recipe_references.revision",
                "recipe revision must be nonzero",
            ));
        }
        validate_digest(&recipe.fingerprint, "recipe_references.fingerprint")?;
        if !recipe_ids.insert(recipe.id.as_str()) {
            return Err(error(
                ConnectionModelErrorCode::DuplicateId,
                "recipe_references.id",
                "duplicate recipe references are forbidden",
            ));
        }
    }
    validate_tunnels(&profile.tunnels)?;
    validate_source(&profile.source)?;
    if let Some(fingerprint) = &profile.approval_fingerprint {
        validate_digest(fingerprint, "approval_fingerprint")?;
    }
    if profile.created_at_ms > profile.updated_at_ms
        || profile
            .last_used_at_ms
            .is_some_and(|value| value < profile.created_at_ms)
    {
        return Err(error(
            ConnectionModelErrorCode::InvalidPolicy,
            "timestamps",
            "profile timestamps are inconsistent",
        ));
    }
    Ok(())
}

fn validate_auth_state(state: &AuthState) -> Result<(), ConnectionModelError> {
    match state {
        AuthState::Unknown | AuthState::Stale { .. } => Ok(()),
        AuthState::Checking { operation_id }
        | AuthState::Authenticating { operation_id } => {
            validate_identifier(operation_id, "auth_state.operation_id")
        }
        AuthState::Ready {
            evidence_id,
            expires_at_ms: _,
        } => validate_identifier(evidence_id, "auth_state.evidence_id"),
        AuthState::Locked { diagnostic_code }
        | AuthState::Missing { diagnostic_code }
        | AuthState::MfaRequired { diagnostic_code }
        | AuthState::Cancelled { diagnostic_code }
        | AuthState::Offline { diagnostic_code }
        | AuthState::Denied { diagnostic_code }
        | AuthState::Unsupported { diagnostic_code }
        | AuthState::Error { diagnostic_code } => {
            validate_identifier(diagnostic_code, "auth_state.diagnostic_code")
        }
        AuthState::Expired {
            previous_evidence_id,
        } => {
            if let Some(evidence) = previous_evidence_id {
                validate_identifier(evidence, "auth_state.previous_evidence_id")?;
            }
            Ok(())
        }
    }
}

pub fn validate_connection_observation(
    observation: &ConnectionObservation,
) -> Result<(), ConnectionModelError> {
    validate_schema(observation.schema_version, "schema_version")?;
    validate_identifier(&observation.connection_id, "connection_id")?;
    validate_auth_state(&observation.auth_state)?;
    if observation.stale_after_ms == 0
        || observation
            .expires_at_ms
            .is_some_and(|value| value < observation.observed_at_ms)
    {
        return Err(error(
            ConnectionModelErrorCode::InvalidPolicy,
            "observation",
            "observation freshness bounds are inconsistent",
        ));
    }
    validate_text(
        &observation.public_identity_summary,
        "public_identity_summary",
        true,
    )?;
    for value in [
        observation.diagnostic_code.as_ref(),
        observation.recovery_action.as_ref(),
    ]
    .into_iter()
    .flatten()
    {
        validate_identifier(value, "observation.code")?;
    }
    Ok(())
}

pub fn validate_connection_intent(
    intent: &ConnectionIntent,
) -> Result<(), ConnectionModelError> {
    validate_schema(intent.schema_version, "schema_version")?;
    validate_identifier(&intent.connection_id, "connection_id")?;
    if intent.profile_revision == 0 {
        return Err(error(
            ConnectionModelErrorCode::InvalidIdentifier,
            "profile_revision",
            "profile revision must be nonzero",
        ));
    }
    validate_text(&intent.source_revision, "source_revision", false)?;
    validate_target(&intent.public_destination, "public_destination")?;
    validate_transport(&intent.transport)?;
    validate_unique_texts(&intent.jump_chain, "jump_chain", MAX_JUMPS)?;
    validate_tunnels(&intent.tunnels)?;
    validate_identity(&intent.identity)?;
    validate_capsule(&intent.capsule)?;
    validate_unique_texts(
        &intent.requested_capabilities,
        "requested_capabilities",
        MAX_CAPABILITIES,
    )?;
    for capability in &intent.requested_capabilities {
        validate_identifier(capability, "requested_capabilities")?;
    }
    if intent.recipe_fingerprints.len() > MAX_RECIPES {
        return Err(error(
            ConnectionModelErrorCode::LimitExceeded,
            "recipe_fingerprints",
            "recipe fingerprints exceed their fixed item ceiling",
        ));
    }
    for fingerprint in &intent.recipe_fingerprints {
        validate_digest(fingerprint, "recipe_fingerprints")?;
    }
    Ok(())
}

pub fn validate_connection_review(
    review: &ConnectionReview,
) -> Result<(), ConnectionModelError> {
    validate_schema(review.schema_version, "schema_version")?;
    validate_connection_intent(&review.normalized_intent)?;
    if review.policy_decisions.len() > MAX_CAPABILITIES
        || review.warnings.len() > MAX_CAPABILITIES
        || review.changed_fields.len() > MAX_CAPABILITIES
        || review.executable_preview.len() > MAX_CAPABILITIES
    {
        return Err(error(
            ConnectionModelErrorCode::LimitExceeded,
            "review",
            "review collection exceeds its fixed item ceiling",
        ));
    }
    let mut decision_codes = HashSet::with_capacity(review.policy_decisions.len());
    for decision in &review.policy_decisions {
        validate_identifier(&decision.code, "policy_decisions.code")?;
        validate_text(&decision.reason, "policy_decisions.reason", false)?;
        if !decision_codes.insert(decision.code.as_str()) {
            return Err(error(
                ConnectionModelErrorCode::DuplicateId,
                "policy_decisions.code",
                "duplicate policy decisions are forbidden",
            ));
        }
    }
    validate_unique_texts(&review.warnings, "warnings", MAX_CAPABILITIES)?;
    validate_unique_texts(&review.changed_fields, "changed_fields", MAX_CAPABILITIES)?;
    let mut executable_ids = HashSet::with_capacity(review.executable_preview.len());
    for preview in &review.executable_preview {
        validate_identifier(&preview.executable_id, "executable_preview.executable_id")?;
        if !executable_ids.insert(preview.executable_id.as_str()) {
            return Err(error(
                ConnectionModelErrorCode::DuplicateId,
                "executable_preview.executable_id",
                "duplicate executable previews are forbidden",
            ));
        }
        if preview.arguments.len() > MAX_STEPS_PER_RECIPE {
            return Err(error(
                ConnectionModelErrorCode::LimitExceeded,
                "executable_preview.arguments",
                "argument preview exceeds its fixed item ceiling",
            ));
        }
        for argument in &preview.arguments {
            validate_text(&argument.label, "executable_preview.arguments.label", true)?;
        }
    }
    match &review.host_trust {
        HostTrustState::FirstUse { fingerprint_sha256 }
        | HostTrustState::Known { fingerprint_sha256 }
        | HostTrustState::Changed { fingerprint_sha256 } => {
            validate_digest(fingerprint_sha256, "host_trust.fingerprint_sha256")?;
        }
        HostTrustState::NotApplicable | HostTrustState::Unknown => {}
    }
    validate_digest(&review.approval_fingerprint, "approval_fingerprint")
}

fn validate_result_state(
    state: &OperationResultState,
) -> Result<(), ConnectionModelError> {
    match state {
        OperationResultState::Warning { diagnostic_code }
        | OperationResultState::Failed { diagnostic_code }
        | OperationResultState::Cancelled { diagnostic_code }
        | OperationResultState::Offline { diagnostic_code }
        | OperationResultState::Denied { diagnostic_code }
        | OperationResultState::Unsupported { diagnostic_code }
        | OperationResultState::Stale { diagnostic_code }
        | OperationResultState::Error { diagnostic_code } => {
            validate_identifier(diagnostic_code, "outcome.diagnostic_code")
        }
        OperationResultState::Pending
        | OperationResultState::Running
        | OperationResultState::WaitingForUser
        | OperationResultState::Succeeded
        | OperationResultState::SkippedByUser => Ok(()),
    }
}

pub fn validate_connection_receipt(
    receipt: &ConnectionReceipt,
) -> Result<(), ConnectionModelError> {
    validate_schema(receipt.schema_version, "schema_version")?;
    validate_identifier(&receipt.operation_id, "operation_id")?;
    validate_identifier(&receipt.session_id, "session_id")?;
    validate_identifier(&receipt.capsule_id, "capsule_id")?;
    validate_digest(&receipt.approved_intent_digest, "approved_intent_digest")?;
    validate_text(&receipt.source_revision, "source_revision", false)?;
    for references in [
        &receipt.process_ownership_references,
        &receipt.route_ownership_references,
        &receipt.tunnel_ownership_references,
    ] {
        validate_unique_references(references, "ownership_references", MAX_CAPABILITIES)?;
    }
    validate_result_state(&receipt.outcome)
}

fn validate_variable(variable: &RecipeVariableV1) -> Result<(), ConnectionModelError> {
    validate_identifier(&variable.id, "variables.id")?;
    if looks_secret_bearing_name(&variable.id) {
        return Err(error(
            ConnectionModelErrorCode::InvalidIdentifier,
            "variables.id",
            "secret-bearing variables are forbidden",
        ));
    }
    validate_text(&variable.prompt, "variables.prompt", false)?;
    if let Some(value) = &variable.public_default {
        validate_text(value, "variables.public_default", true)?;
    }
    Ok(())
}

fn validate_action(action: &AutomationAction) -> Result<(), ConnectionModelError> {
    match action {
        AutomationAction::ResolveConnection => Ok(()),
        AutomationAction::SetSessionEnvironment { name, public_value }
        | AutomationAction::SetRemotePublicEnvironment { name, public_value } => {
            validate_environment_name(name)?;
            validate_text(public_value, "action.public_value", true)
        }
        AutomationAction::UnsetSessionEnvironment { name } => {
            validate_environment_name(name)
        }
        AutomationAction::SetLocalWorkingDirectory { path_reference }
        | AutomationAction::RequireFile { path_reference }
        | AutomationAction::SetRemoteWorkingDirectory { path_reference } => {
            validate_reference(path_reference, "action.path_reference")
        }
        AutomationAction::RequireExecutable {
            executable_id,
            version_requirement,
        } => {
            validate_identifier(executable_id, "action.executable_id")?;
            if let Some(requirement) = version_requirement {
                validate_text(requirement, "action.version_requirement", false)?;
            }
            Ok(())
        }
        AutomationAction::CheckAgentState { agent_kind } => {
            validate_identifier(agent_kind, "action.agent_kind")
        }
        AutomationAction::SetProviderScope {
            profile_reference,
            region,
            ..
        } => {
            validate_reference(profile_reference, "action.profile_reference")?;
            if let Some(region) = region {
                validate_target(region, "action.region")?;
            }
            Ok(())
        }
        AutomationAction::SetKubernetesScope {
            kubeconfig_reference,
            context,
            namespace,
        }
        | AutomationAction::SetOpenShiftScope {
            kubeconfig_reference,
            context,
            namespace,
        } => {
            validate_reference(kubeconfig_reference, "action.kubeconfig_reference")?;
            validate_target(context, "action.context")?;
            if let Some(namespace) = namespace {
                validate_target(namespace, "action.namespace")?;
            }
            Ok(())
        }
        AutomationAction::StartTunnel { tunnel_id } => {
            validate_identifier(tunnel_id, "action.tunnel_id")
        }
        AutomationAction::AuthenticateExternal { owner } => {
            validate_identifier(owner, "action.owner")
        }
        AutomationAction::SwitchRemoteUser { user, .. } => {
            validate_target(user, "action.user")
        }
        AutomationAction::ConnectTransport
        | AutomationAction::VerifyRemoteUser
        | AutomationAction::VerifyRemoteWorkingDirectory
        | AutomationAction::VerifyProviderIdentity { .. }
        | AutomationAction::VerifyContext { .. } => Ok(()),
    }
}

fn action_stage_is_valid(step: &AutomationStepV1) -> bool {
    match &step.action {
        AutomationAction::ResolveConnection => step.stage == ExecutionStage::Resolve,
        AutomationAction::RequireExecutable { .. }
        | AutomationAction::RequireFile { .. }
        | AutomationAction::CheckAgentState { .. } => {
            step.stage == ExecutionStage::Preflight
        }
        AutomationAction::AuthenticateExternal { .. } => {
            step.stage == ExecutionStage::Authenticate
        }
        AutomationAction::SetSessionEnvironment { .. }
        | AutomationAction::UnsetSessionEnvironment { .. }
        | AutomationAction::SetLocalWorkingDirectory { .. }
        | AutomationAction::SetProviderScope { .. }
        | AutomationAction::SetKubernetesScope { .. }
        | AutomationAction::SetOpenShiftScope { .. } => matches!(
            step.stage,
            ExecutionStage::BeforeConnect
                | ExecutionStage::BeforeDisconnect
                | ExecutionStage::Cleanup
        ),
        AutomationAction::ConnectTransport => step.stage == ExecutionStage::Connect,
        AutomationAction::StartTunnel { .. } => matches!(
            step.stage,
            ExecutionStage::BeforeConnect | ExecutionStage::Connect
        ),
        AutomationAction::SetRemoteWorkingDirectory { .. }
        | AutomationAction::SetRemotePublicEnvironment { .. }
        | AutomationAction::SwitchRemoteUser { .. } => {
            step.stage == ExecutionStage::RemoteInitialize
        }
        AutomationAction::VerifyRemoteUser
        | AutomationAction::VerifyRemoteWorkingDirectory
        | AutomationAction::VerifyProviderIdentity { .. }
        | AutomationAction::VerifyContext { .. } => step.stage == ExecutionStage::Verify,
    }
}

fn validate_retry(step: &AutomationStepV1) -> Result<(), ConnectionModelError> {
    let RetryPolicy::Automatic {
        max_attempts,
        initial_backoff_ms,
        max_backoff_ms,
        total_deadline_ms,
        jitter_percent,
        idempotent,
        interaction_free,
        persistent_mutation_free,
        cancellation_safe,
    } = &step.retry_policy
    else {
        return Ok(());
    };
    if *max_attempts == 0
        || *max_attempts > MAX_AUTOMATIC_ATTEMPTS
        || *initial_backoff_ms == 0
        || initial_backoff_ms > max_backoff_ms
        || max_backoff_ms > total_deadline_ms
        || *total_deadline_ms > step.timeout_ms
        || *jitter_percent > 100
        || !(*idempotent
            && *interaction_free
            && *persistent_mutation_free
            && *cancellation_safe)
        || !matches!(
            step.risk,
            ActionRisk::Observe | ActionRisk::SessionLocal | ActionRisk::RemoteSession
        )
    {
        return Err(error(
            ConnectionModelErrorCode::InvalidPolicy,
            "steps.retry_policy",
            "automatic retry requires bounded idempotent noninteractive cancellation-safe work",
        ));
    }
    Ok(())
}

fn validate_step(step: &AutomationStepV1) -> Result<(), ConnectionModelError> {
    validate_schema(step.schema_version, "steps.schema_version")?;
    validate_identifier(&step.id, "steps.id")?;
    validate_action(&step.action)?;
    validate_unique_texts(&step.depends_on, "steps.depends_on", MAX_STEPS_PER_RECIPE)?;
    for dependency in &step.depends_on {
        validate_identifier(dependency, "steps.depends_on")?;
    }
    validate_unique_texts(
        &step.preconditions,
        "steps.preconditions",
        MAX_PRECONDITIONS,
    )?;
    for precondition in &step.preconditions {
        validate_identifier(precondition, "steps.preconditions")?;
    }
    if step.timeout_ms == 0 || step.timeout_ms > MAX_STEP_TIMEOUT_MS {
        return Err(error(
            ConnectionModelErrorCode::LimitExceeded,
            "steps.timeout_ms",
            "step timeout is outside the fixed bound",
        ));
    }
    if !action_stage_is_valid(step) {
        return Err(error(
            ConnectionModelErrorCode::InvalidPolicy,
            "steps.stage",
            "action is not valid in the selected stage",
        ));
    }
    if step.risk < step.action.minimum_risk() {
        return Err(error(
            ConnectionModelErrorCode::InvalidPolicy,
            "steps.risk",
            "step risk understates the typed action",
        ));
    }
    if matches!(
        step.risk,
        ActionRisk::Authenticate
            | ActionRisk::Privileged
            | ActionRisk::NetworkListener
            | ActionRisk::PersistentMutation
            | ActionRisk::CustomCode
    ) && step.failure_policy == FailurePolicy::WarnAndContinue
    {
        return Err(error(
            ConnectionModelErrorCode::InvalidPolicy,
            "steps.failure_policy",
            "high-risk work cannot silently continue after failure",
        ));
    }
    if matches!(step.risk, ActionRisk::Authenticate | ActionRisk::CustomCode)
        && step.confirmation_policy != ConfirmationPolicy::EveryUse
        || matches!(
            step.risk,
            ActionRisk::Privileged | ActionRisk::NetworkListener
        ) && step.confirmation_policy != ConfirmationPolicy::EveryConnection
        || matches!(
            step.risk,
            ActionRisk::PersistentMutation | ActionRisk::CustomCode
        ) && step.reconnect_policy != ReconnectPolicy::NeverAutomaticallyRepeat
    {
        return Err(error(
            ConnectionModelErrorCode::InvalidPolicy,
            "steps.confirmation_policy",
            "risk, confirmation, and reconnect policies are inconsistent",
        ));
    }
    validate_retry(step)
}

fn dependency_cycle(steps: &[AutomationStepV1], indices: &HashMap<&str, usize>) -> bool {
    fn visit(
        index: usize,
        steps: &[AutomationStepV1],
        indices: &HashMap<&str, usize>,
        marks: &mut [u8],
    ) -> bool {
        if marks[index] == 1 {
            return true;
        }
        if marks[index] == 2 {
            return false;
        }
        marks[index] = 1;
        for dependency in &steps[index].depends_on {
            if visit(indices[dependency.as_str()], steps, indices, marks) {
                return true;
            }
        }
        marks[index] = 2;
        false
    }

    let mut marks = vec![0; steps.len()];
    (0..steps.len()).any(|index| visit(index, steps, indices, &mut marks))
}

pub fn validate_recipe(recipe: &AutomationRecipeV1) -> Result<(), ConnectionModelError> {
    validate_schema(recipe.schema_version, "schema_version")?;
    validate_identifier(&recipe.id, "id")?;
    if recipe.revision == 0 {
        return Err(error(
            ConnectionModelErrorCode::InvalidIdentifier,
            "revision",
            "recipe revision must be nonzero",
        ));
    }
    validate_text(&recipe.display_name, "display_name", false)?;
    validate_text(&recipe.description, "description", true)?;
    if recipe.compatible_providers.is_empty()
        || recipe.compatible_transports.is_empty()
        || recipe.compatible_providers.len() > MAX_CAPABILITIES
        || recipe.compatible_transports.len() > MAX_CAPABILITIES
    {
        return Err(error(
            ConnectionModelErrorCode::LimitExceeded,
            "compatibility",
            "compatibility sets must be nonempty and bounded",
        ));
    }
    if recipe
        .compatible_providers
        .iter()
        .collect::<HashSet<_>>()
        .len()
        != recipe.compatible_providers.len()
        || recipe
            .compatible_transports
            .iter()
            .collect::<HashSet<_>>()
            .len()
            != recipe.compatible_transports.len()
    {
        return Err(error(
            ConnectionModelErrorCode::DuplicateId,
            "compatibility",
            "duplicate compatibility entries are forbidden",
        ));
    }
    if recipe.variables.len() > MAX_VARIABLES_PER_RECIPE
        || recipe.steps.len() > MAX_STEPS_PER_RECIPE
    {
        return Err(error(
            ConnectionModelErrorCode::LimitExceeded,
            "recipe",
            "recipe exceeds its fixed variable or step ceiling",
        ));
    }
    let mut variable_ids = HashSet::with_capacity(recipe.variables.len());
    for variable in &recipe.variables {
        validate_variable(variable)?;
        if !variable_ids.insert(variable.id.as_str()) {
            return Err(error(
                ConnectionModelErrorCode::DuplicateId,
                "variables.id",
                "duplicate variable IDs are forbidden",
            ));
        }
    }
    let mut indices = HashMap::with_capacity(recipe.steps.len());
    let mut connect_steps = 0;
    for (index, step) in recipe.steps.iter().enumerate() {
        validate_step(step)?;
        if indices.insert(step.id.as_str(), index).is_some() {
            return Err(error(
                ConnectionModelErrorCode::DuplicateId,
                "steps.id",
                "duplicate step IDs are forbidden",
            ));
        }
        connect_steps +=
            usize::from(matches!(step.action, AutomationAction::ConnectTransport));
    }
    if connect_steps > 1 {
        return Err(error(
            ConnectionModelErrorCode::InvalidPolicy,
            "steps.action",
            "a recipe can declare at most one transport connection",
        ));
    }
    for step in &recipe.steps {
        for dependency in &step.depends_on {
            if !indices.contains_key(dependency.as_str()) {
                return Err(error(
                    ConnectionModelErrorCode::MissingDependency,
                    "steps.depends_on",
                    "step dependency does not exist",
                ));
            }
        }
    }
    if dependency_cycle(&recipe.steps, &indices) {
        return Err(error(
            ConnectionModelErrorCode::DependencyCycle,
            "steps.depends_on",
            "step dependency graph contains a cycle",
        ));
    }
    for step in &recipe.steps {
        for dependency in &step.depends_on {
            let dependency_index = indices[dependency.as_str()];
            if recipe.steps[dependency_index].stage > step.stage {
                return Err(error(
                    ConnectionModelErrorCode::InvalidPolicy,
                    "steps.depends_on",
                    "a step cannot depend on a later execution stage",
                ));
            }
        }
    }
    if let Some(fingerprint) = &recipe.approval_fingerprint {
        validate_digest(fingerprint, "approval_fingerprint")?;
    }
    if recipe.created_at_ms > recipe.updated_at_ms {
        return Err(error(
            ConnectionModelErrorCode::InvalidPolicy,
            "timestamps",
            "recipe timestamps are inconsistent",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hostile_format_filter_covers_controls_and_bidi_isolates() {
        for hostile in ['\0', '\n', '\u{061c}', '\u{202e}', '\u{2066}', '\u{2069}'] {
            assert!(
                validate_text(&format!("safe{hostile}unsafe"), "test", false).is_err()
            );
        }
        assert!(validate_text("مرحبا بالعالم", "test", false).is_ok());
    }

    #[test]
    fn opaque_reference_debug_never_reveals_the_reference() {
        let reference = OpaqueReference::new("credential-canary");
        let debug = format!("{reference:?}");
        assert!(!debug.contains("credential-canary"));
        assert!(debug.contains("redacted"));
    }
}
