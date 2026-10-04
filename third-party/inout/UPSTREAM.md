# InOut source provenance

This directory contains RustCrypto `inout` 0.2.2, licensed under MIT OR Apache-2.0.
The original licenses are included. Source: <https://crates.io/crates/inout/0.2.2>.
Registry archive SHA-256: `4250ce6452e92010fdf7268ccc5d14faa80bb12fc741938534c58f16804e03c7`.
Upstream source revision: `2d5eac49c253b19a064b33fc1f9ca7839732cb30`.

Automexia changes preserve raw pointer provenance when splitting a reserved
buffer, document the invariant, and add in-place, separate-buffer, repeated-use,
empty, zero-sized and non-Copy regression tests. The public API is unchanged.
Formatting follows this workspace. Publishing this local package is disabled.

The reviewed local source is pinned by the recovery architecture checker and
tested as a workspace member. Its Cargo Vet policy explicitly distinguishes
this modified source from the unmodified registry package; it does not certify
the registry package. All transitive registry dependencies retain Cargo Vet
requirements. Updating any local file requires source review and a corresponding
fingerprint update. Replace this override only after an upstream version passes
the same tests and normal dependency review.
