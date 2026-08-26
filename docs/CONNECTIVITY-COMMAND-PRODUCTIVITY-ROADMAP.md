# Connectivity and command-productivity focus roadmap

Status: canonical execution source of truth for the current product focus.

Audited committed baseline:
`20ff7928ea2d1eac5d26f13c62d0cda9b86bc078` on 2026-08-23.

This is the **v0.5 activation-hardening lane**. It may proceed in parallel with
v0.4 release closure, but provider/credential/managed-SSH completion is not a
v0.4 release prerequisite, and v0.4 evidence does not authorize activation.

## Outcome and scope

This roadmap is the ordered implementation checklist for:

- shell-native autocomplete and optional Automexia suggestions;
- typed Quick Actions, aliases, DevOps packs, and trusted local workspace tasks;
- OpenSSH inventory, Connection Hub, reviewed SSH launch, jumps, tunnels, and
  lifecycle;
- reusable connection profiles, automation recipes, and declarative remote
  workspaces; and
- isolated AWS, Azure, Google Cloud, Kubernetes, OpenShift, Teleport, and
  OpenBao workflows plus provider-aware Quick Actions.

It does not make planned commands or screens shipped behavior. Exact current
behavior remains owned by the feature catalog and public references. Public
ecosystem packs, direct provider SDK inventory, embedded SSH, secret custody,
SFTP, collaboration, CP6 selected-input model suggestions, and optional LLM
workflow orchestration remain outside this focus until their separate protected
phases are approved.

## Authority and change control

The documentation authorities have distinct jobs:

1. [Roadmap](ROADMAP.md) owns release sequencing and canonical phase status.
2. This page owns the focused execution order, dependencies, and checklists.
3. The [detailed SSH, connectivity, multi-environment, and multi-cloud plan](SSH-CONNECTIVITY-MULTI-ENVIRONMENT-MULTI-CLOUD-PLAN.md)
   owns the checklist-level M0-M13 implementation and evidence protocol.
4. [Phase implementation audit](PHASE-IMPLEMENTATION-AUDIT.md) owns the
   evidence-backed fully/partially/not-implemented reconciliation.
5. [Command Productivity](COMMAND-PRODUCTIVITY.md), [Connection Hub](CONNECTION-HUB.md),
   [SSH connections and automation](SSH-CONNECTION-AUTOMATION.md), and
   [Terminal-first remote operations](TERMINAL-FIRST-OPERATIONS.md) own detailed
   product and technical contracts.
6. Accepted ADRs own durable trust, process, credential, and architecture
   decisions. An accepted ADR wins if prose conflicts.
7. Source, tests, benchmarks, feature assurance, and current documentation must
   agree before a checkbox or status advances.

When a future request says "next phase", execute the first unchecked primary
phase whose dependencies are complete. Re-audit that phase against source and
tests before editing. Do not skip a security or native-evidence dependency to
reach a later UI or provider slice.

## Status and checkbox rules

| Marker | Meaning |
|---|---|
| Fully done | The defined local/source behavior, automated evidence, and docs agree. External release proof is listed separately. |
| Partially done | Useful implementation or evidence exists, but the phase exit gate is incomplete. |
| Not done | No production capability satisfies the phase exit gate. Detailed specifications may still exist. |
| [x] | Proven by current source, tests, and documentation at the stated boundary. |
| [ ] | Required work or evidence remains. |

Only the three canonical status labels may appear in phase summaries. A phase
update must synchronize this page, the main roadmap, the phase audit, testing
documentation, feature assurance, and a change fragment.

## Current evidence ledger

| Area | Phase | Status | Evidence owner | Remaining exit |
|---|---|---|---|---|
| Provider-neutral contracts | D1 | Fully done | automexia-extension-api, automexia-extension-runtime, automexia-connectivity, automexia-command-productivity, automexia-devops, automexia-ui-model; architecture gates | Hosted release evidence only |
| Generic context and immutable capsules | D2 | Fully done | extension API/runtime plus application session/context tests | Production login/relaunch belongs to D3/D6 |
| SSH decision and native fixture baseline | D0 | Partially done | ADR 0012 is accepted; schema 6 ratchets immutable schemas 1/2/3/4/5 with exact M3-M5 routes/tunnels, trust/status, lifecycle/receipt/reconnect, 23-scenario native-manifest rules, host/artifact binding, a protected manual workflow, and mutations | Obtain ADR 0003's two independent exact-head protected approvals/server enforcement; execute and validate real native F4/F5 manifests |
| Exact-argument launch broker | D3 | Partially done; nonactivated | Production-compiled fail-closed broker, bounded current-executable review worker, exact guarded PTY seam, route publication, process-group/Job Object teardown, PTY-worker joining, bounded lifecycle/audit state, approval UX, and local protected-path policy | Protected approvals/server enforcement, real loader attestation, and native OpenSSH descendant/process/resource/accessibility evidence |
| Static OpenSSH inventory | D4 | Fully done | automexia-devops-ssh, hostile/property tests, fuzz, benchmark, assurance | Remains deliberately disabled until D5 |
| Connection Hub model | D5.0 | Partially done | All local F2 schemas, reducers, dry-run planner, Hub/review/planner projections, fixtures, goldens, fuzz, mutations, and benchmark are implemented | ADR 0003 protected approvals and native evidence remain for D5.2; capability-free D5.1 is complete under ADR 0022 |
| Read-only Connection Hub | D5.1 | Fully done locally; external evidence partial | App-scoped joined runtime, exact reviewed native selection, bounded modal/search/filter/grouping, CAS favorite/tag diffs, read-only recent/library data, disabled authority, tests, benchmark, and release-build evidence | Native macOS/Linux picker/permission plus controlled Narrator/NVDA, VoiceOver, and Orca evidence |
| Managed OpenSSH | D5.2 | Partially done overall; F5.1-F5.3 and the F5.4 local assurance path complete nonactivated | Exact routes/tunnels/trust/lifecycle plus native host/commit/OpenSSH/artifact binding, protected manual workflow, receipts, reconnect, and path-free summary pass locally | Protected activation/attestation, actual status execution, real OpenSSH descendant/listener cleanup, native resources/accessibility, and validated F5.4 real manifests |
| Profiles, recipes, remote workspaces | D5.0-D5.2 | Partially done | Bounded profiles, typed recipes/actions, strict validation, deterministic dry-run planning, approval fingerprints, and private transactional Connection Library persistence/redacted transfer are implemented | Product editor, remote workspace lifecycle, and separately gated execution remain |
| Multi-cloud framework and providers | D6.0-D6.5 | Partially done overall: M7 fully done locally; M8-M11 and M12 Teleport source/cached review plus M11 private lifecycle fully done locally and nonactivated; OpenBao not done | Provider-neutral capsules, independent exact adapters, strict cached publication, six-provider Hub review, bounded private validation/revalidation/revoke/cleanup, tests, and benchmarks | D3 activation, ADR 0024 acceptance plus OpenBao implementation, and real official-CLI/cluster/native/resource/accessibility/release evidence |
| Native completion | CP1 | Fully done | shell integration, xtask completion manager, CP1 contract/tests | Hosted three-OS and longitudinal release evidence |
| Quick Actions and persistence | CP2.0-CP2.2 | Fully done | automexia-command-productivity model, application store/worker/UI/CLI tests | Hosted shell insertion, controlled accessibility, longitudinal evidence |
| Aliases and DevOps packs | CP3.0-CP3.2 | Fully done | compiler, private generations, shell activation, packs, fuzz/bench/contracts | Hosted native and controlled baseline evidence |
| Trusted local workspace tasks | CP3.3 | Fully done | native imports, workspace store/trust/runtime/CLI tests and ADR 0021 | Hosted native/accessibility and longitudinal evidence |
| Provider-aware Quick Actions | CP4 | Fully done locally at product-integrated nonactivating boundary; partially done overall | Seven provider projections, retained cached-product handoff, exact route/session/revision publication, idempotence/revocation, cached search, final revalidation, accessible context, production confirmation, fuzz/benchmark/policy evidence | Approved provider refresh/capsule production, exact provider execution, OpenBao, and native provider/accessibility/release evidence |
| Autocomplete research | CP5.0/P1 | Fully done | Shell/API matrix, pure bounded insertion prototype, locked matcher benchmark, dependency/privacy review, retain-CP1 decision | External low-end/Zsh/Fish/accessibility evidence applies only to a future P2 proposal |
| Automexia suggestion surface | CP5.1-CP5.6/P2-P4 | Partially done overall; CP5.1-CP5.4 fully done at source/local boundaries; CP5.5 inert bridge source complete | Accepted ADR; authenticated request/reply protocol; restrictive endpoints; bounded broker/sources/ranking; pane UI/publication/route exchange; packaged helper target; four bidirectional adapters; real local shell/mutation/lifecycle evidence | Reviewed launcher, signed/attested artifact, WSL relay, live activation, successful three-OS endpoint/shell/accessibility/package/rollback/resource/30-day evidence |
| Situation-aware production operations | PO0-PO8 | PO0 partially done at detailed proposal/checker boundary; PO1-PO8 not done | Canonical specification, exact proposed PO0 contracts, UX/build blueprint, primary-source baseline, proposed ADR 0034, strict JSON/digest/checker/mutations/nonactivation scan, owner/trust/resource/privacy boundaries, future testing ladder and roadmap integration | Protected ADR/exact-digest and owner/dependency/capability acceptance, prototypes and first real-path failures before source; CP5/D6/D3/D7 phase gates; no passport; explain/change/compare/network/SLO/dependency view; candidate; preflight; Incident log/time/handoff; journal; managed operation/diagnostic session; pack; adapter; model; execution; native; or release evidence exists |
| Signed ecosystem packs and selected-input model suggestions | CP6/D7 | Partially done overall; accepted nonactivating source boundary fully done locally | Accepted ADR 0029 and exact digest; private strict domain/runtime crates; signed local verification and protected atomic store; optional no-WASI Wasmtime conformance host; exact capability/lifecycle/consent models; disabled action packs and selected-input product adapter; 17 contract mutations, properties, fuzz harness and benchmarks | Protected release permit and public distribution/SDK/provider decisions; production trust governance; native Linux/macOS, signed-package, visual/accessibility, malicious-component, resource/lifecycle and soak evidence |
| Optional LLM workflow orchestration | LO0-LO5 | LO0 partially done as documentation only; LO1-LO5 not done | Canonical specification, proposed ADR 0033, neutral-workflow direction, provisional limits and future evidence ladder | ADR acceptance and strict machine contract before source work; no model/provider, registry, execution, MCP, UI, native or release evidence exists |

