# Connectivity and command-productivity focus roadmap

Status: canonical execution source of truth for the current product focus.

Audit baseline: 2d9079ab036cf34b313d7a09e5b0d7ea9a2e85b6 on 2026-08-17.

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
SFTP, collaboration, and AI execution remain outside this focus until their
separate protected phases are approved.

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
| Provider-neutral contracts | D1 | Fully done | automexia-extension-api, automexia-extension-runtime, automexia-devops, automexia-ui-model; architecture gates | Hosted release evidence only |
| Generic context and immutable capsules | D2 | Fully done | extension API/runtime plus application session/context tests | Production login/relaunch belongs to D3/D6 |
| SSH decision and native fixture baseline | D0 | Partially done | ADR 0012 is accepted; schema 5 ratchets immutable schemas 1/2/3/4 with exact M3-M5 routes/tunnels, trust/status, lifecycle/receipt/reconnect, 23-scenario native-manifest rules, and mutations | Obtain ADR 0003's two independent exact-head protected approvals/server enforcement; execute and validate real native F4/F5 manifests |
| Exact-argument launch broker | D3 | Partially done; nonactivated | Production-compiled fail-closed broker, one application runner, exact guarded PTY seam, route publication, bounded lifecycle/audit state, approval UX, and local protected-path policy | Protected approvals/server enforcement, real loader attestation, forced descendant teardown proof, native process/resource/accessibility evidence |
| Static OpenSSH inventory | D4 | Fully done | automexia-devops-ssh, hostile/property tests, fuzz, benchmark, assurance | Remains deliberately disabled until D5 |
| Connection Hub model | D5.0 | Partially done | All local F2 schemas, reducers, dry-run planner, Hub/review/planner projections, fixtures, goldens, fuzz, mutations, and benchmark are implemented | ADR 0003 protected approvals and native evidence remain for D5.2; capability-free D5.1 is complete under ADR 0022 |
| Read-only Connection Hub | D5.1 | Fully done locally; external evidence partial | App-scoped joined runtime, exact reviewed native selection, bounded modal/search/filter/grouping, CAS favorite/tag diffs, read-only recent/library data, disabled authority, tests, benchmark, and release-build evidence | Native macOS/Linux picker/permission plus controlled Narrator/NVDA, VoiceOver, and Orca evidence |
| Managed OpenSSH | D5.2 | Partially done overall; F5.1-F5.3 source-complete nonactivated | Exact direct/routed/tunnel argv, typed endpoints, loopback defaults, strong tunnel review, full trust evidence, safe copy, guarded lifecycle, receipts, reconnect, and compact states pass locally | Protected activation/attestation, actual status execution, real OpenSSH/forced cleanup, native resources/accessibility, and F5.4 manifests |
| Profiles, recipes, remote workspaces | D5.0-D5.2 | Partially done | Bounded profiles, typed recipes/actions, strict validation, deterministic dry-run planning, approval fingerprints, and private transactional Connection Library persistence/redacted transfer are implemented | Product editor, remote workspace lifecycle, and separately gated execution remain |
| Multi-cloud framework and providers | D6.0-D6.5 | Partially done overall: D6.0 fully done locally; D6.1-D6.5 not done | Bounded provider-auth capsules, 19-state lifecycle, exact review/isolation, tests, fuzz/mutation, and benchmark | Exact provider adapters, product login controls, real official-CLI/native evidence, provider-by-provider gates |
| Native completion | CP1 | Fully done | shell integration, xtask completion manager, CP1 contract/tests | Hosted three-OS and longitudinal release evidence |
| Quick Actions and persistence | CP2.0-CP2.2 | Fully done | automexia-devops model, application store/worker/UI/CLI tests | Hosted shell insertion, controlled accessibility, longitudinal evidence |
| Aliases and DevOps packs | CP3.0-CP3.2 | Fully done | compiler, private generations, shell activation, packs, fuzz/bench/contracts | Hosted native and controlled baseline evidence |
| Trusted local workspace tasks | CP3.3 | Fully done | native imports, workspace store/trust/runtime/CLI tests and ADR 0021 | Hosted native/accessibility and longitudinal evidence |
| Provider-aware Quick Actions | CP4 | Not done | Specification only | Activated D3 and public cached D5/D6 context |
| Autocomplete research | CP5.0/P1 | Fully done | Shell/API matrix, pure bounded insertion prototype, locked matcher benchmark, dependency/privacy review, retain-CP1 decision | External low-end/Zsh/Fish/accessibility evidence applies only to a future P2 proposal |
| Automexia suggestion surface | CP5.1-CP5.6/P2-P4 | Not done | Detailed bridge/source/ranking/UI plan only | Separate accepted ADR, protocol, sources, ranking, UI, shell and release gates |
| Ecosystem packs and AI | CP6/D7 | Not done | Deferred specifications | Separate sandbox, provenance, privacy, quota, and revocation programs |

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
2. P2/CP5.1 is deferred; a new proposal needs safe-bridge evidence and a separate accepted ADR.
3. P3/CP5.2-CP5.3 adds local-only sources and deterministic ranking.
4. P4/CP5.4-CP5.6 adds the optional UI, shell activation, and release gate.

