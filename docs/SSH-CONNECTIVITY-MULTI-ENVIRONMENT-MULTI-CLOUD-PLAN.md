# SSH, connectivity, multi-environment, and multi-cloud implementation plan

Status: authoritative detailed implementation plan and evidence ledger.

Last reconciled: 2026-08-22.

The [Connectivity and command-productivity focus roadmap](CONNECTIVITY-COMMAND-PRODUCTIVITY-ROADMAP.md)
remains the owner of canonical phase order and phase status. This document is
the detailed, checklist-level source of truth for the SSH, connectivity,
remote-workspace, multi-environment, and multi-cloud backlog. A status change
must update both documents and the phase implementation audit in the same
change.

## 1. Outcome, scope, and acceptance criteria

### Outcome

Deliver an optional DevOps experience that lets a user discover public SSH and
provider context, review an exact action, and open isolated sessions without
turning Automexia into an SSH engine, a credential vault, or a second owner of
provider state.

### In scope

- Read-only Connection Hub discovery over the completed static OpenSSH index.
- One application-owned, exact-argument process/PTY lifecycle boundary.
- Reviewed OpenSSH connections, routes, host-trust explanations, and tunnels.
- Public-only profiles, typed recipes, declarative remote workspaces, and safe
  restoration.
- Isolated public environment capsules and official-CLI-first AWS, Azure,
  Google Cloud, Kubernetes/OpenShift, Teleport, and OpenBao adapters.
- Provider-aware Quick Actions only after the underlying public context and
  process capability are available.

### Explicit non-goals for this plan

- Embedding an SSH implementation, cloud SDK, browser login, credential vault,
  provider token cache, or a second PTY/process manager.
- Implicit provider activity on startup, Hub open, filtering, typing, renderer,
  PTY, resize, or shell-completion paths.
- Capturing passwords, keys, passphrases, tokens, agent protocol data,
  `ExecCredential` payloads, browser cookies, terminal contents, or inherited
  environment values.
- Automatic `known_hosts`, shell-profile, provider-config, kubeconfig, or
  persistent remote-state mutation.
- SFTP, file transfer, collaboration, direct provider SDK/API inventory,
  third-party extensions, and AI execution. They need separate protected
  programs.

### Release-level acceptance criteria

- A user can inspect public source, target, identity reference, route,
  environment, freshness, risk, requested capability, and exact operation
  before authority is exercised.
- Each managed connection gets one new, independently cancellable application
  session, PTY, route, and immutable Environment Capsule. A clone copies
  intent, never a live process, identity, or credential.
- A managed action uses an approved canonical executable and exact argv; no
  command string, shell evaluation, relative executable lookup, implicit PATH
  fallback, or extension-owned process exists.
- OpenSSH and official tools continue to own protocols, host keys, agents,
  MFA, browser/device flows, caches, certificates, and diagnostics.
- Two panes can use distinct account/subscription/project/context/namespace
  selections without observing or changing one another's context.
- Disabling an integration removes its authority and leaves normal manual
  `ssh`, `aws`, `az`, `gcloud`, `kubectl`, and `oc` terminal use unchanged.
- Every shipped slice has deterministic boundary tests, redaction canaries,
  cancellation/replacement/shutdown tests, documentation, and the applicable
  native-platform evidence. No local or cross-compiled test is described as a
  cross-platform release result.

## 2. Architecture, trust, and ownership — fixed constraints

| Concern | Sole owner | Implementation rule |
|---|---|---|
| Windows, tabs, panes, PTYs, routes, process lifetime, capability decisions, redacted audit | `apps/automexia-terminal` | Add one reusable `ExternalToolRunner`; do not create a provider or extension launcher. |
| IDs, manifests, capabilities, exact launch requests, public capsules, bounded text | `automexia-extension-api` | Typed/versioned data only; no process, network, renderer, or credential value. |
| Bounded worker, cache, cancellation, freshness, operation generation | `automexia-extension-runtime` | Publish immutable state before waking UI; reject stale results. |
| Provider-neutral connection/profile/recipe/tunnel/review models | `automexia-devops::connections` | Keep pure, bounded, validated, and execution-disabled until the broker accepts a reviewed request. |
| Hub/review/planner projection and accessibility semantics | `automexia-ui-model::connection_hub` | Renderer-neutral projection; application adapter owns actual screen/input/painting. |
| Static OpenSSH subset and its public metadata | `extensions/devops-ssh` | Exact granted local files only; no `ssh -G`, `Match exec`, `ProxyCommand`, shell expansion, process, or network. |
| SSH protocol, complete config semantics, agents, keys, host keys, certificates | installed system OpenSSH | Wrap it in a normal Automexia PTY; never replace it. |
| Provider login, MFA, browser/device flow, token cache, provider protocol | installed official CLIs and organization tools | Invoke only after reviewed capability approval; expose bounded public state only. |

The build/wrap/adopt decision is **wrap first**. Do not add an SSH engine,
provider SDK, kube client, credential-store crate, or browser component merely
to implement a roadmap item. A future dependency needs its own adoption
checklist, threat model, benchmark, license/advisory review, rollback plan, and
ADR where applicable.

### Mandatory global invariants

- All external work is off renderer, input, PTY, resize, startup, and keystroke
  paths. It is bounded, cancellable, generation-aware, and has one cleanup
  owner.
- All untrusted input — configuration, provider output, remote output, paths,
  labels, imports, completion values, and AI suggestions — is validated and
  rendered as data, never as authority.
- Persistent Automexia state is versioned, size-bounded, private, no-follow,
  atomic, recoverable, removable, and stores public metadata or opaque
  references only.
- A review binds extension/package identity, executable identity, operation,
  session, capsule revision, target, argv, working-directory policy, requested
  capability, and destination. Any material change invalidates approval.
- Strict host-key checking remains on; unknown keys require explicit user-owned
  OpenSSH trust handling, changed keys fail, and agent/X11/TCP forwarding and
  remote commands start disabled.
- UI uses a window-level modal, inert background, keyboard-only route/focus
  handling, text plus color/state, high contrast, reduced motion/transparency,
  and usable wide through tiny/8K layouts at 100–300% scale.

## 3. Evidence-led current-state ledger

The classifications below are based on current source, tests, and documents,
not roadmap prose alone. “Fully done locally” is deliberately narrower than a
native/release claim.

| Area | Status | Current evidence | Remaining exit work |
|---|---|---|---|
| D0 SSH trust/fixture contract | Partially done | ADR 0012 is accepted; schema 5 ratchets immutable schemas 1/2/3/4 with exact direct/routed/tunnel argv, full trust evidence, tunnel lifecycle, recovery/status, receipt/reconnect, 23-scenario native-manifest rules, and mutations | ADR 0003 two exact-head approvals/server enforcement and controlled real native fixture execution. |
| D1 contracts and D2 immutable capsules | Fully done locally | `automexia-extension-api`, runtime, DevOps model and architecture checks | Preserve while adding real login/launch; hosted release assurance remains separate. |
| D3 launch broker | Partially done; nonactivated | The production broker, one application runner, exact executable guard, ContextManager PTY/route seam, bounded lifecycle/audit, approval UX, and mutation checks pass locally | Production constant stays false and linked package unverified until protected approvals, attestation, descendant cleanup, and native evidence pass. |
| D4 static OpenSSH inventory | Fully done locally | `extensions/devops-ssh`: bounded parser, canonical first-value ProxyJump chains (8 hops/2 KiB), grants, hostile/property/fuzz tests, 10,000-alias benchmark, private revisioned metadata | Deliberately remains non-executing; hosted macOS/longitudinal release proof remains external. |
| F2/D5.0 Hub and planning model | Partially done overall; fully done locally | Pure records, validation, reducers, fingerprints, review/planner projection, accessibility goldens, fuzz and benchmark | ADR 0003 protected activation/native evidence blocks phase closure; no production authority is granted. |
| F3/D5.1 read-only Connection Hub | Fully done locally; external evidence partially done | App-owned joined runtime, exact reviewed native selection, compact progressive setup, bounded browse/filter/group, redundant text/icon/color semantics, D4 favorite/tag CAS, read-only recent/library state, disabled authority, Windows tests/benchmark/build/native frame | Native macOS/Linux picker/permission and controlled screen-reader evidence remain external. |
| F3 Connection Library | Fully done locally | `ConnectionLibraryStore` has 16 MiB private documents, CAS, recovery, transfer redaction, fresh import IDs, read-only/disk-full and link tests | Product editor/manager belongs to F6; macOS/Linux native permission and controlled screen-reader evidence remain. |
| F5 managed OpenSSH | Partially done overall; F5.1-F5.3 source-complete nonactivated | Exact direct/routed/tunnel argv; typed host/user/port/endpoints; loopback defaults; strong per-use tunnel review; full trust evidence; guarded lifecycle; receipts; reconnect; and compact tunnel state pass locally | Protected activation/attestation, actual status execution, real OpenSSH/forced cleanup, native resources/accessibility, and validated F5.4 real manifests remain. |
| F6 recipes and remote declarative workspaces | Partially done; review-only source contracts complete locally | Library schema 2/editor/migration previews, exact dependent fingerprints, pure recipe review/lifecycle, typed remote initialization, declarative workspaces/restore, armed broadcast, semantic projections, fuzz, mutation, and benchmarks pass | Proposed ADR 0023 acceptance, product editor/controller/renderer wiring, managed execution adapters, and controlled native/resource/accessibility evidence remain. |
| D6.0/M7 provider-neutral authentication and capsule orchestration | Fully done locally | Bounded strict schemas, immutable provider contexts, 19-state lifecycle, generation/session isolation, exact allow-once review, redacted receipts/audits, passive-status migration, fuzz/mutation/benchmark evidence | D6.1-D6.4 and D6.5 Teleport builders are source-complete/nonactivated; product activation, real official-CLI/native evidence, and OpenBao remain external/not done. |
| D6.1/M8 AWS | Partially done overall; source-complete nonactivated | Independent `automexia-devops-aws`; bounded public profiles; exact SSO/STS/SSM/EKS dry-run contracts; 10 focused tests | D3 product activation/attestation, M11 EKS ingestion, and controlled real official-tool/native/resource/accessibility/release evidence remain. |
| D6.2/M9 Azure | Partially done overall; source-complete nonactivated | Independent `automexia-devops-azure`; bounded public account JSON; exact tenant-scoped login/account observation; AAD-only Bastion plan; opaque transient AKS intent; 8 focused tests | D3 product activation/attestation, M11 AKS ingestion, and controlled real Azure/native/resource/accessibility/release evidence remain. |
| D6.3/M10 Google Cloud | Partially done overall; source-complete nonactivated | Independent `automexia-devops-gcp`; bounded named public config; exact per-command login/project observation; opaque federation; scope-bound IAP; private-environment GKE intent; 8 focused tests | D3 activation/attestation, M11 GKE ingestion, and controlled real Google/native/resource/accessibility/release evidence remain. |
| D6.4/M11 Kubernetes/OpenShift | Partially done overall; source-complete nonactivated | Independent Kubernetes/OpenShift packages; bounded exact-source YAML/JSON parsing and merge; default-denied exec review; exact isolated kubectl/oc plans; 13 tests and benchmark | D3/product activation, real client/cluster/native/resource/accessibility/release evidence. |
| D6.5/M12 Teleport/OpenBao | Partially done overall | Teleport is source-complete and nonactivated through a separate bounded extension with exact version/login/status/ssh/logout plans; OpenBao ADR 0024 remains proposed and unaccepted | Activate and prove Teleport only through D3/native release gates; accept ADR 0024 before any OpenBao code. |
| Provider-aware Quick Actions (CP4/F13) | Not done and blocked | CP2/CP3 typed/persistent/insert-only Quick Actions and static packs exist | Consume only F7+ cached public context; exact execution stays behind F4. |

