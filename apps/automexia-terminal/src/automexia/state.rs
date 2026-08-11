#[cfg(not(target_arch = "wasm32"))]
use std::fs;
#[cfg(not(target_arch = "wasm32"))]
use std::io::Write;
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
    let mut temporary = tempfile::NamedTempFile::new_in(parent)
        .map_err(|error| format!("could not stage {}: {error}", path.display()))?;
    temporary
        .write_all(contents.as_bytes())
        .and_then(|_| temporary.as_file().sync_all())
        .map_err(|error| format!("could not stage {}: {error}", path.display()))?;
    temporary.persist(path).map(|_| ()).map_err(|error| {
        format!("could not activate {}: {}", path.display(), error.error)
    })
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
    // Never mutate the Rio source tree. The Automexia `disabled` marker has
    // higher precedence than both current and legacy installed markers.
    let root = root_dir();
    write_disabled_marker(&root, manifest)
}

#[cfg(not(target_arch = "wasm32"))]
fn write_disabled_marker(
    root: &Path,
    manifest: &ExtensionManifest,
) -> Result<(), String> {
    remove_if_present(&marker_at(root, manifest.id, "installed"))?;
    atomic_write(
        &marker_at(root, manifest.id, "disabled"),
        &marker_contents(manifest, false),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::automexia::builtins::devops;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT: AtomicU64 = AtomicU64::new(1);

    fn temporary_root(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "automexia-state-{name}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ))
    }

    #[test]
    fn local_disable_marker_does_not_touch_legacy_state() {
        let root = temporary_root("disable");
        let legacy = root.join("rio");
        let current = root.join("automexia");
        let manifest = super::super::marketplace::descriptor(devops::ID).unwrap();
        let legacy_installed = marker_at(&legacy, manifest.id, "installed");
        fs::create_dir_all(legacy_installed.parent().unwrap()).unwrap();
        fs::write(&legacy_installed, "enabled=true\n").unwrap();
        let current_installed = marker_at(&current, manifest.id, "installed");
        fs::create_dir_all(current_installed.parent().unwrap()).unwrap();
        fs::write(&current_installed, "enabled=true\n").unwrap();

        write_disabled_marker(&current, manifest).unwrap();

        assert!(legacy_installed.is_file());
        assert!(!current_installed.exists());
        assert!(marker_at(&current, manifest.id, "disabled").is_file());
        let _ = fs::remove_dir_all(root);
    }
}
