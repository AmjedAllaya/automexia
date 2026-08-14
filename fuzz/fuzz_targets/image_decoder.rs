#![no_main]

use automexia_image::decode_bounded_bytes;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = decode_bounded_bytes(data);
});
