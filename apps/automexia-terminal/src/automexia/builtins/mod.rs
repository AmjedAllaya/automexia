use super::api::ExtensionManifest;

pub mod aws;
pub mod azure;
pub mod devops;

/// Trusted first-party catalog compiled into the application.
///
/// Keeping registration here prevents the marketplace UI/model from depending
/// on concrete extension implementations. A future package registry can replace
/// this static slice without changing renderer or terminal-engine contracts.
#[cfg(not(target_arch = "wasm32"))]
pub const MANIFESTS: &[ExtensionManifest] =
    &[aws::MANIFEST, azure::MANIFEST, devops::MANIFEST];
#[cfg(target_arch = "wasm32")]
pub const MANIFESTS: &[ExtensionManifest] = &[];

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    #[test]
    fn aws_is_independently_registered_and_disabled() {
        let manifests = std::hint::black_box(MANIFESTS);
        let aws = manifests
            .iter()
            .find(|manifest| manifest.id == automexia_devops_aws::ID)
            .expect("AWS manifest is registered");
        assert!(!aws.default_enabled);
        assert_eq!(
            aws.capabilities,
            &[
                automexia_extension_api::Capability::ProcessSpawn,
                automexia_extension_api::Capability::Network,
            ]
        );
    }

    #[test]
    fn azure_is_independently_registered_and_disabled() {
        let manifests = std::hint::black_box(MANIFESTS);
        let azure = manifests
            .iter()
            .find(|manifest| manifest.id == automexia_devops_azure::ID)
            .expect("Azure manifest is registered");
        assert!(!azure.default_enabled);
        assert_eq!(
            azure.capabilities,
            &[
                automexia_extension_api::Capability::ProcessSpawn,
                automexia_extension_api::Capability::Network,
            ]
        );
    }
}
