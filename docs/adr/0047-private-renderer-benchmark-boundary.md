# ADR 0047: Private renderer benchmark compilation boundary

Status: Implemented locally; contributor review pending.

## Context

The application owns renderer-private fitting code. Selecting a benchmark in
its package also builds the application binary. Optimized application builds
can exhaust available memory even after the small benchmark has linked.
Exporting a product library API solely to measure private code would widen
visibility without a runtime requirement.

## Decision

The unpublished `automexia-renderer-benchmarks` workspace package under
`tools/renderer-benchmarks` contains only the explicit `text_fit` benchmark.
It includes the application-owned module by path, with no implementation copy.
The same harness includes the private command-result row-band iterator to measure
visible-row geometry against the former single-rectangle submission, without
exporting a production API or compiling an extra application benchmark target.
All dependencies are existing development dependencies. No package may depend
on this package. It owns no library, application, build script, test service,
extension capability or release artifact. Windows and wasm retain WGPU parity;
other targets can explicitly select the existing optional WGPU feature.

The existing application benchmarks keep their current owner. Production
fitting, presentation state, font ownership and release optimization are unchanged.
Architecture checks enforce the boundary using Cargo metadata, with mutations
for missing or duplicate targets, dependency kind, inverse/renamed edges and
publication. Feature assurance retains canonical performance evidence ownership.
The static command-productivity scanner validates this exact benchmark-only
manifest and scans its benchmark Rust files with the existing file-count and
link rules. It rejects runtime/build targets and unexpected source directories
rather than treating a missing runtime source directory as an exemption.

## Consequences and verification

This is an independent development compilation boundary, not a shared runtime
utility crate or graphics extension. Reverting it only moves the harness and
registration back. Criterion case names and settings remain stable for same-host
comparison; record selected features, source and artifact identity. A harness
measurement is not an application build, interactive latency, native window,
controlled-runner release threshold or cross-platform rendering certification.

Cargo documents [benchmark targets](https://doc.rust-lang.org/cargo/reference/cargo-targets.html#benchmarks)
and [feature resolution](https://doc.rust-lang.org/cargo/reference/resolver.html#feature-resolver-version-2).
The metadata and actual build graph, not this prose, are the executable boundary.