### Important distinction: existing legacy DevOps status is not D6

`automexia-devops::context` and the application DevOps runtime now derive only
bounded public status from local files and already-supplied session metadata.
The previous Windows WSL `sh -c`/provider-CLI probe was removed. This legacy
surface remains passive display, not verified authentication, provider
isolation, capability review, or an authoritative managed-pane selection.
D6.0 provider work uses immutable capsule observations instead.

The M7 legacy migration is complete at the local source boundary:

- [x] **Fully done locally** - Keep only bounded, local, public, non-secret
  information needed for the legacy status surface.
- [x] **Fully done locally** - Remove automatic CLI invocation and the `sh -c`
  WSL probe from passive provider discovery. Later provider commands must use
  the existing reviewed runner boundary and an explicit refresh.
- [x] **Fully done locally** - Treat process environment and global CLI
  “active” selection as display hints only; immutable capsule context is the
  managed-session authority.
- [x] **Fully done locally** - Regression and policy tests prove passive status,
  Hub/filter/model projection, and keystroke paths contain no provider-process
  launch authority.

## 4. Ordered delivery plan

Each checkbox is an implementation exit condition, not an aspirational feature
label. Begin a phase by reconciling it against this ledger and the canonical
roadmap; update the status only after its evidence ladder passes.

### M0 — reconcile documentation and protect the current foundations

Status: **Fully done** on the 2026-08-21 local evidence boundary.

- [x] **Fully done (was Partially done)** — Reconcile the current F3 Connection Library implementation in
  `PHASE-IMPLEMENTATION-AUDIT.md`, `ROADMAP.md`, the feature assurance/testing
  pages, and user documentation. Earlier audit prose still lists persistence
  and read-only/disk-full evidence as missing even though the committed store
  and tests now cover them.
- [x] **Fully done (updated)** — Keep the D0/D3 statement exact: ADR 0012 is
  owner-accepted; the production broker/runner/PTY-route boundary remains hard
  disabled and the package unverified until all protected and native gates pass.
- [x] **Fully done (was Not done)** — Add this plan to the documentation navigation without editing unrelated
  current work; update `docs/index.md` only in a coherent documentation commit.
- [x] **Fully done (was Partially done)** — Record the current local evidence commands, platform, revision, test
  results, benchmark hardware, and any unavailable native/release evidence.
- [x] **Fully done (was Partially done)** — Add a `changes/` fragment for each released user-visible slice, not for
  this planning-only document unless project policy requests one.

Exit achieved locally: all relevant roadmap/audit/reference/user documents make
the same truthful claim about F3, D3, and the provider backlog. The Rustfmt
baseline was restored and `cargo ready` passed the repository validation,
PowerShell integration, policy, architecture, formatting, workspace check,
warnings-as-errors Clippy, full workspace tests, dependency policy, application
build, and executable smoke gates. Hosted validation and native macOS/Linux
permission evidence remain external release evidence and do not reopen this
documentation/evidence phase.

### M1 — finish F3/D5.1 as a real read-only Connection Hub

Status: **Fully done at the source and local Windows boundary.** Native macOS/Linux and controlled screen-reader evidence remain **Partially done** external release gates; they do not add missing M1 behavior.

#### M1 implementation audit and bounded design (2026-08-21)

Outcome: ship a product-visible, read-only Connection Hub that can explicitly
review and scan selected local OpenSSH configuration files, browse the bounded
D4 catalog, and edit public favorites/tags. The phase does not add connection,
authentication, provider, process, network, listener, credential, clipboard,
or PTY authority.

Acceptance criteria:

1. One application-scoped service opens the D4 metadata store at
   <config>/extensions/devops-ssh and the Connection Library at
   <config>/connections; recovery/failure is a fixed redacted product state.
2. Opening the Hub never scans or performs filesystem, process, network,
   authentication, provider, or PTY work. An exact native file selection is
   reviewed and confirmed before a bounded scan starts; raw source paths remain
   memory-only and are never logged or persisted.
3. The single worker coalesces obsolete work, cancels scans, rejects stale
   completions, publishes immutable state before its route-specific wake, and
   is explicitly cancelled and joined at application shutdown.
4. The real modal reuses the renderer-neutral virtualized catalog, focus, and
   accessibility model. First-run setup presents one compact primary action;
   catalog search/filter/group controls appear only when useful and are absent
   from hidden pointer, keyboard, IME, and accessibility navigation. Selection/
   review, loading, stale/error/recovery, scaling, contrast, reduced motion/
   transparency, and modal-stack behavior are deterministic.
5. Favorite/tag edits show a public diff, validate the existing D4 schema, use
   revision compare-and-swap, surface contention without overwriting, and
   recompose the last-known-good catalog after success. Recent-use remains
   read-only.
6. Profiles, recipes, and preferences are displayed only as local,
   non-executing configuration. Connect, login, automatic actions, provider
   refresh, and all other authority-bearing actions remain visibly disabled.

Evidence ledger before implementation:

| Item | Classification | Existing owner/evidence | Missing exit proof/action |
|---|---|---|---|
| Bounded exact OpenSSH inventory and D4 metadata CAS/recovery | Fully done | automexia-devops-ssh; hostile, permission, cancellation, fuzz, and benchmark evidence | Preserve as the only inventory/metadata authority. |
| Bounded catalog/query/group/virtualization and modal accessibility/focus projection | Fully done locally | automexia-ui-model::connection_hub; structured layout/accessibility goldens and 10,000-row projection benchmark | Adapt into the product renderer/controller; do not duplicate the model. |
| Connection Library profile/recipe/preference persistence | Fully done locally | ConnectionLibraryStore; validation, redacted transfer, CAS, recovery, link/read-only/disk-full tests | Initialize once and expose only a non-executing snapshot/recovery state. |
| Explicit-grant catalog composition, cancellation, last-known-good state, and setup guidance | Fully done locally | Router-owned `ConnectionHubRuntime`; bounded-saturation and runtime integration tests | One joined worker owns a capacity-two non-blocking inbox, grants, generations, CAS recomposition, route wakes, and deterministic shutdown. |
| Application lifetime and product controller | Fully done locally | Router/application/Screen owners; two-controller isolation and projection-cache tests | One app service and per-screen controller preserve route-owned review tokens, generation, focus, modal isolation, and cached steady-frame projection. |
| Native exact-file selection | Fully done locally; external native coverage partial | Pinned `rfd` 0.17.2 adapter parented to the active window | Explicit multi-file selection is cancellable and memory-only; hosted macOS/Linux picker runs remain external. |
| Product modal and input/accessibility adapter | Fully done locally | Sugarloaf modal, Screen adapter, renderer/controller tests, prompt-ready Windows frame | Compact progressive setup, contextual catalog controls, text-plus-vector-icon semantic color, keyboard, pointer, IME, focus, overlays, disabled authority, and bounded tiny-to-8K geometry are covered; controlled screen-reader evidence remains external. |
| Favorite/tag mutation UI | Fully done locally | Controller/runtime CAS review flow and conflict tests | Public diffs require confirmation; conflicts reload rather than overwrite; recent remains read-only. |
| Windows/macOS/Linux controlled product evidence | Partially done | Windows 11 source, test, dependency, release-build, and benchmark evidence | Native macOS/Linux picker/permission and Narrator/NVDA/VoiceOver/Orca runs remain external. |

