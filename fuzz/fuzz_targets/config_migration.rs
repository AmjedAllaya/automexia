#![no_main]

use libfuzzer_sys::fuzz_target;
use rio_backend::config::product::validate_legacy_config;

fuzz_target!(|data: &[u8]| {
    if let Ok(text) = std::str::from_utf8(data) {
        let _ = validate_legacy_config(text);
    }
});
