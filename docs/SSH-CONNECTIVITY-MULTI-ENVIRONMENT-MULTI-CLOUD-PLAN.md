# SSH, connectivity, multi-environment, and multi-cloud implementation plan

Status: authoritative detailed implementation plan and evidence ledger.

Last reconciled: 2026-08-21.

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
| D0 SSH trust/fixture contract | Partially done | Proposed ADR 0012; schema-2 D0/D3 fixture; mutation checks; four-platform native matrix definition | Protected security review and two protected-path approvals; execute later native fixture matrix. |
| D1 contracts and D2 immutable capsules | Fully done locally | `automexia-extension-api`, runtime, DevOps model and architecture checks | Preserve while adding real login/launch; hosted release assurance remains separate. |
| D3 launch broker | Partially done | `context/launch_broker.rs` has package, capability, resolver, lease, audit and pure lifecycle tests | It is exclusively `#[cfg(test)]`, has a constant production denial, and cannot spawn/attach a PTY. Implement F4 only after ADR gate. |
| D4 static OpenSSH inventory | Fully done locally | `extensions/devops-ssh`: bounded parser, grants, hostile/property/fuzz tests, 10,000-alias benchmark, private revisioned metadata | Deliberately remains non-executing; hosted macOS/longitudinal release proof remains external. |
| F2/D5.0 Hub and planning model | Partially done overall; fully done locally | Pure records, validation, reducers, fingerprints, review/planner projection, accessibility goldens, fuzz and benchmark | ADR 0012 acceptance blocks phase closure; no product authority is granted. |
| F3/D5.1 catalog composition | Partially done | `ConnectionHubRuntime` composes explicit D4 grants and metadata in one cancellable worker; stale last-known-good catalog and setup guidance are tested | No application screen/controller or user-selected-grant flow; no metadata mutation UI; no product visual/accessibility/native evidence. |
| F3 Connection Library | Fully done locally | `ConnectionLibraryStore` has 16 MiB private documents, CAS, recovery, transfer redaction, fresh import IDs, read-only/disk-full and link tests | Product editor/manager belongs to F6; macOS/Linux native permission and controlled screen-reader evidence remain. |
| F5 managed OpenSSH | Not done | Only disabled D3/D4/F2 foundations | No reviewed launch, OpenSSH child, host-trust UX, routes/tunnels, lifecycle, or native sessions. |
| F6 recipes and remote declarative workspaces | Not done | Typed pure profile/recipe models and F3 private persistence are reusable; CP3.3 local task bridges are separately complete | No compiler/runtime/editor/remote shell contract/layout restoration/broadcast. |
| D6 provider-neutral authentication and capsule orchestration | Not done | Public provider/transport model variants and legacy display context exist | No visible official-CLI auth, isolated session pinning, cache/provenance/revocation, or provider adapter. |
| AWS, Azure, Google Cloud, Kubernetes/OpenShift, Teleport, OpenBao | Not done | No corresponding extension directory or application adapter exists; there are only typed model variants and insert-only Quick Action packs | Implement and release each adapter independently after F4/F7 prerequisites. |
| Provider-aware Quick Actions (CP4/F13) | Not done and blocked | CP2/CP3 typed/persistent/insert-only Quick Actions and static packs exist | Consume only F7+ cached public context; exact execution stays behind F4. |

### Important distinction: existing legacy DevOps status is not D6

`automexia-devops::context` and the application DevOps runtime currently derive
best-effort labels from local files/environment and, for Windows WSL sessions,
a fixed short-lived probe that can invoke installed CLIs. It provides passive
status display, not verified authentication, provider isolation, capability
review, or a session-pinned capsule. Do not build new provider functionality on
that ambient mechanism and do not call it evidence for D6.

Before F7 enables any provider adapter, characterize every caller and make the
following migration explicit:

- [ ] Keep only bounded, local, public, non-secret information needed for the
  legacy status surface, or replace it with an explicit cached observation.
- [ ] Remove/provider-gate any automatic CLI invocation and any `sh -c` WSL
  probe from provider discovery. Provider commands must move behind the F4
  runner and a visible, reviewed explicit refresh.
- [ ] Stop treating process environment or a global CLI “active” selection as
  authoritative context for a managed pane; use an immutable capsule snapshot.