Build/wrap/adopt decision: build Automexia's controller, policy, worker
lifecycle, rendering adapter, and semantic state; reuse the existing D4,
Connection Library, route-wakeup, Sugarloaf, and renderer-neutral accessibility
owners; adopt the operating-system file dialogs through rfd rather than
building unsafe Win32/AppKit/portal adapters or a second in-app file browser.
The picker adds no remembered grant and has a fail-closed unavailable/cancelled
path. ADR 0022 records dependency, lifecycle, platform, rollback, and degraded
operation consequences.

Rollback: removing the palette action and Router-owned service makes the Hub
unreachable without affecting normal terminal/OpenSSH use. Removing rfd
removes only exact native selection; D4 metadata and library documents remain
versioned and readable. No migration or source-file mutation is introduced.

#### M1.1 App-owned lifetime, configuration, and explicit scan

- [x] **Fully done** — Locate the application configuration/state-root owner and initialize
  `MetadataStore` and `ConnectionLibraryStore` there once per application
  lifecycle; surface private-store recovery as a redacted actionable state.
- [x] **Fully done** — One joined/cancellable Hub supervisor now owns explicit
  shutdown, scan cancellation, grant clearing, worker wake, and deterministic
  join for the application lifetime. Its capacity-two inbox never blocks the
  caller; saturation publishes `connection-worker-busy` without secret/path data.
- [x] **Fully done** — Add an `Open Connection Hub` typed action,
  command-palette entry, mnemonic `Ctrl+Shift+H` / `Cmd+Shift+H` default, and a
  window-level modal controller. Cross-platform tables are collision-tested;
  Search, Vi, and alternate-screen owners suppress the launcher. Opening the
  Hub must read only the already
  opened private state and must not scan, launch, authenticate, connect, or
  resize a PTY.
- [x] **Fully done** — Implement user-controlled “choose exact files” flow. It must canonicalize
  and review each selected local config file before creating an `InventoryGrant`;
  never silently read candidate locations, follow links/reparse points, or use
  a remembered path as a new grant.
- [x] **Fully done** — Do not persist raw source paths merely for convenience. Initial F3 can ask
  for re-selection after restart; any later persistent grant/reference needs a
  privacy, source-change, revocation, and migration design before it is added.
- [x] **Fully done** — Wire `request_explicit_scan` to the chosen grants only. Coalesce/revoke a
  prior scan, preserve last-known-good records, publish the snapshot before
  rendering wakeup, skip obsolete queued generations, and expose only stable
  redacted diagnostic codes.

#### M1.2 Read-only UX and metadata changes

- [x] **Fully done** — Adapt the existing pure Hub catalog/view model into the real modal:
  compact first-run setup, virtualized rows, local cancellable search,
  contextual filters/grouping/source controls, revision/freshness, selection,
  inspector/detail route, focus trap/restore, and empty/filtered/loading/stale/
  error/setup states. Hidden catalog controls are removed from visual and
  accessibility order and reject pointer, shortcut, and IME input. Unchanged
  frames reuse a cached projection; IME preedit validates without traversal.
- [x] **Fully done** — Implement truthful platform-specific setup guidance for Windows, macOS,
  and Linux. It may show candidate locations but must state that exact selection
  is required and that no process/network/login runs.
- [x] **Fully done** — Add favorite and tag editing through D4 `MetadataStore` only: show a diff,
  validate public text, use revision CAS, handle contention/stale revision, and
  recompose the catalog after a successful write.
- [x] **Fully done** — Keep `recent` read-only until F5 records a successful managed connection.
  Do not mark a record “recent” from selection, review, scan, or failed launch.
- [x] **Fully done** — Expose profile/recipe/preference data only as non-executing local
  configuration in this phase. Full editor, migration UX, and remote workspace
  lifecycle remain F6 work.
- [x] **Fully done** — Make Connect, Login, automatic action, provider refresh, and all network/
  process actions visibly disabled with a short reason and no hidden fallback.
  Selecting a connection may open a disabled review, but never causes a PTY or
  command.

#### M1.3 M1 tests and evidence

- [x] **Fully done** — Add controller integration tests for explicit grants, no-scan-on-open,
  stale result rejection, worker cancellation/join, metadata CAS conflicts,
  selection/focus restoration, disabled-action non-authority, deterministic
  queue saturation, per-controller review isolation, and steady-frame cache reuse.
- [x] **Fully done** — Add renderer-neutral plus product UI tests for keyboard/pointer/IME,
  screen-reader names, 100–300% scale, tiny/normal/ultrawide/8K layouts,
  high contrast, reduced motion/transparency, long Unicode/bidi-safe labels,
  and modal stacking.
- [ ] **Partially done** — Run native static permission/recovery tests on Windows, macOS, and Linux;
  perform controlled Narrator/NVDA, VoiceOver, and Orca verification where
  runners are available. Report unavailable systems honestly.
- [x] **Fully done** — Re-run the 10,000-record filter/projection benchmark against the same-host
  baseline and verify open/search does not affect terminal input/render latency.

#### M1 completion evidence (2026-08-21)

- **Fully done locally — correctness and lifecycle:** 10 runtime, 8 controller,
  3 targeted worker/controller/cache unit, 5 renderer, 33 D4, 33 UI-model,
  5 Connection Library, and 46 command-palette tests passed on Windows 11. The
  contracts include no-scan-on-open, grant token isolation/revocation, bounded
  saturation, stale-generation rejection, steady-frame cache reuse,
  publish-before-wake, explicit joined shutdown, CAS conflict/reload,
  PTY-inert modal input, and disabled authority.
- **Fully done locally — security and dependencies:** `cargo deny check` passed
  advisories, bans, licenses, and sources. `rfd` is the only new direct
  dependency; `pollster` is its only new transitive package. No raw selection
  path is persisted or logged, and fixed diagnostic codes avoid host/path data.
- **Fully done locally — performance and size:** the Windows release Criterion
  50-sample warm run projected 10,000 records in 7.2849–7.5959 ms (target below
  16 ms). An immediate post-LTO run was noisier at 8.5180–9.5607 ms; the longer
  rerun investigated rather than erased that first result. The release
  executable remains 22,670,336 bytes, 650,752 bytes (2.96%) above the same-host
  pre-M1 baseline. Unchanged frames do not reproject or clone the full catalog;
  the runtime stays off PTY/input/render hot paths.
- **Fully done locally — Windows visual evidence:** a feature-gated test
  control waited for the renderer-neutral prompt-active signal before opening
  the Hub. The resulting 1600x950 frame at 125% scale was inspected for complete
  bounds, hierarchy, restrained semantic color, icon/text redundancy, focus,
  and absence of the setup-only catalog toolbar.
- **Partially done — external evidence:** native macOS/Linux file-picker and
  permission/recovery runs plus controlled Narrator/NVDA, VoiceOver, and Orca
  verification require their respective hosted systems. Structural semantics
  and responsive geometry are tested locally but are not reported as those
  external native runs.

Exit: a user can safely browse, diagnose, tag, and favorite a D4 inventory in
the product; no connection, provider, authentication, process, network, or PTY
authority is introduced.

### M2 — complete the D0 external decision and activate F4/D3 safely

Status: **Partially done.** ADR 0012 is accepted by the project owner and the
local runner, guarded executable consumption, ContextManager PTY/route seam, and
approval UX are implemented. Production remains fail-closed: the compile-time
gate is false and the linked extension candidate is unverified.

#### M2 implementation audit (2026-08-22)

| M2 area | Classification | Current evidence | Missing exit proof |
|---|---|---|---|
| ADR/protected review | Partially done | ADR 0012 is accepted; local policy counts distinct non-author, non-bot approvals bound to the exact PR head. | Two independent exact-head approvals and non-bypassable server enforcement required by ADR 0003. |
| Inherited S0/v0.4 gates | Not done externally | Local earlier-phase gates exist. | Green hosted CI, CodeQL, hostile-output, native, and release jobs on the protected M2 revision. |
| Trusted package attestation | Not done | Exact identity/version/contract/digest/verification/revocation policy and fail-closed tests exist; the linked candidate is unverified. | Real loader/build-provenance attestation and live revocation binding. |
| Application runner | Fully done locally; nonactivated | Router owns one shared runner; active operations stop at 50, audits at 256, route/session/lease scope is exact, completion requires publication, and shutdown reconciles. | Protected activation and native process/resource proof. |
| PTY/route lifecycle | Partially done | ContextManager alone consumes the guard, creates the exact PTY, inserts one independent Context, then marks the matching route/session published; natural completion and cancellation reconcile. | Cross-platform forced descendant cleanup, PID reuse, durable audit, listener/tunnel cleanup, and leak proof. |
| Capability/recovery UX | Partially done | Review shows public package/launcher, target, route, risk, exact operation, 60-second approval scope, destination, and deny/allow-once/allow-session with pointer, focus, mnemonics, accessibility, and redacted recovery. | Bind the current resolved executable observation into a fresh pre-launch review after attestation; then obtain controlled pixels, screen-reader, localization, and high-scale evidence. |
| Native OpenSSH evidence | Partially done | Windows guarded-spawn/ConPTY tests pass and macOS cross-check compiles. | Real loopback OpenSSH on native Windows/macOS/Linux and gated WSL plus controlled 1/10/50 resources. |

