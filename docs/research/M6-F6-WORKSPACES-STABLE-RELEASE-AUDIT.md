# M6/F6 workspaces and stable-release assurance audit

Date: 2026-08-27
Scope: M6/F6 typed recipes, declarative multi-environment workspaces, reviewed
broadcast, Connection Library/CLI/Hub composition, and their S1/S2 release
assurance.
Status: local review-only source complete; managed execution and controlled
native release evidence remain external.

## Outcome and acceptance boundary

This audit verifies the existing M6 implementation from JSON ingress through
pure models, transactional storage, application composition, renderer-neutral
views, tests, performance targets, and release policy. A locally complete result
means the product can safely store, edit, migrate, recover, list, and review
recipes/workspaces/broadcasts while every process, network, credential, PTY,
listener, shell-evaluation, and implicit-Enter authority remains disabled.

It does **not** authorize managed execution. ADR 0012/D3 protected activation,
M5 native lifecycle evidence, real OpenSSH/PTY/process/socket/handle cleanup,
current controlled S1 artifacts, and the S2 30-day baseline remain independent
release prerequisites.

## Placement decision

No new dependency or extension was added.

- `automexia-connectivity::connections` remains the capability-free owner of
  strict typed ingress, recipe/broadcast lifecycle, workspace topology, restore,
  fingerprints, limits, and validation.
- `apps/automexia-terminal` remains the owner of the private Connection Library,
  CAS/migration/recovery, CLI, worker snapshot, and product composition.
- `automexia-ui-model` remains the renderer-neutral visual/accessibility owner.
- S1/S2 and the dedicated M6 checker remain repository release-assurance owners.

Putting these contracts in terminal core would widen startup/input/PTY hot paths;
putting persistence or process authority in the pure package would break the
accepted trust boundary; a new extension would duplicate the existing coherent
Connection Hub/application lifecycle.

## Current primary-source baseline

