#![no_main]

use automexia_devops::sanitize_label;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(text) = std::str::from_utf8(data) {
        let sanitized = sanitize_label(text);
        assert!(sanitized.chars().count() <= 96);
        assert!(!sanitized.chars().any(char::is_control));
    }
});
