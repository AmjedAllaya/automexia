# Developer architecture guide

Use [the canonical architecture](../ARCHITECTURE.md) as the authority.

## Dependency direction

Application coordination depends on terminal/session models, which depend on VT
and platform abstractions. Optional code depends on versioned contracts and is
invoked through application-owned capability brokers. Core terminal crates
never depend on product-specific optional integrations.

## Ownership

Keep one owner for each PTY, process tree, terminal state, route, renderer
snapshot, configuration record, and UI surface. Publish immutable
generation-labelled state and reject stale results.

## Hot paths

Input, PTY read/write/resize, VT mutation, snapshot publication, renderer wake,
and basic startup remain bounded and free of filesystem scans, persistence,
network, authentication, and optional-component work.

## Trust boundaries

Treat terminal output, configuration, paths, shell metadata, clipboard, imported
files, extension data, and generated text as untrusted. Use typed executables
and exact argument arrays. No structured action uses shell evaluation or
implicit Enter.

## Placement

Core contains indispensable terminal mechanisms. An existing extension owns
cohesive optional behavior within its current capability boundary. A new
extension requires a distinct optional authority or dependency footprint,
explicit lifecycle, limits, disable/uninstall behavior, tests, and an ADR.

Unreleased feature placement and commercial architecture are private.

## Maintaining existing boundaries

Map a change to its current owner before adding a crate or shared type. Reuse
capability-free contracts when consumers need the same semantics. Directory
names alone do not establish dependency or authority isolation; a mechanical move
needs a concrete maintenance reason and a separate review from behavior changes.

System tools remain responsible for their native protocols and credentials.
Evaluate adapter compatibility, structured output, limits, cancellation and
cleanup rather than assuming either a CLI or library is automatically safer.

## Build and validation ownership

[ADR 0005](../adr/0005-storage-bounded-build-workflow.md) governs the current
incremental application and disposable verification builds. Keep current test,
platform and cleanup requirements when evaluating maintenance improvements.
Report clean and incremental measurements separately, with the exact toolchain,
target, features and evidence limits. Configuration proposals are not implemented
optimizations until the owning source and applicable checks agree.