The 2026-08-21 remote snapshot found insufficient collaborators, no exact-head
PR/hosted run, and unavailable ruleset enforcement on the current plan. Recheck
that external state against the exact activation revision.

#### M2.1 Protected prerequisites

- [ ] **Partially done** — ADR 0012 is accepted by the project owner. Obtain the
  two independent exact-head approvals and non-bypassable server enforcement
  still required by ADR 0003.
- [ ] **Not done externally** — Pass inherited S0/v0.4 hostile-output, hosted
  CI, CodeQL, native, and release gates on the exact M2 revision.
- [ ] **Not done** — Bind the real loader's non-zero digest, publisher, exact
  version/contract, reviewed/signed status, and live revocation result. Never
  promote the current unverified linked candidate.

#### M2.2 One application-owned `ExternalToolRunner`

- [x] **Fully done locally; nonactivated** — Router owns exactly one runner for
  all windows. Typed intent contains one destination, allow-once/session
  decision, route/session/operation scope, and 60-second authorization expiry.
  The runner owns the trusted startup cwd and a bounded 14-name environment
  allowlist; PTY I/O is route-bound and no implicit Enter/background launch
  exists.
- [x] **Fully done locally** — Active operations stop at 50; redacted FIFO
  records stop at 256. Overflow, duplicate publication, pre-publication
  completion, stale lease, replay, capsule mismatch, and shutdown fail closed.
- [x] **Fully done locally** — Broad ProcessSpawn, arbitrary executables,
  shell evaluation/profiles, PATH/cwd executable lookup, extension-selected
  environment, secrets, raw command text, and WSL production remain denied by
  source and mutation checks.
- [x] **Fully done locally; nonactivated** — The runner reopens and compares the
  broker-reviewed file identity. Teletypewriter owns the Windows guarded
  explicit-path/suspended kill-on-close Job Object and Unix descriptor
  execution. ContextManager is the only exact PTY caller.
- [x] **Fully done locally; nonactivated** — Route, session, operation, capsule,
  decision, executable guard, argv, PTY, and Context are bound. Context
  insertion precedes lease publication and renderer wake.
- [ ] **Partially done** — Natural completion, cancellation, revocation,
  stale-lease/sibling isolation, capacity, and app shutdown reconcile locally.
  Cross-platform forced child trees, listener/tunnel cleanup, PID reuse, thread
  joining, and controlled resources remain.
- [ ] **Partially done** — Authorization/completion/cancellation audits are
  bounded and redacted in memory. Durable retention/rotation/recovery awaits a
  reviewed storage/privacy policy.

#### M2.3 Capability/recovery UX

- [ ] **Partially done; nonactivated** — The three-card review exposes public
  package/launcher state, target, direct transport/new route, host trust,
  `session.launch`, fixed-options-plus-one-destination operation, PTY I/O,
  60-second scope, risk, and
  destination without aliases, opaque references, digests, paths, argument
  values, environment values, or terminal data. A current resolved executable
  observation must still be bound into a fresh review after real attestation.
- [x] **Fully done locally; nonactivated** — Deny, allow once, and allow session
  have pointer targets and A/Enter/S/D mnemonics. Enter cannot bypass Back; Tab
  includes PrimaryAction; accessibility buttons mirror the choices. Execution
  remains independently disabled.
- [x] **Fully done locally; nonactivated** — Protected review, attestation,
  missing OpenSSH, executable change, capacity, stale review, cwd/route,
  publication, and denial map to fixed recovery copy; raw codes are not drawn.
- [x] **Fully done** — Grants are allow-once or session-scoped. Persistent
  grants require a separate ADR, migration, revocation UX, and review.

#### M2.4 Tests and evidence

- [x] **Fully done locally for pure/application contracts** — Tests cover
  hostile/oversized destinations, environment/secret denial, expiry/replay,
  executable replacement, 50-operation saturation, 256-record retention,
  route/session publication, completion-before-publication, shutdown,
  pointer/focus/mnemonics, and redaction. Mutations reject test-gating production
  modules or PTY/process authority outside ContextManager.
- [ ] **Partially done for native primitives** — Windows exact-spawn/ConPTY
  lifecycle and macOS compile evidence pass. Native Linux/macOS runtime, real
  OpenSSH, pane/window publication, forced cleanup, and durable audit remain.
- [ ] **Not done externally** — Run the frozen loopback matrix on native
  Windows/macOS/Linux and gated WSL; capture private redacted manifests and all
  six zero-resource cleanup invariants.
- [ ] **Not done externally** — Measure prompt/cancel/input/render latency, CPU,
  memory, process/handle/descriptor/task/route counts, and repeated close on the
  exact approved build.

Exit: the boundary is locally implemented but unavailable. No provider or SSH
request receives a grant until protected, attestation, hosted, native, resource,
and controlled-accessibility gates pass.

### M3 — F5.1 direct reviewed OpenSSH

Status: **Partially done overall; fully done at the nonactivated source
boundary.** Production authorization and native release evidence remain blocked
by M2's external gates.

- [x] **Fully done — destination preparation:** one inventory-typed concrete
  alias or one bounded typed literal host maps to a canonical F2 plan. Hostile,
  option-like, ambiguous, URI/raw-IPv6/tunnel/shell forms fail closed. M4 now
  adds separate typed user/port fields and bounded config-defined jump chains;
  tunnels remain M5.
- [x] **Fully done — review and exact request:** the responsive Hub review keeps
  private identity outside presentation and offers explicit Allow once, Allow
  for session, and Deny. The request freezes 17 application-owned defensive
  OpenSSH options in order followed by exactly one reviewed destination. It
  disables forwarding, multiplexing, proxy/jump commands, local/remote command,
  escape commandline, X11, tunnel, backgrounding, and key addition while leaving
  OpenSSH authentication and the interactive `StrictHostKeyChecking=ask` prompt
  in the ordinary PTY.
- [x] **Fully done — fresh launch binding:** only equality with a newly rebuilt
  full review can create the opaque launch binding. Profile/source/capsule/plan,
  destination, observation generation/freshness, host trust, capability, and
  canonical executable identity are all bound. The broker hashes current native
  file identity and rejects replacement before authorization; no screen path may
  rebuild raw argv from a preparation.
- [x] **Fully done — local lifecycle source:** one Router-owned runner publishes
  only through ContextManager's independent PTY/route seam. Real child exit,
  nonzero exit, unavailable status, cancel, route close, revocation, and shutdown
  map to distinct redacted terminal outcomes and fixed notifications; closing a
  route never implies success.
- [x] **Fully done — receipts and reviewed reconnect source:** terminal outcomes
  produce provider-neutral receipts. A private connection worker persists at
  most 256 records/2 MiB using atomic primary/previous recovery. Destination,
  terminal text, credentials, paths, and executable identity are excluded.
  Reconnect candidates carry only opaque inventory identity/source revision and
  rebuild from current D4 state; changed or missing source fails stale and every
  reconnect still requires a fresh executable/host-trust review and approval.
- [x] **Fully done — contract and deterministic evidence:** immutable schemas
  1-4 are retained. Active schema 5 freezes M3-M5 route/trust/status/tunnel/
  lifecycle/native-manifest rules, preserves upstream post-quantum KEX defaults
  and weak-crypto warnings, and is covered by pure, broker, persistence,
  saturation, restart, redaction, mutation, and feature-assurance tests.
- [ ] **Not done externally / activation blocked:** obtain ADR 0003's two
  independent exact-head approvals and server enforcement; bind real loader
  attestation/revocation and a current executable observation into the product
  controller; then execute real native OpenSSH prompt/diagnostic, before/after
  manual-SSH, graceful/forced descendant cleanup, 1/10/50 resource, controlled
  screen-reader, and Windows/macOS/Linux plus separately gated WSL evidence.

Exit remains unavailable as a shipped connection: the implementation is
additive and fail-closed, but the product cannot start a managed child until all
protected, attestation, and native evidence gates pass.

### M4 — F5.2 explicit routes, host trust, and identity readiness

Status: **Fully done at the local nonactivated source boundary.** Production
activation, real status-process execution, and controlled native evidence remain
**Partially done** external/protected gates inherited from M2/F4.

