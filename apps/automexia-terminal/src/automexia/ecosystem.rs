//! Explicit D7/CP6 application adapter.
//!
//! Nothing in startup, terminal input, PTY, rendering, hover, selection, or
//! ordinary typing calls this module. Local inspection and disabled install are
//! explicit operations; execution, downloads, and provider calls remain denied.

use std::{collections::BTreeSet, path::Path, time::Duration};

use automexia_command_productivity::actions::QuickAction;
use automexia_ecosystem::{
    capability_diff, EcosystemReviewSurface, GrantBinding, ModelConsentSurfaceRequest,
    ModelDisclosure, ModelReview, ModelRisk, ModelSuggestionError, RevocationSnapshot,
    SelectedInput, TrustedPublisher,
};
use automexia_ecosystem_runtime::{
    plan_action_pack_import, read_and_verify_local_bundle, ActionPackError,
    ActionPackImportPlan, BundleError, CleanupReceipt, InstalledGeneration, PackageStore,
    StoreError, VerificationContext, VerifiedBundle,
};

#[derive(Debug)]
pub struct PendingInstallReview {
    pub surface: EcosystemReviewSurface,
    bundle: VerifiedBundle,
}

impl PendingInstallReview {
    pub fn receipt(&self) -> &automexia_ecosystem::VerificationReceipt {
        &self.bundle.receipt
    }
}

#[derive(Debug)]
pub enum EcosystemControllerError {
    Bundle(BundleError),
    Store(StoreError),
    ActionPack(ActionPackError),
    Model(ModelSuggestionError),
    ActivationDenied,
    DownloadsDisabled,
}

impl std::fmt::Display for EcosystemControllerError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Bundle(error) => {
                write!(formatter, "package verification failed: {error}")
            }
            Self::Store(error) => write!(formatter, "package store failed: {error}"),
            Self::ActionPack(error) => {
                write!(formatter, "action pack review failed: {error}")
            }
            Self::Model(error) => {
                write!(formatter, "model suggestion review failed: {error}")
            }
            Self::ActivationDenied => formatter.write_str(
                "ecosystem execution is disabled pending protected release authorization",
            ),
            Self::DownloadsDisabled => formatter.write_str(
                "ecosystem downloads are disabled pending a separate network ADR",
            ),
        }
    }
}

impl std::error::Error for EcosystemControllerError {}

impl From<BundleError> for EcosystemControllerError {
    fn from(error: BundleError) -> Self {
        Self::Bundle(error)
    }
}
impl From<StoreError> for EcosystemControllerError {
    fn from(error: StoreError) -> Self {
        Self::Store(error)
    }
}
impl From<ActionPackError> for EcosystemControllerError {
    fn from(error: ActionPackError) -> Self {
        Self::ActionPack(error)
    }
}
impl From<ModelSuggestionError> for EcosystemControllerError {
    fn from(error: ModelSuggestionError) -> Self {
        Self::Model(error)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct LocalBundleInspectionRequest<'a> {
    pub path: &'a Path,
    pub trusted_publishers: &'a [TrustedPublisher],
    pub revocation: &'a RevocationSnapshot,
    pub now_unix: u64,
    pub minimum_revocation_sequence: u64,
    pub viewport_width: u32,
    pub reduced_motion: bool,
}

#[derive(Debug)]
pub struct EcosystemController {
    store: PackageStore,
}

impl EcosystemController {
    pub fn open(
        store_root: impl Into<std::path::PathBuf>,
    ) -> Result<Self, EcosystemControllerError> {
        Ok(Self {
            store: PackageStore::open(store_root)?,
        })
    }

    pub fn inspect_local_bundle(
        &self,
        request: LocalBundleInspectionRequest<'_>,
    ) -> Result<PendingInstallReview, EcosystemControllerError> {
        let LocalBundleInspectionRequest {
            path,
            trusted_publishers,
            revocation,
            now_unix,
            minimum_revocation_sequence,
            viewport_width,
            reduced_motion,
        } = request;
        let bundle = read_and_verify_local_bundle(
            path,
            VerificationContext {
                now_unix,
                minimum_revocation_sequence,
                supported_sdk_minor: 0,
                trusted_publishers,
                revocation,
            },
        )?;
        let diff = capability_diff(&[], &bundle.receipt.manifest.capabilities);
        let surface = EcosystemReviewSurface::package_review(
            &bundle.receipt,
            &diff,
            viewport_width,
            reduced_motion,
        );
        Ok(PendingInstallReview { surface, bundle })
    }
    pub fn install_disabled(
        &mut self,
        review: PendingInstallReview,
        available_bytes: u64,
        now_unix: u64,
    ) -> Result<InstalledGeneration, EcosystemControllerError> {
        Ok(self
            .store
            .install_verified(&review.bundle, available_bytes, now_unix)?)
    }

