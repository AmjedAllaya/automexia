# S1 and S2 stabilization and release-assurance implementation audit

Status: execution plan for the 2026-08-23 source pass. Local source work may
close only the contracts identified below. Native-host, controlled-hardware,
human-review, and elapsed-time evidence remains external and must never be
reported as passing from this Windows checkout.

## Scope and authority

The user-visible outcome is a reproducible release-assurance path that catches
resize/snapshot regressions, produces reviewable visual differences, preserves
comparable benchmark evidence, and enforces the documented performance ratchet
only after a complete accepted baseline exists.

Authoritative inputs are `docs/STABILIZATION-ROADMAP.md`,
`docs/PHASE-IMPLEMENTATION-AUDIT.md`, `docs/TESTING.md`, `RELEASING.md`, the
existing QA/Nextest/native wrappers, and the release workflow. S1 does not add
product behavior. S2 does not authorize enforcement before the documented
30-day same-runner baseline is complete and accepted.

In scope:

- deterministic resize-queue and test-snapshot publication models;
- a bounded test-only expected/actual/difference PNG comparator;
- normalized bounded benchmark evidence and exact runner comparability;
- baseline eligibility, latency/memory thresholds, scoped waivers, reports,
  nightly collection, CI mutation coverage, and release fail-closed wiring;
- synchronized assurance, testing, roadmap, release, and change documentation.

Out of scope and external:

- fabricating Linux X11/Wayland, macOS, GPU, RDP, screen-reader,
  Application-Verifier, WPR, signing, notarization, or human visual evidence;
- treating cross-compilation, a synthetic fixture, or one local run as native
  or controlled-hardware proof;
- activating the S2 ratchet before 30 consecutive comparable UTC dates and all
  required metric families have an explicitly reviewed accepted baseline;
- adding product telemetry, network upload, a background service, or a runtime
  dependency.

## Evidence ledger

| Phase | Status before this pass | Evidence owner | Missing proof or work | Exit for this pass |
|---|---|---|---|---|
| S1.1 prompt/resize/session/input | Partially implemented | `rio-vt::performer`, native resize/clone scripts, renderer/layout tests | Pure generated resize-queue reference model; Linux/macOS native evidence | Add deterministic shrinking equivalence/invariant coverage; keep native evidence external. |
| S1.2 resources/hardware | Partially implemented | Windows resource storm, AppVerifier/WPR wrappers, QA reports | Controlled elevated and named-hardware executions | Preserve source; verify release ledger keeps every external run explicit. |
| S1.3 visual/frame regression | Partially implemented | Renderer-neutral snapshots and native Windows capture | Repository-owned expected/actual/diff comparator, pinned goldens, other native hosts, human approval | Implement bounded comparator and self-tests; actual approved matrix remains external. |
| S1.4 accessibility | Partially implemented | Inventory, focus/contrast/scale tests, ADR 0013 | Narrator/NVDA, VoiceOver, Orca, later semantic tree | Preserve and report as external. |
| S1.5 context freshness | Partially implemented | bounded provider workers/caches/generations | Controlled CLI latency/resource runs on three hosts | Preserve and make the S2 evidence vocabulary capable of carrying the measurements. |
| S1.6 orchestration/QA | Source tooling fully implemented | Nextest, `tools/ci/qa.py`, `cargo ready` | Retained bundles from controlled hosts | Preserve; integrate new deterministic policy tests and normalized summaries. |
| S1.7 property/model/fuzz | Partially implemented | Proptest, Loom, fuzz, coverage and mutation workflows | Pure resize model and generation-aware snapshot replacement; longer hosted campaigns | Implement the two local models; hosted campaigns remain external. |
| S1.8 performance measurement | Partially implemented | nine controlled Criterion targets and controlled nightly job | Normalized same-runner evidence and retained comparison contract | Add bounded collection/evaluation/reporting without claiming a baseline. |
| S2 enforcement | Not implemented | no current owner | 30-day eligibility, comparison, waiver, activation and release policy | Implement an inactive-by-default fail-closed ratchet; activation remains externally blocked until the exact baseline contract is met. |

## Current-practice and reuse decision

Criterion remains the measurement engine. Its official documentation describes
warmup, bootstrap confidence intervals, saved baselines, statistical comparison,
noise thresholds, and the need for equivalent environments. GitHub Actions
artifacts remain transport only; summaries require explicit retention and may
expire or disappear with their workflow run. Existing `image`, `proptest`,
`insta`, `loom`, QA, and release-trust infrastructure is sufficient. No new
crate, service, or telemetry system is justified.

Build/wrap/adopt decision:

- adopt Criterion output and the existing GitHub artifact boundary;
- wrap them with a small repository-owned, schema-validated normalization and
  policy layer because neither owns Automexia's 30-day, runner, metric-family,
  privacy, waiver, or release rules;
- reuse `image` inside the non-production `xtask` for raster comparison;
- extend the existing `rio-vt` and native test-hook owners instead of adding a
  competing resize or snapshot authority.

## Design and trust boundaries

Performance evidence is untrusted input. Readers reject duplicate JSON keys,
unknown fields, non-regular/symlinked and oversized files, excessive metrics or
days, non-finite/non-positive values, unknown units/families, duplicate
identities, invalid UTC timestamps, unrecognized runner fields, and
cross-runner/toolchain/power-profile comparisons. Reports contain only
allowlisted metadata and metric values and are written by atomic replacement.

