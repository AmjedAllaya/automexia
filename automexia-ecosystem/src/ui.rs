use serde::{Deserialize, Serialize};

use crate::{
    Capability, CapabilityDiff, LifecycleState, ModelDisclosure, ModelRisk,
    VerificationReceipt,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EcosystemIcon {
    Package,
    Verified,
    Shield,
    Warning,
    Quarantine,
    Revoked,
    Model,
    Offline,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SemanticTone {
    Neutral,
    Positive,
    Caution,
    Danger,
    Accent,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ReviewFocus {
    Cancel,
    Details,
    Confirm,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ReviewDensity {
    Compact,
    Comfortable,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewRow {
    pub label: String,
    pub value: String,
    pub accessible_value: String,
    pub tone: SemanticTone,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EcosystemReviewSurface {
    pub title: String,
    pub subtitle: String,
    pub icon: EcosystemIcon,
    pub tone: SemanticTone,
    pub rows: Vec<ReviewRow>,
    pub primary_label: String,
    pub secondary_label: String,
    pub default_focus: ReviewFocus,
    pub restore_focus: bool,
    pub reduced_motion: bool,
    pub density: ReviewDensity,
    pub accessibility_name: String,
    pub accessibility_description: String,
}

#[derive(Clone, Copy, Debug)]
pub struct ModelConsentSurfaceRequest<'a> {
    pub disclosure: &'a ModelDisclosure,
    pub redacted_preview: &'a str,
    pub consent_sha256: &'a str,
    pub selected_bytes: usize,
    pub transferred_bytes: usize,
    pub redaction_count: usize,
    pub environment_risk: ModelRisk,
    pub viewport_width: u32,
    pub reduced_motion: bool,
}

impl EcosystemReviewSurface {
    pub fn package_review(
        receipt: &VerificationReceipt,
        diff: &CapabilityDiff,
        viewport_width: u32,
        reduced_motion: bool,
    ) -> Self {
        let added = capability_list(&diff.added);
        let removed = capability_list(&diff.removed);
        let imports = bounded_list(&receipt.manifest.imports);
        let compatibility = format!(
            "SDK {}.{}–{}.{}",
            receipt.manifest.compatibility.sdk_major,
            receipt.manifest.compatibility.sdk_minor_minimum,
            receipt.manifest.compatibility.sdk_major,
            receipt.manifest.compatibility.sdk_minor_maximum,
        );
        let rows = vec![
            row(
                "Publisher ID (exact)",
                &receipt.publisher_id,
                SemanticTone::Neutral,
            ),
            row(
                "Signing key (exact)",
                &receipt.key_id,
                SemanticTone::Neutral,
            ),
            row("Version", &receipt.version, SemanticTone::Neutral),
            row(
                "Package digest",
                &short_digest(&receipt.package_sha256),
                SemanticTone::Neutral,
            ),
            row("Source", &receipt.source_uri, SemanticTone::Neutral),
            row(
                "Source revision",
                &receipt.source_revision,
                SemanticTone::Neutral,
            ),
            row("Builder ID", &receipt.builder_id, SemanticTone::Neutral),
            row("Compatibility", &compatibility, SemanticTone::Neutral),
            row("Custom WIT imports", &imports, SemanticTone::Neutral),
            row(
                "Added access",
                &added,
                if diff.added.is_empty() {
                    SemanticTone::Neutral
                } else {
                    SemanticTone::Caution
                },
            ),
            row("Removed access", &removed, SemanticTone::Positive),
            row(
                "SBOM / licenses",
                &format!(
                    "{} / {}",
                    short_digest(&receipt.sbom_sha256),
                    short_digest(&receipt.licenses_sha256)
                ),
                SemanticTone::Neutral,
            ),
            row(
                "Revocation snapshot",
                &format!("sequence {}", receipt.revocation_sequence),
                SemanticTone::Positive,
            ),
            row(
                "Install behavior",
                "Disabled; no component, process, network, file, clipboard, credential, or PTY authority",
                SemanticTone::Positive,
            ),
        ];
        Self {
            title: format!("Review {}", receipt.manifest.display_name),
            subtitle: "Verify identity, source, and every requested capability".into(),
            icon: EcosystemIcon::Verified,
            tone: if diff.added.is_empty() {
                SemanticTone::Positive
            } else {
                SemanticTone::Caution
            },
            rows,
            primary_label: "Install disabled".into(),
            secondary_label: "Cancel".into(),
            default_focus: ReviewFocus::Cancel,
            restore_focus: true,
            reduced_motion,
            density: density(viewport_width),
            accessibility_name: format!(
                "Review {} version {} by publisher {}",
                receipt.manifest.display_name, receipt.version, receipt.publisher_id
            ),
            accessibility_description: format!(
                "Signed by key {}. {} added capabilities and {} removed capabilities. Installation does not enable execution.",
                receipt.key_id,
                diff.added.len(),
                diff.removed.len()
            ),
        }
    }
    pub fn model_consent(request: ModelConsentSurfaceRequest<'_>) -> Self {
        let ModelConsentSurfaceRequest {
            disclosure,
            redacted_preview,
            consent_sha256,
            selected_bytes,
            transferred_bytes,
            redaction_count,
            environment_risk,
            viewport_width,
            reduced_motion,
        } = request;
        let locality = format!("{:?}", disclosure.locality);
        let rows = vec![
            row("Provider", &disclosure.provider, SemanticTone::Neutral),
            row("Locality", &locality, SemanticTone::Neutral),
            row("Model", &disclosure.model, SemanticTone::Neutral),
            row(
                "Destination",
                &disclosure.destination,
                SemanticTone::Neutral,
            ),
            row("Purpose", &disclosure.purpose, SemanticTone::Neutral),
            row("Retention", &disclosure.retention, SemanticTone::Neutral),
            row(
                "Environment",
                &disclosure.environment_risk,
                SemanticTone::Caution,
            ),
            row(
                "Selected preview",
                &visible_preview(redacted_preview),
                SemanticTone::Accent,
            ),
            row(
                "Review fingerprint",
                &short_digest(consent_sha256),
                SemanticTone::Neutral,
            ),
            row(
                "Selected data",
                &format!(
                    "{selected_bytes} bytes; {transferred_bytes} after {redaction_count} redactions"
                ),
                SemanticTone::Accent,
            ),
            row(
                "Independent risk",
                &format!("{environment_risk:?}"),
                risk_tone(environment_risk),
            ),
        ];
        Self {
            title: "Share selected text?".into(),
            subtitle: "Only the reviewed redacted selection will be sent".into(),
            icon: EcosystemIcon::Model,
            tone: risk_tone(environment_risk),
            rows,
            primary_label: "Send this selection".into(),
            secondary_label: "Cancel".into(),
            default_focus: ReviewFocus::Cancel,
            restore_focus: true,
            reduced_motion,
            density: density(viewport_width),
            accessibility_name: "Model suggestion data review".into(),
            accessibility_description: format!(
                "{locality} provider {}, destination {}. Selected {selected_bytes} bytes; transferring {transferred_bytes} bytes after {redaction_count} redactions. No tools or automatic execution.",
                disclosure.provider, disclosure.destination
            ),
        }
    }
    pub fn lifecycle_state(name: &str, state: LifecycleState, detail: &str) -> Self {
        let (icon, tone, title) = match state {
            LifecycleState::Quarantined => (
                EcosystemIcon::Quarantine,
                SemanticTone::Caution,
                "Extension quarantined",
            ),
            LifecycleState::Revoked => (
                EcosystemIcon::Revoked,
                SemanticTone::Danger,
                "Extension revoked",
            ),
            LifecycleState::InstalledDisabled | LifecycleState::Disabled => (
                EcosystemIcon::Shield,
                SemanticTone::Neutral,
                "Extension disabled",
            ),
            _ => (
                EcosystemIcon::Package,
                SemanticTone::Neutral,
                "Extension status",
            ),
        };
        Self {
            title: title.into(),
            subtitle: name.into(),
            icon,
            tone,
            rows: vec![row("Status", detail, tone)],
            primary_label: "Close".into(),
            secondary_label: "Details".into(),
            default_focus: ReviewFocus::Details,
            restore_focus: true,
            reduced_motion: true,
            density: ReviewDensity::Compact,
            accessibility_name: format!("{name}, {title}"),
            accessibility_description: detail.into(),
        }
    }
}

fn row(label: &str, value: &str, tone: SemanticTone) -> ReviewRow {
    ReviewRow {
        label: label.into(),
        value: value.into(),
        accessible_value: format!("{label}: {value}"),
        tone,
    }
}

fn density(viewport_width: u32) -> ReviewDensity {
    if viewport_width < 720 {
        ReviewDensity::Compact
    } else {
        ReviewDensity::Comfortable
    }
}

fn risk_tone(risk: ModelRisk) -> SemanticTone {
    match risk {
        ModelRisk::ReadOnly => SemanticTone::Neutral,
        ModelRisk::Mutating => SemanticTone::Caution,
        ModelRisk::Destructive | ModelRisk::Privileged => SemanticTone::Danger,
    }
}

fn visible_preview(value: &str) -> String {
    value
        .chars()
        .map(|character| match character {
            '\n' => '↵',
            '\r' => '↵',
            '\t' => '⇥',
            value
                if value.is_control()
                    || matches!(
                        value,
                        '\u{061c}'
                            | '\u{200e}'
                            | '\u{200f}'
                            | '\u{202a}'..='\u{202e}'
                            | '\u{2066}'..='\u{2069}'
                    ) =>
            {
                '�'
            }
            value => value,
        })
        .collect()
}

fn bounded_list(values: &[String]) -> String {
    if values.is_empty() {
        return "None".into();
    }
    const VISIBLE: usize = 8;
    let shown = values
        .iter()
        .take(VISIBLE)
        .cloned()
        .collect::<Vec<_>>()
        .join(", ");
    if values.len() > VISIBLE {
        format!("{shown}, and {} more", values.len() - VISIBLE)
    } else {
        shown
    }
}
fn capability_list(values: &[Capability]) -> String {
    if values.is_empty() {
        "None".into()
    } else {
        values
            .iter()
            .map(|value| format!("{value:?}"))
            .collect::<Vec<_>>()
            .join(", ")
    }
}

fn short_digest(value: &str) -> String {
    value.get(..12).unwrap_or(value).to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn receipt() -> VerificationReceipt {
        VerificationReceipt {
            schema_version: 1,
            extension_id: "example.extension".into(),
            version: "1.0.0".into(),
            publisher_id: "example.publisher".into(),
            package_sha256: "a".repeat(64),
            content_sha256: "b".repeat(64),
            provenance_sha256: "c".repeat(64),
            sbom_sha256: "d".repeat(64),
            licenses_sha256: "e".repeat(64),
            key_id: "example.key".into(),
            source_uri: "https://example.invalid/source".into(),
            source_revision: "0123456789abcdef".into(),
            builder_id: "example.builder".into(),
            built_at_unix: 10,
            signature_expires_at_unix: 100,
            verified_at_unix: 20,
            revocation_sequence: 3,
            manifest: crate::EcosystemManifest {
                schema_version: 1,
                extension_id: "example.extension".into(),
                display_name: "Example".into(),
                description: "Example".into(),
                publisher_id: "example.publisher".into(),
                version: "1.0.0".into(),
                kind: crate::ExtensionKind::Component,
                compatibility: crate::Compatibility {
                    sdk_major: 1,
                    sdk_minor_minimum: 0,
                    sdk_minor_maximum: 0,
                },
                world: crate::WIT_WORLD.into(),
                imports: vec![],
                capabilities: vec![Capability::SelectedInputReadOnce],
                action_pack_entry: None,
            },
        }
    }
    #[test]
    fn reviews_use_text_not_color_default_to_cancel_and_restore_focus() {
        let diff = CapabilityDiff {
            added: vec![Capability::SelectedInputReadOnce],
            removed: vec![],
            unchanged: vec![],
            fresh_review_required: true,
        };
        let receipt = receipt();
        let surface = EcosystemReviewSurface::package_review(&receipt, &diff, 400, true);
        assert_eq!(surface.default_focus, ReviewFocus::Cancel);
        assert!(surface.restore_focus);
        assert!(surface.reduced_motion);
        assert_eq!(surface.density, ReviewDensity::Compact);
        assert!(surface
            .accessibility_description
            .contains("added capabilities"));
        assert!(surface
            .rows
            .iter()
            .all(|row| row.accessible_value.contains(&row.label)));
        assert!(surface
            .rows
            .iter()
            .any(|row| row.label == "Publisher ID (exact)"));
        assert!(surface
            .rows
            .iter()
            .any(|row| row.label == "Install behavior"));
    }

    #[test]
    fn model_review_discloses_redacted_preview_locality_purpose_and_safe_controls() {
        let disclosure = ModelDisclosure {
            provider: "Example".into(),
            locality: crate::ModelLocality::Remote,
            model: "model-v1".into(),
            destination: "example.invalid".into(),
            purpose: "Explain selection".into(),
            retention: "No retention".into(),
            environment_risk: "Production".into(),
        };
        let digest = "f".repeat(64);
        let surface = EcosystemReviewSurface::model_consent(ModelConsentSurfaceRequest {
            disclosure: &disclosure,
            redacted_preview: "TOKEN=[REDACTED]\nnext\u{202e}",
            consent_sha256: &digest,
            selected_bytes: 24,
            transferred_bytes: 22,
            redaction_count: 1,
            environment_risk: ModelRisk::ReadOnly,
            viewport_width: 640,
            reduced_motion: true,
        });
        assert!(surface.rows.iter().any(|row| row.label == "Purpose"));
        let preview = surface
            .rows
            .iter()
            .find(|row| row.label == "Selected preview")
            .unwrap();
        assert!(preview.value.contains("[REDACTED]"));
        assert!(!preview.value.contains('\u{202e}'));
        assert!(surface
            .accessibility_description
            .contains("Remote provider"));
    }
}