- [RFC 8259, section 4](https://www.rfc-editor.org/rfc/rfc8259#section-4)
  recommends unique JSON object names and documents unpredictable receiver
  behavior for duplicates; M6 therefore rejects duplicates before typed decode.
- [Serde's official attribute reference](https://serde.rs/attributes.html)
  remains the typed schema-attribute authority; duplicate-name detection is a
  separate ingress invariant rather than a replacement for unknown-field and
  enum validation.
- [Rust `u64::checked_add`](https://doc.rust-lang.org/std/primitive.u64.html#method.checked_add)
  provides the fail-closed overflow primitive used for lifecycle deadlines.
- The [W3C modal dialog pattern](https://www.w3.org/WAI/ARIA/apg/patterns/dialog-modal/)
  remains the interaction baseline for contained focus and focus restoration;
  controlled assistive-technology evidence is still required.

## Evidence ledger

| Item | Classification | Current evidence and resulting state |
|---|---|---|
| Ownership and capability boundary | Fully implemented locally | Pure sources contain no filesystem/process/network/PTY authority; app storage and UI projection remain separate. |
| Strict JSON ingress | Fully implemented locally | One bounded recursive decoder rejects duplicate names at every nesting level, including escaped aliases, before typed deserialization. Workspace, recipe/profile/provider documents, private library, redacted import, and recipe context use it. |
| Recipe review integrity | Fully implemented locally | Every reducer event independently revalidates schema, disabled authority ceiling, step origin/order/policy, omitted-hook count, and content fingerprint. |
| Recipe time and retry lifecycle | Fully implemented locally | Start, success, failure, and retry timestamps are monotonic; step/total/retry deadline arithmetic is checked and fails without mutating lifecycle state. |
| Workspace persistence/edit/migration/recovery | Fully implemented locally | Schema-2 CAS, approval invalidation, redacted topology transfer, stale writers, corruption, and explicit recovery pass. A legacy previous generation now requires `recover`; `migrate` no longer offers an impossible apply path. |
| Restore from large libraries | Fully implemented locally | Product composition selects and fingerprints only profiles referenced by the chosen workspace, so 10,000-profile libraries do not violate the 128-connection workspace ceiling. |
| Broadcast review/lifecycle | Fully implemented locally | Review time is fingerprint-bound; arming/results/cancellation reject clock reversal; expiry terminalizes every pending target with digest-only audit records and an explicit accessible `Expired` state. |
| Product CLI/Hub review | Fully implemented locally | Preview-first commands, immutable worker snapshot, current-revision binding, responsive catalog/review, focus restoration, and no-PTY/no-Enter contracts remain in place. |
| Semantic assurance automation | Fully implemented locally | `check_m6_workspaces.py` freezes 10 limits, 8 disabled authorities, two activation blockers, seven source owners, nineteen real-path regressions, fuzz/benchmark ownership, S1 coverage, and CI/full-QA wiring. Its mutation suite removes or weakens each class and expects failure. |
| S1 policy coverage | Source complete; controlled evidence external | Every native/resource/visual/accessibility suite now includes an M6 workflow. Each visual environment requires 9,216 exact captures after adding catalog, restore, and broadcast surfaces. Actual controlled artifacts and human/assistive-technology review are not fabricated. |
| S2 performance release evidence | Source complete; baseline external | Maximum strict-JSON workspace parsing, workspace validation, 128-connection restore resolution, and 50-target broadcast review have Criterion owners. Same-host measurement is required; the 30-day controlled ratchet remains external. |
| Managed execution | External prerequisite | D3/M5 protected activation and real native cleanup evidence are intentionally unresolved; `execution_enabled()` remains false. |

## Defects reproduced and fixed

1. Ordinary Serde decoding accepted duplicate known keys, so two receivers could
   interpret one persisted/imported record differently. Literal, escaped, and
   nested duplicate-key regressions now traverse real parser and store paths.
2. A public reviewed recipe could be mutated after review without recomputing or
   independently checking all policy fields. Reducers now validate the complete
   review before any state transition.
3. Saturating deadline arithmetic and missing start timestamps allowed silent
   overflow or clock-reversed completion/failure. Checked arithmetic and explicit
   active intervals now reject those events without mutation.
4. Broadcast expiry changed only the aggregate state, leaving pending targets
   nonterminal and unaudited. Expiry now closes every pending target exactly once.
5. Workspace restore fingerprinted every profile in the library and failed when
   unrelated profiles exceeded the workspace limit. It now selects only required
   bindings and reports a missing required profile explicitly.
6. `workspaces migrate` treated a schema-1 previous recovery generation as a
   directly writable primary. It now directs that state to explicit recovery.
7. A long Windows checkout could make `cargo ready` discover MSVC's path limit
   only after starting its cold isolated build. The gate now rejects a generated
   target above 160 UTF-16 code units before compilation and supports an
   absolute `AUTOMEXIA_VERIFY_TARGET_ROOT` on the intended drive.

## Threat, failure, and resource result

- Ambiguous/unknown/oversized/hostile Unicode input fails before publication.
- Reviews are bound to exact content, revision, generation, monotonic time, and
  all-false authority ceilings.
- Stale generations, stale CAS, clock reversal, overflow, duplicate terminal
  events, missing profiles, and late broadcast events fail closed.
- Broadcast command text remains transient; debug/audit retain digest, byte
  count, target, outcome, diagnostic code, and timestamp only.
- Collections remain bounded at 256 workspaces, 16 windows, 64 panes, 128
  connections, 32 recipe bindings, 50 broadcast targets, 8 KiB command text,
  60 seconds arming, 10,000 profiles, and a 16 MiB library document.
- No changed M6 source performs work on terminal input, PTY, resize, renderer,
  or startup hot paths.

## Evidence commands

Run the focused and semantic gates:

```text
cargo test -p automexia-connectivity --locked --test connection_automation_m6
cargo test -p automexia-connectivity --locked --test workspace_automation_m6
cargo test -p automexia-terminal --locked --test connection_library
cargo test -p automexia-terminal --locked --test m6_workspace_product
python tools/ci/check_m6_workspaces.py
python tools/ci/test_m6_workspaces.py
python tools/ci/s1_assurance.py check-policy
python tools/ci/test_s1_assurance.py
cargo check --manifest-path fuzz/Cargo.toml --locked --bin connection_planning
cargo bench -p automexia-connectivity --locked --bench connection_planning -- connection_plan_workspace_restore_128_connections
```

Before handoff, run the repository reinforcement gates, warning-denied workspace
Clippy, Nextest, documentation tests, full QA, and `cargo ready` as documented in
[Testing](../TESTING.md#m6-typed-automation-and-multi-environment-workspaces).

## Local evidence recorded on 2026-08-27

- Focused Windows x86_64 results: 9 planner, 7 recipe-lifecycle, 11 workspace,
  11 Connection Library, 11 product, 15 Hub-model, and 6 application interaction
  tests passed. The product maximum test used 10,000 profiles with the referenced
  profile last.
- The M6 semantic checker passed with 7 source owners, 19 named real-path tests,
  10 exact limits, and 3 visual surfaces. Nine mutation groups passed; its
  symlink mutation was skipped because this Windows test context could not create
  a symbolic link. The production linked-file rejection has separate native
  store regressions.
- The 28-suite S1 policy and all 15 S1 mutations passed. Feature reinforcement
  passed for 40 features, 131 scenarios, and 176 evidence owners.
- Criterion on this uncontrolled Windows host measured strict JSON parsing of
  the 16-window/64-pane/128-connection workspace at 467.92–539.52 µs (30
  samples; two mild high outliers) and restore resolution for 128 connections at
  285.08–318.00 µs. These are observations, not an activated S2 baseline.
- The stable fuzz target compiled. A 10,000-execution nightly libFuzzer campaign
  could not start on this host: AddressSanitizer lacked
  `clang_rt.asan_dynamic-x86_64.dll`; the no-sanitizer retry then failed to link
  Windows `sancov` boundary symbols. No executions are claimed. Both disposable
  fuzz build directories were removed after recording the failures.
- `cargo fmt --all -- --check` and warning-denied all-target workspace Clippy
  passed. A clean Nextest run passed 2,278 tests with 7 profile skips.
  Workspace documentation tests passed 46 `corcovado` examples and 18
  `rio-window` examples; 3 upstream `rio-window` examples remained explicitly
  ignored.
- `python3 tools/ci/qa.py --full` passed every locally runnable policy,
  mutation, format, dependency, lint, Nextest, documentation, resize-stress,
  session-clone, and Loom gate. It correctly retained S1 controlled artifacts,
  native OpenSSH/Ghostty/Windows GUI, Application Verifier/WPR, controlled
  benchmarks, coverage, and the 30-day baseline as external or opt-in evidence.
- `cargo ready` passed with a short D-drive
  `AUTOMEXIA_VERIFY_TARGET_ROOT` after independently rebuilding
  the workspace with incremental compilation disabled, rerunning warning-denied
  Clippy and unit/integration/documentation tests, checking dependencies,
  building the application, and smoke-checking `automexia 0.4.0`. The initial
  long D-drive target reproduced `LNK1104` at a 270-character output path; the
  new early path guard and absolute-root regression cover that failure. The
  9.45-GiB isolated tree and the remaining 4.85-GB short-path cache were removed.

## Remaining external exit criteria

- two protected exact-head approvals and D3 activation/attestation;
- real Windows, Linux, and macOS managed OpenSSH/PTY lifecycle and descendant,
  handle, socket, listener, cancellation, disable, and uninstall cleanup;
- current-commit S1 native/resource evidence, 9,216-capture visual matrices,
  Narrator/NVDA/VoiceOver/Orca evidence, and independent review;
- S2 comparable controlled measurements and 30 reviewed consecutive days;
- packaged artifact, signing/notarization, and hosted protected-release evidence.

Until those pass, M6 is accurately described as **fully implemented locally at
the review-only boundary and partially implemented overall**.