- [ ] Add regression tests proving status refresh, Hub open, filtering, and
  keystrokes never launch `aws`, `az`, `gcloud`, `kubectl`, `oc`, `tsh`, or
  another provider tool.

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
- [x] **Fully done (preserved)** — Keep the D0/D3 statement exact: the broker remains test-only and managed
  launch is unavailable until ADR 0012 is accepted or superseded.
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

Status: Partially done; the next executable product phase.

#### M1.1 App-owned lifetime, configuration, and explicit scan

- [ ] Locate the application configuration/state-root owner and initialize
  `MetadataStore` and `ConnectionLibraryStore` there once per application
  lifecycle; surface private-store recovery as a redacted actionable state.
- [ ] Add one joined/cancellable Hub supervisor. The current runtime worker
  drops through a weak reference but has no product-level ownership/join
  contract; add explicit shutdown/cancellation/join behavior before wiring a
  screen.
- [ ] Add an `Open Connection Hub` action/command-palette entry and a
  window-level modal controller. Opening the Hub must read only the already
  opened private state and must not scan, launch, authenticate, connect, or
  resize a PTY.
- [ ] Implement user-controlled “choose exact files” flow. It must canonicalize
  and review each selected local config file before creating an `InventoryGrant`;
  never silently read candidate locations, follow links/reparse points, or use
  a remembered path as a new grant.
- [ ] Do not persist raw source paths merely for convenience. Initial F3 can ask
  for re-selection after restart; any later persistent grant/reference needs a
  privacy, source-change, revocation, and migration design before it is added.
- [ ] Wire `request_explicit_scan` to the chosen grants only. Coalesce/revoke a
  prior scan, preserve last-known-good records, publish the snapshot before
  rendering wakeup, and expose only stable redacted diagnostic codes.

#### M1.2 Read-only UX and metadata changes

- [ ] Adapt the existing pure Hub catalog/view model into the real modal:
  virtualized rows, local cancellable search, filters, grouping, source
  revision/freshness, selection, inspector/detail route, focus trap/restore,
  and empty/filtered/loading/stale/error/setup states.
- [ ] Implement truthful platform-specific setup guidance for Windows, macOS,
  and Linux. It may show candidate locations but must state that exact selection
  is required and that no process/network/login runs.
- [ ] Add favorite and tag editing through D4 `MetadataStore` only: show a diff,
  validate public text, use revision CAS, handle contention/stale revision, and
  recompose the catalog after a successful write.
- [ ] Keep `recent` read-only until F5 records a successful managed connection.
  Do not mark a record “recent” from selection, review, scan, or failed launch.
- [ ] Expose profile/recipe/preference data only as non-executing local
  configuration in this phase. Full editor, migration UX, and remote workspace
  lifecycle remain F6 work.
- [ ] Make Connect, Login, automatic action, provider refresh, and all network/
  process actions visibly disabled with a short reason and no hidden fallback.
  Selecting a connection may open a disabled review, but never causes a PTY or
  command.

#### M1.3 M1 tests and evidence

- [ ] Add controller integration tests for explicit grants, no-scan-on-open,
  stale result rejection, worker cancellation/join, metadata CAS conflicts,
  selection/focus restoration, and disabled-action non-authority.
- [ ] Add renderer-neutral plus product UI tests for keyboard/pointer/IME,
  screen-reader names, 100–300% scale, tiny/normal/ultrawide/8K layouts,
  high contrast, reduced motion/transparency, long Unicode/bidi-safe labels,
  and modal stacking.
- [ ] Run native static permission/recovery tests on Windows, macOS, and Linux;
  perform controlled Narrator/NVDA, VoiceOver, and Orca verification where
  runners are available. Report unavailable systems honestly.
- [ ] Re-run the 10,000-record filter/projection benchmark against the same-host
  baseline and verify open/search does not affect terminal input/render latency.

Exit: a user can safely browse, diagnose, tag, and favorite a D4 inventory in
the product; no connection, provider, authentication, process, network, or PTY
authority is introduced.

### M2 — complete the D0 external decision and activate F4/D3 safely

Status: D0/F4 remain Partially done; no production code may enable launch first.

#### M2.1 Protected prerequisites

- [ ] Obtain the ADR 0012 security review and the two protected-path approvals
  required by ADR 0003, or replace it through an equally specific accepted ADR.
