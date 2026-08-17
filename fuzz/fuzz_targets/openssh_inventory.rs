#![no_main]

use std::fs;

use automexia_devops_ssh::{scan_inventory, GrantKind, InventoryGrant, InventoryLimits};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(root) = tempfile::tempdir() else {
        return;
    };
    let config = root.path().join("config");
    if fs::write(&config, data).is_err() {
        return;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if fs::set_permissions(&config, fs::Permissions::from_mode(0o600)).is_err() {
            return;
        }
    }
    let Ok(grant) =
        InventoryGrant::new("fuzz-config", root.path(), [&config], GrantKind::User)
    else {
        return;
    };
    let _ = scan_inventory(&[grant], InventoryLimits::default(), 1);
});
