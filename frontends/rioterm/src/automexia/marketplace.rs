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
