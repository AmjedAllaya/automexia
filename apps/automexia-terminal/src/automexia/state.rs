#[cfg(not(target_arch = "wasm32"))]
use std::fs;
#[cfg(not(target_arch = "wasm32"))]
use std::path::{Path, PathBuf};

use super::api::ExtensionManifest;

#[cfg(not(target_arch = "wasm32"))]
fn root_dir() -> PathBuf {
    rio_backend::config::config_dir_path().join("extensions")
}

#[cfg(not(target_arch = "wasm32"))]
fn legacy_root_dir() -> PathBuf {
    rio_backend::config::product::legacy_config_dir()
        .join("automexia")
        .join("extensions")
}

#[cfg(not(target_arch = "wasm32"))]
fn marker_at(root: &Path, id: &str, marker: &str) -> PathBuf {
    root.join(id).join(marker)
}

/// Resolve activation with an explicit-disable marker so trusted first-party
/// defaults can be enabled on a fresh install without preventing the user from
/// turning them off permanently.
#[cfg(not(target_arch = "wasm32"))]
pub fn is_installed(manifest: &ExtensionManifest) -> bool {
    let root = root_dir();
    if marker_at(&root, manifest.id, "disabled").is_file() {
        return false;
    }
    if marker_at(&root, manifest.id, "installed").is_file()
        || marker_at(&legacy_root_dir(), manifest.id, "installed").is_file()
    {
        return true;
    }
    manifest.default_enabled
}

#[cfg(target_arch = "wasm32")]
pub fn is_installed(_manifest: &ExtensionManifest) -> bool {
    false
}

#[cfg(not(target_arch = "wasm32"))]
pub fn set_installed(
    manifest: &ExtensionManifest,
    installed: bool,
) -> Result<(), String> {
    if installed {
        install_marker(manifest)
    } else {
        disable_marker(manifest)
    }
}

#[cfg(target_arch = "wasm32")]
pub fn set_installed(
    _manifest: &ExtensionManifest,
    _installed: bool,
) -> Result<(), String> {
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
fn marker_contents(manifest: &ExtensionManifest, enabled: bool) -> String {
    let capabilities = manifest
        .capabilities
        .iter()
        .map(|capability| capability.label())
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "id={}\nname={}\nversion={}\nenabled={}\ncapabilities={}\n",
        manifest.id, manifest.name, manifest.version, enabled, capabilities
    )
}

#[cfg(not(target_arch = "wasm32"))]
fn atomic_write(path: &Path, contents: &str) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("invalid extension marker path: {}", path.display()))?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("could not create {}: {error}", parent.display()))?;
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("marker");
    let temporary = parent.join(format!(".{file_name}.{}.tmp", std::process::id()));
    fs::write(&temporary, contents)
        .map_err(|error| format!("could not stage {}: {error}", path.display()))?;
    fs::rename(&temporary, path).map_err(|error| {
        let _ = fs::remove_file(&temporary);
        format!("could not activate {}: {error}", path.display())
    })?;
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
fn remove_if_present(path: &Path) -> Result<(), String> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("could not remove {}: {error}", path.display())),
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn install_marker(manifest: &ExtensionManifest) -> Result<(), String> {
    let root = root_dir();
    let extension_root = root.join(manifest.id);
    remove_if_present(&extension_root.join("disabled"))?;
    atomic_write(
        &extension_root.join("installed"),
        &marker_contents(manifest, true),
    )
}

#[cfg(not(target_arch = "wasm32"))]
fn disable_marker(manifest: &ExtensionManifest) -> Result<(), String> {
    // Remove both current and legacy installed markers first. The explicit
    // Automexia `disabled` marker then records the user's decision even though
    // this first-party extension is enabled by default on a fresh install.
    let root = root_dir();
    remove_if_present(&marker_at(&root, manifest.id, "installed"))?;
    remove_if_present(&marker_at(&legacy_root_dir(), manifest.id, "installed"))?;
    atomic_write(
        &marker_at(&root, manifest.id, "disabled"),
        &marker_contents(manifest, false),
    )
}
