#![no_main]

use automexia_terminal::automexia::ghostty_migration::preview;
use libfuzzer_sys::fuzz_target;
use std::fs;

const MAX_FUZZ_BYTES: usize = 256 * 1024;

fuzz_target!(|data: &[u8]| {
    let Ok(directory) = tempfile::tempdir() else {
        return;
    };
    let root = directory.path().join("config");
    let first = directory.path().join("first.conf");
    let second = directory.path().join("second.conf");
    let bounded = &data[..data.len().min(MAX_FUZZ_BYTES)];

    // Raw bytes cover malformed UTF-8 and oversized/fragmented binding text.
    if fs::write(&root, bounded).is_ok() {
        let _ = preview(&root);
    }

    // A controlled graph guarantees include traversal, optional includes, and
    // canonical cycle detection are exercised for arbitrary binding payloads.
    let payload = String::from_utf8_lossy(bounded);
    let root_text = format!(
        "# fuzz root\nkeybind = {payload}\nconfig-file = first.conf\nconfig-file = ?missing.conf\n"
    );
    let first_text = "keybind = ctrl+shift+t=new_tab\nconfig-file = second.conf\n";
    let second_text = if bounded.first().is_some_and(|byte| byte & 1 == 1) {
        "config-file = config\n"
    } else {
        "keybind = global:ctrl+g=toggle_quick_terminal\n"
    };
    if fs::write(&root, root_text).is_ok()
        && fs::write(&first, first_text).is_ok()
        && fs::write(&second, second_text).is_ok()
    {
        let _ = preview(&root);
    }
});
