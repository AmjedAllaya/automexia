
# Rust Core Architecture — Current-Compatible Principles

**Current repository:** Rust 1.96.1, edition 2021 according to the supplied source audit.
**Do not mandate:** Rust 1.98, edition 2024, Tokio, SQLite, Wasmtime, or a large crate split without a separate migration/ADR and evidence.

Keep:
- safe Rust by default;
- `#![forbid(unsafe_code)]` where practical;
- tiny reviewed unsafe/FFI/platform islands;
- typed IDs/generations;
- bounded queues;
- explicit cancellation/ownership;
- sensitive wrappers/redaction by construction.

Prefer adapting the existing workspace ownership rather than decomposing it into many new crates for aesthetic consistency.

Upgrade compiler/edition only when:
- dependencies support it;
- CI/native builds pass;
- migration yields a concrete benefit;
- rollback is straightforward.