- [x] **Fully done locally; nonactivated — typed routes:** the literal editor has
  separate bounded host, optional user, and optional decimal port fields. D4
  accepts only first-value, canonical comma-separated `[user@]host[:port]`
  `ProxyJump` chains (maximum 8 hops and 2 KiB; bracketed IPv6 only). Routed argv
  uses one exact `-J` value before one config alias. Free-form `-o`,
  `ProxyCommand`, remote command, shell text, tunnels, KEX overrides, and
  weak-crypto-warning overrides cannot enter the grammar.
- [x] **Fully done locally; nonactivated — host trust:** unknown, first-use,
  known, and changed evidence binds the complete public key algorithm and full
  32-byte OpenSSH `SHA256:` fingerprint into the review. The Safety card wraps
  rather than truncates this evidence. Changed keys are always blocked and
  cannot produce a launch binding; Automexia never accepts, deletes, replaces,
  or writes `known_hosts`.
- [x] **Fully done locally; nonactivated — user-owned recovery:** Connection
  Review exposes mnemonic `C` to copy the exact reviewed command. The handoff
  contains no newline or implicit Enter and is never executed by Automexia.
  First-use confirmation and changed-key recovery remain OpenSSH/user-owned.
- [x] **Fully done locally as a non-executing contract — public identity status:**
  agent, certificate, and hardware references can request only exact
  `ssh-add -l -E sha256`, bounded to 2 seconds, 64 KiB, 64 identities, and
  public bits/algorithm/full fingerprint/safe comment. Parsing rejects hostile,
  duplicate, excessive, or secret/path-like data. The request is not connected
  to process authority while activation is false; expiry remains an external
  owner’s public observation, not inferred locally.
- [x] **Fully done locally; nonactivated — forwarding and freshness:** agent,
  TCP, X11, command, multiplex, and tunnel forwarding remain off. Profile,
  source, capsule, plan, executable, observation, route, full trust, or public
  identity changes invalidate the review. Production routes remain review-only.

Exit achieved locally: routes and trust changes are understandable, reviewable,
full-evidence, and fail closed. This is not a production connection claim.

### M5 — F5.3 typed tunnels and M5.4 native SSH release evidence

Status: **Partially done overall.** F5.3 is fully done locally at the
nonactivated source boundary. F5.4's bounded evidence contract and synthetic
mutation fixture are implemented, while controlled real OpenSSH runs and M2/F4
production activation evidence remain external prerequisites.

M5 working statement (2026-08-22): implement the complete nonactivated source
contract for reviewed local, remote, and dynamic TCP forwarding, make its
session-scoped lifecycle understandable in the Hub, and ratchet the hermetic
native/release evidence protocol. Production activation, package attestation,
installing an SSH server, changing user SSH configuration, secret custody,
embedded SSH, UDP/Unix-socket forwarding, remote dynamic SOCKS, and claiming
unexecuted native evidence are out of scope. Acceptance requires exact typed
argv with no shell, loopback defaults, stronger review for remote/non-loopback/
production use, endpoint-bound fingerprints, fail-closed listener state and
cleanup ownership, deterministic fake/native-harness contract tests, and an
honest external gate for every native run that cannot execute locally.

Evidence ledger before implementation:

| Item | Classification | Existing owner/evidence | Missing exit proof/action |
|---|---|---|---|
| Provider-neutral local/remote/dynamic schema, 32-tunnel ceiling, session lifetime, profile validation, and fingerprint participation | **Partially done** | `automexia-devops::connections::{model,validation,planner}` plus connection-planning fixtures | Require exact IP/host endpoint grammar, distinguish local and remote listener collision domains, derive transport/risk/owner, and preserve endpoint changes in both plan and review fingerprints. |
| Managed OpenSSH tunnel request | **Not done** | M4 `direct_openssh` rejects every profile containing a tunnel and uses `ClearAllForwardings=yes` | Add an exact `-F none` typed-direct grammar with `-L`/`-R`/`-D`, `ExitOnForwardFailure=yes`, compression/agent/X11/command/multiplex/TUN disabled, and no configuration-derived forwarding. Preserve the existing no-tunnel grammar; reject config-dependent aliases/jumps with tunnels. |
| Loopback default and stronger confirmation | **Partially done** | `TunnelDefinitionV1::is_loopback`, production/non-loopback planner warnings, generic NetworkListener risk | Canonicalize the safe default, require strong per-use review for remote, non-loopback, or production tunnels, expose redundant text/icon/color meaning, and bind the decision to exact endpoints. |
| Listener lifecycle, collision, readiness, cancellation, and closure | **Not done** | ContextManager is the sole process/PTY/route owner; the OpenSSH child would own its forwarding sockets | Add a bounded generation/session-scoped pure lifecycle snapshot. Never open a competing application listener or infer readiness from terminal text; accept only owner events, reject stale/terminal reversals, and close every nonterminal tunnel when its route lease ends. |
| Hub and session-detail projection | **Partially done** | Generic Hub review counts tunnels and blocks non-loopback review; M4 review has no tunnel cards/status | Project exact public endpoint direction, risk/confirmation, OpenSSH ownership, and planned/starting/ready/collision/cancelled/failed/closed state with compact and accessible layouts. Production remains visibly protected. |
| Deterministic fake/mock evidence | **Partially done** | Schema-4 matrix, exact-argv broker mutations, lifecycle/receipt/reconnect tests | Ratchet schema 5, add tunnel grammar/lifecycle/collision/staleness/cleanup mutations, and validate a bounded redacted native result manifest. |
| Real system OpenSSH and 1/10/50 native evidence | **External prerequisite** | Four-platform scenario definition exists; CI has native Rust jobs | Execute the hermetic loopback fixture on controlled Windows/macOS/Linux runners with `ssh`, `ssh-add`, `ssh-keygen`, and `sshd`; WSL remains separately denied. This local Windows host has OpenSSH 9.5 client tools but no `sshd`, so it cannot honestly produce server/tunnel evidence and M5 cannot install the server without separate elevated authority. |
| Disable/uninstall/manual-SSH preservation | **Partially done** | Managed activation is const-asserted false and ordinary shell lookup is untouched | Add source/policy/native assertions that the harness is opt-in, never runs at startup, never changes SSH files/services, and records before/after manual-client evidence on controlled runners. |

Build/wrap/adopt decision: keep Automexia's pure validation, review,
fingerprinting, lifecycle, and renderer-neutral presentation owners; wrap the
installed system OpenSSH client with exact argument arrays; adopt OpenSSH's
forwarding/listener implementation and `ExitOnForwardFailure` instead of adding
an embedded SSH stack or competing sockets. Upstream OpenSSH documents that
`ClearAllForwardings=yes` clears command-line forwards too, while `-F none`
loads no configuration files. Therefore tunnel requests use a separate,
configuration-free typed-direct grammar; config-dependent aliases and jump
routes with tunnels fail closed until an equally exact effective-configuration
authority is designed. This preserves M4 compatibility and prevents hidden
configuration forwards from escaping review.

Lifecycle and rollback: the reviewed request owns no socket. The application
route/session lease owns one bounded tunnel snapshot, and the OpenSSH child owns
all actual listeners. Cancellation, child exit, route close, publication
failure, or shutdown closes the child and transitions every remaining entry to
a terminal state before lease release. Removing the M5 preparation/projection
path restores the existing M4 no-tunnel behavior; schema-1 profile documents
remain readable and no migration, credential, SSH-file, service, or startup
change is introduced.

Test/evidence ladder: write exact grammar and hostile endpoint tests first;
then lifecycle model, stale session/generation, duplicate/collision-domain,
review-fingerprint, Hub compact/tiny-to-8K/accessibility, broker mutation, and
bounded 1/10/50 fake-lifecycle tests. Ratchet the immutable fixture contract and
native result checker, run every locally available OpenSSH prerequisite check,
then the focused crate/app gates, architecture/policy/assurance checks,
`cargo ready`, and the user-required full command set. Controlled real OpenSSH,
screen-reader, and three-OS resource runs remain visibly external until their
redacted manifests validate.

- [x] **Fully done locally; nonactivated:** validated local, remote, and dynamic
  descriptors expose exact listen/target endpoints, transport, session lifetime,
  risk/confirmation, OpenSSH listener ownership, and the exact forwarding arg.
- [x] **Fully done locally; nonactivated:** bind defaults canonicalize to
  `127.0.0.1`. Remote, non-loopback, production, or changed endpoints require a
  fresh endpoint-bound strong Allow-once decision; session grants are disabled.
- [x] **Fully done locally; nonactivated:** the bounded owner-event lifecycle
  distinguishes planned/starting/ready/collision/failed/cancelled/closed, rejects
  stale scope and terminal reversal, projects compact redundant icon/color/text
  state into the Hub, and terminalizes every nonterminal entry on lease close.
- [ ] **Partially done locally / external native remainder:** exact fake
  preparation/argv/parser, hostile endpoint, collision, staleness, lifecycle,
  cleanup, decision, UI, and 1/10/50 pure-model tests pass. Active schema 5 and
  the bounded redacted 23-scenario manifest validator reject synthetic release
  claims. Execute the real system OpenSSH matrix for host keys, encrypted keys,
  agents, certificates, jumps, all tunnels, cancellation at DNS/connect/auth,
  offline, hostile output, exit status, and cleanup on Windows/macOS/Linux. WSL
  remains separately denied until its own native outcome passes.
