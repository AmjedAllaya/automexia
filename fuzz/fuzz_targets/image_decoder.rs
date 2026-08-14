#![no_main]

use automexia_image::{
    decode_bounded_bytes, has_supported_extension, image_path_tokens_in_line,
    path_token_at_line,
};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = decode_bounded_bytes(data);
    let text = String::from_utf8_lossy(data);
    let character_count = text.chars().count();
    for token in image_path_tokens_in_line(&text) {
        assert!(token.start < token.end);
        assert!(token.end <= character_count);
        assert!(has_supported_extension(&token.text));
        assert_eq!(
            path_token_at_line(&text, token.start).as_deref(),
            Some(token.text.as_str())
        );
    }
});