The workspace term has two separate meanings and must remain explicit:

- CP3.3 trusted local workspace task bridges are fully done and insert-only.
- Remote declarative workspaces that restore layouts, connections, capsules,
  tunnels, and safe startup recipes are not done and belong to D5/D6.

## Locked architecture and security decisions

- Core owns one capability decision boundary, one exact external-tool runner,
  session/capsule policy, PTY/route attachment, redaction, and UI semantics.
- First-party extensions own bounded provider-specific parsing and workflows.
- System OpenSSH owns SSH protocol, host-key behavior, agent interaction, key
  files, certificates, and connection diagnostics.
- Official AWS, Azure, Google Cloud, Kubernetes, OpenShift, and organization
  CLIs own login, MFA, token caches, browser/device flows, and provider protocol.
- Native shell editors own buffers, cursor, history, quoting, completion, and
  insertion. Terminal cells and remote output are never command intent.
- Automexia persists public metadata and opaque references only. It does not
  persist passwords, private keys, passphrases, cloud tokens, ExecCredential
  payloads, agent messages, browser cookies, or terminal contents.
- All process launches use a validated executable and exact argument array.
  Shell evaluation and command-string concatenation are forbidden.
- Provider, network, authentication, and filesystem discovery stay off startup,
  keystroke, PTY, resize, and renderer hot paths.
- Every operation is bounded, cancellable, route/session/generation isolated,
  revocable, redacted, and cleaned on failure, replacement, and shutdown.
- Disabling every integration must restore a complete ordinary terminal.

## Phase-name normalization

Older documents use D5A-D5E. They map to the canonical roadmap without adding
a second phase system:

| Older work package | Canonical owner |
|---|---|
| D5A contracts and goldens | D5.0 |
| D5B read-only Hub | D5.1 |
| D5C non-executing recipe editor/planner | D5.0 model work, delivered before D5.2 execution |
| D5D managed OpenSSH | D5.2 launch and connection lifecycle |
| D5E safe automatic actions | D5.2 automation slice after basic SSH is proven |

## Dependency and execution order

Primary connectivity chain:

1. F1/D0 closes the decision and fixture baseline.
2. F2/D5.0 freezes non-executing models and UX.
3. F3/D5.1 connects D4 to a read-only Hub.
4. F4/D3 activates the one reviewed launch boundary.
5. F5/D5.2 delivers managed OpenSSH in bounded slices.
6. F6 completes profiles, recipes, and remote workspaces.
7. F7/D6.0 activates provider-neutral auth/capsule orchestration.
8. F8-F12 deliver providers independently.
9. F13/CP4 adds provider-aware Quick Actions after public context exists.

Autocomplete lane:

1. P1/CP5.0 research is fully done and retains CP1 as the complete solution.
2. P2/CP5.1 is fully done at the source boundary and partial for release; authenticated replies, the application route exchange, and exact publication limits are present while activation stays disabled pending native Unix/WSL endpoint evidence.
3. P3/CP5.2-CP5.3 is fully done at the source/local model boundary; named-hardware and activated native fixtures remain release evidence.
4. P4/CP5.4-CP5.6 is partially done overall: CP5.4 UI/publication source and the CP5.5 inert helper/adapters are implemented; launcher/signing/live composition, public activation, and release gates remain open.

CP5 never blocks production SSH. CP1 remains the complete fallback throughout.

Production Operations lane:

1. PO0 accepts the decision, freezes owners/limits/contracts/tests, and preserves
   nonactivation.
2. PO1 follows the relevant D2/D6 read-only context evidence.
3. PO2 adds bounded adapter-owned change/ownership/drift, resource/scheduling
   explanation, healthy cohort/revision comparison, passive network diagnosis,
   approved SLO summaries and dependency/impact graphs.
4. PO3 follows CP5 activation evidence and keeps CP1 as fallback.
5. PO4-PO5 add preflight/policy/GitOps/JIT and Incident Mode with hypotheses,
   time/log navigation, journal and handoff without execution.
6. PO6 waits for independent D3 and provider activation before any managed
   operation, port forward, active probe or debug session.
7. PO7 waits for the ecosystem capability/distribution decision.
8. PO8 adds only independently proven adapters, bounded read-only cross-
   environment comparison and optional local tie-breaking.

## F0 - preserve completed foundations

Status: Fully done. Release assurance remains partially complete.

- [x] Preserve D1 provider-neutral crate and dependency boundaries.
- [x] Preserve D2 immutable capsule, session, cache, cancellation, and freshness
  behavior.
- [x] Preserve D4 as a capability-free, disabled OpenSSH inventory package.
- [x] Preserve CP1 native completion and explicit bounded refresh.
- [x] Preserve CP2 Quick Action model, persistence, search, review, and
  insert-without-Enter behavior.
- [x] Preserve CP3 compiler, aliases, packs, imports, and trusted workspace
  task bridges.
- [ ] Retain hosted Windows/Linux/macOS, controlled accessibility, and 30-day
  performance/resource evidence as separate release gates.

Exit: no later phase weakens these contracts or duplicates their owners.

## F1 - close D0 SSH decision and native fixture baseline

Status: **Partially done.** The complete local D0/D3 contract is implemented;
ADR 0012 is accepted by the project owner. ADR 0003's protected exact-head
approvals/server enforcement and later native runtime evidence remain external
gates. Production launch is still disabled.

- [ ] **Partially done - protected external gate:** preserve the accepted
  ADR 0012 decision and obtain the two current independent exact-head approvals
  plus non-bypassable server-side protection required by ADR 0003.
- [x] **Fully done locally:** preserve manual `ssh host` ownership in
  PowerShell, CMD, Bash, Zsh, and WSL; managed launch is additive, never
  downloads or installs OpenSSH at startup/launch, and missing clients produce
  redacted platform guidance without substitution.
- [x] **Fully done locally:** freeze all nine D0 trust boundaries with accepted
  and returned data, limits, cancellation owner, log policy, and failure mode.
- [x] **Fully done locally:** freeze exact package identity, digest/version
  and contract compatibility, repository-reviewed/first-party-signed proof,
  unverified-package denial, capability grant, expiry, revocation, and audit
  fields.
- [x] **Fully done locally:** freeze system OpenSSH executable resolution for
  Windows, macOS, Linux, and disabled WSL without PATH/cwd ambiguity or
  configured-path fallback.
- [x] **Fully done as a required matrix:** define direct alias, explicit
  destination, user/port, encrypted key, agent, certificate, first/known/
  changed host key, ProxyJump, local/remote/dynamic forwarding, cancellation,
  remote exit, hostile output, offline, shutdown cleanup, and 1/10/50-session
  fixtures for Windows, macOS, Linux, and WSL. Native execution belongs to
  F4/F5 and is not claimed here.
- [x] **Fully done as a fixture protocol:** freeze hermetic loopback setup,
  isolated disposable credentials/`known_hosts`/agent state, bounded probes and
  timeouts, DNS/connect/auth cancellation points, platform activation semantics,
  six zero-resource cleanup invariants, evidence metadata, artifact policy, and
  nine redaction surfaces. Execution remains F4/F5-owned.
- [x] **Fully done locally:** record strict host-key, forwarding, loopback,
  environment, secret, shell, and discovery-command defaults.
- [x] **Fully done locally:** retain immutable schema 1, enforce schema 2, and
  add mutation tests that fail when process, network, secret, environment,
  shell-evaluation, manual-baseline, trust-boundary, fixture, cleanup,
  evidence, or redaction guarantees widen or drift.
- [x] **Fully done locally:** update architecture/security guidance, testing,
  phase audit, roadmaps, feature assurance, decision index, and change fragment.

Exit status: the local fixture/contract gate passes and production launch
remains disabled. F1 cannot become **Fully done** until ADR 0003's protected
review evidence exists; native fixture execution remains owned by F4/F5.

## F2 - implement D5.0 Connection Hub and planning models

Status: Partially done. Every local, non-executing F2 deliverable is fully done;
ADR 0012 is accepted by the project owner, while ADR 0003 protected approvals
and native activation evidence still prevent phase closure or activation.

Second-pass implementation audit (2026-08-17):

| F2 requirement | Before this audit | Final status | Closing evidence |
|---|---|---|---|
| Versioned records and validated types | **Partially done** | **Fully done locally** | Validation wrappers are sealed from external construction; strict parsers remain the only public validated-value ingress. |
| Hostile input, duplicates, bounds, and policy | **Partially done** | **Fully done locally** | Plan overrides now share the bidi/control filter; duplicate policy/executable review entries fail closed. |
| Deterministic approval fingerprints | **Fully done locally** | **Fully done locally** | All material-field and order-canonicalization regressions remained green. |
| Authentication and result state models | **Partially done** | **Fully done locally** | Auth completion/cancellation is correlated to the active operation; late generations and noncanonical IDs fail closed. |
| Hub/review/planner UX and accessibility | **Partially done** | **Fully done locally** | Missing selections retain one roving focus target; loading is live progress; route tab cycles remain trapped; planner labels exclude value contents. |
| Deep assurance and performance ownership | **Fully done locally** | **Fully done locally** | The F2 checker now requires 30 regressions and forbids validation bypasses and planner panic primitives. |
| Process/network/provider/credential/PTY/listener authority | **Fully disabled** | **Fully disabled** | The authority checker and all-false resolved-plan ceiling remain unchanged. |

- [x] **Fully done locally:** added bounded versioned ConnectionDefinition,
  Observation, Intent, Review, Receipt, Profile, Recipe, Step, Tunnel, and
  ResolvedConnectionPlan models in provider-neutral owners; validated wrappers
  cannot be forged through a public conversion.
