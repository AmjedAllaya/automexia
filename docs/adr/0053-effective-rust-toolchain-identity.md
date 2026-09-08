# ADR 0053: Effective Rust toolchain identity

Status: Accepted for source tooling; exact-candidate CI remains required.

## Decision

`rust-toolchain.toml` owns the development and release compiler. The workspace
`rust-version` independently declares the minimum supported compiler. Both are
currently 1.96.1; changing one does not silently change the other. The existing
dated nightly lane remains separate for fuzzing, Miri and sanitizers.

Workflows select `RUSTUP_TOOLCHAIN`, install the required compiler and verify
the actual rustup selection plus both rustc and Cargo versions/commit identities
before use. These bounded, content-free receipts remain in workflow logs tied
to the checked-out commit; no additional public release asset is introduced.
The explicit MSRV step checks every workspace target and feature with the
locked graph, using the manifest minimum. Current equality permits reuse of
compiler artifacts without confusing the two support contracts.

Integration retains the tested environment-file initializer before sccache
startup, with the reviewed `automexia-rust-1.96.1-v2` generation. It derives the
version from the verified selector rather than an unsupported job-level `env`
expression. Both the initializer bytes/order and exact documented cache identity
are checked; malformed or missing MSRV manifests fail with redacted diagnostics.

The [rustup precedence rules](https://rust-lang.github.io/rustup/overrides.html)
place the repository toolchain file above the global default. Installing a
different compiler and setting that default therefore did not establish the
compiler named by the previous CI environment/cache label. Use explicit
selection instead of changing user/global defaults or upgrading dependencies.
The [Cargo MSRV contract](https://doc.rust-lang.org/cargo/reference/rust-version.html)
requires independent supported-version verification.

## Ownership and verification

`tools/ci/rust_toolchain.py` belongs to repository assurance, not a terminal
extension or runtime service. It reuses the QA process-group/termination helpers,
bounds identity capture to 4 KiB, gives each identity process a 20-second response
deadline followed by termination/reader cleanup, and emits only validated
release numbers and compiler hashes. Failed probes never echo tool
output or environment values. Workflow errors use stable job ordinals, not
untrusted job labels. This is not a sandbox for arbitrary executables.

Repository validation parses TOML and workflow mappings, rejects duplicate or
excessive structure, and checks selection, ordering, failure propagation, MSRV
and cache identity, including current compiler/cache documentation. Workflow,
job and step environments cannot replace the verified compiler executable.
Real child-process tests cover successful capture, nonzero
exit, overflow, timeout and reader cleanup; mutations cover missing/forged or
reordered verification and overridden compiler selection. Existing architecture,
feature, security and release gates remain mandatory.

There is no dependency, runtime behavior, settings or data migration. Reverting
the change restores the recorded toolchain-selection defect, not a supported
alternative configuration. Native product, packaging, accessibility and
controlled performance claims still require their own evidence.