- [ ] Confirm all inherited S0/v0.4 hostile-output and release gates are green.
- [ ] Bind a real trusted package-loader/attestation result to the existing
  reviewed package policy: extension ID, publisher, exact version, contract
  version, non-zero digest, first-party/repository verification, and revocation
  state. A test fixture or crate name is not production proof.

#### M2.2 One application-owned `ExternalToolRunner`

- [ ] Design tests first for the actual application launch/PTY seam, then add a
  single runner in `apps/automexia-terminal`. It accepts only a prepared
  canonical executable, exact argument vector, validated cwd, bounded
  allowlisted environment values supplied by the application, explicit stdin/
  output policy, deadline, cancellation token, route destination, and lease.
- [ ] Keep broad `ProcessSpawn`, arbitrary executable, shell profile, inherited
  environment selection, raw command text, and secret references denied.
- [ ] Implement OS-specific executable identity revalidation and close the
  check-to-spawn race. Review a native mechanism for Unix (for example a
  descriptor-based execution strategy) and an explicit Windows application-path
  strategy before implementation; prove replacement resistance natively.
- [ ] Atomically bind verified package, executable identity, capability decision,
  operation lease, session, capsule revision, target/connection reference,
  argv, newly created PTY, route/pane destination, and owned tunnels before
  publication. Wake the renderer only after state is visible.
- [ ] Implement graceful cancellation, bounded forced termination, process-tree
  cleanup, listener/tunnel closure, close/rebind/shutdown reconciliation, and
  PID-reuse-safe lease completion. There must be no unbounded tombstone set.
- [ ] Preserve redacted structured audit fields only. Never record argv, cwd,
  executable path, environment values, username, PID, secrets, terminal bytes,
  or raw provider/OpenSSH diagnostics.

#### M2.3 Capability/recovery UX

- [ ] Render an accessible review/approval surface before any launch. It shows
  extension/package identity, capability, public target, route, risk, exact
  operation summary, expiration, scope, destination, and deny/allow-once/
  allow-session behavior where supported.
- [ ] Make expiry, revocation, loader verification change, stale source/capsule,
  missing executable, cancellation, and failure states distinct and recoverable
  without suggesting a security bypass.
- [ ] Keep grants session-scoped by default. Any persisted grant requires a
  separate threat model, schema/migration, explicit revocation UX, and ADR
  review; it is not part of initial F4.

#### M2.4 M2 tests and native evidence

- [ ] Add deterministic fake-runner tests for hostile/Unicode/whitespace/
  leading-dash/metacharacter/max-size argv, null stdin, output caps, duplicate/
  replay/stale/expired decisions, package/executable replacement, session/window
  isolation, cancellation, shutdown, and 1/10/50 lifecycle cycles.
- [ ] Execute the frozen loopback fixture protocol with real system OpenSSH and
  native PTYs: Windows ConPTY, macOS PTY, Linux PTY, and separately gated WSL.
  Capture the private/redacted required artifact manifest, resource peaks, and
  all six zero-resource cleanup invariants.
- [ ] Measure controlled launch-to-prompt, cancellation, input/render latency,
  CPU, memory, process/handle/descriptor/task/route counts, and repeated-close
  baseline. Fix regressions before widening capability use.

Exit: the generic broker is production-capable and independently testable, but
no provider receives a grant merely because the runner exists.

### M3 — F5.1 direct reviewed OpenSSH

Status: Not done; depends on M2.

- [ ] Add an exact reviewed request grammar for one D4 concrete alias and one
  typed literal destination. Reject option-like, empty, control/bidi, whitespace
  where disallowed, or ambiguous destination values before review.
- [ ] Build Connection Review from the pure F2 plan and show public target,
  identity reference/readiness, canonical `ssh` identity, route, strict host
  trust policy, capabilities, risk, pane/tab/window destination, and redacted
  argv shape.
- [ ] Launch system OpenSSH only through M2 into a new independent PTY. Preserve
  all prompts and diagnostics in that PTY; Automexia must not parse passwords,
  passphrases, MFA, or terminal content to decide authentication.
- [ ] Implement cancel, reconnect, stale-intent/source/capsule invalidation,
  nonintrusive notifications, redacted receipt, and independent session cleanup.
- [ ] Test manual `ssh host` with the extension disabled before/after every
  managed-path test to prove the ordinary terminal remains unchanged.