    pub fn review_action_pack(
        &self,
        review: &PendingInstallReview,
        existing_actions: &[QuickAction],
        now_unix: u64,
        minimum_revocation_sequence: u64,
        revocation: &RevocationSnapshot,
    ) -> Result<ActionPackImportPlan, EcosystemControllerError> {
        let existing = existing_actions
            .iter()
            .map(|action| action.id.clone())
            .collect::<BTreeSet<_>>();
        Ok(plan_action_pack_import(
            &review.bundle,
            &existing,
            now_unix,
            minimum_revocation_sequence,
            revocation,
        )?)
    }

    pub fn model_review(
        &self,
        request_id: u64,
        route_id: &str,
        generation: u64,
        disclosure: ModelDisclosure,
        selection: SelectedInput,
    ) -> Result<ModelReview, EcosystemControllerError> {
        Ok(ModelReview::new(
            request_id, route_id, generation, disclosure, &selection,
        )?)
    }

    pub fn model_consent_surface(
        &self,
        review: &ModelReview,
        risk: ModelRisk,
        viewport_width: u32,
        reduced_motion: bool,
    ) -> EcosystemReviewSurface {
        EcosystemReviewSurface::model_consent(ModelConsentSurfaceRequest {
            disclosure: &review.disclosure,
            redacted_preview: &review.preview.redacted_text,
            consent_sha256: &review.consent_sha256,
            selected_bytes: review.preview.original_bytes,
            transferred_bytes: review.preview.transferred_bytes,
            redaction_count: review.preview.findings.len(),
            environment_risk: risk,
            viewport_width,
            reduced_motion,
        })
    }

    pub fn disable(
        &mut self,
        extension_id: &str,
    ) -> Result<CleanupReceipt, EcosystemControllerError> {
        Ok(self.store.disable(extension_id)?)
    }

    pub fn kill_switch(&mut self) -> Result<CleanupReceipt, EcosystemControllerError> {
        Ok(self.store.kill_switch()?)
    }

    pub fn uninstall(
        &mut self,
        extension_id: &str,
    ) -> Result<CleanupReceipt, EcosystemControllerError> {
        Ok(self.store.uninstall(extension_id)?)
    }

    pub fn activate_component(
        &self,
        _extension_id: &str,
        _deadline: Duration,
    ) -> Result<(), EcosystemControllerError> {
        Err(EcosystemControllerError::ActivationDenied)
    }

    pub fn refresh_distribution(&self) -> Result<(), EcosystemControllerError> {
        Err(EcosystemControllerError::DownloadsDisabled)
    }

    pub fn grant_execution(
        &self,
        _grant: GrantBinding,
    ) -> Result<(), EcosystemControllerError> {
        Err(EcosystemControllerError::ActivationDenied)
    }
}
#[cfg(test)]
mod tests {
    use automexia_ecosystem::{Capability, ModelLocality};

    use super::*;

    fn controller() -> EcosystemController {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.keep().join("ecosystem");
        EcosystemController::open(root).unwrap()
    }

    #[test]
    fn product_adapter_denies_activation_downloads_and_grants_without_side_effects() {
        let controller = controller();
        assert!(matches!(
            controller.activate_component("example.extension", Duration::from_millis(10)),
            Err(EcosystemControllerError::ActivationDenied)
        ));
        assert!(matches!(
            controller.refresh_distribution(),
            Err(EcosystemControllerError::DownloadsDisabled)
        ));
        let grant = GrantBinding {
            publisher_id: "example.publisher".into(),
            extension_id: "example.extension".into(),
            version: "1.0.0".into(),
            package_sha256: "a".repeat(64),
            capability: Capability::SuggestionPublish,
            exact_scope: "pane/1".into(),
            profile_id: "dev".into(),
            expires_at_unix: 100,
            generation: 1,
        };
        assert!(matches!(
            controller.grant_execution(grant),
            Err(EcosystemControllerError::ActivationDenied)
        ));
    }

    #[test]
    fn model_review_uses_only_explicit_selection_and_produces_a_cancel_first_surface() {
        let controller = controller();
        let review = controller
            .model_review(
                7,
                "pane/1",
                2,
                ModelDisclosure {
                    provider: "Example".into(),
                    locality: ModelLocality::Remote,
                    model: "model-v1".into(),
                    destination: "example.invalid".into(),
                    purpose: "Explain selection".into(),
                    retention: "No retention".into(),
                    environment_risk: "Development".into(),
                },
                SelectedInput::new("TOKEN=canary\ngit status").unwrap(),
            )
            .unwrap();
        assert!(!review.preview.redacted_text.contains("canary"));
        let surface =
            controller.model_consent_surface(&review, ModelRisk::ReadOnly, 640, true);
        assert_eq!(
            surface.default_focus,
            automexia_ecosystem::ReviewFocus::Cancel
        );
        assert!(surface.restore_focus);
        assert!(surface.rows.iter().any(
            |row| row.label == "Selected preview" && row.value.contains("[REDACTED]")
        ));
    }
}
