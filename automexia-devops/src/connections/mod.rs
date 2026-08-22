//! Provider-neutral, capability-free connection and automation planning.
//!
//! These contracts intentionally own no filesystem, process, network, provider,
//! credential, PTY, listener, renderer, or GPU authority. They validate public
//! intent and produce immutable dry-run plans for later application-owned review.

mod direct_openssh;
mod documents;
mod model;
mod openssh_tunnels;
mod planner;
mod state;
mod validation;

pub use direct_openssh::{
    parse_direct_openssh_agent_identities, prepare_direct_openssh,
    prepare_direct_openssh_identity_status, review_direct_openssh,
    review_direct_openssh_m4, validate_direct_openssh_arguments,
    DirectOpenSshDestinationKind, DirectOpenSshHostKeyEvidence,
    DirectOpenSshHostKeyProvenance, DirectOpenSshHostTrustEvidence,
    DirectOpenSshHostTrustPolicy, DirectOpenSshIdentityReadiness,
    DirectOpenSshIdentityStatusRequest, DirectOpenSshLaunchBinding,
    DirectOpenSshPreparation, DirectOpenSshPublicIdentity, DirectOpenSshRequest,
    DirectOpenSshReview, DirectOpenSshReviewEvidence, DirectOpenSshRoute,
    DIRECT_OPENSSH_IDENTITY_STATUS_TIMEOUT_MS, DIRECT_OPENSSH_MANAGED_OPTIONS,
    DIRECT_OPENSSH_ROUTED_OPTIONS, MAX_DIRECT_OPENSSH_DESTINATION_BYTES,
    MAX_DIRECT_OPENSSH_IDENTITIES, MAX_DIRECT_OPENSSH_IDENTITY_OUTPUT_BYTES,
    MAX_DIRECT_OPENSSH_JUMPS, MAX_DIRECT_OPENSSH_ROUTE_BYTES,
    MAX_DIRECT_OPENSSH_USER_BYTES,
};
pub use documents::{
    parse_profile_document_json, parse_recipe_document_json, validate_profile_document,
    validate_recipe_document,
};
pub use model::*;
pub use openssh_tunnels::{
    DirectOpenSshTunnelConfirmation, DirectOpenSshTunnelDescriptor,
    DirectOpenSshTunnelEvent, DirectOpenSshTunnelLifecycle,
    DirectOpenSshTunnelLifecycleOwner, DirectOpenSshTunnelPlan, DirectOpenSshTunnelState,
    DirectOpenSshTunnelStatus, DirectOpenSshTunnelTransport,
    DIRECT_OPENSSH_TUNNEL_MANAGED_OPTIONS, MAX_DIRECT_OPENSSH_TUNNEL_HOST_BYTES,
};
pub use planner::{fingerprint_profile, fingerprint_recipe, resolve_connection_plan};
pub use state::{apply_auth_event, apply_result_event};
pub use validation::{
    parse_connection_definition_json, parse_connection_intent_json,
    parse_connection_observation_json, parse_connection_receipt_json,
    parse_connection_review_json, parse_profile_json, parse_recipe_json,
    validate_connection_definition, validate_connection_intent,
    validate_connection_observation, validate_connection_receipt,
    validate_connection_review, validate_profile, validate_recipe,
};