- [ ] **Partially done locally / external native remainder:** deterministic
  1/10/50 session/generation/tunnel isolation and cleanup invariants pass;
  activation stays false and the explicit prerequisite probe performs no install,
  service, network, or SSH-file mutation. Controlled CPU/memory/socket/handle/
  task/route/cache/log/storage measurements and before/after enable, disable,
  uninstall, manual-SSH, and generic-terminal baselines remain external.

Exit remains unavailable: reviewed tunnel preparation is source-complete and
nonactivated, but every declared native platform still requires a validated
private real-evidence manifest and leak-free repeated lifecycle run.

### M6 — F6 typed recipes, remote initialization, and declarative workspaces

Status: Partially done; the bounded review-only source contracts are complete
locally, while product activation and native execution evidence remain gated.

- [x] **Fully done locally:** Connection Library schema 2 extends the existing
  private store with declarative workspaces, strict recipe/profile/workspace
  bindings, preview-first entity edits, atomic dependent revision/fingerprint
  updates, approval invalidation, schema-1 migration preview, CAS conflict and
  recovery handling, and redacted topology-only import/export with fresh IDs.
- [x] **Fully done locally:** recipes resolve deterministically in the exact
  ten-stage `ExecutionStage::ORDER`. A second review boundary revalidates every
  resolved action, stage, risk, confirmation, retry, and timeout before emitting
  a nonexecuting run fingerprint.
- [x] **Fully done locally:** typed remote initialization contains only working
  directory, public environment, `sudo`/`doas` user switch, and verification for
  explicit POSIX-sh or PowerShell dialects. Privilege requires per-connection
  confirmation. No arbitrary script/template, prompt inference, hidden key,
  command string, or implicit Enter contract exists.
- [x] **Fully done locally:** per-step deadline, cancellation, monotonic failure,
  capped eligibility-checked retry with deterministic jitter input, shutdown,
  reconnect-generation invalidation, and reviewed no-hooks recovery are pure
  state contracts.
- [x] **Fully done locally:** declarative layouts cap 256 workspaces, 16 windows,
  64 panes, 128 connection bindings, and 32 recipe bindings. Clone/rebind creates
  isolated IDs/revisions; restore is review-only with automatic reconnect and
  interrupted-action resume false.
- [x] **Fully done locally:** broadcast caps 50 targets, 8 KiB command text, and
  60 seconds of arming. Exact transient preview, explicit arming, separate
  production confirmation, per-target isolation/results, cancellation,
  generation rejection, digest-only audit, redacted debug, and no implicit Enter
  pass, including 1,000 repeated maximum-target generations.
- [x] **Fully done locally:** CP3.3 trusted local workspace task bridges remain
  insert-only and have no M6 remote/provider/broadcast authority.
- [x] **Fully done locally:** clone/rebind, pane graph/cross-window isolation,
  hostile/oversized input, stale profile and recipe fingerprints, privileged
  review, retry/cancel/shutdown generations, migration/recovery/rollback, focus
  restoration, semantic alert/switch/textbox accessibility, fuzz entry points,
  mutation checks, and maximum-cardinality benchmarks have local evidence.
- [ ] **Partially done / external prerequisite:** the library editor, workspace
  restore, and broadcast have model/application APIs and renderer-neutral
  projections but no activated product controller/renderer or public CLI.
  Proposed ADR 0023 must be accepted, and ADR 0012/D3/M5 protected activation,
  real OpenSSH/PTY/process/handle/socket cleanup, native Windows/macOS/Linux,
  controlled screen-reader/visual, and hosted release evidence must pass before
  execution is enabled.

Exit remains unavailable for the shipped product: saved workflows are now
reviewable and recoverable at the internal source boundary without custom remote
code or automatic persistent change, but no M6 product action can execute.

### M7 — F7/D6.0 provider-neutral auth and capsule orchestration

Status: **Fully done locally** at the provider-neutral framework boundary.
D6.1-D6.4 and D6.5 Teleport source adapters are complete and nonactivated;
controlled real official-CLI/native evidence and OpenBao remain separate
external/not-done gates.

- [x] **Fully done locally** - Freeze strict, bounded schemas for provider
  identity/context, authentication observation, immutable capsule template,
  freshness, provenance, risk, capability request, exact operation, isolation,
  browser policy, recovery action, receipt, and redacted audit. Every public
  document ingress rejects unknown fields, malformed semantics, hostile text,
  and documents above 16 MiB.
- [x] **Fully done locally** - Bind each session to one unique nonzero capsule
  ID/revision and public pinned provider context. Rebind creates a fresh
  capsule/session, cancels active work, and rejects sibling/cross-session reads
  and stale generation publication.
- [x] **Fully done locally** - Model 19 truthful states including available,
  refreshing, authenticating, MFA/browser/device pending, ready, expired,
  offline, denied, unsupported, cancelled, stale, and error. Offline, expiry,
  and failed refresh keep bounded last-known-good public context.
- [x] **Fully done locally** - Require an exact visible M2 review and current
  `AllowOnce` decisions for the executable, operation, session, capsule
  revision, process, and applicable network request. The review binds ordered
  arguments, the exact process/network-only capability list, isolation,
  browser flow/origins/callback, and risk. M7 itself cannot launch a process,
  open a network/browser callback, or access credentials; later adapters may
  request an official CLI flow through the existing runner only. The CLI
  continues to own browser/device/WAM/MFA, tokens, certificates, and provider
  caches.
- [x] **Fully done locally** - Implement explicit begin-refresh,
  begin-authentication, publish, expiry, cancellation, revocation,
  disable/uninstall, rebind, and shutdown behavior with fixed capacity,
  generation rejection, truthful initial freshness, recovery actions, and
  cached-only UI projections. Publication fully validates a candidate and
  atomically accepts only the capsule-pinned configuration/provider/risk tuple.
- [x] **Fully done locally** - Add fake exact-argv/capability review contracts
  and canaries proving public observation, receipt, audit, debug, snapshots,
  QA/policy inputs, clipboard/telemetry/AI-facing serializations contain no
  token, browser code, secret flag, origin, or command argument.
- [x] **Fully done locally** - Reject Automexia-managed global context mutation,
  including `az account set`, `gcloud config set`, configuration activation,
  `gcloud init`, `kubectl`/`oc config use-context`, kubeconfig setters, and
  `aws configure set`. Accept only exact arguments, scoped public environment
  names, or a reviewed private transient configuration reference.

Exit is proven at the pure local boundary by two-session isolation, exact
capability binding, stale-result rejection, 16 repeated maximum 64-capsule
lifecycle cycles, a mutation-enforced authority-free source contract, fuzz
registration, and a 64-capsule Criterion target. No real provider CLI,
browser/device login, cloud network, credential cache, provider configuration
write, or native provider adapter ran in M7; those claims begin only in the
independently gated D6.1-D6.5 slices.

### M8 — F8 AWS slice

Status: **Partially done overall; source-complete locally and nonactivated.**
Kubernetes output ingestion still depends on M11, and real official-tool/native
release evidence remains external.

- [x] **Fully done locally** - `automexia-devops-aws` is an independent,
  disabled-by-default extension with exact process/network capabilities. Its
  1 MiB parser reads only named profile plus public region/account/role/source
  hints from supplied granted bytes. Credential, token, `credential_process`,
  web-identity file, SSO cache, unknown, and endpoint fields are never retained;
  duplicate profiles, hostile text, invalid UTF-8, oversize input, and more than
  128 profiles fail closed.
- [x] **Fully done locally** - Exact M7 operations cover AWS CLI IAM Identity
  Center PKCE and deliberate device-code login plus regional STS
  `get-caller-identity`. Profile, region, account/role, provenance, freshness,
  risk, configuration reference, browser origin, executable, ordered arguments,
  network host, session, capsule, revision, timeout, and allow-once capabilities
  are bound. The strict 64 KiB STS decoder accepts only public caller identity
  and never serializes credential material.
- [x] **Fully done locally** - The exact SSM plan names `aws`,
  `session-manager-plugin`, target, profile, region, production risk,
  interactive PTY ownership, process-tree cancellation, and the corresponding
  provider-neutral transport. Source execution remains false so it cannot
  bypass D3 protected activation, actual executable attestation, or application
  cleanup ownership.
- [x] **Fully done locally** - EKS produces only a reviewed `--dry-run` intent
  for M11 private transient ingestion. It never names or merges the user
  kubeconfig and never changes current context.
- [x] **Fully done locally** - Nine deterministic tests cover public profile
  extraction, credential canaries, duplicate/oversize rejection, PKCE/device
  selection, exact STS args/capabilities, strict output, explicit AWS CLI 2.22+
  PKCE and Session Manager plugin 1.1.17+ floors, truthful failure states,
  production risk, capsule isolation, SSM plugin/PTY/tree cleanup, EKS no-write,
  redacted debug/JSON, and disabled least-privilege manifest. Locked tests,
  warning-denied Clippy, and formatting pass on Windows x86_64.
