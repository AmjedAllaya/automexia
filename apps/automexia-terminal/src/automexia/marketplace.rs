use super::api::ExtensionManifest;
use super::builtins;

#[derive(Clone, Debug)]
pub struct MarketItem {
    pub id: String,
    pub name: String,
    pub description: String,
    pub installed: bool,
}

pub fn descriptor(id: &str) -> Option<&'static ExtensionManifest> {
    builtins::MANIFESTS
        .iter()
        .find(|candidate| candidate.id == id)
}

pub fn descriptors() -> &'static [ExtensionManifest] {
    builtins::MANIFESTS
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EcosystemAvailability {
    AcceptedSourceDisabled,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EcosystemStatus {
    pub availability: EcosystemAvailability,
    pub accepted_contract_sha256: &'static str,
    pub local_bundle_review: bool,
    pub disabled_install: bool,
    pub component_execution: bool,
    pub downloads: bool,
    pub model_provider_calls: bool,
    pub detail: &'static str,
}

pub const fn ecosystem_status() -> EcosystemStatus {
    EcosystemStatus {
        availability: EcosystemAvailability::AcceptedSourceDisabled,
        accepted_contract_sha256: automexia_ecosystem::ACCEPTED_CONTRACT_SHA256,
        local_bundle_review: true,
        disabled_install: true,
        component_execution: false,
        downloads: false,
        model_provider_calls: false,
        detail: "Signed local package review and disabled installation are available; execution, downloads, and model calls remain release-gated.",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepted_source_status_never_implies_activation_or_network_authority() {
        let status = ecosystem_status();
        assert!(status.local_bundle_review && status.disabled_install);
        assert!(
            !status.component_execution
                && !status.downloads
                && !status.model_provider_calls
        );
        assert_eq!(
            status.accepted_contract_sha256,
            automexia_ecosystem::ACCEPTED_CONTRACT_SHA256
        );
    }
}