- [x] **Fully done locally:** reject unknown versions, hostile controls/bidi in
  documents and plan overrides, duplicate review identities, cycles, oversized
  values/counts, secret-bearing fields, free-form command strings, and invalid
  risk/failure/retry combinations.
- [x] **Fully done locally:** deterministic fingerprints invalidate approval
  when target, identity, route, executable, tunnel, recipe, capability, or
  source changes; unordered executable/capability inputs canonicalize first.
- [x] **Fully done locally:** authentication/result reducers cover missing,
  locked, expired, MFA, cancelled, offline, denied, unsupported, stale, error,
  active, waiting, success, warning, failure, and skipped states, including
  illegal-transition, operation-generation correlation, canonical event IDs,
  and no-background-authentication rules.
- [x] **Fully done locally:** renderer-neutral wide/medium/narrow Hub, Connection
  Review, and 64-step recipe-planner models cover keyboard, focus trap/return,
  reading order, stale-selection focus recovery, live loading progress,
  route-aware modal tab cycles, value-redacted action labels, high contrast,
  reduced motion/transparency, 100-400% text scale, empty/loading/error states,
  and structured responsive/accessibility goldens.
- [x] **Fully done locally:** synthetic fixtures cover all ten providers and all
  authentication states; integration, property, mutation, accessibility,
  hostile-record, fuzz, and 64-step Criterion benchmark owners are registered.
- [x] **Fully done locally:** process, network, provider, credential, PTY,
  listener, renderer, and GPU authority remain disabled; models are pure and do
  not read files, spawn tools, connect sockets, resize a PTY, or draw a window.
- [ ] **Not done externally:** ADR 0012 is accepted, but two independent
  exact-head approvals and non-bypassable server rules still require plan-
  supported protection, eligible reviewers, and a green exact-revision PR before
  D5 can close or any production connection capability can activate.

Implementation evidence (2026-08-17):

- model owners: `automexia-connectivity/src/connections` and
  `automexia-ui-model/src/connection_hub.rs`;
- frozen contract/fixtures: `tests/fixtures/connection-hub`;
- deterministic tests: all `automexia-connectivity` and `automexia-ui-model` tests,
  plus `tools/ci/test_connection_hub_f2.py`;
- deep owners: `fuzz/fuzz_targets/connection_planning.rs` and the
  `connection_planning` Criterion benchmark;
- architecture and CI ratchets reject authority drift, lost limits, missing
  evidence, public validated-wrapper construction, planner panic primitives,
  late auth results, or weakened modal/accessibility/privacy invariants; 30
  named Rust regressions are required by the F2 checker.

Exit status: the entire local product slice is testable without an account,
network, process, PTY, window system, or GPU. The technical F2 exit passes;
overall phase status remains **Partially done** because ADR 0003 protected
activation evidence is still external. F2 alone implied neither D5.1 nor D5.2;
F3/ADR 0022 now activates
only the capability-free D5.1 product.

## F3 - implement D5.1 read-only Connection Hub

Status: **Fully done at the source and local Windows boundary.** External native
platform/accessibility release evidence is **Partially done**.

- [x] **Fully done** — Connect immutable D4 snapshots to the existing bounded,
  virtualized Hub catalog through one Router-owned service. One joined worker
  uses a capacity-two non-blocking inbox, coalesces obsolete generations,
  cancels scans, publishes before route wake, retains last-known-good state,
  reports redacted saturation, and shuts down deterministically.
- [x] **Fully done** — Require an explicit, parented native multi-file picker,
  exact canonical-path review, and confirmation before creating memory-only D4
  grants. Each review token belongs to its initiating screen; another route
  cannot display, confirm, cancel, or revoke it. No selected path is persisted
  or logged; link/reparse and stale-token validation fail closed.
- [x] **Fully done** — Register a configurable `OpenConnectionHub` action with
  mnemonic `Ctrl+Shift+H` / `Cmd+Shift+H` defaults, exact palette labels, and
  cross-platform collision/mode-ownership tests. The launcher only opens the
  existing read-only controller and adds no terminal or connection authority.
- [x] **Fully done** — Render the real topmost modal with search, source,
  favorites, recent, tags, grouping, clear-filters, virtualized selection,
  inspector, setup/loading/error/filtered states, keyboard, pointer, IME, focus
  restoration, responsive geometry, and disabled Connect/Login/refresh/run.
  Catalog projections/summaries are cached across unchanged frames, and IME
  preedit validation does not traverse the catalog.
- [x] **Fully done** — Edit only public favorites and tags through reviewed
  before/after diffs and D4 revision CAS. Conflicts reload without overwrite;
  hostile control/bidi tags fail closed; recent remains read-only until D5.2.
- [x] **Fully done** — Initialize and display the private Connection Library
  snapshot/recovery state without executing profiles, recipes, or preferences.
- [x] **Fully done** — Cover 10 runtime, 8 controller, 2 bounded-worker/cache
  unit, 4 renderer, 33 D4, 32 UI-model, 5 library, and 46 palette tests on
  Windows 11. `cargo deny check` passed. The 50-sample 10,000-record warm run
  measured 7.2849–7.5959 ms against the below-16-ms target; the first immediate
  post-LTO run measured a noisier 8.5180–9.5607 ms and was investigated. The
  release executable grew 650,752 bytes (2.96%) from the pre-M1 baseline.
- [ ] **Partially done** — Native macOS/Linux picker and static-permission runs,
  plus controlled Narrator/NVDA, VoiceOver, and Orca verification, remain
  external release evidence. Structural accessibility and tiny-to-8K geometry
  are covered locally and are not presented as native screen-reader proof.

Exit: users can safely browse, diagnose, tag, and favorite reviewed D4 inventory
without connection, authentication, provider, process, network, listener, or
PTY authority. D5.2 remains disabled.

### User-visible shortcut completion audit (2026-08-23)

Scope: every command-palette feature that previously had a blank or mismatched
default. Internal/configuration-only actions remain intentionally outside this
table; a shortcut is not added merely to increase coverage.

| Feature | Previous evidence | Resulting status | Windows/Linux/BSD | macOS |
|---|---|---|---|---|
| Connection Hub | Palette action only; no typed/default binding | **Fully done locally** | `Ctrl+Shift+H` | `Cmd+Shift+H` |
| Quick Actions | Palette route only; no typed/default binding | **Fully done locally** | `Ctrl+Shift+O` | `Cmd+Shift+O` |
| Extensions marketplace | Palette route only; no typed/default binding | **Fully done locally** | `Ctrl+Shift+M` | `Cmd+Shift+M` |
| Font browser | Palette route only; no typed/default binding | **Fully done locally** | `Ctrl+Shift+L` | `Cmd+Shift+L` |
| Pane-local terminal search | Separate pane opening with a display-only scope chip | **Fully done locally — one live scope session** | `Ctrl+F` | `Cmd+F` / `Cmd+B` |
| Visible-pane workspace search | Separate global opening with no in-place scope switch | **Fully done locally — one live scope session** | `Ctrl+Shift+F` / `Ctrl+Shift+B` | `Cmd+Shift+F` / `Cmd+Shift+B` |
| Appearance | Windows-only default; Linux/macOS incomplete | **Fully done locally** | `Alt+Shift+T` | `Cmd+Alt+Shift+T` |
| Close tab / close surface / close other tabs | Actions existed; labels/defaults overlapped or were blank | **Fully done locally** | `Ctrl+F4` / `Ctrl+Shift+W` / `Ctrl+Shift+F4` | `Cmd+Shift+W` / `Cmd+W` / `Cmd+Alt+W` |

The four launcher actions and both search scopes have stable configuration
names and share their existing Screen-owned routes. App launchers remain
inactive under Search, Vi, and alternate-screen ownership. Local/global search
shortcuts deliberately remain active under Search so they switch the same
session in place; Vi continues to exclude those launchers and normalizes an
explicit custom invocation to pane scope.

Scope changes retain the bounded query and query focus, recalculate matches,
move the same responsive surface, update a visible-viewport result count capped
at `999+`, and publish a privacy-safe renderer-neutral announcement. Repeating
the active scope shortcut only refocuses the query. Pointer and keyboard scope
selection never inserts or executes PTY input. Host-independent binding tests,
renderer-neutral geometry/semantics tests, and native Windows WGPU/CPU
split-pane runs own local evidence. Native macOS/Linux keyboard-layout,
visual/IME, and controlled assistive-technology evidence remain external
release gates.

## F4 - activate D3 exact-argument process and PTY lifecycle

Status: **Partially done; nonactivated.** ADR 0012 is accepted by the project
owner. All source-local runner, exact-spawn, publication, and approval-UX
obligations below are implemented, but the linked package remains deliberately
unverified and the production activation constant remains false.

- [ ] **Partially done (external prerequisite)** — ADR 0012 records the accepted
  decision. Obtain ADR 0003's two current independent human exact-head approvals
  and non-bypassable server-side protection. Local CI rejects author, bot,
  duplicate-login, stale-head, malformed, or authority-path-bypassing evidence.
- [ ] **Not done (external trust prerequisite)** — Bind the real loader/build
  provenance, signed or repository-reviewed package verification, and live
  revocation state. The linked in-process candidate intentionally reports
  Unverified, so the broker denies before executable or filesystem resolution.
- [x] **Fully done locally; nonactivated** — Router owns one shared
  ExternalToolRunner. It constructs exact typed capability/launch requests,
  admits at most 50 active operations, retains at most 256 redacted FIFO audit
  records, captures one trusted process-start cwd, copies only a bounded
  allowlist of public environment values, and reconciles on route/application
  teardown.
- [x] **Fully done locally; nonactivated** — The Connection Review exposes
  allow once, allow for session, and deny with A/Enter, S, and D shortcuts,
  pointer targets, focus ownership, icons plus text/color, a 60-second decision,
  exact session.launch scope, route, launcher/package, target, risk, one
  literal argument, and PTY I/O. Diagnostics are fixed, actionable, and
  renderer-safe; private aliases, paths, digests, environment, and terminal data
  remain outside the rendered/audit surfaces.