Exit: a reviewed direct SSH session is additive and isolated, with no secret
custody or alternate SSH implementation.

### M4 — F5.2 explicit routes, host trust, and identity readiness

Status: Not done; depends on M3.

- [ ] Add typed host/user/port fields and D4 config-defined `ProxyJump` chains;
  do not support free-form `-o`, `ProxyCommand`, remote command, or shell text.
- [ ] Explain first, known, and changed host-key outcomes using the full public
  fingerprint/algorithm information supplied by OpenSSH policy. Never silently
  accept, delete, replace, or truncate a `known_hosts` decision.
- [ ] Add an explicit user-owned trust handoff/recovery flow. It may open the
  managed SSH PTY or copy a reviewed user command; it cannot edit `known_hosts`
  behind the user’s back.
- [ ] Add public agent/certificate/hardware readiness checks only via separately
  reviewed status operations. Show status/fingerprint/comment/expiry where
  safely public; never read private key, agent message, FIDO secret, or token.
- [ ] Keep agent forwarding off. A later enablement is a per-connection,
  per-session, visibly reviewed, expiring grant; production routes remain
  review-only in this slice.

Exit: routes and trust changes are understandable, reviewable, and fail closed.

### M5 — F5.3 typed tunnels and M5.4 native SSH release evidence

Status: Not done; depends on M4.

- [ ] Add validated descriptors for local, remote, and dynamic forwarding with
  exact listen/target endpoints, transport, session, risk, and lifecycle owner.
- [ ] Bind listeners to loopback by default. Non-loopback, production, remote
  forwarding, or changed endpoint requires stronger confirmation and a fresh
  approval fingerprint.
- [ ] Detect listener collision/readiness without claiming a connection is ready
  prematurely. Surface ownership, state, cancellation, and closure in the Hub
  and session details; close every owned listener when its session/lease ends.
- [ ] Test fake executable/mock server behavior, then real system OpenSSH for
  host keys, encrypted keys, agents, certificates, jumps, all tunnels,
  cancellation at DNS/connect/auth, offline, hostile output, exit status, and
  cleanup on Windows/macOS/Linux. WSL remains separately denied until its own
  native outcome passes.
- [ ] Prove 1/10/50 concurrent session isolation and bounded CPU, memory,
  sockets/listeners, handles/descriptors, tasks, routes, caches, logs, and
  storage; test enable/disable/uninstall and manual SSH preservation.

Exit: reviewed SSH transport support has native evidence for every declared
platform and no process/listener/resource leak under repeated lifecycle tests.

### M6 — F6 typed recipes, remote initialization, and declarative workspaces

Status: Not done; the persistent library is complete locally but is not an
automation engine.

- [ ] Use the existing immutable Profile/Recipe documents and Connection Library
  as storage; add versioned product editor, migration/recovery/import/export
  preview, CAS conflict handling, and approval-fingerprint invalidation.
- [ ] Compile recipes deterministically through exactly: Resolve, Preflight,
  Authenticate, BeforeConnect, Connect, RemoteInitialize, Verify, Ready,
  BeforeDisconnect, Cleanup. Keep compilation pure; runtime executes only a
  reviewed, resolved plan.
- [ ] Implement typed local/session actions first. Permit remote directory,
  public environment, user switch, and verification only for an explicit,
  narrow remote-shell contract. No arbitrary script, command template, terminal
  cell inference, hidden key injection, or implicit environment export.
- [ ] Add per-step deadline, cancellation, safe failure, eligibility-checked
  idempotent retry with capped backoff/jitter, reconnect generation invalidation,
  and a `--no-hooks` recovery path.
- [ ] Persist declarative layout and **connection intent**, not running PTYs,
  credentials, live tunnel state, or interrupted destructive operations. Restore
  as a reviewed plan; never reconnect or resume destructive work automatically.
- [ ] Implement broadcast only after a prominent armed/disarmed state, exact
  command/target preview, production confirmation, per-target results, failure
  isolation, cancellation, and audit/redaction tests.
- [ ] Keep CP3.3 trusted local workspace task bridges distinct: they remain
  insert-only local actions and must never gain remote or provider authority.
