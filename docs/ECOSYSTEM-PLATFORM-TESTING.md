# Sandboxed ecosystem and selected-input model suggestion testing

Status: accepted source implementation is locally verified; release activation
remains disabled and external native/release evidence remains open.

## Local evidence environment

The focused implementation evidence was exercised on Windows x86_64 on
2026-08-25. Results from this host prove only the exact commands and fixtures
listed here. They do not substitute for native Linux/macOS, signed packaging,
controlled GPU/accessibility, trust-root operations, or long-running release
campaigns.

## Fast contract and policy checks

Run:

```text
python tools/ci/check_ecosystem_d7_cp6.py
python tools/ci/test_ecosystem_d7_cp6.py
```

The first checker canonicalizes the immutable proposal contract, verifies its
exact digest, validates the separate acceptance receipt, confirms all 28 limits,
nine threat records, 12 verification domains and ten external gates, and scans
source ownership for prohibited ambient authority. The mutation suite changes
one security or release invariant at a time and proves that duplicate keys,
digest drift, dependency drift, activation, download/provider authority, broad
WASI, grant weakening, stale provenance, unsafe package behavior, unbounded
resources, privacy weakening, and stale documentation are rejected.

## Rust tests

Run the pure contract tests:

```text
cargo test -p automexia-ecosystem --locked
```

They cover strict duplicate-key JSON, depth/string/source limits, manifest and
portable path validation, exact grant/revocation/scope/generation behavior,
lifecycle transitions, crash quarantine, fair bounded queues, offline metadata,
selected-input redaction/consent/replay/staleness, typed response limits, and
renderer-neutral package/model/lifecycle semantics. The property suite uses 512
cases per property for hostile portable paths, strict JSON, grant revocation,
and selection normalization/redaction.

Run local package, store, action-pack, and WIT tests:

```text
cargo test -p automexia-ecosystem-runtime --locked
```

These use real in-memory ZIP files and Ed25519 signatures. They cover exact
content/signature/provenance/SBOM/license receipts, tampering, expiry, revocation,
stale metadata, traversal and linked entries, typed non-executing action packs,
atomic disabled publication, retained generations, state reconstruction,
interrupted publication, cross-identity placement, unowned staging, disable,
kill, exact uninstall, linked roots, and Windows private ACLs. The checked-in
WIT is parsed by `wit-parser` and its exact source world, imports, ABI version,
and export are asserted.

Run the disabled Component Model conformance host:

```text
cargo test -p automexia-ecosystem-runtime --features component-host --locked
```

The `component-host` suite compiles real WAT fixtures through Wasmtime and proves
that public activation is denied, the private conformance permit runs a finite
no-WASI component, deterministic fuel stops an infinite guest, epoch deadline
and cancellation join workers, oversized memory/table declarations fail,
forbidden or mismatched imports fail, generated host interfaces default to no
data and denied publication, and exact grants make selected input one-shot.

Run the product adapter tests:

```text
cargo test -p automexia-terminal --lib automexia::ecosystem::tests --locked
```

They prove local inspection/install-disabled/review plumbing and hard denial of
activation, download, provider calls, and grants without PTY input or process
launch.

## Fuzzing and property evidence

The checked-in fuzz target is:

```text
cargo fuzz run ecosystem_bundle -- -max_total_time=300
```

It feeds arbitrary bundle bytes plus trust/revocation inputs through the bounded
verifier. Compilation of the harness can be checked independently. A live fuzz
campaign requires a Rust nightly toolchain with sanitizer support. If that
prerequisite is missing, record the campaign as not run; do not call a compile
or stable-toolchain rejection a fuzz pass.

Release evidence needs the maintained hostile package/component corpus, fixed
seeds, retained crash artifacts, and an independently reviewed sanitizer run on
each supported native platform.

## Performance and resource evidence

Run same-host Criterion baselines:

```text
cargo bench -p automexia-ecosystem --bench policy
cargo bench -p automexia-ecosystem-runtime --bench verification
```

Record compiler/profile, CPU, memory, cold/warm state, sample counts and noise.
The policy benchmark covers strict manifest validation, grant checks, consent,
and UI snapshot publication. The runtime benchmark covers signed-bundle
verification without activation. Compare a later release candidate to a clean
same-host baseline; a successful compile alone is not a performance claim.

The 2026-08-25 Windows x86_64 baseline used rustc 1.96.1 release benchmarks on
an AMD Ryzen 5 5600H, with Criterion's 100-sample measurement. Warm results were:

