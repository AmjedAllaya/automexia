# Private planning boundary

This retained compatibility path intentionally contains no public feature,
architecture, testing, roadmap, distribution, or commercial details. Historical
material is preserved only in ignored private documentation.

Do not infer a product from this filename, source package, test fixture, or
checker. Public behavior is limited to [the feature catalog](FEATURES.md).

# Extension ecosystem testing

Status: source assurance contract; release activation remains disabled.

## Local gates

Run `cargo test -p automexia-ecosystem` and the repository-owned D7/CP6 policy,
contract, mutation, architecture, confidentiality, and full validation checks.
Exercise manifest parsing, capability denial, incompatible versions, hostile
payloads, size/count/depth ceilings, cancellation, stale generations, crash,
restart, disable, uninstall, rollback, and final cleanup.

The `component-host` tests must independently observe exact broker decisions,
argument vectors, process trees, handles, files, messages, redacted logs, and
forbidden side effects. A mocked success response does not prove isolation.

## Native evidence

Windows x86_64 requires native process, job/handle, package, signature, update,
disable, uninstall, and rollback evidence. Equivalent native Linux and macOS
sandbox, permission, process-tree, packaging, signing, and cleanup evidence is
required for any platform claim. Cross-compilation is not a native run.

## Resource and accessibility evidence

Measure startup impact, request latency, CPU, memory, handles, threads,
processes, queues, caches, storage growth, repeated lifecycle operations, and
final cleanup. Verify keyboard-only review/consent, focus restoration, semantic
status, high contrast, reduced motion, scaling, localization, and applicable
native screen readers.

External evidence still required includes long-duration native stress,
independent security review, final signed artifact identity, revocation and
offline recovery drills, and human accessibility assessment.