- [ ] **External/blocked** - Wire the product provider controller/review UI to
  the protected application runner after D3 activation, bind native executable
  identity and real descendant cleanup, ingest EKS output through completed M11,
  and run controlled IAM Identity Center PKCE/device/MFA, STS, SSM, EKS,
  offline/denied/cancel/revoke/uninstall, Windows/macOS/Linux, resource,
  accessibility, packaging, signing, and release fixtures.

Exit is met for the capability-free AWS source contract and independent package,
not for product activation or release. No AWS CLI, browser/device login, AWS
network, credential cache, Session Manager session, or EKS cluster ran in this
slice.
### M9 — F9 Azure slice

Status: Partially done overall; source-complete and nonactivated on Windows
x86_64, with product activation, AKS ingestion, and external evidence remaining.

- [x] **Fully done locally:** add independent `automexia-devops-azure`, disabled
  by default with only exact process/network capability declarations and a
  separately registered built-in manifest.
- [x] **Fully done locally:** parse at most 256 KiB, 128 public Azure account
  records, 4,096 JSON nodes, depth 32, and 4 KiB public fields from exact
  caller-supplied `az account list --output json` bytes. Normalize GUIDs, reject
  duplicates/hostile text/secret-token keys, and retain only subscription,
  tenant, cloud, state, default flag, public identity, and identity kind.
- [x] **Fully done locally:** construct exact capsule/session/revision-bound
  `az login --tenant ... --output none` system-broker, browser, and device-code operations
  plus `az account show --subscription ... --output json`. No operation uses
  hidden `az account set`, shell evaluation, inherited environment isolation,
  password/client-secret flags, or token/cache reads.
- [x] **Fully done locally:** pin tenant, subscription, cloud, public identity,
  state, provenance, freshness, risk, exact executable/argv, Microsoft endpoint,
  browser policy, timeout, and one-time process/network capability scopes.
  Missing/expired/MFA/cancel/offline/denied/unsupported/error stay non-ready.
- [x] **Fully done locally:** build an immutable AAD-only `az network bastion
  ssh` plan bound to the capsule subscription and target resource ID. It records
  interactive PTY, official-CLI child-SSH possibility, whole-tree cancellation,
  risk, and `execution_enabled = false`.
- [x] **Fully done locally:** build AKS credentials as an opaque private-output
  intent split around `--file`; only M11 may resolve the private transient path.
  No user kubeconfig/current context/path is named or mutated.
- [x] **Fully done locally:** eight parser/secret/complexity/argv/capability/
  isolation/Bastion/AKS/version/failure/redaction tests, app registration,
  warning-denied Clippy, formatting, architecture/identity, and repository
  policy gates pass on Windows x86_64. The 128-account Criterion target measured
  473.69–478.86 µs over 100 samples; eight high-side outliers were reported.
- [ ] **Partially done/external:** connect the source contract to the protected
  D3 product review/runner only after activation/attestation authority exists;
  M11 must allocate, validate, publish, revoke, and clean the AKS transient file.
- [ ] **External prerequisite:** run controlled real Azure CLI 2.61+ WAM/browser/device/
  MFA/conditional-access/cancel/offline cases, Azure Bastion native client (CLI
  floor 2.32+), AKS, Windows/macOS/Linux process-tree/resource/accessibility,
  packaging, signing, and release fixtures.

Exit is met for the independent Azure source contract, not for product
activation or release. No Azure CLI, WAM/browser/device flow, Microsoft network,
token cache, Bastion connection, AKS cluster, PTY, or provider filesystem ran in
this slice.

### M10 — F10 Google Cloud slice

Status: Partially done overall; source-complete and nonactivated on Windows
x86_64, with product activation, GKE ingestion, and external evidence remaining.

- [x] **Fully done locally:** add independent `automexia-devops-gcp`, disabled by
  default with exact process/network declarations and separate app registration.
- [x] **Fully done locally:** parse one exact named configuration with 256 KiB,
  64-section, 512-entry, and 4 KiB-field limits. Retain only account/project/
  region/zone, reject duplicates/invalid UTF-8/hostile values and credential,
  token, secret, password, private-key, login-config, or token-file keys.
- [x] **Fully done locally:** pin configuration/account/project/region/zone,
  provenance/freshness/risk and use `--configuration` on every exact operation.
  User browser/remote bootstrap remains gcloud-owned; public project observation
  contacts only Cloud Resource Manager. No global configuration activation,
  `config set`, ADC update, token output, shell evaluation, or ambient env switch.
- [x] **Fully done locally:** represent Workforce `--login-config` and Workload
  `--cred-file` only as opaque private configuration references in immutable,
  nonexecuting intents. The adapter never reads or serializes their file paths,
  external-account content, tokens, or service-account private keys.
- [x] **Fully done locally:** build project/zone/configuration-bound IAP SSH with
  `--tunnel-through-iap`, interactive PTY, process-tree cancellation, and
  gcloud-owned SSH-key/OS Login behavior. Execution remains false behind D3.
- [x] **Fully done locally:** build GKE credentials only with an opaque M11-owned
  private `KUBECONFIG` environment reference; never name or merge user config.
- [x] **Fully done locally:** eight focused tests, app registration, warning-
  denied all-target Clippy, formatting, and a near-limit Criterion target pass on
  Windows x86_64. The benchmark measured 444.00–460.66 µs over 100 samples and
  reported 9 high-side outliers.
- [ ] **Partially done/external:** connect through protected D3 review/runner only
  after activation/attestation; M11 must allocate/validate/publish/revoke/clean
  the GKE file and approve any resulting exec credential plugin independently.
- [ ] **External prerequisite:** controlled real gcloud user browser/remote/2FA,
  Workforce/Workload federation, IAM denial/offline/cancel, IAP/OS Login, GKE,
  Windows/macOS/Linux cleanup/resources/accessibility, packaging/signing/release.

Exit is met for the independent Google Cloud source contract, not activation or
release. No gcloud process, auth flow, network, credential database, IAP/SSH,
GKE cluster, kubeconfig file, or provider PTY ran in this slice.

### M11 — F11 Kubernetes and OpenShift slice

Status: Partially done overall; source-complete and nonactivated. M7 remains the
capsule authority and D3 remains the only future execution boundary.

- [x] **Fully done locally:** treat every source as code-capable untrusted input.
  The Kubernetes package accepts only exact absolute grants or private transient
  references, uses stable bounded regular-file reads, rejects links/reparse
  points, source drift, external credential paths, documents above 1 MiB,
  depth/event/node/count excess, duplicate identities, and merge collisions.
- [x] **Fully done locally:** parse typed YAML and JSON with duplicate-key and
  merge-key denial; retain only public cluster origin, TLS policy, context,
  user reference, namespace/project, source provenance, revision, and freshness.
  Deterministic source order selects the first current context while any map-key
  collision fails closed instead of creating ambiguous identity.
- [x] **Fully done locally:** `user.exec` is `DenyAll`. Public review retains only
  API version, executable, ordered arguments, environment names, and
  interactivity. Environment values, tokens, passwords, certificates, and key
  data are discarded; secret-bearing flags fail closed. A nonactivated exact
  digest/argv/environment/interactivity/session/revision/deadline/output/tree-
  cancellation review exists but cannot execute.
- [x] **Fully done locally:** capsules pin source-set revision, context, cluster,
  user reference, namespace/project, server origin, TLS policy, provider
  relation (standalone/EKS/AKS/GKE/OpenShift/Teleport), freshness/expiry,
  provenance, and risk. Production rejects insecure TLS.
- [x] **Fully done locally:** immutable nonactivated plans cover `kubectl auth
  whoami`, context inspection, `kubectl exec`, OpenShift web login into a newly
  allocated private output, project inspection, and `oc rsh`. They use exact
  argument arrays, private `KUBECONFIG`, whole-tree cancellation, and never run
  global `use-context` or project mutation.
- [x] **Fully done locally:** eight Kubernetes and five OpenShift tests cover
  hostile YAML/JSON, controls/bidi, relative/linked/changed/oversized sources,
  merge order/collisions, AWS/AKS/GKE transient inputs, exec denial/exact review,
  redaction, session isolation, explicit expiry/offline/denial/plugin failure,
  disable registration, and exact argv. Warning-denied Clippy, dependency policy,
  and the 900 KiB Windows benchmark at 1.8280–1.8788 ms pass.
- [ ] **Partially done/external:** protected D3 product activation, actual private
  transient-file allocation/cleanup, real `kubectl`/`oc` and EKS/AKS/GKE/
  OpenShift fixtures, plugin execution, forced child teardown, sustained
  resource/storage evidence, Linux/macOS native clients, product UI,
  accessibility, packaging, signing, and release evidence remain.

Exit is met for the two independently disabled source packages, not product
activation or release. No Kubernetes/OpenShift executable, plugin, network,
credential, cluster, browser, PTY, or user kubeconfig ran in this slice.
### M12 — F12 organization identity: Teleport, then OpenBao

Status: Partially done overall. Teleport is source-complete and nonactivated;
OpenBao is not implemented and remains an external prerequisite pending ADR
0024 acceptance.

#### M12.1 Teleport

- [x] **Fully done locally:** a separate disabled Teleport extension uses the
  reviewed exact `tsh version --client`, `tsh login`, `tsh status --client
  --format=json`, `tsh ssh`, and `tsh logout` contracts. Teleport’s cache,
  certificates, browser, MFA, and expiry remain authoritative; agent addition
  and ambient Teleport environment overrides are denied.
