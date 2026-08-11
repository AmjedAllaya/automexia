#![no_main]

use automexia_terminal::automexia::migration::validate_legacy_config;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(text) = std::str::from_utf8(data) {
        let _ = validate_legacy_config(text);
    }
});
