#![no_main]

use automexia_terminal::automexia::builtins::devops::classify_row_text;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(text) = std::str::from_utf8(data) {
        let _ = classify_row_text(text);
    }
});