CP5 never blocks production SSH. CP1 remains the complete fallback throughout.

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
- [ ] **Not done externally:** protected reviewers must accept or supersede ADR
  0012 before D5 can close or any production connection capability can activate.

Implementation evidence (2026-08-17):

- model owners: `automexia-devops/src/connections` and
  `automexia-ui-model/src/connection_hub.rs`;
- frozen contract/fixtures: `tests/fixtures/connection-hub`;
- deterministic tests: all `automexia-devops` and `automexia-ui-model` tests,
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

### User-visible shortcut completion audit (2026-08-21)

Scope: every command-palette feature that previously had a blank or mismatched
default. Internal/configuration-only actions remain intentionally outside this
table; a shortcut is not added merely to increase coverage.

| Feature | Previous evidence | Resulting status | Windows/Linux/BSD | macOS |
|---|---|---|---|---|
| Connection Hub | Palette action only; no typed/default binding | **Fully done locally** | `Ctrl+Shift+H` | `Cmd+Shift+H` |
| Quick Actions | Palette route only; no typed/default binding | **Fully done locally** | `Ctrl+Shift+O` | `Cmd+Shift+O` |
| Extensions marketplace | Palette route only; no typed/default binding | **Fully done locally** | `Ctrl+Shift+M` | `Cmd+Shift+M` |
| Font browser | Palette route only; no typed/default binding | **Fully done locally** | `Ctrl+Shift+L` | `Cmd+Shift+L` |
| Search backward | Working binding; palette label missing | **Fully done** | `Ctrl+Shift+B` | `Cmd+B` |
| Appearance | Windows-only default; Linux/macOS incomplete | **Fully done locally** | `Alt+Shift+T` | `Cmd+Alt+Shift+T` |
| Close tab / close surface / close other tabs | Actions existed; labels/defaults overlapped or were blank | **Fully done locally** | `Ctrl+F4` / `Ctrl+Shift+W` / `Ctrl+Shift+F4` | `Cmd+Shift+W` / `Cmd+W` / `Cmd+Alt+W` |

The four launcher actions have stable configuration names and share their
existing Screen-owned routes. They are inactive under Search, Vi, and
alternate-screen ownership and never insert or execute PTY input. Host-
independent tests build all three binding tables; active-platform palette
completeness is tested on each native job. Native macOS/Linux keyboard-layout
and controlled assistive-technology evidence remain external release gates.

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
- [x] **Fully done locally; nonactivated** — ContextManager alone creates the
  guarded PTY, inserts exactly one new route whose numeric route equals the
  reviewed session, and marks publication only after insertion. Any create,
  scope, capacity, or publication failure cancels and revokes the exact lease.
- [ ] **Partially done** — Completion, explicit cancellation, session
  revocation, stale-lease rejection, failed-publication rollback, route-close
  reconciliation, and application shutdown are implemented and deterministic.
  Graceful-then-forced descendant-tree behavior and listener/handle leak freedom
  still require native process evidence on each supported platform.
