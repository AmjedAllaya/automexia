use std::fmt;

use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};
use unicode_normalization::UnicodeNormalization as _;

use crate::{decode_strict_json, safe_text, valid_digest, Limits, StrictJsonError};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelDisclosure {
    pub provider: String,
    pub locality: ModelLocality,
    pub model: String,
    pub destination: String,
    pub purpose: String,
    pub retention: String,
    pub environment_risk: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ModelLocality {
    Local,
    Remote,
}

#[derive(Clone, PartialEq, Eq)]
pub struct SelectedInput {
    text: String,
}

impl SelectedInput {
    pub fn new(text: impl Into<String>) -> Result<Self, ModelSuggestionError> {
        let text = text.into();
        if text.is_empty()
            || text.len() > Limits::SELECTED_MODEL_INPUT_BYTES
            || text.nfc().ne(text.chars())
            || text.chars().any(|character| character == '\0')
        {
            return Err(ModelSuggestionError::InvalidSelection);
        }
        Ok(Self { text })
    }

    pub fn expose_for_review(&self) -> &str {
        &self.text
    }
}

impl fmt::Debug for SelectedInput {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SelectedInput")
            .field("bytes", &self.text.len())
            .field("text", &"[redacted]")
            .finish()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RedactionKind {
    AssignmentSecret,
    AuthorizationHeader,
    PrivateKey,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RedactionFinding {
    pub line: usize,
    pub kind: RedactionKind,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RedactionPreview {
    pub redacted_text: String,
    pub original_bytes: usize,
    pub transferred_bytes: usize,
    pub findings: Vec<RedactionFinding>,
}

impl fmt::Debug for RedactionPreview {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RedactionPreview")
            .field("redacted_text", &"[redacted]")
            .field("original_bytes", &self.original_bytes)
            .field("transferred_bytes", &self.transferred_bytes)
            .field("findings", &self.findings)
            .finish()
    }
}

pub fn redact_selected_input(selection: &SelectedInput) -> RedactionPreview {
    let mut findings = Vec::new();
    let mut lines = Vec::new();
    for (index, line) in selection.text.lines().enumerate() {
        let lower = line.to_ascii_lowercase();
        let private_key =
            lower.contains("-----begin") && lower.contains("private key-----");
        let authorization = lower.trim_start().starts_with("authorization:");
        let assignment = ["password", "passwd", "token", "secret", "api_key", "apikey"]
            .iter()
            .any(|marker| lower.contains(marker))
            && (line.contains('=') || line.contains(':'));
        if private_key {
            findings.push(RedactionFinding {
                line: index + 1,
                kind: RedactionKind::PrivateKey,
            });
            lines.push("[REDACTED PRIVATE KEY]".to_owned());
        } else if authorization {
            findings.push(RedactionFinding {
                line: index + 1,
                kind: RedactionKind::AuthorizationHeader,
            });
            lines.push("Authorization: [REDACTED]".to_owned());
        } else if assignment {
            findings.push(RedactionFinding {
                line: index + 1,
                kind: RedactionKind::AssignmentSecret,
            });
            let delimiter = line.find('=').or_else(|| line.find(':')).unwrap_or(0);
            lines.push(format!(
                "{}=[REDACTED]",
                line[..delimiter].trim_end_matches([' ', ':', '='])
            ));
        } else {
            lines.push(line.to_owned());
        }
    }
    let redacted_text = lines.join("\n");
    RedactionPreview {
        original_bytes: selection.text.len(),
        transferred_bytes: redacted_text.len(),
        redacted_text,
        findings,
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelReview {
    pub request_id: u64,
    pub route_id: String,
    pub generation: u64,
    pub disclosure: ModelDisclosure,
    pub preview: RedactionPreview,
    pub consent_sha256: String,
}

impl ModelReview {
    pub fn new(
        request_id: u64,
        route_id: impl Into<String>,
        generation: u64,
        disclosure: ModelDisclosure,
        selection: &SelectedInput,
    ) -> Result<Self, ModelSuggestionError> {
        let route_id = route_id.into();
        if request_id == 0
            || generation == 0
            || !safe_text(&route_id, false)
            || !disclosure.is_valid()
        {
            return Err(ModelSuggestionError::InvalidDisclosure);
        }
        let preview = redact_selected_input(selection);
        let consent_sha256 =
            review_digest(request_id, &route_id, generation, &disclosure, &preview)?;
        Ok(Self {
            request_id,
            route_id,
            generation,
            disclosure,
            preview,
            consent_sha256,
        })
    }

    pub fn confirm(
        &self,
        displayed_sha256: &str,
        now_unix: u64,
    ) -> Result<ModelConsent, ModelSuggestionError> {
        if displayed_sha256 != self.consent_sha256 || !valid_digest(displayed_sha256) {
            return Err(ModelSuggestionError::ConsentMismatch);
        }
        Ok(ModelConsent {
            request_id: self.request_id,
            route_id: self.route_id.clone(),
            generation: self.generation,
            consent_sha256: self.consent_sha256.clone(),
            confirmed_at_unix: now_unix,
            used: false,
        })
    }
}

impl ModelDisclosure {
    fn is_valid(&self) -> bool {
        [
            self.provider.as_str(),
            self.model.as_str(),
            self.destination.as_str(),
            self.purpose.as_str(),
            self.retention.as_str(),
            self.environment_risk.as_str(),
        ]
        .into_iter()
        .all(|value| safe_text(value, false))
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct ModelConsent {
    request_id: u64,
    route_id: String,
    generation: u64,
    consent_sha256: String,
    confirmed_at_unix: u64,
    used: bool,
}

impl fmt::Debug for ModelConsent {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ModelConsent")
            .field("request_id", &self.request_id)
            .field("route_id", &self.route_id)
            .field("generation", &self.generation)
            .field("consent_sha256", &self.consent_sha256)
            .field("confirmed_at_unix", &self.confirmed_at_unix)
            .field("used", &self.used)
            .finish()
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct ModelRequestEnvelope {
    pub request_id: u64,
    pub route_id: String,
    pub generation: u64,
    pub selected_text: String,
    pub disclosure: ModelDisclosure,
}

impl fmt::Debug for ModelRequestEnvelope {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ModelRequestEnvelope")
            .field("request_id", &self.request_id)
            .field("route_id", &self.route_id)
            .field("generation", &self.generation)
            .field("selected_text", &"[redacted]")
            .field("disclosure", &self.disclosure)
            .finish()
    }
}

impl ModelConsent {
    pub fn consume(
        &mut self,
        review: &ModelReview,
        current_route: &str,
        current_generation: u64,
    ) -> Result<ModelRequestEnvelope, ModelSuggestionError> {
        if self.used {
            return Err(ModelSuggestionError::ConsentAlreadyUsed);
        }
        if self.request_id != review.request_id
            || self.route_id != review.route_id
            || self.route_id != current_route
            || self.generation != review.generation
            || self.generation != current_generation
            || self.consent_sha256 != review.consent_sha256
        {
            return Err(ModelSuggestionError::StaleRequest);
        }
        self.used = true;
        Ok(ModelRequestEnvelope {
            request_id: review.request_id,
            route_id: review.route_id.clone(),
            generation: review.generation,
            selected_text: review.preview.redacted_text.clone(),
            disclosure: review.disclosure.clone(),
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ModelResponseKind {
    Explanation,
    Suggestion,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ModelRisk {
    ReadOnly,
    Mutating,
    Destructive,
    Privileged,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelResponseWire {
    pub schema_version: u32,
    pub request_id: u64,
    pub route_id: String,
    pub generation: u64,
    pub kind: ModelResponseKind,
    pub text: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidatedModelResponse {
    pub request_id: u64,
    pub route_id: String,
    pub generation: u64,
    pub kind: ModelResponseKind,
    pub text: String,
    pub risk: ModelRisk,
}

pub fn validate_model_response(
    source: &[u8],
    request: &ModelRequestEnvelope,
) -> Result<ValidatedModelResponse, ModelSuggestionError> {
    let response: ModelResponseWire =
        decode_strict_json(source, Limits::MODEL_RESPONSE_BYTES)
            .map_err(ModelSuggestionError::Decode)?;
    if response.schema_version != 1
        || response.request_id != request.request_id
        || response.route_id != request.route_id
        || response.generation != request.generation
        || response.text.is_empty()
        || response.text.len() > Limits::MODEL_RESPONSE_BYTES
        || response.text.nfc().ne(response.text.chars())
        || response.text.chars().any(is_output_control)
    {
        return Err(ModelSuggestionError::InvalidResponse);
    }
    let risk = classify_risk(&response.text);
    Ok(ValidatedModelResponse {
        request_id: response.request_id,
        route_id: response.route_id,
        generation: response.generation,
        kind: response.kind,
        text: response.text,
        risk,
    })
}

fn classify_risk(text: &str) -> ModelRisk {
    let lower = text.to_ascii_lowercase();
    if ["sudo ", "runas ", "set-executionpolicy", "chmod 777"]
        .iter()
        .any(|marker| lower.contains(marker))
    {
        ModelRisk::Privileged
    } else if [
        "rm -rf",
        "remove-item",
        "delete ",
        "destroy",
        "drop database",
    ]
    .iter()
    .any(|marker| lower.contains(marker))
    {
        ModelRisk::Destructive
    } else if [
        " create ",
        " update ",
        " apply ",
        " install ",
        " restart ",
        " write ",
    ]
    .iter()
    .any(|marker| format!(" {lower} ").contains(marker))
    {
        ModelRisk::Mutating
    } else {
        ModelRisk::ReadOnly
    }
}

fn review_digest(
    request_id: u64,
    route_id: &str,
    generation: u64,
    disclosure: &ModelDisclosure,
    preview: &RedactionPreview,
) -> Result<String, ModelSuggestionError> {
    let encoded =
        serde_json::to_vec(&(request_id, route_id, generation, disclosure, preview))
            .map_err(|_| ModelSuggestionError::InvalidDisclosure)?;
    Ok(hex_digest(&Sha256::digest(encoded)))
}

fn hex_digest(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output
}

fn is_output_control(character: char) -> bool {
    (character.is_control() && !matches!(character, '\n' | '\r' | '\t'))
        || matches!(character, '\u{061c}' | '\u{200e}' | '\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
}

#[derive(Debug)]
pub enum ModelSuggestionError {
    InvalidSelection,
    InvalidDisclosure,
    ConsentMismatch,
    ConsentAlreadyUsed,
    StaleRequest,
    Decode(StrictJsonError),
    InvalidResponse,
    ProviderCallsDisabled,
}

impl fmt::Display for ModelSuggestionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidSelection => "selected input is empty, oversized, or malformed",
            Self::InvalidDisclosure => {
                "model provider disclosure is incomplete or unsafe"
            }
            Self::ConsentMismatch => "the confirmed review digest does not match",
            Self::ConsentAlreadyUsed => "per-request consent was already consumed",
            Self::StaleRequest => "model request route or generation is stale",
            Self::Decode(_) => "model response is not strict bounded JSON",
            Self::InvalidResponse => {
                "model response is malformed, stale, unsafe, or oversized"
            }
            Self::ProviderCallsDisabled => {
                "model provider calls are disabled in this release"
            }
        })
    }
}

impl std::error::Error for ModelSuggestionError {}

pub fn request_model_provider(
    _request: ModelRequestEnvelope,
) -> Result<Vec<u8>, ModelSuggestionError> {
    Err(ModelSuggestionError::ProviderCallsDisabled)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn disclosure() -> ModelDisclosure {
        ModelDisclosure {
            provider: "Local test provider".into(),
            locality: ModelLocality::Local,
            model: "test-v1".into(),
            destination: "local process".into(),
            purpose: "Explain selected command".into(),
            retention: "No retention".into(),
            environment_risk: "Development".into(),
        }
    }

    #[test]
    fn exact_review_redacts_canaries_and_consent_is_single_use_and_route_bound() {
        let selection = SelectedInput::new("TOKEN=canary\necho ok").unwrap();
        let review = ModelReview::new(7, "pane.4", 9, disclosure(), &selection).unwrap();
        assert!(!review.preview.redacted_text.contains("canary"));
        assert_eq!(review.preview.findings.len(), 1);
        let mut consent = review.confirm(&review.consent_sha256, 100).unwrap();
        let request = consent.consume(&review, "pane.4", 9).unwrap();
        assert!(!request.selected_text.contains("canary"));
        assert!(matches!(
            consent.consume(&review, "pane.4", 9),
            Err(ModelSuggestionError::ConsentAlreadyUsed)
        ));
    }

    #[test]
    fn malformed_tool_shaped_oversized_and_stale_responses_fail_closed() {
        let selection = SelectedInput::new("git status").unwrap();
        let review = ModelReview::new(7, "pane.4", 9, disclosure(), &selection).unwrap();
        let mut consent = review.confirm(&review.consent_sha256, 100).unwrap();
        let request = consent.consume(&review, "pane.4", 9).unwrap();
        let tool = br#"{"schema_version":1,"request_id":7,"route_id":"pane.4","generation":9,"kind":"suggestion","text":"git status","tool_calls":[]}"#;
        assert!(validate_model_response(tool, &request).is_err());
        let stale = br#"{"schema_version":1,"request_id":7,"route_id":"pane.4","generation":8,"kind":"suggestion","text":"git status"}"#;
        assert!(matches!(
            validate_model_response(stale, &request),
            Err(ModelSuggestionError::InvalidResponse)
        ));
        assert!(matches!(
            request_model_provider(request),
            Err(ModelSuggestionError::ProviderCallsDisabled)
        ));
    }
}