- [x] **Fully done locally; nonactivated** — The broker binds package policy,
  executable identity, operation/session/capsule, decision, destination, and
  exact argv. It opens and re-compares the executable guard consumed by the
  platform PTY seam, closing the local check-to-spawn handoff without shell
  evaluation, PATH search, current-directory search, or implicit Enter.
- [x] **Fully done locally; nonactivated** — Approval queues a capacity-one,
  latest-generation current-executable review off input, PTY, renderer, and
  startup hot paths. The worker publishes before the route-bound wake; the
  controller accepts only an exact current preparation within 30 seconds and
  invalidates it on refresh or exit. A second explicit decision binds that
  review, and the guarded identity is revalidated before spawn.
- [x] **Fully done locally; nonactivated** — ContextManager alone creates the
  guarded PTY, inserts exactly one new route whose numeric route equals the
  reviewed session, and marks publication only after insertion. Any create,
  scope, capacity, or publication failure cancels and revokes the exact lease.
- [x] **Fully done locally; native evidence external** — Completion, explicit
  cancellation, session
  revocation, stale-lease rejection, failed-publication rollback, route-close
  reconciliation, and application shutdown are implemented and deterministic.
  Windows uses bounded Job Object termination, Unix retains the waitable leader
  while signalling its owned process group, and PTY workers are joined with a
  deadline. Real OpenSSH descendant/listener/handle leak proof remains native.
- [ ] **Partially done** — Hostile argv, Unicode/spaces/leading dashes,
  executable replacement, replay, stale grants, session isolation, 1/10/50
  pure cycles, runner capacity/audit bounds, publish-before-complete, shutdown,
  route mapping, keyboard focus/mnemonics, accessibility semantics, and
  tiny-to-8K pointer geometry pass locally. Native process, pane/window,
  OpenSSH-server/descendant, screen-reader, and sustained resource runs remain
  external.
- [ ] **Not done (external native evidence)** — Pass Windows ConPTY, macOS PTY,
  Linux PTY, and separately gated WSL process-tree evidence plus controlled
  latency, CPU, memory, handle, listener, audit-storage, and cleanup checks on
  the exact protected build.

Exit remains unavailable: the generic boundary is source-complete locally but
no package can inherit a production grant until every protected, attestation,
native, resource, and accessibility gate above is evidenced.

## F5 - implement D5.2 managed OpenSSH

Status: **Partially done overall; F5.1-F5.3 are fully done at the
nonactivated source boundary.** Production SSH remains blocked by F4/M2
protected gates and F5.4 controlled native evidence.

### F5.1 direct reviewed SSH

- [x] **Fully done — preparation and product review:** one current direct D4
  record or bounded literal host maps to a canonical F2 plan and the responsive,
  value-redacted Connection Review with explicit pointer/keyboard/accessibility
  decisions and the existing focus/IME/tiny-to-8K protections.
- [x] **Fully done — exact managed request:** 17 fixed defensive `-o` options
  precede exactly one typed destination in the F5.1 grammar. M4 adds separate
  typed user/port/config routes; arbitrary options,
  proxy/jump, forwarding/tunnel, local/remote command, shell evaluation, and
  multiplex/background behavior cannot enter the M3 grammar. OpenSSH retains
  authentication, host-key prompt, PTY I/O, post-quantum negotiation defaults,
  and weak-crypto warnings.
- [x] **Fully done — identity and authorization binding:** a fresh full-review
  equality check creates the opaque binding. Profile/source/capsule/plan,
  observation freshness/generation, host trust, capability, destination, and
  executable identity changes invalidate it; the broker revalidates native file
  identity and rejects reordered, omitted, altered, or extra argv.
- [x] **Fully done — lifecycle, receipts, and reconnect source:** ContextManager
  remains the only PTY/route publisher. Actual child status—not route close—owns
  success/failure/unavailable classification. Cancellation and shutdown are
  distinct. Fixed redacted notifications, a 256-record/2-MiB private atomic
  primary/previous receipt store, nonblocking bounded worker dispatch, restart
  recovery, and current-D4/source-revision reconnect preparation pass locally.
  Reconnect never auto-runs and always returns to fresh review and approval.
- [x] **Fully done — source evidence:** active schema 6 plus immutable
  schema-1/schema-2/schema-3/schema-4 hashes, hostile/exact-argv/executable-replacement/
  outcome/redaction/store recovery/saturation/restart/stale-source tests and
  assurance mutations pass.
- [ ] **Not done externally / production blocked:** protected exact-head
  approvals/server enforcement, real loader attestation/revocation, real
  OpenSSH prompts and diagnostics, native proof of the implemented
  graceful/forced child-tree cleanup, before/after manual `ssh`, controlled
  accessibility, and native Windows/macOS/Linux/WSL resource evidence.
### F5.2 explicit routes and host trust

Status: **Fully done locally; nonactivated.**

- [x] **Fully done locally:** separate typed host/user/port fields and bounded
  canonical config-defined jump chains produce exact direct or one-`-J` routed
  argv; free-form options, ProxyCommand, remote commands, shell text, and
  tunnels fail closed.
- [x] **Fully done locally:** first-use/known/changed review shows the algorithm
  and full OpenSSH SHA-256 fingerprint without truncation or `known_hosts`
  mutation; changed keys cannot bind.
- [x] **Fully done locally:** `C` copies the exact reviewed user-owned command
  with no newline, Enter, or execution.
- [x] **Fully done locally as a non-executing contract:** exact bounded
  `ssh-add -l -E sha256` public agent/certificate/hardware status parsing reads
  no private key, token, or agent protocol data. Actual status execution remains
  protected/nonactivated.
- [x] **Fully done locally:** agent forwarding stays off, route/evidence changes
  invalidate review, and production routes remain review-only.

### F5.3 typed tunnels

- [x] **Fully done locally; nonactivated:** validated local, remote, and dynamic
  descriptors compile exact endpoints into a separate configuration-free
  `-F none` OpenSSH grammar. Existing no-tunnel argv remains unchanged and
  config-dependent aliases/jumps with tunnels fail closed.
- [x] **Fully done locally; nonactivated:** listeners default to
  `127.0.0.1`; remote, non-loopback, or production forwarding requires a fresh
  strong Allow-once decision bound to the exact endpoints. Agent, X11, command,
  multiplex, TUN, and hidden configuration forwarding remain disabled.
- [x] **Fully done locally; nonactivated:** a bounded session/generation-scoped
  lifecycle projects planned, starting, ready, collision, failed, cancelled,
  and closed state plus OpenSSH ownership into compact colored, icon-and-text,
  keyboard, pointer, and accessibility review. Stale owner events and terminal
  reversals fail closed; route close terminalizes every nonterminal tunnel.

### F5.4 native and release evidence

- [x] **Fully done locally — deterministic and controlled validation:** exact fake preparation/argv/parser, hostile
  endpoints, independent collision domains, stale scope, terminal lifecycle,
  cleanup, Hub projection, decision, and 1/10/50 pure-model cases pass. Active
  schema 6 and a bounded duplicate-key/size/redaction-aware evidence validator
  freeze 23 ordered scenarios. Real evidence must additionally match the
  executing native OS/architecture, exact clean commit, fixed OpenSSH client
  and server versions, and freshly hashed application binary, package, and
  client. The manual-only protected runner-group workflow uploads only a
  path-free summary. A real hermetic OpenSSH server run remains external and
  is not represented by the synthetic repository fixture.
- [ ] **External prerequisite:** pass real system OpenSSH cases on Windows,
  macOS, and Linux for host keys, agents, encrypted keys, certificates, jumps,
  tunnels, cancellation, offline, hostile output, exit status, and cleanup. WSL
  is separately denied by this release contract until it receives its own
  approved evidence path.
- [ ] **Partially done:** deterministic 1/10/50 scope isolation and cleanup
  invariants pass; bounded native CPU, memory, handle/descriptor, socket, task,
  route, cache, log, and storage measurements remain external.
- [x] **Fully done locally — fail-closed acquisition path:** activation remains
  compile-time false, the evidence probe is explicit/read-only, real baseline
  zero sentinels and linked or changed files fail closed, and the repository
  fixture cannot satisfy a release. The F5 workflow uses read-only permissions,
  a protected environment, exact-commit checkout without persisted credentials,
  and a restricted ephemeral runner group.
- [ ] **External prerequisite:** collect controlled before/after enable, disable,
  uninstall, manual `ssh`, and generic-terminal baselines on every native host.

Exit remains unavailable: the nonactivated tunnel source boundary is complete,
but reviewed production SSH requires protected activation plus validated real
Windows/macOS/Linux evidence without embedded SSH or secret custody.

## F6 - implement connection automation and remote workspaces

Status: **Partially done overall; review/edit source and product surfaces are
fully done locally.** Execution remains gated by protected D3/M5 native evidence.

- [x] **Fully done locally:** schema-2 profile/recipe/workspace persistence,
  entity revisioning, explicit migration/recovery/import/export previews, CAS,
  fresh transfer IDs, exact cross-record fingerprints, and approval invalidation.
- [x] **Fully done locally:** deterministic pure recipe review preserves Resolve,
  Preflight, Authenticate, BeforeConnect, Connect, RemoteInitialize, Verify,
  Ready, BeforeDisconnect, and Cleanup ordering.
- [x] **Fully done locally:** typed local/session actions and a narrow POSIX-sh or
  PowerShell remote directory/public environment/user-switch/verification
  envelope; privileged switches are revalidated and confirmed every connection.
- [x] **Fully done locally:** deadlines, cancellation, safe failure, bounded
  eligibility-checked retry/backoff/jitter, generation invalidation, shutdown,
  and reviewed no-hooks recovery.
- [x] **Fully done locally:** bounded declarative multi-window/pane layout and
  connection intent, isolated clone/rebind, and review-only restore with no live
  PTY/tunnel/credential state, automatic reconnect, or interrupted-action resume.
- [x] **Fully done locally:** bounded reviewed broadcast with exact transient
  command/targets, unmistakable semantic armed/disarmed state, production
  confirmation, no implicit Enter, per-target results, isolation, cancellation,
  and digest-only audit.