- [ ] **Partially done** — Hostile argv, Unicode/spaces/leading dashes,
  executable replacement, replay, stale grants, session isolation, 1/10/50
  pure cycles, runner capacity/audit bounds, publish-before-complete, shutdown,
  route mapping, keyboard focus/mnemonics, accessibility semantics, and
  tiny-to-8K pointer geometry pass locally. Native process, pane/window,
  OpenSSH-server, screen-reader, and sustained resource runs remain external.
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
- [x] **Fully done — source evidence:** active schema 5 plus immutable
  schema-1/schema-2/schema-3/schema-4 hashes, hostile/exact-argv/executable-replacement/
  outcome/redaction/store recovery/saturation/restart/stale-source tests and
  assurance mutations pass.
- [ ] **Not done externally / production blocked:** protected exact-head
  approvals/server enforcement, real loader attestation/revocation, controller
  consumption of a current attested observation, real OpenSSH prompts and
  diagnostics, graceful/forced child-tree cleanup, before/after manual `ssh`,
  controlled accessibility, and native Windows/macOS/Linux/WSL resource evidence.
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

- [ ] **Partially done locally:** exact fake preparation/argv/parser, hostile
  endpoints, independent collision domains, stale scope, terminal lifecycle,
  cleanup, Hub projection, decision, and 1/10/50 pure-model cases pass. Active
  schema 5 and a bounded duplicate-key/size/redaction-aware evidence validator
  freeze 23 ordered scenarios. A real hermetic OpenSSH server run remains
  external and is not represented by the synthetic repository fixture.
- [ ] **External prerequisite:** pass real system OpenSSH cases on Windows,
  macOS, and Linux for host keys, agents, encrypted keys, certificates, jumps,
  tunnels, cancellation, offline, hostile output, exit status, and cleanup. WSL
  is separately denied by this release contract until it receives its own
  approved evidence path.
- [ ] **Partially done:** deterministic 1/10/50 scope isolation and cleanup
  invariants pass; bounded native CPU, memory, handle/descriptor, socket, task,
  route, cache, log, and storage measurements remain external.
- [ ] **Partially done:** activation remains compile-time false, the evidence
  probe is explicit and read-only, and the repository fixture cannot satisfy a
  release. Controlled before/after enable, disable, uninstall, manual `ssh`,
  and generic-terminal baselines remain external.

Exit remains unavailable: the nonactivated tunnel source boundary is complete,
but reviewed production SSH requires protected activation plus validated real
Windows/macOS/Linux evidence without embedded SSH or secret custody.

## F6 - implement connection automation and remote workspaces

Status: Partially done; review-only source contracts are complete locally and
execution/product activation remains gated.

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
- [ ] **Partially done / external prerequisite:** proposed ADR 0023 acceptance,
  product controller/renderer/CLI integration, managed execution adapters, real
  native OpenSSH/PTY/process/resource cleanup, controlled accessibility/visual,
  and hosted Windows/macOS/Linux release evidence remain.

Exit remains unavailable for the shipped product: reusable profiles, recipes,
and remote workspace intent are safe and reviewable internally, while advanced
custom code and every M6 execution path remain disabled.

## F7 - implement D6.0 provider-neutral auth and capsule orchestration

Status: **Fully done locally** at the authority-free D6.0 framework boundary;
D6.1-D6.5 adapters, product controls, and real official-CLI/native evidence are
not done or external.

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

Status: Not done.

- [ ] Parse bounded named profiles and public region/account/role metadata.
- [ ] Use visible AWS CLI IAM Identity Center login and STS identity checks;
  never parse or persist cached credentials.
- [ ] Pin profile, region, account, role, expiry, and provenance in the capsule.
- [ ] Add exact official-CLI SSM Session Manager and EKS context flows.
- [ ] Test PKCE/device flow selection, missing/expired/denied/MFA/cancelled/
  offline states, profile precedence, production risk, isolation, and uninstall.

Exit: AWS is independently enabled, revocable, tested, and releasable.

