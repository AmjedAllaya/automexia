# Implemented feature ownership audit

**Audit date:** 2026-08-26
**Decision:** [ADR 0035](adr/0035-core-domain-and-optional-extension-ownership.md)
**Scope:** every implemented workspace feature family, its production owner,
important consumers, authority boundary, and placement evidence.

## Outcome and acceptance criteria

The audit required that:

- baseline terminal behavior remains available with optional extensions disabled;
- each model, process, PTY, renderer state, persisted record, and UI surface has
  one clear owner;
- provider-neutral contracts do not live under a provider or context extension;
- provider-specific discovery and grammar remain independently disabled;
- terminal engines do not import product domains;
- renderer-independent UI policy remains outside GPU/PTY/VT engines;
- tests, fuzzers, benchmarks, CI, assurance metadata, and documentation follow
  moved sources; and
- machine checks reject a return to mixed ownership.

No user data, public configuration, shortcut, CLI syntax, wire protocol, or
execution authority is intentionally changed by this refactor.

## Evidence ledger

| Feature family | Before audit | Resulting owner | Why this is the logical owner | Evidence / remaining external proof |
|---|---|---|---|---|
| VT parsing, grid, scrollback, selection, terminal modes | Fully implemented and correctly placed | `rio-vt` | Inherited terminal engine state; no product/provider dependency | Workspace tests and architecture dependency check. Native renderer/IME/platform evidence remains release-gated where documented. |
| PTY, ConPTY/Unix process I/O, resize and child lifecycle | Fully implemented and correctly placed | `teletypewriter` plus application session composition | One PTY/process owner; product domains cannot enter the hot path | PTY tests/benchmarks and architecture check; native OS evidence remains governed by S1. |
| Window events, GPU, fonts and rendering primitives | Fully implemented and correctly placed | `rio-window`, `sugarloaf`, `rio-graphics`, `rio-fonts` | Engine/platform primitives are below product policy | Workspace and native visual gates; no domain dependencies. |
| Tabs, panes, routes, search, clipboard, input, chrome and welcome UI | Fully implemented and correctly placed | Desktop application screen/context/renderer adapters | These coordinate live UI/session state and must not become optional extensions | Application tests, keyboard contracts, native/manual visual evidence. |
| Command-result separation, result color and completion pulse | Partially implemented and misplaced | `apps/automexia-terminal/src/renderer/command_results.rs` | Generic per-pane terminal feedback, independent of DevOps semantics | Core route-isolated state, renderer tests, native hooks, architecture and mutation checks. Visual/native matrix still follows S1. |
| Keybindings and Ghostty compatibility | Fully implemented and correctly placed | `automexia-keybindings` with application dispatch | Pure parsing/translation belongs outside screen and engines; app owns actions | Ghostty fixtures, checker, fuzzers and application dispatch tests. |
| Image protocol, decode policy and image state | Fully implemented and correctly placed | `automexia-image` with renderer adapter | Cohesive bounded image domain; renderer owns paint only | Image tests/fuzz/bench and visual/resource gates. |
| Extension contracts and capability vocabulary | Fully implemented and correctly placed | `automexia-extension-api` | Lowest capability-free shared contract layer | Dependency allowlist, conformance tests and protocol fixtures. |
| Extension queues, cache, cancellation and operation lifecycle | Fully implemented and correctly placed | `automexia-extension-runtime` | Shared bounded runtime, separate from provider meaning and app authority | Runtime trust checker, concurrency/model tests and workspace tests. |
| Local DevOps prompt context and semantic classification | Fully implemented but package was over-broad | Narrowed `automexia-devops` | Optional local context/semantic enrichment has an independently disabled lifecycle | Package now contains only context/semantics; no generic domain re-exports. |
| OpenSSH inventory and metadata | Fully implemented and correctly placed | `extensions/devops-ssh` | OpenSSH-specific parsing and inventory are independently disabled provider work | SSH inventory tests, fuzz/bench, D4 contracts and native OpenSSH evidence gate. |
| Provider-neutral connections, plans, direct/routed/tunnel review | Implemented but misplaced under DevOps context | `automexia-connectivity::connections` | Shared capability-free connectivity policy used by app/UI/providers | Moved source/tests/bench/fuzz; D0 schema 6 references current owner while retaining schema 5 exact digest. |
| Provider-neutral authentication and multi-environment workspaces | Implemented but misplaced under DevOps context | `automexia-connectivity::connections::{provider_auth,workspace,automation}` | Shared state/validation without a provider or I/O lifecycle | M6/M7 tests, connection planner benchmark/fuzz and provider contract checks. |
| AWS, Azure, GCP, Kubernetes, OpenShift and Teleport adapters | Fully implemented and correctly placed; dependency edge was misleading | Individual `extensions/devops-*` packages with direct domain dependencies | Each provider owns its bounded parser/version policy/exact typed intent and can be disabled independently | Provider tests/benchmarks/checkers; real accounts, native CLIs and release evidence remain external gates. |
| Quick Actions, aliases, packs, import/projection and provider actions | Implemented but misplaced under DevOps context | `automexia-command-productivity::actions` | Baseline capability-free command productivity, not optional DevOps context | Moved source/tests/benches/fuzz; CP0-CP4 mutation contracts and app integration. |
| Optional native-editor suggestion contracts | Implemented source boundary but misplaced under DevOps context | `automexia-command-productivity::suggestions` | Editor-owned, provider-neutral model with no PTY/grid inference or execution | Moved source/tests/bench/fuzz; CP5 checker and preview-disabled/native evidence status unchanged. |
| Renderer-independent Connection Hub, review, Quick Action and suggestion projections | Fully implemented and correctly placed; dependency now direct | `automexia-ui-model` | Pure layout/accessibility/color/projection policy, no GPU or provider authority | UI model tests and assurance matrix; native accessibility proof remains S1-gated. |
| Ecosystem package validation and sandboxed runtime | Fully implemented at its accepted source boundary and correctly placed | `automexia-ecosystem`, `automexia-ecosystem-runtime` | Separate signed package/sandbox lifecycle; typed actions consume command-productivity contracts directly | D7/CP6 checker/tests; production signing and platform evidence remain release gates. |
| CI, architecture, release and repository policy | Fully implemented and correctly placed | `tools/ci`, `tools/xtask`, workflows and assurance fixtures | Repository policy must remain outside production packages | Ownership checker plus five mutation cases, architecture graph allowlist, full QA/ready gates. |