- [x] **Fully done locally:** custom scripts, hidden key injection, terminal-cell
  readiness inference, and automatic persistent mutation remain absent.
- [x] **Fully done locally:** hostile/limit/cycle/fingerprint, clone/rebind,
  multi-pane/window, retry/reconnect/shutdown, focus/accessibility, recovery,
  redaction, repeated-generation, fuzz, mutation, and benchmark evidence passes.
- [x] **Fully done locally:** accepted ADR 0023, a bounded preview-first public
  workspace CLI, immutable background-worker library publication, and the
  Connection Hub workspace catalog/restore review are wired with CAS writes,
  responsive pointer/keyboard interaction, semantic accessibility projections,
  stale-review invalidation, and no process, PTY, network, or Enter authority.
- [ ] **Partially done / external prerequisite:** protected ADR 0012/D3/M5
  activation and attestation, managed execution adapters, real native OpenSSH/
  PTY/process/resource cleanup, controlled accessibility/visual evidence, and
  hosted Windows/macOS/Linux release evidence remain.

Exit remains unavailable for execution, but the shipped product can safely
manage and review reusable profiles, recipes, and remote workspace intent.
Advanced custom code and every M6 execution path remain disabled.

## F7 - implement D6.0 provider-neutral auth and capsule orchestration

Status: **Fully done locally** at the authority-free D6.0 framework boundary.
D6.1-D6.4 and D6.5 Teleport source adapters are complete and nonactivated;
product controls, real official-CLI/native evidence, and OpenBao remain external
or not done.

- [x] **Fully done locally:** strict bounded public provider identity, auth
  observation, immutable capsule, freshness, provenance, risk, operation,
  isolation/browser policy, recovery, receipt, and redacted audit schemas.
- [x] **Fully done locally:** later official-CLI operations require exact
  visible review plus current process/network `AllowOnce` decisions. M7 owns no
  adapter, login control, process, network, browser callback, or credential.
- [x] **Fully done locally:** browser/device/system-broker/MFA and token/cache
  custody remains with official tools; M7 exposes bounded public state only.
- [x] **Fully done locally:** each fresh session pins provider configuration and
  risk without mutating another pane's account, subscription, project,
  configuration, kube context, namespace, or other global provider state.
- [x] **Fully done locally:** explicit refresh/authentication, last-known-good,
  expiry/offline/denied, cancellation, revocation, disable/uninstall, rebind,
  shutdown, generation rejection, and cross-provider isolation are bounded.
- [x] **Fully done locally:** fake exact-argv/capability contracts, strict
  ingress, redaction canaries, 16×64 lifecycle coverage, fuzz/mutation, and the
  64-capsule benchmark prove the local boundary.

Exit is met locally for two simultaneous provider-neutral contexts without
cross-session state or authority. Provider-specific execution and native login
evidence begin only in independently gated D6.1-D6.5.

## F8 - implement D6.1 AWS slice

Status: **Partially done overall; source and cached product review are fully done
locally, while execution remains nonactivated.**

- [x] **Fully done locally:** independent disabled extension; bounded exact-byte
  named-profile parser; public region/account/role/source fields only; duplicate,
  hostile, invalid, and oversized input denial; no credential/cache retention.
- [x] **Fully done locally:** exact M7 IAM Identity Center PKCE/device login and
  regional STS identity operations, strict public STS decoder, capsule-pinned
  profile/region/account/role/provenance/freshness/risk, and truthful failures.
- [x] **Fully done locally:** exact nonactivated SSM plan names AWS CLI, Session
  Manager plugin, target, risk, PTY and process-tree cleanup; EKS is `--dry-run`
  only for M11 private transient ingestion and never names user kubeconfig.
- [x] **Fully done locally:** ten unit/security/argv/redaction/version/isolation/
  disable tests, locked focused test, warning-denied Clippy, and formatting pass
  on Windows x86_64.
- [x] **Fully done locally:** publish validated cached AWS context in the
  six-provider Hub review with strict capsule replacement and no PTY/execution;
  allocate, validate, revalidate, revoke, and clean private EKS output through M11.
- [ ] **External/blocked:** D3 runner activation and executable attestation,
  real IAM Identity Center/STS/SSM/EKS fixtures,
  three native OSes, forced cleanup/resources, accessibility, packaging, and
  release evidence.

Exit is complete for the local AWS source, cached review, and M11 private lifecycle only. No AWS process,
network, browser/device flow, credential cache, SSM session, or EKS cluster ran.

## F9 - implement D6.2 Azure slice

Status: **Partially done overall; source and cached product review are fully done
locally, while execution remains nonactivated.**

- [x] Parse bounded exact granted public Azure account JSON with byte, account,
  node, depth, field, duplicate, hostile-text, and secret-key limits.
- [x] Preserve Azure CLI-owned WAM/browser/device/MFA behavior through exact tenant-bound
  reviewed login operations; expose public user/workload identity kind without
  accepting passwords, client secrets, tokens, certificates, or caches.
- [x] Pin tenant, subscription, cloud, public identity/state, freshness,
  provenance, risk, executable, argv, endpoint, browser policy, timeout, and
  session/capsule revision. Use `--subscription`; never `az account set`.
- [x] Add immutable AAD-only Bastion and opaque-private-file AKS intents with
  execution disabled, explicit PTY/child-tree cleanup, and no user kubeconfig.
- [x] Pass eight focused tests, app registration, warning-denied Clippy,
  formatting, architecture/identity, and repository/phase policy checks on
  Windows x86_64; the 128-account Criterion target measured 473.69–478.86 µs.
- [x] Publish cached Azure context through the strict six-provider review and
  complete private AKS allocation/validation/revalidation/revoke/cleanup in M11.
- [ ] Activate only through the separately protected D3 runner with a fresh
  one-time execution review.
- [ ] Run controlled Azure CLI 2.61+ WAM/browser/device/MFA/conditional-access,
  Bastion 2.32+, AKS, native cleanup/resources/accessibility, and release proof.

Exit is complete for the local Azure source, cached review, and M11 private lifecycle only. No Azure
process, network, authentication, cache, Bastion, AKS, PTY, or filesystem ran.

## F10 - implement D6.3 Google Cloud slice

Status: **Partially done overall; source and cached product review are fully done
locally, while execution remains nonactivated.**

- [x] Parse one bounded exact granted named configuration and retain only public
  account/project/region/zone; reject external credential/token material.
- [x] Use exact `--configuration` for user browser/remote login and public
  project/IAM observation; never globally activate/set config or update ADC.
- [x] Keep Workforce/Workload config files opaque and nonexecuting; pin complete
  capsule/session/config/project/region/zone/provenance/freshness/risk scope.
- [x] Add scope-bound IAP/OS Login planning with gcloud-owned SSH-key behavior
  and M11-only opaque private-`KUBECONFIG` GKE intent.
- [x] Pass eight focused tests, app registration, warning-denied all-target
  Clippy, formatting, and a near-limit 444.00–460.66 µs Criterion target on
  Windows x86_64.
- [x] Publish cached Google Cloud context through the strict six-provider review
  and complete private GKE allocation/validation/revalidation/revoke/cleanup in M11.
- [ ] Activate through D3 only and run real user/2FA/federation/IAM/IAP/OS Login/
  GKE/native cleanup/resource/accessibility/release evidence.

Exit is complete for the local Google Cloud source, cached review, and M11 private lifecycle only. No gcloud
process, network, authentication, credential DB, IAP, GKE, PTY, or file ran.

## F11 - implement D6.4 Kubernetes and OpenShift slice

Status: **Partially done overall; source, app-owned private transient lifecycle,
and cached product review are fully done locally, while execution remains
nonactivated.**

- [x] **Fully done locally:** exact absolute/private-transient grants, stable
  bounded reads, source revisions, no links/reparse points, and no changed source.
- [x] **Fully done locally:** typed 1 MiB YAML/JSON parsing, explicit structural
  budgets, deterministic source order, public metadata only, and fail-closed
  duplicate/merge collisions and credential paths.
- [x] **Fully done locally:** exec plugins default to `DenyAll`; exact future
  digest/argv/environment-name/interactivity/session/deadline/output/cancellation
  review stays nonactivated and secret-bearing args/values are not retained.
- [x] **Fully done locally:** capsules pin exact source-set/context/cluster/user/
  namespace/project/provider/freshness/expiry/provenance/risk fields.
- [x] **Fully done locally:** exact isolated `kubectl auth whoami`, context view,
  exec, OpenShift private-output web login, project view, and rsh plans never
  mutate shared current context/project.
- [x] **Fully done locally:** 13 focused tests, two app registration tests,
  warning-denied Clippy, dependency policy, and a 50-sample 900 KiB benchmark at
  1.8280–1.8788 ms pass on Windows x86_64.
- [x] **Fully done locally:** app-owned no-follow private output is limited to 16
  active 1 MiB files, validated/revalidated before exact opaque publication,
  tamper-safe, and cleaned on expiry/revoke/provider/session/shutdown/drop; the
  cached Hub review has responsive keyboard/pointer/accessibility/no-PTY evidence.
- [ ] **Partially done/external:** D3 activation, real clients/clusters/exec
  plugins, Unix native no-follow, forced cleanup/resources, controlled native
  UX/accessibility, packaging, signing, and release fixtures remain.

Exit is met for local source, private lifecycle, and cached review only. No client,
cluster, network, credential, browser, PTY, or user kubeconfig ran.
## F12 - implement D6.5 organization identity slices

Status: **Partially done overall:** Teleport source and cached product review are
fully done locally and nonactivated; OpenBao is **Not done** pending ADR 0024.

- [x] **Fully done locally:** implement Teleport through exact reviewed tsh
  version/login/status/ssh/logout flows with Teleport-owned agent/cache/browser/
  MFA/certificate authority and explicit agent/environment isolation.
- [ ] **External prerequisite/not done:** add OpenBao public-key SSH certificate
  signing only after ADR 0024 accepts its token-helper and certificate-file
  boundary.