| Contract | Estimate |
|---|---|
| capability diff, 32 requested entries | 329.03-341.25 ns |
| strict manifest validation | 736.70-744.50 ns |
| exact grant authorization | 341.19-346.91 ns |
| selected-input review/redaction | 2.7259-2.7624 us |
| consent surface publication | 2.9844-3.0055 us |
| real signed ZIP bundle verification | 68.771-69.549 us |

An earlier comparison against the superseded one-function policy benchmark
reported a noisy +5.46% change with 14 outliers. It is retained as an observed
non-passing comparison, not erased by the expanded measurements. The expanded
named baseline is the reviewable starting point for future same-host comparisons;
this run did not measure process memory or activation because activation is
forbidden.

Before activation, measure Wasmtime binary/startup cost, cold and warm component
compile, invocation/cancellation p50/p95/p99, host/guest allocations, memory,
tables, handles, threads, files, storage, and cleanup. Complete 1,000 native
install/enable/call/disable/update/revoke/uninstall cycles and the contract's
30-day multi-extension/multi-session soak without unbounded growth.

## Repository gates

After focused tests, run:

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo nextest run --workspace --locked --profile ci
cargo test --workspace --doc --locked
python3 tools/ci/qa.py --full
cargo ready
```

Also run `cargo deny check`, repository validation, architecture verification,
identity verification, package checks, and `git diff --check`. Investigate the
first failure; retrying does not erase it. If a tool is unavailable, report the
exact prerequisite and retain the other evidence.

## Visual and accessibility evidence

Renderer-neutral tests verify hierarchy, accessible labels/values, textual
trust and risk, cancel-first order, reduced motion, responsive rows, and bounded
safe previews. Release still requires inspection of the actual native product
surface at tiny, normal, ultrawide, split, 4K and 8K-equivalent layouts;
100-300% scale; light/dark/custom themes; long localized Unicode; keyboard,
pointer, IME, focus loss/restore, modal stacking, and package/model failure
states.

Use controlled Windows UI Automation, Linux AT-SPI and macOS VoiceOver evidence
on the native builds. Verify announcements, role/state/value, focus restoration,
no color-only meaning, contrast, no clipping or close-control collision, and
copy/insert without Enter. There is no released native D7/CP6 surface yet, so
these are external release gates, not local passes.

## Security and failure campaigns

Release candidates require fixtures and drills for:

- compromised signer, wrong publisher, root rotation, emergency revocation,
  rollback, freeze, mix-and-match, expiry, offline horizon and cached revoked
  content;
- zip bombs, false sizes, normalization/case collisions, special files, links,
  path traversal, read-only/disk-full/locked storage and interrupted publication;
- forbidden imports, malformed components, traps, panics, reentrancy, output/log/
  transfer storms, memory/table growth, queue saturation, cancel/shutdown races,
  stale generations and cross-profile/session/pane grants;
- selected-data redaction canaries, prompt injection, provider/locality/model
  substitution, malformed/oversized/stale responses, negative persistence,
  support-bundle scans, delete/retention/offline/timeout behavior and no execute;
- exact kill, disable, quarantine, revoke, rollback, uninstall, last-known-good,
  and first-party plus CP1-CP3 fallback.

Logs and reports must include bounded metadata only. Never include selected input,
model output, package payload, secret material, credentials, private history, or
remote host data.

## External evidence still required

The source boundary is complete, but release remains blocked on:

1. two independent exact-head protected approvals and non-bypassable server
   enforcement for every activated authority;
2. named publisher-key, trust-root, repository, revocation, incident-response,
   SDK compatibility and dependency-update owners;
3. controlled malicious-package/component and supply-chain drills;
4. native Windows, Linux and macOS signed-package, sandbox, cleanup,
   accessibility, IME, focus and uninstall evidence;
5. low-end through high-scale performance and complete resource baselines;
6. 1,000 lifecycle cycles and the 30-day soak;
7. privacy/legal approval for each remote model-provider data flow, retention and
   deletion path;
8. verified kill/disable/uninstall/rollback and first-party/CP1-CP3 fallback;
9. a separate network/distribution ADR and maintained update client before any
   public download;
10. a separately authorized native product UI and public SDK publication plan.

Every report must identify the commit, package digest, dependency versions and
features, operating system/architecture, hardware, fixtures, sample count or
duration, limits, resource deltas, cleanup outcome, failures and unavailable
prerequisites. Cross-compilation, synthetic snapshots, stable-toolchain fuzz
rejection, or a retry is not native release evidence.
