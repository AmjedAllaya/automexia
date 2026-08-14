# ADR 0005: Storage-bounded build workflow

- Status: Accepted
- Date: 2026-08-12
- Updated: 2026-08-15

## Context

A complete Rust workspace gate builds multiple representations of many crates:
normal libraries, test harnesses, all-target checks, Clippy metadata, and the
desktop binary. Keeping incremental state for every representation caused a
local Automexia target to grow to roughly 30.8 GiB. CI also uploaded compiled
target trees even though runner, toolchain, feature, and compiler-flag changes
make those caches expensive and fragile. On Windows, launching the canonical
debug executable additionally prevented Cargo from replacing or cleaning it
while Automexia was open.

WSL adds a separate performance boundary. Linux tools operating on a checkout
or Cargo target under `/mnt/<drive>` pay the Windows/WSL filesystem bridge
cost for the workspace's many small files. Moving only `target` leaves source
metadata traffic behind; sharing one checkout also risks incompatible
Windows/Linux build artifacts.

The project still requires the complete check, Clippy, and workspace test gate.
Reducing coverage is not an acceptable storage optimization.

## Decision

1. `cargo automexia` keeps one persistent incremental debug application build.
2. `cargo xtask check`, `cargo ci`, and `cargo ready` run compilation-heavy
   policy in a process/time-named direct child matching
   `automexia-verification-v1-<pid>-<generation>` of the resolved Cargo target
   with `CARGO_INCREMENTAL=0`. Cargo's built-in `cargo check`
   remains an incremental focused-diagnosis command.
3. The isolated directory is deleted on success and ordinary failure. A drop
   guard attempts cleanup during early returns. A process kill or machine loss
   may leave its unique directory; `cargo purge` removes interrupted artifacts.
   Concurrent verification runs never share or delete one another's target.
4. Cleanup is allowed only for a generated direct-child name with the strict
   prefix and numeric process/generation suffix after rejecting symbolic links
   and Windows reparse points.
5. Verification requires 12 GiB free and an app build requires 4 GiB free by
   default. `cargo storage` reports usage; `cargo purge` delegates to Cargo's
   supported clean operation.
6. A launch copies the debug executable to a process/time-named generation in
   `automexia-runtime`. Unlocked stale generations are reclaimed on the next
   launch; a running Windows generation is retained until a later launch.
7. CI, nightly, and release jobs disable incremental compilation. CI caches
   Cargo registry and Git downloads, never compiled `target` products.
8. The test profile disables incremental compilation even when contributors
   invoke `cargo test` outside `xtask`.
9. Windows and WSL use separate, host-native Git checkouts. Source changes move
   between them through commits; Cargo targets and caches are never shared.
10. `cargo xtask doctor` reports source and target filesystem health.
    Compilation-heavy project workflows reject WSL source or target paths under
    `/mnt/<drive>` before building. A named environment override exists only
    for explicit, one-off diagnosis.
11. Windows-triggered decoder fuzzing stages the current source tree once into a
    disposable WSL-native `/tmp` directory. Cargo, libFuzzer, corpus, and
    target activity remains native after staging; targets and Git metadata are
    excluded and a trap removes all campaign state.

## Consequences

- The first complete gate performs a cold non-incremental compilation, but it
  does not permanently consume tens of gigabytes.
- Repeated `cargo ready` runs favor predictable storage over maximum compiler
  cache speed. Daily `cargo automexia` builds remain incremental and fast.
- Diagnostics that require build artifacts can opt in with
  `AUTOMEXIA_KEEP_VERIFY_TARGET=1`; retention is visible and deliberate.
- Contributors must close running Automexia windows before `cargo purge` can
  remove active runtime generations on Windows.
- The repository retains every required verification and test; only artifact
  lifetime and caching policy change.
- Contributors who validate both Windows and Linux maintain two checkouts, but
  avoid the much larger recurring cross-filesystem compilation cost.
- Raw Cargo commands retain standard behavior. Project-owned heavy workflows
  provide the guard because they can give a clear remedy without intercepting
  Cargo itself.