- [x] **Fully done for Teleport:** retain opaque references and bounded public
  proxy/cluster/user/role/login/Kubernetes/expiry/provenance metadata only.
- [x] **Fully done for Teleport source and cached product review:** independent
  disabled registration, exact grants, revoke/logout, strict capsule replacement,
  responsive Hub keyboard/pointer/accessibility/no-PTY evidence, OpenBao rejection,
  docs, hostile/redaction tests, and benchmarks.
- [ ] **External:** real `tsh`, proxy, browser/MFA, cache/certificate/agent, PTY,
  cleanup/resource, accessibility, packaging, signing, and release fixtures.

s’s disabled source package, not product activation
or the combined organization-adapter release.

## F13 - implement CP4 provider-aware Quick Actions

Status: **Fully done locally at the product-integrated nonactivating boundary; partially done overall.**

- [x] **Fully done locally:** bounded cached public capsules project SSH target,
  provider/account/project/subscription, cluster/context/namespace, region/zone,
  infrastructure, freshness/provenance, risk, and state.
- [x] **Fully done locally:** compact search/review labels and accessible names
  show exact target, current/refreshing/stale/missing/expired/offline/error/
  changed state, provenance-backed context, and production risk.
- [x] **Fully done locally:** route/session/capsule/generation keys cancel stale
  results and isolate panes, provider snapshots, and workspace layers; route
  cleanup and explicit clear revoke candidates.
- [x] **Fully done locally:** the retained validated Connection Hub product is
  synchronized on Action Center open only to the exact selected
  route/session/revision; unchanged publication is idempotent and mismatch,
  revocation, or absence clears candidates without provider or keystroke-time
  work.
- [x] **Fully done locally:** current observations remain insert-without-Enter;
  private-environment/exact operations are broker-required, so CP4 cannot
  execute or fall back to ambient provider state.
- [x] **Fully done locally:** SSH/AWS/Azure/GCP/Kubernetes/OpenShift/Teleport
  contributions, fake/model/application tests, final revalidation, redacted
  audit, production confirmation, fuzz, benchmark, and no-keystroke-provider-
  work policy are present. Unsupported OpenBao prevents partial publication.
- [ ] **Partially done — external activation/native evidence:** implement an
  approved provider refresh/capsule producer, activate exact execution through F4,
  implement OpenBao after ADR 0024, and run real provider/account/cluster,
  Linux/macOS, screen-reader/resource, packaging/signing/release fixtures.

Exit is met locally at the nonactivating authority boundary: provider-aware
actions never widen provider or session authority. Product/release exit remains
partial until the final item passes.

## P1 - execute CP5.0 native autocomplete research

Status: Fully done at the non-activating research boundary. CP1 remains the
complete solution and P2 is deferred.

- [x] **Fully done — native shell/API baseline.** PowerShell/PSReadLine,
  Bash/Readline, Zsh/ZLE/compsys, Fish, CMD, and WSL ownership, completion/
  prediction behavior, startup, typing/memory delta, cancellation, resize,
  accessibility, disable behavior, local evidence, and honest external gates
  are recorded in
  [the CP5.0 research report](research/CP5-AUTOCOMPLETE-RESEARCH.md).
- [x] **Fully done — editor-owned prototype.** The bounded pure model validates
  ephemeral buffer/cursor/span/generation state and returns a non-executing
  replacement request without a transport, profile change, or keybinding
  mutation. Shells without a safe common bridge remain native-only.
- [x] **Fully done — matcher and dependency comparison.** The locked standalone
  benchmark covers 32/128/512 candidates, Unicode/combining/common-prefix
  corpora, stable generations, compile/binary/startup data, MPL-2.0, features,
  maintenance, advisories, and same-host performance. Low-end replication
  remains external; nucleo-matcher is rejected for runtime adoption.
- [x] **Fully done — serious alternatives.** Reedline remains a reference and
  Carapace remains an explicit external adapter only; no line editor or
  per-keystroke external process is embedded.
- [x] **Fully done — published decision.** The shell/version matrix, zero
  privacy/authority delta, benchmark evidence, CP1 fallback proof, external
  gates, and rollback are fixed by
  <code>cp50-research-contract-v1.json</code> and its mutation tests.

Exit: satisfied by the reviewed retain-CP1/defer-P2 decision. No runtime
dependency, editor transport, product surface, profile mutation, or keybinding
mutation was added.

## P2 - execute CP5.1 editor bridge

Status: Fully done at the source boundary and partially done for release. ADR
0025 and the schema-1 six-threat contract are accepted. Protocol, endpoint
sources, joined service, and local tests exist while activation remains false.

- [x] Freeze transport, peer/capability/replay, privacy, span, source/ranking,
  pane UI, shell/fallback, lifecycle, limit, verification, and rollback without
  granting activation authority.
- [x] Implement restrictive private Windows named-pipe and mode-0600 filesystem
  Unix-socket source adapters; forbid TCP, OSC, terminal output, and forwarding.
- [x] Bind schema, peer, app/window/tab/pane/session, shell/editor, prompt, buffer
  generation, cursor, replacement span, capability, source and cancellation.
- [x] Bound frames, buffer/candidate counts and bytes, queue depth, cache,
  deadlines, endpoint lifetime, and prefix allocation.
- [x] Keep buffers/candidates memory-only and out of logs, telemetry, crash,
  clipboard, diagnostics, persistence, extensions, and AI.
- [x] **Fully done locally — inert helper and response path:** the package binary
  target, bootstrap/transport/session/endpoint runner, authenticated application
  reply, strict shell envelope, and native generation/span/current-buffer
  replacement without Enter are implemented and tested.
- [ ] Produce the signed/attested helper artifact and reviewed restricted-handle
  launcher; no unsigned local build is release evidence.
- [ ] Complete native Linux/macOS/WSL restart/sleep/churn evidence; source fuzz,
  replay/cross-route, Windows ACL/peer, Unix mode, cleanup and fallback tests exist.

Exit: protocol and ownership gates pass before any Automexia popup is enabled.

## P3 - execute CP5.2-CP5.3 sources and ranking

Status: Fully done at the source/local model boundary and partially done for
release evidence.

- [x] Broker the exact six bounded native, opt-in history, cwd/executable,
  opt-in frequency-ID, cached public CP1/CP4, and typed-action sources.
- [x] Exclude history files, raw command persistence, terminal/remote output,
  clipboard, telemetry, AI, network, authentication, and per-key processes.
- [x] Implement deterministic source precedence, prefix/token/fuzzy ranking,
  provenance/freshness/risk explanations, and stable tie-breaking.
- [x] Bound queue/cache/deadlines and reject stale generations under route,
  pane, worker and shutdown lifecycle changes.
- [ ] Complete native IME/RTL/quotes/spaces/multiline/selection shell fixtures and
  named-hardware latency/allocation/resource distributions; source Unicode,
  grapheme, exact-span, hostile-label, determinism, fuzz and benchmarks exist.

Exit: local-only results are deterministic, explainable, bounded, and safe.

## P4 - execute CP5.4-CP5.6 UI, shell activation, and release

Status: Partially done overall. CP5.4 UI/publication source and the CP5.5 inert
helper plus bidirectional shell adapters are implemented; launcher/signing/live
composition, CP5.6 activation, and release evidence remain open.

- [x] Add a renderer-neutral pane-owned listbox model/controller/renderer with
  cursor, IME, footer, tab, sibling, selection and modal exclusion inputs and no
  PTY path; runtime publication remains disabled.
- [x] Show icon-plus-text value, type, source, freshness, risk and selected state,
  plus compact fallback without color-only meaning.
- [x] Add source-level keyboard/focus lifecycle, reduced-motion, high-contrast,
  tiny-to-8K, 100-300% scale, split-pane and exclusion geometry tests.
- [x] **Fully done locally — response/replacement source:** PowerShell 7, Bash 5,
  Zsh 5.8, and Fish 3.6 adapters use the persistent helper envelope, bounded
  response parsing, current-editor revalidation, and one native replacement.
- [ ] Activate only after the reviewed launcher, signed artifact, live screen
  composition, and native-proven PowerShell/Bash/Zsh/Fish/WSL pairs pass; retain
  truthful Windows PowerShell 5.1, CMD, remote and container fallbacks.
- [ ] Prove native profile, keybinding, predictor, completer, history, alias,
  function, abbreviation and view preservation byte-for-byte.
- [ ] Add public preview/source/privacy/memory settings and staged opt-in; internal
  broker reset/kill/disable/uninstall/LKG controls exist but are not exposed.
- [ ] Pass native three-OS shell/PTY/GPU/accessibility/package/rollback, fuzz/leak/
  resize/multi-pane/sleep, SBOM, named-hardware and 30-day baseline gates.

Exit: maintainers prove a measured UX improvement; otherwise CP1 remains active.

## PO0 - accept the situation-aware production operations contract

Status: Partially done at the detailed proposal/checker boundary. The strict
proposed machine contract, digest, semantic checker, mutation suite and source
nonactivation scan exist; ADR/digest acceptance, owner/dependency/capability
approval, prototypes and first failing real-path tests remain open.

User experience target: freeze nonactive, testable prototypes of the six
reused surfaces and the shortest read-only, reviewed-insertion,
managed-operation, and managed-diagnostic-session journeys. The prototype must
cover comfortable/compact/minimal panes,
keyboard-only and pointer use, accessible names/focus, stale/offline/unknown,
refusal, cancellation, failure, recovery, and plain wording for fact,
correlation, inference, policy, and uncertainty. No prototype control may call a
provider, change the editor, or execute.

- [x] Add the canonical product specification, requested Kubernetes rollout
  example, current evidence ledger, 2026 primary-source baseline, owner map,
  threat model, provisional resource ceilings, delivery phases, and honest
  refusal behavior.
- [x] Add proposed ADR 0034 and the separate future testing/evidence contract.
- [x] Add the canonical surface, copy, responsive, keyboard, accessibility,
  journey, implementation-owner, and UX definition-of-done blueprint.
- [x] Synchronize the main/concise/focused roadmaps, phase audit, architecture,
  feature catalog, testing, assurance, navigation, and change fragment.