## F9 - implement D6.2 Azure slice

Status: Not done.

- [ ] Parse bounded public Azure CLI configuration, tenants, subscriptions, and
  selected account metadata without reading tokens.
- [ ] Use visible az login with WAM/browser/device behavior owned by Azure CLI.
- [ ] Distinguish user MFA from service-principal/managed/workload identity.
- [ ] Pin tenant, subscription, cloud, account kind, expiry, and provenance.
- [ ] Add exact Bastion native-client and AKS context flows.
- [ ] Test subscription selection, tenant changes, MFA, cancellation, offline,
  conditional-access failure, isolation, and uninstall.

Exit: Azure is independently enabled, revocable, tested, and releasable.

## F10 - implement D6.3 Google Cloud slice

Status: Not done.

- [ ] Parse bounded named gcloud configurations and public account/project/
  region/zone metadata without copying credentials.
- [ ] Use visible gcloud-owned user, Workforce, or Workload Identity login.
- [ ] Pin configuration, account reference, project, region/zone, expiry, and
  provenance per capsule instead of changing the global active configuration.
- [ ] Add exact IAP/OS Login and GKE context flows through official tools.
- [ ] Test untrusted credential-configuration rejection, 2FA, IAM denial,
  configuration precedence, cancellation, offline, isolation, and uninstall.

Exit: Google Cloud is independently enabled, revocable, tested, and releasable.

## F11 - implement D6.4 Kubernetes and OpenShift slice

Status: Not done.

- [ ] Treat kubeconfig as code-capable untrusted input and require trusted exact
  sources.
- [ ] Parse bounded public cluster/context/namespace/project metadata and
  preserve kubectl/oc merge and precedence semantics.
- [ ] Deny exec credential plugins by default; add exact executable/digest/
  argument/environment/interactivity allowlists and visible approval.
- [ ] Keep ExecCredential token and client-key data memory-only and redacted.
- [ ] Pin source set, cluster, context, user reference, namespace/project,
  provider link, expiry, and provenance per capsule.
- [ ] Add exact Kubernetes/OpenShift login, exec/rsh, and cloud-cluster context
  flows with offline/expired/denied/plugin-failure states.
- [ ] Test hostile kubeconfig, relative files, link swaps, merge collisions,
  exec plugins, cross-pane isolation, cancellation, and uninstall.

Exit: Kubernetes and OpenShift are independently safe and releasable.

## F12 - implement D6.5 organization identity slices

Status: Not done.

- [ ] Implement Teleport first through exact tsh version/login/status/ssh flows
  with the external agent/cache remaining authoritative.
- [ ] Add OpenBao public-key SSH certificate signing only after its token-helper
  and certificate-file boundary passes separate security review.
- [ ] Keep opaque references and public certificate metadata only.
- [ ] Give each adapter independent grants, cache, revocation, native fixtures,
  documentation, disable, and uninstall behavior.

Exit: each organization adapter is independently approved and releasable.

## F13 - implement CP4 provider-aware Quick Actions

Status: Not done; dependency-blocked by F4-F12.

- [ ] Add capsule, SSH target, provider/account/project/subscription, cluster/
  context/namespace, region/zone, and infrastructure fields from bounded cached
  public context only.
- [ ] Label freshness, provenance, production risk, missing/expired/offline
  state, and the exact target in search and review.
- [ ] Cancel stale generations and isolate panes, sessions, providers, and
  workspaces.
- [ ] Permit exact execution only through F4 grants; preserve insert-without-
  Enter as the safe fallback.
- [ ] Add provider-specific actions one slice at a time with native tests,
  revocation, cancellation, redacted audit, and no keystroke-time refresh.

Exit: provider-aware actions never widen provider or session authority.

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

Status: Not done; deferred by P1's retain-CP1 decision and blocked on a separate
accepted ADR before any runtime implementation.

- [ ] Specify a private Windows named pipe and mode-0600 Unix socket protocol;
  never TCP, OSC, terminal output, or implicit port forwarding.