An accepted baseline must cover at least 30 consecutive distinct UTC dates on
one exact runner fingerprint and every policy-required metric. The repository
ships in `collecting` state. `--require-active` fails closed until a reviewed
baseline changes the state to `active` and passes every eligibility rule.

Latency above 5% and memory above 10% fail. A waiver must bind one metric, the
candidate commit, the accepted-baseline digest, a maximum regression, an HTTPS
tracking URL, an approver, approval time, and a non-expired deadline. There are
no wildcard or permanent waivers.

Visual inputs are test artifacts, not terminal data. The comparator rejects
links, oversized encoded/decoded inputs, geometry mismatch, excessive masks,
invalid tolerance, and unsafe output aliasing. Exact geometry is mandatory;
pixel tolerance and masks are explicit. Failure produces a bounded heatmap and
JSON summary. Ordinary product builds gain no visual hook or file IO.

## Tests-first sequence

1. Add failing performance-policy tests for duplicate keys, bounds, 29-day and
   gap rejection, incomplete metrics, runner mismatch, threshold regressions,
   waiver scope/expiry/digest, report privacy, and atomic output.
2. Implement the pure normalizer/evaluator only to satisfy those contracts.
3. Add failing resize properties against an independent reference reducer,
   then preserve the existing production reducer.
4. Add failing snapshot generation tests for stale rejection and failed-write
   last-known-good preservation, then extend the feature-gated publisher.
5. Add failing raster comparator tests for exact pass, geometry mismatch,
   tolerance/mask behavior, heatmap, bounds, and output ownership, then add the
   `xtask` command.
6. Add workflow/document mutation checks before wiring nightly collection and
   fail-closed release enforcement.

## Platform, UX, lifecycle, and rollback

All new code is contributor/release tooling. Windows, Linux, and macOS use the
same pure policy tests. Native results remain labeled with their actual host,
architecture, display/backend, driver, scale, shell, and power mode.

The visual command returns a concise pass/fail summary and focused artifacts;
it never bulk-accepts goldens. Performance reports list pass, regression,
waived, missing, incompatible, or collecting states without relying on color.
Failures name the exact metric and action. No command launches the application,
changes profiles, uploads data, or installs tools.

Rollback is removal of the new CI/tooling files and workflow steps. The
production terminal remains unchanged. Disabling S2 collection loses no product
state; disabling enforcement requires an explicit policy/release review, not a
silent environment fallback.

## Commit strategy

Use one integrated DCO-signed phase commit because S1 models, S2 normalization,
the shared `xtask`/QA architecture gate, workflows, and status documentation
form one fail-closed release-assurance boundary. Stage only the files listed in
this audit; preserve unrelated work. Rollback removes the complete assurance
boundary without changing ordinary terminal behavior or persisted user state.

## Implementation result

| Item | Resulting status | Evidence now owned by source | Remaining evidence |
|---|---|---|---|
| S1.1 prompt/resize/input | **Partially done** | Existing deterministic/native storms plus a 512-case independent resize queue model | Linux X11/Wayland and macOS native storms |
| S1.2 resources/hardware | **Partially done** | Existing native report now has a strict allowlisted S2 memory normalizer | Elevated/named-hardware/long-soak runs |
| S1.3 visuals | **Partially done** | Bounded `visual-diff` command, reviewed tolerance policy, atomic heatmap/report, focused tests | Approved golden matrices, Linux/macOS captures, human review |
| S1.4 accessibility | **Partially done** | Existing automated contracts preserved | Controlled Narrator/NVDA/VoiceOver/Orca evidence |
| S1.5 provider recovery | **Partially done** | Existing bounded/cancellable/freshness contracts preserved | Real provider/CLI latency and recovery runs |
| S1.6 QA orchestration | **Source tooling fully done** | S2 policy/mutation checks join bounded QA; benchmark targets are unique per run | Retained bundles on every release host |
| S1.7 models/strength | **Partially done** | Resize property model plus bounded generation-aware atomic publisher | Long corpora, branch/region baseline, mutation/vet governance |
| S1.8 measurement | **Partially done / collecting** | Classified Criterion/native-memory composition, exact runner identity, path-free reports, 90-day retention | Thirty complete controlled Windows runs |
| S2 enforcement | **Partially done / collecting** | Digest-frozen 5%/10% policy, baseline builder, exact expiring waivers, fail-closed tagged-release job | Reviewed activation of the externally collected 30-day baseline |

The `collecting` fixture is intentional. Source implementation is present, but
no generated or local evidence was substituted for elapsed controlled time,
native platforms, assistive technology, or human visual approval.

## External completion checklist

- [ ] Linux X11 native GPU/PTY/resize/prompt and frame bundle.
- [ ] Linux Wayland native GPU/PTY/resize/prompt and frame bundle.
- [ ] macOS Intel and Apple-Silicon native bundle.
- [ ] Elevated Windows AppVerifier and reviewed WPR summary.
- [ ] Named Intel/AMD/NVIDIA/RDP/software-fallback resource evidence.
- [ ] Narrator/NVDA, VoiceOver, and Orca smoke records.
- [ ] Human-approved dark/light/font/scale/layout screenshot matrix.
- [ ] Thirty consecutive same-runner UTC days covering every required metric.
- [ ] Reviewed activation change that binds the accepted baseline digest.
