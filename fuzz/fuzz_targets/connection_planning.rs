#![no_main]

use automexia_devops::connections::{
    parse_profile_document_json, parse_profile_json, parse_recipe_document_json,
    parse_recipe_json,
};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = parse_profile_json(data);
    let _ = parse_recipe_json(data);
    let _ = parse_profile_document_json(data);
    let _ = parse_recipe_document_json(data);
});