- [ ] Accept or supersede ADR 0034 through protected owner review.
- [ ] Freeze exact types, capabilities, source dependencies, numeric ceilings,
  risk/refusal/ranking semantics, privacy, lifecycle, and nonactivation in a
  versioned machine contract.
- [ ] Add a fail-closed checker and mutations for deletion, weakening, stale
  paths, fabricated evidence, hidden activation, broadened authority, and
  relaxed resource limits.
- [ ] Add the first failing real-path PO1 tests before production source.

Exit: the accepted machine contract proves documentation cannot activate a
provider, watcher, investigation view, live-log controller, completion source,
managed diagnostic session, UI, model, journal, or execution path.

## PO1 - implement the production passport and context lock

Status: Not done.

User experience target: extend the context line already shown above the command,
not a separate dashboard. It must answer where, who, production classification,
credential expiry, GitOps/incident ownership, and freshness at a glance; retain
production, target, identity, and freshness as space shrinks; never capture
typing focus; and present an exact changed-field diff with clear `Adopt` and
`Keep locked` choices before a locked route can follow a new context.

- [ ] Reuse D2/D6 public route-scoped context; define provider/account/region/
  cluster/namespace/identity/role-expiry/GitOps/incident/freshness fields.
- [ ] Add pane-local lock, exact before/after context diff, explicit adopt/reject,
  expiry, logout/revoke, route-close, restore, disable and uninstall behavior.
- [ ] Block production mutation review on unknown, stale, changed or expired
  context while leaving read-only diagnosis and CP1 available.
- [ ] Add renderer-neutral and native tiny-to-8K, 100-300%, theme, keyboard,
  IME, focus, screen-reader, multi-pane and clone/isolation evidence.
- [ ] Measure zero hot-path provider work, memory, tasks, handles and cleanup.

Exit: a production operator can always see and verify the exact route identity;
one pane cannot silently change or inherit another pane's context.

## PO2 - implement bounded investigation evidence

Status: Not done.

User experience target: show evidence only when the operator requests a
situation or detail. One `Investigate` entry exposes only relevant `Explain
state`, `What changed?`, `Compare with healthy`, `Diagnose connection`, and
`Show user impact` choices. Each result states fact, source authority, age,
scope, coverage, contradiction, explicit unknowns and network vantage where
relevant. Change rows say `correlated with` unless a separate authority proves
cause; comparison explains its cohort and exclusions. Refresh and provenance are
explicit; last-known-good data is visibly stale; a failed adapter explains what
is unavailable without blocking CP1/CP5. No copied log viewer, metric dashboard,
raw-YAML diff, active probe or giant dependency canvas is added.

- [ ] Define strict knowledge/evidence quality, observation, change, field-
  ownership/drift, resource-explanation, cohort/comparison, network-path,
  SLO-summary and resource-node/edge records with source, UID, route, revision,
  observed time, expiry, coverage, redaction class and digest.
- [ ] Implement independently reviewed Kubernetes, cloud, GitOps, observability,
  identity, incident and service-catalog adapters only as needed per slice.
- [ ] Add bounded typed recent-change markers and a deterministic correlation
  window; preserve correlation, inference, and independently proven cause as
  different model, copy, ranking-input, test, and receipt states.
- [ ] Resolve Kubernetes ownership through exact ownerReferences kind/name/UID;
  add service/dependency mappings only from identified reviewed sources.
- [ ] Add kind-specific resource/scheduling explainers; normalize semantic
  cohort/revision differences without reading Secret values or raw manifests;
  and keep observed fact, assessment, recommendation and action separate.
- [ ] Add read-only DNS/Service/EndpointSlice/backend/policy path mapping with
  exact source/destination/vantage and unknown layers; add approved SLO summaries
  with window/request-volume/low-traffic limitations. Active probes remain PO6.
- [ ] Use bounded namespace-scoped list/watch or polling with resource-version,
  relist, throttling, timeout, cancellation, parked-idle and shutdown behavior.
- [ ] Enforce candidate/frame/query/concurrency/object/memory/TTL ceilings; keep
  raw logs, metric series, credentials and evidence disk caches out.
- [ ] Pass hostile/secret canary, graph-cycle/fan-out, stale/replaced UID,
  offline/reconnect, provider-native, resource/leak and disable/uninstall gates.

Exit: adapters publish bounded public evidence independently; no adapter failure
or large environment can block typing, rendering, PTY, startup or CP1.

## PO3 - implement situation-aware diagnostic completion

Status: Not done.

User experience target: after an explicit completion request, reuse the current
CP5 list and show no more than five current-situation rows before normal results.
Each two-line row exposes command, exact target, one reason, freshness, and
`Read-only`, `Review required`, or `Refused`. Arrows move selection, detail is
deliberate, Escape closes, and Tab/Right Arrow replaces only the authenticated
editor span without Enter. Refresh preserves the selected stable candidate or
moves predictably; it never steals focus or reorders an item under confirmation.
CP1 remains immediately usable throughout.

- [ ] Build deterministic hard gates and lexicographic ranking over typed actions;
  record supporting, contradicting, missing and freshness evidence.
- [ ] Implement the Kubernetes matrix for healthy, crash loop, missing config,
  bad image, unschedulable, resource/probe, bad/active/stalled rollout,
  StatefulSet/DaemonSet/custom owner, permission, GitOps and stale-context cases.
- [ ] Show `status`, `history`, `undo`, `restart`, or diagnosis only when
  the exact evidence and controller contract support it.
- [ ] Bind candidate route/passport/UID/evidence/policy/editor generations and
  insert through CP5 without Enter; revalidate immediately before insertion.
- [ ] Preserve CP1 fallback, native editor quoting/selection/IME/history,
  deterministic ties, refusal states, accessibility and no per-key I/O.
- [ ] Pass independent reference-ranker, metamorphic, fuzz, exact editor-byte,
  native shell, pixel, screen-reader, p50/p95/p99 and cleanup evidence.

Exit: Automexia explains relevant safe candidates and uncertainty without
claiming certainty, executing, retargeting, or hiding native shell completion.

## PO4 - implement production impact and authority preflight

Status: Not done.

User experience target: one cancel-first review reads environment, exact target,
observed reason, expected impact, current authority, GitOps/policy/change window,
verification, and recovery in that order. Unknown is visible and blocking where
required. Before PO6, the only positive result is `Insert reviewed command` and
the shell owns any later Enter; Automexia must not imply that it will enforce
policy or monitor that manual execution. After PO6, `Execute reviewed action` is
a separate managed route with final revalidation. JIT access, ticketing, or
approval is an explicit separate request, never a hidden side effect.

- [ ] Show exact executable/argv, environment, target kind/name/UID/count,
  dependencies, current health, rollout strategy and blast radius.
- [ ] Recheck current authorization, credential/JIT expiry, organization policy,
  incident/ticket/approval, change window, GitOps owner/self-heal/sync window,
  verification, timeout, stop condition, rollback and no-rollback disclosure.
- [ ] Label a real provider diff/plan separately from an impact estimate; never
  present unsupported `rollout restart` dry-run behavior.
- [ ] Bind confirmation to the exact candidate digest, context, targets,
  evidence/policy revisions, expiry and one use; deny broad/destructive actions
  in the initial contract.
- [ ] Prove preflight has no provider mutation, credential custody, role
  self-approval, global CLI/config mutation, policy bypass or implicit execution.

Exit: every PO-provided production mutation is either refused, exposed as an
honestly advisory reviewed insertion, or presented with enough current, exact and
independent evidence for a deliberate managed-action decision.

## PO5 - implement Incident Mode, live evidence navigation and the bounded journal

Status: Not done.

User experience target: a deliberate entry opens one workspace with objective,
locked environment, facts, hypotheses, contradictions, missing evidence, a short
timeline and one next safe action. Trusted links return to their source;
approximate terminal anchors say so. Optional source-separated logs are pausable,
bounded, visibly memory-only and expose gaps. Normal completion does not require
Incident Mode. Journaling reads `Session only` by default; opt-in persistence
discloses location and retention. Handoff/export previews included/redacted facts,
hypotheses, outcomes, unresolved questions, links and destination. Exit or route
close restores the prior surface, focus, and normal terminal behavior.

- [ ] Pin incident identity, objective, severity, environment and operator role;
  add hypothesis/negative-evidence state, bounded summary timeline, trusted/
  approximate anchors and situation-filtered candidates.
- [ ] Add a separate bounded per-source live-log controller with source-local
  order, arrival/provider clocks, filters, pause/resume, backpressure, visible
  gaps/drops, memory-only cleanup and a content-free DN marker handoff.
- [ ] Keep the default journal session-only with content-minimized public
  receipts; make protected persistence explicit, bounded and opt-in.
- [ ] Exclude logs, metrics, terminal content, output, credentials, environment
  values, raw provider responses and arbitrary secret-bearing arguments.
- [ ] Add reviewed redacted export/handoff, retention, crash recovery,
  read-only/disk-full/corrupt/interrupted storage, logout, exit, disable and
  exact uninstall behavior.
- [ ] Pass multi-operator/route/workspace isolation, long-session resources,
  accessibility, privacy approval and storage-tree/digest oracles.

Exit: Incident Mode improves shared context without widening authority or
turning Automexia into a telemetry store or external audit authority.

## PO6 - implement reviewed execute, observe, stabilize, verify and recover

Status: Not done and blocked on independent D3/provider activation.

User experience target: only after PO6 activation may production preflight offer
`Execute reviewed action`. A nonmodal monitor shows one exact target in `Preparing`,
`Running`, `Observing`, `Stabilizing`, `Verifying`, `Succeeded`, `Failed`,
`Cancelled`, `Uncertain`, or `Recovery available`, with elapsed time,
before-state, last verified observation, safe cancel/stop, cleanup, receipt, and
one deliberate recovery proposal. The same preflight/monitor pattern owns a
Kubernetes port forward, controlled probe or safe workload-debug session and
shows exact endpoint/vantage/image/profile, authority, lifetime and cleanup. The
terminal remains usable.
Process exit alone never becomes success, loss of observation becomes
`Uncertain`, and no retry, rollback, recovery, or second mutation starts without
a new review. Managed execution never silently falls back to shell insertion.