- [x] **Fully done locally:** bounded public status retains only exact proxy,
  cluster, user, roles, logins, Kubernetes hints, freshness/expiry, and
  provenance. It never reads `~/.tsh`, certificate contents, tokens, identity
  files, agent material, or inherited provider state.
- [x] **Fully done locally:** deterministic contracts cover bytes/nodes/depth/
  profile totals, benign-field discard, sensitive/major-version drift, expiry,
  revocation, offline/cancelled/MFA states,
  proxy/target/session/revision drift, exact argv, relogin/access-request denial,
  redaction, disable/uninstall, and bounded process-tree intent.
- [ ] **External:** D3 product activation/attestation plus real `tsh`, proxy,
  browser/MFA, cache/certificate/agent, PTY, native cleanup/resources,
  accessibility, packaging, signing, and release fixtures remain.

#### M12.2 OpenBao SSH certificates

- [ ] **External prerequisite/not done:** obtain acceptance of proposed ADR 0024
  for OpenBao’s token-helper and certificate-file boundary before any
  implementation. Teleport or generic OpenSSH approval does not cover it.
- [ ] Keep token/helper interaction external; retain only opaque references and
  bounded public certificate metadata. Do not accept password/private-key
  material or serialize certificate contents as an Automexia credential store.
- [ ] Give it an independent capability grant, cache, revocation, certificate
  expiry/cleanup, native fixtures, recovery/uninstall behavior, and docs.

Exit is met only for Teleport’s independently disabled source package. Product
activation and native release evidence remain external. The combined M12 exit
is not met because OpenBao is intentionally absent until ADR 0024 is accepted;
neither organization adapter may grant authority to the other.

### M13 — F13/CP4 provider-aware Quick Actions

Status: Not done and dependency-blocked by M2–M12.

- [ ] Extend Quick Action candidates only from bounded cached public capsules:
  target, provider/account/subscription/project, cluster/context/namespace,
  region/zone, infrastructure reference, freshness/provenance, risk, and state.
- [ ] Show exact target and stale/missing/expired/offline/production signals in
  search and review. Cancel stale query generations and preserve pane/session/
  provider/workspace isolation.
- [ ] Preserve insert-without-Enter as the fallback. Exact action execution must
  use an M2 reviewed grant and each provider slice’s approved grammar.
- [ ] Add actions one provider at a time with fake/native tests, cancellation,
  revocation, audit/redaction, accessibility, disable/uninstall, and explicit
  no-provider-work-on-keystroke proof.

Exit: provider-aware productivity improves discovery without widening provider,
credential, or session authority.

## 5. Research and compatibility decisions to preserve

The plan was checked against current primary documentation on 2026-08-20. These
sources support the wrap-first design and must be rechecked when an adapter is
implemented:

| Area | Current primary-source finding | Delivery consequence |
|---|---|---|
| OpenSSH | [`ssh_config(5)`](https://man.openbsd.org/ssh_config) documents `ForwardAgent` as default-off and cautions that remote users may use the local agent. | Keep forwarding default-off and separately reviewed; let OpenSSH, not D4 parsing, interpret live configuration. |
| AWS | [AWS CLI IAM Identity Center](https://docs.aws.amazon.com/cli/latest/userguide/cli-configure-sso.html) defines named `sso-session`/profile configuration and CLI-owned token flow; [EKS](https://docs.aws.amazon.com/eks/latest/userguide/create-kubeconfig.html) notes default kubeconfig merge behavior. | Read public config only; use visible CLI auth; defer EKS credential/context execution to M11 and avoid default file mutation. |
| Azure | [Azure CLI interactive auth](https://learn.microsoft.com/en-us/cli/azure/authenticate-azure-cli-interactively) uses WAM/browser/device flows; [AKS CLI](https://learn.microsoft.com/en-us/cli/azure/aks) documents default credential merge behavior. | Never capture auth; do not use `az account set` or default AKS merge as a pane switch. |
| Google Cloud | [GKE access](https://cloud.google.com/kubernetes-engine/docs/troubleshooting/introduction-kubectl) uses `gcloud container clusters get-credentials`. | Research scoped configuration/credential handling before activation; M11 must own generated kubeconfig semantics. |
| Kubernetes | [Kubernetes authentication](https://kubernetes.io/docs/reference/access-authn-authz/authentication/) states exec commands can be relative to their config and return an `ExecCredential`; [multi-cluster configuration](https://kubernetes.io/docs/tasks/access-application-cluster/configure-access-multiple-clusters/) defines merged kubeconfig use. | Treat kubeconfig and exec plugins as untrusted/capable input; exact-source grants and default deny are mandatory. |
| Teleport | [`tsh`](https://goteleport.com/docs/connect-your-client/teleport-clients/tsh/) stores certificates/cache externally and can use an SSH agent. | Ask `tsh` for public status through review; never import its cache or agent data. |
| OpenBao | [signed SSH certificates](https://openbao.org/docs/secrets/ssh/signed-ssh-certificates/) warns against providing private-key passwords to OpenBao. | Require a separate token-helper/certificate-file review and retain opaque references only. |

No new runtime dependency is approved by this research. Before each provider
slice, record exact CLI versions and supported operating systems/architectures,
license/advisory status, installer/upgrade ownership, tool discovery policy,
offline behavior, and uninstall/rollback behavior.

## 6. Test, performance, security, UX, and documentation gates

### Tests to write before or with each production increment

- Pure-unit/property tests for schema versions, limits, malformed/hostile text,
  stable ordering, fingerprint invalidation, policy denials, and redaction.
- Deterministic fake-process/CLI/server tests for exact argv, accepted public
  output, deadlines, cancellation checkpoints, retries, stale generation,
  replay, package/executable replacement, and cleanup. Use readiness signals,
  not arbitrary sleeps.
- Integration tests at the application-to-PTY/route, metadata/store, provider
  config, and disabled/uninstall boundaries.
- Fuzz targets/corpus regressions for SSH config, imports, provider config,
  kubeconfig, structured output, and protocol framing as applicable.
- Renderer-neutral goldens before native GUI automation; then inspect real
  screen artifacts for every visible change.

### Evidence ladder for an implementation change

Run the smallest relevant checks after each meaningful increment, then expand:

```text
cargo fmt --all --check
cargo test -p automexia-devops-ssh --locked
cargo test -p automexia-ui-model --locked
cargo test -p automexia-terminal --test connection_hub_runtime --locked
cargo test -p automexia-terminal --test connection_library --locked
python tools/ci/check_session_launch_d0.py
python tools/ci/test_session_launch_d0.py
cargo clippy -p <changed-owner> --all-targets --all-features --locked -- -D warnings
python tools/ci/validate_repository.py
cargo xtask verify architecture
git diff --check
cargo ready
```

Add the relevant fuzz, benchmark, native process/PTY, visual, accessibility,
package, and platform commands in the feature’s own test plan. `cargo dev` is
not a test command. Do not claim a platform or screen reader was verified when
that native environment was unavailable.

### Required quality checks for every phase

- Threat-model malformed/oversized/unicode/bidi/control input; source changes,
  permissions/links, disk-full/read-only/corrupt state; queues/retries/output;
  cancellation/rebind/shutdown; process trees/listeners; and cross-pane/window
  confusion before selecting an API.
- Define hard limits for bytes, records, recursion, source files, queue depth,
  output, deadlines, retries, cache/storage retention, concurrent operations,
  and log/audit fields before implementation.
- Measure same-host baselines for hot-path changes: input/render latency, Hub
  filter/projection, launch-to-prompt, cancellation, 1/10/50 sessions, CPU,
  memory, process/handle/socket/route count, and cleanup after repeated cycles.
- Inspect the complete diff and re-audit source/test/docs ownership after each
  late fix. Search for duplicate authority, process or shell APIs outside the
  app runner, secret-bearing log fields, unbounded work, TODO stubs, and stale
  implementation claims.
- Update the roadmap phase status, phase audit, feature assurance/test docs,
  user guide/recovery/disable/uninstall guide, architecture/ADR/reference docs,
  navigation, and `changes/` fragment whenever behavior changes.

## 7. Commit, push, and handoff policy

When implementation is explicitly authorized, group only one coherent behavior,
its deterministic tests, and its documentation in a commit. Preserve unrelated
worktree changes; inspect status, complete diff, and staged diff; sign commits
with DCO; push the feature branch without force; and verify the remote commit.

Suggested commit boundaries:

1. `docs(connectivity): add evidence-led SSH and multicloud delivery plan`
2. `feat(connection-hub): wire read-only inventory modal`
3. `feat(connection-hub): add reviewed public metadata editing`
4. `feat(session-launch): add application-owned exact runner`
5. `feat(ssh): add reviewed direct OpenSSH sessions`
6. `feat(ssh): add reviewed routes and tunnels`
7. `feat(workspaces): add typed remote workspace orchestration`
8. One commit per provider adapter and one per provider-aware Quick Action slice.

Every final handoff must name changed files, proven acceptance criteria, exact
test results, benchmark/native/visual evidence, documentation updates, commit
SHA/branch/push verification when authorized, and outstanding external gates.
