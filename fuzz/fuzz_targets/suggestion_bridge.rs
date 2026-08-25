#![no_main]

use automexia_devops::suggestions::{
    decode_reply_frame, decode_replacement_frame, decode_request_frame, decode_submission_frame,
    encode_request_frame, SuggestionLimits,
};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if data.len() > SuggestionLimits::FRAME_BYTES.saturating_add(4) {
        return;
    }
    let _ = decode_submission_frame(data);
    let _ = decode_replacement_frame(data);
    let _ = decode_reply_frame(data);
    if let Ok(request) = decode_request_frame(data) {
        let encoded =
            encode_request_frame(&request).expect("validated request re-encodes");
        let decoded = decode_request_frame(&encoded).expect("encoded frame decodes");
        assert_eq!(decoded, request);
    }
});
