# Testing and release assurance

This page is the canonical explanation of how Automexia proves behavior and what evidence is required before a release. It replaces separate testing, readiness-audit, release-trust, branding, and security-debt pages whose repeated status prose made it difficult to distinguish a test command from an unresolved release gate.

## Evidence vocabulary

- **PR:** deterministic checks that run on hosted native runners for every pull request.
- **Nightly/deep:** longer fuzz, sanitizer, Miri/model, benchmark compilation/execution, package, and mutation work.
- **Controlled:** evidence requiring a real display/GPU, elevated tooling, credentials/signing identity, particular hardware, or manual assistive-technology observation.
- **External:** required evidence not observed in the current environment. It never means "passed by inference".

A portable Rust test can prove portable logic; it cannot prove a native window server, PTY adapter, GPU stack, screen reader, installer, signature, notarization, or organization policy on another OS.

## Contributor gates

The normal contributor contract is intentionally small:

```text
cargo xtask doctor
cargo ready
cargo automexia
```

`doctor` is a non-mutating environment/storage/native-tool report. `cargo ready` is the complete local verification gate. After one clean gate, `cargo automexia` is the incremental launch path. `cargo dev` combines the complete gate, application build/version check, shell-integration repair, and launch.

Focused `cargo xtask test ...` commands exist for subsystems such as resize stress, image rendering, OpenSSH inventory, completion, aliases, and provider-neutral models. They are useful during development, but they do not replace the complete gate before merge.

## Pull-request assurance

Every pull request should cover the behavior at the lowest deterministic level that can fail meaningfully:

- locked all-feature compilation, Clippy, tests/doctests/Nextest according to platform ownership;
- repository architecture, identity, policy, documentation/link, changelog, and assurance-ledger validators;
- targeted model/property/mutation tests for state machines and authorization boundaries;
- hostile-input and boundary tests for VT control strings, parsers, configuration, generated artifacts, imports, path handling, and decoders;
- platform-native shell/PTY/display-feature jobs where hosted runners can own them;
- exact generated-artifact/digest/rollback contracts for completion, aliases, packaging, and release metadata.

The feature assurance ledger maps each feature to its guide/reference/explanation and to the quality/platform evidence required for that feature. A behavior-affecting change is incomplete when its tests pass but its docs/evidence declaration is stale.

## Native platform ownership

Windows owns ConPTY, Windows shell integration, custom/native window behavior, WGPU/DX paths, MSI/AuthentiCode, WSL launch semantics, and Windows accessibility/performance tooling. Linux owns Unix PTY plus X11/Wayland configurations and Linux packaging. macOS owns Unix PTY plus native window/Metal behavior, universal assembly, signing/notarization/Gatekeeper, and VoiceOver evidence.

Cross-compilation or a successful source build on another host cannot substitute for these native interfaces.

## Security and hostile-input depth

Nightly/deep assurance extends deterministic tests with fuzzing, sanitizers, Miri/Loom/model checks, mutation strength, longer parser/decoder campaigns, and resource/leak stress. Security-sensitive boundaries should have explicit exact-limit and limit-plus-one cases, cancellation/timeout behavior, malformed/truncated input, stale generation/session handling, and last-known-good recovery.

For commands/processes, verification must prove exact executable/argv/environment/cwd boundaries and that shell interpretation is absent where the design promises it. For files/persistence, tests must cover permissions, symlinks/reparse points, traversal, partial writes, CAS/contention, atomic publication, rollback, and uninstall ownership.

## Performance evidence

Performance claims are not established by one development-host benchmark. The project distinguishes:

1. **Micro/criterion regressions** for hot pure boundaries such as parsers, reflow, search, projection compilation, inventory parsing, and image cache reuse.
2. **Native interaction/resource evidence** for frame/resize/PTY/image lifecycle, handles/threads/private bytes, GPU/texture state, and teardown.
3. **Comparable baseline history** across supported environments before an enforceable latency/memory ratchet is activated.

The S2 source ratchet is implemented but remains in **collecting** state. `tools/ci/performance_assurance.py` strictly normalizes Criterion confidence bounds and the existing native Windows private-byte/working-set report, composes only exact commit/time/runner matches, and rejects symlinks, duplicate keys or identities, unsafe text, non-finite values, unknown release metrics, and oversized documents. The reviewed policy freezes 30-90 consecutive same-runner days, required claims, 5% latency, 10% memory, path-free reports, and exact HTTPS-reviewed waivers that expire within 30 days.

Nightly retains normalized controlled evidence for 90 days. `build-baseline` creates an active file only from complete consecutive evidence plus an explicit maintainer acceptance. Tagged releases run `evaluate --require-active`, so the checked-in collecting template blocks publication until the elapsed baseline exists; local benchmark numbers remain observations, not universal guarantees.

