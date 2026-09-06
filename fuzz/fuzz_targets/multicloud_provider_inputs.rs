#![no_main]

use automexia_connectivity::connections::{OpaqueReference, ProviderKind};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|bytes: &[u8]| {
    let _ = automexia_devops_aws::parse_public_config(bytes);
    let _ = automexia_devops_azure::parse_public_accounts(bytes);
    let _ = automexia_devops_azure::parse_public_account(bytes);
    let _ = automexia_devops_gcp::parse_public_configuration("fuzz", bytes);
    let _ = automexia_devops_kubernetes::parse_private_transient_source(
        OpaqueReference::new("fuzz.source"),
        OpaqueReference::new("fuzz.grant"),
        ProviderKind::Kubernetes,
        bytes,
        0,
    );
    let _ = automexia_devops_teleport::parse_public_status(bytes);
});
