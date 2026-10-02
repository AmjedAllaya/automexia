# Dependency trust review: Wasmtime security patch

The security update selects Wasmtime and Pulley 48.0.4, Cranelift 0.135.4,
WAT 1.254.2, WAST 254.0.2 and the accompanying wasm-tools 0.254.2 packages.
Cargo Audit and Cargo Deny accepted this graph, but locked Cargo Vet rejected
all 35 changed packages because its previous publication evidence and parser
exemptions covered the older releases. This repair changes trust records only;
the dependency graph, runtime features and deployment criteria stay unchanged.

## Published evidence and scope

Retain the existing Bytecode Alliance audit source at immutable revision
[`f543666f4c59adbc8d22eabb975215fb78595b61`](https://github.com/bytecodealliance/wasmtime/blob/f543666f4c59adbc8d22eabb975215fb78595b61/supply-chain/audits.toml).
Its `safe-to-deploy` wildcard records cover publications from
`github:bytecodealliance/wasmtime` and `github:bytecodealliance/wasm-tools` within
the recorded validity periods, through January 8, 2027. Cargo Vet refreshes the
exact crates.io publication evidence; it does not create an Automexia audit.

The imported lock contains exactly the existing 27 Wasmtime, Cranelift and
Pulley packages plus these eight reviewed wasm-tools packages:

- `wasm-encoder`, `wasm-metadata`, `wasmparser`, `wasmprinter`;
- `wast`, `wat`, `wit-component`, `wit-parser`.

Remove only those eight obsolete exact-version exemptions and their ordinary
audit exclusions. Preserve every other local exemption and policy, the original
27 wildcard records, and the upstream URL. No new exemption or broader criterion
is introduced. These remain upstream publisher-based trust records, not a claim
of a comprehensive independent source audit.

Cargo Vet 0.10.2's [import implementation](https://github.com/mozilla/cargo-vet/blob/v0.10.2/src/storage.rs)
applies `exclude` to ordinary audit entries but leaves wildcard audits eligible.
A private refresh reproduced that behavior. The exact reviewed `imports.lock`
and CI's `--locked` requirement are therefore the package/publication ratchet;
the exclusion list alone is not. Future unlocked refreshes require review of
every added publisher and audit record.

## Source delta review

The published old and new crate sources were compared by package, including
normalized manifests, feature and dependency changes, licenses, build scripts,
native sources, and changed Rust code. Both package families remain maintained
by the Bytecode Alliance, with releases in their existing upstream repositories.

The 27-package Wasmtime family retains Rust 1.95 and
`Apache-2.0 WITH LLVM-exception`. Build scripts and native inputs are unchanged.
Apart from version requirements, the source change is Cranelift's safepoint
handling for exception-capable calls: outgoing reference arguments stay live
across the call and reload on successor edges. An upstream regression exercises
normal and exceptional successors. The remaining 26 packages have no changed
Rust/native implementation files in this patch comparison.

The eight wasm-tools packages retain Rust 1.85 and their existing
`Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT` license expression. They add
no build script or native code. Seven have only package/dependency version
changes. `wasmparser` corrects callback signature validation to require three
`i32` parameters and one `i32` result, and requires tag type compatibility in
both directions. See [Wasmtime 48.0.4](https://github.com/bytecodealliance/wasmtime/releases/tag/v48.0.4)
and [wasm-tools 1.254.2](https://github.com/bytecodealliance/wasm-tools/releases/tag/v1.254.2).

The update addresses RUSTSEC-2026-0325, RUSTSEC-2026-0326 and RUSTSEC-2026-0327.
Automexia retains its disabled public component-execution gate and its existing
optional host features and limits. This review does not establish exploit
reachability in that configuration, enable asynchronous component execution or
GC features, or claim additional operating-system support.

## Verification

Check the exact locked trust graph after generating publication evidence.
Machine comparisons must show only the eight intended exclusion/exemption
removals, exactly 35 matching publisher records, unchanged original wildcard
records, and unchanged Cargo lockfiles. Negative checks remove required evidence
or substitute an incorrect publisher/date and must still fail locked vetting.

The dependency update already passed component-host tests and Clippy on native
Windows and Linux; macOS execution remains external. Run the complete
`cargo xtask assurance pre-push` profile for this trust repair, including Cargo
Audit, Cargo Deny and locked Cargo Vet. Keep the failure and verification
evidence with the delivery review; do not regenerate the compatibility baseline.
