# Dependency trust review: October CI repair

The workspace already selected Wasmtime 48.0.3, Cranelift 0.135.3 and
process-wrap 10.0.0. The trust baseline still named Wasmtime 48.0.1, Cranelift
0.135.1 and process-wrap 9.1.0, leaving 28 packages uncovered. This repair keeps
the locked graph and its security fixes; it does not relax the CI criteria.

## Wasmtime, Cranelift and Pulley

Use the Bytecode Alliance's published Cargo Vet records at immutable revision
`f543666f4c59adbc8d22eabb975215fb78595b61`. Cargo Vet's public registry identifies
this as the Wasmtime project's audit source. The reviewed import lock contains
only the 27 already-locked Wasmtime, Cranelift and Pulley packages. Superseded
local exemptions for those packages are removed. The configured exclusion list
does not constrain publisher wildcard audits in Cargo Vet 0.10.2; the
[subsequent security-patch review](2026-10-wasmtime-security-patch.md) documents
that limitation and the expanded, explicitly reviewed lock scope.

These are upstream publisher-based trust records, not newly claimed independent
Automexia audits. Cargo Vet's checked-in import lock records the relevant audit
and publisher evidence. CI continues to use `cargo vet --locked`; updating the
snapshot, publication evidence or dependency versions requires another review.

The source comparison against the previous baseline covered package manifests,
licenses, build scripts, native sources and changed runtime code. The family
retains Apache-2.0 WITH LLVM-exception licensing and Rust 1.95 compatibility.
The patch changes do not add build scripts or native build inputs to this graph.
Most component versions move together; substantive changes include fuel
accounting and compiler compatibility fixes.

[Wasmtime 48.0.3](https://github.com/bytecodealliance/wasmtime/releases/tag/v48.0.3)
fixes fuel accounting across reference calls and exception returns, and dynamic
component-value lifting. Automexia's optional component host retains the same
feature selection and resource limits. WASI HTTP and filesystem fixes in that
upstream release do not enable those integrations in Automexia.

## process-wrap

Retain one exact-version compatibility exemption for 10.0.0, following the
baseline policy in the parent README. No matching published audit was suggested
by Cargo Vet's registry. This is a documented acceptance of the existing
dependency, not a full independent audit or a blanket exemption for new versions.

Compared with 9.1.0, the crate keeps its Apache-2.0 OR MIT license, Rust 1.87
requirement and dependency requirements. It has no build script or bundled
native source. The application enables only `std`, `process-group`, `job-object`
and `creation-flags`; xtask uses the first three. Tokio remains disabled.

The reviewed production changes replace type-erased wrapper extension with typed
extension, keep peer wrappers available during lifecycle hooks, expose fallible
native-child accessors, and preserve direct wrapper layers. Windows changes
preserve creation flags while temporarily suspending a child for job assignment,
borrow process handles, close temporary handles on errors, and terminate a child
when assignment or resumption fails. Process groups and job objects remain the
existing termination owners. The new boxed-child spawning API is not adopted by
this repair. See the [upstream changelog](https://github.com/watchexec/process-wrap/blob/v10.0.0/CHANGELOG.md).

Existing application and xtask process-lifecycle tests exercise native spawning,
output limits, cancellation and descendant cleanup. The review does not claim
that a process group or Windows job is a hostile-code sandbox. Linux and Windows
verification is recorded with the change; macOS runtime evidence remains external.

## Required verification

Keep Cargo Audit, Cargo Deny and Cargo Vet enabled. Check the exact locked graph,
the component-host and process-lifecycle consumers, the contributor gate and the
explicit pre-push assurance profile. Do not regenerate the entire baseline to
accept unrelated dependency changes.