- [ ] Reuse the one app-owned exact-argument runner and provider capability;
  never create a PO process/network path.
- [ ] Implement the bounded draft/preflight/approval/executing/observing/
  stabilizing/verified/failed/uncertain/cancel/recovery state machine with
  fake-clock/model tests before real execution.
- [ ] Revalidate context, UID, permission, policy, evidence and executable before
  launch; monitor declared signals with fixed deadline and stop conditions.
- [ ] Return control after one mutation. Never recurse, retry a mutation
  automatically, or launch a suggested recovery without a new exact review.
- [ ] Add independent typed port-forward/probe/debug slices with exact target,
  loopback endpoint or vantage, immutable image/profile, traffic/time limits,
  one-use authorization/admission, descendants/listeners/temporary resources,
  context/revoke behavior and observable cleanup. Node debug and public
  production listeners remain unavailable initially.
- [ ] Compare real provider state, process trees, network attempts, receipts,
  handles/files/children and final cleanup on controlled disposable systems.
- [ ] Pass rollback/no-rollback, adapter loss, context change, shutdown, native,
  package, security, accessibility, resource and release evidence.

Exit: one explicitly approved action or diagnostic session has an observable
bounded outcome, exact cleanup state, and safe recovery offer where applicable;
unavailable external evidence keeps the phase partial.

## PO7 - implement declarative organization packs and guided workflows

Status: Not done and blocked on the ecosystem capability/distribution decision.

User experience target: installation shows publisher, signature, version,
supported tools, data needs, conflicts, and zero execution grants in one review.
A guided workflow presents one step, the evidence that selected it, expected
effect, verification, and stop condition, then returns control. Continuing is a
new deliberate choice. Disable/uninstall is immediate, and conflict or unsupported
state explains which deterministic built-in behavior remains available.

- [ ] Define signed/versioned schemas for service owners/dependencies/criticality,
  evidence predicates, typed registered actions, risk/approval/change windows,
  success/stop/timeout/recovery predicates, links and compatibility.
- [ ] Keep packs capability-free declarative data with no arbitrary script,
  command string, credential, direct network/process, executable callback or
  execution grant.
- [ ] Implement verified disabled install, provenance/revocation, conflict
  review, atomic update/rollback, disable and exact uninstall.
- [ ] Keep cross-tool workflows guided and one-step-at-a-time; show each owning
  adapter and native command/API operation.
- [ ] Pass malicious bundle/schema/graph/policy conflict, compromised/revoked
  key, rollback/downgrade, native package, resource and removal evidence.

Exit: reviewed organization knowledge improves ranking without becoming an
unreviewed code or authority channel.

## PO8 - expand adapters, compare environments and evaluate optional local tie-breaking

Status: Not done.

User experience target: every provider slice uses the same passport, situation
row, evidence detail, preflight, monitor, and Incident workspace language.
Disconnected or unsupported sources state what is missing and preserve safe
shell completion. If a small local tie-breaker is adopted, settings explain that
it only reorders already-valid candidates; each candidate identifies the
deterministic evidence behind it; disabling the model immediately restores the
stable deterministic order. `Compare environments` is read-only and first shows
reviewed service equivalence, selected environments, independent access/
freshness, field/traffic/window coverage and fan-out. It never creates a cross-
environment mutation. No user needs an LLM, paid API, or network model.

- [ ] Deliver each new provider/tool adapter as an independent versioned slice
  with license/provenance/advisory, capability, quota, privacy, lifecycle,
  native provider, accessibility, resource and uninstall evidence.
- [ ] Add bounded read-only region/cluster comparison only for explicitly mapped
  services; normalize declared fields and telemetry windows without merging
  credentials, policy, approval, state or mutation authority.
- [ ] Evaluate a small local model only as an optional tie-breaker over
  already-valid candidates using bounded redacted features off hot paths.
- [ ] Prove the model cannot create text, commands, targets, grants, approvals or
  execution; deterministic ordering remains the removable fallback.
- [ ] Measure binary/startup/CPU/memory/storage/energy cost on low-end hardware
  and reject adoption without a demonstrated workflow benefit.
- [ ] Complete three-platform native UX/accessibility/packages, controlled
  provider fixtures, rollback, soak and release evidence for claimed slices.

Exit: optional breadth never weakens deterministic policy, core weight, offline
operation, provider isolation, or the user's control.

## Common phase execution checklist

Every future phase request follows [AGENTS.md](../AGENTS.md) and must complete:

- [ ] Restate scope, user outcome, acceptance criteria, non-goals, authority,
  prerequisites, and external evidence.
- [ ] Inspect worktree, owners, callers, recent history, contracts, tests,
  benchmarks, platform adapters, and documentation before editing.
- [ ] Classify each item as Fully done, Partially done, Not done, or external
  gate with source/test evidence.
- [ ] Revalidate security-sensitive assumptions against current official
  primary sources and record build/wrap/adopt decisions.
- [ ] Write deterministic failing tests and resource limits before production
  behavior.
- [ ] Keep process, network, credential, provider, filesystem, and UI authority
  in their existing owners; add an ADR for material changes.
- [ ] Implement in small reversible increments with cancellation, cleanup,
  redaction, stale-result rejection, and safe disabled behavior.
- [ ] Run focused tests, strict Clippy, format, repository/policy gates,
  architecture verification, applicable fuzz/bench/native/accessibility/manual
  evidence, and cargo ready.
- [ ] Re-audit source, tests, limits, UX, security, performance, platforms, and
  stale documentation before marking completion.
- [ ] Update guides, exact references, architecture/ADR, testing, roadmap,
  phase audit, feature assurance, navigation, and a change fragment.
- [ ] Group intended files into coherent DCO-signed commits and push only after
  history and remote verification gates pass.

## External evidence lanes

These do not become silently complete from local Windows development:

- [ ] Hosted Windows, Linux, and macOS jobs for the exact protected commit.
- [ ] Real OpenSSH/provider CLI versions and disposable non-production
  accounts/resources on controlled runners.
- [ ] Narrator/NVDA, VoiceOver, and AT-SPI/Orca evidence.
- [ ] Controlled GPU/PTY/process/handle/socket/leak and energy measurements.
- [ ] Thirty-day comparable performance/resource baselines and enforced
  ratchets.
- [ ] Signed/notarized/package/install/uninstall/SBOM/attestation evidence for
  the exact release artifacts.

## Primary-source assumptions revalidated on 2026-08-17

- [OpenSSH client configuration](https://man.openbsd.org/ssh_config) remains
  execution-capable through directives such as Match exec and defines jumps,
  host trust, forwarding, agent behavior, and listener semantics; passive D4
  discovery must therefore remain a strict static subset.
- [AWS CLI IAM Identity Center](https://docs.aws.amazon.com/cli/latest/userguide/cli-configure-sso.html)
  owns PKCE/device login and credential caching; Automexia invokes the visible
  CLI flow and never copies the cache.
- [Azure CLI interactive authentication](https://learn.microsoft.com/en-us/cli/azure/authenticate-azure-cli-interactively)
  owns WAM/browser/device login and MFA behavior; Automexia distinguishes user
  identities from workload identities without collecting credentials.
- [Google Cloud named configurations](https://docs.cloud.google.com/sdk/docs/configurations),
  [authentication](https://docs.cloud.google.com/docs/authentication),
  [OS Login](https://docs.cloud.google.com/compute/docs/oslogin), and
  [IAP TCP forwarding](https://docs.cloud.google.com/iap/docs/using-tcp-forwarding)
  remain official authorities; capsules select public intent without mutating
  another session's active configuration.
- [Kubernetes kubeconfig guidance](https://kubernetes.io/docs/concepts/configuration/organize-cluster-access-kubeconfig/)
  warns that untrusted kubeconfig can execute code or expose files, and the
  [ExecCredential contract](https://kubernetes.io/docs/reference/access-authn-authz/authentication/)
  carries sensitive authentication material; D6.4 requires exact trust and
  memory-only redaction.
- [PowerShell predictor guidance](https://learn.microsoft.com/en-us/powershell/scripting/dev-cross-plat/create-cmdline-predictor)
  requires PowerShell 7.2+ and PSReadLine 2.2.2+ for the public plug-in model;
  unsupported versions retain CP1/native behavior.

## Explicitly deferred work

- Embedded/native SSH protocol implementation or private-key parsing.
- Direct provider SDK inventory before CLI/config measurements justify it.
- SFTP/file synchronization and persistent background tunnels.
- Arbitrary custom scripts and organization-signed recipe packs.
- Shared PTYs, collaboration relay, team secret sync, and mobile clients.
- Public third-party extensions, sandboxed packs, CP6 selected-input model
  suggestions, optional LLM workflow orchestration, and ambient model access.
- Any background login, provider refresh, or network work triggered by startup,
  search, selection, hover, rendering, or ordinary typing.

## Current next action

ADR 0012 is accepted by the project owner. The current blocking actions are
ADR 0003's two independent exact-head protected approvals/server enforcement,
real package-loader attestation, and the native F4 evidence matrix. F3/D5.1 is
fully implemented locally; F4's source-local runner, current-executable review,
approval UI, guarded PTY, route publication, process-tree teardown, and worker
joining are implemented but nonactivated. F5.2 explicit routes/host trust and F5.3 typed tunnels are now complete locally
and nonactivated. The next M5 action is to configure the protected environment
and ephemeral runner group, then dispatch F5.4's private real OpenSSH evidence
manifests on controlled Windows, macOS, and Linux runners;
source work may proceed to F6 while those external F4/M5 gates are collected.
P1 autocomplete research is **Fully done** with CP1 retained. P2/CP5.1 and
P3/CP5.2-CP5.3 are **Fully done at their source/local model boundaries and
partially done for release**. P4/CP5.4-CP5.6 is **Partially done overall**: UI/publication source and
the inert helper/native replacement adapters exist, while reviewed launcher,
signing/attestation, WSL relay, live activation, and native/accessibility/
package/resource/30-day evidence remain open.