## Build-artifact lifecycle

Fast application builds retain their normal incremental target. Exhaustive verification uses an isolated direct-child target with incremental compilation disabled and removes it after normal success or failure, unless the contributor explicitly keeps it for diagnosis. Windows/MSVC and WSL/Linux toolchains stay on their native filesystems and do not share `target/` directories. Use `cargo storage` before deleting and `cargo purge` for owned workspace artifacts.

## Release artifact trust

Release trust applies to the **final bytes users install**, not to an earlier unsigned intermediate.

A protected release must establish:

- final artifact inventory and SHA-256 checksums;
- SBOM/provenance/attestation bound to those artifacts;
- Windows Authenticode signatures and timestamps, plus controlled malware scanning of the exact public MSI/ZIP;
- macOS Developer ID signing, hardened-runtime/notarization/stapling/Gatekeeper validation for the final application/DMG;
- Linux package/tar artifact validation and checksum/provenance publication;
- clean install/launch/uninstall checks on the platforms/packages claimed;
- immutable publication from the protected workflow rather than an ad-hoc developer upload.

Users should verify checksums and host-native trust indicators on public releases. A malware false positive is investigated against the exact public artifact and submitted to the detecting vendor only after checksum/signature/provenance verification; the response is never to recommend antivirus exclusions or execution-policy bypasses.

## Brand assets as a release gate

The repository can generate platform PNG/ICO/ICNS derivatives from its canonical raster source for development/nightly packages. Stable publication additionally requires the final editable/vector/master variants, redistribution-rights evidence, and named approval recorded in the brand asset manifest. Passing dimension/checksum/package wiring tests does not by itself establish asset rights or final-design approval.

## Known security/maintenance debt

Security debt is tracked separately from product claims. Runtime hardening work includes keeping dependencies current, minimizing duplicate dependency families where they materially expand attack/patch surface, and ensuring new crates do not bypass existing trust/capability boundaries. A dependency duplication is not automatically a vulnerability, but it should have an explicit owner/upgrade rationale when versions diverge significantly or one branch is unmaintained.

## Managed SSH activation evidence

The M2/F4/D3 boundary is production-compiled but deliberately unavailable. Run
the source-local contract before handoff:

```text
python tools/ci/check_session_launch_d0.py
python tools/ci/test_session_launch_d0.py
cargo test -p automexia-ui-model --locked --test direct_openssh_review
cargo test -p automexia-terminal --bin automexia --locked application_runner_
cargo test -p automexia-terminal --bin automexia --locked context::launch_broker::tests
cargo xtask verify architecture
```

These checks prove exact ownership, denial-before-resolution, scope, bounds,
redaction, executable-guard handoff, route publication ordering, cancellation,
shutdown, keyboard/pointer/accessibility semantics, and mutation resistance.
They do not prove production SSH: the activation constant is false and the
linked candidate is unverified. Exact-head approvals/server enforcement,
attestation/revocation, fresh current-executable review, real native OpenSSH and
descendant cleanup, 1/10/50 resource runs, pixels, and controlled screen-reader
evidence remain explicit activation gates.
## M6 review-only evidence

The M6 source boundary has focused Windows x86_64 planner, automation, workspace,
semantic UI, and schema-2 library tests plus warning-denied lint, mutation,
architecture, fuzz ownership, repeated 1,000-generation/50-target bounds, and
maximum-cardinality benchmarks. Exact commands, measurements, the recorded
same-host Criterion noise, and limitations are in
[M6 testing](../TESTING.md#m6-typed-automation-and-multi-environment-workspaces).
These results do not activate a product controller/CLI or prove native managed
OpenSSH/PTY/process/resource/accessibility behavior.
## Current release blockers

The source-level v0.4 S0 gates are complete locally, but stable release assurance remains partial. The important unresolved class is **evidence**, not a hidden claim that everything is done: controlled Linux/macOS visual/GPU/PTY runs, screen-reader evidence, longer security/performance baselines, signing/notarization/packaging proof on protected hosts, and other named release-environment requirements must be recorded before the corresponding release claim is made.

Likewise, locally implemented CP2/CP3 command-productivity work is not promoted to a stable broad release claim until its native/accessibility/performance gates pass. Managed SSH has a nonactivated runner, approval surface, executable guard, and PTY/route publication seam, but launch remains blocked on protected exact-head review, real package attestation/revocation, a fresh current-executable review, and native lifecycle/resource/accessibility evidence.

See [Roadmap](../project/roadmap.md) for phase status. This page owns the meaning of the evidence levels and the testing/release contract; the roadmap should not copy detailed test ledgers.