- [ ] Bind schema, peer, app/window/tab/pane/session, shell/editor, prompt,
  buffer generation, cursor, replacement span, capability, and cancellation.
- [ ] Bound frames, buffer/candidate counts and bytes, queue depth, cache,
  deadlines, and endpoint lifetime before allocation.
- [ ] Keep buffers/candidates memory-only and out of logs, telemetry, crash,
  clipboard, diagnostics, persistence, extensions, and AI.
- [ ] Have the shell editor revalidate generation/span and own final quoting and
  insertion without Enter.
- [ ] Fuzz framing and test downgrade, replay, cross-session rejection,
  permissions/ACLs, cleanup, restart, shutdown, and native fallback.

Exit: protocol and ownership gates pass before any Automexia popup is enabled.

## P3 - execute CP5.2-CP5.3 sources and ranking

Status: Not done.

- [ ] Broker native candidates, opt-in shell-owned history predictions,
  nonrecursive cwd/executable results, opt-in accepted-candidate counters,
  generated CP1 artifacts, cached public CP4 context, and typed Quick Actions.
- [ ] Exclude history-file parsing, raw command storage, terminal/remote output,
  clipboard, telemetry, AI, network, authentication, and per-key processes.
- [ ] Define deterministic source precedence, deduplication, prefix/token/fuzzy
  ranking, provenance/freshness/risk explanations, and stable tie-breaking.
- [ ] Bound work and actively cancel stale generations under rapid typing,
  output, resize, pane switches, clone, and shutdown.
- [ ] Test Unicode/graphemes/IME/RTL, quotes/spaces/multiline/selection,
  exact spans, shell modes, hostile labels, fairness, and latency/resource
  budgets.

Exit: local-only results are deterministic, explainable, bounded, and safe.

## P4 - execute CP5.4-CP5.6 UI, shell activation, and release

Status: Not done.

- [ ] Add a pane-owned accessible listbox that avoids cursor, IME, footer, tabs,
  siblings, selections, and modals and disappears without PTY residue.
- [ ] Show type, source, freshness, risk, selected state, help, loading, empty,
  unavailable, stale, and error states without color-only meaning.
- [ ] Add keyboard, focus restoration, reduced motion, high contrast, tiny-to-
  8K, 100-300% scale, split-pane, and modal/z-order goldens.
- [ ] Activate only version-proven PowerShell, Bash, Zsh, Fish, and WSL adapters;
  keep truthful Windows PowerShell 5.1, CMD, remote, and container fallbacks.
- [ ] Preserve user profiles, keybindings, predictors, completers, histories,
  aliases, functions, abbreviations, and native views byte-for-byte.
- [ ] Ship behind preview and staged opt-in with source controls, local privacy
  explanation, memory/storage display, reset, kill switch, rollback, disable,
  uninstall, and last-known-good recovery.
- [ ] Pass three-OS native shell/PTY/GUI, accessibility, fuzz, leak, resize,
  multi-pane, sleep/resume, dependency, SBOM, and 30-day baseline gates.

Exit: maintainers prove a measured UX improvement; otherwise CP1 remains active.

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
- Public third-party extensions, sandboxed packs, AI execution, and ambient
  model access.
- Any background login, provider refresh, or network work triggered by startup,
  search, selection, hover, rendering, or ordinary typing.

## Current next action

ADR 0012 is accepted by the project owner. The current blocking actions are
ADR 0003's two independent exact-head protected approvals/server enforcement,
real package-loader attestation, and the native F4 evidence matrix. F3/D5.1 is
fully implemented locally; F4's source-local runner, approval UI, guarded PTY,
and route publication are implemented but nonactivated. F5.2 explicit routes/host trust and F5.3 typed tunnels are now complete locally
and nonactivated. The next M5 action is to execute and validate F5.4's private
real OpenSSH evidence manifests on controlled Windows, macOS, and Linux runners;
source work may proceed to F6 while those external F4/M5 gates are collected.
P1 autocomplete research is now **Fully done** with CP1 retained and P2
deferred. P2-P4 remain **Not done**; no later phase may be marked started until
its separate ADR, dependencies, and evidence pass.