## Placement rules used for future work

1. Put behavior in a terminal engine only when it is terminal protocol, grid,
   PTY, platform window, GPU, or font machinery with no Automexia product domain.
2. Put always-available product policy in a private capability-free domain
   package when more than one application/UI/provider adapter consumes it.
3. Put live session, renderer, persistence, process, filesystem, clipboard,
   credential, or window authority in the desktop composition root unless an
   accepted lower owner already exists.
4. Put renderer-independent layout, accessibility, color and presentation models
   in `automexia-ui-model`; keep actual paint in the application renderer.
5. Create or use an extension only for independently disabled provider/domain
   behavior with explicit capabilities, quotas, cancellation and uninstall
   behavior.
6. Do not create a new extension merely to move files. First prove a distinct
   lifecycle, trust boundary, or replaceable provider contract.
7. Keep proposed Automation Studio, Diagnostic Navigator, LLM Orchestration and
   Production Operations work documentation-only until their own accepted phase
   authorizes production owners. They were not treated as implemented features
   in this audit.

## Build, wrap or adopt result

This change uses existing workspace packages and dependencies. It builds two
small private domain libraries from already implemented source, wraps no new
external tool, adopts no dependency, and creates no new runtime/plugin surface.
The official Cargo workspace model supports shared lockfile, output and
workspace-wide checks across these members. Optional Cargo features were rejected
because the moved contracts are baseline product policy, not optional compiled
functionality.

## Security, performance and rollback

- The moved libraries remain capability-free and add no unsafe code, I/O,
  environment reads, credentials, network, process, PTY, renderer or extension
  lifecycle.
- Input limits, validation, redaction, generation isolation and deterministic
  state machines move unchanged with their tests.
- Result animation remains bounded to 540 ms and state is retained only for
  visible route identities.
- No work was added to VT parsing, PTY I/O, input, resize or startup hot paths.
- Persisted compiler/registry identity strings containing `automexia-devops` are
  deliberately retained as compatibility identifiers; changing them would be a
  separate versioned data migration, not an ownership cleanup.
- Rollback is a normal source revert: there is no persisted schema migration.
  Historical D0 fixtures must not be edited during rollback.
- The two pre-existing untracked root documents are user-owned and excluded from
  this change.

## Automated enforcement

`tools/ci/check_feature_ownership.py` rejects:

- generic action/connection/suggestion source under `automexia-devops`;
- UI or provider dependencies through the wrong package;
- command-result code in `devops_status.rs`;
- DevOps-gated result paint or clearing results when DevOps is disabled;
- missing route-isolated result state; and
- product-domain dependencies from terminal engines.

`tools/ci/test_feature_ownership.py` proves the checker fails against deliberate
mutations for extension-gated paint, mixed DevOps source/provider dependencies,
and engine-domain coupling. `cargo xtask verify architecture` runs both on every
architecture verification path.

## Local verification evidence

The completed 2026-08-26 campaign ran on Windows x86_64 with the repository's
locked dependency graph:

- ownership checker mutation suite: 5 passed;
- focused command-result native-hook suite: 15 passed;
- moved connectivity, command-productivity, DevOps-context and UI-model package
  tests: passed;
- AWS, Azure, GCP, Kubernetes, OpenShift and Teleport package tests: passed;
- all workspace fuzz target binaries: compiled with the standalone locked
  fuzz manifest;
- connectivity and command-productivity Criterion benchmarks: compiled;
- `cargo fmt --all -- --check`: passed;
- warning-denied workspace Clippy across all targets: passed;
- workspace Nextest CI profile: 2,235 passed and 7 skipped;
- locked workspace documentation tests: passed;
- full repository QA: passed; and
- `cargo ready`: passed from isolated verification artifacts, including
  repository validation, architecture contracts, workspace tests, dependency
  advisory/license/source policy, and a real `automexia.exe --version` check.

The local campaign does not replace private release manifests, real provider
accounts and CLIs, native non-Windows jobs, controlled GPU/benchmark hardware,
AppVerifier, WPR, screen-reader assessment, or the 30-day stability baseline.
Those remain explicit external release gates rather than inferred passes.