- [ ] Test clone/rebind, multi-window/pane isolation, recipe cycles and hostile
  values, privilege prompts, disconnect/reconnect/shutdown, focus restoration,
  accessibility, long-session resources, recovery, and rollback.

Exit: saved SSH workflows are reviewable and recoverable without enabling custom
remote code or automatic persistent change.

### M7 — F7/D6.0 provider-neutral auth and capsule orchestration

Status: Not done; depends on M2 and must include the legacy-context migration.

- [ ] Freeze public, bounded schemas for provider identity, authentication
  observation, immutable capsule template, freshness, provenance, risk,
  capability request, operation, recovery action, receipt, and redacted audit.
- [ ] Create each new session with a unique capsule ID/revision and public pinned
  context. Rebind cancels old work; no pane/window/session can mutate or read a
  sibling capsule.
- [ ] Model states at least as missing, available, refreshing, authenticating,
  MFA/browser/device pending, ready, expired, offline, denied, unsupported,
  cancelled, stale, and error. Keep last-known-good public context truthful.
- [ ] Start an official CLI login only after visible M2 capability review. The
  official CLI owns browser, device code, WAM, MFA, tokens, certificates, and
  cache; Automexia exposes only bounded public observation and recovery.
- [ ] Add explicit refresh, expiry, cancellation, revocation, disable/uninstall,
  and offline behavior. Opening/searching a Hub or palette must consume cached
  public data only.
- [ ] Add fake CLI contracts and redaction canaries for persistence, logs,
  diagnostics, snapshots, QA bundles, clipboard, telemetry, and AI surfaces.
- [ ] Establish provider configuration isolation rules before coding any adapter:
  never use `az account set`, `gcloud config set`, `kubectl config use-context`,
  or a default-writing kubeconfig command as an Automexia-managed context switch.
  Pass exact scoped flags/config references or use a reviewed private transient
  file only when the relevant provider and Kubernetes boundaries approve it.

Exit: two concurrent provider sessions cannot contaminate identity, config,
cache, result, capability, or process authority.

### M8 — F8 AWS slice

Status: Not done; depends on M7, with Kubernetes execution also depending on M11.

- [ ] Create an independently enabled AWS extension that parses only bounded
  named profiles and public region/account/role hints from an exact user-granted
  configuration source. Do not parse or persist SSO/token/credential cache data.
- [ ] Implement visible AWS CLI IAM Identity Center login and a reviewed STS
  identity observation through M2. Pin profile, region, public account/role,
  expiry/freshness, provenance, and risk in the capsule.
- [ ] Add an exact reviewed SSM Session Manager flow, including plugin/tool
  identity, interactive PTY ownership, cancellation, child-tree cleanup, and
  public target/risk review.
- [ ] Treat EKS as an intent provider until M11 owns trusted kubeconfig/exec
  semantics. Never invoke a default-merging `update-kubeconfig` against the
  user’s config; require a reviewed non-mutating/isolated workflow.
- [ ] Test profile precedence, PKCE/device selection, missing/expired/denied/MFA/
  cancelled/offline states, production risk, session isolation, redaction,
  revocation, disable/uninstall, fake CLI argv, and native optional tools.

Exit: AWS can be enabled/revoked/released independently and uses no ambient
credential or global-profile mutation.

### M9 — F9 Azure slice

Status: Not done; depends on M7, with AKS execution also depending on M11.

- [ ] Create an independently enabled Azure extension that reads only bounded
  public configuration, tenants, subscriptions, cloud, and selected-account
  metadata from an exact user-granted source; it must not read tokens.
- [ ] Use visible `az login` through M2, preserving Azure CLI WAM/browser/device
  and MFA behavior. Distinguish user MFA from service principal, managed, and
  workload identity in public state and recovery guidance.
- [ ] Pin tenant, subscription, cloud, public account kind/reference, freshness,
  expiry, provenance, and risk per capsule. Never run `az account set` as a
  managed switch.
- [ ] Add reviewed Azure Bastion native-client requests with exact resource and
  authentication type. Treat AKS credential/context generation as M11 work;
  avoid the CLI’s default merge into the user kubeconfig and require a reviewed
  isolated file/intent path.
- [ ] Test subscription/tenant changes, WAM/browser/device behavior, MFA,
  conditional-access failure, cancellation, offline, isolation, redaction,
  uninstall, fake CLI argv, and native Bastion availability.

Exit: Azure is independently enabled and never converts a global CLI selection
into hidden per-pane authority.

### M10 — F10 Google Cloud slice

Status: Not done; depends on M7, with GKE execution also depending on M11.

- [ ] Create an independently enabled GCP extension that parses bounded named
  configurations and public account/project/region/zone hints from exact granted
  sources, never copied credentials or external-account token data.
- [ ] Use visible Google CLI-owned user, Workforce, or Workload Identity login
  only after M2 review. Research and document the supported configuration/
  environment isolation mechanism before using it; do not mutate the global
  active configuration.
- [ ] Pin configuration reference, account reference, project, region/zone,
  freshness, expiry, provenance, and risk per capsule.
- [ ] Add exact IAP/OS Login flows through reviewed `gcloud` operations. Treat
  GKE credential generation/context as M11 work and never overwrite user
  kubeconfig as a side effect.
- [ ] Test untrusted external credential configuration, 2FA, IAM denial,
  configuration precedence, cancellation, offline, stale capsule isolation,
  redaction, revocation, and uninstall.

Exit: GCP context is per-session and official CLI credential ownership remains
intact.

### M11 — F11 Kubernetes and OpenShift slice

Status: Not done; depends on M7 and is a hard prerequisite for managed EKS/AKS/
GKE context execution.

- [ ] Treat every kubeconfig source as code-capable untrusted input. Require
  explicit exact source grants; reject oversized documents, relative/untrusted
  credential paths, symlink/reparse races, merge collisions, duplicate identity
  ambiguity, and source changes after review.
- [ ] Implement bounded public parsing of cluster/context/user-reference/
  namespace/project metadata while preserving `kubectl`/`oc` merge and
  precedence semantics. Keep a public source/provenance/freshness label.
- [ ] Deny `user.exec` credential plugins by default. A future enablement needs
  exact executable identity/digest, ordered argv, allowed environment names,
  interactivity policy, capability review, deadline/output/cancellation limits,
  and a specific session/capsule grant. Keep `ExecCredential` and key data
  memory-only and redacted.
- [ ] Pin exact source set, cluster, context, user reference, namespace/project,
  provider relation, freshness/expiry, provenance, and risk in each capsule.
- [ ] Add reviewed exact `kubectl`/`oc` login, context inspection, `exec`/`rsh`,
  and cloud-cluster workflows. Never make an implicit global `use-context` or
  project change on behalf of a managed pane.
- [ ] Test hostile YAML/JSON, controls/bidi, relative paths, symlink swaps,
  KUBECONFIG merge ordering, exec-plugin rejection/approval, token redaction,
  cross-pane isolation, cancellation, expiry/offline/denial/plugin failure,
  cleanup, disable/uninstall, and real client fixtures where available.

Exit: Kubernetes/OpenShift has no ambient executable credential or global
context authority and is independently releasable.

### M12 — F12 organization identity: Teleport, then OpenBao

Status: Not done; implement in two independently approved releases.

#### M12.1 Teleport

- [ ] Add a separate Teleport extension using reviewed exact `tsh version`,
  `tsh login`, `tsh status`, and `tsh ssh` flows. Teleport’s cache, certificate,
  SSH agent integration, browser, MFA, and expiry remain authoritative.
- [ ] Read/publicize only bounded status, cluster/proxy reference, permitted
  public target metadata, certificate freshness/expiry, and provenance; never
  import `~/.tsh` credentials or agent material.
- [ ] Test login/status/expiry/revocation/offline/cancelled states, target and
  proxy changes, agent opt-out, exact argv, session isolation, redaction,
  disable/uninstall, and native fixture behavior.

#### M12.2 OpenBao SSH certificates

- [ ] Write and obtain a separate security ADR/review for OpenBao’s token-helper
  and certificate-file boundary before any implementation. This is not covered
  by Teleport or generic OpenSSH approval.
- [ ] Keep token/helper interaction external; retain only opaque references and
  bounded public certificate metadata. Do not accept password/private-key
  material or serialize certificate contents as an Automexia credential store.
- [ ] Give it independent capability grant, cache, revocation, certificate
  expiry/cleanup, native fixtures, recovery/uninstall behavior, and docs.

Exit: Teleport and OpenBao may be installed, enabled, revoked, and removed
independently; neither grants authority to the other.

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
